//! Application state machine — drives all UI flows.

use crate::crypto::derivation;
use crate::crypto::slip0010::SolanaKeypair;
use crate::gui::flows;
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

/// Top-level application.
pub struct App {
    pub screen: Screen,
    pub wallet: Option<LoadedWallet>,
}

impl App {
    pub fn new() -> Self {
        App { screen: Screen::Splash, wallet: None }
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
                | Screen::ExportSeedQr { .. }) => flows::create::handle(self, s, event),

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
                | Screen::SettingsAbout
                | Screen::SettingsPowerOff { .. }) => flows::settings::handle(self, s, event),
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
