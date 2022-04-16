use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use bytes::{BufMut, BytesMut};
use futures::Stream;
use pixelpwnr_render::Pixmap;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::cmd::{Cmd, CmdResult};
use crate::codec::Lines;
use crate::stats::Stats;

/// The state for each connected client.
pub struct Client<'a, T>
where
    T: AsyncRead + AsyncWrite,
{
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    lines: Pin<&'a mut Lines<'a, T>>,

    /// A pixel map.
    pixmap: Arc<Pixmap>,

    /// A stats manager.
    stats: Arc<Stats>,
}

impl<'a, T> Client<'a, T>
where
    T: AsyncRead + AsyncWrite,
{
    /// Create a new instance of `Client`.
    pub fn new(lines: Pin<&'a mut Lines<'a, T>>, pixmap: Arc<Pixmap>, stats: Arc<Stats>) -> Self {
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
    pub fn respond(
        self: &mut Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        response: &[u8],
    ) -> Result<(), ()> {
        // Write to the buffer
        self.lines.buffer(response);
        self.lines.buffer(b"\r\n");

        // Flush the write buffer to the socket
        // TODO: don't wait on this, flush in the background?
        if let Poll::Pending = self.lines.as_mut().poll_flush(cx) {
            return Err(());
        }

        Ok(())
    }

    /// Respond to the client with the given response as a string.
    ///
    /// A new line is automatically appended to the response.
    ///
    /// This blocks until the written data is flushed to the client.
    pub fn respond_str(
        self: &mut Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        response: String,
    ) -> Result<(), ()> {
        self.respond(cx, response.as_bytes())
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
impl<'a, T> Future for Client<'a, T>
where
    T: AsyncRead + AsyncWrite,
{
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<String> {
        // Read new lines from the socket
        while let Poll::Ready(line) = self.lines.as_mut().poll_next(cx) {
            if let Some(message) = line {
                // Get the input we're working with
                let input = message.freeze();

                // Decode the command to run
                let cmd = match Cmd::decode(input) {
                    Err(err) => {
                        // Report the error to the client
                        self.respond_str(cx, format!("ERR {}", err))
                            .expect("failed to flush write buffer");

                        return Poll::Ready("Command decoding failed".to_string());
                    }
                    Ok(cmd) => cmd,
                };

                // Invoke the command, and catch the result
                let result = cmd.invoke(&self.pixmap, &self.stats);

                // Do something with the result
                match result {
                    // Do nothing
                    CmdResult::Ok => {}

                    // Respond to the client
                    CmdResult::Response(msg) => {
                        // Create a bytes buffer with the message
                        let mut bytes = BytesMut::with_capacity(msg.len());
                        bytes.put_slice(msg.as_bytes());

                        // Respond
                        self.respond(cx, &bytes)
                            .expect("failed to flush write buffer");
                    }

                    // Report the error to the user
                    CmdResult::ClientErr(err) => {
                        // Report the error to the client
                        self.respond_str(cx, format!("ERR {}", err))
                            .expect("failed to flush write buffer");

                        return Poll::Ready(format!("Client error: {}", err));
                    }

                    // Report the error to the server
                    CmdResult::ServerErr(err) => {
                        // Show an error message in the console
                        println!("Client error \"{}\" occurred, disconnecting...", err);

                        // Disconnect the client
                        return Poll::Ready(format!("Server error occured. {}", err));
                    }

                    // Quit the connection
                    CmdResult::Quit => return Poll::Ready("Client quit".to_string()),
                }
            } else {
                // EOF was reached. The remote client has disconnected. There is
                // nothing more to do.
                return Poll::Ready("Eof was reached".to_string());
            }
        }

        // As always, it is important to not just return `NotReady` without
        // ensuring an inner future also returned `NotReady`.
        //
        // We know we got a `NotReady` from either `self.rx` or `self.lines`, so
        // the contract is respected.
        Poll::Pending
    }
}
