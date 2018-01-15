pub struct Pixelmap {
    map: Vec<u8>,
    width: usize,
    channels: usize,
}

impl Pixelmap {
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Pixelmap {
            map: vec![255u8; width * height * channels],
            width,
            channels,
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        let offset = self.pixel_offset(x, y);

        self.map[offset] = r;
        self.map[offset + 1] = g;
        self.map[offset + 2] = b;
    }

    fn pixel_offset(&self, x: usize, y: usize) -> usize {
        (y * self.width + x) * self.channels
    }

    pub fn render_data(&self) -> &[u8] {
        &self.map
    }
}
