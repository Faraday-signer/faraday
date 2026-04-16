//! Application state machine — drives all UI flows.

use crate::crypto::{bip39, derivation};
use crate::crypto::slip0010::SolanaKeypair;
use crate::models::decode_qr;
use zeroize::Zeroize;

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
    CreateMethod { word_count: usize, selected: usize }, // 0=Random, 1=Coin Flips, 2=Dice
    CreateCameraEntropy { word_count: usize, frames_collected: usize, entropy: Vec<u8> },
    CreateCoinFlips { word_count: usize, bits: Vec<bool>, selected: usize }, // 0=Heads, 1=Tails
    CreateDiceRolls { word_count: usize, rolls: Vec<u8>, selected: usize }, // 0-5 = dice 1-6
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
    ExportSeedQr {
        seed_qr_data: String,
        compact_data: Vec<u8>,
        compact_mode: bool,
        from_settings: bool,
    },

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
pub const GRID_ROWS: usize = 6; // 5 char rows + 1 action row
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
        CharGrid {
            text: String::new(),
            row: 0,
            col: 0,
            caps: false,
        }
    }

    /// Get the character or action at the current cursor position.
    pub fn current_cell(&self) -> Result<char, GridAction> {
        if self.row < 5 {
            let ch = GRID_CHARS[self.row][self.col];
            if self.caps && ch.is_ascii_lowercase() {
                Ok(ch.to_ascii_uppercase())
            } else {
                Ok(ch)
            }
        } else {
            // Action row: 0-1=Space, 2-3=Caps, 4-6=Delete, 7-9=Done
            match self.col {
                0..=1 => Err(GridAction::Space),
                2..=3 => Err(GridAction::Caps),
                4..=6 => Err(GridAction::Delete),
                _ => Err(GridAction::Done),
            }
        }
    }

    /// Which action region the cursor is in (for highlighting).
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
                        // Snap to nearest action button start
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
                    // Jump between action buttons
                    match self.col {
                        0..=1 => {} // already at leftmost
                        2..=3 => self.col = 0,    // Caps → Space
                        4..=6 => self.col = 2,    // Delete → Caps
                        _ => self.col = 4,         // Done → Delete
                    }
                } else if self.col > 0 {
                    self.col -= 1;
                }
            }
            InputEvent::Right => {
                if self.row == 5 {
                    // Jump between action buttons
                    match self.col {
                        0..=1 => self.col = 2,    // Space → Caps
                        2..=3 => self.col = 4,    // Caps → Delete
                        4..=6 => self.col = 7,    // Delete → Done
                        _ => {}                    // already at rightmost
                    }
                } else if self.col < GRID_COLS - 1 {
                    self.col += 1;
                }
            }
            InputEvent::Confirm => {
                match self.current_cell() {
                    Ok(ch) => {
                        if self.text.len() < 64 {
                            self.text.push(ch);
                        }
                    }
                    Err(GridAction::Space) => {
                        if self.text.len() < 64 {
                            self.text.push(' ');
                        }
                    }
                    Err(GridAction::Caps) => {
                        self.caps = !self.caps;
                    }
                    Err(GridAction::Delete) => {
                        self.text.pop();
                    }
                    Err(GridAction::Done) => {
                        return true; // signal: done entering passphrase
                    }
                }
            }
            InputEvent::Back => {
                // Delete last char, or if empty, signal cancel
                if self.text.is_empty() {
                    return true; // cancel
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
    pub char_cursor: u8, // 0-25 for a-z
    pub list_selected: usize,
    pub word_index: usize,  // which word we're entering (0-based)
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
        bip39::words_with_prefix(&preview)
    }

    /// Handle input. Returns Some(word) if a word was selected.
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
                if self.list_selected > 0 {
                    self.list_selected -= 1;
                }
            }
            InputEvent::Down => {
                let filtered = self.filtered_words();
                if self.list_selected + 1 < filtered.len() {
                    self.list_selected += 1;
                }
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
                // Append current char to prefix (narrow filter)
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
    #[cfg(feature = "simulator")]
    pub sim_camera: Option<crate::gui::sim_camera::SimCamera>,
    #[cfg(feature = "simulator")]
    pub latest_frame: Option<crate::gui::sim_camera::Frame>,
    #[cfg(feature = "simulator")]
    pub camera_error: Option<String>,
    #[cfg(feature = "simulator")]
    pub scanned_qr: Option<Vec<u8>>,
}

impl App {
    pub fn new() -> Self {
        App {
            screen: Screen::Splash,
            wallet: None,
            last_activity: std::time::Instant::now(),
            blank_timeout_ms: DEFAULT_BLANK_TIMEOUT_MS,
            #[cfg(feature = "simulator")]
            sim_camera: None,
            #[cfg(feature = "simulator")]
            latest_frame: None,
            #[cfg(feature = "simulator")]
            camera_error: None,
            #[cfg(feature = "simulator")]
            scanned_qr: None,
        }
    }

