//! Jupiter v6 aggregator instruction parser.
//!
//! Parses Jupiter swap instructions offline, extracting amounts, slippage,
//! and token identification via the static account list or ATA derivation.
//!
//! Based on the Jupiter v6 IDL. Jupiter uses **snake_case** names for its
//! Anchor discriminators (e.g. `shared_accounts_route`, not `sharedAccountsRoute`).

use crate::parser::anchor;
use crate::parser::bytes::read_disc8;
use crate::parser::token_registry::{self, AtaMap};
use crate::parser::{ParsedInstruction, ReviewItem};

// ── Instruction types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum DataLayout {
    RoutePlanFirst,
    AmountsFirst,
}

#[derive(Debug, Clone, PartialEq)]
enum JupiterInstruction {
    Route,
    RouteV2,
    RouteWithTokenLedger,
    SharedAccountsRoute,
    SharedAccountsRouteV2,
    SharedAccountsRouteWithTokenLedger,
    ExactOutRoute,
    ExactOutRouteV2,
    SharedAccountsExactOutRoute,
    SharedAccountsExactOutRouteV2,
}

impl JupiterInstruction {
    fn data_layout(&self) -> DataLayout {
        match self {
            Self::RouteV2
            | Self::SharedAccountsRouteV2
            | Self::ExactOutRouteV2
            | Self::SharedAccountsExactOutRouteV2 => DataLayout::AmountsFirst,
            _ => DataLayout::RoutePlanFirst,
        }
    }

    fn is_exact_out(&self) -> bool {
        matches!(
            self,
            Self::ExactOutRoute
                | Self::ExactOutRouteV2
                | Self::SharedAccountsExactOutRoute
                | Self::SharedAccountsExactOutRouteV2
        )
    }

    fn is_shared_accounts(&self) -> bool {
        matches!(
            self,
            Self::SharedAccountsRoute
                | Self::SharedAccountsRouteV2
                | Self::SharedAccountsRouteWithTokenLedger
                | Self::SharedAccountsExactOutRoute
                | Self::SharedAccountsExactOutRouteV2
        )
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::RouteV2 => "route_v2",
            Self::RouteWithTokenLedger => "route_with_token_ledger",
            Self::SharedAccountsRoute => "shared_accounts_route",
            Self::SharedAccountsRouteV2 => "shared_accounts_route_v2",
            Self::SharedAccountsRouteWithTokenLedger => "shared_accounts_route_with_token_ledger",
            Self::ExactOutRoute => "exact_out_route",
            Self::ExactOutRouteV2 => "exact_out_route_v2",
            Self::SharedAccountsExactOutRoute => "shared_accounts_exact_out_route",
            Self::SharedAccountsExactOutRouteV2 => "shared_accounts_exact_out_route_v2",
        }
    }
}

// ── Discriminator tables ─────────────────────────────────────────────────────

fn identify_instruction(disc: &[u8; 8]) -> Option<JupiterInstruction> {
    let table: &[(&str, JupiterInstruction)] = &[
        ("route", JupiterInstruction::Route),
        ("route_v2", JupiterInstruction::RouteV2),
        (
            "route_with_token_ledger",
            JupiterInstruction::RouteWithTokenLedger,
        ),
        (
            "shared_accounts_route",
            JupiterInstruction::SharedAccountsRoute,
        ),
        (
            "shared_accounts_route_v2",
            JupiterInstruction::SharedAccountsRouteV2,
        ),
        (
            "shared_accounts_route_with_token_ledger",
            JupiterInstruction::SharedAccountsRouteWithTokenLedger,
        ),
        ("exact_out_route", JupiterInstruction::ExactOutRoute),
        ("exact_out_route_v2", JupiterInstruction::ExactOutRouteV2),
        (
            "shared_accounts_exact_out_route",
            JupiterInstruction::SharedAccountsExactOutRoute,
        ),
        (
            "shared_accounts_exact_out_route_v2",
            JupiterInstruction::SharedAccountsExactOutRouteV2,
        ),
    ];

    table
        .iter()
        .find(|(name, _)| anchor::discriminator(name) == *disc)
        .map(|(_, ix)| ix.clone())
}

