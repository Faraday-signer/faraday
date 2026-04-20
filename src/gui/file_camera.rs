//! File-based camera replacement for the no-cam simulator.
//!
//! Instead of capturing from a webcam, opens a native file picker so the
//! user can select a PNG image. The image is loaded, displayed as a preview
//! frame, and scanned for QR codes.

use std::sync::Mutex;

pub use crate::camera::Frame;

pub struct FileCamera {
    frame: Option<Frame>,
    pending_qr: Mutex<Option<Vec<u8>>>,
}

impl FileCamera {
    pub fn open() -> Result<Self, String> {
        let path = rfd::FileDialog::new()
            .add_filter("Image", &["png"])
            .set_title("Select QR code image")
            .pick_file()
            .ok_or_else(|| "No file selected".to_string())?;

        let img = image::open(&path)
            .map_err(|e| format!("Failed to load image: {e}"))?;
        let rgb_img = img.to_rgb8();
        let (w, h) = rgb_img.dimensions();
        let rgb = rgb_img.into_raw();

        let frame = Frame { width: w, height: h, rgb };
        let qr_data = crate::camera::try_decode_qr(&frame);

        Ok(FileCamera {
            frame: Some(frame),
            pending_qr: Mutex::new(qr_data),
        })
    }

    pub fn latest(&self) -> Option<Frame> {
        self.frame.clone()
    }

    pub fn take_qr(&self) -> Option<Vec<u8>> {
        self.pending_qr.lock().ok().and_then(|mut g| g.take())
    }

    pub fn set_decode_enabled(&self, _on: bool) {}

    pub fn take_fatal_err(&self) -> Option<String> {
        None
    }
}
