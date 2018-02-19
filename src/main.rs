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
mod client;
mod cmd;
mod codec;

use std::env;
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
use client::Client;
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

/// Start the pixel map renderer.
fn render(pixmap: &Pixmap) {
    // Build and run the renderer
    let mut renderer = Renderer::new(APP_NAME, pixmap);
    renderer.run();
}
