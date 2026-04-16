//! Solana legacy-transaction parser.
//!
//! Decodes a legacy-format tx enough to:
//! - List required signers (for Sign-gating).
//! - Identify a primary instruction kind (SOL / SPL transfer, stake, approve, memo,
//!   or generic "other"), extracting human-relevant fields.
//! - Compute the fee (base + ComputeBudget priority) so the Review screen can
//!   show what the user is about to pay.
//!
//! Never panics on malformed input — unrecognized or truncated txs fall through
//! to `TxKind::Other` and the UI shows a conservative generic view.

// Well-known Solana program IDs (base58).
const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const STAKE_PROGRAM: &str = "Stake11111111111111111111111111111111111111";
const MEMO_PROGRAM: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const MEMO_PROGRAM_V1: &str = "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo";
const COMPUTE_BUDGET: &str = "ComputeBudget111111111111111111111111111111";

/// Lamports per signature (base fee on Solana).
pub const LAMPORTS_PER_SIGNATURE: u64 = 5_000;

/// Mainnet mints we ship with symbol labels. Air-gapped — can't query a registry,
/// so only a small curated list. Unknown mints show as raw addresses.
fn known_token_symbol(mint: &str) -> Option<&'static str> {
    match mint {
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => Some("USDC"),
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => Some("USDT"),
        "So11111111111111111111111111111111111111112" => Some("WSOL"),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum TxKind {
    SolTransfer {
        from: String,
        to: String,
        lamports: u64,
    },
    SplTransfer {
        source: String,
        dest: String,
        amount_raw: u64,
        mint: Option<String>,
        decimals: Option<u8>,
        symbol: Option<&'static str>,
    },
    SplApprove {
        source: String,
        delegate: String,
        amount_raw: u64,
        mint: Option<String>,
        decimals: Option<u8>,
        symbol: Option<&'static str>,
    },
    StakeDelegate {
        stake_account: String,
        vote_account: String,
    },
    StakeDeactivate {
        stake_account: String,
    },
    StakeWithdraw {
        stake_account: String,
        to: String,
        lamports: u64,
    },
    Memo {
        text: String,
    },
    Other {
        programs: Vec<String>, // unique program ids excluding ComputeBudget
    },
}

#[derive(Debug, Clone)]
pub struct TxSummary {
    pub signers: Vec<[u8; 32]>,
    pub kind: TxKind,
    pub fee_lamports: u64,
    pub size: usize,
}

// Back-compat API kept for callers that only want transfer extraction.
#[derive(Debug, Clone)]
pub struct TransferInfo {
    pub from: String,
    pub to: String,
    pub lamports: u64,
}

/// Extract the required-signer pubkeys (`keys[0..num_required_signatures]`).
pub fn signers(tx: &[u8]) -> Option<Vec<[u8; 32]>> {
    let msg = parse_message(tx)?;
    Some(msg.required_signers().to_vec())
}

/// Parse the first System::Transfer instruction. Returns None for any other shape.
pub fn parse_transfer(tx: &[u8]) -> Option<TransferInfo> {
    let msg = parse_message(tx)?;
    for ix in msg.instructions() {
        if msg.program_id_b58(ix.program_idx) == SYSTEM_PROGRAM
            && ix.data.len() >= 12
            && ix.data[..4] == [2, 0, 0, 0]
            && ix.accounts.len() >= 2
        {
            let from = msg.key_b58(ix.accounts[0] as usize)?;
            let to = msg.key_b58(ix.accounts[1] as usize)?;
            let lamports_bytes: [u8; 8] = ix.data[4..12].try_into().ok()?;
            return Some(TransferInfo {
                from,
                to,
                lamports: u64::from_le_bytes(lamports_bytes),
            });
        }
    }
    None
}

/// Rich summary of a legacy transaction.
pub fn summarize(tx: &[u8]) -> Option<TxSummary> {
    let msg = parse_message(tx)?;
    let signers = msg.required_signers().to_vec();

    // Fee = base (sigs × 5000) + priority (ComputeBudget::SetComputeUnitPrice × Limit).
    let base_fee = (signers.len() as u64).saturating_mul(LAMPORTS_PER_SIGNATURE);
    let (cu_limit, cu_price_micro) = extract_compute_budget(&msg);
    // price_micro is micro-lamports per CU: priority_lamports = price_micro × limit / 1_000_000
    let priority = ((cu_price_micro as u128) * (cu_limit as u128) / 1_000_000u128) as u64;
    let fee_lamports = base_fee.saturating_add(priority);

    let kind = classify(&msg).unwrap_or_else(|| TxKind::Other {
        programs: unique_programs(&msg),
    });

    Some(TxSummary {
        signers,
        kind,
        fee_lamports,
        size: tx.len(),
    })
}

fn classify(msg: &Message<'_>) -> Option<TxKind> {
    // Walk instructions; skip ComputeBudget; use the first meaningful one.
    for ix in msg.instructions() {
        let pid = msg.program_id_b58(ix.program_idx);
        if pid == COMPUTE_BUDGET {
            continue;
        }

        // System::Transfer (instr 2, u32 LE)
        if pid == SYSTEM_PROGRAM
            && ix.data.len() >= 12
            && ix.data[..4] == [2, 0, 0, 0]
            && ix.accounts.len() >= 2
        {
            let from = msg.key_b58(ix.accounts[0] as usize)?;
            let to = msg.key_b58(ix.accounts[1] as usize)?;
            let lamports = u64::from_le_bytes(ix.data[4..12].try_into().ok()?);
            return Some(TxKind::SolTransfer { from, to, lamports });
        }

        // SPL Token / Token-2022
        if (pid == TOKEN_PROGRAM || pid == TOKEN_2022) && !ix.data.is_empty() {
            match ix.data[0] {
                // Legacy Transfer (3): [3, amount u64 LE]. Accounts: source, dest, owner
                3 if ix.data.len() >= 9 && ix.accounts.len() >= 3 => {
                    let source = msg.key_b58(ix.accounts[0] as usize)?;
                    let dest = msg.key_b58(ix.accounts[1] as usize)?;
                    let amount_raw = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
                    return Some(TxKind::SplTransfer {
                        source,
                        dest,
                        amount_raw,
                        mint: None,
                        decimals: None,
                        symbol: None,
                    });
                }
                // TransferChecked (12): [12, amount u64 LE, decimals u8]. Accounts: source, mint, dest, owner
                12 if ix.data.len() >= 10 && ix.accounts.len() >= 4 => {
                    let source = msg.key_b58(ix.accounts[0] as usize)?;
                    let mint = msg.key_b58(ix.accounts[1] as usize)?;
                    let dest = msg.key_b58(ix.accounts[2] as usize)?;
                    let amount_raw = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
                    let decimals = ix.data[9];
                    let symbol = known_token_symbol(&mint);
                    return Some(TxKind::SplTransfer {
                        source,
                        dest,
                        amount_raw,
                        mint: Some(mint),
                        decimals: Some(decimals),
                        symbol,
                    });
                }
                // Approve (4): [4, amount u64 LE]. Accounts: source, delegate, owner
                4 if ix.data.len() >= 9 && ix.accounts.len() >= 3 => {
                    let source = msg.key_b58(ix.accounts[0] as usize)?;
                    let delegate = msg.key_b58(ix.accounts[1] as usize)?;
                    let amount_raw = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
                    return Some(TxKind::SplApprove {
                        source,
                        delegate,
                        amount_raw,
                        mint: None,
                        decimals: None,
                        symbol: None,
                    });
                }
                // ApproveChecked (13): [13, amount u64 LE, decimals u8]. Accounts: source, mint, delegate, owner
                13 if ix.data.len() >= 10 && ix.accounts.len() >= 4 => {
                    let source = msg.key_b58(ix.accounts[0] as usize)?;
                    let mint = msg.key_b58(ix.accounts[1] as usize)?;
                    let delegate = msg.key_b58(ix.accounts[2] as usize)?;
                    let amount_raw = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
                    let decimals = ix.data[9];
                    let symbol = known_token_symbol(&mint);
                    return Some(TxKind::SplApprove {
                        source,
                        delegate,
                        amount_raw,
                        mint: Some(mint),
                        decimals: Some(decimals),
                        symbol,
                    });
                }
                _ => {}
            }
        }

        // Stake Program
        if pid == STAKE_PROGRAM && ix.data.len() >= 4 {
            // Instruction discriminator is u32 LE.
            let disc = u32::from_le_bytes(ix.data[..4].try_into().ok()?);
            match disc {
                // DelegateStake: accounts = [stake, vote, clock sysvar, stake history sysvar, stake config, stake authority]
                2 if ix.accounts.len() >= 2 => {
                    let stake_account = msg.key_b58(ix.accounts[0] as usize)?;
                    let vote_account = msg.key_b58(ix.accounts[1] as usize)?;
                    return Some(TxKind::StakeDelegate {
                        stake_account,
                        vote_account,
                    });
                }
                // Withdraw: [4, lamports u64 LE]. Accounts: stake, recipient, clock sysvar, stake history sysvar, withdraw authority
                4 if ix.data.len() >= 12 && ix.accounts.len() >= 2 => {
                    let stake_account = msg.key_b58(ix.accounts[0] as usize)?;
                    let to = msg.key_b58(ix.accounts[1] as usize)?;
                    let lamports = u64::from_le_bytes(ix.data[4..12].try_into().ok()?);
                    return Some(TxKind::StakeWithdraw {
                        stake_account,
                        to,
                        lamports,
                    });
                }
                // Deactivate: just accounts = [stake, clock sysvar, stake authority]
                5 if !ix.accounts.is_empty() => {
                    let stake_account = msg.key_b58(ix.accounts[0] as usize)?;
                    return Some(TxKind::StakeDeactivate { stake_account });
                }
                _ => {}
            }
        }

        // Memo: data is UTF-8 text.
        if pid == MEMO_PROGRAM || pid == MEMO_PROGRAM_V1 {
            let text = std::str::from_utf8(ix.data).unwrap_or("").to_string();
            return Some(TxKind::Memo { text });
        }

        // First non-ComputeBudget instruction that we didn't recognize — bail to Other.
        break;
    }
    None
}

fn extract_compute_budget(msg: &Message<'_>) -> (u32, u64) {
    let mut limit: u32 = 200_000; // Solana default when not set.
    let mut price_micro: u64 = 0;
    for ix in msg.instructions() {
        if msg.program_id_b58(ix.program_idx) != COMPUTE_BUDGET || ix.data.is_empty() {
            continue;
        }
        match ix.data[0] {
            // SetComputeUnitLimit: [2, limit u32 LE]
            2 if ix.data.len() >= 5 => {
                limit = u32::from_le_bytes(ix.data[1..5].try_into().unwrap_or([0; 4]));
            }
            // SetComputeUnitPrice: [3, price u64 LE micro-lamports per CU]
            3 if ix.data.len() >= 9 => {
                price_micro = u64::from_le_bytes(ix.data[1..9].try_into().unwrap_or([0; 8]));
            }
            _ => {}
        }
    }
    (limit, price_micro)
}

fn unique_programs(msg: &Message<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for ix in msg.instructions() {
        let pid = msg.program_id_b58(ix.program_idx);
        if pid == COMPUTE_BUDGET {
            continue;
        }
        if !out.iter().any(|p| p == &pid) {
            out.push(pid);
        }
    }
    out
}

/// Format lamports as a SOL amount with trailing-zero trimming.
pub fn format_sol(lamports: u64) -> String {
    format_with_decimals(lamports, 9)
}

/// Format a raw token amount using the given decimals. Trims trailing zeros.
/// Falls back to raw units when decimals is unknown.
pub fn format_token_amount(raw: u64, decimals: Option<u8>) -> String {
    match decimals {
        Some(d) if d <= 18 => format_with_decimals(raw, d as u32),
        _ => format!("{} (raw)", raw),
    }
}

fn format_with_decimals(raw: u64, decimals: u32) -> String {
    if decimals == 0 {
        return format!("{}", raw);
    }
    let divisor = 10u128.pow(decimals);
    let whole = (raw as u128) / divisor;
    let frac = (raw as u128) % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let s = format!("{}.{:0width$}", whole, frac, width = decimals as usize);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// --- Message parsing primitives ---

struct Instruction<'a> {
    program_idx: usize,
    accounts: &'a [u8],
    data: &'a [u8],
}

struct Message<'a> {
    tx: &'a [u8],
    num_required_signatures: u8,
    key_offsets: Vec<usize>, // byte offsets into tx[] for each account key (32 bytes)
    instructions: Vec<Instruction<'a>>,
}