fn is_known_non_swap(disc: &[u8; 8]) -> bool {
    const NON_SWAP: &[&str] = &[
        "set_token_ledger",
        "create_token_account",
        "create_token_ledger",
        "open_order_initialize",
        "close_order",
    ];
    NON_SWAP
        .iter()
        .any(|name| anchor::discriminator(name) == *disc)
}

// ── Swap enum byte size calculator ───────────────────────────────────────────
//
// Jupiter's `Swap` enum encodes which AMM to use for each route step.
// Jupiter keeps adding new AMMs, so unknown variants default to fieldless
// (0 extra bytes) — the majority pattern for new integrations.

fn swap_enum_byte_size(data: &[u8]) -> Result<usize, &'static str> {
    if data.is_empty() {
        return Err("Empty swap enum");
    }
    let extra: usize = match data[0] {
        // Fieldless variants
        0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 9 | 10 | 11 | 13 | 14 | 19 | 20 | 22 | 25 | 26 | 30
        | 31 | 32 | 34 | 35 | 36 | 37 | 38 | 40 | 41 | 42 | 48 | 50 | 51 | 52 | 53 | 54 | 55
        | 56 | 57 | 59 | 61 | 63 => 0,

        // Single-field: bool or Side enum (1 byte)
        8 | 12 | 15 | 16 | 17 | 18 | 21 | 23 | 24 | 27 | 28 | 39 | 60 | 62 => 1,

        // Symmetry: fromTokenId(u64) + toTokenId(u64)
        29 => 16,

        // StakeDexSwapViaStake / StakeDexPrefundWithdrawStakeAndDepositStake: u32
        33 | 43 => 4,

        // Clone: poolIndex(u8) + quantityIsInput(bool) + quantityIsCollateral(bool)
        44 => 3,

        // SanctumS: u8 + u8 + u32 + u32
        45 => 10,

        // SanctumSAddLiquidity / SanctumSRemoveLiquidity: u8 + u32
        46 | 47 => 5,

        // WhirlpoolSwapV2: aToB(bool=1) + RemainingAccountsInfo{slices: Vec<{u8,u8}>}
        49 => {
            if data.len() < 6 {
                return Err("WhirlpoolSwapV2 data too short");
            }
            let n = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
            1 + 4 + n * 2
        }

        // StabbleStableSwap: Option<RemainingAccountsInfo>
        58 => {
            if data.len() < 2 {
                return Err("StabbleStableSwap data too short");
            }
            match data[1] {
                0 => 1,
                1 => {
                    if data.len() < 6 {
                        return Err("StabbleStableSwap vec too short");
                    }
                    let n = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
                    1 + 4 + n * 2
                }
                _ => return Err("Invalid StabbleStableSwap option tag"),
            }
        }

        // Unknown variants: assume fieldless (most new AMM integrations are)
        _ => 0,
    };
    Ok(1 + extra)
}

/// Advances past a Borsh-encoded `Vec<RoutePlanStep>`.
fn skip_route_plan(data: &[u8]) -> Result<usize, &'static str> {
    if data.len() < 4 {
        return Err("Route plan too short");
    }
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut pos = 4;
    for _ in 0..count {
        if pos >= data.len() {
            return Err("Route plan truncated");
        }
        pos += swap_enum_byte_size(&data[pos..])?;
        pos += 3; // percent(u8) + input_index(u8) + output_index(u8)
    }
    Ok(pos)
}

// ── Data readers ─────────────────────────────────────────────────────────────

fn read_u64(data: &[u8], pos: usize) -> Result<u64, &'static str> {
    data.get(pos..pos + 8)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or("Insufficient data for u64")
}

fn read_u16(data: &[u8], pos: usize) -> Result<u16, &'static str> {
    data.get(pos..pos + 2)
        .and_then(|b| b.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or("Insufficient data for u16")
}

fn get_account(account_indices: &[u8], idx: usize, all_accounts: &[[u8; 32]]) -> Option<[u8; 32]> {
    let key_idx = *account_indices.get(idx)? as usize;
    all_accounts.get(key_idx).copied()
}

// ── Amount parsing ───────────────────────────────────────────────────────────

