//! High-level Solana keypair derivation from BIP39 mnemonics.
//!
//! Combines bip39 (mnemonic → seed) and slip0010 (seed → keypair).

use crate::crypto::bip39;
use crate::crypto::slip0010::{self, SolanaKeypair};

/// Derive a Solana keypair from a BIP39 mnemonic.
///
/// Uses the standard wallet path: m/44'/501'/{account}'/0'
pub fn derive_keypair(mnemonic: &str, passphrase: &str, account: u32) -> Option<SolanaKeypair> {
    let seed = bip39::mnemonic_to_seed(mnemonic, passphrase);
    slip0010::derive_solana_keypair(&seed, account)
}

/// Get the base58 Solana address from a keypair.
pub fn address(keypair: &SolanaKeypair) -> String {
    bs58::encode(&keypair.public_key).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known BIP39 test vector.
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn derives_account_zero() {
        let kp = derive_keypair(TEST_MNEMONIC, "", 0).unwrap();
        // Address should be a 32-byte base58 pubkey.
        let addr = address(&kp);
        let bytes = bs58::decode(&addr).into_vec().unwrap();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn passphrase_changes_derivation() {
        let no_pass = address(&derive_keypair(TEST_MNEMONIC, "", 0).unwrap());
        let with_pass = address(&derive_keypair(TEST_MNEMONIC, "correct", 0).unwrap());
        assert_ne!(no_pass, with_pass);
    }

    #[test]
    fn different_accounts_give_different_addresses() {
        let a0 = address(&derive_keypair(TEST_MNEMONIC, "", 0).unwrap());
        let a1 = address(&derive_keypair(TEST_MNEMONIC, "", 1).unwrap());
        assert_ne!(a0, a1);
    }
}
