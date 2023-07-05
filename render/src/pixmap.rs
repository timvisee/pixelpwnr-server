use std::sync::atomic::{AtomicU32, Ordering};

use crate::color::Color;

/// A struct representing a pixelmap for pixelflut.
///
/// This struct holds the data for each pixel, and can be concidered a bitmap.
/// For each pixel, a u32 (DWORD) value is used containing 4 bytes that define
/// the value for each of the 4 color channels.
///
/// This data structure is focussed on performance and multithreaded use with
/// multiple readers and writers.  This structure does not use any kind of
/// locks. Instead, it is assumed that the operations done on the internal map
/// are atomic (on a pixel basis).  This is perfectly fine for what this
/// pixelmap is used for.
///
/// Because this structure is aligned to 4 bytes in memory, each raw color
/// value (u32) is also aligned to 4 bytes. This makes direct reads and writes
/// on these values on most CPUs (but not all!). The fact that this may not be
/// atomic in some cases is accepted for this structure. The speed of not using
/// locks is preferred over the minor side effect of seldom rendering artifact
/// on some systems.
///
/// More info: https://stackoverflow.com/a/5002256/1000145
///
/// Important: this data structure is considered unsafe, but is perfectly
/// usable for pixelflut applications.
#[repr(align(4))]
pub struct Pixmap {
    /// A map with a raw color value for each pixel in the map, where each
    /// pixel consists of 4 bytes in a single u32 for each color channel.
    map: Vec<AtomicU32>,

    /// Pixelmap dimensions, width and height
    dimensions: (usize, usize),
}

impl Clone for Pixmap {
    fn clone(&self) -> Self {
        let map = self
            .map
            .iter()
            .map(|v| AtomicU32::new(v.load(Ordering::Relaxed)))
            .collect();

        Self {
            map,
            dimensions: self.dimensions.clone(),
        }
    }
}

impl Pixmap {
    const DEFAULT_PIXEL: u32 = Color::black().to_raw();

    /// Construct a new
    pub fn new(width: usize, height: usize) -> Self {
        Pixmap {
            // Build a pixel map, with the default value and the proper sizeto
            // fit each pixel
            map: (0..width * height)
                .map(|_| AtomicU32::new(Self::DEFAULT_PIXEL))
                .collect(),

            // Set the dimensions
            dimensions: (width, height),
        }
    }

    /// Get the width of the pixel map.
    pub fn width(&self) -> usize {
        self.dimensions.0
    }

    /// Get the height of the pixel map.
    pub fn height(&self) -> usize {
        self.dimensions.1
    }

    /// Get the dimensions of the pixel map.
    #[allow(dead_code)]
    pub fn dimensions(&self) -> (usize, usize) {
        self.dimensions
    }

    /// Get the pixel at the given coordinate, as color.
    #[allow(dead_code)]
    pub fn pixel(&self, x: usize, y: usize) -> Result<Color, PixmapErr> {
        let pixel_index = self.pixel_index(x, y)?;
        let pixel_value = self.map[pixel_index].load(Ordering::Relaxed);
        Ok(Color::new(pixel_value))
    }

    /// Set the pixel at the given coordinate, to the given color.
    pub fn set_pixel(&self, x: usize, y: usize, color: Color) -> Result<(), PixmapErr> {
        let pixel_index = self.pixel_index(x, y)?;

        let mut current_color = Color::new(self.map[pixel_index].load(Ordering::Relaxed));
        current_color.blend(color);
        self.map[pixel_index].store(current_color.to_raw(), Ordering::Relaxed);
        Ok(())
    }

    /// Get the index a pixel is at, for the given coordinate.
    fn pixel_index(&self, x: usize, y: usize) -> Result<usize, PixmapErr> {
        // Check pixel bounds
        if x >= self.dimensions.0 {
            return Err(PixmapErr::OutOfBound("x coordinate out of bound"));
        } else if y >= self.dimensions.1 {
            return Err(PixmapErr::OutOfBound("y coordinate out of bound"));
        }

        // Determine the index and return
        Ok(y * self.dimensions.0 + x)
    }

    /// Get the pixelmap data, as a slice of bytes.
    ///
    /// Each pixel consumes a sequence of 4 bytes, each defining the value of
    /// a different color channel.
    ///
    /// This data may be used to send to the GPU, as raw texture buffer, for
    /// rendering.
    pub fn as_bytes<'me>(&'me mut self) -> &[u8] {
        let map = &self.map;

        let len = map.len() * 4;

        // We get a pointer to the start of the U32 list
        //
        // Casting *const AtomicU32 to *const u32 is OK because Atomicu32
        // has the same in-memory representation as u32
        let ptr = map.as_ptr() as *const u32 as *const u8;

        // We create the slice from the pointer
        //
        // # SAFETY
        // `ptr` points to `len` u32s, and that has the same size as
        // `(mem::size_of::<u32>()/mem::size_of::<u8>) * len` = `4 * len`, so
        // turning it into a &[u8] of that length is valid.
        //
        // A correctly aligned [u32] will most likely also constitute a correctly
        // aligned [u8]
        //
        // Because we are borrowing `self` for 'me (by means of a mutable borrow),
        // we can safely create an immutable slice of the memory that we're
        // pointing to that for 'me.
        let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
        slice
    }
}

unsafe impl Send for Pixmap {}
unsafe impl Sync for Pixmap {}

/// An error representation for pixel map operations.
#[derive(Debug)]
pub enum PixmapErr<'a> {
    /// The given pixel coordinate or index is out of bound.
    OutOfBound(&'a str),
}
