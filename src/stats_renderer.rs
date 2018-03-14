extern crate gfx_text;

use std::cmp::max;
use std::iter::Extend;
use std::sync::{Arc, Mutex};

use gfx::{CommandBuffer, Encoder, Factory, Resources};
use gfx::format::RenderFormat;
use gfx::handle::RenderTargetView;
use self::gfx_text::{
    Error as GfxTextError,
    HorizontalAnchor,
    Renderer,
    VerticalAnchor,
};

pub struct StatsRenderer<F: Factory<R>, R: Resources> {
    /// The corner to render the stats in.
    corner: Corner,

    /// The text to render.
    text: Arc<Mutex<String>>,

    /// The text renderer.
    renderer: Option<Renderer<R, F>>,
}

impl<F: Factory<R>, R: Resources> StatsRenderer<F, R> {
    /// Construct a new stats renderer.
    pub fn new(corner: Corner) -> Self {
        StatsRenderer {
            corner,
            text: Arc::new(Mutex::new(String::new())),
            renderer: None,
        }
    }

    /// Initialize the renderer.
    pub fn init(&mut self, factory: F, size: u8) -> Result<(), GfxTextError> {
        // Build the text renderer
        self.renderer = Some(
            gfx_text::new(factory)
                .with_size(size)
                .build()?
        );

        Ok(())
    }

    /// Get a reference to the text that is rendered.
    pub fn text(&self) -> Arc<Mutex<String>> {
        self.text.clone()
    }

    /// Check whether any text is set to render.
    pub fn has_text(&self) -> bool {
        self.text.lock()
            .unwrap()
            .trim()
            .is_empty()
    }

    /// Set the text that is rendered.
    pub fn set_text(&self, text: String) {
        *self.text.lock().unwrap() = text;
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

        // Draw formatted text
        Self::draw_format(
            (10, 10),
            renderer,
            &self.text.lock().unwrap(),
        );

        // Draw the text
        renderer.draw(encoder, target)
    }

    /// Draw text in a formatted way.
    /// This method allows a string to be rendered as table.
    /// Rows are separated by `\n`, while columns are separated by `\t`.
    fn draw_format(
        pos: (u32, u32),
        renderer: &mut Renderer<R, F>,
        text: &str,
    ) {
        Self::draw_table(
            pos,
            renderer,
            text.split("\n")
                .map(|row| row.split("\t").collect())
                .collect(),
        );
    }

    /// Draw a table of text with the given `renderer`.
    /// The text table to draw should be defined in the `text` vectors:
    /// `Rows(Columns)`
    fn draw_table(
        pos: (u32, u32),
        renderer: &mut Renderer<R, F>,
        text: Vec<Vec<&str>>,
    ) {
        // Build a table of text bounds
        let bounds: Vec<Vec<(i32, i32)>> = text.iter()
            .map(|col| col.iter()
                .map(|text| renderer.measure(text))
                .collect()
            ).collect();

        // Find the maximum height for each row
        let rows_max: Vec<i32> = bounds.iter()
            .map(|col| col.iter()
                 .map(|size| size.1)
                 .max()
                 .unwrap_or(0)
            ).collect();

        // Find the maximum width for each column
        let cols_max: Vec<i32> = bounds.iter()
            .map(|row| row.iter().map(|size| size.0).collect())
            .fold(Vec::new(), |acc: Vec<i32>, row: Vec<i32>| {
                // Iterate over widths in acc and row,
                // select the largest one
                let mut acc: Vec<i32> = acc.iter()
                    .zip(row.iter())
                    .map(|(a, b)| max(*a, *b))
                    .collect();

                // If there were additional widths in row, just add them
                let acc_len = acc.len();
                if acc_len < row.len() {
                    acc.extend(row.iter().skip(acc_len));
                }

                acc
            });

        // Render each text
        for (row, text) in text.iter().enumerate() {
            for (col, text) in text.iter().enumerate() {
                // Find the coordinate to use
                let (mut x, mut y): (i32, i32) = (
                    cols_max.iter().take(col).sum::<i32>(),
                    rows_max.iter().take(row).sum::<i32>(),
                );

                // Add the offset and additional spacing
                x += pos.0 as i32 + 20i32 * col as i32;
                y += pos.1 as i32;

                // Render the text
                renderer.add_anchored(
                    text,
                    [x, y],
                    HorizontalAnchor::Left, VerticalAnchor::Top,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
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
    BottomRight
}
