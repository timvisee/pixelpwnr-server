use draw_state::state::{Blend, BlendChannel, BlendValue, Equation, Factor};
use gfx_glutin::WindowUpdateExt;
use glutin::{PossiblyCurrent, WindowedContext};
use parking_lot::Mutex;
use std::cmp::max;
use std::iter::Extend;
use std::sync::Arc;

use gfx::format::RenderFormat;
use gfx::handle::{DepthStencilView, RenderTargetView};
use gfx::traits::FactoryExt;
use gfx::{self, *};

use gfx_text::{Error as GfxTextError, HorizontalAnchor, Renderer as TextRenderer, VerticalAnchor};
use old_school_gfx_glutin_ext as gfx_glutin;

use super::ref_values::RefValuesWrapper;
use crate::primitive::create_quad;
use crate::renderer::{ColorFormat, DepthFormat, R};
use crate::vertex::*;

/// White color definition with 4 channels.
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const BLEND: Blend = Blend {
    color: BlendChannel {
        equation: Equation::Add,
        source: Factor::Zero,
        destination: Factor::ZeroPlus(BlendValue::ConstAlpha),
    },
    alpha: BlendChannel {
        equation: Equation::Add,
        source: Factor::Zero,
        destination: Factor::Zero,
    },
};

// Screen shader data pipeline
gfx_defines! {
    pipeline bg_pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::BlendTarget<ColorFormat> = (
            "Target0",
            gfx::state::ColorMask::all(),
            BLEND,
        ),
        ref_values: RefValuesWrapper = RefValuesWrapper::new(),
    }
}

pub struct StatsRenderer<F: Factory<R> + Clone> {
    /// The corner to render the stats in.
    #[allow(unused)]
    corner: Corner,

    /// The rendering offset.
    offset: (u32, u32),

    /// The rendering padding.
    padding: i32,

    /// The column spacing amount.
    col_spacing: i32,

    /// The text to render.
    text: Arc<Mutex<String>>,

    /// The text renderer.
    renderer: Option<TextRenderer<R, F>>,

    /// A factory to build new model instances if required.
    factory: Option<F>,

    /// The dimensions the rendering window has, used for text placement.
    window_dimensions: Option<(f32, f32)>,

    /// The depth stencil for background rendering.
    bg_depth: Option<DepthStencilView<R, DepthFormat>>,

    /// The PSO for background rendering.
    bg_pso: Option<PipelineState<R, bg_pipe::Meta>>,

    /// The vertex slice for the background quad.
    bg_slice: Option<Slice<R>>,

    /// The background rendering data.
    bg_data: Option<bg_pipe::Data<R>>,
}

impl<F: Factory<R> + Clone> StatsRenderer<F> {
    /// Construct a new stats renderer.
    pub fn new(corner: Corner) -> Self {
        StatsRenderer {
            corner,
            offset: (0, 0),
            padding: 0,
            col_spacing: 0,
            text: Arc::new(Mutex::new(String::new())),
            renderer: None,
            factory: None,
            window_dimensions: None,
            bg_depth: None,
            bg_pso: None,
            bg_slice: None,
            bg_data: None,
        }
    }

