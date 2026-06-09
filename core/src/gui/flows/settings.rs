//! Settings / wallet data flow.

use crate::gui::app::{App, HelpTopic, InputEvent, Screen};

const WALLET_DATA_ITEMS: usize = 4;

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::SettingsMenu { mut selected } => {
            match event {
                InputEvent::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                InputEvent::Down => {
                    if selected + 1 < WALLET_DATA_ITEMS {
                        selected += 1;
                    }
                }
                InputEvent::Confirm => {
                    return match selected {
                        0 => Screen::SettingsShowAddressText,
                        1 => Screen::SettingsShowAddress,
                        2 => {
                            if app.guided {
                                let compact_data = app.wallet.as_ref()
                                    .and_then(|w| crate::qr::encode_qr::encode_compact_seed_qr(&w.mnemonic).ok())
                                    .unwrap_or_default();
                                let next = Screen::ExportSeedQrMenu {
                                    compact_data,
                                    selected: 0,
                                    from_settings: true,
                                };
                                app.maybe_help(HelpTopic::BackupSeed, next)
                            } else {
                                Screen::ExportSeedWarning {
                                    selected: 0,
                                    from_settings: true,
                                }
                            }
                        }
                        3 => Screen::SettingsPowerOff { selected: 0 },
                        _ => Screen::SettingsMenu { selected },
                    };
                }
                InputEvent::Back => {
                    let idx = app.menu_index_of(3);
                    return Screen::MainMenu { selected: idx };
                }
                _ => {}
            }
            Screen::SettingsMenu { selected }
        }

        Screen::SettingsShowAddress => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    return Screen::SettingsMenu { selected: 1 }
                }
                _ => {}
            }
            Screen::SettingsShowAddress
        }

        Screen::SettingsShowAddressText => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    return Screen::SettingsMenu { selected: 0 }
                }
                _ => {}
            }
            Screen::SettingsShowAddressText
        }

        Screen::SettingsAbout => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    let idx = app.menu_index_of(4);
                    return Screen::MainMenu { selected: idx };
                }
                _ => {}
            }
            Screen::SettingsAbout
        }

        // Reset-wallet confirm. Row 0 = NO (safe default), Row 1 = YES
        // (destructive — wipes the in-memory wallet). Reachable from the
        // wallet-data menu and from the long-press Back kill-switch.
        Screen::SettingsPowerOff { mut selected } => {
            match event {
                InputEvent::Up => {
                    selected = 0;
                }
                InputEvent::Down => {
                    selected = 1;
                }
                InputEvent::Confirm => {
                    if selected == 1 {
                        app.wallet = None;
                        return Screen::MainMenu { selected: 0 };
                    }
                    return Screen::SettingsMenu { selected: 3 };
                }
                InputEvent::Back => {
                    return Screen::SettingsMenu { selected: 3 };
                }
                _ => {}
            }
            Screen::SettingsPowerOff { selected }
        }

        _ => screen,
    }
}