fn parse_amounts(data: &[u8], layout: &DataLayout) -> Result<(u64, u64, u16, u8), &'static str> {
    let mut pos = 8; // skip discriminator
    if matches!(layout, DataLayout::RoutePlanFirst) {
        pos += skip_route_plan(&data[pos..])?;
    }
    let a1 = read_u64(data, pos)?;
    pos += 8;
    let a2 = read_u64(data, pos)?;
    pos += 8;
    let slippage = read_u16(data, pos)?;
    pos += 2;
    let fee = *data.get(pos).ok_or("Insufficient data for fee")?;
    Ok((a1, a2, slippage, fee))
}

// ── Swap result ──────────────────────────────────────────────────────────────

struct SwapResult {
    ix_type: JupiterInstruction,
    in_amount: u64,
    out_amount: u64,
    slippage_bps: u16,
    platform_fee_bps: u8,
    source_mint: Option<[u8; 32]>,
    dest_mint: Option<[u8; 32]>,
    source_token_account: Option<[u8; 32]>,
    dest_token_account: Option<[u8; 32]>,
    parse_error: Option<String>,
}

fn build_swap_result(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ix_type: &JupiterInstruction,
) -> SwapResult {
    let (source_mint, dest_mint, source_ta, dest_ta) = if ix_type.is_shared_accounts() {
        (
            get_account(account_indices, 7, all_accounts),
            get_account(account_indices, 8, all_accounts),
            get_account(account_indices, 3, all_accounts),
            get_account(account_indices, 6, all_accounts),
        )
    } else if matches!(
        ix_type,
        JupiterInstruction::RouteV2 | JupiterInstruction::ExactOutRouteV2
    ) {
        (
            None,
            None,
            get_account(account_indices, 1, all_accounts),
            get_account(account_indices, 2, all_accounts),
        )
    } else {
        (
            None,
            get_account(account_indices, 5, all_accounts),
            get_account(account_indices, 2, all_accounts),
            get_account(account_indices, 3, all_accounts),
        )
    };

    let (in_amount, out_amount, slippage_bps, platform_fee_bps, parse_error) =
        match parse_amounts(data, &ix_type.data_layout()) {
            Ok((a1, a2, s, f)) => {
                let (in_a, out_a) = if ix_type.is_exact_out() {
                    (a2, a1)
                } else {
                    (a1, a2)
                };
                (in_a, out_a, s, f, None)
            }
            Err(e) => (0, 0, 0, 0, Some(e.to_string())),
        };

    SwapResult {
        ix_type: ix_type.clone(),
        in_amount,
        out_amount,
        slippage_bps,
        platform_fee_bps,
        source_mint,
        dest_mint,
        source_token_account: source_ta,
        dest_token_account: dest_ta,
        parse_error,
    }
}

fn resolve_mints_from_ata(result: &mut SwapResult, ata_map: &AtaMap) {
    if result.source_mint.is_none() {
        if let Some(ta) = result.source_token_account {
            if let Some(entry) = ata_map.get(&ta) {
                result.source_mint = Some(entry.mint);
            }
        }
    }
    if result.dest_mint.is_none() {
        if let Some(ta) = result.dest_token_account {
            if let Some(entry) = ata_map.get(&ta) {
                result.dest_mint = Some(entry.mint);
            }
        }
    }
}

/// Last-resort mint resolution for swap variants where the mint doesn't
/// appear in the ix account args *and* the token account is created in
/// an inner CPI we can't see (Jupiter's `RouteV2` / `ExactOutRouteV2`
/// temp WSOL wrappers, notably). Scans the whole tx account list for
/// any known-token mint pubkey and fills in whichever side is still None
/// — but only when exactly one candidate is present, so we don't mis-label
/// a token ↔ token swap where both sides are unknown.
///
/// The alternative (leaving it as "raw units") produces the user-visible
/// `"11598426 raw units"` regression. A small heuristic that gets the
/// common SOL-side of swaps right is better than a principled one that
/// never fires.
fn resolve_mints_from_account_list(result: &mut SwapResult, all_accounts: &[[u8; 32]]) {
    if result.source_mint.is_some() && result.dest_mint.is_some() {
        return;
    }

    let known: Vec<[u8; 32]> = all_accounts
        .iter()
        .copied()
        .filter(|k| token_registry::lookup(k).is_some())
        .collect();

    // Collect what's already in use so we don't double-assign.
    let mut used: Vec<[u8; 32]> = Vec::new();
    if let Some(m) = result.source_mint {
        used.push(m);
    }
    if let Some(m) = result.dest_mint {
        used.push(m);
    }

    let candidates: Vec<[u8; 32]> = known
        .into_iter()
        .filter(|k| !used.contains(k))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    if candidates.len() == 1 {
        let only = candidates[0];
        // Assign to whichever side is unresolved. If both are, we can't
        // disambiguate which is source vs dest from a single candidate —
        // do nothing rather than guess and mislabel direction.
        match (result.source_mint, result.dest_mint) {
            (None, Some(_)) => result.source_mint = Some(only),
            (Some(_), None) => result.dest_mint = Some(only),
            _ => {}
        }
    }
}

