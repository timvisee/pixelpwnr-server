use glium::backend::{Context, Facade};
use glium::Surface;
use glium_glyph::glyph_brush::ab_glyph::FontRef;
use glium_glyph::glyph_brush::{GlyphCruncher, Section, Text};
use glium_glyph::GlyphBrush;
use ordered_float::OrderedFloat;
use parking_lot::Mutex;
use std::ops::Deref;
use std::sync::Arc;

const FONT_BYTES: &[u8] = include_bytes!("../../fonts/DejaVuSans-2.37.ttf");

/// White color definition with 4 channels.
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const FONT_SCALE: f32 = 40.0;

const COL_SPACING: f32 = 40.0;
const OFFSET: (f32, f32) = (40.0, 40.0);

pub struct StatsRender {
    /// Font file used for rendering
    font: FontRef<'static>,

    /// The rendering offset.
    offset: (u32, u32),

    /// The rendering padding.
    padding: i32,

    /// The column spacing amount.
    col_spacing: i32,

    /// The text to render.
    text: Arc<Mutex<String>>,

    /// The dimensions the rendering window has, used for text placement.
    window_dimensions: Option<(f32, f32)>,
}

// impl<F: Factory<R> + Clone> StatsRenderer<F> {
impl StatsRender {
    /// Construct a new stats renderer.
    pub fn new(text: Arc<Mutex<String>>) -> Self {
        let font = FontRef::try_from_slice(FONT_BYTES).unwrap();

        StatsRender {
            font,
            offset: (0, 0),
            padding: 0,
            col_spacing: 0,
            text,
            window_dimensions: None,
        }
    }

    /// Initialize the renderer.
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        &mut self,
        window_dimensions: (f32, f32),
        offset: (u32, u32),
        padding: i32,
        col_spacing: i32,
    ) {
        // Set the window dimensions, offset and padding
        self.window_dimensions = Some(window_dimensions);
        self.offset = offset;
        self.padding = padding;
        self.col_spacing = col_spacing;

        // // Build the text renderer
        // self.renderer = Some(
        //     gfx_text::new(factory.clone())
        //         .with_size(font_size)
        //         .build()?,
        // );

        // // Create a shader pipeline for the stats background
        // self.bg_pso = Some(
        //     factory
        //         .create_pipeline_simple(
        //             include_bytes!(concat!(
        //                 env!("CARGO_MANIFEST_DIR"),
        //                 "/shaders/stats_bg.glslv"
        //             )),
        //             include_bytes!(concat!(
        //                 env!("CARGO_MANIFEST_DIR"),
        //                 "/shaders/stats_bg.glslf"
        //             )),
        //             bg_pipe::new(),
        //         )
        //         .unwrap(),
        // );

        // // Create a background plane
        // let bg_plane = create_quad((-1f32, 0f32), (0.2f32, 0.95f32));
        // let (vertex_buffer, slice) = bg_plane.create_vertex_buffer(&mut factory);

        // // Store the slice, and build the background pipe data
        // self.bg_slice = Some(slice);
        // self.bg_data = Some(bg_pipe::Data {
        //     vbuf: vertex_buffer,
        //     out: main_color,
        //     ref_values: (),
        // });

        // // Set the factory and depth stencil
        // self.factory = Some(factory);
        // self.bg_depth = Some(main_depth);
    }

    /// Get a reference to the text that is rendered.
    pub fn text(&self) -> Arc<Mutex<String>> {
        self.text.clone()
    }

    /// Set the text that is rendered.
    pub fn set_text(&self, text: String) {
        *self.text.lock() = text;
    }

    /// Draw stats text to surface
    ///
    /// Method should be called once for each frame.
    pub fn draw<C: Facade + Deref<Target = Context>, S: Surface>(
        &self,
        facade: &C,
        surface: &mut S,
    ) {
        let text = self.text.lock().clone();
        if text.trim().is_empty() {
            return;
        }

        let cells = text.lines().map(|row| row.split('\t').collect()).collect();

        self.draw_table(facade, surface, cells);
    }

    /// Draw stats text to surface using table layout
    pub fn draw_table<C: Facade + Deref<Target = Context>, S: Surface>(
        &self,
        facade: &C,
        surface: &mut S,
        cells: Vec<Vec<&str>>,
    ) {
        if cells.is_empty() {
            return;
        }

        // TODO: don't recreate this every frame?
        let mut glyph_brush = GlyphBrush::new(facade, vec![&self.font]);

        let sections: Vec<Vec<_>> = cells
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| {
                        Section::default().add_text(
                            Text::new(cell)
                                .with_scale(FONT_SCALE) // Font size
                                .with_color(WHITE), // White
                        )
                    })
                    .collect()
            })
            .collect();

        let bounds: Vec<Vec<_>> = sections
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| glyph_brush.glyph_bounds(cell).unwrap_or_default())
                    .collect()
            })
            .collect();

        // Queue drawing for each section
        let mut y_offset = OFFSET.1;
        for (row, row_bounds) in sections.into_iter().zip(&bounds) {
            let mut x_offset = OFFSET.0;
            for (i, cell) in row.into_iter().enumerate() {
                glyph_brush.queue(cell.with_screen_position((x_offset, y_offset)));

                let cell_width = bounds
                    .iter()
                    .flat_map(|row| row.get(i))
                    .map(|cell| OrderedFloat(cell.width()))
                    .max()
                    .unwrap_or_default()
                    .0;
                x_offset += cell_width + COL_SPACING;
            }

            let row_height = row_bounds
                .iter()
                .map(|b| OrderedFloat(b.height()))
                .max()
                .unwrap_or_default()
                .0;
            y_offset += row_height;
        }

        glyph_brush.draw_queued(facade, surface);
    }

    /// Update the stats rendering view, and the window dimensions.
    /// This should be called when the GL rendering window is resized.
    // TODO: also update the text view here
    pub fn update_views(
        &mut self,
        // window: &WindowedContext<PossiblyCurrent>,
        dimensions: (f32, f32),
    ) {
        // // Update the views
        // if let Some(data) = self.bg_data.as_mut() {
        //     window.update_gfx(&mut data.out, self.bg_depth.as_mut().unwrap());
        // }

        // Update the window dimensions
        self.window_dimensions = Some(dimensions);
    }
}
