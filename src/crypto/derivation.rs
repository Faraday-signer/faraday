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

/// Derive multiple account keypairs from a single mnemonic.
pub fn derive_multiple_accounts(mnemonic: &str, passphrase: &str, count: u32) -> Vec<SolanaKeypair> {
    let seed = bip39::mnemonic_to_seed(mnemonic, passphrase);
    (0..count)
        .filter_map(|i| slip0010::derive_solana_keypair(&seed, i))
        .collect()
}

/// Get the base58 Solana address from a keypair.
pub fn address(keypair: &SolanaKeypair) -> String {
    bs58::encode(&keypair.public_key).into_string()
}

/// Which path, if any, of the loaded seed produced a given address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressMatch {
    /// Matched the standard Phantom/Solflare path `m/44'/501'/N'/0'`.
    /// This is the one path every modern Solana wallet uses.
    Standard { account: u32 },
    /// Address looked valid (base58 → 32 bytes) but wasn't derivable from this seed.
    NotFound,
    /// Input wasn't a valid Solana address at all — e.g. a URL, text, or
    /// wrong-length base58. We return this instead of NotFound so the UI
    /// can say "not a Solana address" rather than the scarier "NOT YOURS".
    InvalidFormat,
}

impl AddressMatch {
    pub fn is_match(&self) -> bool {
        matches!(self, AddressMatch::Standard { .. })
    }

    /// Human-readable derivation path for display ("—" when no match).
    pub fn path_str(&self) -> String {
        match self {
            AddressMatch::Standard { account } => format!("m/44'/501'/{}'/0'", account),
            AddressMatch::NotFound | AddressMatch::InvalidFormat => "—".to_string(),
        }
    }
}

/// True if `s` decodes from base58 into exactly 32 bytes — a valid Solana
/// Ed25519 pubkey shape. We don't attempt on-curve checks here.
pub fn is_valid_solana_address(s: &str) -> bool {
    matches!(bs58::decode(s).into_vec(), Ok(bytes) if bytes.len() == 32)
}

/// Strip common URI wrappers (e.g. `solana:<addr>?amount=1`) and return
/// just the trimmed address portion. Unrecognized inputs are returned as-is.
pub fn normalize_address_input(s: &str) -> String {
    let trimmed = s.trim();
    let without_scheme = trimmed.strip_prefix("solana:").unwrap_or(trimmed);
    // Drop any query string or fragment.
    let end = without_scheme
        .find(|c| c == '?' || c == '&' || c == '#')
        .unwrap_or(without_scheme.len());
    without_scheme[..end].trim().to_string()
}

/// Verify whether a given base58 Solana address can be derived from this
/// mnemonic + passphrase. Searches the first `max_accounts` standard
/// accounts and the legacy CLI path. Returns the matching path, or
/// `NotFound`.
///
/// The search is exhaustive across the checked paths — we do NOT short-circuit
/// on string-prefix matches, so vanity-address attacks that share a prefix
/// with a user's real address do not cause a false positive.
pub fn verify_address(
    mnemonic: &str,
    passphrase: &str,
    address: &str,
    max_accounts: u32,
) -> AddressMatch {
    // Reject anything that isn't a valid 32-byte base58 pubkey up front, so
    // URL/text/garbage QRs render as "Invalid" rather than "NOT YOURS" — that
    // distinction matters: "NOT YOURS" on a URL would scare the user for no
    // reason.
    if !is_valid_solana_address(address) {
        return AddressMatch::InvalidFormat;
    }
    let seed = bip39::mnemonic_to_seed(mnemonic, passphrase);
    for i in 0..max_accounts {
        if let Some(kp) = slip0010::derive_solana_keypair(&seed, i) {
            if address_eq(&kp, address) {
                return AddressMatch::Standard { account: i };
            }
        }
    }
    AddressMatch::NotFound
}

