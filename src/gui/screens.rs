//! Screen layouts — all UI pages.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_9X15, ascii::FONT_9X15_BOLD, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};
use u8g2_fonts::{fonts, U8g2TextStyle};

use crate::gui::app::{App, Screen};
use crate::gui::colors;
use crate::gui::components::{
    Card, draw_button_bar, draw_char_grid, draw_option_list,
    draw_qr, draw_status_bar, draw_text_centered, draw_word_picker,
};
use crate::gui::icons;
use crate::gui::logo;

/// Menu item. Brutalist layout: one hero label + subtitle at a time.
struct MenuItem {
    label: &'static str,
    subtitle: &'static str,
}

const MENU_ITEMS: [MenuItem; 4] = [
    MenuItem { label: "CREATE",   subtitle: "a new wallet" },
    MenuItem { label: "LOAD",     subtitle: "an existing wallet" },
    MenuItem { label: "SIGN TX",  subtitle: "a transaction" },
    MenuItem { label: "SETTINGS", subtitle: "and device info" },
];

impl App {
    /// Draw the current screen.
    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        match &self.screen {
            Screen::Splash => draw_splash(display),

            Screen::MainMenu { selected } => {
                let addr = self.wallet.as_ref().map(|w| w.address.as_str());
                draw_main_menu(display, *selected, self.seed_loaded(), addr)
            }

            // Create flow
            Screen::CreateWordCount { selected } => {
                draw_create_word_count(display, *selected)
            }
            Screen::CreateMethod { selected, .. } => {
                draw_create_method(display, *selected)
            }
            Screen::CreateCameraEntropy { word_count, frames_collected, .. } => {
                draw_camera_entropy(display, *word_count, *frames_collected, self.seed_loaded())
            }
            Screen::CreateCoinFlips { word_count, bits, selected } => {
                draw_coin_flips(display, *word_count, bits, *selected, self.seed_loaded())
            }
            Screen::CreateDiceRolls { word_count, rolls, selected } => {
                draw_dice_rolls(display, *word_count, rolls, *selected, self.seed_loaded())
            }
            Screen::CreateShowWords { mnemonic, page, word_count } => {
                draw_show_words(display, mnemonic, *page, *word_count, self.seed_loaded())
            }
            Screen::CreateVerify { checks, current, options, correct_idx: _, selected, mnemonic: _ } => {
                let word_num = checks[*current] + 1;
                draw_verify_word(display, word_num, options, *selected, *current + 1, checks.len())
            }
            Screen::CreatePassphrasePrompt { selected, .. } => {
                draw_passphrase_prompt(display, *selected)
            }
            Screen::CreatePassphraseInput { grid, .. } => {
                draw_passphrase_grid(display, grid, "PASSPHRASE")
            }
            Screen::CreatePassphraseConfirm { grid, .. } => {
                draw_passphrase_grid(display, grid, "CONFIRM")
            }
            Screen::CreatePassphraseMismatch { .. } => {
                draw_passphrase_mismatch(display, self.seed_loaded())
            }
            Screen::CreateConfirm { address, .. } => {
                draw_wallet_confirm(display, "NEW WALLET", address)
            }
            Screen::ExportSeedWarning { selected, .. } => {
                draw_export_seed_warning(display, *selected)
            }
            Screen::ExportSeedQr { seed_qr_data, compact_data, compact_mode, .. } => {
                let hex_data;
                let data: &str = if *compact_mode {
                    hex_data = hex::encode(compact_data);
                    &hex_data
                } else {
                    seed_qr_data
                };
                draw_seed_qr(display, data)
            }

            // Load flow
            Screen::LoadMethod { selected } => {
                draw_load_method(display, *selected)
            }
            Screen::LoadScanQr => {
                #[cfg(feature = "simulator")]
                {
                    draw_message(display, "Scan SeedQR", "Simulator mode:\nEnter = load test wallet\nEsc = back", self.seed_loaded())?;
                }
                #[cfg(not(feature = "simulator"))]
                {
                    draw_message(display, "Scan SeedQR", "Point camera at\nSeedQR code", self.seed_loaded())?;
                }
                Ok(())
            }
            Screen::LoadWordCount { selected } => {
                // Same visual as Create's word-count picker — the choice is
                // the same, only the state-machine edges differ.
                draw_create_word_count(display, *selected)
            }
            Screen::LoadEnterWords { picker, .. } => {
                draw_word_picker(display, picker, self.seed_loaded())
            }
            Screen::LoadInvalidMnemonic { word_count } => {
                draw_invalid_mnemonic(display, *word_count)
            }
            Screen::LoadPassphrasePrompt { selected, .. } => {
                draw_passphrase_prompt(display, *selected)
            }
            Screen::LoadPassphraseInput { grid, .. } => {
                draw_passphrase_grid(display, grid, "PASSPHRASE")
            }
            Screen::LoadPassphraseConfirm { grid, .. } => {
                draw_passphrase_grid(display, grid, "CONFIRM")
            }
            Screen::LoadPassphraseMismatch { .. } => {
                draw_passphrase_mismatch(display, self.seed_loaded())
            }
            Screen::LoadConfirm { address, .. } => {
                draw_wallet_confirm(display, "LOAD WALLET", address)
            }

            // Sign TX flow
            Screen::SignNoWallet => {
                draw_message(display, "Sign TX", "Load a wallet first", self.seed_loaded())
            }
            Screen::SignScanTx => {
                draw_message(display, "Sign TX", "Scan unsigned TX QR\nX: Sign Message", self.seed_loaded())
            }
            Screen::SignReview { info_lines, scroll, selected, .. } => {
                draw_tx_review(display, info_lines, *scroll, *selected, self.seed_loaded())
            }
            Screen::SignShowQr { data } => {
                draw_qr(display, "Signed TX", data, self.seed_loaded())
            }
            Screen::SignMessageInput { grid } => {
                draw_passphrase_grid(display, grid, "SIGN MSG")
            }
            Screen::SignMessageResult { signature_hex } => {
                draw_qr(display, "Signature", signature_hex, self.seed_loaded())
            }

            // Settings
            Screen::SettingsMenu { selected } => {
                draw_settings_menu(display, *selected, self.seed_loaded())
            }
            Screen::SettingsAccounts { accounts, selected } => {
                draw_accounts(display, accounts, *selected, self.seed_loaded())
            }
            Screen::SettingsShowAddress => {
                draw_show_address(display, self.wallet.as_ref().map(|w| w.address.as_str()))
            }
            Screen::SettingsAbout => {
                draw_about(display, self.seed_loaded())
            }
            Screen::SettingsPowerOff { selected } => {
                draw_power_off(display, *selected)
            }
        }
    }
}

