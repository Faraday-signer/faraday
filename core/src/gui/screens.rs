//! Screen layouts — all UI pages.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_9X15, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};

use crate::gui::app::{App, Screen};
use crate::gui::colors;
use crate::gui::components::draw_status_bar;
use crate::gui::logo;
use crate::ui::Theme;

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
    /// Draw the current screen, then overlay any platform chrome (e.g. the
    /// battery icon) on top.
    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        self.draw_screen(display)?;
        #[cfg(feature = "touch-ui")]
        if let Some(battery) = self.battery {
            // The icon sits in the middle footer cell. Skip it where that cell
            // is already a Secondary control, and on the boot splash (no footer).
            if !matches!(self.screen, Screen::Splash) && !self.footer_has_secondary() {
                draw_battery(display, &self.theme, battery)?;
            }
        }
        Ok(())
    }

    /// Draw the current screen.
    fn draw_screen<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // 1 Hz text-box cursor blink (500 ms on / 500 ms off), phased off the
        // never-reset animation anchor. Only the touch keyboard reads it.
        let cursor_on = (self.splash_anim_start.elapsed().as_millis() / 500) % 2 == 0;
        match &self.screen {
            Screen::Splash => draw_boot_splash(display, &self.theme),

            Screen::ModeSelect { selected, .. } => draw_mode_select(display, &self.theme, *selected),

            Screen::Help { topic } => draw_help(display, &self.theme, *topic),

            Screen::MainMenu { selected } => {
                let addr = self.wallet.as_ref().map(|w| w.address.as_str());
                draw_main_menu(display, &self.theme, *selected, self.seed_loaded(), addr)
            }

            // Create flow
            Screen::CreateWordCount { selected } => draw_create_word_count(display, &self.theme, *selected),
            Screen::CreateMethod { selected, .. } => draw_create_method(display, &self.theme, *selected),
            Screen::CreateCameraEntropy {
                word_count,
                frames_collected,
                ..
            } => draw_camera_entropy(
                display,
                &self.theme,
                *word_count,
                *frames_collected,
                self.seed_loaded(),
                self.has_camera_frame(),
            ),
            Screen::CreateCoinFlips {
                word_count,
                bits,
                selected,
            } => draw_coin_flips(display, &self.theme, *word_count, bits, *selected, self.seed_loaded()),
            Screen::CreateDiceRolls {
                word_count,
                rolls,
                selected,
            } => draw_dice_rolls(display, &self.theme, *word_count, rolls, *selected, self.seed_loaded()),
            Screen::CreateBackupWarning { selected, .. } => {
                draw_create_backup_warning(display, &self.theme, *selected)
            }
            Screen::CreateShowWords {
                mnemonic,
                page,
                word_count,
            } => draw_show_words(display, &self.theme, mnemonic, *page, *word_count, self.seed_loaded()),
            Screen::CreateVerify {
                checks,
                current,
                options,
                correct_idx,
                selected,
                mnemonic: _,
            } => {
                let word_num = checks[*current] + 1;
                draw_verify_word(
                    display,
                    &self.theme,
                    word_num,
                    options,
                    *selected,
                    *current + 1,
                    checks.len(),
                    *correct_idx,
                    self.verify_flash,
                )
            }
            Screen::CreatePassphrasePrompt { selected, .. } => {
                draw_passphrase_prompt(display, &self.theme, *selected)
            }
            Screen::CreatePassphraseInput { grid, .. } => {
                draw_passphrase_grid(display, &self.theme, grid, "PASSPHRASE", cursor_on)
            }
            Screen::CreatePassphraseConfirm { grid, .. } => {
                draw_passphrase_grid(display, &self.theme, grid, "CONFIRM", cursor_on)
            }
            Screen::CreatePassphraseMismatch { .. } => {
                draw_passphrase_mismatch(display, &self.theme, self.seed_loaded())
            }
            Screen::CreateConfirm {
                address,
                passphrase,
                mnemonic,
                ..
            } => {
                let wc = mnemonic.split_whitespace().count();
                draw_wallet_confirm(display, &self.theme, "NEW WALLET CONFIRMATION", address, !passphrase.is_empty(), wc)
            }
            Screen::ExportSeedWarning { selected, .. } => {
                draw_export_seed_warning(display, &self.theme, *selected)
            }
            Screen::ShowWordsWarning { selected, .. } => {
                // Same red-banner gate, specifically before plaintext words.
                draw_export_seed_warning(display, &self.theme, *selected)
            }
            Screen::ExportSeedQrMenu { selected, .. } => {
                draw_export_seed_qr_menu(display, &self.theme, self.seedqr_title(), *selected)
            }
            Screen::ExportShowWords {
                mnemonic,
                page,
                word_count,
                ..
            } => draw_show_words(display, &self.theme, mnemonic, *page, *word_count, self.seed_loaded()),
            Screen::ExportSeedQr { compact_data, .. } => {
                // CompactSeedQR: raw 16/32 entropy bytes at ECL L so the grid
                // stays as small as possible for hand-transcription (12w →
                // V1 21×21, 24w → V2 25×25).
                // Seed-QR compare screen. `quiet: 2` (instead of the
                // standard 4) makes the matrix ~10% larger so side-by-side
                // visual check vs. the hand-transcribed paper is easier.
                draw_export_full_qr(display, &self.theme, compact_data)
            }
            Screen::ExportSeedQrBlock {
                compact_data,
                block_index,
                ..
            } => draw_qr_block(display, &self.theme, compact_data, *block_index, self.seed_loaded()),

            // Load flow
            Screen::LoadMethod { selected } => draw_load_method(display, &self.theme, *selected),
            Screen::LoadScanQr => draw_scan_overlay(
                display,
                &self.theme,
                "Scan SeedQR",
                "Point camera at SeedQR",
                self.seed_loaded(),
                self.has_camera_frame(),
                self.camera_error_str(),
                self.scan_diag,
            ),
            Screen::LoadWordCount { selected } => {
                // Same visual as Create's word-count picker — the choice is
                // the same, only the state-machine edges differ.
                draw_create_word_count(display, &self.theme, *selected)
            }
            Screen::LoadEnterWords { picker, .. } => draw_word_picker_new(display, &self.theme, picker),
            Screen::LoadWordCommitted {
                just_committed,
                picker,
                word_count,
                ..
            } => draw_word_committed(display, &self.theme, just_committed, picker.words.len(), *word_count),
            Screen::LoadInvalidMnemonic { word_count } => {
                draw_invalid_mnemonic(display, &self.theme, *word_count)
            }
            Screen::LoadFinalize { preview_address, selected, .. } => {
                draw_load_finalize(display, &self.theme, preview_address, *selected)
            }
            Screen::LoadPassphrasePrompt { selected, .. } => {
                draw_passphrase_prompt(display, &self.theme, *selected)
            }
            Screen::LoadPassphraseInput { grid, .. } => {
                draw_passphrase_grid(display, &self.theme, grid, "PASSPHRASE", cursor_on)
            }
            Screen::LoadPassphraseConfirm { grid, .. } => {
                draw_passphrase_grid(display, &self.theme, grid, "CONFIRM", cursor_on)
            }
            Screen::LoadPassphraseMismatch { .. } => {
                draw_passphrase_mismatch(display, &self.theme, self.seed_loaded())
            }
            Screen::LoadConfirm {
                address,
                passphrase,
                mnemonic,
                ..
            } => {
                let wc = mnemonic.split_whitespace().count();
                draw_wallet_confirm(display, &self.theme, "LOAD WALLET", address, !passphrase.is_empty(), wc)
            }

            Screen::DerivationError => draw_derivation_error(display, &self.theme),

            // Sign TX flow
            Screen::SignScanTx => draw_scan_overlay(
                display,
                &self.theme,
                "Sign TX",
                "Point camera at TX QR",
                self.seed_loaded(),
                self.has_camera_frame(),
                self.camera_error_str(),
                self.scan_diag,
            ),
            Screen::SignReview {
                tx_bytes,
                info_lines,
                parsed,
                page,
                scroll,
                selected,
                can_sign,
                ..
            } => {
                // Page 0 is the existing summary (hero + chunked details).
                // Pages 1..K-1 are new structured detail pages that read
                // from the parsed tx struct directly; page K-1 is the raw
                // bytes preview. The renderer for each detail page draws
                // the pagination counter (page+1)/K in the header.
                let interesting = crate::parser::interesting_ix_indices(parsed);
                let total_pages = 3 + interesting.len() + 1;
                match *page {
                    0 => {
                        let wallet_pk = self.wallet.as_ref().map(|w| w.keypair.public_key);
                        let zoned = crate::parser::extract_zoned(tx_bytes, parsed);
                        if let Some(action) = zoned {
                            draw_tx_review_zoned(
                                display,
                                &self.theme,
                                &action,
                                wallet_pk.as_ref(),
                                *can_sign,
                                Some((1, total_pages)),
                            )
                        } else {
                            draw_tx_review(
                                display,
                                &self.theme,
                                info_lines,
                                *scroll,
                                *selected,
                                *can_sign,
                                self.seed_loaded(),
                                Some((1, total_pages)),
                            )
                        }
                    }
                    1 => draw_tx_metadata(display, &self.theme, parsed, total_pages),
                    2 => draw_tx_ix_list(display, &self.theme, parsed, total_pages),
                    p if p < 3 + interesting.len() => {
                        let ix_index = interesting[p - 3];
                        draw_tx_ix_detail(display, &self.theme, parsed, ix_index, total_pages)
                    }
                    _ => draw_tx_raw(display, &self.theme, tx_bytes, total_pages),
                }
            }
            Screen::SignShowQr { data } => draw_fullscreen_qr(
                display,
                &self.theme,
                data.as_bytes(),
                crate::qr::encode_qr::QrEcLevel::M,
                4,
            ),
            Screen::SignMessageReview {
                message_bytes,
                scroll,
                ..
            } => draw_message_review(display, &self.theme, message_bytes, *scroll, self.seed_loaded()),
            Screen::SignMessageInput { grid } => draw_passphrase_grid(display, &self.theme, grid, "SIGN MSG", cursor_on),
            Screen::SignMessageResult { signature_hex } => draw_fullscreen_qr(
                display,
                &self.theme,
                signature_hex.as_bytes(),
                crate::qr::encode_qr::QrEcLevel::M,
                4,
            ),

            // Settings
            Screen::SettingsMenu { selected } => {
                draw_settings_menu(display, &self.theme, *selected)
            }
            Screen::SettingsShowAddress => {
                draw_show_address(display, &self.theme, self.wallet.as_ref().map(|w| w.address.as_str()))
            }
            Screen::SettingsShowAddressText => {
                draw_show_address_text(display, &self.theme, self.wallet.as_ref().map(|w| w.address.as_str()))
            }
            Screen::SettingsAbout => draw_about(display, &self.theme, self.seed_loaded()),
            Screen::SettingsPowerOff { selected } => draw_reset_wallet(display, &self.theme, *selected),

            // Verify backup flow
            Screen::VerifyBackupScan => {
                #[cfg(any(feature = "_desktop_sim", target_os = "linux", feature = "touch-ui"))]
                {
                    draw_scan_overlay(
                        display,
                        &self.theme,
                        "Verify Backup",
                        "Scan your paper SeedQR",
                        self.seed_loaded(),
                        self.has_camera_frame(),
                        self.camera_error_str(),
                        self.scan_diag,
                    )
                }
                // Fallback for `simulator_no_cam` builds (window without nokhwa) on
                // non-Linux hosts: no live camera, so prompt the user instead.
                #[cfg(not(any(feature = "_desktop_sim", target_os = "linux", feature = "touch-ui")))]
                {
                    draw_message(
                        display,
                        &self.theme,
                        "Verify Backup",
                        "Scan your paper\nSeedQR",
                        self.seed_loaded(),
                    )
                }
            }
            Screen::VerifyBackupSeedMismatch => draw_verify_backup_seed_mismatch(display, &self.theme),
            Screen::VerifyBackupPassphrase { grid } => {
                draw_passphrase_grid(display, &self.theme, grid, "PASSPHRASE", cursor_on)
            }
            Screen::VerifyBackupPassphraseMismatch => {
                draw_verify_backup_passphrase_mismatch(display, &self.theme)
            }
            Screen::VerifyBackupSuccess => {
                let has_passphrase = self
                    .wallet
                    .as_ref()
                    .map_or(false, |w| !w.passphrase.is_empty());
                draw_verify_backup_success(display, &self.theme, has_passphrase)
            }
        }
    }
}

