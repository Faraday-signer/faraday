//! QR decode backend for ESP32 — quircs.
//!
//! Pipeline mirrors the Pi's (raspberry-pi/src/camera.rs) minus the
//! rxing-specific parts: luma plane → optional center-square downsample →
//! Otsu binarize → quircs detect+decode. quircs compiles for xtensa, which
//! rxing does not.

use faraday_core::camera::{downsample_center_square, Frame, ScanMode};

/// Decode a QR code from `frame`. Returns the raw payload bytes on success.
pub fn try_decode_qr(frame: &Frame, mode: ScanMode) -> Option<Vec<u8>> {
    let (w, h, mut luma) = match mode {
        ScanMode::Full => (frame.width, frame.height, frame.luma.clone()),
        ScanMode::SmallQr => downsample_center_square(frame, 2),
    };

    let t = faraday_core::qr::threshold::otsu_threshold(&luma);
    faraday_core::qr::threshold::binarize_in_place(&mut luma, t);

    let mut decoder = quircs::Quirc::default();
    let codes = decoder.identify(w as usize, h as usize, &luma);
    for code in codes {
        let Ok(code) = code else { continue };
        if let Ok(data) = code.decode() {
            return Some(data.payload);
        }
    }
    None
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
