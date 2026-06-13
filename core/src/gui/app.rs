//! Application state machine — drives all UI flows.

use crate::crypto::derivation;
use crate::crypto::slip0010::SolanaKeypair;
use crate::gui::flows;
use crate::ui::Theme;
use zeroize::Zeroizing;

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
    /// Emitted by the input layer when the user long-presses Back (Key3).
    /// Jumps straight to the Power Off confirm screen regardless of which
    /// screen is active — hardware-wallet shortcut for "kill the session".
    PowerOffShortcut,
}

/// Topics for the guided-mode interstitial help screens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HelpTopic {
    Welcome,
    CreateWallet,
    LoadWallet,
    SignTx,
    WalletData,
    ChooseEntropyMethod,
    ScanSeedQr,
    TypeWords,
    BackupSeed,
    VerifyWords,
    Passphrase,
}

impl HelpTopic {
    pub fn title(self) -> &'static str {
        match self {
            Self::Welcome => "WELCOME",
            Self::CreateWallet => "CREATE WALLET",
            Self::LoadWallet => "LOAD WALLET",
            Self::SignTx => "SIGN",
            Self::WalletData => "WALLET DATA",
            Self::ChooseEntropyMethod => "ENTROPY",
            Self::ScanSeedQr => "SCAN SEEDQR",
            Self::TypeWords => "TYPE WORDS",
            Self::BackupSeed => "BACKUP",
            Self::VerifyWords => "VERIFY",
            Self::Passphrase => "PASSPHRASE",
        }
    }

    pub fn body(self) -> &'static str {
        match self {
            Self::Welcome => "Faraday is an offline Solana signer. Create or load a wallet to start.",
            Self::CreateWallet => "Generate a brand new wallet from random entropy. You will back it up on paper.",
            Self::LoadWallet => "Restore a wallet you already backed up, by scanning its QR or typing the words.",
            Self::SignTx => "Scan a transaction QR from your computer, review it, and sign with your loaded key.",
            Self::WalletData => "View your address as text or QR, or back up your seed.",
            Self::ChooseEntropyMethod => "Pick how randomness is generated: auto, camera noise, coin flips, or dice rolls.",
            Self::ScanSeedQr => "Point the camera at your paper SeedQR. It will be decoded automatically.",
            Self::TypeWords => "Enter each BIP39 word using the grid. Words auto-complete after a few letters.",
            Self::BackupSeed => "Your seed is the ONLY way to recover funds. Write it on paper or metal. Keep it safe.",
            Self::VerifyWords => "We will now check that you wrote the words correctly. A few will be asked.",
            Self::Passphrase => "DONE: no passphrase. ENCRYPT adds one you must enter every time. Forget it and the funds are gone.",
        }
    }
}

/// Current screen with its mutable state.
pub enum Screen {
    Splash,
    ModeSelect {
        selected: usize,
        shown_at: std::time::Instant,
    },
    Help {
        topic: HelpTopic,
    },
    MainMenu {
        selected: usize,
    },

