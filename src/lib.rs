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
pub mod model;
pub mod pixmap;
pub mod primitive;
pub mod renderer;
pub mod vertex;
