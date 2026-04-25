//! System Program instruction parser.
//!
//! Reference: https://docs.rs/solana-sdk/latest/solana_sdk/system_instruction/enum.SystemInstruction.html

use crate::parser::bytes::{read_u32_le, read_u64_le};
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

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
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
}
