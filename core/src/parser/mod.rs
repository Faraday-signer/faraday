//! Transaction parser — converts raw Solana tx bytes into human-readable review items.
//!
//! Entry point: `parse(tx_bytes)` → `ParsedTransaction`
//!
//! To add support for a new program:
//!   1. Create `src/parser/<program>.rs` with `pub fn parse(data, accounts) -> ParsedInstruction`
//!   2. Add the program ID to `programs::identify()`
//!   3. Add a match arm in the `dispatch()` function below

mod classification;
mod lookup_tables;
mod message;
mod programs;
mod stake;
mod system;
mod token;
mod unknown;

// Shared modules — reusable across dApp parsers
pub(crate) mod anchor;
pub(crate) mod bytes;
pub mod token_registry;

// dApp parsers
mod dflow;
mod jupiter;
mod jupiter_rfq;
mod jupiter_ultra;
mod raydium;

use bytes::pubkey_short;
use sha2::Digest;

// === Public types ===

pub struct ParsedTransaction {
    pub version: TransactionVersion,
    pub fee_payer: String,
    pub signers: Vec<[u8; 32]>,
    pub instructions: Vec<ParsedInstruction>,
    pub fee_lamports: u64,
    pub size: usize,
}

pub enum TransactionVersion {
    Legacy,
    /// Contains the number of address lookup tables. These cannot be resolved
    /// air-gapped (no RPC), so accounts from lookup tables show as unresolved.
    V0 {
        address_table_lookups: usize,
    },
}

pub struct ParsedInstruction {
    pub program: String,
    pub items: Vec<ReviewItem>,
}

pub enum ReviewItem {
    Header(String),
    Field { label: String, value: String },
    Warning(String),
}

/// Structured summary of the dominant action in a transaction, used by the
/// 3-zone review screen. Carries full pubkey bytes so the renderer can
/// base58-encode them once at the right size and run a `(you)` check
/// against the loaded wallet.
pub enum ZonedAction {
    Send {
        from: [u8; 32],
        to: [u8; 32],
        amount_lamports: u64,
    },
    Swap {
        sell_amount: String,
        sell_symbol: String,
        buy_amount: String,
        buy_symbol: String,
        fee_lamports: u64,
        fee_payer: [u8; 32],
        dex_name: String,
        /// Slippage in raw bps (e.g. 22 = 0.22%). `None` when the parser
        /// couldn't surface it — renderer then shows ROUTE or PAYER instead.
        slippage_bps: Option<u16>,
        /// Number of route-plan hops, when the parser can pull it from the
        /// instruction data. Used for the p3 ROUTE row when slippage is
        /// unknown.
        route_hops: Option<u32>,
    },
}

/// Extract a `ZonedAction` from a parsed tx (and its raw bytes), if the
/// dominant action matches a layout the zoned renderer can show. Returns
/// `None` when the tx is more complex (multi-step, unknown programs) so
/// the caller falls back to the legacy review screen.
pub fn extract_zoned(tx_bytes: &[u8], parsed: &ParsedTransaction) -> Option<ZonedAction> {
    // Try SEND first — the strict "single System.Transfer" shape lives at
    // the message-bytes level so we can pull full pubkey bytes for FROM /
    // TO. SWAP detection then runs against the parser's enriched fields.
    if let Some(send) = extract_send(tx_bytes) {
        return Some(send);
    }

    // DFlow splits the swap across two ixs (prepare carries spend, swap
    // carries the opaque route plan), so we aggregate spend/receive across
    // every ix of the same swap program. Single-ix DEXes are a no-op here.
    let fee_payer_bytes = decode_b58_pubkey(&parsed.fee_payer)?;
    let mut spend_value: Option<&str> = None;
    let mut receive_value: Option<&str> = None;
    let mut receive_token_only: Option<&str> = None;
    let mut slippage_bps: Option<u16> = None;
    let mut route_hops: Option<u32> = None;
    let mut dex_name: Option<&str> = None;
    for ix in &parsed.instructions {
        if !is_swap_program_name(&ix.program) {
            continue;
        }
        if spend_value.is_none() {
            spend_value =
                field_value(ix, "You spend").or_else(|| field_value(ix, "You spend (max)"));
        }
        if receive_value.is_none() {
            receive_value =
                field_value(ix, "You receive").or_else(|| field_value(ix, "You receive (min)"));
        }
        if receive_token_only.is_none() {
            receive_token_only = field_value(ix, "Receive token");
        }
        if slippage_bps.is_none() {
            slippage_bps = field_value(ix, "Slippage_bps").and_then(|s| s.parse().ok());
        }
        if route_hops.is_none() {
            route_hops = field_value(ix, "Route_hops").and_then(|s| s.parse().ok());
        }
        if dex_name.is_none() {
            dex_name = Some(&ix.program);
        }
    }
    if let Some(spend) = spend_value {
        let (sell_amount, sell_symbol) = split_amount_symbol_owned(spend);
        if sell_symbol == "?" {
            return None;
        }
        let (buy_amount, buy_symbol) = match receive_value {
            Some(r) => {
                let (a, s) = split_amount_symbol_owned(r);
                if s == "?" {
                    // Amount IS verified from signed bytes; the denomination
                    // (decimals + symbol) isn't. Show the raw number with an
                    // empty symbol — the renderer surfaces "?" so the user
                    // knows the scale is unverified and can cross-check the
                    // exact number against their dApp.
                    (a, String::new())
                } else if a == "?" {
                    (String::new(), s)
                } else {
                    (a, s)
                }
            }
            None => (
                String::new(),
                receive_token_only.map(str::to_string).unwrap_or_default(),
            ),
        };
        // DFlow swap data doesn't name source/dest in fixed slots; the per-ix
        // parser emits the raw u64 receive amount with no symbol. Cross-ref
        // the prepare's source mint against the swap's account list to pick
        // dest deterministically (the unique non-source user-ATA).
        let (buy_amount, buy_symbol) = if dex_name == Some("DFlow") {
            match resolve_dflow_dest(tx_bytes) {
                Some((raw_out, slip, decimals, symbol)) => {
                    let factor = 10_000_u64.saturating_sub(slip as u64);
                    let min_out = (raw_out as u128 * factor as u128 / 10_000) as u64;
                    (
                        token_registry::format_amount(min_out, decimals),
                        symbol.to_string(),
                    )
                }
                None => (buy_amount, buy_symbol),
            }
        } else {
            (buy_amount, buy_symbol)
        };
        // Hardware-wallet invariant: NEVER render the zoned approval screen
        // with an unverified receive symbol. A `?` next to a Sign button is
        // worse than no zoned screen at all — user might think they're getting
        // USDC and actually get something else. Falling back to None here
        // routes the renderer to the legacy paginated review where the warning
        // is explicit and the dest is clearly marked "Mint unresolved".
        if dex_name.is_some() && buy_symbol.is_empty() {
            return None;
        }
        return Some(ZonedAction::Swap {
            sell_amount,
            sell_symbol,
            buy_amount,
            buy_symbol,
            fee_lamports: parsed.fee_lamports,
            fee_payer: fee_payer_bytes,
            dex_name: dex_name.map(|s| s.to_string()).unwrap_or_default(),
            slippage_bps,
            route_hops,
        });
    }

    // Unknown-aggregator path: the tx invokes a program we don't have a
    // dedicated parser for (Jupiter Ultra's `iris` router, DFlow with an
    // undocumented disc, future aggregators). We can't decode the
    // aggregator's instruction data, so the receive amount is genuinely
    // unknown to the device — but we CAN identify the *send* and *receive
    // tokens* offline by deriving the user's canonical ATAs for every
    // KNOWN_TOKEN and matching them against the accounts the tx touches.
    //
    // This works regardless of which aggregator is used and regardless
    // of whether the tx references the mint pubkey directly or buries it
    // in an unknown ALT — we never need the mint bytes, only the ATA
    // addresses, which are determined entirely by the signer + token
    // program + mint + ATA program.
    detect_swap_shape(tx_bytes, parsed.fee_lamports)
}

