use std::fmt;
use std::num::ParseIntError;

/// The default alpha channel value, if not specified. (0xFF = opaque)
const DEFAULT_ALPHA: u8 = 0xFF;

#[derive(Debug, Clone, Copy)]
pub enum ParseColorError {
    /// 6 or 8 characters are required
    /// Value is the actual amount
    InvalidCharCount(usize),
    /// An invalid character was encountered
    InvalidChar(u8),
}

/// Struct representing a color value.
///
/// This color uses 4 channels, for red, green, blue and alpha.
/// Each channel may be a value from 0 to 255.
///
/// Internally, this struct stores the color channels as a single u32 (DWORD)
/// value, which is aligned to 4 bytes in memory. This allows atomic use when
/// directly writing the value in most cases (but not all!).
#[repr(align(4))]
#[derive(PartialEq, Clone, Copy)]
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
    pub fn red(&self) -> u32 {
        self.value & 0xFF
    }

    /// Get green green value, in the range `[0, 255)`.
    pub fn green(&self) -> u32 {
        (self.value & 0xFF00) >> 8
    }

    /// Get the blue value, in the range `[0, 255)`.
    pub fn blue(&self) -> u32 {
        (self.value & 0xFF0000) >> 16
    }

    /// Get the alpha value, in the range `[0, 255)`.
    pub fn alpha(&self) -> u32 {
        (self.value & 0xFF000000) >> 24
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

    /// Construct a new color, from the given slice.
    /// The slice should represent hexadecimal characters as ASCII characters,
    /// meaning that they should be between b'0' and b'9', between b'a' and b'f', or
    /// between b'A' and b'F'
    pub fn from_hex_raw(value: &[u8]) -> Result<Self, ParseColorError> {
        let len = value.len();

        /// This always returns a value 0 <= v <= 15
        fn parse_char(input: u8) -> Result<u8, ParseColorError> {
            if input >= b'a' && input <= b'f' {
                Ok(input - b'a' + 10)
            } else if input >= b'A' && input <= b'F' {
                Ok(input - b'A' + 10)
            } else if input >= b'0' && input <= b'9' {
                Ok(input - b'0')
            } else {
                Err(ParseColorError::InvalidChar(input))
            }
        }

        let build = || {
            let mut raw_u32 = 0u32;
            for char in value.iter() {
                raw_u32 <<= 4;
                raw_u32 |= parse_char(*char)? as u32;
            }
            Ok(raw_u32)
        };

        if len == 6 {
            let mut value = build()?;
            // No Alpha byte
            value = (value << 8) | 0xFF;
            Ok(Color {
                value: value.to_be(),
            })
        } else if len == 8 {
            let value = build()?;
            Ok(Color {
                value: value.to_be(),
            })
        } else {
            Err(ParseColorError::InvalidCharCount(len))
        }
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

    /// Blend this color with another
    ///
    /// Self should be the current value, and `other` should be the incoming value
    pub fn blend(&mut self, other: Color) {
        // Self = destination = ptr
        // Other = source = rgba

        let mut r = other.red();
        let mut g = other.green();
        let mut b = other.blue();
        let mut a = other.alpha();

        if a == 0 {
            return;
        } else if a < u8::MAX as u32 {
            let na = u8::MAX as u32 - a;
            r = ((a * r) + (na * self.red())) / 0xFF;
            g = ((a * g) + (na * self.green())) / 0xFF;
            b = ((a * b) + (na * self.blue())) / 0xFF;
            a = a + self.alpha();
        }
        self.value = r & 0xFF | (g & 0xFF) << 8 | (b & 0xFF) << 16 | (a & 0xFF) << 24;
    }
}

impl fmt::Debug for Color {
    /// Nicely format the color in a human readable RGB(A) format.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Only debug the alpha channel if it isn't the default value
        if self.alpha() == 0 {
            write!(
                f,
                "ColorRGB({:X}, {:X}, {:X})",
                self.red(),
                self.green(),
                self.blue()
            )
        } else {
            write!(
                f,
                "ColorRGBA({:X}, {:X}, {:X}, {:X})",
                self.red(),
                self.green(),
                self.blue(),
                self.alpha()
            )
        }
    }
}

#[test]
fn from_hex_raw() {
    macro_rules! test {
        ($in: literal, $out: expr, $print: literal) => {
            let color_raw = Color::from_hex_raw($in.as_bytes()).unwrap();
            let color = Color::from_hex($in).unwrap();
            assert_eq!(color, color_raw);
            assert_eq!(Color::new($out), color_raw);
            assert_eq!(format!("{:?}", color_raw), $print);
        };
    }

    test!("ABCDEFBA", 0xBAEFCDAB, "ColorRGBA(AB, CD, EF, BA)");
    test!("AABBCC", 0xFFCCBBAA, "ColorRGBA(AA, BB, CC, FF)");
    test!("ABCDEF00", 0x00EFCDAB, "ColorRGB(AB, CD, EF)");
}
