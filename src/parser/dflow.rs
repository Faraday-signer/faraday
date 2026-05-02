//! DFlow Aggregator instruction parser.
//!
//! DFlow (`DF1ow4tspfHX9JwWJsAb9epbkA8hmpSEAtxXy1V27QBH`) is a Jupiter-style
//! swap aggregator. Its primary swap instruction uses the standard Anchor
//! sighash for `global:swap` (`f8c69e91e17587c8`).
//!
//! No public IDL is published, so the data layout is reverse-engineered from
//! on-chain transactions — see `testdata/parser/dflow_*.txt` for the samples
//! used to derive the offsets. Two known assumptions:
//!
//!   1. Data layout follows the Jupiter `RoutePlanFirst` shape:
//!      `[disc(8) | route_plan_len(u32) | <opaque route bytes> | in_amount(u64) | out_amount(u64) | slippage_bps(u16) | platform_fee_bps(u8)]`.
//!      Decoded bytes from real txs match this; if amounts come back nonsense we
//!      fall back to leaving them at 0 and emit a "Mint unresolved" hint.
//!
//!   2. Source/dest mints are not at fixed account positions like Jupiter's
//!      shared-accounts variants. Instead we resolve them via the same ATA
//!      map used by the Jupiter parser — the user's known token accounts are
//!      already mapped to mints + symbols at parse time. We pick the two
//!      accounts referenced by this instruction whose ATAs match a known
//!      mint, and label them as source/dest.
//!
//! Two other DFlow discriminators (`414b3f4ceb5b5b88`, `2f3e9bac83cd25c9`)
//! observed in the wild aren't standard Anchor names — they're rendered as
//! "DFlow: unknown action (<hex>)" so the user knows which slot they are
//! without us guessing their semantics.

use crate::parser::anchor;
use crate::parser::bytes::{read_disc8, read_u64_le};
use crate::parser::token_registry::{self, AtaMap};
use crate::parser::{ParsedInstruction, ReviewItem};

/// DFlow's `global:swap` Anchor sighash.
fn swap_disc() -> [u8; 8] {
    anchor::discriminator("swap")
}

pub fn parse(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    let disc = match read_disc8(data, 0) {
        Ok(d) => d,
        Err(_) => return error_ix("DFlow data too short for discriminator"),
    };

    if disc == swap_disc() {
        return parse_swap(data, account_indices, all_accounts, ata_map);
    }

    // Unknown discriminator — surface the hex so the user (and us, on a
    // future debug pass) can identify which DFlow flow this was. We treat
    // it as a reviewable action rather than failing outright; the user can
    // still drill into pages 2..K to verify the underlying Token transfers.
    ParsedInstruction {
        program: "DFlow".into(),
        items: vec![
            ReviewItem::Header(format!(
                "DFlow: unknown action ({:02x}{:02x}{:02x}{:02x}…)",
                disc[0], disc[1], disc[2], disc[3]
            )),
            ReviewItem::Warning(
                "Decoder doesn't recognize this DFlow instruction yet. Verify the inner transfers on the per-instruction pages."
                    .into(),
            ),
        ],
    }
}

fn error_ix(msg: &'static str) -> ParsedInstruction {
    ParsedInstruction {
        program: "DFlow".into(),
        items: vec![ReviewItem::Warning(msg.into())],
    }
}

/// Pull the user's source/dest mints out of this instruction's account list
/// by intersecting it with the ATA map. Returns up to two distinct mints in
/// the order they appear in the instruction — `(source, dest)`.
///
/// This is the opposite of Jupiter's fixed-position approach. DFlow's
/// account ordering varies per route topology (different pool families
/// land at different slots), so we lean on the wallet's ATA map instead:
/// the user's spend-side ATA is in there, and so is their receive-side ATA
/// (when the receiver is also their own ATA — i.e. nearly always).
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

/// Best-effort decode of the trailing amounts/slippage/fee in DFlow's
/// `swap` data, mirroring Jupiter's `RoutePlanFirst` layout. We can't
/// confidently parse the variable-length route plan in the middle without
/// the IDL, so we read the trailing fields from the END of the data
/// instead: the last 19 bytes are `in_amount(u64) | out_amount(u64) |
/// slippage_bps(u16) | platform_fee_bps(u8)`.
///
/// Returns `None` when the data is shorter than 19 bytes — we'd rather
/// emit a "no amounts" hero than a wildly wrong number.
fn parse_trailing_amounts(data: &[u8]) -> Option<(u64, u64, u16, u8)> {
    const FOOTER: usize = 8 + 8 + 2 + 1;
    if data.len() < 8 + FOOTER {
        return None;
    }
    let pos = data.len() - FOOTER;
    let in_amount = read_u64_le(data, pos).ok()?;
    let out_amount = read_u64_le(data, pos + 8).ok()?;
    let slippage_lo = *data.get(pos + 16)?;
    let slippage_hi = *data.get(pos + 17)?;
    let slippage_bps = u16::from_le_bytes([slippage_lo, slippage_hi]);
    let fee_bps = *data.get(pos + 18)?;
    Some((in_amount, out_amount, slippage_bps, fee_bps))
}