/// Splash / reposo screen. Full-pixel-art Faraday logo, centered, at 2x scale
/// on the dark-navy background. Doubles as the idle screen.
pub fn draw_splash<D: DrawTarget<Color = Rgb565>>(display: &mut D) -> Result<(), D::Error> {
    display.clear(colors::FD_BG)?;

    let scale: u32 = 2;
    let logo_w = logo::LOGO_WIDTH * scale;
    let logo_h = logo::LOGO_HEIGHT * scale;
    let x = (240 - logo_w as i32) / 2;
    let y = (240 - logo_h as i32) / 2;
    logo::draw_logo(display, x, y, scale, colors::FD_ACCENT)?;

    Ok(())
}

/// Main menu: list register (Header + List + ButtonBar) via `src/ui/`.
fn draw_main_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
    _seed_loaded: bool,
    _address: Option<&str>,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let total = MENU_ITEMS.len();
    let sel = selected.min(total - 1);

    let rows: [ListRow; 4] = [
        ListRow::with_subtitle(MENU_ITEMS[0].label, MENU_ITEMS[0].subtitle),
        ListRow::with_subtitle(MENU_ITEMS[1].label, MENU_ITEMS[1].subtitle),
        ListRow::with_subtitle(MENU_ITEMS[2].label, MENU_ITEMS[2].subtitle),
        ListRow::with_subtitle(MENU_ITEMS[3].label, MENU_ITEMS[3].subtitle),
    ];

    ListScreen {
        header: HeaderKind::Brand,
        counter: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("SELECT"),
    }
    .draw(display, &theme)
}

/// Invalid mnemonic card. Shown when the 12/24 entered words don't form a
/// valid BIP39 seed. CONFIRM retries from scratch, BACK bails out of Load.
fn draw_invalid_mnemonic<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, CardRow, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();
    let count_str = if word_count == 12 { "12 WORDS" } else { "24 WORDS" };
    let rows: [CardRow; 2] = [
        CardRow::new("ENTERED", count_str),
        CardRow::new("CHECK", "BIP39 failed"),
    ];
    let body = [
        "Those words do not form",
        "a valid recovery seed.",
        "Check spelling and order.",
    ];

    CardScreen {
        header: HeaderKind::Title("INVALID SEED"),
        counter: None,
        title: Some("NO MATCH"),
        subtitle: Some("Mnemonic checksum invalid"),
        body_lines: &body,
        rows: &rows,
        buttons: ButtonBar::new().back("CANCEL").confirm("RETRY"),
    }
    .draw(display, &theme)
}

