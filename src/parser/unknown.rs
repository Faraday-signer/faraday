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
