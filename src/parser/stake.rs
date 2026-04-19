//! Stake Program instruction parser.
//!
//! Reference: https://docs.rs/solana-sdk/latest/solana_sdk/stake/instruction/enum.StakeInstruction.html

use crate::parser::{ParsedInstruction, ReviewItem};
use crate::parser::system::lamports_to_sol;

pub fn parse(data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let items = match decode(data, accounts) {
        Ok(items) => items,
        Err(e) => vec![
            ReviewItem::Header("Stake".into()),
            ReviewItem::Warning(format!("Parse error: {}", e)),
        ],
    };
    ParsedInstruction { program: "Stake".into(), items }
}

fn decode(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.is_empty() { return Err("Instruction data too short"); }

    // The Stake program uses bincode varint encoding for the enum discriminator.
    // Values 0-13 fit in a single byte; try u32 (4-byte) first for compatibility
    // with older transactions, fall back to u8 (1-byte) for current format.
    let (ix_type, payload) = if data.len() >= 4 {
        let u32_disc = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if u32_disc <= MAX_VARIANT {
            (u32_disc, &data[4..])
        } else {
            (data[0] as u32, &data[1..])
        }
    } else {
        (data[0] as u32, &data[1..])
    };

    match ix_type {
        0 => parse_initialize(accounts),
        2 => parse_delegate(accounts),
        3 => parse_split(payload, accounts),
        4 => parse_withdraw(payload, accounts),
        5 => parse_deactivate(accounts),
        7 => parse_merge(accounts),
        _ => Ok(vec![
            ReviewItem::Header("Stake".into()),
            ReviewItem::Field { label: "Action".into(), value: format!("Type {}", ix_type) },
        ]),
    }
}

const MAX_VARIANT: u32 = 13;

fn parse_initialize(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let stake_account = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    Ok(vec![
        ReviewItem::Header("Initialize Stake".into()),
        ReviewItem::Field { label: "Stake account".into(), value: stake_account },
    ])
}

fn parse_delegate(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let stake_account = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let vote_account = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());
    Ok(vec![
        ReviewItem::Header("Delegate Stake".into()),
        ReviewItem::Field { label: "Stake".into(), value: stake_account },
        ReviewItem::Field { label: "Validator".into(), value: vote_account },
    ])
}

fn parse_split(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("Split data too short"); }
    let lamports = u64::from_le_bytes(data[..8].try_into().unwrap());
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let dest = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Split Stake".into()),
        ReviewItem::Field { label: "From".into(), value: source },
        ReviewItem::Field { label: "To".into(), value: dest },
        ReviewItem::Field { label: "Amount".into(), value: lamports_to_sol(lamports) },
    ])
}

fn parse_withdraw(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("Withdraw data too short"); }
    let lamports = u64::from_le_bytes(data[..8].try_into().unwrap());
    let stake_account = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let dest = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Withdraw Stake".into()),
        ReviewItem::Field { label: "Stake".into(), value: stake_account },
        ReviewItem::Field { label: "To".into(), value: dest },
        ReviewItem::Field { label: "Amount".into(), value: lamports_to_sol(lamports) },
    ])
}

fn parse_deactivate(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let stake_account = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    Ok(vec![
        ReviewItem::Header("Deactivate Stake".into()),
        ReviewItem::Field { label: "Stake".into(), value: stake_account },
    ])
}

fn parse_merge(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let dest = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let source = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());
    Ok(vec![
        ReviewItem::Header("Merge Stake".into()),
        ReviewItem::Field { label: "Into".into(), value: dest },
        ReviewItem::Field { label: "From".into(), value: source },
    ])
}

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; 32] { [byte; 32] }

    fn has_header(items: &[ReviewItem], title: &str) -> bool {
        items.iter().any(|i| matches!(i, ReviewItem::Header(h) if h == title))
    }

    fn has_warning(items: &[ReviewItem]) -> bool {
        items.iter().any(|i| matches!(i, ReviewItem::Warning(_)))
    }

    fn field_value<'a>(items: &'a [ReviewItem], label: &str) -> Option<&'a str> {
        items.iter().find_map(|item| match item {
            ReviewItem::Field { label: l, value } if l == label => Some(value.as_str()),
            _ => None,
        })
    }

    #[test]
    fn test_delegate() {
        let data = vec![2u8, 0, 0, 0]; // DelegateStake
        let accounts = [key(0x01), key(0x02)];
        let ix = parse(&data, &accounts);
        assert_eq!(ix.program, "Stake");
        assert!(has_header(&ix.items, "Delegate Stake"));
        assert!(field_value(&ix.items, "Stake").is_some());
        assert!(field_value(&ix.items, "Validator").is_some());
    }

    #[test]
    fn test_deactivate() {
        let data = vec![5u8, 0, 0, 0]; // Deactivate
        let accounts = [key(0x01)];
        let ix = parse(&data, &accounts);
        assert!(has_header(&ix.items, "Deactivate Stake"));
    }

    #[test]
    fn test_withdraw() {
        let mut data = vec![4u8, 0, 0, 0]; // Withdraw
        data.extend_from_slice(&2_000_000_000u64.to_le_bytes()); // 2 SOL
        let accounts = [key(0x01), key(0x02)];
        let ix = parse(&data, &accounts);
        assert!(has_header(&ix.items, "Withdraw Stake"));
        assert_eq!(field_value(&ix.items, "Amount"), Some("2 SOL"));
    }

    #[test]
    fn test_split() {
        let mut data = vec![3u8, 0, 0, 0]; // Split
        data.extend_from_slice(&500_000_000u64.to_le_bytes()); // 0.5 SOL
        let accounts = [key(0x01), key(0x02)];
        let ix = parse(&data, &accounts);
        assert!(has_header(&ix.items, "Split Stake"));
        assert_eq!(field_value(&ix.items, "Amount"), Some("0.5 SOL"));
    }

    #[test]
    fn test_initialize() {
        let data = vec![0u8, 0, 0, 0]; // Initialize
        let accounts = [key(0x01)];
        let ix = parse(&data, &accounts);
        assert!(has_header(&ix.items, "Initialize Stake"));
    }

    #[test]
    fn test_merge() {
        let data = vec![7u8, 0, 0, 0]; // Merge
        let accounts = [key(0x01), key(0x02)];
        let ix = parse(&data, &accounts);
        assert!(has_header(&ix.items, "Merge Stake"));
    }

    #[test]
    fn test_unknown_discriminant_does_not_panic() {
        let data = vec![99u8, 0, 0, 0];
        let ix = parse(&data, &[]);
        assert_eq!(ix.program, "Stake");
        assert!(!has_warning(&ix.items));
    }

    #[test]
    fn test_empty_data_returns_warning() {
        let ix = parse(&[], &[]);
        assert!(has_warning(&ix.items));
    }
}
