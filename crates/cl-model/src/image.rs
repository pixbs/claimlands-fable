/// A CPU-side RGBA8 texture, row-major, top row first (canvas order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub data: Vec<u8>,
}

impl RgbaImage {
    /// Transparent black image.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    fn offset(&self, x: u32, y: u32) -> usize {
        assert!(
            x < self.width && y < self.height,
            "texel ({x},{y}) outside {}x{}",
            self.width,
            self.height
        );
        ((y as usize) * (self.width as usize) + (x as usize)) * 4
    }

    /// Write one texel.
    pub fn put(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let o = self.offset(x, y);
        self.data[o..o + 4].copy_from_slice(&rgba);
    }

    /// Read one texel.
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let o = self.offset(x, y);
        [
            self.data[o],
            self.data[o + 1],
            self.data[o + 2],
            self.data[o + 3],
        ]
    }
}

/// `#rrggbb` to bytes; the prototype's `rgb()`.
///
/// # Panics
/// On anything but seven characters starting with `#` followed by hex digits.
pub fn hex_rgb(hex: &str) -> [u8; 3] {
    assert!(
        hex.len() == 7 && hex.starts_with('#'),
        "expected #rrggbb, got {hex:?}"
    );
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex digit");
    [byte(1), byte(3), byte(5)]
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn put_get_round_trip() {
        let mut img = RgbaImage::new(3, 2);
        img.put(2, 1, [1, 2, 3, 4]);
        assert_eq!(img.get(2, 1), [1, 2, 3, 4]);
        assert_eq!(img.get(0, 0), [0, 0, 0, 0]);
        assert_eq!(img.data.len(), 24);
    }

    #[test]
    fn hex_parses_prototype_palette() {
        assert_eq!(hex_rgb("#3f7d34"), [0x3f, 0x7d, 0x34]);
        assert_eq!(hex_rgb("#ffffff"), [255, 255, 255]);
    }
}
