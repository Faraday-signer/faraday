//! Faraday ESP32-S3 family — shared firmware.
//!
//! Code common to every ESP32-S3 board (camera, QR decode, BOOT power button,
//! the main event loop). Board-specific pieces (display, battery, touch) are
//! supplied by each board's binary crate through the [`board`] traits and handed
//! to [`run`].

pub mod board;
pub mod camera;
pub mod power;
pub mod qr_decode;
mod run;

pub use board::{BoardBattery, BoardDisplay, BoardTouch, NoBattery, TouchEvent};
pub use run::run;
