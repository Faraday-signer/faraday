//! Dev helper: emit one QR per code path in the `clear-msig-ika` integration.
//! Generates:
//!   - 8 sign-message QRs (off-chain approval bodies — propose/approve/cancel
//!     × lamport-transfer / SPL-transfer / meta-intents / cross-chain fallback)
//!   - 8 sign-tx QRs       (one per Quasar instruction disc 0..=7)
//!
//! Outputs land in `testdata/examples/ika/`. Each fixture has both a PNG (to
//! display on a Mac screen and scan with the iPhone-as-camera via Continuity
//! Camera) and a `.bin` (raw bytes for the parser unit tests / debugging).
//!
//! Run:
//!   cargo run --features simulator --bin gen-ika-fixtures
//!
//! Account ordering and instruction layouts match
//! `Iamknownasfesal/clear-msig-ika/cli/src/quasar_client/*.rs`. Account values
//! are synthetic (0x20, 0x21, …) so the device review shows distinct slots —
//! none of these txs are submittable to devnet.

// Pulls whole modules via `#[path]` but only uses a subset.
#![allow(dead_code)]

#[path = "../crypto/mod.rs"]
mod crypto;

#[path = "../qr/mod.rs"]
mod qr;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::{GrayImage, Luma};

const DEFAULT_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

const IKA_PROGRAM_B58: &str = "2jsLpMRZAJUJJ7weNhBJqVAgLjpngi6xTEPUbttmTUjA";