impl<'a> Message<'a> {
    fn required_signers(&self) -> Vec<[u8; 32]> {
        let mut out = Vec::with_capacity(self.num_required_signatures as usize);
        for i in 0..self.num_required_signatures as usize {
            if let Some(off) = self.key_offsets.get(i) {
                let mut k = [0u8; 32];
                k.copy_from_slice(&self.tx[*off..*off + 32]);
                out.push(k);
            }
        }
        out
    }

    fn key_b58(&self, i: usize) -> Option<String> {
        let off = *self.key_offsets.get(i)?;
        Some(bs58::encode(&self.tx[off..off + 32]).into_string())
    }

    fn program_id_b58(&self, i: usize) -> String {
        self.key_b58(i).unwrap_or_default()
    }

    fn instructions(&self) -> &[Instruction<'a>] {
        &self.instructions
    }
}

fn parse_message<'a>(tx: &'a [u8]) -> Option<Message<'a>> {
    let mut i = 0;

    let (n_sigs, used) = compact_u16(&tx[i..])?;
    i = i.checked_add(used)?;
    i = i.checked_add(64usize.checked_mul(n_sigs as usize)?)?;

    if tx.len() < i + 3 {
        return None;
    }
    let num_required_signatures = tx[i];
    i += 3;

    let (n_keys, used) = compact_u16(&tx[i..])?;
    i += used;
    let mut key_offsets = Vec::with_capacity(n_keys as usize);
    for _ in 0..n_keys {
        if tx.len() < i + 32 {
            return None;
        }
        key_offsets.push(i);
        i += 32;
    }

    if tx.len() < i + 32 {
        return None;
    }
    i += 32; // recent blockhash

    let (n_ix, used) = compact_u16(&tx[i..])?;
    i += used;
    let mut instructions = Vec::with_capacity(n_ix as usize);
    for _ in 0..n_ix {
        if tx.len() < i + 1 {
            return None;
        }
        let program_idx = tx[i] as usize;
        i += 1;
        let (n_acc, used) = compact_u16(&tx[i..])?;
        i += used;
        if tx.len() < i + n_acc as usize {
            return None;
        }
        let accounts = &tx[i..i + n_acc as usize];
        i += n_acc as usize;
        let (d_len, used) = compact_u16(&tx[i..])?;
        i += used;
        if tx.len() < i + d_len as usize {
            return None;
        }
        let data = &tx[i..i + d_len as usize];
        i += d_len as usize;
        instructions.push(Instruction {
            program_idx,
            accounts,
            data,
        });
    }

    Some(Message {
        tx,
        num_required_signatures,
        key_offsets,
        instructions,
    })
}

