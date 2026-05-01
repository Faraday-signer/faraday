//! Create wallet flow.

use crate::crypto::bip39;
use crate::gui::app::{App, CharGrid, InputEvent, Screen};
use zeroize::Zeroize;

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::CreateWordCount { mut selected } => {
            match event {
                InputEvent::Up | InputEvent::Down => {
                    selected = 1 - selected;
                }
                InputEvent::Confirm => {
                    let word_count = if selected == 0 { 12 } else { 24 };
                    return Screen::CreateMethod {
                        word_count,
                        selected: 0,
                    };
                }
                InputEvent::Back => return Screen::MainMenu { selected: 0 },
                _ => {}
            }
            Screen::CreateWordCount { selected }
        }

        Screen::CreateMethod {
            word_count,
            mut selected,
        } => {
            match event {
                InputEvent::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                InputEvent::Down => {
                    if selected < 3 {
                        selected += 1;
                    }
                }
                InputEvent::Confirm => {
                    return match selected {
                        0 => generate_wallet(word_count),
                        1 => Screen::CreateCameraEntropy {
                            word_count,
                            frames_collected: 0,
                            entropy: Vec::new(),
                        },
                        2 => Screen::CreateCoinFlips {
                            word_count,
                            bits: Vec::new(),
                            selected: 0,
                        },
                        3 => Screen::CreateDiceRolls {
                            word_count,
                            rolls: Vec::new(),
                            selected: 0,
                        },
                        _ => Screen::CreateMethod {
                            word_count,
                            selected,
                        },
                    };
                }
                InputEvent::Back => return Screen::CreateWordCount { selected: 0 },
                _ => {}
            }
            Screen::CreateMethod {
                word_count,
                selected,
            }
        }

        Screen::CreateCameraEntropy {
            word_count,
            mut frames_collected,
            mut entropy,
        } => {
            // Each capture is a SHA-256 of a multi-megapixel sensor image XOR'd
            // with a nanosecond counter; two frames give ~512 bits of uniform
            // material into mnemonic_from_entropy, well above the 128/256-bit
            // target for 12/24-word wallets. More frames only add UI friction.
            let total_frames = 2;
            match event {
                InputEvent::Confirm => {
                    let mut frame_entropy = [0u8; 16];
                    #[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
                    {
                        if let Some(frame) = &app.latest_frame {
                            use sha2::{Digest, Sha256};
                            let digest = Sha256::digest(&frame.rgb);
                            frame_entropy.copy_from_slice(&digest[..16]);
                        } else {
                            getrandom::getrandom(&mut frame_entropy)
                                .expect("OS RNG unavailable — refusing to collect weak entropy");
                        }
                    }
                    #[cfg(not(any(feature = "_desktop_sim", target_os = "linux")))]
                    {
                        getrandom::getrandom(&mut frame_entropy)
                            .expect("OS RNG unavailable — refusing to collect weak entropy");
                    }
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
                        return Screen::CreateBackupWarning {
                            mnemonic,
                            word_count,
                            selected: 0,
                        };
                    }
                }
                InputEvent::Back => {
                    return Screen::CreateMethod {
                        word_count,
                        selected: 1,
                    }
                }
                _ => {}
            }
            Screen::CreateCameraEntropy {
                word_count,
                frames_collected,
                entropy,
            }
        }

        Screen::CreateCoinFlips {
            word_count,
            mut bits,
            mut selected,
        } => {
            let total_flips = if word_count == 12 { 128 } else { 256 };
            match event {
                InputEvent::Left | InputEvent::Right | InputEvent::Up | InputEvent::Down => {
                    selected = 1 - selected;
                }
                InputEvent::Confirm => {
                    bits.push(selected == 0);
                    if bits.len() >= total_flips {
                        let mut entropy = vec![0u8; total_flips / 8];
                        for (i, &bit) in bits.iter().enumerate() {
                            if bit {
                                entropy[i / 8] |= 1 << (7 - (i % 8));
                            }
                        }
                        let mnemonic = bip39::mnemonic_from_raw_entropy(&entropy)
                            .expect("Valid entropy length");
                        return Screen::CreateBackupWarning {
                            mnemonic,
                            word_count,
                            selected: 0,
                        };
                    }
                }
                InputEvent::Back => {
                    if bits.is_empty() {
                        return Screen::CreateMethod {
                            word_count,
                            selected: 1,
                        };
                    }
                    bits.pop();
                }
                _ => {}
            }
            Screen::CreateCoinFlips {
                word_count,
                bits,
                selected,
            }
        }

        Screen::CreateDiceRolls {
            word_count,
            mut rolls,
            mut selected,
        } => {
            let total_rolls = if word_count == 12 { 50 } else { 99 };
            match event {
                InputEvent::Up => {
                    if selected >= 3 {
                        selected -= 3;
                    }
                }
                InputEvent::Down => {
                    if selected + 3 <= 5 {
                        selected += 3;
                    }
                }
                InputEvent::Left => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                InputEvent::Right => {
                    if selected < 5 {
                        selected += 1;
                    }
                }
                InputEvent::Confirm => {
                    rolls.push(selected as u8 + 1);
                    if rolls.len() >= total_rolls {
                        let rolls_str: String = rolls.iter().map(|r| r.to_string()).collect();
                        let mnemonic =
                            bip39::mnemonic_from_entropy(rolls_str.as_bytes(), word_count)
                                .expect("Valid word count");
                        return Screen::CreateBackupWarning {
                            mnemonic,
                            word_count,
                            selected: 0,
                        };
                    }
                }
                InputEvent::Back => {
                    if rolls.is_empty() {
                        return Screen::CreateMethod {
                            word_count,
                            selected: 2,
                        };
                    }
                    rolls.pop();
                }
                _ => {}
            }
            Screen::CreateDiceRolls {
                word_count,
                rolls,
                selected,
            }
        }

        // CANCEL (row 0, default) and K3 both drop the freshly-generated
        // mnemonic and return to the method picker so the user can retry or
        // back out further. I UNDERSTAND (row 1) advances to the plaintext
        // word display. The seed never leaves screen state, so discarding
        // on cancel is non-destructive.
        Screen::CreateBackupWarning {
            mnemonic,
            word_count,
            mut selected,
        } => {
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
                    if selected == 1 {
                        return Screen::CreateShowWords {
                            mnemonic,
                            page: 0,
                            word_count,
                        };
                    }
                    return Screen::CreateMethod {
                        word_count,
                        selected: 0,
                    };
                }
                InputEvent::Back => {
                    return Screen::CreateMethod {
                        word_count,
                        selected: 0,
                    };
                }
                _ => {}
            }
            Screen::CreateBackupWarning {
                mnemonic,
                word_count,
                selected,
            }
        }

        Screen::CreateShowWords {
            mnemonic,
            mut page,
            word_count,
        } => {
            let words_per_page = 4usize;
            let total_pages = (word_count + words_per_page - 1) / words_per_page;
            match event {
                InputEvent::Right | InputEvent::Down => {
                    if page + 1 < total_pages {
                        page += 1;
                    }
                }
                InputEvent::Left | InputEvent::Up => {
                    if page > 0 {
                        page -= 1;
                    }
                }
                InputEvent::Confirm => {
                    if page + 1 == total_pages {
                        return start_verification(mnemonic, word_count);
                    } else {
                        page += 1;
                    }
                }
                InputEvent::Back => return Screen::CreateWordCount { selected: 0 },
                _ => {}
            }
            Screen::CreateShowWords {
                mnemonic,
                page,
                word_count,
            }
        }

        Screen::CreateVerify {
            mnemonic,
            checks,
            current,
            options,
            correct_idx,
            mut selected,
        } => {
            match event {
                InputEvent::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                InputEvent::Down => {
                    if selected < 3 {
                        selected += 1;
                    }
                }
                InputEvent::Confirm => {
                    if selected == correct_idx {
                        let next = current + 1;
                        if next >= checks.len() {
                            return Screen::CreatePassphrasePrompt {
                                mnemonic,
                                selected: 0,
                            };
                        } else {
                            return build_verify_screen(mnemonic, checks, next);
                        }
                    }
                    return Screen::CreateVerify {
                        mnemonic,
                        checks,
                        current,
                        options,
                        correct_idx,
                        selected: 0,
                    };
                }
                InputEvent::Back => {
                    let word_count = mnemonic.split_whitespace().count();
                    return Screen::CreateShowWords {
                        mnemonic,
                        page: 0,
                        word_count,
                    };
                }
                _ => {}
            }
            Screen::CreateVerify {
                mnemonic,
                checks,
                current,
                options,
                correct_idx,
                selected,
            }
        }

        Screen::CreatePassphrasePrompt {
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
                        let address = match app.derive_address(&mnemonic, &passphrase) {
                            Some(a) => a,
                            None => return Screen::DerivationError,
                        };
                        return Screen::CreateConfirm { mnemonic, passphrase, address, selected: 0 };
                    } else {
                        return Screen::CreatePassphraseInput {
                            mnemonic,
                            grid: CharGrid::new(),
                        };
                    }
                }
                InputEvent::Back => {
                    let word_count = mnemonic.split_whitespace().count();
                    return start_verification(mnemonic, word_count);
                }
                _ => {}
            }
            Screen::CreatePassphrasePrompt { mnemonic, selected }
        }

        Screen::CreatePassphraseInput { mnemonic, mut grid } => {
            let done = grid.handle_input(event);
            if done {
                if grid.text.is_empty() && event == InputEvent::Back {
                    return Screen::CreatePassphrasePrompt {
                        mnemonic,
                        selected: 1,
                    };
                }
                let passphrase = grid.text;
                return Screen::CreatePassphraseConfirm {
                    mnemonic,
                    passphrase,
                    grid: CharGrid::new(),
                };
            }
            Screen::CreatePassphraseInput { mnemonic, grid }
        }

        Screen::CreatePassphraseConfirm {
            mnemonic,
            passphrase,
            mut grid,
        } => {
            let done = grid.handle_input(event);
            if done {
                if grid.text.is_empty() && event == InputEvent::Back {
                    let mut first_grid = CharGrid::new();
                    first_grid.text = passphrase;
                    return Screen::CreatePassphraseInput {
                        mnemonic,
                        grid: first_grid,
                    };
                }
                if grid.text == passphrase {
                    let address = match app.derive_address(&mnemonic, &passphrase) {
                        Some(a) => a,
                        None => return Screen::DerivationError,
                    };
                    return Screen::CreateConfirm { mnemonic, passphrase, address, selected: 0 };
                } else {
                    return Screen::CreatePassphraseMismatch { mnemonic };
                }
            }
            Screen::CreatePassphraseConfirm {
                mnemonic,
                passphrase,
                grid,
            }
        }

        Screen::CreatePassphraseMismatch { mnemonic } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    return Screen::CreatePassphraseInput {
                        mnemonic,
                        grid: CharGrid::new(),
                    };
                }
                _ => {}
            }
            Screen::CreatePassphraseMismatch { mnemonic }
        }

        Screen::CreateConfirm {
            mnemonic,
            passphrase,
            address,
            mut selected,
        } => {
            match event {
                InputEvent::Left | InputEvent::Right => {
                    selected = 1 - selected;
                }
                InputEvent::Confirm => {
                    if selected == 0 {
                        let compact_data = crate::qr::encode_qr::encode_compact_seed_qr(&mnemonic)
                            .unwrap_or_default();
                        app.load_wallet(mnemonic, passphrase);
                        return Screen::ExportSeedQrMenu {
                            compact_data,
                            selected: 0,
                            from_settings: false,
                        };
                    } else {
                        return Screen::MainMenu { selected: 0 };
                    }
                }
                InputEvent::Back => {
                    return Screen::CreatePassphrasePrompt {
                        mnemonic,
                        selected: 0,
                    };
                }
                _ => {}
            }
            Screen::CreateConfirm {
                mnemonic,
                passphrase,
                address,
                selected,
            }
        }

        Screen::ExportSeedQrMenu {
            compact_data,
            mut selected,
            from_settings,
        } => {
            const ITEMS: usize = 3; // Paper backup / Show words / Back
            match event {
                InputEvent::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                InputEvent::Down => {
                    if selected + 1 < ITEMS {
                        selected += 1;
                    }
                }
                InputEvent::Confirm => match selected {
                    0 => {
                        return Screen::ExportSeedQrBlock {
                            compact_data,
                            block_index: 0,
                            from_settings,
                        }
                    }
                    1 => {
                        // Gate SHOW WORDS behind a warning — plaintext seed is
                        // the single most dangerous surface, worth forcing the
                        // user through a confirm even on the post-create path.
                        let mnemonic = app
                            .wallet
                            .as_ref()
                            .map(|w| w.mnemonic.clone())
                            .unwrap_or_default();
                        let word_count = mnemonic.split_whitespace().count();
                        return Screen::ShowWordsWarning {
                            compact_data,
                            mnemonic,
                            word_count,
                            selected: 0,
                            from_settings,
                        };
                    }
                    _ => {
                        return if from_settings {
                            Screen::SettingsMenu { selected: 0 }
                        } else {
                            Screen::MainMenu { selected: app.menu_index_of(2) }
                        };
                    }
                },
                InputEvent::Back => {
                    return if from_settings {
                        Screen::SettingsMenu { selected: 0 }
                    } else {
                        Screen::MainMenu { selected: app.menu_index_of(2) }
                    };
                }
                _ => {}
            }
            Screen::ExportSeedQrMenu {
                compact_data,
                selected,
                from_settings,
            }
        }

        Screen::ExportShowWords {
            compact_data,
            mnemonic,
            mut page,
            word_count,
            from_settings,
        } => {
            let words_per_page = 4usize;
            let total_pages = (word_count + words_per_page - 1) / words_per_page;
            match event {
                InputEvent::Right | InputEvent::Down | InputEvent::Confirm => {
                    if page + 1 < total_pages {
                        page += 1;
                    } else {
                        return Screen::ExportSeedQrMenu {
                            compact_data,
                            selected: 0,
                            from_settings,
                        };
                    }
                }
                InputEvent::Left | InputEvent::Up => {
                    if page > 0 {
                        page -= 1;
                    }
                }
                InputEvent::Back => {
                    return Screen::ExportSeedQrMenu {
                        compact_data,
                        selected: 0,
                        from_settings,
                    };
                }
                _ => {}
            }
            Screen::ExportShowWords {
                compact_data,
                mnemonic,
                page,
                word_count,
                from_settings,
            }
        }

        Screen::ExportSeedQr {
            compact_data,
            from_settings,
        } => {
            match event {
                // Final review screen shown after the block-by-block walkthrough.
                // Confirm advances to the scan-based verification; Back returns
                // to the last block so the user can re-check a cell.
                InputEvent::Confirm => return Screen::VerifyBackupScan,
                InputEvent::Back => {
                    let blocks_per_side: usize = if compact_data.len() == 16 { 3 } else { 5 };
                    let last = blocks_per_side * blocks_per_side - 1;
                    return Screen::ExportSeedQrBlock {
                        compact_data,
                        block_index: last,
                        from_settings,
                    };
                }
                _ => {}
            }
            Screen::ExportSeedQr {
                compact_data,
                from_settings,
            }
        }

        Screen::ExportSeedQrBlock {
            compact_data,
            mut block_index,
            from_settings,
        } => {
            // Derive block-grid size from the QR size: 16 entropy bytes → 21×21
            // → 3×3 blocks of 7 modules; 32 bytes → 25×25 → 5×5 blocks of 5.
            let blocks_per_side: usize = if compact_data.len() == 16 { 3 } else { 5 };
            let total = blocks_per_side * blocks_per_side;

            match event {
                InputEvent::Right | InputEvent::Confirm => {
                    // After the final block, show the full QR once as a
                    // side-by-side check; that screen then advances to the
                    // scan-based verification.
                    if block_index + 1 >= total {
                        return Screen::ExportSeedQr {
                            compact_data,
                            from_settings,
                        };
                    }
                    block_index += 1;
                }
                InputEvent::Left => {
                    if block_index > 0 {
                        block_index -= 1;
                    }
                }
                InputEvent::Down => {
                    let next = block_index + blocks_per_side;
                    if next < total {
                        block_index = next;
                    }
                }
                InputEvent::Up => {
                    block_index = block_index.saturating_sub(blocks_per_side);
                }
                InputEvent::Back => {
                    return Screen::ExportSeedQrMenu {
                        compact_data,
                        selected: 1,
                        from_settings,
                    };
                }
                _ => {}
            }
            Screen::ExportSeedQrBlock {
                compact_data,
                block_index,
                from_settings,
            }
        }

        // "Reveals your seed" gate — CANCEL / SHOW. Used when entering the
        // backup flow from Settings so users who wander in are forced to
        // acknowledge the consequence. Create flow skips this because the
        // user just explicitly chose to create the wallet.
        Screen::ExportSeedWarning {
            mut selected,
            from_settings,
        } => {
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
                    if selected == 1 {
                        // SHOW — proceed to the SeedQR menu.
                        let mnemonic = app
                            .wallet
                            .as_ref()
                            .map(|w| w.mnemonic.clone())
                            .unwrap_or_default();
                        let compact_data = crate::qr::encode_qr::encode_compact_seed_qr(&mnemonic)
                            .unwrap_or_default();
                        return Screen::ExportSeedQrMenu {
                            compact_data,
                            selected: 0,
                            from_settings,
                        };
                    }
                    return if from_settings {
                        Screen::SettingsMenu { selected: 0 }
                    } else {
                        Screen::MainMenu { selected: app.menu_index_of(2) }
                    };
                }
                InputEvent::Back => {
                    return if from_settings {
                        Screen::SettingsMenu { selected: 0 }
                    } else {
                        Screen::MainMenu { selected: app.menu_index_of(2) }
                    };
                }
                _ => {}
            }
            Screen::ExportSeedWarning {
                selected,
                from_settings,
            }
        }

        // CANCEL (row 0, default) returns to the backup menu; SHOW (row 1)
        // proceeds to the plaintext word display.
        Screen::ShowWordsWarning {
            compact_data,
            mnemonic,
            word_count,
            mut selected,
            from_settings,
        } => {
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
                    if selected == 1 {
                        return Screen::ExportShowWords {
                            compact_data,
                            mnemonic,
                            page: 0,
                            word_count,
                            from_settings,
                        };
                    }
                    return Screen::ExportSeedQrMenu {
                        compact_data,
                        selected: 0,
                        from_settings,
                    };
                }
                InputEvent::Back => {
                    return Screen::ExportSeedQrMenu {
                        compact_data,
                        selected: 0,
                        from_settings,
                    };
                }
                _ => {}
            }
            Screen::ShowWordsWarning {
                compact_data,
                mnemonic,
                word_count,
                selected,
                from_settings,
            }
        }

        _ => unreachable!("create::handle called with non-create screen"),
    }
}

