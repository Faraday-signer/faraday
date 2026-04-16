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
