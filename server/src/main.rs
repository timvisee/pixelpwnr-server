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

use codec::{CodecOptions, Lines};
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
    let opts = arg_handler.clone().into();
    let tokio_runtime = std::thread::spawn(move || {
        runtime.block_on(async move {
            for _ in 0..16 {
                synthetic_client(net_pixmap.clone(), net_stats.clone(), opts);
            }
            listen(listener, net_pixmap, net_stats, opts).await;
            net_running_2.store(false, Ordering::Relaxed);
        })
    });

    if !arg_handler.no_render {
        render(&arg_handler, pixmap, stats, net_running);
    } else {
        tokio_runtime.join().unwrap()
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
            println!("Failed to accept a connection");
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
        let mut lines_val = Lines::new(socket, stats.clone(), pixmap, opts);
        let lines = Pin::new(&mut lines_val);

        let result = lines.await;

        // Print a disconnect message
        println!("A client disconnected (from: {}). Reason: {}", addr, result);

        // Decreasde the client connections number
        disconnect_stats.dec_clients();
    });
}

fn synthetic_client(pixmap: Arc<Pixmap>, stats: Arc<Stats>, opts: CodecOptions) {
    use std::io::Error;
    use std::task::{Context, Poll};

    let mut buffer = Vec::new();

    for x in 0..800u16 {
        for y in 0..600u16 {
            buffer.extend_from_slice(b"PB");
            buffer.extend_from_slice(&x.to_le_bytes());
            buffer.extend_from_slice(&y.to_le_bytes());
            buffer.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        }
    }

    struct RepeatWriter {
        cursor: usize,
        buffer: Vec<u8>,
    }

    impl tokio::io::AsyncRead for RepeatWriter {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            assert!(buf.filled().len() == 0);

            let mut left_to_write = buf.capacity();

            while left_to_write != 0 {
                let cursor = self.cursor;
                let to_write = &self.buffer[cursor..];

                let to_write_len = left_to_write.min(self.buffer.len() - cursor);

                buf.put_slice(&to_write[..to_write_len]);
                self.as_mut().cursor = (cursor + to_write_len) % self.buffer.len();

                left_to_write -= to_write_len;
            }

            Poll::Ready(Ok(()))
        }
    }

    impl tokio::io::AsyncWrite for RepeatWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            println!("{}", String::from_utf8_lossy(buf));
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    let mut repeat_writer = RepeatWriter { cursor: 0, buffer };

    tokio::spawn(async move {
        let socket = Pin::new(&mut repeat_writer);

        // Wrap the socket with the Lines codec,
        // to interact with lines instead of raw bytes
        let mut lines_val = Lines::new(socket, stats.clone(), pixmap, opts);
        let lines = Pin::new(&mut lines_val);

        let result = lines.await;

        println!("{result}");
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
