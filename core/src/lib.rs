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
