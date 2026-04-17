//! Fallback parser for unrecognized programs.
//!
//! Shows the program ID and raw data so the user can make an informed decision
//! rather than signing a black box.

use crate::parser::{ParsedInstruction, ReviewItem};

pub fn parse(program_id: &[u8; 32], data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let program_b58 = bs58::encode(program_id).into_string();
    let program_short = format!("{}..{}", &program_b58[..4], &program_b58[program_b58.len() - 4..]);

    let mut items = vec![
        ReviewItem::Header("Unknown Program".into()),
        ReviewItem::Warning("Unrecognized program — review carefully".into()),
        ReviewItem::Field { label: "Program".into(), value: program_b58 },
        ReviewItem::Field {
            label: "Accounts".into(),
            value: format!("{}", accounts.len()),
        },
    ];

    // Show first 16 bytes of instruction data as hex
    if !data.is_empty() {
        let preview_len = data.len().min(16);
        let hex: String = data[..preview_len].iter().map(|b| format!("{:02x}", b)).collect();
        let suffix = if data.len() > 16 { format!("... ({} bytes)", data.len()) } else { String::new() };
        items.push(ReviewItem::Field {
            label: "Data".into(),
            value: format!("{}{}", hex, suffix),
        });
    }

    // Show first 3 account addresses
    for (i, account) in accounts.iter().take(3).enumerate() {
        let b58 = bs58::encode(account).into_string();
        items.push(ReviewItem::Field {
            label: format!("Acct {}", i),
            value: format!("{}..{}", &b58[..4], &b58[b58.len() - 4..]),
        });
    }
    if accounts.len() > 3 {
        items.push(ReviewItem::Field {
            label: String::new(),
            value: format!("... {} more", accounts.len() - 3),
        });
    }

    ParsedInstruction { program: format!("Unknown ({})", program_short), items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shows_warning() {
        let ix = parse(&[0x42u8; 32], &[], &[]);
        let has_warning = ix.items.iter().any(|i| matches!(i, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    #[test]
    fn test_shows_program_id_in_name() {
        let program_id = [0x42u8; 32];
        let ix = parse(&program_id, &[], &[]);
        let b58 = bs58::encode(&program_id).into_string();
        // Full program ID should appear in a Field item
        let has_full_id = ix.items.iter().any(|i| matches!(
            i, ReviewItem::Field { value, .. } if value == &b58
        ));
        assert!(has_full_id);
    }

    #[test]
    fn test_shows_data_hex() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let ix = parse(&[0u8; 32], &data, &[]);
        let has_data = ix.items.iter().any(|i| matches!(
            i, ReviewItem::Field { label, value } if label == "Data" && value.contains("deadbeef")
        ));
        assert!(has_data);
    }

    #[test]
    fn test_truncates_long_data() {
        let data = vec![0xABu8; 32]; // 32 bytes — over the 16-byte preview limit
        let ix = parse(&[0u8; 32], &data, &[]);
        let data_field = ix.items.iter().find_map(|i| match i {
            ReviewItem::Field { label, value } if label == "Data" => Some(value.as_str()),
            _ => None,
        });
        assert!(data_field.unwrap().contains("32 bytes"));
    }

    #[test]
    fn test_shows_up_to_three_accounts() {
        let accounts = [[0x01u8; 32], [0x02u8; 32], [0x03u8; 32], [0x04u8; 32]];
        let ix = parse(&[0u8; 32], &[], &accounts);
        let acct_fields = ix.items.iter().filter(|i| matches!(
            i, ReviewItem::Field { label, .. } if label.starts_with("Acct")
        )).count();
        assert_eq!(acct_fields, 3);
        // Should indicate there's one more
        let has_more = ix.items.iter().any(|i| matches!(
            i, ReviewItem::Field { value, .. } if value.contains("1 more")
        ));
        assert!(has_more);
    }

    #[test]
    fn test_empty_data_no_data_field() {
        let ix = parse(&[0u8; 32], &[], &[]);
        let has_data_field = ix.items.iter().any(|i| matches!(
            i, ReviewItem::Field { label, .. } if label == "Data"
        ));
        assert!(!has_data_field);
    }
}
