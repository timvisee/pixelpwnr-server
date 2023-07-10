use parking_lot::Mutex;
use std::cmp::min;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::stats::Stats;

/// A struct that is used to periodically report stats.
pub struct StatReporter {
    /// The interval to update the screen stats with.
    /// If none, no screen stats should be reported.
    screen_interval: Option<Duration>,

    /// The interval to update the stdout stats with.
    /// If none, no screen stats should be reported.
    stdout_interval: Option<Duration>,

    /// The interval to save the persistent file and/or influxdb2 with.
    /// If none, no stats will be saved.
    save_interval: Option<Duration>,

    /// The file to save persistent stats to.
    save_path: Option<PathBuf>,

    /// A stats manager.
    stats: Arc<Stats>,

    /// A string mutex for text on the screen.
    screen: Option<Arc<Mutex<String>>>,

    /// The InfluxDB client to report stats to
    #[cfg(feature = "influxdb2")]
    influxdb_client: Option<crate::influxdb::InfluxDB>,
}

impl StatReporter {
    /// Construct a new stats reporter.
    pub fn new(
        screen_interval: Option<Duration>,
        stdout_interval: Option<Duration>,
        save_interval: Option<Duration>,
        save_path: Option<PathBuf>,
        stats: Arc<Stats>,
        screen: Option<Arc<Mutex<String>>>,
        #[cfg(feature = "influxdb2")] influxdb_client: Option<crate::influxdb::InfluxDB>,
    ) -> Self {
        StatReporter {
            screen_interval,
            stdout_interval,
            save_interval,
            save_path,
            stats,
            screen: screen,
            #[cfg(feature = "influxdb2")]
            influxdb_client,
        }
    }

    /// Start the reporter, and spawn a thread internally which controls the
    /// reporting.
    pub async fn run(mut self) {
        // Do not actually start a thread if there is nothing to report
        let should_stop = self.screen_interval.is_none() && self.stdout_interval.is_none();

        #[cfg(feature = "influxdb2")]
        let should_stop = should_stop && self.influxdb_client.is_none();

        if should_stop {
            return;
        }

        // Clone the arcs for use in the reporter thread
        let mut screen_last = Instant::now();
        let mut stdout_last = Instant::now();
        let mut save_last = Instant::now();

        // Update the statistics text each second in a separate thread

        loop {
            // When the next update should happen, at least once a second
            let mut next_update = Duration::from_secs(1);

            // Check the screen update time
            if let Some(interval) = self.screen_interval {
                // Get the number of elapsed seconds since the last report
                let elapsed = screen_last.elapsed();

                // Report stats to the screen
                if elapsed >= interval {
                    if let Some(screen) = &self.screen {
                        Self::report_screen(&self.stats, &mut screen.lock());
                        screen_last = Instant::now();
                    }
                }

                // See how long we should take, update the next update time
                next_update = min(
                    next_update,
                    interval.checked_sub(elapsed).unwrap_or(interval),
                );
            }

            // Check the stdout update time
            if let Some(interval) = self.stdout_interval {
                // Get the number of elapsed seconds since the last report
                let elapsed = stdout_last.elapsed();

                // Report stats to the stdout
                if elapsed >= interval {
                    Self::report_stdout(&self.stats);
                    stdout_last = Instant::now();
                }

                // See how long we should take, update the next update time
                next_update = min(
                    next_update,
                    interval.checked_sub(elapsed).unwrap_or(interval),
                );
            }

            // Check the stats save update time
            if let Some(interval) = self.save_interval {
                // Get the number of elapsed seconds since the last save
                let elapsed = save_last.elapsed();

                // Report stats to the stdout
                if elapsed >= interval {
                    // Create a raw stats instance
                    log::debug!("Saving persistent stats...");
                    let raw = self.stats.to_raw();

                    // Save the raw stats
                    if let Some(save_path) = &self.save_path {
                        raw.save(save_path.as_path())
                    }

                    if let Some(client) = &mut self.influxdb_client {
                        if let Err(e) = client.write_stats(&self.stats).await {
                            log::error!("Failed to write stats to influxdb: {e}");
                        }
                    }

                    save_last = Instant::now();
                }

                // See how long we should take, update the next update time
                next_update = min(
                    next_update,
                    interval.checked_sub(elapsed).unwrap_or(interval),
                );
            }

            // Sleep for the specified duration
            tokio::time::sleep(next_update).await;
        }
    }

    /// Report the stats to the screen.
    fn report_screen(stats: &Stats, screen: &mut String) {
        *screen = format!(
            "CONNECT WITH:        \tpx:\t{}\t{}\tclients: {}\ntelnet localhost 1234        \tin:\t{}\t{}",
            stats.pixels_human(),
            stats.pixels_sec_human(),
            stats.clients(),
            stats.bytes_read_human(),
            stats.bytes_read_sec_human(),
        );
    }

    /// Report the stats to stdout.
    fn report_stdout(stats: &Arc<Stats>) {
        println!(
            "\
                {: <7} {: <15} {: <12}\n\
                {: <7} {: <15} {: <12}\n\
                {: <7} {: <15} {: <12}\
            ",
            "STATS",
            "Total:",
            "Per sec:",
            "Pixels:",
            stats.pixels_human(),
            stats.pixels_sec_human(),
            "Input:",
            stats.bytes_read_human(),
            stats.bytes_read_sec_human(),
        );
    }
}
