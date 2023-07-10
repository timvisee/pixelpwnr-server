mod arg_handler;
mod cmd;
mod codec;
mod stat_monitor;
mod stat_reporter;
mod stats;

#[cfg(feature = "influxdb2")]
pub mod influxdb;

use std::{
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use arg_handler::StatsOptions;
use clap::StructOpt;
use pixelpwnr_render::{Pixmap, Renderer};
use tokio::net::{TcpListener, TcpStream};

use codec::{CodecOptions, Lines};
use stat_reporter::StatReporter;
use stats::{Stats, StatsRaw};

use crate::arg_handler::{Opts, StatsSaveMethod};

fn main() {
    pretty_env_logger::formatted_builder()
        .parse_filters(
            &std::env::vars()
                .find(|(n, _)| n == "RUST_LOG")
                .map(|(_, v)| v)
                .unwrap_or("info,gfx_device_gl=off,winit=off".to_string()),
        )
        .init();

    // Create a new runtime to be ran on a different (set of) OS threads
    // so that we don't block the runtime by running the renderer on it
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let arg_handler = Opts::parse();

    let stat_save_opts = &arg_handler.stat_options;
    let (width, height) = arg_handler.size();
    log::info!("Canvas size: {}x{}", width, height);

    let pixmap = Arc::new(Pixmap::new(width, height));
    let keep_running = Arc::new(AtomicBool::new(true));

    let stats = runtime.block_on(build_stats(stat_save_opts));
    let stats = Arc::new(stats);

    if let Some(dir) = arg_handler.save_dir.clone() {
        let pixmap = pixmap.clone();
        runtime.spawn(spawn_save_image(
            dir,
            pixmap,
            Duration::from_secs(arg_handler.save_interval),
        ));
    }

    // Create a std threa first. Tokio's [`TcpStream::listen`] automatically sets
    // SO_REUSEADDR which means that it won't return an error if another program is
    // already listening on our port/address. Weird.
    let host = arg_handler.host;
    let listener = match std::net::TcpListener::bind(&host) {
        Ok(v) => v,
        Err(e) => panic!("Failed to bind to address {:?}. Error: {:?}", &host, e),
    };
    log::info!("Listening on: {}", host);

    let net_pixmap = pixmap.clone();
    let net_stats = stats.clone();
    let net_running_2 = keep_running.clone();
    let opts = arg_handler.clone().into();

    let renderer = build_renderer(&arg_handler, pixmap, stats, keep_running, &runtime);

    let tokio_runtime = std::thread::spawn(move || {
        runtime.block_on(async move {
            listen(listener, net_pixmap, net_stats, opts).await;
            net_running_2.store(false, Ordering::Relaxed);
        })
    });

    if !arg_handler.no_render {
        renderer();
    } else {
        tokio_runtime.join().unwrap()
    }
}

async fn build_stats(stat_opts: &StatsOptions) -> Stats {
    match stat_opts.load_on_start {
        Some(StatsSaveMethod::File) => {
            if let Some(path) = &stat_opts.stats_file {
                StatsRaw::load(path.as_path())
                    .as_ref()
                    .map(Stats::from_raw)
                    .unwrap_or(Stats::new())
            } else {
                log::warn!("stat loading is set to be from file, but stats file was not provided. Continuing with empty stats.");
                Stats::new()
            }
        }
        #[cfg(feature = "influxdb2")]
        Some(StatsSaveMethod::Influxdb) => {
            if let Some(influxdb_config) = stat_opts.influxdb_config() {
                let mut client = influxdb::InfluxDB::new(influxdb_config);
                client.load_stats().await
            } else {
                log::warn!(
                    "stat loading is set to be from influxdb, but influxdb config was not provided. Continuing with empty stats."
                );
                Stats::new()
            }
        }
        None => Stats::new(),
    }
}

async fn listen(
    listener: std::net::TcpListener,
    pixmap: Arc<Pixmap>,
    stats: Arc<Stats>,
    opts: CodecOptions,
) {
    let listener = TcpListener::from_std(listener).unwrap();

    loop {
        let pixmap_worker = pixmap.clone();
        let stats_worker = stats.clone();
        let (socket, _) = if let Ok(res) = listener.accept().await {
            res
        } else {
            log::warn!("Failed to accept a connection");
            continue;
        };
        handle_socket(socket, pixmap_worker, stats_worker, opts);
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

        let mut pixmap = (*pixmap).clone();

        image::save_buffer(
            path,
            pixmap.as_bytes(),
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        )
        .unwrap();

        tokio::time::sleep(interval).await;
    }
}

/// Spawn a new task with the given socket
fn handle_socket(
    mut socket: TcpStream,
    pixmap: Arc<Pixmap>,
    stats: Arc<Stats>,
    opts: CodecOptions,
) {
    // A client connected, ensure we're able to get it's address
    let addr = socket.peer_addr().expect("failed to get remote address");
    log::info!("A client connected (from: {})", addr);

    // Increase the number of clients
    stats.inc_clients();

    let disconnect_stats = stats.clone();

    let pixmap = pixmap.clone();
    let stats = stats.clone();

    tokio::spawn(async move {
        let socket = Pin::new(&mut socket);

        // Wrap the socket with the Lines codec,
        // to interact with lines instead of raw bytes
        let mut lines_val = Lines::new(socket, stats.clone(), pixmap, opts);
        let lines = Pin::new(&mut lines_val);

        let result = lines.await;

        // Print a disconnect message
        log::info!("A client disconnected (from: {}). Reason: {}", addr, result);

        // Decreasde the client connections number
        disconnect_stats.dec_clients();
    });
}

/// Start the pixel map renderer.
fn build_renderer<'a>(
    arg_handler: &'a Opts,
    pixmap: Arc<Pixmap>,
    stats: Arc<Stats>,
    net_running: Arc<AtomicBool>,
    runtime: &tokio::runtime::Runtime,
) -> impl FnOnce() -> () + 'a {
    // Build the renderer
    let renderer = Renderer::new(env!("CARGO_PKG_NAME"), pixmap);

    // Borrow the statistics text
    let stats_text = renderer.stats().text();

    // Create a stats reporter, and start reporting
    let reporter = StatReporter::new(
        arg_handler.stats_screen_interval(),
        arg_handler.stats_stdout_interval(),
        arg_handler.stat_options.stats_save_interval(),
        arg_handler.stat_options.stats_file.clone(),
        stats,
        Some(stats_text),
        #[cfg(feature = "influxdb2")]
        arg_handler
            .stat_options
            .influxdb_config()
            .map(|c| influxdb::InfluxDB::new(c)),
    );
    runtime.spawn(reporter.run());

    // Render the canvas
    || {
        renderer.run(
            arg_handler.fullscreen,
            arg_handler.stats_font_size,
            arg_handler.stats_offset(),
            arg_handler.stats_padding,
            arg_handler.stats_col_spacing,
            net_running,
        )
    }
}
