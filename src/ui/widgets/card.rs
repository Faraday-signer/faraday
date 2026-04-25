//! Card widget: read-only information display. Used for the commitment
//! register (tx review, seed words, confirmations) and info screens (About,
//! address display).
//!
//! Shape:
//!   - Optional `title` rendered in accent, lg, left-aligned at the top.
//!   - Optional `rows` of (label, value) pairs, each on one line.
//!
//! Unlike `List`, no row is "selected" — card rows are purely informational.

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
    Drawable,
};

use crate::ui::Theme;

#[derive(Clone, Copy)]
pub struct CardRow<'a> {
    pub label: &'a str,
    pub value: &'a str,
}

impl<'a> CardRow<'a> {
    pub const fn new(label: &'a str, value: &'a str) -> Self {
        Self { label, value }
    }
}

pub struct Card<'a> {
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    /// Optional multi-line body rendered between the subtitle and the rows.
    /// Each line is drawn left-aligned at `theme.text`. Use this for wrapped
    /// long values (addresses, hashes) that don't fit in a single row.
    pub body_lines: &'a [&'a str],
    pub rows: &'a [CardRow<'a>],
    /// Render the title in `theme.danger` instead of `theme.accent`. Used
    /// on mismatch / error screens so the header hero reads as a warning.
    pub title_danger: bool,
}

impl<'a> Card<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
        rect: Rectangle,
    ) -> Result<(), D::Error> {
        let mut cursor_y = rect.top_left.y;
        let right_x = rect.top_left.x + rect.size.width as i32 - theme.space_md;
        let left_x = rect.top_left.x + theme.space_md;

        // Hero block (title + subtitle).
        if let Some(title) = self.title {
            cursor_y += 28;
            let title_color = if self.title_danger {
                theme.danger
            } else {
                theme.accent
            };
            Text::with_alignment(
                title,
                Point::new(left_x, cursor_y),
                theme.style_lg(title_color),
                Alignment::Left,
            )
            .draw(display)?;

            if let Some(sub) = self.subtitle {
                cursor_y += 22;
                Text::with_alignment(
                    sub,
                    Point::new(left_x, cursor_y),
                    theme.style_sm(theme.muted),
                    Alignment::Left,
                )
                .draw(display)?;
            }

            // Divider under the hero block — positioned at the 1/3 mark of
            // the body rect so it lands exactly on the K1/K2 gutter cell
            // boundary. Only drawn when body or rows follow.
            if !self.rows.is_empty() || !self.body_lines.is_empty() {
                cursor_y = rect.top_left.y + (rect.size.height as i32 / 3);
                Rectangle::new(
                    Point::new(rect.top_left.x, cursor_y),
                    Size::new(rect.size.width, 1),
                )
                .into_styled(PrimitiveStyle::with_fill(theme.border))
                .draw(display)?;
                cursor_y += theme.space_md;
            }
        }

        // Multi-line body. One line per `body_lines` entry, left-aligned.
        for line in self.body_lines {
            cursor_y += 18;
            Text::with_alignment(
                line,
                Point::new(left_x, cursor_y),
                theme.style_sm(theme.text),
                Alignment::Left,
            )
            .draw(display)?;
        }
        if !self.body_lines.is_empty() && !self.rows.is_empty() {
            cursor_y += theme.space_md;
        }

        // Key/value rows.
        if !self.rows.is_empty() {
            let remaining_h = rect.size.height as i32 - (cursor_y - rect.top_left.y);
            let row_h = remaining_h / self.rows.len() as i32;
            for (i, row) in self.rows.iter().enumerate() {
                let baseline = cursor_y + row_h * i as i32 + row_h / 2 + 6;
                Text::with_alignment(
                    row.label,
                    Point::new(left_x, baseline),
                    theme.style_sm(theme.dim),
                    Alignment::Left,
                )
                .draw(display)?;
                Text::with_alignment(
                    row.value,
                    Point::new(right_x, baseline),
                    theme.style_sm(theme.text),
                    Alignment::Right,
                )
                .draw(display)?;
            }
        }

        Ok(())
    }
}
