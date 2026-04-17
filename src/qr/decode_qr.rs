//! QR code decoding and type detection.

use crate::crypto::bip39;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrType {
    SeedQr,
    CompactSeedQr,
    SolanaAddress,
    SolanaTxBase64,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DecodedQr {
    pub qr_type: QrType,
    pub raw_data: Vec<u8>,
    pub mnemonic: Option<String>,
    pub tx_bytes: Option<Vec<u8>>,
    pub address: Option<String>,
}

/// Detect QR type and decode.
pub fn detect_and_decode(data: &[u8]) -> DecodedQr {
    // Binary: try CompactSeedQR
    if data.len() == 17 || data.len() == 33 {
        if let Some(decoded) = try_decode_compact_seed_qr(data) {
            return decoded;
        }
    }

    // Try as UTF-8 string
    let text = match std::str::from_utf8(data) {
        Ok(s) => s.trim(),
        Err(_) => return DecodedQr { qr_type: QrType::Unknown, raw_data: data.to_vec(), mnemonic: None, tx_bytes: None, address: None },
    };

    // SeedQR: all digits, 48 or 96 chars
    if (text.len() == 48 || text.len() == 96) && text.chars().all(|c| c.is_ascii_digit()) {
        if let Some(decoded) = try_decode_seed_qr(text) {
            return decoded;
        }
    }

    // Solana address: base58, 32-44 chars
    if text.len() >= 32 && text.len() <= 44 && is_base58(text) {
        return DecodedQr {
            qr_type: QrType::SolanaAddress,
            raw_data: data.to_vec(),
            mnemonic: None,
            tx_bytes: None,
            address: Some(text.to_string()),
        };
    }

    // Base64 transaction: > 50 chars
    if text.len() > 50 {
        if let Ok(tx_bytes) = BASE64.decode(text) {
            if tx_bytes.len() >= 65 {
                return DecodedQr {
                    qr_type: QrType::SolanaTxBase64,
                    raw_data: data.to_vec(),
                    mnemonic: None,
                    tx_bytes: Some(tx_bytes),
                    address: None,
                };
            }
        }
    }

    DecodedQr { qr_type: QrType::Unknown, raw_data: data.to_vec(), mnemonic: None, tx_bytes: None, address: None }
}

fn try_decode_seed_qr(data: &str) -> Option<DecodedQr> {
    let mut words = Vec::new();
    for i in (0..data.len()).step_by(4) {
        let idx: usize = data[i..i + 4].parse().ok()?;
        if idx >= 2048 { return None; }
        words.push(bip39::get_word(idx).to_string());
    }
    let mnemonic = words.join(" ");
    if !bip39::validate_mnemonic(&mnemonic) { return None; }

    Some(DecodedQr {
        qr_type: QrType::SeedQr,
        raw_data: data.as_bytes().to_vec(),
        mnemonic: Some(mnemonic),
        tx_bytes: None,
        address: None,
    })
}

fn try_decode_compact_seed_qr(data: &[u8]) -> Option<DecodedQr> {
    let word_count = match data.len() {
        17 => 12,
        33 => 24,
        _ => return None,
    };

    let bits: Vec<u8> = data.iter()
        .flat_map(|byte| (0..8).rev().map(move |i| (byte >> i) & 1))
        .collect();

    let mut words = Vec::new();
    for i in 0..word_count {
        let start = i * 11;
        let mut idx: usize = 0;
        for j in 0..11 {
            idx = (idx << 1) | (bits[start + j] as usize);
        }
        if idx >= 2048 { return None; }
        words.push(bip39::get_word(idx).to_string());
    }

    let mnemonic = words.join(" ");
    if !bip39::validate_mnemonic(&mnemonic) { return None; }

    Some(DecodedQr {
        qr_type: QrType::CompactSeedQr,
        raw_data: data.to_vec(),
        mnemonic: Some(mnemonic),
        tx_bytes: None,
        address: None,
    })
}

fn is_base58(s: &str) -> bool {
    s.chars().all(|c| matches!(c,
        '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z'
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qr::encode_qr;

    #[test]
    fn test_seed_qr_roundtrip() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let encoded = encode_qr::encode_seed_qr(mnemonic).unwrap();
        let decoded = detect_and_decode(encoded.as_bytes());
        assert_eq!(decoded.qr_type, QrType::SeedQr);
        assert_eq!(decoded.mnemonic.as_deref(), Some(mnemonic));
    }

    #[test]
    fn test_compact_seed_qr_roundtrip() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let compact = encode_qr::encode_compact_seed_qr(mnemonic).unwrap();
        let decoded = detect_and_decode(&compact);
        assert_eq!(decoded.qr_type, QrType::CompactSeedQr);
        assert_eq!(decoded.mnemonic.as_deref(), Some(mnemonic));
    }

    #[test]
    fn test_solana_address() {
        let addr = "11111111111111111111111111111111";
        let decoded = detect_and_decode(addr.as_bytes());
        assert_eq!(decoded.qr_type, QrType::SolanaAddress);
        assert_eq!(decoded.address.as_deref(), Some(addr));
    }

    #[test]
    fn test_base64_tx() {
        // Fake 100-byte transaction
        let fake_tx = vec![0u8; 100];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&fake_tx);
        let decoded = detect_and_decode(b64.as_bytes());
        assert_eq!(decoded.qr_type, QrType::SolanaTxBase64);
        assert_eq!(decoded.tx_bytes.as_deref(), Some(fake_tx.as_slice()));
    }
}
