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
    if data.len() < 4 { return Err("Instruction data too short"); }
    let ix_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    match ix_type {
        0 => parse_initialize(accounts),
        2 => parse_delegate(accounts),
        3 => parse_split(&data[4..], accounts),
        4 => parse_withdraw(&data[4..], accounts),
        5 => parse_deactivate(accounts),
        7 => parse_merge(accounts),
        _ => Ok(vec![
            ReviewItem::Header("Stake".into()),
            ReviewItem::Field { label: "Action".into(), value: format!("Type {}", ix_type) },
        ]),
    }
}

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
