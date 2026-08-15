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
                            // Cheap first-line UX router: bounce an obvious tx
                            // scan back to the scanner. The authoritative
                            // anti-forge check is `is_transaction_message`
                            // inside `sign_message`, which refuses to sign
                            // anything that parses as a transaction (#79).
                            if looks_like_solana_tx(&message_bytes) {
                                return Screen::SignScanTx;
                            }
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
                        let (info_lines, can_sign, parsed) =
                            build_review_lines(&tx_bytes, &wallet.keypair.public_key);
                        return Screen::SignReview {
                            tx_bytes,
                            tx_base64,
                            info_lines,
                            parsed: Box::new(parsed),
                            page: 0,
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
                InputEvent::Back => return Screen::MainMenu { selected: app.menu_index_of(2) },
                _ => {}
            }
            Screen::SignScanTx
        }

        Screen::SignReview {
            tx_bytes,
            tx_base64,
            info_lines,
            parsed,
            mut page,
            mut scroll,
            mut selected,
            can_sign,
        } => {
            // Page 0 = summary, 1 = metadata, 2 = ix overview, then one per
            // *interesting* ix (ComputeBudget pages skipped), then raw bytes.
            let interesting_count = crate::parser::interesting_ix_indices(parsed.as_ref()).len();
            let total_pages = 3 + interesting_count + 1;

            match event {
                // Touch: swipe up/left steps to the previous page (stops at the
                // first); swipe down/right and a body tap (Secondary) advance to
                // the next page, wrapping back to the first after the last. The
                // SIGN footer cell (Confirm) signs directly — there is no
                // Sign/Reject selection toggle.
                InputEvent::Up | InputEvent::Left if app.touch_input() => {
                    page = page.saturating_sub(1);
                }
                InputEvent::Down | InputEvent::Right | InputEvent::Secondary
                    if app.touch_input() =>
                {
                    page = (page + 1) % total_pages;
                }

                // Keys: K2 (Down) advances to the next page, wrapping back to
                // the summary after the last page. Up steps backward. Left/Right
                // toggle the Sign/Reject selection. The navigation model stays
                // "K1 = sign, K2 = next, K3 = cancel".
                InputEvent::Down => {
                    page = (page + 1) % total_pages;
                    scroll = 0;
                }
                InputEvent::Up => {
                    // Step backward through pages — symmetric with Down so
                    // the user can correct a misclick without cycling all
                    // the way around. Wraps to the last page from page 0.
                    page = if page == 0 { total_pages - 1 } else { page - 1 };
                    scroll = 0;
                }
                InputEvent::Left | InputEvent::Right => {
                    if can_sign {
                        selected = 1 - selected;
                    } else {
                        selected = 1;
                    }
                }
                InputEvent::Confirm => {
                    // Touch: the SIGN cell signs whenever the wallet can sign.
                    // Keys: only when the Sign option (selected == 0) is active.
                    let want_sign = if app.touch_input() {
                        can_sign
                    } else {
                        selected == 0 && can_sign
                    };
                    if want_sign {
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
                    }
                    // Keys: a Confirm on Sign when signing isn't possible bumps
                    // the selection to Reject so the next Confirm cancels
                    // cleanly; any other Confirm cancels. Touch never cancels
                    // via Confirm — the SIGN cell only signs and is hidden when
                    // the wallet can't sign, so a tap on its blank zone falls
                    // through to redraw the same screen below.
                    if !app.touch_input() {
                        if selected == 0 && !can_sign {
                            return Screen::SignReview {
                                tx_bytes,
                                tx_base64,
                                info_lines,
                                parsed,
                                page,
                                scroll,
                                selected: 1,
                                can_sign,
                            };
                        }
                        return Screen::MainMenu { selected: app.menu_index_of(2) };
                    }
                }
                InputEvent::Back => return Screen::MainMenu { selected: app.menu_index_of(2) },
                _ => {}
            }
            Screen::SignReview {
                tx_bytes,
                tx_base64,
                info_lines,
                parsed,
                page,
                scroll,
                selected,
                can_sign,
            }
        }

        Screen::SignShowQr { data } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => return Screen::MainMenu { selected: app.menu_index_of(2) },
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
                        match crate::signer::sign_message(
                            &message_bytes,
                            &wallet.keypair.private_key,
                        ) {
                            Ok(sig) => {
                                let signature_hex = hex::encode(&sig);
                                return Screen::SignMessageResult { signature_hex };
                            }
                            // Any refusal (tx-shaped #79, or over-length) shows
                            // the refusal outcome — never a signature, never a
                            // silent drop to the menu.
                            Err(_) => return Screen::SignMessageRefused,
                        }
                    }
                    return Screen::MainMenu { selected: app.menu_index_of(2) };
                }
                InputEvent::Back => return Screen::MainMenu { selected: app.menu_index_of(2) },
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
                if event == InputEvent::Back {
                    return Screen::SignScanTx;
                }
                if let Some(wallet) = &app.wallet {
                    match crate::signer::sign_message(
                        grid.text.as_bytes(),
                        &wallet.keypair.private_key,
                    ) {
                        Ok(sig) => {
                            let signature_hex = hex::encode(&sig);
                            return Screen::SignMessageResult { signature_hex };
                        }
                        Err(_) => return Screen::SignMessageRefused,
                    }
                }
            }
            Screen::SignMessageInput { grid }
        }

        Screen::SignMessageResult { signature_hex } => {
            match event {
                InputEvent::Confirm | InputEvent::Back => return Screen::MainMenu { selected: app.menu_index_of(2) },
                _ => {}
            }
            Screen::SignMessageResult { signature_hex }
        }

        Screen::SignMessageRefused => {
            match event {
                InputEvent::Confirm | InputEvent::Back => {
                    return Screen::MainMenu { selected: app.menu_index_of(2) }
                }
                _ => {}
            }
            Screen::SignMessageRefused
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
        let addr = b"11111111111111111111111111111111";
        assert!(!looks_like_solana_tx(addr));
    }

    #[test]
    fn looks_like_tx_accepts_real_self_transfer_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../raspberry-pi/testdata/examples/self_transfer.bin");
        let tx = match std::fs::read(&path) {
            Ok(data) => data,
            Err(_) => return,
        };
        assert!(looks_like_solana_tx(&tx));
    }
}

