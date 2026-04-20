//! Application state machine — drives all UI flows.

use crate::crypto::derivation;
use crate::crypto::slip0010::SolanaKeypair;
use crate::gui::flows;
use zeroize::Zeroize;

// Camera backend selection. Exactly one of these is compiled per build:
// macOS simulator uses nokhwa; Pi Linux uses V4L2. On targets with neither
// (e.g. headless cross-platform builds) the camera fields on `App` are absent.
#[cfg(feature = "simulator")]
type Camera = crate::gui::sim_camera::SimCamera;
#[cfg(feature = "simulator_no_cam")]
type Camera = crate::gui::file_camera::FileCamera;
#[cfg(all(target_os = "linux", not(feature = "_desktop_sim")))]
type Camera = crate::hardware::pi_camera::PiCamera;

/// Platform-independent input event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Secondary,
}

/// Current screen with its mutable state.
pub enum Screen {
    Splash,
    MainMenu { selected: usize },

    // Create wallet flow
    CreateWordCount { selected: usize },
    CreateMethod { word_count: usize, selected: usize },
    CreateCameraEntropy { word_count: usize, frames_collected: usize, entropy: Vec<u8> },
    CreateCoinFlips { word_count: usize, bits: Vec<bool>, selected: usize },
    CreateDiceRolls { word_count: usize, rolls: Vec<u8>, selected: usize },
    CreateShowWords {
        mnemonic: String,
        page: usize,
        word_count: usize,
    },
    CreateVerify {
        mnemonic: String,
        checks: Vec<usize>,
        current: usize,
        options: Vec<String>,
        correct_idx: usize,
        selected: usize,
    },
    CreatePassphrasePrompt { mnemonic: String, selected: usize },
    CreatePassphraseInput { mnemonic: String, grid: CharGrid },
    CreatePassphraseConfirm { mnemonic: String, passphrase: String, grid: CharGrid },
    CreatePassphraseMismatch { mnemonic: String },
    CreateConfirm {
        mnemonic: String,
        passphrase: String,
        address: String,
        selected: usize,
    },
    /// Landing page for all seed-backup actions.
    ExportSeedQrMenu {
        compact_data: Vec<u8>,
        selected: usize,
        from_settings: bool,
    },
    /// Paged view of the 12/24-word mnemonic for write-it-down backup.
    ExportShowWords {
        compact_data: Vec<u8>,
        mnemonic: String,
        page: usize,
        word_count: usize,
        from_settings: bool,
    },
    /// Full QR display — shown as the final check after the paper-backup
    /// block walkthrough. Not reachable directly from the menu.
    ExportSeedQr {
        compact_data: Vec<u8>,
        from_settings: bool,
    },
    /// Zoomed block-by-block view for hand transcription onto the paper
    /// template. `block_index` is row-major over the 3×3 (21×21) or 5×5
    /// (25×25) block grid.
    ExportSeedQrBlock {
        compact_data: Vec<u8>,
        block_index: usize,
        from_settings: bool,
    },

    // Verify backup flow: scan the paper QR, confirm mnemonic matches the
    // loaded wallet; if that wallet has a passphrase, also prompt for it
    // and check the derived address matches.
    VerifyBackupScan,
    VerifyBackupSeedMismatch,
    VerifyBackupPassphrase { grid: CharGrid },
    VerifyBackupPassphraseMismatch,
    VerifyBackupSuccess,

    // Load wallet flow
    LoadMethod { selected: usize },
    LoadScanQr,
    LoadWordCount { selected: usize },
    LoadEnterWords {
        words: Vec<String>,
        word_count: usize,
        picker: WordPicker,
    },
    LoadPassphrasePrompt { mnemonic: String, selected: usize },
    LoadPassphraseInput { mnemonic: String, grid: CharGrid },
    LoadPassphraseConfirm { mnemonic: String, passphrase: String, grid: CharGrid },
    LoadPassphraseMismatch { mnemonic: String },
    LoadConfirm {
        mnemonic: String,
        passphrase: String,
        address: String,
        selected: usize,
    },

    // Sign TX flow
    SignNoWallet,
    SignScanTx,
    SignReview {
        tx_bytes: Vec<u8>,
        tx_base64: String,
        info_lines: Vec<String>,
        scroll: usize,
        selected: usize,
        /// False when the loaded wallet's pubkey is not in the tx's required
        /// signer set — Sign is disabled in that case.
        can_sign: bool,
    },
    SignShowQr { data: String },
    SignMessageInput { grid: CharGrid },
    SignMessageResult { signature_hex: String },

