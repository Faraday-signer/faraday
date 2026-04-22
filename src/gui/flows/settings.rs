//! Settings flow.

use crate::crypto::derivation;
use crate::gui::app::{App, InputEvent, Screen};

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::SettingsMenu { mut selected } => {
            let item_count = if app.wallet.is_some() { 6 } else { 2 };
            match event {
                InputEvent::Up => { if selected > 0 { selected -= 1; } }
                InputEvent::Down => { if selected + 1 < item_count { selected += 1; } }
                InputEvent::Confirm => {
                    if app.wallet.is_some() {
                        return match selected {
                            0 => Screen::SettingsShowAddress,
                            1 => Screen::ExportSeedWarning {
                                selected: 0,
                                from_settings: true,
                            },
                            2 => {
                                let accounts = build_accounts_list(app);
                                Screen::SettingsAccounts { accounts, selected: 0 }
                            }
                            3 => Screen::SettingsVerifyAddressScan,
                            4 => Screen::SettingsAbout,
                            5 => Screen::SettingsPowerOff { selected: 0 },
                            _ => Screen::SettingsMenu { selected },
                        };
                    } else {
                        return match selected {
                            0 => Screen::SettingsAbout,
                            1 => Screen::SettingsPowerOff { selected: 0 },
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
                InputEvent::Confirm | InputEvent::Back => return Screen::SettingsMenu { selected: 0 },
                _ => {}
            }
            Screen::SettingsShowAddress
        }

        Screen::SettingsAccounts { accounts, mut selected } => {
            match event {
                InputEvent::Up => { if selected > 0 { selected -= 1; } }
                InputEvent::Down => { if selected + 1 < accounts.len() { selected += 1; } }
                InputEvent::Confirm | InputEvent::Back => return Screen::SettingsMenu { selected: 2 },
                _ => {}
            }
            Screen::SettingsAccounts { accounts, selected }
        }

        Screen::SettingsVerifyAddressScan => {
            match event {
                InputEvent::Confirm => {
                    let wallet = match &app.wallet {
                        Some(w) => w,
                        None => return Screen::SettingsMenu { selected: 3 },
                    };
                    // Only advance on a real scan. The dev fallback used to
                    // auto-succeed by using the wallet's own address.
                    let raw: String = match app
                        .scanned_qr
                        .take()
                        .and_then(|b| String::from_utf8(b).ok())
                    {
                        Some(s) => s,
                        None => return Screen::SettingsVerifyAddressScan,
                    };

                    let addr = derivation::normalize_address_input(&raw);
                    let result = derivation::verify_address(
                        &wallet.mnemonic,
                        &wallet.passphrase,
                        &addr,
                        10,
                    );
                    let display_addr = if matches!(result, derivation::AddressMatch::InvalidFormat) {
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

        Screen::SettingsAbout => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    let idx = if app.wallet.is_some() { 4 } else { 0 };
                    return Screen::SettingsMenu { selected: idx };
                }
                _ => {}
            }
            Screen::SettingsAbout
        }

        // Row 0 = NO (safe default), Row 1 = YES (destructive).
        Screen::SettingsPowerOff { mut selected } => {
            match event {
                InputEvent::Up => { selected = 0; }
                InputEvent::Down => { selected = 1; }
                InputEvent::Confirm => {
                    if selected == 1 {
                        // YES — wipe the wallet from RAM and power off.
                        app.wallet = None;
                        #[cfg(target_os = "linux")]
                        {
                            let _ = std::process::Command::new("poweroff").status();
                        }
                        return Screen::Splash;
                    }
                    let idx = if app.wallet.is_some() { 5 } else { 1 };
                    return Screen::SettingsMenu { selected: idx };
                }
                InputEvent::Back => {
                    let idx = if app.wallet.is_some() { 5 } else { 1 };
                    return Screen::SettingsMenu { selected: idx };
                }
                _ => {}
            }
            Screen::SettingsPowerOff { selected }
        }

        _ => unreachable!("settings::handle called with non-settings screen"),
    }
}

fn build_accounts_list(app: &App) -> Vec<(String, String)> {
    let wallet = match &app.wallet {
        Some(w) => w,
        None => return Vec::new(),
    };

    let accounts = derivation::derive_multiple_accounts(&wallet.mnemonic, &wallet.passphrase, 3);
    accounts.iter()
        .map(|kp| (kp.derivation_path.clone(), derivation::address(kp)))
        .collect()
}