/// Detect a swap-shaped transaction by tracking the user's funds rather
/// than parsing each aggregator's bespoke data layout. Returns a
/// `ZonedAction::Swap` with the SEND side fully resolved (amount + symbol)
/// and the RECEIVE side carrying the symbol but a sentinel empty
/// `buy_amount` — we deliberately do **not** invent a number for an
/// amount we cannot verify offline. The renderer treats empty
/// `buy_amount` as "not verified by device, see dApp quote".
///
/// The algorithm:
///   1. Build the signer's canonical ATA → mint map for every KNOWN_TOKEN.
///   2. Walk top-level instructions:
///      - System.Transfer FROM signer to a destination that is the
///        signer's canonical WSOL ATA → SOL outflow (wrap-and-swap).
///      - Token{,2022}.Transfer / TransferChecked FROM the signer's
///        canonical ATA for some known mint → token outflow.
///      - AssocToken Create whose ATA matches a signer canonical ATA →
///        record that mint as a candidate receive token.
///      - Any non-builtin program with a non-trivial account list →
///        flags this as an aggregator call (we don't decode the bytes).
///   3. The SEND mint is the mint we observed outflow against. The
///      RECEIVE mint is whichever signer-ATA was *touched* (created,
///      synced, closed) that is NOT the send mint.
fn detect_swap_shape(tx_bytes: &[u8], fee_lamports: u64) -> Option<ZonedAction> {
    let msg = message::deserialize(tx_bytes).ok()?;
    let all_accounts = lookup_tables::expand_accounts(&msg.accounts, &msg.address_table_lookups);
    let fee_payer = *all_accounts.first()?;
    let n_signers = (msg.num_required_signers as usize).min(all_accounts.len());
    let signers = &all_accounts[..n_signers];

    // Canonical ATA map for the signer(s). Keys are ATA pubkeys so we
    // can do `ata_map.get(&account)` to ask "is this account the signer's
    // canonical ATA, and if so for which token?".
    let ata_map = token_registry::build_ata_map(signers);
    if ata_map.is_empty() {
        return None;
    }

    // Outflow tracking. A swap moves at most one SOL outflow OR one token
    // outflow at the top level — the rest happens inside the aggregator
    // CPI and isn't visible to us.
    let mut sol_outflow_into_wrap: u64 = 0;
    let mut token_outflow: Option<(u64, token_registry::AtaEntry)> = None;

    // Receive-side tracking: any signer-canonical ATA the tx touches
    // (creates via AssocToken, transfers into via Token, syncs as WSOL)
    // is a candidate for the swap's terminal token. Multiple may be
    // touched (e.g. wrap-and-swap touches both WSOL + the receive
    // token); we disambiguate later by excluding the SEND mint.
    let mut signer_atas_touched: Vec<token_registry::AtaEntry> = Vec::new();
    let record_touched = |entry: token_registry::AtaEntry,
                          acc: &mut Vec<token_registry::AtaEntry>| {
        if !acc.iter().any(|e| e.mint == entry.mint) {
            acc.push(entry);
        }
    };

    // Did we see any non-builtin program ix with a non-trivial account
    // list? That's our heuristic for "this is an aggregator call".
    // Without it we don't classify as a swap — a tx that's just
    // ComputeBudget + Token.Transfer is a plain token send, not a swap.
    let mut has_aggregator_call = false;

    for ix in &msg.instructions {
        let pid = match all_accounts.get(ix.program_id_index) {
            Some(k) => k,
            None => continue,
        };
        let program_name = programs::identify(pid).map(|p| p.name);

        match program_name {
            Some("ComputeBudget") | Some("Memo") | Some("Stake") | Some("Vote") => {}

            Some("System") if ix.data.len() >= 12 => {
                let ix_type =
                    u32::from_le_bytes([ix.data[0], ix.data[1], ix.data[2], ix.data[3]]);
                if ix_type == 2 {
                    // System.Transfer
                    let from_idx = *ix.account_indices.first()? as usize;
                    let to_idx = *ix.account_indices.get(1)? as usize;
                    let from = all_accounts.get(from_idx)?;
                    let to = all_accounts.get(to_idx)?;
                    if from == &fee_payer {
                        let lamports = u64::from_le_bytes([
                            ix.data[4], ix.data[5], ix.data[6], ix.data[7],
                            ix.data[8], ix.data[9], ix.data[10], ix.data[11],
                        ]);
                        // Only count the outflow toward the swap when the
                        // destination is the signer's own WSOL ATA (i.e.
                        // wrap-and-swap). A System.Transfer to an arbitrary
                        // address is a plain SOL send and would have been
                        // caught by `extract_send` earlier.
                        if let Some(entry) = ata_map.get(to) {
                            if token_registry::lookup(&entry.mint).map_or(false, |i| i.symbol == "SOL") {
                                sol_outflow_into_wrap =
                                    sol_outflow_into_wrap.saturating_add(lamports);
                                record_touched(*entry, &mut signer_atas_touched);
                            }
                        }
                    }
                }
            }
            Some("System") => {} // CreateAccount / Allocate etc. — fine

            Some("AssocToken") => {
                // AssocToken Create / CreateIdempotent layout:
                // [funder, ata, owner, mint, system, token, rent].
                // We identify the mint by checking if `ata` is the
                // signer's canonical ATA for any KNOWN_TOKEN — we never
                // need the mint pubkey itself, which may be in an ALT we
                // don't have hardcoded.
                let disc = ix.data.first().copied().unwrap_or(0);
                if disc == 0 || disc == 1 {
                    if let Some(ata_idx) = ix.account_indices.get(1) {
                        if let Some(ata) = all_accounts.get(*ata_idx as usize) {
                            if let Some(entry) = ata_map.get(ata) {
                                record_touched(*entry, &mut signer_atas_touched);
                            }
                        }
                    }
                }
            }

            Some("Token") | Some("Token-2022") => {
                let disc = ix.data.first().copied().unwrap_or(0);
                match disc {
                    // Transfer (3) / TransferChecked (12): amount is u64
                    // at data[1..9]. Track outflow when the source ATA
                    // is the signer's canonical ATA for a known mint.
                    3 | 12 if ix.data.len() >= 9 => {
                        let amount = u64::from_le_bytes([
                            ix.data[1], ix.data[2], ix.data[3], ix.data[4],
                            ix.data[5], ix.data[6], ix.data[7], ix.data[8],
                        ]);
                        if let Some(src_idx) = ix.account_indices.first() {
                            if let Some(src) = all_accounts.get(*src_idx as usize) {
                                if let Some(entry) = ata_map.get(src) {
                                    if token_outflow.is_none() {
                                        token_outflow = Some((amount, *entry));
                                    }
                                }
                            }
                        }
                    }
                    // Close (9) / SyncNative (17) / InitializeAccount{,2,3}
                    // (1, 16, 18): touch tracking only — mark the signer
                    // ATA as touched so its mint is a receive candidate.
                    1 | 9 | 16 | 17 | 18 => {
                        if let Some(acct_idx) = ix.account_indices.first() {
                            if let Some(acct) = all_accounts.get(*acct_idx as usize) {
                                if let Some(entry) = ata_map.get(acct) {
                                    record_touched(*entry, &mut signer_atas_touched);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Known DEX / aggregator with a real parser. If we got here
            // it means the parser didn't emit spend/receive fields (e.g.
            // DFlow's unknown disc 0x2f3e9bac), so we still want to
            // attempt the pattern-based detection — count it as an
            // aggregator call.
            Some(name) if is_swap_program_name(name) => {
                has_aggregator_call = true;
            }

            // Truly unknown program (Jupiter Ultra's `iris` router
            // `proVF4pMXVa…`, future aggregators, etc.). A non-trivial
            // account list (>= 5) is our best offline signal that this
            // is the swap call rather than a tiny utility ix.
            None if ix.account_indices.len() >= 5 => {
                has_aggregator_call = true;
            }

            _ => {}
        }
    }

    if !has_aggregator_call {
        return None;
    }

    // Determine SEND. SOL wrap-and-swap takes priority — if we saw a
    // System.Transfer into the signer's WSOL ATA, that's the source side.
    // Otherwise fall back to a top-level Token.Transfer outflow.
    let (send_symbol, send_amount_raw, send_decimals, send_mint) = if sol_outflow_into_wrap > 0 {
        let sol_entry = ata_map
            .values()
            .find(|e| token_registry::lookup(&e.mint).map_or(false, |i| i.symbol == "SOL"))
            .copied()?;
        let sol_info = token_registry::lookup(&sol_entry.mint)?;
        (
            sol_info.symbol,
            sol_outflow_into_wrap,
            sol_info.decimals,
            sol_entry.mint,
        )
    } else if let Some((amount, entry)) = token_outflow {
        let info = token_registry::lookup(&entry.mint)?;
        (info.symbol, amount, info.decimals, entry.mint)
    } else {
        return None;
    };

    // Determine RECEIVE: a touched signer ATA whose mint is NOT the SEND
    // mint. Multiple candidates is also reasonable to bail on (we'd be
    // guessing which side is the swap output) but is rare in practice.
    let receive_entry = signer_atas_touched
        .iter()
        .find(|e| e.mint != send_mint)
        .copied()?;

    let send_amount = token_registry::format_amount(send_amount_raw, send_decimals);

    Some(ZonedAction::Swap {
        sell_amount: send_amount,
        sell_symbol: send_symbol.to_string(),
        // Empty `buy_amount` is the explicit "device cannot verify this
        // number offline" signal. The renderer surfaces it as a dash.
        buy_amount: String::new(),
        buy_symbol: token_registry::lookup(&receive_entry.mint).map_or_else(
            || bs58::encode(receive_entry.mint).into_string(),
            |i| i.symbol.to_string(),
        ),
        fee_lamports,
        fee_payer,
        dex_name: String::new(),
        slippage_bps: None,
        route_hops: None,
    })
}

/// Walks a tx that contains a DFlow `prepare` + `swap` ix pair. Returns
/// `(raw_out_amount, slippage_bps, dest_decimals, dest_symbol)` only when
/// the dest mint can be identified deterministically:
///   - `prepare` ix's user-ATA pubkey → source mint (one match in ata_map).
///   - `swap` ix's account list has EXACTLY ONE user-ATA whose mint differs
///     from the source. That mint is the dest.
/// Returns `None` if the dest is ambiguous (multi-hop touches multiple user
/// ATAs) — the caller falls back to raw-units display rather than guessing.
fn resolve_dflow_dest(tx_bytes: &[u8]) -> Option<(u64, u16, u8, &'static str)> {
    const PREPARE_DISC: [u8; 8] = [0x2f, 0x3e, 0x9b, 0xac, 0x83, 0xcd, 0x25, 0xc9];
    let swap_disc: [u8; 8] = anchor::discriminator("swap");

    let msg = message::deserialize(tx_bytes).ok()?;
    let all_accounts = lookup_tables::expand_accounts(&msg.accounts, &msg.address_table_lookups);
    let n_signers = (msg.num_required_signers as usize).min(all_accounts.len());
    let ata_map = token_registry::build_ata_map(&all_accounts[..n_signers]);
    if ata_map.is_empty() {
        return None;
    }

    let mut source_mint: Option<[u8; 32]> = None;
    let mut swap_account_indices: Option<&[u8]> = None;
    let mut swap_data: Option<&[u8]> = None;

    for ix in &msg.instructions {
        let pid = all_accounts.get(ix.program_id_index)?;
        if programs::identify(pid).map(|p| p.name) != Some("DFlow") {
            continue;
        }
        if ix.data.len() < 8 {
            continue;
        }
        let disc: [u8; 8] = ix.data[..8].try_into().ok()?;
        if disc == PREPARE_DISC {
            for &idx in &ix.account_indices {
                let Some(acct) = all_accounts.get(idx as usize) else {
                    continue;
                };
                if let Some(entry) = ata_map.get(acct) {
                    source_mint = Some(entry.mint);
                    break;
                }
            }
        } else if disc == swap_disc {
            swap_account_indices = Some(&ix.account_indices);
            swap_data = Some(&ix.data);
        }
    }

    let source = source_mint?;
    let swap_idx = swap_account_indices?;
    let swap_d = swap_data?;

    let mut candidates: Vec<token_registry::AtaEntry> = Vec::new();
    for &idx in swap_idx {
        let Some(acct) = all_accounts.get(idx as usize) else {
            continue;
        };
        if let Some(entry) = ata_map.get(acct) {
            if entry.mint != source && !candidates.iter().any(|e| e.mint == entry.mint) {
                candidates.push(*entry);
            }
        }
    }
    if candidates.len() != 1 {
        return None;
    }
    let dest = candidates[0];

    if swap_d.len() < 12 {
        return None;
    }
    let pos = swap_d.len() - 12;
    let raw_out = u64::from_le_bytes(swap_d[pos..pos + 8].try_into().ok()?);
    let slip = u16::from_le_bytes(swap_d[pos + 8..pos + 10].try_into().ok()?);

    let dest_info = token_registry::lookup(&dest.mint)?;
    Some((raw_out, slip, dest_info.decimals, dest_info.symbol))
}

fn extract_send(tx_bytes: &[u8]) -> Option<ZonedAction> {
    let msg = message::deserialize(tx_bytes).ok()?;
    let all_accounts = lookup_tables::expand_accounts(&msg.accounts, &msg.address_table_lookups);
    let fee_payer = *all_accounts.first()?;

    let mut transfer: Option<(usize, u64)> = None;
    for ix in &msg.instructions {
        let pid = all_accounts.get(ix.program_id_index)?;
        match programs::identify(pid).as_ref().map(|p| p.name) {
            Some("ComputeBudget") => continue,
            Some("System") if ix.data.len() >= 12 => {
                let ix_type = u32::from_le_bytes([ix.data[0], ix.data[1], ix.data[2], ix.data[3]]);
                if ix_type == 2 {
                    let to_idx = *ix.account_indices.get(1)? as usize;
                    let lamports = u64::from_le_bytes([
                        ix.data[4], ix.data[5], ix.data[6], ix.data[7],
                        ix.data[8], ix.data[9], ix.data[10], ix.data[11],
                    ]);
                    if transfer.is_some() {
                        return None;
                    }
                    transfer = Some((to_idx, lamports));
                    continue;
                }
                return None;
            }
            _ => return None,
        }
    }

    let (to_idx, amount_lamports) = transfer?;
    let to = *all_accounts.get(to_idx)?;
    Some(ZonedAction::Send {
        from: fee_payer,
        to,
        amount_lamports,
    })
}

fn split_amount_symbol_owned(value: &str) -> (String, String) {
    let trimmed = value.trim();
    // Split at the FIRST space so `"195000000 raw units"` (unresolved
    // mint) → `("195000000", "raw units")` instead of the broken
    // `("195000000 raw", "units")` that `rfind` produced — the latter
    // dragged the word "raw" into the amount cell and left the symbol
    // slot showing only "units".
    if let Some(idx) = trimmed.find(' ') {
        let amt = trimmed[..idx].trim();
        let sym = trimmed[idx + 1..].trim();
        if !amt.is_empty() && !sym.is_empty() {
            // Collapse the parser's "raw units" sentinel to a short "?"
            // marker. The literal phrase is two words wide in profont22
            // (~108 px) and overlaps the bumped-up label on the device
            // (real test: RECEIVE + "raw units" collided in the middle
            // of the cell). A "?" still signals "denomination unknown,
            // see detail pages" without breaking the layout.
            let sym_owned = if sym == "raw units" {
                "?".to_string()
            } else {
                sym.to_string()
            };
            return (compact_amount(amt), sym_owned);
        }
    }
    (compact_amount(trimmed), String::new())
}

/// Compact a decimal amount for the zoned-amount cell.
///
/// Caps fractional digits at 6 (matches USDC / SPL native precision) and
/// strips trailing zeros so `1.200000013` renders as `1.2` and a fee of
/// `0.000010005` renders as `0.000010` then trims to `0.00001`.
///
/// Whole numbers are returned unchanged. We deliberately do **not** apply
/// `K`/`M`/`B` suffixes — when the mint isn't resolved the parser surfaces
/// raw token units (e.g. `1510072000`), and abbreviating that to `1.51B`
/// reads as billions of denominated tokens, which is misleading. Better
/// to show the full number even if long; the user can tell at a glance
/// it's an unresolved raw amount and read the detail pages.
pub fn compact_amount(s: &str) -> String {
    let s = s.trim();
    if let Some(dot) = s.find('.') {
        let whole = &s[..dot];
        let frac = &s[dot + 1..];
        let cap = frac.len().min(6);
        let trimmed = frac[..cap].trim_end_matches('0');
        if trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{}.{}", whole, trimmed)
        }
    } else {
        s.to_string()
    }
}

fn decode_b58_pubkey(s: &str) -> Option<[u8; 32]> {
    let bytes = bs58::decode(s).into_vec().ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Some(out)
}

// === Entry point ===

pub fn parse(tx_bytes: &[u8]) -> ParsedTransaction {
    let msg = match message::deserialize(tx_bytes) {
        Ok(m) => m,
        Err(e) => {
            return ParsedTransaction {
                version: TransactionVersion::Legacy,
                fee_payer: "?".into(),
                signers: Vec::new(),
                instructions: vec![ParsedInstruction {
                    program: "Error".into(),
                    items: vec![ReviewItem::Warning(format!(
                        "Failed to parse transaction: {}",
                        e
                    ))],
                }],
                fee_lamports: 0,
                size: tx_bytes.len(),
            }
        }
    };

    let version = match msg.version {
        message::MessageVersion::Legacy => TransactionVersion::Legacy,
        message::MessageVersion::V0 => TransactionVersion::V0 {
            address_table_lookups: msg.address_table_lookups.len(),
        },
    };

    // Expand account list with resolved ALT entries (v0 transactions)
    let all_accounts = lookup_tables::expand_accounts(&msg.accounts, &msg.address_table_lookups);

    let n_signers = (msg.num_required_signers as usize).min(all_accounts.len());
    let signers = all_accounts[..n_signers].to_vec();

    let fee_payer = all_accounts
        .first()
        .map(|k| bs58::encode(k).into_string())
        .unwrap_or_else(|| "?".into());

    // Fee = base (sigs × 5000) + priority (ComputeBudget price × limit / 1_000_000)
    let base_fee = (signers.len() as u64).saturating_mul(5_000);
    let (cu_limit, cu_price_micro) =
        extract_compute_budget_values(&msg.instructions, &all_accounts);
    let priority = ((cu_price_micro as u128) * (cu_limit as u128) / 1_000_000u128) as u64;
    let fee_lamports = base_fee.saturating_add(priority);

    // Build ATA map for offline token resolution (only when an aggregator
    // / AMM is present — these parsers resolve mints by intersecting
    // instruction accounts against the wallet's known token accounts).
    let needs_ata = all_accounts
        .iter()
        .any(|acct| programs::identify(acct).map_or(false, |p| is_swap_program_name(p.name)));
    let mut ata_map = if needs_ata {
        token_registry::build_ata_map(&all_accounts[..n_signers])
    } else {
        token_registry::AtaMap::new()
    };

    // Augment the static (signer × known-mint) ATA map with token accounts
    // this tx creates or touches explicitly. Motivating case: Jupiter /
    // Raydium often route through a temporary wrapped-SOL account that
    // isn't the signer's canonical ATA. Without this, the dest mint stays
    // unresolved and the review screen shows "11598426 raw units" instead
    // of "0.012 SOL". See `discover_dynamic_token_accounts`.
    if needs_ata {
        augment_ata_map_from_ixs(&mut ata_map, &msg.instructions, &all_accounts);
    }

    let instructions = msg
        .instructions
        .iter()
        .map(|ix| dispatch(ix, &all_accounts, &ata_map))
        .collect();

    ParsedTransaction {
        version,
        fee_payer,
        signers,
        instructions,
        fee_lamports,
        size: tx_bytes.len(),
    }
}

/// Convert a `ParsedTransaction` into a flat list of strings for display.
/// Keeps `SignReview` screen unchanged while the renderer is simple text-only.
pub fn to_lines(tx: &ParsedTransaction) -> Vec<String> {
    let mut lines = Vec::new();

    let version_str = match &tx.version {
        TransactionVersion::Legacy => "Legacy".into(),
        TransactionVersion::V0 {
            address_table_lookups: 0,
        } => "v0".into(),
        TransactionVersion::V0 {
            address_table_lookups: n,
        } => format!("v0 ({} lookup tables)", n),
    };

    let payer_short = if tx.fee_payer.len() >= 8 {
        format!(
            "{}..{}",
            &tx.fee_payer[..4],
            &tx.fee_payer[tx.fee_payer.len() - 4..]
        )
    } else {
        tx.fee_payer.clone()
    };

    lines.push(format!("Tx: {}  Signer: {}", version_str, payer_short));
    lines.push(format!("Instructions: {}", tx.instructions.len()));
    lines.push(String::new());

    let multi = tx.instructions.len() > 1;
    for (i, ix) in tx.instructions.iter().enumerate() {
        if multi {
            lines.push(format!(
                "-- {}/{}: {} --",
                i + 1,
                tx.instructions.len(),
                ix.program
            ));
        }
        for item in &ix.items {
            match item {
                ReviewItem::Header(s) => lines.push(format!("[{}]", s)),
                ReviewItem::Field { label, value } => {
                    if label.is_empty() {
                        lines.push(format!("  {}", value));
                    } else {
                        lines.push(format!("  {}: {}", label, value));
                    }
                }
                ReviewItem::Warning(s) => lines.push(format!("! {}", s)),
            }
        }
        if multi && i + 1 < tx.instructions.len() {
            lines.push(String::new());
        }
    }

    lines.push(String::new());
    lines.push(format!("Fee: {}", system::lamports_to_sol(tx.fee_lamports)));
    lines.push(format!("Size: {} bytes", tx.size));

    lines
}

/// Build review lines for the Sign screen, including a `can_sign` check
/// and the structured `ParsedTransaction` (so detail-page renderers can
/// read instructions / accounts directly without re-parsing).
///
/// The flat lines start with a conservative pre-sign classification, then
/// include full decoded transaction details so the user can verify every
/// instruction on the page-0 summary.
pub fn build_review_lines(
    tx_bytes: &[u8],
    wallet_pubkey: &[u8; 32],
) -> (Vec<String>, bool, ParsedTransaction) {
    let parsed = parse(tx_bytes);
    let can_sign = parsed.signers.iter().any(|s| s == wallet_pubkey);
    let mut lines = Vec::new();
    let classification = classification::classify(tx_bytes, wallet_pubkey);
    let primary = primary_instruction(&parsed);
    let signing_hash = signing_message_sha256_hex(tx_bytes);

    if let Some(classification) = &classification {
        if classification.high_risk {
            lines.push(format!("! {}", classification.headline()));
            lines.push(format!("! {}", classification.summary));
        }
    }

    add_hero_action_lines(&mut lines, primary, classification.as_ref());

    // Signer-required warning is the only @HM we keep in the hero zone —
    // it's a blocker the user must see at a glance. Everything else
    // (fee payer, msg hash, hints) lives in the scrollable details block
    // so the hero stays compact and the detail zone actually has room.
    if !can_sign {
        if let Some(needed) = parsed.signers.first() {
            lines.push(format!("@HM Signer required: {}", pubkey_short(needed)));
        }
    }

    // Scrollable details block. The signing hash used to sit near the
    // top of this block but it's only useful for power users doing
    // external verification — readers kept confusing "Message SHA256"
    // for the decoded message itself. Moved to the very end of details
    // so the first things the user encounters on scroll are the actual
    // decoded instructions.
    lines.push(String::new());
    lines.push(format!("Fee payer: {}", short_b58(&parsed.fee_payer)));
    lines.push(String::new());
    lines.push("Review full transaction details:".to_string());
    lines.push(String::new());
    lines.extend(to_lines(&parsed));
    if let Some(hash) = &signing_hash {
        lines.push(String::new());
        lines.push("Signing hash (SHA-256):".to_string());
        lines.push(hash.clone());
    }

    if !can_sign {
        lines.push(String::new());
        lines.push("! Cannot sign this TX".to_string());
        if let Some(needed) = parsed.signers.first() {
            let addr = bs58::encode(needed).into_string();
            lines.push("! Need wallet:".to_string());
            for chunk in addr.as_bytes().chunks(22) {
                lines.push(format!("!  {}", std::str::from_utf8(chunk).unwrap_or("")));
            }
        }
    }

    (lines, can_sign, parsed)
}

fn primary_instruction(parsed: &ParsedTransaction) -> Option<&ParsedInstruction> {
    let candidates: Vec<&ParsedInstruction> = parsed
        .instructions
        .iter()
        .filter(|ix| ix.program != "ComputeBudget" && ix.program != "Error")
        .collect();

    // Priority order — earlier wins:
    //   1. Any ix that parsed cleanly with swap fields (`You spend`/`You receive`).
    //   2. Any ix from a known DEX/aggregator program — even if its specific
    //      variant didn't parse to swap fields. This catches the case where a
    //      Jupiter `RouteV2` falls through to a generic header but the tx
    //      *also* contains a side-effect like "Create Token Account": the
    //      user wants to see `SWAP`, not the scaffolding.
    //   3. Any ix with transfer fields (`From`/`To`).
    //   4. First non-Unknown ix.
    //   5. First candidate as last resort.
    candidates
        .iter()
        .copied()
        .find(|ix| has_swap_fields(ix))
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .find(|ix| is_swap_program_name(&ix.program))
        })
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .find(|ix| has_transfer_fields(ix))
        })
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .find(|ix| !ix.program.starts_with("Unknown"))
        })
        .or_else(|| candidates.first().copied())
}