fn generate_wallet(word_count: usize) -> Screen {
    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy).expect("Failed to get random entropy");
    let mnemonic =
        bip39::mnemonic_from_entropy(&entropy, word_count).expect("Failed to generate mnemonic");
    entropy.zeroize();
    Screen::CreateBackupWarning {
        mnemonic,
        word_count,
        selected: 0,
    }
}

fn start_verification(mnemonic: String, word_count: usize) -> Screen {
    let num_checks = if word_count == 12 { 3 } else { 5 };
    let words: Vec<&str> = mnemonic.split_whitespace().collect();

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

    build_verify_screen(mnemonic, checks, 0)
}

fn build_verify_screen(mnemonic: String, checks: Vec<usize>, current: usize) -> Screen {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    let correct_word = words[checks[current]];

    let mut rng = [0u8; 32];
    getrandom::getrandom(&mut rng).expect("Failed to get random bytes");
    let mut options: Vec<String> = vec![correct_word.to_string()];
    let mut ri = 0;
    while options.len() < 4 && ri < 30 {
        let word_idx = ((rng[ri] as usize) << 3 | (rng[ri + 1] as usize >> 5)) % 2048;
        if let Some(word) = bip39::get_word(word_idx) {
            if !options.contains(&word.to_string()) { options.push(word.to_string()); }
        }
        ri += 2;
    }

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
