#![feature(try_trait)]

extern crate bufstream;
extern crate pixelpwnr_render;

mod app;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use bufstream::BufStream;
use pixelpwnr_render::Color;
use pixelpwnr_render::Pixmap;
use pixelpwnr_render::Renderer;

/// Main application entrypoint.
fn main() {
    // Build a pixelmap, create a reference for the connection thread
    let pixmap = Arc::new(Pixmap::new(800, 600));
    let pixmap_thread = pixmap.clone();

    // Spawn the server thread
    thread::spawn(move || {
        // Set up a listener for the server
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
        println!("Listening on 127.0.0.1:8080...");

        // Accept connections and process them, spawning a new thread for each one
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    // Create a pixmap reference for the client
                    let pixmap_client = pixmap_thread.clone();

                    // Spawn the client handling thread
                    thread::spawn(move || handle_client(stream, pixmap_client));
                },
                Err(_) => {}, // Connection failed
            }
        }
    });

    // Render the pixelflut screen
    render(&pixmap);
}

/// Handle a client connection.
fn handle_client(stream: TcpStream, pixmap: Arc<Pixmap>) {
    // Create a buffered reader
    let mut reader = BufStream::new(stream);

    // A client has connected
    println!("A client has connected");

    // Read client input
    loop {
        // Read a new line
        let mut data = String::new();
        if let Err(_) = reader.read_line(&mut data) {
            println!("An error occurred, closing stream");
            return;
        }

        // Split the input data, and get the first split
        let mut splits = data.trim()
            .split(" ")
            .filter(|x| !x.is_empty());
        let cmd = match splits.next() {
            Some(c) => c,
            None => continue,
        };

        // Process the command
        // TODO: improve response handling
        match process_command(cmd, splits, &pixmap) {
            CmdResponse::Ok => {},
            CmdResponse::Response(msg) => {
                write!(reader, "{}", msg).expect("failed to write response");
                reader.flush().expect("failed to flush stream");
            },
            CmdResponse::ClientErr(err) => {
                write!(reader, "ERR {}", err).expect("failed to write error");
                reader.flush().expect("failed to flush stream");
            },
            CmdResponse::InternalErr(err) => {
                println!("Error: \"{}\". Closing connection...", err);
                return;
            },
        }
    }
}

enum CmdResponse<'a> {
    Ok,
    Response(String),
    ClientErr(&'a str),
    InternalErr(&'a str),
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