fn has_swap_fields(ix: &ParsedInstruction) -> bool {
    let has_spend =
        field_value(ix, "You spend").is_some() || field_value(ix, "You spend (max)").is_some();
    let has_receive =
        field_value(ix, "You receive").is_some() || field_value(ix, "You receive (min)").is_some();
    has_spend && has_receive
}

fn has_transfer_fields(ix: &ParsedInstruction) -> bool {
    field_value(ix, "From").is_some() && field_value(ix, "To").is_some()
}

fn add_hero_action_lines(
    lines: &mut Vec<String>,
    primary: Option<&ParsedInstruction>,
    classification: Option<&classification::Classification>,
) {
    let Some(ix) = primary else {
        lines.push("@H1 REVIEW".to_string());
        lines.push("@H2 Unable to decode action".to_string());
        return;
    };

    // Classification headline ("Likely: DeFi swap (90%)") was previously
    // emitted as an @HM hero-meta line. Dropped: the @H1 title already
    // communicates the tx category ("SWAP" / "TRANSFER" / ...) without the
    // confidence-score noise. High-risk classifications still emit "!"
    // warning lines above the hero.
    let _ = classification;

    let spend = field_value(ix, "You spend").or_else(|| field_value(ix, "You spend (max)"));
    let receive = field_value(ix, "You receive").or_else(|| field_value(ix, "You receive (min)"));
    if let (Some(spend), Some(receive)) = (spend, receive) {
        lines.push("@H1 SWAP".to_string());
        // Two-column layout when both sides split cleanly into "<amount>
        // <symbol>": amounts stacked over symbols on each side with a
        // small "to" connector between. Handles large amounts gracefully
        // because each side gets ~half the body width independently.
        // Falls back to the previous vertical layout if either side can't
        // be split (e.g. unresolved mint → no symbol).
        if let (Some((amt_in, sym_in)), Some((amt_out, sym_out))) =
            (split_amount_symbol(spend), split_amount_symbol(receive))
        {
            // Each hero column is ~9 chars wide at 9×15 font. Longer
            // amounts would collide with the "to" connector, so trim
            // fractional digits until they fit. If even the whole part is
            // too long, fall through to the vertical stack so nothing
            // gets cropped.
            const COL_BUDGET: usize = 9;
            let amt_in_hero = truncate_amount(amt_in, COL_BUDGET);
            let amt_out_hero = truncate_amount(amt_out, COL_BUDGET);
            if amt_in_hero.len() <= COL_BUDGET && amt_out_hero.len() <= COL_BUDGET {
                // `\t` is a safe delimiter — SPL symbols are base58-ish
                // short tokens, never contain tab characters. The renderer
                // splits on tabs to recover the four fields for columns.
                lines.push(format!(
                    "@SWAPPAIR {}\t{}\t{}\t{}",
                    amt_in_hero, sym_in, amt_out_hero, sym_out
                ));
            } else {
                lines.push(format!("@H2 {} {}", amt_in, sym_in));
                lines.push("@HM   ↓".to_string());
                lines.push(format!("@H2 {} {}", amt_out, sym_out));
            }
        } else {
            lines.push(format!("@H2 {}", spend));
            lines.push("@HM   ↓".to_string());
            lines.push(format!("@H2 {}", receive));
        }
        return;
    }

    let from = field_value(ix, "From");
    let to = field_value(ix, "To");
    if let (Some(_from), Some(to)) = (from, to) {
        let amount = field_value(ix, "Amount").unwrap_or("?");
        // Hero stays at ~3 rows to match the swap hero, so the scrollable
        // detail zone gets consistent vertical space across tx types.
        // "From" drops off the hero: the source is always the paired
        // wallet in a self-signed tx, which the user already knows —
        // the recipient is the piece of info that actually needs review.
        // Full source address is still in the detail block via `to_lines`.
        lines.push("@H1 TRANSFER".to_string());
        lines.push(format!("@H2 {}", amount));
        lines.push(format!("@HM → {}", to));
        return;
    }

    if is_swap_program_name(&ix.program) {
        lines.push("@H1 SWAP".to_string());
        if let Some(header) = first_header(ix) {
            lines.push(format!("@H2 {}", header));
        } else {
            lines.push(format!("@H2 {}", ix.program));
        }
        return;
    }

    // ATA-creation txs (AssocToken Create) often arrive on their own — a
    // wallet sends a one-shot setup tx before the actual swap/transfer.
    // The generic "ACTION / Create Token Account" copy left the user with
    // no idea what they were actually approving. Use a dedicated SETUP
    // hero so this kind of preflight is visually distinct from a real
    // action, and the `parse_assoc_token` enriched header tells them
    // *which token* the account is for.
    let tag = if ix.program.starts_with("Unknown") {
        "@H1 UNKNOWN"
    } else if ix.program == "AssocToken" {
        "@H1 SETUP"
    } else {
        "@H1 ACTION"
    };
    if let Some(header) = first_header(ix) {
        lines.push(tag.to_string());
        lines.push(format!("@H2 {}", header));
    } else {
        lines.push(tag.to_string());
        lines.push(format!("@H2 {}", ix.program));
    }
    // For SETUP, add a small hint so the user knows they may need to sign
    // a follow-up tx (the actual swap/transfer this account was set up for).
    if ix.program == "AssocToken" {
        lines.push("@HM One-time setup".to_string());
    }
}

