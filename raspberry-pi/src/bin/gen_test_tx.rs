//! Dev helper: build an unsigned Solana System::Transfer (0.01 SOL self-send)
//! for a given mnemonic, base64-encode it, and render as a PNG QR so you can
//! scan-test the Sign TX flow on a Pi device. Also writes the raw tx bytes
//! alongside for parser testing.
//!
//! Outputs land in `testdata/examples/` (committed to the repo so demo
//! materials are available without re-running this tool).
//!
//! Default mnemonic is the canonical BIP39 test vector (`abandon × 11 + about`)
//! — never put a real seed in this file.

use faraday_core::crypto;
use faraday_core::qr;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::{GrayImage, Luma};

const DEFAULT_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

const LAMPORTS: u64 = 10_000_000; // 0.01 SOL

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mnemonic = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let mnemonic = if mnemonic.trim().is_empty() {
        DEFAULT_MNEMONIC.to_string()
    } else {
        mnemonic
    };

    let keypair = crypto::derivation::derive_keypair(&mnemonic, "", 0)
        .ok_or("key derivation failed")?;
    let address = crypto::derivation::address(&keypair);

    // Unsigned legacy Solana transaction with a single System::Transfer
    // instruction sending `LAMPORTS` from the signer back to itself. The
    // signature slot is 64 zero bytes — the Faraday fills it in on sign.
    let mut tx = Vec::new();
    tx.push(1u8); // signatures count (compact-u16)
    tx.extend_from_slice(&[0u8; 64]); // placeholder signature
    // Message header
    tx.push(1); // num_required_signatures
    tx.push(0); // num_readonly_signed
    tx.push(1); // num_readonly_unsigned (system program)
    // Account keys: [signer, system-program]
    tx.push(2); // compact-u16 count
    tx.extend_from_slice(&keypair.public_key);
    tx.extend_from_slice(&[0u8; 32]); // system program id
    // Recent blockhash (placeholder — this tx isn't submittable, but that's
    // fine: we're demoing the Sign flow, not broadcasting).
    tx.extend_from_slice(&[0xABu8; 32]);
    // Instructions: 1
    tx.push(1); // compact-u16 count
    tx.push(1); // program_id_index (system program)
    tx.push(2); // num accounts in instruction
    tx.push(0); // from = accounts[0] = signer
    tx.push(0); // to   = accounts[0] = signer (self-transfer)
    tx.push(12); // data length (compact-u16)
    tx.extend_from_slice(&[2u8, 0, 0, 0]); // Transfer opcode (u32 LE)
    tx.extend_from_slice(&LAMPORTS.to_le_bytes());

    let tx_b64 = BASE64.encode(&tx);

    // QR render
    let (matrix, size) = qr::encode_qr::generate_qr_matrix(tx_b64.as_bytes(), qr::encode_qr::QrEcLevel::M)?;
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

    let dir = "testdata/examples";
    std::fs::create_dir_all(dir)?;
    let png_path = format!("{dir}/self_transfer.png");
    let bin_path = format!("{dir}/self_transfer.bin");
    img.save(&png_path)?;
    std::fs::write(&bin_path, &tx)?;

    println!(
        "wrote {png_path} ({px} px square, QR {size}x{size}, {} tx bytes, {} base64 chars)",
        tx.len(),
        tx_b64.len()
    );
    println!("wrote {bin_path} (raw tx bytes for parser testing)");
    println!("from / to: {address}  (self-transfer)");
    println!("amount:    {} lamports ({} SOL)", LAMPORTS, LAMPORTS as f64 / 1e9);
    println!();
    println!("Note: placeholder blockhash — this tx can't be submitted; it's");
    println!("for demoing the Faraday Sign TX flow end-to-end.");
    Ok(())
}
