//! Edge hints: a vertical chrome column on the right, below the header,
//! divided into 3 equal cells — one per Waveshare HAT physical key (K1
//! top, K2 middle, K3 bottom). Each cell shows what the adjacent hardware
//! key does on this screen.
//!
//! Cells with `EdgeIcon::None` render a small dot so every physical key
//! has a visual slot — no guessing whether the column is "missing" an
//! icon vs. the key is genuinely unused here.
//!
//! Glyph vocabulary:
//!   - `Check`      → commit / confirm / sign / select / done / retry
//!   - `Cross`      → terminal reject / cancel (destructive)
//!   - `ArrowRight` → advance (non-committal "next page" etc.)
//!   - `ArrowLeft`  → back (non-committal "previous page", "to menu")
//!   - `None`       → key does nothing here (shown as a small dot)

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
};

use crate::ui::Theme;

/// Width reserved on the right side for the edge-hint column. Screens
/// that use `EdgeHints` should shrink their body rect by this amount so
/// content doesn't bleed under the icon column.
///
/// On touch builds the gutter is gone (replaced by a bottom action bar), so
/// no width is reserved and `FOOTER_H` reserves height instead.
pub const GUTTER_W: u32 = if cfg!(feature = "touch-ui") { 0 } else { 28 };

/// Height reserved at the bottom for the horizontal touch action bar. Zero on
/// builds with physical keys — they reserve width via `GUTTER_W` instead. The
/// two are mutually exclusive so body reservation can always subtract both.
pub const FOOTER_H: u32 = if cfg!(feature = "touch-ui") { 44 } else { 0 };

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EdgeIcon {
    None,
    Check,
    Cross,
    /// Same X glyph as `Cross`, but rendered in the danger color. Used for an
    /// explicit "reject" action (e.g. the TX sign-approval screen) so it reads
    /// as a refusal rather than a neutral back/dismiss.
    CrossDanger,
    ArrowRight,
    /// Curved "return / go-back" glyph — a left-pointing arrow at the top
    /// with a short vertical hook on the right. Used for K3 = "back to
    /// previous screen" on every navigation screen.
    ArrowLeft,
    /// Backspace — a keyboard delete-key glyph (⌫): a left-pointing
    /// pentagon "key" with an × inside. Used for K2 = "delete last
    /// character" on keyboard / char-grid screens.
    Delete,
}

pub struct EdgeHints {
    pub k1: EdgeIcon,
    pub k2: EdgeIcon,
    pub k3: EdgeIcon,
}

impl EdgeHints {
    pub const fn new() -> Self {
        Self {
            k1: EdgeIcon::None,
            k2: EdgeIcon::None,
            k3: EdgeIcon::None,
        }
    }

    pub const fn k1(mut self, icon: EdgeIcon) -> Self {
        self.k1 = icon;
        self
    }

    pub const fn k2(mut self, icon: EdgeIcon) -> Self {
        self.k2 = icon;
        self
    }

    pub const fn k3(mut self, icon: EdgeIcon) -> Self {
        self.k3 = icon;
        self
    }

    pub const fn is_empty(&self) -> bool {
        matches!(self.k1, EdgeIcon::None)
            && matches!(self.k2, EdgeIcon::None)
            && matches!(self.k3, EdgeIcon::None)
    }

