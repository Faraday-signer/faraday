//! Screen layouts — all UI pages.

use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, ascii::FONT_6X10, ascii::FONT_9X15, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};

use crate::gui::app::{App, Screen};
use crate::gui::colors;
use crate::gui::components::{
    draw_char_grid, draw_option_list, draw_qr, draw_status_bar,
};
use crate::gui::logo;

/// Menu item. Brutalist layout: one hero label + subtitle at a time.
struct MenuItem {
    label: &'static str,
    subtitle: &'static str,
}

const MENU_ITEMS: [MenuItem; 5] = [
    MenuItem {
        label: "CREATE",
        subtitle: "new wallet",
    },
    MenuItem {
        label: "LOAD",
        subtitle: "existing wallet",
    },
    MenuItem {
        label: "SIGN",
        subtitle: "transaction",
    },
    MenuItem {
        label: "WALLET DATA",
        subtitle: "addresses & backup",
    },
    MenuItem {
        label: "ABOUT",
        subtitle: "about faraday",
    },
];

impl App {
    /// Draw the current screen.
    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        match &self.screen {
            Screen::Splash => draw_boot_splash(display),

            Screen::ModeSelect { selected, .. } => draw_mode_select(display, *selected),

            Screen::Help { topic } => draw_help(display, *topic),

            Screen::MainMenu { selected } => {
                let addr = self.wallet.as_ref().map(|w| w.address.as_str());
                draw_main_menu(display, *selected, self.seed_loaded(), addr)
            }

            // Create flow
            Screen::CreateWordCount { selected } => draw_create_word_count(display, *selected),
            Screen::CreateMethod { selected, .. } => draw_create_method(display, *selected),
            Screen::CreateCameraEntropy {
                word_count,
                frames_collected,
                ..
            } => draw_camera_entropy(
                display,
                *word_count,
                *frames_collected,
                self.seed_loaded(),
                self.has_camera_frame(),
            ),
            Screen::CreateCoinFlips {
                word_count,
                bits,
                selected,
            } => draw_coin_flips(display, *word_count, bits, *selected, self.seed_loaded()),
            Screen::CreateDiceRolls {
                word_count,
                rolls,
                selected,
            } => draw_dice_rolls(display, *word_count, rolls, *selected, self.seed_loaded()),
            Screen::CreateBackupWarning { selected, .. } => {
                draw_create_backup_warning(display, *selected)
            }
            Screen::CreateShowWords {
                mnemonic,
                page,
                word_count,
            } => draw_show_words(display, mnemonic, *page, *word_count, self.seed_loaded()),
            Screen::CreateVerify {
                checks,
                current,
                options,
                correct_idx: _,
                selected,
                mnemonic: _,
            } => {
                let word_num = checks[*current] + 1;
                draw_verify_word(
                    display,
                    word_num,
                    options,
                    *selected,
                    *current + 1,
                    checks.len(),
                )
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
            Screen::CreateConfirm {
                address,
                passphrase,
                mnemonic,
                ..
            } => {
                let wc = mnemonic.split_whitespace().count();
                draw_wallet_confirm(display, "NEW WALLET CONFIRMATION", address, !passphrase.is_empty(), wc)
            }
            Screen::ExportSeedWarning { selected, .. } => {
                draw_export_seed_warning(display, *selected)
            }
            Screen::ShowWordsWarning { selected, .. } => {
                // Same red-banner gate, specifically before plaintext words.
                draw_export_seed_warning(display, *selected)
            }
            Screen::ExportSeedQrMenu { selected, .. } => {
                draw_export_seed_qr_menu(display, self.seedqr_title(), *selected)
            }
            Screen::ExportShowWords {
                mnemonic,
                page,
                word_count,
                ..
            } => draw_show_words(display, mnemonic, *page, *word_count, self.seed_loaded()),
            Screen::ExportSeedQr { compact_data, .. } => {
                // CompactSeedQR: raw 16/32 entropy bytes at ECL L so the grid
                // stays as small as possible for hand-transcription (12w →
                // V1 21×21, 24w → V2 25×25).
                // Seed-QR compare screen. `quiet: 2` (instead of the
                // standard 4) makes the matrix ~10% larger so side-by-side
                // visual check vs. the hand-transcribed paper is easier.
                draw_fullscreen_qr(display, compact_data, crate::qr::encode_qr::QrEcLevel::L, 2)
            }
            Screen::ExportSeedQrBlock {
                compact_data,
                block_index,
                ..
            } => draw_qr_block(display, compact_data, *block_index, self.seed_loaded()),

            // Load flow
            Screen::LoadMethod { selected } => draw_load_method(display, *selected),
            Screen::LoadScanQr => {
                #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
                {
                    draw_scan_overlay(
                        display,
                        "Scan SeedQR",
                        "Point camera at SeedQR",
                        self.seed_loaded(),
                        self.has_camera_frame(),
                        self.camera_error_str(),
                        self.scan_diag,
                    )?;
                }
                #[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
                {
                    draw_message(
                        display,
                        "Scan SeedQR",
                        "Point camera at\nSeedQR code",
                        self.seed_loaded(),
                    )?;
                }
                Ok(())
            }
            Screen::LoadWordCount { selected } => {
                // Same visual as Create's word-count picker — the choice is
                // the same, only the state-machine edges differ.
                draw_create_word_count(display, *selected)
            }
            Screen::LoadEnterWords { picker, .. } => draw_word_picker_new(display, picker),
            Screen::LoadWordCommitted {
                just_committed,
                picker,
                word_count,
                ..
            } => draw_word_committed(display, just_committed, picker.words.len(), *word_count),
            Screen::LoadInvalidMnemonic { word_count } => {
                draw_invalid_mnemonic(display, *word_count)
            }
            Screen::LoadSeedLoaded { .. } => draw_load_seed_loaded(display),
            Screen::LoadFinalize { preview_address, selected, .. } => {
                draw_load_finalize(display, preview_address, *selected)
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
            Screen::LoadConfirm {
                address,
                passphrase,
                mnemonic,
                ..
            } => {
                let wc = mnemonic.split_whitespace().count();
                draw_wallet_confirm(display, "LOAD WALLET", address, !passphrase.is_empty(), wc)
            }

            Screen::DerivationError => draw_derivation_error(display),

            // Sign TX flow
            Screen::SignScanTx => {
                #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
                {
                    draw_scan_overlay(
                        display,
                        "Sign TX",
                        "Point camera at TX QR",
                        self.seed_loaded(),
                        self.has_camera_frame(),
                        self.camera_error_str(),
                        self.scan_diag,
                    )
                }
                #[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
                {
                    draw_message(
                        display,
                        "Sign TX",
                        "Scan unsigned TX QR\nX: Sign Message",
                        self.seed_loaded(),
                    )
                }
            }
            Screen::SignReview {
                info_lines,
                scroll,
                selected,
                can_sign,
                ..
            } => draw_tx_review(
                display,
                info_lines,
                *scroll,
                *selected,
                *can_sign,
                self.seed_loaded(),
            ),
            Screen::SignShowQr { data } => draw_fullscreen_qr(
                display,
                data.as_bytes(),
                crate::qr::encode_qr::QrEcLevel::M,
                4,
            ),
            Screen::SignMessageReview {
                message_bytes,
                scroll,
                ..
            } => draw_message_review(display, message_bytes, *scroll, self.seed_loaded()),
            Screen::SignMessageInput { grid } => draw_passphrase_grid(display, grid, "SIGN MSG"),
            Screen::SignMessageResult { signature_hex } => draw_fullscreen_qr(
                display,
                signature_hex.as_bytes(),
                crate::qr::encode_qr::QrEcLevel::M,
                4,
            ),

            // Settings
            Screen::SettingsMenu { selected } => {
                draw_settings_menu(display, *selected)
            }
            Screen::SettingsAccounts { accounts, selected } => {
                draw_accounts_list(display, accounts, *selected)
            }
            Screen::SettingsShowAddress => {
                draw_show_address(display, self.wallet.as_ref().map(|w| w.address.as_str()))
            }
            Screen::SettingsVerifyAddressScan => {
                #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
                {
                    draw_scan_overlay(
                        display,
                        "Verify Address",
                        "Point camera at address QR",
                        self.seed_loaded(),
                        self.has_camera_frame(),
                        self.camera_error_str(),
                        self.scan_diag,
                    )
                }
                #[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
                {
                    draw_message(
                        display,
                        "Verify Address",
                        "Scan address QR\nto verify it's yours",
                        self.seed_loaded(),
                    )
                }
            }
            Screen::SettingsVerifyAddressResult { address, result } => {
                draw_verify_address_result_card(display, address, result)
            }
            Screen::SettingsAbout => draw_about(display, self.seed_loaded()),
            Screen::SettingsPowerOff { selected } => draw_reset_wallet(display, *selected),

            // Verify backup flow
            Screen::VerifyBackupScan => {
                #[cfg(any(feature = "simulator", target_os = "linux"))]
                {
                    draw_scan_overlay(
                        display,
                        "Verify Backup",
                        "Scan your paper SeedQR",
                        self.seed_loaded(),
                        self.has_camera_frame(),
                        self.camera_error_str(),
                        self.scan_diag,
                    )
                }
                #[cfg(not(any(feature = "simulator", target_os = "linux")))]
                {
                    draw_message(
                        display,
                        "Verify Backup",
                        "Scan your paper\nSeedQR",
                        self.seed_loaded(),
                    )
                }
            }
            Screen::VerifyBackupSeedMismatch => draw_verify_backup_seed_mismatch(display),
            Screen::VerifyBackupPassphrase { grid } => {
                draw_passphrase_grid(display, grid, "PASSPHRASE")
            }
            Screen::VerifyBackupPassphraseMismatch => {
                draw_verify_backup_passphrase_mismatch(display)
            }
            Screen::VerifyBackupSuccess => {
                let has_passphrase = self
                    .wallet
                    .as_ref()
                    .map_or(false, |w| !w.passphrase.is_empty());
                draw_verify_backup_success(display, has_passphrase)
            }
        }
    }
}

/// Boot splash: logo centered on the screen, no animation. Shown for the
/// ~2 s power-on beat before the menu comes up.
pub fn draw_boot_splash<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
) -> Result<(), D::Error> {
    display.clear(colors::FD_BG)?;
    const SCREEN: i32 = 240;
    const SCALE: u32 = 3;
    let logo_w = (logo::LOGO_WIDTH * SCALE) as i32;
    let logo_h = (logo::LOGO_HEIGHT * SCALE) as i32;
    let x = (SCREEN - logo_w) / 2;
    let y = (SCREEN - logo_h) / 2;
    logo::draw_logo(display, x, y, SCALE, colors::FD_ACCENT)?;
    Ok(())
}

