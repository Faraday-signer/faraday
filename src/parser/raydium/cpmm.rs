//! Raydium CPMM (Constant Product Market Maker v2) instruction parser.
//!
//! Anchor-based program with 8-byte discriminators.
//! Program ID: CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C

use crate::parser::anchor;
use crate::parser::bytes::{read_disc8, read_u64_le};
use crate::parser::raydium::{self, SwapInfo};
use crate::parser::token_registry::AtaMap;
use crate::parser::{ParsedInstruction, ReviewItem};

// Mints directly in accounts
const INPUT_MINT_IDX: usize = 10;
const OUTPUT_MINT_IDX: usize = 11;

pub fn parse(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    _ata_map: &AtaMap,
) -> ParsedInstruction {
    let disc = match read_disc8(data, 0) {
        Ok(d) => d,
        Err(_) => return raydium::error("Raydium CPMM", "Instruction data too short"),
    };

    if disc == anchor::discriminator("swap_base_input") {
        parse_swap(data, account_indices, all_accounts, true)
    } else if disc == anchor::discriminator("swap_base_output") {
        parse_swap(data, account_indices, all_accounts, false)
    } else {
        ParsedInstruction {
            program: "Raydium CPMM".into(),
            items: vec![ReviewItem::Header("Raydium CPMM".into())],
        }
    }
}

fn parse_swap(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    is_base_input: bool,
) -> ParsedInstruction {
    // disc(8) + amount_a(8) + amount_b(8) = 24
    let (amount_a, amount_b) = match (read_u64_le(data, 8), read_u64_le(data, 16)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return raydium::error("Raydium CPMM", "Swap data too short"),
    };

    let source_mint = raydium::get_account(account_indices, INPUT_MINT_IDX, all_accounts);
    let dest_mint = raydium::get_account(account_indices, OUTPUT_MINT_IDX, all_accounts);

    let (in_label, out_label) = if is_base_input {
        ("You spend", "You receive (min)")
    } else {
        ("You spend (max)", "You receive")
    };

    raydium::format_swap(&SwapInfo {
        program_label: "Raydium CPMM",
        variant: if is_base_input {
            "swap_base_input"
        } else {
            "swap_base_output"
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

    fn build_swap_data(name: &str, amount_a: u64, amount_b: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor::discriminator(name));
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

    fn sol_mint() -> [u8; 32] {
        let bytes = bs58::decode("So11111111111111111111111111111111111111112")
            .into_vec()
            .unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        key
    }

    fn usdc_mint() -> [u8; 32] {
        let bytes = bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .into_vec()
            .unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        key
    }

    #[test]
    fn test_swap_base_input_with_mints() {
        let data = build_swap_data("swap_base_input", 1_000_000_000, 150_000_000);
        let sol = sol_mint();
        let usdc = usdc_mint();
        let dummy = [0xAA; 32];
        let mut accounts = vec![dummy; 13];
        accounts[10] = sol;
        accounts[11] = usdc;
        let indices: Vec<u8> = (0..13).collect();
        let ix = parse(&data, &indices, &accounts, &AtaMap::new());
        assert!(has_header(&ix.items, "Raydium CPMM Swap"));
        assert_eq!(field_value(&ix.items, "Type"), Some("swap_base_input"));
        assert_eq!(field_value(&ix.items, "You spend"), Some("1 SOL"));
        assert_eq!(
            field_value(&ix.items, "You receive (min)"),
            Some("150 USDC")
        );
        assert!(!has_warning(&ix.items));
    }

    #[test]
    fn test_swap_base_output() {
        let data = build_swap_data("swap_base_output", 2_000_000_000, 100_000_000);
        let sol = sol_mint();
        let usdc = usdc_mint();
        let dummy = [0xBB; 32];
        let mut accounts = vec![dummy; 13];
        accounts[10] = sol;
        accounts[11] = usdc;
        let indices: Vec<u8> = (0..13).collect();
        let ix = parse(&data, &indices, &accounts, &AtaMap::new());
        assert_eq!(field_value(&ix.items, "Type"), Some("swap_base_output"));
        assert_eq!(field_value(&ix.items, "You spend (max)"), Some("2 SOL"));
        assert_eq!(field_value(&ix.items, "You receive"), Some("100 USDC"));
    }

    #[test]
    fn test_too_short_data() {
        let ix = parse(&[0u8; 4], &[], &[], &AtaMap::new());
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_unknown_instruction() {
        let ix = parse(&[0xFF; 8], &[], &[], &AtaMap::new());
        assert!(has_header(&ix.items, "Raydium CPMM"));
        assert!(!has_warning(&ix.items));
    }
}
