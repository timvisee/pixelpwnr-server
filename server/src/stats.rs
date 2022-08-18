extern crate number_prefix;

use parking_lot::Mutex;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use self::number_prefix::NumberPrefix::{self, Prefixed, Standalone};
use serde::{Deserialize, Serialize};

use crate::stat_monitor::StatMonitor;

/// A statistics manager, that keeps track of some statistics while the
/// server is running.
///
/// For example, this manager tracks how many pixels have been written by
/// clients in total or in the last seconds. And how many bytes have been read
/// from clients.
pub struct Stats {
    /// The number of clients that are currently connected.
    clients: AtomicUsize,

    /// The total number of pixels that have been written by clients to the
    /// screen.
    pixels: AtomicUsize,

    /// A monitor for the number of pixels beign written this second.
    pixels_monitor: Mutex<StatMonitor>,

    /// The total amount of bytes that have been read.
    bytes_read: AtomicUsize,

    /// A monitor for the number of bytes being read this second.
    bytes_read_monitor: Mutex<StatMonitor>,
}

impl Stats {
    /// Construct a new stats object.
    pub fn new() -> Self {
        Stats {
            pixels: AtomicUsize::new(0),
            pixels_monitor: Mutex::new(StatMonitor::new()),
            clients: AtomicUsize::new(0),
            bytes_read: AtomicUsize::new(0),
            bytes_read_monitor: Mutex::new(StatMonitor::new()),
        }
    }

