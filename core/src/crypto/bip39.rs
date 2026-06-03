//! BIP39 mnemonic implementation.
//!
//! Seed derivation uses PBKDF2-HMAC-SHA512 (2048 rounds). On ESP32-S3 with the
//! `hardware-sha512` feature, this is handled by ESP-IDF's mbedtls using the
//! hardware SHA accelerator. Otherwise it falls back to pure-Rust sha2/pbkdf2.
//! The BIP39 wordlist is embedded at compile time.

#[cfg(not(feature = "hardware-sha512"))]
use hmac::Hmac;
#[cfg(not(feature = "hardware-sha512"))]
use pbkdf2::pbkdf2;
use sha2::{Digest, Sha256};
#[cfg(not(feature = "hardware-sha512"))]
use sha2::Sha512;
use zeroize::{Zeroize, Zeroizing};

/// BIP39 English wordlist, fetched from bitcoin/bips at build time and SHA256-verified.
/// Source: https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt
const WORDLIST_RAW: &str = include_str!(concat!(env!("OUT_DIR"), "/bip39_english.txt"));

/// Get the BIP39 wordlist as a Vec of &str.
fn wordlist() -> Vec<&'static str> {
    WORDLIST_RAW
        .lines()
        .filter(|l| !l.is_empty())
        .collect()
}

/// Get BIP39 word at index (0-2047).
pub fn get_word(index: usize) -> Option<&'static str> {
    wordlist().get(index).copied()
}

/// Get index of a BIP39 word.
pub fn word_index(word: &str) -> Option<usize> {
    wordlist().iter().position(|&w| w == word)
}

/// Get all BIP39 words matching a prefix.
pub fn words_with_prefix(prefix: &str) -> Vec<(usize, &'static str)> {
    wordlist()
        .iter()
        .enumerate()
        .filter(|(_, w)| w.starts_with(prefix))
        .map(|(i, w)| (i, *w))
        .collect()
}


/// Generate a BIP39 mnemonic from raw entropy.
///
/// Entropy is SHA256-hashed, then truncated to 16 bytes (12 words) or 32 bytes (24 words).
pub fn mnemonic_from_entropy(entropy: &[u8], word_count: usize) -> Result<String, &'static str> {
    let mut hashed = Sha256::digest(entropy);

    let ent_bytes: &[u8] = match word_count {
        12 => &hashed[..16], // 128 bits
        24 => &hashed[..32], // 256 bits
        _ => return Err("word_count must be 12 or 24"),
    };

    let mnemonic = entropy_to_mnemonic(ent_bytes);
    hashed.as_mut_slice().zeroize();
    Ok(mnemonic)
}

/// Generate a BIP39 mnemonic from exact raw entropy (16 bytes = 12 words, 32 bytes = 24 words).
///
/// Unlike `mnemonic_from_entropy`, this uses the bytes directly without hashing.
/// Use this for coin flips or other true random sources with exact bit counts.
pub fn mnemonic_from_raw_entropy(ent_bytes: &[u8]) -> Result<String, &'static str> {
    match ent_bytes.len() {
        16 | 32 => Ok(entropy_to_mnemonic(ent_bytes)),
        _ => Err("entropy must be 16 bytes (12 words) or 32 bytes (24 words)"),
    }
}

