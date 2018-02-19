extern crate atoi;
extern crate bufstream;
extern crate bytes;
#[macro_use]
extern crate futures;
extern crate futures_cpupool;
extern crate num_cpus;
extern crate pixelpwnr_render;
extern crate tokio;
#[macro_use]
extern crate tokio_io;

mod app;

use std::env;
use std::io;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use atoi::atoi;
use futures::prelude::*;
use futures::future::Executor;
use futures::sync::mpsc;
use futures_cpupool::CpuPool;
use tokio_io::AsyncRead;
use tokio::net::{TcpStream, TcpListener};

use bytes::{BufMut, Bytes, BytesMut};
use pixelpwnr_render::{Color, Pixmap, Renderer};

// TODO: use some constant for new lines

/// Main application entrypoint.
fn main() {
    // Build a pixelmap
    let pixmap = Arc::new(Pixmap::new(800, 600));

    // Start a server listener in a new thread
    let pixmap_thread = pixmap.clone();
    thread::spawn(move || {
        // First argument, the address to bind
        let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
        let addr = addr.parse::<SocketAddr>().unwrap();

        // Second argument, the number of threads we'll be using
        let num_threads = env::args().nth(2).and_then(|s| s.parse().ok())
            .unwrap_or(num_cpus::get());

        let listener = TcpListener::bind(&addr).expect("failed to bind");
        println!("Listening on: {}", addr);

        // Spin up our worker threads, creating a channel routing to each worker
        // thread that we'll use below.
        let mut channels = Vec::new();
        for _ in 0..num_threads {
            let (tx, rx) = mpsc::unbounded();
            channels.push(tx);
            let pixmap_worker = pixmap_thread.clone();
            thread::spawn(|| worker(rx, pixmap_worker));
        }

        // Infinitely accept sockets from our `TcpListener`. Each socket is then
        // shipped round-robin to a particular thread which will associate the
        // socket with the corresponding event loop and process the connection.
        let mut next = 0;
        let srv = listener.incoming().for_each(|socket| {
            channels[next].unbounded_send(socket).expect("worker thread died");
            next = (next + 1) % channels.len();
            Ok(())
        });

        srv.wait().unwrap();
    });

    // Render the pixelflut screen
    render(&pixmap);
}

fn worker(rx: mpsc::UnboundedReceiver<TcpStream>, pixmap: Arc<Pixmap>) {
    // TODO: Define a better pool size
    let pool = CpuPool::new(1);

    let done = rx.for_each(move |socket| {
        // A client connected, ensure we're able to get it's address
        let addr = socket.peer_addr().expect("failed to get remote address");
        println!("A client connected from {}", addr);

        // Wrap the socket with the Lines codec,
        // to interact with lines instead of raw bytes
        let lines = Lines::new(socket);

        // Define a peer as connection
        let connection = Peer::new(lines, pixmap.clone())
            .map_err(|e| {
                println!("connection error = {:?}", e);
            });

        // Add the connection future to the pool on this thread
        pool.execute(connection).unwrap();

        Ok(())
    });

    // Handle all connection futures, and wait until we're done
    done.wait().unwrap();
}

/// Line based codec
///
/// This decorates a socket and presents a line based read / write interface.
///
/// As a user of `Lines`, we can focus on working at the line level. So, we
/// send and receive values that represent entire lines. The `Lines` codec will
/// handle the encoding and decoding as well as reading from and writing to the
/// socket.
#[derive(Debug)]
struct Lines {
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
    fn new(socket: TcpStream) -> Self {
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
    fn buffer(&mut self, line: &[u8]) {
        // Push the line onto the end of the write buffer.
        //
        // The `put` function is from the `BufMut` trait.
        self.wr.extend_from_slice(line);
    }

    /// Flush the write buffer to the socket
    fn poll_flush(&mut self) -> Poll<(), io::Error> {
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
        // First, read any new data that might have been received off the socket
        let sock_closed = self.fill_read_buf()?.is_ready();

        // Now, try finding lines
        let pos = self.rd
            .windows(2)
            .position(|bytes| bytes == b"\r\n");

        if let Some(pos) = pos {
            // Remove the line from the read buffer and set it to `line`.
            let mut line = self.rd.split_to(pos + 2);

            // Drop the trailing \r\n
            line.split_off(pos);

            // Return the line
            return Ok(Async::Ready(Some(line)));
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

/// The state for each connected client.
struct Peer {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    lines: Lines,

    /// A pixel map.
    pixmap: Arc<Pixmap>,
}

impl Peer {
    /// Create a new instance of `Peer`.
    fn new(lines: Lines, pixmap: Arc<Pixmap>) -> Peer {
        Peer {
            lines,
            pixmap,
        }
    }

    /// Respond to the client with the given response.
    ///
    /// A new line is automatically appended to the response.
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
    pub fn respond_str(&mut self, response: String) -> Result<(), io::Error> {
        self.respond(response.as_bytes())
    }
}

/// This is where a connected client is managed.
///
/// A `Peer` is also a future representing completly processing the client.
///
/// When a `Peer` is created, the first line (representing the client's name)
/// has already been read. When the socket closes, the `Peer` future completes.
///
/// While processing, the peer future implementation will:
///
/// 1) Receive messages on its message channel and write them to the socket.
/// 2) Receive messages from the socket and broadcast them to all peers.
///
impl Future for Peer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        // Flush the write buffer to the socket
        let _ = self.lines.poll_flush()?;

        // Read new lines from the socket
        while let Async::Ready(line) = self.lines.poll()? {
            if let Some(message) = line {
                // Get the input we're working with
                let input = message.freeze();

                // Parse the command to run
                let cmd = Cmd::parse(input);
                if let Err(err) = cmd {
                    // Report the error to the client
                    self.respond_str(format!("ERR {}", err))
                        .expect("failed to flush write buffer");

                    // TODO: disconnect the client
                    continue;
                }
                let cmd = cmd.unwrap();

                // Invoke the command, and catch the result
                let result = cmd.invoke(&self.pixmap);

                // Do something with the result
                match result {
                    // Do nothing
                    CmdResult::Ok => {},

                    // Respond to the client
                    CmdResult::Response(bytes) =>
                        self.respond(&bytes)
                            .expect("failed to flush write buffer"),

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

/// A set of pixel commands a client might send.
///
/// These commands may then be invoked on the pixel map state.
/// A command might get or set the color of a pixel, or it
/// might request help.
enum Cmd {
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
enum CmdResult<'a> {
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

/// Start the pixel map renderer.
fn render(pixmap: &Pixmap) {
    // Build and run the renderer
    let mut renderer = Renderer::new(app::APP_NAME, pixmap);
    renderer.run();
}