/// Sleep / reposo screen. Same approach as before (small bitmap rendered
/// at integer scale so every "pixel" is a chunky block) — only the art has
/// changed. Adds a DVD-screensaver bounce: each axis moves independently,
/// with a hard inset from the edges so the logo never touches the corners.
pub fn draw_splash<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    elapsed_ms: u64,
) -> Result<(), D::Error> {
    display.clear(colors::FD_BG)?;

    const SCREEN: i32 = 240;
    const INSET: i32 = 16; // hard margin from any edge — corners never touch
    const SCALE: u32 = 3;
    let logo_w = (logo::LOGO_WIDTH * SCALE) as i32;
    let logo_h = (logo::LOGO_HEIGHT * SCALE) as i32;

    let x_min = INSET;
    let x_max = SCREEN - INSET - logo_w;
    let y_min = INSET;
    let y_max = SCREEN - INSET - logo_h;

    // Slow overall, and y clearly faster than x relative to its range so
    // the logo climbs / falls across several rows between side bounces.
    let (x, y) = bounce(elapsed_ms, x_min, x_max, y_min, y_max, 6, 16);

    logo::draw_logo(display, x, y, SCALE, colors::FD_ACCENT)?;

    Ok(())
}

/// Triangle-wave position: given `t` ms since start and a range `[min, max]`,
/// returns the current coordinate bouncing at `speed_pps` pixels/sec. The
/// wave is symmetric (out-and-back takes `2 * range / speed` seconds).
fn bounce(
    t_ms: u64,
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
    x_speed_pps: u64,
    y_speed_pps: u64,
) -> (i32, i32) {
    (
        triangle(t_ms, x_min, x_max, x_speed_pps),
        triangle(t_ms, y_min, y_max, y_speed_pps),
    )
}

fn triangle(t_ms: u64, min: i32, max: i32, speed_pps: u64) -> i32 {
    let range = (max - min).max(1) as u64;
    // Position traversed at `speed_pps * t_ms / 1000`, wrapped into a
    // triangle of period 2*range.
    let travel = (speed_pps.saturating_mul(t_ms)) / 1000;
    let phase = travel % (2 * range);
    let offset = if phase <= range { phase } else { 2 * range - phase };
    min + offset as i32
}

fn draw_mode_select<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("EXPERT", "I know Faraday"),
        ListRow::with_subtitle("GUIDED", "Show me how"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Brand,
        counter: None,
        right_label: Some("5s"),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 2,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check),
    }
    .draw(display, &theme)
}

fn draw_help<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    topic: crate::gui::app::HelpTopic,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();
    let raw_body = topic.body();
    let lines: Vec<&str> = raw_body.lines().collect();

    CardScreen {
        header: HeaderKind::Title(topic.title()),
        counter: None,
        right_label: None,
        title: None,
        subtitle: None,
        body_lines: &lines,
        rows: &[],
        title_danger: false,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Main menu: list register (Header + List + right-edge hints) via `src/ui/`.
fn draw_main_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
    seed_loaded: bool,
    address: Option<&str>,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();

    let row = |i: usize| ListRow::with_subtitle(MENU_ITEMS[i].label, MENU_ITEMS[i].subtitle);

    let visible: &[usize] = if seed_loaded {
        &[2, 3, 4] // SIGN, WALLET DATA, ABOUT
    } else {
        &[0, 1, 4] // CREATE, LOAD, ABOUT
    };
    let rows: Vec<ListRow> = visible.iter().map(|&i| row(i)).collect();
    let sel = selected.min(rows.len() - 1);

    // `first4…last4` of the loaded wallet, shown as a chip in the header's
    // top-right slot so users can see which key is mounted at a glance.
    let short = address.map(shorten_address);

    ListScreen {
        header: HeaderKind::Brand,
        counter: None,
        right_label: short.as_deref(),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        // Root screen — K3 has no "back" target, so only K1 (select) shows.
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check),
    }
    .draw(display, &theme)
}

/// Format a Solana base58 address as `first4…last4`.
fn shorten_address(addr: &str) -> String {
    if addr.len() <= 10 {
        return addr.to_string();
    }
    // ASCII "..." instead of the unicode ellipsis — the bitmap font
    // used on the Pi panel doesn't carry `…`, so `GAth…MsfT` renders
    // as `GAthMsfT` with no visible separator. Three literal dots keep
    // the intent clear on any glyph set.
    alloc::format!("{}...{}", &addr[..4], &addr[addr.len() - 4..])
}

/// Invalid mnemonic card. Shown when the 12/24 entered words don't form a
/// valid BIP39 seed. CONFIRM retries from scratch, BACK bails out of Load.
fn draw_invalid_mnemonic<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word_count: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let _ = word_count;
    let theme = Theme::faraday_240();
    let rows: [CardRow; 1] = [
        CardRow::new("CHECK", "BIP39 failed"),
    ];
    let body = [
        "Those words don't",
        "form a valid seed.",
        "",
        "Try again.",
    ];

    CardScreen {
        header: HeaderKind::Title("INVALID SEED"),
        counter: None,
        right_label: None,
        title: Some("INVALID"),
        subtitle: Some("Checksum failed"),
        body_lines: &body,
        rows: &rows,
        title_danger: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::Cross),
    }
    .draw(display, &theme)
}

