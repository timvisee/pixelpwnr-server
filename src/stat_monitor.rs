extern crate time;

use std::fmt;

use self::time::PreciseTime;

/// The maximum number of ticks that may be remembered to use for calculations.
/// You don't want too many ticks, as the calculation would possibly be
/// averaged over a longer time.
/// There must be at least 2 ticks.
const TICKS_MAX: usize = 10;

/// The minimum allowed interval between registered ticks.
/// This prevents too many ticks from being collected in a short time frame,
/// possibly making the result unreliable.
const TICKS_MIN_INTERVAL_MICRO: u64 = 100_000;

/// The maximum age of a registered tick.
/// This prevents old and now incorrect data from affecting the calculation.
/// If the tick has been registered longer ago than the specified age, it is
/// dropped.
/// This value is also the maximum period the calculated result is averaged at.
const TICKS_MAX_AGE_MICRO: u64 = 2_500_000;

/// A monitor, that measures a cumulative value over time to report how much
/// the value grows each second.
///
/// This may be used to determine the current data rate per second for some
/// stream, based on a cumulative amount.
///
/// This monitor remembers the values in various points in time (called ticks),
/// to determine the result as reliably as possible.
/// Note that the result is thus approximate (and not exact).
pub struct StatMonitor {
    ticks: Vec<(usize, PreciseTime)>,
}

impl StatMonitor {
    /// Construct a new monitor.
    pub fn new() -> Self {
        StatMonitor {
            ticks: Vec::new(),
        }
    }

    /// Update the monitor by pushing a new value.
    /// The value should be updated about once or twice a second for an optimal result.
    ///
    /// A result is calculated based on the input, which is then returned.
    /// Note that this value is approximate and that `None` might be returned
    /// in some cases. See the documentation of `calculate()` for more details.
    pub fn update(&mut self, value: usize) -> Option<f64> {
        // Get the current time, and the last time that was recorded
        let now = PreciseTime::now();
        let last = self.ticks.first().map(|i| i.1);

        // The difference between now and the last time must be large enough
        if let Some(last) = last {
            if (last.to(now).num_microseconds().unwrap() as u64) < TICKS_MIN_INTERVAL_MICRO {
                return self.calculate();
            }
        }

        // Add the timing to the list of ticks
        self.ticks.insert(0, (value, now));

        // Decay old and invalid ticks
        self.decay(now);

        // Calculate the result and return
        self.calculate()
    }

    /// Decay any ticks that have become outdated.
    /// This cleans up the list of ticks, to ensure the list is up-to-date
    /// for reliable calculations.
    ///
    /// This removes ticks if there are too many in the list.
    /// And it removes ticks that have been in the list for too long.
    ///
    /// The current time should be passed to `now`.
    fn decay(&mut self, now: PreciseTime) {
        // Truncate ticks if we have to many
        self.ticks.truncate(TICKS_MAX);

        // Make sure there are any ticks that may be to old
        if !self.ticks.is_empty() {
            // Get the lifetime of the oldest tick
            let old = self.ticks.last().unwrap().1;

            // If the tick is too old, truncate all ticks that are too old
            if old.to(now).num_microseconds().unwrap() as u64 > TICKS_MAX_AGE_MICRO {
                // Find the truncation point and truncate
                if let Some(pos) = self.ticks.iter().position(
                    |&(_, time)| time.to(now).num_microseconds().unwrap() as u64 > TICKS_MAX_AGE_MICRO
                ) {
                    self.ticks.truncate(pos);
                }
            }
        }
    }

    /// Calculate the average value difference in the last second.
    /// If no value could be approximated because there is no data or the
    /// data is too old, `None` is returned.
    ///
    /// This value is approximate.
    /// The precision depends on the `TICKS_MAX`, `TICKS_MAX_AGE_MICRO`
    /// constant values, and on the data that is available.
    fn calculate(&self) -> Option<f64> {
        // There must be at least two known ticks to approximate a result
        if self.ticks.len() < 2 {
            return None;
        }

        // Get the oldest and newest timings to calculate with
        let old = self.ticks.last().unwrap();
        let new = self.ticks.first().unwrap();

        // Determine the difference in value and time
        let delta = (new.0 - old.0) as f64;
        let passed = old.1.to(new.1).num_microseconds().unwrap() as f64;

        // Return the approximate the value change each second
        Some(delta / passed * 1_000_000f64)
    }

    /// Reset the monitor.
    pub fn reset(&mut self) {
        self.ticks.clear();
    }
}

impl fmt::Debug for StatMonitor {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "StatMonitor")
    }
}
