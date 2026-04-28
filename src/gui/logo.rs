//! Faraday mark + wordmark — pixel-art bitmaps.
//!
//! Extracted from assets/brand/{faraday-mark.svg, faraday-logo.svg}.
//! Render at integer scales only — the pixel grid IS the brand.

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

pub const MARK_SIZE: u32 = 8;

#[rustfmt::skip]
const MARK: [u32; 8] = [
    0b1111_0011,
    0b1111_0011,
    0b1111_0000,
    0b1111_0000,
    0b0000_1111,
    0b0000_1111,
    0b1100_1111,
    0b1100_1111,
];

/// Draw the Faraday mark at (x, y) with integer scale.
/// Scale 1 = 8px, 2 = 16px, 4 = 32px, 8 = 64px.
pub fn draw_mark<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    x: i32,
    y: i32,
    scale: u32,
    color: Rgb565,
) -> Result<(), D::Error> {
    let s = scale as i32;
    let style = PrimitiveStyle::with_fill(color);
    let last = (MARK_SIZE - 1) as i32;
    for (row, &bits) in MARK.iter().enumerate() {
        for col in 0..MARK_SIZE as i32 {
            if (bits >> (last - col)) & 1 == 1 {
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
// Full logo (mark + wordmark). Decoded from the SVG path geometry at
// the native 60x10 grid (unit = 12.836). Each cell is sampled at its
// center — no rasterization, no filter artifacts. 1 bit per pixel,
// packed MSB-first into two u32 chunks per row.
// =====================================================================

pub const LOGO_WIDTH: u32 = 60;
pub const LOGO_HEIGHT: u32 = 10;

#[rustfmt::skip]
const LOGO: [[u32; 2]; 10] = [
    [0xf30f8000, 0x00080000],
    [0xf3080000, 0x00080000],
    [0xf0080e36, 0x3868e220],
    [0xf00f0118, 0x04981220],
    [0x0f080f10, 0x3c88f220],
    [0x0f081110, 0x44891140],
    [0xcf081310, 0x4c993140],
    [0xcf080d3c, 0x3468d080],
    [0x00000000, 0x00000080],
    [0x00000000, 0x00000300],
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
