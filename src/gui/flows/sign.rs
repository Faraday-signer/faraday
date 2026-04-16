//! Sign transaction flow.

use crate::gui::app::{App, CharGrid, InputEvent, Screen};
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
                        let test_tx = build_test_transaction(&wallet.keypair.public_key);
                        let tx_base64 = test_tx.clone();
                        let decoded = decode_qr::detect_and_decode(test_tx.as_bytes());
                        if let Some(tx_bytes) = decoded.tx_bytes {
                            let info_lines = vec![
                                format!("From: {}...{}", &wallet.address[..4], &wallet.address[wallet.address.len()-4..]),
                                "To: 11111...1111".to_string(),
                                "Amount: 1.0 SOL".to_string(),
                                format!("Type: {}", match decoded.qr_type {
                                    decode_qr::QrType::SolanaTxBase64 => "Transaction",
                                    _ => "Unknown",
                                }),
                                format!("Size: {} bytes", tx_bytes.len()),
                            ];
                            return Screen::SignReview {
                                tx_bytes, tx_base64, info_lines, scroll: 0, selected: 0,
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

        Screen::SignReview { tx_bytes, tx_base64, info_lines, mut scroll, mut selected } => {
            match event {
                InputEvent::Up => { if scroll > 0 { scroll -= 1; } }
                InputEvent::Down => { if scroll + 8 < info_lines.len() { scroll += 1; } }
                InputEvent::Left | InputEvent::Right => { selected = 1 - selected; }
                InputEvent::Confirm => {
                    if selected == 0 {
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
                    }
                    return Screen::MainMenu { selected: 2 };
                }
                InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignReview { tx_bytes, tx_base64, info_lines, scroll, selected }
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
