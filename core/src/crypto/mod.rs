//! Cryptographic primitives: BIP39, SLIP-0010, key derivation.
//!
//! Public API used by both the GUI app and headless sanity checks.
//! Some functions are only exercised by the GUI or tests.
#[allow(dead_code)]
pub mod bip39;
#[allow(dead_code)]
pub mod slip0010;
#[allow(dead_code)]
pub mod derivation;
#[allow(dead_code)]
pub mod pda;
