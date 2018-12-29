use std::io;
use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use futures::prelude::*;
use pixelpwnr_render::Pixmap;

use cmd::{Cmd, CmdResult};
use codec::Lines;
use stats::Stats;

/// The state for each connected client.
pub struct Client {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    lines: Lines,

    /// A pixel map.
    pixmap: Arc<Pixmap>,

    /// A stats manager.
    stats: Arc<Stats>,
}

impl Client {
    /// Create a new instance of `Client`.
    pub fn new(lines: Lines, pixmap: Arc<Pixmap>, stats: Arc<Stats>) -> Client {
        Client {
            lines,
            pixmap,
            stats,
        }
    }

    /// Respond to the client with the given response.
    ///
    /// A new line is automatically appended to the response.
    ///
    /// This blocks until the written data is flushed to the client.
    pub fn respond(&mut self, response: &[u8]) -> Result<(), io::Error> {
        // Write to the buffer
        self.lines.buffer(response);
        self.lines.buffer(b"\r\n");

        // Flush the write buffer to the socket
        // TODO: don't wait on this, flush in the background?
        self.lines.poll_flush()?;

        Ok(())
    }

    /// Respond to the client with the given response as a string.
    ///
    /// A new line is automatically appended to the response.
    ///
    /// This blocks until the written data is flushed to the client.
    pub fn respond_str(&mut self, response: String) -> Result<(), io::Error> {
        self.respond(response.as_bytes())
    }
}

/// This is where a connected client is managed.
///
/// A `Client` is also a future representing completly processing the client.
///
/// When a `Client` is created, the first line (representing the client's name)
/// has already been read. When the socket closes, the `Client` future completes.
///
/// While processing, the client future implementation will:
///
/// 1) Receive messages on its message channel and write them to the socket.
/// 2) Receive messages from the socket and broadcast them to all clients.
///
impl Future for Client {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        // Read new lines from the socket
        while let Async::Ready(line) = self.lines.poll()? {
            if let Some(message) = line {
                // Get the input we're working with
                let input = message.freeze();

                // Decode the command to run
                let cmd = match Cmd::decode(&input) {
                    Err(err) => {
                        // Report the error to the client
                        self.respond_str(format!("ERR {}", err))
                            .expect("failed to flush write buffer");

                        continue;
                    },
                    Ok(cmd) => cmd,
                };

                // Invoke the command, and catch the result
                let result = cmd.invoke(&self.pixmap, &self.stats);

                // Do something with the result
                match result {
                    // Do nothing
                    CmdResult::Ok => {},

                    // Respond to the client
                    CmdResult::Response(msg) => {
                        // Create a bytes buffer with the message
                        let mut bytes = BytesMut::with_capacity(msg.len());
                        bytes.put_slice(msg.as_bytes());

                        // Respond
                        self.respond(&bytes)
                            .expect("failed to flush write buffer");
                    },

                    // Report the error to the user
                    CmdResult::ClientErr(err) => {
                        // Report the error to the client
                        self.respond_str(format!("ERR {}", err))
                            .expect("failed to flush write buffer");

                        // TODO: disconnect the client after sending
                    },

                    // Report the error to the server
                    CmdResult::ServerErr(err) => {
                        // Show an error message in the console
                        println!("Client error \"{}\" occurred, disconnecting...", err);

                        // Disconnect the client
                        return Ok(Async::Ready(()));
                    },

                    // Quit the connection
                    CmdResult::Quit => return Ok(Async::Ready(())),
                }
            } else {
                // EOF was reached. The remote client has disconnected. There is
                // nothing more to do.
                return Ok(Async::Ready(()));
            }
        }

        // As always, it is important to not just return `NotReady` without
        // ensuring an inner future also returned `NotReady`.
        //
        // We know we got a `NotReady` from either `self.rx` or `self.lines`, so
        // the contract is respected.
        Ok(Async::NotReady)
    }
}
