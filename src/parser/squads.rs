//! Squads v4 multisig program parser.
//!
//! Surfaces human-readable review fields for Squads instructions so the
//! signer can verify what's being created/approved on-device instead of
//! seeing "Unrecognized program" + raw hex.
//!
//! Currently parses:
//!   * `multisig_create_v2` — new co-signed account with N approvers
//!     and a threshold (e.g. 2-of-3).
//!
//! Other lifecycle instructions (`vault_transaction_create`,
//! `proposal_create`, `proposal_approve`, `vault_transaction_execute`)
//! are recognised by name only — extending the parser per-variant is
//! the natural follow-up as the dashboard grows beyond create-only.

use crate::parser::anchor;
use crate::parser::bytes::{read_disc8, read_u32_le};
use crate::parser::{ParsedInstruction, ReviewItem};

const PROGRAM_NAME: &str = "Squads";

pub fn parse(data: &[u8], _accounts: &[[u8; 32]]) -> ParsedInstruction {
    let disc = match read_disc8(data, 0) {
        Ok(d) => d,
        Err(_) => return short_warn("Instruction data too short"),
    };

    if disc == anchor::discriminator("multisig_create_v2") {
        return parse_multisig_create_v2(data);
    }

    // Recognised-but-not-yet-decoded variants get a friendly name instead
    // of "Unknown" so reviewers know roughly what's being asked.
    let name = recognised_variant_name(&disc);
    let header = name.unwrap_or("Unknown instruction").to_string();
    let mut items = vec![ReviewItem::Header(format!("Squads · {header}"))];
    if name.is_none() {
        items.push(ReviewItem::Warning(
            "Faraday doesn't know this Squads instruction yet — review raw bytes carefully"
                .into(),
        ));
    }
    ParsedInstruction { program: PROGRAM_NAME.into(), items }
}

fn parse_multisig_create_v2(data: &[u8]) -> ParsedInstruction {
    let mut items = vec![ReviewItem::Header("Squads · Create multisig".into())];
    let mut p = 8usize; // skip discriminator

    // configAuthority: Option<Pubkey>
    let cfg_authority = match read_option_pubkey(data, &mut p) {
        Ok(v) => v,
        Err(e) => return push_warn(items, e),
    };

    // threshold: u16
    let threshold = match data.get(p..p + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]),
        None => return push_warn(items, "truncated: threshold"),
    };
    p += 2;

    // members: Vec<Member { key: Pubkey, permissions: u8 }>
    let n_members = match read_u32_le(data, p) {
        Ok(n) => n as usize,
        Err(e) => return push_warn(items, e),
    };
    p += 4;

    items.push(ReviewItem::Field {
        label: "Threshold".into(),
        value: format!("{} of {}", threshold, n_members),
    });

    for i in 0..n_members {
        let key = match read_pubkey(data, &mut p) {
            Ok(k) => k,
            Err(e) => return push_warn(items, e),
        };
        let perms = match data.get(p) {
            Some(&pp) => pp,
            None => return push_warn(items, "truncated: permissions"),
        };
        p += 1;
        items.push(ReviewItem::Field {
            label: format!("Approver {}", i + 1),
            value: format!("{} ({})", pubkey_short(&key), format_permissions(perms)),
        });
    }

    // timeLock: u32 (seconds before an approved proposal can be executed)
    let time_lock = match read_u32_le(data, p) {
        Ok(t) => t,
        Err(e) => return push_warn(items, e),
    };
    p += 4;
    if time_lock > 0 {
        items.push(ReviewItem::Field {
            label: "Time lock".into(),
            value: format!("{} sec", time_lock),
        });
    }

    // rentCollector: Option<Pubkey> (where reclaimed rent goes when a
    // transaction PDA is closed; harmless for plain multisig usage)
    match read_option_pubkey(data, &mut p) {
        Ok(Some(rc)) => items.push(ReviewItem::Field {
            label: "Rent collector".into(),
            value: pubkey_short(&rc),
        }),
        Ok(None) => {}
        Err(e) => return push_warn(items, e),
    }

    // memo: Option<String> — surfaced as "Label" since users typically use
    // it as a human-readable account name (e.g. "Operations · Payroll").
    match read_option_string(data, &mut p) {
        Ok(Some(s)) => items.push(ReviewItem::Field { label: "Label".into(), value: s }),
        Ok(None) => {}
        Err(e) => return push_warn(items, e),
    }

    // Surface the autonomy property at the end where it stands out — a
    // multisig with a configAuthority can have its members/threshold
    // changed unilaterally by that authority, which is rarely what HR
    // wants for payroll.
    if let Some(ca) = cfg_authority {
        items.push(ReviewItem::Warning(format!(
            "Config authority set ({}) — not autonomous",
            pubkey_short(&ca),
        )));
    }

    ParsedInstruction { program: PROGRAM_NAME.into(), items }
}

