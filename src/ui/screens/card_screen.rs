//! Card screen template: Header (top) + Card (body) + ButtonBar (bottom).
//!
//! Canonical screen for the card register — read-only info and
//! commit-target displays (About, address, tx review, seed words).

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};

use crate::ui::layout::{split_bottom, split_top};
use crate::ui::widgets::{ButtonBar, Card, CardRow, Header, HeaderKind};
use crate::ui::Theme;

pub struct CardScreen<'a> {
    pub header: HeaderKind<'a>,
    pub counter: Option<(usize, usize)>,
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub body_lines: &'a [&'a str],
    pub rows: &'a [CardRow<'a>],
    pub buttons: ButtonBar<'a>,
}

impl<'a> CardScreen<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
    ) -> Result<(), D::Error> {
        let screen = Rectangle::new(
            Point::zero(),
            Size::new(theme.width, theme.height),
        );
        display.fill_solid(&screen, theme.bg)?;

        let (header_rect, rest) = split_top(screen, theme.header_h as i32);
        let (body_rect, footer_rect) = split_bottom(rest, theme.footer_h as i32);

        let kind = match self.header {
            HeaderKind::Title(t) => HeaderKind::Title(t),
            HeaderKind::Brand => HeaderKind::Brand,
        };
        Header { kind, counter: self.counter }
            .draw(display, theme, header_rect)?;

        Card {
            title: self.title,
            subtitle: self.subtitle,
            body_lines: self.body_lines,
            rows: self.rows,
        }
        .draw(display, theme, body_rect)?;

        self.buttons.draw(display, theme, footer_rect)?;

        Ok(())
    }
}