fn draw_derivation_error<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, CardRow, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [CardRow; 1] = [
        CardRow::new("CAUSE", "HMAC / crypto lib"),
    ];
    let body = [
        "Key derivation",
        "failed.",
        "",
        "Should not happen.",
        "Please report this.",
    ];

    CardScreen {
        header: HeaderKind::Title("ERROR"),
        counter: None,
        right_label: None,
        title: Some("DERIVATION"),
        subtitle: Some("Key derivation failed"),
        body_lines: &body,
        rows: &rows,
        title_danger: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check),
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
    use crate::gui::app::{GridAction, GRID_COLS};
    use crate::ui::layout::{split_bottom, split_top};
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    // Body shrinks by the gutter width — keyboard cells don't overlap the
    // K1/K2/K3 icon column on the right.
    let body_rect = Rectangle::new(
        rest.top_left,
        Size::new(rest.size.width - GUTTER_W, rest.size.height),
    );

    Header {
        kind: HeaderKind::Title(title),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Preview band.
    let preview_h = 28i32;
    let (preview_rect, grid_rect) = split_top(body_rect, preview_h);
    draw_preview(display, &theme, preview_rect, &grid.text)?;

    // Grid: 6 rows (5 char rows + 1 action row), 10 cols each. Cell width
    // derives from the narrowed body width so cells stay inside the gutter.
    let cell_w = (body_rect.size.width / GRID_COLS as u32) as i32;
    let row_h = grid_rect.size.height as i32 / 6;

    // Character cells (rows 0-4).
    for row in 0..5usize {
        for col in 0..GRID_COLS {
            let x = grid_rect.top_left.x + col as i32 * cell_w;
            let y = grid_rect.top_left.y + row as i32 * row_h;
            let cell = Rectangle::new(Point::new(x, y), Size::new(cell_w as u32, row_h as u32));
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
        let rect = Rectangle::new(Point::new(x, action_y), Size::new(w as u32, row_h as u32));
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

    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k2(EdgeIcon::Delete)
        .k3(EdgeIcon::ArrowLeft)
        .draw(
            display,
            &theme,
            Rectangle::new(
                Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
                Size::new(GUTTER_W, theme.height - theme.header_h),
            ),
        )?;

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
/// Post-scan / post-word-entry confirmation. Shows the preview Solana address
/// (derived with no passphrase) so the user can see something was actually
/// read, then offers DONE (no passphrase) or ADD PASSPHRASE. Replaces the
/// previous SKIP/ADD prompt which read as a negative framing.
/// Post-create (and settings-entry) seed-backup landing page. Four options:
/// Show words, Paper backup, Verify backup, Back. Header title adapts via
/// `App::seedqr_title()` so it shows `SeedQR +P` when a passphrase is set.
fn draw_export_seed_qr_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 3] = [
        ListRow::with_subtitle("PAPER BACKUP", "Transcribe QR blocks"),
        ListRow::with_subtitle("SHOW WORDS", "to write them down"),
        ListRow::with_subtitle("BACK", "Return to menu"),
    ];
    let sel = selected.min(2);

    ListScreen {
        header: HeaderKind::Title(title),
        counter: None,
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Transient "seed loaded" splash. No chrome, no prompts — a big
/// brand-coloured "SEED LOADED ✓" centered on the screen. `tick()`
/// auto-advances to the passphrase decision after a short beat so
/// the user never has to press anything to dismiss it.
fn draw_load_seed_loaded<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
) -> Result<(), D::Error> {
    use crate::ui::Theme;
    use embedded_graphics::{
        primitives::{PrimitiveStyle, Line},
        text::{Alignment, Text},
    };

    let theme = Theme::faraday_240();

    let screen = Rectangle::new(
        Point::zero(),
        Size::new(theme.width, theme.height),
    );
    display.fill_solid(&screen, theme.bg)?;

    let cx = theme.width as i32 / 2;
    let cy = theme.height as i32 / 2;

    // Tick glyph, drawn as two lines so it scales cleanly on the 240 px
    // panel. The profont glyphs don't include a standalone check, so
    // hand-rolling keeps it big and unambiguous.
    let tick_color = theme.accent;
    let tick_stroke = PrimitiveStyle::with_stroke(tick_color, 4);
    let short_start = Point::new(cx - 34, cy - 30);
    let pivot = Point::new(cx - 10, cy - 6);
    let long_end = Point::new(cx + 36, cy - 56);
    Line::new(short_start, pivot).into_styled(tick_stroke).draw(display)?;
    Line::new(pivot, long_end).into_styled(tick_stroke).draw(display)?;

    Text::with_alignment(
        "SEED LOADED",
        Point::new(cx, cy + 40),
        theme.style_lg(theme.accent),
        Alignment::Center,
    )
    .draw(display)?;

    Ok(())
}

fn draw_load_finalize<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    preview_address: &str,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();

    // Address moves to the header chip (same slot as the main menu's
    // wallet short) so users keep continuity from the preceding
    // verification register. `first4…last4` is the house style.
    let addr_short = shorten_address(preview_address);

    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("DONE", "No passphrase"),
        ListRow::with_subtitle("ENCRYPT", "Add passphrase"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("PASSPHRASE"),
        counter: None,
        right_label: Some(&addr_short),
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

fn draw_passphrase_prompt<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("SKIP", "Words only"),
        ListRow::with_subtitle("ENCRYPT", "Extra security"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("PASSPHRASE"),
        counter: None,
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
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
    has_passphrase: bool,
    word_count: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();

    // Split the 32–44 char address into two halves so it wraps inside the
    // card body. Rendered via `body_lines` (not rows) because 22+ base58
    // chars don't fit in the right-aligned value slot.
    let mid = address.len() / 2;
    let first = &address[..mid];
    let second = &address[mid..];
    let body = [first, second];

    // Passphrase status goes in the subtitle — the place the user's eyes
    // already land, no extra chrome.
    let subtitle = if has_passphrase {
        "Passphrase-protected"
    } else {
        "Seed-only wallet"
    };
    // Seed length row: quick sanity check that the created/loaded wallet
    // matches the word count the user picked. Path/network are both fixed
    // and would just be noise.
    let length = if word_count == 24 {
        "24 WORDS"
    } else {
        "12 WORDS"
    };
    let rows: [CardRow; 1] = [CardRow::new("SEED:", length)];

    CardScreen {
        header: HeaderKind::Title(title),
        counter: None,
        right_label: None,
        title: Some("CONFIRM"),
        subtitle: Some(subtitle),
        body_lines: &body,
        rows: &rows,
        title_danger: false,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::Cross),
    }
    .draw(display, &theme)
}

/// Load-method picker. Scan an existing SeedQR or type the words in manually.
fn draw_load_method<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("SCAN QR", "From SeedQR backup"),
        ListRow::with_subtitle("TYPE", "Enter BIP39 words"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("LOAD WALLET"),
        counter: None,
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Word-count picker (step 1 of create). 12 or 24 BIP39 words.
fn draw_create_word_count<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("12 WORDS", "128-bit entropy"),
        ListRow::with_subtitle("24 WORDS", "256-bit entropy"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("WORD COUNT"),
        counter: None,
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Entropy-method picker (step 2 of create).
fn draw_create_method<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
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
        counter: None,
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Settings menu: list register with Title header. Items depend on whether
/// a wallet is loaded.
/// Derived-accounts list. Each row: numbered prefix (01/02/…) + short
/// address label + full derivation path as subtitle. Read-cursor only —
/// Confirm/Back both exit back to Settings (state-machine decides).
fn draw_accounts_list<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    accounts: &[(String, String)],
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();

    if accounts.is_empty() {
        // Degenerate — settings flow normally populates at least one entry,
        // but guard against the empty case rather than indexing into nothing.
        let rows: [ListRow; 1] = [ListRow::new("(no accounts)")];
        return ListScreen {
            header: HeaderKind::Title("ACCOUNTS"),
            counter: None,
            right_label: None,
            description: None,
            items: &rows,
            selected: 0,
            max_visible: 1,
            selectable: false,
            // Nothing selectable — only K3 (back) is meaningful.
            edge_hints: EdgeHints::new().k3(EdgeIcon::ArrowLeft),
        }
        .draw(display, &theme);
    }

    // Own the formatted strings so ListRow borrows survive past this closure.
    let nums: Vec<String> = (1..=accounts.len())
        .map(|i| alloc::format!("{:02}", i))
        .collect();
    let shorts: Vec<String> = accounts
        .iter()
        .map(|(_, addr)| shorten_address(addr))
        .collect();
    let rows: Vec<ListRow> = (0..accounts.len())
        .map(|i| ListRow {
            prefix: Some(&nums[i]),
            label: &shorts[i],
            subtitle: Some(&accounts[i].0),
        })
        .collect();

    let total = accounts.len();
    let sel = selected.min(total - 1);

    ListScreen {
        header: HeaderKind::Title("ACCOUNTS"),
        counter: None,
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

fn draw_settings_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();

    let items: [ListRow; 5] = [
        ListRow::with_subtitle("ADDRESS", "to receive payments"),
        ListRow::with_subtitle("BACKUP", "of your wallet"),
        ListRow::new("ACCOUNTS"),
        ListRow::with_subtitle("VERIFY", "an address"),
        ListRow::with_subtitle("RESET WALLET", "wipe memory"),
    ];
    let sel = selected.min(items.len() - 1);

    ListScreen {
        header: HeaderKind::Title("WALLET DATA"),
        counter: None,
        right_label: None,
        description: None,
        items: &items,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
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
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};
    use embedded_graphics::geometry::Size;

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
        right_label: None,
        description: None,
        items: &rows,
        selected: 0,
        max_visible: words_per_page,
        selectable: false,
        edge_hints: EdgeHints::new()
            .k1(if is_last {
                EdgeIcon::Check
            } else {
                EdgeIcon::ArrowRight
            })
            .k3(EdgeIcon::ArrowLeft),
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
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let title = alloc::format!("WORD {:02}?", word_num);
    let rows: Vec<ListRow> = options.iter().map(|s| ListRow::new(s)).collect();
    let sel = selected.min(options.len().saturating_sub(1));

    ListScreen {
        header: HeaderKind::Title(&title),
        counter: Some((check_num, total_checks)),
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 4,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Address confirmation screen.
/// Zoomed single-block view of the CompactSeedQR for hand transcription onto
/// the paper template. `block_index` is row-major over the block grid
/// (3×3 for 21×21 QR, 5×5 for 25×25 QR). A minimap at the bottom shows the
/// full QR with the current block highlighted.
fn draw_qr_block<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    compact_data: &[u8],
    block_index: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;

    let theme = Theme::faraday_240();
    display.fill_solid(
        &Rectangle::new(Point::zero(), Size::new(theme.width, theme.height)),
        theme.bg,
    )?;

    // Header spans full width; the body (zoom + minimap) shrinks by the
    // gutter so content doesn't overlap the K1/K2/K3 column.
    let header_rect = Rectangle::new(Point::zero(), Size::new(theme.width, theme.header_h));
    let body_w = (theme.width - GUTTER_W) as i32;

    // Must match the ECL used for the full seed QR so the block view shows
    // the same matrix the user is transcribing.
    let (matrix, qr_size) = match crate::qr::encode_qr::generate_qr_matrix(
        compact_data,
        crate::qr::encode_qr::QrEcLevel::L,
    ) {
        Ok(m) => m,
        Err(_) => {
            Header {
                kind: HeaderKind::Title("TRANSCRIBE"),
                counter: None,
                right_label: None,
            }
            .draw(display, &theme, header_rect)?;
            return Ok(());
        }
    };

    // Block grid: 3×3 for 21×21, 5×5 for 25×25.
    let block_side: usize = if qr_size == 21 { 7 } else { 5 };
    let blocks_per_side = qr_size / block_side;
    let total = blocks_per_side * blocks_per_side;
    let clamped = block_index.min(total - 1);
    let br = clamped / blocks_per_side;
    let bc = clamped % blocks_per_side;

    // Header + block counter in the right slot.
    Header {
        kind: HeaderKind::Title("TRANSCRIBE"),
        counter: Some((clamped + 1, total)),
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Zoomed block: fit within a 160×160 area below the header.
    let zoom_area = 160i32;
    let cell_size = (zoom_area / block_side as i32).max(1);
    let block_pixel = cell_size * block_side as i32;
    let zoom_x = (body_w - block_pixel) / 2;
    let zoom_y = theme.header_h as i32 + 6;

    // Grey backing under the zoomed block. Each cell is then filled at a
    // 1px inset, so the backing shows through as a thin border around every
    // module — stops runs of consecutive blacks (or whites) from visually
    // merging into one big rectangle.
    Rectangle::new(
        Point::new(zoom_x - 2, zoom_y - 2),
        Size::new((block_pixel + 4) as u32, (block_pixel + 4) as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(colors::CELL_BORDER))
    .draw(display)?;

    let inner = (cell_size - 1).max(1);
    for r in 0..block_side {
        for c in 0..block_side {
            let module_r = br * block_side + r;
            let module_c = bc * block_side + c;
            let on =
                module_r < qr_size && module_c < qr_size && matrix[module_r * qr_size + module_c];
            let fill = if on { colors::BLACK } else { colors::WHITE };
            Rectangle::new(
                Point::new(zoom_x + c as i32 * cell_size, zoom_y + r as i32 * cell_size),
                Size::new(inner as u32, inner as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(fill))
            .draw(display)?;
        }
    }

    // Minimap: full QR at 2px per module, current block highlighted in
    // brand cyan. We skip drawing modules that fall inside the current block
    // — the zoomed view above already shows those, and a blank highlighted
    // area makes "you-are-here" unambiguous at a glance.
    let mini_scale = 2i32;
    let mini_size = qr_size as i32 * mini_scale;
    let mini_x = (body_w - mini_size) / 2;
    let mini_y = 240 - mini_size - 4;

    Rectangle::new(
        Point::new(mini_x - 2, mini_y - 2),
        Size::new((mini_size + 4) as u32, (mini_size + 4) as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(colors::WHITE))
    .draw(display)?;

    let block_row_start = br * block_side;
    let block_row_end = block_row_start + block_side;
    let block_col_start = bc * block_side;
    let block_col_end = block_col_start + block_side;
    for r in 0..qr_size {
        for c in 0..qr_size {
            if r >= block_row_start
                && r < block_row_end
                && c >= block_col_start
                && c < block_col_end
            {
                continue;
            }
            if matrix[r * qr_size + c] {
                Rectangle::new(
                    Point::new(
                        mini_x + c as i32 * mini_scale,
                        mini_y + r as i32 * mini_scale,
                    ),
                    Size::new(mini_scale as u32, mini_scale as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(colors::BLACK))
                .draw(display)?;
            }
        }
    }

    // Current block highlight: thick brand-cyan stroke + a centred filled
    // dot inside. The dot gives a clean "you-are-here" marker without
    // relying on the block's actual QR data to fill the area.
    let hl_x = mini_x + block_col_start as i32 * mini_scale;
    let hl_y = mini_y + block_row_start as i32 * mini_scale;
    let hl_size = block_side as i32 * mini_scale;
    Rectangle::new(
        Point::new(hl_x, hl_y),
        Size::new(hl_size as u32, hl_size as u32),
    )
    .into_styled(
        PrimitiveStyleBuilder::new()
            .stroke_color(colors::BRAND_CYAN)
            .stroke_width(2)
            .build(),
    )
    .draw(display)?;

    let dot = 4i32;
    Rectangle::new(
        Point::new(hl_x + (hl_size - dot) / 2, hl_y + (hl_size - dot) / 2),
        Size::new(dot as u32, dot as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(colors::BRAND_CYAN))
    .draw(display)?;

    EdgeHints::new()
        .k1(EdgeIcon::ArrowRight)
        .k3(EdgeIcon::ArrowLeft)
        .draw(
            display,
            &theme,
            Rectangle::new(
                Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
                Size::new(GUTTER_W, theme.height - theme.header_h),
            ),
        )?;

    Ok(())
}

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
        Text::with_alignment(line, Point::new(120, y), style, Alignment::Center).draw(display)?;
        y += 20;
    }

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(
        "Press any key",
        Point::new(120, 230),
        hint,
        Alignment::Center,
    )
    .draw(display)?;

    Ok(())
}

/// TX review screen.
/// Transaction review. Scrollable list of info lines (`!`-prefixed lines
/// get danger color), with REJECT (back) / SIGN (confirm) buttons. SIGN is
/// rendered in dim color when `can_sign` is false — the loaded wallet's
/// pubkey isn't in the tx's required-signer set.
fn draw_tx_review<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    info_lines: &[String],
    scroll: usize,
    _selected: usize,
    can_sign: bool,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::layout::{split_bottom, split_top};
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;
    use embedded_graphics::primitives::{Line, PrimitiveStyle};

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    let (body_rect, _footer_rect) = split_bottom(rest, theme.footer_h as i32);

    let line_h: i32 = 14;
    let content_w = body_rect.size.width as i32 - GUTTER_W as i32 - theme.space_md - theme.space_sm;
    // Width per glyph.
    //  - HeroTitle uses embedded_graphics FONT_9X15 (fixed 9px).
    //  - Everything else uses u8g2_font_profont17_mr. Profont17 is nominally
    //    a 9-wide pixel font: the previous `/7` estimate was optimistic and
    //    let lines render ~25% wider than the available body, which is why
    //    long amounts visibly poked past the right edge into the gutter.
    //    Treat both as 9px so wrap caps match what actually renders.
    let max_chars_small = (content_w / 9).max(8) as usize;
    let max_chars_title = (content_w / 9).max(6) as usize;
    let rows = build_review_rows(info_lines, max_chars_small, max_chars_title);

    // Split the row list into the pinned hero block (runs of @H1/@H2/@HM/
    // @SWAPPAIR at the top) and the scrollable detail block (everything
    // after). A tx without hero markers collapses to "details only" and
    // renders like before.
    let hero_end = rows
        .iter()
        .position(|r| {
            !matches!(
                r.kind,
                ReviewRowKind::HeroTitle
                    | ReviewRowKind::HeroSub
                    | ReviewRowKind::HeroMeta
                    | ReviewRowKind::SwapPair
            )
        })
        .unwrap_or(rows.len());
    let hero_rows = &rows[..hero_end];
    let detail_rows = &rows[hero_end..];

    // Hero zone: each row contributes its natural height. SwapPair takes
    // two lines (amount row + symbol row) so it needs 2× line_h. HeroTitle
    // uses the 9x15 font which wants a little extra vertical room so
    // ascenders don't clip into the divider.
    let hero_pad_top = theme.space_sm;
    let hero_content_h: i32 = hero_rows
        .iter()
        .map(|r| match r.kind {
            ReviewRowKind::SwapPair => 2 * line_h + 2,
            _ => line_h,
        })
        .sum::<i32>()
        + if hero_rows
            .iter()
            .any(|r| matches!(r.kind, ReviewRowKind::HeroTitle))
        {
            2
        } else {
            0
        };
    let hero_zone_h = if hero_rows.is_empty() {
        0
    } else {
        hero_pad_top + hero_content_h + theme.space_sm
    };

    let (hero_rect, below_hero) = split_top(body_rect, hero_zone_h);
    let divider_h = if !hero_rows.is_empty() && !detail_rows.is_empty() {
        1
    } else {
        0
    };
    let (divider_rect, detail_rect) = split_top(below_hero, divider_h);

    // Scroll + counter apply only to the detail block. Counter is rendered
    // in the header so the user knows how much detail is below the fold.
    let detail_h = detail_rect.size.height as i32 - theme.space_sm * 2;
    let visible_details = (detail_h / line_h).max(1) as usize;
    let total_details = detail_rows.len();
    let max_scroll = total_details.saturating_sub(visible_details);
    let counter = if total_details > visible_details {
        Some((scroll.min(max_scroll) + 1, max_scroll + 1))
    } else {
        None
    };

    Header {
        kind: HeaderKind::Title("REVIEW TX"),
        counter,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Pinned hero block. No scroll applied — this stays put so the "what am
    // I signing?" summary is always visible.
    let hero_x = hero_rect.top_left.x + theme.space_md;
    let hero_right_x = hero_rect.top_left.x + hero_rect.size.width as i32
        - GUTTER_W as i32
        - theme.space_sm;
    let mut hero_y = hero_rect.top_left.y + hero_pad_top + 10;
    for row in hero_rows {
        match row.kind {
            ReviewRowKind::HeroTitle => {
                let style = MonoTextStyle::new(&FONT_9X15, theme.accent);
                Text::with_alignment(&row.text, Point::new(hero_x, hero_y), style, Alignment::Left)
                    .draw(display)?;
                hero_y += line_h + 2;
            }
            ReviewRowKind::HeroMeta => {
                Text::with_alignment(
                    &row.text,
                    Point::new(hero_x, hero_y),
                    theme.style_sm(theme.muted),
                    Alignment::Left,
                )
                .draw(display)?;
                hero_y += line_h;
            }
            ReviewRowKind::SwapPair => {
                // Four fields in tab-delimited text: amt_in / sym_in /
                // amt_out / sym_out. Columns split the hero content width;
                // amount renders in accent (cyan) on the amount row, symbol
                // in muted small on the line below. A small "to" connector
                // sits between the two columns on the amount row.
                let parts: Vec<&str> = row.text.splitn(4, '\t').collect();
                let (amt_in, sym_in, amt_out, sym_out) = match parts.as_slice() {
                    [a, b, c, d] => (*a, *b, *c, *d),
                    _ => ("?", "?", "?", "?"),
                };

                // "to" is rendered in profont17_mr at ~18px wide. Position
                // it centered between the two columns and leave >= 4px of
                // breathing room on each side — the right column has to
                // start *after* "to" ends, not overlap with it. The old
                // `+/- 6` offsets allowed the right column to start 6px
                // inside the "to" glyphs.
                let center_x = (hero_x + hero_right_x) / 2;
                let col_left_x = hero_x;
                let connector_x = center_x - 9; // shift to visually centre "to"
                let col_right_x = center_x + 13; // 9 (half of "to") + 4 padding

                // MonoTextStyle is Copy; u8g2 styles aren't — just rebuild
                // them per use.
                let amount_style = MonoTextStyle::new(&FONT_9X15, theme.accent);

                // Row 1: amount | "to" | amount
                Text::with_alignment(
                    amt_in,
                    Point::new(col_left_x, hero_y),
                    amount_style,
                    Alignment::Left,
                )
                .draw(display)?;
                Text::with_alignment(
                    "to",
                    Point::new(connector_x, hero_y),
                    theme.style_sm(theme.dim),
                    Alignment::Left,
                )
                .draw(display)?;
                Text::with_alignment(
                    amt_out,
                    Point::new(col_right_x, hero_y),
                    amount_style,
                    Alignment::Left,
                )
                .draw(display)?;

                // Row 2: symbol | (blank) | symbol
                let sym_y = hero_y + line_h + 2;
                Text::with_alignment(
                    sym_in,
                    Point::new(col_left_x, sym_y),
                    theme.style_sm(theme.muted),
                    Alignment::Left,
                )
                .draw(display)?;
                Text::with_alignment(
                    sym_out,
                    Point::new(col_right_x, sym_y),
                    theme.style_sm(theme.muted),
                    Alignment::Left,
                )
                .draw(display)?;

                hero_y += 2 * line_h + 2;
            }
            _ => {
                // HeroSub and any accidental non-hero that snuck through —
                // render as body text so we don't silently drop content.
                Text::with_alignment(
                    &row.text,
                    Point::new(hero_x, hero_y),
                    theme.style_sm(theme.text),
                    Alignment::Left,
                )
                .draw(display)?;
                hero_y += line_h;
            }
        }
    }

    // Thin divider between hero and details (only when both exist).
    if divider_h > 0 {
        Line::new(
            Point::new(
                divider_rect.top_left.x + theme.space_md,
                divider_rect.top_left.y,
            ),
            Point::new(
                divider_rect.top_left.x + divider_rect.size.width as i32
                    - GUTTER_W as i32
                    - theme.space_sm,
                divider_rect.top_left.y,
            ),
        )
        .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
        .draw(display)?;
    }

    // Scrollable detail block. Reserve the bottom line of the detail zone
    // for a "more below" hint when the user hasn't reached the end of the
    // details — that way the scroll affordance is always visible without
    // burning a row in the pinned hero.
    let show_more_hint = scroll < max_scroll;
    let content_visible = if show_more_hint {
        visible_details.saturating_sub(1).max(1)
    } else {
        visible_details
    };
    let detail_x = detail_rect.top_left.x + theme.space_md;
    let detail_start_y = detail_rect.top_left.y + theme.space_sm + 10;
    let end = total_details.min(scroll + content_visible);
    for (vi, idx) in (scroll..end).enumerate() {
        let row = &detail_rows[idx];
        let y = detail_start_y + vi as i32 * line_h;
        match row.kind {
            ReviewRowKind::Danger => {
                Text::with_alignment(
                    &row.text,
                    Point::new(detail_x, y),
                    theme.style_sm(theme.danger),
                    Alignment::Left,
                )
                .draw(display)?;
            }
            _ => {
                Text::with_alignment(
                    &row.text,
                    Point::new(detail_x, y),
                    theme.style_sm(theme.text),
                    Alignment::Left,
                )
                .draw(display)?;
            }
        }
    }

    if show_more_hint {
        let hint_y = detail_rect.top_left.y + detail_rect.size.height as i32 - theme.space_sm - 2;
        Text::with_alignment(
            "more below ▼",
            Point::new(detail_x, hint_y),
            theme.style_sm(theme.muted),
            Alignment::Left,
        )
        .draw(display)?;
    }

    EdgeHints::new()
        .k1(if can_sign {
            EdgeIcon::Check
        } else {
            EdgeIcon::None
        })
        .k3(EdgeIcon::Cross)
        .draw(
            display,
            &theme,
            Rectangle::new(
                Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
                Size::new(GUTTER_W, theme.height - theme.header_h),
            ),
        )?;

    Ok(())
}

#[derive(Clone, Copy)]
enum ReviewRowKind {
    HeroTitle,
    HeroSub,
    HeroMeta,
    /// Two-column swap pair. The row's `text` field carries four
    /// tab-separated values: `amount_in \t symbol_in \t amount_out \t symbol_out`.
    /// Rendered as two stacked columns (amount above symbol) with a
    /// small "to" connector between — scales to any amount length because
    /// each side gets its own column instead of fighting for one line.
    SwapPair,
    Normal,
    Danger,
}

struct ReviewRow {
    text: String,
    kind: ReviewRowKind,
}

fn build_review_rows(info_lines: &[String], max_small: usize, max_title: usize) -> Vec<ReviewRow> {
    let mut rows = Vec::new();
    for line in info_lines {
        // Swap-pair marker is kept as a single row — the renderer lays
        // the four fields out manually into two columns, so wrapping it
        // would destroy that structure.
        if let Some(rest) = line.strip_prefix("@SWAPPAIR ") {
            rows.push(ReviewRow {
                text: rest.to_string(),
                kind: ReviewRowKind::SwapPair,
            });
            continue;
        }

        let (kind, text) = if let Some(rest) = line.strip_prefix("@H1 ") {
            (ReviewRowKind::HeroTitle, rest)
        } else if let Some(rest) = line.strip_prefix("@H2 ") {
            (ReviewRowKind::HeroSub, rest)
        } else if let Some(rest) = line.strip_prefix("@HM ") {
            (ReviewRowKind::HeroMeta, rest)
        } else if let Some(rest) = line.strip_prefix("! ") {
            (ReviewRowKind::Danger, rest)
        } else if let Some(rest) = line.strip_prefix('!') {
            (ReviewRowKind::Danger, rest)
        } else {
            (ReviewRowKind::Normal, line.as_str())
        };

        let max_chars = match kind {
            ReviewRowKind::HeroTitle => max_title,
            _ => max_small,
        };

        for wrapped in wrap_line_for_width(text, max_chars) {
            rows.push(ReviewRow {
                text: wrapped,
                kind,
            });
        }
    }
    rows
}

fn wrap_line_for_width(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut out = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if word.len() > max_chars {
            if !current.is_empty() {
                out.push(current.clone());
                current.clear();
            }
            let mut start = 0;
            while start < word.len() {
                let end = (start + max_chars).min(word.len());
                out.push(word[start..end].to_string());
                start = end;
            }
            continue;
        }

        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            out.push(current.clone());
            current.clear();
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

fn draw_message_review<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    message_bytes: &[u8],
    scroll: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, GUTTER_W};
    use crate::ui::Theme;
    use embedded_graphics::geometry::Size;

    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "Sign Message", seed_loaded)?;

    let label_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    let text_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_SECONDARY);

    Text::new("Message:", Point::new(5, 35), label_style).draw(display)?;

    let text = core::str::from_utf8(message_bytes).unwrap_or("(binary data)");
    let max_chars_per_line = 38usize;
    let lines: Vec<&str> = text
        .as_bytes()
        .chunks(max_chars_per_line)
        .map(|chunk| core::str::from_utf8(chunk).unwrap_or(""))
        .collect();
    let max_visible = 12usize;
    let clamped_scroll = scroll.min(lines.len().saturating_sub(max_visible));
    for (vi, i) in (clamped_scroll..lines.len().min(clamped_scroll + max_visible)).enumerate() {
        let y = 50 + vi as i32 * 12;
        Text::new(lines[i], Point::new(5, y), text_style).draw(display)?;
    }

    Text::new(
        &format!("{} bytes", message_bytes.len()),
        Point::new(5, 50 + max_visible as i32 * 12 + 5),
        label_style,
    )
    .draw(display)?;

    let theme = Theme::faraday_240();
    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k3(EdgeIcon::Cross)
        .draw(
            display,
            &theme,
            Rectangle::new(
                Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
                Size::new(GUTTER_W, theme.height - theme.header_h),
            ),
        )?;

    Ok(())
}

/// Address verification result screen: shows the scanned address with full
/// characters and whether it belongs to the loaded seed.
fn draw_verify_address_result<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    address: &str,
    result: &crate::crypto::derivation::AddressMatch,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::crypto::derivation::AddressMatch;
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, "Verify Address", seed_loaded)?;

    let label = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    let addr_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_SECONDARY);

    Text::new("Scanned:", Point::new(8, 34), label).draw(display)?;
    let mut y = 46i32;
    for chunk in address.as_bytes().chunks(22) {
        let s = std::str::from_utf8(chunk).unwrap_or("");
        Text::new(s, Point::new(12, y), addr_style).draw(display)?;
        y += 12;
    }

    // Big result banner.
    let banner_y = 110i32;
    let (banner_text, banner_color) = match result {
        AddressMatch::Standard { .. } => ("MATCH", colors::SUCCESS),
        AddressMatch::NotFound => ("NOT YOURS", colors::DANGER),
        AddressMatch::InvalidFormat => ("INVALID", colors::WARNING),
    };
    Rectangle::new(Point::new(0, banner_y), Size::new(240, 30))
        .into_styled(PrimitiveStyle::with_fill(colors::BG_CARD))
        .draw(display)?;
    let banner_style = MonoTextStyle::new(&FONT_10X20, banner_color);
    Text::with_alignment(
        banner_text,
        Point::new(120, banner_y + 22),
        banner_style,
        Alignment::Center,
    )
    .draw(display)?;

    // Detail line: path / not-derived / not-an-address explanation.
    let detail_style = MonoTextStyle::new(&FONT_6X10, colors::TEXT_SECONDARY);
    let sub = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    match result {
        AddressMatch::Standard { .. } => {
            let path = result.path_str();
            Text::with_alignment(
                &alloc::format!("Path: {}", path),
                Point::new(120, 160),
                detail_style,
                Alignment::Center,
            )
            .draw(display)?;
        }
        AddressMatch::NotFound => {
            Text::with_alignment(
                "Not derived from this seed",
                Point::new(120, 158),
                detail_style,
                Alignment::Center,
            )
            .draw(display)?;
            Text::with_alignment(
                "(checked 10 std + CLI paths)",
                Point::new(120, 172),
                sub,
                Alignment::Center,
            )
            .draw(display)?;
        }
        AddressMatch::InvalidFormat => {
            Text::with_alignment(
                "Not a Solana address",
                Point::new(120, 158),
                detail_style,
                Alignment::Center,
            )
            .draw(display)?;
            Text::with_alignment(
                "Scan a plain address QR",
                Point::new(120, 172),
                sub,
                Alignment::Center,
            )
            .draw(display)?;
        }
    }

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(
        "Press any key to return",
        Point::new(120, 230),
        hint,
        Alignment::Center,
    )
    .draw(display)?;

    Ok(())
}

/// Show the wallet's public address as a QR. Users verify the QR in a hot
/// wallet; the truncated caption is for a quick visual double-check.
fn draw_show_address<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    address: Option<&str>,
) -> Result<(), D::Error> {
    use crate::ui::Theme;

    let theme = Theme::faraday_240();

    match address {
        Some(addr) => {
            // Render a Solana URI envelope so third-party wallets can scan it.
            // Full-screen QR — any chrome shrinks the scan target; BACK button
            // returns to settings.
            use crate::ui::widgets::Qr;
            use embedded_graphics::{
                geometry::{Point, Size},
                primitives::Rectangle,
            };

            let uri = alloc::format!("solana:{}", addr);
            let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
            display.fill_solid(&screen, theme.bg)?;
            Qr {
                data: uri.as_bytes(),
                ec: crate::qr::encode_qr::QrEcLevel::M,
                quiet: 4,
            }
            .draw(display, &theme, screen)
        }
        None => {
            // No wallet loaded — chunky alert: "!" sigil + NO WALLET title.
            draw_no_wallet_alert(display, "ADDRESS", "Create or load a", "wallet to view.")
        }
    }
}

/// About screen. Hero block uses the full pixel-art Faraday logo (mark +
/// wordmark) instead of plain text, then a muted subtitle, then key/value
/// reference rows.
fn draw_about<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::gui::logo;
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title("ABOUT"),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height),
    );
    let left_x = body_inner.top_left.x + theme.space_md;
    let right_x = body_inner.top_left.x + body_inner.size.width as i32 - theme.space_md;
    let center_x = body_inner.top_left.x + body_inner.size.width as i32 / 2;

    // Hero: full logo (mark + FARADAY wordmark) at scale 3 — same scale
    // we use on the splash so the "ABOUT" screen reads as the brand.
    let logo_scale: u32 = 3;
    let logo_w = (logo::LOGO_WIDTH * logo_scale) as i32;
    let logo_h = (logo::LOGO_HEIGHT * logo_scale) as i32;
    let logo_x = body_inner.top_left.x + (body_inner.size.width as i32 - logo_w) / 2;
    let logo_y = body_inner.top_left.y + 18;
    logo::draw_logo(display, logo_x, logo_y, logo_scale, theme.accent)?;

    // Subtitle, centered under the logo.
    let sub_y = logo_y + logo_h + 22;
    Text::with_alignment(
        "Air-gapped signer",
        Point::new(center_x, sub_y),
        theme.style_sm(theme.muted),
        Alignment::Center,
    )
    .draw(display)?;

    // Divider — same 1px hairline used by the Card widget.
    let div_y = sub_y + 12;
    Rectangle::new(
        Point::new(body_inner.top_left.x, div_y),
        Size::new(body_inner.size.width, 1),
    )
    .into_styled(PrimitiveStyle::with_fill(theme.border))
    .draw(display)?;

    // Key/value rows below.
    let rows = [
        ("VERSION", "v0.1.0"),
        ("NETWORK", "Solana"),
        ("HARDWARE", "Pi Zero 1.3"),
        ("KEYS", "RAM only"),
    ];
    let rows_top = div_y + theme.space_md;
    let remaining_h = body_inner.top_left.y + body_inner.size.height as i32 - rows_top;
    let row_h = remaining_h / rows.len() as i32;
    for (i, (label, value)) in rows.iter().enumerate() {
        let baseline = rows_top + row_h * i as i32 + row_h / 2 + 6;
        Text::with_alignment(
            label,
            Point::new(left_x, baseline),
            theme.style_sm(theme.dim),
            Alignment::Left,
        )
        .draw(display)?;
        Text::with_alignment(
            value,
            Point::new(right_x, baseline),
            theme.style_sm(theme.text),
            Alignment::Right,
        )
        .draw(display)?;
    }

    let gutter = Rectangle::new(
        Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
        Size::new(GUTTER_W, theme.height - theme.header_h),
    );
    EdgeHints::new()
        .k3(EdgeIcon::ArrowLeft)
        .draw(display, &theme, gutter)?;

    Ok(())
}

