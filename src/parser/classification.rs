//! Pre-sign transaction classification for review UX.
//!
//! This is intentionally conservative: if signal quality is weak (for example,
//! unknown programs are present), the classifier falls back to `other` so users
//! rely on the full decoded transaction details.

use crate::parser::bytes::{read_u32_le, read_u64_le};
use crate::parser::{lookup_tables, message, programs};

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
    matches!(
        program_name,
        "Jupiter" | "Raydium AMM" | "Raydium CLMM" | "Raydium CPMM"
    )
}

fn is_authority_change(program_name: &str, data: &[u8]) -> bool {
    if matches!(program_name, "Token" | "Token-2022") {
        if matches!(data.first(), Some(6)) {
            return true;
        }
    }

    false
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
    Some(bs58::encode(account).into_string())
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
