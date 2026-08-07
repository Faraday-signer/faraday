//! Scrollable list of rows. Renders at most `max_visible` rows into the
//! provided rectangle. Selected row is filled in `theme.accent` with dark
//! text; unselected rows are borderless with muted text.
//!
//! Optional 2-line mode: if any row has a subtitle, all rows reserve the
//! subtitle slot so vertical rhythm stays uniform.

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
pub struct ListRow<'a> {
    pub label: &'a str,
    pub subtitle: Option<&'a str>,
    /// Optional short prefix rendered in a fixed-width column on the left —
    /// typically a number or tag (e.g. `"01"`, `"02"` for seed words).
    pub prefix: Option<&'a str>,
}

impl<'a> ListRow<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            subtitle: None,
            prefix: None,
        }
    }

    pub const fn with_subtitle(label: &'a str, subtitle: &'a str) -> Self {
        Self {
            label,
            subtitle: Some(subtitle),
            prefix: None,
        }
    }

    pub const fn with_prefix(prefix: &'a str, label: &'a str) -> Self {
        Self {
            label,
            subtitle: None,
            prefix: Some(prefix),
        }
    }
}

pub struct List<'a> {
    pub items: &'a [ListRow<'a>],
    pub selected: usize,
    pub max_visible: usize,
    /// When false, no row draws a selection highlight (read-only display,
    /// e.g. seed-word pages).
    pub selectable: bool,
}

/// Compute the first-visible index for a sliding window over `len` items
/// with `visible` slots, keeping `selected` on screen. Extracted so the
/// math is pure and testable.
pub fn visible_start(len: usize, visible: usize, selected: usize) -> usize {
    if len == 0 || visible == 0 {
        return 0;
    }
    let last = len - 1;
    let selected = selected.min(last);
    let visible = visible.min(len);
    if selected < visible {
        0
    } else if selected > last.saturating_sub(visible / 2) {
        len - visible
    } else {
        selected - visible / 2
    }
}

