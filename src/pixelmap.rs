use std::mem;

use super::color::Color;

// Align to at least 4 bytes in memory, which aligns the i32 map
// This greatly improves the chance of raw i32 operations being atomic,
// which is preferred bahaviour for this data structure.
#[repr(align(4))]
pub struct Pixelmap {
    map: Vec<u32>,

    width: usize,
    channels: usize,
}

impl Pixelmap {
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Pixelmap {
            map: vec![0x000000FFu32.to_be(); width * height],
            width,
            channels,
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        self.set_pixel_raw(x, y, color.to_raw());
    }

    pub fn set_pixel_raw(&mut self, x: usize, y: usize, raw: u32) {
        // Determine the pixel index
        let index = self.pixel_index(x, y);

        // Set the value
        self.map[index] = raw;
    }

    fn pixel_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            mem::transmute(self.map.as_slice())
        }
    }
}

unsafe impl Send for Pixelmap {}
unsafe impl Sync for Pixelmap {}
