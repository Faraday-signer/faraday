//! Offline token registry for known Solana tokens.
//!
//! Provides a hardcoded list of well-known tokens (mints, symbols, decimals) and
//! utilities for offline token identification via ATA derivation.
//!
//! No network access required — all data is embedded in the binary.

use crate::crypto::pda;
use std::collections::HashMap;

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

/// Hero-friendly variant of `format_amount`. Below 1,000,000 whole units
/// it returns the full-precision form (so normal dapp txs look exactly
/// like before). Above that, it collapses to `<mantissa>.<cc><suffix>`
/// with 2 decimal places and an SI suffix so huge balances like
/// "1234567890.123456 USDC" don't overflow the 240-px hero line.
///
/// Used in swap parsers where the rendered value ends up on the pinned
/// `@H2` hero row; plain SPL Token transfers still use `format_amount`
/// because their value goes into a scrollable detail row and full
/// precision is worth preserving there.
pub fn format_amount_short(amount: u64, decimals: u8) -> String {
    let full = format_amount(amount, decimals);
    let (whole_str, frac_str) = match full.split_once('.') {
        Some((w, f)) => (w, f),
        None => (full.as_str(), ""),
    };
    if whole_str.len() <= 6 {
        return full;
    }

    // Pick suffix by digit count of the whole part. u64 max fits in "Q".
    //   7–9   → M (million,     10^6)
    //   10–12 → B (billion,     10^9)
    //   13–15 → T (trillion,    10^12)
    //   16+   → Q (quadrillion, 10^15)
    let (suffix, shift) = match whole_str.len() {
        7..=9 => ("M", 6u32),
        10..=12 => ("B", 9),
        13..=15 => ("T", 12),
        _ => ("Q", 15),
    };

    // u128 so scaling a 20-digit whole part can't overflow. Two decimal
    // places of mantissa precision — enough to tell "1.23M" from "1.24M".
    let whole_val: u128 = whole_str.parse().unwrap_or(0);
    let scale = 10u128.pow(shift);
    let mantissa_whole = whole_val / scale;
    let mantissa_frac = (whole_val % scale) / 10u128.pow(shift - 2);
    // If the caller had subunit fractional digits they'd live past the
    // precision we keep; dropping them is fine because the mantissa
    // precision dominates at this scale. Keep `frac_str` only for the
    // sub-1M branch above.
    let _ = frac_str;
    format!("{}.{:02}{}", mantissa_whole, mantissa_frac, suffix)
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
                map.insert(
                    ata,
                    AtaEntry {
                        mint,
                        symbol,
                        decimals,
                    },
                );
            }
        }
    }
    map
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn pubkey_from_b58(s: &str) -> [u8; 32] {
    let bytes = bs58::decode(s)
        .into_vec()
        .expect("invalid base58 in token registry");
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
    fn test_format_amount_short_preserves_small_values() {
        // 6 or fewer whole digits → identical to format_amount.
        assert_eq!(format_amount_short(1_500_000, 6), "1.5");
        assert_eq!(format_amount_short(999_999_000_000, 6), "999999");
        assert_eq!(format_amount_short(123_456_789, 6), "123.456789");
    }

    #[test]
    fn test_format_amount_short_collapses_millions() {
        // 1M whole units (6 decimals) → 10^12 raw → 7 whole digits → "M"
        assert_eq!(format_amount_short(1_000_000_000_000, 6), "1.00M");
        // 1.23M USDC
        assert_eq!(format_amount_short(1_234_567_000_000, 6), "1.23M");
        // 999.99M USDC (just below the billion threshold)
        assert_eq!(format_amount_short(999_990_000_000_000, 6), "999.99M");
    }

    #[test]
    fn test_format_amount_short_collapses_billions_and_up() {
        // 1B USDC (6 decimals): whole = 1e9 → 10 digits → "B"
        assert_eq!(format_amount_short(1_000_000_000_000_000, 6), "1.00B");
        // 1.5T USDC: whole = 1.5e12 → 13 digits → "T"
        // Raw = 1.5e12 * 1e6 = 1.5e18 (fits in u64, max ~1.8e19)
        assert_eq!(format_amount_short(1_500_000_000_000_000_000, 6), "1.50T");
    }

    #[test]
    fn test_format_amount_short_handles_zero_decimals() {
        // NFTs / integer-only tokens: same thresholds apply to the raw count.
        assert_eq!(format_amount_short(1_000, 0), "1000");
        assert_eq!(format_amount_short(1_000_000, 0), "1.00M");
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
