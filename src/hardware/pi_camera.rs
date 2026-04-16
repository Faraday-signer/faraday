//! Raspberry Pi camera (OV5647) via V4L2.
//!
//! Opens `/dev/video0`, streams MJPEG frames in a background thread,
//! decodes each frame to RGB for preview + QR detection. Mirrors the
//! `gui::sim_camera::SimCamera` API so `App::tick` can treat both the
//! same way.
//!
//! Dropping the struct stops the stream and joins the capture thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use v4l::buffer::Type as BufType;
use v4l::io::mmap::Stream as MmapStream;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;
use v4l::{Device, Format, FourCC};

pub use crate::camera::Frame;

/// Capture resolution. Keep modest — 640x480 is plenty for QR scanning and
/// gives ~30 FPS on a Pi Zero. Larger images make `rqrr` detection slower
/// without meaningful decode-rate improvement.
const CAPTURE_W: u32 = 640;
const CAPTURE_H: u32 = 480;

pub struct PiCamera {
    latest: Arc<Mutex<Option<Frame>>>,
    pending_qr: Arc<Mutex<Option<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    decode_enabled: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl PiCamera {
    /// Open /dev/video0 and start streaming. Surface open errors synchronously
    /// via a short-lived init channel so the caller can show "Camera unavailable".
    pub fn open() -> Result<Self, String> {
        let latest: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
        let pending_qr: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let decode_enabled = Arc::new(AtomicBool::new(false));
        let latest_w = Arc::clone(&latest);
        let qr_w = Arc::clone(&pending_qr);
        let stop_w = Arc::clone(&stop);
        let decode_w = Arc::clone(&decode_enabled);
        let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(), String>>();

        let handle = thread::spawn(move || {
            let dev = match Device::with_path("/dev/video0") {
                Ok(d) => d,
                Err(e) => {
                    let _ = init_tx.send(Err(format!("open /dev/video0: {e}")));
                    return;
                }
            };

            // Request MJPEG @ 640x480. MJPEG is compact + universally supported
            // by V4L2 drivers; decoding to RGB is done in-thread via the `image`
            // crate. Falling back to the driver's offered format if this fails.
            let requested = Format::new(CAPTURE_W, CAPTURE_H, FourCC::new(b"MJPG"));
            let fmt = match Capture::set_format(&dev, &requested) {
                Ok(f) => f,
                Err(e) => {
                    let _ = init_tx.send(Err(format!("set_format: {e}")));
                    return;
                }
            };
            let is_mjpeg = &fmt.fourcc.repr == b"MJPG";

            let mut stream = match MmapStream::with_buffers(&dev, BufType::VideoCapture, 4) {
                Ok(s) => s,
                Err(e) => {
                    let _ = init_tx.send(Err(format!("stream: {e}")));
                    return;
                }
            };
            let _ = init_tx.send(Ok(()));

            let width = fmt.width;
            let height = fmt.height;

            while !stop_w.load(Ordering::Relaxed) {
                let (buf, _meta) = match CaptureStream::next(&mut stream) {
                    Ok(r) => r,
                    Err(_) => break,
                };

                let rgb = if is_mjpeg {
                    match image::load_from_memory_with_format(buf, image::ImageFormat::Jpeg) {
                        Ok(img) => img.to_rgb8().into_raw(),
                        Err(_) => continue,
                    }
                } else {
                    // Unknown format — best-effort: assume RGB24. If the driver
                    // handed us something else (YUYV etc.) the preview will be
                    // garbled but we won't crash.
                    buf.to_vec()
                };

                let frame = Frame {
                    width,
                    height,
                    rgb,
                };

                // QR decode only on scan screens (toggled by main thread) and
                // only when the main thread hasn't yet consumed the last hit.
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
            // stream drops, releasing buffers
        });

        match init_rx.recv() {
            Ok(Ok(())) => Ok(PiCamera {
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

    pub fn take_qr(&self) -> Option<Vec<u8>> {
        self.pending_qr.lock().ok().and_then(|mut g| g.take())
    }

    pub fn set_decode_enabled(&self, on: bool) {
        self.decode_enabled.store(on, Ordering::Relaxed);
    }
}

impl Drop for PiCamera {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
