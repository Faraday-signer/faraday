//! Pre-sign transaction classification for review UX.
//!
//! This is intentionally conservative: if signal quality is weak (for example,
//! unknown programs are present), the classifier falls back to `other` so users
//! rely on the full decoded transaction details.

use crate::parser::bytes::{read_u32_le, read_u64_le};
use crate::parser::{lookup_tables, message, programs, token_registry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegDirection {
    Inflow,
    Outflow,
    Internal,
    External,
}

#[derive(Debug, Clone)]
struct TransferLeg {
    amount: u64,
    decimals: Option<u8>,
    asset_hint: &'static str,
    direction: LegDirection,
}

#[derive(Debug, Clone)]
pub struct Classification {
    pub category: &'static str,
    pub confidence: f32,
    pub summary: String,
    pub high_risk: bool,
}

impl Classification {
    pub fn headline(&self) -> String {
        let label = match self.category {
            "security_authority_change" => "Security: authority change",
            "defi_swap" => "Likely: DeFi swap",
            "stake_deposit" => "Likely: Stake deposit",
            "stake_withdraw" => "Likely: Stake withdraw",
            "fee_only" => "Likely: Fee-only transaction",
            "token_transfer" => "Likely: Token transfer",
            "token_transfer_batch" => "Likely: Token transfer batch",
            _ => "Likely: Other / unclassified",
        };
        format!("{} ({:.0}%)", label, self.confidence * 100.0)
    }
}

#[derive(Debug, Default)]
struct Features {
    has_unknown_program: bool,
    only_infra_programs: bool,
    has_swap_program: bool,
    has_stake_delegate: bool,
    has_stake_withdraw: bool,
    has_authority_change: bool,
    legs: Vec<TransferLeg>,
}

pub fn classify(tx_bytes: &[u8], wallet_pubkey: &[u8; 32]) -> Option<Classification> {
    let msg = message::deserialize(tx_bytes).ok()?;
    let all_accounts = lookup_tables::expand_accounts(&msg.accounts, &msg.address_table_lookups);
    let focal = bs58::encode(wallet_pubkey).into_string();

    let mut features = Features {
        only_infra_programs: true,
        ..Features::default()
    };

    for ix in &msg.instructions {
        let Some(program_id) = all_accounts.get(ix.program_id_index) else {
            features.has_unknown_program = true;
            features.only_infra_programs = false;
            continue;
        };

        let known_name = programs::identify(program_id).map(|p| p.name);
        let program_name = known_name.unwrap_or("Unknown");

        if known_name.is_none() {
            features.has_unknown_program = true;
        }

        if !is_infra_program(program_name) {
            features.only_infra_programs = false;
        }

        if is_swap_program(program_name) {
            features.has_swap_program = true;
        }

        if is_authority_change(program_name, &ix.data) {
            features.has_authority_change = true;
        }

        if program_name == "Stake" {
            match stake_discriminant(&ix.data) {
                Some(2) => features.has_stake_delegate = true,
                Some(4) => features.has_stake_withdraw = true,
                _ => {}
            }
        }

        if let Some(leg) = extract_transfer_leg(ix, &all_accounts, &focal, program_name) {
            features.legs.push(leg);
        }
    }

    Some(classify_features(&features))
}

/// A single fund-moving leg the signing wallet authorizes to leave its
/// control — SOL (System.Transfer from the wallet) or an SPL token transfer
/// whose authority is the wallet. Carries a grouping `key` (same key ⇒ same
/// asset, so amounts are summable) plus the info needed to format a total.
pub struct OutflowLeg {
    /// Index into the transaction's instruction list (aligns 1:1 with the
    /// parsed instructions), so the hero can match its primary leg to a group.
    pub ix_index: usize,
    /// Asset identity: `"sol"` for native SOL, else the source token account
    /// (one token account holds exactly one mint, so it is a safe mint proxy
    /// even for plain `Transfer` where the mint isn't in the instruction).
    pub key: String,
    pub amount: u64,
    pub decimals: Option<u8>,
    pub symbol: Option<&'static str>,
}

/// Collect the outflow legs the signing wallet authorizes. Used by the review
/// hero to detect multi-transfer batches and never under-represent the largest
/// outflow behind a small primary leg. Swaps are excluded so their spend legs
/// don't read as a raw transfer batch, mirroring the `token_transfer_batch`
/// guard in `classify_features`.
pub fn wallet_outflow_legs(tx_bytes: &[u8], wallet_pubkey: &[u8; 32]) -> Vec<OutflowLeg> {
    let Ok(msg) = message::deserialize(tx_bytes) else {
        return Vec::new();
    };
    let all_accounts = lookup_tables::expand_accounts(&msg.accounts, &msg.address_table_lookups);
    let focal = bs58::encode(wallet_pubkey).into_string();

    let mut legs = Vec::new();
    for (i, ix) in msg.instructions.iter().enumerate() {
        let Some(program_id) = all_accounts.get(ix.program_id_index) else {
            continue;
        };
        let program_name = programs::identify(program_id).map(|p| p.name).unwrap_or("Unknown");
        if is_swap_program(program_name) {
            return Vec::new();
        }
        if let Some(leg) = extract_outflow_leg(i, ix, &all_accounts, &focal, program_name) {
            legs.push(leg);
        }
    }
    legs
}

/// Extract an outflow leg only when the signing wallet is the party moving the
/// funds: the source for a System transfer, or the transfer authority for an
/// SPL token transfer (the account that must sign).
fn extract_outflow_leg(
    ix_index: usize,
    ix: &message::RawInstruction,
    all_accounts: &[[u8; 32]],
    focal: &str,
    program_name: &str,
) -> Option<OutflowLeg> {
    match program_name {
        "System" => {
            let disc = read_u32_le(&ix.data, 0).ok()?;
            if disc != 2 {
                return None;
            }
            let source = resolve_ix_account(ix, 0, all_accounts)?;
            if source != focal {
                return None;
            }
            let amount = read_u64_le(&ix.data, 4).ok()?;
            Some(OutflowLeg {
                ix_index,
                key: "sol".into(),
                amount,
                decimals: Some(9),
                symbol: Some("SOL"),
            })
        }
        "Token" | "Token-2022" => {
            // Transfer accounts: [source, dest, authority]; TransferChecked:
            // [source, mint, dest, authority]. The authority must be the
            // wallet for this to be the wallet's outflow.
            let (amount, decimals, mint_pos, authority_pos) = match *ix.data.first()? {
                3 => (read_u64_le(&ix.data, 1).ok()?, None, None, 2usize),
                12 => (read_u64_le(&ix.data, 1).ok()?, Some(*ix.data.get(9)?), Some(1usize), 3usize),
                _ => return None,
            };
            if resolve_ix_account(ix, authority_pos, all_accounts)? != focal {
                return None;
            }
            // Prefer the registry's trusted decimals/symbol for a known mint;
            // the on-wire decimals byte is attacker-controlled.
            let (decimals, symbol) = match mint_pos
                .and_then(|p| ix_account_bytes(ix, p, all_accounts))
                .and_then(token_registry::lookup)
            {
                Some(info) => (Some(info.decimals), Some(info.symbol)),
                None => (decimals, None),
            };
            Some(OutflowLeg {
                ix_index,
                key: resolve_ix_account(ix, 0, all_accounts)?,
                amount,
                decimals,
                symbol,
            })
        }
        _ => None,
    }
}

fn ix_account_bytes<'a>(
    ix: &message::RawInstruction,
    pos: usize,
    all_accounts: &'a [[u8; 32]],
) -> Option<&'a [u8; 32]> {
    let idx = *ix.account_indices.get(pos)? as usize;
    all_accounts.get(idx)
}

