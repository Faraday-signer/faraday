//! Jupiter Ultra `iris` router (`proVF4pMXVaYqmy4NjniPh4pqKNfMmsihgd4wdkCX3u`).
//!
//! Layout for the `swap` instruction (disc `aa2955b184501f35`), confirmed
//! against multiple real txs:
//!
//! ```text
//!   [0..8]    discriminator
//!   [8..16]   opaque (quote id / deadline / fee — not decoded)
//!   [16..24]  u64 in_amount
//!   [24..32]  u64 minimum_out_amount
//!   [32..34]  u16 slippage_bps
//!   [34..]    route plan + remaining-accounts info
//! ```
//!
//! Source/dest mints come from the ATA map (the iris ix references the
//! user's source and dest ATAs in its account list).

use crate::parser::bytes::{pubkey_short, read_disc8, read_u16_le, read_u64_le};
use crate::parser::token_registry::{self, AtaMap};
use crate::parser::{ParsedInstruction, ReviewItem};

const SWAP_DISC: [u8; 8] = [0xaa, 0x29, 0x55, 0xb1, 0x84, 0x50, 0x1f, 0x35];

const PLAUSIBLE_AMOUNT_CAP: u64 = 10_000_000_000_000_000;

pub fn parse(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    let disc = match read_disc8(data, 0) {
        Ok(d) => d,
        Err(_) => return error_ix("Jupiter Ultra data too short for discriminator"),
    };

    if disc == SWAP_DISC {
        return parse_swap(data, account_indices, all_accounts, ata_map);
    }

    ParsedInstruction {
        program: "Jupiter Ultra".into(),
        items: vec![
            ReviewItem::Header(format!(
                "Jupiter Ultra: unknown action ({:02x}{:02x}{:02x}{:02x}…)",
                disc[0], disc[1], disc[2], disc[3]
            )),
            ReviewItem::Warning(
                "Decoder doesn't recognize this Jupiter Ultra instruction yet.".into(),
            ),
        ],
    }
}

fn parse_swap(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    let mut items: Vec<ReviewItem> = Vec::new();
    items.push(ReviewItem::Header("Jupiter Ultra swap".into()));

    let in_amount = read_u64_le(data, 16).ok();
    let out_amount = read_u64_le(data, 24).ok();
    let plausible = matches!(
        (in_amount, out_amount),
        (Some(i), Some(o))
            if i > 0 && o > 0 && i < PLAUSIBLE_AMOUNT_CAP && o < PLAUSIBLE_AMOUNT_CAP
    );

    let (source_mint, dest_mint) = resolve_user_mints(account_indices, all_accounts, ata_map);

    if plausible {
        let slip = read_u16_le(data, 32).unwrap_or(0);
        let in_a = in_amount.unwrap();
        let quoted_out = out_amount.unwrap();
        let min_out = ((quoted_out as u128 * (10_000_u64 - slip as u64) as u128) / 10_000) as u64;
        items.extend(format_token_side("You spend", source_mint, in_a));
        items.extend(format_token_side("You receive (min)", dest_mint, min_out));
        items.push(ReviewItem::Field {
            label: "Slippage".into(),
            value: format!("{:.2}%", slip as f32 / 100.0),
        });
        items.push(ReviewItem::Field {
            label: "Slippage_bps".into(),
            value: slip.to_string(),
        });
    } else {
        items.push(ReviewItem::Warning("Amounts not decoded.".into()));
    }

    ParsedInstruction {
        program: "Jupiter Ultra".into(),
        items,
    }
}

fn error_ix(msg: &'static str) -> ParsedInstruction {
    ParsedInstruction {
        program: "Jupiter Ultra".into(),
        items: vec![ReviewItem::Warning(msg.into())],
    }
}

fn resolve_user_mints(
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> (Option<[u8; 32]>, Option<[u8; 32]>) {
    let mut found: Vec<[u8; 32]> = Vec::new();
    for &idx in account_indices {
        let Some(acct) = all_accounts.get(idx as usize) else {
            continue;
        };
        if let Some(entry) = ata_map.get(acct) {
            if !found.contains(&entry.mint) {
                found.push(entry.mint);
                if found.len() == 2 {
                    break;
                }
            }
        }
    }
    let mut iter = found.into_iter();
    (iter.next(), iter.next())
}

fn format_token_side(label: &str, mint: Option<[u8; 32]>, amount: u64) -> Vec<ReviewItem> {
    let mut items = Vec::new();
    match mint {
        Some(m) => match token_registry::lookup(&m) {
            Some(info) => {
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!(
                        "{} {}",
                        token_registry::format_amount(amount, info.decimals),
                        info.symbol
                    ),
                });
            }
            None => {
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!("{} raw units", amount),
                });
                items.push(ReviewItem::Field {
                    label: "Mint".into(),
                    value: pubkey_short(&m),
                });
            }
        },
        None => {
            items.push(ReviewItem::Field {
                label: label.into(),
                value: format!("{} raw units", amount),
            });
            items.push(ReviewItem::Warning(
                "Mint unresolved — not in the wallet's ATA map.".into(),
            ));
        }
    }
    items
}
