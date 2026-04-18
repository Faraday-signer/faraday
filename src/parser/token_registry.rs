//! Offline token registry for known Solana tokens.
//!
//! Provides a hardcoded list of well-known tokens (mints, symbols, decimals) and
//! utilities for offline token identification via ATA derivation.
//!
//! No network access required — all data is embedded in the binary.

use std::collections::HashMap;
use crate::crypto::pda;

// ── Public types ─────────────────────────────────────────────────────────────

pub struct TokenInfo {
    pub symbol: &'static str,
    pub decimals: u8,
}

pub struct AtaEntry {
    pub mint: [u8; 32],
    pub symbol: &'static str,
    pub decimals: u8,
}

/// Map from ATA address → token info for offline mint resolution.
pub type AtaMap = HashMap<[u8; 32], AtaEntry>;

// ── Token lookup ─────────────────────────────────────────────────────────────

/// Looks up a mint address in the hardcoded token list.
pub fn lookup(mint: &[u8; 32]) -> Option<TokenInfo> {
    let addr = bs58::encode(mint).into_string();
    KNOWN_TOKENS
        .iter()
        .find(|(m, _, _)| *m == addr.as_str())
        .map(|(_, symbol, decimals)| TokenInfo {
            symbol,
            decimals: *decimals,
        })
}

// ── Amount formatting ────────────────────────────────────────────────────────

/// Formats a raw token amount with decimal scaling.
///
/// Trailing zeros after the decimal point are trimmed.
pub fn format_amount(amount: u64, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }
    let divisor = 10u64.pow(decimals as u32);
    let whole = amount / divisor;
    let frac = amount % divisor;
    if frac == 0 {
        return whole.to_string();
    }
    let frac_str = format!("{:0width$}", frac, width = decimals as usize);
    format!("{}.{}", whole, frac_str.trim_end_matches('0'))
}

// ── Offline ATA resolution ───────────────────────────────────────────────────

