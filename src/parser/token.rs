//! SPL Token instruction parser (Token and Token-2022).
//!
//! Reference: https://docs.rs/spl-token/latest/spl_token/instruction/enum.TokenInstruction.html

use crate::parser::bytes::read_u64_le;
use crate::parser::token_registry;
use crate::parser::{ParsedInstruction, ReviewItem};

pub fn parse(program_name: &str, data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let items = match decode(data, accounts) {
        Ok(items) => items,
        Err(e) => vec![
            ReviewItem::Header(program_name.into()),
            ReviewItem::Warning(format!("Parse error: {}", e)),
        ],
    };
    ParsedInstruction {
        program: program_name.into(),
        items,
    }
}

fn decode(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let discriminant = *data.first().ok_or("Empty instruction data")?;

    match discriminant {
        3 => parse_transfer(&data[1..], accounts),
        4 => parse_approve(&data[1..], accounts),
        5 => parse_revoke(accounts),
        6 => parse_set_authority(&data[1..], accounts),
        7 => parse_mint_to(&data[1..], accounts),
        8 => parse_burn(&data[1..], accounts),
        9 => parse_close_account(accounts),
        12 => parse_transfer_checked(&data[1..], accounts),
        13 => parse_approve_checked(&data[1..], accounts),
        14 => parse_mint_to_checked(&data[1..], accounts),
        15 => parse_burn_checked(&data[1..], accounts),
        _ => Ok(vec![
            ReviewItem::Header("Token".into()),
            ReviewItem::Field {
                label: "Action".into(),
                value: format!("Type {}", discriminant),
            },
        ]),
    }
}

fn parse_transfer(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let source = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let dest = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Token Transfer".into()),
        ReviewItem::Field {
            label: "From".into(),
            value: source,
        },
        ReviewItem::Field {
            label: "To".into(),
            value: dest,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: amount.to_string(),
        },
        ReviewItem::Warning("Decimals unknown — verify amount".into()),
    ])
}

fn parse_transfer_checked(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let decimals = *data.get(8).ok_or("TransferChecked data too short")?;
    let source = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    // Resolve the mint twice: as raw bytes for the symbol lookup, and as a
    // shortened display string for the "Mint" review row. The lookup pulls
    // a known symbol (USDC, USDT, JUP, …) out of the offline registry so
    // the Amount row reads "0.99915 USDC" instead of bare "0.99915".
    let mint_bytes = accounts.get(1).copied();
    let mint_display = mint_bytes
        .as_ref()
        .map(|b| pubkey_short(b))
        .unwrap_or_else(|| "?".into());
    let symbol = mint_bytes
        .as_ref()
        .and_then(|b| token_registry::lookup(b))
        .map(|info| info.symbol);
    let dest = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    let amount_str = match symbol {
        Some(sym) => format!("{} {}", token_registry::format_amount(amount, decimals), sym),
        None => token_registry::format_amount(amount, decimals),
    };

    Ok(vec![
        ReviewItem::Header("Token Transfer".into()),
        ReviewItem::Field {
            label: "From".into(),
            value: source,
        },
        ReviewItem::Field {
            label: "To".into(),
            value: dest,
        },
        ReviewItem::Field {
            label: "Mint".into(),
            value: mint_display,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: amount_str,
        },
    ])
}

fn parse_approve(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let source = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let delegate = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Token Approve".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: source,
        },
        ReviewItem::Field {
            label: "Delegate".into(),
            value: delegate,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: amount.to_string(),
        },
        ReviewItem::Warning("Granting spend authority".into()),
    ])
}

fn parse_approve_checked(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let decimals = *data.get(8).ok_or("ApproveChecked data too short")?;
    let source = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let delegate = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Token Approve".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: source,
        },
        ReviewItem::Field {
            label: "Delegate".into(),
            value: delegate,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: token_registry::format_amount(amount, decimals),
        },
        ReviewItem::Warning("Granting spend authority".into()),
    ])
}

fn parse_revoke(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let source = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    Ok(vec![
        ReviewItem::Header("Token Revoke".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: source,
        },
    ])
}

