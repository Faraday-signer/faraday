//! Software framebuffer that can target ST7789 or desktop window.
//!
//! All UI rendering goes through this abstraction. On Pi, it pushes
//! to the ST7789 via SPI. On desktop (--features simulator), it
//! opens a minifb window.

use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::Rgb565,
    Pixel,
};

pub const WIDTH: u32 = 240;
pub const HEIGHT: u32 = 240;

/// Software framebuffer — stores pixels as RGB565 big-endian.
pub struct Framebuffer {
    pub pixels: Vec<u16>,
}

impl Framebuffer {
    pub fn new() -> Self {
        Framebuffer {
            pixels: vec![0u16; (WIDTH * HEIGHT) as usize],
        }
    }

    /// Convert pixel buffer to RGB888 (for desktop display).
    pub fn to_rgb888(&self) -> Vec<u32> {
        self.pixels.iter().map(|&p| {
            let r = ((p >> 11) & 0x1F) as u32;
            let g = ((p >> 5) & 0x3F) as u32;
            let b = (p & 0x1F) as u32;
            // Scale up: 5-bit to 8-bit, 6-bit to 8-bit
            let r8 = (r << 3) | (r >> 2);
            let g8 = (g << 2) | (g >> 4);
            let b8 = (b << 3) | (b >> 2);
            (r8 << 16) | (g8 << 8) | b8
        }).collect()
    }

    /// Convert to raw bytes for ST7789 SPI (big-endian RGB565).
    /// Used on Pi hardware to push framebuffer to display via SPI.
    #[cfg(target_os = "linux")]
    pub fn to_spi_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 2);
        for &p in &self.pixels {
            bytes.push((p >> 8) as u8);
            bytes.push((p & 0xFF) as u8);
        }
        bytes
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < WIDTH as i32
                && coord.y >= 0
                && coord.y < HEIGHT as i32
            {
                let idx = (coord.y as u32 * WIDTH + coord.x as u32) as usize;
                let raw = embedded_graphics_core::pixelcolor::raw::RawU16::from(color);
                self.pixels[idx] = embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw);
            }
        }
        Ok(())
    }
}
