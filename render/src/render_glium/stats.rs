use glium::backend::{Context, Facade};
use glium::Surface;
use glium_glyph::glyph_brush::ab_glyph::FontArc;
use glium_glyph::glyph_brush::{GlyphCruncher, Section, Text};
use glium_glyph::GlyphBrush;
use ordered_float::OrderedFloat;
use parking_lot::Mutex;
use std::ops::Deref;
use std::sync::Arc;

const FONT_BYTES: &[u8] = include_bytes!("../../fonts/DejaVuSans-2.37.ttf");

const FONT_SCALE: f32 = 40.0;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const TABLE_SPACING: (f32, f32) = (40.0, 10.0);
const OFFSET: (f32, f32) = (40.0, 40.0);

pub struct StatsRender {
    /// Glyph brush used for drawing text
    glyph_brush: GlyphBrush<'static, FontArc>,

    /// Text buffer to render.
    text: Arc<Mutex<String>>,
}

impl StatsRender {
    /// Construct a new stats renderer
    pub fn new<C: Facade + Deref<Target = Context>>(text: Arc<Mutex<String>>, facade: &C) -> Self {
        let font = FontArc::try_from_slice(FONT_BYTES).unwrap();
        let glyph_brush = GlyphBrush::new(facade, vec![font]);

        StatsRender { glyph_brush, text }
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
    ///
    /// Call `draw_queued` after to actually draw to a surface.
    pub fn queue_draw(&mut self) {
        let text = self.text.lock().clone();
        if text.trim().is_empty() {
            return;
        }

        let cells = text.lines().map(|row| row.split('\t').collect()).collect();

        self.queue_draw_table(cells);
    }

    /// Queue drawing of stats text using table layout
    fn queue_draw_table(&mut self, cells: Vec<Vec<&str>>) {
        if cells.is_empty() {
            return;
        }

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
                    .map(|cell| self.glyph_brush.glyph_bounds(cell).unwrap_or_default())
                    .collect()
            })
            .collect();

        // Queue drawing for each section
        let mut y_offset = OFFSET.1;
        for (row, row_bounds) in sections.into_iter().zip(&bounds) {
            let mut x_offset = OFFSET.0;
            for (i, cell) in row.into_iter().enumerate() {
                self.glyph_brush
                    .queue(cell.with_screen_position((x_offset, y_offset)));

                let cell_width = bounds
                    .iter()
                    .flat_map(|row| row.get(i))
                    .map(|cell| OrderedFloat(cell.width()))
                    .max()
                    .unwrap_or_default()
                    .0;
                x_offset += cell_width + TABLE_SPACING.0;
            }

            let row_height = row_bounds
                .iter()
                .map(|b| OrderedFloat(b.height()))
                .max()
                .unwrap_or_default()
                .0;
            y_offset += row_height + TABLE_SPACING.1;
        }
    }

    /// Draw queued to given surface
    pub fn draw_queued<C: Facade + Deref<Target = Context>, S: Surface>(
        &mut self,
        facade: &C,
        surface: &mut S,
    ) {
        self.glyph_brush.draw_queued(facade, surface);
    }
}
