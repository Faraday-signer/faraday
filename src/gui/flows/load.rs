//! Load wallet flow.

use crate::crypto::bip39;
use crate::gui::app::{App, CharGrid, InputEvent, Screen, WordPicker};
use crate::qr::decode_qr;

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
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
                        let address = app.derive_address(&mnemonic, &passphrase);
                        return Screen::LoadConfirm { mnemonic, passphrase, address, selected: 0 };
                    } else {
                        return Screen::LoadPassphraseInput { mnemonic, grid: CharGrid::new() };
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
                return Screen::LoadPassphraseConfirm { mnemonic, passphrase, grid: CharGrid::new() };
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
                    let address = app.derive_address(&mnemonic, &passphrase);
                    return Screen::LoadConfirm { mnemonic, passphrase, address, selected: 0 };
                } else {
                    return Screen::LoadPassphraseMismatch { mnemonic };
                }
            }
            Screen::LoadPassphraseConfirm { mnemonic, passphrase, grid }
        }

        Screen::LoadPassphraseMismatch { mnemonic } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    return Screen::LoadPassphraseInput { mnemonic, grid: CharGrid::new() };
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
                        app.load_wallet(mnemonic, passphrase);
                    }
                    return Screen::MainMenu { selected: 0 };
                }
                InputEvent::Back => {
                    return Screen::LoadPassphrasePrompt { mnemonic, selected: 0 };
                }
                _ => {}
            }
            Screen::LoadConfirm { mnemonic, passphrase, address, selected }
        }

        _ => unreachable!("load::handle called with non-load screen"),
    }
}