/// Passphrase / message character grid. First input-register screen.
/// Layout: header + dot/count preview + 5-row char grid + action row + button bar.
/// The selected cell renders full-bleed cyan with the char in bg color (inverted).
fn draw_passphrase_grid<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    grid: &crate::gui::app::CharGrid,
    title: &str,
) -> Result<(), D::Error> {
    use crate::gui::app::{GRID_COLS, GridAction};
    use crate::ui::layout::{split_bottom, split_top};
    use crate::ui::widgets::{ButtonBar, Header, HeaderKind};
    use crate::ui::Theme;
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(
        Point::zero(),
        Size::new(theme.width, theme.height),
    );
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    let (body_rect, footer_rect) = split_bottom(rest, theme.footer_h as i32);

    Header {
        kind: HeaderKind::Title(title),
        counter: None,
    }
    .draw(display, &theme, header_rect)?;

    // Preview band.
    let preview_h = 28i32;
    let (preview_rect, grid_rect) = split_top(body_rect, preview_h);
    draw_preview(display, &theme, preview_rect, &grid.text)?;

    // Grid: 6 rows (5 char rows + 1 action row), 10 cols each, edge-to-edge.
    let cell_w = (theme.width / GRID_COLS as u32) as i32;
    let row_h = grid_rect.size.height as i32 / 6;

    // Character cells (rows 0-4).
    for row in 0..5usize {
        for col in 0..GRID_COLS {
            let x = grid_rect.top_left.x + col as i32 * cell_w;
            let y = grid_rect.top_left.y + row as i32 * row_h;
            let cell = Rectangle::new(
                Point::new(x, y),
                Size::new(cell_w as u32, row_h as u32),
            );
            let is_selected = grid.row == row && grid.col == col;

            if is_selected {
                cell.into_styled(PrimitiveStyle::with_fill(theme.accent))
                    .draw(display)?;
            }

            let mut ch = crate::gui::app::GRID_CHARS[row][col];
            if grid.caps && ch.is_ascii_lowercase() {
                ch = ch.to_ascii_uppercase();
            }
            let color = if is_selected { theme.bg } else { theme.text };
            let mut buf = [0u8; 4];
            let s = ch.encode_utf8(&mut buf);
            Text::with_alignment(
                s,
                Point::new(x + cell_w / 2, y + row_h / 2 + 5),
                theme.style_sm(color),
                Alignment::Center,
            )
            .draw(display)?;
        }
    }

    // Action row (row 5). Four buttons of widths [2, 2, 3, 3] cells.
    let action_y = grid_rect.top_left.y + 5 * row_h;
    let actions: [(usize, usize, GridAction, &str); 4] = [
        (0, 2, GridAction::Space, "SPC"),
        (2, 2, GridAction::Caps, "CAPS"),
        (4, 3, GridAction::Delete, "DEL"),
        (7, 3, GridAction::Done, "DONE"),
    ];
    let current_action = grid.action_region();
    for (start_col, span, action, label) in actions {
        let x = grid_rect.top_left.x + start_col as i32 * cell_w;
        let w = span as i32 * cell_w;
        let rect = Rectangle::new(
            Point::new(x, action_y),
            Size::new(w as u32, row_h as u32),
        );
        let is_selected = current_action == Some(action);
        if is_selected {
            rect.into_styled(PrimitiveStyle::with_fill(theme.accent))
                .draw(display)?;
        }
        let color = if is_selected { theme.bg } else { theme.muted };
        Text::with_alignment(
            label,
            Point::new(x + w / 2, action_y + row_h / 2 + 5),
            theme.style_sm(color),
            Alignment::Center,
        )
        .draw(display)?;
    }

    ButtonBar::new()
        .back("BACK")
        .confirm("SELECT")
        .draw(display, &theme, footer_rect)?;

    Ok(())
}