    // Create wallet flow
    CreateWordCount {
        selected: usize,
    },
    CreateMethod {
        word_count: usize,
        selected: usize,
    },
    CreateCameraEntropy {
        word_count: usize,
        frames_collected: usize,
        entropy: Vec<u8>,
    },
    CreateCoinFlips {
        word_count: usize,
        bits: Vec<bool>,
        selected: usize,
    },
    CreateDiceRolls {
        word_count: usize,
        rolls: Vec<u8>,
        selected: usize,
    },
    /// Mandatory acknowledgment before plaintext words are revealed for the
    /// first time on the create flow. Forces the user to read the
    /// "pen and paper" instruction and explicitly choose I UNDERSTAND
    /// (advance to show words) or CANCEL (back to method picker).
    CreateBackupWarning {
        mnemonic: Zeroizing<String>,
        word_count: usize,
        selected: usize,
    },
    CreateShowWords {
        mnemonic: Zeroizing<String>,
        page: usize,
        word_count: usize,
    },
    CreateVerify {
        mnemonic: Zeroizing<String>,
        checks: Vec<usize>,
        current: usize,
        options: Vec<String>,
        correct_idx: usize,
        selected: usize,
    },
    CreatePassphrasePrompt {
        mnemonic: Zeroizing<String>,
        selected: usize,
    },
    CreatePassphraseInput {
        mnemonic: Zeroizing<String>,
        grid: CharGrid,
    },
    CreatePassphraseConfirm {
        mnemonic: Zeroizing<String>,
        passphrase: Zeroizing<String>,
        grid: CharGrid,
    },
    CreatePassphraseMismatch {
        mnemonic: Zeroizing<String>,
    },
    CreateConfirm {
        mnemonic: Zeroizing<String>,
        passphrase: Zeroizing<String>,
        keypair: SolanaKeypair,
        address: String,
        selected: usize,
    },
    /// Pre-export warning. Forces the user to read the consequence before any
    /// seed-backup action (menu entry, show-words, block view, etc.).
    ExportSeedWarning {
        selected: usize,
        from_settings: bool,
    },
    /// Specific warning before SHOW WORDS — the plaintext seed is the most
    /// dangerous surface, so it gates that action even on the happy post-create
    /// path (where the entry-level warning is skipped).
    ShowWordsWarning {
        compact_data: Vec<u8>,
        mnemonic: Zeroizing<String>,
        word_count: usize,
        selected: usize,
        from_settings: bool,
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
        mnemonic: Zeroizing<String>,
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
    VerifyBackupPassphrase {
        grid: CharGrid,
    },
    VerifyBackupPassphraseMismatch,
    VerifyBackupSuccess,

    // Load wallet flow
    LoadMethod {
        selected: usize,
    },
    LoadScanQr,
    /// Shown when the 12 or 24 typed words fail the BIP39 checksum.
    LoadInvalidMnemonic {
        word_count: usize,
    },
    LoadWordCount {
        selected: usize,
    },
    LoadEnterWords {
        words: Vec<String>,
        word_count: usize,
        picker: WordPicker,
    },
    /// Transient flash shown for ~1s after a word auto-commits during entry.
    /// Tick advances to the next-word LoadEnterWords (or to the validation
    /// branch — LoadFinalize / LoadInvalidMnemonic / DerivationError — when
    /// this was the last word).
    LoadWordCommitted {
        just_committed: String,
        picker: WordPicker,
        word_count: usize,
        shown_at: std::time::Instant,
    },
    /// Passphrase decision: Done (no passphrase) / Add passphrase. Short
    /// address shown in the header chip so users keep visual continuity
    /// with the preceding confirmation.
    LoadFinalize {
        mnemonic: Zeroizing<String>,
        preview_address: String,
        keypair: SolanaKeypair,
        selected: usize,
    },
    LoadPassphrasePrompt {
        mnemonic: Zeroizing<String>,
        selected: usize,
    },
    LoadPassphraseInput {
        mnemonic: Zeroizing<String>,
        grid: CharGrid,
    },
    LoadPassphraseConfirm {
        mnemonic: Zeroizing<String>,
        passphrase: Zeroizing<String>,
        grid: CharGrid,
    },
    LoadPassphraseMismatch {
        mnemonic: Zeroizing<String>,
    },
    LoadConfirm {
        mnemonic: Zeroizing<String>,
        passphrase: Zeroizing<String>,
        keypair: SolanaKeypair,
        address: String,
        selected: usize,
    },

    // Fatal: key derivation failed (HMAC or crypto library error).
    DerivationError,

    // Sign TX flow
    SignScanTx,
    SignReview {
        tx_bytes: Vec<u8>,
        tx_base64: String,
        info_lines: Vec<String>,
        /// Structured parse of `tx_bytes`. Carried alongside `info_lines` so
        /// detail pages (metadata / instruction list / per-instruction / raw)
        /// can render directly from the parsed instructions instead of
        /// re-parsing or re-flattening. Boxed because `ParsedTransaction` is
        /// large and `Screen` is moved on every input event.
        parsed: Box<crate::parser::ParsedTransaction>,
        /// Which review page is currently shown. K2 advances; the renderer
        /// dispatches on this index. Wraps after the last page.
        // Page 0 = Summary (hero + first chunk of details);
        // Page 1 = Tx metadata; Page 2 = Instructions overview;
        // Pages 3..3+N-1 = one per instruction; Page 3+N = Raw bytes.
        page: usize,
        // Scroll position within page 0's detail block. Reset to 0 on
        // page change. Pages 1..K don't scroll — they fit in one screen.
        scroll: usize,
        selected: usize,
        /// False when the loaded wallet's pubkey is not in the tx's required
        /// signer set — Sign is disabled in that case.
        can_sign: bool,
    },
    SignShowQr {
        data: String,
    },
    SignMessageInput {
        grid: CharGrid,
    },
    SignMessageReview {
        message_bytes: Vec<u8>,
        scroll: usize,
    },
    SignMessageResult {
        signature_hex: String,
    },

    // Settings
    SettingsMenu {
        selected: usize,
    },
    SettingsShowAddress,
    /// Address rendered as wrapped text so the user can read it digit-by-digit
    /// or transcribe it onto paper.
    SettingsShowAddressText,
    SettingsAbout,
    /// Wipe-in-memory-wallet confirm. Reachable only via the long-press Back
    /// shortcut; not exposed in the wallet-data menu.
    SettingsPowerOff {
        selected: usize,
    },
}

/// Character grid for passphrase entry.
pub struct CharGrid {
    pub text: String,
    pub row: usize,
    pub col: usize,
    pub caps: bool,
}

pub const GRID_CHARS: [[char; 10]; 5] = [
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
        CharGrid {
            text: String::new(),
            row: 0,
            col: 0,
            caps: false,
        }
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
            InputEvent::Confirm => match self.current_cell() {
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
                    return true;
                }
            },
            InputEvent::Back => {
                // K3 = always return to the previous screen. Deleting
                // last character is K2's job now.
                return true;
            }
            InputEvent::Secondary => {
                // K2 = delete last character. Caps toggle still available
                // via the CAPS button in the action row.
                self.text.pop();
            }
            InputEvent::PowerOffShortcut => {}
        }
        false
    }
}

/// BIP39 word picker with prefix filtering.
/// 6×5 alphabet keyboard for word entry. The first 26 cells (row-major)
/// are A..Z; the last 4 cells (row 4, cols 2..5) are blank and never
/// receive the cursor. Distinct from `GRID_COLS` / `GRID_ROWS` above,
/// which size the passphrase `CharGrid`.
pub const WORD_GRID_COLS: u8 = 6;
pub const WORD_GRID_ROWS: u8 = 5;

pub struct WordPicker {
    pub prefix: String,
    pub cursor_row: u8,
    pub cursor_col: u8,
    pub word_index: usize,
    pub word_count: usize,
    pub words: Vec<String>,
}

impl WordPicker {
    pub fn new(word_count: usize) -> Self {
        let mut picker = WordPicker {
            prefix: String::new(),
            cursor_row: 0,
            cursor_col: 0,
            word_index: 0,
            word_count,
            words: Vec::new(),
        };
        picker.snap_to_valid();
        picker
    }

    /// Letter at grid cell (row, col), or None for the blank trailing cells.
    pub const fn cell_letter(row: u8, col: u8) -> Option<char> {
        let idx = row as u32 * WORD_GRID_COLS as u32 + col as u32;
        if idx < 26 {
            Some((b'a' + idx as u8) as char)
        } else {
            None
        }
    }