fn parse_set_authority(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let authority_type = *data.first().ok_or("SetAuthority data too short")?;
    let account = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let authority_name = match authority_type {
        0 => "MintTokens",
        1 => "FreezeAccount",
        2 => "AccountOwner",
        3 => "CloseAccount",
        _ => "Unknown",
    };

    let new_authority = match data.get(1) {
        Some(0) => "None".to_string(),
        Some(1) if data.len() >= 34 => {
            let mut key = [0u8; 32];
            key.copy_from_slice(&data[2..34]);
            pubkey_short(&key)
        }
        Some(1) => "?".to_string(),
        _ => "?".to_string(),
    };

    Ok(vec![
        ReviewItem::Header("Set Authority".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: account,
        },
        ReviewItem::Field {
            label: "Authority".into(),
            value: authority_name.into(),
        },
        ReviewItem::Field {
            label: "New".into(),
            value: new_authority,
        },
        ReviewItem::Warning("Authority change — high risk".into()),
    ])
}

fn parse_mint_to(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let mint = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let dest = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Mint Tokens".into()),
        ReviewItem::Field {
            label: "Mint".into(),
            value: mint,
        },
        ReviewItem::Field {
            label: "To".into(),
            value: dest,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: amount.to_string(),
        },
    ])
}

fn parse_mint_to_checked(
    data: &[u8],
    accounts: &[[u8; 32]],
) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let decimals = *data.get(8).ok_or("MintToChecked data too short")?;
    let mint = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let dest = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Mint Tokens".into()),
        ReviewItem::Field {
            label: "Mint".into(),
            value: mint,
        },
        ReviewItem::Field {
            label: "To".into(),
            value: dest,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: token_registry::format_amount(amount, decimals),
        },
    ])
}

fn parse_burn(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let source = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let mint = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Burn Tokens".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: source,
        },
        ReviewItem::Field {
            label: "Mint".into(),
            value: mint,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: amount.to_string(),
        },
    ])
}

fn parse_burn_checked(data: &[u8], accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let amount = read_u64_le(data, 0)?;
    let decimals = *data.get(8).ok_or("BurnChecked data too short")?;
    let source = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let mint = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Burn Tokens".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: source,
        },
        ReviewItem::Field {
            label: "Mint".into(),
            value: mint,
        },
        ReviewItem::Field {
            label: "Amount".into(),
            value: token_registry::format_amount(amount, decimals),
        },
    ])
}

