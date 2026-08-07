//! Shared camera types used by all platform backends.

/// A decoded frame from the camera, with both an interleaved RGB buffer
/// for preview blitting and a raw luma (8-bit grayscale) buffer for QR
/// decoding.
///
/// `rgb` is length `width * height * 3`, `luma` is length `width * height`.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
    pub luma: Vec<u8>,
}

/// Live state from the scan pipeline for on-device diagnostics.
#[derive(Clone, Copy, Default, Debug)]
pub struct ScanDiagnostics {
    pub last_qr_at: Option<std::time::Instant>,
    pub ur_progress: Option<(usize, usize)>,
}

/// Scan mode hint for the QR decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanMode {
    Full,
    SmallQr,
}

/// Convert interleaved RGB to 8-bit grayscale (BT.601 luma).
#[cfg(feature = "_desktop_sim")]
pub fn rgb_to_gray(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((width * height) as usize);
    for px in rgb.chunks_exact(3) {
        let y = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
        out.push(y as u8);
    }
    out
}

/// Center-crop the frame's luma plane to a square, then nearest-neighbour
/// downsample by `factor`. Returns `(out_w, out_h, pixels)`.
pub fn downsample_center_square(frame: &Frame, factor: u32) -> (u32, u32, Vec<u8>) {
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

/// UR-aware QR decode wrapper. Decodes a single QR from a camera frame
/// using the provided `decode_fn`, then feeds the result through the UR
/// accumulator for multi-part reassembly.
///
/// Returns `(payload, saw_qr)`: payload is `Some` when a complete message
/// (single-frame or fully reassembled UR) is ready; `saw_qr` is true if
/// the decoder found any QR at all (even an incomplete UR fragment).
#[allow(dead_code)]
pub fn try_decode_qr_ur_diag(
    frame: &Frame,
    accumulator: &mut crate::qr::ur_decoder::UrAccumulator,
    mode: ScanMode,
    decode_fn: fn(&Frame, ScanMode) -> Option<Vec<u8>>,
) -> (Option<Vec<u8>>, bool) {
    let raw = match decode_fn(frame, mode) {
        Some(bytes) => bytes,
        None => return (None, false),
    };

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
