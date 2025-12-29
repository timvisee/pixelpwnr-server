use glium::backend::{Context, Facade};
use glium::draw_parameters::Blend;
use glium::index::PrimitiveType;
use glium::uniforms::{EmptyUniforms, UniformsStorage};
use glium::{program, uniform, DrawParameters, Surface};
use glium_glyph::glyph_brush::ab_glyph::{FontArc, Point};
use glium_glyph::glyph_brush::{GlyphCruncher, Section, Text};
use glium_glyph::GlyphBrush;
use ordered_float::OrderedFloat;
use parking_lot::Mutex;
use std::cmp::max;
use std::ops::Deref;
use std::sync::Arc;

use crate::render_glium::Vertex;

const FONT_BYTES: &[u8] = include_bytes!("../../fonts/DejaVuSans-2.37.ttf");

const FONT_SIZE: f32 = 20.0;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const TABLE_SPACING: (f32, f32) = (20.0, 5.0);
const OFFSET: (f32, f32) = (20.0, 20.0);

pub struct StatsRender {
    /// Glyph brush used for drawing text
    glyph_brush: GlyphBrush<'static, FontArc>,

    /// Window scale factor (DPI)
    scale_factor: f64,

    /// Text buffer to render
    text: Arc<Mutex<String>>,

    /// Last rendered text, used to detect changes
    last_text: String,

    bg_vertex_buffer: Option<glium::VertexBuffer<Vertex>>,
    bg_index_buffer: glium::IndexBuffer<u16>,
    bg_program: glium::Program,
    bg_draw_params: DrawParameters<'static>,
    bg_uniforms: UniformsStorage<'static, [[f32; 4]; 4], EmptyUniforms>,
    bg_last_size: Option<Point>,
}

impl StatsRender {
    /// Construct a new stats renderer
    pub fn new<C: Facade + Deref<Target = Context>>(
        text: Arc<Mutex<String>>,
        facade: &C,
        scale_factor: f64,
    ) -> Self {
        let font = FontArc::try_from_slice(FONT_BYTES).unwrap();
        let glyph_brush = GlyphBrush::new(facade, vec![font]);

        // Building the index buffer
        let index_buffer =
            glium::IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &[1u16, 2, 0, 3])
                .unwrap();

        let bg_program = program!(
            facade,
            140 => {
                vertex: "
                    #version 140
                    uniform mat4 matrix;
                    in vec2 position;
                    void main() {
                        gl_Position = matrix * vec4(position, 0.0, 1.0);
                    }
                ",

                fragment: "
                    #version 140
                    out vec4 f_color;
                    void main() {
                        f_color = vec4(0.0, 0.0, 0.0, 0.5);
                    }
                "
            },

            110 => {
                vertex: "
                    #version 110
                    uniform mat4 matrix;
                    attribute vec2 position;
                    void main() {
                        gl_Position = matrix * vec4(position, 0.0, 1.0);
                    }
                ",

                fragment: "
                    #version 110
                    void main() {
                        gl_FragColor = vec4(0.0, 0.0, 0.0, 0.5);
                    }
                ",
            },

            100 => {
                vertex: "
                    #version 100
                    uniform lowp mat4 matrix;
                    attribute lowp vec2 position;
                    void main() {
                        gl_Position = matrix * vec4(position, 0.0, 1.0);
                    }
                ",

                fragment: "
                    #version 100
                    void main() {
                        gl_FragColor = vec4(0.0, 0.0, 0.0, 0.5);
                    }
                    WINIT_UNIX_BACKEND=x11
                ",
            },
        )
        .unwrap();

        let bg_draw_params = DrawParameters {
            blend: Blend::alpha_blending(),
            // blend: Blend {
            //     color: BlendingFunction::Addition {
            //         source: LinearBlendingFactor::SourceAlpha,
            //         destination: LinearBlendingFactor::OneMinusSourceAlpha,
            //     },
            //     alpha: BlendingFunction::Addition {
            //         source: LinearBlendingFactor::SourceAlpha,
            //         destination: LinearBlendingFactor::OneMinusSourceAlpha,
            //     },
            //     constant_value: (0.0, 0.0, 0.0, 0.0),
            // },
            // blend: Blend {
            //     color: BlendingFunction::Addition {
            //         source: LinearBlendingFactor::Zero,
            //         destination: LinearBlendingFactor::Zero,
            //     },
            //     alpha: BlendingFunction::Addition {
            //         source: LinearBlendingFactor::Zero,
            //         destination: LinearBlendingFactor::Zero,
            //     },
            //     constant_value: (0.0, 0.0, 0.0, 0.0),
            // },
            ..Default::default()
        };