// Realistic-looking pubkeys for the message body fixtures, so the device
// shortener produces something other than all-the-same. These are not real
// account holders — they're picked from the BIP39 test vector universe.
const SAMPLE_RECIPIENT: &str = "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer = crypto::derivation::derive_keypair(DEFAULT_MNEMONIC, "", 0)
        .ok_or("key derivation failed")?;
    let signer_pubkey = signer.public_key;
    let signer_b58 = crypto::derivation::address(&signer);

    let ika_program: [u8; 32] = bs58::decode(IKA_PROGRAM_B58)
        .into_vec()?
        .try_into()
        .map_err(|_| "ika program id is not 32 bytes")?;

    let dir = "testdata/examples/ika";
    std::fs::create_dir_all(dir)?;

    let mut written = Vec::<String>::new();

    // ---- Message fixtures ----

    let messages: &[(&str, String)] = &[
        (
            "msg_approve_transfer",
            format!(
                "expires 2030-01-01 00:00:00: approve transfer 1000000000 lamports to {} | wallet: treasury proposal: 42",
                SAMPLE_RECIPIENT
            ),
        ),
        (
            "msg_propose_transfer",
            format!(
                "expires 2030-01-01 00:00:00: propose transfer 500000000 lamports to {} | wallet: treasury proposal: 43",
                SAMPLE_RECIPIENT
            ),
        ),
        (
            "msg_cancel_transfer",
            format!(
                "expires 2030-01-01 00:00:00: cancel transfer 1000000000 lamports to {} | wallet: treasury proposal: 42",
                SAMPLE_RECIPIENT
            ),
        ),
        (
            "msg_approve_spl",
            format!(
                "expires 2030-01-01 00:00:00: approve transfer 1500000 of mint {} to {} | wallet: treasury proposal: 12",
                USDC_MINT, SAMPLE_RECIPIENT
            ),
        ),
        (
            "msg_approve_add_intent",
            "expires 2030-01-01 00:00:00: approve add intent definition_hash: \
             0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
             | wallet: treasury proposal: 1"
                .to_string(),
        ),
        (
            "msg_approve_remove_intent",
            "expires 2030-01-01 00:00:00: approve remove intent 3 | wallet: treasury proposal: 2"
                .to_string(),
        ),
        (
            "msg_approve_update_intent",
            "expires 2030-01-01 00:00:00: approve update intent 2 definition_hash: \
             deadbeefcafebabe0123456789abcdef0123456789abcdef0123456789abcdef \
             | wallet: treasury proposal: 4"
                .to_string(),
        ),
        (
            // Cross-chain content shape — exercises the device's fallback
            // text-only review.
            "msg_btc_fallback",
            "expires 2030-01-01 00:00:00: approve send 12345 sats to bc1q-pkh:0xdeadbeef \
             from utxo 0xabcd:0 | wallet: treasury proposal: 5"
                .to_string(),
        ),
    ];

    for (name, body) in messages {
        let wrapped = build_solana_offchain(body);
        let mut transport = Vec::with_capacity(1 + wrapped.len());
        transport.push(0xFF); // Faraday sign-message QR routing byte
        transport.extend_from_slice(&wrapped);
        let b64 = BASE64.encode(&transport);

        let png_path = format!("{dir}/{name}.png");
        let bin_path = format!("{dir}/{name}.bin");
        write_qr_png(b64.as_bytes(), &png_path)?;
        // .bin is the *wrapped* off-chain bytes (what the device verifies the
        // signature over) — without the outer 0xFF transport byte.
        std::fs::write(&bin_path, &wrapped)?;
        written.push(png_path);
    }

    // ---- Transaction fixtures ----

    // Synthetic account slots used across the txs. None of these are real
    // PDAs; the parser doesn't care, it just renders whatever pubkey lands
    // in each instruction-account slot.
    let intent = filler(0x21);
    let proposal = filler(0x22);
    let vault = filler(0x23);
    let rent_refund = filler(0x24);
    let ika_config = filler(0x25);
    let dwallet_ownership = filler(0x26);
    let dwallet = filler(0x27);
    let message_approval = filler(0x28);
    let coordinator = filler(0x29);
    let cpi_authority = filler(0x2a);
    let caller_program = filler(0x2b);
    let dwallet_program = filler(0x2c);
    let system_program = [0u8; 32];

    let txs: Vec<(&str, Vec<u8>)> = vec![
        (
            "tx_create_wallet",
            // disc 0 + approval_threshold u8 + cancellation_threshold u8 +
            // timelock_seconds u32 LE + 40 bytes of opaque wincode tail.
            {
                let mut d = vec![0u8, 2, 1];
                d.extend_from_slice(&3600u32.to_le_bytes());
                d.extend_from_slice(&[0u8; 40]);
                build_tx(&signer_pubkey, &ika_program, &[signer_pubkey, intent, proposal, system_program], &d)
            },
        ),
        (
            "tx_propose",
            // disc 1 + proposal_idx u64 + expiry i64 + proposer [32] + sig [64] + params (24B).
            {
                let mut d = vec![1u8];
                d.extend_from_slice(&7u64.to_le_bytes());
                d.extend_from_slice(&1_893_456_000i64.to_le_bytes());
                d.extend_from_slice(&signer_pubkey); // proposer = signer
                d.extend_from_slice(&[0u8; 64]); // signature
                d.extend_from_slice(&[0xAAu8; 24]); // params_data
                // Accounts (per quasar_client/propose.rs):
                //   [payer, wallet, intent, proposal, system]
                build_tx(
                    &signer_pubkey,
                    &ika_program,
                    &[signer_pubkey, signer_pubkey, intent, proposal, system_program],
                    &d,
                )
            },
        ),
        (
            "tx_approve",
            // disc 2 + expiry i64 + approver_idx u8 + sig [64] = 74 bytes.
            {
                let mut d = vec![2u8];
                d.extend_from_slice(&1_893_456_000i64.to_le_bytes());
                d.push(3u8); // approver_index
                d.extend_from_slice(&[0u8; 64]);
                // Accounts: [wallet, intent, proposal]
                build_tx(
                    &signer_pubkey,
                    &ika_program,
                    &[signer_pubkey, intent, proposal],
                    &d,
                )
            },
        ),
        (
            "tx_cancel",
            {
                let mut d = vec![3u8];
                d.extend_from_slice(&1_893_456_000i64.to_le_bytes());
                d.push(5u8); // canceller_index
                d.extend_from_slice(&[0u8; 64]);
                build_tx(
                    &signer_pubkey,
                    &ika_program,
                    &[signer_pubkey, intent, proposal],
                    &d,
                )
            },
        ),
        (
            "tx_execute",
            // disc 4 only. Accounts: [wallet, vault, intent, proposal, system]
            build_tx(
                &signer_pubkey,
                &ika_program,
                &[signer_pubkey, vault, intent, proposal, system_program],
                &[4u8],
            ),
        ),
        (
            "tx_cleanup",
            // disc 5 only. Accounts: [proposal, rent_refund]
            build_tx(
                &signer_pubkey,
                &ika_program,
                &[proposal, rent_refund],
                &[5u8],
            ),
        ),
        (
            "tx_bind_dwallet",
            // disc 6 + chain_kind u8 + user_pubkey [32] + sig_scheme u16 LE + cpi_bump u8.
            bind_dwallet_tx(
                0, // Solana
                &signer_pubkey,
                &ika_program,
                &ika_config,
                &dwallet_ownership,
                &dwallet,
                &cpi_authority,
                &caller_program,
                &dwallet_program,
                &system_program,
            ),
        ),
        (
            "tx_bind_dwallet_evm",
            bind_dwallet_tx(
                1, // EVM (1559)
                &signer_pubkey,
                &ika_program,
                &ika_config,
                &dwallet_ownership,
                &dwallet,
                &cpi_authority,
                &caller_program,
                &dwallet_program,
                &system_program,
            ),
        ),
        (
            "tx_bind_dwallet_btc",
            bind_dwallet_tx(
                2, // Bitcoin
                &signer_pubkey,
                &ika_program,
                &ika_config,
                &dwallet_ownership,
                &dwallet,
                &cpi_authority,
                &caller_program,
                &dwallet_program,
                &system_program,
            ),
        ),
        (
            "tx_bind_dwallet_zcash",
            bind_dwallet_tx(
                3, // Zcash
                &signer_pubkey,
                &ika_program,
                &ika_config,
                &dwallet_ownership,
                &dwallet,
                &cpi_authority,
                &caller_program,
                &dwallet_program,
                &system_program,
            ),
        ),
        (
            "tx_bind_dwallet_erc20",
            bind_dwallet_tx(
                4, // ERC-20
                &signer_pubkey,
                &ika_program,
                &ika_config,
                &dwallet_ownership,
                &dwallet,
                &cpi_authority,
                &caller_program,
                &dwallet_program,
                &system_program,
            ),
        ),
        (
            "tx_ika_sign",
            // disc 7 + msg_approval_bump u8 + cpi_authority_bump u8 + 3×32B hashes.
            {
                let mut d = vec![7u8, 255, 254];
                d.extend_from_slice(&[0xa1u8; 32]);
                d.extend_from_slice(&[0xb2u8; 32]);
                d.extend_from_slice(&[0xc3u8; 32]);
                // Accounts (per quasar_client/ika_sign.rs):
                //   [payer, wallet, intent, proposal, ika_config,
                //    dwallet_ownership, dwallet, message_approval,
                //    coordinator, cpi_authority, caller_program,
                //    dwallet_program, system]
                build_tx(
                    &signer_pubkey,
                    &ika_program,
                    &[
                        signer_pubkey,
                        signer_pubkey,
                        intent,
                        proposal,
                        ika_config,
                        dwallet_ownership,
                        dwallet,
                        message_approval,
                        coordinator,
                        cpi_authority,
                        caller_program,
                        dwallet_program,
                        system_program,
                    ],
                    &d,
                )
            },
        ),
    ];

    for (name, tx) in &txs {
        let b64 = BASE64.encode(tx);
        let png_path = format!("{dir}/{name}.png");
        let bin_path = format!("{dir}/{name}.bin");
        write_qr_png(b64.as_bytes(), &png_path)?;
        std::fs::write(&bin_path, tx)?;
        written.push(png_path);
    }

    println!("Signer (account 0, no passphrase): {signer_b58}");
    println!("Ika program: {IKA_PROGRAM_B58}");
    println!();
    println!("Wrote {} fixtures to {dir}/:", written.len());
    for p in &written {
        println!("  {p}");
    }
    println!();
    println!("Display any .png on your Mac and scan from the simulator's Sign");
    println!("flow. Note: all signatures are placeholder zeros and blockhashes");
    println!("are placeholder bytes — these fixtures exercise device review +");
    println!("classifier paths, not on-chain submission.");
    Ok(())
}

