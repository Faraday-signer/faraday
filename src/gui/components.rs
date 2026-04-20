//! UI components: cards, status bar, text rendering, option lists, char grid.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_9X15, ascii::FONT_9X15_BOLD, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

use crate::gui::colors;
use crate::gui::icons::{self, Icon};
use crate::gui::app::{CharGrid, GridAction, GRID_COLS};

/// A card in the main menu grid.
pub struct Card<'a> {
    pub label: &'a str,
    pub icon: Icon,
    pub selected: bool,
}

impl<'a> Card<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) -> Result<(), D::Error> {
        let (bg, border, text_color, icon_color) = if self.selected {
            (colors::BG_CARD_SELECTED, colors::BORDER_SELECTED, colors::TEXT_PRIMARY, colors::SOLANA_GREEN)
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_SECONDARY, colors::TEXT_SECONDARY)
        };

        let card_style = PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(1)
            .build();

        RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(x, y), Size::new(w, h)),
            Size::new(6, 6),
        )
        .into_styled(card_style)
        .draw(display)?;

        if self.selected {
            let glow = colors::blend(colors::BG_CARD_SELECTED, colors::ACCENT, 80);
            Rectangle::new(Point::new(x + 2, y + 1), Size::new(w - 4, 2))
                .into_styled(PrimitiveStyle::with_fill(glow))
                .draw(display)?;
        }

        // Icon centered in upper portion
        let icon_x = x + (w as i32 - 16) / 2;
        let icon_y = y + (h as i32) / 2 - 20;
        draw_icon_colored(display, &self.icon, icon_x, icon_y, icon_color)?;

        // Label centered at bottom
        let label_style = MonoTextStyle::new(&FONT_9X15_BOLD, text_color);
        let text_x = x + w as i32 / 2;
        let text_y = y + h as i32 - 10;
        Text::with_alignment(self.label, Point::new(text_x, text_y), label_style, Alignment::Center)
            .draw(display)?;

        Ok(())
    }
}

/// Draw a monochrome icon with a specific color.
pub fn draw_icon_colored<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    icon: &Icon,
    x: i32,
    y: i32,
    color: Rgb565,
) -> Result<(), D::Error> {
    let data = icon.data;
    for row in 0..16i32 {
        let byte_hi = data[row as usize * 2];
        let byte_lo = data[row as usize * 2 + 1];
        let word = ((byte_hi as u16) << 8) | (byte_lo as u16);
        for col in 0..16i32 {
            if (word >> (15 - col)) & 1 == 1 {
                Pixel(Point::new(x + col, y + row), color)
                    .draw(display)?;
            }
        }
    }
    Ok(())
}

/// Status bar at top of screen.
pub fn draw_status_bar<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    Rectangle::new(Point::zero(), Size::new(240, 20))
        .into_styled(PrimitiveStyle::with_fill(colors::BG_DARK))
        .draw(display)?;

    Rectangle::new(Point::new(0, 19), Size::new(240, 1))
        .into_styled(PrimitiveStyle::with_fill(colors::BORDER_DEFAULT))
        .draw(display)?;

    let style = MonoTextStyle::new(&FONT_9X15_BOLD, colors::TEXT_SECONDARY);
    Text::with_alignment(title, Point::new(120, 15), style, Alignment::Center)
        .draw(display)?;

    if seed_loaded {
        Rectangle::new(Point::new(224, 6), Size::new(8, 8))
            .into_styled(PrimitiveStyle::with_fill(colors::SUCCESS))
            .draw(display)?;
    }

    Ok(())
}

/// Draw centered text.
pub fn draw_text_centered<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    text: &str,
    y: i32,
    color: Rgb565,
) -> Result<(), D::Error> {
    let style = MonoTextStyle::new(&FONT_10X20, color);
    Text::with_alignment(text, Point::new(120, y), style, Alignment::Center)
        .draw(display)?;
    Ok(())
}

/// Max visible option rows before the list scrolls. Keeps `draw_option_list`
/// pure w.r.t. list length.
const OPTION_LIST_MAX_VISIBLE: usize = 4;

/// Compute the scroll offset so `selected` stays inside a viewport of
/// `max_visible` rows. Tries to keep the selected row pinned near the
/// bottom while scrolling down; top when scrolling up.
pub fn option_list_scroll_offset(len: usize, selected: usize, max_visible: usize) -> usize {
    if len <= max_visible || max_visible == 0 {
        return 0;
    }
    if selected < max_visible {
        return 0;
    }
    let max_scroll = len - max_visible;
    (selected + 1 - max_visible).min(max_scroll)
}

