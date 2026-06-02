//! Load wallet flow.

use crate::gui::app::{App, CharGrid, HelpTopic, InputEvent, Screen, WordPicker};
use crate::qr::decode_qr;

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::LoadMethod { mut selected } => {
            match event {
                InputEvent::Up | InputEvent::Down => {
                    selected = 1 - selected;
                }
                InputEvent::Confirm => {
                    if selected == 0 {
                        return app.maybe_help(HelpTopic::ScanSeedQr, Screen::LoadScanQr);
                    } else {
                        return app.maybe_help(HelpTopic::TypeWords, Screen::LoadWordCount { selected: 0 });
                    }
                }
                InputEvent::Back => return Screen::MainMenu { selected: app.menu_index_of(1) },
                _ => {}
            }
            Screen::LoadMethod { selected }
        }

        Screen::LoadScanQr => {
            match event {
                InputEvent::Confirm => {
                    // Only advance when a camera frame actually decoded a QR.
                    // Previously this fell back to a canonical test mnemonic
                    // so sim users could press Enter without a real QR, but
                    // that made the device appear to accept a scan that never
                    // happened. Stay put instead.
                    let data = match app.scanned_qr.take() {
                        Some(d) => d,
                        None => return Screen::LoadScanQr,
                    };
                    let decoded = decode_qr::detect_and_decode(&data);

                    #[cfg(feature = "_desktop_sim")]
                    println!(
                        "QR decoded: {:?} ({} bytes raw)",
                        decoded.qr_type,
                        decoded.raw_data.len()
                    );

                    if let Some(mnemonic) = decoded.mnemonic {
                        let (keypair, preview_address) = match app.derive_keypair_and_address(&mnemonic, "") {
                            Some(pair) => pair,
                            None => return Screen::DerivationError,
                        };
                        return Screen::LoadFinalize {
                            mnemonic,
                            preview_address,
                            keypair,
                            selected: 0,
                        };
                    }
                    if let Some(_addr) = &decoded.address {
                        #[cfg(feature = "_desktop_sim")]
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
                InputEvent::Up | InputEvent::Down => {
                    selected = 1 - selected;
                }
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

        Screen::LoadEnterWords {
            mut words,
            word_count,
            mut picker,
        } => {
            // K3 (Back) — always navigates word-level: discards any partial
            // prefix and either pops the previous word or exits to the word
            // count picker. K2 (Secondary) is the in-word delete.
            if event == InputEvent::Back {
                if !picker.prefix.is_empty() {
                    picker.prefix.clear();
                    picker.cursor_row = 0;
                    picker.cursor_col = 0;
                    picker.snap_to_valid();
                    return Screen::LoadEnterWords {
                        words,
                        word_count,
                        picker,
                    };
                }
                if words.is_empty() {
                    return Screen::LoadWordCount { selected: 0 };
                }
                words.pop();
                picker.word_index = words.len();
                picker.words = words.clone();
                return Screen::LoadEnterWords {
                    words,
                    word_count,
                    picker,
                };
            }
            if let Some(word) = picker.handle_input(event) {
                // Auto-commit fired. Show the just-typed word as a brief
                // flash; the tick handler in App::tick handles validation
                // and routing once the flash window closes.
                return Screen::LoadWordCommitted {
                    just_committed: word,
                    picker,
                    word_count,
                    shown_at: std::time::Instant::now(),
                };
            }
            Screen::LoadEnterWords {
                words,
                word_count,
                picker,
            }
        }

        // Transient flash — the screen ignores all input. Tick auto-advances
        // after ~900ms.
        Screen::LoadWordCommitted {
            just_committed,
            picker,
            word_count,
            shown_at,
        } => Screen::LoadWordCommitted {
            just_committed,
            picker,
            word_count,
            shown_at,
        },

        Screen::LoadInvalidMnemonic { word_count } => {
            match event {
                InputEvent::Confirm => {
                    return Screen::LoadEnterWords {
                        words: Vec::new(),
                        word_count,
                        picker: WordPicker::new(word_count),
                    };
                }
                InputEvent::Back => return Screen::LoadMethod { selected: 1 },
                _ => {}
            }
            Screen::LoadInvalidMnemonic { word_count }
        }

        Screen::LoadFinalize { mnemonic, preview_address, keypair, mut selected } => {
            match event {
                InputEvent::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                InputEvent::Down => {
                    if selected < 1 {
                        selected += 1;
                    }
                }
                InputEvent::Confirm => {
                    if selected == 0 {
                        // DONE — load wallet with no passphrase and go home.
                        app.set_wallet(mnemonic, String::new(), keypair);
                        return Screen::MainMenu { selected: app.menu_index_of(2) };
                    }
                    // ADD PASSPHRASE — jump straight into the char grid.
                    return Screen::LoadPassphraseInput {
                        mnemonic,
                        grid: CharGrid::new(),
                    };
                }
                InputEvent::Back => return Screen::LoadMethod { selected: 0 },
                _ => {}
            }
            Screen::LoadFinalize {
                mnemonic,
                preview_address,
                keypair,
                selected,
            }
        }

        Screen::LoadPassphrasePrompt {
            mnemonic,
            mut selected,
        } => {
            match event {
                InputEvent::Up | InputEvent::Down => {
                    selected = 1 - selected;
                }
                InputEvent::Confirm => {
                    if selected == 0 {
                        let passphrase = String::new();
                        let (keypair, address) = match app.derive_keypair_and_address(&mnemonic, &passphrase) {
                            Some(pair) => pair,
                            None => return Screen::DerivationError,
                        };
                        return Screen::LoadConfirm { mnemonic, passphrase, keypair, address, selected: 0 };
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
                    return Screen::LoadPassphrasePrompt {
                        mnemonic,
                        selected: 1,
                    };
                }
                let passphrase = grid.text;
                return Screen::LoadPassphraseConfirm {
                    mnemonic,
                    passphrase,
                    grid: CharGrid::new(),
                };
            }
            Screen::LoadPassphraseInput { mnemonic, grid }
        }

        Screen::LoadPassphraseConfirm {
            mnemonic,
            passphrase,
            mut grid,
        } => {
            let done = grid.handle_input(event);
            if done {
                if grid.text.is_empty() && event == InputEvent::Back {
                    let mut first_grid = CharGrid::new();
                    first_grid.text = passphrase;
                    return Screen::LoadPassphraseInput {
                        mnemonic,
                        grid: first_grid,
                    };
                }
                if grid.text == passphrase {
                    let (keypair, address) = match app.derive_keypair_and_address(&mnemonic, &passphrase) {
                        Some(pair) => pair,
                        None => return Screen::DerivationError,
                    };
                    return Screen::LoadConfirm { mnemonic, passphrase, keypair, address, selected: 0 };
                } else {
                    return Screen::LoadPassphraseMismatch { mnemonic };
                }
            }
            Screen::LoadPassphraseConfirm {
                mnemonic,
                passphrase,
                grid,
            }
        }

        Screen::LoadPassphraseMismatch { mnemonic } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    return Screen::LoadPassphraseInput {
                        mnemonic,
                        grid: CharGrid::new(),
                    };
                }
                _ => {}
            }
            Screen::LoadPassphraseMismatch { mnemonic }
        }

        Screen::LoadConfirm {
            mnemonic,
            passphrase,
            keypair,
            address,
            mut selected,
        } => {
            match event {
                InputEvent::Left | InputEvent::Right => {
                    selected = 1 - selected;
                }
                InputEvent::Confirm => {
                    if selected == 0 {
                        app.set_wallet(mnemonic, passphrase, keypair);
                        return Screen::MainMenu { selected: app.menu_index_of(2) };
                    }
                    return Screen::MainMenu { selected: 0 };
                }
                InputEvent::Back => {
                    return Screen::LoadPassphrasePrompt {
                        mnemonic,
                        selected: 0,
                    };
                }
                _ => {}
            }
            Screen::LoadConfirm {
                mnemonic,
                passphrase,
                keypair,
                address,
                selected,
            }
        }

        _ => unreachable!("load::handle called with non-load screen"),
    }
}