fn classify_features(features: &Features) -> Classification {
    if features.has_authority_change {
        return Classification {
            category: "security_authority_change",
            confidence: 0.99,
            summary: "Authority update detected. Verify all account targets.".into(),
            high_risk: true,
        };
    }

    if features.has_unknown_program {
        return Classification {
            category: "other",
            confidence: 0.50,
            summary: "Unknown program present. Review all decoded instructions carefully.".into(),
            high_risk: false,
        };
    }

    let sol_out = sum_legs(&features.legs, |leg| {
        leg.direction == LegDirection::Outflow && leg.asset_hint == "sol"
    });
    let sol_in = sum_legs(&features.legs, |leg| {
        leg.direction == LegDirection::Inflow && leg.asset_hint == "sol"
    });

    if features.has_stake_delegate && sol_out > 0 {
        return Classification {
            category: "stake_deposit",
            confidence: 0.89,
            summary: "Stake delegation with outbound SOL detected.".into(),
            high_risk: false,
        };
    }

    if features.has_stake_withdraw && sol_in > 0 {
        return Classification {
            category: "stake_withdraw",
            confidence: 0.89,
            summary: "Stake withdrawal with inbound SOL detected.".into(),
            high_risk: false,
        };
    }

    if features.has_swap_program {
        let has_inflow = features
            .legs
            .iter()
            .any(|leg| leg.direction == LegDirection::Inflow);
        let has_outflow = features
            .legs
            .iter()
            .any(|leg| leg.direction == LegDirection::Outflow);
        if has_inflow && has_outflow {
            return Classification {
                category: "defi_swap",
                confidence: 0.90,
                summary: "DEX route with both spend and receive legs detected.".into(),
                high_risk: false,
            };
        }
    }

    if features.legs.is_empty() && features.only_infra_programs {
        return Classification {
            category: "fee_only",
            confidence: 0.90,
            summary: "No transfer legs detected; transaction appears infrastructure-only.".into(),
            high_risk: false,
        };
    }

    let focal_legs = features
        .legs
        .iter()
        .filter(|leg| leg.direction != LegDirection::External)
        .count();

    if focal_legs > 0 && !features.has_swap_program {
        let has_nft_like = features
            .legs
            .iter()
            .any(|leg| leg.decimals == Some(0) && leg.amount == 1);
        if focal_legs == 1 && !has_nft_like {
            return Classification {
                category: "token_transfer",
                confidence: 0.90,
                summary: "Single transfer leg for the signing wallet detected.".into(),
                high_risk: false,
            };
        }
        if !has_nft_like {
            return Classification {
                category: "token_transfer_batch",
                confidence: 0.74,
                summary: "Multiple transfer legs for the signing wallet detected.".into(),
                high_risk: false,
            };
        }
    }

    Classification {
        category: "other",
        confidence: 0.50,
        summary: "No high-confidence pre-sign classification matched.".into(),
        high_risk: false,
    }
}

