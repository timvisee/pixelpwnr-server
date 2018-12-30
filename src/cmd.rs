use atoi::atoi;
use futures::{
    Future,
    future::ok,
    sink::Sink,
};
use pixelpwnr_render::{Color, Pixmap, PixmapErr};

use app::{APP_NAME, APP_VERSION};
use stats::Stats;

/// Server request result type, returned by the pixel codec when parsing commands.
pub type RequestResult = Result<Request, RequestErr>;

/// A request command sent to the pixelflut server, comming from a client.
///
/// These requests may then be invoked on the pixel map state.
/// A command might get or set the color of a pixel, or it
/// might request help.
#[derive(Debug)]
pub enum Request {
    /// Get the color of a pixel.
    ///
    /// The `x` and `y` coordinate.
    GetPixel(usize, usize),

    /// Set a pixel color.
    ///
    /// The `x` and `y` coordinate, with a `color`.
    SetPixel(usize, usize, Color),

    /// Request the size of the screen.
    Size,

    /// Request help.
    Help,

    /// Quit, break the connection.
    Quit,

    /// Do nothing, just continue.
    /// This is returned when an empty command was received.
    Nop,
}

impl Request {
    /// Decode the command to run, from the given input bytes.
    ///
    /// Decoding is strict and case sensitive.
    /// Surrounding whitespaces are ignored.
    // TODO: benchmark/improve parsing performance, take look into nom
    pub fn decode<'a>(input: &[u8]) -> RequestResult {
        // Iterate over input parts
        let mut input = input
            .split(|b| b == &b' ')
            .filter(|part| !part.is_empty());

        // Decode the command
        match input.next() {
            Some(cmd) => match cmd {
                // Pixel command
                b"PX" => {
                    // Get and parse coordinates
                    let (x, y) = (
                        atoi(
                            input.next().ok_or(RequestErr::InvalidCoordinate)?
                        ).ok_or(RequestErr::InvalidCoordinate)?,
                        atoi(
                            input.next().ok_or(RequestErr::InvalidCoordinate)?
                        ).ok_or(RequestErr::InvalidCoordinate)?,
                    );

                    // Get the color part, determine whether this is a get/set
                    // command
                    match input.next() {
                        // Color part found, set the pixel command
                        // TODO: don't convert to a string here
                        Some(color) => Ok(Request::SetPixel(
                            x,
                            y,
                            Color::from_hex(&String::from_utf8_lossy(color))
                                .map_err(|_| RequestErr::InvalidColor)?,
                        )),

                        // No color part found, get the pixel color
                        None => Ok(Request::GetPixel(x, y))
                    }
                },

                // Basic commands
                b"SIZE" => Ok(Request::Size),
                b"HELP" => Ok(Request::Help),
                b"QUIT" => Ok(Request::Quit),

                // Unknown command
                _ => Err(RequestErr::Unknown),
            },

            // If no command was specified, do nothing
            None => Ok(Request::Nop),
        }
    }

    /// Invoke the request command, and return the result.
    pub fn invoke<'a>(self, pixmap: &'a Pixmap, stats: &Stats) -> ActionResult {
        // Match the command, invoke the proper action
        match self {
            Request::SetPixel(x, y, color) => {
                // Update the pixel statistics
                stats.inc_pixels();

                // Set the pixel
                match pixmap.set_pixel(x, y, color) {
                    Ok(_) => ActionResult::Ok,
                    Err(e) => e.into(),
                }
            },

            Request::GetPixel(x, y) =>
                pixmap.pixel(x, y)
                    .map(|c| ActionResult::Response(Response::Pixel(x, y, c)))
                    .unwrap_or_else(|e| e.into()),

            Request::Size => {
                let (w, h) = pixmap.dimentions();
                ActionResult::Response(Response::Size(w, h))
            },

            Request::Help => ActionResult::Response(Response::Help),
            Request::Quit => ActionResult::Quit,
            Request::Nop => ActionResult::Ok,
        }
    }
}

