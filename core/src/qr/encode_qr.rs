//! QR code encoding for Faraday outputs.

use crate::crypto::bip39;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

pub use qrcode::EcLevel as QrEcLevel;

/// Encode a signed transaction as base64 for QR display.
pub fn encode_signed_tx(signed_tx_bytes: &[u8]) -> String {
    BASE64.encode(signed_tx_bytes)
}

/// Envelope version for the signature-only QR. Bumped if the payload
/// layout below ever changes (new field order, added session hash, etc.).
pub const SIG_ENVELOPE_VERSION: u8 = 1;

/// Encode just the produced signature + signer pubkey for QR display,
/// prefixed with the `faraday:sig:` envelope. The extension side already
/// has the full unsigned transaction in its session state, so it only
/// needs the signature + pubkey to locate the correct signer slot and
/// splice the bytes in — no need to re-ship the whole tx on the return
/// leg. Payload is 1 (version) + 32 (pubkey) + 64 (sig) = 97 bytes →
/// ~144-char base64 renders as a V8 QR (49×49) on the Pi's 240×240
/// screen at ~4.9 px/module, readable by any webcam.
pub fn encode_signature_envelope(
    signature: &[u8; 64],
    signer_pubkey: &[u8; 32],
) -> String {
    let mut payload = Vec::with_capacity(1 + 32 + 64);
    payload.push(SIG_ENVELOPE_VERSION);
    payload.extend_from_slice(signer_pubkey);
    payload.extend_from_slice(signature);
    let mut s = String::from("faraday:sig:");
    s.push_str(&BASE64.encode(&payload));
    s
}

/// Encode a Solana public key as a base58 address for QR display.
pub fn encode_address(public_key: &[u8; 32]) -> String {
    bs58::encode(public_key).into_string()
}

/// Encode a BIP39 mnemonic as SeedQR (concatenated 4-digit word indices).
pub fn encode_seed_qr(mnemonic: &str) -> Result<String, &'static str> {
    let mut result = String::new();
    for word in mnemonic.split_whitespace() {
        let idx = bip39::word_index(word).ok_or("Unknown BIP39 word")?;
        result.push_str(&format!("{:04}", idx));
    }
    Ok(result)
}

/// Encode a BIP39 mnemonic as CompactSeedQR: raw entropy only, no checksum.
/// 16 bytes for 12 words, 32 bytes for 24 words. Produces a tiny QR in byte
/// mode; the checksum is recomputed from entropy on decode.
pub fn encode_compact_seed_qr(mnemonic: &str) -> Result<Vec<u8>, &'static str> {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    if words.len() != 12 && words.len() != 24 {
        return Err("mnemonic must be 12 or 24 words");
    }

    let mut bits = Vec::with_capacity(words.len() * 11);
    for word in &words {
        let idx = bip39::word_index(word).ok_or("Unknown BIP39 word")?;
        for i in (0..11).rev() {
            bits.push(((idx >> i) & 1) as u8);
        }
    }

    let mut bytes = Vec::new();
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for (i, &bit) in chunk.iter().enumerate() {
            byte |= bit << (7 - i);
        }
        bytes.push(byte);
    }
    // Drop the trailing checksum bits — 12 words → 16 B, 24 words → 32 B.
    bytes.truncate(words.len() * 4 / 3);
    Ok(bytes)
}

/// Generate a QR code as a boolean matrix (true = black module).
///
/// `ec` picks the error-correction level. Use `QrEcLevel::L` for seed-backup
/// QRs (smallest grid — 21×21 for a 12-word CompactSeedQR) and `QrEcLevel::M`
/// for anything that will be scanned in the field (tx, signature, address),
/// where extra ECC headroom matters more than grid size.
///
/// Returns (matrix, size) where matrix is row-major and size is the dimension.
pub fn generate_qr_matrix(data: &[u8], ec: QrEcLevel) -> Result<(Vec<bool>, usize), &'static str> {
    use qrcode::QrCode;
    let code = QrCode::with_error_correction_level(data, ec).map_err(|_| "QR encoding failed")?;
    let width = code.width();
    let matrix: Vec<bool> = code
        .into_colors()
        .into_iter()
        .map(|c| c == qrcode::Color::Dark)
        .collect();
    Ok((matrix, width))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_address() {
        let key = [0u8; 32];
        let addr = encode_address(&key);
        assert_eq!(addr, "11111111111111111111111111111111");
    }

    #[test]
    fn test_seed_qr_roundtrip() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let encoded = encode_seed_qr(mnemonic).unwrap();
        assert_eq!(encoded, "000000000000000000000000000000000000000000000003");
        assert_eq!(encoded.len(), 48); // 12 words * 4 digits
    }

    #[test]
    fn test_compact_seed_qr() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let compact = encode_compact_seed_qr(mnemonic).unwrap();
        assert_eq!(compact.len(), 16); // 128 bits of entropy, no checksum
    }

    #[test]
    fn test_generate_qr_matrix() {
        let (matrix, size) = generate_qr_matrix(b"test", QrEcLevel::M).unwrap();
        assert!(size > 0);
        assert_eq!(matrix.len(), size * size);
    }

    #[test]
    fn compact_seed_qr_12w_is_v1_at_ec_l() {
        let mn = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let compact = encode_compact_seed_qr(mn).unwrap();
        let (_, size) = generate_qr_matrix(&compact, QrEcLevel::L).unwrap();
        assert_eq!(size, 21, "12-word CompactSeedQR should be V1 21×21 at ECL L");
    }

    #[test]
    fn test_signed_tx_base64() {
        let tx = vec![1, 2, 3, 4, 5];
        let encoded = encode_signed_tx(&tx);
        assert_eq!(encoded, "AQIDBAU=");
    }
}