    // Settings
    SettingsMenu { selected: usize },
    SettingsShowAddress,
    SettingsAccounts { accounts: Vec<(String, String)>, selected: usize },
    SettingsVerifyAddressScan,
    SettingsVerifyAddressResult {
        address: String,
        result: crate::crypto::derivation::AddressMatch,
    },
    SettingsAbout,
    SettingsPowerOff { selected: usize },
}

/// Character grid for passphrase entry.
pub struct CharGrid {
    pub text: String,
    pub row: usize,
    pub col: usize,
    pub caps: bool,
}

const GRID_CHARS: [[char; 10]; 5] = [
    ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
    ['k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't'],
    ['u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3'],
    ['4', '5', '6', '7', '8', '9', '!', '@', '#', '$'],
    ['%', '^', '&', '*', '.', '-', '_', '+', '=', '/'],
];
pub const GRID_ROWS: usize = 6;
pub const GRID_COLS: usize = 10;

/// What action row cell maps to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridAction {
    Space,
    Caps,
    Delete,
    Done,
}

impl CharGrid {
    pub fn new() -> Self {
        CharGrid { text: String::new(), row: 0, col: 0, caps: false }
    }

    pub fn current_cell(&self) -> Result<char, GridAction> {
        if self.row < 5 {
            let ch = GRID_CHARS[self.row][self.col];
            if self.caps && ch.is_ascii_lowercase() {
                Ok(ch.to_ascii_uppercase())
            } else {
                Ok(ch)
            }
        } else {
            match self.col {
                0..=1 => Err(GridAction::Space),
                2..=3 => Err(GridAction::Caps),
                4..=6 => Err(GridAction::Delete),
                _ => Err(GridAction::Done),
            }
        }
    }

    pub fn action_region(&self) -> Option<GridAction> {
        if self.row == 5 {
            match self.col {
                0..=1 => Some(GridAction::Space),
                2..=3 => Some(GridAction::Caps),
                4..=6 => Some(GridAction::Delete),
                _ => Some(GridAction::Done),
            }
        } else {
            None
        }
    }

    pub fn handle_input(&mut self, event: InputEvent) -> bool {
        match event {
            InputEvent::Up => {
                if self.row > 0 {
                    self.row -= 1;
                    if self.row < 5 && self.col >= GRID_COLS {
                        self.col = GRID_COLS - 1;
                    }
                }
            }
            InputEvent::Down => {
                if self.row < GRID_ROWS - 1 {
                    self.row += 1;
                    if self.row == 5 {
                        self.col = match self.col {
                            0..=1 => 0,
                            2..=4 => 2,
                            5..=6 => 4,
                            _ => 7,
                        };
                    }
                }
            }
            InputEvent::Left => {
                if self.row == 5 {
                    match self.col {
                        0..=1 => {}
                        2..=3 => self.col = 0,
                        4..=6 => self.col = 2,
                        _ => self.col = 4,
                    }
                } else if self.col > 0 {
                    self.col -= 1;
                }
            }
            InputEvent::Right => {
                if self.row == 5 {
                    match self.col {
                        0..=1 => self.col = 2,
                        2..=3 => self.col = 4,
                        4..=6 => self.col = 7,
                        _ => {}
                    }
                } else if self.col < GRID_COLS - 1 {
                    self.col += 1;
                }
            }
            InputEvent::Confirm => {
                match self.current_cell() {
                    Ok(ch) => {
                        if self.text.len() < 64 { self.text.push(ch); }
                    }
                    Err(GridAction::Space) => {
                        if self.text.len() < 64 { self.text.push(' '); }
                    }
                    Err(GridAction::Caps) => {
                        self.caps = !self.caps;
                    }
                    Err(GridAction::Delete) => {
                        self.text.pop();
                    }
                    Err(GridAction::Done) => {
                        return true;
                    }
                }
            }
            InputEvent::Back => {
                if self.text.is_empty() {
                    return true;
                }
                self.text.pop();
            }
            InputEvent::Secondary => {
                self.caps = !self.caps;
            }
        }
        false
    }
}

/// BIP39 word picker with prefix filtering.
pub struct WordPicker {
    pub prefix: String,
    pub char_cursor: u8,
    pub list_selected: usize,
    pub word_index: usize,
    pub word_count: usize,
    pub words: Vec<String>,
}

