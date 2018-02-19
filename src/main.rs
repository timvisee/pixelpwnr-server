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

use futures::prelude::*;
use futures::future::{Either, Executor};
use futures::sync::mpsc;
use futures_cpupool::CpuPool;
use tokio_io::AsyncRead;
use tokio_io::io::copy;
use tokio::net::{TcpStream, TcpListener};

use std::env;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use bufstream::BufStream;
use bytes::{BufMut, BytesMut};
use pixelpwnr_render::Color;
use pixelpwnr_render::Pixmap;
use pixelpwnr_render::Renderer;

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

    // // Render the pixelflut screen
    render(&pixmap);
}

fn worker(rx: mpsc::UnboundedReceiver<TcpStream>, pixmap: Arc<Pixmap>) {
    let pool = CpuPool::new(1);

    let done = rx.for_each(move |socket| {
        // A client connected, ensure we're able to get it's address
        let addr = socket.peer_addr().expect("failed to get remote address");
        println!("A client connected");

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

        // // Like the single-threaded `echo` example we split the socket halves
        // // and use the `copy` helper to ship bytes back and forth. Afterwards we
        // // spawn the task to run concurrently on this thread, and then print out
        // // what happened afterwards
        // let (reader, writer) = socket.split();
        // let amt = copy(reader, writer);
        // let msg = amt.then(move |result| {
        //     match result {
        //         Ok((amt, _, _)) => println!("wrote {} bytes to {}", amt, addr),
        //         Err(e) => println!("error on {}: {}", addr, e),
        //     }

        //     Ok(())
        // });
        // pool.execute(msg).unwrap();

        Ok(())
    });

    // Handle all connection futures, and wait until we're done
    done.wait().unwrap();
}

/// Line based codec
///
/// This decorates a socket and presents a line based read / write interface.
///
/// As a user of `Lines`, we can focus on working at the line level. So, we send
/// and receive values that represent entire lines. The `Lines` codec will
/// handle the encoding and decoding as well as reading from and writing to the
/// socket.
#[derive(Debug)]
struct Lines {
    /// The TCP socket.
    socket: TcpStream,

    /// Buffer used when reading from the socket. Data is not returned from this
    /// buffer until an entire line has been read.
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
        self.wr.put(line);
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
                // Append the peer's name to the front of the line:
                // TODO: use a better conversion method here
                let mut line = message;

                // We're using `Bytes`, which allows zero-copy clones (by
                // storing the data in an Arc internally).
                //
                // However, before cloning, we must freeze the data. This
                // converts it from mutable -> immutable, allowing zero copy
                // cloning.
                let line = line.freeze();



                // Split the input data, and get the first split
                // TODO: trim?
                // TODO: don't convert to a string here
                let line = String::from_utf8(line.to_vec()).expect("failed to encode input as string");
                let mut splits = line
                    .split(" ")
                    .filter(|x| !x.is_empty());
                let cmd = match splits.next() {
                    Some(c) => c,
                    None => continue,
                };

                // Process the command
                // TODO: improve response handling
                match process_command(cmd, splits, &self.pixmap) {
                    CmdResponse::Ok => {},
                    CmdResponse::Response(msg) => {
                        // TODO: write back
                        // write!(reader, "{}", msg).expect("failed to write response");
                        // reader.flush().expect("failed to flush stream");
                        self.lines.buffer(msg.as_bytes());
                    },
                    CmdResponse::ClientErr(err) => {
                        // TODO: write back
                        // write!(reader, "ERR {}", err).expect("failed to write error");
                        // reader.flush().expect("failed to flush stream");
                    },
                    // CmdResponse::InternalErr(err) => {
                    //     println!("Error: \"{}\". Closing connection...", err);
                    //     return;
                    // },
                }



                // TODO: process the line as command

                // // Now, send the line to all other peers
                // for (addr, tx) in &self.state.borrow().peers {
                //     // Don't send the message to ourselves
                //     if *addr != self.addr {
                //         // The send only fails if the rx half has been dropped,
                //         // however this is impossible as the `tx` half will be
                //         // removed from the map before the `rx` is dropped.
                //         tx.unbounded_send(line.clone()).unwrap();
                //     }
                // }
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








// /// Handle a client connection.
// fn handle_client(stream: TcpStream, pixmap: Arc<Pixmap>) {
//     // Create a buffered reader
//     let mut reader = BufStream::new(stream);

//     // A client has connected
//     println!("A client has connected");

//     // Read client input
//     loop {
//         // Read a new line
//         let mut data = String::new();
//         if let Err(_) = reader.read_line(&mut data) {
//             println!("An error occurred, closing stream");
//             return;
//         }

//         // Split the input data, and get the first split
//         let mut splits = data.trim()
//             .split(" ")
//             .filter(|x| !x.is_empty());
//         let cmd = match splits.next() {
//             Some(c) => c,
//             None => continue,
//         };

//         // Process the command
//         // TODO: improve response handling
//         match process_command(cmd, splits, &pixmap) {
//             CmdResponse::Ok => {},
//             CmdResponse::Response(msg) => {
//                 write!(reader, "{}", msg).expect("failed to write response");
//                 reader.flush().expect("failed to flush stream");
//             },
//             CmdResponse::ClientErr(err) => {
//                 write!(reader, "ERR {}", err).expect("failed to write error");
//                 reader.flush().expect("failed to flush stream");
//             },
//             // CmdResponse::InternalErr(err) => {
//             //     println!("Error: \"{}\". Closing connection...", err);
//             //     return;
//             // },
//         }
//     }
// }

enum CmdResponse<'a> {
    Ok,
    Response(String),
    ClientErr(&'a str),
    // InternalErr(&'a str),
}

fn process_command<'a, I: Iterator<Item=&'a str>>(
    cmd: &str,
    mut data: I,
    pixmap: &Pixmap
) -> CmdResponse<'a> {
    match cmd {
        "PX" => {
            // Get and parse pixel data, and set the pixel
            match data.next()
                .ok_or("missing x coordinate")
                .and_then(|x| x.parse()
                    .map_err(|_| "invalid x coordinate")
                )
                .and_then(|x|
                    data.next()
                        .ok_or("missing y coordinate")
                        .and_then(|y| y.parse()
                            .map_err(|_| "invalid y coordinate")
                        )
                        .map(|y| (x, y))
                )
                .and_then(|(x, y)|
                    data.next()
                        .ok_or("missing color value")
                        .and_then(|color| Color::from_hex(color)
                            .map_err(|_| "invalid color value")
                        )
                        .map(|color| (x, y, color))
                )
            {
                Ok((x, y, color)) => {
                    // Set the pixel
                    pixmap.set_pixel(x, y, color);
                    CmdResponse::Ok
                },
                Err(msg) =>
                    // An error occurred, respond with it
                    CmdResponse::ClientErr(msg),
            }
        },
        "SIZE" => {
            // Get the screen dimentions
            let (width, height) = pixmap.dimentions();

            // Respond
            CmdResponse::Response(
                format!("SIZE {} {}\n", width, height),
            )
        },
        _ => CmdResponse::ClientErr("unknown command"),
    }
}

fn render(pixmap: &Pixmap) {
    // Build and run the renderer
    let mut renderer = Renderer::new(app::APP_NAME, pixmap);
    renderer.run();
}