/// Preview band for the passphrase grid: `••••• 5 CHARS` style.
fn draw_preview<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &crate::ui::Theme,
    rect: Rectangle,
    text: &str,
) -> Result<(), D::Error> {
    let n = text.chars().count();
    let cx = rect.top_left.x + rect.size.width as i32 / 2;
    let cy = rect.top_left.y + rect.size.height as i32 / 2;

    if n == 0 {
        Text::with_alignment(
            "—",
            Point::new(cx, cy + 6),
            theme.style_sm(theme.dim),
            Alignment::Center,
        )
        .draw(display)?;
        return Ok(());
    }

    // Draw up to 12 square dots + a count label.
    let show = n.min(12);
    let dot = 5i32;
    let gap = 3i32;
    let dots_w = show as i32 * (dot + gap) - gap;
    let count_str = alloc::format!("{}", n);
    // Approximate label width (profont17 glyph ~10px) for centering.
    let label_w = count_str.len() as i32 * 11 + 40; // "N CHARS"
    let total_w = dots_w + 12 + label_w;
    let start_x = cx - total_w / 2;

    let dot_y = cy - dot / 2;
    for i in 0..show {
        let x = start_x + i as i32 * (dot + gap);
        Rectangle::new(Point::new(x, dot_y), Size::new(dot as u32, dot as u32))
            .into_styled(PrimitiveStyle::with_fill(theme.accent))
            .draw(display)?;
    }

    let label = alloc::format!("{} CHARS", n);
    Text::with_alignment(
        &label,
        Point::new(start_x + dots_w + 12, cy + 6),
        theme.style_sm(theme.muted),
        Alignment::Left,
    )
    .draw(display)?;

    Ok(())
}

/// Shared passphrase prompt for the Create and Load flows.
fn draw_passphrase_prompt<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("SKIP", "No passphrase"),
        ListRow::with_subtitle("ADD", "Extra security layer"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("PASSPHRASE"),
        counter: Some((sel + 1, 2)),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 2,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("SELECT"),
    }
    .draw(display, &theme)
}

/// Wallet confirmation. Card register — shows derived address so the user
/// can verify before the wallet is committed. Decision is driven by the
/// button bar (BACK = cancel, CONFIRM = accept) — no row selection needed.
fn draw_wallet_confirm<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    address: &str,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, CardRow, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();

    // Split the 32–44 char address into two halves so it wraps inside the
    // card body. Rendered via `body_lines` (not rows) because 22+ base58
    // chars don't fit in the right-aligned value slot.
    let mid = address.len() / 2;
    let first = &address[..mid];
    let second = &address[mid..];
    let body = [first, second];

    let rows: [CardRow; 1] = [CardRow::new("PATH", "m/44'/501'/0'/0'")];

    CardScreen {
        header: HeaderKind::Title(title),
        counter: None,
        title: Some("CONFIRM"),
        subtitle: Some("Verify the address"),
        body_lines: &body,
        rows: &rows,
        buttons: ButtonBar::new().back("CANCEL").confirm("CONFIRM"),
    }
    .draw(display, &theme)
}

/// Load-method picker. Scan an existing SeedQR or type the words in manually.
fn draw_load_method<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("SCAN QR", "From SeedQR backup"),
        ListRow::with_subtitle("TYPE", "Enter BIP39 words"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("LOAD WALLET"),
        counter: Some((sel + 1, 2)),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 2,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("SELECT"),
    }
    .draw(display, &theme)
}

/// Word-count picker (step 1 of create). 12 or 24 BIP39 words.
fn draw_create_word_count<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("12 WORDS", "128-bit entropy"),
        ListRow::with_subtitle("24 WORDS", "256-bit entropy"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("WORD COUNT"),
        counter: Some((sel + 1, 2)),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 2,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("SELECT"),
    }
    .draw(display, &theme)
}

/// Entropy-method picker (step 2 of create).
fn draw_create_method<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 4] = [
        ListRow::with_subtitle("RANDOM", "Device entropy"),
        ListRow::with_subtitle("CAMERA", "Sensor entropy"),
        ListRow::with_subtitle("COINS", "Flip your own"),
        ListRow::with_subtitle("DICE", "Roll your own"),
    ];
    let sel = selected.min(3);

    ListScreen {
        header: HeaderKind::Title("METHOD"),
        counter: Some((sel + 1, 4)),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("SELECT"),
    }
    .draw(display, &theme)
}

/// Settings menu: list register with Title header. Items depend on whether
/// a wallet is loaded.
fn draw_settings_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();

    let loaded: [ListRow; 5] = [
        ListRow::new("ADDRESS"),
        ListRow::new("EXPORT QR"),
        ListRow::new("ACCOUNTS"),
        ListRow::new("ABOUT"),
        ListRow::new("POWER OFF"),
    ];
    let empty: [ListRow; 2] = [
        ListRow::new("ABOUT"),
        ListRow::new("POWER OFF"),
    ];
    let items: &[ListRow] = if seed_loaded { &loaded } else { &empty };
    let total = items.len();
    let sel = selected.min(total - 1);

    ListScreen {
        header: HeaderKind::Title("SETTINGS"),
        counter: Some((sel + 1, total)),
        description: None,
        items,
        selected: sel,
        max_visible: 3,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("SELECT"),
    }
    .draw(display, &theme)
}

