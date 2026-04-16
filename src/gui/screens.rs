//! Screen layouts — all UI pages.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_9X15, ascii::FONT_9X15_BOLD, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

use crate::gui::app::{App, Screen};
use crate::gui::colors;
use crate::gui::components::{
    Card, draw_button_bar, draw_char_grid, draw_option_list,
    draw_qr, draw_status_bar, draw_text_centered, draw_word_picker,
};
use crate::gui::icons;

/// Menu item for the 2x2 grid.
struct MenuItem {
    label: &'static str,
    icon_fn: fn() -> icons::Icon,
}

const MENU_ITEMS: [MenuItem; 4] = [
    MenuItem { label: "Create", icon_fn: icons::key },
    MenuItem { label: "Load", icon_fn: icons::camera },
    MenuItem { label: "Sign TX", icon_fn: icons::transaction },
    MenuItem { label: "Settings", icon_fn: icons::tools },
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
                draw_option_list(display, "Create Wallet", &["12 Words", "24 Words"], *selected, self.seed_loaded())
            }
            Screen::CreateMethod { word_count, selected } => {
                let title = alloc::format!("Create {} Words", word_count);
                draw_option_list(display, &title, &["Random", "Camera", "Coin Flips", "Dice Rolls"], *selected, self.seed_loaded())
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
                draw_verify_word(display, word_num, options, *selected, *current + 1, checks.len(), self.seed_loaded())
            }
            Screen::CreatePassphrasePrompt { selected, .. } => {
                draw_option_list(display, "Passphrase", &["Skip", "Enter Passphrase"], *selected, self.seed_loaded())
            }
            Screen::CreatePassphraseInput { grid, .. } => {
                draw_char_grid(display, grid, "Passphrase", self.seed_loaded())
            }
            Screen::CreatePassphraseConfirm { grid, .. } => {
                draw_char_grid(display, grid, "Confirm Passphrase", self.seed_loaded())
            }
            Screen::CreatePassphraseMismatch { .. } => {
                draw_passphrase_mismatch(display, self.seed_loaded())
            }
            Screen::CreateConfirm { address, selected, .. } => {
                let path = self.wallet.as_ref()
                    .map(|w| w.keypair.derivation_path.as_str())
                    .unwrap_or("m/44'/501'/0'/0'");
                draw_confirm_address(display, "New Wallet", address, path, *selected, self.seed_loaded())
            }
            Screen::ExportSeedQr { seed_qr_data, compact_data, compact_mode, .. } => {
                if *compact_mode {
                    // Compact SeedQR: binary data displayed as hex for QR
                    let hex_data = hex::encode(compact_data);
                    draw_qr(display, "Compact SeedQR", &hex_data, self.seed_loaded())
                } else {
                    draw_qr(display, "SeedQR Backup", seed_qr_data, self.seed_loaded())
                }
            }

            // Load flow
            Screen::LoadMethod { selected } => {
                draw_option_list(display, "Load Wallet", &["Scan SeedQR", "Enter Words"], *selected, self.seed_loaded())
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
                draw_option_list(display, "Word Count", &["12 Words", "24 Words"], *selected, self.seed_loaded())
            }
            Screen::LoadEnterWords { picker, .. } => {
                draw_word_picker(display, picker, self.seed_loaded())
            }
            Screen::LoadPassphrasePrompt { selected, .. } => {
                draw_option_list(display, "Passphrase", &["Skip", "Enter Passphrase"], *selected, self.seed_loaded())
            }
            Screen::LoadPassphraseInput { grid, .. } => {
                draw_char_grid(display, grid, "Passphrase", self.seed_loaded())
            }
            Screen::LoadPassphraseConfirm { grid, .. } => {
                draw_char_grid(display, grid, "Confirm Passphrase", self.seed_loaded())
            }
            Screen::LoadPassphraseMismatch { .. } => {
                draw_passphrase_mismatch(display, self.seed_loaded())
            }
            Screen::LoadConfirm { address, selected, .. } => {
                let path = self.wallet.as_ref()
                    .map(|w| w.keypair.derivation_path.as_str())
                    .unwrap_or("m/44'/501'/0'/0'");
                draw_confirm_address(display, "Load Wallet", address, path, *selected, self.seed_loaded())
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
                draw_char_grid(display, grid, "Sign Message", self.seed_loaded())
            }
            Screen::SignMessageResult { signature_hex } => {
                draw_qr(display, "Signature", signature_hex, self.seed_loaded())
            }

            // Settings
            Screen::SettingsMenu { selected } => {
                let opts: Vec<&str> = if self.seed_loaded() {
                    vec!["Show Address", "Export SeedQR", "Accounts", "About", "Power Off"]
                } else {
                    vec!["About", "Power Off"]
                };
                draw_option_list(display, "Settings", &opts, *selected, self.seed_loaded())
            }
            Screen::SettingsAccounts { accounts, selected } => {
                draw_accounts(display, accounts, *selected, self.seed_loaded())
            }
            Screen::SettingsShowAddress => {
                if let Some(wallet) = &self.wallet {
                    draw_qr(display, "Address", &wallet.address, true)
                } else {
                    draw_message(display, "Address", "No wallet loaded", false)
                }
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

/// Splash screen shown at boot.
pub fn draw_splash<D: DrawTarget<Color = Rgb565>>(display: &mut D) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;

    // Gradient accent at top
    for x in 0..240i32 {
        let factor = ((x as f32 / 240.0) * 255.0) as u8;
        let color = colors::blend(colors::SOLANA_PURPLE, colors::SOLANA_GREEN, factor);
        Rectangle::new(Point::new(x, 0), Size::new(1, 3))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
    }

    draw_text_centered(display, "Faraday", 110, colors::TEXT_PRIMARY)?;
    draw_text_centered(display, "Air-gapped Signer", 135, colors::TEXT_SECONDARY)?;

    let style = MonoTextStyle::new(&FONT_9X15, colors::TEXT_MUTED);
    Text::with_alignment("v0.1.0", Point::new(120, 165), style, Alignment::Center)
        .draw(display)?;

    // Gradient accent at bottom
    for x in 0..240i32 {
        let factor = ((x as f32 / 240.0) * 255.0) as u8;
        let color = colors::blend(colors::SOLANA_GREEN, colors::SOLANA_PURPLE, factor);
        Rectangle::new(Point::new(x, 237), Size::new(1, 3))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
    }

    Ok(())
}

