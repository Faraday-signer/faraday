//! Solana-inspired color palette for the 240x240 display.
//!
//! All colors are RGB565 for direct use with embedded-graphics.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;

// Base
pub const BLACK: Rgb565 = Rgb565::BLACK;
pub const WHITE: Rgb565 = Rgb565::WHITE;

// Solana brand
pub const SOLANA_PURPLE: Rgb565 = Rgb565::new(19, 8, 31);  // #9945FF
pub const SOLANA_GREEN: Rgb565 = Rgb565::new(2, 60, 18);   // #14F195
pub const SOLANA_TEAL: Rgb565 = Rgb565::new(0, 48, 24);    // #00C2C2 ish

// Backgrounds
pub const BG_DARK: Rgb565 = Rgb565::new(1, 2, 2);          // #0A1014
pub const BG_CARD: Rgb565 = Rgb565::new(2, 5, 4);          // #141E20
pub const BG_CARD_SELECTED: Rgb565 = Rgb565::new(4, 6, 6); // #1E2830

// Text
pub const TEXT_PRIMARY: Rgb565 = Rgb565::new(29, 59, 29);   // #EEF0EE
pub const TEXT_SECONDARY: Rgb565 = Rgb565::new(16, 33, 16); // #848884
pub const TEXT_MUTED: Rgb565 = Rgb565::new(8, 16, 8);       // #404040

// Accent / semantic
pub const ACCENT: Rgb565 = SOLANA_PURPLE;
pub const SUCCESS: Rgb565 = SOLANA_GREEN;
pub const WARNING: Rgb565 = Rgb565::new(31, 48, 0);         // #FFC107
pub const DANGER: Rgb565 = Rgb565::new(31, 8, 7);           // #FF453A

/// Faraday brand cyan (#1AF8FF). Used for locator / "you-are-here" overlays
/// and any place we want the backup UI to visually match the printable
/// template's accent.
pub const BRAND_CYAN: Rgb565 = Rgb565::new(3, 61, 31);

/// Neutral separator for drawing visible borders between adjacent QR cells.
pub const CELL_BORDER: Rgb565 = Rgb565::new(14, 28, 14);    // mid-grey

// Card borders
pub const BORDER_DEFAULT: Rgb565 = Rgb565::new(4, 8, 5);    // #202828
pub const BORDER_SELECTED: Rgb565 = SOLANA_PURPLE;

// Faraday brand (from .faraday-design-bundle/.../colors_and_type.css)
pub const FD_BG: Rgb565 = Rgb565::new(0, 5, 4);           // #001721 deep navy
pub const FD_ACCENT: Rgb565 = Rgb565::new(3, 62, 31);     // #1AF8FF cyan
pub const FD_TEXT: Rgb565 = Rgb565::new(28, 57, 28);      // #E7E7E7 off-white
pub const FD_TEXT_MUTED: Rgb565 = Rgb565::new(17, 39, 21); // #8C9CA8 navy-gray

/// Blend two colors by a factor (0-255). 0 = full a, 255 = full b.
pub fn blend(a: Rgb565, b: Rgb565, factor: u8) -> Rgb565 {
    let f = factor as u16;
    let inv = 255 - f;

    let r_a = ((u16::from(RgbColor::r(&a))) * inv + (u16::from(RgbColor::r(&b))) * f) / 255;
    let g_a = ((u16::from(RgbColor::g(&a))) * inv + (u16::from(RgbColor::g(&b))) * f) / 255;
    let b_a = ((u16::from(RgbColor::b(&a))) * inv + (u16::from(RgbColor::b(&b))) * f) / 255;

    Rgb565::new(r_a as u8, g_a as u8, b_a as u8)
}