/// Show mnemonic words, 6 per page in a 2x3 card grid.
fn draw_show_words<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    mnemonic: &str,
    page: usize,
    word_count: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let words_per_page = 4usize;
    let total_pages = (word_count + words_per_page - 1) / words_per_page;
    let page = page.min(total_pages - 1);
    let start = page * words_per_page;
    let end = (start + words_per_page).min(word_count);
    let is_last = page + 1 == total_pages;

    let words: Vec<&str> = mnemonic.split_whitespace().collect();

    // Own the number strings so the `ListRow` borrows have a stable lifetime.
    let nums: Vec<String> = (start..end)
        .map(|i| alloc::format!("{:02}", i + 1))
        .collect();
    let rows: Vec<ListRow> = (0..nums.len())
        .map(|i| ListRow::with_prefix(&nums[i], words[start + i]))
        .collect();

    ListScreen {
        header: HeaderKind::Title("SEED"),
        counter: Some((page + 1, total_pages)),
        description: None,
        items: &rows,
        selected: 0,
        max_visible: words_per_page,
        selectable: false,
        buttons: ButtonBar::new()
            .back("BACK")
            .confirm(if is_last { "VERIFY" } else { "NEXT" }),
    }
    .draw(display, &theme)
}

/// Format `n` as a zero-padded 2-digit string in a stack buffer.
fn fmt_num(buf: &mut [u8; 4], n: usize) -> &str {
    use core::fmt::Write;
    struct W<'a> {
        buf: &'a mut [u8; 4],
        pos: usize,
    }
    impl core::fmt::Write for W<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let b = s.as_bytes();
            let n = b.len().min(self.buf.len() - self.pos);
            self.buf[self.pos..self.pos + n].copy_from_slice(&b[..n]);
            self.pos += n;
            Ok(())
        }
    }
    let mut w = W { buf, pos: 0 };
    let _ = write!(&mut w, "{:02}", n);
    core::str::from_utf8(&w.buf[..w.pos]).unwrap_or("")
}

/// Word verification quiz. List register — the question is the header title
/// (e.g. "WORD 04?"), the check counter sits in the header's counter slot,
/// and the 4 options are list rows. Selected row gets the cyan highlight as
/// usual.
fn draw_verify_word<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_num: usize,
    options: &[String],
    selected: usize,
    check_num: usize,
    total_checks: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let title = alloc::format!("WORD {:02}?", word_num);
    let rows: Vec<ListRow> = options.iter().map(|s| ListRow::new(s)).collect();
    let sel = selected.min(options.len().saturating_sub(1));

    ListScreen {
        header: HeaderKind::Title(&title),
        counter: Some((check_num, total_checks)),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 4,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("SELECT"),
    }
    .draw(display, &theme)
}

/// Address confirmation screen.
fn draw_confirm_address<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    address: &str,
    derivation_path: &str,
    selected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, title, seed_loaded)?;

    // "Your address:" label
    let label_style = MonoTextStyle::new(&FONT_9X15, colors::TEXT_MUTED);
    Text::with_alignment("Your address:", Point::new(120, 55), label_style, Alignment::Center)
        .draw(display)?;

    // Address (split across 2 lines for readability)
    let addr_style = MonoTextStyle::new(&FONT_6X10, colors::SOLANA_GREEN);
    if address.len() > 22 {
        let mid = address.len() / 2;
        Text::with_alignment(&address[..mid], Point::new(120, 85), addr_style, Alignment::Center)
            .draw(display)?;
        Text::with_alignment(&address[mid..], Point::new(120, 100), addr_style, Alignment::Center)
            .draw(display)?;
    } else {
        Text::with_alignment(address, Point::new(120, 90), addr_style, Alignment::Center)
            .draw(display)?;
    }

    // Derivation path
    let path_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(derivation_path, Point::new(120, 125), path_style, Alignment::Center)
        .draw(display)?;

    // Confirm / Cancel buttons
    draw_button_bar(display, "Confirm", "Cancel", selected)?;

    Ok(())
}

/// Generic message screen.
fn draw_message<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    message: &str,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, title, seed_loaded)?;

    let style = MonoTextStyle::new(&FONT_9X15, colors::TEXT_SECONDARY);
    let mut y = 100i32;
    for line in message.split('\n') {
        Text::with_alignment(line, Point::new(120, y), style, Alignment::Center)
            .draw(display)?;
        y += 20;
    }

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Press any key", Point::new(120, 230), hint, Alignment::Center)
        .draw(display)?;

    Ok(())
}

