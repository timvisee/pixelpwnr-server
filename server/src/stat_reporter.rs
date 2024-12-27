use parking_lot::Mutex;
use std::cmp::min;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::{Duration, SystemTime};

use crate::stats::Stats;

const PUBLIC_IP_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// A struct that is used to periodically report stats.
pub struct StatReporter {
    /// The interval to update the screen stats with.
    /// If none, no screen stats should be reported.
    screen_interval: Option<Duration>,

    /// The interval to update the stdout stats with.
    /// If none, no screen stats should be reported.
    stdout_interval: Option<Duration>,

    /// The interval to save the persistent file with.
    /// If none, no stats will be saved.
    save_interval: Option<Duration>,

    /// The file to save persistent stats to.
    save_path: Option<PathBuf>,

    /// The last time the screen stats were updated.
    screen_last: Arc<Mutex<Option<SystemTime>>>,

    /// The last time the stdout stats were updated.
    stdout_last: Arc<Mutex<Option<SystemTime>>>,

    /// The last time the stats were saved.
    save_last: Arc<Mutex<Option<SystemTime>>>,

    /// A stats manager.
    stats: Arc<Stats>,

    /// A string mutex for text on the screen.
    screen: Arc<Option<Arc<Mutex<String>>>>,

    host: String,
    port: u16,
}

impl StatReporter {
    /// Construct a new stats reporter.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        screen_interval: Option<Duration>,
        stdout_interval: Option<Duration>,
        save_interval: Option<Duration>,
        save_path: Option<PathBuf>,
        stats: Arc<Stats>,
        screen: Option<Arc<Mutex<String>>>,
        host: String,
        port: u16,
    ) -> Self {
        StatReporter {
            screen_interval,
            stdout_interval,
            save_interval,
            save_path,
            screen_last: Arc::new(Mutex::new(None)),
            stdout_last: Arc::new(Mutex::new(None)),
            save_last: Arc::new(Mutex::new(None)),
            stats,
            screen: Arc::new(screen),
            host,
            port,
        }
    }

    /// Start the reporter, and spawn a thread internally which controls the
    /// reporting.
    pub fn start(&self) {
        // Do not actually start a thread if there is nothing to report
        if self.screen_interval.is_none() && self.stdout_interval.is_none() {
            return;
        }

        // Clone the arcs for use in the reporter thread
        let stats = self.stats.clone();
        let screen = self.screen.clone();
        let screen_interval = self.screen_interval;
        let stdout_interval = self.stdout_interval;
        let save_interval = self.save_interval;
        let screen_last = self.screen_last.clone();
        let stdout_last = self.stdout_last.clone();
        let save_last = self.save_last.clone();
        let save_path = self.save_path.clone();
        let host = self.host.clone();
        let port = self.port;

        let public_ips: Arc<Mutex<Vec<String>>> = Default::default();
        let public_ips2 = public_ips.clone();

        // Update the statistics text each second in a separate thread
        thread::spawn(move || {
            loop {
                // When the next update should happen, at least once a second
                let mut next_update = Duration::from_secs(1);

                // Check the screen update time
                if let Some(interval) = screen_interval {
                    // Get the last screen time
                    let mut last = screen_last.lock();

                    // Get the number of elapsed seconds since the last report
                    let elapsed = last
                        .map(|last| last.elapsed().ok())
                        .unwrap_or(None)
                        .unwrap_or(Duration::from_secs(0));

                    let connect = match public_ips.lock() {
                        ips if ips.len() >= 2 => {
                            format!("telnet {} {port}  (or {})", ips[0], ips[1..].join(", "),)
                        }
                        ips if ips.len() == 1 => {
                            format!("telnet {} {port}", ips[0],)
                        }
                        _ => format!("telnet {host} {port}"),
                    };

                    // Report stats to the screen
                    if last.is_none() || elapsed >= interval {
                        if let Some(ref screen) = *screen {
                            Self::report_screen(&stats, screen, &connect);
                            *last = Some(SystemTime::now());
                        }
                    }

                    // See how long we should take, update the next update time
                    next_update = min(
                        next_update,
                        interval.checked_sub(elapsed).unwrap_or(interval),
                    );
                }

                // Check the stdout update time
                if let Some(interval) = stdout_interval {
                    // Get the last stdout time
                    let mut last = stdout_last.lock();

                    // Get the number of elapsed seconds since the last report
                    let elapsed = last
                        .map(|last| last.elapsed().ok())
                        .unwrap_or(None)
                        .unwrap_or(Duration::from_secs(0));

                    // Report stats to the stdout
                    if last.is_none() || elapsed >= interval {
                        Self::report_stdout(&stats);
                        *last = Some(SystemTime::now());
                    }

                    // See how long we should take, update the next update time
                    next_update = min(
                        next_update,
                        interval.checked_sub(elapsed).unwrap_or(interval),
                    );
                }

                // Check the stats save update time
                if let Some(interval) = save_interval {
                    // Get the last save time
                    let mut last = save_last.lock();

                    // Get the number of elapsed seconds since the last save
                    let elapsed = last
                        .map(|last| last.elapsed().ok())
                        .unwrap_or(None)
                        .unwrap_or(Duration::from_secs(0));

                    // Report stats to the stdout
                    if last.is_none() || elapsed >= interval {
                        // Create a raw stats instance
                        println!("Saving persistent stats...");
                        let raw = stats.to_raw();

                        // Save the raw stats
                        if let Some(save_path) = &save_path {
                            raw.save(save_path.as_path())
                        }

                        *last = Some(SystemTime::now());
                    }

                    // See how long we should take, update the next update time
                    next_update = min(
                        next_update,
                        interval.checked_sub(elapsed).unwrap_or(interval),
                    );
                }

                // Sleep for the specified duration
                sleep(next_update);
            }
        });

        // Background task to refresh public IPs
        thread::spawn(move || loop {
            let ips = std::process::Command::new("bash")
                        .arg("-c")
                        .arg("ifconfig | grep 'inet ' | grep -E '(151\\.217\\.|94\\.45\\.)' | awk '{ print $2 }'")
                        .output()
                        .ok()
                        .and_then(|ips| String::from_utf8(ips.stdout).ok());

            if let Some(ips) = ips {
                *public_ips2.lock() = ips.lines().map(|ip| ip.trim().to_string()).collect();
            }

            sleep(PUBLIC_IP_REFRESH_INTERVAL);
        });
    }

    /// Report the stats to the screen.
    fn report_screen(stats: &Arc<Stats>, screen: &Arc<Mutex<String>>, connect: &str) {
        *screen.lock() = format!(
            "CONNECT WITH:        \tpx:\t{}\t{}\tclients: {}\n{}        \tin:\t{}\t{}",
            stats.pixels_human(),
            stats.pixels_sec_human(),
            stats.clients(),
            connect,
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
