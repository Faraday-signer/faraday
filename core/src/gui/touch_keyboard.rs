//! Touch-only on-screen keyboard for passphrase / message entry.
//!
//! Replaces the shared `CharGrid` navigation grid on touch builds with a
//! split QWERTY keyboard sized for the small portrait display: the alphabet
//! is divided into a **left** and a **right** half (swipe left/right to
//! switch), plus a **symbols** page reached with the `SYM` key and left
//! with `ABC`. A text box at the top shows the buffer; a slider beneath it
//! indicates which half is active.
//!
//! Accept / Back live in the bottom action bar (the shared touch footer),
//! not in the keyboard itself — see `EdgeHints`. The only state this owns
//! lives on `CharGrid` (`text`, `caps`, `page`); rendering and hit-testing
//! are pure functions of that state plus the `Theme`.

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{
        CornerRadii, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
    },
    text::{Alignment, Text},
    Drawable,
};

use crate::gui::app::CharGrid;
use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H};
use crate::ui::Theme;

/// Which keyboard page is showing. Preserved across taps and stored on
/// `CharGrid` so the active half survives screen redraws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KbPage {
    Left,
    Right,
    Symbols,
}

/// A single key. `Char` carries the lowercase letter / literal symbol; the
/// caller upper-cases letters when caps-lock is on. The rest are controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Shift,
    Sym,
    Abc,
    Space,
    Backspace,
}

/// One key cell: the key plus a layout weight. Cells in a row split the
/// full body width in proportion to their weights, so a wide `space` is
/// just a heavier weight.
type Cell = (Key, u8);
type Row = &'static [Cell];

// Left QWERTY half. Bottom row: SYM | space | backspace.
const LEFT: &[Row] = &[
    &[
        (Key::Char('q'), 1),
        (Key::Char('w'), 1),
        (Key::Char('e'), 1),
        (Key::Char('r'), 1),
        (Key::Char('t'), 1),
    ],
    &[
        (Key::Char('a'), 1),
        (Key::Char('s'), 1),
        (Key::Char('d'), 1),
        (Key::Char('f'), 1),
        (Key::Char('g'), 1),
    ],
    &[
        (Key::Shift, 1),
        (Key::Char('z'), 1),
        (Key::Char('x'), 1),
        (Key::Char('c'), 1),
        (Key::Char('v'), 1),
        (Key::Char('b'), 1),
    ],
    &[(Key::Sym, 3), (Key::Space, 8), (Key::Backspace, 3)],
];

// Right QWERTY half. Shift lives on the right here, mirroring the left page.
const RIGHT: &[Row] = &[
    &[
        (Key::Char('y'), 1),
        (Key::Char('u'), 1),
        (Key::Char('i'), 1),
        (Key::Char('o'), 1),
        (Key::Char('p'), 1),
    ],
    &[
        (Key::Char('h'), 1),
        (Key::Char('j'), 1),
        (Key::Char('k'), 1),
        (Key::Char('l'), 1),
        (Key::Char(';'), 1),
        (Key::Char('\''), 1),
    ],
    &[
        (Key::Char('n'), 1),
        (Key::Char('m'), 1),
        (Key::Char(','), 1),
        (Key::Char('.'), 1),
        (Key::Char('/'), 1),
        (Key::Shift, 1),
    ],
    &[(Key::Sym, 3), (Key::Space, 8), (Key::Backspace, 3)],
];

