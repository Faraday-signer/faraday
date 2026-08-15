//! QR predictability audit (FA-25 investigation).
//!
//! Drives the real firmware pipeline — entropy → BIP39 mnemonic →
//! CompactSeedQR bytes → QR matrix — many times and tests statistically
//! that QR codes from independent wallets share nothing beyond the QR
//! standard's fixed structure. Written after a user report of "visible
//! patterns between QRs"; kept as a regression suite because any future
//! regression here (entropy reuse, truncation, encode bug) would show up
//! as these distributions collapsing. Thresholds are deliberately loose
//! so the suite never flakes on honest randomness.

use faraday_core::crypto::bip39;
use faraday_core::qr::encode_qr::{encode_compact_seed_qr, generate_qr_matrix, QrEcLevel};

/// Deterministic xorshift so the audit itself is reproducible.
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            let n = chunk.len();
            chunk.copy_from_slice(&v[..n]);
        }
    }
}

/// entropy → mnemonic → CompactSeedQR → QR matrix, via the real code.
/// `mnemonic_from_entropy` SHA256-whitens its input (documented), so the
/// mnemonic's underlying entropy — and therefore the backup QR — is
/// sha256(input) truncated. The round-trip that matters for backups:
/// the QR bytes must equal exactly the entropy the words encode.
fn qr_for_entropy(entropy: &[u8], words: usize) -> (Vec<bool>, usize) {
    use sha2::{Digest, Sha256};
    let mnemonic = bip39::mnemonic_from_entropy(entropy, words).expect("mnemonic");
    let compact = encode_compact_seed_qr(&mnemonic).expect("compact");
    let expected = &Sha256::digest(entropy)[..compact.len()];
    assert_eq!(
        compact, expected,
        "CompactSeedQR must equal the whitened entropy the words encode"
    );
    generate_qr_matrix(&compact, QrEcLevel::L).expect("qr")
}

/// Split modules into structural (identical across all samples) and
/// variable. The variable set is where wallet data lives; structural
/// modules (finders, timing, dark module…) are *expected* to repeat and
/// are what makes any two QRs look like siblings to the eye.
fn variable_modules(matrices: &[Vec<bool>]) -> Vec<usize> {
    let len = matrices[0].len();
    (0..len)
        .filter(|&i| {
            let first = matrices[0][i];
            matrices.iter().any(|m| m[i] != first)
        })
        .collect()
}

fn audit(words: usize, ent_bytes: usize, expect_size: usize) {
    const N: usize = 400;
    let mut rng = Rng(0x5EED_5EED_5EED_5EED);

    let mut matrices = Vec::with_capacity(N);
    for _ in 0..N {
        let mut e = vec![0u8; ent_bytes];
        rng.fill(&mut e);
        let (m, size) = qr_for_entropy(&e, words);
        assert_eq!(size, expect_size, "unexpected QR version for {words}w");
        matrices.push(m);
    }

    let var = variable_modules(&matrices);
    let total = expect_size * expect_size;
    // Sanity: a healthy pipeline leaves a large fraction of modules
    // data-driven. If almost everything is constant, entropy is dead.
    assert!(
        var.len() * 2 > total,
        "{words}w: only {}/{} modules vary across {N} wallets — entropy collapse",
        var.len(),
        total
    );

    // Pairwise agreement over variable modules: independent wallets must
    // agree on ~50% of data-driven modules. Systematic correlation would
    // push the mean or individual pairs far from 0.5.
    let mut pair_rng = Rng(0xA0D1_7A0D_17A0_D17A);
    let mut mean_acc = 0.0f64;
    let (mut min_a, mut max_a) = (1.0f64, 0.0f64);
    const PAIRS: usize = 2000;
    for _ in 0..PAIRS {
        let i = (pair_rng.next_u64() as usize) % N;
        let mut j = (pair_rng.next_u64() as usize) % N;
        if i == j {
            j = (j + 1) % N;
        }
        let agree = var
            .iter()
            .filter(|&&k| matrices[i][k] == matrices[j][k])
            .count() as f64
            / var.len() as f64;
        mean_acc += agree;
        min_a = min_a.min(agree);
        max_a = max_a.max(agree);
    }
    let mean = mean_acc / PAIRS as f64;
    println!(
        "{words}w: {}x{} QR, {}/{} variable modules, pairwise agreement mean {mean:.4} (min {min_a:.3}, max {max_a:.3})",
        expect_size, expect_size, var.len(), total
    );
    assert!(
        (0.45..=0.55).contains(&mean),
        "{words}w: mean pairwise agreement {mean:.4} deviates from 0.5 — QRs are correlated"
    );
    // No single pair of independent wallets may be near-identical or
    // near-inverse in the data region.
    assert!(
        min_a > 0.25 && max_a < 0.75,
        "{words}w: an individual pair agrees at {min_a:.3}/{max_a:.3} — investigate those seeds"
    );
}