/// TX review screen.
fn draw_tx_review<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    info_lines: &[String],
    scroll: usize,
    selected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "Review TX", seed_loaded)?;

    let line_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_SECONDARY);
    let max_lines = 15usize;

    for (vi, i) in (scroll..info_lines.len().min(scroll + max_lines)).enumerate() {
        let y = 35 + vi as i32 * 12;
        Text::new(&info_lines[i], Point::new(5, y), line_style)
            .draw(display)?;
    }

    draw_button_bar(display, "Sign", "Reject", selected)?;

    Ok(())
}

/// Show the wallet's public address as a QR. Users verify the QR in a hot
/// wallet; the truncated caption is for a quick visual double-check.
fn draw_show_address<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    address: Option<&str>,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, CardRow, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();

    match address {
        Some(addr) => {
            // Render a Solana URI envelope so third-party wallets can scan it.
            // Full-screen QR — any chrome shrinks the scan target; BACK button
            // returns to settings.
            use crate::ui::widgets::Qr;
            use embedded_graphics::{geometry::{Point, Size}, primitives::Rectangle};

            let uri = alloc::format!("solana:{}", addr);
            let screen = Rectangle::new(
                Point::zero(),
                Size::new(theme.width, theme.height),
            );
            display.fill_solid(&screen, theme.bg)?;
            Qr { data: &uri }.draw(display, &theme, screen)
        }
        None => {
            // No wallet loaded — card with a single info row.
            let rows: [CardRow; 1] = [CardRow::new("STATUS", "No wallet loaded")];
            CardScreen {
                header: HeaderKind::Title("ADDRESS"),
                counter: None,
                title: Some("NO WALLET"),
                subtitle: Some("Create or load one first"),
                body_lines: &[],
                rows: &rows,
                buttons: ButtonBar::new().back("BACK"),
            }
            .draw(display, &theme)
        }
    }
}

/// About screen. Card register — hero title + key/value reference rows.
fn draw_about<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, CardRow, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [CardRow; 4] = [
        CardRow::new("VERSION", "v0.1.0"),
        CardRow::new("NETWORK", "Solana"),
        CardRow::new("HARDWARE", "Pi Zero 1.3"),
        CardRow::new("KEYS", "RAM only"),
    ];

    CardScreen {
        header: HeaderKind::Title("ABOUT"),
        counter: None,
        title: Some("FARADAY"),
        subtitle: Some("Air-gapped Solana signer"),
        body_lines: &[],
        rows: &rows,
        buttons: ButtonBar::new().back("BACK"),
    }
    .draw(display, &theme)
}

/// Seed-export warning. Shown before the SeedQR to force the user to
/// acknowledge that the QR reveals the full recovery seed. Default
/// selection is CANCEL.
fn draw_export_seed_warning<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("CANCEL", "Keep the seed private"),
        ListRow::with_subtitle("SHOW", "I accept the risk"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("EXPORT SEED"),
        counter: None,
        description: Some("Reveals your seed"),
        items: &rows,
        selected: sel,
        max_visible: 2,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("CONFIRM"),
    }
    .draw(display, &theme)
}

/// Power-off confirmation. List register with the destructive consequence
/// exposed as the subtitle on the YES row. Default selection is NO.
fn draw_power_off<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("NO", "Back to settings"),
        ListRow::with_subtitle("YES", "Wipes wallet from RAM"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("POWER OFF"),
        counter: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 2,
        selectable: true,
        buttons: ButtonBar::new().back("BACK").confirm("CONFIRM"),
    }
    .draw(display, &theme)
}

/// Passphrase mismatch error screen.
/// Full-screen SeedQR export. Chromeless — the QR fills the display so a
/// phone camera has the largest possible scan target. CONFIRM / BACK exit,
/// SECONDARY (Key2) toggles standard vs compact mode (handled by the state
/// machine).
fn draw_seed_qr<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    data: &str,
) -> Result<(), D::Error> {
    use crate::ui::widgets::Qr;
    use crate::ui::Theme;
    use embedded_graphics::{geometry::{Point, Size}, primitives::Rectangle};

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(
        Point::zero(),
        Size::new(theme.width, theme.height),
    );
    display.fill_solid(&screen, theme.bg)?;
    Qr { data }.draw(display, &theme, screen)
}

