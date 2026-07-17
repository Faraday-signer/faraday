//! Ed25519 transaction and message signing for Solana.
//!
//! Implements signing without the `solders` crate — directly manipulates
//! the Solana transaction wire format.
//!
//! Wire format (legacy):
//!   [num_signatures: u8]
//!   [signatures: 64 bytes each]
//!   [message bytes...]
//!
//! The message contains account keys. The signer's position in the
//! signatures array matches their position in the account keys array.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Result of signing a transaction.
pub struct SignedTransaction {
    #[cfg(any(test, feature = "_desktop_sim"))]
    pub signed_bytes: Vec<u8>,
    /// `faraday:sig:<base64(v || pubkey || sig)>` — the compact return
    /// payload shown on the Pi's `SignShowQr` screen so the reader can
    /// read a tiny QR instead of a dense one. See
    /// `crate::qr::encode_qr::encode_signature_envelope`.
    pub signature_envelope: String,
    #[cfg(any(test, feature = "_desktop_sim"))]
    pub signature: [u8; 64],
    #[cfg(any(test, feature = "_desktop_sim"))]
    pub signer_pubkey: String,
}

/// Sign a serialized Solana transaction.
///
/// Finds the signer's pubkey in the account keys, signs the message portion,
/// and replaces the corresponding placeholder signature.
pub fn sign_transaction_bytes(
    unsigned_tx_bytes: &[u8],
    private_key: &[u8; 32],
    public_key: &[u8; 32],
) -> Result<SignedTransaction, &'static str> {
    if unsigned_tx_bytes.is_empty() {
        return Err("Empty transaction");
    }

    // Parse the wire format
    let num_sigs = unsigned_tx_bytes[0] as usize;
    if unsigned_tx_bytes.len() < 1 + num_sigs * 64 + 1 {
        return Err("Transaction too short");
    }

    let sigs_end = 1 + num_sigs * 64;
    let message_bytes = &unsigned_tx_bytes[sigs_end..];

    // Versioned-tx detection. Legacy messages start with the header; V0+
    // prefix one version byte whose high bit is set (`0x80` = V0). We only
    // need the prefix length for *parsing*; the signature is computed over
    // the message bytes as they sit on the wire (prefix included), so
    // `signing_key.sign(message_bytes)` below stays unchanged.
    let version_prefix_len = match message_bytes.first() {
        Some(&b) if b & 0x80 != 0 => 1,
        _ => 0,
    };

    // Find the signer's position in the account keys
    // Message format: [version_prefix?: u8]
    //                 [num_required_sigs: u8] [num_readonly_signed: u8] [num_readonly_unsigned: u8]
    //                 [num_account_keys: compact-u16] [account_keys: 32 bytes each] [...]
    let keys_count_offset = version_prefix_len + 3;
    if message_bytes.len() < keys_count_offset + 1 {
        return Err("Message too short");
    }

    let num_account_keys = read_compact_u16(message_bytes, keys_count_offset)?;
    let keys_start = keys_count_offset + compact_u16_len(message_bytes, keys_count_offset);

    let signer_index = find_signer_index(message_bytes, keys_start, num_account_keys.0, public_key)?;

    // Sign the message
    let signing_key = SigningKey::from_bytes(private_key);
    let signature = signing_key.sign(message_bytes);
    let sig_bytes = signature.to_bytes();

    // Defence-in-depth: verify our own signature against the message + claimed
    // public key before we hand it back. This catches (a) a private/public key
    // pair that doesn't actually correspond — which would otherwise produce a
    // signature that gets rejected by relayers — and (b) any internal bug that
    // silently corrupts the sig bytes between signing and returning.
    verify_signature(message_bytes, &sig_bytes, public_key)
        .map_err(|_| "Post-sign verification failed")?;

    // Build the signed transaction
    let mut signed = unsigned_tx_bytes.to_vec();
    let sig_offset = 1 + signer_index * 64;
    signed[sig_offset..sig_offset + 64].copy_from_slice(&sig_bytes);

    let signature_envelope =
        crate::qr::encode_qr::encode_signature_envelope(&sig_bytes, public_key);

    Ok(SignedTransaction {
        #[cfg(any(test, feature = "_desktop_sim"))]
        signed_bytes: signed,
        signature_envelope,
        #[cfg(any(test, feature = "_desktop_sim"))]
        signature: sig_bytes,
        #[cfg(any(test, feature = "_desktop_sim"))]
        signer_pubkey: bs58::encode(public_key).into_string(),
    })
}