fn compact_u16(buf: &[u8]) -> Option<(u16, usize)> {
    let mut val: u32 = 0;
    let mut shift = 0u32;
    for (i, &b) in buf.iter().enumerate().take(3) {
        val |= ((b & 0x7F) as u32) << shift;
        if b & 0x80 == 0 {
            if val > u16::MAX as u32 {
                return None;
            }
            return Some((val as u16, i + 1));
        }
        shift += 7;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as B64, Engine};

    #[test]
    fn parses_0_01_sol_transfer() {
        let tx_b64 = "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDywkoNI1j+nah055+LRl/5r74IARS0MSvHfPPW5usTeAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACbZKN5QkVXQQH+5BYJje2PQK9UFAivDK+ncn3rilJV8BAgIAAQwCAAAAgJaYAAAAAAA=";
        let tx = B64.decode(tx_b64).unwrap();
        let info = parse_transfer(&tx).expect("parse");
        assert_eq!(info.lamports, 10_000_000);
        assert_eq!(info.from, "EfZrx3EoqE158pP1Ep4ntEH4Ru7ZC5TNQZ7HxxsYLoP5");
        assert_eq!(info.to, "11111111111111111111111111111112");
    }

    #[test]
    fn summary_identifies_sol_transfer_and_fee() {
        let tx_b64 = "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDywkoNI1j+nah055+LRl/5r74IARS0MSvHfPPW5usTeAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACbZKN5QkVXQQH+5BYJje2PQK9UFAivDK+ncn3rilJV8BAgIAAQwCAAAAgJaYAAAAAAA=";
        let tx = B64.decode(tx_b64).unwrap();
        let s = summarize(&tx).expect("summary");
        assert_eq!(s.signers.len(), 1);
        assert_eq!(s.fee_lamports, 5_000); // 1 sig, no ComputeBudget
        match s.kind {
            TxKind::SolTransfer { lamports, .. } => assert_eq!(lamports, 10_000_000),
            _ => panic!("expected SolTransfer"),
        }
    }

    #[test]
    fn formats_amounts() {
        assert_eq!(format_sol(10_000_000), "0.01");
        assert_eq!(format_sol(5_000), "0.000005");
        assert_eq!(format_token_amount(1_500_000, Some(6)), "1.5");
        assert_eq!(format_token_amount(1_500_000, None), "1500000 (raw)");
    }
}
