mod color;
mod fps_counter;
mod model;
mod pixmap;
mod primitive;
mod renderer;
mod stats_renderer;
mod vertex;

use crate::color::Color;
use crate::pixmap::Pixmap;
use crate::renderer::Renderer;

pub fn main() {
    // Build a pixelmap
    let pixmap = Pixmap::new(800, 600);
    pixmap.set_pixel(10, 10, Color::from_rgb(255, 0, 0)).unwrap();
    pixmap.set_pixel(20, 20, Color::from_hex("FF00FFFF").unwrap()).unwrap();

    // Build the renderer
    let mut renderer = Renderer::new("pixelpwnr-render", &pixmap);

    // Run
    renderer.run_default();
}