/// Draw a vertical option list (for simple choice screens). Scrolls when the
/// option count exceeds the visible viewport, keeping the selected row visible.
pub fn draw_option_list<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    options: &[&str],
    selected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, title, seed_loaded)?;

    let max_visible = OPTION_LIST_MAX_VISIBLE;
    let scroll = option_list_scroll_offset(options.len(), selected, max_visible);
    let visible_count = options.len().saturating_sub(scroll).min(max_visible);
    let row_h: i32 = 38;
    let gap: i32 = 7;
    let step: i32 = row_h + gap;
    let total_height: i32 = visible_count as i32 * step - gap;
    let start_y = 24 + (216 - total_height) / 2;

    for row in 0..visible_count {
        let i = scroll + row;
        let option = options[i];
        let y = start_y + row as i32 * step;
        let is_selected = i == selected;

        let (bg, border, text_color) = if is_selected {
            (colors::BG_CARD_SELECTED, colors::BORDER_SELECTED, colors::TEXT_PRIMARY)
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_SECONDARY)
        };

        let style = PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(1)
            .build();

        RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(20, y), Size::new(200, row_h as u32)),
            Size::new(6, 6),
        )
        .into_styled(style)
        .draw(display)?;

        if is_selected {
            let glow = colors::blend(colors::BG_CARD_SELECTED, colors::ACCENT, 80);
            Rectangle::new(Point::new(22, y + 1), Size::new(196, 2))
                .into_styled(PrimitiveStyle::with_fill(glow))
                .draw(display)?;
        }

        let text_style = MonoTextStyle::new(&FONT_9X15_BOLD, text_color);
        Text::with_alignment(option, Point::new(120, y + 24), text_style, Alignment::Center)
            .draw(display)?;
    }

    // Scroll indicators when off-screen rows exist.
    let arrow_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    if scroll > 0 {
        Text::with_alignment("▲", Point::new(230, 30), arrow_style, Alignment::Center).draw(display)?;
    }
    if scroll + max_visible < options.len() {
        Text::with_alignment("▼", Point::new(230, 235), arrow_style, Alignment::Center).draw(display)?;
    }

    Ok(())
}

/// Draw a two-button bar at the bottom (e.g., Confirm/Cancel).
pub fn draw_button_bar<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    left: &str,
    right: &str,
    selected: usize,
) -> Result<(), D::Error> {
    draw_button_bar_ex(display, left, right, selected, true)
}

/// Button bar with an optional disabled-state for the left button.
/// When `left_enabled` is false, the left button renders muted and
/// selection should not land on it.
pub fn draw_button_bar_ex<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    left: &str,
    right: &str,
    selected: usize,
    left_enabled: bool,
) -> Result<(), D::Error> {
    let y = 210i32;
    let btn_w = 105u32;

    for (i, label) in [left, right].iter().enumerate() {
        let x = if i == 0 { 10 } else { 125 };
        let is_selected = i == selected;
        let is_disabled = i == 0 && !left_enabled;

        let (bg, border, text_color) = if is_disabled {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_MUTED)
        } else if is_selected {
            if i == 0 {
                (colors::BG_CARD_SELECTED, colors::SUCCESS, colors::SUCCESS)
            } else {
                (colors::BG_CARD_SELECTED, colors::DANGER, colors::DANGER)
            }
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_MUTED)
        };

        let style = PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(1)
            .build();

        RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(x, y), Size::new(btn_w, 26)),
            Size::new(4, 4),
        )
        .into_styled(style)
        .draw(display)?;

        let text_style = MonoTextStyle::new(&FONT_9X15_BOLD, text_color);
        Text::with_alignment(label, Point::new(x + btn_w as i32 / 2, y + 18), text_style, Alignment::Center)
            .draw(display)?;
    }

    Ok(())
}

