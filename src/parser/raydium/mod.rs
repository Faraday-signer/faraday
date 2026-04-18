//! Raydium DEX instruction parsers.
//!
//! Covers three Raydium programs:
//! - **AMM v4** — legacy constant-product AMM (non-Anchor, u8 discriminator)
//! - **CLMM** — concentrated liquidity market maker (Anchor)
//! - **CPMM** — constant-product v2 (Anchor)

pub mod amm_v4;
pub mod clmm;
pub mod cpmm;

use crate::parser::{ParsedInstruction, ReviewItem};
use crate::parser::token_registry;

// ── Shared types ────────────────────────────────────────────────────────────

pub struct SwapInfo {
    pub program_label: &'static str,
    pub variant: &'static str,
    pub in_amount: u64,
    pub out_amount: u64,
    pub in_label: &'static str,
    pub out_label: &'static str,
    pub source_mint: Option<[u8; 32]>,
    pub dest_mint: Option<[u8; 32]>,
}

// ── Shared helpers ──────────────────────────────────────────────────────────

pub fn format_swap(info: &SwapInfo) -> ParsedInstruction {
    let mut items = vec![
        ReviewItem::Header(format!("{} Swap", info.program_label)),
        ReviewItem::Field {
            label: "Type".into(),
            value: info.variant.into(),
        },
    ];
    items.extend(format_token_side(info.in_label, &info.source_mint, info.in_amount));
    items.extend(format_token_side(info.out_label, &info.dest_mint, info.out_amount));
    ParsedInstruction {
        program: info.program_label.into(),
        items,
    }
}

pub fn error(program: &str, msg: &str) -> ParsedInstruction {
    ParsedInstruction {
        program: program.into(),
        items: vec![
            ReviewItem::Header(program.into()),
            ReviewItem::Warning(msg.into()),
        ],
    }
}

pub fn get_account(
    account_indices: &[u8],
    idx: usize,
    all_accounts: &[[u8; 32]],
) -> Option<[u8; 32]> {
    let key_idx = *account_indices.get(idx)? as usize;
    all_accounts.get(key_idx).copied()
}

pub fn resolve_mint_via_ata(
    account_indices: &[u8],
    ta_idx: usize,
    all_accounts: &[[u8; 32]],
    ata_map: &token_registry::AtaMap,
) -> Option<[u8; 32]> {
    let ta = get_account(account_indices, ta_idx, all_accounts)?;
    ata_map.get(&ta).map(|e| e.mint)
}

fn format_token_side(label: &str, mint: &Option<[u8; 32]>, amount: u64) -> Vec<ReviewItem> {
    let mut items = Vec::new();
    match mint {
        Some(m) => match token_registry::lookup(m) {
            Some(info) => {
                let formatted = token_registry::format_amount(amount, info.decimals);
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!("{} {}", formatted, info.symbol),
                });
            }
            None => {
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!("{} raw units", amount),
                });
                items.push(ReviewItem::Field {
                    label: "Mint".into(),
                    value: pubkey_short(m),
                });
            }
        },
        None => {
            if amount > 0 {
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!("{} raw units", amount),
                });
            } else {
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: "? (could not parse)".into(),
                });
            }
            items.push(ReviewItem::Warning(
                "Mint unresolved — in lookup table".into(),
            ));
        }
    }
    items
}

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
}