fn address_eq(kp: &SolanaKeypair, expected: &str) -> bool {
    bs58::encode(&kp.public_key).into_string() == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known BIP39 test vector.
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn addr_at_account(account: u32) -> String {
        address(&derive_keypair(TEST_MNEMONIC, "", account).unwrap())
    }

    #[test]
    fn matches_standard_account_zero() {
        let addr = addr_at_account(0);
        let m = verify_address(TEST_MNEMONIC, "", &addr, 5);
        assert_eq!(m, AddressMatch::Standard { account: 0 });
        assert!(m.is_match());
        assert_eq!(m.path_str(), "m/44'/501'/0'/0'");
    }

    #[test]
    fn matches_standard_account_three() {
        let addr = addr_at_account(3);
        let m = verify_address(TEST_MNEMONIC, "", &addr, 10);
        assert_eq!(m, AddressMatch::Standard { account: 3 });
    }

    #[test]
    fn does_not_match_account_beyond_search_range() {
        // If the address is at account 5 but we only search 0..3, miss it.
        let addr = addr_at_account(5);
        let m = verify_address(TEST_MNEMONIC, "", &addr, 3);
        assert_eq!(m, AddressMatch::NotFound);
    }

    #[test]
    fn rejects_address_from_different_seed() {
        // An address derived from a different mnemonic should not match.
        let other = "legal winner thank year wave sausage worth useful legal winner thank yellow";
        let foreign = address(&derive_keypair(other, "", 0).unwrap());
        let m = verify_address(TEST_MNEMONIC, "", &foreign, 10);
        assert_eq!(m, AddressMatch::NotFound);
    }

    #[test]
    fn valid_base58_but_not_derived_returns_not_found() {
        // 11111111111111111111111111111112 IS a valid 32-byte base58 address
        // (all zeros + 0x01) — so it should return NotFound, not InvalidFormat.
        let m = verify_address(TEST_MNEMONIC, "", "11111111111111111111111111111112", 10);
        assert_eq!(m, AddressMatch::NotFound);
    }

    #[test]
    fn empty_input_is_invalid_format() {
        let m = verify_address(TEST_MNEMONIC, "", "", 10);
        assert_eq!(m, AddressMatch::InvalidFormat);
        assert!(!m.is_match());
    }

    #[test]
    fn url_is_invalid_format_not_not_yours() {
        // This is what the user saw when scanning a QR that encoded a URL.
        let m = verify_address(TEST_MNEMONIC, "", "https://qr.codes/7jLtm6", 10);
        assert_eq!(m, AddressMatch::InvalidFormat);
    }

    #[test]
    fn random_text_is_invalid_format() {
        let m = verify_address(TEST_MNEMONIC, "", "just some text", 10);
        assert_eq!(m, AddressMatch::InvalidFormat);
    }

    #[test]
    fn base58_with_wrong_length_is_invalid_format() {
        // Valid base58 chars but short — decodes to <32 bytes.
        let m = verify_address(TEST_MNEMONIC, "", "abcdef", 10);
        assert_eq!(m, AddressMatch::InvalidFormat);
    }

    #[test]
    fn non_base58_chars_are_invalid_format() {
        // Base58 excludes 0, O, I, l to avoid ambiguity.
        let m = verify_address(TEST_MNEMONIC, "", "0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl", 10);
        assert_eq!(m, AddressMatch::InvalidFormat);
    }

    #[test]
    fn is_valid_solana_address_accepts_canonical_addresses() {
        assert!(is_valid_solana_address(&addr_at_account(0)));
        assert!(is_valid_solana_address("11111111111111111111111111111111")); // system program, 32 zeros
    }

    #[test]
    fn is_valid_solana_address_rejects_garbage() {
        assert!(!is_valid_solana_address(""));
        assert!(!is_valid_solana_address("not-an-address"));
        assert!(!is_valid_solana_address("https://example.com"));
        assert!(!is_valid_solana_address("abcdef")); // too short
    }

    #[test]
    fn normalize_strips_solana_uri_scheme() {
        let addr = addr_at_account(0);
        assert_eq!(normalize_address_input(&addr), addr);
        assert_eq!(normalize_address_input(&format!("solana:{}", addr)), addr);
        assert_eq!(
            normalize_address_input(&format!("solana:{}?amount=1", addr)),
            addr
        );
        assert_eq!(
            normalize_address_input(&format!("solana:{}&ref=xyz", addr)),
            addr
        );
    }

    #[test]
    fn normalize_trims_whitespace() {
        let addr = addr_at_account(0);
        assert_eq!(normalize_address_input(&format!("  {}  ", addr)), addr);
        assert_eq!(normalize_address_input(&format!("\n{}\t", addr)), addr);
    }

    #[test]
    fn normalize_leaves_unknown_schemes_alone() {
        assert_eq!(normalize_address_input("https://qr.codes/xyz"), "https://qr.codes/xyz");
    }

    #[test]
    fn passphrase_changes_derivation() {
        // Same mnemonic with vs without a passphrase should produce different
        // addresses — so verifying across the wrong passphrase must not match.
        let no_pass = address(&derive_keypair(TEST_MNEMONIC, "", 0).unwrap());
        let with_pass = address(&derive_keypair(TEST_MNEMONIC, "correct", 0).unwrap());
        assert_ne!(no_pass, with_pass);

        let m = verify_address(TEST_MNEMONIC, "wrong", &no_pass, 3);
        assert_eq!(m, AddressMatch::NotFound);

        let m2 = verify_address(TEST_MNEMONIC, "correct", &with_pass, 3);
        assert_eq!(m2, AddressMatch::Standard { account: 0 });
    }

    #[test]
    fn zero_max_accounts_never_matches() {
        // Degenerate: if caller searches no accounts, even the user's own
        // address should come back as NotFound (but still valid format).
        assert_eq!(
            verify_address(TEST_MNEMONIC, "", &addr_at_account(0), 0),
            AddressMatch::NotFound
        );
    }

    #[test]
    fn prefix_collision_does_not_false_match() {
        // Construct a fake address that shares a long prefix with the real one
        // but differs in the middle — verify must not match on prefix alone.
        let real = addr_at_account(0);
        assert!(real.len() > 10);
        // Flip a char in the middle that is NOT a base58 digit collision with itself.
        let mut fake: Vec<char> = real.chars().collect();
        let mid = fake.len() / 2;
        fake[mid] = if fake[mid] == 'x' { 'y' } else { 'x' };
        let fake_addr: String = fake.into_iter().collect();
        assert_ne!(real, fake_addr);
        assert_eq!(
            verify_address(TEST_MNEMONIC, "", &fake_addr, 10),
            AddressMatch::NotFound
        );
    }
}
