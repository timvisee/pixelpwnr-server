#![feature(repr_align, attr_literals)]

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate image;
#[macro_use]
extern crate lazy_static;

mod color;
mod model;
mod pixmap;
mod primitive;
mod vertex;

use std::time::SystemTime;

use gfx::Device;
use gfx::texture::{AaMode, Kind};
use gfx::traits::FactoryExt;
use gfx_window_glutin as gfx_glutin;
use glutin::VirtualKeyCode;
use glutin::WindowEvent::*;

use color::Color;
use vertex::Vertex;
use pixmap::Pixmap;
use primitive::create_quad;

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines! {
    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        image: gfx::TextureSampler<[f32; 4]> = "t_Image",
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

pub fn main() {
    // Start a new glutin event loop
    let events_loop = glutin::EventsLoop::new();

    // Define a window builder
    let builder = glutin::WindowBuilder::new()
        .with_title("pixelpwnr-render-test".to_string())
        .with_dimensions(800, 600);
        // .with_vsync();

    // Initialize glutin
    let (
        window,
        mut device,
        mut factory,
        main_color,
        mut main_depth,
    ) = gfx_glutin::init::<ColorFormat, DepthFormat>(builder, &events_loop);

    // Create the command encoder
    let mut encoder: gfx::Encoder<_, _> = factory
        .create_command_buffer()
        .into();

    // Create a shader pipeline
    let pso = factory.create_pipeline_simple(
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/screen.glslv")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/screen.glslf")),
        pipe::new(),
    ).unwrap();

    // Create the vertices and indices, define the vertex buffer
    let quad = create_quad();
    let (vertices, indices) = (quad.vertices(), quad.indices());
    let (vertex_buffer, mut slice) = factory
        .create_vertex_buffer_with_slice(&vertices, &*indices);

    // Build a pixelmap
    let mut pixmap = Pixmap::new(800, 600);
    pixmap.set_pixel(10, 10, Color::from_rgb(255, 0, 0));
    pixmap.set_pixel(20, 20, Color::from_hex("FF00FFFF").unwrap());

    // Define the texture kind
    let texture_kind = Kind::D2(800, 600, AaMode::Single);

    // Create a base image
    let base_image = (
        create_texture(&mut factory, pixmap.as_bytes(), texture_kind),
        factory.create_sampler_linear()
    );

    // Build pipe data
    let mut data = pipe::Data {
        vbuf: vertex_buffer,
        image: base_image,
        out: main_color,
    };

    // Rendering flags
    let mut running = true;
    let mut update = false;
    let mut dimentions = (800.0, 600.0);

    // FPS counting
    let mut frame = 0;
    let mut report_next = SystemTime::now();

    // Keep rendering until we're done
    while running {
        // Create a texture with the new data, set it to upload
        data.image = (
            create_texture(&mut factory, pixmap.as_bytes(), texture_kind),
            factory.create_sampler_linear(),
        );

        // Update graphics when required
        if update {
            // TODO: can we remove this?
            let quad = create_quad();
            let (vertices, indices) = (quad.vertices(), quad.indices());
            let (vertex_buff, sl) = factory
                .create_vertex_buffer_with_slice(&vertices, &*indices);

            // Redefine the vertex buffer and slice
            data.vbuf = vertex_buff;
            slice = sl;

            // We've successfully updated
            update = false
        }

        // Poll vor events
        events_loop.poll_events(|glutin::Event::WindowEvent{window_id: _, event}| {
            match event {
                // Stop running when escape is pressed
                KeyboardInput(_, _, Some(VirtualKeyCode::Escape), _)
                | Closed => running = false,

                // Update the view when the window is resized
                Resized(w, h) => {
                    gfx_glutin::update_views(&window, &mut data.out, &mut main_depth);
                    dimentions = (w as f32, h as f32);
                    update = true
                },

                _ => (),
            }
        });

        // Clear the buffer
        encoder.clear(&data.out, BLACK);

        // Draw through the pipeline
        encoder.draw(&slice, &pso, &data);

        encoder.flush(&mut device);

        // Swap the frame buffers
        window.swap_buffers().unwrap();

        device.cleanup();

        // Count frames
        frame += 1;
        if report_next.elapsed().unwrap().as_secs() >= 1 {
            println!("FPS: {}", frame);
            frame = 0;
            report_next = SystemTime::now();
        }
    }
}

/// Load a texture from the given `path`.
fn create_texture<F, R>(factory: &mut F, data: &[u8], kind: gfx::texture::Kind)
    -> gfx::handle::ShaderResourceView<R, [f32; 4]>
    where
        F: gfx::Factory<R>,
        R: gfx::Resources,
{
    // // Ope the image from the given path
    // let img = image::open(path).unwrap().to_rgba();
    // let (width, height) = img.dimensions();

    // // Define the texture kind
    // let kind = gfx::texture::Kind::D2(
    //     width as u16,
    //     height as u16,
    //     gfx::texture::AaMode::Single,
    // );

    // Create a GPU texture
    let (_, view) = factory.create_texture_immutable_u8::<ColorFormat>(
        kind,
        &[data],
    ).unwrap();

    // let (texture, view, target) = factory.create_render_target::<ColorFormat>(
    //     width as u16,
    //     height as u16,
    // ).unwrap();

    // println!("INF: {:?}", texture.get_info());

    view
}
