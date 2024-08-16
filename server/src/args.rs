use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

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

    /// Render with nearest-neighbor scaling, instead of linear
    #[clap(short, long)]
    pub nearest_neighbor: bool,

    /// Render in fullscreen
    #[clap(short, long)]
    pub fullscreen: bool,

    /// The file to use for persistent stats
    #[clap(long, alias = "file", value_name = "FILE")]
    pub stats_file: Option<PathBuf>,

    /// How often to save persistent stats
    #[clap(long, value_name = "SECONDS", alias = "stats-save-interval")]
    stats_file_interval: Option<u64>,

    /// Whether to show real-time stats on the top left [default: true].
    #[clap(long, action = clap::ArgAction::Set, value_name = "ENABLED", default_value_t = true)]
    pub stats_enabled: bool,

    /// Reporting interval of stats on screen [default: 1]
    #[clap(
        long,
        value_name = "SECONDS",
        alias = "stats-screen",
        alias = "stats-screen-interval",
        default_value_t = 1.0
    )]
    stats_interval: f32,

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

    /// Custom host to connect to in stats [default: host]
    #[clap(long, value_name = "DISPLAY_HOST")]
    pub stats_host: Option<SocketAddr>,

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

macro_rules! map_duration {
    ($val: expr) => {
        $val.map(|d| Duration::from_secs(d))
    };
}

impl Opts {
    /// Get the canvas size.
    pub fn size(&self) -> (usize, usize) {
        // TODO: use the current screen size as default here
        (self.width.unwrap_or(800), self.height.unwrap_or(600))
    }

    /// Get the stats save interval
    pub fn stats_save_interval(&self) -> Option<Duration> {
        map_duration!(self.stats_file_interval)
    }

    /// Get the stats screen interval
    pub fn stats_screen_interval(&self) -> Option<Duration> {
        self.stats_enabled
            .then_some(Duration::from_secs_f32(self.stats_interval))
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
            panic!("Invalid stats offset");
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
