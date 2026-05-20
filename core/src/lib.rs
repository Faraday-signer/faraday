//! Faraday core — shared platform-agnostic library.
//!
//! Contains crypto, transaction parsing, QR encoding/decoding,
//! UI widgets, and the application state machine. Platform-specific
//! code (display drivers, input hardware, camera backends) lives in
//! the per-target binary crates (`raspberry-pi/`, `esp32/`).