// ── Output formatting ────────────────────────────────────────────────────────

fn format_token_side(label: &str, mint: &Option<[u8; 32]>, amount: u64) -> Vec<ReviewItem> {
    let mut items = Vec::new();

    match mint {
        Some(m) => match token_registry::lookup(m) {
            Some(info) => {
                // Hero @H2 row consumer — short form so whale-sized balances
                // don't overflow the hero line. See format_amount_short docs.
                let formatted = token_registry::format_amount_short(amount, info.decimals);
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!("{} {}", formatted, info.symbol),
                });
            }
            None => {
                let mint_short = pubkey_short(m);
                items.push(ReviewItem::Field {
                    label: label.into(),
                    value: format!("{} raw units", amount),
                });
                items.push(ReviewItem::Field {
                    label: "Mint".into(),
                    value: mint_short,
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

fn format_swap(result: &SwapResult) -> ParsedInstruction {
    let header = if result.ix_type.is_exact_out() {
        "Jupiter Exact-Out Swap"
    } else {
        "Jupiter Swap"
    };

    let mut items = vec![
        ReviewItem::Header(header.into()),
        ReviewItem::Field {
            label: "Type".into(),
            value: result.ix_type.name().into(),
        },
    ];

    items.extend(format_token_side(
        "You spend",
        &result.source_mint,
        result.in_amount,
    ));
    items.extend(format_token_side(
        if result.ix_type.is_exact_out() {
            "You receive"
        } else {
            "You receive (min)"
        },
        &result.dest_mint,
        result.out_amount,
    ));

    if result.parse_error.is_none() {
        items.push(ReviewItem::Field {
            label: "Slippage".into(),
            value: format!(
                "{} bps ({:.2}%)",
                result.slippage_bps,
                result.slippage_bps as f64 / 100.0
            ),
        });
        if result.platform_fee_bps > 0 {
            items.push(ReviewItem::Field {
                label: "Platform fee".into(),
                value: format!("{} bps", result.platform_fee_bps),
            });
        }
    }

    if let Some(err) = &result.parse_error {
        items.push(ReviewItem::Warning(format!(
            "Could not parse amounts: {}",
            err
        )));
    }

    ParsedInstruction {
        program: "Jupiter".into(),
        items,
    }
}

// ── Public entry point ───────────────────────────────────────────────────────

pub fn parse(
    data: &[u8],
    account_indices: &[u8],
    all_accounts: &[[u8; 32]],
    ata_map: &AtaMap,
) -> ParsedInstruction {
    let disc = match read_disc8(data, 0) {
        Ok(d) => d,
        Err(_) => {
            return ParsedInstruction {
                program: "Jupiter".into(),
                items: vec![
                    ReviewItem::Header("Jupiter".into()),
                    ReviewItem::Warning("Instruction data too short".into()),
                ],
            }
        }
    };
    let ix_type = match identify_instruction(&disc) {
        Some(t) => t,
        None => {
            if is_known_non_swap(&disc) {
                return ParsedInstruction {
                    program: "Jupiter".into(),
                    items: vec![ReviewItem::Header("Jupiter (non-swap)".into())],
                };
            }
            return ParsedInstruction {
                program: "Jupiter".into(),
                items: vec![
                    ReviewItem::Header("Jupiter".into()),
                    ReviewItem::Warning("Unknown instruction variant".into()),
                ],
            };
        }
    };

    let mut result = build_swap_result(data, account_indices, all_accounts, &ix_type);
    resolve_mints_from_ata(&mut result, ata_map);
    resolve_mints_from_account_list(&mut result, all_accounts);
    format_swap(&result)
}

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn field_value<'a>(items: &'a [ReviewItem], label: &str) -> Option<&'a str> {
        items.iter().find_map(|item| match item {
            ReviewItem::Field { label: l, value: v } if l == label => Some(v.as_str()),
            _ => None,
        })
    }

    fn has_warning(items: &[ReviewItem]) -> bool {
        items
            .iter()
            .any(|item| matches!(item, ReviewItem::Warning(_)))
    }

    fn has_header(items: &[ReviewItem], text: &str) -> bool {
        items
            .iter()
            .any(|item| matches!(item, ReviewItem::Header(h) if h == text))
    }

    fn usdc_mint() -> [u8; 32] {
        let bytes = bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .into_vec()
            .unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        key
    }

    fn sol_mint() -> [u8; 32] {
        let bytes = bs58::decode("So11111111111111111111111111111111111111112")
            .into_vec()
            .unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        key
    }

    /// Builds a minimal shared_accounts_route instruction with explicit mints.
    fn build_shared_accounts_route_data(
        in_amount: u64,
        out_amount: u64,
        slippage_bps: u16,
        fee: u8,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        // 8-byte discriminator
        data.extend_from_slice(&anchor::discriminator("shared_accounts_route"));
        // route_plan: empty vec (u32 count = 0)
        data.extend_from_slice(&0u32.to_le_bytes());
        // amounts
        data.extend_from_slice(&in_amount.to_le_bytes());
        data.extend_from_slice(&out_amount.to_le_bytes());
        data.extend_from_slice(&slippage_bps.to_le_bytes());
        data.push(fee);
        data
    }

    #[test]
    fn test_too_short_data() {
        let ix = parse(&[0u8; 4], &[], &[], &AtaMap::new());
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_unknown_discriminator() {
        let ix = parse(&[0xFF; 8], &[], &[], &AtaMap::new());
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_non_swap_instruction() {
        let disc = anchor::discriminator("set_token_ledger");
        let ix = parse(&disc, &[], &[], &AtaMap::new());
        assert!(has_header(&ix.items, "Jupiter (non-swap)"));
        assert!(!has_warning(&ix.items));
    }

    #[test]
    fn test_shared_accounts_route_with_explicit_mints() {
        let data = build_shared_accounts_route_data(
            1_500_000_000, // 1.5 SOL
            150_000_000,   // 150 USDC
            50,
            0,
        );

        // Build account list: indices [0..12] map to accounts
        // shared_accounts_route: src_mint at [7], dst_mint at [8]
        let sol = sol_mint();
        let usdc = usdc_mint();
        let dummy = [0xAA; 32];
        let accounts = [
            dummy, dummy, dummy, dummy, dummy, dummy, dummy, sol, usdc, dummy, dummy, dummy, dummy,
        ];
        let account_indices: Vec<u8> = (0..13).collect();

        let ix = parse(&data, &account_indices, &accounts, &AtaMap::new());

        assert_eq!(ix.program, "Jupiter");
        assert!(has_header(&ix.items, "Jupiter Swap"));
        assert_eq!(
            field_value(&ix.items, "Type"),
            Some("shared_accounts_route")
        );
        assert_eq!(field_value(&ix.items, "You spend"), Some("1.5 SOL"));
        assert_eq!(
            field_value(&ix.items, "You receive (min)"),
            Some("150 USDC")
        );
        assert_eq!(field_value(&ix.items, "Slippage"), Some("50 bps (0.50%)"));
        assert!(!has_warning(&ix.items));
    }

    #[test]
    fn test_exact_out_route_header() {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor::discriminator("exact_out_route"));
        // empty route plan
        data.extend_from_slice(&0u32.to_le_bytes());
        // exact-out: a1=guaranteed output, a2=max input
        data.extend_from_slice(&100_000_000u64.to_le_bytes()); // 100 USDC out
        data.extend_from_slice(&2_000_000_000u64.to_le_bytes()); // max 2 SOL in
        data.extend_from_slice(&100u16.to_le_bytes());
        data.push(0);

        let dest_mint = usdc_mint();
        let dummy = [0xBB; 32];
        let accounts = [dummy, dummy, dummy, dummy, dummy, dest_mint, dummy, dummy];
        let account_indices: Vec<u8> = (0..8).collect();

        let ix = parse(&data, &account_indices, &accounts, &AtaMap::new());
        assert!(has_header(&ix.items, "Jupiter Exact-Out Swap"));
        // exact-out: in_amount = a2 (max input), out_amount = a1 (guaranteed output)
        assert_eq!(field_value(&ix.items, "You receive"), Some("100 USDC"));
    }

    #[test]
    fn test_v2_amounts_first_layout() {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor::discriminator("shared_accounts_route_v2"));
        // v2 layout: amounts immediately after discriminator
        data.extend_from_slice(&1_000_000_000u64.to_le_bytes()); // 1 SOL
        data.extend_from_slice(&50_000_000u64.to_le_bytes()); // 50 USDC
        data.extend_from_slice(&25u16.to_le_bytes());
        data.push(0);

        let sol = sol_mint();
        let usdc = usdc_mint();
        let dummy = [0xCC; 32];
        let accounts = [
            dummy, dummy, dummy, dummy, dummy, dummy, dummy, sol, usdc, dummy,
        ];
        let account_indices: Vec<u8> = (0..10).collect();

        let ix = parse(&data, &account_indices, &accounts, &AtaMap::new());
        assert_eq!(field_value(&ix.items, "You spend"), Some("1 SOL"));
        assert_eq!(field_value(&ix.items, "You receive (min)"), Some("50 USDC"));
    }

    #[test]
    fn test_unresolved_mints_show_warning() {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor::discriminator("route_v2"));
        // v2: amounts first
        data.extend_from_slice(&1_000_000u64.to_le_bytes());
        data.extend_from_slice(&2_000_000u64.to_le_bytes());
        data.extend_from_slice(&50u16.to_le_bytes());
        data.push(0);

        // route_v2: no mints in static accounts (they're in LUTs)
        // token accounts at indices [1] and [2] won't match anything without ATA map
        let dummy = [0xDD; 32];
        let accounts = [dummy, dummy, dummy];
        let account_indices: Vec<u8> = (0..3).collect();

        let ix = parse(&data, &account_indices, &accounts, &AtaMap::new());
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn test_platform_fee_shown_when_nonzero() {
        let data = build_shared_accounts_route_data(1_000, 2_000, 50, 5);
        let dummy = [0xAA; 32];
        let accounts = vec![dummy; 13];
        let account_indices: Vec<u8> = (0..13).collect();

        let ix = parse(&data, &account_indices, &accounts, &AtaMap::new());
        assert_eq!(field_value(&ix.items, "Platform fee"), Some("5 bps"));
    }

    #[test]
    fn test_platform_fee_hidden_when_zero() {
        let data = build_shared_accounts_route_data(1_000, 2_000, 50, 0);
        let dummy = [0xAA; 32];
        let accounts = vec![dummy; 13];
        let account_indices: Vec<u8> = (0..13).collect();

        let ix = parse(&data, &account_indices, &accounts, &AtaMap::new());
        assert!(field_value(&ix.items, "Platform fee").is_none());
    }

    #[test]
    fn test_skip_route_plan_empty() {
        let data = [0, 0, 0, 0]; // count=0
        assert_eq!(skip_route_plan(&data).unwrap(), 4);
    }

    #[test]
    fn test_skip_route_plan_single_fieldless_step() {
        // One step with swap variant 0 (Saber, fieldless) + 3 bytes
        let data = [
            1, 0, 0, 0,  // count=1
            0,  // swap variant 0 (Saber): 1 byte
            50, // percent
            0,  // input_index
            0,  // output_index
        ];
        assert_eq!(skip_route_plan(&data).unwrap(), 4 + 1 + 3);
    }

    #[test]
    fn test_swap_enum_fieldless() {
        assert_eq!(swap_enum_byte_size(&[0]).unwrap(), 1);
        assert_eq!(swap_enum_byte_size(&[7]).unwrap(), 1);
    }

    #[test]
    fn test_swap_enum_single_field() {
        assert_eq!(swap_enum_byte_size(&[17, 1]).unwrap(), 2); // Whirlpool { aToB: bool }
    }

    #[test]
    fn test_swap_enum_symmetry() {
        let mut data = vec![29];
        data.extend_from_slice(&[0u8; 16]); // fromTokenId + toTokenId
        assert_eq!(swap_enum_byte_size(&data).unwrap(), 17);
    }
}
