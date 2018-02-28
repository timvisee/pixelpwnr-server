extern crate time;

use std::fmt;

use self::time::PreciseTime;

// TODO: improve this stats monitor. Base the calculation on more than just one value.

/// A monitor, that measures a cumulative value over time to report how much
/// the value grows for a given interval.
///
/// This may be used to determine the current data rate per second for some
/// stream, based on a cumulative amount.
pub struct StatMonitor {
    /// The last known value.
    value: Option<u64>,

    /// The last time the value was updated
    time: Option<PreciseTime>,
}

impl StatMonitor {
    /// Construct a new monitor.
    pub fn new() -> Self {
        StatMonitor {
            value: None,
            time: None,
        }
    }

    /// Update the monitor, and return the growth amount this second.
    pub fn update(&mut self, value: u64) -> Option<u64> {
        // Get the current time
        let now = PreciseTime::now();

        // Determine the time that has passed, and the value difference
        let passed = self.time.map(|prev| prev.to(now).num_microseconds().unwrap_or(0));
        let delta = self.value.map(|prev| value - prev);

        // Update the value and time
        self.value = Some(value);
        self.time = Some(now);

        // If no difference could be calculated now, return none
        if passed.is_none() || delta.is_none() {
            return None;
        }

        // Unwrap the values
        let passed = (passed.unwrap() as u64) / 1_000_000u64;
        let delta = delta.unwrap();

        // Make sure some time has passed
        if passed == 0 {
            return None;
        }

        // Calculate the growth this second, and return
        Some((delta as u64) / passed)
    }
}

impl fmt::Debug for StatMonitor {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "StatMonitor")
    }
}