/// First-time create-flow backup warning. Shown immediately after entropy
/// is collected and the mnemonic is generated, before any plaintext word is
/// rendered. Top third holds the pen-and-paper instruction; bottom two
/// thirds hold the binary choice (CANCEL / I UNDERSTAND). Default
/// selection is CANCEL.
fn draw_create_backup_warning<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, List, ListRow, GUTTER_W};
    use crate::ui::Theme;

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title("BACKUP SEED"),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Reserve right gutter for the K1/K3 hints; rows + text live in the
    // remaining width.
    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height),
    );

    // Top third: instruction. Lines fit in ~20 chars at style_sm (mono
    // profont17, ~9px wide) inside the inner body width minus left padding.
    let third = body_inner.size.height as i32 / 3;
    let (text_rect, list_rect) = split_top(body_inner, third);

    let left_x = text_rect.top_left.x + theme.space_md;
    let mut y = text_rect.top_left.y + 18;
    for line in &["Prepare pen and paper", "to write the words", "down."] {
        Text::with_alignment(
            line,
            Point::new(left_x, y),
            theme.style_sm(theme.text),
            Alignment::Left,
        )
        .draw(display)?;
        y += 18;
    }

    // Bottom two thirds: binary choice.
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("I AM READY", "Show the words"),
        ListRow::with_subtitle("CANCEL", "Go back"),
    ];
    List {
        items: &rows,
        selected: selected.min(1),
        max_visible: 2,
        selectable: true,
    }
    .draw(display, &theme, list_rect)?;

    let gutter = Rectangle::new(
        Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
        Size::new(GUTTER_W, theme.height - theme.header_h),
    );
    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k3(EdgeIcon::ArrowLeft)
        .draw(display, &theme, gutter)?;

    Ok(())
}

