//! SPL Token instruction parser (Token and Token-2022).
//!
//! Reference: https://docs.rs/spl-token/latest/spl_token/instruction/enum.TokenInstruction.html

use crate::parser::{ParsedInstruction, ReviewItem};

pub fn parse(program_name: &str, data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let items = match decode(data, accounts) {
        Ok(items) => items,
        Err(e) => vec![
            ReviewItem::Header(program_name.into()),
            ReviewItem::Warning(format!("Parse error: {}", e)),
        ],
    };
    ParsedInstruction { program: program_name.into(), items }
}

fn decode(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let discriminant = *data.first().ok_or("Empty instruction data")?;

    match discriminant {
        3 => parse_transfer(&data[1..], accounts),
        4 => parse_approve(&data[1..], accounts),
        5 => parse_revoke(accounts),
        7 => parse_mint_to(&data[1..], accounts),
        8 => parse_burn(&data[1..], accounts),
        9 => parse_close_account(accounts),
        12 => parse_transfer_checked(&data[1..], accounts),
        13 => parse_approve_checked(&data[1..], accounts),
        14 => parse_mint_to_checked(&data[1..], accounts),
        15 => parse_burn_checked(&data[1..], accounts),
        _ => Ok(vec![
            ReviewItem::Header("Token".into()),
            ReviewItem::Field { label: "Action".into(), value: format!("Type {}", discriminant) },
        ]),
    }
}

fn parse_transfer(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("Transfer data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let dest = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Token Transfer".into()),
        ReviewItem::Field { label: "From".into(), value: source },
        ReviewItem::Field { label: "To".into(), value: dest },
        ReviewItem::Field { label: "Amount".into(), value: amount.to_string() },
        ReviewItem::Warning("Decimals unknown — verify amount".into()),
    ])
}

fn parse_transfer_checked(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 9 { return Err("TransferChecked data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let decimals = data[8];
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let mint = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());
    let dest = accounts.get(2).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Token Transfer".into()),
        ReviewItem::Field { label: "From".into(), value: source },
        ReviewItem::Field { label: "To".into(), value: dest },
        ReviewItem::Field { label: "Mint".into(), value: mint },
        ReviewItem::Field { label: "Amount".into(), value: format_token_amount(amount, decimals) },
    ])
}

fn parse_approve(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("Approve data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let delegate = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Token Approve".into()),
        ReviewItem::Field { label: "Account".into(), value: source },
        ReviewItem::Field { label: "Delegate".into(), value: delegate },
        ReviewItem::Field { label: "Amount".into(), value: amount.to_string() },
        ReviewItem::Warning("Granting spend authority".into()),
    ])
}

fn parse_approve_checked(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 9 { return Err("ApproveChecked data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let decimals = data[8];
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let delegate = accounts.get(2).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Token Approve".into()),
        ReviewItem::Field { label: "Account".into(), value: source },
        ReviewItem::Field { label: "Delegate".into(), value: delegate },
        ReviewItem::Field { label: "Amount".into(), value: format_token_amount(amount, decimals) },
        ReviewItem::Warning("Granting spend authority".into()),
    ])
}

fn parse_revoke(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    Ok(vec![
        ReviewItem::Header("Token Revoke".into()),
        ReviewItem::Field { label: "Account".into(), value: source },
    ])
}

fn parse_mint_to(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("MintTo data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let mint = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let dest = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Mint Tokens".into()),
        ReviewItem::Field { label: "Mint".into(), value: mint },
        ReviewItem::Field { label: "To".into(), value: dest },
        ReviewItem::Field { label: "Amount".into(), value: amount.to_string() },
    ])
}

fn parse_mint_to_checked(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 9 { return Err("MintToChecked data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let decimals = data[8];
    let mint = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let dest = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Mint Tokens".into()),
        ReviewItem::Field { label: "Mint".into(), value: mint },
        ReviewItem::Field { label: "To".into(), value: dest },
        ReviewItem::Field { label: "Amount".into(), value: format_token_amount(amount, decimals) },
    ])
}

fn parse_burn(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 8 { return Err("Burn data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let mint = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Burn Tokens".into()),
        ReviewItem::Field { label: "Account".into(), value: source },
        ReviewItem::Field { label: "Mint".into(), value: mint },
        ReviewItem::Field { label: "Amount".into(), value: amount.to_string() },
    ])
}

fn parse_burn_checked(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    if data.len() < 9 { return Err("BurnChecked data too short"); }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
    let decimals = data[8];
    let source = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let mint = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Burn Tokens".into()),
        ReviewItem::Field { label: "Account".into(), value: source },
        ReviewItem::Field { label: "Mint".into(), value: mint },
        ReviewItem::Field { label: "Amount".into(), value: format_token_amount(amount, decimals) },
    ])
}

fn parse_close_account(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let account = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let dest = accounts.get(1).map(pubkey_short).unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Close Token Account".into()),
        ReviewItem::Field { label: "Account".into(), value: account },
        ReviewItem::Field { label: "Rent to".into(), value: dest },
    ])
}

fn format_token_amount(amount: u64, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }
    let divisor = 10u64.pow(decimals as u32);
    let whole = amount / divisor;
    let frac = amount % divisor;
    let frac_str = format!("{:0width$}", frac, width = decimals as usize);
    format!("{}.{}", whole, frac_str.trim_end_matches('0'))
}

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
}
