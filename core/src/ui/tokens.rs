//! Design tokens.
//!
//! A `Theme` carries every value that varies by screen size or brand:
//! dimensions, chrome heights, colors, spacing, font sizes. Widgets read
//! from a theme instead of hardcoding numbers, so a different target is
//! a different `Theme`.

use embedded_graphics::pixelcolor::Rgb565;
use u8g2_fonts::{fonts, U8g2TextStyle};

#[derive(Clone)]
pub struct Theme {
    // Screen geometry
    pub width: u32,
    pub height: u32,

    // Reserved chrome zones (header + button bar).
    pub header_h: u32,
    pub footer_h: u32,

    // Colors.
    pub bg: Rgb565,
    pub text: Rgb565,
    pub muted: Rgb565,
    pub dim: Rgb565,
    pub accent: Rgb565, // cyan — means "selected" or "you are about to commit"
    pub border: Rgb565,
    pub danger: Rgb565,

    // Spacing scale.
    pub space_sm: i32,
    pub space_md: i32,
}

impl Theme {
    /// Faraday default theme for the 240x240 ST7789.
    pub fn faraday_240() -> Self {
        Self {
            width: 240,
            height: 240,
            header_h: 29,
            footer_h: 26,

            bg: Rgb565::new(0, 5, 4),        // #001721
            text: Rgb565::new(28, 57, 28),   // #E7E7E7
            muted: Rgb565::new(17, 39, 21),  // #8C9CA8
            dim: Rgb565::new(11, 28, 16),    // #5E7180
            accent: Rgb565::new(3, 62, 31),  // #1AF8FF
            border: Rgb565::new(2, 16, 11),  // #114358
            danger: Rgb565::new(31, 13, 13), // #FF6B6B

            space_sm: 8,
            space_md: 12,
        }
    }

    /// Faraday theme for the 240x320 ESP32-S3 portrait display.
    /// Width stays 240 so horizontal layouts are identical to the Pi;
    /// the extra 80px height gives more room for list items and touch targets.
    pub fn faraday_320() -> Self {
        Self {
            width: 240,
            height: 320,
            header_h: 29,
            footer_h: 32,

            bg: Rgb565::new(0, 5, 4),
            text: Rgb565::new(28, 57, 28),
            muted: Rgb565::new(17, 39, 21),
            dim: Rgb565::new(11, 28, 16),
            accent: Rgb565::new(3, 62, 31),
            border: Rgb565::new(2, 16, 11),
            danger: Rgb565::new(31, 13, 13),

            space_sm: 8,
            space_md: 12,
        }
    }

    /// Swap in the light palette — the site's light side on the device:
    /// paper background, ink text, and the darker brand-ink accent
    /// (#0E7580), because bright cyan is unreadable on light (the same
    /// lesson the site's `--brand-ink` token encodes). Geometry is
    /// untouched, so this is safe to call at runtime.
    pub fn set_light_palette(&mut self) {
        self.bg = Rgb565::new(30, 61, 30); // #F2F6F7 paper
        self.text = Rgb565::new(1, 7, 5); // #0B1F28 ink
        self.muted = Rgb565::new(8, 24, 13); // #47606B
        self.dim = Rgb565::new(15, 36, 19); // #78909B
        self.accent = Rgb565::new(1, 29, 16); // #0E7580 brand-ink
        self.border = Rgb565::new(24, 53, 27); // #C6D4D9
        self.danger = Rgb565::new(24, 12, 6); // #C03030
    }

    /// Swap back to the dark palette (the boot default).
    pub fn set_dark_palette(&mut self) {
        self.bg = Rgb565::new(0, 5, 4); // #001721
        self.text = Rgb565::new(28, 57, 28); // #E7E7E7
        self.muted = Rgb565::new(17, 39, 21); // #8C9CA8
        self.dim = Rgb565::new(11, 28, 16); // #5E7180
        self.accent = Rgb565::new(3, 62, 31); // #1AF8FF
        self.border = Rgb565::new(2, 16, 11); // #114358
        self.danger = Rgb565::new(31, 13, 13); // #FF6B6B
    }

    /// Whether the light palette is active (bg is the light paper tone).
    pub fn is_light(&self) -> bool {
        self.bg == Rgb565::new(30, 61, 30)
    }

    /// Primary label text (list row label, card value).
    pub fn style_lg(&self, color: Rgb565) -> U8g2TextStyle<Rgb565> {
        U8g2TextStyle::new(fonts::u8g2_font_profont29_mr, color)
    }

    /// Body copy that needs to be readable from arm's length (help screens,
    /// long instructions). One step up from `style_sm` — fewer chars per
    /// line but easier to read.
    pub fn style_md(&self, color: Rgb565) -> U8g2TextStyle<Rgb565> {
        U8g2TextStyle::new(fonts::u8g2_font_profont22_mr, color)
    }

    /// Secondary text (list row subtitle, header title, footer legend, counters).
    /// On a 240x240 this is the smallest size that still reads well — `profont17`.
    pub fn style_sm(&self, color: Rgb565) -> U8g2TextStyle<Rgb565> {
        U8g2TextStyle::new(fonts::u8g2_font_profont17_mr, color)
    }
}