/// Boot splash: logo centered on the screen, no animation. Shown for the
/// ~2 s power-on beat before the menu comes up.
/// Draw the external-battery indicator in the middle footer cell: an outlined
/// battery whose interior fills proportionally to charge (red at ≤20%, green
/// otherwise).
#[cfg(feature = "touch-ui")]
fn draw_battery<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    status: crate::gui::app::BatteryStatus,
) -> Result<(), D::Error> {
    let footer_h = crate::ui::widgets::FOOTER_H as i32;
    // Centre of the middle footer third.
    let cx = theme.width as i32 / 2;
    let cy = theme.height as i32 - footer_h / 2;

    const BODY_W: i32 = 30;
    const BODY_H: i32 = 16;
    const NUB_W: i32 = 3;
    const NUB_H: i32 = 6;
    const PAD: i32 = 2;

    let x = cx - (BODY_W + NUB_W) / 2;
    let y = cy - BODY_H / 2;

    // Outlined body.
    Rectangle::new(Point::new(x, y), Size::new(BODY_W as u32, BODY_H as u32))
        .into_styled(PrimitiveStyle::with_stroke(theme.text, 1))
        .draw(display)?;
    // Positive terminal nub on the right.
    Rectangle::new(
        Point::new(x + BODY_W, cy - NUB_H / 2),
        Size::new(NUB_W as u32, NUB_H as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(theme.text))
    .draw(display)?;

    // Proportional charge fill: red when low (≤20%), green otherwise.
    let pct = status.percent.min(100) as i32;
    let max_fill = BODY_W - 2 * PAD;
    let fill_w = max_fill * pct / 100;
    let fill_color = if pct <= 20 {
        theme.danger
    } else {
        Rgb565::new(4, 58, 8) // green
    };
    if fill_w > 0 {
        Rectangle::new(
            Point::new(x + PAD, y + PAD),
            Size::new(fill_w as u32, (BODY_H - 2 * PAD) as u32),
        )
        .into_styled(PrimitiveStyle::with_fill(fill_color))
        .draw(display)?;
    }

    Ok(())
}

pub fn draw_boot_splash<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
) -> Result<(), D::Error> {
    display.clear(colors::FD_BG)?;
    let screen_w = theme.width as i32;
    let screen_h = theme.height as i32;
    const SCALE: u32 = 3;
    let logo_w = (logo::LOGO_WIDTH * SCALE) as i32;
    let logo_h = (logo::LOGO_HEIGHT * SCALE) as i32;
    let x = (screen_w - logo_w) / 2;
    let y = (screen_h - logo_h) / 2;
    logo::draw_logo(display, x, y, SCALE, colors::FD_ACCENT)?;
    Ok(())
}

