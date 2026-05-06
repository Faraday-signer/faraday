//! Jupiter v6 aggregator instruction parser.
//!
//! Parses Jupiter swap instructions offline, extracting amounts, slippage,
//! and token identification via the static account list or ATA derivation.
//!
//! Based on the Jupiter v6 IDL. Jupiter uses **snake_case** names for its
//! Anchor discriminators (e.g. `shared_accounts_route`, not `sharedAccountsRoute`).

use crate::parser::anchor;
use crate::parser::bytes::{pubkey_short, read_disc8, read_swap_footer};
use crate::parser::token_registry::{self, AtaMap};
use crate::parser::{ParsedInstruction, ReviewItem};

// ── Instruction types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum DataLayout {
    RoutePlanFirst,
    AmountsFirst,
    /// shared_accounts_route_v2 / shared_accounts_exact_out_route_v2:
    /// `[disc | id(u8) | in_amount(u64) | out_amount(u64) | slip(u16) | fee(u8) | route_plan…]`.
    SharedAccountsAmountsFirst,
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
            Self::SharedAccountsRouteV2 | Self::SharedAccountsExactOutRouteV2 => {
                DataLayout::SharedAccountsAmountsFirst
            }
            Self::RouteV2 | Self::ExactOutRouteV2 => DataLayout::AmountsFirst,
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
        "create_open_orders",
        "create_program_open_orders",
    ];
    NON_SWAP
        .iter()
        .any(|name| anchor::discriminator(name) == *disc)
}

/// `claim` / `claim_token` transfer accumulated referral / platform-fee
/// output from a Jupiter escrow back to the user — they move funds and
/// must be surfaced distinctly, not bundled with the no-op ix.
fn claim_action(disc: &[u8; 8]) -> Option<&'static str> {
    if anchor::discriminator("claim") == *disc {
        Some("Jupiter: claim referral")
    } else if anchor::discriminator("claim_token") == *disc {
        Some("Jupiter: claim referral token")
    } else {
        None
    }
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
    match layout {
        DataLayout::RoutePlanFirst => read_swap_footer(data),
        DataLayout::AmountsFirst => parse_leading_amounts(data, 8),
        DataLayout::SharedAccountsAmountsFirst => parse_shared_accounts_v2_amounts(data),
    }
}

fn parse_leading_amounts(data: &[u8], start: usize) -> Result<(u64, u64, u16, u8), &'static str> {
    let mut pos = start;
    let a1 = read_u64(data, pos)?;
    pos += 8;
    let a2 = read_u64(data, pos)?;
    pos += 8;
    let slippage = read_u16(data, pos)?;
    pos += 2;
    let fee = *data.get(pos).ok_or("Insufficient data for fee")?;
    Ok((a1, a2, slippage, fee))
}

fn parse_shared_accounts_v2_amounts(data: &[u8]) -> Result<(u64, u64, u16, u8), &'static str> {
    parse_leading_amounts(data, 9)
}

/// `quoted * (10000 - slip) / 10000`, saturating. Min received on exact-in.
fn apply_slippage_down(quoted: u64, slippage_bps: u16) -> u64 {
    let factor = 10_000_u64.saturating_sub(slippage_bps as u64);
    ((quoted as u128 * factor as u128) / 10_000) as u64
}