    /// Get the total number of clients currently connected.
    pub fn clients(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients.
    pub fn pixels(&self) -> usize {
        self.pixels.load(Ordering::Relaxed)
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients as a string in a humanly readable format.
    pub fn pixels_human(&self) -> String {
        match NumberPrefix::decimal(self.pixels() as f64) {
            Standalone(b) => format!("{:.00} P", b.ceil()),
            Prefixed(p, n) => {
                if n < 10f64 {
                    format!("{:.02} {}P", n, p)
                } else if n < 100f64 {
                    format!("{:.01} {}P", n, p)
                } else {
                    format!("{:.00} {}P", n, p)
                }
            }
        }
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients in the last second. The returned value is approximate.
    ///
    /// If the number of pixels in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn pixels_sec(&self) -> Option<f64> {
        // Get a lock on the value monitor, update and retrieve the result
        self.pixels_monitor.lock().update(self.pixels())
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients in the last second as a string in a humanly readable
    /// format. The returned value is approximate.
    ///
    /// If the number of pixels in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn pixels_sec_human(&self) -> String {
        match self.pixels_sec() {
            Some(px) => match NumberPrefix::decimal(px) {
                Standalone(b) => format!("{:.00} P/s", b.ceil()),
                Prefixed(p, n) => {
                    if n < 10f64 {
                        format!("{:.02} {}P/s", n, p)
                    } else if n < 100f64 {
                        format!("{:.01} {}P/s", n, p)
                    } else {
                        format!("{:.00} {}P/s", n, p)
                    }
                }
            },
            None => String::from("~"),
        }
    }

    /// Increment the number of clients that are connected, by one.
    pub fn inc_clients(&self) {
        self.clients.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrease the number of clients that are connected, by one.
    pub fn dec_clients(&self) {
        self.clients.fetch_sub(1, Ordering::SeqCst);
    }

    /// Increase the number of pixels that have been written to the screen by
    /// n.
    ///
    /// This method must be called by the logic chaining pixels in
    /// the server, to update the number of changed pixels.
    /// This method should not be invoked by something else to prevent
    /// poisoning the statistics.
    pub fn inc_pixels_by_n(&self, n: usize) {
        self.pixels.fetch_add(n, Ordering::Relaxed);
    }

    /// Get the total number of bytes that have been read from clients.
    pub fn bytes_read(&self) -> usize {
        self.bytes_read.load(Ordering::SeqCst)
    }

    /// Get the total number of bytes that have been read from clients
    /// as a string in a humanly readable format.
    pub fn bytes_read_human(&self) -> String {
        match NumberPrefix::binary(self.bytes_read() as f64) {
            Standalone(b) => format!("{:.00} B", b.ceil()),
            Prefixed(p, n) => {
                if n < 10f64 {
                    format!("{:.02} {}B", n, p)
                } else if n < 100f64 {
                    format!("{:.01} {}B", n, p)
                } else {
                    format!("{:.00} {}B", n, p)
                }
            }
        }
    }

    /// Get the total number of bytes that have been read from clients in the
    /// last second. The returned value is approximate.
    ///
    /// If the number of read bytes in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn bytes_read_sec(&self) -> Option<f64> {
        // Get a lock on the value monitor, update and retrieve the result
        self.bytes_read_monitor.lock().update(self.bytes_read())
    }

    /// Get the total number of bytes that have been read from clients in the
    /// last second as a string in a humanly readable format.
    /// The returned value is approximate.
    ///
    /// If the number of read bytes in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn bytes_read_sec_human(&self) -> String {
        match self.bytes_read_sec() {
            Some(bytes) => match NumberPrefix::decimal(bytes * 8f64) {
                Standalone(b) => format!("{:.00} b/s", b.ceil()),
                Prefixed(p, n) => {
                    if n < 10f64 {
                        format!("{:.02} {}b/s", n, p)
                    } else if n < 100f64 {
                        format!("{:.01} {}b/s", n, p)
                    } else {
                        format!("{:.00} {}b/s", n, p)
                    }
                }
            },
            None => String::from("~"),
        }
    }

    /// Increase the number of bytes that have been read from clients by the
    /// given `amount`.
    ///
    /// This method must be called by the logic reading bytes from clients,
    /// to update the number of changed pixels.
    /// This method should not be invoked by something else to prevent
    /// poisoning the statistics.
    pub fn inc_bytes_read(&self, amount: usize) {
        self.bytes_read.fetch_add(amount, Ordering::SeqCst);
    }

    /// Load data from the given raw stats object.
    /// This overwrites the current stats data.
    pub fn from_raw(raw: &StatsRaw) -> Self {
        // Store the values

        let mut me = Self::new();
        me.pixels = AtomicUsize::new(raw.pixels);
        me.bytes_read = AtomicUsize::new(raw.bytes_read);
        me.pixels_monitor.lock().reset();
        me.bytes_read_monitor.lock().reset();

        me
    }

    /// Convert this data in a raw stats object.
    pub fn to_raw(&self) -> StatsRaw {
        StatsRaw::new(self.pixels(), self.bytes_read())
    }
}

/// A struct that contains raw stats data.
/// This struct can be used to store and load stats data.
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsRaw {
    /// The total number of pixels that have been written by clients to the
    /// screen.
    pub pixels: usize,

    /// The total amount of bytes that have been read.
    pub bytes_read: usize,
}

impl StatsRaw {
    /// Construct a new raw stats object.
    pub fn new(pixels: usize, bytes_read: usize) -> Self {
        Self { pixels, bytes_read }
    }

    /// Load the raw stats to the file at the given path.
    /// If no stats could be loaded, `None` is returned.
    pub fn load(path: &Path) -> Option<Self> {
        // Make sure the file exists
        if !path.is_file() {
            println!("Not loading persistent stats, file not found");
            return None;
        }

        // Open a file
        let mut file = File::open(path).expect("failed to open persistent stats file");

        // Create a buffer, read from the file
        let mut data = String::new();
        file.read_to_string(&mut data)
            .expect("failed to read persistent stats from file");

        // Load the raw state
        return serde_yaml::from_str(&data)
            .map_err(|_| println!("failed to load persistent stats, malformed data"))
            .ok();
    }

    /// Save the raw stats to the file at the given path.
    pub fn save(&self, path: &Path) {
        // Save the object to a string.
        let data = serde_yaml::to_string(&self).expect("failed to serialize");

        // Write the data to the file
        let mut file = File::create(path).expect("failed to create persistent stats file");
        file.write_all(data.as_bytes())
            .expect("failed to write to persistent stats file");
    }
}
