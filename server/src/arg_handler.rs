use std::path::PathBuf;
use std::time::Duration;
use std::{net::SocketAddr, str::FromStr};

use clap::{Args, Parser};

use crate::codec::{CodecOptions, RateLimit};

#[derive(Parser, Clone)]
pub struct Opts {
    /// The host to bind to
    #[clap(long, default_value = "0.0.0.0:1337", alias = "bind")]
    pub host: SocketAddr,

    /// Canvas width (def: screen width)
    #[clap(short, long, value_name = "PIXELS")]
    pub width: Option<usize>,

    /// Canvas heigth (def: screen height)
    #[clap(short, long, value_name = "PIXELS")]
    pub height: Option<usize>,

    /// Do not render the canvas
    #[clap(long)]
    pub no_render: bool,

    /// Render in fullscreen
    #[clap(short, long)]
    pub fullscreen: bool,

    #[clap(flatten)]
    pub stat_options: StatsOptions,

    /// Reporting interval of stats on screen
    #[clap(long, value_name = "SECONDS", alias = "stats-screen-interval")]
    stats_screen: Option<u64>,

    /// Reporting interval of stats to stdout
    #[clap(
        long,
        alias = "stats-console",
        alias = "stats-terminal",
        alias = "stats-stdout-interval"
    )]
    stats_stdout: Option<u64>,

    /// Screen stats font size in pixels
    #[clap(long, value_name = "PX", default_value = "20", alias = "font-size")]
    pub stats_font_size: u8,

    /// Screen stats padding
    #[clap(long, value_name = "PX", default_value = "12", alias = "padding")]
    pub stats_padding: i32,

    /// Screen stats offset
    #[clap(long, value_name = "XxY", default_value = "10x10", alias = "offset")]
    stats_offset: String,

    /// Screen stats column spacing
    #[clap(long, alias = "stats-column-spacing", default_value = "20")]
    pub stats_col_spacing: i32,

    /// The directory under which to save images.
    #[clap(long, short)]
    pub save_dir: Option<PathBuf>,

    /// The interval at which to save the current frame, in seconds
    ///
    /// This value is only relevant if --save-dir is specified
    #[clap(long, default_value = "60")]
    pub save_interval: u64,

    /// The maximum bandwidth at which a single client is
    /// allowed to send data to the server, in bits per second. Default is unlimited.
    #[clap(long)]
    pub bw_limit: Option<usize>,

    /// Disable binary commands
    #[clap(long)]
    pub no_binary: bool,
}

#[derive(Clone, Debug, Args)]
pub struct StatsOptions {
    /// The interval at which to save stats.
    #[clap(long)]
    pub stats_save_interval_ms: Option<u64>,

    /// If this is set, the stats will be loaded using the provided
    /// method.
    #[cfg_attr(feature = "influxdb2", doc = "(Available: file, influxdb)")]
    #[cfg_attr(not(feature = "influxdb2"), doc = "(Available: file)")]
    #[clap(long)]
    pub load_on_start: Option<StatsSaveMethod>,

    /// The path that the stats should be saved to, and optionally loaded
    /// from if `load-on-startup` is `file`.
    pub stats_file: Option<PathBuf>,

    /// The YAML configuration file describing to what influxdb2 the
    /// stats should be written (over HTTP)
    #[cfg(feature = "influxdb2")]
    #[clap(long = "influxdb-config")]
    stats_influxdb_config: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatsSaveMethod {
    File,
    #[cfg(feature = "influxdb2")]
    Influxdb,
}

impl FromStr for StatsSaveMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let method = match s.to_ascii_lowercase().as_str() {
            "file" => Self::File,
            #[cfg(feature = "influxdb2")]
            "influxdb" => Self::Influxdb,
            _ => return Err(format!("Unknown save method {s}")),
        };
        Ok(method)
    }
}

macro_rules! map_duration {
    ($val: expr) => {
        $val.map(|d| Duration::from_secs(d))
    };
}

impl StatsOptions {
    pub fn stats_save_interval(&self) -> Option<Duration> {
        self.stats_save_interval_ms.map(Duration::from_millis)
    }
}

#[cfg(feature = "influxdb2")]
impl StatsOptions {
    pub fn influxdb_config(&self) -> Option<crate::influxdb::InfluxDBOptions> {
        let config = self.stats_influxdb_config.as_ref()?;

        let file = match std::fs::File::open(config) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "Could not open influxdb config file (\"{}\"). {e}",
                    config.as_os_str().to_str().unwrap()
                );
                std::process::exit(1);
            }
        };

        let options = match serde_yaml::from_reader(file) {
            Ok(o) => o,
            Err(e) => {
                log::error!(
                    "Failed to parse influxdb config (\"{}\"). {e}",
                    config.as_os_str().to_str().unwrap()
                );
                std::process::exit(1);
            }
        };

        Some(options)
    }
}

impl Opts {
    /// Get the canvas size.
    pub fn size(&self) -> (usize, usize) {
        // TODO: use the current screen size as default here
        (self.width.unwrap_or(800), self.height.unwrap_or(600))
    }

    /// Get the stats screen interval
    pub fn stats_screen_interval(&self) -> Option<Duration> {
        map_duration!(self.stats_screen)
    }

    /// Get the stats stdout interval
    pub fn stats_stdout_interval(&self) -> Option<Duration> {
        map_duration!(self.stats_stdout)
    }

    /// Get the stats screen offset
    pub fn stats_offset(&self) -> (u32, u32) {
        let lower_case = self.stats_offset.to_lowercase();
        let mut parts = lower_case.split("x");

        if parts.clone().count() != 2 {
            log::error!("Invalid stats offset");
            std::process::exit(1);
        }

        (
            parts
                .next()
                .unwrap()
                .parse()
                .expect("Invalid X offset for stats"),
            parts
                .next()
                .unwrap()
                .parse()
                .expect("Invalid Y offset for stats"),
        )
    }
}

impl From<Opts> for CodecOptions {
    fn from(opts: Opts) -> Self {
        CodecOptions {
            rate_limit: opts
                .bw_limit
                .map(|bps| RateLimit::BitsPerSecond { limit: bps }),
            allow_binary_cmd: !opts.no_binary,
        }
    }
}
