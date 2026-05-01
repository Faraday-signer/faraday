//! Verify backup flow — dry-run of restoring the wallet from paper + memory.
//!
//! The user scans the transcribed CompactSeedQR and optionally enters their
//! passphrase. Both are checked against the currently-loaded wallet:
//!   - decoded mnemonic must match `app.wallet.mnemonic`
//!   - derived address from (mnemonic + entered passphrase) must match `app.wallet.address`

use crate::gui::app::{App, CharGrid, InputEvent, Screen};
use crate::qr::decode_qr;

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::VerifyBackupScan => {
            match event {
                InputEvent::Confirm => {
                    // Only advance on a real scan.
                    let data = match app.scanned_qr.take() {
                        Some(d) if !d.is_empty() => d,
                        _ => return Screen::VerifyBackupScan,
                    };

                    let decoded = decode_qr::detect_and_decode(&data);
                    let scanned_mn = match decoded.mnemonic {
                        Some(m) => m,
                        None => return Screen::VerifyBackupSeedMismatch,
                    };

                    let wallet = match app.wallet.as_ref() {
                        Some(w) => w,
                        // No wallet loaded — nothing to verify against. Bail.
                        None => return Screen::MainMenu { selected: app.menu_index_of(2) },
                    };

                    if scanned_mn != wallet.mnemonic {
                        return Screen::VerifyBackupSeedMismatch;
                    }

                    if wallet.passphrase.is_empty() {
                        Screen::VerifyBackupSuccess
                    } else {
                        Screen::VerifyBackupPassphrase {
                            grid: CharGrid::new(),
                        }
                    }
                }
                InputEvent::Back => {
                    // Back = up one level to the Paper Backup menu, not
                    // cancel-to-MainMenu. User can re-select VERIFY from
                    // there, or navigate elsewhere in the backup flow.
                    let Some(wallet) = app.wallet.as_ref() else {
                        return Screen::MainMenu { selected: app.menu_index_of(2) };
                    };
                    let compact_data =
                        crate::qr::encode_qr::encode_compact_seed_qr(&wallet.mnemonic)
                            .unwrap_or_default();
                    Screen::ExportSeedQrMenu {
                        compact_data,
                        selected: 2,
                        from_settings: false,
                    }
                }
                _ => Screen::VerifyBackupScan,
            }
        }

        Screen::VerifyBackupSeedMismatch => match event {
            // K1 = redo paper backup — user goes back to the transcribe
            // walkthrough (block 1 of 9) so they can correct their paper.
            InputEvent::Confirm => {
                let Some(wallet) = app.wallet.as_ref() else {
                    return Screen::MainMenu { selected: app.menu_index_of(2) };
                };
                let compact_data = crate::qr::encode_qr::encode_compact_seed_qr(&wallet.mnemonic)
                    .unwrap_or_default();
                Screen::ExportSeedQrBlock {
                    compact_data,
                    block_index: 0,
                    from_settings: false,
                }
            }
            // K3 = back to the scan screen — user can re-scan without
            // redoing the paper (e.g. the camera missed it the first time).
            InputEvent::Back => Screen::VerifyBackupScan,
            _ => Screen::VerifyBackupSeedMismatch,
        },

        Screen::VerifyBackupPassphrase { mut grid } => {
            let done = grid.handle_input(event);
            if done {
                if grid.text.is_empty() && event == InputEvent::Back {
                    return Screen::MainMenu { selected: app.menu_index_of(2) };
                }
                let wallet = match app.wallet.as_ref() {
                    Some(w) => w,
                    None => return Screen::MainMenu { selected: app.menu_index_of(2) },
                };
                let derived = match app.derive_address(&wallet.mnemonic, &grid.text) {
                    Some(a) => a,
                    None => return Screen::DerivationError,
                };
                if derived == wallet.address {
                    Screen::VerifyBackupSuccess
                } else {
                    Screen::VerifyBackupPassphraseMismatch
                }
            } else {
                Screen::VerifyBackupPassphrase { grid }
            }
        }

        Screen::VerifyBackupPassphraseMismatch => match event {
            // K1 = retry passphrase entry (fresh grid).
            InputEvent::Confirm => Screen::VerifyBackupPassphrase {
                grid: CharGrid::new(),
            },
            // K3 = back to scan — user may want to re-scan the paper
            // before blaming the passphrase.
            InputEvent::Back => Screen::VerifyBackupScan,
            _ => Screen::VerifyBackupPassphraseMismatch,
        },

        Screen::VerifyBackupSuccess => match event {
            InputEvent::Confirm | InputEvent::Back => Screen::MainMenu { selected: app.menu_index_of(2) },
            _ => Screen::VerifyBackupSuccess,
        },

        _ => unreachable!("verify::handle called with non-verify screen"),
    }
}
