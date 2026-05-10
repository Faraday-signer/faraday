//! Card screen template: Header (top) + Card (body) + right-edge hints.
//!
//! Canonical screen for the card register — read-only info and
//! commit-target displays (About, address, tx review, seed words).

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};

use crate::ui::layout::split_top;
use crate::ui::widgets::{Card, CardRow, EdgeHints, Header, HeaderKind, GUTTER_W};
use crate::ui::Theme;

pub struct CardScreen<'a> {
    pub header: HeaderKind<'a>,
    pub counter: Option<(usize, usize)>,
    pub right_label: Option<&'a str>,
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub body_lines: &'a [&'a str],
    pub rows: &'a [CardRow<'a>],
    pub edge_hints: EdgeHints,
    /// Render the title in `theme.danger` — for mismatch / error screens.
    pub title_danger: bool,
}

impl<'a> CardScreen<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
    ) -> Result<(), D::Error> {
        let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
        display.fill_solid(&screen, theme.bg)?;

        // Header spans the full screen width; gutter only narrows the body.
        let (header_rect, rest) = split_top(screen, theme.header_h as i32);

        let body_rect = if self.edge_hints.is_empty() {
            rest
        } else {
            Rectangle::new(
                rest.top_left,
                Size::new(rest.size.width - GUTTER_W, rest.size.height),
            )
        };

        let kind = match self.header {
            HeaderKind::Title(t) => HeaderKind::Title(t),
            HeaderKind::Brand => HeaderKind::Brand,
        };
        Header {
            kind,
            counter: self.counter,
            right_label: self.right_label,
        }
        .draw(display, theme, header_rect)?;

        Card {
            title: self.title,
            subtitle: self.subtitle,
            body_lines: self.body_lines,
            rows: self.rows,
            title_danger: self.title_danger,
        }
        .draw(display, theme, body_rect)?;

        if !self.edge_hints.is_empty() {
            let gutter = Rectangle::new(
                Point::new(
                    rest.top_left.x + rest.size.width as i32 - GUTTER_W as i32,
                    rest.top_left.y,
                ),
                Size::new(GUTTER_W, rest.size.height),
            );
            self.edge_hints.draw(display, theme, gutter)?;
        }

        Ok(())
    }
}