        let bg_uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32]
            ],
        };

        StatsRender {
            glyph_brush,
            scale_factor,
            text,
            last_text: String::new(),
            bg_vertex_buffer: None,
            bg_index_buffer: index_buffer,
            bg_program,
            bg_draw_params,
            bg_uniforms,
            bg_last_size: None,
        }
    }

    /// Get a reference to the text that is rendered.
    pub fn text(&self) -> Arc<Mutex<String>> {
        self.text.clone()
    }

    /// Set the text that is rendered.
    pub fn set_text(&mut self, text: String) {
        *self.text.lock() = text;
        self.invalidate_background();
    }

    /// Draw stats text to surface
    ///
    /// Method should be called once for each frame.
    ///
    /// Call `draw_queued` after to actually draw to a surface.
    pub fn queue_draw(&mut self) {
        let text = self.text.lock().clone();
        if text != self.last_text {
            self.invalidate_background();
        }
        if text.trim().is_empty() {
            return;
        }

        let cells = text.lines().map(|row| row.split('\t').collect()).collect();

        let bg_bounds = self.queue_draw_table(cells);
        self.bg_last_size = bg_bounds;

        self.last_text = text;
    }

    /// Queue drawing of stats text using table layout
    fn queue_draw_table(&mut self, cells: Vec<Vec<&str>>) -> Option<Point> {
        if cells.is_empty() {
            return None;
        }

        let sections: Vec<Vec<_>> = cells
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| {
                        Section::default().add_text(
                            Text::new(cell)
                                .with_scale(FONT_SIZE * self.scale_factor as f32)
                                .with_color(WHITE),
                        )
                    })
                    .collect()
            })
            .collect();

        let bounds: Vec<Vec<_>> = sections
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| self.glyph_brush.glyph_bounds(cell).unwrap_or_default())
                    .collect()
            })
            .collect();

        let mut max_x = OrderedFloat(0.0);
        let mut max_y = OrderedFloat(0.0);

        // Queue drawing for each section
        let mut y_offset = OFFSET.1 * self.scale_factor as f32;
        for (row, row_bounds) in sections.into_iter().zip(&bounds) {
            let mut x_offset = OFFSET.0 * self.scale_factor as f32;
            for (i, (cell, cell_bounds)) in row.into_iter().zip(row_bounds).enumerate() {
                self.glyph_brush
                    .queue(cell.with_screen_position((x_offset, y_offset)));

                max_x = max(OrderedFloat(x_offset + cell_bounds.width()), max_x);
                max_y = max(OrderedFloat(y_offset + cell_bounds.height()), max_y);

                let cell_width = bounds
                    .iter()
                    .flat_map(|row| row.get(i))
                    .map(|cell| OrderedFloat(cell.width()))
                    .max()
                    .unwrap_or_default()
                    .0;
                x_offset += cell_width + TABLE_SPACING.0 * self.scale_factor as f32;
            }

            let row_height = row_bounds
                .iter()
                .map(|b| OrderedFloat(b.height()))
                .max()
                .unwrap_or_default()
                .0;
            y_offset += row_height + TABLE_SPACING.1 * self.scale_factor as f32;
        }

        Some(Point {
            x: max_x.0,
            y: max_y.0,
        })
    }

    /// Draw queued to given surface
    pub fn draw_queued<C: Facade + Deref<Target = Context>, S: Surface>(
        &mut self,
        facade: &C,
        surface: &mut S,
    ) {
        self.draw_background(facade, surface);

        self.glyph_brush.draw_queued(facade, surface);
    }

    fn draw_background<C: Facade + Deref<Target = Context>, S: Surface>(
        &mut self,
        facade: &C,
        surface: &mut S,
    ) {
        let dims = surface.get_dimensions();

        let Some(bounds) = self.bg_last_size else {
            return;
        };

        // Calculate background vertex buffer if not set
        if self.bg_vertex_buffer.is_none() {
            let w = bounds.x / dims.0 as f32 * 2f32;
            let h = bounds.y / dims.1 as f32 * 2f32;
            let x = -1f32 + (OFFSET.0 * self.scale_factor as f32) / dims.0 as f32;
            let y = 1f32 - (OFFSET.1 * self.scale_factor as f32) / dims.1 as f32 - h;
            self.bg_vertex_buffer.replace(
                glium::VertexBuffer::new(
                    facade,
                    &[
                        Vertex {
                            position: [x + w, y],
                            tex_coords: [1.0, 1.0],
                        },
                        Vertex {
                            position: [x, y],
                            tex_coords: [0.0, 1.0],
                        },
                        Vertex {
                            position: [x, y + h],
                            tex_coords: [0.0, 0.0],
                        },
                        Vertex {
                            position: [x + w, y + h],
                            tex_coords: [1.0, 0.0],
                        },
                    ],
                )
                .unwrap(),
            );
        }

        surface
            .draw(
                self.bg_vertex_buffer.as_ref().unwrap(),
                &self.bg_index_buffer,
                &self.bg_program,
                &self.bg_uniforms,
                &self.bg_draw_params,
            )
            .unwrap();
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        self.invalidate_background();
    }

    /// Invalidate background and recalculate positioning on next render
    pub fn invalidate_background(&mut self) {
        self.bg_vertex_buffer.take();
    }
}