/// Indices of instructions worth dedicating a detail page to. Filters out
/// boilerplate (ComputeBudget) so the per-instruction navigation focuses on
/// the few ixs that actually move funds.
pub fn interesting_ix_indices(parsed: &ParsedTransaction) -> Vec<usize> {
    parsed
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, ix)| ix.program != "ComputeBudget")
        .map(|(i, _)| i)
        .collect()
}

pub(crate) fn is_swap_program_name(program: &str) -> bool {
    matches!(
        program,
        "Jupiter"
            | "Jupiter v4 (legacy)"
            | "Jupiter Ultra"
            | "Jupiter RFQ"
            | "Raydium AMM"
            | "Raydium CLMM"
            | "Raydium CPMM"
            | "DFlow"
            | "Orca Whirlpools"
            | "Meteora DLMM"
            | "Phoenix"
            | "Pump.fun"
    )
}

fn first_header(ix: &ParsedInstruction) -> Option<&str> {
    ix.items.iter().find_map(|item| match item {
        ReviewItem::Header(s) => Some(s.as_str()),
        _ => None,
    })
}

/// Split a formatted token value of the form `"<amount> <symbol>"` into
/// its two parts. Returns None if the string has no internal whitespace —
/// that shape means the mint was unresolved and the caller should fall
/// back to the vertical-stack hero layout instead of the two-column one.
fn split_amount_symbol(value: &str) -> Option<(&str, &str)> {
    let trimmed = value.trim();
    let idx = trimmed.rfind(' ')?;
    let (amt, sym) = (trimmed[..idx].trim(), trimmed[idx + 1..].trim());
    if amt.is_empty() || sym.is_empty() {
        return None;
    }
    Some((amt, sym))
}