// Symbols page. The QWERTY halves carry no digits, so the digit row lives
// here at the top. Bottom row: ABC (back to letters) | space.
const SYMBOLS: &[Row] = &[
    &[
        (Key::Char('1'), 1),
        (Key::Char('2'), 1),
        (Key::Char('3'), 1),
        (Key::Char('4'), 1),
        (Key::Char('5'), 1),
        (Key::Char('6'), 1),
        (Key::Char('7'), 1),
        (Key::Char('8'), 1),
        (Key::Char('9'), 1),
        (Key::Char('0'), 1),
    ],
    &[
        (Key::Char('~'), 1),
        (Key::Char('`'), 1),
        (Key::Char('!'), 1),
        (Key::Char('@'), 1),
        (Key::Char('#'), 1),
        (Key::Char('$'), 1),
        (Key::Char('%'), 1),
    ],
    &[
        (Key::Char('^'), 1),
        (Key::Char('&'), 1),
        (Key::Char('*'), 1),
        (Key::Char('('), 1),
        (Key::Char(')'), 1),
        (Key::Char('_'), 1),
        (Key::Char('-'), 1),
    ],
    &[
        (Key::Char('+'), 1),
        (Key::Char('='), 1),
        (Key::Char('['), 1),
        (Key::Char(']'), 1),
        (Key::Char('{'), 1),
        (Key::Char('}'), 1),
        (Key::Char('|'), 1),
    ],
    &[
        (Key::Char('\\'), 1),
        (Key::Char('<'), 1),
        (Key::Char('>'), 1),
        (Key::Backspace, 4),
    ],
    &[(Key::Abc, 2), (Key::Space, 5)],
];

impl KbPage {
    fn rows(self) -> &'static [Row] {
        match self {
            KbPage::Left => LEFT,
            KbPage::Right => RIGHT,
            KbPage::Symbols => SYMBOLS,
        }
    }
}

// Layout constants (pixels), measured down from the header.
const HPAD: i32 = 8; // outer horizontal padding for the text box
const TB_TOP_GAP: i32 = 6; // gap below the header before the text box
const TB_H: i32 = 38; // text box height
const SLIDER_H: i32 = 30; // height of the slider / "SYMBOLS" band
const KEY_INSET: i32 = 3; // gap between a key's cell and its drawn rect
const CHAR_W: i32 = 10; // approx profont17 advance, for text-box clipping

fn keys_top(theme: &Theme) -> i32 {
    theme.header_h as i32 + TB_TOP_GAP + TB_H + SLIDER_H
}

fn keys_bottom(theme: &Theme) -> i32 {
    theme.height as i32 - FOOTER_H as i32
}

/// Map a body tap `(x, y)` to the key under it, or `None` if the tap falls
/// above the key area (text box / slider band). The caller has already
/// excluded the footer.
pub fn hit_test(theme: &Theme, page: KbPage, x: u16, y: u16) -> Option<Key> {
    let top = keys_top(theme);
    let bottom = keys_bottom(theme);
    let (x, y) = (x as i32, y as i32);
    if y < top || y >= bottom {
        return None;
    }
    let rows = page.rows();
    let row_h = (bottom - top) / rows.len() as i32;
    if row_h <= 0 {
        return None;
    }
    let row_idx = ((y - top) / row_h) as usize;
    let row = rows.get(row_idx)?;

    let total: u32 = row.iter().map(|(_, w)| *w as u32).sum();
    let w = theme.width;
    let mut acc: u32 = 0;
    for (key, weight) in row.iter() {
        let x0 = (acc * w / total) as i32;
        acc += *weight as u32;
        let x1 = (acc * w / total) as i32;
        if x >= x0 && x < x1 {
            return Some(*key);
        }
    }
    None
}

/// Render the full keyboard screen: header, text box, slider, keys, and the
/// Back / Accept action bar.
pub fn draw<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    grid: &CharGrid,
    title: &str,
    cursor_on: bool,
) -> Result<(), D::Error> {
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let header_rect = Rectangle::new(Point::zero(), Size::new(theme.width, theme.header_h));
    Header {
        kind: HeaderKind::Title(title),
        counter: None,
        right_label: None,
    }
    .draw(display, theme, header_rect)?;

    draw_textbox(display, theme, &grid.text, cursor_on)?;

    let band = Rectangle::new(
        Point::new(0, theme.header_h as i32 + TB_TOP_GAP + TB_H),
        Size::new(theme.width, SLIDER_H as u32),
    );
    if grid.page == KbPage::Symbols {
        Text::with_alignment(
            "SYMBOLS",
            Point::new(
                theme.width as i32 / 2,
                band.top_left.y + SLIDER_H / 2 + 6,
            ),
            theme.style_sm(theme.accent),
            Alignment::Center,
        )
        .draw(display)?;
    } else {
        draw_slider(display, theme, band, grid.page)?;
    }

    draw_keys(display, theme, grid)?;

    // Bottom action bar: Back (left) + Accept (right). The middle cell is
    // blank — backspace lives on the keyboard. `EdgeHints` on touch builds
    // renders this as the horizontal footer and ignores the passed rect.
    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k3(EdgeIcon::ArrowLeft)
        .draw(display, theme, screen)?;

    Ok(())
}

