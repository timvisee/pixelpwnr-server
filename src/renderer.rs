use gfx;
use gfx::Device;
use gfx::texture::{AaMode, Kind};
use gfx::traits::FactoryExt;
use gfx_window_glutin as gfx_glutin;
use glutin::{EventsLoop, VirtualKeyCode};
use glutin;
use glutin::WindowEvent::*;

use fps_counter::FpsCounter;
use pixmap::Pixmap;
use primitive::create_quad;
use vertex::Vertex;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

/// Black color definition with 4 channels.
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// Screen shader data pipeline
gfx_defines! {
    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        image: gfx::TextureSampler<[f32; 4]> = "t_Image",
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

/// The renderer.
pub struct Renderer<'a> {
    // The window title.
    title: &'a str,

    // Pixel map holding the screen data. 
    pixmap: &'a Pixmap,

    // Glutin events loop.
    events_loop: EventsLoop,

    // A FPS counter for the renderer.
    fps: FpsCounter,
}

impl<'a> Renderer<'a> {
    /// Construct a new renderer.
    ///
    /// The renderer window title should be given to `title`.
    /// The pixel map that is rendered should be given to `pixmap`.
    pub fn new(
        title: &'a str,
        pixmap: &'a Pixmap,
    ) -> Renderer<'a> {
        // Construct and return the renderer
        Renderer {
            title,
            pixmap,
            events_loop: glutin::EventsLoop::new(),
            fps: FpsCounter::new(),
        }
    }

    pub fn run(&mut self) {
        // Define a window builder
        let builder = glutin::WindowBuilder::new()
            .with_title(self.title.to_string())
            .with_dimensions(800, 600)
            .with_vsync();

        // Initialize glutin
        let (
            window,
            mut device,
            mut factory,
            main_color,
            mut main_depth,
        ) = gfx_glutin::init::<ColorFormat, DepthFormat>(builder, &self.events_loop);

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

        // Create a full screen quad, plane, that is rendered on
        let plane = create_quad();
        let (vertex_buffer, mut slice) = plane.create_vertex_buffer(&mut factory);

        // Define the texture kind
        let texture_kind = Kind::D2(800, 600, AaMode::Single);

        // Create a base image
        let base_image = (
            Renderer::create_texture(&mut factory, self.pixmap.as_bytes(), texture_kind),
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

        // Keep rendering until we're done
        while running {
            // Create a texture with the new data, set it to upload
            data.image = (
                Renderer::create_texture(&mut factory, self.pixmap.as_bytes(), texture_kind),
                factory.create_sampler_linear(),
            );

            // Update graphics when required
            if update {
                // TODO: can we remove this?
                let (vertex_buffer, slice_new) = plane.create_vertex_buffer(&mut factory);

                // Redefine the vertex buffer and slice
                data.vbuf = vertex_buffer;
                slice = slice_new;

                // We've successfully updated
                update = false
            }

            // Poll vor events
            self.events_loop.poll_events(|glutin::Event::WindowEvent{window_id: _, event}| {
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

            // Tick the FPS counter
            self.fps.tick();
        }
    }

    /// Load a texture from the given `path`.
    fn create_texture<F, R>(factory: &mut F, data: &[u8], kind: gfx::texture::Kind)
        -> gfx::handle::ShaderResourceView<R, [f32; 4]>
        where
            F: gfx::Factory<R>,
            R: gfx::Resources,
    {
        // Create a GPU texture
        let (_, view) = factory.create_texture_immutable_u8::<ColorFormat>(
            kind,
            &[data],
        ).unwrap();

        view
    }
}