    /// Letter currently under the cursor, if the cursor sits on a real cell.
    pub fn cursor_letter(&self) -> Option<char> {
        Self::cell_letter(self.cursor_row, self.cursor_col)
    }

    /// `[bool; 26]` — true at index `i` if at least one BIP39 word starting
    /// with `prefix` has letter `'a' + i` at position `prefix.len()`.
    /// I.e. "letters that, if appended next, still keep at least one
    /// candidate alive." Any letter outside this set must be dimmed.
    pub fn valid_letters(&self) -> [bool; 26] {
        let mut valid = [false; 26];
        let pos = self.prefix.len();
        for (_, word) in crate::crypto::bip39::words_with_prefix(&self.prefix) {
            let bytes = word.as_bytes();
            if bytes.len() <= pos {
                continue;
            }
            let next = bytes[pos];
            if (b'a'..=b'z').contains(&next) {
                valid[(next - b'a') as usize] = true;
            }
        }
        valid
    }

    /// If the cursor is on a dimmed cell or a blank cell, jump it to the
    /// first valid cell (row-major). Idempotent.
    pub fn snap_to_valid(&mut self) {
        let valid = self.valid_letters();
        if let Some(ch) = self.cursor_letter() {
            if valid[(ch as u8 - b'a') as usize] {
                return;
            }
        }
        for r in 0..WORD_GRID_ROWS {
            for c in 0..WORD_GRID_COLS {
                if let Some(ch) = Self::cell_letter(r, c) {
                    if valid[(ch as u8 - b'a') as usize] {
                        self.cursor_row = r;
                        self.cursor_col = c;
                        return;
                    }
                }
            }
        }
    }

    pub fn handle_input(&mut self, event: InputEvent) -> Option<String> {
        let valid = self.valid_letters();
        match event {
            InputEvent::Right => {
                self.move_right(&valid);
            }
            InputEvent::Left => {
                self.move_left(&valid);
            }
            InputEvent::Down => self.move_vertical(&valid, /* down = */ true),
            InputEvent::Up => self.move_vertical(&valid, /* down = */ false),
            InputEvent::Confirm => {
                // K1: append the highlighted letter to the prefix. If the
                // prefix now uniquely identifies a word (typically after
                // 3–5 letters because BIP39's first 4 chars are unique),
                // auto-commit the full word and advance.
                if let Some(ch) = self.cursor_letter() {
                    if !valid[(ch as u8 - b'a') as usize] {
                        return None;
                    }
                    self.prefix.push(ch);
                    let matches = crate::crypto::bip39::words_with_prefix(&self.prefix);
                    if matches.len() == 1 {
                        let word = matches[0].1.to_string();
                        self.words.push(word.clone());
                        self.word_index += 1;
                        self.prefix.clear();
                        self.cursor_row = 0;
                        self.cursor_col = 0;
                        self.snap_to_valid();
                        return Some(word);
                    }
                    self.snap_to_valid();
                }
            }
            InputEvent::Secondary => {
                // K2: delete last letter.
                if !self.prefix.is_empty() {
                    self.prefix.pop();
                    self.snap_to_valid();
                }
            }
            InputEvent::Back | InputEvent::PowerOffShortcut => {
                // Word-level / global navigation handled by the caller.
            }
        }
        None
    }

    /// Walk forward, preferring within-row wrap. If the current row has no
    /// other valid letter (so within-row wrap would leave the cursor stuck),
    /// fall through to a row-major scan across the full grid so the user is
    /// never trapped — they always advance to the next valid letter somewhere.
    fn move_right(&mut self, valid: &[bool; 26]) {
        let row_len = Self::row_letter_count(self.cursor_row);
        if row_len > 0 {
            for offset in 1..row_len {
                let c = (self.cursor_col + offset) % row_len;
                if let Some(ch) = Self::cell_letter(self.cursor_row, c) {
                    if valid[(ch as u8 - b'a') as usize] {
                        self.cursor_col = c;
                        return;
                    }
                }
            }
        }
        let total = WORD_GRID_ROWS as u32 * WORD_GRID_COLS as u32;
        let start = self.cursor_row as u32 * WORD_GRID_COLS as u32 + self.cursor_col as u32;
        for offset in 1..=total {
            let idx = (start + offset) % total;
            let r = (idx / WORD_GRID_COLS as u32) as u8;
            let c = (idx % WORD_GRID_COLS as u32) as u8;
            if let Some(ch) = Self::cell_letter(r, c) {
                if valid[(ch as u8 - b'a') as usize] {
                    self.cursor_row = r;
                    self.cursor_col = c;
                    return;
                }
            }
        }
    }

    /// Reverse of `move_right`: within-row wrap first, then row-major fallback.
    fn move_left(&mut self, valid: &[bool; 26]) {
        let row_len = Self::row_letter_count(self.cursor_row);
        if row_len > 0 {
            for offset in 1..row_len {
                let c = (self.cursor_col + row_len - offset) % row_len;
                if let Some(ch) = Self::cell_letter(self.cursor_row, c) {
                    if valid[(ch as u8 - b'a') as usize] {
                        self.cursor_col = c;
                        return;
                    }
                }
            }
        }
        let total = WORD_GRID_ROWS as u32 * WORD_GRID_COLS as u32;
        let start = self.cursor_row as u32 * WORD_GRID_COLS as u32 + self.cursor_col as u32;
        for offset in 1..=total {
            let idx = (start + total - offset) % total;
            let r = (idx / WORD_GRID_COLS as u32) as u8;
            let c = (idx % WORD_GRID_COLS as u32) as u8;
            if let Some(ch) = Self::cell_letter(r, c) {
                if valid[(ch as u8 - b'a') as usize] {
                    self.cursor_row = r;
                    self.cursor_col = c;
                    return;
                }
            }
        }
    }