/// Sleep / reposo screen. Same approach as before (small bitmap rendered
/// at integer scale so every "pixel" is a chunky block) — only the art has
/// changed. Adds a DVD-screensaver bounce: each axis moves independently,
/// with a hard inset from the edges so the logo never touches the corners.
pub fn draw_splash<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    elapsed_ms: u64,
) -> Result<(), D::Error> {
    display.clear(colors::FD_BG)?;

    let screen_w = theme.width as i32;
    let screen_h = theme.height as i32;
    const INSET: i32 = 16;
    const SCALE: u32 = 3;
    let logo_w = (logo::LOGO_WIDTH * SCALE) as i32;
    let logo_h = (logo::LOGO_HEIGHT * SCALE) as i32;

    let x_min = INSET;
    let x_max = screen_w - INSET - logo_w;
    let y_min = INSET;
    let y_max = screen_h - INSET - logo_h;

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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


    // Mode-named rows so the screen reads at a glance — no question to
    // parse, no YES/NO ambiguity. The brand logo header keeps the
    // welcome feeling without competing for the eye. Row order is
    // chosen so the first-time-user default (GUIDED) is the top row.
    let rows: [ListRow; 2] = [
        ListRow::with_subtitle("GUIDED MODE", "Help between steps"),
        ListRow::with_subtitle("QUICK MODE", "Skip the help screens"),
    ];
    let sel = selected.clamp(0, 1);

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

/// Help interstitial. Bigger body font (profont22) than the default card so
/// the copy is comfortably readable from arm's length; lines are wrapped with
/// `wrap_line_for_width` so no word is ever cut. The body width is sized so a
/// 22-character word still fits — long words simply move to the next line
/// rather than truncating.
fn draw_help<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    topic: crate::gui::app::HelpTopic,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};



    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);
    Header {
        kind: HeaderKind::Title(topic.title()),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Body sits inside the chrome: left of the right-edge gutter (key builds)
    // or above the bottom action bar (touch builds). One const is zero.
    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height - FOOTER_H),
    );
    let left_x = body_inner.top_left.x + theme.space_md;
    let usable_w = body_inner.size.width as i32 - 2 * theme.space_md;

    // profont22 is ~12 px wide. Pick the largest char count that fits the
    // usable width with a small safety margin so descenders / wider glyphs
    // never bleed past the gutter.
    let approx_char_w = 12i32;
    let max_chars = ((usable_w - 4) / approx_char_w).max(8) as usize;

    let mut wrapped: Vec<String> = Vec::new();
    for raw_line in topic.body().split('\n') {
        if raw_line.trim().is_empty() {
            wrapped.push(String::new());
            continue;
        }
        wrapped.extend(wrap_line_for_width(raw_line, max_chars));
    }

    // Center the block vertically within the body so short copies don't sit
    // glued to the top edge, and long copies still have breathing room.
    let line_h = 26i32;
    let block_h = wrapped.len() as i32 * line_h;
    let body_h = body_inner.size.height as i32;
    let top_padding = ((body_h - block_h) / 2).max(theme.space_md);
    let mut cursor_y = body_inner.top_left.y + top_padding + 18;

    for line in &wrapped {
        Text::with_alignment(
            line,
            Point::new(left_x, cursor_y),
            theme.style_md(theme.text),
            Alignment::Left,
        )
        .draw(display)?;
        cursor_y += line_h;
    }

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

/// Main menu: list register (Header + List + right-edge hints) via `src/ui/`.
fn draw_main_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    selected: usize,
    seed_loaded: bool,
    address: Option<&str>,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;



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
        edge_hints: EdgeHints::new().k1(EdgeIcon::Check).k3(EdgeIcon::ArrowLeft),
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
    theme: &Theme,
    word_count: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::screens::CardScreen;

    let _ = word_count;

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

pub fn draw_computing<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, HeaderKind};
    use crate::ui::screens::CardScreen;

    let body = ["Deriving keys.", "", "Please wait…"];

    CardScreen {
        header: HeaderKind::Title("COMPUTING"),
        counter: None,
        right_label: None,
        title: Some("KEY DERIVATION"),
        subtitle: None,
        body_lines: &body,
        rows: &[],
        title_danger: false,
        edge_hints: EdgeHints::new(),
    }
    .draw(display, theme)
}

/// Shown briefly when the user long-presses BOOT to power the device off, just
/// before it enters deep sleep.
pub fn draw_powering_off<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, HeaderKind};
    use crate::ui::screens::CardScreen;

    let body = ["Powering off.", "", "Please wait…"];

    CardScreen {
        header: HeaderKind::Title("POWER OFF"),
        counter: None,
        right_label: None,
        title: Some("SHUTTING DOWN"),
        subtitle: None,
        body_lines: &body,
        rows: &[],
        title_danger: false,
        edge_hints: EdgeHints::new(),
    }
    .draw(display, theme)
}

fn draw_derivation_error<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, CardRow, HeaderKind};
    use crate::ui::screens::CardScreen;


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
    theme: &Theme,
    grid: &crate::gui::app::CharGrid,
    title: &str,
    cursor_on: bool,
) -> Result<(), D::Error> {
    // Touch builds use the split QWERTY on-screen keyboard instead of the
    // physical-key navigation grid.
    #[cfg(feature = "touch-ui")]
    {
        crate::gui::touch_keyboard::draw(display, theme, grid, title, cursor_on)
    }
    #[cfg(not(feature = "touch-ui"))]
    {
    let _ = cursor_on; // cursor blink is a touch-keyboard concern only
    use crate::gui::app::{GridAction, GRID_COLS};
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};

    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };


    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    // Body shrinks to clear the chrome — the right gutter (key builds) or the
    // bottom action bar (touch builds) — so keyboard cells never overlap it.
    let body_rect = Rectangle::new(
        rest.top_left,
        Size::new(rest.size.width - GUTTER_W, rest.size.height - FOOTER_H),
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
    draw_preview(display, theme, preview_rect, &grid.text)?;

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
}

/// Preview band for the passphrase grid: `••••• 5 CHARS` style.
#[cfg(not(feature = "touch-ui"))]
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
    theme: &Theme,
    title: &str,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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

fn draw_load_finalize<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    preview_address: &str,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;



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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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
    theme: &Theme,
    title: &str,
    address: &str,
    has_passphrase: bool,
    word_count: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::screens::CardScreen;



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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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
fn draw_settings_menu<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;



    let items: [ListRow; 4] = [
        ListRow::with_subtitle("ADDRESS", "Read or write down"),
        ListRow::with_subtitle("ADDRESS QR", "Scan to receive"),
        ListRow::with_subtitle("BACKUP", "Save your seed"),
        ListRow::with_subtitle("RESET WALLET", "Wipe from memory"),
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
    theme: &Theme,
    mnemonic: &str,
    page: usize,
    word_count: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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

/// Word verification quiz. List register — the question is the header title
/// (e.g. "WORD 04?"), the check counter sits in the header's counter slot,
/// and the 4 options are list rows. Selected row gets the cyan highlight as
/// usual.
fn draw_verify_word<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    word_num: usize,
    options: &[String],
    selected: usize,
    check_num: usize,
    total_checks: usize,
    correct_idx: usize,
    flashing: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


    let title = alloc::format!("WORD {:02}?", word_num);
    let rows: Vec<ListRow> = options.iter().map(|s| ListRow::new(s)).collect();
    let sel = selected.min(options.len().saturating_sub(1));

    // While a tapped pick is being flashed, recolour the selection accent for
    // this render only: green for the correct word (about to advance), red for
    // a wrong one (about to reset). The resting cyan accent just means
    // "selected", so it would read as neither right nor wrong here.
    let mut theme = theme.clone();
    if flashing {
        theme.accent = if sel == correct_idx {
            colors::SUCCESS
        } else {
            theme.danger
        };
    }

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
    theme: &Theme,
    compact_data: &[u8],
    block_index: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};



    display.fill_solid(
        &Rectangle::new(Point::zero(), Size::new(theme.width, theme.height)),
        theme.bg,
    )?;

    // Header spans full width; the body (zoom + minimap) shrinks to clear the
    // chrome — the right gutter (key builds) or the bottom bar (touch builds).
    let header_rect = Rectangle::new(Point::zero(), Size::new(theme.width, theme.header_h));
    let body_w = (theme.width - GUTTER_W) as i32;
    let body_bottom = (theme.height - FOOTER_H) as i32;

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
    let mini_y = body_bottom - mini_size - 4;

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