fn parse_close_account(accounts: &[[u8; 32]]) -> Result<Vec<ReviewItem>, &'static str> {
    let account = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    let dest = accounts
        .get(1)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());

    Ok(vec![
        ReviewItem::Header("Close Token Account".into()),
        ReviewItem::Field {
            label: "Account".into(),
            value: account,
        },
        ReviewItem::Field {
            label: "Rent to".into(),
            value: dest,
        },
    ])
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

    fn field_value<'a>(items: &'a [ReviewItem], label: &str) -> Option<&'a str> {
        items.iter().find_map(|item| match item {
            ReviewItem::Field { label: l, value } if l == label => Some(value.as_str()),
            _ => None,
        })
    }

    fn has_warning(items: &[ReviewItem]) -> bool {
        items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)))
    }

    // --- format_amount (now in token_registry) ---

    #[test]
    fn test_format_no_decimals() {
        assert_eq!(token_registry::format_amount(1000, 0), "1000");
    }

    #[test]
    fn test_format_with_decimals() {
        assert_eq!(token_registry::format_amount(1_000_000, 6), "1");
        assert_eq!(token_registry::format_amount(1_500_000, 6), "1.5");
        assert_eq!(token_registry::format_amount(1_000, 6), "0.001");
    }

    #[test]
    fn test_format_trims_trailing_zeros() {
        assert_eq!(token_registry::format_amount(1_100_000, 6), "1.1");
    }

    // --- Transfer ---

    #[test]
    fn test_transfer_basic() {
        let mut data = vec![3u8]; // Transfer discriminant
        data.extend_from_slice(&500u64.to_le_bytes());
        let accounts = [key(0x01), key(0x02), key(0x03)];
        let ix = parse("Token", &data, &accounts);
        assert_eq!(ix.program, "Token");
        assert_eq!(field_value(&ix.items, "Amount"), Some("500"));
        assert!(has_warning(&ix.items)); // "Decimals unknown"
    }

    // --- TransferChecked ---

    #[test]
    fn test_transfer_checked() {
        let mut data = vec![12u8]; // TransferChecked
        data.extend_from_slice(&1_500_000u64.to_le_bytes());
        data.push(6); // decimals
        let accounts = [key(0x01), key(0x02), key(0x03), key(0x04)];
        let ix = parse("Token", &data, &accounts);
        // Unknown mint (key(0x02) isn't USDC) → falls back to bare amount.
        assert_eq!(field_value(&ix.items, "Amount"), Some("1.5"));
        assert!(!has_warning(&ix.items)); // checked variant has decimals, no warning
    }

    #[test]
    fn test_transfer_checked_known_mint_appends_symbol() {
        // Using the canonical USDC mint as the second account (= the mint
        // slot in TransferChecked). Registry lookup should hit and append
        // "USDC" to the Amount field.
        let usdc_b58 = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let mut usdc_mint = [0u8; 32];
        let decoded = bs58::decode(usdc_b58).into_vec().expect("valid base58");
        usdc_mint.copy_from_slice(&decoded);

        let mut data = vec![12u8]; // TransferChecked
        data.extend_from_slice(&1_500_000u64.to_le_bytes());
        data.push(6); // USDC decimals
        let accounts = [key(0x01), usdc_mint, key(0x03), key(0x04)];
        let ix = parse("Token", &data, &accounts);
        assert_eq!(field_value(&ix.items, "Amount"), Some("1.5 USDC"));
    }

    // --- MintTo ---

    #[test]
    fn test_mint_to() {
        let mut data = vec![7u8]; // MintTo
        data.extend_from_slice(&1_000u64.to_le_bytes());
        let accounts = [key(0x01), key(0x02), key(0x03)];
        let ix = parse("Token", &data, &accounts);
        assert!(ix
            .items
            .iter()
            .any(|i| matches!(i, ReviewItem::Header(h) if h == "Mint Tokens")));
        assert_eq!(field_value(&ix.items, "Amount"), Some("1000"));
    }

    // --- Burn ---

    #[test]
    fn test_burn() {
        let mut data = vec![8u8]; // Burn
        data.extend_from_slice(&200u64.to_le_bytes());
        let accounts = [key(0x01), key(0x02), key(0x03)];
        let ix = parse("Token", &data, &accounts);
        assert!(ix
            .items
            .iter()
            .any(|i| matches!(i, ReviewItem::Header(h) if h == "Burn Tokens")));
    }

    // --- CloseAccount ---

    #[test]
    fn test_close_account() {
        let data = vec![9u8]; // CloseAccount
        let accounts = [key(0x01), key(0x02), key(0x03)];
        let ix = parse("Token", &data, &accounts);
        assert!(ix
            .items
            .iter()
            .any(|i| matches!(i, ReviewItem::Header(h) if h == "Close Token Account")));
    }

    // --- Approve ---

    #[test]
    fn test_approve_has_warning() {
        let mut data = vec![4u8]; // Approve
        data.extend_from_slice(&1_000u64.to_le_bytes());
        let accounts = [key(0x01), key(0x02), key(0x03)];
        let ix = parse("Token", &data, &accounts);
        assert!(
            has_warning(&ix.items),
            "Approve should warn about delegating spend authority"
        );
    }

    #[test]
    fn test_set_authority_has_warning() {
        let mut data = vec![6u8]; // SetAuthority
        data.push(2u8); // AccountOwner
        data.push(0u8); // COption::None
        let accounts = [key(0x01), key(0x02), key(0x03)];
        let ix = parse("Token", &data, &accounts);
        assert!(ix
            .items
            .iter()
            .any(|i| matches!(i, ReviewItem::Header(h) if h == "Set Authority")));
        assert!(has_warning(&ix.items));
    }

    // --- Token-2022 program name passthrough ---

    #[test]
    fn test_token_2022_program_name() {
        let mut data = vec![3u8];
        data.extend_from_slice(&100u64.to_le_bytes());
        let ix = parse("Token-2022", &data, &[key(0x01), key(0x02), key(0x03)]);
        assert_eq!(ix.program, "Token-2022");
    }

    // --- Error cases ---

    #[test]
    fn test_empty_data_returns_warning() {
        let ix = parse("Token", &[], &[]);
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_transfer_too_short_returns_warning() {
        let data = vec![3u8, 0, 0]; // Transfer needs 9 bytes total
        let ix = parse("Token", &data, &[]);
        assert!(has_warning(&ix.items));
    }
}