/// Seed-export warning. Shown before the SeedQR to force the user to
/// acknowledge that the QR reveals the full recovery seed. Default
/// selection is CANCEL.
fn draw_export_seed_warning<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
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
        right_label: None,
        description: Some("Display your seed?"),
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Power-off confirmation. List register with the destructive consequence
/// exposed as the subtitle on the YES row. Default selection is NO.
fn draw_reset_wallet<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::{screens::ListScreen, Theme};

    let theme = Theme::faraday_240();
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("NO", "Back to wallet data"),
        ListRow::with_subtitle("YES", "Wipes wallet from RAM"),
    ];
    let sel = selected.min(1);

    ListScreen {
        header: HeaderKind::Title("RESET WALLET"),
        counter: None,
        right_label: None,
        description: None,
        items: &rows,
        selected: sel,
        max_visible: 3,
        selectable: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Passphrase mismatch error screen.
/// BIP39 word picker. Header shows progress ("WORD 4/12"), a preview band
/// renders the typed prefix with the current cursor letter highlighted
/// ("app[l]"), and the filtered candidates appear as a scrollable list.
fn draw_word_picker_new<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    picker: &crate::gui::app::WordPicker,
) -> Result<(), D::Error> {
    use crate::gui::app::{WordPicker, WORD_GRID_COLS, WORD_GRID_ROWS};
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title("WORD"),
        counter: Some((picker.word_index + 1, picker.word_count)),
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Reserve right gutter for K1/K2/K3 hints.
    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height),
    );

    // Top band: the prefix being assembled. `_` cursor follows it so the
    // user reads the live input as a single string.
    let prefix_h = 36i32;
    let (prefix_rect, grid_rect) = split_top(body_inner, prefix_h);
    let prefix_baseline = prefix_rect.top_left.y + prefix_rect.size.height as i32 - 8;
    let composed = alloc::format!("{}_", picker.prefix);
    Text::with_alignment(
        &composed,
        Point::new(prefix_rect.top_left.x + theme.space_md, prefix_baseline),
        theme.style_lg(theme.accent),
        Alignment::Left,
    )
    .draw(display)?;

    // 6×5 alphabet grid. Each cell is one letter; the trailing 4 cells in
    // the last row are blank. Selected = cyan-filled; valid = bright text;
    // dimmed = dim text (no BIP39 word starts with prefix + this letter).
    let valid = picker.valid_letters();
    let cell_w = grid_rect.size.width as i32 / WORD_GRID_COLS as i32;
    let cell_h = grid_rect.size.height as i32 / WORD_GRID_ROWS as i32;
    let grid_origin_x = grid_rect.top_left.x;
    let grid_origin_y = grid_rect.top_left.y;

    for r in 0..WORD_GRID_ROWS {
        for c in 0..WORD_GRID_COLS {
            let x = grid_origin_x + c as i32 * cell_w;
            let y = grid_origin_y + r as i32 * cell_h;
            let letter = WordPicker::cell_letter(r, c);
            let Some(ch) = letter else {
                continue;
            };
            let is_valid = valid[(ch as u8 - b'a') as usize];
            let is_selected = r == picker.cursor_row && c == picker.cursor_col;

            if is_selected && is_valid {
                Rectangle::new(
                    Point::new(x + 2, y + 2),
                    Size::new((cell_w - 4) as u32, (cell_h - 4) as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(theme.accent))
                .draw(display)?;
            }

            let color = if is_selected && is_valid {
                theme.bg
            } else if is_valid {
                theme.text
            } else {
                theme.dim
            };
            let mut buf = [0u8; 4];
            let upper = ch.to_ascii_uppercase();
            let s = upper.encode_utf8(&mut buf);
            let cx = x + cell_w / 2;
            let cy = y + cell_h / 2 + 9;
            Text::with_alignment(
                s,
                Point::new(cx, cy),
                theme.style_lg(color),
                Alignment::Center,
            )
            .draw(display)?;
        }
    }

    let gutter = Rectangle::new(
        Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
        Size::new(GUTTER_W, theme.height - theme.header_h),
    );
    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k2(EdgeIcon::Delete)
        .k3(EdgeIcon::ArrowLeft)
        .draw(display, &theme, gutter)?;

    Ok(())
}

