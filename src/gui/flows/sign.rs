//! Sign transaction flow.

use crate::gui::app::{App, CharGrid, InputEvent, Screen, build_review_lines};
use crate::qr::decode_qr;

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::SignNoWallet => {
            match event {
                InputEvent::Confirm | InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignNoWallet
        }

        Screen::SignScanTx => {
            match event {
                InputEvent::Confirm => {
                    if let Some(wallet) = &app.wallet {
                        #[cfg(any(feature = "simulator", target_os = "linux"))]
                        let tx_base64: String = app.scanned_qr.take()
                            .and_then(|b| String::from_utf8(b).ok())
                            .unwrap_or_else(|| build_test_transaction(&wallet.keypair.public_key));
                        #[cfg(not(any(feature = "simulator", target_os = "linux")))]
                        let tx_base64: String = build_test_transaction(&wallet.keypair.public_key);
                        let decoded = decode_qr::detect_and_decode(tx_base64.as_bytes());
                        if let Some(tx_bytes) = decoded.tx_bytes {
                            let (info_lines, can_sign) =
                                build_review_lines(&tx_bytes, &wallet.keypair.public_key);
                            return Screen::SignReview {
                                tx_bytes,
                                tx_base64,
                                info_lines,
                                scroll: 0,
                                selected: if can_sign { 0 } else { 1 },
                                can_sign,
                            };
                        }
                    }
                }
                InputEvent::Secondary => return Screen::SignMessageInput { grid: CharGrid::new() },
                InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignScanTx
        }

        Screen::SignReview { tx_bytes, tx_base64, info_lines, mut scroll, mut selected, can_sign } => {
            match event {
                InputEvent::Up => { if scroll > 0 { scroll -= 1; } }
                InputEvent::Down => { if scroll + 8 < info_lines.len() { scroll += 1; } }
                InputEvent::Left | InputEvent::Right => {
                    if can_sign {
                        selected = 1 - selected;
                    } else {
                        selected = 1;
                    }
                }
                InputEvent::Confirm => {
                    if selected == 0 && can_sign {
                        if let Some(wallet) = &app.wallet {
                            if let Ok(signed) = crate::signer::sign_transaction_base64(
                                &tx_base64,
                                &wallet.keypair.private_key,
                                &wallet.keypair.public_key,
                            ) {
                                #[cfg(feature = "simulator")]
                                {
                                    println!("Signed by: {}", signed.signer_pubkey);
                                    println!("Signature: {}...", hex::encode(&signed.signature[..16]));
                                    println!("TX size: {} bytes", signed.signed_bytes.len());
                                }
                                return Screen::SignShowQr { data: signed.signed_base64 };
                            }
                        }
                    } else if selected == 0 && !can_sign {
                        return Screen::SignReview { tx_bytes, tx_base64, info_lines, scroll, selected: 1, can_sign };
                    }
                    return Screen::MainMenu { selected: 2 };
                }
                InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignReview { tx_bytes, tx_base64, info_lines, scroll, selected, can_sign }
        }

        Screen::SignShowQr { data } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => return Screen::MainMenu { selected: 2 },
                InputEvent::Secondary => return Screen::SignScanTx,
                _ => {}
            }
            Screen::SignShowQr { data }
        }

        Screen::SignMessageInput { mut grid } => {
            let done = grid.handle_input(event);
            if done {
                if grid.text.is_empty() && event == InputEvent::Back {
                    return Screen::SignScanTx;
                }
                if let Some(wallet) = &app.wallet {
                    let sig = crate::signer::sign_message(
                        grid.text.as_bytes(),
                        &wallet.keypair.private_key,
                    );
                    let signature_hex = hex::encode(&sig);
                    return Screen::SignMessageResult { signature_hex };
                }
            }
            Screen::SignMessageInput { grid }
        }

        Screen::SignMessageResult { signature_hex } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignMessageResult { signature_hex }
        }

        _ => unreachable!("sign::handle called with non-sign screen"),
    }
}

/// Build a minimal valid Solana transaction for simulator testing.
fn build_test_transaction(signer_pubkey: &[u8; 32]) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;

    let mut tx = Vec::new();
    tx.push(1u8);
    tx.extend_from_slice(&[0u8; 64]);
    tx.push(1); // num_required_sigs
    tx.push(0); // num_readonly_signed
    tx.push(1); // num_readonly_unsigned
    tx.push(2); // num_account_keys (compact-u16)
    tx.extend_from_slice(signer_pubkey);
    tx.extend_from_slice(&[0u8; 32]); // system program
    tx.extend_from_slice(&[0xAB; 32]); // recent blockhash (fake)
    tx.push(1); // 1 instruction
    tx.push(1); // program_id_index
    tx.push(1); // num accounts in instruction
    tx.push(0); // account index
    tx.push(0); // data length

    BASE64.encode(&tx)
}
