use std::mem;

use super::color::Color;

lazy_static! {
    /// The default color value for each pixel
    static ref DEFAULT_PIXEL: u32 = 0x000000FFu32.to_be();
}

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
pub struct Pixelmap {
    /// A map with a raw color value for each pixel in the map, where each
    /// pixel consists of 4 bytes in a single u32 for each color channel.
    map: Vec<u32>,

    /// The width of the pixelmap.
    width: usize,
}

impl Pixelmap {
    /// Construct a new 
    pub fn new(width: usize, height: usize) -> Self {
        Pixelmap {
            // Build a pixel map, with the default value and the proper sizeto
            // fit each pixel
            map: vec![*DEFAULT_PIXEL; width * height],

            // The width of the pixelmap
            width,
        }
    }

    /// Set the pixel at the given coordinate, to the given color.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        self.set_pixel_raw(x, y, color.to_raw());
    }

    /// Set the pixel at the given coordinate, to the given raw color value.
    pub fn set_pixel_raw(&mut self, x: usize, y: usize, raw: u32) {
        // Determine the pixel index
        let index = self.pixel_index(x, y);

        // Set the value
        self.map[index] = raw;
    }

    /// Get the index a pixel is at, for the given coordinate.
    fn pixel_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Get the pixelmap data, as slice with the raw color value of each
    /// pixel.
    ///
    /// Note: this method returns a single u32 for each pixel, instead of 4
    /// u8 bytes for each pixel as the `as_bytes()` method does.
    pub fn as_slice(&self) -> &[u32] {
        self.map.as_slice()
    }

    /// Get the pixelmap data, as a slice of bytes.
    ///
    /// Each pixel consumes a sequence of 4 bytes, each defining the value of
    /// a different color channel.
    ///
    /// This data may be used to send to the GPU, as raw texture buffer, for
    /// rendering.
    pub fn as_bytes(&self) -> &[u8] {
        // The following code transmutes the raw slice bytes from the
        // `[u32; size]` type into `[u8; size * 4]`. Cloning the data array
        // and casting each raw value to 4 u8 bytes if a very expensive
        // operation to do each frame for such a big array of pixels.
        // Transmuting is considered unsafe, but usually is about a 1000 times
        // faster resulting in insane performance gains. This unsafe bit of
        // code is desirable over safe code that is enormously slower.
        // The implementation below is memory safe.
        unsafe {
            mem::transmute(self.as_slice())
        }
    }
}

unsafe impl Send for Pixelmap {}
unsafe impl Sync for Pixelmap {}