impl WordPicker {
    pub fn new(word_count: usize) -> Self {
        WordPicker {
            prefix: String::new(),
            char_cursor: 0,
            list_selected: 0,
            word_index: 0,
            word_count,
            words: Vec::new(),
        }
    }

    pub fn current_char(&self) -> char {
        (b'a' + self.char_cursor) as char
    }

    pub fn filtered_words(&self) -> Vec<(usize, &'static str)> {
        let preview = format!("{}{}", self.prefix, self.current_char());
        crate::crypto::bip39::words_with_prefix(&preview)
    }

    pub fn handle_input(&mut self, event: InputEvent) -> Option<String> {
        match event {
            InputEvent::Left => {
                if self.char_cursor > 0 {
                    self.char_cursor -= 1;
                    self.list_selected = 0;
                }
            }
            InputEvent::Right => {
                if self.char_cursor < 25 {
                    self.char_cursor += 1;
                    self.list_selected = 0;
                }
            }
            InputEvent::Up => {
                if self.list_selected > 0 { self.list_selected -= 1; }
            }
            InputEvent::Down => {
                let filtered = self.filtered_words();
                if self.list_selected + 1 < filtered.len() { self.list_selected += 1; }
            }
            InputEvent::Confirm => {
                let filtered = self.filtered_words();
                if !filtered.is_empty() && self.list_selected < filtered.len() {
                    let word = filtered[self.list_selected].1.to_string();
                    self.words.push(word.clone());
                    self.word_index += 1;
                    self.prefix.clear();
                    self.char_cursor = 0;
                    self.list_selected = 0;
                    return Some(word);
                }
            }
            InputEvent::Secondary => {
                self.prefix.push(self.current_char());
                self.char_cursor = 0;
                self.list_selected = 0;
            }
            InputEvent::Back => {
                if !self.prefix.is_empty() {
                    self.prefix.pop();
                    self.char_cursor = 0;
                    self.list_selected = 0;
                }
            }
        }
        None
    }
}

/// Wallet loaded in memory.
pub struct LoadedWallet {
    pub mnemonic: String,
    pub passphrase: String,
    pub keypair: SolanaKeypair,
    pub address: String,
}

impl Drop for LoadedWallet {
    fn drop(&mut self) {
        self.mnemonic.zeroize();
        self.passphrase.zeroize();
    }
}

/// Default screen-blanking timeout. Zero disables blanking entirely.
pub const DEFAULT_BLANK_TIMEOUT_MS: u64 = 120_000; // 2 minutes

/// Pure decision function: should the screen render as blank given these inputs?
///
/// Kept as a free function so it can be exhaustively unit-tested without
/// constructing an `App` or dealing with wall-clock time.
pub fn should_blank(idle_ms: u64, timeout_ms: u64, on_camera_screen: bool) -> bool {
    // Camera screens never blank — the user is actively looking at a live preview.
    // Timeout of 0 is the "never blank" sentinel.
    !on_camera_screen && timeout_ms > 0 && idle_ms >= timeout_ms
}

/// Top-level application.
pub struct App {
    pub screen: Screen,
    pub wallet: Option<LoadedWallet>,
    pub last_activity: std::time::Instant,
    pub blank_timeout_ms: u64,
    #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
    pub camera: Option<Camera>,
    #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
    pub latest_frame: Option<crate::camera::Frame>,
    #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
    pub camera_error: Option<String>,
    #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
    pub scanned_qr: Option<Vec<u8>>,
}

impl App {
    pub fn new() -> Self {
        App {
            screen: Screen::Splash,
            wallet: None,
            last_activity: std::time::Instant::now(),
            blank_timeout_ms: DEFAULT_BLANK_TIMEOUT_MS,
            #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
            camera: None,
            #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
            latest_frame: None,
            #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
            camera_error: None,
            #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
            scanned_qr: None,
        }
    }

    pub fn seed_loaded(&self) -> bool {
        self.wallet.is_some()
    }

