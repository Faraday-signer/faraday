//! System Program instruction parser.
//!
//! Reference: https://docs.rs/solana-sdk/latest/solana_sdk/system_instruction/enum.SystemInstruction.html

use crate::parser::{ParsedInstruction, ReviewItem};

pub fn parse(data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let items = match decode(data, accounts) {
        Ok(items) => items,
        Err(e) => vec![
            ReviewItem::Header("System".into()),
            ReviewItem::Warning(format!("Parse error: {}", e)),
        ],
    };
    ParsedInstruction { program: "System".into(), items }
}

fn decode(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 4 { return Err("Instruction data too short"); }
    let ix_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    match ix_type {
        0 => parse_create_account(&data[4..], accounts),
        2 => parse_transfer(&data[4..], accounts),
        3 => parse_create_account_with_seed(&data[4..], accounts),
        8 => parse_allocate(&data[4..], accounts),
        11 => parse_transfer_with_seed(&data[4..], accounts),
        _ => Ok(vec![
            ReviewItem::Header("System".into()),
            ReviewItem::Field { label: "Action".into(), value: format!("Type {}", ix_type) },
        ]),
    }
}

fn parse_transfer(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("Transfer data too short"); }
    let lamports = u64::from_le_bytes(data[..8].try_into().unwrap());

    let from = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let to = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("SOL Transfer".into()),
        ReviewItem::Field { label: "From".into(), value: from },
        ReviewItem::Field { label: "To".into(), value: to },
        ReviewItem::Field { label: "Amount".into(), value: lamports_to_sol(lamports) },
    ])
}

fn parse_create_account(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 16 { return Err("CreateAccount data too short"); }
    let lamports = u64::from_le_bytes(data[..8].try_into().unwrap());
    let space = u64::from_le_bytes(data[8..16].try_into().unwrap());

    let funder = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let new_account = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Create Account".into()),
        ReviewItem::Field { label: "Funder".into(), value: funder },
        ReviewItem::Field { label: "New account".into(), value: new_account },
        ReviewItem::Field { label: "Rent".into(), value: lamports_to_sol(lamports) },
        ReviewItem::Field { label: "Space".into(), value: format!("{} bytes", space) },
    ])
}

fn parse_create_account_with_seed(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let funder = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let new_account = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    // base: 32 bytes, seed_len: u64, seed: variable
    if data.len() < 40 { return Err("CreateAccountWithSeed data too short"); }
    let seed_len = u64::from_le_bytes(data[32..40].try_into().unwrap()) as usize;
    if data.len() < 40 + seed_len + 8 { return Err("CreateAccountWithSeed truncated"); }
    let seed = std::str::from_utf8(&data[40..40 + seed_len]).unwrap_or("?");
    let lamports_offset = 40 + seed_len;
    let lamports = u64::from_le_bytes(data[lamports_offset..lamports_offset + 8].try_into().unwrap());

    Ok(vec![
        ReviewItem::Header("Create Account (seed)".into()),
        ReviewItem::Field { label: "Funder".into(), value: funder },
        ReviewItem::Field { label: "New account".into(), value: new_account },
        ReviewItem::Field { label: "Seed".into(), value: seed.to_string() },
        ReviewItem::Field { label: "Rent".into(), value: lamports_to_sol(lamports) },
    ])
}

fn parse_allocate(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("Allocate data too short"); }
    let space = u64::from_le_bytes(data[..8].try_into().unwrap());
    let account = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Allocate".into()),
        ReviewItem::Field { label: "Account".into(), value: account },
        ReviewItem::Field { label: "Space".into(), value: format!("{} bytes", space) },
    ])
}

fn parse_transfer_with_seed(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("TransferWithSeed data too short"); }
    let lamports = u64::from_le_bytes(data[..8].try_into().unwrap());
    let from = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let to = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("SOL Transfer (seed)".into()),
        ReviewItem::Field { label: "From".into(), value: from },
        ReviewItem::Field { label: "To".into(), value: to },
        ReviewItem::Field { label: "Amount".into(), value: lamports_to_sol(lamports) },
    ])
}

pub fn lamports_to_sol(lamports: u64) -> String {
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
