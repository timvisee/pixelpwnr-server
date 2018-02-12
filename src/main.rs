extern crate pixelpwnr_render;

use std::io::BufReader;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;

use pixelpwnr_render::Color;
use pixelpwnr_render::Pixmap;
use pixelpwnr_render::Renderer;

/// Main application entrypoint.
fn main() {
    // Set up a listener for the server
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Listening on 127.0.0.1:8080...");

    // Accept connections and process them, spawning a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_client(stream));
            },
            Err(_) => {}, // Connection failed
        }
    }

    // Render the pixelflut screen
    render();
}

/// Handle a client connection.
fn handle_client(stream: TcpStream) {
    // Create a buffered reader
    let mut reader = BufReader::new(stream);

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

        // Show the received data
        println!("Received: {}", data.trim());
    }
}

fn render() {
    // Build a pixelmap
    let mut pixmap = Pixmap::new(800, 600);
    pixmap.set_pixel(10, 10, Color::from_rgb(255, 0, 0));
    pixmap.set_pixel(20, 40, Color::from_rgb(0, 255, 0));

    // Build and run the renderer
    let mut renderer = Renderer::new(&pixmap);
    renderer.run();
}