    /// Title used for every screen in the SeedQR backup flow. Appends "+P"
    /// when a passphrase is set, so the user is continuously reminded the
    /// QR alone isn't enough to restore the wallet.
    pub fn seedqr_title(&self) -> &'static str {
        let has_passphrase = self.wallet.as_ref().map_or(false, |w| !w.passphrase.is_empty());
        if has_passphrase { "SeedQR +P" } else { "SeedQR" }
    }

    pub fn enter_main_menu(&mut self) {
        self.screen = Screen::MainMenu { selected: 0 };
    }

    pub fn handle_input(&mut self, event: InputEvent) {
        let was_blanked = self.is_blanked();
        self.last_activity = std::time::Instant::now();
        if was_blanked {
            // Any input wakes the screen — but we consume the input so the user
            // doesn't accidentally confirm a dialog they couldn't see.
            return;
        }
        let screen = std::mem::replace(&mut self.screen, Screen::Splash);
        self.screen = self.transition(screen, event);
    }

    /// True when the screen should render as blank right now.
    pub fn is_blanked(&self) -> bool {
        let idle_ms = self.last_activity.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let on_camera = {
            #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
            { self.wants_camera() }
            #[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
            { false }
        };
        should_blank(idle_ms, self.blank_timeout_ms, on_camera)
    }

    /// Camera error message, if any. None when camera isn't supported on this build.
    pub fn camera_error_str(&self) -> Option<&str> {
        #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
        {
            self.camera_error.as_deref()
        }
        #[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
        {
            None
        }
    }

    /// True if a webcam frame is currently available.
    pub fn has_camera_frame(&self) -> bool {
        #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
        {
            self.latest_frame.is_some()
        }
        #[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
        {
            false
        }
    }

    /// True when the current screen wants the webcam.
    #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
    pub fn wants_camera(&self) -> bool {
        matches!(
            &self.screen,
            Screen::LoadScanQr
                | Screen::SignScanTx
                | Screen::CreateCameraEntropy { .. }
                | Screen::SettingsVerifyAddressScan
                | Screen::VerifyBackupScan
        )
    }

    /// Per-frame update — manages camera lifecycle and auto-advances on QR detect.
    /// Shared between simulator (macOS nokhwa) and Pi (V4L2) via the `Camera` type alias.
    #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
    pub fn tick(&mut self) {
        let wants = self.wants_camera();
        if wants && self.camera.is_none() && self.camera_error.is_none() {
            match Camera::open() {
                Ok(cam) => self.camera = Some(cam),
                Err(e) => {
                    eprintln!("Camera unavailable: {e}");
                    self.camera_error = Some(e);
                }
            }
        } else if !wants && self.camera.is_some() {
            self.camera = None;
            self.latest_frame = None;
            self.scanned_qr = None;
        }
        if !wants {
            self.camera_error = None;
        }

        let is_scan_screen = matches!(
            self.screen,
            Screen::LoadScanQr
                | Screen::SignScanTx
                | Screen::SettingsVerifyAddressScan
                | Screen::VerifyBackupScan
        );
        let pending_qr = if let Some(cam) = &self.camera {
            cam.set_decode_enabled(is_scan_screen);
            if let Some(f) = cam.latest() {
                self.latest_frame = Some(f);
            }
            // Watchdog from the Pi camera thread — if the stream opened but
            // never produced a frame (or errored mid-capture), surface it as
            // a UI error so the user isn't stuck on "Opening camera...".
            if let Some(err) = cam.take_fatal_err() {
                eprintln!("Camera fatal: {err}");
                self.camera_error = Some(err);
                self.camera = None;
                self.latest_frame = None;
                None
            } else {
                cam.take_qr()
            }
        } else {
            None
        };

        if let Some(data) = pending_qr {
            if matches!(
                self.screen,
                Screen::LoadScanQr
                    | Screen::SignScanTx
                    | Screen::SettingsVerifyAddressScan
                    | Screen::VerifyBackupScan
            ) {
                self.scanned_qr = Some(data);
                self.handle_input(InputEvent::Confirm);
            }
        }
    }

    fn transition(&mut self, screen: Screen, event: InputEvent) -> Screen {
        match screen {
            Screen::Splash => Screen::MainMenu { selected: 0 },

            Screen::MainMenu { mut selected } => {
                // Vertical list: Up/Down move one item at a time; Left/Right
                // behave like Up/Down for joystick ergonomics.
                match event {
                    InputEvent::Up | InputEvent::Left => {
                        if selected > 0 { selected -= 1; }
                    }
                    InputEvent::Down | InputEvent::Right => {
                        if selected < 3 { selected += 1; }
                    }
                    InputEvent::Confirm => return self.menu_select(selected),
                    _ => {}
                }
                Screen::MainMenu { selected }
            }

            s @ (Screen::CreateWordCount { .. }
                | Screen::CreateMethod { .. }
                | Screen::CreateCameraEntropy { .. }
                | Screen::CreateCoinFlips { .. }
                | Screen::CreateDiceRolls { .. }
                | Screen::CreateShowWords { .. }
                | Screen::CreateVerify { .. }
                | Screen::CreatePassphrasePrompt { .. }
                | Screen::CreatePassphraseInput { .. }
                | Screen::CreatePassphraseConfirm { .. }
                | Screen::CreatePassphraseMismatch { .. }
                | Screen::CreateConfirm { .. }
                | Screen::ExportSeedQrMenu { .. }
                | Screen::ExportShowWords { .. }
                | Screen::ExportSeedQr { .. }
                | Screen::ExportSeedQrBlock { .. }) => flows::create::handle(self, s, event),

            s @ (Screen::LoadMethod { .. }
                | Screen::LoadScanQr
                | Screen::LoadWordCount { .. }
                | Screen::LoadEnterWords { .. }
                | Screen::LoadPassphrasePrompt { .. }
                | Screen::LoadPassphraseInput { .. }
                | Screen::LoadPassphraseConfirm { .. }
                | Screen::LoadPassphraseMismatch { .. }
                | Screen::LoadConfirm { .. }) => flows::load::handle(self, s, event),

            s @ (Screen::SignNoWallet
                | Screen::SignScanTx
                | Screen::SignReview { .. }
                | Screen::SignShowQr { .. }
                | Screen::SignMessageInput { .. }
                | Screen::SignMessageResult { .. }) => flows::sign::handle(self, s, event),

            s @ (Screen::SettingsMenu { .. }
                | Screen::SettingsShowAddress
                | Screen::SettingsAccounts { .. }
                | Screen::SettingsVerifyAddressScan
                | Screen::SettingsVerifyAddressResult { .. }
                | Screen::SettingsAbout
                | Screen::SettingsPowerOff { .. }) => flows::settings::handle(self, s, event),

            s @ (Screen::VerifyBackupScan
                | Screen::VerifyBackupSeedMismatch
                | Screen::VerifyBackupPassphrase { .. }
                | Screen::VerifyBackupPassphraseMismatch
                | Screen::VerifyBackupSuccess) => flows::verify::handle(self, s, event),
        }
    }

    fn menu_select(&mut self, selected: usize) -> Screen {
        match selected {
            0 => Screen::CreateWordCount { selected: 0 },
            1 => Screen::LoadMethod { selected: 0 },
            2 => {
                if self.wallet.is_some() { Screen::SignScanTx } else { Screen::SignNoWallet }
            }
            3 => Screen::SettingsMenu { selected: 0 },
            _ => Screen::MainMenu { selected },
        }
    }

    pub(crate) fn derive_address(&self, mnemonic: &str, passphrase: &str) -> String {
        let keypair = derivation::derive_keypair(mnemonic, passphrase, 0);
        derivation::address(&keypair)
    }

    pub(crate) fn load_wallet(&mut self, mnemonic: String, passphrase: String) {
        let keypair = derivation::derive_keypair(&mnemonic, &passphrase, 0);
        let address = crate::qr::encode_qr::encode_address(&keypair.public_key);
        self.wallet = Some(LoadedWallet { mnemonic, passphrase, keypair, address });
    }
}