/// Shrink a formatted decimal amount so it fits within `max_chars` by
/// dropping fractional digits from the right. Preserves the whole part
/// (and the decimal point if any fractional digit survives). Full
/// precision is still available in the scrollable detail block — this
/// only trims the hero-line copy where horizontal space is tight.
///
/// If the whole part alone is already over budget, returns the input
/// unchanged so the caller can detect the case and fall back to a
/// different layout.
fn truncate_amount(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    if let Some((whole, frac)) = s.split_once('.') {
        if whole.len() >= max_chars {
            // Can't even fit the whole part — surface unchanged so the
            // caller knows truncation didn't help.
            return s.to_string();
        }
        let available = max_chars - whole.len() - 1; // -1 for the "."
        if available == 0 {
            return whole.to_string();
        }
        let take = frac.len().min(available);
        return format!("{}.{}", whole, &frac[..take]);
    }
    s.to_string()
}

fn field_value<'a>(ix: &'a ParsedInstruction, label: &str) -> Option<&'a str> {
    ix.items.iter().find_map(|item| match item {
        ReviewItem::Field {
            label: item_label,
            value,
        } if item_label == label => Some(value.as_str()),
        _ => None,
    })
}

fn short_b58(value: &str) -> String {
    if value.len() >= 8 {
        format!("{}..{}", &value[..4], &value[value.len() - 4..])
    } else {
        value.to_string()
    }
}

fn signing_message_sha256_hex(tx_bytes: &[u8]) -> Option<String> {
    let message = signing_message_bytes(tx_bytes)?;
    let digest = sha2::Sha256::digest(message);
    Some(hex::encode(digest))
}

fn signing_message_bytes(tx_bytes: &[u8]) -> Option<&[u8]> {
    if tx_bytes.is_empty() {
        return None;
    }
    let num_sigs = tx_bytes[0] as usize;
    let sig_bytes = num_sigs.checked_mul(64)?;
    let start = 1usize.checked_add(sig_bytes)?;
    tx_bytes.get(start..)
}