#[test]
fn seed_qrs_of_independent_wallets_are_uncorrelated_12w() {
    audit(12, 16, 21);
}

#[test]
fn seed_qrs_of_independent_wallets_are_uncorrelated_24w() {
    audit(24, 32, 25);
}

/// Avalanche: flipping ONE bit of entropy must rewrite roughly half of
/// the variable QR modules (BIP39 wordlist mapping is bijective on the
/// entropy, and the QR's ECC codewords amplify local changes).
#[test]
fn one_bit_entropy_flip_avalanches_through_the_qr() {
    let mut rng = Rng(0xF11B_D00D_0000_0001u64 ^ 0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..50 {
        let mut e = [0u8; 16];
        rng.fill(&mut e);
        let (a, size) = qr_for_entropy(&e, 12);
        let bit = (rng.next_u64() % 128) as usize;
        e[bit / 8] ^= 1 << (bit % 8);
        let (b, _) = qr_for_entropy(&e, 12);
        let diff = a.iter().zip(b.iter()).filter(|(x, y)| x != y).count();
        let frac = diff as f64 / (size * size) as f64;
        // Function patterns (~38% of a V1 QR) never change; of the rest,
        // expect wide rewrites. Loose floor: at least 15% of ALL modules.
        assert!(
            frac > 0.15,
            "single-bit flip changed only {frac:.3} of modules — no avalanche"
        );
    }
}

/// Same entropy must always produce the identical QR (a re-export of the
/// same wallet repeating exactly is correct behavior, not a finding).
#[test]
fn same_wallet_reexport_is_deterministic() {
    let e = [0x42u8; 16];
    let (a, _) = qr_for_entropy(&e, 12);
    let (b, _) = qr_for_entropy(&e, 12);
    assert_eq!(a, b);
}

/// The create-flow mixing replica: OS RNG XOR frame-hash XOR nanos, as
/// in flows/create.rs. Byte-frequency chi-square over many runs — a
/// grossly biased mix (e.g. an XOR partner accidentally zeroing bytes)
/// would blow this up.
#[test]
fn create_flow_mixing_replica_has_uniform_bytes() {
    use sha2::{Digest, Sha256};
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    let mut counts = [0u64; 256];
    const RUNS: usize = 2000;
    for run in 0..RUNS {
        // Replica of one capture: 16 OS-RNG bytes (xorshift stands in),
        // XOR sha256(frame), XOR nanosecond counter in the first 4.
        let mut frame = vec![0u8; 512];
        rng.fill(&mut frame);
        let mut e = [0u8; 16];
        rng.fill(&mut e);
        let digest = Sha256::digest(&frame);
        for (b, d) in e.iter_mut().zip(digest.iter()) {
            *b ^= d;
        }
        let nanos = ((run as u32).wrapping_mul(997)).to_le_bytes();
        for (i, n) in nanos.iter().enumerate() {
            e[i] ^= n;
        }
        for b in e {
            counts[b as usize] += 1;
        }
    }
    let expected = (RUNS * 16) as f64 / 256.0;
    let chi2: f64 = counts
        .iter()
        .map(|&c| {
            let d = c as f64 - expected;
            d * d / expected
        })
        .sum();
    // 255 dof; > 400 would be wildly non-uniform (p ~ 1e-9 territory).
    println!("mixing chi-square: {chi2:.1} (255 dof, expected ≈ 255)");
    assert!(chi2 < 400.0, "byte distribution non-uniform: chi2 {chi2:.1}");
}