    /// Initialize the renderer.
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        &mut self,
        mut factory: F,
        window_dimensions: (f32, f32),
        main_color: RenderTargetView<R, ColorFormat>,
        main_depth: DepthStencilView<R, DepthFormat>,
        font_size: u8,
        offset: (u32, u32),
        padding: i32,
        col_spacing: i32,
    ) -> Result<(), GfxTextError> {
        // Set the window dimensions, offset and padding
        self.window_dimensions = Some(window_dimensions);
        self.offset = offset;
        self.padding = padding;
        self.col_spacing = col_spacing;

        // Build the text renderer
        self.renderer = Some(
            gfx_text::new(factory.clone())
                .with_size(font_size)
                .build()?,
        );

        // Create a shader pipeline for the stats background
        self.bg_pso = Some(
            factory
                .create_pipeline_simple(
                    include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/shaders/stats_bg.glslv"
                    )),
                    include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/shaders/stats_bg.glslf"
                    )),
                    bg_pipe::new(),
                )
                .unwrap(),
        );

        // Create a background plane
        let bg_plane = create_quad((-1f32, 0f32), (0.2f32, 0.95f32));
        let (vertex_buffer, slice) = bg_plane.create_vertex_buffer(&mut factory);

        // Store the slice, and build the background pipe data
        self.bg_slice = Some(slice);
        self.bg_data = Some(bg_pipe::Data {
            vbuf: vertex_buffer,
            out: main_color,
            ref_values: (),
        });

        // Set the factory and depth stencil
        self.factory = Some(factory);
        self.bg_depth = Some(main_depth);

        Ok(())
    }

    /// Get a reference to the text that is rendered.
    pub fn text(&self) -> Arc<Mutex<String>> {
        self.text.clone()
    }

    /// Check whether any text is set to render.
    pub fn has_text(&self) -> bool {
        self.text.lock().trim().is_empty()
    }

    /// Set the text that is rendered.
    pub fn set_text(&self, text: String) {
        *self.text.lock() = text;
    }

    /// Draw the renderer to the given context.
    ///
    /// This method should be called once each render loop iteration,
    /// to properly draw the stats.
    pub fn draw<C: CommandBuffer<R>, T: RenderFormat>(
        &mut self,
        encoder: &mut Encoder<R, C>,
        target: &RenderTargetView<R, T>,
    ) -> Result<(), GfxTextError> {
        // Do not draw if no renderer is available yet,
        // or if there is no text to draw
        if self.renderer.is_none() || self.has_text() {
            return Ok(());
        }

        // Unwrap the renderer
        let renderer = self.renderer.as_mut().unwrap();

        // Draw formatted text on the text scene
        let bounds = Self::scene_draw_format(
            self.offset,
            self.padding,
            self.col_spacing,
            renderer,
            &self.text.lock(),
        );

        // Draw the background quad, if there are some bounds
        if bounds != (0f32, 0f32)
            && self.bg_slice.is_some()
            && self.bg_pso.is_some()
            && self.bg_data.is_some()
        {
            // Get the window dimensions
            let win = self.window_dimensions.unwrap();

            // Determine the position and size of the background quad
            let w = bounds.0 / win.0 * 2f32;
            let h = bounds.1 / win.1 * 2f32;
            let x = -1f32 + self.offset.0 as f32 / win.0 * 2f32;
            let y = 1f32 - self.offset.1 as f32 / win.1 * 2f32 - h;

            // Rebuild the vertex buffer and slice data
            let (vertex_buffer, slice) =
                create_quad((x, y), (w, h)).create_vertex_buffer(self.factory.as_mut().unwrap());

            *self.bg_slice.as_mut().unwrap() = slice;
            self.bg_data.as_mut().unwrap().vbuf = vertex_buffer;

            encoder.draw(
                self.bg_slice.as_ref().unwrap(),
                self.bg_pso.as_ref().unwrap(),
                self.bg_data.as_ref().unwrap(),
            );
        }

        // Draw the text scene
        renderer.draw(encoder, target)
    }

    /// Draw text in a formatted way.
    /// This method allows a string to be rendered as table.
    /// Rows are separated by `\n`, while columns are separated by `\t`.
    ///
    /// The drawing bounds are returned.
    fn scene_draw_format(
        pos: (u32, u32),
        padding: i32,
        col_spacing: i32,
        renderer: &mut TextRenderer<R, F>,
        text: &str,
    ) -> (f32, f32) {
        Self::scene_draw_table(
            pos,
            padding,
            col_spacing,
            renderer,
            text.split("\n")
                .map(|row| row.split("\t").collect())
                .collect(),
        )
    }

    /// Draw a table of text with the given `renderer`.
    /// The text table to draw should be defined in the `text` vectors:
    /// `Rows(Columns)`
    ///
    /// The drawing bounds are returned.
    fn scene_draw_table(
        pos: (u32, u32),
        padding: i32,
        col_spacing: i32,
        renderer: &mut TextRenderer<R, F>,
        text: Vec<Vec<&str>>,
    ) -> (f32, f32) {
        // Build a table of text bounds
        let bounds: Vec<Vec<(i32, i32)>> = text
            .iter()
            .map(|col| col.iter().map(|text| renderer.measure(text)).collect())
            .collect();

        // Find the maximum height for each row
        let rows_max: Vec<i32> = bounds
            .iter()
            .map(|col| col.iter().map(|size| size.1).max().unwrap_or(0))
            .collect();

        // Find the maximum width for each column
        let mut cols_max: Vec<i32> = bounds
            .iter()
            .map(|row| row.iter().map(|size| size.0).collect())
            .fold(Vec::new(), |acc: Vec<i32>, row: Vec<i32>| {
                // Iterate over widths in acc and row,
                // select the largest one
                let mut out: Vec<i32> = acc
                    .iter()
                    .zip(row.iter())
                    .map(|(a, b)| max(*a, *b))
                    .collect();

                // Extend the output if there are any widths left
                let out_len = out.len();
                if out_len < acc.len() || out_len < row.len() {
                    out.extend(acc.iter().skip(out_len));
                    out.extend(row.iter().skip(out_len));
                }

                out
            });
        cols_max
            .iter_mut()
            .rev()
            .skip(1)
            .map(|width| *width += col_spacing)
            .count();

        // Render each text
        for (row, text) in text.iter().enumerate() {
            for (col, text) in text.iter().enumerate() {
                // Find the coordinate to use
                let (mut x, mut y): (i32, i32) = (
                    cols_max.iter().take(col).sum::<i32>(),
                    rows_max.iter().take(row).sum::<i32>(),
                );

                // Add the offset and additional spacing
                x += pos.0 as i32 + padding;
                y += pos.1 as i32 + padding;

                // Render the text
                renderer.add_anchored(
                    text,
                    [x, y],
                    HorizontalAnchor::Left,
                    VerticalAnchor::Top,
                    WHITE,
                );
            }
        }

        // Find the total width and height, return it
        (
            cols_max.iter().sum::<i32>() as f32 + padding as f32 * 2f32,
            rows_max.iter().sum::<i32>() as f32 + padding as f32 * 2f32,
        )
    }

    /// Update the stats rendering view, and the window dimensions.
    /// This should be called when the GL rendering window is resized.
    // TODO: also update the text view here
    pub fn update_views(
        &mut self,
        window: &WindowedContext<PossiblyCurrent>,
        dimensions: (f32, f32),
    ) {
        // Update the views
        if let Some(data) = self.bg_data.as_mut() {
            window.update_gfx(&mut data.out, self.bg_depth.as_mut().unwrap());
        }

        // Update the window dimensions
        self.window_dimensions = Some(dimensions);
    }
}

/// The corner to render stats in.
pub enum Corner {
    /// The top left corner of the screen.
    TopLeft,

    /// The top right corner of the screen.
    TopRight,

    /// The bottom left corner of the screen.
    BottomLeft,

    /// The bottom right corner of the screen.
    BottomRight,
}