/// Main menu: 2x2 card grid.
fn draw_main_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
    seed_loaded: bool,
    address: Option<&str>,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "Faraday", seed_loaded)?;

    // Show truncated address below status bar when wallet is loaded
    if let Some(addr) = address {
        let truncated = if addr.len() > 12 {
            alloc::format!("{}...{}", &addr[..4], &addr[addr.len()-4..])
        } else {
            addr.to_string()
        };
        let addr_style = MonoTextStyle::new(&FONT_6X10, colors::SOLANA_GREEN);
        Text::with_alignment(&truncated, Point::new(120, 30), addr_style, Alignment::Center)
            .draw(display)?;
    }

    let margin = 10i32;
    let gap = 8i32;
    let card_w = ((240 - margin * 2 - gap) / 2) as u32; // ~106px
    let card_h = 96u32;
    let top_offset = if address.is_some() { 35i32 } else { 28i32 };

    for (i, item) in MENU_ITEMS.iter().enumerate() {
        let col = (i % 2) as i32;
        let row = (i / 2) as i32;
        let x = margin + col * (card_w as i32 + gap);
        let y = top_offset + row * (card_h as i32 + gap);

        let card = Card {
            label: item.label,
            icon: (item.icon_fn)(),
            selected: i == selected,
        };
        card.draw(display, x, y, card_w, card_h)?;
    }

    Ok(())
}

