mod ref_values;
pub mod stats_renderer;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gfx::handle::ShaderResourceView;
use gfx::texture::{AaMode, Kind, Mipmap};
use gfx::traits::FactoryExt;
use gfx_glutin::{ContextBuilderExt, WindowInitExt, WindowUpdateExt};
use glutin::dpi::LogicalSize;
use glutin::event::{Event, VirtualKeyCode, WindowEvent};
use glutin::{ContextBuilder, GlProfile, GlRequest, Robustness};

use gfx::{self, *};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::{Fullscreen, WindowBuilder};
use old_school_gfx_glutin_ext as gfx_glutin;

use crate::fps_counter::FpsCounter;
use crate::pixmap::Pixmap;
use crate::primitive::create_quad_max;
use crate::vertex::Vertex;
use stats_renderer::{Corner, StatsRenderer};

/// Define used types
pub(crate) type ColorFormat = gfx::format::Rgba8;
pub(crate) type DepthFormat = gfx::format::DepthStencil;
type F = gfx_device_gl::Factory;
pub(crate) type R = gfx_device_gl::Resources;

/// Black color definition with 4 channels.
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

// Screen shader data pipeline
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
    pixmap: Arc<Pixmap>,

    // Used to render statistics on the canvas.
    stats: StatsRenderer<F>,

    // Glutin events loop.
    events_loop: EventLoop<()>,

    // A FPS counter for the renderer.
    #[allow(unused)]
    fps: FpsCounter,
}

impl<'a> Renderer<'a> {
    /// Construct a new renderer.
    ///
    /// The renderer window title should be given to `title`.
    /// The pixel map that is rendered should be given to `pixmap`.
    pub fn new(title: &'a str, pixmap: Arc<Pixmap>) -> Renderer<'a> {
        // Construct and return the renderer
        Renderer {
            title,
            pixmap,
            stats: StatsRenderer::new(Corner::TopLeft),
            events_loop: EventLoop::new(),
            fps: FpsCounter::new(),
        }
    }

    pub fn run(
        mut self,
        fullscreen: bool,
        stats_size: u8,
        stats_offset: (u32, u32),
        stats_padding: i32,
        stats_col_spacing: i32,
        keep_running: Arc<AtomicBool>,
    ) {
        // Get the size of the canvas
        let size = self.pixmap.dimentions();

        // Select a monitor for full screening
        // TODO: allow selecting a specific monitor
        let monitor = if fullscreen {
            Some(Fullscreen::Borderless(self.events_loop.primary_monitor()))
        } else {
            None
        };

        // Define a window builder
        let builder = WindowBuilder::new()
            .with_title(self.title.to_string())
            .with_fullscreen(monitor)
            .with_inner_size(LogicalSize {
                width: size.0 as f64,
                height: size.1 as f64,
            });

        // Define the graphics context
        // TODO: properly configure this context
        let (window, mut device, mut factory, mut main_color, mut main_depth) =
            ContextBuilder::new()
                .with_srgb(true)
                .with_gl(GlRequest::Latest)
                .with_gl_robustness(Robustness::TryRobustNoResetNotification)
                .with_gl_profile(GlProfile::Core)
                .with_multisampling(1)
                .with_gfx_color_depth::<ColorFormat, DepthFormat>()
                .with_vsync(true)
                .build_windowed(builder, &self.events_loop)
                .unwrap()
                .init_gfx();

        let my_window_id = window.window().id();

        // command_buffer.set_ref_values(RefValues {
        //     stencil: (0, 0),
        //     blend: [0.5, 0.5, 0.5, 0.5],
        // });

        // Create the command encoder
        let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

        // Create a shader pipeline
        let pso = factory
            .create_pipeline_simple(
                include_bytes!("../../shaders/screen.glslv"),
                include_bytes!("../../shaders/screen.glslf"),
                pipe::new(),
            )
            .unwrap();

        // Create a full screen quad, plane, that is rendered on
        let plane = create_quad_max();
        let (vertex_buffer, slice) = plane.create_vertex_buffer(&mut factory);

        // Define the texture kind
        let texture_kind = Kind::D2(size.0 as u16, size.1 as u16, AaMode::Single);

        // Create a base image
        let base_image = (
            Renderer::create_texture(&mut factory, self.pixmap.as_bytes(), texture_kind),
            factory.create_sampler_linear(),
        );

        // Build pipe data
        let mut data_depth = main_depth.clone();
        let mut data = pipe::Data {
            vbuf: vertex_buffer,
            image: base_image,
            out: main_color.clone(),
        };

        let dimentions = (size.0 as f32, size.1 as f32);
        // Build the stats renderer
        self.stats
            .init(
                factory.clone(),
                dimentions,
                main_color.clone(),
                main_depth.clone(),
                stats_size,
                stats_offset,
                stats_padding,
                stats_col_spacing,
            )
            .expect("failed to initialize stats text renderer");

        let mut next_frame_time = Instant::now();

        self.events_loop.run(move |event, _target, control_flow| {
            if !keep_running.load(Ordering::SeqCst) {
                *control_flow = ControlFlow::Exit;
                return;
            }

            let keycode = if let Event::WindowEvent { window_id, event } = &event {
                if window_id == &my_window_id {
                    if let WindowEvent::KeyboardInput { input, .. } = event {
                        input.virtual_keycode
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let is_close_request = if let Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } = &event
            {
                window_id == &my_window_id
            } else {
                false
            };

            let exit = keycode == Some(VirtualKeyCode::Escape) || is_close_request;

            if let Event::WindowEvent {
                event: WindowEvent::Resized(s),
                ..
            } = event
            {
                let dimentions = (s.width as f32, s.height as f32);
                // Update the main color and depth
                window.update_gfx(&mut main_color, &mut main_depth);

                // Update the pixel texture
                window.update_gfx(&mut data.out, &mut data_depth);

                // Update the stats text
                self.stats.update_views(&window, dimentions);
            }

            // We don't want to re-render the whole frame each time someone moves their mouse, so let's
            // put a time limit on it
            if Instant::now() > next_frame_time || event == Event::MainEventsCleared {
                data.image = (
                    Renderer::create_texture(&mut factory, self.pixmap.as_bytes(), texture_kind),
                    factory.create_sampler_linear(),
                );

                // Clear the buffer
                encoder.clear(&data.out, BLACK);

                // Draw through the pipeline
                encoder.draw(&slice, &pso, &data);

                // Draw the stats
                self.stats.draw(&mut encoder, &main_color).unwrap();

                encoder.flush(&mut device);

                // Swap the frame buffers
                window.swap_buffers().unwrap();

                device.cleanup();
                // Reserve at most 1 ms for processing input events
                next_frame_time = Instant::now() + Duration::from_millis(1);
            }

            if exit || !keep_running.load(Ordering::SeqCst) {
                keep_running.store(false, Ordering::SeqCst);
            } else {
                *control_flow = ControlFlow::WaitUntil(next_frame_time);
            }
        });
    }

    /// This will run forever, or until an escape character is input
    pub fn run_default(self) {
        self.run(false, 20, (10, 10), 12, 20, Arc::new(AtomicBool::new(true)));
    }

    pub fn stats(&self) -> &StatsRenderer<F> {
        &self.stats
    }

    /// Load a texture from the given `path`.
    fn create_texture(factory: &mut F, data: &[u8], kind: Kind) -> ShaderResourceView<R, [f32; 4]> {
        // Create a GPU texture
        // TODO: make sure the mipmap state is correct
        let (_, view) = factory
            .create_texture_immutable_u8::<ColorFormat>(kind, Mipmap::Provided, &[data])
            .unwrap();

        view
    }
}
