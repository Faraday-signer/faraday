//! `clear-msig-ika` instruction parser.
//!
//! Reference: https://github.com/Iamknownasfesal/clear-msig-ika
//! Quasar-style program — single-byte discriminator at offset 0.
//!
//! Instructions handled (verified against `cli/src/quasar_client/*.rs`):
//!
//! | disc | name              | args                                                               | accounts                                                     |
//! |------|-------------------|--------------------------------------------------------------------|--------------------------------------------------------------|
//! | 0    | create_wallet     | thresholds u8/u8, timelock u32 LE, then wincode name/proposers/approvers | payer, name_hash, wallet, 3× intent PDAs, system        |
//! | 1    | propose           | proposal_idx u64 LE, expiry i64 LE, proposer [32], sig [64], params… | payer, wallet, intent, proposal, system                   |
//! | 2    | approve           | expiry i64 LE, approver_idx u8, signature [64]                     | wallet, intent, proposal                                     |
//! | 3    | cancel            | expiry i64 LE, canceller_idx u8, signature [64]                    | wallet, intent, proposal                                     |
//! | 4    | execute           | (no args beyond disc)                                              | wallet, vault, intent, proposal, system, …                   |
//! | 5    | cleanup_proposal  | (no args beyond disc)                                              | proposal (close), rent_refund                                |
//! | 6    | bind_dwallet      | chain_kind u8, user_pubkey [32], sig_scheme u16 LE, bump u8        | payer, wallet, ika_config, dwallet_ownership, dwallet, …     |
//! | 7    | ika_sign          | msg_approval_bump u8, cpi_authority_bump u8, blake2b_hashes [96]   | payer, wallet, intent, proposal, …, dwallet, message_approval, …, dwallet_program |

use crate::parser::bytes::pubkey_short;
use crate::parser::{ParsedInstruction, ReviewItem};

const PROGRAM: &str = "Ika clear-msig";

pub fn parse(data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let disc = match data.first() {
        Some(b) => *b,
        None => return error("empty instruction data"),
    };
    let items = match disc {
        0 => decode_create_wallet(&data[1..]),
        1 => decode_propose(&data[1..], accounts),
        2 => decode_vote("Ika approve", "Approver", &data[1..], accounts),
        3 => decode_vote("Ika cancel", "Canceller", &data[1..], accounts),
        4 => Ok(decode_execute(accounts)),
        5 => Ok(decode_cleanup(accounts)),
        6 => decode_bind_dwallet(&data[1..], accounts),
        7 => decode_ika_sign(&data[1..], accounts),
        _ => Ok(vec![
            ReviewItem::Header(PROGRAM.into()),
            ReviewItem::Field {
                label: "Action".into(),
                value: format!("Discriminator {}", disc),
            },
        ]),
    };

    match items {
        Ok(items) => ParsedInstruction {
            program: PROGRAM.into(),
            items,
        },
        Err(msg) => error(msg),
    }
}

fn error(msg: &'static str) -> ParsedInstruction {
    ParsedInstruction {
        program: PROGRAM.into(),
        items: vec![
            ReviewItem::Header(PROGRAM.into()),
            ReviewItem::Warning(format!("Parse error: {}", msg)),
        ],
    }
}

fn decode_vote(
    header: &'static str,
    index_label: &'static str,
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    // After the disc byte the payload is fixed:
    //   expiry i64 LE (8) | index u8 (1) | signature (64) = 73 bytes
    // The signature is omitted from the review — it's redundant with the
    // off-chain message preview the user already approved on the device.
    if data.len() < 9 {
        return Err("data too short for approve/cancel");
    }
    let expiry = i64::from_le_bytes(data[0..8].try_into().expect("8 bytes"));
    let index = data[8];

    let wallet = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let intent = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let proposal = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header(header.into()),
        ReviewItem::Field {
            label: "Wallet".into(),
            value: wallet,
        },
        ReviewItem::Field {
            label: "Proposal".into(),
            value: proposal,
        },
        ReviewItem::Field {
            label: "Intent".into(),
            value: intent,
        },
        ReviewItem::Field {
            label: index_label.into(),
            value: format!("#{}", index),
        },
        ReviewItem::Field {
            label: "Expires (unix)".into(),
            value: expiry.to_string(),
        },
    ])
}

