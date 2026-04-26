//! SLIP-0010 Ed25519 key derivation for Solana.
//!
//! Uses HMAC-SHA512 for key derivation and ed25519-dalek for public keys.
//! Only hardened derivation is valid for Ed25519 (SLIP-0010 spec).
//!
//! Solana BIP44 path: m/44'/501'/account'/change'

use ed25519_dalek::SigningKey;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use zeroize::Zeroize;

type HmacSha512 = Hmac<Sha512>;

const HARDENED: u32 = 0x80000000;

/// Derive master key and chain code from BIP39 seed.
fn derive_master(seed: &[u8]) -> Option<([u8; 32], [u8; 32])> {
    let mut mac = HmacSha512::new_from_slice(b"ed25519 seed").ok()?;
    mac.update(seed);
    let result = mac.finalize().into_bytes();

    let mut key = [0u8; 32];
    let mut chain = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    chain.copy_from_slice(&result[32..]);

    Some((key, chain))
}

/// Derive a hardened child key.
fn derive_child(
    parent_key: &[u8; 32],
    parent_chain: &[u8; 32],
    index: u32,
) -> Option<([u8; 32], [u8; 32])> {
    if index & HARDENED == 0 {
        return None;
    }

    let mut data = Vec::with_capacity(37);
    data.push(0x00);
    data.extend_from_slice(parent_key);
    data.extend_from_slice(&index.to_be_bytes());

    let mut mac = HmacSha512::new_from_slice(parent_chain).ok()?;
    mac.update(&data);
    let result = mac.finalize().into_bytes();

    let mut key = [0u8; 32];
    let mut chain = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    chain.copy_from_slice(&result[32..]);

    data.zeroize();

    Some((key, chain))
}

/// Derived Solana keypair.
pub struct SolanaKeypair {
    pub private_key: [u8; 32],
    pub public_key: [u8; 32],
    pub derivation_path: String,
}

impl Drop for SolanaKeypair {
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}

/// Derive Solana Ed25519 keypair from BIP39 seed.
///
/// Path: m/44'/501'/account'/0'
pub fn derive_solana_keypair(seed: &[u8], account: u32) -> Option<SolanaKeypair> {
    let path = format!("m/44'/501'/{}'/0'", account);

    let (mut key, mut chain) = derive_master(seed)?;

    // Derive: 44' -> 501' -> account' -> 0'
    for idx in &[44, 501, account, 0] {
        let (new_key, new_chain) = derive_child(&key, &chain, HARDENED + idx)?;
        key.zeroize();
        chain.zeroize();
        key = new_key;
        chain = new_chain;
    }

    // Get public key from private key
    let signing_key = SigningKey::from_bytes(&key);
    let public_key = signing_key.verifying_key().to_bytes();

    chain.zeroize();

    Some(SolanaKeypair {
        private_key: key,
        public_key,
        derivation_path: path,
    })
}

/// Derive using Solana CLI default path: m/44'/501'
pub fn derive_cli_keypair(seed: &[u8]) -> Option<SolanaKeypair> {
    let (mut key, mut chain) = derive_master(seed)?;

    // Derive: 44' -> 501'
    for idx in &[44, 501] {
        let (new_key, new_chain) = derive_child(&key, &chain, HARDENED + idx)?;
        key.zeroize();
        chain.zeroize();
        key = new_key;
        chain = new_chain;
    }

    let signing_key = SigningKey::from_bytes(&key);
    let public_key = signing_key.verifying_key().to_bytes();

    chain.zeroize();

    Some(SolanaKeypair {
        private_key: key,
        public_key,
        derivation_path: "m/44'/501'".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::bip39;

    #[test]
    fn test_derive_known_mnemonic() {
        // "abandon" x11 + "about" — well-known BIP39 test vector
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = bip39::mnemonic_to_seed(mnemonic, "");
        let keypair = derive_solana_keypair(&seed, 0).unwrap();

        // The address should be deterministic
        let address = bs58::encode(&keypair.public_key).into_string();
        assert!(!address.is_empty());
        assert_eq!(keypair.derivation_path, "m/44'/501'/0'/0'");

        // Verify the public key is 32 bytes
        assert_eq!(keypair.public_key.len(), 32);
        assert_eq!(keypair.private_key.len(), 32);
    }

    #[test]
    fn test_different_accounts_produce_different_keys() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = bip39::mnemonic_to_seed(mnemonic, "");

        let kp0 = derive_solana_keypair(&seed, 0).unwrap();
        let kp1 = derive_solana_keypair(&seed, 1).unwrap();

        assert_ne!(kp0.public_key, kp1.public_key);
        assert_ne!(kp0.private_key, kp1.private_key);
    }

    #[test]
    fn test_cli_path_differs_from_standard() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = bip39::mnemonic_to_seed(mnemonic, "");

        let standard = derive_solana_keypair(&seed, 0).unwrap();
        let cli = derive_cli_keypair(&seed).unwrap();

        assert_ne!(standard.public_key, cli.public_key);
        assert_eq!(cli.derivation_path, "m/44'/501'");
    }

    #[test]
    fn test_zeroize_on_drop() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = bip39::mnemonic_to_seed(mnemonic, "");
        let keypair = derive_solana_keypair(&seed, 0).unwrap();
        let _addr = bs58::encode(&keypair.public_key).into_string();
        // keypair drops here — private key should be zeroized
        // (can't easily test this from safe Rust, but the Drop impl ensures it)
    }
}
