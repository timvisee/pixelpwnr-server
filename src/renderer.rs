use std::sync::{Arc, Mutex};

use gfx;
use gfx::{Device, Factory};
use gfx::handle::ShaderResourceView;
use gfx::texture::{AaMode, Kind, Mipmap};
use gfx::traits::FactoryExt;
use gfx_device_gl;
use gfx_window_glutin as gfx_glutin;
use glutin;
use glutin::{
    ContextBuilder,
    EventsLoop,
    GlContext,
    GlProfile,
    GlRequest,
    KeyboardInput,
    Robustness,
    VirtualKeyCode,
    WindowBuilder,
};
use glutin::Event::WindowEvent;
use glutin::WindowEvent::{
    Closed,
    KeyboardInput as WindowKeyboardInput,
    Resized,
};

use fps_counter::FpsCounter;
use pixmap::Pixmap;
use primitive::create_quad;
use stats_renderer::{Corner, StatsRenderer};
use vertex::Vertex;

/// Define used types
pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;
type F = gfx_device_gl::Factory;
type R = gfx_device_gl::Resources;

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

    // Used to render statistics on the canvas.
    stats: StatsRenderer<F, R>,

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
            stats: StatsRenderer::new(Corner::TopLeft),
            events_loop: glutin::EventsLoop::new(),
            fps: FpsCounter::new(),
        }
    }

    pub fn run(&mut self) {
        // Get the size of the canvas
        let size = self.pixmap.dimentions();

        // Define a window builder
        let builder = WindowBuilder::new()
            .with_title(self.title.to_string())
            .with_dimensions(size.0 as u32, size.1 as u32);

        // Define the graphics context
        // TODO: properly configure this context
        let context = ContextBuilder::new()
            .with_srgb(true)
            .with_gl(GlRequest::Latest)
            .with_gl_robustness(Robustness::TryRobustNoResetNotification)
            .with_gl_profile(GlProfile::Core)
            .with_multisampling(1)
            .with_vsync(true);

        // Initialize glutin
        let (
            window,
            mut device,
            mut factory,
            main_color,
            mut main_depth,
        ) = gfx_glutin::init::<ColorFormat, DepthFormat>(
            builder,
            context,
            &self.events_loop
        );

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
        let texture_kind = Kind::D2(size.0 as u16, size.1 as u16, AaMode::Single);

        // Create a base image
        let base_image = (
            Renderer::create_texture(&mut factory, self.pixmap.as_bytes(), texture_kind),
            factory.create_sampler_linear()
        );

        // Build pipe data
        let mut data = pipe::Data {
            vbuf: vertex_buffer,
            image: base_image,
            out: main_color.clone(),
        };

        // Build the stats renderer
        self.stats.init(factory.clone(), 20);
        self.stats.set_text("telnet localhost 1234".into());

        // Rendering flags
        let mut running = true;
        let mut update = false;
        let mut dimentions = (size.0 as f32, size.1 as f32);

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
            self.events_loop.poll_events(|event| {
                match event {
                    WindowEvent {
                        window_id: _,
                        event
                    } => match event {
                        // Stop running when escape is pressed
                        WindowKeyboardInput  {
                            device_id: _,
                            input: KeyboardInput {
                                scancode: _,
                                state: _,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                modifiers: _
                            }
                        } | Closed => running = false,

                        // Update the view when the window is resized
                        Resized(w, h) => {
                            gfx_glutin::update_views(&window, &mut data.out, &mut main_depth);
                            dimentions = (w as f32, h as f32);
                            update = true
                        },

                        _ => {},
                    },

                    _ => {},
                }
            });

            // Clear the buffer
            encoder.clear(&data.out, BLACK);

            // Draw through the pipeline
            encoder.draw(&slice, &pso, &data);

            self.stats.draw(&mut encoder, &main_color).unwrap();

            encoder.flush(&mut device);

            // Swap the frame buffers
            window.swap_buffers().unwrap();

            device.cleanup();

            // Tick the FPS counter
            //self.fps.tick();
        }
    }

    pub fn stats(&self) -> &StatsRenderer<F, R> {
        &self.stats
    }

    /// Load a texture from the given `path`.
    fn create_texture(
        factory: &mut F,
        data: &[u8],
        kind: Kind,
    ) -> ShaderResourceView<R, [f32; 4]> {
        // Create a GPU texture
        // TODO: make sure the mipmap state is correct
        let (_, view) = factory.create_texture_immutable_u8::<ColorFormat>(
            kind,
            Mipmap::Provided,
            &[data],
        ).unwrap();

        view
    }
}
