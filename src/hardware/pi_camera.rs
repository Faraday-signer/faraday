//! Raspberry Pi camera (OV5647) via the MMAL userland stack.
//!
//! Approach: spawn `/usr/bin/raspividyuv` as a child process, configure it
//! to stream raw YUV420 frames to stdout, and read fixed-size frames in a
//! background thread. This uses the MMAL/VCHIQ userland path to talk to
//! the GPU camera firmware — the only route that delivers frames reliably
//! on this Pi Zero + OV5647 combination (the kernel V4L2 shim opens the
//! stream but never DMAs frames into userspace buffers).
//!
//! Frame layout: `raspividyuv` emits YUV420 planar — Y plane first
//! (W*H bytes), then U plane (W/2 * H/2), then V plane. For 640x480 that
//! is 460,800 bytes per frame. We convert to interleaved RGB for the
//! shared `Frame` struct used by preview + QR decoding.
//!
//! Diagnostics: because the Pi has no shell or logs we can read, this
//! module does extensive pre-flight checks (`/usr/bin/raspividyuv` must be
//! executable, `/dev/vchiq` must exist, firmware blobs must be present)
//! and pipes the child's stderr into a background thread that accumulates
//! it into a string. On any failure that string is included in the error
//! returned via `take_fatal_err()` so we see MMAL's actual complaint
//! ("MMAL_ENOSPC", "No data received", etc.).

use std::io::Read;
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub use crate::camera::Frame;

const RASPIVIDYUV: &str = "/usr/bin/raspividyuv";
const VCHIQ_DEV: &str = "/dev/vchiq";

/// Capture resolution. 640x480 is a clean multiple of 32/16 (no stride
/// padding from raspividyuv) and plenty for QR scanning on a Pi Zero.
const CAPTURE_W: usize = 640;
const CAPTURE_H: usize = 480;
const Y_BYTES: usize = CAPTURE_W * CAPTURE_H;
const UV_BYTES: usize = (CAPTURE_W / 2) * (CAPTURE_H / 2);
const FRAME_BYTES: usize = Y_BYTES + 2 * UV_BYTES;

const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(6);
const STDERR_MAX_BYTES: usize = 512;

pub struct PiCamera {
    latest: Arc<Mutex<Option<Frame>>>,
    pending_qr: Arc<Mutex<Option<Vec<u8>>>>,
    diag: Arc<Mutex<crate::camera::ScanDiagnostics>>,
    fatal_err: Arc<Mutex<Option<String>>>,
    stop: Arc<AtomicBool>,
    decode_enabled: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    stderr_buf: Arc<Mutex<Vec<u8>>>,
    _reader: Option<JoinHandle<()>>,
    _stderr_handle: Option<JoinHandle<()>>,
    _decoder: Option<JoinHandle<()>>,
}

impl PiCamera {
    pub fn open() -> Result<Self, String> {
        // --- Pre-flight checks -------------------------------------------
        // Fail fast with a specific message instead of letting spawn()
        // return a generic "No such file or directory".
        if !Path::new(RASPIVIDYUV).exists() {
            return Err(format!("missing {RASPIVIDYUV} (rpi-userland not in rootfs?)"));
        }
        if !Path::new(VCHIQ_DEV).exists() {
            return Err(format!(
                "missing {VCHIQ_DEV} (VCHIQ kernel driver not loaded; check CONFIG_BCM2835_VCHIQ / devtmpfs)"
            ));
        }

        // --- Spawn raspividyuv -------------------------------------------
        let mut child = Command::new(RASPIVIDYUV)
            .arg("--nopreview")
            .arg("--width")
            .arg(CAPTURE_W.to_string())
            .arg("--height")
            .arg(CAPTURE_H.to_string())
            .arg("--framerate")
            .arg("15")
            .arg("--timeout")
            .arg("0")
            .arg("--output")
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn {RASPIVIDYUV}: {e}"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "raspividyuv: no stdout pipe".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "raspividyuv: no stderr pipe".to_string())?;

        let latest: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
        let pending_qr: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let diag: Arc<Mutex<crate::camera::ScanDiagnostics>> =
            Arc::new(Mutex::new(crate::camera::ScanDiagnostics::default()));
        let fatal_err: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let decode_enabled = Arc::new(AtomicBool::new(false));
        let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let child_slot: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(Some(child)));

        // Stderr collector thread — capped buffer so a noisy child can't
        // eat all RAM.
        let stderr_w = Arc::clone(&stderr_buf);
        let stop_e = Arc::clone(&stop);
        let stderr_handle = thread::spawn(move || {
            stderr_loop(stderr, stderr_w, stop_e);
        });

