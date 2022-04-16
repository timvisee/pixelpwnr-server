use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use clap::{App, Arg, ArgMatches};

use crate::app::*;

/// CLI argument handler.
pub struct ArgHandler<'a> {
    matches: ArgMatches<'a>,
}

impl<'a: 'b, 'b> ArgHandler<'a> {
    /// Parse CLI arguments.
    pub fn parse() -> ArgHandler<'a> {
        // Handle/parse arguments
        let matches = App::new(APP_NAME)
            .version(APP_VERSION)
            .author(APP_AUTHOR)
            .about(APP_ABOUT)
            .arg(
                Arg::with_name("bind")
                    .short("b")
                    .long("bind")
                    .alias("host")
                    .value_name("HOST")
                    .help("Host to bind to \"host:port\"")
                    .default_value("0.0.0.0:1337")
                    .display_order(1),
            )
            .arg(
                Arg::with_name("no-render")
                    .long("no-render")
                    .help("Do not render the canvas"),
            )
            .arg(
                Arg::with_name("width")
                    .short("w")
                    .long("width")
                    .value_name("PIXELS")
                    .help("Canvas width (def: screen width)")
                    .display_order(2)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("height")
                    .short("h")
                    .long("height")
                    .value_name("PIXELS")
                    .help("Canvas height (def: screen height)")
                    .display_order(3)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("fullscreen")
                    .long("fullscreen")
                    .short("f")
                    .help("Render in full screen"),
            )
            .arg(
                Arg::with_name("stats-file")
                    .long("stats-file")
                    .alias("file")
                    .value_name("FILE")
                    .help("File to use for persistent stats")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stats-file-interval")
                    .long("stats-file-interval")
                    .alias("stats-save-interval")
                    .value_name("SECONDS")
                    .help("How often to save persistent stats")
                    .default_value("60")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stats-screen")
                    .long("stats-screen")
                    .value_name("SECONDS")
                    .help("Reporting interval of stats on screen")
                    .default_value("1")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stats-stdout")
                    .long("stats-stdout")
                    .alias("stats-console")
                    .alias("stats-terminal")
                    .value_name("SECONDS")
                    .help("Reporting interval of stats to stdout")
                    .default_value("5")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stats-font-size")
                    .long("stats-font-size")
                    .alias("font-size")
                    .value_name("PX")
                    .help("Screen stats font size in pixels")
                    .default_value("20")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stats-padding")
                    .long("stats-padding")
                    .alias("font-size")
                    .value_name("PX")
                    .help("Screen stats padding")
                    .default_value("12")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stats-offset")
                    .long("stats-offset")
                    .value_name("XxY")
                    .help("Screen stats offset")
                    .default_value("10x10")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stats-col-spacing")
                    .long("stats-col-spacing")
                    .alias("stats-column-spacing")
                    .value_name("PX")
                    .help("Screen stats column spacing")
                    .default_value("20")
                    .takes_value(true),
            )
            .get_matches();

        // Instantiate
        ArgHandler { matches }
    }

    /// Get the host to bind to.
    pub fn host(&'a self) -> SocketAddr {
        // Get the raw host
        let host = self
            .matches
            .value_of("bind")
            .expect("please specify a host to bind to");

        // Parse the host, and return
        host.parse().expect("invalid host given")
    }

    /// Check whether to render a pixelflut window.
    pub fn no_render(&self) -> bool {
        self.matches.is_present("no-render")
    }

    /// Get the canvas size.
    pub fn size(&self) -> (usize, usize) {
        // TODO: use the current screen size as default here
        (
            self.matches
                .value_of("width")
                .map(|width| width.parse().expect("Invalid image width"))
                .unwrap_or(800),
            self.matches
                .value_of("height")
                .map(|height| height.parse().expect("Invalid image height"))
                .unwrap_or(600),
        )
    }

    /// Check whether we should render in full screen.
    pub fn fullscreen(&self) -> bool {
        self.matches.is_present("fullscreen")
    }

    /// Get the file to use for persistent stats.
    pub fn stats_file(&self) -> Option<PathBuf> {
        self.matches
            .value_of("stats-file")
            .map(|path| PathBuf::from(path))
    }

    /// The interval of stats saving.
    pub fn stats_save_interval(&self) -> Option<Duration> {
        self.matches
            .value_of("stats-file-interval")
            .expect("missing stats save interval option")
            .parse::<f64>()
            .map(|sec: f64| (sec * 1000f64) as u64)
            .map(|millis| {
                if millis > 0 {
                    Some(Duration::from_millis(millis))
                } else {
                    panic!("invalid stats save interval, must be >1ms");
                }
            })
            .expect("invalid stats save interval, must be number of seconds")
    }

    /// The interval of stats reporting on the screen.
    ///
    /// If no stats should be reported, `None` is returned.
    pub fn stats_screen_interval(&self) -> Option<Duration> {
        self.matches
            .value_of("stats-screen")
            .map(|raw| {
                raw.parse::<f64>()
                    .map(|sec: f64| (sec * 1000f64) as u64)
                    .expect("invalid screen stats update interval, must be number of seconds")
            })
            .map(|millis| {
                if millis > 0 {
                    Some(Duration::from_millis(millis))
                } else if millis == 0 {
                    None
                } else {
                    panic!("invalid screen stats update interval, must be 0 or >1ms");
                }
            })
            .unwrap()
    }

    /// The interval of stats reporting to stdout.
    ///
    /// If no stats should be reported, `None` is returned.
    pub fn stats_stdout_interval(&self) -> Option<Duration> {
        self.matches
            .value_of("stats-stdout")
            .map(|raw| {
                raw.parse::<f64>()
                    .map(|sec: f64| (sec * 1000f64) as u64)
                    .expect("invalid stdout stats update interval, must be number of seconds")
            })
            .map(|millis| {
                if millis > 0 {
                    Some(Duration::from_millis(millis))
                } else if millis == 0 {
                    None
                } else {
                    panic!("invalid stdout stats update interval, must be 0 or >1ms");
                }
            })
            .unwrap()
    }

    /// Get the font size to use for the status text on the screen.
    pub fn stats_font_size(&self) -> u8 {
        self.matches
            .value_of("stats-font-size")
            .map(|raw| raw.parse::<u8>().expect("invalid font size"))
            .unwrap()
    }

    /// Get the number of pixels to offset the status with.
    pub fn stats_offset(&self) -> (u32, u32) {
        self.matches
            .value_of("stats-offset")
            .map(|raw| {
                raw.to_lowercase()
                    .split("x")
                    .map(|val| val.to_owned())
                    .collect()
            })
            .map(|sizes: Vec<String>| {
                if sizes.len() != 2 {
                    panic!("invalid stats offset");
                } else {
                    (sizes[0].clone(), sizes[1].clone())
                }
            })
            .map(|(x, y)| {
                (
                    x.parse().expect("invalid x offset for stats"),
                    y.parse().expect("invalid x offset for stats"),
                )
            })
            .unwrap()
    }

    /// Get the number of pixels to use for padding for the stats on the screen.
    pub fn stats_padding(&self) -> i32 {
        self.matches
            .value_of("stats-padding")
            .map(|raw| raw.parse::<i32>().expect("invalid padding size"))
            .unwrap()
    }

    /// Get the number of pixels for column spacing in the stats.
    pub fn stats_column_spacing(&self) -> i32 {
        self.matches
            .value_of("stats-col-spacing")
            .map(|raw| raw.parse::<i32>().expect("invalid column spacing size"))
            .unwrap()
    }
}
