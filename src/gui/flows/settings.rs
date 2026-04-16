//! Settings flow.

use crate::crypto::derivation;
use crate::gui::app::{App, InputEvent, Screen};

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::SettingsMenu { mut selected } => {
            let item_count = if app.wallet.is_some() { 5 } else { 2 };
            match event {
                InputEvent::Up => { if selected > 0 { selected -= 1; } }
                InputEvent::Down => { if selected + 1 < item_count { selected += 1; } }
                InputEvent::Confirm => {
                    if app.wallet.is_some() {
                        return match selected {
                            0 => Screen::SettingsShowAddress,
                            1 => {
                                let mnemonic = &app.wallet.as_ref().unwrap().mnemonic;
                                let seed_qr_data = crate::qr::encode_qr::encode_seed_qr(mnemonic)
                                    .unwrap_or_default();
                                let compact_data = crate::qr::encode_qr::encode_compact_seed_qr(mnemonic)
                                    .unwrap_or_default();
                                Screen::ExportSeedQr {
                                    seed_qr_data, compact_data, compact_mode: false, from_settings: true,
                                }
                            }
                            2 => {
                                let accounts = build_accounts_list(app);
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

        Screen::SettingsAbout => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    let idx = if app.wallet.is_some() { 3 } else { 0 };
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
                        app.wallet = None;
                        #[cfg(target_os = "linux")]
                        {
                            let _ = std::process::Command::new("poweroff").status();
                        }
                        return Screen::Splash;
                    }
                    let idx = if app.wallet.is_some() { 4 } else { 1 };
                    return Screen::SettingsMenu { selected: idx };
                }
                InputEvent::Back => {
                    let idx = if app.wallet.is_some() { 4 } else { 1 };
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
    let mut list: Vec<(String, String)> = accounts.iter()
        .map(|kp| (kp.derivation_path.clone(), derivation::address(kp)))
        .collect();

    let cli_kp = derivation::derive_keypair_cli_path(&wallet.mnemonic, &wallet.passphrase);
    list.push((cli_kp.derivation_path.clone(), derivation::address(&cli_kp)));

    list
}
