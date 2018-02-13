#![feature(attr_literals)]

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate image;
#[macro_use]
extern crate lazy_static;

pub mod color;
pub mod fps_counter;
mod model;
pub mod pixmap;
mod primitive;
pub mod renderer;
mod vertex;

// Reexport types
pub use color::Color;
pub use pixmap::Pixmap;
pub use renderer::Renderer;
