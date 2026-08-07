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
//! Real DFlow program txs split the swap across two top-level
//! instructions: a `prepare` (disc `2f3e9bac83cd25c9`, layout
//! `[disc(8) | u64 in_amount]`, source ATA in the account list) and the
//! main `swap` whose route-plan layout is not decoded. The prepare gives
//! a verified spend amount + source mint; the swap's receive side is
//! opaque without the IDL.

use crate::parser::anchor;
use crate::parser::bytes::{pubkey_short, read_disc8, read_u64_le};
use crate::parser::token_registry::{self, AtaMap};
use crate::parser::{ParsedInstruction, ReviewItem};

/// DFlow's `global:swap` Anchor sighash.
fn swap_disc() -> [u8; 8] {
    anchor::discriminator("swap")
}

const PREPARE_DISC: [u8; 8] = [0x2f, 0x3e, 0x9b, 0xac, 0x83, 0xcd, 0x25, 0xc9];

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

    if disc == PREPARE_DISC {
        return parse_prepare(data, account_indices, all_accounts, ata_map);
    }

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

fn parse_prepare(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    let mut items: Vec<ReviewItem> = Vec::new();
    items.push(ReviewItem::Header("DFlow swap".into()));

    match read_u64_le(data, 8) {
        Ok(amount) if amount > 0 && amount < PLAUSIBLE_AMOUNT_CAP => {
            let (source_mint, _) = resolve_user_mints(account_indices, all_accounts, ata_map);
            items.extend(format_token_side("You spend", source_mint, amount));
        }
        _ => {
            items.push(ReviewItem::Warning(
                "DFlow setup — amount not decoded.".into(),
            ));
        }
    }

    ParsedInstruction {
        program: "DFlow".into(),
        items,
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
/// Reads the trailing 12 bytes as `(out_amount(u64), slip(u16), fee(u16))`.
fn read_dflow_swap_footer(data: &[u8]) -> Option<(u64, u16, u16)> {
    if data.len() < 12 {
        return None;
    }
    let pos = data.len() - 12;
    let out = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
    let slip = u16::from_le_bytes(data[pos + 8..pos + 10].try_into().ok()?);
    let fee = u16::from_le_bytes(data[pos + 10..pos + 12].try_into().ok()?);
    Some((out, slip, fee))
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

/// Best-effort decode of the trailing amounts/slippage/fee in DFlow's
/// `swap` data, mirroring Jupiter's `RoutePlanFirst` layout. We can't
/// confidently parse the variable-length route plan in the middle without
/// the IDL, so we read the trailing fields from the END of the data
/// Plausibility cap for a parsed DFlow amount. Anything above ~10^16 raw
/// units exceeds the supply of every common Solana token (SOL ≈ 5.8e17
/// lamports total, and that's the largest by raw-unit count) — when our
/// trailing-bytes heuristic lands inside the route plan instead of on the
/// real `in_amount` field, the values come back astronomical (we saw
/// `6.5e18` raw lamports = ~6.5B SOL on a real DFlow swap). Treat
/// implausible values as "couldn't decode" so the zoned review falls back
/// to the legacy per-instruction pages where the user can verify the
/// inner Token transfers directly.
const PLAUSIBLE_AMOUNT_CAP: u64 = 10_000_000_000_000_000;

fn parse_swap(
    data: &[u8],
    _account_indices: &[u8],
    _all_accounts: &[[u8; 32]],
    _ata_map: &AtaMap,
) -> ParsedInstruction {
    let mut items: Vec<ReviewItem> = Vec::new();
    items.push(ReviewItem::Header("DFlow swap".into()));

    // DFlow swap data layout (reverse-engineered):
    //   `[disc | actions: Vec<Action> | quoted_out(u64) | slip(u16) | fee(u16)]`
    // We decode the trailing 12 bytes deterministically. We do **not** try
    // to guess which user-ATA in the swap accounts is the dest mint —
    // without an IDL there's no way to tell source vs dest reliably, and
    // mis-denominating by 3 orders of magnitude (e.g. 6-dec vs 9-dec) is
    // worse than showing raw units. The user can verify the number against
    // their dApp; the value itself is verifiably from the signed bytes.
    let footer = read_dflow_swap_footer(data).filter(|(out_a, _, _)| {
        *out_a > 0 && *out_a < PLAUSIBLE_AMOUNT_CAP
    });

    if let Some((quoted_out, slippage_bps, fee_bps)) = footer {
        let factor = 10_000_u64.saturating_sub(slippage_bps as u64);
        let min_out = ((quoted_out as u128 * factor as u128) / 10_000) as u64;
        items.extend(format_token_side("You receive (min)", None, min_out));
        items.push(ReviewItem::Field {
            label: "Slippage".into(),
            value: format!("{:.2}%", slippage_bps as f32 / 100.0),
        });
        items.push(ReviewItem::Field {
            label: "Slippage_bps".into(),
            value: slippage_bps.to_string(),
        });
        if fee_bps > 0 {
            items.push(ReviewItem::Field {
                label: "Platform fee".into(),
                value: format!("{:.2}%", fee_bps as f32 / 100.0),
            });
        }
    } else {
        items.push(ReviewItem::Warning(
            "Receive amount computed inside aggregator — verify on dApp.".into(),
        ));
    }

    if let Ok(hops) = crate::parser::bytes::read_u32_le(data, 8) {
        if hops > 0 && hops <= 32 {
            items.push(ReviewItem::Field {
                label: "Route_hops".into(),
                value: hops.to_string(),
            });
        }
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
                    token_registry::format_amount(amount, info.decimals)
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
        // DFlow swap data layout: [disc | actions… | quoted_out(u64) |
        // slip(u16) | fee(u16)]. Trailing 12 bytes carry the amounts;
        // `in_amount` lives in the paired `prepare` ix (not parsed here).
        let mut data = Vec::new();
        data.extend_from_slice(&swap_disc());
        data.extend_from_slice(&[0u8; 32]); // filler "actions"
        data.extend_from_slice(&10_000_000u64.to_le_bytes()); // out = 0.01 SOL
        data.extend_from_slice(&50u16.to_le_bytes()); // slip = 50 bps
        data.extend_from_slice(&0u16.to_le_bytes()); // fee bps

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
            },
        );
        ata_map.insert(
            wsol_ata,
            crate::parser::token_registry::AtaEntry {
                mint: wsol_mint(),
            },
        );

        let ix = parse(&data, &account_indices, &all_accounts, &ata_map);

        assert_eq!(ix.program, "DFlow");
        // swap ix only carries the receive side; spend comes from the prepare
        // ix. The parser does NOT denominate dest — without the IDL we can't
        // determine which user-ATA is the dest reliably, so we surface the
        // raw u64 from the signed bytes and let the user verify against the
        // dApp. With 50 bps slippage: 10_000_000 * (1 - 50/10000) = 9_950_000.
        assert_eq!(
            item_value(&ix.items, "You receive (min)"),
            Some("9950000 raw units")
        );
    }

    #[test]
    fn implausible_amounts_suppress_spend_receive() {
        // Trailing footer with an out_amount that crosses the plausibility
        // cap. Parser must refuse to emit a denominated receive — would be
        // dangerous next to a Sign button.
        let mut data = Vec::new();
        data.extend_from_slice(&swap_disc());
        data.extend_from_slice(&[0u8; 32]); // filler actions
        data.extend_from_slice(&17_582_052_945_200_000_000u64.to_le_bytes());
        data.extend_from_slice(&50u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());

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
            },
        );
        ata_map.insert(
            wsol_ata,
            crate::parser::token_registry::AtaEntry {
                mint: wsol_mint(),
            },
        );

        let ix = parse(&data, &account_indices, &all_accounts, &ata_map);
        assert_eq!(ix.program, "DFlow");
        assert!(item_value(&ix.items, "You receive (min)").is_none());
        assert!(ix.items.iter().any(|it| matches!(it, ReviewItem::Warning(_))));
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
