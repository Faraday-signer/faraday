//! Raydium CLMM (Concentrated Liquidity Market Maker) instruction parser.
//!
//! Anchor-based program with 8-byte discriminators.
//! Program ID: CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK

use crate::parser::anchor;
use crate::parser::bytes::{read_disc8, read_u64_le};
use crate::parser::raydium::{self, SwapInfo};
use crate::parser::token_registry::AtaMap;
use crate::parser::{ParsedInstruction, ReviewItem};

// swap v1: no mints in accounts, resolve via ATA from user token accounts
const SWAP_USER_INPUT_IDX: usize = 3;
const SWAP_USER_OUTPUT_IDX: usize = 4;

// swap_v2: mints directly in accounts
const SWAP_V2_INPUT_MINT_IDX: usize = 11;
const SWAP_V2_OUTPUT_MINT_IDX: usize = 12;

pub fn parse(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    let disc = match read_disc8(data, 0) {
        Ok(d) => d,
        Err(_) => return raydium::error("Raydium CLMM", "Instruction data too short"),
    };

    if disc == anchor::discriminator("swap") {
        parse_swap(data, account_indices, all_accounts, ata_map, false)
    } else if disc == anchor::discriminator("swap_v2") {
        parse_swap(data, account_indices, all_accounts, ata_map, true)
    } else {
        ParsedInstruction {
            program: "Raydium CLMM".into(),
            items: vec![ReviewItem::Header("Raydium CLMM".into())],
        }
    }
}

fn parse_swap(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
    is_v2: bool,
) -> ParsedInstruction {
    // disc(8) + amount(8) + other_amount_threshold(8) + sqrt_price_limit_x64(16) + is_base_input(1) = 41
    let amount = match read_u64_le(data, 8) {
        Ok(n) => n,
        Err(_) => return raydium::error("Raydium CLMM", "Swap data too short"),
    };
    let threshold = match read_u64_le(data, 16) {
        Ok(n) => n,
        Err(_) => return raydium::error("Raydium CLMM", "Swap data too short"),
    };
    // data[24..40] = sqrt_price_limit_x64 (u128, not displayed)
    let is_base_input = match data.get(40) {
        Some(&b) => b != 0,
        None => return raydium::error("Raydium CLMM", "Swap data too short"),
    };

    let (source_mint, dest_mint) = if is_v2 {
        (
            raydium::get_account(account_indices, SWAP_V2_INPUT_MINT_IDX, all_accounts),
            raydium::get_account(account_indices, SWAP_V2_OUTPUT_MINT_IDX, all_accounts),
        )
    } else {
        (
            raydium::resolve_mint_via_ata(
                account_indices,
                SWAP_USER_INPUT_IDX,
                all_accounts,
                ata_map,
            ),
            raydium::resolve_mint_via_ata(
                account_indices,
                SWAP_USER_OUTPUT_IDX,
                all_accounts,
                ata_map,
            ),
        )
    };

    let (in_amount, out_amount, in_label, out_label) = if is_base_input {
        (amount, threshold, "You spend", "You receive (min)")
    } else {
        (threshold, amount, "You spend (max)", "You receive")
    };

    raydium::format_swap(&SwapInfo {
        program_label: "Raydium CLMM",
        variant: if is_v2 { "swap_v2" } else { "swap" },
        in_amount,
        out_amount,
        in_label,
        out_label,
        source_mint,
        dest_mint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_swap_data(name: &str, amount: u64, threshold: u64, is_base_input: bool) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor::discriminator(name));
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&threshold.to_le_bytes());
        data.extend_from_slice(&0u128.to_le_bytes()); // sqrt_price_limit_x64
        data.push(u8::from(is_base_input));
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
    fn test_swap_base_input() {
        let data = build_swap_data("swap", 1_000_000_000, 50_000_000, true);
        let dummy = [0xAA; 32];
        let accounts = vec![dummy; 13];
        let indices: Vec<u8> = (0..13).collect();
        let ix = parse(&data, &indices, &accounts, &AtaMap::new());
        assert!(has_header(&ix.items, "Raydium CLMM Swap"));
        assert_eq!(field_value(&ix.items, "Type"), Some("swap"));
    }

    #[test]
    fn test_swap_base_output_flips_amounts() {
        let data = build_swap_data("swap", 100_000_000, 2_000_000_000, false);
        let dummy = [0xAA; 32];
        let accounts = vec![dummy; 13];
        let indices: Vec<u8> = (0..13).collect();
        let ix = parse(&data, &indices, &accounts, &AtaMap::new());
        // is_base_input=false: in=threshold(2B), out=amount(100M)
        assert!(field_value(&ix.items, "You spend (max)").is_some());
        assert!(field_value(&ix.items, "You receive").is_some());
    }

    #[test]
    fn test_swap_v2_with_explicit_mints() {
        let data = build_swap_data("swap_v2", 1_000_000_000, 150_000_000, true);
        let sol = sol_mint();
        let usdc = usdc_mint();
        let dummy = [0xBB; 32];
        let mut accounts = vec![dummy; 13];
        accounts[11] = sol;
        accounts[12] = usdc;
        let indices: Vec<u8> = (0..13).collect();
        let ix = parse(&data, &indices, &accounts, &AtaMap::new());
        assert_eq!(field_value(&ix.items, "Type"), Some("swap_v2"));
        assert_eq!(field_value(&ix.items, "You spend"), Some("1 SOL"));
        assert_eq!(
            field_value(&ix.items, "You receive (min)"),
            Some("150 USDC")
        );
        assert!(!has_warning(&ix.items));
    }

    #[test]
    fn test_too_short_data() {
        let ix = parse(&[0u8; 4], &[], &[], &AtaMap::new());
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_unknown_instruction() {
        let ix = parse(&[0xFF; 8], &[], &[], &AtaMap::new());
        assert!(has_header(&ix.items, "Raydium CLMM"));
        assert!(!has_warning(&ix.items));
    }
}
