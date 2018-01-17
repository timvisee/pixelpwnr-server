extern crate time;

use self::time::PreciseTime;

/// The maximum number of frame times that is stored.
const FRAME_BUFFER_MAX: usize = 512;

/// The maximum time in milliseconds a frame time is remembered.
const FRAME_TTL_MS: i64 = 5000;

/// The interval in milliseconds to report the FPS count at to the console.
const REPORT_INTERVAL_MS: i64 = 1000;

/// The minimum number of frames to collect before the first report.
const REPORT_FIRST_FRAMES_MIN: usize = 5;

/// An accurate FPS counter, that remembers samples frame times in order to
/// calculate the current FPS as accurratly as possible.
///
/// The frame sampler has a limited size, when it grows to big, the oldest
/// frames are dropped.
/// Frames that become outdated (frames that have been timed more than 5
/// seconds ago) are also dropped.
///
/// To count frames, the `tick()` method must be called repeatedly, to sample
/// a frame.
pub struct FpsCounter {
    /// A history of frame times, used to calculate the FPS.
    frames: Vec<PreciseTime>,

    /// The time the FPS was last reported at, none if not reported.
    last_report: Option<PreciseTime>,
}

impl FpsCounter {
    /// Create a new FPS counter.
    pub fn new() -> FpsCounter {
        FpsCounter {
            frames: Vec::with_capacity(FRAME_BUFFER_MAX),
            last_report: None,
        }
    }

    /// Tick/count a new frame, and report the FPS.
    pub fn tick(&mut self) {
        // Make sure there's enough room in the vector
        if self.frames.len() >= FRAME_BUFFER_MAX {
            self.frames.remove(0);
        }

        // Add the current time to the list
        self.frames.push(PreciseTime::now());

        // Periodically report the FPS
        self.report_periodically();
    }

    /// Calculate the FPS based on the known frame times.
    ///
    /// If we are unable to calculate the FPS, None is returned.
    pub fn calculate_fps(&mut self) -> Option<f64> {
        // Clean up the frame times database
        self.cleanup_frames();

        // Make sure we have at least one frame available
        if self.frames.len() == 0 {
            return None;
        }

        // Find the numbers of milliseconds passed since the first frame
        let passed = self.frames[0].to(PreciseTime::now()).num_microseconds()?;

        // Calculate the FPS
        Some(
            (self.frames.len() as f64) / ((passed as f64) / 1_000_000f64)
        )
    }

    /// Report the FPS to the console periodically.
    /// By default this happens each second.
    ///
    /// If this method is invoked but the FPS has been reported too recently,
    /// nothing happens.
    pub fn report_periodically(&mut self) {
        // Make sure we've collected enough frames
        if self.last_report.is_none() && self.frames.len() < REPORT_FIRST_FRAMES_MIN {
            return;
        }

        // Check if the report time has passed
        if let Some(last_report) = self.last_report {
            // Calculate the passed time
            let passed = last_report.to(PreciseTime::now()).num_milliseconds();

            // Make sure enough time has passed
            if passed < REPORT_INTERVAL_MS {
                return;
            }
        }

        // Report
        self.report();
    }

    /// Report the current FPS to the console.
    pub fn report(&mut self) {
        // Calculate the FPS
        if let Some(fps) = self.calculate_fps() {
            // Report the FPS
            println!("FPS: {:.1}", fps);

            // Set the last report time
            self.last_report = Some(PreciseTime::now());
        }
    }

    /// Clean up frame times that are outdated, or excessive frames if we
    /// have too many.
    fn cleanup_frames(&mut self) {
        // Drain frames if we're over the buffer maximum
        if self.frames.len() > FRAME_BUFFER_MAX {
            // Count the frames to remove, and drain it
            let overhead = self.frames.len() - FRAME_BUFFER_MAX;
            self.frames.drain(0..overhead);
        }

        // Find the number of outdated/dead frames
        let now = PreciseTime::now();
        let dead = self.frames
            .iter()
            .take_while(
                |frame| frame.to(now).num_milliseconds() > FRAME_TTL_MS
            )
            .count();

        // Remove the dead frames
        if dead > 0 {
            self.frames.drain(0..dead);
        }
    }
}