// ── Other Squads instructions: name only ─────────────────────────────────────

fn recognised_variant_name(disc: &[u8; 8]) -> Option<&'static str> {
    const NAMES: &[&str] = &[
        "multisig_create",            // legacy v1
        "config_transaction_create",
        "config_transaction_execute",
        "vault_transaction_create",
        "vault_transaction_execute",
        "proposal_create",
        "proposal_approve",
        "proposal_reject",
        "proposal_cancel",
        "spending_limit_create",
        "spending_limit_remove",
        "spending_limit_use",
    ];
    NAMES.iter().find(|n| anchor::discriminator(n) == *disc).copied()
}

// ── Borsh-ish readers ────────────────────────────────────────────────────────

fn read_pubkey(data: &[u8], p: &mut usize) -> Result<[u8; 32], &'static str> {
    let s = data.get(*p..*p + 32).ok_or("truncated: pubkey")?;
    let mut k = [0u8; 32];
    k.copy_from_slice(s);
    *p += 32;
    Ok(k)
}

fn read_option_pubkey(
    data: &[u8],
    p: &mut usize,
) -> Result<Option<[u8; 32]>, &'static str> {
    let tag = *data.get(*p).ok_or("truncated: option tag")?;
    *p += 1;
    match tag {
        0 => Ok(None),
        1 => read_pubkey(data, p).map(Some),
        _ => Err("invalid option tag"),
    }
}

fn read_option_string(
    data: &[u8],
    p: &mut usize,
) -> Result<Option<String>, &'static str> {
    let tag = *data.get(*p).ok_or("truncated: option tag")?;
    *p += 1;
    match tag {
        0 => Ok(None),
        1 => {
            let len = read_u32_le(data, *p)? as usize;
            *p += 4;
            let bytes = data.get(*p..*p + len).ok_or("truncated: string body")?;
            *p += len;
            // Cap displayed text — labels longer than ~48 chars are suspect
            // (and won't fit on the device anyway).
            let text: String = std::str::from_utf8(bytes)
                .unwrap_or("(invalid UTF-8)")
                .chars()
                .take(48)
                .collect();
            Ok(Some(text))
        }
        _ => Err("invalid option tag"),
    }
}

// ── Formatting helpers ───────────────────────────────────────────────────────

/// Squads encodes member permissions as a bitmask:
///   bit 0 = Initiate, bit 1 = Vote, bit 2 = Execute.
fn format_permissions(p: u8) -> String {
    if p == 0 {
        return "no perms".into();
    }
    if p == 0x07 {
        return "init+vote+exec".into();
    }
    let mut parts: Vec<&str> = Vec::new();
    if p & 0x01 != 0 { parts.push("init"); }
    if p & 0x02 != 0 { parts.push("vote"); }
    if p & 0x04 != 0 { parts.push("exec"); }
    if parts.is_empty() {
        format!("0x{:02x}", p)
    } else {
        parts.join("+")
    }
}

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
}

fn short_warn(msg: &str) -> ParsedInstruction {
    ParsedInstruction {
        program: PROGRAM_NAME.into(),
        items: vec![
            ReviewItem::Header(PROGRAM_NAME.into()),
            ReviewItem::Warning(msg.into()),
        ],
    }
}

