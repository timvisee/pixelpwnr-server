use std::fmt;
use std::num::ParseIntError;

/// The default alpha channel value, if not specified. (0xFF = opaque)
const DEFAULT_ALPHA: u8 = 0xFF;

/// Struct representing a color value.
///
/// This color uses 4 channels, for red, green, blue and alpha.
/// Each channel may be a value from 0 to 255.
///
/// Internally, this struct stores the color channels as a single u32 (DWORD)
/// value, which is aligned to 4 bytes in memory. This allows atomic use when
/// directly writing the value in most cases (but not all!).
#[repr(align(4))]
pub struct Color {
    /// Defines the color with a byte for each of the 4 color channels.
    ///
    /// Bytes are ordered as RGBA, little endian.
    value: u32,
}

impl Color {
    /// Construct a new color, from a raw color value.
    ///
    /// This color value defines the value of all 4 color channels.
    pub fn new(value: u32) -> Self {
        Color { value }
    }

    /// Construct a new color, from RGB values.
    ///
    /// The alpha channel will be set to 0xFF.
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Color::from_rgba(r, g, b, DEFAULT_ALPHA)
    }

    /// Construct a new color, from RGBA values.
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color::new(r as u32 | (g as u32) << 8 | (b as u32) << 16 | (a as u32) << 24)
    }

    /// Get the red value, in the range `[0, 255)`.
    pub fn red(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    /// Get green green value, in the range `[0, 255)`.
    pub fn green(&self) -> u8 {
        ((self.value & 0xFF00) >> 8) as u8
    }

    /// Get the blue value, in the range `[0, 255)`.
    pub fn blue(&self) -> u8 {
        ((self.value & 0xFF0000) >> 16) as u8
    }

    /// Get the alpha value, in the range `[0, 255)`.
    pub fn alpha(&self) -> u8 {
        ((self.value & 0xFF000000) >> 24) as u8
    }

    /// Construct a new color, from the given hexadecimal string.
    ///
    /// If parsing the hexadecimal string failed, an error is returned.
    pub fn from_hex(value: &str) -> Result<Self, ParseIntError> {
        // Parse the hexadecimal value
        let mut raw = u32::from_str_radix(value, 16)?;

        // Shift and add an alpha channel, if it wasn't set
        if value.len() <= 6 {
            raw = (raw << 8) | 0xFF;
        }

        // Construct and return the color

        let color = Color::new(raw.to_be());

        Ok(color)
    }

    /// Get the hexadecimal value of the color.
    #[allow(dead_code)]
    pub fn hex(&self) -> String {
        format!("{:06X}", self.value.to_be() >> 8)
    }

    /// A black color, with the default alpha.
    pub fn black() -> Self {
        Color::from_rgb(0, 0, 0)
    }

    /// Get the raw color value, as single u32.
    pub fn to_raw(&self) -> u32 {
        self.value
    }
}

impl fmt::Debug for Color {
    /// Nicely format the color in a human readable RGB(A) format.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Only debug the alpha channel if it isn't the default value
        if self.alpha() == 0 {
            write!(
                f,
                "ColorRGB({}, {}, {})",
                self.red(),
                self.green(),
                self.blue()
            )
        } else {
            write!(
                f,
                "ColorRGBA({}, {}, {}, {})",
                self.red(),
                self.green(),
                self.blue(),
                self.alpha()
            )
        }
    }
}
