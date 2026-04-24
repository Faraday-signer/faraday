//! Faraday mark + wordmark — pixel-art bitmaps.
//!
//! Extracted from assets/brand/{faraday-mark.svg, faraday-logo.svg}.
//! Render at integer scales only — the pixel grid IS the brand.

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

pub const MARK_SIZE: u32 = 21;

#[rustfmt::skip]
const MARK: [u32; 21] = [
    0b111111110000011111111,
    0b111111110000011111111,
    0b111111110000011111111,
    0b111000000000000000111,
    0b111000000000000000111,
    0b111000000000000000111,
    0b111000000000000000111,
    0b111000000000000000111,
    0b000000001111100000000,
    0b000000001111100000000,
    0b000000001111100000000,
    0b000000001111100000000,
    0b000000001111100000000,
    0b111000000000000000111,
    0b111000000000000000111,
    0b111000000000000000111,
    0b111000000000000000111,
    0b111000000000000000111,
    0b111111110000011111111,
    0b111111110000011111111,
    0b111111110000011111111,
];

/// Draw the Faraday mark at (x, y) with integer scale.
/// Scale 1 = 21px, 2 = 42px, 3 = 63px, 4 = 84px.
pub fn draw_mark<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    x: i32,
    y: i32,
    scale: u32,
    color: Rgb565,
) -> Result<(), D::Error> {
    let s = scale as i32;
    let style = PrimitiveStyle::with_fill(color);
    for (row, &bits) in MARK.iter().enumerate() {
        for col in 0..21i32 {
            if (bits >> (20 - col)) & 1 == 1 {
                Rectangle::new(
                    Point::new(x + col * s, y + row as i32 * s),
                    Size::new(scale, scale),
                )
                .into_styled(style)
                .draw(display)?;
            }
        }
    }
    Ok(())
}

// =====================================================================
// Full logo (mark + wordmark). Extracted directly from the SVG path
// geometry at the native 60x10 grid (unit = 12.836). Each cell is
// sampled at its center — no rasterization, no filter artifacts.
// 1 bit per pixel, packed MSB-first into two u32 chunks per row.
// =====================================================================

pub const LOGO_WIDTH: u32 = 60;
pub const LOGO_HEIGHT: u32 = 10;

#[rustfmt::skip]
const LOGO: [[u32; 2]; 10] = [
    [0x661f0000, 0x00100000],
    [0x81100000, 0x00100000],
    [0x81101c6c, 0x70d1c440],
    [0x181e0230, 0x09302440],
    [0x18101e20, 0x7911e440],
    [0x81102220, 0x89122280],
    [0x81102620, 0x99326280],
    [0x66101a78, 0x68d1a100],
    [0x00000000, 0x00000100],
    [0x00000000, 0x00000600],
];

/// Draw the Faraday logo (mark + wordmark) at (x, y) with integer scale.
/// Scale 1 = 60x10, 2 = 120x20, 3 = 180x30, 4 = 240x40 (fills screen width).
pub fn draw_logo<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    x: i32,
    y: i32,
    scale: u32,
    color: Rgb565,
) -> Result<(), D::Error> {
    let s = scale as i32;
    let style = PrimitiveStyle::with_fill(color);
    for (row, chunks) in LOGO.iter().enumerate() {
        for col in 0..LOGO_WIDTH as i32 {
            let chunk = chunks[(col / 32) as usize];
            let bit = (chunk >> (31 - (col % 32))) & 1;
            if bit == 1 {
                Rectangle::new(
                    Point::new(x + col * s, y + row as i32 * s),
                    Size::new(scale, scale),
                )
                .into_styled(style)
                .draw(display)?;
            }
        }
    }
    Ok(())
}

