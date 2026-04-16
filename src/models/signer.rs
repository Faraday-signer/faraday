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

use ed25519_dalek::{Signer, SigningKey};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Result of signing a transaction.
pub struct SignedTransaction {
    pub signed_bytes: Vec<u8>,
    pub signed_base64: String,
    pub signature: [u8; 64],
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

    // Find the signer's position in the account keys
    // Message format: [num_required_sigs: u8] [num_readonly_signed: u8] [num_readonly_unsigned: u8]
    //                 [num_account_keys: compact-u16] [account_keys: 32 bytes each] [...]
    if message_bytes.len() < 4 {
        return Err("Message too short");
    }

    let num_account_keys = read_compact_u16(message_bytes, 3)?;
    let keys_start = 3 + compact_u16_len(message_bytes, 3);

    let signer_index = find_signer_index(message_bytes, keys_start, num_account_keys.0, public_key)?;

    // Sign the message
    let signing_key = SigningKey::from_bytes(private_key);
    let signature = signing_key.sign(message_bytes);
    let sig_bytes = signature.to_bytes();

    // Build the signed transaction
    let mut signed = unsigned_tx_bytes.to_vec();
    let sig_offset = 1 + signer_index * 64;
    signed[sig_offset..sig_offset + 64].copy_from_slice(&sig_bytes);

    let signer_pubkey = bs58::encode(public_key).into_string();
    let signed_base64 = crate::models::encode_qr::encode_signed_tx(&signed);

    Ok(SignedTransaction {
        signed_bytes: signed,
        signed_base64,
        signature: sig_bytes,
        signer_pubkey,
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

/// Sign an arbitrary message with Ed25519.
pub fn sign_message(
    message: &[u8],
    private_key: &[u8; 32],
) -> [u8; 64] {
    let signing_key = SigningKey::from_bytes(private_key);
    let signature = signing_key.sign(message);
    signature.to_bytes()
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

    #[test]
    fn test_sign_message() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = bip39::mnemonic_to_seed(mnemonic, "");
        let keypair = slip0010::derive_solana_keypair(&seed, 0);

        let message = b"hello solana";
        let sig = sign_message(message, &keypair.private_key);
        assert_eq!(sig.len(), 64);

        // Verify the signature
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&keypair.public_key).unwrap();
        let signature = ed25519_dalek::Signature::from_bytes(&sig);
        assert!(verifying_key.verify_strict(message, &signature).is_ok());
    }

    #[test]
    fn test_compact_u16() {
        // Single byte
        assert_eq!(read_compact_u16(&[5], 0).unwrap(), (5, 1));
        // Two bytes
        assert_eq!(read_compact_u16(&[0x80, 0x01], 0).unwrap(), (128, 2));
    }
}
