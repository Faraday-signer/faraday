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
    SolanaSignMessage,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DecodedQr {
    pub qr_type: QrType,
    pub raw_data: Vec<u8>,
    pub mnemonic: Option<String>,
    pub tx_bytes: Option<Vec<u8>>,
    pub message_bytes: Option<Vec<u8>>,
    pub address: Option<String>,
}

/// Detect QR type and decode.
///
/// Text formats are tried first so a 32-char base58 Solana address isn't
/// ambiguously classified as a 24-word CompactSeedQR (both are 32 bytes).
pub fn detect_and_decode(data: &[u8]) -> DecodedQr {
    if let Ok(s) = std::str::from_utf8(data) {
        let text = s.trim();

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
                message_bytes: None,
                address: Some(text.to_string()),
            };
        }

        // Base64 payload: transaction or sign-message
        if text.len() > 50 {
            if let Ok(bytes) = BASE64.decode(text) {
                if bytes.first() == Some(&0xFF) {
                    return DecodedQr {
                        qr_type: QrType::SolanaSignMessage,
                        raw_data: data.to_vec(),
                        mnemonic: None,
                        tx_bytes: None,
                        message_bytes: Some(bytes[1..].to_vec()),
                        address: None,
                    };
                }
                if bytes.len() >= 65 {
                    return DecodedQr {
                        qr_type: QrType::SolanaTxBase64,
                        raw_data: data.to_vec(),
                        mnemonic: None,
                        tx_bytes: Some(bytes),
                        message_bytes: None,
                        address: None,
                    };
                }
            }
        }
    }

    // Binary CompactSeedQR: raw entropy, 16 bytes (12 words) or 32 (24 words)
    if data.len() == 16 || data.len() == 32 {
        if let Some(decoded) = try_decode_compact_seed_qr(data) {
            return decoded;
        }
    }

    DecodedQr { qr_type: QrType::Unknown, raw_data: data.to_vec(), mnemonic: None, tx_bytes: None, message_bytes: None, address: None }
}

fn try_decode_seed_qr(data: &str) -> Option<DecodedQr> {
    let mut words = Vec::new();
    for i in (0..data.len()).step_by(4) {
        let idx: usize = data[i..i + 4].parse().ok()?;
        words.push(bip39::get_word(idx)?.to_string());
    }
    let mnemonic = words.join(" ");
    if !bip39::validate_mnemonic(&mnemonic) { return None; }

    Some(DecodedQr {
        qr_type: QrType::SeedQr,
        raw_data: data.as_bytes().to_vec(),
        mnemonic: Some(mnemonic),
        tx_bytes: None,
        message_bytes: None,
        address: None,
    })
}

fn try_decode_compact_seed_qr(data: &[u8]) -> Option<DecodedQr> {
    // Raw entropy (16 or 32 bytes); rebuild the mnemonic via the BIP39 spec,
    // which recomputes the checksum from the entropy and yields a validated
    // mnemonic.
    let mnemonic = bip39::mnemonic_from_raw_entropy(data).ok()?;
    Some(DecodedQr {
        qr_type: QrType::CompactSeedQr,
        raw_data: data.to_vec(),
        mnemonic: Some(mnemonic),
        tx_bytes: None,
        message_bytes: None,
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
    fn test_compact_seed_qr_has_no_integrity_check() {
        // The Compact format is raw entropy — the mnemonic is rebuilt and its
        // checksum recomputed from whatever bytes arrive. Any 16 or 32 bytes
        // therefore decode as a valid CompactSeedQr with no way to detect a
        // substituted seed. This is why the Load flow gates it behind a
        // blocking address confirmation (finding #89).
        for len in [16usize, 32] {
            let data = vec![0xABu8; len];
            let decoded = detect_and_decode(&data);
            assert_eq!(decoded.qr_type, QrType::CompactSeedQr);
            assert!(decoded.mnemonic.is_some());
        }
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

    #[test]
    fn test_sign_message_prefix() {
        let msg = b"Hello Faraday";
        let mut payload = vec![0xFF];
        payload.extend_from_slice(msg);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&payload);
        // Pad to >50 chars so the base64 detection branch triggers
        let padded = if b64.len() <= 50 {
            let mut long_msg = vec![0xFF];
            long_msg.extend_from_slice(&[0x41; 60]);
            base64::engine::general_purpose::STANDARD.encode(&long_msg)
        } else {
            b64
        };
        let decoded = detect_and_decode(padded.as_bytes());
        assert_eq!(decoded.qr_type, QrType::SolanaSignMessage);
        assert!(decoded.message_bytes.is_some());
        assert!(decoded.tx_bytes.is_none());
    }
}