fn decode_propose(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    // After the disc byte:
    //   proposal_index u64 LE (8) | expiry i64 LE (8) | proposer [32] | signature [64] | params_data (tail)
    // = 112 bytes fixed, then variable params_data.
    if data.len() < 112 {
        return Err("data too short for propose");
    }
    let proposal_index = u64::from_le_bytes(data[0..8].try_into().expect("8 bytes"));
    let expiry = i64::from_le_bytes(data[8..16].try_into().expect("8 bytes"));
    let proposer: [u8; 32] = data[16..48].try_into().expect("32 bytes");
    let params_len = data.len() - 112;

    let wallet = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let intent = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let proposal_acct = accounts
        .get(3)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Ika propose".into()),
        ReviewItem::Field {
            label: "Wallet".into(),
            value: wallet,
        },
        ReviewItem::Field {
            label: "Proposal".into(),
            value: proposal_acct,
        },
        ReviewItem::Field {
            label: "Intent".into(),
            value: intent,
        },
        ReviewItem::Field {
            label: "Proposal #".into(),
            value: proposal_index.to_string(),
        },
        ReviewItem::Field {
            label: "Proposer".into(),
            value: pubkey_short(&proposer),
        },
        ReviewItem::Field {
            label: "Expires (unix)".into(),
            value: expiry.to_string(),
        },
        ReviewItem::Field {
            label: "Params".into(),
            value: format!("{} bytes", params_len),
        },
    ])
}

fn decode_cleanup(accounts: &[[u8; 32]]) -> Vec<ReviewItem> {
    let proposal = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let rent_refund = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    vec![
        ReviewItem::Header("Ika cleanup proposal".into()),
        ReviewItem::Field {
            label: "Proposal".into(),
            value: proposal,
        },
        ReviewItem::Field {
            label: "Rent refund".into(),
            value: rent_refund,
        },
    ]
}

fn decode_bind_dwallet(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    // After the disc byte:
    //   chain_kind u8 (1) | user_pubkey [32] | sig_scheme u16 LE (2) | bump u8 (1) = 36 bytes
    if data.len() < 36 {
        return Err("data too short for bind_dwallet");
    }
    let chain_kind = data[0];
    let user_pubkey: [u8; 32] = data[1..33].try_into().expect("32 bytes");
    let sig_scheme = u16::from_le_bytes([data[33], data[34]]);

    let wallet = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let dwallet = accounts
        .get(4)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Ika bind dWallet".into()),
        ReviewItem::Field {
            label: "Wallet".into(),
            value: wallet,
        },
        ReviewItem::Field {
            label: "dWallet".into(),
            value: dwallet,
        },
        ReviewItem::Field {
            label: "Chain".into(),
            value: chain_kind_name(chain_kind).to_string(),
        },
        ReviewItem::Field {
            label: "User key".into(),
            value: pubkey_short(&user_pubkey),
        },
        ReviewItem::Field {
            label: "Sig scheme".into(),
            value: sig_scheme.to_string(),
        },
    ])
}

/// `ChainKind` enum from `programs/clear-wallet/src/chains/mod.rs:56-77`.
pub(crate) fn chain_kind_name(kind: u8) -> &'static str {
    match kind {
        0 => "Solana",
        1 => "EVM (1559)",
        2 => "Bitcoin",
        3 => "Zcash",
        4 => "ERC-20",
        _ => "Unknown",
    }
}