/// Wrap a UTF-8 (restricted-ASCII) body in the Solana off-chain message
/// envelope (`\xffsolana offchain || ver=0 || fmt=0 || len_le_u16 || body`).
fn build_solana_offchain(body: &str) -> Vec<u8> {
    let body_bytes = body.as_bytes();
    let mut out = Vec::with_capacity(20 + body_bytes.len());
    out.extend_from_slice(b"\xffsolana offchain");
    out.push(0); // version
    out.push(0); // format (restricted ASCII)
    out.extend_from_slice(&(body_bytes.len() as u16).to_le_bytes());
    out.extend_from_slice(body_bytes);
    out
}

/// Build a minimal unsigned legacy Solana tx with exactly one custom
/// instruction. `ix_accounts` is the per-instruction account list in the
/// order the parser expects; this builder merges them with `[signer,
/// program]` into the tx's master account table and emits matching indices.
fn build_tx(
    signer: &[u8; 32],
    program_id: &[u8; 32],
    ix_accounts: &[[u8; 32]],
    ix_data: &[u8],
) -> Vec<u8> {
    // Master account layout:
    //   [0]    = signer (writable signer)
    //   [1]    = program_id
    //   [2..N] = unique accounts referenced by the ix (in original order)
    let mut master: Vec<[u8; 32]> = vec![*signer, *program_id];
    let mut ix_indices = Vec::with_capacity(ix_accounts.len());
    for acct in ix_accounts {
        let idx = match master.iter().position(|m| m == acct) {
            Some(i) => i,
            None => {
                master.push(*acct);
                master.len() - 1
            }
        };
        ix_indices.push(idx as u8);
    }

    let num_accounts = master.len();
    // Header: [num_required_signatures, num_readonly_signed, num_readonly_unsigned]
    // Slot 0 is the signer (1 required, 0 ro-signed). Everything else
    // (program + extra accounts) is ro-unsigned.
    let num_required_signatures = 1u8;
    let num_readonly_signed = 0u8;
    let num_readonly_unsigned = (num_accounts - 1) as u8;

    let mut tx = Vec::new();
    tx.push(1u8); // signature count (compact-u16, fits in 1 byte for n=1)
    tx.extend_from_slice(&[0u8; 64]); // placeholder signature

    tx.push(num_required_signatures);
    tx.push(num_readonly_signed);
    tx.push(num_readonly_unsigned);

    // Account keys
    tx.extend_from_slice(&encode_compact_u16(num_accounts as u16));
    for key in &master {
        tx.extend_from_slice(key);
    }

    // Recent blockhash (placeholder — not submittable)
    tx.extend_from_slice(&[0xABu8; 32]);

    // Instructions
    tx.push(1u8); // ix count
    let program_id_index = 1u8;
    tx.push(program_id_index);
    tx.extend_from_slice(&encode_compact_u16(ix_indices.len() as u16));
    tx.extend_from_slice(&ix_indices);
    tx.extend_from_slice(&encode_compact_u16(ix_data.len() as u16));
    tx.extend_from_slice(ix_data);

    tx
}

