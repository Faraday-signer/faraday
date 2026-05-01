//! Sign transaction flow.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::gui::app::{build_review_lines, App, CharGrid, InputEvent, Screen};
use crate::qr::decode_qr;

pub fn handle(app: &mut App, screen: Screen, event: InputEvent) -> Screen {
    match screen {
        Screen::SignScanTx => {
            match event {
                InputEvent::Confirm => {
                    if let Some(wallet) = &app.wallet {
                        let data = match app.scanned_qr.take() {
                            Some(d) if !d.is_empty() => d,
                            _ => return Screen::SignScanTx,
                        };

                        // Two valid QR shapes land here:
                        //   1. Base64-encoded tx text (single static QR)       — detect_and_decode routes it.
                        //   2. Raw tx bytes reassembled from a UR animated QR  — falls through as binary.
                        // Historically this handler gated on `String::from_utf8` and silently
                        // dropped case (2). The UTF-8 check was not a security boundary: the
                        // real gate is user review + the hardened parser (PR #19). Accepting
                        // raw bytes is a strict superset of base64 text — anything an attacker
                        // could put in raw they could already put in base64.
                        let decoded = decode_qr::detect_and_decode(&data);

                        // Message-signing path: a `faraday:msg:` envelope
                        // decodes to message_bytes. Route to the dedicated
                        // review screen before falling through to tx parsing.
                        if let Some(message_bytes) = decoded.message_bytes.clone() {
                            return Screen::SignMessageReview {
                                message_bytes,
                                scroll: 0,
                            };
                        }

                        let tx_bytes = match decoded.tx_bytes {
                            Some(b) => b,
                            // Fallback for raw-binary UR payloads detect_and_decode doesn't
                            // classify. The shape check is UX only — it filters obvious non-tx
                            // scans (URLs, address QRs, text) so the review screen doesn't show
                            // "Parse error" for them. Anything that passes still goes through
                            // the parser and the user-review gate before any signing.
                            None if looks_like_solana_tx(&data) => data,
                            _ => return Screen::SignScanTx,
                        };

                        let tx_base64 = BASE64.encode(&tx_bytes);
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
                InputEvent::Secondary => {
                    return Screen::SignMessageInput {
                        grid: CharGrid::new(),
                    }
                }
                InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignScanTx
        }

        Screen::SignReview {
            tx_bytes,
            tx_base64,
            info_lines,
            mut scroll,
            mut selected,
            can_sign,
        } => {
            match event {
                // Jump one "chunk" per key press instead of one line.
                // Chunks are delimited by blank rows in `info_lines` (the
                // parser inserts `String::new()` between sections), so we
                // advance past the next blank (Down) or retreat past the
                // previous blank (Up) and land at the start of that chunk.
                // Falls back to single-line stepping when no blank is
                // found before the end / beginning.
                InputEvent::Up => {
                    scroll = prev_chunk(&info_lines, scroll);
                }
                InputEvent::Down => {
                    let max_scroll = info_lines.len().saturating_sub(8);
                    scroll = next_chunk(&info_lines, scroll).min(max_scroll);
                }
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
                                #[cfg(feature = "_desktop_sim")]
                                {
                                    println!("Signed by: {}", signed.signer_pubkey);
                                    println!(
                                        "Signature: {}...",
                                        hex::encode(&signed.signature[..16])
                                    );
                                    println!("TX size: {} bytes", signed.signed_bytes.len());
                                }
                                // Display the compact `faraday:sig:` envelope
                                // (version + pubkey + 64-byte signature), not
                                // the full signed tx — the extension already
                                // has the unsigned bytes and will splice our
                                // sig into the right slot. ~144-char payload
                                // renders as a V8 QR (49×49), readable by any
                                // webcam off the Pi's 240 px screen.
                                return Screen::SignShowQr {
                                    data: signed.signature_envelope,
                                };
                            }
                        }
                    } else if selected == 0 && !can_sign {
                        return Screen::SignReview {
                            tx_bytes,
                            tx_base64,
                            info_lines,
                            scroll,
                            selected: 1,
                            can_sign,
                        };
                    }
                    return Screen::MainMenu { selected: 2 };
                }
                InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignReview {
                tx_bytes,
                tx_base64,
                info_lines,
                scroll,
                selected,
                can_sign,
            }
        }

        Screen::SignShowQr { data } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => return Screen::MainMenu { selected: 2 },
                InputEvent::Secondary => return Screen::SignScanTx,
                _ => {}
            }
            Screen::SignShowQr { data }
        }

        Screen::SignMessageReview {
            message_bytes,
            mut scroll,
        } => {
            match event {
                InputEvent::Up => {
                    if scroll > 0 {
                        scroll -= 1;
                    }
                }
                InputEvent::Down => {
                    scroll += 1;
                }
                InputEvent::Confirm => {
                    if let Some(wallet) = &app.wallet {
                        let sig = crate::signer::sign_message(
                            &message_bytes,
                            &wallet.keypair.private_key,
                        );
                        let signature_hex = hex::encode(&sig);
                        return Screen::SignMessageResult { signature_hex };
                    }
                    return Screen::MainMenu { selected: 2 };
                }
                InputEvent::Back => return Screen::MainMenu { selected: 2 },
                _ => {}
            }
            Screen::SignMessageReview {
                message_bytes,
                scroll,
            }
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

/// Cheap shape check — filters obvious non-tx QR scans (URLs, text, addresses)
/// so the review screen doesn't show "Parse error" for them. The check is
/// intentionally permissive: anything plausibly tx-shaped is handed to the
/// real parser, which is the actual validator.
///
/// Legitimate txs start with a compact-u16 signature count (1..=127 fits in
/// one byte) followed by that many 64-byte signatures and at least a 3-byte
/// message header. A count of 0 is rejected because a tx with no signatures
/// can't be signed or broadcast, and capping at 5 is a sanity bound (nothing
/// legitimate we've seen exceeds this) that also limits the damage a crafted
/// oversized scan can do before the parser catches it.
fn looks_like_solana_tx(data: &[u8]) -> bool {
    let sigs = match data.first() {
        Some(&n) if (1..=5).contains(&n) => n as usize,
        _ => return false,
    };
    let min_len = 1 + sigs * 64 + 3;
    data.len() >= min_len
}

/// Find the scroll index at the start of the next chunk, where "chunk"
/// means "the line after the next blank line." Returns `current + 1` as a
/// fallback when no blank line appears ahead — so single-line progress
/// still works near the end of the list. Caller is expected to clamp
/// against the end-of-list bound.
fn next_chunk(lines: &[String], current: usize) -> usize {
    let mut i = current;
    // Step past the current chunk's body until we hit a blank row.
    while i < lines.len() && !lines[i].is_empty() {
        i += 1;
    }
    // Step past any run of consecutive blanks.
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    if i == current {
        current + 1
    } else {
        i
    }
}

/// Mirror of `next_chunk` going backwards. Lands on the start of the
/// previous chunk (the line after the blank that separates it from the
/// current one). Returns 0 when there's no earlier chunk.
fn prev_chunk(lines: &[String], current: usize) -> usize {
    if current == 0 {
        return 0;
    }
    let mut i = current.saturating_sub(1);
    // Step past any blanks immediately above the current position.
    while i > 0 && lines[i].is_empty() {
        i -= 1;
    }
    // Rewind through the previous chunk's body.
    while i > 0 && !lines[i - 1].is_empty() {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_tx_accepts_minimal_single_sig_tx() {
        let mut tx = vec![1u8]; // one signature
        tx.extend_from_slice(&[0u8; 64]); // signature bytes
        tx.extend_from_slice(&[0u8; 3]); // message header
        assert!(looks_like_solana_tx(&tx));
    }

    #[test]
    fn looks_like_tx_rejects_zero_sig_count() {
        let tx = vec![0u8; 200];
        assert!(!looks_like_solana_tx(&tx));
    }

    #[test]
    fn looks_like_tx_rejects_short_payload() {
        // claims one sig but doesn't carry 64 bytes of it
        let short = vec![1u8, 0, 0, 0, 0];
        assert!(!looks_like_solana_tx(&short));
    }

    #[test]
    fn looks_like_tx_rejects_text_blob() {
        // a URL, 9x 'h' would also be rejected by the sig-count test
        let url = b"https://example.com/some/path";
        assert!(!looks_like_solana_tx(url));
    }

    #[test]
    fn looks_like_tx_rejects_solana_address() {
        // 32-byte base58 address is way under the min tx length
        let addr = b"GAthe6Gh8xEuJobQWB3cLUBFjsGtyvsk";
        assert!(!looks_like_solana_tx(addr));
    }

    #[test]
    fn looks_like_tx_accepts_real_self_transfer_fixture() {
        // The committed demo tx in testdata/examples/ is exactly the shape
        // we expect from a UR-reassembled payload.
        let tx = std::fs::read("testdata/examples/self_transfer.bin")
            .expect("self_transfer fixture present");
        assert!(looks_like_solana_tx(&tx));
    }
}