    pub fn seed_loaded(&self) -> bool {
        self.wallet.is_some()
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
            #[cfg(feature = "simulator")]
            { self.wants_camera() }
            #[cfg(not(feature = "simulator"))]
            { false }
        };
        should_blank(idle_ms, self.blank_timeout_ms, on_camera)
    }

    /// Camera error message, if any (sim-only; always None on Pi).
    pub fn camera_error_str(&self) -> Option<&str> {
        #[cfg(feature = "simulator")]
        {
            self.camera_error.as_deref()
        }
        #[cfg(not(feature = "simulator"))]
        {
            None
        }
    }

    /// True if a webcam frame is currently available (sim-only).
    pub fn has_camera_frame(&self) -> bool {
        #[cfg(feature = "simulator")]
        {
            self.latest_frame.is_some()
        }
        #[cfg(not(feature = "simulator"))]
        {
            false
        }
    }

    /// True when the current screen wants the webcam.
    #[cfg(feature = "simulator")]
    pub fn wants_camera(&self) -> bool {
        matches!(
            &self.screen,
            Screen::LoadScanQr
                | Screen::SignScanTx
                | Screen::CreateCameraEntropy { .. }
                | Screen::SettingsVerifyAddressScan
        )
    }

    /// Per-frame update — manages camera lifecycle and auto-advances on QR detect.
    #[cfg(feature = "simulator")]
    pub fn tick(&mut self) {
        let wants = self.wants_camera();
        if wants && self.sim_camera.is_none() && self.camera_error.is_none() {
            match crate::gui::sim_camera::SimCamera::open() {
                Ok(cam) => self.sim_camera = Some(cam),
                Err(e) => {
                    eprintln!("Camera unavailable: {e}");
                    self.camera_error = Some(e);
                }
            }
        } else if !wants && self.sim_camera.is_some() {
            self.sim_camera = None;
            self.latest_frame = None;
            self.scanned_qr = None;
        }
        if !wants {
            self.camera_error = None;
        }

        let is_scan_screen = matches!(
            self.screen,
            Screen::LoadScanQr | Screen::SignScanTx | Screen::SettingsVerifyAddressScan
        );
        let pending_qr = if let Some(cam) = &self.sim_camera {
            cam.set_decode_enabled(is_scan_screen);
            if let Some(f) = cam.latest() {
                self.latest_frame = Some(f);
            }
            cam.take_qr()
        } else {
            None
        };

        if let Some(data) = pending_qr {
            if matches!(
                self.screen,
                Screen::LoadScanQr | Screen::SignScanTx | Screen::SettingsVerifyAddressScan
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
                match event {
                    InputEvent::Up => { if selected >= 2 { selected -= 2; } }
                    InputEvent::Down => { if selected < 2 { selected += 2; } }
                    InputEvent::Left => { if selected % 2 > 0 { selected -= 1; } }
                    InputEvent::Right => { if selected % 2 < 1 { selected += 1; } }
                    InputEvent::Confirm => return self.menu_select(selected),
                    _ => {}
                }
                Screen::MainMenu { selected }
            }

            // === Create Wallet Flow ===
            Screen::CreateWordCount { mut selected } => {
                match event {
                    InputEvent::Up | InputEvent::Down => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        let word_count = if selected == 0 { 12 } else { 24 };
                        return Screen::CreateMethod { word_count, selected: 0 };
                    }
                    InputEvent::Back => return Screen::MainMenu { selected: 0 },
                    _ => {}
                }
                Screen::CreateWordCount { selected }
            }

            Screen::CreateMethod { word_count, mut selected } => {
                match event {
                    InputEvent::Up => { if selected > 0 { selected -= 1; } }
                    InputEvent::Down => { if selected < 3 { selected += 1; } }
                    InputEvent::Confirm => {
                        return match selected {
                            0 => self.generate_wallet(word_count),
                            1 => Screen::CreateCameraEntropy { word_count, frames_collected: 0, entropy: Vec::new() },
                            2 => Screen::CreateCoinFlips { word_count, bits: Vec::new(), selected: 0 },
                            3 => Screen::CreateDiceRolls { word_count, rolls: Vec::new(), selected: 0 },
                            _ => Screen::CreateMethod { word_count, selected },
                        };
                    }
                    InputEvent::Back => return Screen::CreateWordCount { selected: 0 },
                    _ => {}
                }
                Screen::CreateMethod { word_count, selected }
            }

            Screen::CreateCameraEntropy { word_count, mut frames_collected, mut entropy } => {
                // Each button press captures "camera noise" as entropy.
                // In simulator: hashes the current webcam frame. Falls back to
                // timing jitter + OS RNG when the webcam isn't available.
                // On Pi: reads raw bytes from camera sensor (TODO).
                let total_frames = if word_count == 12 { 10 } else { 20 };
                match event {
                    InputEvent::Confirm => {
                        let mut frame_entropy = [0u8; 16];
                        #[cfg(feature = "simulator")]
                        {
                            if let Some(frame) = &self.latest_frame {
                                use sha2::{Digest, Sha256};
                                let digest = Sha256::digest(&frame.rgb);
                                frame_entropy.copy_from_slice(&digest[..16]);
                            } else {
                                getrandom::getrandom(&mut frame_entropy).ok();
                            }
                        }
                        #[cfg(not(feature = "simulator"))]
                        {
                            getrandom::getrandom(&mut frame_entropy).ok();
                        }
                        // Always mix in timing jitter so repeated frames don't repeat entropy.
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        let nanos = now.subsec_nanos().to_le_bytes();
                        for (i, b) in nanos.iter().enumerate() {
                            frame_entropy[i] ^= b;
                        }
                        entropy.extend_from_slice(&frame_entropy);
                        frames_collected += 1;

                        if frames_collected >= total_frames {
                            let mnemonic = bip39::mnemonic_from_entropy(&entropy, word_count)
                                .expect("Valid word count");
                            return Screen::CreateShowWords { mnemonic, page: 0, word_count };
                        }
                    }
                    InputEvent::Back => {
                        return Screen::CreateMethod { word_count, selected: 1 };
                    }
                    _ => {}
                }
                Screen::CreateCameraEntropy { word_count, frames_collected, entropy }
            }

            Screen::CreateCoinFlips { word_count, mut bits, mut selected } => {
                let total_flips = if word_count == 12 { 128 } else { 256 };
                match event {
                    InputEvent::Left | InputEvent::Right => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        bits.push(selected == 0); // 0=Heads=1, 1=Tails=0
                        if bits.len() >= total_flips {
                            // Pack bits into bytes
                            let mut entropy = vec![0u8; total_flips / 8];
                            for (i, &bit) in bits.iter().enumerate() {
                                if bit {
                                    entropy[i / 8] |= 1 << (7 - (i % 8));
                                }
                            }
                            let mnemonic = bip39::mnemonic_from_raw_entropy(&entropy)
                                .expect("Valid entropy length");
                            return Screen::CreateShowWords { mnemonic, page: 0, word_count };
                        }
                    }
                    InputEvent::Back => {
                        if bits.is_empty() {
                            return Screen::CreateMethod { word_count, selected: 1 };
                        }
                        bits.pop(); // Undo last flip
                    }
                    _ => {}
                }
                Screen::CreateCoinFlips { word_count, bits, selected }
            }

            Screen::CreateDiceRolls { word_count, mut rolls, mut selected } => {
                let total_rolls = if word_count == 12 { 50 } else { 99 };
                match event {
                    InputEvent::Up => { if selected >= 3 { selected -= 3; } }
                    InputEvent::Down => { if selected + 3 <= 5 { selected += 3; } }
                    InputEvent::Left => { if selected > 0 { selected -= 1; } }
                    InputEvent::Right => { if selected < 5 { selected += 1; } }
                    InputEvent::Confirm => {
                        rolls.push(selected as u8 + 1); // 1-6
                        if rolls.len() >= total_rolls {
                            // Hash all rolls to get entropy
                            let rolls_str: String = rolls.iter().map(|r| r.to_string()).collect();
                            let mnemonic = bip39::mnemonic_from_entropy(rolls_str.as_bytes(), word_count)
                                .expect("Valid word count");
                            return Screen::CreateShowWords { mnemonic, page: 0, word_count };
                        }
                    }
                    InputEvent::Back => {
                        if rolls.is_empty() {
                            return Screen::CreateMethod { word_count, selected: 2 };
                        }
                        rolls.pop(); // Undo last roll
                    }
                    _ => {}
                }
                Screen::CreateDiceRolls { word_count, rolls, selected }
            }

            Screen::CreateShowWords { mnemonic, mut page, word_count } => {
                let words_per_page = 6usize;
                let total_pages = (word_count + words_per_page - 1) / words_per_page;
                match event {
                    InputEvent::Right | InputEvent::Down => {
                        if page + 1 < total_pages { page += 1; }
                    }
                    InputEvent::Left | InputEvent::Up => {
                        if page > 0 { page -= 1; }
                    }
                    InputEvent::Confirm => {
                        if page + 1 == total_pages {
                            // Last page — proceed to verify
                            return self.start_verification(mnemonic, word_count);
                        } else {
                            page += 1;
                        }
                    }
                    InputEvent::Back => return Screen::CreateWordCount { selected: 0 },
                    _ => {}
                }
                Screen::CreateShowWords { mnemonic, page, word_count }
            }

            Screen::CreateVerify { mnemonic, checks, current, options, correct_idx, mut selected } => {
                match event {
                    InputEvent::Up => { if selected > 0 { selected -= 1; } }
                    InputEvent::Down => { if selected < 3 { selected += 1; } }
                    InputEvent::Confirm => {
                        if selected == correct_idx {
                            let next = current + 1;
                            if next >= checks.len() {
                                return Screen::CreatePassphrasePrompt {
                                    mnemonic,
                                    selected: 0,
                                };
                            } else {
                                return self.build_verify_screen(mnemonic, checks, next);
                            }
                        }
                        // Wrong — stay on same question, reset selection
                        return Screen::CreateVerify {
                            mnemonic, checks, current, options, correct_idx,
                            selected: 0,
                        };
                    }
                    InputEvent::Back => {
                        // Go back to show words so user can re-check
                        let word_count = mnemonic.split_whitespace().count();
                        return Screen::CreateShowWords { mnemonic, page: 0, word_count };
                    }
                    _ => {}
                }
                Screen::CreateVerify { mnemonic, checks, current, options, correct_idx, selected }
            }

            Screen::CreatePassphrasePrompt { mnemonic, mut selected } => {
                match event {
                    InputEvent::Up | InputEvent::Down => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        if selected == 0 {
                            // Skip passphrase
                            let passphrase = String::new();
                            let address = self.derive_address(&mnemonic, &passphrase);
                            return Screen::CreateConfirm {
                                mnemonic, passphrase, address, selected: 0,
                            };
                        } else {
                            return Screen::CreatePassphraseInput {
                                mnemonic,
                                grid: CharGrid::new(),
                            };
                        }
                    }
                    InputEvent::Back => {
                        let word_count = mnemonic.split_whitespace().count();
                        return self.start_verification(mnemonic, word_count);
                    }
                    _ => {}
                }
                Screen::CreatePassphrasePrompt { mnemonic, selected }
            }

            Screen::CreatePassphraseInput { mnemonic, mut grid } => {
                let done = grid.handle_input(event);
                if done {
                    if grid.text.is_empty() && event == InputEvent::Back {
                        return Screen::CreatePassphrasePrompt { mnemonic, selected: 1 };
                    }
                    let passphrase = grid.text;
                    return Screen::CreatePassphraseConfirm {
                        mnemonic, passphrase, grid: CharGrid::new(),
                    };
                }
                Screen::CreatePassphraseInput { mnemonic, grid }
            }

            Screen::CreatePassphraseConfirm { mnemonic, passphrase, mut grid } => {
                let done = grid.handle_input(event);
                if done {
                    if grid.text.is_empty() && event == InputEvent::Back {
                        // Go back to first entry, pre-filled
                        let mut first_grid = CharGrid::new();
                        first_grid.text = passphrase;
                        return Screen::CreatePassphraseInput { mnemonic, grid: first_grid };
                    }
                    if grid.text == passphrase {
                        let address = self.derive_address(&mnemonic, &passphrase);
                        return Screen::CreateConfirm {
                            mnemonic, passphrase, address, selected: 0,
                        };
                    } else {
                        return Screen::CreatePassphraseMismatch { mnemonic };
                    }
                }
                Screen::CreatePassphraseConfirm { mnemonic, passphrase, grid }
            }

            Screen::CreatePassphraseMismatch { mnemonic } => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::CreatePassphraseInput {
                            mnemonic, grid: CharGrid::new(),
                        };
                    }
                    _ => {}
                }
                Screen::CreatePassphraseMismatch { mnemonic }
            }

            Screen::CreateConfirm { mnemonic, passphrase, address, mut selected } => {
                match event {
                    InputEvent::Left | InputEvent::Right => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        if selected == 0 {
                            // Confirm — load wallet, then offer SeedQR export
                            let seed_qr_data = crate::models::encode_qr::encode_seed_qr(&mnemonic)
                                .unwrap_or_default();
                            let compact_data = crate::models::encode_qr::encode_compact_seed_qr(&mnemonic)
                                .unwrap_or_default();
                            self.load_wallet(mnemonic, passphrase);
                            return Screen::ExportSeedQr { seed_qr_data, compact_data, compact_mode: false, from_settings: false };
                        } else {
                            return Screen::MainMenu { selected: 0 };
                        }
                    }
                    InputEvent::Back => {
                        return Screen::CreatePassphrasePrompt { mnemonic, selected: 0 };
                    }
                    _ => {}
                }
                Screen::CreateConfirm { mnemonic, passphrase, address, selected }
            }

            Screen::ExportSeedQr { seed_qr_data, compact_data, compact_mode, from_settings } => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        if from_settings {
                            return Screen::SettingsMenu { selected: 0 };
                        }
                        return Screen::MainMenu { selected: 0 };
                    }
                    InputEvent::Secondary => {
                        // Toggle between standard and compact SeedQR
                        return Screen::ExportSeedQr {
                            seed_qr_data, compact_data, compact_mode: !compact_mode, from_settings,
                        };
                    }
                    _ => {}
                }
                Screen::ExportSeedQr { seed_qr_data, compact_data, compact_mode, from_settings }
            }

            // === Load Wallet Flow ===
            Screen::LoadMethod { mut selected } => {
                match event {
                    InputEvent::Up | InputEvent::Down => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        if selected == 0 {
                            return Screen::LoadScanQr;
                        } else {
                            return Screen::LoadWordCount { selected: 0 };
                        }
                    }
                    InputEvent::Back => return Screen::MainMenu { selected: 1 },
                    _ => {}
                }
                Screen::LoadMethod { selected }
            }

            Screen::LoadScanQr => {
                match event {
                    InputEvent::Confirm => {
                        // In simulator: if a QR was scanned via webcam, use it; otherwise
                        // fall back to a hardcoded test SeedQR so the flow still works offline.
                        // On Pi: TODO actual camera QR scanning.
                        #[cfg(feature = "simulator")]
                        let data: Vec<u8> = self.scanned_qr.take().unwrap_or_else(|| {
                            "000000000000000000000000000000000000000000000003".as_bytes().to_vec()
                        });
                        #[cfg(not(feature = "simulator"))]
                        let data: Vec<u8> =
                            "000000000000000000000000000000000000000000000003".as_bytes().to_vec();
                        let decoded = decode_qr::detect_and_decode(&data);

                        #[cfg(feature = "simulator")]
                        println!("QR decoded: {:?} ({} bytes raw)", decoded.qr_type, decoded.raw_data.len());

                        if let Some(mnemonic) = decoded.mnemonic {
                            return Screen::LoadPassphrasePrompt { mnemonic, selected: 0 };
                        }
                        if let Some(_addr) = &decoded.address {
                            #[cfg(feature = "simulator")]
                            println!("Scanned address: {}", _addr);
                        }
                    }
                    InputEvent::Back => return Screen::LoadMethod { selected: 0 },
                    _ => {}
                }
                Screen::LoadScanQr
            }

            Screen::LoadWordCount { mut selected } => {
                match event {
                    InputEvent::Up | InputEvent::Down => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        let word_count = if selected == 0 { 12 } else { 24 };
                        return Screen::LoadEnterWords {
                            words: Vec::new(),
                            word_count,
                            picker: WordPicker::new(word_count),
                        };
                    }
                    InputEvent::Back => return Screen::LoadMethod { selected: 1 },
                    _ => {}
                }
                Screen::LoadWordCount { selected }
            }

            Screen::LoadEnterWords { mut words, word_count, mut picker } => {
                if event == InputEvent::Back && picker.prefix.is_empty() && words.is_empty() {
                    return Screen::LoadWordCount { selected: 0 };
                }
                if event == InputEvent::Back && picker.prefix.is_empty() && !words.is_empty() {
                    // Remove last entered word
                    words.pop();
                    picker.word_index = words.len();
                    picker.words = words.clone();
                    return Screen::LoadEnterWords { words, word_count, picker };
                }
                if let Some(_word) = picker.handle_input(event) {
                    let entered = picker.words.clone();
                    if entered.len() == word_count {
                        let mnemonic = entered.join(" ");
                        if bip39::validate_mnemonic(&mnemonic) {
                            return Screen::LoadPassphrasePrompt { mnemonic, selected: 0 };
                        }
                        // Invalid checksum — reset picker for last word
                        picker.words.pop();
                        picker.word_index -= 1;
                    }
                    words = entered;
                }
                Screen::LoadEnterWords { words, word_count, picker }
            }

            Screen::LoadPassphrasePrompt { mnemonic, mut selected } => {
                match event {
                    InputEvent::Up | InputEvent::Down => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        if selected == 0 {
                            let passphrase = String::new();
                            let address = self.derive_address(&mnemonic, &passphrase);
                            return Screen::LoadConfirm {
                                mnemonic, passphrase, address, selected: 0,
                            };
                        } else {
                            return Screen::LoadPassphraseInput {
                                mnemonic,
                                grid: CharGrid::new(),
                            };
                        }
                    }
                    InputEvent::Back => return Screen::LoadMethod { selected: 1 },
                    _ => {}
                }
                Screen::LoadPassphrasePrompt { mnemonic, selected }
            }

            Screen::LoadPassphraseInput { mnemonic, mut grid } => {
                let done = grid.handle_input(event);
                if done {
                    if grid.text.is_empty() && event == InputEvent::Back {
                        return Screen::LoadPassphrasePrompt { mnemonic, selected: 1 };
                    }
                    let passphrase = grid.text;
                    return Screen::LoadPassphraseConfirm {
                        mnemonic, passphrase, grid: CharGrid::new(),
                    };
                }
                Screen::LoadPassphraseInput { mnemonic, grid }
            }

            Screen::LoadPassphraseConfirm { mnemonic, passphrase, mut grid } => {
                let done = grid.handle_input(event);
                if done {
                    if grid.text.is_empty() && event == InputEvent::Back {
                        let mut first_grid = CharGrid::new();
                        first_grid.text = passphrase;
                        return Screen::LoadPassphraseInput { mnemonic, grid: first_grid };
                    }
                    if grid.text == passphrase {
                        let address = self.derive_address(&mnemonic, &passphrase);
                        return Screen::LoadConfirm {
                            mnemonic, passphrase, address, selected: 0,
                        };
                    } else {
                        return Screen::LoadPassphraseMismatch { mnemonic };
                    }
                }
                Screen::LoadPassphraseConfirm { mnemonic, passphrase, grid }
            }

            Screen::LoadPassphraseMismatch { mnemonic } => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::LoadPassphraseInput {
                            mnemonic, grid: CharGrid::new(),
                        };
                    }
                    _ => {}
                }
                Screen::LoadPassphraseMismatch { mnemonic }
            }

            Screen::LoadConfirm { mnemonic, passphrase, address, mut selected } => {
                match event {
                    InputEvent::Left | InputEvent::Right => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        if selected == 0 {
                            self.load_wallet(mnemonic, passphrase);
                            return Screen::MainMenu { selected: 0 };
                        } else {
                            return Screen::MainMenu { selected: 0 };
                        }
                    }
                    InputEvent::Back => {
                        return Screen::LoadPassphrasePrompt { mnemonic, selected: 0 };
                    }
                    _ => {}
                }
                Screen::LoadConfirm { mnemonic, passphrase, address, selected }
            }

            // === Sign TX Flow ===
            Screen::SignNoWallet => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::MainMenu { selected: 2 };
                    }
                    _ => {}
                }
                Screen::SignNoWallet
            }

            Screen::SignScanTx => {
                match event {
                    InputEvent::Confirm => {
                        // In simulator: if a QR was scanned via webcam, use it; otherwise
                        // build a canned test transaction. On Pi: TODO actual camera QR scanning.
                        if let Some(wallet) = &self.wallet {
                            #[cfg(feature = "simulator")]
                            let tx_base64: String = self.scanned_qr.take()
                                .and_then(|b| String::from_utf8(b).ok())
                                .unwrap_or_else(|| self.build_test_transaction(&wallet.keypair.public_key));
                            #[cfg(not(feature = "simulator"))]
                            let tx_base64: String = self.build_test_transaction(&wallet.keypair.public_key);
                            let decoded = decode_qr::detect_and_decode(tx_base64.as_bytes());
                            if let Some(tx_bytes) = decoded.tx_bytes {
                                let (info_lines, can_sign) =
                                    build_review_lines(&tx_bytes, &wallet.keypair.public_key);
                                return Screen::SignReview {
                                    tx_bytes,
                                    tx_base64,
                                    info_lines,
                                    scroll: 0,
                                    // Preselect Reject when we can't sign so a stray Enter is safe.
                                    selected: if can_sign { 0 } else { 1 },
                                    can_sign,
                                };
                            }
                        }
                    }
                    InputEvent::Secondary => {
                        // Sign a message instead of a transaction
                        return Screen::SignMessageInput { grid: CharGrid::new() };
                    }
                    InputEvent::Back => return Screen::MainMenu { selected: 2 },
                    _ => {}
                }
                Screen::SignScanTx
            }

            Screen::SignReview { tx_bytes, tx_base64, info_lines, mut scroll, mut selected, can_sign } => {
                match event {
                    InputEvent::Up => { if scroll > 0 { scroll -= 1; } }
                    InputEvent::Down => {
                        if scroll + 8 < info_lines.len() { scroll += 1; }
                    }
                    InputEvent::Left | InputEvent::Right => {
                        // When signing is blocked, the Sign button is disabled — don't let
                        // navigation land on it.
                        if can_sign {
                            selected = 1 - selected;
                        } else {
                            selected = 1;
                        }
                    }
                    InputEvent::Confirm => {
                        if selected == 0 && can_sign {
                            // Sign using base64 entry point (reviewed before reaching here)
                            if let Some(wallet) = &self.wallet {
                                if let Ok(signed) = crate::models::signer::sign_transaction_base64(
                                    &tx_base64,
                                    &wallet.keypair.private_key,
                                    &wallet.keypair.public_key,
                                ) {
                                    // Log signature and signer for debugging
                                    #[cfg(feature = "simulator")]
                                    {
                                        println!("Signed by: {}", signed.signer_pubkey);
                                        println!("Signature: {}...", hex::encode(&signed.signature[..16]));
                                        println!("TX size: {} bytes", signed.signed_bytes.len());
                                    }
                                    return Screen::SignShowQr { data: signed.signed_base64 };
                                }
                            }
                        } else if selected == 0 && !can_sign {
                            // Sign pressed while disabled — stay on the review screen.
                            return Screen::SignReview { tx_bytes, tx_base64, info_lines, scroll, selected: 1, can_sign };
                        }
                        return Screen::MainMenu { selected: 2 };
                    }
                    InputEvent::Back => return Screen::MainMenu { selected: 2 },
                    _ => {}
                }
                Screen::SignReview { tx_bytes, tx_base64, info_lines, scroll, selected, can_sign }
            }

            Screen::SignShowQr { data } => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::MainMenu { selected: 2 };
                    }
                    InputEvent::Secondary => {
                        // Sign next — go back to scan
                        return Screen::SignScanTx;
                    }
                    _ => {}
                }
                Screen::SignShowQr { data }
            }

            Screen::SignMessageInput { mut grid } => {
                let done = grid.handle_input(event);
                if done {
                    if grid.text.is_empty() && event == InputEvent::Back {
                        return Screen::SignScanTx;
                    }
                    if let Some(wallet) = &self.wallet {
                        let sig = crate::models::signer::sign_message(
                            grid.text.as_bytes(),
                            &wallet.keypair.private_key,
                        );
                        let signature_hex = hex::encode(&sig);
                        return Screen::SignMessageResult { signature_hex };
                    }
                }
                Screen::SignMessageInput { grid }
            }

            Screen::SignMessageResult { signature_hex } => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::MainMenu { selected: 2 };
                    }
                    _ => {}
                }
                Screen::SignMessageResult { signature_hex }
            }

            // === Settings ===
            Screen::SettingsMenu { mut selected } => {
                let item_count = if self.wallet.is_some() { 6 } else { 2 };
                match event {
                    InputEvent::Up => { if selected > 0 { selected -= 1; } }
                    InputEvent::Down => { if selected + 1 < item_count { selected += 1; } }
                    InputEvent::Confirm => {
                        if self.wallet.is_some() {
                            return match selected {
                                0 => Screen::SettingsShowAddress,
                                1 => {
                                    let mnemonic = &self.wallet.as_ref().unwrap().mnemonic;
                                    let seed_qr_data = crate::models::encode_qr::encode_seed_qr(mnemonic)
                                        .unwrap_or_default();
                                    let compact_data = crate::models::encode_qr::encode_compact_seed_qr(mnemonic)
                                        .unwrap_or_default();
                                    Screen::ExportSeedQr { seed_qr_data, compact_data, compact_mode: false, from_settings: true }
                                }
                                2 => {
                                    let accounts = self.build_accounts_list();
                                    Screen::SettingsAccounts { accounts, selected: 0 }
                                }
                                3 => Screen::SettingsVerifyAddressScan,
                                4 => Screen::SettingsAbout,
                                5 => Screen::SettingsPowerOff { selected: 1 },
                                _ => Screen::SettingsMenu { selected },
                            };
                        } else {
                            return match selected {
                                0 => Screen::SettingsAbout,
                                1 => Screen::SettingsPowerOff { selected: 1 },
                                _ => Screen::SettingsMenu { selected },
                            };
                        }
                    }
                    InputEvent::Back => return Screen::MainMenu { selected: 3 },
                    _ => {}
                }
                Screen::SettingsMenu { selected }
            }

            Screen::SettingsVerifyAddressScan => {
                match event {
                    InputEvent::Confirm => {
                        // In simulator: if a QR was scanned via webcam, use it;
                        // otherwise fall back to the loaded wallet's own address
                        // so we can test the happy path without a camera.
                        let wallet = match &self.wallet {
                            Some(w) => w,
                            None => return Screen::SettingsMenu { selected: 3 },
                        };
                        #[cfg(feature = "simulator")]
                        let raw: String = self.scanned_qr.take()
                            .and_then(|b| String::from_utf8(b).ok())
                            .unwrap_or_else(|| wallet.address.clone());
                        #[cfg(not(feature = "simulator"))]
                        let raw: String = wallet.address.clone();

                        // Strip `solana:` URI wrappers + query strings + whitespace.
                        let addr = crate::crypto::derivation::normalize_address_input(&raw);

                        let result = crate::crypto::derivation::verify_address(
                            &wallet.mnemonic,
                            &wallet.passphrase,
                            &addr,
                            10, // search first 10 standard accounts + CLI path
                        );
                        // Preserve the original raw scan in the result so the user
                        // can see what was actually scanned (e.g. a URL) when
                        // format is invalid.
                        let display_addr = if matches!(result, crate::crypto::derivation::AddressMatch::InvalidFormat) {
                            raw
                        } else {
                            addr
                        };
                        return Screen::SettingsVerifyAddressResult { address: display_addr, result };
                    }
                    InputEvent::Back => return Screen::SettingsMenu { selected: 3 },
                    _ => {}
                }
                Screen::SettingsVerifyAddressScan
            }

            Screen::SettingsVerifyAddressResult { address, result } => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::SettingsMenu { selected: 3 };
                    }
                    _ => {}
                }
                Screen::SettingsVerifyAddressResult { address, result }
            }

            Screen::SettingsShowAddress => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::SettingsMenu { selected: 0 };
                    }
                    _ => {}
                }
                Screen::SettingsShowAddress
            }

            Screen::SettingsAccounts { accounts, mut selected } => {
                match event {
                    InputEvent::Up => { if selected > 0 { selected -= 1; } }
                    InputEvent::Down => { if selected + 1 < accounts.len() { selected += 1; } }
                    InputEvent::Confirm | InputEvent::Back => {
                        return Screen::SettingsMenu { selected: 2 };
                    }
                    _ => {}
                }
                Screen::SettingsAccounts { accounts, selected }
            }

            Screen::SettingsAbout => {
                match event {
                    InputEvent::Confirm | InputEvent::Back => {
                        // Return to correct index depending on wallet state
                        let idx = if self.wallet.is_some() { 4 } else { 0 };
                        return Screen::SettingsMenu { selected: idx };
                    }
                    _ => {}
                }
                Screen::SettingsAbout
            }

            Screen::SettingsPowerOff { mut selected } => {
                match event {
                    InputEvent::Left | InputEvent::Right => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        if selected == 0 {
                            // Power off — on Pi this would trigger shutdown
                            self.wallet = None;
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("poweroff").status();
                            }
                            return Screen::Splash;
                        }
                        let idx = if self.wallet.is_some() { 5 } else { 1 };
                        return Screen::SettingsMenu { selected: idx };
                    }
                    InputEvent::Back => {
                        let idx = if self.wallet.is_some() { 5 } else { 1 };
                        return Screen::SettingsMenu { selected: idx };
                    }
                    _ => {}
                }
                Screen::SettingsPowerOff { selected }
            }
        }
    }

    // === Helper methods ===

    fn menu_select(&mut self, selected: usize) -> Screen {
        match selected {
            0 => Screen::CreateWordCount { selected: 0 },
            1 => Screen::LoadMethod { selected: 0 },
            2 => {
                if self.wallet.is_some() {
                    Screen::SignScanTx
                } else {
                    Screen::SignNoWallet
                }
            }
            3 => Screen::SettingsMenu { selected: 0 },
            _ => Screen::MainMenu { selected },
        }
    }

    fn generate_wallet(&self, word_count: usize) -> Screen {
        let mut entropy = [0u8; 32];
        getrandom::getrandom(&mut entropy).expect("Failed to get random entropy");
        let mnemonic = bip39::mnemonic_from_entropy(&entropy, word_count)
            .expect("Failed to generate mnemonic");
        entropy.zeroize();

        Screen::CreateShowWords {
            mnemonic,
            page: 0,
            word_count,
        }
    }

    fn start_verification(&self, mnemonic: String, word_count: usize) -> Screen {
        let num_checks = if word_count == 12 { 3 } else { 5 };
        let words: Vec<&str> = mnemonic.split_whitespace().collect();

        // Pick random word indices to verify
        let mut rng = [0u8; 32];
        getrandom::getrandom(&mut rng).expect("Failed to get random bytes");
        let mut checks: Vec<usize> = Vec::new();
        let mut i = 0;
        while checks.len() < num_checks && i < 32 {
            let idx = rng[i] as usize % words.len();
            if !checks.contains(&idx) {
                checks.push(idx);
            }
            i += 1;
        }
        checks.sort();

        self.build_verify_screen(mnemonic, checks, 0)
    }

    fn build_verify_screen(&self, mnemonic: String, checks: Vec<usize>, current: usize) -> Screen {
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        let correct_word = words[checks[current]];

        // Generate 3 random wrong options
        let mut rng = [0u8; 32];
        getrandom::getrandom(&mut rng).expect("Failed to get random bytes");
        let mut options: Vec<String> = vec![correct_word.to_string()];
        let mut ri = 0;
        while options.len() < 4 && ri < 30 {
            let word_idx = ((rng[ri] as usize) << 3 | (rng[ri + 1] as usize >> 5)) % 2048;
            let word = bip39::get_word(word_idx);
            if !options.contains(&word.to_string()) {
                options.push(word.to_string());
            }
            ri += 2;
        }

        // Shuffle: put correct answer at random position
        let correct_pos = rng[31] as usize % 4;
        options.swap(0, correct_pos);

        Screen::CreateVerify {
            mnemonic,
            checks,
            current,
            options,
            correct_idx: correct_pos,
            selected: 0,
        }
    }

    /// Build accounts list showing standard and CLI derivation paths.
    fn build_accounts_list(&self) -> Vec<(String, String)> {
        let wallet = match &self.wallet {
            Some(w) => w,
            None => return Vec::new(),
        };

        // Standard accounts (m/44'/501'/N'/0')
        let accounts = derivation::derive_multiple_accounts(&wallet.mnemonic, &wallet.passphrase, 3);
        accounts
            .iter()
            .map(|kp| (kp.derivation_path.clone(), derivation::address(kp)))
            .collect()
    }

    /// Build a minimal valid Solana transaction for simulator testing.
    /// Wire format: [num_sigs][signatures...][message...]
    fn build_test_transaction(&self, signer_pubkey: &[u8; 32]) -> String {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;

        let mut tx = Vec::new();
        // 1 signature required
        tx.push(1u8);
        // Placeholder signature (64 zero bytes)
        tx.extend_from_slice(&[0u8; 64]);
        // Message: [num_required_sigs, num_readonly_signed, num_readonly_unsigned]
        tx.push(1); // 1 required signer
        tx.push(0); // 0 readonly signed
        tx.push(1); // 1 readonly unsigned (system program)
        // num_account_keys (compact-u16): 2 accounts
        tx.push(2);
        // Account key 1: signer
        tx.extend_from_slice(signer_pubkey);
        // Account key 2: system program (all zeros)
        tx.extend_from_slice(&[0u8; 32]);
        // Recent blockhash (32 bytes, fake)
        tx.extend_from_slice(&[0xAB; 32]);
        // 1 instruction
        tx.push(1);
        // Instruction: program_id_index=1 (system program), 1 account, 0 data
        tx.push(1); // program index
        tx.push(1); // num accounts in instruction
        tx.push(0); // account index 0 (signer)
        tx.push(0); // data length

        BASE64.encode(&tx)
    }

    fn derive_address(&self, mnemonic: &str, passphrase: &str) -> String {
        let keypair = derivation::derive_keypair(mnemonic, passphrase, 0);
        derivation::address(&keypair)
    }

    fn load_wallet(&mut self, mnemonic: String, passphrase: String) {
        let keypair = derivation::derive_keypair(&mnemonic, &passphrase, 0);
        let address = crate::models::encode_qr::encode_address(&keypair.public_key);
        self.wallet = Some(LoadedWallet {
            mnemonic,
            passphrase,
            keypair,
            address,
        });
    }
}