/// SignReview navigation semantics under both input models. These used to be
/// `#[cfg(feature = "touch-ui")]` forks, so only one interpretation was ever
/// compiled and none were tested. With the runtime `InputModel` on `App` both
/// interpretations compile on every build and are exercised here regardless of
/// which feature the test binary was built with.
#[cfg(test)]
mod input_model_tests {
    use super::*;
    use crate::gui::app::InputModel;
    use crate::parser::{ParsedTransaction, TransactionVersion};
    use crate::ui::Theme;

    fn review_screen(page: usize, selected: usize, can_sign: bool) -> Screen {
        Screen::SignReview {
            tx_bytes: vec![1, 2, 3],
            tx_base64: String::new(),
            info_lines: Vec::new(),
            parsed: Box::new(ParsedTransaction {
                version: TransactionVersion::Legacy,
                fee_payer: String::new(),
                signers: Vec::new(),
                instructions: Vec::new(),
                fee_lamports: 0,
                size: 0,
                has_unresolved_accounts: false,
            }),
            page,
            scroll: 0,
            selected,
            can_sign,
        }
    }

    fn app_with(model: InputModel) -> App {
        let mut app = App::new(Theme::faraday_240());
        app.input_model = model;
        app
    }

    #[test]
    fn keys_left_toggles_selection_and_leaves_page() {
        let mut app = app_with(InputModel::Keys);
        match handle(&mut app, review_screen(0, 0, true), InputEvent::Left) {
            Screen::SignReview { page, selected, .. } => {
                assert_eq!(page, 0, "keys Left must not page");
                assert_eq!(selected, 1, "keys Left toggles Sign/Reject");
            }
            _ => panic!("expected SignReview"),
        }
    }

    #[test]
    fn touch_left_pages_back_and_leaves_selection() {
        let mut app = app_with(InputModel::Touch);
        match handle(&mut app, review_screen(1, 0, true), InputEvent::Left) {
            Screen::SignReview { page, selected, .. } => {
                assert_eq!(page, 0, "touch Left steps to the previous page");
                assert_eq!(selected, 0, "touch Left must not toggle selection");
            }
            _ => panic!("expected SignReview"),
        }
    }

    #[test]
    fn touch_right_pages_forward() {
        let mut app = app_with(InputModel::Touch);
        match handle(&mut app, review_screen(0, 0, true), InputEvent::Right) {
            Screen::SignReview { page, .. } => assert_eq!(page, 1),
            _ => panic!("expected SignReview"),
        }
    }

    #[test]
    fn keys_confirm_on_reject_cancels_to_menu() {
        // selected == 1 (Reject): Confirm cancels straight to the menu — no
        // wallet needed, and it must never sign.
        let mut app = app_with(InputModel::Keys);
        let next = handle(&mut app, review_screen(0, 1, true), InputEvent::Confirm);
        assert!(
            matches!(next, Screen::MainMenu { .. }),
            "keys Confirm on Reject cancels to menu"
        );
    }

    #[test]
    fn touch_confirm_without_wallet_never_cancels() {
        // Touch has no Reject cell: Confirm can only sign. With no wallet loaded
        // it can't produce a signature, so it must fall through and redraw the
        // same review screen — never route to the menu the way a keys cancel does.
        let mut app = app_with(InputModel::Touch);
        let next = handle(&mut app, review_screen(0, 0, true), InputEvent::Confirm);
        assert!(
            matches!(next, Screen::SignReview { .. }),
            "touch Confirm never cancels via menu"
        );
    }

    #[test]
    fn keys_back_on_message_keyboard_cancels_without_signing() {
        // Regression (#84): on a keys build, Back with non-empty text must
        // cancel to SignScanTx and never sign with the live key. A wallet is
        // loaded so that any errant signing would route to SignMessageResult —
        // reaching the cancel screen instead proves no signature was produced.
        use crate::crypto::slip0010::SolanaKeypair;
        use crate::gui::app::LoadedWallet;
        use zeroize::Zeroizing;

        let mut app = app_with(InputModel::Keys);
        app.wallet = Some(LoadedWallet {
            mnemonic: Zeroizing::new(String::new()),
            passphrase: Zeroizing::new(String::new()),
            keypair: SolanaKeypair {
                private_key: [7u8; 32],
                public_key: [0u8; 32],
                derivation_path: String::new(),
            },
            address: String::new(),
        });

        let mut grid = CharGrid::new();
        grid.text = "hello".to_string();
        let screen = Screen::SignMessageInput { grid };

        let next = handle(&mut app, screen, InputEvent::Back);
        assert!(
            matches!(next, Screen::SignScanTx),
            "keys Back on message keyboard must cancel, not sign"
        );
    }
}
