//! Transaction parser — converts raw Solana tx bytes into human-readable review items.
//!
//! Entry point: `parse(tx_bytes)` → `ParsedTransaction`
//!
//! To add support for a new program:
//!   1. Create `src/parser/<program>.rs` with `pub fn parse(data, accounts) -> ParsedInstruction`
//!   2. Add the program ID to `programs::identify()`
//!   3. Add a match arm in the `dispatch()` function below

mod message;
mod programs;
mod system;
mod token;
mod stake;
mod unknown;

// Shared modules — reusable across dApp parsers
pub(crate) mod anchor;
pub(crate) mod token_registry;

// dApp parsers
mod jupiter;

// === Public types ===

pub struct ParsedTransaction {
    pub version: TransactionVersion,
    pub fee_payer: String,
    pub num_signers: u8,
    pub instructions: Vec<ParsedInstruction>,
}

pub enum TransactionVersion {
    Legacy,
    /// Contains the number of address lookup tables. These cannot be resolved
    /// air-gapped (no RPC), so accounts from lookup tables show as unresolved.
    V0 { address_table_lookups: usize },
}

pub struct ParsedInstruction {
    pub program: String,
    pub items: Vec<ReviewItem>,
}

pub enum ReviewItem {
    Header(String),
    Field { label: String, value: String },
    Warning(String),
    Separator,
}

// === Entry point ===

pub fn parse(tx_bytes: &[u8]) -> ParsedTransaction {
    let msg = match message::deserialize(tx_bytes) {
        Ok(m) => m,
        Err(e) => return ParsedTransaction {
            version: TransactionVersion::Legacy,
            fee_payer: "?".into(),
            num_signers: 0,
            instructions: vec![ParsedInstruction {
                program: "Error".into(),
                items: vec![ReviewItem::Warning(format!("Failed to parse transaction: {}", e))],
            }],
        },
    };

    let version = match msg.version {
        message::MessageVersion::Legacy => TransactionVersion::Legacy,
        message::MessageVersion::V0 => TransactionVersion::V0 {
            address_table_lookups: msg.address_table_lookups.len(),
        },
    };

    let fee_payer = msg.accounts.first()
        .map(|k| bs58::encode(k).into_string())
        .unwrap_or_else(|| "?".into());

    // Build ATA map for offline token resolution (only when Jupiter is present)
    let needs_ata = msg.accounts.iter().any(|acct| {
        programs::identify(acct).as_ref().map(|p| p.name) == Some("Jupiter")
    });
    let ata_map = if needs_ata {
        let n = (msg.num_required_signers as usize).min(msg.accounts.len());
        token_registry::build_ata_map(&msg.accounts[..n])
    } else {
        token_registry::AtaMap::new()
    };

    let instructions = msg.instructions.iter()
        .map(|ix| dispatch(ix, &msg.accounts, &ata_map))
        .collect();

    ParsedTransaction {
        version,
        fee_payer,
        num_signers: msg.num_required_signers,
        instructions,
    }
}

/// Convert a `ParsedTransaction` into a flat list of strings for display.
/// Keeps `SignReview` screen unchanged while the renderer is simple text-only.
pub fn to_lines(tx: &ParsedTransaction) -> Vec<String> {
    let mut lines = Vec::new();

    let version_str = match &tx.version {
        TransactionVersion::Legacy => "Legacy".into(),
        TransactionVersion::V0 { address_table_lookups: 0 } => "v0".into(),
        TransactionVersion::V0 { address_table_lookups: n } => format!("v0 ({} lookup tables)", n),
    };

    let payer_short = if tx.fee_payer.len() >= 8 {
        format!("{}..{}", &tx.fee_payer[..4], &tx.fee_payer[tx.fee_payer.len() - 4..])
    } else {
        tx.fee_payer.clone()
    };

    lines.push(format!("Tx: {}  Signer: {}", version_str, payer_short));
    lines.push(format!("Instructions: {}", tx.instructions.len()));
    lines.push(String::new());

    let multi = tx.instructions.len() > 1;
    for (i, ix) in tx.instructions.iter().enumerate() {
        if multi {
            lines.push(format!("-- {}/{}: {} --", i + 1, tx.instructions.len(), ix.program));
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
                ReviewItem::Separator => lines.push(String::new()),
            }
        }
        if multi && i + 1 < tx.instructions.len() {
            lines.push(String::new());
        }
    }

    lines
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

    let resolved_accounts: Vec<[u8; 32]> = ix.account_indices.iter()
        .filter_map(|&idx| all_accounts.get(idx as usize).copied())
        .collect();

    match programs::identify(program_id).as_ref().map(|p| p.name) {
        Some("System")       => system::parse(&ix.data, &resolved_accounts),
        Some("Token")        => token::parse("Token", &ix.data, &resolved_accounts),
        Some("Token-2022")   => token::parse("Token-2022", &ix.data, &resolved_accounts),
        Some("Stake")        => stake::parse(&ix.data, &resolved_accounts),
        Some("AssocToken")   => parse_assoc_token(program_id, &ix.data, &resolved_accounts),
        Some("Memo")         => parse_memo(&ix.data),
        Some("ComputeBudget") => parse_compute_budget(&ix.data),
        Some("Jupiter")      => jupiter::parse(&ix.data, &ix.account_indices, all_accounts, ata_map),
        Some(name)           => ParsedInstruction {
            program: name.into(),
            items: vec![ReviewItem::Header(name.into())],
        },
        None => unknown::parse(program_id, &ix.data, &resolved_accounts),
    }
}

