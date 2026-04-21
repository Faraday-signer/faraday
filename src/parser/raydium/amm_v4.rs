//! Raydium AMM v4 instruction parser.
//!
//! Legacy constant-product AMM with u8 instruction discriminator (non-Anchor).
//! Program ID: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8

use crate::parser::bytes::read_u64_le;
use crate::parser::token_registry::AtaMap;
use crate::parser::raydium::{self, SwapInfo};
use crate::parser::{ParsedInstruction, ReviewItem};

const SWAP_BASE_IN: u8 = 9;
const SWAP_BASE_OUT: u8 = 11;

// User token account positions in the standard 18-account swap layout
const USER_SOURCE_IDX: usize = 15;
const USER_DEST_IDX: usize = 16;

pub fn parse(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    if data.is_empty() {
        return raydium::error("Raydium AMM", "Instruction data empty");
    }

    match data[0] {
        SWAP_BASE_IN => parse_swap(data, account_indices, all_accounts, ata_map, true),
        SWAP_BASE_OUT => parse_swap(data, account_indices, all_accounts, ata_map, false),
        other => ParsedInstruction {
            program: "Raydium AMM".into(),
            items: vec![
                ReviewItem::Header("Raydium AMM".into()),
                ReviewItem::Field {
                    label: "Instruction".into(),
                    value: format!("#{}", other),
                },
            ],
        },
    }
}

fn parse_swap(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
    is_base_in: bool,
) -> ParsedInstruction {
    // Data layout: discriminator(1) + amount_a(8) + amount_b(8) = 17 bytes
    let (amount_a, amount_b) = match (read_u64_le(data, 1), read_u64_le(data, 9)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return raydium::error("Raydium AMM", "Swap data too short"),
    };

    let source_mint =
        raydium::resolve_mint_via_ata(account_indices, USER_SOURCE_IDX, all_accounts, ata_map);
    let dest_mint =
        raydium::resolve_mint_via_ata(account_indices, USER_DEST_IDX, all_accounts, ata_map);

    let (in_label, out_label) = if is_base_in {
        ("You spend", "You receive (min)")
    } else {
        ("You spend (max)", "You receive")
    };

    raydium::format_swap(&SwapInfo {
        program_label: "Raydium AMM",
        variant: if is_base_in {
            "swap_base_in"
        } else {
            "swap_base_out"
        },
        in_amount: amount_a,
        out_amount: amount_b,
        in_label,
        out_label,
        source_mint,
        dest_mint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_swap_data(disc: u8, amount_a: u64, amount_b: u64) -> Vec<u8> {
        let mut data = vec![disc];
        data.extend_from_slice(&amount_a.to_le_bytes());
        data.extend_from_slice(&amount_b.to_le_bytes());
        data
    }

    fn field_value<'a>(items: &'a [ReviewItem], label: &str) -> Option<&'a str> {
        items.iter().find_map(|item| match item {
            ReviewItem::Field {
                label: l,
                value: v,
            } if l == label => Some(v.as_str()),
            _ => None,
        })
    }

    fn has_warning(items: &[ReviewItem]) -> bool {
        items.iter().any(|i| matches!(i, ReviewItem::Warning(_)))
    }

    fn has_header(items: &[ReviewItem], text: &str) -> bool {
        items
            .iter()
            .any(|i| matches!(i, ReviewItem::Header(h) if h == text))
    }

    #[test]
    fn test_swap_base_in() {
        let data = build_swap_data(9, 1_000_000_000, 50_000_000);
        let dummy = [0xAA; 32];
        let accounts = vec![dummy; 18];
        let indices: Vec<u8> = (0..18).collect();
        let ix = parse(&data, &indices, &accounts, &AtaMap::new());
        assert!(has_header(&ix.items, "Raydium AMM Swap"));
        assert_eq!(field_value(&ix.items, "Type"), Some("swap_base_in"));
    }

    #[test]
    fn test_swap_base_out() {
        let data = build_swap_data(11, 2_000_000_000, 100_000_000);
        let dummy = [0xAA; 32];
        let accounts = vec![dummy; 18];
        let indices: Vec<u8> = (0..18).collect();
        let ix = parse(&data, &indices, &accounts, &AtaMap::new());
        assert_eq!(field_value(&ix.items, "Type"), Some("swap_base_out"));
    }

    #[test]
    fn test_empty_data() {
        let ix = parse(&[], &[], &[], &AtaMap::new());
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_too_short_swap_data() {
        let ix = parse(&[9, 0, 0], &[], &[], &AtaMap::new());
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_unknown_instruction_shows_number() {
        let data = vec![99; 17];
        let ix = parse(&data, &[], &[], &AtaMap::new());
        assert!(has_header(&ix.items, "Raydium AMM"));
        assert_eq!(field_value(&ix.items, "Instruction"), Some("#99"));
    }
}