fn is_infra_program(program_name: &str) -> bool {
    matches!(program_name, "ComputeBudget" | "Memo")
}

fn is_swap_program(program_name: &str) -> bool {
    super::is_swap_program_name(program_name)
}

fn is_authority_change(program_name: &str, data: &[u8]) -> bool {
    match program_name {
        // SPL-Token / Token-2022 SetAuthority (u8 discriminant 6).
        "Token" | "Token-2022" => matches!(data.first(), Some(6)),
        // Stake Authorize (1) / AuthorizeWithSeed (7) and their checked
        // variants AuthorizeChecked (9) / AuthorizeCheckedWithSeed (10)
        // reassign staker or withdraw authority; no transfer leg, so flag
        // them explicitly. The checked co-sign requirement is no barrier to
        // an attacker whose new authority is their own key.
        "Stake" => matches!(stake_discriminant(data), Some(1) | Some(7) | Some(9) | Some(10)),
        // BPF Upgradeable Loader Upgrade (3) swaps program code; SetAuthority
        // (4) / SetAuthorityChecked (6) swap the upgrade authority (u32-LE).
        "BPF Upgradeable Loader" => matches!(read_u32_le(data, 0), Ok(3) | Ok(4) | Ok(6)),
        _ => false,
    }
}

fn stake_discriminant(data: &[u8]) -> Option<u32> {
    if data.is_empty() {
        return None;
    }
    if data.len() >= 4 {
        let disc = read_u32_le(data, 0).ok()?;
        if disc <= 13 {
            return Some(disc);
        }
    }
    Some(data[0] as u32)
}