        // Frame reader thread — reads YUV, converts to RGB, publishes.
        // Kept decode-free so preview stays smooth; QR work runs on the
        // decoder thread below.
        let latest_w = Arc::clone(&latest);
        let fatal_w = Arc::clone(&fatal_err);
        let stderr_r = Arc::clone(&stderr_buf);
        let child_r = Arc::clone(&child_slot);
        let stop_r = Arc::clone(&stop);
        let reader = thread::spawn(move || {
            reader_loop(stdout, latest_w, fatal_w, stderr_r, child_r, stop_r);
        });

        // Dedicated QR decoder thread — rqrr on a 640x480 Y-plane costs
        // 200-400 ms on a Pi Zero; if we ran it inside the reader we'd
        // effectively drop the preview to ~2-3 fps on scan screens, and
        // the user would struggle to keep a QR in frame long enough for
        // an attempt to complete. Running decode independently keeps
        // preview at the full raspividyuv rate and lets decode attempts
        // overlap with fresh frame arrivals.
        let latest_d = Arc::clone(&latest);
        let qr_d = Arc::clone(&pending_qr);
        let diag_d = Arc::clone(&diag);
        let stop_d = Arc::clone(&stop);
        let decode_d = Arc::clone(&decode_enabled);
        let decoder = thread::spawn(move || {
            decoder_loop(latest_d, qr_d, diag_d, stop_d, decode_d);
        });

        // Watchdog: first frame or bust. Reports raspividyuv's stderr if the
        // child is silent for 6 s.
        let latest_wd = Arc::clone(&latest);
        let fatal_wd = Arc::clone(&fatal_err);
        let stop_wd = Arc::clone(&stop);
        let stderr_wd = Arc::clone(&stderr_buf);
        let child_wd = Arc::clone(&child_slot);
        thread::spawn(move || {
            thread::sleep(FIRST_FRAME_TIMEOUT);
            if stop_wd.load(Ordering::Relaxed) {
                return;
            }
            let got_frame = latest_wd.lock().ok().map(|g| g.is_some()).unwrap_or(false);
            if got_frame {
                return;
            }
            let mut msg = String::from("no_first_frame in 6s");
            let exit_status = child_wd
                .lock()
                .ok()
                .and_then(|mut g| g.as_mut().and_then(|c| c.try_wait().ok().flatten()));
            if let Some(status) = exit_status {
                msg = format!("raspividyuv exited early (status {status})");
            }
            let se = stderr_snapshot(&stderr_wd);
            if !se.is_empty() {
                msg.push_str(" | stderr: ");
                msg.push_str(&se);
            }
            if let Ok(mut g) = fatal_wd.lock() {
                if g.is_none() {
                    *g = Some(msg);
                }
            }
            stop_wd.store(true, Ordering::Relaxed);
        });

        Ok(PiCamera {
            latest,
            pending_qr,
            diag,
            fatal_err,
            stop,
            decode_enabled,
            child: child_slot,
            stderr_buf,
            _reader: Some(reader),
            _stderr_handle: Some(stderr_handle),
            _decoder: Some(decoder),
        })
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

    pub fn take_fatal_err(&self) -> Option<String> {
        self.fatal_err.lock().ok().and_then(|mut g| g.take())
    }

    pub fn diagnostics(&self) -> crate::camera::ScanDiagnostics {
        self.diag.lock().ok().map(|g| *g).unwrap_or_default()
    }
}

impl Drop for PiCamera {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut slot) = self.child.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn reader_loop(
    mut stdout: ChildStdout,
    latest: Arc<Mutex<Option<Frame>>>,
    fatal: Arc<Mutex<Option<String>>>,
    stderr_buf: Arc<Mutex<Vec<u8>>>,
    child_slot: Arc<Mutex<Option<Child>>>,
    stop: Arc<AtomicBool>,
) {
    let mut buf = vec![0u8; FRAME_BYTES];
    while !stop.load(Ordering::Relaxed) {
        if let Err(e) = stdout.read_exact(&mut buf) {
            let mut msg = format!("read: {e}");
            // Pipe closed usually means raspividyuv exited — grab its
            // exit status and stderr so we learn *why*.
            let exit_status = child_slot
                .lock()
                .ok()
                .and_then(|mut g| g.as_mut().and_then(|c| c.try_wait().ok().flatten()));
            if let Some(status) = exit_status {
                msg = format!("raspividyuv exited (status {status}) | read: {e}");
            }
            let se = stderr_snapshot(&stderr_buf);
            if !se.is_empty() {
                msg.push_str(" | stderr: ");
                msg.push_str(&se);
            }
            if let Ok(mut g) = fatal.lock() {
                if g.is_none() {
                    *g = Some(msg);
                }
            }
            return;
        }

        let rgb = yuv420_to_rgb(&buf, CAPTURE_W, CAPTURE_H);
        // YUV420's Y-plane is already 8-bit grayscale — the exact input
        // rqrr wants. Copy it out here (free: it's just the first
        // `CAPTURE_W * CAPTURE_H` bytes of the frame buffer) so the
        // decoder thread doesn't redo this conversion on every attempt.
        let luma = buf[..Y_BYTES].to_vec();
        let frame = Frame {
            width: CAPTURE_W as u32,
            height: CAPTURE_H as u32,
            rgb,
            luma,
        };

        if let Ok(mut g) = latest.lock() {
            *g = Some(frame);
        }
    }
}