// Only compiled for `simulator_no_cam` on non-Linux hosts (the one place
// VerifyBackupScan needs a no-camera fallback). See `Screen::VerifyBackupScan`.
#[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
fn draw_message<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    title: &str,
    message: &str,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, theme, title, seed_loaded)?;

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

/// 3-zone TX review for simple, single-action transactions (today: System
/// transfers). Header reads as a verb (`SEND`); the body splits into three
/// equal-height cells aligned with the K1/K2/K3 button gutter so every row
/// divider lines up with a button-cell boundary. Full base58 addresses are
/// shown — no truncation — wrapped to two lines via the existing helper.
/// Falls back to `draw_tx_review` for anything more complex (swaps,
/// multi-step txs, unknown programs).
fn draw_tx_review_zoned<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    action: &crate::parser::ZonedAction,
    wallet_pubkey: Option<&[u8; 32]>,
    can_sign: bool,
    page_counter: Option<(usize, usize)>,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};

    use embedded_graphics::primitives::{Line, PrimitiveStyle};


    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);
    let title = build_zoned_title(action);
    Header {
        kind: HeaderKind::Title(title.as_ref()),
        counter: page_counter,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Three equal body zones, fitted above the chrome (gutter or bottom bar).
    // The remainder pixel from `body_h % 3` goes to the bottom zone so the
    // K-cell dividers land on integer y values.
    let body_h = (body_rect.size.height - FOOTER_H) as i32;
    let zone_h = body_h / 3;
    let bottom_h = body_h - 2 * zone_h;
    let zone1_top = body_rect.top_left.y;
    let zone2_top = zone1_top + zone_h;
    let zone3_top = zone2_top + zone_h;

    let v_x = theme.width as i32 - GUTTER_W as i32;
    Line::new(
        Point::new(v_x, body_rect.top_left.y),
        Point::new(v_x, theme.height as i32 - 1),
    )
    .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
    .draw(display)?;

    for y in [zone2_top, zone3_top] {
        Line::new(Point::new(0, y), Point::new(theme.width as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
            .draw(display)?;
    }

    let zone_rect = |top_y: i32, height: u32| {
        Rectangle::new(
            Point::new(body_rect.top_left.x, top_y),
            Size::new(body_rect.size.width - GUTTER_W, height),
        )
    };

    match action {
        crate::parser::ZonedAction::Send {
            from,
            to,
            amount_lamports,
            ..
        } => {
            let from_b58 = bs58::encode(from).into_string();
            let to_b58 = bs58::encode(to).into_string();
            let from_is_self = wallet_pubkey.map_or(false, |w| w == from);

            // profont17 ≈ 9 px wide on a 240 px panel with 12 px padding +
            // 24 px gutter leaves ~22 chars per line, so a 32–44 char base58
            // address wraps to 2 lines max.
            const ADDR_WRAP: usize = 22;

            draw_zone_address(
                display,
                &theme,
                zone_rect(zone1_top, zone_h as u32),
                "FROM",
                &from_b58,
                from_is_self,
                ADDR_WRAP,
            )?;

            draw_zone_address(
                display,
                &theme,
                zone_rect(zone2_top, zone_h as u32),
                "TO",
                &to_b58,
                false,
                ADDR_WRAP,
            )?;

            let amount_str = crate::parser::compact_amount(
                &crate::parser::token_registry::format_amount(*amount_lamports, 9),
            );
            draw_zone_amount(
                display,
                &theme,
                zone_rect(zone3_top, bottom_h as u32),
                "AMOUNT",
                &amount_str,
                "SOL",
            )?;
        }
        crate::parser::ZonedAction::Swap {
            sell_amount,
            sell_symbol,
            buy_amount,
            buy_symbol,
            fee_lamports,
            fee_payer,
            dex_name,
            slippage_bps,
            route_hops,
            ..
        } => {
            draw_zone_amount(
                display,
                &theme,
                zone_rect(zone1_top, zone_h as u32),
                "SEND",
                sell_amount,
                if sell_symbol.is_empty() { "?" } else { sell_symbol.as_str() },
            )?;

            // Empty `buy_amount` is the explicit "device can't verify"
            // sentinel — render as em dash rather than a fabricated value.
            let receive_display = if buy_amount.is_empty() {
                "—"
            } else {
                buy_amount.as_str()
            };
            draw_zone_amount(
                display,
                &theme,
                zone_rect(zone2_top, zone_h as u32),
                "RECEIVE",
                receive_display,
                if buy_symbol.is_empty() { "?" } else { buy_symbol.as_str() },
            )?;

            let fee_value = crate::parser::compact_amount(
                &crate::parser::token_registry::format_amount(*fee_lamports, 9),
            );
            let fee_str = alloc::format!("{} SOL", fee_value);
            let payer_b58 = bs58::encode(fee_payer).into_string();
            let payer_is_self = wallet_pubkey.map_or(false, |w| w == fee_payer);
            draw_zone_fee(
                display,
                &theme,
                zone_rect(zone3_top, bottom_h as u32),
                &fee_str,
                &payer_b58,
                payer_is_self,
                *slippage_bps,
                *route_hops,
                dex_name.as_str(),
            )?;
        }
    }

    // K1 = sign (dim if can't), K2 = next page, K3 = reject.
    let gutter = Rectangle::new(
        Point::new(v_x, body_rect.top_left.y),
        Size::new(GUTTER_W, theme.height - theme.header_h),
    );
    // Touch builds page on a body tap, so the middle action-bar cell is
    // dropped; key builds keep the "→ next" stand-in.
    let next_hint = if cfg!(feature = "touch-ui") {
        EdgeIcon::None
    } else {
        EdgeIcon::ArrowRight
    };
    let mut hints = EdgeHints::new()
        .k2(next_hint)
        .k3(EdgeIcon::CrossDanger);
    if can_sign {
        hints = hints.k1(EdgeIcon::Check);
    }
    hints.draw(display, &theme, gutter)?;

    Ok(())
}

/// Render an address-bearing zone: small label on top, the address itself
/// wrapped to (at most) two lines below. `is_self` adds a `(you)` chip in
/// the accent color next to the label.
fn draw_zone_address<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &crate::ui::Theme,
    rect: Rectangle,
    label: &str,
    address: &str,
    is_self: bool,
    wrap_chars: usize,
) -> Result<(), D::Error> {
    let inner_x = rect.top_left.x + theme.space_md;
    let label_y = rect.top_left.y + 14;
    Text::with_alignment(
        label,
        Point::new(inner_x, label_y),
        theme.style_sm(theme.muted),
        Alignment::Left,
    )
    .draw(display)?;
    if is_self {
        let chip_x = inner_x + label.len() as i32 * 9 + 6;
        Text::with_alignment(
            "(you)",
            Point::new(chip_x, label_y),
            theme.style_sm(theme.accent),
            Alignment::Left,
        )
        .draw(display)?;
    }

    let wrapped = wrap_line_for_width(address, wrap_chars);
    let mut y = label_y + 18;
    for line in wrapped.iter().take(2) {
        Text::with_alignment(
            line,
            Point::new(inner_x, y),
            theme.style_sm(theme.text),
            Alignment::Left,
        )
        .draw(display)?;
        y += 18;
    }

    Ok(())
}

/// Render the amount zone: small label top-left, currency symbol pinned
/// to the top-right (always visible regardless of amount length), big
/// number on the bottom row.
fn draw_zone_amount<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &crate::ui::Theme,
    rect: Rectangle,
    label: &str,
    amount: &str,
    symbol: &str,
) -> Result<(), D::Error> {
    let inner_x = rect.top_left.x + theme.space_md;
    let right_x = rect.top_left.x + rect.size.width as i32 - theme.space_sm;

    let label_y = rect.top_left.y + 18;
    Text::with_alignment(
        label,
        Point::new(inner_x, label_y),
        theme.style_md(theme.muted),
        Alignment::Left,
    )
    .draw(display)?;
    Text::with_alignment(
        symbol,
        Point::new(right_x, label_y),
        theme.style_md(theme.accent),
        Alignment::Right,
    )
    .draw(display)?;

    let amount_y = rect.top_left.y + rect.size.height as i32 - 14;
    Text::with_alignment(
        amount,
        Point::new(right_x, amount_y),
        theme.style_lg(theme.text),
        Alignment::Right,
    )
    .draw(display)?;

    Ok(())
}