fn decode_ika_sign(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    // After the disc byte:
    //   message_approval_bump u8 (1) | cpi_authority_bump u8 (1) | blake2b_hashes [96] = 98 bytes
    // 96 hash bytes = 3 × 32-byte hashes (one per message being signed).
    if data.len() < 98 {
        return Err("data too short for ika_sign");
    }
    let hashes = &data[2..98];

    let wallet = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let intent = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let proposal = accounts
        .get(3)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let dwallet = accounts
        .get(6)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    let mut items = vec![
        ReviewItem::Header("Ika sign (MPC)".into()),
        ReviewItem::Field {
            label: "Wallet".into(),
            value: wallet,
        },
        ReviewItem::Field {
            label: "Proposal".into(),
            value: proposal,
        },
        ReviewItem::Field {
            label: "Intent".into(),
            value: intent,
        },
        ReviewItem::Field {
            label: "dWallet".into(),
            value: dwallet,
        },
    ];
    for (i, chunk) in hashes.chunks_exact(32).enumerate() {
        items.push(ReviewItem::Field {
            label: format!("Hash {}", i + 1).into(),
            value: short_hex(chunk),
        });
    }
    Ok(items)
}

fn decode_create_wallet(data: &[u8]) -> Result<Vec<ReviewItem>, &'static str> {
    // After the disc byte:
    //   approval_threshold u8 | cancellation_threshold u8 | timelock_seconds u32 LE
    //   ... then wincode-encoded name + proposers + approvers (variable, format unverified).
    // We surface only the fixed prefix — the rest is shown as a byte count so
    // the user has *something* to verify, without us guessing the wincode
    // length-prefix shape.
    if data.len() < 6 {
        return Err("data too short for create_wallet");
    }
    let approval_threshold = data[0];
    let cancellation_threshold = data[1];
    let timelock_seconds = u32::from_le_bytes(data[2..6].try_into().expect("4 bytes"));
    let variable_bytes = data.len() - 6;

    Ok(vec![
        ReviewItem::Header("Ika create wallet".into()),
        ReviewItem::Field {
            label: "Approval thr.".into(),
            value: approval_threshold.to_string(),
        },
        ReviewItem::Field {
            label: "Cancel thr.".into(),
            value: cancellation_threshold.to_string(),
        },
        ReviewItem::Field {
            label: "Timelock".into(),
            value: format_seconds(timelock_seconds),
        },
        ReviewItem::Field {
            label: "Definition".into(),
            value: format!("{} bytes", variable_bytes),
        },
    ])
}

/// Render seconds as a human-friendly duration. Falls back to the raw value
/// for anything past 1 day so the user can spot suspiciously large windows.
pub(crate) fn format_seconds(secs: u32) -> String {
    if secs == 0 {
        "none".to_string()
    } else if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d ({}s)", secs / 86_400, secs)
    }
}

fn short_hex(bytes: &[u8]) -> String {
    let full = hex::encode(bytes);
    if full.len() <= 16 {
        full
    } else {
        format!("{}...{}", &full[..6], &full[full.len() - 6..])
    }
}

