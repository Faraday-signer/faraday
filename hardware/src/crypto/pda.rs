//! Solana Program Derived Address (PDA) derivation.
//!
//! PDAs are off-curve points computed deterministically from seeds and a program ID.
//! This module implements the derivation entirely offline using SHA-256 and the
//! ed25519 curve check — no Solana SDK or network access required.

use sha2::{Digest, Sha256};

/// Finds the first valid PDA for the given seeds and program ID.
///
/// Iterates bump seeds from 255 down to 0, returning the first `(address, bump)`
/// pair where the SHA-256 hash does not land on the ed25519 curve.
pub fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> Option<([u8; 32], u8)> {
    for bump in (0..=255u8).rev() {
        if let Some(addr) = create_program_address(seeds, program_id, bump) {
            return Some((addr, bump));
        }
    }
    None
}

fn create_program_address(seeds: &[&[u8]], program_id: &[u8; 32], bump: u8) -> Option<[u8; 32]> {
    let mut hasher = Sha256::new();
    for seed in seeds {
        hasher.update(seed);
    }
    hasher.update([bump]);
    hasher.update(program_id);
    hasher.update(b"ProgramDerivedAddress");
    let hash = hasher.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);

    if is_on_curve(&result) {
        return None;
    }
    Some(result)
}

fn is_on_curve(bytes: &[u8; 32]) -> bool {
    ed25519_dalek::VerifyingKey::from_bytes(bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_program_address_returns_some() {
        let program_id = [1u8; 32];
        let result = find_program_address(&[b"test"], &program_id);
        assert!(result.is_some());
    }

    #[test]
    fn test_result_is_not_on_curve() {
        let program_id = [1u8; 32];
        let (addr, _bump) = find_program_address(&[b"test"], &program_id).unwrap();
        assert!(!is_on_curve(&addr));
    }

    #[test]
    fn test_deterministic() {
        let program_id = [1u8; 32];
        let a = find_program_address(&[b"seed"], &program_id);
        let b = find_program_address(&[b"seed"], &program_id);
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_seeds_different_addresses() {
        let program_id = [1u8; 32];
        let a = find_program_address(&[b"one"], &program_id).unwrap();
        let b = find_program_address(&[b"two"], &program_id).unwrap();
        assert_ne!(a.0, b.0);
    }

    #[test]
    fn test_multiple_seeds() {
        let program_id = [1u8; 32];
        let result = find_program_address(&[b"a", b"b", b"c"], &program_id);
        assert!(result.is_some());
    }

}
