//! Dev helper: generate a CompactSeedQR PNG from a test mnemonic. Used to
//! point a Pi camera at a clean machine-generated QR when debugging scans.

// Pulls whole modules via `#[path]` but only uses a subset.
#![allow(dead_code)]

#[path = "../crypto/mod.rs"]
mod crypto;

#[path = "../qr/mod.rs"]
mod qr;

use image::{GrayImage, Luma};

// Canonical all-zero-entropy 12-word test mnemonic.
const DEFAULT_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mnemonic = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let mnemonic = if mnemonic.trim().is_empty() {
        DEFAULT_MNEMONIC.to_string()
    } else {
        mnemonic
    };

    // CompactSeedQR: raw entropy bytes in byte mode.
    let compact = qr::encode_qr::encode_compact_seed_qr(&mnemonic)?;
    let (matrix, size) = qr::encode_qr::generate_qr_matrix(&compact, qr::encode_qr::QrEcLevel::L)?;

    let scale = 16u32;
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

    let out = "/tmp/faraday_test_qr.png";
    img.save(out)?;

    let keypair = crypto::derivation::derive_keypair(&mnemonic, "", 0)
        .ok_or("key derivation failed")?;
    let address = crypto::derivation::address(&keypair);

    println!("wrote {out} ({px} px square, CompactSeedQR {size}x{size})");
    println!("mnemonic: {mnemonic}");
    println!("account 0 address (no passphrase): {address}");
    Ok(())
}
