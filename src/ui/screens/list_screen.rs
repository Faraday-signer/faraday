//! List screen template: Header (top) + List (body) + right-edge hints.
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

use crate::ui::widgets::{EdgeHints, Header, HeaderKind, List, ListRow, GUTTER_W};
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
    /// Per-key boxes on the right gutter. When non-empty the body / header
    /// / footer rects are shrunk by `GUTTER_W` so content doesn't overlap.
    pub edge_hints: EdgeHints,
}

impl<'a> ListScreen<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
    ) -> Result<(), D::Error> {
        let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
        display.fill_solid(&screen, theme.bg)?;

        // Header spans the full screen width (edge to edge). Same height
        // for Title and Brand so flows don't visually shift when moving
        // between screens.
        let (header_rect, rest) = split_top(screen, theme.header_h as i32);

        // Reserve the right gutter for edge hints when present.
        let body_rect = if self.edge_hints.is_empty() {
            rest
        } else {
            Rectangle::new(
                rest.top_left,
                Size::new(rest.size.width - GUTTER_W, rest.size.height),
            )
        };

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
            let sigil_baseline = band_rect.top_left.y + band_rect.size.height as i32 / 2 + 10;
            Text::with_alignment(
                "!",
                Point::new(band_rect.top_left.x + theme.space_md + 4, sigil_baseline),
                theme.style_lg(theme.bg),
                Alignment::Left,
            )
            .draw(display)?;

            // Message next to the sigil, inverted (bg on danger).
            let msg_baseline = band_rect.top_left.y + band_rect.size.height as i32 / 2 + 6;
            Text::with_alignment(
                desc,
                Point::new(band_rect.top_left.x + theme.space_md + 32, msg_baseline),
                theme.style_sm(theme.bg),
                Alignment::Left,
            )
            .draw(display)?;

            rest
        } else {
            body_rect
        };

        // Rows span the body edge-to-edge so row heights match the gutter
        // cells — no extra top/bottom padding, no inter-row gap.
        List {
            items: self.items,
            selected: self.selected,
            max_visible: self.max_visible,
            selectable: self.selectable,
        }
        .draw(display, theme, body_rect)?;

        // Gutter column: starts at the right edge of the body, immediately
        // below the header hairline, and extends to the bottom of the screen.
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