/// Build the Review TX `info_lines` from a parsed tx. Returns the lines plus
/// `can_sign` — true only when the loaded wallet's pubkey is in the tx's
/// required-signer set. Lines starting with `!` render in danger red.
///
/// Delegates to the unified parser (`crate::parser`) which supports both
/// legacy and v0 transactions.
pub fn build_review_lines(tx_bytes: &[u8], wallet_pubkey: &[u8; 32]) -> (Vec<String>, bool) {
    crate::parser::build_review_lines(tx_bytes, wallet_pubkey)
}

#[cfg(test)]
mod blanking_tests {
    use super::should_blank;

    #[test]
    fn below_timeout_does_not_blank() {
        assert!(!should_blank(0, 1000, false));
        assert!(!should_blank(500, 1000, false));
        assert!(!should_blank(999, 1000, false));
    }

    #[test]
    fn at_or_above_timeout_blanks() {
        assert!(should_blank(1000, 1000, false));
        assert!(should_blank(1500, 1000, false));
        assert!(should_blank(u64::MAX, 1000, false));
    }

    #[test]
    fn camera_screen_is_never_blanked() {
        // Even far past the timeout, blanking is suppressed during camera use.
        assert!(!should_blank(0, 1000, true));
        assert!(!should_blank(1_000_000, 1000, true));
        assert!(!should_blank(u64::MAX, 1, true));
    }

