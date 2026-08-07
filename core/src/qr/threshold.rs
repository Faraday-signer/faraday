//! Otsu global-threshold binarization.
//!
//! The downstream QR decoder (`rxing`) has an adaptive binarizer of its own,
//! but it struggles on hand-drawn CompactSeedQR sheets where the paper's
//! 1 mm grid lines survive its threshold and confuse finder-pattern
//! detection. Pre-binarizing with Otsu first — which picks the cutoff that
//! maximises between-class variance on the frame's intensity histogram —
//! collapses the grid into the white class and leaves the inked modules
//! cleanly black, so the decoder sees a textbook bimodal image.
//!
//! Normal camera / screenshot frames binarize fine too: Otsu degenerates
//! to a sensible threshold (127 on a bimodal screenshot, near-0 on a
//! pre-binarized PNG). No special casing needed.

/// Find the threshold 0..=255 that maximises between-class variance on
/// the luma histogram. Single pass over the 256-bin histogram; O(N) in
/// frame pixels for the histogram build.
pub fn otsu_threshold(luma: &[u8]) -> u8 {
    let mut hist = [0u64; 256];
    for &p in luma {
        hist[p as usize] += 1;
    }
    let total = luma.len() as f64;

    let sum_total: f64 = (0..256).map(|i| (i as f64) * hist[i] as f64).sum();

    let mut sum_bg: f64 = 0.0;
    let mut w_bg: f64 = 0.0;
    let mut best_var: f64 = -1.0;
    let mut best: u8 = 127;

    for t in 0..256 {
        w_bg += hist[t] as f64;
        if w_bg == 0.0 {
            continue;
        }
        let w_fg = total - w_bg;
        if w_fg <= 0.0 {
            break;
        }
        sum_bg += (t as f64) * hist[t] as f64;
        let mean_bg = sum_bg / w_bg;
        let mean_fg = (sum_total - sum_bg) / w_fg;
        let diff = mean_bg - mean_fg;
        // `w_bg*w_fg*(μ_bg − μ_fg)²` — the between-class variance numerator.
        let var = w_bg * w_fg * diff * diff;
        if var > best_var {
            best_var = var;
            best = t as u8;
        }
    }
    best
}

/// Binarize `luma` in place: pixels ≤ threshold → 0, others → 255.
pub fn binarize_in_place(luma: &mut [u8], threshold: u8) {
    for p in luma.iter_mut() {
        *p = if *p <= threshold { 0 } else { 255 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otsu_bimodal_black_white() {
        // Purely black / white pixels — Otsu should return 0 (or any cutoff
        // that separates the two classes; the code's tie-break gives 0).
        let luma = [0u8, 0, 255, 255, 0, 255];
        assert!(otsu_threshold(&luma) < 255);
    }

    #[test]
    fn otsu_centres_between_two_peaks() {
        // Peaks at 40 and 200 — threshold should land between them.
        let mut luma = Vec::new();
        for _ in 0..500 {
            luma.push(40);
        }
        for _ in 0..500 {
            luma.push(200);
        }
        let t = otsu_threshold(&luma);
        assert!(t >= 40 && t < 200, "threshold {} should sit between peaks", t);
    }

    #[test]
    fn binarize_splits_on_threshold() {
        let mut luma = [0u8, 50, 100, 127, 128, 200, 255];
        binarize_in_place(&mut luma, 127);
        assert_eq!(luma, [0, 0, 0, 0, 255, 255, 255]);
    }
}