/// Transient "word committed" flash. Big cyan word centered, with the
/// "WORD N/M" counter in the header. Auto-dismissed by `App::tick`.
fn draw_word_committed<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    word: &str,
    committed: usize,
    word_count: usize,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{Header, HeaderKind};
    use crate::ui::Theme;

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title("WORD"),
        counter: Some((committed, word_count)),
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    let cx = body_rect.top_left.x + body_rect.size.width as i32 / 2;
    let cy = body_rect.top_left.y + body_rect.size.height as i32 / 2 + 8;
    Text::with_alignment(
        word,
        Point::new(cx, cy),
        theme.style_lg(theme.accent),
        Alignment::Center,
    )
    .draw(display)?;

    Ok(())
}

/// "Load a wallet first" — user hit SIGN on the main menu without a seed.
/// Shared "no wallet" alert — chunky exclamation sigil + danger-coloured
/// "NO WALLET" title + two short body lines. Used by SIGN and ADDRESS
/// when the user reaches a wallet-required screen with no seed loaded.
fn draw_no_wallet_alert<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    header_title: &str,
    body_l1: &str,
    body_l2: &str,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title(header_title),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height),
    );
    let cx = body_inner.top_left.x + body_inner.size.width as i32 / 2;

    // Chunky "!" sigil — vertical bar over a square dot, in danger red.
    // Hand-drawn rectangles instead of a glyph so the icon reads as a
    // visual alert weight, not a typographic one.
    let sigil_w: i32 = 12;
    let sigil_top = body_inner.top_left.y + 24;
    let bar_h: i32 = 44;
    let dot_gap: i32 = 8;
    let dot_h: i32 = 12;
    let sigil_x = cx - sigil_w / 2;

    Rectangle::new(
        Point::new(sigil_x, sigil_top),
        Size::new(sigil_w as u32, bar_h as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(theme.danger))
    .draw(display)?;
    Rectangle::new(
        Point::new(sigil_x, sigil_top + bar_h + dot_gap),
        Size::new(sigil_w as u32, dot_h as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(theme.danger))
    .draw(display)?;

    let title_y = sigil_top + bar_h + dot_gap + dot_h + 30;
    Text::with_alignment(
        "NO WALLET",
        Point::new(cx, title_y),
        theme.style_lg(theme.danger),
        Alignment::Center,
    )
    .draw(display)?;

    let mut y = title_y + 26;
    for line in [body_l1, body_l2] {
        Text::with_alignment(
            line,
            Point::new(cx, y),
            theme.style_sm(theme.muted),
            Alignment::Center,
        )
        .draw(display)?;
        y += 18;
    }

    let gutter = Rectangle::new(
        Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
        Size::new(GUTTER_W, theme.height - theme.header_h),
    );
    EdgeHints::new()
        .k3(EdgeIcon::ArrowLeft)
        .draw(display, &theme, gutter)?;

    Ok(())
}

/// Address-verification result. Shows whether the scanned address was
/// derived from the loaded seed and, if so, at which account index.
fn draw_verify_address_result_card<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    address: &str,
    result: &crate::crypto::derivation::AddressMatch,
) -> Result<(), D::Error> {
    use crate::crypto::derivation::AddressMatch;
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();
    let short = shorten_address(address);
    let body = [short.as_str()];

    let (title, subtitle, account_line) = match result {
        AddressMatch::Standard { account } => {
            let line = alloc::format!("Account {}", account);
            ("MATCH", "Derived from your seed", Some(line))
        }
        AddressMatch::NotFound => ("NOT YOURS", "Not derivable from your seed", None),
        AddressMatch::InvalidFormat => ("INVALID", "Not a Solana address", None),
    };

    // Account row only on matches — no point showing "Account —" otherwise.
    let account_rows: Vec<CardRow> = account_line
        .as_deref()
        .map(|v| vec![CardRow::new("ACCOUNT", v)])
        .unwrap_or_default();

    CardScreen {
        header: HeaderKind::Title("VERIFY ADDRESS"),
        counter: None,
        right_label: None,
        title: Some(title),
        subtitle: Some(subtitle),
        body_lines: &body,
        rows: &account_rows,
        title_danger: false,
        edge_hints: EdgeHints::new().k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Paper-seed didn't decode to the currently-loaded wallet's mnemonic.
fn draw_verify_backup_seed_mismatch<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};
    let theme = Theme::faraday_240();
    let body = ["The scanned QR is not", "this wallet's seed."];
    let rows: [CardRow; 0] = [];
    CardScreen {
        header: HeaderKind::Title("VERIFY BACKUP"),
        counter: None,
        right_label: None,
        title: Some("MISMATCH"),
        subtitle: Some("Paper doesn't match"),
        body_lines: &body,
        rows: &rows,
        title_danger: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Passphrase entered during backup-verify doesn't derive the expected
/// wallet address.
fn draw_verify_backup_passphrase_mismatch<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};
    let theme = Theme::faraday_240();
    let body = ["Passphrase doesn't", "match this wallet."];
    let rows: [CardRow; 0] = [];
    CardScreen {
        header: HeaderKind::Title("VERIFY BACKUP"),
        counter: None,
        right_label: None,
        title: Some("MISMATCH"),
        subtitle: Some("Wrong passphrase"),
        body_lines: &body,
        rows: &rows,
        title_danger: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
    }
    .draw(display, &theme)
}