fn parse_swap(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    let mut items: Vec<ReviewItem> = Vec::new();
    items.push(ReviewItem::Header("DFlow swap".into()));

    let (source_mint, dest_mint) = resolve_user_mints(account_indices, all_accounts, ata_map);
    let amounts = parse_trailing_amounts(data);

    let in_amount = amounts.map(|(a, _, _, _)| a).unwrap_or(0);
    let out_amount = amounts.map(|(_, b, _, _)| b).unwrap_or(0);

    items.extend(format_token_side("You spend", source_mint, in_amount));
    items.extend(format_token_side("You receive (min)", dest_mint, out_amount));

    if let Some((_, _, slippage_bps, fee_bps)) = amounts {
        if slippage_bps > 0 {
            items.push(ReviewItem::Field {
                label: "Slippage".into(),
                value: format!("{:.2}%", slippage_bps as f32 / 100.0),
            });
        }
        if fee_bps > 0 {
            items.push(ReviewItem::Field {
                label: "Platform fee".into(),
                value: format!("{:.2}%", fee_bps as f32 / 100.0),
            });
        }
    } else {
        items.push(ReviewItem::Warning(
            "Amounts not decoded — verify on per-instruction pages.".into(),
        ));
    }

    ParsedInstruction {
        program: "DFlow".into(),
        items,
    }
}

/// Mirrors Jupiter's `format_token_side` so the hero builder treats both
/// programs identically. When the mint is in the offline registry we emit
/// "<amount> <symbol>"; when it isn't, we emit raw units + a Mint row so
/// the user can verify the address themselves.
fn format_token_side(label: &str, mint: Option<[u8; 32]>, amount: u64) -> Vec<ReviewItem> {
    let mut items = Vec::new();
    match mint {
        Some(m) => match token_registry::lookup(&m) {
            Some(info) => {
                let formatted = if amount > 0 {
                    token_registry::format_amount_short(amount, info.decimals)
                } else {
                    "?".into()
                };
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!("{} {}", formatted, info.symbol),
                });
            }
            None => {
                let mint_short = pubkey_short(&m);
                if amount > 0 {
                    items.push(ReviewItem::Field {
                        label: label.into(),
                        value: format!("{} raw units", amount),
                    });
                } else {
                    items.push(ReviewItem::Field {
                        label: label.into(),
                        value: "?".into(),
                    });
                }
                items.push(ReviewItem::Field {
                    label: "Mint".into(),
                    value: mint_short,
                });
            }
        },
        None => {
            // No mint resolved — best we can do is acknowledge the side
            // exists so the hero builder still sees both spend+receive
            // fields and renders the "swap pair" layout.
            items.push(ReviewItem::Field {
                label: label.into(),
                value: if amount > 0 {
                    format!("{} raw units", amount)
                } else {
                    "?".into()
                },
            });
            items.push(ReviewItem::Warning(
                "Mint unresolved — not in the wallet's ATA map.".into(),
            ));
        }
    }
    items
}

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    if b58.len() >= 8 {
        format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
    } else {
        b58
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn usdc_mint() -> [u8; 32] {
        let mut out = [0u8; 32];
        let v = bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .into_vec()
            .unwrap();
        out.copy_from_slice(&v);
        out
    }

    fn wsol_mint() -> [u8; 32] {
        let mut out = [0u8; 32];
        let v = bs58::decode("So11111111111111111111111111111111111111112")
            .into_vec()
            .unwrap();
        out.copy_from_slice(&v);
        out
    }

    fn item_value<'a>(items: &'a [ReviewItem], label: &str) -> Option<&'a str> {
        items.iter().find_map(|it| match it {
            ReviewItem::Field { label: l, value } if l == label => Some(value.as_str()),
            _ => None,
        })
    }

    #[test]
    fn swap_disc_matches_anchor_sighash() {
        // Sanity check — if Anchor changes its discriminator algorithm or
        // we've got the name wrong, this catches it before any tx ever
        // hits the parser.
        let expected = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];
        assert_eq!(swap_disc(), expected);
    }

    #[test]
    fn swap_emits_spend_receive_with_known_mints() {
        // Build a minimal data buffer: [swap_disc | filler route bytes |
        // in_amount=1_000_000 (1 USDC at 6 decimals) | out_amount=10_000_000 (0.01 SOL at 9 decimals) |
        // slippage_bps=50 | fee_bps=0]
        let mut data = Vec::new();
        data.extend_from_slice(&swap_disc());
        data.extend_from_slice(&[0u8; 32]); // filler "route plan"
        data.extend_from_slice(&1_000_000u64.to_le_bytes());
        data.extend_from_slice(&10_000_000u64.to_le_bytes());
        data.extend_from_slice(&50u16.to_le_bytes());
        data.push(0);

        // Account list: [user_signer, usdc_ata, wsol_ata, ...]. The ATA
        // map maps usdc_ata → USDC mint and wsol_ata → SOL/WSOL mint.
        let user = key(0x01);
        let usdc_ata = key(0x02);
        let wsol_ata = key(0x03);
        let all_accounts = vec![user, usdc_ata, wsol_ata];
        let account_indices = vec![0u8, 1, 2];

        let mut ata_map = AtaMap::new();
        ata_map.insert(
            usdc_ata,
            crate::parser::token_registry::AtaEntry {
                mint: usdc_mint(),
                symbol: "USDC",
                decimals: 6,
            },
        );
        ata_map.insert(
            wsol_ata,
            crate::parser::token_registry::AtaEntry {
                mint: wsol_mint(),
                symbol: "SOL",
                decimals: 9,
            },
        );

        let ix = parse(&data, &account_indices, &all_accounts, &ata_map);

        assert_eq!(ix.program, "DFlow");
        assert_eq!(item_value(&ix.items, "You spend"), Some("1 USDC"));
        assert_eq!(item_value(&ix.items, "You receive (min)"), Some("0.01 SOL"));
    }

    #[test]
    fn unknown_discriminator_emits_warning_with_hex() {
        let mut data = vec![0u8; 16];
        data[..4].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        let ix = parse(&data, &[], &[], &AtaMap::new());
        assert_eq!(ix.program, "DFlow");
        let header = ix.items.iter().find_map(|it| match it {
            ReviewItem::Header(h) => Some(h.as_str()),
            _ => None,
        });
        assert!(header.unwrap().contains("deadbeef"));
    }
}