/// Convert entropy bytes directly to mnemonic (BIP39 spec).
fn entropy_to_mnemonic(ent_bytes: &[u8]) -> String {
    let ent_bits = ent_bytes.len() * 8;
    let cs_bits = ent_bits / 32;

    // Checksum = first (ent_bits/32) bits of SHA256(entropy)
    let mut checksum = Sha256::digest(ent_bytes);

    // Build bitstring: entropy + checksum
    let mut bits = Vec::with_capacity(ent_bits + cs_bits);
    for byte in ent_bytes {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    for i in (0..cs_bits).rev() {
        let byte_idx = (cs_bits - 1 - i) / 8;
        let bit_idx = 7 - ((cs_bits - 1 - i) % 8);
        bits.push((checksum[byte_idx] >> bit_idx) & 1);
    }

    checksum.as_mut_slice().zeroize();

    // Split into 11-bit groups -> word indices
    let wl = wordlist();
    let mut words = Vec::new();
    for chunk in bits.chunks(11) {
        let mut idx: usize = 0;
        for &bit in chunk {
            idx = (idx << 1) | (bit as usize);
        }
        words.push(wl[idx]);
    }

    bits.zeroize();

    words.join(" ")
}

/// Validate a BIP39 mnemonic (check words and checksum).
pub fn validate_mnemonic(mnemonic: &str) -> bool {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();

    if words.len() != 12 && words.len() != 24 {
        return false;
    }

    let wl = wordlist();

    // Convert words to indices
    let indices: Vec<usize> = match words
        .iter()
        .map(|w| wl.iter().position(|&wl_word| wl_word == *w))
        .collect()
    {
        Some(indices) => indices,
        None => return false,
    };

    // Reconstruct bitstring
    let total_bits = words.len() * 11;
    let mut bits = Vec::with_capacity(total_bits);
    for &idx in &indices {
        for i in (0..11).rev() {
            bits.push(((idx >> i) & 1) as u8);
        }
    }

    // Split entropy and checksum
    let cs_bits = total_bits / 33;
    let ent_bits = total_bits - cs_bits;

    // Reconstruct entropy bytes
    let mut ent_bytes = vec![0u8; ent_bits / 8];
    for i in 0..ent_bits {
        ent_bytes[i / 8] |= bits[i] << (7 - (i % 8));
    }

    // Verify checksum
    let checksum = Sha256::digest(&ent_bytes);
    for i in 0..cs_bits {
        let expected = (checksum[i / 8] >> (7 - (i % 8))) & 1;
        if bits[ent_bits + i] != expected {
            return false;
        }
    }

    true
}

/// BIP39 seed generation: PBKDF2-HMAC-SHA512, 2048 rounds.
///
/// Returns a 64-byte seed. The seed is zeroized on drop.
/// Callers must borrow (`&seed`), not clone — copying the inner Vec
/// produces a plain buffer that won't be zeroized.
#[cfg(not(feature = "hardware-sha512"))]
pub fn mnemonic_to_seed(mnemonic: &str, passphrase: &str) -> Zeroizing<Vec<u8>> {
    let salt = format!("mnemonic{}", passphrase);
    let mut seed = Zeroizing::new(vec![0u8; 64]);

    pbkdf2::<Hmac<Sha512>>(
        mnemonic.as_bytes(),
        salt.as_bytes(),
        2048,
        &mut seed,
    )
    .expect("PBKDF2 should not fail");

    seed
}

/// BIP39 seed generation using ESP32-S3 hardware SHA512 accelerator via mbedtls.
///
/// `mbedtls_pkcs5_pbkdf2_hmac_ext` is not included in esp-idf-sys's generated
/// bindings, so we declare it manually. The symbol is present in the linked
/// ESP-IDF libraries.
#[cfg(feature = "hardware-sha512")]
pub fn mnemonic_to_seed(mnemonic: &str, passphrase: &str) -> Zeroizing<Vec<u8>> {
    extern "C" {
        fn mbedtls_pkcs5_pbkdf2_hmac_ext(
            md_type: u32,
            password: *const u8,
            plen: usize,
            salt: *const u8,
            slen: usize,
            iteration_count: u32,
            key_length: u32,
            output: *mut u8,
        ) -> i32;
    }

    const MBEDTLS_MD_SHA512: u32 = 11;

    let salt = format!("mnemonic{}", passphrase);
    let mut seed = Zeroizing::new(vec![0u8; 64]);

    let ret = unsafe {
        mbedtls_pkcs5_pbkdf2_hmac_ext(
            MBEDTLS_MD_SHA512,
            mnemonic.as_bytes().as_ptr(),
            mnemonic.len(),
            salt.as_bytes().as_ptr(),
            salt.len(),
            2048,
            64,
            seed.as_mut_ptr(),
        )
    };

    assert_eq!(ret, 0, "mbedtls PBKDF2 failed: {}", ret);

    seed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wordlist_length() {
        assert_eq!(wordlist().len(), 2048);
    }

    #[test]
    fn test_get_word() {
        assert_eq!(get_word(0), Some("abandon"));
        assert_eq!(get_word(2047), Some("zoo"));
        assert_eq!(get_word(2048), None);
    }

    #[test]
    fn test_word_index() {
        assert_eq!(word_index("abandon"), Some(0));
        assert_eq!(word_index("zoo"), Some(2047));
        assert_eq!(word_index("notaword"), None);
    }

    // BIP39 test vector (from the spec)
    #[test]
    fn test_validate_known_mnemonic() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        assert!(validate_mnemonic(mnemonic));
    }

    #[test]
    fn test_validate_bad_checksum() {
        // Last word changed — checksum should fail
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon zoo";
        assert!(!validate_mnemonic(mnemonic));
    }

    #[test]
    fn test_mnemonic_to_seed_vector() {
        // BIP39 test vector
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = mnemonic_to_seed(mnemonic, "");
        assert_eq!(seed.len(), 64);
        // Known seed hex for this mnemonic (no passphrase)
        assert_eq!(
            hex::encode(&seed[..4]),
            "5eb00bbd"
        );
    }

    #[test]
    fn test_roundtrip_24_words() {
        let mnemonic = mnemonic_from_entropy(b"test entropy source data for roundtrip", 24).unwrap();
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 24);
        assert!(validate_mnemonic(&mnemonic));
    }

    #[test]
    fn test_roundtrip_12_words() {
        let mnemonic = mnemonic_from_entropy(b"test entropy source", 12).unwrap();
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12);
        assert!(validate_mnemonic(&mnemonic));
    }
}