/// Text box showing the live buffer with a trailing cursor. When the text
/// is longer than fits, the tail (most recent characters) is shown.
fn draw_textbox<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    text: &str,
    cursor_on: bool,
) -> Result<(), D::Error> {
    let rect = Rectangle::new(
        Point::new(HPAD, theme.header_h as i32 + TB_TOP_GAP),
        Size::new((theme.width as i32 - 2 * HPAD) as u32, TB_H as u32),
    );
    RoundedRectangle::with_equal_corners(rect, Size::new(6, 6))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(theme.border)
                .stroke_width(1)
                .fill_color(theme.bg)
                .build(),
        )
        .draw(display)?;

    let inner_left = rect.top_left.x + 8;
    let baseline = rect.top_left.y + TB_H / 2 + 6;
    let inner_w = rect.size.width as i32 - 16;
    let max_chars = (inner_w / CHAR_W - 1).max(0) as usize;

    let n = text.chars().count();
    let visible: String = if n > max_chars {
        text.chars().skip(n - max_chars).collect()
    } else {
        text.into()
    };
    // Text in the foreground color; the trailing cursor drawn separately in
    // grey so it reads as a caret rather than part of the passphrase. The
    // caret blinks at 1 Hz (`cursor_on`).
    let end = Text::with_alignment(
        &visible,
        Point::new(inner_left, baseline),
        theme.style_sm(theme.text),
        Alignment::Left,
    )
    .draw(display)?;
    if cursor_on {
        Text::with_alignment("|", end, theme.style_sm(theme.muted), Alignment::Left)
            .draw(display)?;
    }
    Ok(())
}

/// Two-segment slider under the text box: labels above a split track whose
/// active half is filled with the accent color.
fn draw_slider<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    band: Rectangle,
    page: KbPage,
) -> Result<(), D::Error> {
    let cx = theme.width as i32 / 2;
    let on_left = page == KbPage::Left;

    // Labels: the active half is drawn in the text color, the other dim.
    let label_y = band.top_left.y + 12;
    Text::with_alignment(
        "left",
        Point::new(cx - 16, label_y),
        theme.style_sm(if on_left { theme.text } else { theme.dim }),
        Alignment::Right,
    )
    .draw(display)?;
    Text::with_alignment(
        "right",
        Point::new(cx + 16, label_y),
        theme.style_sm(if on_left { theme.dim } else { theme.text }),
        Alignment::Left,
    )
    .draw(display)?;

    // Split track: full bar dim, active half accent.
    let half = 50i32;
    let track_y = band.top_left.y + SLIDER_H - 9;
    let track = Rectangle::new(
        Point::new(cx - half, track_y),
        Size::new((half * 2) as u32, 4),
    );
    RoundedRectangle::with_equal_corners(track, Size::new(2, 2))
        .into_styled(PrimitiveStyle::with_fill(theme.dim))
        .draw(display)?;
    let active_x = if on_left { cx - half } else { cx };
    let active = Rectangle::new(Point::new(active_x, track_y), Size::new(half as u32, 4));
    RoundedRectangle::with_equal_corners(active, Size::new(2, 2))
        .into_styled(PrimitiveStyle::with_fill(theme.accent))
        .draw(display)?;
    Ok(())
}

fn draw_keys<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    grid: &CharGrid,
) -> Result<(), D::Error> {
    let top = keys_top(theme);
    let bottom = keys_bottom(theme);
    let rows = grid.page.rows();
    let row_h = (bottom - top) / rows.len() as i32;
    let w = theme.width;

    for (r, row) in rows.iter().enumerate() {
        let total: u32 = row.iter().map(|(_, w)| *w as u32).sum();
        let y = top + r as i32 * row_h;
        let mut acc: u32 = 0;
        for (key, weight) in row.iter() {
            let x0 = (acc * w / total) as i32;
            acc += *weight as u32;
            let x1 = (acc * w / total) as i32;
            let cell = Rectangle::new(
                Point::new(x0, y),
                Size::new((x1 - x0) as u32, row_h as u32),
            );
            draw_key(display, theme, *key, cell, grid.caps)?;
        }
    }
    Ok(())
}

