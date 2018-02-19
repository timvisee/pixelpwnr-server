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
mod cmd;
mod codec;

use std::env;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use futures::prelude::*;
use futures::future::Executor;
use futures::sync::mpsc;
use futures_cpupool::CpuPool;
use pixelpwnr_render::{Pixmap, Renderer};
use tokio::net::{TcpStream, TcpListener};

use app::APP_NAME;
use cmd::{Cmd, CmdResult};
use codec::Lines;

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

        // Define a client as connection
        let connection = Client::new(lines, pixmap.clone())
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

/// The state for each connected client.
struct Client {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    lines: Lines,

    /// A pixel map.
    pixmap: Arc<Pixmap>,
}

impl Client {
    /// Create a new instance of `Client`.
    fn new(lines: Lines, pixmap: Arc<Pixmap>) -> Client {
        Client {
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

/// Start the pixel map renderer.
fn render(pixmap: &Pixmap) {
    // Build and run the renderer
    let mut renderer = Renderer::new(APP_NAME, pixmap);
    renderer.run();
}