/// Solana compact-u16: 7 bits per byte, little-endian, MSB set if more bytes follow.
fn encode_compact_u16(mut n: u16) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut b = (n & 0x7f) as u8;
        n >>= 7;
        if n != 0 {
            b |= 0x80;
            out.push(b);
        } else {
            out.push(b);
            break;
        }
    }
    out
}

fn filler(byte: u8) -> [u8; 32] {
    [byte; 32]
}

#[allow(clippy::too_many_arguments)]
fn bind_dwallet_tx(
    chain_kind: u8,
    signer_pubkey: &[u8; 32],
    ika_program: &[u8; 32],
    ika_config: &[u8; 32],
    dwallet_ownership: &[u8; 32],
    dwallet: &[u8; 32],
    cpi_authority: &[u8; 32],
    caller_program: &[u8; 32],
    dwallet_program: &[u8; 32],
    system_program: &[u8; 32],
) -> Vec<u8> {
    let mut d = vec![6u8, chain_kind];
    d.extend_from_slice(signer_pubkey); // user_pubkey
    d.extend_from_slice(&1u16.to_le_bytes()); // sig_scheme
    d.push(255); // cpi_authority_bump
    build_tx(
        signer_pubkey,
        ika_program,
        &[
            *signer_pubkey,
            *signer_pubkey,
            *ika_config,
            *dwallet_ownership,
            *dwallet,
            *cpi_authority,
            *caller_program,
            *dwallet_program,
            *system_program,
        ],
        &d,
    )
}

fn write_qr_png(text_bytes: &[u8], path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (matrix, size) =
        qr::encode_qr::generate_qr_matrix(text_bytes, qr::encode_qr::QrEcLevel::M)?;
    let scale = 8u32;
    let quiet = 4u32;
    let px = (size as u32 + 2 * quiet) * scale;

    let mut img = GrayImage::from_pixel(px, px, Luma([255u8]));
    for r in 0..size {
        for c in 0..size {
            if matrix[r * size + c] {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = (quiet + c as u32) * scale + dx;
                        let y = (quiet + r as u32) * scale + dy;
                        img.put_pixel(x, y, Luma([0u8]));
                    }
                }
            }
        }
    }
    img.save(path)?;
    Ok(())
}