/// Show mnemonic words, 6 per page in a 2x3 card grid.
fn draw_show_words<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    mnemonic: &str,
    page: usize,
    word_count: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    let words_per_page = 6usize;
    display.clear(colors::BG_DARK)?;

    let total_pages = (word_count + words_per_page - 1) / words_per_page;
    let start = page * words_per_page;
    let end = (start + words_per_page).min(word_count);
    let title = alloc::format!("Words {}-{}", start + 1, end);
    draw_status_bar(display, &title, seed_loaded)?;

    let words: Vec<&str> = mnemonic.split_whitespace().collect();

    // 2x3 grid of word cards
    let margin = 8i32;
    let gap = 6i32;
    let card_w = ((240 - margin * 2 - gap) / 2) as u32; // ~109px
    let card_h = 56u32;
    let top_offset = 26i32;

    let num_style = MonoTextStyle::new(&FONT_6X10, colors::SOLANA_GREEN);
    let word_style = MonoTextStyle::new(&FONT_9X15_BOLD, colors::TEXT_PRIMARY);

    for i in 0..words_per_page {
        let word_idx = start + i;
        if word_idx >= words.len() { break; }

        let col = (i % 2) as i32;
        let row = (i / 2) as i32;
        let x = margin + col * (card_w as i32 + gap);
        let y = top_offset + row * (card_h as i32 + gap);

        // Card background
        let card_style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .fill_color(colors::BG_CARD)
            .stroke_color(colors::BORDER_DEFAULT)
            .stroke_width(1)
            .build();

        embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(x, y), Size::new(card_w, card_h)),
            Size::new(5, 5),
        )
        .into_styled(card_style)
        .draw(display)?;

        // Word number (top-left of card)
        let num_str = alloc::format!("#{}", word_idx + 1);
        Text::new(&num_str, Point::new(x + 6, y + 14), num_style)
            .draw(display)?;

        // Word (centered in card)
        Text::with_alignment(
            words[word_idx],
            Point::new(x + card_w as i32 / 2, y + 40),
            word_style,
            Alignment::Center,
        )
        .draw(display)?;
    }

    // Navigation hint
    let hint_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    let hint = if page + 1 == total_pages {
        "Enter: verify  Esc: back"
    } else {
        "</>: page  Enter: next"
    };
    Text::with_alignment(hint, Point::new(120, 232), hint_style, Alignment::Center)
        .draw(display)?;

    // Page indicator
    let page_str = alloc::format!("{}/{}", page + 1, total_pages);
    Text::with_alignment(&page_str, Point::new(230, 232), hint_style, Alignment::Right)
        .draw(display)?;

    Ok(())
}

/// Word verification quiz.
fn draw_verify_word<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_num: usize,
    options: &[String],
    selected: usize,
    check_num: usize,
    total_checks: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;

    let title = alloc::format!("Verify {}/{}", check_num, total_checks);
    draw_status_bar(display, &title, seed_loaded)?;

    // Question
    let question = alloc::format!("Word #{}?", word_num);
    let q_style = MonoTextStyle::new(&FONT_10X20, colors::TEXT_PRIMARY);
    Text::with_alignment(&question, Point::new(120, 50), q_style, Alignment::Center)
        .draw(display)?;

    // Options
    let opt_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
    let start_y = 70i32;

    for (i, opt) in opt_refs.iter().enumerate() {
        let y = start_y + i as i32 * 38;
        let is_selected = i == selected;

        let (bg, border, text_color) = if is_selected {
            (colors::BG_CARD_SELECTED, colors::BORDER_SELECTED, colors::TEXT_PRIMARY)
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_SECONDARY)
        };

        let style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(1)
            .build();

        embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(30, y), Size::new(180, 32)),
            Size::new(4, 4),
        )
        .into_styled(style)
        .draw(display)?;

        let text_style = MonoTextStyle::new(&FONT_9X15_BOLD, text_color);
        Text::with_alignment(opt, Point::new(120, y + 22), text_style, Alignment::Center)
            .draw(display)?;
    }

    Ok(())
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

