#![cfg_attr(not(feature = "hardware-sha512"), forbid(unsafe_code))]
#![cfg_attr(feature = "hardware-sha512", deny(unsafe_code))]
//! Faraday core — shared platform-agnostic library.
//!
//! Contains crypto, transaction parsing, QR encoding/decoding,
//! and camera types. Platform-specific code (display drivers, input
//! hardware, camera backends) lives in the per-target binary crates.

pub mod crypto;
pub mod parser;
pub mod signer;
pub mod qr;
pub mod camera;
pub mod gui;
pub mod ui;

/// Compiled-in state of the ESP32-only `touch-ui` feature (bottom action bar
/// instead of the physical-key edge gutter). Exposed so key-based targets can
/// assert at compile time that Cargo feature unification (e.g. a
/// `cargo build --workspace` that pulls the ESP32 crate into the graph) hasn't
/// silently enabled it for them and changed their chrome.
pub const TOUCH_UI: bool = cfg!(feature = "touch-ui");

/// Compiled-in state of the ESP32-only `hardware-sha512` feature (mbedtls seed
/// derivation). Exposed for the same compile-time guard as [`TOUCH_UI`].
pub const HARDWARE_SHA512: bool = cfg!(feature = "hardware-sha512");
