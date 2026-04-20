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
                        None => return Screen::MainMenu { selected: 0 },
                    };

                    if scanned_mn != wallet.mnemonic {
                        return Screen::VerifyBackupSeedMismatch;
                    }

                    if wallet.passphrase.is_empty() {
                        Screen::VerifyBackupSuccess
                    } else {
                        Screen::VerifyBackupPassphrase { grid: CharGrid::new() }
                    }
                }
                InputEvent::Back => Screen::MainMenu { selected: 0 },
                _ => Screen::VerifyBackupScan,
            }
        }

        Screen::VerifyBackupSeedMismatch => match event {
            InputEvent::Confirm | InputEvent::Back => Screen::MainMenu { selected: 0 },
            _ => Screen::VerifyBackupSeedMismatch,
        },

        Screen::VerifyBackupPassphrase { mut grid } => {
            let done = grid.handle_input(event);
            if done {
                if grid.text.is_empty() && event == InputEvent::Back {
                    return Screen::MainMenu { selected: 0 };
                }
                let wallet = match app.wallet.as_ref() {
                    Some(w) => w,
                    None => return Screen::MainMenu { selected: 0 },
                };
                let derived = app.derive_address(&wallet.mnemonic, &grid.text);
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
            InputEvent::Confirm => Screen::VerifyBackupPassphrase { grid: CharGrid::new() },
            InputEvent::Back => Screen::MainMenu { selected: 0 },
            _ => Screen::VerifyBackupPassphraseMismatch,
        },

        Screen::VerifyBackupSuccess => match event {
            InputEvent::Confirm | InputEvent::Back => Screen::MainMenu { selected: 0 },
            _ => Screen::VerifyBackupSuccess,
        },

        _ => unreachable!("verify::handle called with non-verify screen"),
    }
}
