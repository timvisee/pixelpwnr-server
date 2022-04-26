use atoi::atoi;
use bytes::Bytes;
use pixelpwnr_render::{Color, Pixmap, PixmapErr};

use crate::stats::Stats;

/// A set of pixel commands a client might send.
///
/// These commands may then be invoked on the pixel map state.
/// A command might get or set the color of a pixel, or it
/// might request help.
pub enum Cmd {
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
    None,
}

impl Cmd {
    /// Decode the command to run, from the given input bytes.
    pub fn decode_line<'a>(input_bytes: Bytes) -> Result<Self, &'a str> {
        // Iterate over input parts
        let mut input = input_bytes
            .split(|b| b == &b' ')
            .filter(|part| !part.is_empty());

        // Decode the command
        match input.next() {
            Some(cmd) => match cmd {
                // Pixel command
                b"PX" => {
                    // Get and parse coordinates
                    let (x, y) = (
                        atoi(input.next().ok_or("missing x coordinate")?)
                            .ok_or("invalid x coordinate")?,
                        atoi(input.next().ok_or("missing y coordinate")?)
                            .ok_or("invalid y coordinate")?,
                    );

                    // Get the color part, determine whether this is a get/set
                    // command
                    match input.next() {
                        // Color part found, set the pixel command
                        Some(color) => {
                            let color =
                                Color::from_hex_raw(&color).map_err(|_| "invalid color value")?;
                            Ok(Cmd::SetPixel(x, y, color))
                        }

                        // No color part found, get the pixel color
                        None => Ok(Cmd::GetPixel(x, y)),
                    }
                }

                // Basic commands
                b"SIZE" => Ok(Cmd::Size),
                b"HELP" => Ok(Cmd::Help),
                b"QUIT" => Ok(Cmd::Quit),
                b"" => Ok(Cmd::None),
                // Unknown command
                _ => Err("unknown command, use HELP"),
            },

            // If no command was specified, do nothing
            None => Ok(Cmd::None),
        }
    }

    /// Invoke the command, and return the result.
    pub fn invoke<'a>(self, pixmap: &'a Pixmap, stats: &Stats) -> CmdResult {
        // Match the command, invoke the proper action
        match self {
            // Set the pixel on the pixel map
            Cmd::SetPixel(x, y, color) => {
                // Update the pixel statistics
                stats.inc_pixels();

                // Set the pixel
                if let Err(err) = pixmap.set_pixel(x, y, color) {
                    return CmdResult::from_pixmap_err(err);
                }
            }

            // Get a pixel color from the pixel map
            Cmd::GetPixel(x, y) => {
                // Get the hexadecimal color value of a pixel
                let color = match pixmap.pixel(x, y) {
                    Err(err) => return CmdResult::from_pixmap_err(err),
                    Ok(color) => color.hex(),
                };

                // Send the response
                return CmdResult::Response(format!("PX {} {} {}", x, y, color));
            }

            // Get the size of the screen
            Cmd::Size => {
                // Get the size
                let (x, y) = pixmap.dimensions();

                // Send the response
                return CmdResult::Response(format!("SIZE {} {}", x, y));
            }

            // Show help
            Cmd::Help => return CmdResult::Response(Self::help_list()),

            // Quit the connection
            Cmd::Quit => return CmdResult::Quit,

            // Do nothing
            Cmd::None => {}
        }

        // Everything went right
        CmdResult::Ok
    }

    /// Get a list of command help, to respond to a client.
    pub fn help_list() -> String {
        let mut help = format!(
            "\
            HELP {} v{}\r\n\
            HELP Commands:\r\n\
            HELP - PX <x> <y> <RRGGBB[AA]>\r\n\
            HELP - PX <x> <y>   >>  PX <x> <y> <RRGGBB>\r\n\
            HELP - SIZE         >>  SIZE <width> <height>\r\n\
            HELP - HELP         >>  HELP ...\r\n\
            HELP - QUIT\
        ",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );

        if cfg!(feature = "binary-pixel-cmd") {
            help.push_str(
                "\r\nHELP - PBxyrgba (NO newline, x, y = 2 byte LE u16, r, g, b, a = single byte)",
            );
        }

        help
    }
}

/// A result, returned when invoking a command.
///
/// This result defines the status of the command that was invoked.
/// Some response might need to be send to the client,
/// or an error might have occurred.
pub enum CmdResult {
    /// The command has been invoked successfully.
    Ok,

    /// The command has been invoked successfully, and the following response
    /// should be send to the client.
    Response(String),

    /// The following error occurred while invoking a command, based on the
    /// clients input.
    ClientErr(String),

    /// The following error occurred while invoking a command on the server.
    ServerErr(String),

    /// The connection should be closed.
    Quit,
}

impl CmdResult {
    /// Build a command result from the given pixmap error that has occurred
    /// when invoking a command.
    pub fn from_pixmap_err(err: PixmapErr) -> CmdResult {
        match err {
            PixmapErr::OutOfBound(msg) => CmdResult::ClientErr(msg.into()),
        }
    }
}