/// Sign a base64-encoded transaction.
pub fn sign_transaction_base64(
    unsigned_tx_base64: &str,
    private_key: &[u8; 32],
    public_key: &[u8; 32],
) -> Result<SignedTransaction, &'static str> {
    let tx_bytes = BASE64.decode(unsigned_tx_base64).map_err(|_| "Invalid base64")?;
    sign_transaction_bytes(&tx_bytes, private_key, public_key)
}

/// Upper bound on off-chain message length (Solana packet MTU). Longer
/// payloads are rejected before signing.
const MAX_OFFCHAIN_MSG: usize = 1232;

/// Sign an arbitrary message under the Solana off-chain-message domain.
///
/// WHY the domain prefix: signing the bare bytes lets an attacker submit a
/// transaction message through the "sign message" channel and receive a valid
/// *transaction* signature, bypassing the tx-review UI. Signing a
/// domain-separated preimage guarantees the result can never validate as
/// `ed25519(pubkey, tx_message)`.
pub fn sign_message(
    message: &[u8],
    private_key: &[u8; 32],
) -> Result<[u8; 64], &'static str> {
    if message.len() > MAX_OFFCHAIN_MSG {
        return Err("Message too long");
    }
    let signing_key = SigningKey::from_bytes(private_key);
    let signature = signing_key.sign(&offchain_preimage(message));
    Ok(signature.to_bytes())
}

/// Build the Solana off-chain-message preimage:
/// `b"\xffsolana offchain" || version(0u8) || len(u16 le) || message`.
fn offchain_preimage(message: &[u8]) -> Vec<u8> {
    let mut preimage = Vec::with_capacity(16 + 1 + 2 + message.len());
    preimage.extend_from_slice(b"\xffsolana offchain");
    preimage.push(0u8); // version
    preimage.extend_from_slice(&(message.len() as u16).to_le_bytes());
    preimage.extend_from_slice(message);
    preimage
}

/// Strict ed25519 verification of (message, signature, public_key).
/// Uses `verify_strict` which enforces the canonical signature form required
/// by Solana validators — catches malleability + non-canonical encodings.
pub fn verify_signature(
    message: &[u8],
    signature: &[u8; 64],
    public_key: &[u8; 32],
) -> Result<(), &'static str> {
    let vk = VerifyingKey::from_bytes(public_key).map_err(|_| "Invalid public key")?;
    let sig = Signature::from_bytes(signature);
    vk.verify_strict(message, &sig)
        .map_err(|_| "Signature verification failed")
}

/// Read a compact-u16 from the buffer at the given offset.
/// Returns (value, bytes_consumed).
fn read_compact_u16(buf: &[u8], offset: usize) -> Result<(usize, usize), &'static str> {
    if offset >= buf.len() { return Err("Buffer too short for compact-u16"); }

    let first = buf[offset] as usize;
    if first < 0x80 {
        return Ok((first, 1));
    }

    if offset + 1 >= buf.len() { return Err("Buffer too short for compact-u16"); }
    let second = buf[offset + 1] as usize;
    if second < 0x80 {
        return Ok((((first & 0x7F) | (second << 7)), 2));
    }

    if offset + 2 >= buf.len() { return Err("Buffer too short for compact-u16"); }
    let third = buf[offset + 2] as usize;
    Ok((((first & 0x7F) | ((second & 0x7F) << 7) | (third << 14)), 3))
}

/// Get the byte length of a compact-u16 at the given offset.
fn compact_u16_len(buf: &[u8], offset: usize) -> usize {
    read_compact_u16(buf, offset).map(|(_, len)| len).unwrap_or(1)
}

