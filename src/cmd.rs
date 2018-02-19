use std::io::Write;

use atoi::atoi;
use bytes::{BufMut, Bytes, BytesMut};
use pixelpwnr_render::{Color, Pixmap};

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
    /// Parse the command to run, from the given input bytes.
    pub fn parse<'a>(input: Bytes) -> Result<Self, &'a str> {
        // Iterate over input parts
        let mut input = input
            .split(|b| b == &b' ')
            .filter(|part| !part.is_empty());

        // Parse the command
        match input.next() {
            Some(cmd) => match cmd {
                // Pixel command
                b"PX" => {
                    // Get and parse coordinates
                    let (x, y) = (
                        atoi(
                            input.next().ok_or("missing x coordinate")?
                        ).ok_or("invalid x coordinate")?,
                        atoi(
                            input.next().ok_or("missing y coordinate")?
                        ).ok_or("invalid y coordinate")?,
                    );

                    // Get the color part, determine whether this is a get/set
                    // command
                    match input.next() {
                        // Color part found, set the pixel command
                        // TODO: don't convert to a string here
                        Some(color) => Ok(Cmd::SetPixel(
                            x,
                            y,
                            Color::from_hex(&String::from_utf8_lossy(color))
                                .map_err(|_| "invalid color value")?,
                        )),

                        // No color part found, get the pixel color
                        None => Ok(Cmd::GetPixel(x, y))
                    }
                },

                // Basic commands
                b"SIZE" => Ok(Cmd::Size),
                b"HELP" => Ok(Cmd::Help),
                b"QUIT" => Ok(Cmd::Quit),

                // Unknown command
                _ => Err("unknown command, use HELP"),
            },

            // If no command was specified, do nothing
            None => Ok(Cmd::None),
        }
    }

    /// Invoke the command, and return the result.
    pub fn invoke<'a>(self, pixmap: &Pixmap) -> CmdResult<'a> {
        // Match the command, invoke the proper action
        match self {
            // Set the pixel on the pixel map
            Cmd::SetPixel(x, y, color) => pixmap.set_pixel(x, y, color),

            // Get a pixel color from the pixel map
            Cmd::GetPixel(x, y) => {
                // Get the hexadecimal color value
                let color = pixmap.pixel(x, y).hex();

                // Build the response
                let mut response = BytesMut::new().writer();
                if write!(response, "PX {} {} {}", x, y, color).is_err() {
                    return CmdResult::ServerErr("failed to write response to buffer");
                }

                // Send the response
                return CmdResult::Response(
                    response.into_inner().freeze(),
                );
            },

            // Get the size of the screen
            Cmd::Size => {
                // Get the size
                let (x, y) = pixmap.dimentions();

                // Build the response
                let mut response = BytesMut::new().writer();
                if write!(response, "SIZE {} {}", x, y).is_err() {
                    return CmdResult::ServerErr("failed to write response to buffer");
                }

                // Send the response
                return CmdResult::Response(
                    response.into_inner().freeze(),
                );
            },

            // Show help
            Cmd::Help => return CmdResult::Response(Self::help_list()),

            // Quit the connection
            Cmd::Quit => return CmdResult::Quit,

            // Do nothing
            Cmd::None => {},
        }

        // Everything went right
        CmdResult::Ok
    }

    /// Get a list of command help, to respond to a client.
    pub fn help_list() -> Bytes {
        // Create a bytes buffer
        let mut help = BytesMut::new();

        // Append the commands
        help.extend_from_slice(b"\
            HELP Commands:\r\n\
            HELP - PX <x> <y> <RRGGBB[AA]>\r\n\
            HELP - PX <x> <y>   >>  PX <x> <y> <RRGGBB>\r\n\
            HELP - SIZE         >>  SIZE <width> <height>\r\n\
            HELP - HELP         >>  HELP ...\r\n\
            HELP - QUIT\
        ");

        // Freeze the bytes, and return
        help.freeze()
    }
}

/// A result, returned when invoking a command.
///
/// This result defines the status of the command that was invoked.
/// Some response might need to be send to the client,
/// or an error might have occurred.
pub enum CmdResult<'a> {
    /// The command has been invoked successfully.
    Ok,

    /// The command has been invoked successfully, and the following response
    /// should be send to the client.
    Response(Bytes),

    /// The following error occurred while invoking a command, based on the
    /// clients input.
    ClientErr(&'a str),

    /// The following error occurred while invoking a command on the server.
    ServerErr(&'a str),

    /// The connection should be closed.
    Quit,
}