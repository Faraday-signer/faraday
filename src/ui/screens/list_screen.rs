//! List screen template: Header (top) + List (body) + ButtonBar (bottom).
//!
//! The canonical screen for navigation: main menu, create/load method pickers,
//! settings, account switcher. Compose a `ListScreen` and call `draw()`.

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};

use crate::ui::layout::{split_bottom, split_top};
use embedded_graphics::{
    text::{Alignment, Text},
    Drawable,
};

use crate::ui::widgets::{ButtonBar, Header, HeaderKind, List, ListRow};
use crate::ui::Theme;

pub struct ListScreen<'a> {
    pub header: HeaderKind<'a>,
    pub counter: Option<(usize, usize)>,
    /// Optional right-aligned header label (e.g. short wallet pubkey).
    /// Only rendered when `counter` is None.
    pub right_label: Option<&'a str>,
    /// Optional context line rendered between header and list. Reserved for
    /// warnings / consequence framing on commitment screens. Renders in
    /// `theme.danger` so it visibly sits apart from chrome.
    pub description: Option<&'a str>,
    pub items: &'a [ListRow<'a>],
    pub selected: usize,
    pub max_visible: usize,
    /// When false, no row draws a selection highlight (read-only display).
    pub selectable: bool,
    /// Empty ButtonBar (`ButtonBar::new()`) reclaims the footer space for
    /// the list body — useful for pure navigation menus where Key3/Key1
    /// always mean Back/Confirm, so labels are noise.
    pub buttons: ButtonBar<'a>,
}

impl<'a> ListScreen<'a> {
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

        // Brand header needs a bit more Y padding around the pixel logo
        // than the tight text-title bar.
        let header_h = match self.header {
            HeaderKind::Brand => theme.header_h as i32 + 8,
            HeaderKind::Title(_) => theme.header_h as i32,
        };
        let (header_rect, rest) = split_top(screen, header_h);
        // Nav screens pass an empty ButtonBar to reclaim the footer band —
        // Key3/Key1 still work as Back/Confirm even without on-screen labels.
        let footer_h = if self.buttons.is_empty() { 0 } else { theme.footer_h as i32 };
        let (body_rect, footer_rect) = split_bottom(rest, footer_h);

        // Pattern-move the header kind so we can re-consume the one we own.
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

        // Optional warning banner. Full-bleed band filled with `theme.danger`;
        // left-aligned "!" sigil + inverted text so the whole strip reads as
        // an active warning, not a footnote.
        let body_rect = if let Some(desc) = self.description {
            let band_h = 42i32;
            let (band_rect, rest) = split_top(body_rect, band_h);

            // Danger fill.
            display.fill_solid(&band_rect, theme.danger)?;

            // Big "!" sigil on the left, in the bg color (inverted).
            let sigil_baseline = band_rect.top_left.y
                + band_rect.size.height as i32 / 2 + 10;
            Text::with_alignment(
                "!",
                Point::new(
                    band_rect.top_left.x + theme.space_md + 4,
                    sigil_baseline,
                ),
                theme.style_lg(theme.bg),
                Alignment::Left,
            )
            .draw(display)?;

            // Message next to the sigil, inverted (bg on danger).
            let msg_baseline = band_rect.top_left.y
                + band_rect.size.height as i32 / 2 + 6;
            Text::with_alignment(
                desc,
                Point::new(
                    band_rect.top_left.x + theme.space_md + 32,
                    msg_baseline,
                ),
                theme.style_sm(theme.bg),
                Alignment::Left,
            )
            .draw(display)?;

            rest
        } else {
            body_rect
        };

        // Rows span the full viewport width; padding is applied inside each
        // row so the selected highlight reaches edge-to-edge.
        let body_rect = Rectangle::new(
            Point::new(
                body_rect.top_left.x,
                body_rect.top_left.y + theme.space_sm,
            ),
            Size::new(
                body_rect.size.width,
                body_rect.size.height.saturating_sub(theme.space_sm as u32 * 2),
            ),
        );

        List {
            items: self.items,
            selected: self.selected,
            max_visible: self.max_visible,
            selectable: self.selectable,
        }
        .draw(display, theme, body_rect)?;

        self.buttons.draw(display, theme, footer_rect)?;

        Ok(())
    }
}
