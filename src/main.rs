extern crate bufstream;
extern crate pixelpwnr_render;

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

        // Handle the screen size command
        // if pattern_size.is_match(&data) {
        if data.trim() == "SIZE" {
            let (width, height) = pixmap.dimentions();
            write!(reader, "SIZE {} {}\n", width, height).expect("failed to write");
            reader.flush().expect("failed to flush");
            continue;
        }

        // Handle pixel set command
        let mut splits = data.trim().split(" ");
        if splits.next().unwrap() == "PX" {
            let x: usize = splits.next().unwrap().parse().expect("invalid x coordinate");
            let y: usize = splits.next().unwrap().parse().expect("invalid x coordinate");
            let color: Color = Color::from_hex(splits.next().unwrap()).expect("invalid color value");
            // let color = Color::from_rgb(0, 0, 255);
            pixmap.set_pixel(x, y, color);
            continue;
        }
    }
}

fn render(pixmap: &Pixmap) {
    // Build and run the renderer
    let mut renderer = Renderer::new("pixelpwnr-server", pixmap);
    renderer.run();
}
