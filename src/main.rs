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
extern crate tokio_io_pool;

mod app;
mod arg_handler;
// mod client;
mod cmd;
// TODO: remove this module
mod codec;
mod pix_codec;
mod stat_monitor;
mod stat_reporter;
mod stats;

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use futures::Future;
use futures::future::Executor;
use futures::future::ok;
use futures::prelude::*;
use futures::sync::mpsc;
use futures_cpupool::Builder;
use pixelpwnr_render::{Pixmap, Renderer};
use tokio::net::{TcpListener, TcpStream};
use tokio::codec::Framed;

use app::APP_NAME;
use arg_handler::ArgHandler;
// use client::Client;
use cmd::ActionResult;
use cmd::Response;
use codec::Lines;
use pix_codec::PixCodec;
use stat_reporter::StatReporter;
use stats::{Stats, StatsRaw};

// TODO: implement: tk-listen
// TODO: implement: some sort of timeout

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

    // Get the host to use
    let host = arg_handler.host();

    // Start a server listener in a new thread
    let pixmap_thread = pixmap.clone();
    let stats_thread = stats.clone();
    let server_thread = thread::spawn(move || {
        // Build the server future, run it in a threadpool
        tokio_io_pool::run(
            server(&host, pixmap_thread.clone(), stats_thread.clone()),
        );
    });

    // Render the pixelflut screen
    if !arg_handler.no_render() {
        // TODO: build a future for rendering
        render(&arg_handler, &pixmap, stats);
    } else {
        // Do not render, wait on the server thread instead
        println!("Not rendering canvas, disabled with the --no-render flag");
        server_thread.join().unwrap();
    }
}

/// Build the pixelflut server future.
fn server(host: &SocketAddr, pixmap: Arc<Pixmap>, stats: Arc<Stats>)
    -> impl Future<Item = (), Error = ()>
{
    // Set up the connection listener
    let listener = TcpListener::bind(host)
        .expect("failed to bind");
    println!("Listening on: {}", host);

    let stats_connect = stats.clone();

    listener
        .incoming()
        .map_err(|e| eprintln!("Listener error: {}", e))
        .inspect(move |ref socket| {
            // Client connected, report the address
            if let Ok(addr) = socket.peer_addr() {
                println!("Client connected (from: {})", addr);
            } else {
                println!("Client connected (unknown address)");
            }

            // Increase the client count
            stats_connect.inc_clients();
        })
        .for_each(move |socket| {
            // Define the codec to use for the socket, split stream and sink
            let codec = PixCodec::new();
            let framed = Framed::new(socket, codec);
            let (tx, rx) = framed.split();

            // Clone references to the pixmap and stats objects
            let pixmap_shared = pixmap.clone();
            let stats_shared = stats.clone();
            let stats_disconnect = stats.clone();

            // Build the connection handling future, spawn it on the thread pool
            let rx_future = rx
                .map_err(|e| eprintln!("Socket codec error: {:?}", e))
                .map(move |line| match line {
                    Ok(line) => line.invoke(&pixmap_shared, &stats_shared),
                    Err(err) => {
                        println!("TODO ERR: {:?}", err);
                        ActionResult::ServerErr(err.description().into())
                    },
                })
                .map(|result| -> Option<Response> {
                    match result {
                        ActionResult::Ok => None,
                        ActionResult::Response(r) => Some(r),
                        ActionResult::ClientErr(e) => {
                            eprintln!("TODO: Client err: {}", e);
                            Some(Response::Error(e))
                        },
                        ActionResult::ServerErr(e) => {
                            eprintln!("TODO: Server err: {}", e);
                            Some(Response::Error(e))
                        },
                        ActionResult::Quit => {
                            eprintln!("TODO: Quit client!");
                            None
                        },
                    }
                })
                // TODO: remove throwing away errors
                .map_err(|_| io::Error::last_os_error())
                .filter_map(|r| r);

            // Build the future for responding
            let tx_future = tx
                .send_all(rx_future)
                .map(|_| ())
                .map_err(|_| ());

            let disconnect_future = tx_future
                .inspect(move |_| {
                    // TODO: chain after map instead of for_each

                    println!("Client disconnected");

                    // Increase the client count
                    stats_disconnect.dec_clients();
                });

            // Spawn the task
            tokio::spawn(disconnect_future)
        })
}

// fn worker(rx: mpsc::UnboundedReceiver<TcpStream>, pixmap: Arc<Pixmap>, stats: Arc<Stats>) {
//     let done = rx.for_each(move |socket| {
//         // A client connected, ensure we're able to get it's address
//         let addr = socket.peer_addr().expect("failed to get remote address");
//         println!("A client connected (from: {})", addr);

//         // Increase the number of clients
//         stats.inc_clients();

//         // Wrap the socket with the Lines codec,
//         // to interact with lines instead of raw bytes
//         let lines = Lines::new(socket, stats.clone());

//         // Define a client as connection
//         let disconnect_stats = stats.clone();
//         let connection = Client::new(lines, pixmap.clone(), stats.clone())
//             .map_err(|e| {
//                 // Handle connection errors, show an error message
//                 println!("Client connection error: {:?}", e);
//             })
//             .then(move |_| -> Result<_, _> {
//                 // Print a disconnect message
//                 println!("A client disconnected (from: {})", addr);

//                 // Decreasde the client connections number
//                 disconnect_stats.dec_clients();

//                 Ok(())
//             });

//         // Add the connection future to the pool on this thread
//         pool.execute(connection).unwrap();

//         Ok(())
//     });

//     // Handle all connection futures, and wait until we're done
//     done.wait().unwrap();
// }

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
