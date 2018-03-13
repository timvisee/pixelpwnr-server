extern crate number_prefix;

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use self::number_prefix::{
    binary_prefix,
    decimal_prefix,
    Standalone,
    Prefixed,
};

use stat_monitor::StatMonitor;

/// A statistics manager, that keeps track of some statistics while the
/// server is running.
///
/// For example, this manager tracks how many pixels have been written by
/// clients in total or in the last seconds. And how many bytes have been read
/// from clients.
#[derive(Debug)]
pub struct Stats {
    /// The total number of pixels that have been written by clients to the
    /// screen.
    pixels: AtomicU64,

    /// A monitor for the number of pixels beign written this second.
    pixels_monitor: Mutex<StatMonitor>,

    /// The total amount of bytes that have been read.
    bytes_read: AtomicU64,

    /// A monitor for the number of bytes being read this second.
    bytes_read_monitor: Mutex<StatMonitor>,
}

impl Stats {
    /// Construct a new stats object.
    pub fn new() -> Self {
        Stats {
            pixels: AtomicU64::new(0),
            pixels_monitor: Mutex::new(StatMonitor::new()),
            bytes_read: AtomicU64::new(0),
            bytes_read_monitor: Mutex::new(StatMonitor::new()),
        }
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients.
    pub fn pixels(&self) -> u64 {
        self.pixels.load(Ordering::Relaxed)
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients as a string in a humanly readable format.
    pub fn pixels_human(&self) -> String {
        match decimal_prefix(self.pixels() as f64) {
            Standalone(b) => format!("{:.00} P", b.ceil()),
            Prefixed(p, n) => format!("{:.02} {}P", n, p),
        }
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients in the last second. The returned value is approximate.
    ///
    /// If the number of pixels in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn pixels_sec(&self) -> Option<f64> {
        // Get a lock on the value monitor, update and retrieve the result
        self.pixels_monitor.lock()
            .ok()?
            .update(self.pixels())
    }

    /// Get the total number of pixels that have been written to the screen
    /// by clients in the last second as a string in a humanly readable
    /// format. The returned value is approximate.
    ///
    /// If the number of pixels in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn pixels_sec_human(&self) -> String {
        match self.pixels_sec() {
            Some(px) =>
                match decimal_prefix(px) {
                    Standalone(b) => format!("{:.00} P/s", b.ceil()),
                    Prefixed(p, n) => format!("{:.02} {}P/s", n, p),
                },
            None => String::from("~"),
        }
    }

    /// Increase the number of pixels that have been written to the screen by
    /// one.
    ///
    /// This method must be called by the logic chaining pixels in
    /// the server, to update the number of changed pixels.
    /// This method should not be invoked by something else to prevent
    /// poisoning the statistics.
    pub fn inc_pixels(&self) {
        self.pixels.fetch_add(1, Ordering::SeqCst);
    }

    /// Get the total number of bytes that have been read from clients.
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    /// Get the total number of bytes that have been read from clients
    /// as a string in a humanly readable format.
    pub fn bytes_read_human(&self) -> String {
        match binary_prefix(self.bytes_read() as f64) {
            Standalone(b) => format!("{:.00} B", b.ceil()),
            Prefixed(p, n) => format!("{:.02} {}B", n, p),
        }
    }

    /// Get the total number of bytes that have been read from clients in the
    /// last second. The returned value is approximate.
    ///
    /// If the number of read bytes in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn bytes_read_sec(&self) -> Option<f64> {
        // Get a lock on the value monitor, update and retrieve the result
        self.bytes_read_monitor.lock()
            .ok()?
            .update(self.bytes_read())
    }

    /// Get the total number of bytes that have been read from clients in the
    /// last second as a string in a humanly readable format.
    /// The returned value is approximate.
    ///
    /// If the number of read bytes in this second couldn't be determined
    /// reliably, `None` is returned.
    pub fn bytes_read_sec_human(&self) -> String {
        match self.bytes_read_sec() {
            Some(bytes) =>
                match decimal_prefix(bytes * 8f64) {
                    Standalone(b) => format!("{:.00} b/s", b.ceil()),
                    Prefixed(p, n) => format!("{:.02} {}b/s", n, p),
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
        self.bytes_read.fetch_add(amount as u64, Ordering::SeqCst);
    }
}
