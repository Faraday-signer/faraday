//! System Program instruction parser.
//!
//! Reference: https://docs.rs/solana-sdk/latest/solana_sdk/system_instruction/enum.SystemInstruction.html

use crate::parser::bytes::{pubkey_short, read_u32_le, read_u64_le};
use crate::parser::{ParsedInstruction, ReviewItem};

pub fn parse(data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let items = match decode(data, accounts) {
        Ok(items) => items,
        Err(e) => vec![
            ReviewItem::Header("System".into()),
            ReviewItem::Warning(format!("Parse error: {}", e)),
        ],
    };
    ParsedInstruction {
        program: "System".into(),
        items,
    }
}

fn decode(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let ix_type = read_u32_le(data, 0)?;

    match ix_type {
        0 => parse_create_account(&data[4..], accounts),
        2 => parse_transfer(&data[4..], accounts),
        3 => parse_create_account_with_seed(&data[4..], accounts),
        4 => parse_advance_nonce_account(accounts),
        6 => parse_initialize_nonce_account(&data[4..], accounts),
        8 => parse_allocate(&data[4..], accounts),
        11 => parse_transfer_with_seed(&data[4..], accounts),
        _ => Ok(vec![
            ReviewItem::Header("System".into()),
            ReviewItem::Field {
                label: "Action".into(),
                value: format!("Type {}", ix_type),
            },
        ]),
    }
}

fn parse_transfer(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let lamports = read_u64_le(data, 0)?;

    let from = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let to = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("SOL Transfer".into()),
        ReviewItem::Field {
            label: "From".into(),
            value: from,
        },
        ReviewItem::Field {
            label: "To".into(),
            value: to,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: lamports_to_sol(lamports),
        },
    ])
}

fn parse_create_account(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let lamports = read_u64_le(data, 0)?;
    let space = read_u64_le(data, 8)?;

    let funder = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let new_account = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Create Account".into()),
        ReviewItem::Field {
            label: "Funder".into(),
            value: funder,
        },
        ReviewItem::Field {
            label: "New account".into(),
            value: new_account,
        },
        ReviewItem::Field {
            label: "Rent".into(),
            value: lamports_to_sol(lamports),
        },
        ReviewItem::Field {
            label: "Space".into(),
            value: format!("{} bytes", space),
        },
    ])
}

fn parse_create_account_with_seed(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let funder = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let new_account = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    // base: 32 bytes, seed_len: u64, seed: variable
    let seed_len = read_u64_le(data, 32)? as usize;
    let seed_end = 40usize
        .checked_add(seed_len)
        .ok_or("CreateAccountWithSeed seed too long")?;
    let seed = data
        .get(40..seed_end)
        .and_then(|s| std::str::from_utf8(s).ok())
        .unwrap_or("?");
    let lamports = read_u64_le(data, seed_end)?;

    Ok(vec![
        ReviewItem::Header("Create Account (seed)".into()),
        ReviewItem::Field {
            label: "Funder".into(),
            value: funder,
        },
        ReviewItem::Field {
            label: "New account".into(),
            value: new_account,
        },
        ReviewItem::Field {
            label: "Seed".into(),
            value: seed.to_string(),
        },
        ReviewItem::Field {
            label: "Rent".into(),
            value: lamports_to_sol(lamports),
        },
    ])
}

fn parse_allocate(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let space = read_u64_le(data, 0)?;
    let account = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Allocate".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: account,
        },
        ReviewItem::Field {
            label: "Space".into(),
            value: format!("{} bytes", space),
        },
    ])
}

fn parse_transfer_with_seed(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let lamports = read_u64_le(data, 0)?;
    let from = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let to = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("SOL Transfer (seed)".into()),
        ReviewItem::Field {
            label: "From".into(),
            value: from,
        },
        ReviewItem::Field {
            label: "To".into(),
            value: to,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: lamports_to_sol(lamports),
        },
    ])
}

