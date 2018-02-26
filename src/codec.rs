use std::io;
use std::io::prelude::*;

use bytes::BytesMut;
use futures::prelude::*;
use tokio::net::TcpStream;
use tokio_io::AsyncRead;

/// Line based codec.
///
/// This decorates a socket and presents a line based read / write interface.
///
/// As a user of `Lines`, we can focus on working at the line level. So, we
/// send and receive values that represent entire lines. The `Lines` codec will
/// handle the encoding and decoding as well as reading from and writing to the
/// socket.
#[derive(Debug)]
pub struct Lines {
    /// The TCP socket.
    socket: TcpStream,

    /// Buffer used when reading from the socket. Data is not returned from
    /// this buffer until an entire line has been read.
    rd: BytesMut,

    /// Buffer used to stage data before writing it to the socket.
    wr: BytesMut,
}

impl Lines {
    /// Create a new `Lines` codec backed by the socket
    pub fn new(socket: TcpStream) -> Self {
        Lines {
            socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }

    /// Buffer a line.
    ///
    /// This writes the line to an internal buffer. Calls to `poll_flush` will
    /// attempt to flush this buffer to the socket.
    pub fn buffer(&mut self, line: &[u8]) {
        // Push the line onto the end of the write buffer.
        //
        // The `put` function is from the `BufMut` trait.
        self.wr.extend_from_slice(line);
    }

    /// Flush the write buffer to the socket
    pub fn poll_flush(&mut self) -> Poll<(), io::Error> {
        // As long as there is buffered data to write, try to write it.
        while !self.wr.is_empty() {
            // `try_nb` is kind of like `try_ready`, but for operations that
            // return `io::Result` instead of `Async`.
            //
            // In the case of `io::Result`, an error of `WouldBlock` is
            // equivalent to `Async::NotReady.
            let n = try_nb!(self.socket.write(&self.wr));

            // As long as the wr is not empty, a successful write should
            // never write 0 bytes.
            assert!(n > 0);

            // This discards the first `n` bytes of the buffer.
            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }

    /// Read data from the socket.
    ///
    /// This only returns `Ready` when the socket has closed.
    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        loop {
            // Ensure the read buffer has capacity.
            //
            // This might result in an internal allocation.
            self.rd.reserve(1024);

            // Read data into the buffer.
            let n = try_ready!(self.socket.read_buf(&mut self.rd));

            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl Stream for Lines {
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // Keep trying to read until a line is read, or the connection closed
        loop {
            // First, read any new data into the read buffer
            let closed = self.fill_read_buf()?.is_ready();

            // Try finding lines
            // TODO: find any variation of new lines?
            let pos = self.rd
                .windows(2)
                .position(|bytes| bytes == b"\r\n");

            // Get the line, return it
            if let Some(pos) = pos {
                // Pull the line of the read buffer
                let mut line = self.rd.split_to(pos + 2);

                // Skip empty lines
                if pos == 0 {
                    continue;
                }

                // Drop trailing new line characters
                line.split_off(pos);

                // Return the line
                return Ok(Async::Ready(Some(line)));
            }

            // We don't have new data, or close the connection
            if closed {
                return Ok(Async::Ready(None));
            } else {
                return Ok(Async::NotReady);
            }
        }
    }
}