fn build_zoned_title(action: &crate::parser::ZonedAction) -> alloc::borrow::Cow<'static, str> {
    use alloc::borrow::Cow;
    match action {
        crate::parser::ZonedAction::Send { .. } => Cow::Borrowed("APPROVE SEND"),
        crate::parser::ZonedAction::Swap { .. } => Cow::Borrowed("APPROVE SWAP"),
    }
}

/// FEE row + secondary row, picked in priority order:
/// `SLIPPAGE` (numeric, color-coded) → `ROUTE` (dex/hops) → `PAYER` (full addr).
fn draw_zone_fee<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &crate::ui::Theme,
    rect: Rectangle,
    fee: &str,
    payer: &str,
    payer_is_self: bool,
    slippage_bps: Option<u16>,
    route_hops: Option<u32>,
    dex_name: &str,
) -> Result<(), D::Error> {
    let inner_x = rect.top_left.x + theme.space_md;
    let right_x = rect.top_left.x + rect.size.width as i32 - theme.space_sm;
    let fee_y = rect.top_left.y + 14;
    Text::with_alignment(
        "FEE",
        Point::new(inner_x, fee_y),
        theme.style_sm(theme.muted),
        Alignment::Left,
    )
    .draw(display)?;
    Text::with_alignment(
        fee,
        Point::new(right_x, fee_y),
        theme.style_sm(theme.text),
        Alignment::Right,
    )
    .draw(display)?;

    let row2_y = fee_y + 18;
    if let Some(bps) = slippage_bps {
        // > 5% danger, > 1% accent (warning), else neutral.
        let color = if bps > 500 {
            theme.danger
        } else if bps > 100 {
            theme.accent
        } else {
            theme.text
        };
        Text::with_alignment(
            "SLIPPAGE",
            Point::new(inner_x, row2_y),
            theme.style_sm(theme.muted),
            Alignment::Left,
        )
        .draw(display)?;
        let slip_str = alloc::format!("{:.2}%", bps as f32 / 100.0);
        Text::with_alignment(
            &slip_str,
            Point::new(right_x, row2_y),
            theme.style_sm(color),
            Alignment::Right,
        )
        .draw(display)?;
        return Ok(());
    }

    if let Some(hops) = route_hops {
        Text::with_alignment(
            "ROUTE",
            Point::new(inner_x, row2_y),
            theme.style_sm(theme.muted),
            Alignment::Left,
        )
        .draw(display)?;
        let route_str = if dex_name.is_empty() {
            alloc::format!("{} hops", hops)
        } else {
            alloc::format!("{} · {} hops", dex_name, hops)
        };
        Text::with_alignment(
            &route_str,
            Point::new(right_x, row2_y),
            theme.style_sm(theme.text),
            Alignment::Right,
        )
        .draw(display)?;
        return Ok(());
    }

    Text::with_alignment(
        "PAYER",
        Point::new(inner_x, row2_y),
        theme.style_sm(theme.muted),
        Alignment::Left,
    )
    .draw(display)?;
    if payer_is_self {
        let chip_x = inner_x + "PAYER".len() as i32 * 9 + 6;
        Text::with_alignment(
            "(you)",
            Point::new(chip_x, row2_y),
            theme.style_sm(theme.accent),
            Alignment::Left,
        )
        .draw(display)?;
    }

    let wrapped = wrap_line_for_width(payer, 22);
    let mut y = row2_y + 16;
    for line in wrapped.iter().take(2) {
        Text::with_alignment(
            line,
            Point::new(inner_x, y),
            theme.style_sm(theme.text),
            Alignment::Left,
        )
        .draw(display)?;
        y += 16;
    }

    Ok(())
}

/// TX review screen.
/// Transaction review. Scrollable list of info lines (`!`-prefixed lines
/// get danger color), with REJECT (back) / SIGN (confirm) buttons. SIGN is
/// rendered in dim color when `can_sign` is false — the loaded wallet's
/// pubkey isn't in the tx's required-signer set.
fn draw_tx_review<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    info_lines: &[String],
    scroll: usize,
    _selected: usize,
    can_sign: bool,
    _seed_loaded: bool,
    // `page_counter`: when `Some((page, total))` the header counter shows
    // page navigation (e.g. "1/8") instead of the in-page scroll counter
    // ("3/39"). The SignReview pagination model treats page 0 as one page
    // among N, so we override the legacy scroll counter to match.
    page_counter: Option<(usize, usize)>,
) -> Result<(), D::Error> {
    use crate::ui::layout::{split_bottom, split_top};
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};

    use embedded_graphics::primitives::{Line, PrimitiveStyle};


    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    // Reserve the bottom strip: existing breathing margin on key builds, or
    // the action bar on touch builds — whichever is larger.
    let (body_rect, _footer_rect) = split_bottom(rest, theme.footer_h.max(FOOTER_H) as i32);

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
    // `page_counter` is `Some((page+1, total_pages))` when called from the
    // SignReview pagination dispatch — wins over the legacy scroll counter
    // because the new model doesn't scroll within page 0. Falls back to
    // the scroll counter for any caller that doesn't pass one (none today,
    // but keeps the function general).
    let counter = page_counter.or_else(|| {
        if total_details > visible_details {
            Some((scroll.min(max_scroll) + 1, max_scroll + 1))
        } else {
            None
        }
    });

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

    // K2 advances to the next review page (TX METADATA, instructions,
    // raw bytes) on key builds. Touch builds page on a body tap, so the
    // middle action-bar cell is dropped — it had no handler there.
    let next_hint = if cfg!(feature = "touch-ui") {
        EdgeIcon::None
    } else {
        EdgeIcon::ArrowRight
    };
    EdgeHints::new()
        .k1(if can_sign {
            EdgeIcon::Check
        } else {
            EdgeIcon::None
        })
        .k2(next_hint)
        .k3(EdgeIcon::CrossDanger)
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

