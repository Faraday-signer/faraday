//! Shared camera types + QR-decode helpers used by both the macOS
//! simulator (`gui::sim_camera`) and the Pi V4L2 driver (`hardware::pi_camera`).
//!
//! Exactly one of those two backends is compiled per build: simulator on
//! `--features simulator` (any host), Pi on `target_os = "linux"` without the
//! simulator feature. This module itself has no platform gating — the `Frame`
//! struct and helpers are always available, so `App` / `Framebuffer` can refer
//! to them regardless of backend.

/// A decoded RGB frame from the camera. Interleaved `R, G, B, R, G, B, …`,
/// length `width * height * 3`.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// Live state from the scan pipeline for on-device diagnostics. Surfaced on
/// the scan screens so the user can tell at a glance whether the camera is
/// picking up anything, rather than staring at a silent reticle.
#[derive(Clone, Copy, Default, Debug)]
pub struct ScanDiagnostics {
    /// Time the decoder last returned any QR payload. The UI renders a dot
    /// that's green when this is within the last ~2s, dim otherwise.
    pub last_qr_at: Option<std::time::Instant>,
    /// `(seq, total)` from the most recent UR fragment header. Present
    /// whenever a UR fragment was parsed; cleared on scan-screen exit.
    pub last_ur_part: Option<(usize, usize)>,
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

/// Try to decode a QR code from the frame. Returns the raw decoded bytes on
/// success. Does no format validation — caller inspects the bytes.
pub fn try_decode_qr(frame: &Frame) -> Option<Vec<u8>> {
    let gray = rgb_to_gray(&frame.rgb, frame.width, frame.height);
    let gimg = image::GrayImage::from_raw(frame.width, frame.height, gray)?;
    let mut prepared = rqrr::PreparedImage::prepare(gimg);
    for grid in prepared.detect_grids() {
        let mut out = Vec::new();
        if grid.decode_to(&mut out).is_ok() && !out.is_empty() {
            return Some(out);
        }
    }
    None
}

/// Try to decode a QR from the frame and feed it to the UR accumulator.
/// Returns the fully reconstructed payload when all fountain parts are received,
/// or a single-frame payload if the QR is not UR-encoded.
pub fn try_decode_qr_ur(
    frame: &Frame,
    accumulator: &mut crate::qr::ur_decoder::UrAccumulator,
) -> Option<Vec<u8>> {
    try_decode_qr_ur_diag(frame, accumulator).0
}

/// Same as `try_decode_qr_ur` but also reports whether rqrr detected *any*
/// QR this call (even a fragment or a format we didn't finish reassembling).
/// Camera backends use the extra signal to drive the scan-screen heartbeat
/// so the user can distinguish "camera sees nothing" from "camera sees a
/// fragment, waiting for more".
pub fn try_decode_qr_ur_diag(
    frame: &Frame,
    accumulator: &mut crate::qr::ur_decoder::UrAccumulator,
) -> (Option<Vec<u8>>, bool) {
    let raw = match try_decode_qr(frame) {
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