fn parse_assoc_token(_program_id: &[u8; 32], _data: &[u8], accounts: &[[u8; 32]]) -> ParsedInstruction {
    let wallet = accounts.first().map(pubkey_short).unwrap_or_else(|| "?".into());
    let mint = accounts.get(2).map(pubkey_short).unwrap_or_else(|| "?".into());
    ParsedInstruction {
        program: "AssocToken".into(),
        items: vec![
            ReviewItem::Header("Create Token Account".into()),
            ReviewItem::Field { label: "Wallet".into(), value: wallet },
            ReviewItem::Field { label: "Mint".into(), value: mint },
        ],
    }
}

fn parse_memo(data: &[u8]) -> ParsedInstruction {
    let text = std::str::from_utf8(data)
        .unwrap_or("(invalid UTF-8)")
        .chars().take(64).collect::<String>();
    ParsedInstruction {
        program: "Memo".into(),
        items: vec![
            ReviewItem::Header("Memo".into()),
            ReviewItem::Field { label: "Text".into(), value: text },
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
            ReviewItem::Field { label: "Setting".into(), value: detail },
        ],
    }
}

fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
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
        tx.push(1); tx.push(0); tx.push(1);
        // 2 accounts: signer + system program (all zeros)
        tx.push(2);
        tx.extend_from_slice(&from);
        tx.extend_from_slice(&[0u8; 32]);
        // blockhash
        tx.extend_from_slice(&[0xABu8; 32]);
        // 1 instruction
        tx.push(1);
        tx.push(1);  // program_id_index = 1 (system program)
        tx.push(1); tx.push(0); // 1 account: index 0
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
        tx.push(1); tx.push(0); tx.push(1);
        tx.push(2);
        tx.extend_from_slice(&from);
        tx.extend_from_slice(&[0u8; 32]);
        tx.extend_from_slice(&[0xABu8; 32]);
        tx.push(1);
        tx.push(1); tx.push(1); tx.push(0);
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
        let has_amount = parsed.instructions[0].items.iter().any(|i| matches!(
            i, ReviewItem::Field { label, value } if label == "Amount" && value == "2 SOL"
        ));
        assert!(has_amount);
    }

    #[test]
    fn test_parse_v0_transaction() {
        let tx = v0_system_transfer_tx([0x01; 32], 1_000_000_000);
        let parsed = parse(&tx);
        assert!(matches!(parsed.version, TransactionVersion::V0 { address_table_lookups: 0 }));
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
        let has_warning = parsed.instructions[0].items.iter().any(|i| matches!(i, ReviewItem::Warning(_)));
        assert!(has_warning);
    }

    #[test]
    fn test_parse_empty_bytes_returns_error_instruction() {
        let parsed = parse(&[]);
        let has_warning = parsed.instructions[0].items.iter().any(|i| matches!(i, ReviewItem::Warning(_)));
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
        tx.push(1); tx.push(0); tx.push(1);
        tx.push(2);
        tx.extend_from_slice(&[0x01u8; 32]);
        tx.extend_from_slice(&[0x00u8; 32]);
        tx.extend_from_slice(&[0xABu8; 32]);
        tx.push(2); // 2 instructions (compact-u16)
        for _ in 0..2 {
            tx.push(1); tx.push(1); tx.push(0);
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
}
