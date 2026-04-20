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
// Full logo (mark + pixel-art wordmark).
// Extracted from assets/brand/faraday-logo-small.svg, rasterized at 1:1
// (95x21), cropped to the bbox of the inked pixels. 1 bit per pixel.
// =====================================================================

pub const LOGO_WIDTH: u32 = 83;
pub const LOGO_HEIGHT: u32 = 15;

#[rustfmt::skip]
const LOGO: [[u32; 3]; 15] = [
    [0xf1f07f00, 0x00000006, 0x00000000],
    [0xf1f07f00, 0x00000006, 0x00000000],
    [0xc0304000, 0x00000006, 0x00000000],
    [0xc030400f, 0x0ef1f076, 0x1e186000],
    [0xce30400f, 0x0ef1f076, 0x1e186000],
    [0x0e007e00, 0xc38008ce, 0x01986000],
    [0x0e00400f, 0xc201f8c6, 0x1f986000],
    [0xce30403f, 0xc203f8c6, 0x7f9de000],
    [0xc0304030, 0xc20308c6, 0x61858000],
    [0xc0304031, 0xc20338ce, 0x63858000],
    [0xf1f0400e, 0xcfc1e876, 0x1d830000],
    [0xf1f0400e, 0xcfc1e876, 0x1d830000],
    [0x00000000, 0x00000000, 0x00030000],
    [0x00000000, 0x00000000, 0x001c0000],
    [0x00000000, 0x00000000, 0x001c0000],
];

/// Draw the Faraday logo (mark + wordmark) at (x, y) with integer scale.
/// Scale 1 = 83x15, 2 = 166x30, 3 = 249x45 (too wide for 240 screen).
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