    #[test]
    fn zero_timeout_disables_blanking() {
        // Explicit "never blank" sentinel.
        assert!(!should_blank(0, 0, false));
        assert!(!should_blank(u64::MAX, 0, false));
        assert!(!should_blank(u64::MAX, 0, true));
    }

    #[test]
    fn large_values_do_not_overflow() {
        // Confidence check that we're not doing anything that overflows with
        // extreme inputs.
        assert!(should_blank(u64::MAX, 1, false));
        assert!(!should_blank(0, u64::MAX, false));
    }
}

#[cfg(test)]
mod review_lines_tests {
    use super::*;
    use crate::crypto::{bip39, slip0010};
    use base64::{engine::general_purpose::STANDARD as B64, Engine};

    fn loaded_keypair() -> slip0010::SolanaKeypair {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = bip39::mnemonic_to_seed(mnemonic, "");
        slip0010::derive_solana_keypair(&seed, 0)
    }

    /// Minimal valid SOL-transfer tx to a given signer. Same shape as
    /// App::build_test_transaction.
    fn build_tx_for(signer: &[u8; 32]) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1);
        tx.extend_from_slice(&[0u8; 64]);
        tx.push(1);
        tx.push(0);
        tx.push(1);
        tx.push(2);
        tx.extend_from_slice(signer);
        tx.extend_from_slice(&[0u8; 32]);
        tx.extend_from_slice(&[0u8; 32]);
        tx.push(1);
        tx.push(1);
        tx.push(2);
        tx.push(0);
        tx.push(1);
        tx.push(12);
        tx.extend_from_slice(&[2, 0, 0, 0]);
        tx.extend_from_slice(&1_000_000u64.to_le_bytes());
        tx
    }

    #[test]
    fn matching_wallet_allows_signing() {
        let kp = loaded_keypair();
        let tx = build_tx_for(&kp.public_key);
        let (lines, can_sign) = build_review_lines(&tx, &kp.public_key);
        assert!(can_sign);
        assert!(lines.iter().any(|l| l.contains("SOL Transfer")));
        assert!(!lines.iter().any(|l| l.starts_with('!')));
    }

    #[test]
    fn mismatched_wallet_blocks_signing_and_shows_required_signer() {
        let other_pubkey = [0x11u8; 32];
        let kp = loaded_keypair();
        assert_ne!(kp.public_key, other_pubkey);

        let tx = build_tx_for(&other_pubkey);
        let (lines, can_sign) = build_review_lines(&tx, &kp.public_key);

        assert!(!can_sign, "sign must be blocked on mismatch");
        // Warning banner present.
        assert!(lines.iter().any(|l| l.contains("Cannot sign")));
        // The required signer address (base58 of other_pubkey) must appear wrapped
        // somewhere in the lines so the user knows which wallet to load.
        let expected = bs58::encode(&other_pubkey).into_string();
        let joined: String = lines.join("\n");
        assert!(
            joined.contains(&expected[..22]),
            "expected signer prefix to appear in review lines:\n{joined}"
        );
    }

    #[test]
    fn unparseable_tx_blocks_signing() {
        let kp = loaded_keypair();
        let garbage = vec![0xFFu8; 10];
        let (lines, can_sign) = build_review_lines(&garbage, &kp.public_key);
        assert!(!can_sign);
        assert!(lines.iter().any(|l| l.contains("Failed to parse")));
    }

    #[test]
    fn integrates_with_real_tx_from_user() {
        // The exact base64 tx the user pasted earlier — signer EfZr... which
        // won't match our abandon-abandon wallet.
        let tx_b64 = "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDywkoNI1j+nah055+LRl/5r74IARS0MSvHfPPW5usTeAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACbZKN5QkVXQQH+5BYJje2PQK9UFAivDK+ncn3rilJV8BAgIAAQwCAAAAgJaYAAAAAAA=";
        let tx = B64.decode(tx_b64).unwrap();
        let kp = loaded_keypair();
        let (lines, can_sign) = build_review_lines(&tx, &kp.public_key);
        assert!(!can_sign, "abandon-abandon wallet should not match EfZr signer");
        assert!(lines.iter().any(|l| l.contains("0.01 SOL")));
        assert!(lines.iter().any(|l| l.contains("SOL Transfer")));
    }
}
