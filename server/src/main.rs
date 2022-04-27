mod arg_handler;
mod cmd;
mod codec;
mod stat_monitor;
mod stat_reporter;
mod stats;

use std::{
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use clap::StructOpt;
use pixelpwnr_render::{Pixmap, Renderer};
use tokio::net::{TcpListener, TcpStream};

use codec::Lines;
use stat_reporter::StatReporter;
use stats::{Stats, StatsRaw};

use crate::arg_handler::Opts;

// TODO: use some constant for new lines

fn main() {
    let arg_handler = Opts::parse();

    // Build a stats manager, load persistent stats
    let stats = arg_handler
        .stats_file
        .as_ref()
        .map(|f| StatsRaw::load(f.as_path()))
        .flatten()
        .map(|s| Stats::from_raw(&s))
        .unwrap_or(Stats::new());

    let stats = Arc::new(stats);

    let (width, height) = arg_handler.size();
    let pixmap = Arc::new(Pixmap::new(width, height));
    println!("Canvas size: {}x{}", width, height);

    // Create a new runtime to be ran on a different (set of) OS threads
    // so that we don't block the runtime by running the renderer on it
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    if let Some(dir) = arg_handler.save_dir.clone() {
        let pixmap = pixmap.clone();
        runtime.spawn(spawn_save_image(
            dir,
            pixmap,
            Duration::from_secs(arg_handler.save_interval),
        ));
    }

    let net_running = Arc::new(AtomicBool::new(true));

    // Create a std threa first. Tokio's [`TcpStream::listen`] automatically sets
    // SO_REUSEADDR which means that it won't return an error if another program is
    // already listening on our port/address. Weird.
    let host = arg_handler.host;
    let listener = match std::net::TcpListener::bind(&host) {
        Ok(v) => v,
        Err(e) => panic!("Failed to bind to address {:?}. Error: {:?}", &host, e),
    };
    println!("Listening on: {}", host);

    let net_pixmap = pixmap.clone();
    let net_stats = stats.clone();
    let net_running_2 = net_running.clone();
    let tokio_runtime = std::thread::spawn(move || {
        runtime.block_on(async move {
            listen(listener, net_pixmap, net_stats).await;
            net_running_2.store(false, Ordering::Relaxed);
        })
    });

    if !arg_handler.no_render {
        render(&arg_handler, pixmap, stats, net_running);
    } else {
        tokio_runtime.join().unwrap()
    }
}

async fn listen(listener: std::net::TcpListener, pixmap: Arc<Pixmap>, stats: Arc<Stats>) {
    let listener = TcpListener::from_std(listener).unwrap();

    loop {
        let pixmap_worker = pixmap.clone();
        let stats_worker = stats.clone();
        let (socket, _) = if let Ok(res) = listener.accept().await {
            res
        } else {
            println!("Failed to accept a connection");
            continue;
        };
        handle_socket(socket, pixmap_worker, stats_worker);
    }
}

/// Save the current canvas at the current interval
async fn spawn_save_image(dir: PathBuf, pixmap: Arc<Pixmap>, interval: Duration) {
    std::fs::create_dir_all(&dir).unwrap();

    loop {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut path = dir.clone();
        path.push(format!("{}.png", now));

        let (width, height) = pixmap.dimensions();

        let image_data = pixmap.as_bytes();

        image::save_buffer(
            path,
            image_data,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        )
        .unwrap();
        tokio::time::sleep(interval).await;
    }
}

/// Spawn a new task with the given socket
fn handle_socket(mut socket: TcpStream, pixmap: Arc<Pixmap>, stats: Arc<Stats>) {
    // A client connected, ensure we're able to get it's address
    let addr = socket.peer_addr().expect("failed to get remote address");
    println!("A client connected (from: {})", addr);

    // Increase the number of clients
    stats.inc_clients();

    let disconnect_stats = stats.clone();

    let pixmap = pixmap.clone();
    let stats = stats.clone();

    tokio::spawn(async move {
        let socket = Pin::new(&mut socket);

        // Wrap the socket with the Lines codec,
        // to interact with lines instead of raw bytes
        let mut lines_val = Lines::new(socket, stats.clone(), pixmap);
        let lines = Pin::new(&mut lines_val);

        let result = lines.await;

        // Print a disconnect message
        println!("A client disconnected (from: {}). Reason: {}", addr, result);

        // Decreasde the client connections number
        disconnect_stats.dec_clients();
    });
}

/// Start the pixel map renderer.
fn render(
    arg_handler: &Opts,
    pixmap: Arc<Pixmap>,
    stats: Arc<Stats>,
    net_running: Arc<AtomicBool>,
) {
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
        net_running,
    );
}