/// Passphrase mismatch error card. Any key retries the input.
fn draw_passphrase_mismatch<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{ButtonBar, CardRow, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();
    let body = [
        "Your two entries did",
        "not match. Try again.",
    ];
    let rows: [CardRow; 0] = [];

    CardScreen {
        header: HeaderKind::Title("MISMATCH"),
        counter: None,
        title: Some("NO MATCH"),
        subtitle: Some("Passphrases don't match"),
        body_lines: &body,
        rows: &rows,
        buttons: ButtonBar::new().confirm("RETRY"),
    }
    .draw(display, &theme)
}

/// Camera entropy collection screen.
/// Camera entropy capture. No value picker — user just presses CAPTURE to
/// collect a frame of sensor noise.
fn draw_camera_entropy<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
    frames_collected: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::layout::{split_bottom, split_top};
    use crate::ui::widgets::{ButtonBar, Header, HeaderKind};
    use crate::ui::Theme;
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };

    let theme = Theme::faraday_240();
    let target = if word_count == 12 { 10 } else { 20 };
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    let (body_rect, footer_rect) = split_bottom(rest, theme.footer_h as i32);

    Header {
        kind: HeaderKind::Title("CAMERA"),
        counter: Some((frames_collected, target)),
    }
    .draw(display, &theme, header_rect)?;

    let (progress_rect, rest) = split_top(body_rect, 20);
    draw_progress_bar(display, &theme, progress_rect, frames_collected, target)?;

    // Big instructional text, centered in the remaining region.
    let cx = rest.top_left.x + rest.size.width as i32 / 2;
    let cy = rest.top_left.y + rest.size.height as i32 / 2;
    Text::with_alignment(
        "CAPTURE",
        Point::new(cx, cy - 10),
        theme.style_lg(theme.accent),
        Alignment::Center,
    )
    .draw(display)?;
    Text::with_alignment(
        "Move the device",
        Point::new(cx, cy + 14),
        theme.style_sm(theme.muted),
        Alignment::Center,
    )
    .draw(display)?;

    ButtonBar::new()
        .back("CANCEL")
        .confirm("CAPTURE")
        .draw(display, &theme, footer_rect)?;

    Ok(())
}

/// Filled cyan progress bar over a dim track. Uses the full width of `rect`,
/// fills a 6px strip with current/target ratio.
fn draw_progress_bar<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &crate::ui::Theme,
    rect: Rectangle,
    current: usize,
    total: usize,
) -> Result<(), D::Error> {
    let inset_x = theme.space_md;
    let track_w = rect.size.width as i32 - inset_x * 2;
    if track_w <= 0 {
        return Ok(());
    }
    let bar_h = 6u32;
    let y = rect.top_left.y + (rect.size.height as i32 - bar_h as i32) / 2;

    // Track.
    Rectangle::new(
        Point::new(rect.top_left.x + inset_x, y),
        Size::new(track_w as u32, bar_h),
    )
    .into_styled(PrimitiveStyle::with_fill(theme.border))
    .draw(display)?;

    // Fill.
    let filled = if total == 0 {
        0
    } else {
        (current.min(total) as i32 * track_w) / total as i32
    };
    if filled > 0 {
        Rectangle::new(
            Point::new(rect.top_left.x + inset_x, y),
            Size::new(filled as u32, bar_h),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.accent))
        .draw(display)?;
    }

    Ok(())
}

/// Coin flip entropy input screen.
/// Coin-flip entropy collector. H / T picker, selected gets the cyan highlight.
fn draw_coin_flips<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
    bits: &[bool],
    selected: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    let target = if word_count == 12 { 128 } else { 256 };
    let start = bits.len().saturating_sub(20);
    let recent: String = bits[start..]
        .iter()
        .map(|&b| if b { 'H' } else { 'T' })
        .collect();
    draw_entropy_picker(
        display,
        "COIN FLIPS",
        bits.len(),
        target,
        &recent,
        &["HEADS", "TAILS"],
        selected,
        PickerLayout::Row,
    )
}

/// Dice-roll entropy collector. 1–6 picker, selected gets the cyan highlight.
fn draw_dice_rolls<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
    rolls: &[u8],
    selected: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    let target = if word_count == 12 { 50 } else { 99 };
    let start = rolls.len().saturating_sub(20);
    let recent: String = rolls[start..]
        .iter()
        .map(|r| alloc::format!("{}", r + 1))
        .collect();
    draw_entropy_picker(
        display,
        "DICE ROLLS",
        rolls.len(),
        target,
        &recent,
        &["1", "2", "3", "4", "5", "6"],
        selected,
        PickerLayout::Grid { cols: 3, rows: 2 },
    )
}

