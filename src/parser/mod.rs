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
pub(crate) mod token_registry;

// dApp parsers
mod dflow;
mod jupiter;
mod raydium;

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
    let needs_ata = all_accounts.iter().any(|acct| {
        matches!(
            programs::identify(acct).as_ref().map(|p| p.name),
            Some("Jupiter" | "DFlow" | "Raydium AMM" | "Raydium CLMM" | "Raydium CPMM")
        )
    });
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
            // too long after `format_amount_short` already SI-suffixed it,
            // fall through to the vertical stack so nothing gets cropped.
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

fn is_swap_program_name(program: &str) -> bool {
    matches!(
        program,
        "Jupiter"
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
/// If the whole part alone is already over budget (shouldn't normally
/// happen because `format_amount_short` already applies an SI suffix
/// above 6 digits), returns the input unchanged so the caller can detect
/// the case and fall back to a different layout.
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
    let wallet = accounts
        .first()
        .map(pubkey_short)
        .unwrap_or_else(|| "?".into());
    // Resolve the mint twice: as full bytes for the offline registry
    // lookup (so the header reads "Setup USDC account" instead of the
    // bare "Create Token Account"), and as a shortened display string
    // for the Mint review row.
    let mint_bytes = accounts.get(2).copied();
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

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
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