fn push_warn(mut items: Vec<ReviewItem>, msg: &str) -> ParsedInstruction {
    items.push(ReviewItem::Warning(msg.into()));
    ParsedInstruction { program: PROGRAM_NAME.into(), items }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pk(s: &str) -> [u8; 32] {
        let bytes = bs58::decode(s).into_vec().unwrap();
        let mut k = [0u8; 32];
        k.copy_from_slice(&bytes);
        k
    }

    fn build_create_v2(
        threshold: u16,
        members: &[([u8; 32], u8)],
        memo: Option<&str>,
    ) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&anchor::discriminator("multisig_create_v2"));
        d.push(0); // configAuthority None
        d.extend_from_slice(&threshold.to_le_bytes());
        d.extend_from_slice(&(members.len() as u32).to_le_bytes());
        for (key, perms) in members {
            d.extend_from_slice(key);
            d.push(*perms);
        }
        d.extend_from_slice(&0u32.to_le_bytes()); // timeLock
        d.push(0); // rentCollector None
        match memo {
            Some(s) => {
                d.push(1);
                d.extend_from_slice(&(s.len() as u32).to_le_bytes());
                d.extend_from_slice(s.as_bytes());
            }
            None => d.push(0),
        }
        d
    }

    fn field_value<'a>(items: &'a [ReviewItem], label: &str) -> Option<&'a str> {
        items.iter().find_map(|i| match i {
            ReviewItem::Field { label: l, value: v } if l == label => Some(v.as_str()),
            _ => None,
        })
    }

    #[test]
    fn known_program_id() {
        use crate::parser::programs::identify;
        let id = pk("SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf");
        assert_eq!(identify(&id).unwrap().name, "Squads");
    }

    #[test]
    fn create_v2_full_decode() {
        let a = pk("GAthe6Gh8xEuJobQWB3cLUBFjsGtyvsk7Y3BeQMkMsfT");
        let b = pk("HbK77RskcRmdRquYz7PeJcXZTGdobykG2fxfmgPR5cgm");
        let c = pk("FHgN463Nr8khShhcbhaT2PYGAn4uDCHUMHQWg44rFDv");
        let data = build_create_v2(2, &[(a, 0x07), (b, 0x07), (c, 0x07)], None);

        let ix = parse(&data, &[]);
        assert_eq!(ix.program, "Squads");
        assert_eq!(field_value(&ix.items, "Threshold"), Some("2 of 3"));
        assert_eq!(
            field_value(&ix.items, "Approver 1"),
            Some("GAth..MsfT (init+vote+exec)"),
        );
        assert_eq!(
            field_value(&ix.items, "Approver 2"),
            Some("HbK7..5cgm (init+vote+exec)"),
        );
        assert_eq!(
            field_value(&ix.items, "Approver 3"),
            Some("FHgN..rFDv (init+vote+exec)"),
        );
        // No warnings for a clean autonomous multisig.
        assert!(!ix.items.iter().any(|i| matches!(i, ReviewItem::Warning(_))));
    }

    #[test]
    fn create_v2_with_memo() {
        let a = pk("GAthe6Gh8xEuJobQWB3cLUBFjsGtyvsk7Y3BeQMkMsfT");
        let data = build_create_v2(1, &[(a, 0x07)], Some("Operations · Payroll"));
        let ix = parse(&data, &[]);
        assert_eq!(field_value(&ix.items, "Label"), Some("Operations · Payroll"));
    }

    #[test]
    fn create_v2_real_bytes_match() {
        // Reproduces the bytes the dashboard built for the user's 2-of-3
        // mainnet test (verified against the device screenshot).
        let a = pk("GAthe6Gh8xEuJobQWB3cLUBFjsGtyvsk7Y3BeQMkMsfT");
        let b = pk("HbK77RskcRmdRquYz7PeJcXZTGdobykG2fxfmgPR5cgm");
        let c = pk("FHgN463Nr8khShhcbhaT2PYGAn4uDCHUMHQWg44rFDv");
        let data = build_create_v2(2, &[(a, 0x07), (b, 0x07), (c, 0x07)], None);
        // First 16 bytes must match what the device screen showed (spot
        // check — the rest is regenerated deterministically from the SDK).
        assert_eq!(
            &data[..16],
            &[0x32, 0xdd, 0xc7, 0x5d, 0x28, 0xf5, 0x8b, 0xe9,
              0x00, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0xe1],
        );
    }

    #[test]
    fn truncated_data_warns() {
        let ix = parse(&[0u8; 4], &[]);
        assert!(ix.items.iter().any(|i| matches!(i, ReviewItem::Warning(_))));
    }

    #[test]
    fn unknown_disc_lists_program() {
        let ix = parse(&[0xFFu8; 8], &[]);
        assert!(ix.items.iter().any(|i| matches!(
            i,
            ReviewItem::Header(h) if h.starts_with("Squads")
        )));
    }

    #[test]
    fn permissions_partial_set() {
        assert_eq!(format_permissions(0x00), "no perms");
        assert_eq!(format_permissions(0x07), "init+vote+exec");
        assert_eq!(format_permissions(0x03), "init+vote");
        assert_eq!(format_permissions(0x06), "vote+exec");
        assert_eq!(format_permissions(0x04), "exec");
    }
}