/// How the picker cells are arranged inside the body.
#[derive(Clone, Copy)]
enum PickerLayout {
    /// Single row of `choices.len()` equal-width cells.
    Row,
    /// `cols × rows` grid. `cols * rows` must be >= choices.len().
    Grid { cols: usize, rows: usize },
}

/// Shared entropy-collection layout: header + progress bar + recent-history
/// strip + N-way value picker + button bar. Drives both the coin-flip and
/// dice-roll screens.
fn draw_entropy_picker<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    progress: usize,
    target: usize,
    recent: &str,
    choices: &[&str],
    selected: usize,
    layout: PickerLayout,
) -> Result<(), D::Error> {
    use crate::ui::layout::{split_bottom, split_top};
    use crate::ui::widgets::{ButtonBar, Header, HeaderKind};
    use crate::ui::Theme;
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    let (body_rect, footer_rect) = split_bottom(rest, theme.footer_h as i32);

    Header {
        kind: HeaderKind::Title(title),
        counter: Some((progress, target)),
    }
    .draw(display, &theme, header_rect)?;

    // Progress bar band, then recent-history strip, then picker fills the rest.
    let (progress_rect, rest) = split_top(body_rect, 16);
    draw_progress_bar(display, &theme, progress_rect, progress, target)?;

    let (recent_rect, picker_rect) = split_top(rest, 22);
    let cx = recent_rect.top_left.x + recent_rect.size.width as i32 / 2;
    let cy = recent_rect.top_left.y + recent_rect.size.height as i32 - 6;
    Text::with_alignment(
        recent,
        Point::new(cx, cy),
        theme.style_sm(theme.dim),
        Alignment::Center,
    )
    .draw(display)?;

    // Picker cells: full-bleed cyan for selected, with inverted text.
    // Layout chooses whether to line them up or grid them.
    let (cols, rows) = match layout {
        PickerLayout::Row => (choices.len(), 1),
        PickerLayout::Grid { cols, rows } => (cols, rows),
    };
    if cols > 0 && rows > 0 {
        let cell_w = picker_rect.size.width as i32 / cols as i32;
        let cell_h = picker_rect.size.height as i32 / rows as i32;
        for (i, label) in choices.iter().enumerate() {
            let col = (i % cols) as i32;
            let row = (i / cols) as i32;
            let x = picker_rect.top_left.x + col * cell_w;
            let y = picker_rect.top_left.y + row * cell_h;
            let cell = Rectangle::new(
                Point::new(x, y),
                Size::new(cell_w as u32, cell_h as u32),
            );
            let is_selected = i == selected;
            if is_selected {
                cell.into_styled(PrimitiveStyle::with_fill(theme.accent))
                    .draw(display)?;
            }
            let color = if is_selected { theme.bg } else { theme.text };
            Text::with_alignment(
                label,
                Point::new(x + cell_w / 2, y + cell_h / 2 + 10),
                theme.style_lg(color),
                Alignment::Center,
            )
            .draw(display)?;
        }
    }

    ButtonBar::new()
        .back("UNDO")
        .confirm("SELECT")
        .draw(display, &theme, footer_rect)?;

    Ok(())
}

/// Accounts / derivation paths screen.
fn draw_accounts<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    accounts: &[(String, String)],
    selected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "Accounts", seed_loaded)?;

    let path_style = MonoTextStyle::new(&FONT_6X10, colors::SOLANA_TEAL);
    let addr_style_normal = MonoTextStyle::new(&FONT_6X10, colors::TEXT_SECONDARY);
    let addr_style_selected = MonoTextStyle::new(&FONT_6X10, colors::SOLANA_GREEN);

    for (i, (path, addr)) in accounts.iter().enumerate() {
        let y = 35 + i as i32 * 42;
        let is_selected = i == selected;

        let (bg, border) = if is_selected {
            (colors::BG_CARD_SELECTED, colors::BORDER_SELECTED)
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT)
        };

        let style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(1)
            .build();

        embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(8, y), Size::new(224, 36)),
            Size::new(4, 4),
        )
        .into_styled(style)
        .draw(display)?;

        // Derivation path
        Text::new(path, Point::new(14, y + 13), path_style)
            .draw(display)?;

        // Truncated address
        let truncated = if addr.len() > 30 {
            alloc::format!("{}...{}", &addr[..12], &addr[addr.len()-8..])
        } else {
            addr.clone()
        };
        let a_style = if is_selected { addr_style_selected } else { addr_style_normal };
        Text::new(&truncated, Point::new(14, y + 28), a_style)
            .draw(display)?;
    }

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Press any key to return", Point::new(120, 232), hint, Alignment::Center)
        .draw(display)?;

    Ok(())
}

extern crate alloc;
