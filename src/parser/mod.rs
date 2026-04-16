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

    let instructions = msg.instructions.iter()
        .map(|ix| dispatch(ix, &msg.accounts))
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

fn dispatch(ix: &message::RawInstruction, all_accounts: &[[u8; 32]]) -> ParsedInstruction {
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