fn draw_key<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    key: Key,
    cell: Rectangle,
    caps: bool,
) -> Result<(), D::Error> {
    let rect = Rectangle::new(
        Point::new(cell.top_left.x + KEY_INSET, cell.top_left.y + KEY_INSET),
        Size::new(
            cell.size.width.saturating_sub(2 * KEY_INSET as u32),
            cell.size.height.saturating_sub(2 * KEY_INSET as u32),
        ),
    );

    // Shift is highlighted (accent fill) while caps-lock is active.
    let highlighted = matches!(key, Key::Shift) && caps;
    let fill = if highlighted { theme.accent } else { theme.bg };
    RoundedRectangle::new(rect, CornerRadii::new(Size::new(6, 6)))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(theme.border)
                .stroke_width(1)
                .fill_color(fill)
                .build(),
        )
        .draw(display)?;

    let cx = rect.top_left.x + rect.size.width as i32 / 2;
    let cy = rect.top_left.y + rect.size.height as i32 / 2;
    let center = Point::new(cx, cy);

    match key {
        Key::Char(c) => {
            let c = if caps && c.is_ascii_lowercase() {
                c.to_ascii_uppercase()
            } else {
                c
            };
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            Text::with_alignment(
                s,
                Point::new(cx, cy + 6),
                theme.style_sm(theme.text),
                Alignment::Center,
            )
            .draw(display)?;
        }
        Key::Shift => {
            let color = if highlighted { theme.bg } else { theme.text };
            draw_shift(display, center, color)?;
        }
        Key::Backspace => draw_backspace(display, center, theme.text)?,
        // SYM / ABC are mode switches — drawn in accent to read as actions.
        Key::Sym => label(display, theme, center, "SYM", theme.accent)?,
        Key::Abc => label(display, theme, center, "ABC", theme.accent)?,
        Key::Space => label(display, theme, center, "space", theme.text)?,
    }
    Ok(())
}

fn label<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    center: Point,
    text: &str,
    color: Rgb565,
) -> Result<(), D::Error> {
    Text::with_alignment(
        text,
        Point::new(center.x, center.y + 6),
        theme.style_sm(color),
        Alignment::Center,
    )
    .draw(display)?;
    Ok(())
}

/// Up-arrow (caps) glyph: a chevron over a short stem.
fn draw_shift<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    center: Point,
    color: Rgb565,
) -> Result<(), D::Error> {
    let style = PrimitiveStyle::with_stroke(color, 2);
    let (cx, cy) = (center.x, center.y);
    Line::new(Point::new(cx - 6, cy + 1), Point::new(cx, cy - 6))
        .into_styled(style)
        .draw(display)?;
    Line::new(Point::new(cx + 6, cy + 1), Point::new(cx, cy - 6))
        .into_styled(style)
        .draw(display)?;
    Line::new(Point::new(cx, cy - 6), Point::new(cx, cy + 6))
        .into_styled(style)
        .draw(display)?;
    Ok(())
}

/// Backspace glyph: a left-pointing arrow with a small × in its body.
fn draw_backspace<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    center: Point,
    color: Rgb565,
) -> Result<(), D::Error> {
    let style = PrimitiveStyle::with_stroke(color, 2);
    let (cx, cy) = (center.x, center.y);
    // Shaft + arrowhead pointing left.
    Line::new(Point::new(cx - 8, cy), Point::new(cx + 8, cy))
        .into_styled(style)
        .draw(display)?;
    Line::new(Point::new(cx - 8, cy), Point::new(cx - 3, cy - 5))
        .into_styled(style)
        .draw(display)?;
    Line::new(Point::new(cx - 8, cy), Point::new(cx - 3, cy + 5))
        .into_styled(style)
        .draw(display)?;
    // × near the right end.
    Line::new(Point::new(cx + 2, cy - 4), Point::new(cx + 8, cy + 4))
        .into_styled(style)
        .draw(display)?;
    Line::new(Point::new(cx + 2, cy + 4), Point::new(cx + 8, cy - 4))
        .into_styled(style)
        .draw(display)?;
    Ok(())
}