/// Builds a lookup table mapping `ATA address → (mint, symbol, decimals)`.
///
/// For every `(signer, known_mint)` pair, the canonical Associated Token Account
/// is derived using `find_program_address` — entirely offline.
pub fn build_ata_map(signers: &[[u8; 32]]) -> AtaMap {
    let ata_program = pubkey_from_b58(ATA_PROGRAM_ID);
    let token_prog = pubkey_from_b58(TOKEN_PROGRAM_ID);
    let mut map = AtaMap::new();

    for signer in signers {
        for &(mint_str, symbol, decimals) in KNOWN_TOKENS {
            let mint = pubkey_from_b58(mint_str);
            if let Some((ata, _bump)) = pda::find_program_address(
                &[signer.as_ref(), token_prog.as_ref(), mint.as_ref()],
                &ata_program,
            ) {
                map.insert(ata, AtaEntry { mint, symbol, decimals });
            }
        }
    }
    map
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn pubkey_from_b58(s: &str) -> [u8; 32] {
    let bytes = bs58::decode(s).into_vec().expect("invalid base58 in token registry");
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    key
}

// ── Constants ────────────────────────────────────────────────────────────────

const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ATA_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

const KNOWN_TOKENS: &[(&str, &str, u8)] = &[
    // Native / wrapped
    ("So11111111111111111111111111111111111111112", "SOL", 9),
    // Stablecoins
    ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC", 6),
    ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", "USDT", 6),
    ("USDH1SM1ojwWUga67PGrgFWUHibbjqMvuMaDkRJTgkX", "USDH", 6),
    ("UXPhBoR3qG4UCiGNJfV7MqhHyFqKN68g45GoYvAeL2M", "UXD", 9),
    // Liquid Staking Tokens
    ("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", "mSOL", 9),
    ("7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj", "stSOL", 9),
    ("bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1", "bSOL", 9),
    ("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", "JitoSOL", 9),
    ("jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v", "jupSOL", 9),
    ("he1iusmfkpAdwvxLNGV8Y1iSbj4rUy6yMhEA3fotn9A", "hSOL", 9),
    ("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Posu", "JTO", 9),
    // DeFi tokens
    ("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R", "RAY", 6),
    ("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE", "ORCA", 6),
    ("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", "JUP", 6),
    ("27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4", "JLP", 6),
    ("MNDEFzGvMt87ueuHvVU9VcTqsAP5b3fTGPsHuuPA5ey", "MNDE", 9),
    ("SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt", "SRM", 6),
    ("HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3", "PYTH", 6),
    ("EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm", "WIF", 6),
    ("85VBFQZC9TZkfaptBWjvUw7YbZjy52A6mjtPGjstQAmQ", "W", 6),
    // Memecoins
    ("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", "BONK", 5),
    ("7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr", "POPCAT", 9),
    ("ukHH6c7mMyiWCf1b9pnWe25TSpkDDt3H5pQZgZ74J82", "BOME", 6),
    ("ED5nyyWEzpPPiWimP8vYm7sD7TD3LAt3Q3gRTWHzc8yy", "MEW", 6),
    // Other notable tokens
    ("ATLASXmbPQxBUYbxPsV97usA3fPQYEqzQBUHgiFCUsXx", "ATLAS", 8),
    ("poLisWXnNRwC6oBu1vHiuKQzFjGL4XDSu4g9qjz9qVk", "POLIS", 8),
    ("StepAscQoEioFxxWGnh2sLBDFp9d8rvKz2Yp39iDpyT", "STEP", 9),
    ("kinXdEcpDQeHPEuQnqmUgtYykqKCSVY6PNsDMnVSZ9F", "KIN", 5),
    ("nosXBVoaCTtYdLvKY6Csb4AC8JCdQKKAaWYtx2ZMoo7", "NOS", 6),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_known_token() {
        let usdc_mint = pubkey_from_b58("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        let info = lookup(&usdc_mint).unwrap();
        assert_eq!(info.symbol, "USDC");
        assert_eq!(info.decimals, 6);
    }

    #[test]
    fn test_lookup_sol() {
        let sol_mint = pubkey_from_b58("So11111111111111111111111111111111111111112");
        let info = lookup(&sol_mint).unwrap();
        assert_eq!(info.symbol, "SOL");
        assert_eq!(info.decimals, 9);
    }

    #[test]
    fn test_lookup_unknown_returns_none() {
        let unknown = [0x42u8; 32];
        assert!(lookup(&unknown).is_none());
    }

    #[test]
    fn test_format_amount_no_decimals() {
        assert_eq!(format_amount(1000, 0), "1000");
    }

    #[test]
    fn test_format_amount_with_decimals() {
        assert_eq!(format_amount(1_000_000, 6), "1");
        assert_eq!(format_amount(1_500_000, 6), "1.5");
        assert_eq!(format_amount(1_000, 6), "0.001");
    }

    #[test]
    fn test_format_amount_trims_trailing_zeros() {
        assert_eq!(format_amount(1_100_000, 6), "1.1");
    }

    #[test]
    fn test_build_ata_map_produces_entries() {
        let signer = [1u8; 32];
        let map = build_ata_map(&[signer]);
        assert!(!map.is_empty());
        assert_eq!(map.len(), KNOWN_TOKENS.len());
    }

    #[test]
    fn test_ata_map_deterministic() {
        let signer = [1u8; 32];
        let a = build_ata_map(&[signer]);
        let b = build_ata_map(&[signer]);
        assert_eq!(a.len(), b.len());
        for (key, entry_a) in &a {
            let entry_b = b.get(key).unwrap();
            assert_eq!(entry_a.mint, entry_b.mint);
            assert_eq!(entry_a.symbol, entry_b.symbol);
        }
    }

    #[test]
    fn test_ata_entries_have_valid_mints() {
        let signer = [1u8; 32];
        let map = build_ata_map(&[signer]);
        for entry in map.values() {
            assert!(lookup(&entry.mint).is_some());
        }
    }
}
