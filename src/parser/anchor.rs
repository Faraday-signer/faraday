//! Anchor framework utilities.
//!
//! Anchor programs use `sha256("global:{instruction_name}")[..8]` as their 8-byte
//! instruction discriminator. This module computes discriminators offline for
//! matching against transaction instruction data.

use sha2::{Digest, Sha256};

/// Computes the 8-byte Anchor instruction discriminator for `name`.
///
/// Formula: `sha256("global:{name}")[..8]`
pub fn discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{name}");
    let hash = Sha256::digest(preimage.as_bytes());
    // Safe: SHA256 always produces 32 bytes, slicing [..8] always fits [u8; 8].
    hash[..8].try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        assert_eq!(discriminator("route"), discriminator("route"));
    }

    #[test]
    fn test_different_names_differ() {
        assert_ne!(discriminator("route"), discriminator("shared_accounts_route"));
    }

    #[test]
    fn test_matches_manual_sha256() {
        let hash = Sha256::digest(b"global:route");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(discriminator("route"), expected);
    }
}
