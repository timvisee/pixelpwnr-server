#[cfg(feature = "stats")]
pub mod stats;

use std::num::NonZeroU32;
use std::sync::Arc;

use glium::index::PrimitiveType;
use glium::texture::pixel_buffer::PixelBuffer;
use glium::texture::{MipmapsOption, Texture2d, UncompressedFloatFormat};
use glium::{implement_vertex, program, uniform};
use glium::{Display, Surface};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::NotCurrentGlContext;
use glutin::surface::WindowSurface;
use parking_lot::Mutex;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{WindowId, WindowLevel};

use crate::fps_counter::FpsCounter;
use crate::pixmap::Pixmap;
#[cfg(feature = "stats")]
use crate::render_glium::stats::StatsRender;

#[cfg(not(feature = "stats"))]
type StatsRender = ();

/// Whether to clear to black each frame
const CLEAR_FRAME: bool = false;

type StatsText = Arc<Mutex<String>>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);

pub struct Application {
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u16>,
    /// Pixelfut pixel buffer object, used to upload pixel data to GPU much more efficiently
    image_pbo: PixelBuffer<(u8, u8, u8, u8)>,
    /// Pixelfut OpenGL texture, used to actually render the image
    image_texture: Texture2d,
    program: glium::Program,
}

pub struct State<T> {
    display: glium::Display<WindowSurface>,
    window: glium::winit::window::Window,
    context: T,
    pixmap: Arc<Pixmap>,
    fps: FpsCounter,
    stats: StatsRender,
    config: Config,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub fullscreen: bool,
    /// Whether to use nearest neighbor image scaling
    pub nearest_neighbor: bool,
    #[cfg(feature = "stats")]
    pub stats_font_size_px: f32,
    #[cfg(feature = "stats")]
    pub stats_offset_px: (f32, f32),
    #[cfg(feature = "stats")]
    pub stats_spacing_px: (f32, f32),
    #[cfg(feature = "stats")]
    pub stats_padding_px: f32,
}

/// Based upon <https://github.com/glium/glium/blob/master/examples/image.rs>
struct App<T> {
    state: Option<State<T>>,
    visible: bool,
    config: Config,
    close_requested: bool,
    title: String,
    pixmap: Arc<Pixmap>,
    stats_text: StatsText,
}

