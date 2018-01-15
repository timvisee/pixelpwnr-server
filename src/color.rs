use std::num::ParseIntError;

pub struct Color {
    value: u32,
}

impl Color {
    pub fn new(value: u32) -> Self {
        Color {
            value,
        }
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        // TODO: use a default alpha from a constant
        Color::from_rgba(r, g, b, 0xFF)
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color::new(
            (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | a as u32
        )
    }

    pub fn from_hex(value: &str) -> Result<Self, ParseIntError> {
        u32::from_str_radix(value, 16)
            .map(|raw| Color::new(raw))
    }

    pub fn to_raw(&self) -> u32 {
        self.value
    }
}
