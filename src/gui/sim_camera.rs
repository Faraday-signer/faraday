//! Webcam capture for the desktop simulator.
//!
//! Opens the default camera in a background thread that owns the handle
//! (nokhwa's Camera isn't Send on macOS). The thread continuously pulls
//! frames, attempts QR detection on each, and publishes both the latest
//! frame (for preview) and any pending QR decode to the main thread.
//! Dropping SimCamera stops the stream and joins the thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

pub use crate::camera::Frame;

pub struct SimCamera {
    latest: Arc<Mutex<Option<Frame>>>,
    pending_qr: Arc<Mutex<Option<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    decode_enabled: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl SimCamera {
    pub fn open() -> Result<Self, String> {
        let latest: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
        let pending_qr: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        // QR decoding is expensive (100-200ms/frame on 1280x720). Off by default
        // so the preview stays responsive; main thread flips it on for scan screens.
        let decode_enabled = Arc::new(AtomicBool::new(false));
        let latest_w = Arc::clone(&latest);
        let qr_w = Arc::clone(&pending_qr);
        let stop_w = Arc::clone(&stop);
        let decode_w = Arc::clone(&decode_enabled);
        let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(), String>>();

        let handle = thread::spawn(move || {
            let index = CameraIndex::Index(0);
            let requested = RequestedFormat::new::<RgbFormat>(
                RequestedFormatType::AbsoluteHighestFrameRate,
            );
            let mut camera = match Camera::new(index, requested) {
                Ok(c) => c,
                Err(e) => {
                    let _ = init_tx.send(Err(format!("open: {e}")));
                    return;
                }
            };
            if let Err(e) = camera.open_stream() {
                let _ = init_tx.send(Err(format!("stream: {e}")));
                return;
            }
            let _ = init_tx.send(Ok(()));

            while !stop_w.load(Ordering::Relaxed) {
                let buf = match camera.frame() {
                    Ok(b) => b,
                    Err(_) => break,
                };
                let img = match buf.decode_image::<RgbFormat>() {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let (w, h) = img.dimensions();
                let rgb = img.into_raw();

                let frame = Frame { width: w, height: h, rgb };

                // QR decode only on scan screens (enabled by the main thread) and
                // only when the main thread hasn't yet consumed the previous hit.
                let should_decode = decode_w.load(Ordering::Relaxed)
                    && qr_w.lock().ok().map(|g| g.is_none()).unwrap_or(false);
                if should_decode {
                    if let Some(decoded) = crate::camera::try_decode_qr(&frame) {
                        if let Ok(mut g) = qr_w.lock() {
                            *g = Some(decoded);
                        }
                    }
                }

                if let Ok(mut g) = latest_w.lock() {
                    *g = Some(frame);
                }
            }
            let _ = camera.stop_stream();
        });

        match init_rx.recv() {
            Ok(Ok(())) => Ok(SimCamera {
                latest,
                pending_qr,
                stop,
                decode_enabled,
                handle: Some(handle),
            }),
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(e)
            }
            Err(_) => Err("camera thread died before init".into()),
        }
    }

    pub fn latest(&self) -> Option<Frame> {
        self.latest.lock().ok().and_then(|g| g.clone())
    }

    /// Consume the pending QR decode, if any. Returns None until a new one is detected.
    pub fn take_qr(&self) -> Option<Vec<u8>> {
        self.pending_qr.lock().ok().and_then(|mut g| g.take())
    }

    /// Turn QR detection on/off. Detection is expensive; keep off unless on a scan screen.
    pub fn set_decode_enabled(&self, on: bool) {
        self.decode_enabled.store(on, Ordering::Relaxed);
    }

    /// API parity with PiCamera; simulator webcam has no watchdog path.
    pub fn take_fatal_err(&self) -> Option<String> {
        None
    }
}

impl Drop for SimCamera {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