/// `AdvanceNonceAccount` (SystemInstruction disc 4). This is the leading
/// instruction on every Faraday-built durable-nonce transaction: it consumes
/// the nonce so the signature stays valid until the nonce next advances,
/// however long the QR relay takes. Account layout per the SDK:
///   [nonce account, RecentBlockhashes sysvar, nonce authority].
/// The instruction carries no data past the discriminant.
fn parse_advance_nonce_account(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let nonce_account = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let authority = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Advance Nonce".into()),
        ReviewItem::Field {
            label: "Nonce account".into(),
            value: nonce_account,
        },
        ReviewItem::Field {
            label: "Authority".into(),
            value: authority,
        },
    ])
}

/// `InitializeNonceAccount` (SystemInstruction disc 6). Emitted once, when
/// Faraday sets up the wallet's nonce account. The instruction data is the
/// 32-byte nonce authority pubkey; account layout is
///   [nonce account, RecentBlockhashes sysvar, Rent sysvar].
fn parse_initialize_nonce_account(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let authority_bytes: [u8; 32] = data
        .get(0..32)
        .and_then(|s| s.try_into().ok())
        .ok_or("InitializeNonceAccount authority truncated")?;

    let nonce_account = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Initialize Nonce".into()),
        ReviewItem::Field {
            label: "Nonce account".into(),
            value: nonce_account,
        },
        ReviewItem::Field {
            label: "Authority".into(),
            value: pubkey_short(&authority_bytes),
        },
    ])
}