/// About screen.
fn draw_about<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "About", seed_loaded)?;

    draw_text_centered(display, "Faraday", 70, colors::TEXT_PRIMARY)?;

    let style = MonoTextStyle::new(&FONT_9X15, colors::TEXT_SECONDARY);
    Text::with_alignment("v0.1.0", Point::new(120, 95), style, Alignment::Center)
        .draw(display)?;
    Text::with_alignment("Air-gapped Solana", Point::new(120, 120), style, Alignment::Center)
        .draw(display)?;
    Text::with_alignment("transaction signer", Point::new(120, 140), style, Alignment::Center)
        .draw(display)?;

    let muted = MonoTextStyle::new(&FONT_6X10, colors::SOLANA_TEAL);
    Text::with_alignment("Air-gapped Solana signer", Point::new(120, 175), muted, Alignment::Center)
        .draw(display)?;
    Text::with_alignment("Pure Rust on Pi Zero", Point::new(120, 190), muted, Alignment::Center)
        .draw(display)?;

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Press any key to return", Point::new(120, 230), hint, Alignment::Center)
        .draw(display)?;

    Ok(())
}

/// Power off confirmation.
fn draw_power_off<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "Power Off", false)?;

    // Power icon (scaled 3x)
    let icon = icons::power();
    let icon_x = 108i32;
    let icon_y = 50i32;
    let scale = 3i32;
    let data = icon.data;
    for row in 0..16i32 {
        let byte_hi = data[row as usize * 2];
        let byte_lo = data[row as usize * 2 + 1];
        let word = ((byte_hi as u16) << 8) | (byte_lo as u16);
        for col in 0..16i32 {
            if (word >> (15 - col)) & 1 == 1 {
                Rectangle::new(
                    Point::new(icon_x + col * scale, icon_y + row * scale),
                    Size::new(scale as u32, scale as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(colors::WARNING))
                .draw(display)?;
            }
        }
    }

    let style = MonoTextStyle::new(&FONT_10X20, colors::WARNING);
    Text::with_alignment("Power off?", Point::new(120, 115), style, Alignment::Center)
        .draw(display)?;

    let sub = MonoTextStyle::new(&FONT_9X15, colors::TEXT_MUTED);
    Text::with_alignment("Wallet will be cleared", Point::new(120, 130), sub, Alignment::Center)
        .draw(display)?;

    draw_button_bar(display, "Yes", "No", selected)?;

    Ok(())
}

/// Passphrase mismatch error screen.
fn draw_passphrase_mismatch<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "Error", seed_loaded)?;

    // Red X icon (drawn manually)
    let cx = 120i32;
    let cy = 80i32;
    let size = 20i32;
    for i in -2..=2i32 {
        for d in 0..size {
            // Draw two crossing lines
            Pixel(Point::new(cx - size + d + i, cy - size + d), colors::DANGER).draw(display)?;
            Pixel(Point::new(cx + size - d + i, cy - size + d), colors::DANGER).draw(display)?;
        }
    }

    let msg_style = MonoTextStyle::new(&FONT_10X20, colors::DANGER);
    Text::with_alignment("Passphrases", Point::new(120, 130), msg_style, Alignment::Center)
        .draw(display)?;
    Text::with_alignment("don't match!", Point::new(120, 155), msg_style, Alignment::Center)
        .draw(display)?;

    let sub = MonoTextStyle::new(&FONT_9X15, colors::TEXT_SECONDARY);
    Text::with_alignment("Try again", Point::new(120, 185), sub, Alignment::Center)
        .draw(display)?;

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Press any key", Point::new(120, 230), hint, Alignment::Center)
        .draw(display)?;

    Ok(())
}

