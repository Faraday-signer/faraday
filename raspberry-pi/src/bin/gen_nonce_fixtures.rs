//! Dev helper: emit durable-nonce transaction fixtures for the parser tests.
//!
//! Writes legacy + v0 transactions whose **first instruction** is
//! `SystemProgram::AdvanceNonceAccount`, followed by a SOL transfer — the exact
//! shape Faraday builds so a slow QR relay can't expire the signature. The
//! parser must label the advance-nonce instruction (never the unknown-
//! instruction warning path); these fixtures pin that behaviour at the byte
//! level.
//!
//! Every account is a canonical public example address (the `@solana/kit`
//! durable-nonce docs vectors + a BIP39-universe recipient); the nonce value
//! and signature slots are synthetic. None of these are submittable to any
//! cluster.
//!
//! Outputs land in `testdata/examples/nonce/`:
//!   - tx_advance_nonce_transfer_legacy.bin
//!   - tx_advance_nonce_transfer_v0.bin
//!
//! Run:
//!   cargo run --features simulator --bin gen-nonce-fixtures

// Canonical public example addresses. Not real account holders.
const NONCE_AUTHORITY_B58: &str = "4KD1Rdrd89NG7XbzW3xsX9Aqnx2EExJvExiNme6g9iAT";
const NONCE_ACCOUNT_B58: &str = "EGtMh4yvXswwHhwVhyPxGrVV2TkLTgUqGodbATEPvojZ";
const RECIPIENT_B58: &str = "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk";
const RECENT_BLOCKHASHES_SYSVAR_B58: &str = "SysvarRecentB1ockHashes11111111111111111111";
const SYSTEM_PROGRAM_B58: &str = "11111111111111111111111111111111";

const LAMPORTS: u64 = 1_000_000_000; // 1 SOL

fn decode32(b58: &str) -> [u8; 32] {
    bs58::decode(b58)
        .into_vec()
        .expect("valid base58")
        .try_into()
        .expect("32-byte pubkey")
}

/// Single-byte shortvec (all our counts are < 128).
fn push_shortvec(out: &mut Vec<u8>, n: usize) {
    assert!(n < 128, "shortvec fixture helper only handles n < 128");
    out.push(n as u8);
}

/// Assemble the transaction message body (everything after the signature
/// slots). `versioned` prepends the v0 (0x80) prefix byte and appends the
/// trailing address-table-lookup count.
fn build_message(versioned: bool) -> Vec<u8> {
    let authority = decode32(NONCE_AUTHORITY_B58);
    let nonce_account = decode32(NONCE_ACCOUNT_B58);
    let recipient = decode32(RECIPIENT_B58);
    let recent_blockhashes = decode32(RECENT_BLOCKHASHES_SYSVAR_B58);
    let system_program = decode32(SYSTEM_PROGRAM_B58);

    // For a durable-nonce tx the message's "recent blockhash" field carries the
    // nonce value stored in the nonce account. Synthetic but fixed here.
    let nonce_value = [0x11u8; 32];

    // Account table, ordered per the Solana message spec:
    //   writable signers → readonly signers → writable non-signers → readonly non-signers
    //   0: authority          (writable, signer  — fee payer + nonce authority)
    //   1: nonce_account      (writable, non-signer)
    //   2: recipient          (writable, non-signer)
    //   3: recent_blockhashes (readonly, non-signer)
    //   4: system_program     (readonly, non-signer)
    let accounts: [[u8; 32]; 5] = [
        authority,
        nonce_account,
        recipient,
        recent_blockhashes,
        system_program,
    ];

    let mut msg = Vec::new();
    if versioned {
        msg.push(0x80); // v0 version prefix
    }
    // Header: numRequiredSignatures, numReadonlySigned, numReadonlyUnsigned.
    msg.push(1);
    msg.push(0);
    msg.push(2); // recent_blockhashes + system_program are readonly non-signers
    // Account table.
    push_shortvec(&mut msg, accounts.len());
    for acct in &accounts {
        msg.extend_from_slice(acct);
    }
    // Recent blockhash slot = nonce value.
    msg.extend_from_slice(&nonce_value);

    // Instructions.
    push_shortvec(&mut msg, 2);

    // 1) AdvanceNonceAccount — MUST be first. program = System (index 4).
    //    accounts: [nonce_account(1), recent_blockhashes(3), authority(0)].
    //    data: u32 LE discriminant 4, no trailing bytes.
    msg.push(4); // program_id_index
    push_shortvec(&mut msg, 3);
    msg.extend_from_slice(&[1, 3, 0]);
    push_shortvec(&mut msg, 4);
    msg.extend_from_slice(&[4, 0, 0, 0]);

    // 2) Transfer — program = System (index 4).
    //    accounts: [authority(0), recipient(2)].
    //    data: u32 LE discriminant 2 + u64 LE lamports.
    msg.push(4); // program_id_index
    push_shortvec(&mut msg, 2);
    msg.extend_from_slice(&[0, 2]);
    let mut transfer_data = vec![2u8, 0, 0, 0];
    transfer_data.extend_from_slice(&LAMPORTS.to_le_bytes());
    push_shortvec(&mut msg, transfer_data.len());
    msg.extend_from_slice(&transfer_data);

    if versioned {
        push_shortvec(&mut msg, 0); // 0 address table lookups
    }

    msg
}

/// Wrap a message in the wire envelope: shortvec signature count + one empty
/// (all-zero) signature slot for the single required signer.
fn wrap_unsigned(message: Vec<u8>) -> Vec<u8> {
    let mut tx = Vec::new();
    push_shortvec(&mut tx, 1); // 1 signature
    tx.extend_from_slice(&[0u8; 64]); // empty slot
    tx.extend_from_slice(&message);
    tx
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = "testdata/examples/nonce";
    std::fs::create_dir_all(dir)?;

    let legacy = wrap_unsigned(build_message(false));
    let v0 = wrap_unsigned(build_message(true));

    let legacy_path = format!("{}/tx_advance_nonce_transfer_legacy.bin", dir);
    let v0_path = format!("{}/tx_advance_nonce_transfer_v0.bin", dir);

    std::fs::write(&legacy_path, &legacy)?;
    std::fs::write(&v0_path, &v0)?;

    println!("wrote {} ({} bytes)", legacy_path, legacy.len());
    println!("wrote {} ({} bytes)", v0_path, v0.len());
    Ok(())
}
