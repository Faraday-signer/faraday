//! Boot-time RNG liveness checks — the statistical half, extracted from the
//! ESP32 boot path so it can be tested on the host (the firmware build
//! compiles but never executes in CI, so untested logic there has never
//! once been observed to fail).
//!
//! Honest scope: these tests catch a **dead, stuck, cycling, or grossly
//! biased** generator. They cannot certify entropy — `esp_random()` is
//! *conditioned* output, which stays statistically plausible even when the
//! true entropy source silently fails to engage (the Coldcard-shaped
//! failure). Closing that gap needs a raw-noise-source test upstream of the
//! conditioner, which this hardware does not expose. Defense in depth for
//! wallet creation therefore stays where it is: camera entropy XORed with
//! the RNG, so the seed is never weaker than the stronger source.
//!
//! Tests over 256 samples (8192 bits):
//! - **Repetition count** (NIST SP 800-90B §4.4.1 shape, cutoff C = 2 per
//!   the spec formula at 32-bit samples): two consecutive identical values
//!   fail. False-positive odds ≈ 255 × 2⁻³² ≈ 6 × 10⁻⁸ per boot.
//! - **Monobit** (FIPS 140-2 lineage): total ones must sit within 6σ of
//!   4096 — the closed interval [3825, 4367]. False-positive ≈ 2 × 10⁻⁹.
//! - **Distinct values**: at least 250 of the 256 samples must be unique.
//!   A healthy 32-bit RNG collides at all in 256 draws with p ≈ 7.6 × 10⁻⁶,
//!   so 7+ collisions is effectively impossible — while any short-cycle
//!   source (e.g. alternating between two values, which sails through both
//!   tests above) fails instantly.

/// Number of 32-bit samples the boot check draws.
pub const RNG_HEALTH_SAMPLES: usize = 256;

/// Monobit bounds: 4096 ± 6·√2048, rounded inward to integers.
const ONES_MIN: u32 = 3825;
const ONES_MAX: u32 = 4367;

/// Minimum distinct values among the 256 samples.
const DISTINCT_MIN: usize = 250;

/// Evaluate the boot RNG health tests over `samples`. `Err` carries a
/// human-readable reason for the boot assert.
pub fn check_samples(samples: &[u32; RNG_HEALTH_SAMPLES]) -> Result<(), &'static str> {
    // Repetition count, spec cutoff C = 2: any adjacent pair equal fails.
    if samples.windows(2).any(|w| w[0] == w[1]) {
        return Err("repetition count: two consecutive identical values");
    }

    // Monobit over all 8192 bits.
    let ones: u32 = samples.iter().map(|v| v.count_ones()).sum();
    if !(ONES_MIN..=ONES_MAX).contains(&ones) {
        return Err("monobit: ones count outside 6-sigma bounds");
    }

    // Distinct values: kills short-cycle sources the two tests above miss.
    let mut sorted = *samples;
    sorted.sort_unstable();
    let distinct = 1 + sorted.windows(2).filter(|w| w[0] != w[1]).count();
    if distinct < DISTINCT_MIN {
        return Err("distinct values: sample set collapses to a short cycle");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic xorshift64 — a statistically fine stand-in for a
    /// healthy generator in these coarse tests.
    fn healthy() -> [u32; RNG_HEALTH_SAMPLES] {
        let mut state: u64 = 0x9E3779B97F4A7C15;
        core::array::from_fn(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 32) as u32
        })
    }

    #[test]
    fn healthy_sequence_passes() {
        assert!(check_samples(&healthy()).is_ok());
    }

    #[test]
    fn all_zero_fails() {
        assert!(check_samples(&[0u32; RNG_HEALTH_SAMPLES]).is_err());
    }

    #[test]
    fn all_ones_fails() {
        assert!(check_samples(&[u32::MAX; RNG_HEALTH_SAMPLES]).is_err());
    }

    #[test]
    fn one_adjacent_repeat_fails() {
        // A single stuck pair in an otherwise healthy stream must fail (C=2).
        let mut s = healthy();
        s[100] = s[99];
        assert_eq!(
            check_samples(&s),
            Err("repetition count: two consecutive identical values")
        );
    }

    #[test]
    fn alternating_values_fail() {
        // The counter-example from the PR #119 review: 1 bit of entropy per
        // sample, balanced popcount, no adjacent repeats — passes RCT and
        // monobit, must be killed by the distinct-value test.
        let s: [u32; RNG_HEALTH_SAMPLES] =
            core::array::from_fn(|i| if i % 2 == 0 { 0x0000_0000 } else { 0xFFFF_FFFF });
        assert_eq!(
            check_samples(&s),
            Err("distinct values: sample set collapses to a short cycle")
        );
    }

    #[test]
    fn biased_stream_fails_monobit() {
        // Healthy timing, but only the low 16 bits ever set: ~2048 ones,
        // far below the 3825 floor.
        let mut h = healthy();
        for v in h.iter_mut() {
            *v &= 0x0000_FFFF;
        }
        assert_eq!(
            check_samples(&h),
            Err("monobit: ones count outside 6-sigma bounds")
        );
    }

}
