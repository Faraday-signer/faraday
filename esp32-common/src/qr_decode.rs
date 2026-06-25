//! QR decode backend for ESP32 — rxing (zxing port), the most tolerant decoder,
//! same as the Pi. rxing builds for xtensa. Pipeline mirrors the Pi: center-crop
//! + 2×2 box-average, Otsu binarize, then rxing with TryHarder. rxing's
//! robustness lets it read Faraday's dense ~61-module fragments at the lower,
//! faster resolution where quircs/rqrr failed.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use faraday_core::camera::{Frame, ScanMode};

/// Pixels trimmed off the square scan region before averaging. 40 on a 600-tall
/// SVGA frame → a 560 crop → 280×280 after ÷2.
const SCAN_MARGIN: u32 = 40;

thread_local! {
    // Throttle for diagnostic logging (≤ 1 line/sec).
    static LAST_LOG: RefCell<Instant> = RefCell::new(Instant::now());
}

/// Decode a QR code from `frame`. Returns the raw payload bytes on success.
///
/// Resolution adapts to the scan type. Small QRs (seed, ~25 modules) decode
/// fine from the fast 280 px box-averaged crop. Dense fragments (TX / animated
/// UR, ~61 modules) need more pixels-per-module, so Full mode feeds rxing the
/// ~580 px native crop (~9.5 px/module — the same density at which the seed
/// decodes reliably). The Full pass is slower per attempt but actually decodes.
pub fn try_decode_qr(frame: &Frame, mode: ScanMode) -> Option<Vec<u8>> {
    let (w, h, mut luma) = match mode {
        ScanMode::SmallQr => crop_square_boxavg2(frame, SCAN_MARGIN),
        ScanMode::Full => crop_center_square(frame, 20),
    };

    // Otsu is a *global* threshold. It's needed for SmallQr (hand-drawn
    // CompactSeedQR sheets, whose 1 mm paper grid lines survive rxing's adaptive
    // binarizer and confuse finder detection — see core::qr::threshold). But on
    // the dense Full (TX / UR) path it actively hurts off-perpendicular scans:
    // tilt introduces an illumination gradient that one global cutoff can't
    // track, smearing the foreshortened far side of the code. Passing raw luma
    // lets rxing's HybridBinarizer threshold *locally* and adapt to the gradient.
    if matches!(mode, ScanMode::SmallQr) {
        let t = faraday_core::qr::threshold::otsu_threshold(&luma);
        faraday_core::qr::threshold::binarize_in_place(&mut luma, t);
    }

    // NOTE: TryHarder is intentionally NOT enabled. It makes rxing try many
    // binarizations/rotations and on some noisy camera frames it spins for a
    // very long time (the decoder thread froze mid-scan). rxing's single-pass
    // base decoder is still far more tolerant than quircs (it read the dense
    // 61-module fragment at 280 px) and is bounded in time.
    let mut hints = rxing::DecodeHints::default();
    let result = rxing::helpers::detect_in_luma_with_hints(
        luma,
        w as u32,
        h as u32,
        Some(rxing::BarcodeFormat::QR_CODE),
        &mut hints,
    );

    let payload = result
        .ok()
        .map(|r| r.getText().chars().map(|c| c as u32 as u8).collect::<Vec<u8>>());

    LAST_LOG.with(|cell| {
        let mut last = cell.borrow_mut();
        if last.elapsed() >= Duration::from_millis(1000) {
            log::info!("qr {}x{} decoded={}", w, h, payload.is_some());
            *last = Instant::now();
        }
    });

    payload
}

/// Center-crop `frame.luma` to a `(min(w,h) - margin)` square at native
/// resolution (no downsampling — preserves pixels-per-module for dense QRs).
fn crop_center_square(frame: &Frame, margin: u32) -> (usize, usize, Vec<u8>) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let side = (frame.width.min(frame.height).saturating_sub(margin)) as usize;
    let cx = (w - side) / 2;
    let cy = (h - side) / 2;
    let luma = &frame.luma;
    let mut buf = vec![0u8; side * side];
    for y in 0..side {
        let src = (cy + y) * w + cx;
        buf[y * side..(y + 1) * side].copy_from_slice(&luma[src..src + side]);
    }
    (side, side, buf)
}

/// Center-crop `frame.luma` to a `(min(w,h) - margin)` square, then 2×2
/// box-average ÷2 (clean modules + fewer pixels → faster rxing).
fn crop_square_boxavg2(frame: &Frame, margin: u32) -> (usize, usize, Vec<u8>) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let side = (frame.width.min(frame.height).saturating_sub(margin)) as usize & !1;
    let cx = (w - side) / 2;
    let cy = (h - side) / 2;
    let out = side / 2;
    let luma = &frame.luma;
    let mut buf = vec![0u8; out * out];
    for y in 0..out {
        let row0 = (cy + y * 2) * w + cx;
        let row1 = row0 + w;
        for x in 0..out {
            let sx = x * 2;
            let sum = luma[row0 + sx] as u16
                + luma[row0 + sx + 1] as u16
                + luma[row1 + sx] as u16
                + luma[row1 + sx + 1] as u16;
            buf[y * out + x] = (sum >> 2) as u8;
        }
    }
    (out, out, buf)
}

/// UR-aware decode wrapper — delegates to the core helper with `try_decode_qr`
/// as the decode function.
pub fn try_decode_qr_ur_diag(
    frame: &Frame,
    accumulator: &mut faraday_core::qr::ur_decoder::UrAccumulator,
    mode: ScanMode,
) -> (Option<Vec<u8>>, bool) {
    faraday_core::camera::try_decode_qr_ur_diag(frame, accumulator, mode, try_decode_qr)
}