/// Draw the passphrase character grid.
pub fn draw_char_grid<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    grid: &CharGrid,
    title: &str,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, title, seed_loaded)?;

    // Keyboard icon next to title
    draw_icon_colored(display, &icons::keyboard(), 5, 3, colors::TEXT_MUTED)?;

    // Show entered text (masked except last char)
    let display_text = if grid.text.is_empty() {
        String::from("_")
    } else {
        let len = grid.text.len();
        let masked: String = (0..len - 1).map(|_| '*').collect();
        let last = grid.text.chars().last().unwrap();
        format!("{}{}", masked, last)
    };
    // Truncate display if too long
    let show: &str = if display_text.len() > 22 {
        &display_text[display_text.len() - 22..]
    } else {
        &display_text
    };
    let text_style = MonoTextStyle::new(&FONT_9X15_BOLD, colors::TEXT_PRIMARY);
    Text::with_alignment(show, Point::new(120, 40), text_style, Alignment::Center)
        .draw(display)?;

    // Separator
    Rectangle::new(Point::new(10, 48), Size::new(220, 1))
        .into_styled(PrimitiveStyle::with_fill(colors::BORDER_DEFAULT))
        .draw(display)?;

    // Character grid (5 rows of 10 chars)
    let grid_start_x = 12i32;
    let grid_start_y = 55i32;
    let cell_w = 22i32;
    let cell_h = 22i32;

    let char_style_normal = MonoTextStyle::new(&FONT_9X15, colors::TEXT_SECONDARY);
    let char_style_selected = MonoTextStyle::new(&FONT_9X15_BOLD, colors::TEXT_PRIMARY);

    let chars = [
        ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
        ['k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't'],
        ['u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3'],
        ['4', '5', '6', '7', '8', '9', '!', '@', '#', '$'],
        ['%', '^', '&', '*', '.', '-', '_', '+', '=', '/'],
    ];

    for row in 0..5usize {
        for col in 0..GRID_COLS {
            let x = grid_start_x + col as i32 * cell_w;
            let y = grid_start_y + row as i32 * cell_h;
            let is_selected = grid.row == row && grid.col == col;

            if is_selected {
                let sel_style = PrimitiveStyleBuilder::new()
                    .fill_color(colors::BG_CARD_SELECTED)
                    .stroke_color(colors::BORDER_SELECTED)
                    .stroke_width(1)
                    .build();
                Rectangle::new(Point::new(x - 1, y - 1), Size::new(cell_w as u32, cell_h as u32))
                    .into_styled(sel_style)
                    .draw(display)?;
            }

            let mut ch = chars[row][col];
            if grid.caps && ch.is_ascii_lowercase() {
                ch = ch.to_ascii_uppercase();
            }

            let style = if is_selected { char_style_selected } else { char_style_normal };
            let ch_str = alloc::string::String::from(ch);
            Text::with_alignment(
                &ch_str,
                Point::new(x + cell_w / 2 - 1, y + 14),
                style,
                Alignment::Center,
            )
            .draw(display)?;
        }
    }

    // Action row (row 5)
    let action_y = grid_start_y + 5 * cell_h;
    let actions = [
        (0, 2, "SPC"),
        (2, 2, if grid.caps { "abc" } else { "ABC" }),
        (4, 3, "DEL"),
        (7, 3, "DONE"),
    ];

    let active_action = grid.action_region();

    for &(start_col, span, label) in &actions {
        let x = grid_start_x + start_col as i32 * cell_w;
        let w = (span as i32 * cell_w) as u32;

        let action_kind = match start_col {
            0 => GridAction::Space,
            2 => GridAction::Caps,
            4 => GridAction::Delete,
            _ => GridAction::Done,
        };
        let is_selected = active_action == Some(action_kind);

        let (bg, border, text_color) = if is_selected {
            if label == "DONE" {
                (colors::BG_CARD_SELECTED, colors::SUCCESS, colors::SUCCESS)
            } else if label == "DEL" {
                (colors::BG_CARD_SELECTED, colors::DANGER, colors::DANGER)
            } else {
                (colors::BG_CARD_SELECTED, colors::BORDER_SELECTED, colors::TEXT_PRIMARY)
            }
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_MUTED)
        };

        let style = PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(1)
            .build();

        RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(x, action_y), Size::new(w - 2, cell_h as u32)),
            Size::new(3, 3),
        )
        .into_styled(style)
        .draw(display)?;

        let text_style = MonoTextStyle::new(&FONT_6X10, text_color);
        Text::with_alignment(
            label,
            Point::new(x + w as i32 / 2 - 1, action_y + 15),
            text_style,
            Alignment::Center,
        )
        .draw(display)?;
    }

    // Hint bar
    let hint_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(
        "X:caps  Esc:del/back",
        Point::new(120, 235),
        hint_style,
        Alignment::Center,
    )
    .draw(display)?;

    Ok(())
}