/// Paper backup confirmed to derive the loaded wallet (with passphrase if set).
fn draw_verify_backup_success<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    has_passphrase: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};
    let theme = Theme::faraday_240();
    let subtitle = if has_passphrase {
        "Seed + passphrase OK"
    } else {
        "Seed matches wallet"
    };
    let body = [
        "Your paper backup",
        "will restore this",
        "wallet.",
    ];
    let rows: [CardRow; 0] = [];
    CardScreen {
        header: HeaderKind::Title("VERIFY BACKUP"),
        counter: None,
        right_label: None,
        title: Some("VERIFIED"),
        subtitle: Some(subtitle),
        body_lines: &body,
        rows: &rows,
        title_danger: false,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check),
    }
    .draw(display, &theme)
}

/// Full-screen chromeless QR. Used for every "device shows a QR for the
/// world to scan" moment — seed backup, signed tx, signature, anything
/// else. Max scan target for phone cameras; any keypress returns to the
/// previous screen (handled by the state machine).
fn draw_fullscreen_qr<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    data: &[u8],
    ec: crate::qr::encode_qr::QrEcLevel,
    quiet: u32,
) -> Result<(), D::Error> {
    use crate::ui::widgets::Qr;
    use crate::ui::Theme;
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;
    Qr { data, ec, quiet }.draw(display, &theme, screen)
}

/// Passphrase mismatch error card. Any key retries the input.
fn draw_passphrase_mismatch<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::{screens::CardScreen, Theme};

    let theme = Theme::faraday_240();
    let body = ["Your two entries did", "not match. Try again."];
    let rows: [CardRow; 0] = [];

    CardScreen {
        header: HeaderKind::Title("MISMATCH"),
        counter: None,
        right_label: None,
        title: Some("NO MATCH"),
        subtitle: Some("Entries differ"),
        body_lines: &body,
        rows: &rows,
        title_danger: true,
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check),
    }
    .draw(display, &theme)
}