fn decode_execute(accounts: &[[u8; 32]]) -> Vec<ReviewItem> {
    let wallet = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let vault = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let intent = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let proposal = accounts
        .get(3)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    vec![
        ReviewItem::Header("Ika execute".into()),
        ReviewItem::Field {
            label: "Wallet".into(),
            value: wallet,
        },
        ReviewItem::Field {
            label: "Proposal".into(),
            value: proposal,
        },
        ReviewItem::Field {
            label: "Intent".into(),
            value: intent,
        },
        ReviewItem::Field {
            label: "Vault".into(),
            value: vault,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn header_text(ix: &ParsedInstruction) -> Option<&str> {
        ix.items.iter().find_map(|item| match item {
            ReviewItem::Header(h) => Some(h.as_str()),
            _ => None,
        })
    }

    fn field<'a>(ix: &'a ParsedInstruction, label: &str) -> Option<&'a str> {
        ix.items.iter().find_map(|item| match item {
            ReviewItem::Field { label: l, value } if l == label => Some(value.as_str()),
            _ => None,
        })
    }

    fn vote_data(disc: u8, expiry: i64, index: u8) -> Vec<u8> {
        let mut d = vec![disc];
        d.extend_from_slice(&expiry.to_le_bytes());
        d.push(index);
        d.extend_from_slice(&[0u8; 64]); // signature
        d
    }

    #[test]
    fn parses_approve() {
        let ix = parse(
            &vote_data(2, 1_893_456_000, 3),
            &[key(0x11), key(0x22), key(0x33)],
        );
        assert_eq!(ix.program, "Ika clear-msig");
        assert_eq!(header_text(&ix), Some("Ika approve"));
        assert_eq!(field(&ix, "Approver"), Some("#3"));
        assert_eq!(field(&ix, "Expires (unix)"), Some("1893456000"));
        // Wallet/proposal pubkeys are present and shortened.
        assert!(field(&ix, "Wallet").is_some());
        assert!(field(&ix, "Proposal").is_some());
    }

    #[test]
    fn parses_cancel() {
        let ix = parse(
            &vote_data(3, 1_893_456_000, 5),
            &[key(0x11), key(0x22), key(0x33)],
        );
        assert_eq!(header_text(&ix), Some("Ika cancel"));
        assert_eq!(field(&ix, "Canceller"), Some("#5"));
    }

    #[test]
    fn parses_execute() {
        let ix = parse(
            &[4u8],
            &[key(0x11), key(0x22), key(0x33), key(0x44), key(0x55)],
        );
        assert_eq!(header_text(&ix), Some("Ika execute"));
        assert!(field(&ix, "Wallet").is_some());
        assert!(field(&ix, "Vault").is_some());
        assert!(field(&ix, "Intent").is_some());
        assert!(field(&ix, "Proposal").is_some());
    }

    #[test]
    fn unknown_discriminator_does_not_panic() {
        let ix = parse(&[99u8], &[]);
        assert_eq!(ix.program, "Ika clear-msig");
        assert!(field(&ix, "Action").is_some());
    }

    #[test]
    fn empty_data_returns_warning() {
        let ix = parse(&[], &[]);
        let has_warning = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    #[test]
    fn truncated_vote_data_returns_warning() {
        // disc + 5 bytes — too short for expiry+index+sig.
        let ix = parse(&[2u8, 0, 0, 0, 0, 0], &[]);
        let has_warning = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    fn propose_data(proposal_idx: u64, expiry: i64, params_len: usize) -> Vec<u8> {
        let mut d = vec![1u8];
        d.extend_from_slice(&proposal_idx.to_le_bytes());
        d.extend_from_slice(&expiry.to_le_bytes());
        d.extend_from_slice(&[7u8; 32]); // proposer
        d.extend_from_slice(&[0u8; 64]); // signature
        d.extend_from_slice(&vec![0u8; params_len]); // params_data
        d
    }

    #[test]
    fn parses_propose() {
        let ix = parse(
            &propose_data(7, 1_893_456_000, 24),
            &[key(0xaa), key(0xbb), key(0xcc), key(0xdd), key(0xee)],
        );
        assert_eq!(header_text(&ix), Some("Ika propose"));
        assert_eq!(field(&ix, "Proposal #"), Some("7"));
        assert_eq!(field(&ix, "Expires (unix)"), Some("1893456000"));
        assert_eq!(field(&ix, "Params"), Some("24 bytes"));
        assert!(field(&ix, "Proposer").is_some());
    }

    #[test]
    fn parses_propose_with_empty_params() {
        let ix = parse(
            &propose_data(1, 0, 0),
            &[key(0xaa), key(0xbb), key(0xcc), key(0xdd), key(0xee)],
        );
        assert_eq!(field(&ix, "Params"), Some("0 bytes"));
    }

    #[test]
    fn parses_cleanup_proposal() {
        let ix = parse(&[5u8], &[key(0x01), key(0x02)]);
        assert_eq!(header_text(&ix), Some("Ika cleanup proposal"));
        assert!(field(&ix, "Proposal").is_some());
        assert!(field(&ix, "Rent refund").is_some());
    }

    #[test]
    fn parses_bind_dwallet_solana() {
        let mut data = vec![6u8, 0u8]; // disc, chain_kind=Solana
        data.extend_from_slice(&[9u8; 32]); // user_pubkey
        data.extend_from_slice(&0u16.to_le_bytes()); // sig_scheme
        data.push(255); // cpi_authority_bump
        let ix = parse(
            &data,
            &[
                key(0x01), key(0x02), key(0x03), key(0x04),
                key(0xff), // dwallet at slot 4
            ],
        );
        assert_eq!(header_text(&ix), Some("Ika bind dWallet"));
        assert_eq!(field(&ix, "Chain"), Some("Solana"));
        assert!(field(&ix, "dWallet").is_some());
        assert!(field(&ix, "User key").is_some());
    }

    #[test]
    fn bind_dwallet_chain_kind_mapping() {
        let make_data = |kind: u8| {
            let mut data = vec![6u8, kind];
            data.extend_from_slice(&[1u8; 32]);
            data.extend_from_slice(&1u16.to_le_bytes());
            data.push(0);
            data
        };
        for (kind, expected) in [
            (0u8, "Solana"),
            (1, "EVM (1559)"),
            (2, "Bitcoin"),
            (3, "Zcash"),
            (4, "ERC-20"),
            (99, "Unknown"),
        ] {
            let ix = parse(&make_data(kind), &[key(0); 5]);
            assert_eq!(field(&ix, "Chain"), Some(expected), "kind={}", kind);
        }
    }

    #[test]
    fn parses_ika_sign() {
        let mut data = vec![7u8, 255, 254]; // disc + two bumps
        data.extend_from_slice(&[0xa1u8; 32]); // hash 1
        data.extend_from_slice(&[0xb2u8; 32]); // hash 2
        data.extend_from_slice(&[0xc3u8; 32]); // hash 3
        let ix = parse(
            &data,
            &[
                key(0x01), key(0x02), key(0x03), key(0x04),
                key(0x05), key(0x06),
                key(0xab), // dwallet at slot 6
            ],
        );
        assert_eq!(header_text(&ix), Some("Ika sign (MPC)"));
        assert!(field(&ix, "dWallet").is_some());
        assert_eq!(field(&ix, "Hash 1"), Some("a1a1a1...a1a1a1"));
        assert_eq!(field(&ix, "Hash 2"), Some("b2b2b2...b2b2b2"));
        assert_eq!(field(&ix, "Hash 3"), Some("c3c3c3...c3c3c3"));
    }

    #[test]
    fn parses_create_wallet_prefix() {
        let mut data = vec![0u8, 2u8, 1u8]; // disc, approval=2, cancel=1
        data.extend_from_slice(&3600u32.to_le_bytes()); // timelock 1h
        data.extend_from_slice(&[0u8; 40]); // opaque wincode tail
        let ix = parse(&data, &[]);
        assert_eq!(header_text(&ix), Some("Ika create wallet"));
        assert_eq!(field(&ix, "Approval thr."), Some("2"));
        assert_eq!(field(&ix, "Cancel thr."), Some("1"));
        assert_eq!(field(&ix, "Timelock"), Some("1h"));
        assert_eq!(field(&ix, "Definition"), Some("40 bytes"));
    }

    #[test]
    fn format_seconds_buckets() {
        assert_eq!(format_seconds(0), "none");
        assert_eq!(format_seconds(45), "45s");
        assert_eq!(format_seconds(3600), "1h");
        assert_eq!(format_seconds(172_800), "2d (172800s)");
    }
}
