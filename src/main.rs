mod arg_handler;
mod client;
mod cmd;
mod codec;
mod stat_monitor;
mod stat_reporter;
mod stats;

use std::{pin::Pin, sync::Arc};

use clap::StructOpt;
use pixelpwnr_render::{Pixmap, Renderer};
use tokio::net::{TcpListener, TcpStream};

use client::Client;
use codec::Lines;
use stat_reporter::StatReporter;
use stats::{Stats, StatsRaw};

use crate::arg_handler::Opts;

// TODO: use some constant for new lines

/// Main application entrypoint.
#[tokio::main]
async fn main() {
    // Parse CLI arguments
    let arg_handler = Opts::parse();

    // Build the pixelmap size
    let size = arg_handler.size();
    let pixmap = Arc::new(Pixmap::new(size.0, size.1));
    println!("Canvas size: {}x{}", size.0, size.1);

    // Build a stats manager, load persistent stats
    let mut stats = Stats::new();
    if let Some(path) = arg_handler.stats_file.clone() {
        if let Some(raw) = StatsRaw::load(path.as_path()) {
            stats.from_raw(&raw);
        }
    }
    let stats = Arc::new(stats);

    // Start a server listener in a new thread
    let pixmap_thread = pixmap.clone();
    let stats_thread = stats.clone();
    let host = arg_handler.host;

    let server_thread = tokio::spawn(async move {
        let listener = TcpListener::bind(&host).await.expect("Bind error");
        println!("Listening on: {}", host);

        // Infinitely accept sockets from our `TcpListener`.
        // Send work to the worker

        loop {
            // Create a worker thread that assigns work to a futures threadpool
            let pixmap_worker = pixmap_thread.clone();
            let stats_worker = stats_thread.clone();
            let (socket, _) = if let Ok(res) = listener.accept().await {
                res
            } else {
                println!("Failed to accept a connection");
                continue;
            };
            handle_socket(socket, pixmap_worker, stats_worker);
        }
    });

    // Render the pixelflut screen
    if !arg_handler.no_render {
        render(&arg_handler, pixmap, stats);
    } else {
        // Do not render, wait on the server thread instead
        println!("Not rendering canvas, disabled with the --no-render flag");
        server_thread.await.unwrap();
    }
}

/// Spawn a new task with the given socket
fn handle_socket(mut socket: TcpStream, pixmap: Arc<Pixmap>, stats: Arc<Stats>) {
    // A client connected, ensure we're able to get it's address
    let addr = socket.peer_addr().expect("failed to get remote address");
    println!("A client connected (from: {})", addr);

    // Increase the number of clients
    stats.inc_clients();

    // Wrap the socket with the Lines codec,
    // to interact with lines instead of raw bytes

    // Define a client as connection
    let disconnect_stats = stats.clone();

    let pixmap = pixmap.clone();
    let stats = stats.clone();

    tokio::spawn(async move {
        let socket = Pin::new(&mut socket);

        let mut lines = Lines::new(socket, stats.clone());
        let lines = Pin::new(&mut lines);
        let connection = Client::new(lines, pixmap, stats);

        let result = connection.await;

        // Print a disconnect message
        println!("A client disconnected (from: {}). Reason: {}", addr, result);

        // Decreasde the client connections number
        disconnect_stats.dec_clients();
    });
}

/// Start the pixel map renderer.
fn render(arg_handler: &Opts, pixmap: Arc<Pixmap>, stats: Arc<Stats>) {
    // Build the renderer
    let renderer = Renderer::new(env!("CARGO_PKG_NAME"), pixmap);

    // Borrow the statistics text
    let stats_text = renderer.stats().text();

    // Create a stats reporter, and start reporting
    let reporter = StatReporter::new(
        arg_handler.stats_screen_interval(),
        arg_handler.stats_stdout_interval(),
        arg_handler.stats_save_interval(),
        arg_handler.stats_file.clone(),
        stats,
        Some(stats_text),
    );
    reporter.start();

    // Render the canvas
    renderer.run(
        arg_handler.fullscreen,
        arg_handler.stats_font_size,
        arg_handler.stats_offset(),
        arg_handler.stats_padding,
        arg_handler.stats_col_spacing,
    );
}