fn extract_transfer_leg(
    ix: &message::RawInstruction,
    all_accounts: &[[u8; 32]],
    focal: &str,
    program_name: &str,
) -> Option<TransferLeg> {
    match program_name {
        "System" => {
            let disc = read_u32_le(&ix.data, 0).ok()?;
            if disc != 2 {
                return None;
            }
            let amount = read_u64_le(&ix.data, 4).ok()?;
            let source = resolve_ix_account(ix, 0, all_accounts)?;
            let destination = resolve_ix_account(ix, 1, all_accounts)?;
            Some(TransferLeg {
                amount,
                decimals: Some(9),
                asset_hint: "sol",
                direction: leg_direction(&source, &destination, focal),
            })
        }
        "Token" | "Token-2022" => {
            let disc = *ix.data.first()?;
            match disc {
                3 => {
                    let amount = read_u64_le(&ix.data, 1).ok()?;
                    let source = resolve_ix_account(ix, 0, all_accounts)?;
                    let destination = resolve_ix_account(ix, 1, all_accounts)?;
                    Some(TransferLeg {
                        amount,
                        decimals: None,
                        asset_hint: "spl_token",
                        direction: leg_direction(&source, &destination, focal),
                    })
                }
                12 => {
                    let amount = read_u64_le(&ix.data, 1).ok()?;
                    let decimals = *ix.data.get(9)?;
                    let source = resolve_ix_account(ix, 0, all_accounts)?;
                    let destination = resolve_ix_account(ix, 2, all_accounts)?;
                    Some(TransferLeg {
                        amount,
                        decimals: Some(decimals),
                        asset_hint: "spl_token",
                        direction: leg_direction(&source, &destination, focal),
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn resolve_ix_account(
    ix: &message::RawInstruction,
    ix_account_position: usize,
    all_accounts: &[[u8; 32]],
) -> Option<String> {
    let account_index = *ix.account_indices.get(ix_account_position)? as usize;
    let account = all_accounts.get(account_index)?;
    // Full-length form (leg strings are compared against a full-base58 focal),
    // but never the raw sentinel — that would leak as a plausible address.
    Some(crate::parser::bytes::render_account(account))
}

fn leg_direction(source: &str, destination: &str, focal: &str) -> LegDirection {
    if source == focal && destination == focal {
        return LegDirection::Internal;
    }
    if source == focal {
        return LegDirection::Outflow;
    }
    if destination == focal {
        return LegDirection::Inflow;
    }
    if source == destination {
        return LegDirection::Internal;
    }
    LegDirection::External
}

fn sum_legs<F>(legs: &[TransferLeg], mut predicate: F) -> u128
where
    F: FnMut(&TransferLeg) -> bool,
{
    legs.iter()
        .filter(|leg| predicate(leg))
        .map(|leg| leg.amount as u128)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leg(direction: LegDirection, amount: u64, asset_hint: &'static str) -> TransferLeg {
        TransferLeg {
            amount,
            decimals: Some(9),
            asset_hint,
            direction,
        }
    }

    #[test]
    fn authority_change_takes_priority() {
        let features = Features {
            has_authority_change: true,
            has_swap_program: true,
            legs: vec![
                leg(LegDirection::Outflow, 10, "spl_token"),
                leg(LegDirection::Inflow, 20, "spl_token"),
            ],
            ..Features::default()
        };
        let out = classify_features(&features);
        assert_eq!(out.category, "security_authority_change");
        assert!(out.high_risk);
    }

    #[test]
    fn unknown_program_forces_other() {
        let features = Features {
            has_unknown_program: true,
            legs: vec![leg(LegDirection::Outflow, 10, "spl_token")],
            ..Features::default()
        };
        let out = classify_features(&features);
        assert_eq!(out.category, "other");
    }

    fn pubkey_from_b58(s: &str) -> [u8; 32] {
        let bytes = bs58::decode(s).into_vec().unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        key
    }

    /// Minimal legacy tx: 1 signer + 1 program account, single instruction
    /// with the given program id and data bytes.
    fn legacy_program_tx(program_id: &[u8; 32], data: &[u8]) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1u8); // num_signatures
        tx.extend_from_slice(&[0u8; 64]); // signature placeholder

        tx.push(1); // num_required_signers
        tx.push(0); // num_readonly_signed
        tx.push(1); // num_readonly_unsigned

        tx.push(2); // num_accounts
        tx.extend_from_slice(&[0x01u8; 32]); // signer
        tx.extend_from_slice(program_id); // program

        tx.extend_from_slice(&[0xABu8; 32]); // recent blockhash

        tx.push(1); // num_instructions
        tx.push(1); // program_id_index = 1
        tx.push(1); // 1 account index
        tx.push(0); // account_indices[0] = 0
        tx.push(data.len() as u8); // data_len (compact-u16, <0x80)
        tx.extend_from_slice(data);
        tx
    }

    #[test]
    fn stake_authorize_is_high_risk() {
        let stake = pubkey_from_b58("Stake11111111111111111111111111111111111111");
        // Authorize discriminant = 1 (u32-LE).
        let tx = legacy_program_tx(&stake, &[1, 0, 0, 0]);
        let out = classify(&tx, &[0x01u8; 32]).unwrap();
        assert_eq!(out.category, "security_authority_change");
        assert!(out.high_risk);
    }

    #[test]
    fn stake_authorize_with_seed_is_high_risk() {
        let stake = pubkey_from_b58("Stake11111111111111111111111111111111111111");
        // AuthorizeWithSeed discriminant = 7 (u32-LE).
        let tx = legacy_program_tx(&stake, &[7, 0, 0, 0]);
        let out = classify(&tx, &[0x01u8; 32]).unwrap();
        assert_eq!(out.category, "security_authority_change");
        assert!(out.high_risk);
    }

    #[test]
    fn stake_authorize_checked_is_high_risk() {
        let stake = pubkey_from_b58("Stake11111111111111111111111111111111111111");
        // AuthorizeChecked discriminant = 9 (u32-LE); the co-sign requirement
        // is no barrier when the attacker's own key is the new authority.
        let tx = legacy_program_tx(&stake, &[9, 0, 0, 0]);
        let out = classify(&tx, &[0x01u8; 32]).unwrap();
        assert_eq!(out.category, "security_authority_change");
        assert!(out.high_risk);
    }

    #[test]
    fn stake_delegate_is_not_high_risk() {
        let stake = pubkey_from_b58("Stake11111111111111111111111111111111111111");
        // DelegateStake discriminant = 2 (u32-LE) must not trip the guard.
        let tx = legacy_program_tx(&stake, &[2, 0, 0, 0]);
        let out = classify(&tx, &[0x01u8; 32]).unwrap();
        assert_ne!(out.category, "security_authority_change");
        assert!(!out.high_risk);
    }

    #[test]
    fn loader_set_authority_is_high_risk() {
        let loader = pubkey_from_b58("BPFLoaderUpgradeab1e11111111111111111111111");
        // SetAuthority discriminant = 4 (u32-LE).
        let tx = legacy_program_tx(&loader, &[4, 0, 0, 0]);
        let out = classify(&tx, &[0x01u8; 32]).unwrap();
        assert_eq!(out.category, "security_authority_change");
        assert!(out.high_risk);
    }

    #[test]
    fn loader_upgrade_is_high_risk() {
        let loader = pubkey_from_b58("BPFLoaderUpgradeab1e11111111111111111111111");
        // Upgrade discriminant = 3 (u32-LE): a program code swap.
        let tx = legacy_program_tx(&loader, &[3, 0, 0, 0]);
        let out = classify(&tx, &[0x01u8; 32]).unwrap();
        assert_eq!(out.category, "security_authority_change");
        assert!(out.high_risk);
    }

    #[test]
    fn bidirectional_swap_is_classified() {
        let features = Features {
            has_swap_program: true,
            legs: vec![
                leg(LegDirection::Outflow, 100, "spl_token"),
                leg(LegDirection::Inflow, 90, "spl_token"),
            ],
            ..Features::default()
        };
        let out = classify_features(&features);
        assert_eq!(out.category, "defi_swap");
    }
}