pub(crate) fn lamports_to_sol(lamports: u64) -> String {
    let sol = lamports / 1_000_000_000;
    let frac = lamports % 1_000_000_000;
    if frac == 0 {
        format!("{} SOL", sol)
    } else {
        let frac_str = format!("{:09}", frac);
        format!("{}.{} SOL", sol, frac_str.trim_end_matches('0'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn transfer_data(lamports: u64) -> Vec<u8> {
        let mut d = vec![2u8, 0, 0, 0]; // Transfer discriminant
        d.extend_from_slice(&lamports.to_le_bytes());
        d
    }

    // --- lamports_to_sol ---

    #[test]
    fn test_lamports_zero() {
        assert_eq!(lamports_to_sol(0), "0 SOL");
    }

    #[test]
    fn test_lamports_exact_sol() {
        assert_eq!(lamports_to_sol(1_000_000_000), "1 SOL");
        assert_eq!(lamports_to_sol(5_000_000_000), "5 SOL");
    }

    #[test]
    fn test_lamports_half_sol() {
        assert_eq!(lamports_to_sol(500_000_000), "0.5 SOL");
    }

    #[test]
    fn test_lamports_one_lamport() {
        assert_eq!(lamports_to_sol(1), "0.000000001 SOL");
    }

    #[test]
    fn test_lamports_trims_trailing_zeros() {
        assert_eq!(lamports_to_sol(1_500_000_000), "1.5 SOL");
        assert_eq!(lamports_to_sol(1_050_000_000), "1.05 SOL");
    }

    // --- parse (public entry point) ---

    #[test]
    fn test_transfer_1_sol() {
        let data = transfer_data(1_000_000_000);
        let accounts = [key(0x01), key(0x02)];
        let ix = parse(&data, &accounts);
        assert_eq!(ix.program, "System");
        let has_amount = ix.items.iter().any(|item| {
            matches!(
                item, ReviewItem::Field { label, value } if label == "Amount" && value == "1 SOL"
            )
        });
        assert!(has_amount, "Expected Amount: 1 SOL");
    }

    #[test]
    fn test_transfer_shows_from_and_to() {
        let data = transfer_data(1_000_000_000);
        let accounts = [key(0x01), key(0x02)];
        let ix = parse(&data, &accounts);
        let labels: Vec<&str> = ix
            .items
            .iter()
            .filter_map(|item| match item {
                ReviewItem::Field { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&"From"));
        assert!(labels.contains(&"To"));
    }

    #[test]
    fn test_create_account() {
        let mut data = vec![0u8, 0, 0, 0]; // CreateAccount discriminant
        data.extend_from_slice(&2_039_280u64.to_le_bytes()); // lamports (rent)
        data.extend_from_slice(&165u64.to_le_bytes()); // space
        data.extend_from_slice(&[0u8; 32]); // owner
        let accounts = [key(0x01), key(0x02)];
        let ix = parse(&data, &accounts);
        let has_space = ix.items.iter().any(|item| {
            matches!(
                item, ReviewItem::Field { label, .. } if label == "Space"
            )
        });
        assert!(has_space);
    }

    #[test]
    fn test_unknown_discriminant_does_not_panic() {
        let data = vec![99u8, 0, 0, 0]; // unrecognized type
        let ix = parse(&data, &[]);
        assert_eq!(ix.program, "System");
    }

    #[test]
    fn test_transfer_data_too_short_returns_warning() {
        let data = vec![2u8, 0, 0, 0, 0]; // only 5 bytes, need 12
        let ix = parse(&data, &[]);
        let has_warning = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    #[test]
    fn test_empty_data_returns_warning() {
        let ix = parse(&[], &[]);
        let has_warning = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    // --- AdvanceNonceAccount (disc 4) ---

    #[test]
    fn test_advance_nonce_is_labeled() {
        let data = vec![4u8, 0, 0, 0]; // AdvanceNonceAccount, no trailing data
        let accounts = [key(0xAA), key(0xBB), key(0xCC)]; // nonce, sysvar, authority
        let ix = parse(&data, &accounts);
        assert_eq!(ix.program, "System");
        let has_header = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Header(h) if h == "Advance Nonce"));
        assert!(has_header, "Expected 'Advance Nonce' header");
        // No unknown-instruction fallback and no warning.
        let has_warning = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)));
        assert!(!has_warning, "Advance Nonce must not warn");
    }

    #[test]
    fn test_advance_nonce_shows_account_and_authority() {
        let data = vec![4u8, 0, 0, 0];
        let accounts = [key(0xAA), key(0xBB), key(0xCC)];
        let ix = parse(&data, &accounts);
        let labels: Vec<&str> = ix
            .items
            .iter()
            .filter_map(|item| match item {
                ReviewItem::Field { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&"Nonce account"));
        assert!(labels.contains(&"Authority"));
    }

    #[test]
    fn test_advance_nonce_missing_authority_slot_does_not_panic() {
        // Only the nonce account present — authority slot renders as "?",
        // never a panic or a guessed value.
        let data = vec![4u8, 0, 0, 0];
        let accounts = [key(0xAA)];
        let ix = parse(&data, &accounts);
        let has_header = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Header(h) if h == "Advance Nonce"));
        assert!(has_header);
    }

    // --- InitializeNonceAccount (disc 6) ---

    #[test]
    fn test_initialize_nonce_is_labeled() {
        let mut data = vec![6u8, 0, 0, 0]; // InitializeNonceAccount
        data.extend_from_slice(&[0xDD; 32]); // authority pubkey in data
        let accounts = [key(0xAA), key(0xBB), key(0xCC)]; // nonce, sysvar, rent
        let ix = parse(&data, &accounts);
        assert_eq!(ix.program, "System");
        let has_header = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Header(h) if h == "Initialize Nonce"));
        assert!(has_header, "Expected 'Initialize Nonce' header");
        let labels: Vec<&str> = ix
            .items
            .iter()
            .filter_map(|item| match item {
                ReviewItem::Field { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&"Nonce account"));
        assert!(labels.contains(&"Authority"));
    }

    #[test]
    fn test_initialize_nonce_truncated_authority_warns() {
        // Discriminant present but the 32-byte authority is cut short — must
        // fail safe with a warning, never pretty-print a partial key.
        let mut data = vec![6u8, 0, 0, 0];
        data.extend_from_slice(&[0xDD; 16]); // only half an authority pubkey
        let ix = parse(&data, &[key(0xAA)]);
        let has_warning = ix
            .items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)));
        assert!(has_warning, "Truncated InitializeNonceAccount must warn");
    }
}