/// Camera entropy collection screen.
/// Camera entropy capture. No value picker — user just presses CAPTURE to
/// collect a frame of sensor noise.
fn draw_camera_entropy<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    _word_count: usize,
    frames_collected: usize,
    _seed_loaded: bool,
    has_frame: bool,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };

    let theme = Theme::faraday_240();
    // Keep in sync with src/gui/flows/create.rs::handle CreateCameraEntropy.
    let target = 2;
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    // Main loop has already blit'd the live camera frame behind us. Paint
    // only the header strip so the preview stays visible everywhere else.
    // No footer band — camera fills the full body below the header.
    if has_frame {
        display.fill_solid(&header_rect, theme.bg)?;
    } else {
        display.fill_solid(&screen, theme.bg)?;
    }

    // Counter reads as "which photo are you taking now" (1-indexed). The
    // screen transitions out the moment frames_collected hits `target`, so
    // `frames_collected + 1` is always in 1..=target while this draws.
    Header {
        kind: HeaderKind::Title("CAMERA"),
        counter: Some((frames_collected + 1, target)),
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    if !has_frame {
        let cx = body_rect.top_left.x + body_rect.size.width as i32 / 2;
        let cy = body_rect.top_left.y + body_rect.size.height as i32 / 2;
        Text::with_alignment(
            "Opening camera...",
            Point::new(cx, cy),
            theme.style_sm(theme.muted),
            Alignment::Center,
        )
        .draw(display)?;
    }

    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k3(EdgeIcon::Cross)
        .draw(
            display,
            &theme,
            Rectangle::new(
                Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
                Size::new(GUTTER_W, theme.height - theme.header_h),
            ),
        )?;

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
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
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
        right_label: None,
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
            let cell = Rectangle::new(Point::new(x, y), Size::new(cell_w as u32, cell_h as u32));
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

    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k3(EdgeIcon::ArrowLeft)
        .draw(
            display,
            &theme,
            Rectangle::new(
                Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
                Size::new(GUTTER_W, theme.height - theme.header_h),
            ),
        )?;

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
        Text::new(path, Point::new(14, y + 13), path_style).draw(display)?;

        // Truncated address
        let truncated = if addr.len() > 30 {
            alloc::format!("{}...{}", &addr[..12], &addr[addr.len() - 8..])
        } else {
            addr.clone()
        };
        let a_style = if is_selected {
            addr_style_selected
        } else {
            addr_style_normal
        };
        Text::new(&truncated, Point::new(14, y + 28), a_style).draw(display)?;
    }

    let hint = MonoTextStyle::new(&FONT_6X10, colors::TEXT_MUTED);
    Text::with_alignment(
        "Press any key to return",
        Point::new(120, 232),
        hint,
        Alignment::Center,
    )
    .draw(display)?;

    Ok(())
}

extern crate alloc;

/// Camera-backed scan overlay. Ported unchanged from the image-shrink branch —
/// diagnostic panel when the camera fails, "Opening camera..." placeholder
/// while warming up, and a centered reticle + hint bar once a frame is live.
fn draw_scan_overlay<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    _hint: &str,
    _seed_loaded: bool,
    has_frame: bool,
    error: Option<&str>,
    diag: crate::camera::ScanDiagnostics,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};
    use crate::ui::Theme;

    let theme = Theme::faraday_240();
    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));

    // Camera states we handle differently:
    // - error: dark full-screen card with the MMAL/V4L2 failure wrapped out
    // - no frame yet: dark screen with "opening..." placeholder
    // - live: translucent chrome over the live camera blit
    let has_chrome_bg = !has_frame || error.is_some();
    if has_chrome_bg {
        display.fill_solid(&screen, theme.bg)?;
    }

    // Error: dedicated card, no reticle, diagnostic body.
    if let Some(err) = error {
        let header_h = theme.header_h as i32;
        let (header_rect, body_rect) = split_top(screen, header_h);
        Header {
            kind: HeaderKind::Title(title),
            counter: None,
            right_label: None,
        }
        .draw(display, &theme, header_rect)?;

        let cx = body_rect.top_left.x + body_rect.size.width as i32 / 2;
        let mut y = body_rect.top_left.y + 24;
        Text::with_alignment(
            "Camera unavailable",
            Point::new(cx, y),
            theme.style_lg(theme.danger),
            Alignment::Center,
        )
        .draw(display)?;
        y += 22;

        const LINE_CHARS: usize = 24;
        const MAX_LINES: usize = 4;
        let mut remaining = err.as_bytes();
        for _ in 0..MAX_LINES {
            if remaining.is_empty() {
                break;
            }
            let take = remaining.len().min(LINE_CHARS);
            let line = std::str::from_utf8(&remaining[..take]).unwrap_or("");
            Text::with_alignment(
                line,
                Point::new(cx, y),
                theme.style_sm(theme.muted),
                Alignment::Center,
            )
            .draw(display)?;
            y += 16;
            remaining = &remaining[take..];
        }
        return Ok(());
    }

    // Header strip (always drawn, over the camera feed when live). Fill with
    // the theme bg so the title reads — overlay translucence isn't available
    // on a 1-bit-pixel framebuffer.
    let header_h = theme.header_h as i32;
    let (header_rect, rest) = split_top(screen, header_h);
    display.fill_solid(&header_rect, theme.bg)?;
    Header {
        kind: HeaderKind::Title(title),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    if !has_frame {
        // Warming up — show a placeholder centered in the body.
        let cx = rest.top_left.x + rest.size.width as i32 / 2;
        let cy = rest.top_left.y + rest.size.height as i32 / 2;
        Text::with_alignment(
            "OPENING CAMERA…",
            Point::new(cx, cy),
            theme.style_sm(theme.muted),
            Alignment::Center,
        )
        .draw(display)?;
        return Ok(());
    }

    // Content zone is the body minus the right gutter. No bottom banner —
    // the camera feed fills everything below the header, making the live
    // area roughly square (~212×211).
    let content_w = (rest.size.width - GUTTER_W) as i32;

    // Live reticle centered in the content zone.
    let ret_side: i32 = 160;
    let ret_x = rest.top_left.x + (content_w - ret_side) / 2;
    let ret_y = rest.top_left.y + (rest.size.height as i32 - ret_side) / 2;
    Rectangle::new(
        Point::new(ret_x, ret_y),
        Size::new(ret_side as u32, ret_side as u32),
    )
    .into_styled(PrimitiveStyle::with_stroke(theme.accent, 2))
    .draw(display)?;

    // Scan-pipeline heartbeat (pulsing dot + UR seq/total when assembling).
    draw_scan_diag(display, diag)?;

    // Right-edge hint column — K3 = back on every scan screen. K1/K2 have
    // no explicit action (scan auto-advances on a successful decode), so
    // they render as placeholder dots.
    let gutter = Rectangle::new(
        Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
        Size::new(GUTTER_W, theme.height - theme.header_h),
    );
    EdgeHints::new()
        .k3(EdgeIcon::ArrowLeft)
        .draw(display, &theme, gutter)?;

    Ok(())
}

fn draw_scan_diag<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    diag: crate::camera::ScanDiagnostics,
) -> Result<(), D::Error> {
    // Only render the diagnostic strip when an animated UR stream is in
    // progress — otherwise the live camera fills the full body. The
    // heartbeat dot + "no QR yet" text was dev chrome; the UR progress
    // bar is the only diag users actually need.
    let Some((n, total)) = diag.ur_progress else {
        return Ok(());
    };

    // Thin band just under the status bar.
    let strip = Rectangle::new(Point::new(0, 28), Size::new(240, 14));
    strip
        .into_styled(PrimitiveStyle::with_fill(colors::BG_DARK))
        .draw(display)?;

    let recent = diag
        .last_qr_at
        .map(|t| t.elapsed().as_millis() < 2000)
        .unwrap_or(false);
    let dot_color = if recent {
        colors::SOLANA_GREEN
    } else {
        colors::TEXT_MUTED
    };
    Rectangle::new(Point::new(6, 33), Size::new(6, 6))
        .into_styled(PrimitiveStyle::with_fill(dot_color))
        .draw(display)?;

    {
        // Solana-green progress bar filling left-to-right on a dim
        // track, with a small `n/total` label pinned to the right of
        // the strip. Much clearer than a text-only `UR 3/7` as frames
        // arrive — the bar width doubles as an instantly-readable
        // completion cue.
        let bar_x: i32 = 18;
        let bar_y: i32 = 34;
        let bar_h: u32 = 4;
        let bar_w_total: u32 = 180;
        Rectangle::new(Point::new(bar_x, bar_y), Size::new(bar_w_total, bar_h))
            .into_styled(PrimitiveStyle::with_fill(colors::BORDER_DEFAULT))
            .draw(display)?;
        let filled = if total == 0 {
            0
        } else {
            (n.min(total) as u32 * bar_w_total) / total as u32
        };
        if filled > 0 {
            Rectangle::new(Point::new(bar_x, bar_y), Size::new(filled, bar_h))
                .into_styled(PrimitiveStyle::with_fill(colors::FD_ACCENT))
                .draw(display)?;
        }
        let label = format!("{}/{}", n, total);
        let label_color = if n >= total {
            colors::FD_ACCENT
        } else {
            colors::TEXT_SECONDARY
        };
        let style = MonoTextStyle::new(&FONT_6X10, label_color);
        Text::with_alignment(&label, Point::new(234, 39), style, Alignment::Right).draw(display)?;
    }

    Ok(())
}
