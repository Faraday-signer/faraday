//! Solana-inspired color palette for the 240x240 display.
//!
//! All colors are RGB565 for direct use with embedded-graphics.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;

// Base
pub const BLACK: Rgb565 = Rgb565::BLACK;
pub const WHITE: Rgb565 = Rgb565::WHITE;

// Solana brand
pub const SOLANA_GREEN: Rgb565 = Rgb565::new(2, 60, 18); // #14F195

// Backgrounds
pub const BG_DARK: Rgb565 = Rgb565::new(1, 2, 2); // #0A1014

// Text
pub const TEXT_SECONDARY: Rgb565 = Rgb565::new(16, 33, 16); // #848884
pub const TEXT_MUTED: Rgb565 = Rgb565::new(8, 16, 8); // #404040

// Accent / semantic
pub const SUCCESS: Rgb565 = SOLANA_GREEN;

/// Faraday brand cyan (#1AF8FF). Used for locator / "you-are-here" overlays
/// and any place we want the backup UI to visually match the printable
/// template's accent.
pub const BRAND_CYAN: Rgb565 = Rgb565::new(3, 61, 31);

/// Neutral separator for drawing visible borders between adjacent QR cells.
pub const CELL_BORDER: Rgb565 = Rgb565::new(14, 28, 14); // mid-grey

// Card borders
pub const BORDER_DEFAULT: Rgb565 = Rgb565::new(4, 8, 5); // #202828

// Faraday brand (from .faraday-design-bundle/.../colors_and_type.css)
pub const FD_BG: Rgb565 = Rgb565::new(0, 5, 4); // #001721 deep navy
pub const FD_ACCENT: Rgb565 = Rgb565::new(3, 62, 31); // #1AF8FF cyan
