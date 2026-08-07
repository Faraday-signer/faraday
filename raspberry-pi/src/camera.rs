//! Camera QR-decode helpers (rxing backend).
//!
//! Platform-agnostic types (`Frame`, `ScanDiagnostics`, `ScanMode`) live
//! in `faraday_core::camera`. This module provides the rxing-based
//! `try_decode_qr` used by the Pi and desktop simulator backends.

pub use faraday_core::camera::{Frame, ScanDiagnostics, ScanMode, downsample_center_square};
#[cfg(feature = "_desktop_sim")]
pub use faraday_core::camera::rgb_to_gray;

/// Try to decode a QR code from a camera frame using rxing.
///
/// Pipeline: luma plane → optional downsample → Otsu binarize → rxing
/// with TryHarder for maximum detection rate on noisy camera frames.
pub fn try_decode_qr(frame: &Frame, mode: ScanMode) -> Option<Vec<u8>> {
    let (w, h, mut luma_buf) = match mode {
        ScanMode::Full => (frame.width, frame.height, frame.luma.clone()),
        ScanMode::SmallQr => downsample_center_square(frame, 2),
    };

    let t = faraday_core::qr::threshold::otsu_threshold(&luma_buf);
    faraday_core::qr::threshold::binarize_in_place(&mut luma_buf, t);

    let mut hints = rxing::DecodeHints::default();
    hints.TryHarder = Some(true);
    match rxing::helpers::detect_in_luma_with_hints(
        luma_buf,
        w,
        h,
        Some(rxing::BarcodeFormat::QR_CODE),
        &mut hints,
    ) {
        Ok(result) => Some(faraday_core::qr::result_bytes::payload_bytes(&result)),
        Err(_) => None,
    }
}

/// UR-aware decode wrapper. Delegates to `faraday_core::camera::try_decode_qr_ur_diag`
/// with the rxing-based decoder.
#[allow(dead_code)]
pub fn try_decode_qr_ur_diag(
    frame: &Frame,
    accumulator: &mut faraday_core::qr::ur_decoder::UrAccumulator,
    mode: ScanMode,
) -> (Option<Vec<u8>>, bool) {
    faraday_core::camera::try_decode_qr_ur_diag(frame, accumulator, mode, try_decode_qr)
}
