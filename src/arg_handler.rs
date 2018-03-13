extern crate clap;
extern crate num_cpus;

use std::net::SocketAddr;
use std::time::Duration;

use clap::{Arg, ArgMatches, App};

use app::*;

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
            .arg(Arg::with_name("bind")
                .short("b")
                .long("bind")
                .alias("host")
                .value_name("HOST")
                .help("Host to bind to \"host:port\"")
                .default_value("0.0.0.0:1234")
                .display_order(1))
            .arg(Arg::with_name("no-render")
                .long("no-render")
                .help("Do not render the canvas"))
            .arg(Arg::with_name("width")
                .short("w")
                .long("width")
                .value_name("PIXELS")
                .help("Canvas width (def: screen width)")
                .display_order(2)
                .takes_value(true))
            .arg(Arg::with_name("height")
                .short("h")
                .long("height")
                .value_name("PIXELS")
                .help("Canvas height (def: screen height)")
                .display_order(3)
                .takes_value(true))
            .arg(Arg::with_name("stats-screen")
                .long("stats-screen")
                .value_name("SECONDS")
                .help("Reporting interval of stats on screen")
                .default_value("1")
                .takes_value(true))
            .arg(Arg::with_name("stats-stdout")
                .long("stats-stdout")
                .alias("stats-console")
                .alias("stats-terminal")
                .value_name("SECONDS")
                .help("Reporting interval of stats to stdout")
                .default_value("5")
                .takes_value(true))
            .get_matches();

        // Instantiate
        ArgHandler {
            matches,
        }
    }

    /// Get the host to bind to.
    pub fn host(&'a self) -> SocketAddr {
        // Get the raw host
        let host = self.matches.value_of("bind")
            .expect("please specify a host to bind to");

        // Parse the host, and return
        host.parse()
            .expect("invalid host given")
    }

    /// Check whether to render a pixelflut window.
    pub fn no_render(&self) -> bool {
        self.matches.is_present("no-render")
    }

    /// Get the canvas size.
    pub fn size(&self) -> (usize, usize) {
        // TODO: use the current screen size as default here
        (
            self.matches.value_of("width")
                .map(|width| width.parse()
                    .expect("Invalid image width")
                )
                .unwrap_or(800),
            self.matches.value_of("height")
                .map(|height| height.parse()
                    .expect("Invalid image height")
                )
                .unwrap_or(600),
        )
    }

    /// The interval of stats reporting on the screen.
    ///
    /// If no stats should be reported, `None` is returned.
    pub fn stats_screen_interval(&self) -> Option<Duration> {
        self.matches.value_of("stats-screen")
            .map(|raw| raw.parse::<f64>()
                .map(|sec: f64| (sec * 1000f64) as u64)
                .expect("invalid screen stats update interval, must be number of seconds")
            )
            .map(|millis| if millis > 0 {
                Some(Duration::from_millis(millis))
            } else if millis == 0 {
                None
            } else {
                panic!("invalid screen stats update interval, must be 0 or >1ms");
            })
            .unwrap()
    }

    /// The interval of stats reporting to stdout.
    ///
    /// If no stats should be reported, `None` is returned.
    pub fn stats_stdout_interval(&self) -> Option<Duration> {
        self.matches.value_of("stats-stdout")
            .map(|raw| raw.parse::<f64>()
                .map(|sec: f64| (sec * 1000f64) as u64)
                .expect("invalid stdout stats update interval, must be number of seconds")
            )
            .map(|millis| if millis > 0 {
                Some(Duration::from_millis(millis))
            } else if millis == 0 {
                None
            } else {
                panic!("invalid stdout stats update interval, must be 0 or >1ms");
            })
            .unwrap()
    }
}