#[derive(Debug)]
pub enum RequestErr {
    /// An unknown command was given.
    Unknown,

    /// A given coordinate was invalid or out of the screen area.
    InvalidCoordinate,

    /// An invalid color value was given.
    InvalidColor,
}

impl RequestErr {
    pub fn description(&self) -> &'static str {
        match self {
            RequestErr::Unknown => "unknown command, use HELP",
            RequestErr::InvalidCoordinate => "invalid pixel coordinate",
            RequestErr::InvalidColor => "invalid pixel color",
        }
    }
}

/// A repsonse command sent to a client, comming from the pixelflut server.
///
/// Such a response command might describe the help information, or tell the client information
/// about the pixelflut screen.
#[derive(Debug)]
pub enum Response {
    /// Respond with the color of a given pixel.
    ///
    /// The `x` and `y` coordinate, and the pixel color.
    Pixel(usize, usize, Color),

    /// Respond with the current size of the pixelflut screen.
    /// The `width` and `height` of the screen.
    Size(usize, usize),

    /// Respond with help.
    /// The `help` information.
    Help,
}

impl Response {
    /// Transform the response into a string, which may be sent to clients.
    pub fn to_string(&self) -> String {
        match self {
            Response::Pixel(x, y, c) => format!("PX {} {} {}", x, y, c.hex()),
            Response::Size(w, h) => format!("SIZE {} {}", w, h),
            Response::Help => format!("\
                    HELP {} v{}\r\n\
                    HELP Commands:\r\n\
                    HELP - PX <x> <y> <RRGGBB[AA]>\r\n\
                    HELP - PX <x> <y>   >>  PX <x> <y> <RRGGBB>\r\n\
                    HELP - SIZE         >>  SIZE <width> <height>\r\n\
                    HELP - HELP         >>  HELP ...\r\n\
                    HELP - QUIT\
                ", APP_NAME, APP_VERSION),
        }
    }
}

/// A result, returned when invoking a command.
///
/// This result defines the status of the command that was invoked.
/// Some response might need to be send to the client,
/// or an error might have occurred.
pub enum ActionResult {
    /// The action has been invoked successfully.
    Ok,

    /// The command has been invoked successfully, and the following response
    /// should be send to the client.
    Response(Response),

    /// The following error occurred while invoking a command, based on the
    /// clients input.
    // TODO: use an enum as error description type
    ClientErr(String),

    /// The following error occurred while invoking a command on the server.
    // TODO: use an enum as error description type
    ServerErr(String),

    /// The connection should be closed.
    Quit,
}

// impl ActionResult {
//     /// Invoke the request command, and return the result.
//     pub fn invoke<S>(self, socket: &S)
//         -> Box<Future<Item = (), Error = ()>>
//         where S: Sink<SinkItem = Response> + 'static,
//     {
//         // Match the command, invoke the proper action
//         match self {
//             ActionResult::Ok => Box::new(ok(())),
//             ActionResult::Response(r) => {
//                 socket.start_send(r);
//                 let a = socket
//                     .poll_complete()
//                     .unwrap();
//                 Box::new(
//                     a
//                 )
//             },
//             ActionResult::ClientErr(e) => {
//                 eprintln!("TODO: Client err: {}", e);
//                 Box::new(ok(()))
//             },
//             ActionResult::ServerErr(e) => {
//                 eprintln!("TODO: Server err: {}", e);
//                 Box::new(ok(()))
//             },
//             ActionResult::Quit => {
//                 eprintln!("TODO: Quit client!");
//                 Box::new(ok(()))
//             },
//         }
//     }
// }

impl<'a> From<PixmapErr<'a>> for ActionResult {
    fn from(err: PixmapErr<'a>) -> ActionResult {
        match err {
            PixmapErr::OutOfBound(msg) => ActionResult::ClientErr(msg.into()),
        }
    }
}