// ────────────────────────────────────────────────────────────────────
// Detail pages (1..K-1) for SignReview.
//
// Each page renders directly from `ParsedTransaction` — no marker
// strings, no flat-list partitioning. They share the same shell
// layout: header with title + (page+1)/K counter, gutter on the right
// with the standard sign / next-page / cancel hints, body filled with
// label-value rows.
// ────────────────────────────────────────────────────────────────────

/// Rows for a detail page. Keeps the renderer dumb — pages just build a
/// list of these, the shell handles layout.
struct DetailRow<'a> {
    label: &'a str,
    value: String,
}

/// Shared shell for pages 1..K-1. Header = title + pagination counter,
/// body = a vertically-stacked list of `LABEL  value` rows, gutter =
/// standard sign / next / cancel triplet.
fn draw_detail_shell<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    title: &str,
    page_one_indexed: usize,
    total_pages: usize,
    rows: &[DetailRow<'_>],
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};



    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title(title),
        counter: Some((page_one_indexed, total_pages)),
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Body sits inside the chrome: left of the gutter (key builds) or above the
    // bottom action bar (touch builds).
    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height - FOOTER_H),
    );
    let label_x = body_inner.top_left.x + theme.space_md;
    // Label column width — "FEE PAYER" (9), "INSTRUCTIONS" (12), "PROGRAM" (7)
    // are the headline labels. Profont17 is ~9 px per glyph, so 13 chars
    // gives the value column a consistent left edge with one char of
    // breathing room before the longest label.
    let label_col_chars: i32 = 13;
    let approx_char_w: i32 = 9;
    let label_col_w = label_col_chars * approx_char_w;
    let value_x = label_x + label_col_w;
    // Available width for the value column (left of the gutter, minus a
    // small breathing margin) and the matching character cap.
    let value_w_px = body_inner.size.width as i32 - label_col_w - theme.space_md - 4;
    let value_max_chars = (value_w_px / approx_char_w).max(4) as usize;
    // List-style rows (empty label) get the full body width — no point
    // indenting them under an empty label column.
    let full_w_px = body_inner.size.width as i32 - 2 * theme.space_md;
    let full_max_chars = (full_w_px / approx_char_w).max(4) as usize;
    let row_h: i32 = 18;
    let mut y = body_inner.top_left.y + theme.space_md + 12;

    for row in rows {
        if row.label.is_empty() {
            // No label → value runs from the left margin to the gutter.
            Text::with_alignment(
                &truncate_with_ellipsis(&row.value, full_max_chars),
                Point::new(label_x, y),
                theme.style_sm(theme.text),
                Alignment::Left,
            )
            .draw(display)?;
        } else {
            Text::with_alignment(
                row.label,
                Point::new(label_x, y),
                theme.style_sm(theme.muted),
                Alignment::Left,
            )
            .draw(display)?;
            Text::with_alignment(
                &truncate_with_ellipsis(&row.value, value_max_chars),
                Point::new(value_x, y),
                theme.style_sm(theme.text),
                Alignment::Left,
            )
            .draw(display)?;
        }
        y += row_h;
    }

    let gutter = Rectangle::new(
        Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
        Size::new(GUTTER_W, theme.height - theme.header_h),
    );
    // K2 advanced pages on key builds (shown as a "→ next" stand-in until a
    // dedicated down-arrow exists). Touch builds page on a body tap instead,
    // so the middle action-bar cell is dropped (it had no handler there).
    let next_hint = if cfg!(feature = "touch-ui") {
        EdgeIcon::None
    } else {
        EdgeIcon::ArrowRight
    };
    EdgeHints::new()
        .k1(EdgeIcon::Check)
        .k2(next_hint)
        .k3(EdgeIcon::CrossDanger)
        .draw(display, &theme, gutter)?;

    Ok(())
}

fn short_pubkey(s: &str) -> String {
    if s.len() <= 12 {
        s.to_string()
    } else {
        format!("{}…{}", &s[..6], &s[s.len() - 6..])
    }
}

/// Trim a string to fit a column-character cap, appending `…` when it
/// gets cut. Char-aware so multi-byte glyphs (e.g. the existing pubkey
/// truncation indicator) don't trip on byte boundaries.
fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    if max_chars <= 1 {
        return "…".to_string();
    }
    let take = max_chars - 1;
    let mut out: String = s.chars().take(take).collect();
    out.push('…');
    out
}

/// Match the formatting used by `parser::system::lamports_to_sol` so the
/// detail page reads identically to the values the parser emits in the
/// flat info_lines view (e.g. "0.001 SOL", "1.5 SOL", "1 SOL"). Inlined
/// rather than reaching into `parser::system` because that module is
/// crate-private.
fn lamports_to_sol_str(lamports: u64) -> String {
    let sol = lamports / 1_000_000_000;
    let frac = lamports % 1_000_000_000;
    if frac == 0 {
        format!("{} SOL", sol)
    } else {
        let frac_str = format!("{:09}", frac);
        format!("{}.{} SOL", sol, frac_str.trim_end_matches('0'))
    }
}

/// Page 1 — top-level transaction metadata. Reads straight off the
/// `ParsedTransaction`: program list, fee payer, fee, sig count, version,
/// and tx size. Designed so a power user can verify the tx's shape at a
/// glance before drilling into instructions.
fn draw_tx_metadata<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    parsed: &crate::parser::ParsedTransaction,
    total_pages: usize,
) -> Result<(), D::Error> {
    let version = match &parsed.version {
        crate::parser::TransactionVersion::Legacy => "Legacy".to_string(),
        crate::parser::TransactionVersion::V0 {
            address_table_lookups: 0,
        } => "v0".to_string(),
        crate::parser::TransactionVersion::V0 {
            address_table_lookups: n,
        } => format!("v0 +{}", n),
    };

    let rows = [
        DetailRow {
            label: "FEE PAYER",
            value: short_pubkey(&parsed.fee_payer),
        },
        DetailRow {
            label: "FEE",
            value: lamports_to_sol_str(parsed.fee_lamports),
        },
        DetailRow {
            label: "SIGNERS",
            value: parsed.signers.len().to_string(),
        },
        DetailRow {
            label: "IX COUNT",
            value: parsed.instructions.len().to_string(),
        },
        DetailRow {
            label: "VERSION",
            value: version,
        },
        DetailRow {
            label: "SIZE",
            value: format!("{} B", parsed.size),
        },
    ];

    draw_detail_shell(display, theme, "TX METADATA", 2, total_pages, &rows)
}

/// Page 2 — Instructions overview. One row per top-level instruction
/// showing index + program. Drilling into a specific instruction
/// happens on pages 3..3+N.
fn draw_tx_ix_list<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    parsed: &crate::parser::ParsedTransaction,
    total_pages: usize,
) -> Result<(), D::Error> {
    let rows: Vec<DetailRow<'_>> = parsed
        .instructions
        .iter()
        .enumerate()
        .map(|(i, ix)| DetailRow {
            label: "", // index goes in the value column to keep alignment uniform
            value: format!("{:>2}  {}", i + 1, ix.program),
        })
        .collect();

    draw_detail_shell(display, theme, "INSTRUCTIONS", 3, total_pages, &rows)
}