/// Draw the BIP39 word picker screen.
pub fn draw_word_picker<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    picker: &crate::gui::app::WordPicker,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;

    let title = alloc::format!("Word {} of {}", picker.word_index + 1, picker.word_count);
    draw_status_bar(display, &title, seed_loaded)?;

    // Show already entered words count
    if !picker.words.is_empty() {
        let entered = alloc::format!("Entered: {}", picker.words.len());
        let style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
        Text::with_alignment(&entered, Point::new(120, 34), style, Alignment::Center)
            .draw(display)?;
    }

    // Current prefix + cursor character
    let preview = alloc::format!("{}[{}]", picker.prefix, picker.current_char());
    let prefix_style = MonoTextStyle::new(&FONT_10X20, colors::TEXT_PRIMARY);
    Text::with_alignment(&preview, Point::new(120, 58), prefix_style, Alignment::Center)
        .draw(display)?;

    // Hint: left/right to change letter, X to append
    let hint_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(
        "</>:letter  X:add  Enter:pick",
        Point::new(120, 72),
        hint_style,
        Alignment::Center,
    )
    .draw(display)?;

    // Separator
    Rectangle::new(Point::new(10, 76), Size::new(220, 1))
        .into_styled(PrimitiveStyle::with_fill(colors::BORDER_DEFAULT))
        .draw(display)?;

    // Filtered word list
    let filtered = picker.filtered_words();
    let max_visible = 8usize;
    let scroll_offset = if picker.list_selected >= max_visible {
        picker.list_selected - max_visible + 1
    } else {
        0
    };

    let list_style_normal = MonoTextStyle::new(&FONT_9X15, colors::TEXT_SECONDARY);
    let list_style_selected = MonoTextStyle::new(&FONT_9X15_BOLD, colors::TEXT_PRIMARY);

    for (vi, i) in (scroll_offset..filtered.len().min(scroll_offset + max_visible)).enumerate() {
        let y = 90 + vi as i32 * 18;
        let (_idx, word) = filtered[i];
        let is_selected = i == picker.list_selected;

        if is_selected {
            Rectangle::new(Point::new(10, y - 13), Size::new(220, 17))
                .into_styled(PrimitiveStyle::with_fill(colors::BG_CARD_SELECTED))
                .draw(display)?;
        }

        let style = if is_selected { list_style_selected } else { list_style_normal };
        let display_str = alloc::format!("  {}", word);
        Text::new(&display_str, Point::new(15, y), style)
            .draw(display)?;
    }

    if filtered.is_empty() {
        let style = MonoTextStyle::new(&FONT_9X15, colors::TEXT_MUTED);
        Text::with_alignment("(no matches)", Point::new(120, 120), style, Alignment::Center)
            .draw(display)?;
    }

    Ok(())
}

/// Draw a QR code centered on screen.
pub fn draw_qr<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    data: &str,
    seed_loaded: bool,
    ec: crate::qr::encode_qr::QrEcLevel,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, title, seed_loaded)?;

    if let Ok((matrix, size)) = crate::qr::encode_qr::generate_qr_matrix(data, ec) {
        let max_area = 200u32;
        let module_size = max_area / size as u32;
        let qr_size = module_size * size as u32;
        let offset_x = (240 - qr_size as i32) / 2;
        let offset_y = 25 + (210 - qr_size as i32) / 2;

        // White background behind QR
        Rectangle::new(
            Point::new(offset_x - 4, offset_y - 4),
            Size::new(qr_size + 8, qr_size + 8),
        )
        .into_styled(PrimitiveStyle::with_fill(colors::WHITE))
        .draw(display)?;

        // Draw QR modules
        for qr_y in 0..size {
            for qr_x in 0..size {
                if matrix[qr_y * size + qr_x] {
                    Rectangle::new(
                        Point::new(
                            offset_x + qr_x as i32 * module_size as i32,
                            offset_y + qr_y as i32 * module_size as i32,
                        ),
                        Size::new(module_size, module_size),
                    )
                    .into_styled(PrimitiveStyle::with_fill(colors::BLACK))
                    .draw(display)?;
                }
            }
        }
    }

    Ok(())
}

extern crate alloc;

#[cfg(test)]
mod scroll_tests {
    use super::option_list_scroll_offset;

    #[test]
    fn small_list_never_scrolls() {
        for sel in 0..5 {
            assert_eq!(option_list_scroll_offset(5, sel, 5), 0);
        }
    }

    #[test]
    fn initial_items_in_large_list_dont_scroll() {
        // With 6 items and max=5, scroll stays 0 while selected is 0..4.
        for sel in 0..5 {
            assert_eq!(option_list_scroll_offset(6, sel, 5), 0, "sel={sel}");
        }
    }

    #[test]
    fn scrolls_to_keep_selected_visible() {
        // 6 items, max=5, selected=5 (last item) → scroll by 1.
        assert_eq!(option_list_scroll_offset(6, 5, 5), 1);
    }

    #[test]
    fn scroll_clamped_to_max() {
        // 8 items, max=5. Scroll offset never exceeds len - max = 3.
        assert_eq!(option_list_scroll_offset(8, 7, 5), 3);
        // Even if we somehow asked for a selected past the end (shouldn't
        // happen in practice), we still clamp.
        assert_eq!(option_list_scroll_offset(8, 100, 5), 3);
    }

    #[test]
    fn zero_visible_returns_zero_safely() {
        // Defensive: avoid subtract-overflow if caller passes 0.
        assert_eq!(option_list_scroll_offset(5, 2, 0), 0);
    }

    #[test]
    fn empty_list_is_harmless() {
        assert_eq!(option_list_scroll_offset(0, 0, 5), 0);
    }
}