/// `quoted * (10000 + slip) / 10000`, saturating. Max spent on exact-out.
fn apply_slippage_up(quoted: u64, slippage_bps: u16) -> u64 {
    let factor = 10_000_u64 + slippage_bps as u64;
    ((quoted as u128 * factor as u128) / 10_000) as u64
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
    let (source_mint, dest_mint, source_ta, dest_ta) = match ix_type {
        // shared_accounts_route_v2 / shared_accounts_exact_out_route_v2:
        // input mint at slot 6, output mint at slot 7 (V1 used 7, 8). The
        // user's TAs aren't in the named slots — Jupiter routes through
        // program-owned TAs — so skip the source/dest TA hints entirely.
        JupiterInstruction::SharedAccountsRouteV2
        | JupiterInstruction::SharedAccountsExactOutRouteV2 => (
            get_account(account_indices, 6, all_accounts),
            get_account(account_indices, 7, all_accounts),
            None,
            None,
        ),
        t if t.is_shared_accounts() => (
            get_account(account_indices, 7, all_accounts),
            get_account(account_indices, 8, all_accounts),
            get_account(account_indices, 3, all_accounts),
            get_account(account_indices, 6, all_accounts),
        ),
        JupiterInstruction::RouteV2 | JupiterInstruction::ExactOutRouteV2 => (
            None,
            None,
            get_account(account_indices, 1, all_accounts),
            get_account(account_indices, 2, all_accounts),
        ),
        _ => (
            None,
            get_account(account_indices, 5, all_accounts),
            get_account(account_indices, 2, all_accounts),
            get_account(account_indices, 3, all_accounts),
        ),
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

    // ALT entries we don't have hardcoded come back as `[0xFF; 32]`
    // (`lookup_tables::UNRESOLVED`). The shared_accounts variants point
    // source/dest mint slots into the ALT, and on aggregator swaps with
    // a fresh per-trade ALT (the one we hit was `DttEs7CN...`) those
    // resolve to UNRESOLVED — which our `format_token_side` then renders
    // as a `JEKN..WxFG` "mint". Strip them to None so the downstream
    // ATA-map / known-mint resolution paths get a chance to fill in.
    let source_mint = source_mint.filter(|k| !crate::parser::lookup_tables::is_unresolved(k));
    let dest_mint = dest_mint.filter(|k| !crate::parser::lookup_tables::is_unresolved(k));

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
                let formatted = token_registry::format_amount(amount, info.decimals);
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
                "Mint unresolved — not in lookup table".into(),
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

    let (spend_label, spend_amount, recv_label, recv_amount) = if result.ix_type.is_exact_out() {
        let max_in = apply_slippage_up(result.in_amount, result.slippage_bps);
        ("You spend (max)", max_in, "You receive", result.out_amount)
    } else {
        let min_out = apply_slippage_down(result.out_amount, result.slippage_bps);
        ("You spend", result.in_amount, "You receive (min)", min_out)
    };
    items.extend(format_token_side(spend_label, &result.source_mint, spend_amount));
    items.extend(format_token_side(recv_label, &result.dest_mint, recv_amount));

    if result.parse_error.is_none() {
        items.push(ReviewItem::Field {
            label: "Slippage".into(),
            value: format!("{:.2}%", result.slippage_bps as f64 / 100.0),
        });
        items.push(ReviewItem::Field {
            label: "Slippage_bps".into(),
            value: result.slippage_bps.to_string(),
        });
        if result.platform_fee_bps > 0 {
            items.push(ReviewItem::Field {
                label: "Platform fee".into(),
                value: format!("{:.2}%", result.platform_fee_bps as f64 / 100.0),
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
            if let Some(label) = claim_action(&disc) {
                return ParsedInstruction {
                    program: "Jupiter".into(),
                    items: vec![
                        ReviewItem::Header(label.into()),
                        ReviewItem::Warning(
                            "Pulls accumulated referral / fee output to your wallet. Verify on dApp."
                                .into(),
                        ),
                    ],
                };
            }
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

    #[test]
    fn claim_emits_distinct_header_with_warning() {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor::discriminator("claim"));
        data.push(0); // id: u8
        let ix = parse(&data, &[], &[], &AtaMap::new());
        assert!(has_header(&ix.items, "Jupiter: claim referral"));
        assert!(has_warning(&ix.items));
    }

    #[test]
    fn claim_token_emits_distinct_header_with_warning() {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor::discriminator("claim_token"));
        data.push(0);
        let ix = parse(&data, &[], &[], &AtaMap::new());
        assert!(has_header(&ix.items, "Jupiter: claim referral token"));
        assert!(has_warning(&ix.items));
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
            Some("149.25 USDC")
        );
        assert_eq!(field_value(&ix.items, "Slippage"), Some("0.50%"));
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
        data.push(0x0c); // id
        data.extend_from_slice(&1_000_000_000u64.to_le_bytes()); // 1 SOL
        data.extend_from_slice(&50_000_000u64.to_le_bytes()); // 50 USDC
        data.extend_from_slice(&25u16.to_le_bytes());
        data.push(0);

        let sol = sol_mint();
        let usdc = usdc_mint();
        let dummy = [0xCC; 32];
        // V2 IDL: source mint at slot 6, dest mint at slot 7.
        let accounts = [
            dummy, dummy, dummy, dummy, dummy, dummy, sol, usdc, dummy, dummy,
        ];
        let account_indices: Vec<u8> = (0..10).collect();

        let ix = parse(&data, &account_indices, &accounts, &AtaMap::new());
        assert_eq!(field_value(&ix.items, "You spend"), Some("1 SOL"));
        assert_eq!(field_value(&ix.items, "You receive (min)"), Some("49.875 USDC"));
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
        assert_eq!(field_value(&ix.items, "Platform fee"), Some("0.05%"));
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

}