/// Extend the ATA map with `(token_account → mint)` pairs discovered by
/// scanning the tx's own instructions. Catches accounts created mid-tx
/// (Jupiter temp WSOL wrappers, any fresh ATA) that the static signer-based
/// map can't know about.
///
/// Only inserts when the mint is in `KNOWN_TOKENS` — an unknown mint wouldn't
/// give us usable `symbol` / `decimals`, and the fallback "raw units" output
/// is still better than a misleading zero-decimal number.
fn augment_ata_map_from_ixs(
    ata_map: &mut token_registry::AtaMap,
    instructions: &[message::RawInstruction],
    all_accounts: &[[u8; 32]],
) {
    const WSOL_MINT_B58: &str = "So11111111111111111111111111111111111111112";
    let wsol_mint: [u8; 32] = {
        let bytes = bs58::decode(WSOL_MINT_B58).into_vec().unwrap_or_default();
        let mut out = [0u8; 32];
        if bytes.len() == 32 {
            out.copy_from_slice(&bytes);
        }
        out
    };

    let resolve = |idx: Option<&u8>| -> Option<[u8; 32]> {
        let i = *idx? as usize;
        all_accounts.get(i).copied()
    };
    let try_insert = |map: &mut token_registry::AtaMap, account: [u8; 32], mint: [u8; 32]| {
        if map.contains_key(&account) {
            return;
        }
        if token_registry::lookup(&mint).is_some() {
            map.insert(
                account,
                token_registry::AtaEntry { mint },
            );
        }
    };

    for ix in instructions {
        let program_id = match all_accounts.get(ix.program_id_index) {
            Some(k) => k,
            None => continue,
        };
        let program_name = programs::identify(program_id).map(|p| p.name);
        let idxs = &ix.account_indices;

        match program_name {
            // Associated Token Account program. Both Create (implicit/0) and
            // CreateIdempotent (1) use the same account layout:
            //   [funder, ata, owner, mint, system, token, rent]
            // RecoverNested (disc 2) has a different shape — skip.
            Some("AssocToken") => {
                let disc = ix.data.first().copied().unwrap_or(0);
                if disc == 0 || disc == 1 {
                    if let (Some(ata), Some(mint)) = (resolve(idxs.get(1)), resolve(idxs.get(3))) {
                        try_insert(ata_map, ata, mint);
                    }
                }
            }
            // SPL Token program. InitializeAccount variants write the mint
            // directly as accounts[1]. SyncNative targets a wrapped-SOL
            // account (mint is implicitly WSOL) so we pin it to WSOL.
            //   1  InitializeAccount    [acct, mint, owner, rent]
            //   16 InitializeAccount2   [acct, mint, rent]     (owner in data)
            //   18 InitializeAccount3   [acct, mint]           (owner in data)
            //   17 SyncNative           [acct]                 (mint = WSOL)
            Some("Token") | Some("Token-2022") => match ix.data.first().copied() {
                Some(1) | Some(16) | Some(18) => {
                    if let (Some(acct), Some(mint)) = (resolve(idxs.get(0)), resolve(idxs.get(1)))
                    {
                        try_insert(ata_map, acct, mint);
                    }
                }
                Some(17) => {
                    if let Some(acct) = resolve(idxs.get(0)) {
                        try_insert(ata_map, acct, wsol_mint);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

// === Internal dispatcher ===

fn dispatch(
    ix: &message::RawInstruction,
    all_accounts: &[[u8; 32]],
    ata_map: &token_registry::AtaMap,
) -> ParsedInstruction {
    let program_id = match all_accounts.get(ix.program_id_index) {
        Some(id) => id,
        None => return unknown::parse(&[0u8; 32], &ix.data, &[]),
    };

    let resolved_accounts: Vec<[u8; 32]> = ix
        .account_indices
        .iter()
        .filter_map(|&idx| all_accounts.get(idx as usize).copied())
        .collect();

    match programs::identify(program_id).as_ref().map(|p| p.name) {
        Some("System") => system::parse(&ix.data, &resolved_accounts),
        Some("Token") => token::parse("Token", &ix.data, &resolved_accounts),
        Some("Token-2022") => token::parse("Token-2022", &ix.data, &resolved_accounts),
        Some("Stake") => stake::parse(&ix.data, &resolved_accounts),
        Some("AssocToken") => parse_assoc_token(program_id, &ix.data, &resolved_accounts),
        Some("Memo") => parse_memo(&ix.data),
        Some("ComputeBudget") => parse_compute_budget(&ix.data),
        Some("Jupiter") => jupiter::parse(&ix.data, &ix.account_indices, all_accounts, ata_map),
        Some("Jupiter Ultra") => {
            jupiter_ultra::parse(&ix.data, &ix.account_indices, all_accounts, ata_map)
        }
        Some("Jupiter RFQ") => {
            jupiter_rfq::parse(&ix.data, &ix.account_indices, all_accounts)
        }
        Some("DFlow") => dflow::parse(&ix.data, &ix.account_indices, all_accounts, ata_map),
        Some("Raydium AMM") => {
            raydium::amm_v4::parse(&ix.data, &ix.account_indices, all_accounts, ata_map)
        }
        Some("Raydium CLMM") => {
            raydium::clmm::parse(&ix.data, &ix.account_indices, all_accounts, ata_map)
        }
        Some("Raydium CPMM") => {
            raydium::cpmm::parse(&ix.data, &ix.account_indices, all_accounts, ata_map)
        }
        Some(name) => ParsedInstruction {
            program: name.into(),
            items: vec![ReviewItem::Header(name.into())],
        },
        None => unknown::parse(program_id, &ix.data, &resolved_accounts),
    }
}

fn parse_assoc_token(
    _program_id: &[u8; 32],
    _data: &[u8],
    accounts: &[[u8; 32]],
) -> ParsedInstruction {
    // Per AssocToken IDL the account layout is
    //   [funder, ata, owner, mint, system, token, rent]
    // We were previously reading wallet=accounts[0] (funder) and
    // mint=accounts[2] (owner), which produced the bizarre device
    // display `Mint: 36cB..SJon` — the signer pubkey rendered as a
    // mint. Read from the actual IDL-mandated slots.
    let wallet = accounts
        .get(2)
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    // Resolve the mint twice: as full bytes for the offline registry
    // lookup (so the header reads "Setup USDC account" instead of the
    // bare "Create Token Account"), and as a shortened display string
    // for the Mint review row.
    let mint_bytes = accounts.get(3).copied();
    let mint_display = mint_bytes
        .as_ref()
        .map(|b| pubkey_short(b))
        .unwrap_or_else(|| "?".into());
    let symbol = mint_bytes
        .as_ref()
        .and_then(|b| token_registry::lookup(b))
        .map(|info| info.symbol);

    // Header drives the @H2 hero line. With a known mint we can name the
    // account the user is setting up; without one we keep it generic but
    // friendlier than "Create Token Account."
    let header = match symbol {
        Some(sym) => format!("Setup {} account", sym),
        None => "Token account setup".to_string(),
    };

    ParsedInstruction {
        program: "AssocToken".into(),
        items: vec![
            ReviewItem::Header(header),
            ReviewItem::Field {
                label: "Wallet".into(),
                value: wallet,
            },
            ReviewItem::Field {
                label: "Mint".into(),
                value: mint_display,
            },
        ],
    }
}

fn parse_memo(data: &[u8]) -> ParsedInstruction {
    let text = std::str::from_utf8(data)
        .unwrap_or("(invalid UTF-8)")
        .chars()
        .take(64)
        .collect::<String>();
    ParsedInstruction {
        program: "Memo".into(),
        items: vec![
            ReviewItem::Header("Memo".into()),
            ReviewItem::Field {
                label: "Text".into(),
                value: text,
            },
        ],
    }
}

fn parse_compute_budget(data: &[u8]) -> ParsedInstruction {
    let discriminant = data.first().copied().unwrap_or(0xff);
    let detail = match discriminant {
        0x02 if data.len() >= 5 => {
            let units = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
            format!("Limit: {} CU", units)
        }
        0x03 if data.len() >= 5 => {
            let price = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
            format!("Price: {} microlamports/CU", price)
        }
        _ => format!("Type {}", discriminant),
    };
    ParsedInstruction {
        program: "ComputeBudget".into(),
        items: vec![
            ReviewItem::Header("Compute Budget".into()),
            ReviewItem::Field {
                label: "Setting".into(),
                value: detail,
            },
        ],
    }
}

/// Extract CU limit and price from raw instructions (before dispatch).
fn extract_compute_budget_values(
    instructions: &[message::RawInstruction],
    all_accounts: &[[u8; 32]],
) -> (u32, u64) {
    let mut limit: u32 = 200_000;
    let mut price_micro: u64 = 0;
    for ix in instructions {
        let pid = match all_accounts.get(ix.program_id_index) {
            Some(id) => id,
            None => continue,
        };
        let is_cb = programs::identify(pid).as_ref().map(|p| p.name) == Some("ComputeBudget");
        if !is_cb || ix.data.is_empty() {
            continue;
        }
        match ix.data[0] {
            2 if ix.data.len() >= 5 => {
                limit = u32::from_le_bytes(ix.data[1..5].try_into().unwrap_or([0; 4]));
            }
            3 if ix.data.len() >= 9 => {
                price_micro = u64::from_le_bytes(ix.data[1..9].try_into().unwrap_or([0; 8]));
            }
            _ => {}
        }
    }
    (limit, price_micro)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a legacy System Transfer transaction.
    fn system_transfer_tx(from: [u8; 32], lamports: u64) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1u8);
        tx.extend_from_slice(&[0u8; 64]);
        // header
        tx.push(1);
        tx.push(0);
        tx.push(1);
        // 2 accounts: signer + system program (all zeros)
        tx.push(2);
        tx.extend_from_slice(&from);
        tx.extend_from_slice(&[0u8; 32]);
        // blockhash
        tx.extend_from_slice(&[0xABu8; 32]);
        // 1 instruction
        tx.push(1);
        tx.push(1); // program_id_index = 1 (system program)
        tx.push(1);
        tx.push(0); // 1 account: index 0
        tx.push(12); // data len
        tx.extend_from_slice(&[2u8, 0, 0, 0]); // Transfer
        tx.extend_from_slice(&lamports.to_le_bytes());
        tx
    }

    fn v0_system_transfer_tx(from: [u8; 32], lamports: u64) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1u8);
        tx.extend_from_slice(&[0u8; 64]);
        tx.push(0x80); // v0 prefix
        tx.push(1);
        tx.push(0);
        tx.push(1);
        tx.push(2);
        tx.extend_from_slice(&from);
        tx.extend_from_slice(&[0u8; 32]);
        tx.extend_from_slice(&[0xABu8; 32]);
        tx.push(1);
        tx.push(1);
        tx.push(1);
        tx.push(0);
        tx.push(12);
        tx.extend_from_slice(&[2u8, 0, 0, 0]);
        tx.extend_from_slice(&lamports.to_le_bytes());
        tx.push(0); // 0 address table lookups
        tx
    }

    // --- parse() ---

    #[test]
    fn test_parse_legacy_system_transfer() {
        let tx = system_transfer_tx([0x01; 32], 2_000_000_000);
        let parsed = parse(&tx);
        assert!(matches!(parsed.version, TransactionVersion::Legacy));
        assert_eq!(parsed.instructions.len(), 1);
        assert_eq!(parsed.instructions[0].program, "System");
        let has_amount = parsed.instructions[0].items.iter().any(|i| {
            matches!(
                i, ReviewItem::Field { label, value } if label == "Amount" && value == "2 SOL"
            )
        });
        assert!(has_amount);
    }

    #[test]
    fn test_parse_v0_transaction() {
        let tx = v0_system_transfer_tx([0x01; 32], 1_000_000_000);
        let parsed = parse(&tx);
        assert!(matches!(
            parsed.version,
            TransactionVersion::V0 {
                address_table_lookups: 0
            }
        ));
        assert_eq!(parsed.instructions.len(), 1);
    }

    #[test]
    fn test_parse_fee_payer_is_first_account() {
        let from = [0xAAu8; 32];
        let tx = system_transfer_tx(from, 1_000_000_000);
        let parsed = parse(&tx);
        assert_eq!(parsed.fee_payer, bs58::encode(&from).into_string());
    }

    #[test]
    fn test_parse_invalid_bytes_returns_error_instruction() {
        let parsed = parse(&[0xFF, 0x00]);
        assert_eq!(parsed.instructions.len(), 1);
        let has_warning = parsed.instructions[0]
            .items
            .iter()
            .any(|i| matches!(i, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    #[test]
    fn test_parse_empty_bytes_returns_error_instruction() {
        let parsed = parse(&[]);
        let has_warning = parsed.instructions[0]
            .items
            .iter()
            .any(|i| matches!(i, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    // --- to_lines() ---

    #[test]
    fn test_to_lines_contains_version() {
        let tx = system_transfer_tx([0x01; 32], 1_000_000_000);
        let parsed = parse(&tx);
        let lines = to_lines(&parsed);
        assert!(lines[0].contains("Legacy"));
    }

    #[test]
    fn test_to_lines_v0_shows_version() {
        let tx = v0_system_transfer_tx([0x01; 32], 1_000_000_000);
        let parsed = parse(&tx);
        let lines = to_lines(&parsed);
        assert!(lines[0].contains("v0"));
    }

    #[test]
    fn test_to_lines_contains_amount() {
        let tx = system_transfer_tx([0x01; 32], 500_000_000);
        let parsed = parse(&tx);
        let lines = to_lines(&parsed);
        let has_amount = lines.iter().any(|l| l.contains("0.5 SOL"));
        assert!(has_amount);
    }

    #[test]
    fn test_to_lines_multi_instruction_shows_count() {
        // Build a tx with 2 identical Transfer instructions
        let mut tx = Vec::new();
        tx.push(1u8);
        tx.extend_from_slice(&[0u8; 64]);
        tx.push(1);
        tx.push(0);
        tx.push(1);
        tx.push(2);
        tx.extend_from_slice(&[0x01u8; 32]);
        tx.extend_from_slice(&[0x00u8; 32]);
        tx.extend_from_slice(&[0xABu8; 32]);
        tx.push(2); // 2 instructions (compact-u16)
        for _ in 0..2 {
            tx.push(1);
            tx.push(1);
            tx.push(0);
            tx.push(12);
            tx.extend_from_slice(&[2u8, 0, 0, 0]);
            tx.extend_from_slice(&1_000_000_000u64.to_le_bytes());
        }
        let parsed = parse(&tx);
        assert_eq!(parsed.instructions.len(), 2);
        let lines = to_lines(&parsed);
        // Multi-instruction lines include "1/2" and "2/2" markers
        let has_counter = lines.iter().any(|l| l.contains("1/2"));
        assert!(has_counter);
    }

    // --- build_review_lines() ---

    #[test]
    fn test_build_review_lines_includes_full_instruction_details() {
        let mut tx = Vec::new();
        tx.push(1u8);
        tx.extend_from_slice(&[0u8; 64]);
        tx.push(1);
        tx.push(0);
        tx.push(1);
        tx.push(2);
        tx.extend_from_slice(&[0x01u8; 32]);
        tx.extend_from_slice(&[0x00u8; 32]);
        tx.extend_from_slice(&[0xABu8; 32]);
        tx.push(2);
        for _ in 0..2 {
            tx.push(1);
            tx.push(1);
            tx.push(0);
            tx.push(12);
            tx.extend_from_slice(&[2u8, 0, 0, 0]);
            tx.extend_from_slice(&1_000_000_000u64.to_le_bytes());
        }

        let (lines, can_sign, _parsed) = build_review_lines(&tx, &[0x01u8; 32]);
        assert!(can_sign);
        assert!(lines.iter().any(|line| line == "@H1 TRANSFER"));
        assert!(lines.iter().any(|line| line == "@H2 1 SOL"));
        assert!(lines
            .iter()
            .any(|line| line == "Review full transaction details:"));
        assert!(lines.iter().any(|line| line.contains("-- 1/2: System --")));
        assert!(lines.iter().any(|line| line.contains("-- 2/2: System --")));
    }

    #[test]
    fn test_build_review_lines_marks_set_authority_as_security() {
        fn pubkey_from_b58(s: &str) -> [u8; 32] {
            let bytes = bs58::decode(s).into_vec().unwrap();
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            key
        }

        let wallet = [0x01u8; 32];
        let token_program = pubkey_from_b58("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let token_account = [0x09u8; 32];

        let mut tx = Vec::new();
        tx.push(1u8);
        tx.extend_from_slice(&[0u8; 64]);
        tx.push(1);
        tx.push(0);
        tx.push(1);
        tx.push(3);
        tx.extend_from_slice(&wallet);
        tx.extend_from_slice(&token_program);
        tx.extend_from_slice(&token_account);
        tx.extend_from_slice(&[0xABu8; 32]);
        tx.push(1);
        tx.push(1);
        tx.push(2);
        tx.push(2);
        tx.push(0);
        tx.push(3);
        tx.extend_from_slice(&[6u8, 2u8, 0u8]);

        let (lines, can_sign, _parsed) = build_review_lines(&tx, &wallet);
        assert!(can_sign);
        assert!(lines
            .iter()
            .any(|line| line.contains("Security: authority change")));
        assert!(lines.iter().any(|line| line.contains("Set Authority")));
    }

    #[test]
    fn test_hero_action_prefers_swap_spend_receive() {
        let ix = ParsedInstruction {
            program: "Jupiter".to_string(),
            items: vec![
                ReviewItem::Header("Jupiter Swap".to_string()),
                ReviewItem::Field {
                    label: "You spend".to_string(),
                    value: "1 SOL".to_string(),
                },
                ReviewItem::Field {
                    label: "You receive (min)".to_string(),
                    value: "150 USDC".to_string(),
                },
            ],
        };

        let mut lines = Vec::new();
        add_hero_action_lines(&mut lines, Some(&ix), None);
        assert!(lines.iter().any(|line| line == "@H1 SWAP"));
        // Two-column hero: the spend/receive pair is emitted as a single
        // @SWAPPAIR row with tab-separated amount/symbol fields. The
        // renderer splits this into two stacked columns with a "to"
        // connector between them — handles large amounts without overflow.
        assert!(lines
            .iter()
            .any(|line| line == "@SWAPPAIR 1\tSOL\t150\tUSDC"));
    }

    #[test]
    fn test_primary_instruction_prefers_swap_candidate() {
        let parsed = ParsedTransaction {
            version: TransactionVersion::Legacy,
            fee_payer: "payer".to_string(),

            signers: vec![[0u8; 32]],
            instructions: vec![
                ParsedInstruction {
                    program: "Jupiter".to_string(),
                    items: vec![ReviewItem::Header("Jupiter (non-swap)".to_string())],
                },
                ParsedInstruction {
                    program: "Jupiter".to_string(),
                    items: vec![
                        ReviewItem::Header("Jupiter Swap".to_string()),
                        ReviewItem::Field {
                            label: "You spend".to_string(),
                            value: "1 SOL".to_string(),
                        },
                        ReviewItem::Field {
                            label: "You receive (min)".to_string(),
                            value: "150 USDC".to_string(),
                        },
                    ],
                },
            ],
            fee_lamports: 5000,
            size: 123,
        };

        let primary = primary_instruction(&parsed).expect("primary instruction exists");
        assert!(has_swap_fields(primary));
    }

    /// Regression: a Jupiter swap that *failed* to parse to spend/receive
    /// fields (e.g. an unknown Jupiter variant) used to be passed over in
    /// favor of a sibling AssocToken `Create Token Account` instruction.
    /// The hero would say "Create Token Account" instead of "SWAP" — wrong
    /// summary for a user reviewing their swap. The priority logic now
    /// falls through "is this a known DEX program?" before considering
    /// AssocToken / generic non-Unknown ixs.
    #[test]
    fn test_primary_picks_jupiter_over_create_token_account() {
        let parsed = ParsedTransaction {
            version: TransactionVersion::Legacy,
            fee_payer: "payer".to_string(),

            signers: vec![[0u8; 32]],
            instructions: vec![
                // Side-effect: ATA creation for the dest token.
                ParsedInstruction {
                    program: "AssocToken".to_string(),
                    items: vec![ReviewItem::Header("Create Token Account".to_string())],
                },
                // The actual swap intent — but its variant didn't parse
                // to spend/receive (simulating an unknown Jupiter variant).
                ParsedInstruction {
                    program: "Jupiter".to_string(),
                    items: vec![ReviewItem::Header("Jupiter".to_string())],
                },
            ],
            fee_lamports: 5000,
            size: 200,
        };

        let primary = primary_instruction(&parsed).expect("primary instruction exists");
        assert_eq!(primary.program, "Jupiter");
    }

    /// Same prioritisation across all DEX programs we recognise — ensures
    /// the rule isn't Jupiter-specific.
    #[test]
    fn test_primary_picks_dex_over_create_token_account() {
        for dex in [
            "Jupiter",
            "Raydium AMM",
            "Raydium CLMM",
            "Raydium CPMM",
        ] {
            let parsed = ParsedTransaction {
                version: TransactionVersion::Legacy,
                fee_payer: "payer".to_string(),
    
                signers: vec![[0u8; 32]],
                instructions: vec![
                    ParsedInstruction {
                        program: "AssocToken".to_string(),
                        items: vec![ReviewItem::Header("Create Token Account".to_string())],
                    },
                    ParsedInstruction {
                        program: dex.to_string(),
                        items: vec![ReviewItem::Header(dex.to_string())],
                    },
                ],
                fee_lamports: 5000,
                size: 200,
            };
            let primary = primary_instruction(&parsed)
                .unwrap_or_else(|| panic!("no primary chosen for {}", dex));
            assert_eq!(primary.program, dex, "expected {} to win over AssocToken", dex);
        }
    }

    /// Sanity check the priority hasn't accidentally been stretched too far:
    /// when the *only* candidate is AssocToken (no DEX program present),
    /// we still surface it rather than returning None.
    #[test]
    fn test_primary_falls_through_to_assoc_token_when_no_dex() {
        let parsed = ParsedTransaction {
            version: TransactionVersion::Legacy,
            fee_payer: "payer".to_string(),

            signers: vec![[0u8; 32]],
            instructions: vec![ParsedInstruction {
                program: "AssocToken".to_string(),
                items: vec![ReviewItem::Header("Create Token Account".to_string())],
            }],
            fee_lamports: 5000,
            size: 200,
        };
        let primary = primary_instruction(&parsed).expect("primary instruction exists");
        assert_eq!(primary.program, "AssocToken");
    }

    #[test]
    fn test_build_review_lines_includes_message_hash_line() {
        let tx = system_transfer_tx([0x01; 32], 1_000_000_000);
        let (lines, _, _) = build_review_lines(&tx, &[0x01; 32]);
        // Hash moved to the end of details + relabelled so it's not
        // confused with the decoded message content. The signing-hash
        // label sits one line above the raw hex on its own line.
        assert!(lines
            .iter()
            .any(|line| line.starts_with("Signing hash (SHA-256):")));
    }

    // --- Real transaction tests (from testdata/test_txs_bin/*.bin) ---

    #[test]
    fn test_real_transactions_parse_without_panic() {
        let dir = std::path::Path::new("testdata/test_txs_bin");
        if !dir.exists() {
            return;
        }

        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "bin")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        if entries.is_empty() {
            return;
        }

        let mut passed = 0;
        for entry in &entries {
            let path = entry.path();
            let name = path.file_stem().unwrap().to_string_lossy();
            let bytes = std::fs::read(&path).unwrap();

            let parsed = parse(&bytes);

            // Must not fail to deserialize the transaction structure
            assert_ne!(
                parsed.instructions[0].program,
                "Error",
                "{}: failed to deserialize transaction: {:?}",
                name,
                parsed.instructions[0].items.iter().find_map(|i| match i {
                    ReviewItem::Warning(w) => Some(w.as_str()),
                    _ => None,
                })
            );

            assert!(
                !parsed.instructions.is_empty(),
                "{}: no instructions parsed",
                name,
            );

            // Print decoded output for visual inspection
            let lines = to_lines(&parsed);
            let programs: Vec<_> = parsed
                .instructions
                .iter()
                .map(|ix| ix.program.as_str())
                .collect();
            eprintln!(
                "  {} — {} ix {:?}",
                name,
                parsed.instructions.len(),
                programs
            );
            for line in &lines {
                eprintln!("    {}", line);
            }
            eprintln!();

            // Optional expectations sidecar. If `<name>.expected` exists
            // next to `<name>.bin`, parse it as a simple `key=value` file
            // and assert the listed predicates. Unknown keys are ignored
            // so future expectations can be added without breaking
            // existing fixtures.
            //
            // Supported keys (more can be added — see `apply_expected`):
            //   primary_program=Jupiter        # primary_instruction().program
            //   hero_title=SWAP                # @H1 row in build_review_lines
            //   hero_contains=USDC             # any hero row contains substring
            //   not_primary_program=AssocToken # primary must NOT be this
            let expected_path = path.with_extension("expected");
            if expected_path.exists() {
                let raw = std::fs::read_to_string(&expected_path).unwrap();
                apply_expected(&name, &bytes, &parsed, &raw);
            }

            passed += 1;
        }

        eprintln!("{} real transactions parsed successfully", passed);
    }

    #[test]
    fn extract_zoned_send_resolves_full_pubkeys_and_lamports() {
        // Build a 3-account legacy tx: [from_signer, to, system_program].
        // The existing `system_transfer_tx` helper omits the recipient
        // (single-account ix), which is fine for parser-shape tests but
        // not for the SEND-zoned extractor that pulls TO from index 1
        // of the instruction's account list.
        let from = [0x42u8; 32];
        let to = [0x99u8; 32];
        let mut tx = Vec::new();
        tx.push(1u8);
        tx.extend_from_slice(&[0u8; 64]);
        tx.push(1); // num_required_signers
        tx.push(0);
        tx.push(1);
        tx.push(3); // 3 accounts
        tx.extend_from_slice(&from);
        tx.extend_from_slice(&to);
        tx.extend_from_slice(&[0u8; 32]); // system program
        tx.extend_from_slice(&[0xABu8; 32]); // blockhash
        tx.push(1); // 1 instruction
        tx.push(2); // program_id_index = 2 (system)
        tx.push(2); // 2 account indices
        tx.push(0); // from
        tx.push(1); // to
        tx.push(12); // data len
        tx.extend_from_slice(&[2u8, 0, 0, 0]); // System.Transfer
        tx.extend_from_slice(&8_300_000u64.to_le_bytes()); // 0.0083 SOL

        let parsed = parse(&tx);
        let zoned = extract_zoned(&tx, &parsed).expect("send extracts zoned action");
        match zoned {
            ZonedAction::Send {
                from: f,
                to: t,
                amount_lamports,
                ..
            } => {
                assert_eq!(f, from);
                assert_eq!(t, to);
                assert_eq!(amount_lamports, 8_300_000);
            }
            _ => panic!("expected Send variant"),
        }
    }

    #[test]
    fn extract_zoned_swap_resolves_jupiter_shared_accounts_route() {
        // Real Jupiter v6 swap captured via /swap/v1: 10 USDC → SOL via
        // SolFi V2, shared_accounts_route variant. With the trailing-bytes
        // amount parser and the UNRESOLVED-mint filter this must resolve
        // both sides cleanly (no `?`, no `raw units`), so the device
        // shows the zoned SEND/RECEIVE layout for the most common
        // Jupiter shape rather than falling back to legacy.
        let dir = std::path::Path::new("testdata/test_txs_bin");
        let path = dir.join("jupiter_usdc_to_sol.bin");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let parsed = parse(&bytes);
        let zoned = extract_zoned(&bytes, &parsed)
            .expect("shared_accounts_route swap must produce a Swap zoned action");
        match zoned {
            ZonedAction::Swap {
                sell_amount,
                sell_symbol,
                buy_amount,
                buy_symbol,
                ..
            } => {
                assert_eq!(sell_amount, "10");
                assert_eq!(sell_symbol, "USDC");
                assert!(
                    buy_symbol == "SOL",
                    "buy symbol expected SOL, got {:?}",
                    buy_symbol
                );
                // Receive amount must start with 0. (small SOL fraction).
                assert!(
                    buy_amount.starts_with("0."),
                    "buy amount expected SOL fraction, got {:?}",
                    buy_amount
                );
            }
            _ => panic!("expected Swap variant"),
        }
    }

    #[test]
    fn extract_zoned_swap_resolves_jup_ag_route_v2() {
        // Captured directly from the user's jup.ag SOL→USDC swap that
        // the previous firmware mis-identified as "DFlow unknown action
        // 2f3e9bac". It's actually a Jupiter `route_v2` (one of Ultra's
        // Jupiter-program flavours) and the trailing-bytes parser must
        // resolve both sides cleanly so the device shows the zoned
        // layout instead of legacy fallback.
        let dir = std::path::Path::new("testdata/test_txs_bin");
        let path = dir.join("jup_ag_dflow_swap.bin");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let parsed = parse(&bytes);
        let zoned = extract_zoned(&bytes, &parsed)
            .expect("jup.ag route_v2 swap must produce a Swap zoned action");
        match zoned {
            ZonedAction::Swap {
                sell_amount,
                sell_symbol,
                buy_symbol,
                ..
            } => {
                assert_eq!(sell_amount, "0.01");
                assert_eq!(sell_symbol, "SOL");
                assert_eq!(buy_symbol, "USDC");
            }
            _ => panic!("expected Swap variant"),
        }
    }

    #[test]
    fn extract_zoned_swap_resolves_jupiter_shared_accounts_route_v2() {
        // Real jup.ag Market tx (USDC → SOL, 1.0 USDC). Disc d19853937cfed8e9
        // = `shared_accounts_route_v2`, layout has an `id: u8` and a 1-byte
        // separator that the V1 v2 path didn't account for, producing
        // garbage 0.256 SOL + 81.92% slippage on screen.
        let dir = std::path::Path::new("testdata/test_txs_bin");
        let path = dir.join("jupiter_shared_accounts_route_v2.bin");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let parsed = parse(&bytes);
        let zoned = extract_zoned(&bytes, &parsed)
            .expect("shared_accounts_route_v2 must produce a Swap zoned action");
        match zoned {
            ZonedAction::Swap {
                sell_amount,
                sell_symbol,
                buy_amount,
                buy_symbol,
                ..
            } => {
                assert_eq!(sell_amount, "1");
                assert_eq!(sell_symbol, "USDC");
                assert!(
                    !buy_amount.is_empty() && buy_symbol == "SOL",
                    "expected SOL receive, got {} {}",
                    buy_amount,
                    buy_symbol
                );
            }
            _ => panic!("expected Swap variant"),
        }
    }

    #[test]
    fn extract_zoned_swap_resolves_jupiter_ultra_iris() {
        // Jupiter Ultra `iris` router (proVF4pMXVa…). Layout:
        // disc(8) | opaque(8) | in_amount(u64) | min_out_amount(u64) | …
        for fixture in ["jupiter_ultra_iris_swap.bin", "jup_ag_dflow_swap2.bin"] {
            let dir = std::path::Path::new("testdata/test_txs_bin");
            let path = dir.join(fixture);
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let parsed = parse(&bytes);
            let zoned = extract_zoned(&bytes, &parsed)
                .unwrap_or_else(|| panic!("{}: must produce a Swap zoned action", fixture));
            match zoned {
                ZonedAction::Swap {
                    sell_amount,
                    sell_symbol,
                    buy_amount,
                    buy_symbol,
                    dex_name,
                    ..
                } => {
                    assert_eq!(dex_name, "Jupiter Ultra", "{}: dex_name", fixture);
                    assert_eq!(sell_symbol, "SOL", "{}: sell_symbol", fixture);
                    assert_eq!(buy_symbol, "USDC", "{}: buy_symbol", fixture);
                    assert!(
                        !sell_amount.is_empty() && sell_amount != "?",
                        "{}: sell_amount must be denominated, got {}",
                        fixture,
                        sell_amount
                    );
                    assert!(
                        !buy_amount.is_empty() && buy_amount != "?",
                        "{}: buy_amount must be denominated, got {}",
                        fixture,
                        buy_amount
                    );
                }
                _ => panic!("{}: expected Swap variant", fixture),
            }
        }
    }

    #[test]
    fn extract_zoned_swap_resolves_dflow_program_via_prepare_ix() {
        // Real DFlow program (DF1ow…) txs captured from jup.ag. The swap
        // is split across two top-level ixs: a `prepare` (disc 2f3e9bac…,
        // 16 bytes = `[disc | u64 in_amount]`, source ATA in the account
        // list) and the main `swap` whose route-plan layout we don't
        // decode. The prepare gives us a verified spend amount + source
        // mint; receive side stays empty (renders "—" in the zoned cell).
        let dir = std::path::Path::new("testdata/test_txs_bin");
        // Single-hop fixtures resolve dest mint deterministically → emit a
        // Swap zoned action. The long-route fixture can't disambiguate dest
        // (multi-hop touches multiple user ATAs) → must fall to legacy
        // review by returning None from extract_zoned. We never display "?"
        // next to a Sign button on the zoned screen.
        let cases: &[(&str, Option<(&str, &str)>)] = &[
            ("dflow_program_swap.bin", Some(("0.01", "USDC"))),
            ("dflow_program_swap2.bin", Some(("0.01", "USDC"))),
            ("dflow_program_swap_long_route.bin", None),
        ];
        for (fixture, expected) in cases {
            let path = dir.join(fixture);
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let parsed = parse(&bytes);
            let zoned = extract_zoned(&bytes, &parsed);
            match expected {
                Some((expected_sell, expected_buy_symbol)) => {
                    let zoned = zoned.unwrap_or_else(|| {
                        panic!("{}: must produce a Swap zoned action", fixture)
                    });
                    match zoned {
                        ZonedAction::Swap {
                            sell_amount,
                            sell_symbol,
                            buy_amount,
                            buy_symbol,
                            dex_name,
                            ..
                        } => {
                            assert_eq!(dex_name, "DFlow", "{}: dex_name", fixture);
                            assert_eq!(sell_amount, *expected_sell, "{}: sell_amount", fixture);
                            assert_eq!(sell_symbol, "SOL", "{}: sell_symbol", fixture);
                            assert!(!buy_amount.is_empty(), "{}: buy_amount", fixture);
                            assert_eq!(buy_symbol, *expected_buy_symbol, "{}: buy_symbol", fixture);
                        }
                        _ => panic!("{}: expected Swap variant", fixture),
                    }
                }
                None => assert!(
                    zoned.is_none(),
                    "{}: must refuse zoned (dest unresolvable)",
                    fixture
                ),
            }
        }
    }

    #[test]
    fn extract_zoned_swap_falls_back_when_mint_unresolved() {
        // jupiter_swap_1.bin is a route_v2 swap whose source/dest mints
        // aren't included as explicit accounts — the parser can't resolve
        // them offline, so it emits "X raw units". extract_zoned must
        // refuse to build a Swap action in that case (showing a big
        // confident integer with unknown unit next to a Sign button is
        // dangerous), so the renderer falls back to the legacy paginated
        // review where inner Token transfers carry self-evident amounts.
        let dir = std::path::Path::new("testdata/test_txs_bin");
        let path = dir.join("jupiter_swap_1.bin");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let parsed = parse(&bytes);
        assert!(
            extract_zoned(&bytes, &parsed).is_none(),
            "swap with unresolved mints must not produce a Swap zoned action"
        );
    }

    #[test]
    fn split_amount_symbol_owned_collapses_raw_units_to_question_mark() {
        // The parser emits "X raw units" when the mint isn't resolvable
        // from the wallet's ATA map. The literal phrase overflows the
        // right-aligned symbol slot on the device (profont22 puts it at
        // ~108 px wide and the label takes the other half of the cell).
        // The split helper collapses it to "?" so the layout holds.
        let (a, s) = split_amount_symbol_owned("195000000 raw units");
        assert_eq!(a, "195000000");
        assert_eq!(s, "?");
    }

    #[test]
    fn split_amount_symbol_owned_handles_normal_token() {
        let (a, s) = split_amount_symbol_owned("0.1187 SOL");
        assert_eq!(a, "0.1187");
        assert_eq!(s, "SOL");
    }

    #[test]
    fn compact_amount_preserves_typical_swap_values() {
        assert_eq!(compact_amount("0.01"), "0.01");
        assert_eq!(compact_amount("0.1187"), "0.1187");
        assert_eq!(compact_amount("10"), "10");
        // Caps at 6 fractional digits, strips trailing zeros.
        assert_eq!(compact_amount("1.123456789"), "1.123456");
        assert_eq!(compact_amount("0.000005000"), "0.000005");
    }

    /// Apply `<name>.expected` sidecar predicates to a captured tx. Each
    /// non-empty, non-comment line is `key=value`; unknown keys are
    /// skipped (so adding a new key in the future doesn't break existing
    /// fixtures, and typos surface as silent no-ops which is acceptable
    /// for an opt-in development tool).
    fn apply_expected(name: &str, tx_bytes: &[u8], parsed: &ParsedTransaction, raw: &str) {
        let primary = primary_instruction(parsed);
        let (lines, _, _) = build_review_lines(tx_bytes, &[0u8; 32]);
        let hero_title = lines
            .iter()
            .find_map(|l| l.strip_prefix("@H1 ").map(str::to_string));

        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = match line.split_once('=') {
                Some(kv) => (kv.0.trim(), kv.1.trim()),
                None => continue,
            };

            match key {
                "primary_program" => {
                    let actual = primary.map(|ix| ix.program.as_str()).unwrap_or("<none>");
                    assert_eq!(
                        actual, value,
                        "{}: expected primary_program={}, got {}",
                        name, value, actual
                    );
                }
                "not_primary_program" => {
                    let actual = primary.map(|ix| ix.program.as_str()).unwrap_or("<none>");
                    assert_ne!(
                        actual, value,
                        "{}: primary_program should NOT be {}",
                        name, value
                    );
                }
                "hero_title" => {
                    let actual = hero_title.as_deref().unwrap_or("<none>");
                    assert_eq!(
                        actual, value,
                        "{}: expected hero_title={}, got {}",
                        name, value, actual
                    );
                }
                "hero_contains" => {
                    // Substring across all hero rows (@H1 / @H2 / @HM /
                    // @SWAPPAIR). Useful for asserting "the SOL side
                    // resolved" without committing to exact formatting.
                    let hero_text: String = lines
                        .iter()
                        .filter(|l| {
                            l.starts_with("@H1 ")
                                || l.starts_with("@H2 ")
                                || l.starts_with("@HM ")
                                || l.starts_with("@SWAPPAIR ")
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                    assert!(
                        hero_text.contains(value),
                        "{}: expected hero to contain {:?}, got:\n{}",
                        name,
                        value,
                        hero_text
                    );
                }
                _ => {
                    // Unknown key — log and skip so new expectation
                    // formats can be experimented with without breaking
                    // the suite.
                    eprintln!("  [{}.expected] skipping unknown key: {}", name, key);
                }
            }
        }
    }
}
