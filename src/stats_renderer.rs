extern crate gfx_text;

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
        let mut renderer = self.renderer.as_mut().unwrap();

        // TODO: draw a background box

        // Build up the text renderer
        // TODO: don't unwrap
        renderer.add_anchored(
            &self.text.lock().unwrap(),
            [10, 10],
            HorizontalAnchor::Left, VerticalAnchor::Top,
            [1.0, 1.0, 1.0, 1.0],
        );

        // Draw the text
        renderer.draw(encoder, target)
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
