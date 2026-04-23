//! Shared camera types + QR-decode helpers used by both the macOS
//! simulator (`gui::sim_camera`) and the Pi V4L2 driver (`hardware::pi_camera`).
//!
//! Exactly one of those two backends is compiled per build: simulator on
//! `--features simulator` (any host), Pi on `target_os = "linux"` without the
//! simulator feature. This module itself has no platform gating — the `Frame`
//! struct and helpers are always available, so `App` / `Framebuffer` can refer
//! to them regardless of backend.

/// A decoded frame from the camera, with both an interleaved RGB buffer
/// for preview blitting and a raw luma (8-bit grayscale) buffer for QR
/// decoding. Keeping both on the producer thread avoids the Pi-Zero-hot
/// `RGB → grayscale` conversion in the decoder loop — on YUV420 cameras
/// the Y-plane *is* the luma we want, so the cost is zero; on RGB-native
/// cameras (macOS simulator) we do the conversion once in the reader
/// instead of every decode attempt.
///
/// `rgb` is length `width * height * 3`, `luma` is length `width * height`.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
    pub luma: Vec<u8>,
}

/// Live state from the scan pipeline for on-device diagnostics. Surfaced on
/// the scan screens so the user can tell at a glance whether the camera is
/// picking up anything, rather than staring at a silent reticle.
#[derive(Clone, Copy, Default, Debug)]
pub struct ScanDiagnostics {
    /// Time the decoder last returned any QR payload. The UI renders a dot
    /// that's green when this is within the last ~2s, dim otherwise.
    pub last_qr_at: Option<std::time::Instant>,
    /// UR fountain progress: `(unique_received, total_expected)`. Counts
    /// distinct fragment sequence numbers that were accepted by the
    /// decoder — duplicates do not inflate `unique_received`. Cleared on
    /// scan-screen exit.
    pub ur_progress: Option<(usize, usize)>,
}

/// Convert interleaved RGB to 8-bit grayscale (BT.601 luma).
pub fn rgb_to_gray(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((width * height) as usize);
    for px in rgb.chunks_exact(3) {
        let y = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
        out.push(y as u8);
    }
    out
}

/// Scan mode hint from the caller. The seed-style scan screens only ever
/// see tiny QRs (CompactSeedQR V1/V3, address QRs ≤ V5), which we can
/// downsample aggressively before handing to the decoder. The Sign-TX
/// screen can see dense single-frame txs (V20+) that need full resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanMode {
    /// Full-resolution decode. Use when QR density is unbounded.
    Full,
    /// Center-crop square, 2× nearest-neighbour downsample. Use when the
    /// caller *knows* the QR is small (≤ V5); modules at the reticle
    /// framing land at ~4–7 px post-downsample — well above the decoder's
    /// floor — and the decoder walks 4× fewer pixels.
    SmallQr,
}

/// Try to decode a QR code from the frame. Pipeline:
///   1. Pick the luma plane (optionally center-cropped + downsampled for
///      SmallQr screens).
///   2. Otsu binarize (`src/qr/threshold.rs`) so hand-drawn CompactSeedQR
///      sheets on gridded paper survive the decoder's finder-pattern
///      detection — the decoder's own adaptive binarizer trips on the
///      1 mm paper grid and fails where Otsu + a clean b/w frame works.
///   3. Hand to `rxing` for finder-pattern detection, grid extraction,
///      and Reed-Solomon ECC.
///
/// Returns the raw decoded bytes on success. Does no format validation —
/// caller inspects the bytes.
pub fn try_decode_qr(frame: &Frame, mode: ScanMode) -> Option<Vec<u8>> {
    let (w, h, mut luma_buf) = match mode {
        ScanMode::Full => (frame.width, frame.height, frame.luma.clone()),
        ScanMode::SmallQr => downsample_center_square(frame, 2),
    };

    let t = crate::qr::threshold::otsu_threshold(&luma_buf);
    crate::qr::threshold::binarize_in_place(&mut luma_buf, t);

    let mut hints = rxing::DecodeHints::default();
    hints.TryHarder = Some(true);
    match rxing::helpers::detect_in_luma_with_hints(
        luma_buf,
        w,
        h,
        Some(rxing::BarcodeFormat::QR_CODE),
        &mut hints,
    ) {
        Ok(result) => Some(decode_result_to_bytes(&result)),
        Err(_) => None,
    }
}

/// Recover the original byte payload from an rxing `RXingResult`. For
/// ASCII text QRs we can use `getText()` directly; for binary payloads
/// (CompactSeedQR raw entropy, base64 tx, etc.) rxing ISO-8859-1-encodes
/// the bytes into a UTF-8 string, so we map each char's low byte back.
fn decode_result_to_bytes(result: &rxing::RXingResult) -> Vec<u8> {
    let text = result.getText();
    text.chars().map(|c| (c as u32) as u8).collect()
}

/// Center-crop the frame's luma plane to a square, then nearest-neighbour
/// downsample by `factor`. Returns `(out_w, out_h, pixels)`.
fn downsample_center_square(frame: &Frame, factor: u32) -> (u32, u32, Vec<u8>) {
    let (w, h) = (frame.width, frame.height);
    let side = w.min(h);
    let crop_x = (w - side) / 2;
    let crop_y = (h - side) / 2;
    let out_side = side / factor;
    let mut out = vec![0u8; (out_side * out_side) as usize];
    for y in 0..out_side {
        let src_y = crop_y + y * factor;
        let src_row = (src_y * w + crop_x) as usize;
        for x in 0..out_side {
            out[(y * out_side + x) as usize] = frame.luma[src_row + (x * factor) as usize];
        }
    }
    (out_side, out_side, out)
}

/// Try to decode a QR from the frame and feed it to the UR accumulator.
/// Returns the fully reconstructed payload when all fountain parts are received,
/// or a single-frame payload if the QR is not UR-encoded.
pub fn try_decode_qr_ur(
    frame: &Frame,
    accumulator: &mut crate::qr::ur_decoder::UrAccumulator,
    mode: ScanMode,
) -> Option<Vec<u8>> {
    try_decode_qr_ur_diag(frame, accumulator, mode).0
}

/// Same as `try_decode_qr_ur` but also reports whether the decoder saw
/// *any* QR this call (even a fragment or a format we didn't finish
/// reassembling). Camera backends use the extra signal to drive the scan-
/// screen heartbeat so the user can distinguish "camera sees nothing"
/// from "camera sees a fragment, waiting for more".
pub fn try_decode_qr_ur_diag(
    frame: &Frame,
    accumulator: &mut crate::qr::ur_decoder::UrAccumulator,
    mode: ScanMode,
) -> (Option<Vec<u8>>, bool) {
    let raw = match try_decode_qr(frame, mode) {
        Some(bytes) => bytes,
        None => return (None, false),
    };

    // Binary payloads (e.g. CompactSeedQR raw entropy) almost always contain
    // non-UTF-8 bytes. A UR message is by spec a printable `ur:<type>/<seq>`
    // text string, so if the bytes aren't valid UTF-8 they can't be UR —
    // return them verbatim. Without this, random entropy bytes look like
    // "decode failed" to the caller and valid scans get silently dropped.
    let text = match std::str::from_utf8(&raw) {
        Ok(t) => t,
        Err(_) => return (Some(raw), true),
    };

    if crate::qr::ur_decoder::UrAccumulator::is_ur(text) {
        match accumulator.receive(text) {
            Ok(true) => {
                let msg = accumulator.message();
                accumulator.reset();
                (msg, true)
            }
            Ok(false) | Err(_) => (None, true),
        }
    } else {
        (Some(raw), true)
    }
}