    /// Draw the three-cell column. `rect` is the gutter area below the
    /// header — the caller is responsible for positioning it so content
    /// doesn't overlap.
    #[cfg(not(feature = "touch-ui"))]
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
        rect: Rectangle,
    ) -> Result<(), D::Error> {
        // Extend the rect 1px upward so the gutter's top border lands on the
        // header's hairline rule (which sits at y = header_h - 1). Without
        // this the two parallel borders sit one pixel apart and read as a
        // doubled line at the junction.
        let rect = Rectangle::new(
            Point::new(rect.top_left.x, rect.top_left.y - 1),
            Size::new(rect.size.width, rect.size.height + 1),
        );

        // Opaque fill so the column reads as chrome on top of live camera
        // frames — same `theme.bg` as the header.
        display.fill_solid(&rect, theme.bg)?;

        // Outer column border — left edge doubles as the content/chrome
        // separator; right edge hugs the screen. Top/bottom close the box.
        Rectangle::new(rect.top_left, rect.size)
            .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
            .draw(display)?;

        let h = rect.size.height as i32;
        let w = rect.size.width as i32;
        let x_left = rect.top_left.x;
        let y_top = rect.top_left.y;

        // Two horizontal dividers split the column into three equal cells.
        let y_div1 = y_top + h / 3;
        let y_div2 = y_top + (2 * h) / 3;
        Line::new(
            Point::new(x_left, y_div1),
            Point::new(x_left + w - 1, y_div1),
        )
        .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
        .draw(display)?;
        Line::new(
            Point::new(x_left, y_div2),
            Point::new(x_left + w - 1, y_div2),
        )
        .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
        .draw(display)?;

        // Icon (or placeholder dot) centered in each cell.
        let cx = x_left + w / 2;
        let cy_k1 = y_top + h / 6;
        let cy_k2 = y_top + h / 2;
        let cy_k3 = y_top + (5 * h) / 6;

        draw_cell(
            display,
            self.k1,
            Point::new(cx, cy_k1),
            icon_color(self.k1, theme.accent, theme),
            theme.dim,
        )?;
        draw_cell(
            display,
            self.k2,
            Point::new(cx, cy_k2),
            icon_color(self.k2, theme.muted, theme),
            theme.dim,
        )?;
        draw_cell(
            display,
            self.k3,
            Point::new(cx, cy_k3),
            icon_color(self.k3, theme.muted, theme),
            theme.dim,
        )?;

        Ok(())
    }

    /// Touch build: render the hints as a horizontal action bar pinned to the
    /// bottom of the screen instead of a right-edge gutter. The bar is divided
    /// into three equal cells — left = `k3` (Back), middle = `k2` (Secondary),
    /// right = `k1` (Accept) — matching the platform's footer tap-zone thirds.
    /// `rect` is ignored: the bar always spans the full screen width at the
    /// bottom. Cells whose icon is `None` are left blank (no placeholder dot),
    /// so most screens show just Back + Accept.
    #[cfg(feature = "touch-ui")]
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
        _rect: Rectangle,
    ) -> Result<(), D::Error> {
        let w = theme.width as i32;
        let h = FOOTER_H as i32;
        let y_top = theme.height as i32 - h;
        let bar = Rectangle::new(Point::new(0, y_top), Size::new(theme.width, FOOTER_H));

        // Opaque chrome fill so the bar reads on top of live camera frames.
        display.fill_solid(&bar, theme.bg)?;
        // Outer border — top edge doubles as the body/chrome separator.
        bar.into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
            .draw(display)?;

        // Two vertical dividers split the bar into three equal cells.
        let x_div1 = w / 3;
        let x_div2 = (2 * w) / 3;
        Line::new(Point::new(x_div1, y_top), Point::new(x_div1, y_top + h - 1))
            .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
            .draw(display)?;
        Line::new(Point::new(x_div2, y_top), Point::new(x_div2, y_top + h - 1))
            .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
            .draw(display)?;

        let cy = y_top + h / 2;
        let cx_left = w / 6; // k3 — Back
        let cx_mid = w / 2; // k2 — Secondary
        let cx_right = (5 * w) / 6; // k1 — Accept

        // Only the Accept cell (primary action) uses the accent color; the
        // others render in the full-strength text color so they read as
        // active controls rather than disabled grey.
        if !matches!(self.k3, EdgeIcon::None) {
            draw_cell(
                display,
                self.k3,
                Point::new(cx_left, cy),
                icon_color(self.k3, theme.text, theme),
                theme.dim,
            )?;
        }
        if !matches!(self.k2, EdgeIcon::None) {
            draw_cell(
                display,
                self.k2,
                Point::new(cx_mid, cy),
                icon_color(self.k2, theme.text, theme),
                theme.dim,
            )?;
        }
        if !matches!(self.k1, EdgeIcon::None) {
            draw_cell(
                display,
                self.k1,
                Point::new(cx_right, cy),
                icon_color(self.k1, theme.accent, theme),
                theme.dim,
            )?;
        }

        Ok(())
    }
}

/// Stroke color for a cell: the danger color for an explicit reject glyph,
/// otherwise the cell's default emphasis color.
fn icon_color(icon: EdgeIcon, default: Rgb565, theme: &Theme) -> Rgb565 {
    match icon {
        EdgeIcon::CrossDanger => theme.danger,
        _ => default,
    }
}

