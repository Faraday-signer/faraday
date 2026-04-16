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
    },
    SignShowQr { data: String },
    SignMessageInput { grid: CharGrid },
    SignMessageResult { signature_hex: String },

    // Settings
    SettingsMenu { selected: usize },
    SettingsShowAddress,
    SettingsAccounts { accounts: Vec<(String, String)>, selected: usize },
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

/// Top-level application.
pub struct App {
    pub screen: Screen,
    pub wallet: Option<LoadedWallet>,
}

impl App {
    pub fn new() -> Self {
        App {
            screen: Screen::Splash,
            wallet: None,
        }
    }

    pub fn seed_loaded(&self) -> bool {
        self.wallet.is_some()
    }

    pub fn enter_main_menu(&mut self) {
        self.screen = Screen::MainMenu { selected: 0 };
    }

    pub fn handle_input(&mut self, event: InputEvent) {
        let screen = std::mem::replace(&mut self.screen, Screen::Splash);
        self.screen = self.transition(screen, event);
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
                // Each button press captures "camera noise" as entropy
                // On Pi: reads raw bytes from camera sensor
                // In simulator: uses timestamp + random mix as entropy source
                let total_frames = if word_count == 12 { 10 } else { 20 };
                match event {
                    InputEvent::Confirm => {
                        // Gather entropy from timing + system randomness
                        let mut frame_entropy = [0u8; 16];
                        getrandom::getrandom(&mut frame_entropy).ok();
                        // Mix in timing jitter
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
                        // In simulator: decode a test SeedQR through the full pipeline
                        // On Pi: TODO actual camera QR scanning
                        let test_seed_qr = "000000000000000000000000000000000000000000000003";
                        let decoded = decode_qr::detect_and_decode(test_seed_qr.as_bytes());

                        #[cfg(feature = "simulator")]
                        println!("QR decoded: {:?} ({} bytes raw)", decoded.qr_type, decoded.raw_data.len());

                        if let Some(mnemonic) = decoded.mnemonic {
                            return Screen::LoadPassphrasePrompt { mnemonic, selected: 0 };
                        }
                        if let Some(addr) = decoded.address {
                            #[cfg(feature = "simulator")]
                            println!("Scanned address: {}", addr);
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
                        // In simulator: decode a test base64 transaction through the pipeline
                        // On Pi: TODO actual camera QR scanning
                        if let Some(wallet) = &self.wallet {
                            let test_tx = self.build_test_transaction(&wallet.keypair.public_key);
                            let tx_base64 = test_tx.clone();
                            let decoded = decode_qr::detect_and_decode(test_tx.as_bytes());
                            if let Some(tx_bytes) = decoded.tx_bytes {
                                let info_lines = vec![
                                    format!("From: {}...{}", &wallet.address[..4], &wallet.address[wallet.address.len()-4..]),
                                    "To: 11111...1111".to_string(),
                                    "Amount: 1.0 SOL".to_string(),
                                    format!("Type: {}", match decoded.qr_type {
                                        decode_qr::QrType::SolanaTxBase64 => "Transaction",
                                        _ => "Unknown",
                                    }),
                                    format!("Size: {} bytes", tx_bytes.len()),
                                ];
                                return Screen::SignReview {
                                    tx_bytes,
                                    tx_base64,
                                    info_lines,
                                    scroll: 0,
                                    selected: 0,
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

            Screen::SignReview { tx_bytes, tx_base64, info_lines, mut scroll, mut selected } => {
                match event {
                    InputEvent::Up => { if scroll > 0 { scroll -= 1; } }
                    InputEvent::Down => {
                        if scroll + 8 < info_lines.len() { scroll += 1; }
                    }
                    InputEvent::Left | InputEvent::Right => { selected = 1 - selected; }
                    InputEvent::Confirm => {
                        if selected == 0 {
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
                        }
                        return Screen::MainMenu { selected: 2 };
                    }
                    InputEvent::Back => return Screen::MainMenu { selected: 2 },
                    _ => {}
                }
                Screen::SignReview { tx_bytes, tx_base64, info_lines, scroll, selected }
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
                let item_count = if self.wallet.is_some() { 5 } else { 2 };
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
                                3 => Screen::SettingsAbout,
                                4 => Screen::SettingsPowerOff { selected: 1 },
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
                        let idx = if self.wallet.is_some() { 3 } else { 0 };
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
                        let idx = if self.wallet.is_some() { 4 } else { 1 };
                        return Screen::SettingsMenu { selected: idx };
                    }
                    InputEvent::Back => {
                        let idx = if self.wallet.is_some() { 4 } else { 1 };
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
        let mut list: Vec<(String, String)> = accounts.iter()
            .enumerate()
            .map(|(_i, kp)| {
                let addr = derivation::address(kp);
                (kp.derivation_path.clone(), addr)
            })
            .collect();

        // CLI-compatible path (m/44'/501')
        let cli_kp = derivation::derive_keypair_cli_path(&wallet.mnemonic, &wallet.passphrase);
        list.push((cli_kp.derivation_path.clone(), derivation::address(&cli_kp)));

        list
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