/// Pages 3..3+N — single-instruction detail. Renders the program name as
/// the title and the instruction's `items` (Header / Field / Warning /
/// Separator) as body rows. Long values truncate to fit the value column;
/// the user can fall back to the raw bytes page if they need the full hex.
fn draw_tx_ix_detail<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    parsed: &crate::parser::ParsedTransaction,
    ix_index: usize,
    total_pages: usize,
) -> Result<(), D::Error> {
    let Some(ix) = parsed.instructions.get(ix_index) else {
        // Defensive — should not happen since the input handler bounds page
        // by `3 + parsed.instructions.len()`. Render a stub instead of
        // panicking.
        return draw_detail_shell(
            display,
            theme,
            "INSTRUCTION",
            ix_index + 4,
            total_pages,
            &[DetailRow {
                label: "ERROR",
                value: "no instruction".to_string(),
            }],
        );
    };

    let mut rows: Vec<DetailRow<'_>> = Vec::new();
    rows.push(DetailRow {
        label: "PROGRAM",
        value: ix.program.clone(),
    });

    for item in &ix.items {
        match item {
            crate::parser::ReviewItem::Header(s) => rows.push(DetailRow {
                label: "",
                value: format!("[{}]", s),
            }),
            crate::parser::ReviewItem::Field { label, value } => {
                // Internal machine-readable fields the zoned renderer pulls.
                if label == "Slippage_bps" || label == "Route_hops" {
                    continue;
                }
                rows.push(DetailRow {
                    label: "",
                    value: if label.is_empty() {
                        value.clone()
                    } else {
                        format!("{}: {}", label, value)
                    },
                });
            }
            crate::parser::ReviewItem::Warning(s) => rows.push(DetailRow {
                label: "!",
                value: s.clone(),
            }),
        }
    }

    // Title is `IX <n>/<total>` so the user knows which instruction they're
    // looking at without doing the page-counter math.
    let title = format!("IX {}/{}", ix_index + 1, parsed.instructions.len());
    // SAFETY: title leaks one allocation per render. Acceptable for now —
    // the firmware redraws on event, not 60Hz. If it becomes hot, swap to a
    // fixed-size stack buffer.
    let title_static: &'static str = Box::leak(title.into_boxed_str());
    draw_detail_shell(display, theme, title_static, ix_index + 4, total_pages, &rows)
}

/// Last page — raw bytes preview. Shows length, signing hash, and
/// the first/last 32 bytes as hex. Intentionally not a full hex dump:
/// the point is "I can verify nothing's hidden in here" — a length +
/// hash + boundary bytes covers that without paginating across many
/// screens. The user already scanned the QR for the full bytes.
fn draw_tx_raw<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    tx_bytes: &[u8],
    total_pages: usize,
) -> Result<(), D::Error> {
    let len = tx_bytes.len();
    // 6 bytes per HEAD/TAIL line: each byte renders as "ff " (3 chars), so
    // 6 bytes = 17 chars (no trailing space) — fits comfortably inside the
    // value column at the chosen label width. Anything larger gets cut by
    // the truncation pass anyway; we'd rather show clean bytes than
    // mid-byte ellipsis.
    let preview_bytes: usize = 6;
    let format_bytes = |slice: &[u8]| -> String {
        slice
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let head = format_bytes(&tx_bytes[..preview_bytes.min(len)]);
    let tail = if len > preview_bytes * 2 {
        format_bytes(&tx_bytes[len - preview_bytes..])
    } else {
        String::new()
    };

    let mut rows = vec![
        DetailRow {
            label: "SIZE",
            value: format!("{} B", len),
        },
        DetailRow {
            label: "HEAD",
            value: head,
        },
    ];
    if !tail.is_empty() {
        rows.push(DetailRow {
            label: "TAIL",
            value: tail,
        });
    }

    draw_detail_shell(display, theme, "RAW", total_pages, total_pages, &rows)
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
    theme: &Theme,
    message_bytes: &[u8],
    scroll: usize,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, GUTTER_W};

    use embedded_graphics::geometry::Size;

    display.clear(colors::BG_DARK)?;
    draw_status_bar(display, theme, "Sign Message", seed_loaded)?;

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

/// Show the wallet's public address as wrapped text — for users who need to
/// read or copy it by hand, character by character. Splits the 32–44 char
/// base58 string into fixed-width chunks so the eye can track each row, but
/// the chunking never breaks a "word" (the address itself is one token, so
/// chunk boundaries are pure visual aids — no spaces are inserted that would
/// alter the address). Rendered with the same big body font used by help
/// screens.
fn draw_show_address_text<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    address: Option<&str>,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};



    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);
    Header {
        kind: HeaderKind::Title("ADDRESS"),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height - FOOTER_H),
    );
    let center_x = body_inner.top_left.x + body_inner.size.width as i32 / 2;

    match address {
        Some(addr) => {
            // Chunk size picked so the longest possible Solana address (44
            // chars) lays out as four equal rows of 11 — even visual rhythm,
            // and the row width fits comfortably with the big font.
            let chunk_size = 11usize;
            let chunks: Vec<&str> = addr
                .as_bytes()
                .chunks(chunk_size)
                .map(|b| core::str::from_utf8(b).unwrap_or(""))
                .collect();

            let line_h = 26i32;
            let block_h = chunks.len() as i32 * line_h;
            let body_h = body_inner.size.height as i32;
            let top_padding = ((body_h - block_h) / 2).max(theme.space_md);
            let mut cursor_y = body_inner.top_left.y + top_padding + 18;

            for chunk in &chunks {
                Text::with_alignment(
                    chunk,
                    Point::new(center_x, cursor_y),
                    theme.style_md(theme.text),
                    Alignment::Center,
                )
                .draw(display)?;
                cursor_y += line_h;
            }
        }
        None => {
            return draw_no_wallet_alert(display, theme, "ADDRESS", "Create or load a", "wallet to view.");
        }
    }

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

/// Show the wallet's public address as a QR. Users verify the QR in a hot
/// wallet; the truncated caption is for a quick visual double-check.
fn draw_show_address<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    address: Option<&str>,
) -> Result<(), D::Error> {




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
            draw_no_wallet_alert(display, theme, "ADDRESS", "Create or load a", "wallet to view.")
        }
    }
}

/// About screen. Hero block uses the full pixel-art Faraday logo (mark +
/// wordmark) instead of plain text, then a muted subtitle, then key/value
/// reference rows.
fn draw_about<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::gui::logo;
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};



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
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height - FOOTER_H),
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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{
        EdgeHints, EdgeIcon, Header, HeaderKind, List, ListRow, FOOTER_H, GUTTER_W,
    };



    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title("BACKUP SEED"),
        counter: None,
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Reserve chrome (gutter or bottom bar); rows + text live in what remains.
    // `tap_layout` for this screen mirrors the same third split.
    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height - FOOTER_H),
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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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
    theme: &Theme,
    selected: usize,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{EdgeHints, EdgeIcon, HeaderKind, ListRow};
    use crate::ui::screens::ListScreen;


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
    theme: &Theme,
    picker: &crate::gui::app::WordPicker,
) -> Result<(), D::Error> {
    use crate::gui::app::{WordPicker, WORD_GRID_COLS, WORD_GRID_ROWS};
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};



    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, body_rect) = split_top(screen, theme.header_h as i32);

    Header {
        kind: HeaderKind::Title("WORD"),
        counter: Some((picker.word_index + 1, picker.word_count)),
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Reserve chrome (gutter or bottom bar); the grid fills what remains.
    // `tap_word_grid` mirrors this height so taps map to the drawn cells.
    let body_inner = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height - FOOTER_H),
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
            // Touch builds have no cursor: letters are selected by tapping
            // them directly, so nothing is highlighted.
            #[cfg(not(feature = "touch-ui"))]
            let is_selected = r == picker.cursor_row && c == picker.cursor_col;
            #[cfg(feature = "touch-ui")]
            let is_selected = false;

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
    // Touch builds drop the Check (confirm-letter) cell: letters are committed
    // by tapping them, so the footer carries only Delete and Back.
    #[cfg(feature = "touch-ui")]
    EdgeHints::new()
        .k2(EdgeIcon::Delete)
        .k3(EdgeIcon::ArrowLeft)
        .draw(display, &theme, gutter)?;
    #[cfg(not(feature = "touch-ui"))]
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
    theme: &Theme,
    word: &str,
    committed: usize,
    word_count: usize,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{Header, HeaderKind};



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
    theme: &Theme,
    header_title: &str,
    body_l1: &str,
    body_l2: &str,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};



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

