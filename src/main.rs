#![feature(integer_atomics)]

extern crate atoi;
extern crate bufstream;
extern crate bytes;
extern crate clap;
#[macro_use]
extern crate futures;
extern crate futures_cpupool;
extern crate pixelpwnr_render;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate tokio;
#[macro_use]
extern crate tokio_io;

mod app;
mod arg_handler;
mod client;
mod cmd;
mod codec;
mod stat_monitor;
mod stat_reporter;
mod stats;

use std::sync::Arc;
use std::thread;

use futures::future::Executor;
use futures::prelude::*;
use futures::sync::mpsc;
use futures_cpupool::Builder;
use pixelpwnr_render::{Pixmap, Renderer};
use tokio::net::{TcpListener, TcpStream};

use app::APP_NAME;
use arg_handler::ArgHandler;
use client::Client;
use codec::Lines;
use stat_reporter::StatReporter;
use stats::{Stats, StatsRaw};

// TODO: use some constant for new lines

/// Main application entrypoint.
fn main() {
    // Parse CLI arguments
    let arg_handler = ArgHandler::parse();

    // Build the pixelmap size
    let size = arg_handler.size();
    let pixmap = Arc::new(Pixmap::new(size.0, size.1));
    println!("Canvas size: {}x{}", size.0, size.1);

    // Build a stats manager, load persistent stats
    let mut stats = Stats::new();
    if let Some(path) = arg_handler.stats_file() {
        if let Some(raw) = StatsRaw::load(path.as_path()) {
            stats.from_raw(&raw);
        }
    }
    let stats = Arc::new(stats);

    // Start a server listener in a new thread
    let pixmap_thread = pixmap.clone();
    let stats_thread = stats.clone();
    let host = arg_handler.host();
    let server_thread = thread::spawn(move || {
        let listener = TcpListener::bind(&host).expect("failed to bind");
        println!("Listening on: {}", host);

        // Create a worker thread that assigns work to a futures threadpool
        let (tx, rx) = mpsc::unbounded();
        let pixmap_worker = pixmap_thread.clone();
        let stats_worker = stats_thread.clone();
        thread::spawn(|| worker(rx, pixmap_worker, stats_worker));

        // Infinitely accept sockets from our `TcpListener`.
        // Send work to the worker
        let srv = listener.incoming().for_each(|socket| {
            tx.unbounded_send(socket).expect("worker thread died");
            Ok(())
        });

        srv.wait().unwrap();
    });

    // Render the pixelflut screen
    if !arg_handler.no_render() {
        render(&arg_handler, &pixmap, stats);
    } else {
        // Do not render, wait on the server thread instead
        println!("Not rendering canvas, disabled with the --no-render flag");
        server_thread.join().unwrap();
    }
}

fn worker(rx: mpsc::UnboundedReceiver<TcpStream>, pixmap: Arc<Pixmap>, stats: Arc<Stats>) {
    let pool = Builder::new()
        .name_prefix(format!("{}-worker", APP_NAME))
        .create();

    let done = rx.for_each(move |socket| {
        // A client connected, ensure we're able to get it's address
        let addr = socket.peer_addr().expect("failed to get remote address");
        println!("A client connected (from: {})", addr);

        // Increase the number of clients
        stats.inc_clients();

        // Wrap the socket with the Lines codec,
        // to interact with lines instead of raw bytes
        let lines = Lines::new(socket, stats.clone());

        // Define a client as connection
        let disconnect_stats = stats.clone();
        let connection = Client::new(lines, pixmap.clone(), stats.clone())
            .map_err(|e| {
                // Handle connection errors, show an error message
                println!("Client connection error: {:?}", e);
            })
            .then(move |_| -> Result<_, _> {
                // Print a disconnect message
                println!("A client disconnected (from: {})", addr);

                // Decreasde the client connections number
                disconnect_stats.dec_clients();

                Ok(())
            });

        // Add the connection future to the pool on this thread
        pool.execute(connection).unwrap();

        Ok(())
    });

    // Handle all connection futures, and wait until we're done
    done.wait().unwrap();
}

/// Start the pixel map renderer.
fn render(arg_handler: &ArgHandler, pixmap: &Pixmap, stats: Arc<Stats>) {
    // Build the renderer
    let mut renderer = Renderer::new(APP_NAME, pixmap);

    // Borrow the statistics text
    let stats_text = renderer.stats().text();

    // Create a stats reporter, and start reporting
    let reporter = StatReporter::new(
        arg_handler.stats_screen_interval(),
        arg_handler.stats_stdout_interval(),
        arg_handler.stats_save_interval(),
        arg_handler.stats_file(),
        stats,
        Some(stats_text),
    );
    reporter.start();

    // Render the canvas
    renderer.run(
        arg_handler.fullscreen(),
        arg_handler.stats_font_size(),
        arg_handler.stats_offset(),
        arg_handler.stats_padding(),
        arg_handler.stats_column_spacing(),
    );
}