/// Runs QR detection on whatever frame is currently latest. Sleeps briefly
/// when idle or when a result is already waiting for the main thread to
/// consume, so this thread barely costs anything off the scan screens.
fn decoder_loop(
    latest: Arc<Mutex<Option<Frame>>>,
    pending_qr: Arc<Mutex<Option<Vec<u8>>>>,
    diag: Arc<Mutex<crate::camera::ScanDiagnostics>>,
    stop: Arc<AtomicBool>,
    decode_enabled: Arc<AtomicBool>,
) {
    let mut ur_acc = crate::qr::ur_decoder::UrAccumulator::new();
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if !decode_enabled.load(Ordering::Relaxed) {
            ur_acc.reset();
            if let Ok(mut g) = diag.lock() {
                *g = crate::camera::ScanDiagnostics::default();
            }
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        // Skip if a previous decode is still waiting to be consumed.
        let has_pending = pending_qr.lock().ok().map(|g| g.is_some()).unwrap_or(false);
        if has_pending {
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        // Snapshot the latest frame. Clone is ~1MB but avoids holding the
        // lock across the 200-400ms rqrr call.
        let frame = match latest.lock().ok().and_then(|g| g.clone()) {
            Some(f) => f,
            None => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        };
        let (decoded, saw_qr) = crate::camera::try_decode_qr_ur_diag(&frame, &mut ur_acc);
        if saw_qr {
            if let Ok(mut g) = diag.lock() {
                g.last_qr_at = Some(std::time::Instant::now());
                g.ur_progress = ur_acc.progress();
            }
        }
        if let Some(bytes) = decoded {
            if let Ok(mut g) = pending_qr.lock() {
                *g = Some(bytes);
            }
        }
        // Tiny yield so the reader and main threads get scheduler time.
        thread::sleep(Duration::from_millis(10));
    }
}

fn stderr_loop(mut stderr: ChildStderr, buf: Arc<Mutex<Vec<u8>>>, stop: Arc<AtomicBool>) {
    let mut chunk = [0u8; 256];
    while !stop.load(Ordering::Relaxed) {
        match stderr.read(&mut chunk) {
            Ok(0) => return,
            Ok(n) => {
                if let Ok(mut g) = buf.lock() {
                    let available = STDERR_MAX_BYTES.saturating_sub(g.len());
                    let take = n.min(available);
                    if take > 0 {
                        g.extend_from_slice(&chunk[..take]);
                    }
                }
            }
            Err(_) => return,
        }
    }
}

/// Return a sanitized single-line snapshot of the child's stderr for use
/// in the on-screen error panel. Newlines/tabs collapsed to spaces,
/// non-printables stripped, trimmed, capped at 200 chars.
fn stderr_snapshot(buf: &Arc<Mutex<Vec<u8>>>) -> String {
    let Ok(g) = buf.lock() else { return String::new() };
    let s = String::from_utf8_lossy(&g);
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.len() > 200 {
        trimmed[..200].to_string()
    } else {
        trimmed.to_string()
    }
}

/// YUV420 planar -> interleaved RGB using BT.601 coefficients (integer math).
fn yuv420_to_rgb(yuv: &[u8], w: usize, h: usize) -> Vec<u8> {
    let y_plane = &yuv[..w * h];
    let u_plane = &yuv[w * h..w * h + (w / 2) * (h / 2)];
    let v_plane = &yuv[w * h + (w / 2) * (h / 2)..];
    let mut rgb = vec![0u8; w * h * 3];
    for row in 0..h {
        for col in 0..w {
            let yv = y_plane[row * w + col] as i32;
            let ci = (row / 2) * (w / 2) + (col / 2);
            let uv = u_plane[ci] as i32;
            let vv = v_plane[ci] as i32;
            let c = yv - 16;
            let d = uv - 128;
            let e = vv - 128;
            let r = ((298 * c + 409 * e + 128) >> 8).clamp(0, 255) as u8;
            let g = ((298 * c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8;
            let b = ((298 * c + 516 * d + 128) >> 8).clamp(0, 255) as u8;
            let off = (row * w + col) * 3;
            rgb[off] = r;
            rgb[off + 1] = g;
            rgb[off + 2] = b;
        }
    }
    rgb
}