impl<'a> List<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
        rect: Rectangle,
    ) -> Result<(), D::Error> {
        if self.items.is_empty() || self.max_visible == 0 {
            return Ok(());
        }

        let visible = self.max_visible.min(self.items.len());
        let has_subtitle = self.items.iter().any(|r| r.subtitle.is_some());
        let has_prefix = self.items.iter().any(|r| r.prefix.is_some());
        let selected = self.selected.min(self.items.len() - 1);
        let start = visible_start(self.items.len(), visible, self.selected);

        // Slot count is fixed to `max_visible` for the layout grid so row
        // heights match the edge-hint gutter cells. Short lists leave the
        // trailing slots empty (e.g. a 2-option menu occupies the top two
        // slots, aligning with K1 and K2 on the right).
        let slots = self.max_visible.max(1) as i32;
        let row_h = (rect.size.height as i32 / slots).max(0);

        for slot in 0..self.max_visible {
            let idx = start + slot;
            if idx >= self.items.len() {
                break;
            }
            let row = self.items[idx];
            let y = rect.top_left.y + slot as i32 * row_h;
            let row_rect = Rectangle::new(
                Point::new(rect.top_left.x, y),
                Size::new(rect.size.width, row_h as u32),
            );
            let is_selected = self.selectable && idx == selected;
            self.draw_row(
                display,
                theme,
                row_rect,
                row,
                is_selected,
                has_subtitle,
                has_prefix,
            )?;
        }

        // Scrollbar on the right edge when there's overflow. Wide enough to
        // read without stealing obvious horizontal room: dim track the full
        // height, bright cyan thumb sized to the visible fraction.
        if visible < self.items.len() {
            let bar_w: i32 = 4;
            let bar_x = rect.top_left.x + rect.size.width as i32 - bar_w - 2;
            let track_top = rect.top_left.y + 4;
            let track_h = rect.size.height as i32 - 8;

            // Track.
            Rectangle::new(
                Point::new(bar_x, track_top),
                Size::new(bar_w as u32, track_h as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(theme.border))
            .draw(display)?;

            // Thumb.
            let thumb_h = ((visible as i32 * track_h) / self.items.len() as i32).max(12);
            let thumb_y = track_top
                + (start as i32 * (track_h - thumb_h))
                    / (self.items.len() as i32 - visible as i32).max(1);
            // Thumb color: off-white (`theme.text`) — reads on both the dark
            // track and the cyan selected row without merging into either.
            Rectangle::new(
                Point::new(bar_x, thumb_y),
                Size::new(bar_w as u32, thumb_h as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(theme.text))
            .draw(display)?;
        }

        Ok(())
    }

    fn draw_row<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
        rect: Rectangle,
        row: ListRow<'_>,
        selected: bool,
        reserve_subtitle: bool,
        reserve_prefix: bool,
    ) -> Result<(), D::Error> {
        if selected {
            rect.into_styled(PrimitiveStyle::with_fill(theme.accent))
                .draw(display)?;
        }

        let label_color = if selected { theme.bg } else { theme.text };
        let subtitle_color = if selected { theme.bg } else { theme.muted };
        let prefix_color = if selected { theme.bg } else { theme.dim };

        let left = rect.top_left.x + theme.space_md;
        // When any row has a prefix, reserve a fixed column for it so labels
        // across rows line up.
        let prefix_col_w = if reserve_prefix { 40 } else { 0 };
        let x = left + prefix_col_w;

        if reserve_subtitle {
            // 2-line layout. Center the (label + subtitle) pair vertically
            // inside the row so spare space splits evenly top/bottom,
            // regardless of row height.
            let label_y = rect.top_left.y + rect.size.height as i32 / 2 - 1;
            // Subtitle sits ~4px below the label's baseline descender.
            let sub_y = label_y + 20;

            if let Some(pfx) = row.prefix {
                Text::with_alignment(
                    pfx,
                    Point::new(left, label_y),
                    theme.style_sm(prefix_color),
                    Alignment::Left,
                )
                .draw(display)?;
            }

            Text::with_alignment(
                row.label,
                Point::new(x, label_y),
                theme.style_lg(label_color),
                Alignment::Left,
            )
            .draw(display)?;

            if let Some(sub) = row.subtitle {
                Text::with_alignment(
                    sub,
                    Point::new(x, sub_y),
                    theme.style_sm(subtitle_color),
                    Alignment::Left,
                )
                .draw(display)?;
            }
        } else {
            // 1-line layout: vertically centered.
            let y = rect.top_left.y + rect.size.height as i32 / 2 + 10;

            if let Some(pfx) = row.prefix {
                Text::with_alignment(
                    pfx,
                    Point::new(left, y),
                    theme.style_sm(prefix_color),
                    Alignment::Left,
                )
                .draw(display)?;
            }

            Text::with_alignment(
                row.label,
                Point::new(x, y),
                theme.style_lg(label_color),
                Alignment::Left,
            )
            .draw(display)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_fits_everything() {
        // 3 items, 3 slots — start is always 0 regardless of selected.
        assert_eq!(visible_start(3, 3, 0), 0);
        assert_eq!(visible_start(3, 3, 1), 0);
        assert_eq!(visible_start(3, 3, 2), 0);
    }

    #[test]
    fn window_scrolls_as_selected_moves_past_top_half() {
        // 5 items, 3 slots. No scroll while selected is within the initial
        // visible window (indices 0..visible). Scroll kicks in once the
        // selected index can't be shown without shifting.
        assert_eq!(visible_start(5, 3, 0), 0);
        assert_eq!(visible_start(5, 3, 1), 0);
        assert_eq!(visible_start(5, 3, 2), 0);
        assert_eq!(visible_start(5, 3, 3), 2);
        assert_eq!(visible_start(5, 3, 4), 2);
    }

    #[test]
    fn window_clamps_for_end_items() {
        // 10 items, 3 visible; last selected should never push start past len-3.
        assert_eq!(visible_start(10, 3, 9), 7);
        assert_eq!(visible_start(10, 3, 8), 7);
    }

    #[test]
    fn window_out_of_range_selected_is_clamped() {
        assert_eq!(visible_start(5, 3, 99), 2);
    }

    #[test]
    fn window_empty_or_zero_visible() {
        assert_eq!(visible_start(0, 3, 0), 0);
        assert_eq!(visible_start(5, 0, 2), 0);
    }
}