impl<T: ApplicationContext + 'static> ApplicationHandler<()> for App<T> {
    /// Resume handler mostly for Android compatibility
    ///
    /// For convenience sake, this is also called on all other platforms.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(State::new(
            event_loop,
            self.visible,
            self.config.clone(),
            &self.title,
            self.pixmap.clone(),
            self.stats_text.clone(),
        ));
        if !self.visible && self.close_requested {
            event_loop.exit();
        }
    }

    /// Suspend handler mostly for Android compatibility
    ///
    /// For convenience sake, this is also called on all other platforms.
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.state = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            glium::winit::event::WindowEvent::Resized(new_size) => {
                if let Some(state) = &mut self.state {
                    state.display.resize(new_size.into());
                    #[cfg(feature = "stats")]
                    state.stats.invalidate_background();
                }
            }
            #[cfg(feature = "stats")]
            glium::winit::event::WindowEvent::Focused(true) => {
                if let Some(state) = &mut self.state {
                    state.stats.invalidate_background();
                }
            }
            #[cfg(feature = "stats")]
            glium::winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(state) = &mut self.state {
                    state.stats.set_scale_factor(scale_factor);
                }
            }
            glium::winit::event::WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.context.update();
                    state.context.draw_frame(
                        &state.display,
                        &state.config,
                        &state.pixmap,
                        &mut state.stats,
                    );
                    state.fps.tick();
                    if self.close_requested {
                        event_loop.exit();
                    }
                }
            }
            // Exit the event loop when requested (by closing the window for example) or when
            // pressing the Esc key.
            glium::winit::event::WindowEvent::CloseRequested
            | glium::winit::event::WindowEvent::KeyboardInput {
                event:
                    glium::winit::event::KeyEvent {
                        state: glium::winit::event::ElementState::Pressed,
                        logical_key:
                            glium::winit::keyboard::Key::Named(glium::winit::keyboard::NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            // Every other event
            ev => {
                if let Some(state) = &mut self.state {
                    state.context.handle_window_event(&ev, &state.window);
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

impl<T: ApplicationContext + 'static> State<T> {
    pub fn new(
        event_loop: &glium::winit::event_loop::ActiveEventLoop,
        visible: bool,
        config: Config,
        title: &str,
        pixmap: Arc<Pixmap>,
        stats_text: StatsText,
    ) -> Self {
        let window_attributes = winit::window::Window::default_attributes()
            .with_title(title)
            .with_active(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            // Full screen on current monitor
            .with_fullscreen(
                config
                    .fullscreen
                    .then_some(winit::window::Fullscreen::Borderless(None)),
            )
            // Base window size on pixmap
            .with_inner_size(LogicalSize::new(
                pixmap.width() as u32,
                pixmap.height() as u32,
            ))
            .with_visible(visible);
        let config_template_builder = glutin::config::ConfigTemplateBuilder::new();
        let display_builder =
            glutin_winit::DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        // First we create a window
        let (window, gl_config) = display_builder
            .build(event_loop, config_template_builder, |mut configs| {
                // Just use the first configuration since we don't have any special preferences here
                configs.next().unwrap()
            })
            .unwrap();
        let window = window.unwrap();

        // Then the configuration which decides which OpenGL version we'll end up using, here we just use the default which is currently 3.3 core
        // When this fails we'll try and create an ES context, this is mainly used on mobile devices or various ARM SBC's
        // If you depend on features available in modern OpenGL Versions you need to request a specific, modern, version. Otherwise things will very likely fail.
        let window_handle = window
            .window_handle()
            .expect("couldn't obtain window handle");
        let context_attributes =
            glutin::context::ContextAttributesBuilder::new().build(Some(window_handle.into()));
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(Some(window_handle.into()));

        let not_current_gl_context = unsafe {
            gl_config
                .display()
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_config
                        .display()
                        .create_context(&gl_config, &fallback_context_attributes)
                        .expect("failed to create context")
                })
        };

        // Determine our framebuffer size based on the window size, or default to pixmap size if invisible
        let (width, height): (u32, u32) = if visible {
            window.inner_size().into()
        } else {
            (pixmap.width() as u32, pixmap.height() as u32)
        };

        let attrs = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.into(),
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
        // Now we can create our surface, use it to make our context current and finally create our display
        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };
        let current_context = not_current_gl_context.make_current(&surface).unwrap();
        let display = glium::Display::from_context_surface(current_context, surface).unwrap();

        Self::from_display_window(display, window, config, pixmap, stats_text)
    }

    pub fn from_display_window(
        display: glium::Display<WindowSurface>,
        window: glium::winit::window::Window,
        config: Config,
        pixmap: Arc<Pixmap>,
        #[cfg_attr(not(feature = "stats"), allow(unused))] stats_text: StatsText,
    ) -> Self {
        let context = T::new(&display, &pixmap);

        #[cfg(feature = "stats")]
        let stats = StatsRender::new(stats_text, &display, window.scale_factor());
        #[cfg(not(feature = "stats"))]
        let stats = ();

        Self {
            display,
            window,
            context,
            config,
            pixmap,
            fps: FpsCounter::default(),
            stats,
        }
    }

    /// Start the event_loop and keep rendering frames until the program is closed
    pub fn run_loop(title: String, config: Config, pixmap: Arc<Pixmap>, stats_text: StatsText) {
        let event_loop = glium::winit::event_loop::EventLoop::builder()
            .build()
            .expect("glium event loop building");
        let mut app = App::<T> {
            state: None,
            visible: true,
            config,
            close_requested: false,
            title,
            pixmap,
            stats_text,
        };
        let result = event_loop.run_app(&mut app);
        result.unwrap();
    }
}

pub trait ApplicationContext {
    fn draw_frame(
        &mut self,
        _display: &Display<WindowSurface>,
        _config: &Config,
        _pixmap: &Pixmap,
        _stats: &mut StatsRender,
    ) {
    }
    fn new(display: &Display<WindowSurface>, pixmap: &Pixmap) -> Self;
    fn update(&mut self) {}
    fn handle_window_event(
        &mut self,
        _event: &glium::winit::event::WindowEvent,
        _window: &glium::winit::window::Window,
    ) {
    }
}

impl ApplicationContext for Application {
    fn new(display: &Display<WindowSurface>, pixmap: &Pixmap) -> Self {
        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;

        // Create pixelflut OpenGL pixel buffer and texture
        let image_pbo = PixelBuffer::new_empty(display, (width * height) as usize);
        let image_texture = Texture2d::empty_with_format(
            display,
            UncompressedFloatFormat::U8U8U8U8,
            MipmapsOption::NoMipmap,
            width,
            height,
        )
        .unwrap();

        // Build vertex buffer, contains all the vertices we will draw
        let vertex_buffer = {
            glium::VertexBuffer::new(
                display,
                &[
                    Vertex {
                        position: [-1.0, -1.0],
                        tex_coords: [0.0, 0.0],
                    },
                    Vertex {
                        position: [-1.0, 1.0],
                        tex_coords: [0.0, 1.0],
                    },
                    Vertex {
                        position: [1.0, 1.0],
                        tex_coords: [1.0, 1.0],
                    },
                    Vertex {
                        position: [1.0, -1.0],
                        tex_coords: [1.0, 0.0],
                    },
                ],
            )
            .unwrap()
        };

        // building the index buffer
        let index_buffer =
            glium::IndexBuffer::new(display, PrimitiveType::TriangleStrip, &[1u16, 2, 0, 3])
                .unwrap();

        // Compile shaders and link them
        let program = program!(display,
            140 => {
                vertex: "
                    #version 140
                    uniform mat4 matrix;
                    in vec2 position;
                    in vec2 tex_coords;
                    out vec2 v_tex_coords;
                    void main() {
                        gl_Position = matrix * vec4(position, 0.0, 1.0);
                        v_tex_coords = tex_coords;
                    }
                ",
                fragment: "
                    #version 140
                    uniform sampler2D tex;
                    in vec2 v_tex_coords;
                    out vec4 f_color;
                    void main() {
                        f_color = texture(tex, v_tex_coords);
                    }
                ",
            },

            110 => {
                vertex: "
                    #version 110
                    uniform mat4 matrix;
                    attribute vec2 position;
                    attribute vec2 tex_coords;
                    varying vec2 v_tex_coords;
                    void main() {
                        gl_Position = matrix * vec4(position, 0.0, 1.0);
                        v_tex_coords = tex_coords;
                    }
                ",
                fragment: "
                    #version 110
                    uniform sampler2D tex;
                    varying vec2 v_tex_coords;
                    void main() {
                        gl_FragColor = texture2D(tex, v_tex_coords);
                    }
                ",
            },

            100 => {
                vertex: "
                    #version 100
                    uniform lowp mat4 matrix;
                    attribute lowp vec2 position;
                    attribute lowp vec2 tex_coords;
                    varying lowp vec2 v_tex_coords;
                    void main() {
                        gl_Position = matrix * vec4(position, 0.0, 1.0);
                        v_tex_coords = tex_coords;
                    }
                ",
                fragment: "
                    #version 100
                    uniform lowp sampler2D tex;
                    varying lowp vec2 v_tex_coords;
                    void main() {
                        gl_FragColor = texture2D(tex, v_tex_coords);
                    }
                    WINIT_UNIX_BACKEND=x11
                ",
            },
        )
        .unwrap();

        Self {
            vertex_buffer,
            index_buffer,
            image_texture,
            image_pbo,
            program,
        }
    }

    fn draw_frame(
        &mut self,
        display: &Display<WindowSurface>,
        config: &Config,
        pixmap: &Pixmap,
        #[cfg_attr(not(feature = "stats"), allow(unused))] stats: &mut StatsRender,
    ) {
        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;
        let pixels = pixmap.as_u8u8u8u8();

        // Upload new pixels to PBO on GPU
        self.image_pbo.write(pixels);

        #[cfg(feature = "stats")]
        stats.queue_draw(config);

        let mut frame = display.draw();

        // Load pixels from PBO into texture on GPU
        self.image_texture
            .main_level()
            .raw_upload_from_pixel_buffer(self.image_pbo.as_slice(), 0..width, 0..height, 0..1);

        // Configure texture sampling
        let mut tex_sampler = glium::uniforms::Sampler::new(&self.image_texture);
        if config.nearest_neighbor {
            tex_sampler = tex_sampler
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);
        }

        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32]
            ],
            tex: tex_sampler,
        };

        // Clearing the frame is not required since we always draw the image full screen
        if CLEAR_FRAME {
            frame.clear_color(0.0, 0.0, 0.0, 0.0);
        }

        frame
            .draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();

        #[cfg(feature = "stats")]
        stats.draw_queued(config, display, &mut frame);

        frame.finish().unwrap();
    }
}
