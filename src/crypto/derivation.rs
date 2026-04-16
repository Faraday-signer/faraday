//! High-level Solana keypair derivation from BIP39 mnemonics.
//!
//! Combines bip39 (mnemonic → seed) and slip0010 (seed → keypair).

use crate::crypto::bip39;
use crate::crypto::slip0010::{self, SolanaKeypair};

/// Derive a Solana keypair from a BIP39 mnemonic.
///
/// Uses the standard wallet path: m/44'/501'/{account}'/0'
pub fn derive_keypair(mnemonic: &str, passphrase: &str, account: u32) -> SolanaKeypair {
    let seed = bip39::mnemonic_to_seed(mnemonic, passphrase);
    slip0010::derive_solana_keypair(&seed, account)
}

/// Derive using the Solana CLI default path: m/44'/501'
pub fn derive_keypair_cli_path(mnemonic: &str, passphrase: &str) -> SolanaKeypair {
    let seed = bip39::mnemonic_to_seed(mnemonic, passphrase);
    slip0010::derive_cli_keypair(&seed)
}

/// Derive multiple account keypairs from a single mnemonic.
pub fn derive_multiple_accounts(mnemonic: &str, passphrase: &str, count: u32) -> Vec<SolanaKeypair> {
    let seed = bip39::mnemonic_to_seed(mnemonic, passphrase);
    (0..count)
        .map(|i| slip0010::derive_solana_keypair(&seed, i))
        .collect()
}

/// Get the base58 Solana address from a keypair.
pub fn address(keypair: &SolanaKeypair) -> String {
    bs58::encode(&keypair.public_key).into_string()
}