/// Camera entropy collection screen.
fn draw_camera_entropy<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
    frames_collected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    let total = if word_count == 12 { 10 } else { 20 };
    display.clear(colors::BG_DARK)?;

    let title = alloc::format!("Capture {}/{}", frames_collected, total);
    draw_status_bar(display, &title, seed_loaded)?;

    // Progress bar
    let progress = (frames_collected as f32 / total as f32 * 200.0) as u32;
    Rectangle::new(Point::new(20, 30), Size::new(200, 6))
        .into_styled(PrimitiveStyle::with_fill(colors::BG_CARD))
        .draw(display)?;
    if progress > 0 {
        Rectangle::new(Point::new(20, 30), Size::new(progress, 6))
            .into_styled(PrimitiveStyle::with_fill(colors::SOLANA_GREEN))
            .draw(display)?;
    }

    // Camera icon (large)
    let icon = icons::camera();
    // Draw 3x scaled icon in center
    let icon_x = 88i32;
    let icon_y = 70i32;
    let scale = 4i32;
    let data = icon.data;
    for row in 0..16i32 {
        let byte_hi = data[row as usize * 2];
        let byte_lo = data[row as usize * 2 + 1];
        let word = ((byte_hi as u16) << 8) | (byte_lo as u16);
        for col in 0..16i32 {
            if (word >> (15 - col)) & 1 == 1 {
                Rectangle::new(
                    Point::new(icon_x + col * scale, icon_y + row * scale),
                    Size::new(scale as u32, scale as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(colors::SOLANA_GREEN))
                .draw(display)?;
            }
        }
    }

    // Instructions
    let style = MonoTextStyle::new(&FONT_9X15, colors::TEXT_SECONDARY);
    Text::with_alignment("Press Enter to", Point::new(120, 160), style, Alignment::Center)
        .draw(display)?;
    Text::with_alignment("capture frame", Point::new(120, 178), style, Alignment::Center)
        .draw(display)?;

    let sub = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Move camera around for randomness", Point::new(120, 200), sub, Alignment::Center)
        .draw(display)?;

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Esc: cancel", Point::new(120, 232), hint, Alignment::Center)
        .draw(display)?;

    Ok(())
}

/// Coin flip entropy input screen.
fn draw_coin_flips<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
    bits: &[bool],
    selected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    let total = if word_count == 12 { 128 } else { 256 };
    display.clear(colors::BG_DARK)?;

    let title = alloc::format!("Flip {} of {}", bits.len() + 1, total);
    draw_status_bar(display, &title, seed_loaded)?;

    // Progress bar
    let progress = (bits.len() as f32 / total as f32 * 200.0) as u32;
    Rectangle::new(Point::new(20, 30), Size::new(200, 6))
        .into_styled(PrimitiveStyle::with_fill(colors::BG_CARD))
        .draw(display)?;
    if progress > 0 {
        Rectangle::new(Point::new(20, 30), Size::new(progress, 6))
            .into_styled(PrimitiveStyle::with_fill(colors::SOLANA_GREEN))
            .draw(display)?;
    }

    // Recent flips display (last 16)
    let recent_start = if bits.len() > 16 { bits.len() - 16 } else { 0 };
    let recent: String = bits[recent_start..].iter().map(|&b| if b { 'H' } else { 'T' }).collect();
    let recent_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(&recent, Point::new(120, 50), recent_style, Alignment::Center)
        .draw(display)?;

    // Two big buttons: H and T
    let btn_w = 100u32;
    let btn_h = 100u32;
    let gap = 10i32;
    let total_w = btn_w as i32 * 2 + gap;
    let start_x = (240 - total_w) / 2;
    let y = 65i32;

    for (i, label) in ["H", "T"].iter().enumerate() {
        let x = start_x + i as i32 * (btn_w as i32 + gap);
        let is_selected = i == selected;

        let (bg, border, text_color) = if is_selected {
            (colors::BG_CARD_SELECTED, colors::BORDER_SELECTED, colors::TEXT_PRIMARY)
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_SECONDARY)
        };

        let style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(2)
            .build();

        embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(x, y), Size::new(btn_w, btn_h)),
            Size::new(8, 8),
        )
        .into_styled(style)
        .draw(display)?;

        if is_selected {
            let glow = colors::blend(colors::BG_CARD_SELECTED, colors::ACCENT, 80);
            Rectangle::new(Point::new(x + 3, y + 1), Size::new(btn_w - 6, 3))
                .into_styled(PrimitiveStyle::with_fill(glow))
                .draw(display)?;
        }

        let text_style = MonoTextStyle::new(&FONT_10X20, text_color);
        Text::with_alignment(label, Point::new(x + btn_w as i32 / 2, y + btn_h as i32 / 2 + 7), text_style, Alignment::Center)
            .draw(display)?;

        // Sub-label
        let sub = if i == 0 { "Heads" } else { "Tails" };
        let sub_style = MonoTextStyle::new(&FONT_6X10, if is_selected { colors::TEXT_SECONDARY } else { colors::TEXT_MUTED });
        Text::with_alignment(sub, Point::new(x + btn_w as i32 / 2, y + btn_h as i32 / 2 + 22), sub_style, Alignment::Center)
            .draw(display)?;
    }

    // Hints
    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Esc: undo last flip", Point::new(120, 232), hint, Alignment::Center)
        .draw(display)?;

    Ok(())
}

