extern crate pixelpwnr_render;

use pixelpwnr_render::color::Color;
use pixelpwnr_render::pixmap::Pixmap;
use pixelpwnr_render::renderer::Renderer;

fn main() {
    // Build a pixelmap
    let mut pixmap = Pixmap::new(800, 600);
    pixmap.set_pixel(10, 10, Color::from_rgb(255, 0, 0));
    pixmap.set_pixel(20, 40, Color::from_rgb(0, 255, 0));

    // Build and run the renderer
    let mut renderer = Renderer::new(&pixmap);
    renderer.run();
}
