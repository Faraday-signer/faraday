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
    /// Optional right-aligned label (typically a short wallet pubkey).
    /// Rendered only when `counter` is None, so screens that use the
    /// counter slot (paginated readers, quizzes) keep it for that.
    pub right_label: Option<&'a str>,
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
                // Full pixel-art logo (mark + wordmark). Scale 2 makes the
                // chunky pixels legible inside the 29 px band; small left
                // padding keeps it close to the edge without hugging it.
                const BRAND_SCALE: u32 = 2;
                const BRAND_LEFT_PAD: i32 = 6;
                let logo_h = (logo::LOGO_HEIGHT * BRAND_SCALE) as i32;
                let x = rect.top_left.x + BRAND_LEFT_PAD;
                // +1 to the centered y so there's a hair more air above the
                // logo than below it — looks more balanced against the
                // menu's hard horizontal divider underneath.
                let y = rect.top_left.y
                    + (rect.size.height as i32 - logo_h) / 2
                    + 1;
                logo::draw_logo(display, x, y, BRAND_SCALE, theme.accent)?;
            }
        }

        // Right side: counter OR right_label. Counter wins when both set.
        // For Brand headers we align the right text to the logo's vertical
        // center so the pubkey chip sits on the same visual line as the
        // logo instead of hugging the bottom edge of the (taller) band.
        let x_right =
            rect.top_left.x + rect.size.width as i32 - theme.space_md;
        let right_baseline = match self.kind {
            HeaderKind::Brand => {
                rect.top_left.y + rect.size.height as i32 / 2 + 5
            }
            HeaderKind::Title(_) => baseline,
        };
        if let Some((now, total)) = self.counter {
            let mut buf = [0u8; 8];
            let s = fmt_counter(&mut buf, now, total);
            Text::with_alignment(
                s,
                Point::new(x_right, right_baseline),
                theme.style_sm(theme.dim),
                Alignment::Right,
            )
            .draw(display)?;
        } else if let Some(label) = self.right_label {
            Text::with_alignment(
                label,
                Point::new(x_right, right_baseline),
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