fn draw_cell<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    icon: EdgeIcon,
    center: Point,
    active: Rgb565,
    placeholder: Rgb565,
) -> Result<(), D::Error> {
    if matches!(icon, EdgeIcon::None) {
        // 3×3 filled square — "key exists but does nothing on this screen".
        Rectangle::new(Point::new(center.x - 1, center.y - 1), Size::new(3, 3))
            .into_styled(PrimitiveStyle::with_fill(placeholder))
            .draw(display)?;
        return Ok(());
    }

    // 3px stroke — visibly heavy on the 240×240 display.
    let style = PrimitiveStyle::with_stroke(active, 3);
    match icon {
        EdgeIcon::None => unreachable!(),
        EdgeIcon::Check => {
            Line::new(
                Point::new(center.x - 7, center.y + 1),
                Point::new(center.x - 2, center.y + 6),
            )
            .into_styled(style)
            .draw(display)?;
            Line::new(
                Point::new(center.x - 2, center.y + 6),
                Point::new(center.x + 7, center.y - 5),
            )
            .into_styled(style)
            .draw(display)?;
        }
        EdgeIcon::Cross | EdgeIcon::CrossDanger => {
            Line::new(
                Point::new(center.x - 6, center.y - 6),
                Point::new(center.x + 6, center.y + 6),
            )
            .into_styled(style)
            .draw(display)?;
            Line::new(
                Point::new(center.x - 6, center.y + 6),
                Point::new(center.x + 6, center.y - 6),
            )
            .into_styled(style)
            .draw(display)?;
        }
        EdgeIcon::ArrowRight => {
            Line::new(
                Point::new(center.x - 7, center.y),
                Point::new(center.x + 7, center.y),
            )
            .into_styled(style)
            .draw(display)?;
            Line::new(
                Point::new(center.x + 2, center.y - 4),
                Point::new(center.x + 7, center.y),
            )
            .into_styled(style)
            .draw(display)?;
            Line::new(
                Point::new(center.x + 2, center.y + 4),
                Point::new(center.x + 7, center.y),
            )
            .into_styled(style)
            .draw(display)?;
        }
        EdgeIcon::ArrowLeft => {
            // Curved return: top horizontal line with a left arrowhead,
            // and a vertical hook dropping from the right end — the
            // "back / return to previous screen" metaphor.
            Line::new(
                Point::new(center.x - 6, center.y - 4),
                Point::new(center.x + 5, center.y - 4),
            )
            .into_styled(style)
            .draw(display)?;
            // Arrowhead at the left end pointing left.
            Line::new(
                Point::new(center.x - 2, center.y - 7),
                Point::new(center.x - 6, center.y - 4),
            )
            .into_styled(style)
            .draw(display)?;
            Line::new(
                Point::new(center.x - 2, center.y - 1),
                Point::new(center.x - 6, center.y - 4),
            )
            .into_styled(style)
            .draw(display)?;
            // Vertical hook dropping down from the right end.
            Line::new(
                Point::new(center.x + 5, center.y - 4),
                Point::new(center.x + 5, center.y + 5),
            )
            .into_styled(style)
            .draw(display)?;
        }
        EdgeIcon::Delete => {
            // Backspace key glyph (⌫): a pentagon "key" pointing left — a
            // rectangle body whose left edge tapers to a point — with a small
            // × inside it. Reads as a keyboard delete key, not a bare arrow.
            //
            // Pentagon outline, traced as connected edges:
            //   tip → upper bend → top-right → bottom-right → lower bend → tip
            let tip = Point::new(center.x - 9, center.y);
            let up_bend = Point::new(center.x - 3, center.y - 7);
            let top_right = Point::new(center.x + 8, center.y - 7);
            let bot_right = Point::new(center.x + 8, center.y + 7);
            let low_bend = Point::new(center.x - 3, center.y + 7);
            for (a, b) in [
                (tip, up_bend),
                (up_bend, top_right),
                (top_right, bot_right),
                (bot_right, low_bend),
                (low_bend, tip),
            ] {
                Line::new(a, b).into_styled(style).draw(display)?;
            }

            // × inside the body, on the right half of the key. Thinner stroke
            // so the cross stays legible against the heavier outline.
            let cross = PrimitiveStyle::with_stroke(active, 2);
            Line::new(
                Point::new(center.x, center.y - 3),
                Point::new(center.x + 6, center.y + 3),
            )
            .into_styled(cross)
            .draw(display)?;
            Line::new(
                Point::new(center.x, center.y + 3),
                Point::new(center.x + 6, center.y - 3),
            )
            .into_styled(cross)
            .draw(display)?;
        }
    }
    Ok(())
}