/// Find the signer's index in the account keys.
fn find_signer_index(
    message: &[u8],
    keys_start: usize,
    num_keys: usize,
    public_key: &[u8; 32],
) -> Result<usize, &'static str> {
    for i in 0..num_keys {
        let key_offset = keys_start + i * 32;
        if key_offset + 32 > message.len() {
            return Err("Message too short for account keys");
        }
        if &message[key_offset..key_offset + 32] == public_key {
            return Ok(i);
        }
    }
    Err("Signer not found in transaction account keys")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::bip39;
    use crate::crypto::slip0010;

    fn test_keypair(account: u32) -> crate::crypto::slip0010::SolanaKeypair {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = bip39::mnemonic_to_seed(mnemonic, "");
        slip0010::derive_solana_keypair(&seed, account).unwrap()
    }

    /// Minimal valid legacy tx with one signer. Mirrors the shape App::build_test_transaction
    /// produces, but self-contained so we don't depend on GUI code from tests.
    fn build_unsigned_tx(signer_pubkey: &[u8; 32]) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1);                         // num_signatures
        tx.extend_from_slice(&[0u8; 64]);   // placeholder sig
        tx.push(1);                         // num_required_signatures
        tx.push(0);                         // num_readonly_signed
        tx.push(1);                         // num_readonly_unsigned
        tx.push(2);                         // num_account_keys (compact-u16)
        tx.extend_from_slice(signer_pubkey);
        tx.extend_from_slice(&[0u8; 32]);   // system program
        tx.extend_from_slice(&[7u8; 32]);   // recent blockhash
        tx.push(0);                         // num_instructions
        tx
    }

    /// Same shape but with the `0x80` V0 versioned-message prefix. Regression
    /// test for the legacy-only parser that used to drop V0 txs on the floor
    /// (signer couldn't locate its pubkey in the account keys because the
    /// parse offset was shifted by the version byte).
    fn build_unsigned_v0_tx(signer_pubkey: &[u8; 32]) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1);                         // num_signatures
        tx.extend_from_slice(&[0u8; 64]);   // placeholder sig
        tx.push(0x80);                      // v0 version prefix
        tx.push(1);                         // num_required_signatures
        tx.push(0);                         // num_readonly_signed
        tx.push(1);                         // num_readonly_unsigned
        tx.push(2);                         // num_account_keys (compact-u16)
        tx.extend_from_slice(signer_pubkey);
        tx.extend_from_slice(&[0u8; 32]);   // system program
        tx.extend_from_slice(&[7u8; 32]);   // recent blockhash
        tx.push(0);                         // num_instructions
        // No address-table lookups — empty list.
        tx.push(0);
        tx
    }

    #[test]
    fn test_sign_message() {
        let kp = test_keypair(0);
        let message = b"hello solana";
        let sig = sign_message(message, &kp.private_key).unwrap();
        assert_eq!(sig.len(), 64);
        // Verifies against the domain-separated preimage, not the bare message.
        assert!(verify_signature(&offchain_preimage(message), &sig, &kp.public_key).is_ok());
        assert!(verify_signature(message, &sig, &kp.public_key).is_err());
    }

    /// Crown-jewel regression (#79): a message-channel signature must never
    /// validate as a transaction signature over the same bytes. Domain
    /// separation is what breaks the forge.
    #[test]
    fn sign_message_cannot_forge_tx_signature() {
        let kp = test_keypair(0);
        let tx = build_unsigned_tx(&kp.public_key);
        let sigs_end = 1 + 1 * 64;
        let tx_message = &tx[sigs_end..]; // the bytes a relayer verifies as a tx sig
        let sig = sign_message(tx_message, &kp.private_key).unwrap();
        assert!(verify_signature(tx_message, &sig, &kp.public_key).is_err());
    }

    #[test]
    fn sign_message_rejects_over_length() {
        let kp = test_keypair(0);
        let too_long = vec![0u8; MAX_OFFCHAIN_MSG + 1];
        assert!(sign_message(&too_long, &kp.private_key).is_err());
    }

    #[test]
    fn test_compact_u16() {
        assert_eq!(read_compact_u16(&[5], 0).unwrap(), (5, 1));
        assert_eq!(read_compact_u16(&[0x80, 0x01], 0).unwrap(), (128, 2));
    }

    // --- verify_signature edge cases ---

    #[test]
    fn verify_accepts_valid_signature() {
        let kp = test_keypair(0);
        let msg = b"canonical input";
        let sig = sign_message(msg, &kp.private_key).unwrap();
        assert!(verify_signature(&offchain_preimage(msg), &sig, &kp.public_key).is_ok());
    }

    #[test]
    fn verify_rejects_tampered_message() {
        let kp = test_keypair(0);
        let msg = b"original message";
        let sig = sign_message(msg, &kp.private_key).unwrap();
        let tampered = b"tampered message";
        assert!(verify_signature(&offchain_preimage(tampered), &sig, &kp.public_key).is_err());
    }

    #[test]
    fn verify_rejects_corrupted_signature() {
        let kp = test_keypair(0);
        let msg = b"hello";
        let mut sig = sign_message(msg, &kp.private_key).unwrap();
        sig[0] ^= 0x01; // flip one bit
        assert!(verify_signature(&offchain_preimage(msg), &sig, &kp.public_key).is_err());
    }

    #[test]
    fn verify_rejects_wrong_pubkey() {
        let kp_a = test_keypair(0);
        let kp_b = test_keypair(1); // different account → different keypair
        let msg = b"hello";
        let sig = sign_message(msg, &kp_a.private_key).unwrap();
        assert!(verify_signature(&offchain_preimage(msg), &sig, &kp_b.public_key).is_err());
    }

    #[test]
    fn verify_rejects_all_zero_signature() {
        let kp = test_keypair(0);
        let zero_sig = [0u8; 64];
        assert!(verify_signature(b"any", &zero_sig, &kp.public_key).is_err());
    }

    // --- sign_transaction_bytes with built-in post-sign verify ---

    #[test]
    fn sign_tx_round_trip_verifies_externally() {
        let kp = test_keypair(0);
        let tx = build_unsigned_tx(&kp.public_key);
        let signed = sign_transaction_bytes(&tx, &kp.private_key, &kp.public_key).unwrap();

        // Extract the message portion from the signed tx and re-verify externally.
        let sigs_end = 1 + 1 * 64;
        let message = &signed.signed_bytes[sigs_end..];
        assert!(verify_signature(message, &signed.signature, &kp.public_key).is_ok());
    }

    #[test]
    fn signature_envelope_payload_shape() {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;

        let kp = test_keypair(0);
        let tx = build_unsigned_tx(&kp.public_key);
        let signed = sign_transaction_bytes(&tx, &kp.private_key, &kp.public_key).unwrap();

        // Envelope shape: `faraday:sig:<base64(v || pubkey_32 || sig_64)>`.
        assert!(signed.signature_envelope.starts_with("faraday:sig:"));
        let b64 = signed.signature_envelope.trim_start_matches("faraday:sig:");
        let payload = BASE64.decode(b64).expect("envelope must be valid base64");
        assert_eq!(payload.len(), 1 + 32 + 64);
        assert_eq!(
            payload[0],
            crate::qr::encode_qr::SIG_ENVELOPE_VERSION,
            "version byte must match current envelope version"
        );
        assert_eq!(&payload[1..33], &kp.public_key, "pubkey slot must carry signer pubkey");
        assert_eq!(&payload[33..97], &signed.signature, "sig slot must carry the produced signature");

        // The signature inside the envelope must verify against the full
        // message bytes (prefix included) — what the extension will do.
        let sigs_end = 1 + 1 * 64;
        let message = &signed.signed_bytes[sigs_end..];
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&payload[33..97]);
        assert!(verify_signature(message, &sig_arr, &kp.public_key).is_ok());
    }

    #[test]
    fn sign_tx_v0_versioned_round_trip() {
        // V0 versioned-tx signing was silently failing before the fix: the
        // parser assumed the header started at message byte 0, so on a V0
        // tx (where byte 0 is `0x80`) it read bogus values for the account-
        // keys offset and `find_signer_index` couldn't locate the signer.
        let kp = test_keypair(0);
        let tx = build_unsigned_v0_tx(&kp.public_key);
        let signed = sign_transaction_bytes(&tx, &kp.private_key, &kp.public_key).unwrap();
        let sigs_end = 1 + 1 * 64;
        let message = &signed.signed_bytes[sigs_end..];
        assert_eq!(message[0], 0x80, "v0 prefix must survive signing unchanged");
        assert!(verify_signature(message, &signed.signature, &kp.public_key).is_ok());
    }

    #[test]
    fn sign_tx_detects_mismatched_keypair() {
        // Legit bug we want to catch: caller passes private of A but claims B's pubkey.
        // The tx is built around B (so find_signer_index succeeds), signing proceeds
        // with A's private key, and the internal verify catches that A's sig doesn't
        // validate against B's pubkey.
        let kp_a = test_keypair(0);
        let kp_b = test_keypair(1);
        assert_ne!(kp_a.public_key, kp_b.public_key);

        let tx = build_unsigned_tx(&kp_b.public_key);
        let result = sign_transaction_bytes(&tx, &kp_a.private_key, &kp_b.public_key);

        assert!(result.is_err(), "signing should fail on keypair mismatch");
        assert_eq!(result.err().unwrap(), "Post-sign verification failed");
    }

    #[test]
    fn sign_tx_errors_when_pubkey_not_in_accounts() {
        // Building the tx around someone else: our pubkey isn't in the account list,
        // so we should bail before signing.
        let kp_a = test_keypair(0);
        let kp_b = test_keypair(1);
        let tx = build_unsigned_tx(&kp_b.public_key);
        let result = sign_transaction_bytes(&tx, &kp_a.private_key, &kp_a.public_key);
        assert!(result.is_err());
    }

    #[test]
    fn sign_tx_errors_on_empty_input() {
        let kp = test_keypair(0);
        let result = sign_transaction_bytes(&[], &kp.private_key, &kp.public_key);
        assert_eq!(result.err().unwrap(), "Empty transaction");
    }

    #[test]
    fn sign_tx_errors_on_truncated_input() {
        // Header claims 1 signature but buffer ends before message.
        let kp = test_keypair(0);
        let tx = vec![1u8]; // num_sigs only
        let result = sign_transaction_bytes(&tx, &kp.private_key, &kp.public_key);
        assert!(result.is_err());
    }
}