/// Paper-seed didn't decode to the currently-loaded wallet's mnemonic.
fn draw_verify_backup_seed_mismatch<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::screens::CardScreen;

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
    theme: &Theme,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::screens::CardScreen;

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
    theme: &Theme,
    has_passphrase: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::screens::CardScreen;

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
    theme: &Theme,
    data: &[u8],
    ec: crate::qr::encode_qr::QrEcLevel,
    quiet: u32,
) -> Result<(), D::Error> {
    use crate::ui::widgets::Qr;

    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };


    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;
    Qr { data, ec, quiet }.draw(display, &theme, screen)
}

/// Final SeedQR review (side-by-side compare against the hand-transcribed
/// paper). On key builds this is just the full-screen QR. On touch builds it
/// adds the bottom action bar — Back returns to the transcribe walkthrough,
/// Accept advances to the verify-by-scan step — since there are no physical
/// keys to leave the screen otherwise.
fn draw_export_full_qr<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    data: &[u8],
) -> Result<(), D::Error> {
    let ec = crate::qr::encode_qr::QrEcLevel::L;
    let quiet = 2u32;

    #[cfg(not(feature = "touch-ui"))]
    {
        draw_fullscreen_qr(display, theme, data, ec, quiet)
    }

    #[cfg(feature = "touch-ui")]
    {
        use crate::ui::layout::split_bottom;
        use crate::ui::widgets::{EdgeHints, EdgeIcon, Qr, FOOTER_H};

        let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
        display.fill_solid(&screen, theme.bg)?;

        // QR fills the body above the action bar; the bar carries Back / Accept.
        let (body, _footer) = split_bottom(screen, FOOTER_H as i32);
        Qr { data, ec, quiet }.draw(display, &theme, body)?;
        EdgeHints::new()
            .k1(EdgeIcon::Check)
            .k3(EdgeIcon::ArrowLeft)
            .draw(display, &theme, screen)?;
        Ok(())
    }
}

/// Passphrase mismatch error card. Any key retries the input.
fn draw_passphrase_mismatch<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    use crate::ui::widgets::{CardRow, EdgeHints, EdgeIcon, HeaderKind};
    use crate::ui::screens::CardScreen;


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
    theme: &Theme,
    _word_count: usize,
    frames_collected: usize,
    _seed_loaded: bool,
    has_frame: bool,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};

    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };


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
    theme: &Theme,
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
        theme,
        "COIN FLIPS",
        bits.len(),
        target,
        &recent,
        &["HEADS", "TAILS"],
        selected,
        PickerLayout::Grid { cols: 1, rows: 2 },
    )
}

/// Dice-roll entropy collector. 1–6 picker, selected gets the cyan highlight.
fn draw_dice_rolls<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    word_count: usize,
    rolls: &[u8],
    selected: usize,
    _seed_loaded: bool,
) -> Result<(), D::Error> {
    let target = if word_count == 12 { 50 } else { 99 };
    let start = rolls.len().saturating_sub(20);
    let recent: String = rolls[start..]
        .iter()
        .map(|r| alloc::format!("{}", r))
        .collect();
    draw_entropy_picker(
        display,
        theme,
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
    /// `cols × rows` grid. `cols * rows` must be >= choices.len().
    Grid { cols: usize, rows: usize },
}

/// Shared entropy-collection layout: header + progress bar + recent-history
/// strip + N-way value picker + button bar. Drives both the coin-flip and
/// dice-roll screens.
fn draw_entropy_picker<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    title: &str,
    progress: usize,
    target: usize,
    recent: &str,
    choices: &[&str],
    selected: usize,
    layout: PickerLayout,
) -> Result<(), D::Error> {
    use crate::ui::layout::{split_bottom, split_top};
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, FOOTER_H, GUTTER_W};

    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };


    let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
    display.fill_solid(&screen, theme.bg)?;

    let (header_rect, rest) = split_top(screen, theme.header_h as i32);
    // Reserve the bottom strip: breathing margin on key builds, action bar on
    // touch builds — whichever is larger.
    let (body_rect, _footer_rect) = split_bottom(rest, theme.footer_h.max(FOOTER_H) as i32);
    let body_rect = Rectangle::new(
        body_rect.top_left,
        Size::new(body_rect.size.width - GUTTER_W, body_rect.size.height),
    );

    Header {
        kind: HeaderKind::Title(title),
        counter: Some((progress, target)),
        right_label: None,
    }
    .draw(display, &theme, header_rect)?;

    // Progress bar band, then recent-history strip, then picker fills the rest.
    let (progress_rect, rest) = split_top(body_rect, 16);
    draw_progress_bar(display, theme, progress_rect, progress, target)?;

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

extern crate alloc;

/// Camera-backed scan overlay. Ported unchanged from the image-shrink branch —
/// diagnostic panel when the camera fails, "Opening camera..." placeholder
/// while warming up, and a centered reticle + hint bar once a frame is live.
fn draw_scan_overlay<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    title: &str,
    _hint: &str,
    _seed_loaded: bool,
    has_frame: bool,
    error: Option<&str>,
    diag: crate::camera::ScanDiagnostics,
) -> Result<(), D::Error> {
    use crate::ui::layout::split_top;
    use crate::ui::widgets::{EdgeHints, EdgeIcon, Header, HeaderKind, GUTTER_W};



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

        // Back affordance — the same K3/ArrowLeft hint the live scan branch
        // draws, so the user can leave a camera-error screen. On touch builds
        // this renders as the bottom action bar (left third = Back, mapped to
        // InputEvent::Back by the footer tap-zone); on key builds it's the
        // right-gutter K3 hint. Without it the error card had no way out.
        let gutter = Rectangle::new(
            Point::new(theme.width as i32 - GUTTER_W as i32, theme.header_h as i32),
            Size::new(GUTTER_W, theme.height - theme.header_h),
        );
        EdgeHints::new()
            .k3(EdgeIcon::ArrowLeft)
            .draw(display, &theme, gutter)?;
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

    // Scan-pipeline heartbeat (pulsing dot + UR seq/total when assembling).
    draw_scan_diag(display, theme, diag)?;

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

    // Reticle outlines the entire camera frame edge-to-edge so the whole feed
    // reads as the scan target — users shouldn't think they must fit the QR
    // inside a small box. The camera is a `width`×`width` square blitted at the
    // top of the display, so it ends at row `theme.width` (below that, on the
    // taller ESP32 panel, is GUI chrome, not live feed). Drawn last so the right
    // border sits on top of the gutter rather than being clipped behind it.
    let ret_y = rest.top_left.y;
    let cam_bottom = theme.width as i32;
    Rectangle::new(
        Point::new(rest.top_left.x, ret_y),
        Size::new(rest.size.width, (cam_bottom - ret_y) as u32),
    )
    .into_styled(PrimitiveStyle::with_stroke(theme.accent, 2))
    .draw(display)?;

    Ok(())
}

fn draw_scan_diag<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
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
    let strip = Rectangle::new(Point::new(0, 28), Size::new(theme.width, 14));
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