    /// Number of real letter cells on `row`. The last row of the 6×5 grid
    /// only carries 'y' and 'z' — the trailing 4 cells are blank and must
    /// not participate in row-wrap.
    fn row_letter_count(row: u8) -> u8 {
        let mut count = 0u8;
        for c in 0..WORD_GRID_COLS {
            if Self::cell_letter(row, c).is_some() {
                count += 1;
            }
        }
        count
    }

    /// Move the cursor row-by-row (with wrap), preferring the same column
    /// at the new row. If that cell is dimmed/blank, scan the new row
    /// left-to-right for the first valid cell. If the new row is entirely
    /// dim, advance to the next row and repeat. Always lands on something
    /// as long as one valid letter exists.
    fn move_vertical(&mut self, valid: &[bool; 26], down: bool) {
        let rows = WORD_GRID_ROWS as i32;
        for r_offset in 1..=rows {
            let r = if down {
                ((self.cursor_row as i32 + r_offset).rem_euclid(rows)) as u8
            } else {
                ((self.cursor_row as i32 - r_offset).rem_euclid(rows)) as u8
            };
            if let Some(ch) = Self::cell_letter(r, self.cursor_col) {
                if valid[(ch as u8 - b'a') as usize] {
                    self.cursor_row = r;
                    return;
                }
            }
            for c in 0..WORD_GRID_COLS {
                if let Some(ch) = Self::cell_letter(r, c) {
                    if valid[(ch as u8 - b'a') as usize] {
                        self.cursor_row = r;
                        self.cursor_col = c;
                        return;
                    }
                }
            }
        }
    }
}

/// Wallet loaded in memory.
pub struct LoadedWallet {
    pub mnemonic: Zeroizing<String>,
    pub passphrase: Zeroizing<String>,
    pub keypair: SolanaKeypair,
    pub address: String,
}

/// Layout information for the selectable list on the current screen.
/// Returned by `App::tap_layout()` for use by touch-screen platforms.
pub struct TapLayout {
    /// Number of slots the list widget divides the body area into.
    pub max_visible: usize,
    /// Total number of items in the list (may exceed `max_visible`).
    pub total_items: usize,
    /// Currently highlighted item index (absolute, not slot-relative).
    pub current_selected: usize,
    /// Y coordinate (pixels) where the tappable list area begins.
    /// For standard `ListScreen`s this equals `theme.header_h`.
    /// For screens with a text preamble (e.g. `CreateBackupWarning`)
    /// it is further down the screen.
    pub list_top: u16,
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

/// External-battery state, surfaced as a footer icon on touch builds.
///
/// Populated by the platform layer (ESP32 only — the Pi has no battery and
/// never sets it). `percent` is clamped 0..=100.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryStatus {
    pub percent: u8,
}

/// Top-level application.
pub struct App {
    pub theme: Theme,
    pub screen: Screen,
    pub guided: bool,
    pub pending_screen: Option<Screen>,
    help_return: Option<HelpTopic>,
    help_return_for: Option<std::mem::Discriminant<Screen>>,
    pub wallet: Option<LoadedWallet>,
    pub last_activity: std::time::Instant,
    pub blank_timeout_ms: u64,
    /// Wall-clock anchor for the splash-screen DVD-style bounce. Set once
    /// at construction and never reset — the bounce position is a pure
    /// deterministic function of `elapsed()`, so transitions in and out of
    /// the splash don't cause it to jump.
    pub splash_anim_start: std::time::Instant,
    pub latest_frame: Option<crate::camera::Frame>,
    pub camera_error: Option<String>,
    pub scanned_qr: Option<Vec<u8>>,
    pub scan_diag: crate::camera::ScanDiagnostics,
    /// External battery state, or `None` when no battery is connected (or on
    /// platforms without one). Drawn as a footer icon on touch builds.
    pub battery: Option<BatteryStatus>,
}

impl App {
    pub fn new(theme: Theme) -> Self {
        App {
            theme,
            screen: Screen::Splash,
            guided: false,
            pending_screen: None,
            help_return: None,
            help_return_for: None,
            wallet: None,
            last_activity: std::time::Instant::now(),
            blank_timeout_ms: DEFAULT_BLANK_TIMEOUT_MS,
            splash_anim_start: std::time::Instant::now(),
            latest_frame: None,
            camera_error: None,
            scanned_qr: None,
            scan_diag: crate::camera::ScanDiagnostics::default(),
            battery: None,
        }
    }

    /// True when the current screen uses the middle footer cell (`k2`) for a
    /// Secondary action — keyboard/word-entry delete, or TX-review paging. The
    /// battery icon lives in that same cell, so it must yield when a control is
    /// already there.
    #[cfg(feature = "touch-ui")]
    pub(crate) fn footer_has_secondary(&self) -> bool {
        matches!(
            self.screen,
            Screen::CreatePassphraseInput { .. }
                | Screen::CreatePassphraseConfirm { .. }
                | Screen::LoadPassphraseInput { .. }
                | Screen::LoadPassphraseConfirm { .. }
                | Screen::VerifyBackupPassphrase { .. }
                | Screen::SignMessageInput { .. }
                | Screen::LoadEnterWords { .. }
                | Screen::SignReview { .. }
        )
    }

    pub fn seed_loaded(&self) -> bool {
        self.wallet.is_some()
    }