/// Build the Review TX `info_lines` from a parsed tx. Returns the lines plus
/// `can_sign` — true only when the loaded wallet's pubkey is in the tx's
/// required-signer set. Lines starting with `!` render in danger red.
fn build_review_lines(tx_bytes: &[u8], wallet_pubkey: &[u8; 32]) -> (Vec<String>, bool) {
    use crate::models::tx_parser::{
        format_sol, format_token_amount, summarize, TxKind,
    };

    fn push_wrapped(lines: &mut Vec<String>, label: &str, addr: &str, prefix: &str) {
        lines.push(format!("{label}:"));
        for chunk in addr.as_bytes().chunks(22) {
            lines.push(format!("{prefix}  {}", std::str::from_utf8(chunk).unwrap_or("")));
        }
    }
    fn push_addr(lines: &mut Vec<String>, label: &str, addr: &str) {
        push_wrapped(lines, label, addr, "");
    }

    let mut lines: Vec<String> = Vec::with_capacity(16);

    let summary = match summarize(tx_bytes) {
        Some(s) => s,
        None => {
            lines.push("Type: Unparseable".to_string());
            lines.push(format!("Size: {} bytes", tx_bytes.len()));
            lines.push("! Cannot sign this TX".to_string());
            return (lines, false);
        }
    };
    let can_sign = summary.signers.iter().any(|s| s == wallet_pubkey);

    match &summary.kind {
        TxKind::SolTransfer { from, to, lamports } => {
            lines.push("Type: Send SOL".to_string());
            push_addr(&mut lines, "From", from);
            push_addr(&mut lines, "To", to);
            lines.push(format!("Amount: {} SOL", format_sol(*lamports)));
        }
        TxKind::SplTransfer { source, dest, amount_raw, mint, decimals, symbol } => {
            match symbol {
                Some(sym) => lines.push(format!("Type: Send {}", sym)),
                None => lines.push("Type: Send Token".to_string()),
            }
            if symbol.is_none() {
                if let Some(m) = mint {
                    push_addr(&mut lines, "Mint", m);
                }
            }
            push_addr(&mut lines, "From", source);
            push_addr(&mut lines, "To", dest);
            let amt = format_token_amount(*amount_raw, *decimals);
            match symbol {
                Some(sym) => lines.push(format!("Amount: {} {}", amt, sym)),
                None => lines.push(format!("Amount: {}", amt)),
            }
            if decimals.is_none() {
                lines.push("(legacy: decimals unknown)".to_string());
            }
        }
        TxKind::SplApprove { source, delegate, amount_raw, mint, decimals, symbol } => {
            match symbol {
                Some(sym) => lines.push(format!("Type: Approve {}", sym)),
                None => lines.push("Type: Approve Token".to_string()),
            }
            if symbol.is_none() {
                if let Some(m) = mint {
                    push_addr(&mut lines, "Mint", m);
                }
            }
            push_addr(&mut lines, "Owner", source);
            push_addr(&mut lines, "Delegate", delegate);
            let amt = format_token_amount(*amount_raw, *decimals);
            match symbol {
                Some(sym) => lines.push(format!("Amount: {} {}", amt, sym)),
                None => lines.push(format!("Amount: {}", amt)),
            }
        }
        TxKind::StakeDelegate { stake_account, vote_account } => {
            lines.push("Type: Stake (Delegate)".to_string());
            push_addr(&mut lines, "Stake acct", stake_account);
            push_addr(&mut lines, "Validator", vote_account);
        }
        TxKind::StakeDeactivate { stake_account } => {
            lines.push("Type: Unstake (Deactivate)".to_string());
            push_addr(&mut lines, "Stake acct", stake_account);
        }
        TxKind::StakeWithdraw { stake_account, to, lamports } => {
            lines.push("Type: Unstake (Withdraw)".to_string());
            push_addr(&mut lines, "Stake acct", stake_account);
            push_addr(&mut lines, "To", to);
            lines.push(format!("Amount: {} SOL", format_sol(*lamports)));
        }
        TxKind::Memo { text } => {
            lines.push("Type: Memo".to_string());
            if text.is_empty() {
                lines.push("(empty)".to_string());
            } else {
                lines.push("Text:".to_string());
                for chunk in text.as_bytes().chunks(36) {
                    lines.push(format!("  {}", std::str::from_utf8(chunk).unwrap_or("?")));
                }
            }
        }
        TxKind::Other { programs } => {
            lines.push("Type: Unknown".to_string());
            if programs.is_empty() {
                lines.push("(no program info)".to_string());
            } else {
                lines.push(format!("Programs ({}):", programs.len()));
                for p in programs {
                    for chunk in p.as_bytes().chunks(22) {
                        lines.push(format!("  {}", std::str::from_utf8(chunk).unwrap_or("")));
                    }
                }
            }
        }
    }

    lines.push(format!("Fee: {} SOL", format_sol(summary.fee_lamports)));
    lines.push(format!("Size: {} bytes", summary.size));

    if !can_sign {
        lines.push(String::new());
        lines.push("! Cannot sign this TX".to_string());
        if let Some(needed) = summary.signers.first() {
            let addr = bs58::encode(needed).into_string();
            push_wrapped(&mut lines, "! Need wallet", &addr, "!");
        }
    }

    (lines, can_sign)
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
        assert!(lines.iter().any(|l| l == "Type: Send SOL"));
        // No warning lines when we can sign.
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
        assert!(lines.iter().any(|l| l == "Type: Unparseable"));
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
        assert!(lines.iter().any(|l| l.contains("Amount: 0.01 SOL")));
        assert!(lines.iter().any(|l| l == "Type: Send SOL"));
    }
}
