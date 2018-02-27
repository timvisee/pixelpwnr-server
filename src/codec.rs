use std::io;
use std::io::prelude::*;

use bytes::BytesMut;
use futures::prelude::*;
use tokio::net::TcpStream;
use tokio_io::AsyncRead;

/// The capacity of the read and write buffer in bytes.
const BUF_SIZE: usize = 64_000;

/// The threshold length on which to fill the buffer again in bytes.
///
/// When this threshold is reached, new memory may be allocated in the buffer
/// to satisfy the preferred buffer capacity.
/// This theshold must be larger than the longest frame we might receive over
/// the network, or else the frame might be incomplete when read.
///
/// Should be less than `BUF_SIZE` to prevent constant socket reads.
const BUF_THRESHOLD: usize = 16_000;

/// The maximum length of a line in bytes.
/// If a received line is longer than than the specified amount of bytes,
/// the search for a newline character (marking the end of a line) will be stalled,
/// and the line stream will end.
/// This is to prevent the stream from blocking, when no complete line could be read from a
/// full buffer.
///
/// This value must be smaller than `BUF_THRESHOLD` to prevent the server from getting
/// stuck as it can't find the end of a line within a full buffer.
const LINE_MAX_LENGTH: usize = 1024;

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
            rd: BytesMut::with_capacity(BUF_SIZE),
            wr: BytesMut::with_capacity(BUF_SIZE),
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

            // This discards the first `n` bytes of the buffer.
            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }

    /// Read data from the socket if the buffer isn't full enough,
    /// and it's length reached the lower size threshold.
    ///
    /// If the size threshold is reached, and there isn't enough data
    /// in the buffer, memory is allocated to give the buffer enough capacity.
    /// The buffer is then filled with data from the socket with all the data
    /// that is available.
    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        // Get the length of buffer contents
        let len = self.rd.len();

        // We've enough data to continue
        if len > BUF_THRESHOLD {
            return Ok(Async::Ready(()));
        }

        // Allocate enough capacity to fill the buffer
        self.rd.reserve(BUF_SIZE - len);

        // Read data and try to fill the buffer
        try_ready!(self.socket.read_buf(&mut self.rd));

        // We're done reading
        return Ok(Async::Ready(()));
    }
}

impl Stream for Lines {
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // First, read any new data into the read buffer
        let closed = self.fill_read_buf()?.is_ready();

        // Make sure the buffer has enough data in it to be a valid command
        if self.rd.len() < 2 {
            return Ok(Async::NotReady);
        }

        // Try finding lines
        // TODO: find any variation of new lines?
        let pos = self.rd
            .windows(2)
            .take(LINE_MAX_LENGTH)
            .position(|bytes| bytes == b"\r\n");

        // Get the line, return it
        if let Some(pos) = pos {
            // Pull the line of the read buffer
            let mut line = self.rd.split_to(pos + 2);

            // Skip empty lines
            if pos == 0 {
                return self.poll();
            }

            // Drop trailing new line characters
            line.split_off(pos);

            // Return the line
            return Ok(Async::Ready(Some(line)));
        }

        // If no line ending was found, and the buffer is larger than the
        // maximum command length, disconnect
        if self.rd.len() > LINE_MAX_LENGTH {
            // TODO: report this error to the client
            println!(
                "Client sent a line longer than {} characters, disconnecting",
                LINE_MAX_LENGTH,
            );

            // Break the connection, by ending the lines stream
            return Ok(Async::Ready(None));
        }

        // We don't have new data, or close the connection
        if closed {
            return Ok(Async::Ready(None));
        } else {
            return Ok(Async::NotReady);
        }
    }
}