    /// Title used for every screen in the SeedQR backup flow. Appends "+P"
    /// when a passphrase is set, so the user is continuously reminded the
    /// QR alone isn't enough to restore the wallet.
    pub fn seedqr_title(&self) -> &'static str {
        let has_passphrase = self
            .wallet
            .as_ref()
            .map_or(false, |w| !w.passphrase.is_empty());
        if has_passphrase {
            "SeedQR +P"
        } else {
            "SeedQR"
        }
    }

    pub fn enter_main_menu(&mut self) {
        self.screen = Screen::ModeSelect {
            selected: 0,
            shown_at: std::time::Instant::now(),
        };
    }

    /// Drop the in-memory wallet — the same wipe the reset-wallet confirm
    /// performs — zeroizing the seed/passphrase/keys on drop, and return to the
    /// main menu. Used by the hardware power-off path so no key material is left
    /// in RAM while the device sleeps.
    pub fn wipe_wallet(&mut self) {
        self.wallet = None;
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
        // Long-press Back shortcut: jump straight to Reset Wallet confirm
        // from anywhere. Only meaningful when a wallet is loaded.
        if matches!(event, InputEvent::PowerOffShortcut) {
            if self.wallet.is_some() {
                self.screen = Screen::SettingsPowerOff { selected: 0 };
            }
            return;
        }
        let screen = std::mem::replace(&mut self.screen, Screen::Splash);
        self.screen = self.transition(screen, event);
    }

    /// True when the screen should render as blank right now.
    pub fn is_blanked(&self) -> bool {
        let idle_ms = self
            .last_activity
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let on_camera = self.wants_camera();
        should_blank(idle_ms, self.blank_timeout_ms, on_camera)
    }

    pub fn camera_error_str(&self) -> Option<&str> {
        self.camera_error.as_deref()
    }

    pub fn has_camera_frame(&self) -> bool {
        self.latest_frame.is_some()
    }

    pub fn wants_camera(&self) -> bool {
        matches!(
            &self.screen,
            Screen::LoadScanQr
                | Screen::SignScanTx
                | Screen::CreateCameraEntropy { .. }
                | Screen::VerifyBackupScan
        )
    }

    /// Per-frame update — handles timers, auto-dismiss, and scanned-QR auto-advance.
    ///
    /// Camera lifecycle (open/close, pulling frames, feeding `scanned_qr`) is
    /// the responsibility of each platform's main loop. Call this after updating
    /// the camera fields.
    pub fn tick(&mut self) {
        if let Screen::ModeSelect { shown_at, .. } = &self.screen {
            if shown_at.elapsed() >= std::time::Duration::from_secs(5) {
                self.guided = false;
                self.screen = Screen::MainMenu { selected: 0 };
            }
        }

        if let Screen::LoadWordCommitted { shown_at, .. } = &self.screen {
            if shown_at.elapsed() >= std::time::Duration::from_millis(900) {
                let taken = std::mem::replace(&mut self.screen, Screen::Splash);
                if let Screen::LoadWordCommitted {
                    picker, word_count, ..
                } = taken
                {
                    if picker.words.len() == word_count {
                        let mnemonic = Zeroizing::new(picker.words.join(" "));
                        if crate::crypto::bip39::validate_mnemonic(&mnemonic) {
                            match self.derive_keypair_and_address(&mnemonic, "") {
                                Some((keypair, preview_address)) => {
                                    self.screen = Screen::LoadFinalize {
                                        mnemonic,
                                        preview_address,
                                        keypair,
                                        selected: 0,
                                    };
                                }
                                None => {
                                    self.screen = Screen::DerivationError;
                                }
                            }
                        } else {
                            self.screen = Screen::LoadInvalidMnemonic { word_count };
                        }
                    } else {
                        let words = picker.words.clone();
                        self.screen = Screen::LoadEnterWords {
                            words,
                            word_count,
                            picker,
                        };
                    }
                }
            }
        }

        if let Some(data) = self.scanned_qr.take() {
            if matches!(
                self.screen,
                Screen::LoadScanQr | Screen::SignScanTx | Screen::VerifyBackupScan
            ) {
                self.scanned_qr = Some(data);
                self.handle_input(InputEvent::Confirm);
            }
        }
    }

    pub fn is_scan_screen(&self) -> bool {
        matches!(
            self.screen,
            Screen::LoadScanQr | Screen::SignScanTx | Screen::VerifyBackupScan
        )
    }

    /// Geometry of the selectable list on the current screen, if any.
    /// Used by touch-screen platforms to map a tap y-coordinate to a row
    /// index. Returns `None` for screens without a tappable list.
    pub fn tap_layout(&self) -> Option<TapLayout> {
        let header_h = self.theme.header_h as u16;
        // Most screens are standard ListScreens whose list begins right
        // below the header.
        let t = |max_visible, total_items, current_selected| TapLayout {
            max_visible,
            total_items,
            current_selected,
            list_top: header_h,
        };
        match &self.screen {
            Screen::ModeSelect { selected, .. } => Some(t(2, 2, *selected)),
            Screen::MainMenu { selected } => Some(t(3, self.menu_items().len(), *selected)),
            Screen::CreateWordCount { selected } => Some(t(3, 2, *selected)),
            Screen::LoadWordCount { selected } => Some(t(3, 2, *selected)),
            Screen::CreateMethod { selected, .. } => Some(t(3, 4, *selected)),
            Screen::LoadMethod { selected } => Some(t(3, 2, *selected)),
            Screen::CreatePassphrasePrompt { selected, .. } => Some(t(3, 2, *selected)),
            Screen::LoadPassphrasePrompt { selected, .. } => Some(t(3, 2, *selected)),
            Screen::LoadFinalize { selected, .. } => Some(t(3, 2, *selected)),
            Screen::CreateVerify { options, selected, .. } => Some(t(4, options.len(), *selected)),
            Screen::ExportSeedQrMenu { selected, .. } => Some(t(3, 3, *selected)),
            Screen::ExportSeedWarning { selected, .. } => Some(t(3, 2, *selected)),
            Screen::ShowWordsWarning { selected, .. } => Some(t(3, 2, *selected)),
            Screen::SettingsMenu { selected } => Some(t(3, 4, *selected)),
            Screen::SettingsPowerOff { selected } => Some(t(3, 2, *selected)),
            // Custom layout: top third is instruction text, bottom two-thirds
            // is the 2-item list.
            Screen::CreateBackupWarning { selected, .. } => {
                // Mirror draw_create_backup_warning: the body is shrunk by the
                // chrome (FOOTER_H on touch builds) before the third split, so
                // the tappable list begins at the same y the rows are drawn.
                let footer_h = crate::ui::widgets::FOOTER_H as u16;
                let body_h = self.theme.height as u16 - header_h - footer_h;
                Some(TapLayout {
                    max_visible: 2,
                    total_items: 2,
                    current_selected: *selected,
                    list_top: header_h + body_h / 3,
                })
            }
            _ => None,
        }
    }

    /// True when a body tap on the current screen should page forward through
    /// structured content rather than fire `Confirm`. The TX review screen
    /// uses this on touch builds: tapping advances to the next review page,
    /// leaving `Confirm` (the SIGN footer cell) to actually sign.
    pub fn tap_pages_review(&self) -> bool {
        matches!(self.screen, Screen::SignReview { .. })
    }

    /// True while the BIP39 word-entry alphabet grid is showing. Touch builds
    /// use this to keep that screen tap-only: swipes don't move the cursor and
    /// taps off the letter cells don't commit anything.
    pub fn on_word_picker(&self) -> bool {
        matches!(self.screen, Screen::LoadEnterWords { .. })
    }

    /// Move the cursor to `row` on the current screen without triggering a
    /// screen transition. Used by touch-screen platforms to highlight the
    /// tapped row before firing a `Confirm` event.
    pub fn set_selected(&mut self, row: usize) {
        match &mut self.screen {
            Screen::ModeSelect { selected, .. } => *selected = row,
            Screen::MainMenu { selected } => *selected = row,
            Screen::CreateWordCount { selected } => *selected = row,
            Screen::LoadWordCount { selected } => *selected = row,
            Screen::CreateMethod { selected, .. } => *selected = row,
            Screen::LoadMethod { selected } => *selected = row,
            Screen::CreatePassphrasePrompt { selected, .. } => *selected = row,
            Screen::LoadPassphrasePrompt { selected, .. } => *selected = row,
            Screen::LoadFinalize { selected, .. } => *selected = row,
            Screen::CreateVerify { selected, .. } => *selected = row,
            Screen::ExportSeedQrMenu { selected, .. } => *selected = row,
            Screen::ExportSeedWarning { selected, .. } => *selected = row,
            Screen::ShowWordsWarning { selected, .. } => *selected = row,
            Screen::CreateBackupWarning { selected, .. } => *selected = row,
            Screen::SettingsMenu { selected } => *selected = row,
            Screen::SettingsPowerOff { selected } => *selected = row,
            _ => {}
        }
    }

    /// If the current screen contains a `CharGrid` (passphrase / message
    /// entry), compute which grid cell `(x, y)` falls in, move the cursor
    /// there, and return `true`. The caller should then fire `Confirm` to
    /// type the character. Returns `false` if the tap is outside the grid
    /// or the screen has no `CharGrid`.
    pub fn tap_char_grid(&mut self, x: u16, y: u16) -> bool {
        let is_grid = matches!(
            &self.screen,
            Screen::CreatePassphraseInput { .. }
            | Screen::CreatePassphraseConfirm { .. }
            | Screen::LoadPassphraseInput { .. }
            | Screen::LoadPassphraseConfirm { .. }
            | Screen::VerifyBackupPassphrase { .. }
            | Screen::SignMessageInput { .. }
        );
        if !is_grid { return false; }

        const PREVIEW_H: u16 = 28;
        let gutter_w = crate::ui::widgets::GUTTER_W as u16;
        let footer_h = crate::ui::widgets::FOOTER_H as u16;
        let grid_top = self.theme.header_h as u16 + PREVIEW_H;
        let body_w = self.theme.width as u16 - gutter_w;
        let body_bottom = self.theme.height as u16 - footer_h;
        if y < grid_top || y >= body_bottom || x >= body_w { return false; }
        let cell_w = body_w / GRID_COLS as u16;
        let row_h = (body_bottom - grid_top) / GRID_ROWS as u16;
        if cell_w == 0 || row_h == 0 { return false; }

        let col = ((x / cell_w) as usize).min(GRID_COLS - 1);
        let row = (((y - grid_top) / row_h) as usize).min(GRID_ROWS - 1);

        match &mut self.screen {
            Screen::CreatePassphraseInput { grid, .. }
            | Screen::CreatePassphraseConfirm { grid, .. }
            | Screen::LoadPassphraseInput { grid, .. }
            | Screen::LoadPassphraseConfirm { grid, .. }
            | Screen::VerifyBackupPassphrase { grid }
            | Screen::SignMessageInput { grid } => {
                grid.row = row;
                grid.col = col;
            }
            _ => return false,
        }
        true
    }

    /// If the current screen is `LoadEnterWords`, compute which alphabet
    /// cell `(x, y)` falls in. Moves the cursor to that cell and returns
    /// `true` only if the letter is non-blank and currently valid (not
    /// dimmed). The caller should then fire `Confirm` to append the letter.
    pub fn tap_word_grid(&mut self, x: u16, y: u16) -> bool {
        const PREFIX_H: u16 = 36;
        let gutter_w = crate::ui::widgets::GUTTER_W as u16;
        let footer_h = crate::ui::widgets::FOOTER_H as u16;
        let grid_top = self.theme.header_h as u16 + PREFIX_H;
        let body_w = self.theme.width as u16 - gutter_w;
        let body_bottom = self.theme.height as u16 - footer_h;
        if y < grid_top || y >= body_bottom || x >= body_w { return false; }
        let cell_w = body_w / WORD_GRID_COLS as u16;
        let cell_h = (body_bottom - grid_top) / WORD_GRID_ROWS as u16;
        if cell_w == 0 || cell_h == 0 { return false; }

        let col = ((x / cell_w) as u8).min(WORD_GRID_COLS - 1);
        let row = (((y - grid_top) / cell_h) as u8).min(WORD_GRID_ROWS - 1);

        // Reject blank trailing cells (the last row of the 6×5 grid only
        // carries 'y' and 'z'; the remaining 4 cells are empty).
        let Some(ch) = WordPicker::cell_letter(row, col) else { return false };

        let Screen::LoadEnterWords { picker, .. } = &mut self.screen else { return false };

        // Only confirm letters that are still valid for the current prefix.
        let valid = picker.valid_letters();
        if !valid[(ch as u8 - b'a') as usize] { return false; }

        picker.cursor_row = row;
        picker.cursor_col = col;
        true
    }

    pub fn wants_small_qr_scan(&self) -> bool {
        matches!(
            self.screen,
            Screen::LoadScanQr | Screen::VerifyBackupScan
        )
    }

    /// Returns true when the next `InputEvent::Confirm` will synchronously run
    /// PBKDF2 key derivation. Callers that can draw a frame before the blocking
    /// work should check this and show a "please wait" screen first.
    pub fn confirm_will_derive(&self) -> bool {
        match &self.screen {
            Screen::LoadPassphrasePrompt { selected: 0, .. }
            | Screen::CreatePassphrasePrompt { selected: 0, .. } => true,
            Screen::LoadPassphraseConfirm { passphrase, grid, .. }
            | Screen::CreatePassphraseConfirm { passphrase, grid, .. } => {
                grid.action_region() == Some(GridAction::Done) && grid.text.as_str() == passphrase.as_str()
            }
            Screen::VerifyBackupPassphrase { grid } => {
                grid.action_region() == Some(GridAction::Done)
            }
            _ => false,
        }
    }

    fn transition(&mut self, screen: Screen, event: InputEvent) -> Screen {
        if event == InputEvent::Back {
            if let (Some(topic), Some(disc)) = (self.help_return, self.help_return_for) {
                if disc == std::mem::discriminant(&screen) {
                    self.help_return = None;
                    self.help_return_for = None;
                    self.pending_screen = Some(screen);
                    return Screen::Help { topic };
                }
            }
        }
        if let Some(disc) = self.help_return_for {
            if disc != std::mem::discriminant(&screen) {
                self.help_return = None;
                self.help_return_for = None;
            }
        }

        match screen {
            Screen::Splash => Screen::ModeSelect {
                selected: 1,
                shown_at: std::time::Instant::now(),
            },

            Screen::ModeSelect { mut selected, .. } => {
                // Row 0 = GUIDED MODE (walk through help screens), row 1 =
                // EXPERT MODE (skip help, straight to main menu). `guided`
                // is true when the user picks GUIDED.
                match event {
                    InputEvent::Up => selected = 0,
                    InputEvent::Down => selected = 1,
                    InputEvent::Confirm => {
                        let sel = selected.clamp(0, 1);
                        self.guided = sel == 0;
                        return self.maybe_help(HelpTopic::Welcome, Screen::MainMenu { selected: 0 });
                    }
                    _ => {}
                }
                Screen::ModeSelect {
                    selected: selected.clamp(0, 1),
                    shown_at: std::time::Instant::now(),
                }
            }

            Screen::Help { topic } => {
                match event {
                    InputEvent::Confirm => {
                        let next = self.pending_screen.take().unwrap_or(Screen::MainMenu { selected: 0 });
                        self.help_return = Some(topic);
                        self.help_return_for = Some(std::mem::discriminant(&next));
                        next
                    }
                    InputEvent::Back => {
                        self.pending_screen = None;
                        self.help_return = None;
                        self.help_return_for = None;
                        if topic == HelpTopic::Welcome {
                            Screen::ModeSelect {
                                selected: 0,
                                shown_at: std::time::Instant::now(),
                            }
                        } else {
                            Screen::MainMenu { selected: 0 }
                        }
                    }
                    _ => screen,
                }
            }

            Screen::MainMenu { mut selected } => {
                let menu_len = self.menu_items().len();
                match event {
                    InputEvent::Up | InputEvent::Left => {
                        selected = selected.saturating_sub(1);
                    }
                    InputEvent::Down | InputEvent::Right => {
                        if selected + 1 < menu_len {
                            selected += 1;
                        }
                    }
                    InputEvent::Confirm => return self.menu_select(selected),
                    InputEvent::Back => {
                        return Screen::ModeSelect {
                            selected: 0,
                            shown_at: std::time::Instant::now(),
                        };
                    }
                    _ => {}
                }
                Screen::MainMenu { selected }
            }

            s @ (Screen::CreateWordCount { .. }
            | Screen::CreateMethod { .. }
            | Screen::CreateCameraEntropy { .. }
            | Screen::CreateCoinFlips { .. }
            | Screen::CreateDiceRolls { .. }
            | Screen::CreateBackupWarning { .. }
            | Screen::CreateShowWords { .. }
            | Screen::CreateVerify { .. }
            | Screen::CreatePassphrasePrompt { .. }
            | Screen::CreatePassphraseInput { .. }
            | Screen::CreatePassphraseConfirm { .. }
            | Screen::CreatePassphraseMismatch { .. }
            | Screen::CreateConfirm { .. }
            | Screen::ExportSeedWarning { .. }
            | Screen::ShowWordsWarning { .. }
            | Screen::ExportSeedQrMenu { .. }
            | Screen::ExportShowWords { .. }
            | Screen::ExportSeedQr { .. }
            | Screen::ExportSeedQrBlock { .. }) => flows::create::handle(self, s, event),

            s @ (Screen::LoadMethod { .. }
                | Screen::LoadScanQr
                | Screen::LoadInvalidMnemonic { .. }
                | Screen::LoadWordCount { .. }
                | Screen::LoadEnterWords { .. }
                | Screen::LoadWordCommitted { .. }
                | Screen::LoadFinalize { .. }
                | Screen::LoadPassphrasePrompt { .. }
                | Screen::LoadPassphraseInput { .. }
                | Screen::LoadPassphraseConfirm { .. }
                | Screen::LoadPassphraseMismatch { .. }
                | Screen::LoadConfirm { .. }) => flows::load::handle(self, s, event),

            Screen::DerivationError => Screen::MainMenu { selected: 0 },

            s @ (Screen::SignScanTx
            | Screen::SignReview { .. }
            | Screen::SignShowQr { .. }
            | Screen::SignMessageInput { .. }
            | Screen::SignMessageReview { .. }
            | Screen::SignMessageResult { .. }) => flows::sign::handle(self, s, event),

            s @ (Screen::SettingsMenu { .. }
            | Screen::SettingsShowAddress
            | Screen::SettingsShowAddressText
            | Screen::SettingsAbout
            | Screen::SettingsPowerOff { .. }) => flows::settings::handle(self, s, event),

            s @ (Screen::VerifyBackupScan
            | Screen::VerifyBackupSeedMismatch
            | Screen::VerifyBackupPassphrase { .. }
            | Screen::VerifyBackupPassphraseMismatch
            | Screen::VerifyBackupSuccess) => flows::verify::handle(self, s, event),
        }
    }

    /// Visible menu action indices.
    /// With wallet: SIGN(2), WALLET DATA(3), ABOUT(4).
    /// Without:     CREATE(0), LOAD(1), ABOUT(4).
    fn menu_items(&self) -> &'static [usize] {
        if self.wallet.is_some() {
            &[2, 3, 4]
        } else {
            &[0, 1, 4]
        }
    }

    fn menu_select(&mut self, selected: usize) -> Screen {
        let items = self.menu_items();
        let action = items.get(selected).copied().unwrap_or(usize::MAX);
        match action {
            0 => self.maybe_help(HelpTopic::CreateWallet, Screen::CreateWordCount { selected: 0 }),
            1 => self.maybe_help(HelpTopic::LoadWallet, Screen::LoadMethod { selected: 0 }),
            2 => self.maybe_help(HelpTopic::SignTx, Screen::SignScanTx),
            3 => self.maybe_help(HelpTopic::WalletData, Screen::SettingsMenu { selected: 0 }),
            4 => Screen::SettingsAbout,
            _ => Screen::MainMenu { selected },
        }
    }

    pub(crate) fn maybe_help(&mut self, topic: HelpTopic, next: Screen) -> Screen {
        if self.guided {
            self.pending_screen = Some(next);
            Screen::Help { topic }
        } else {
            next
        }
    }

    /// Visual index of a given action (0=CREATE, 1=LOAD, 2=SIGN,
    /// 3=WALLET DATA, 4=ABOUT) in the current menu layout.
    pub(crate) fn menu_index_of(&self, action: usize) -> usize {
        self.menu_items().iter().position(|&a| a == action).unwrap_or(0)
    }

    pub(crate) fn derive_keypair_and_address(&self, mnemonic: &str, passphrase: &str) -> Option<(SolanaKeypair, String)> {
        let keypair = derivation::derive_keypair(mnemonic, passphrase, 0)?;
        let address = derivation::address(&keypair);
        Some((keypair, address))
    }

    pub(crate) fn derive_address(&self, mnemonic: &str, passphrase: &str) -> Option<String> {
        self.derive_keypair_and_address(mnemonic, passphrase).map(|(_, addr)| addr)
    }

    pub(crate) fn set_wallet(&mut self, mnemonic: Zeroizing<String>, passphrase: Zeroizing<String>, keypair: SolanaKeypair) {
        let address = derivation::address(&keypair);
        self.wallet = Some(LoadedWallet { mnemonic, passphrase, keypair, address });
    }
}

/// Build the Review TX `info_lines` from a parsed tx. Returns the lines plus
/// `can_sign` — true only when the loaded wallet's pubkey is in the tx's
/// required-signer set. Lines starting with `!` render in danger red.
///
/// Delegates to the unified parser (`crate::parser`) which supports both
/// legacy and v0 transactions.
pub fn build_review_lines(
    tx_bytes: &[u8],
    wallet_pubkey: &[u8; 32],
) -> (Vec<String>, bool, crate::parser::ParsedTransaction) {
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
        slip0010::derive_solana_keypair(&seed, 0).unwrap()
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
        let (lines, can_sign, _parsed) = build_review_lines(&tx, &kp.public_key);
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
        let (lines, can_sign, _parsed) = build_review_lines(&tx, &kp.public_key);

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
        let (lines, can_sign, _parsed) = build_review_lines(&garbage, &kp.public_key);
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
        let (lines, can_sign, _parsed) = build_review_lines(&tx, &kp.public_key);
        assert!(
            !can_sign,
            "abandon-abandon wallet should not match EfZr signer"
        );
        assert!(lines.iter().any(|l| l.contains("0.01 SOL")));
        assert!(lines.iter().any(|l| l.contains("SOL Transfer")));
    }
}