/// Dice roll entropy input screen.
fn draw_dice_rolls<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
    rolls: &[u8],
    selected: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    let total = if word_count == 12 { 50 } else { 99 };
    display.clear(colors::BG_DARK)?;

    let title = alloc::format!("Roll {} of {}", rolls.len() + 1, total);
    draw_status_bar(display, &title, seed_loaded)?;

    // Progress bar
    let progress = (rolls.len() as f32 / total as f32 * 200.0) as u32;
    Rectangle::new(Point::new(20, 30), Size::new(200, 6))
        .into_styled(PrimitiveStyle::with_fill(colors::BG_CARD))
        .draw(display)?;
    if progress > 0 {
        Rectangle::new(Point::new(20, 30), Size::new(progress, 6))
            .into_styled(PrimitiveStyle::with_fill(colors::SOLANA_GREEN))
            .draw(display)?;
    }

    // Recent rolls display (last 20)
    let recent_start = if rolls.len() > 20 { rolls.len() - 20 } else { 0 };
    let recent: String = rolls[recent_start..].iter().map(|r| r.to_string()).collect();
    let recent_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(&recent, Point::new(120, 50), recent_style, Alignment::Center)
        .draw(display)?;

    // 2x3 grid of dice faces
    let die_size = 60u32;
    let gap = 8i32;
    let grid_w = 3 * die_size as i32 + 2 * gap;
    let start_x = (240 - grid_w) / 2;
    let start_y = 58i32;

    for i in 0..6usize {
        let col = (i % 3) as i32;
        let row = (i / 3) as i32;
        let x = start_x + col * (die_size as i32 + gap);
        let y = start_y + row * (die_size as i32 + gap);
        let is_selected = i == selected;

        let (bg, border, text_color) = if is_selected {
            (colors::BG_CARD_SELECTED, colors::BORDER_SELECTED, colors::TEXT_PRIMARY)
        } else {
            (colors::BG_CARD, colors::BORDER_DEFAULT, colors::TEXT_SECONDARY)
        };

        let style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(border)
            .stroke_width(if is_selected { 2 } else { 1 })
            .build();

        embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(x, y), Size::new(die_size, die_size)),
            Size::new(6, 6),
        )
        .into_styled(style)
        .draw(display)?;

        // Die number
        let num = alloc::format!("{}", i + 1);
        let text_style = MonoTextStyle::new(&FONT_10X20, text_color);
        Text::with_alignment(&num, Point::new(x + die_size as i32 / 2, y + die_size as i32 / 2 + 7), text_style, Alignment::Center)
            .draw(display)?;
    }

    // Hints
    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment("Esc: undo last roll", Point::new(120, 232), hint, Alignment::Center)
        .draw(display)?;

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
