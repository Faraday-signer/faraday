//! Screen header.
//!
//! Two variants:
//! - `HeaderKind::Title(&str)` — normal screen title (accent, left) + optional
//!   counter (dim, right). Hairline rule underneath.
//! - `HeaderKind::Brand` — reserved for the main menu. Renders the Faraday
//!   pixel mark + wordmark.

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
    Drawable,
};

use crate::gui::logo;
use crate::ui::Theme;

pub enum HeaderKind<'a> {
    Title(&'a str),
    Brand,
}

pub struct Header<'a> {
    pub kind: HeaderKind<'a>,
    pub counter: Option<(usize, usize)>,
}

impl<'a> Header<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
        rect: Rectangle,
    ) -> Result<(), D::Error> {
        let baseline = rect.top_left.y + rect.size.height as i32 - 8;

        match self.kind {
            HeaderKind::Title(title) => {
                Text::with_alignment(
                    title,
                    Point::new(rect.top_left.x + theme.space_md, baseline),
                    theme.style_sm(theme.accent),
                    Alignment::Left,
                )
                .draw(display)?;
            }
            HeaderKind::Brand => {
                // Full pixel-art logo (mark + wordmark) from the small SVG.
                let x = rect.top_left.x + theme.space_sm;
                let y = rect.top_left.y
                    + (rect.size.height as i32 - logo::LOGO_HEIGHT as i32) / 2;
                logo::draw_logo(display, x, y, 1, theme.accent)?;
            }
        }

        // Counter (right, dim). Optional.
        if let Some((now, total)) = self.counter {
            let mut buf = [0u8; 8];
            let s = fmt_counter(&mut buf, now, total);
            let x_right =
                rect.top_left.x + rect.size.width as i32 - theme.space_md;
            Text::with_alignment(
                s,
                Point::new(x_right, baseline),
                theme.style_sm(theme.dim),
                Alignment::Right,
            )
            .draw(display)?;
        }

        // Hairline rule at bottom edge.
        let line_y = rect.top_left.y + rect.size.height as i32 - 1;
        Rectangle::new(
            Point::new(rect.top_left.x, line_y),
            Size::new(rect.size.width, 1),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.border))
        .draw(display)?;

        Ok(())
    }
}

/// Write `N/N` into a stack buffer. Returns the `&str` view. Width is
/// unpadded so low counts don't carry a stale-looking leading zero.
fn fmt_counter(buf: &mut [u8; 8], now: usize, total: usize) -> &str {
    use core::fmt::Write;
    let mut cursor = Cursor { buf, pos: 0 };
    let _ = write!(&mut cursor, "{}/{}", now, total);
    let end = cursor.pos;
    core::str::from_utf8(&buf[..end]).unwrap_or("")
}

struct Cursor<'a> {
    buf: &'a mut [u8; 8],
    pos: usize,
}

impl core::fmt::Write for Cursor<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len() - self.pos;
        let n = bytes.len().min(remaining);
        self.buf[self.pos..self.pos + n].copy_from_slice(&bytes[..n]);
        self.pos += n;
        Ok(())
    }
}
