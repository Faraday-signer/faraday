//! Lossless recovery of a decoded QR's payload bytes.
//!
//! Shared by the Pi and ESP32 rxing backends so binary payloads (raw-entropy
//! CompactSeedQR) survive the decode identically on both.

use rxing::{RXingResult, RXingResultMetadataType, RXingResultMetadataValue};

/// Recover the exact payload bytes from a decoded QR `result`.
///
/// rxing decodes byte-mode QR data through a charset guesser with
/// `DecoderTrap::Strict`, and `getText()` exposes only that decoded string:
/// bytes in 0x7F–0x9F (or an attacker-picked ECI) make the strict decode fail
/// silently to `""` or a length-changed value. Round-tripping binary through
/// `getText()` therefore imports a genuine CompactSeedQR as nothing or as a
/// *different* wallet. The `BYTE_SEGMENTS` metadata carries the raw bytes read
/// straight off the byte-mode segments, before any charset conversion — read
/// those so the bytes come back 1:1, independent of the declared ECI. Numeric /
/// alphanumeric QRs (SeedQR digits, base58 address, base64 tx) have no byte
/// segments and are pure ASCII, so `getText()` round-trips them exactly.
pub fn payload_bytes(result: &RXingResult) -> Vec<u8> {
    if let Some(RXingResultMetadataValue::ByteSegments(segments)) = result
        .getRXingResultMetadata()
        .get(&RXingResultMetadataType::BYTE_SEGMENTS)
    {
        return segments.concat();
    }
    result.getText().as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rxing::BarcodeFormat;

    /// Mirror how rxing reports a byte-mode segment: the exact bytes in
    /// `BYTE_SEGMENTS`, with a lossy strict-decoded `getText()` (empty here, as
    /// a strict charset decode of 0x80/0x9F yields).
    fn byte_mode_result(payload: &[u8]) -> RXingResult {
        let mut r = RXingResult::new("", Vec::new(), Vec::new(), BarcodeFormat::QR_CODE);
        r.putMetadata(
            RXingResultMetadataType::BYTE_SEGMENTS,
            RXingResultMetadataValue::ByteSegments(vec![payload.to_vec()]),
        );
        r
    }

    #[test]
    fn recovers_high_bytes_losslessly() {
        let payload = [0x80u8, 0x9F, 0xFF, 0x00, 0x7F, 0xAB];
        let r = byte_mode_result(&payload);
        assert_eq!(payload_bytes(&r), payload);
    }

    #[test]
    fn full_32_byte_entropy_survives() {
        // getText() would have collapsed this to "" — length must not change.
        let payload: Vec<u8> = (0u8..32).map(|i| i.wrapping_mul(9).wrapping_add(0x80)).collect();
        let r = byte_mode_result(&payload);
        let out = payload_bytes(&r);
        assert_eq!(out.len(), 32);
        assert_eq!(out, payload);
    }

    #[test]
    fn ascii_text_without_byte_segments() {
        // Numeric / alphanumeric QRs carry no BYTE_SEGMENTS; fall back to text.
        let r = RXingResult::new("123456781234", Vec::new(), Vec::new(), BarcodeFormat::QR_CODE);
        assert_eq!(payload_bytes(&r), b"123456781234".to_vec());
    }
}
