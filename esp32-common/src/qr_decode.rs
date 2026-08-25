//! QR decode backend for ESP32 — rxing (zxing port), the most tolerant decoder,
//! same as the Pi. rxing builds for xtensa. Pipeline mirrors the Pi: center-crop
//! + 2×2 box-average, Otsu binarize, then rxing (TryHarder off — see below).
//! rxing's robustness lets it read Faraday's dense ~61-module fragments at the
//! lower, faster resolution where quircs/rqrr failed.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use rxing::Reader;

use faraday_core::camera::{Frame, ScanMode};

/// Keep the full 600-pixel center square, then box-average 2×2. The resulting
/// 300×300 view is the best measured hand-drawn SmallQR configuration: it keeps
/// more quiet-zone and finder-pattern context while remaining below the fresh
/// camera frame interval even when both decode paths run.
const SCAN_MARGIN: u32 = 0;

thread_local! {
    // Failures are throttled; successes always log before the scan screen exits.
    static LAST_LOG: RefCell<Instant> = RefCell::new(Instant::now());
    static ATTEMPTS_SINCE_LOG: Cell<u32> = const { Cell::new(0) };
}

/// Decode a QR code from `frame`. Returns the raw payload bytes on success.
///
/// Resolution adapts to the scan type. Small QRs (seed, ~25 modules) decode
/// fine from the fast 300 px box-averaged crop. Dense fragments (TX / animated
/// UR, ~61 modules) need more pixels-per-module, so Full mode feeds rxing the
/// ~580 px native crop (~9.5 px/module — the same density at which the seed
/// decodes reliably). The Full pass is slower per attempt but actually decodes.
pub fn try_decode_qr(frame: &Frame, mode: ScanMode) -> Option<Vec<u8>> {
    let started = Instant::now();
    let (w, h, luma, histogram) = match mode {
        ScanMode::SmallQr => {
            let (w, h, luma, histogram) = crop_square_boxavg2(frame, SCAN_MARGIN, true);
            (w, h, luma, histogram)
        }
        ScanMode::Full => {
            let (w, h, luma) = crop_center_square(frame, 20);
            (w, h, luma, None)
        }
    };
    let crop_done = Instant::now();

    // SmallQR gets both bounded fast paths on every fresh camera frame. The raw
    // pass handles illumination gradients; if it fails, Otsu suppresses the
    // paper grid before another HybridBinarizer pass. Together they remain
    // below the camera's ~200 ms fresh-frame interval. Full mode stays raw-only.
    let raw_started = Instant::now();
    let mut payload = decode_luma(luma.clone(), w, h);
    let raw_done = Instant::now();
    let mut used_otsu = false;
    if payload.is_none() && matches!(mode, ScanMode::SmallQr) {
        used_otsu = true;
        let mut otsu_luma = luma;
        let t = faraday_core::qr::threshold::otsu_threshold_from_histogram(
            histogram.as_ref().expect("SmallQr histogram"),
            otsu_luma.len(),
        );
        faraday_core::qr::threshold::binarize_in_place(&mut otsu_luma, t);
        payload = decode_luma(otsu_luma, w, h);
    }
    let decode_done = Instant::now();

    let attempts = ATTEMPTS_SINCE_LOG.with(|count| {
        let attempts = count.get().saturating_add(1);
        count.set(attempts);
        attempts
    });
    LAST_LOG.with(|cell| {
        let mut last = cell.borrow_mut();
        if payload.is_some() || last.elapsed() >= Duration::from_millis(1000) {
            log::info!(
                "qr {}x{} decoded={} prep={} attempts={} crop={}ms raw={}ms otsu={}ms total={}ms",
                w,
                h,
                payload.is_some(),
                if used_otsu { "raw+otsu" } else { "raw" },
                attempts,
                crop_done.duration_since(started).as_millis(),
                raw_done.duration_since(raw_started).as_millis(),
                if used_otsu {
                    decode_done.duration_since(raw_done).as_millis()
                } else {
                    0
                },
                decode_done.duration_since(started).as_millis(),
            );
            *last = Instant::now();
            ATTEMPTS_SINCE_LOG.with(|count| count.set(0));
        }
    });

    payload
}

/// Run one bounded QR-only rxing pass. TryHarder must remain off because rxing
/// has no time budget for its retry loop on noisy ESP32 camera frames.
fn decode_luma(luma: Vec<u8>, width: usize, height: usize) -> Option<Vec<u8>> {
    let mut hints = rxing::DecodeHints::default();
    hints.TryHarder = Some(false);
    let mut reader = rxing::qrcode::QRCodeReader::new();
    reader
        .decode_with_hints(
            &mut rxing::BinaryBitmap::new(rxing::common::HybridBinarizer::new(
                rxing::Luma8LuminanceSource::new(luma, width as u32, height as u32),
            )),
            &hints,
        )
        .ok()
        .map(|result| faraday_core::qr::result_bytes::payload_bytes(&result))
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
fn crop_square_boxavg2(
    frame: &Frame,
    margin: u32,
    collect_histogram: bool,
) -> (usize, usize, Vec<u8>, Option<[u64; 256]>) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let side = (frame.width.min(frame.height).saturating_sub(margin)) as usize & !1;
    let cx = (w - side) / 2;
    let cy = (h - side) / 2;
    let out = side / 2;
    let luma = &frame.luma;
    let mut buf = vec![0u8; out * out];
    let mut histogram = [0u64; 256];
    for y in 0..out {
        let row0 = (cy + y * 2) * w + cx;
        let row1 = row0 + w;
        for x in 0..out {
            let sx = x * 2;
            let sum = luma[row0 + sx] as u16
                + luma[row0 + sx + 1] as u16
                + luma[row1 + sx] as u16
                + luma[row1 + sx + 1] as u16;
            let averaged = (sum >> 2) as u8;
            buf[y * out + x] = averaged;
            if collect_histogram {
                histogram[averaged as usize] += 1;
            }
        }
    }
    (out, out, buf, collect_histogram.then_some(histogram))
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
