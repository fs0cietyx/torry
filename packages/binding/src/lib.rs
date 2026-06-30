//! NAPI-RS binding layer for Torry.
//!
//! This crate is an **ultra-thin bridge** between Node.js and `torry-core`.
//! It contains ONLY `#[napi]` function signatures and type conversions.
//! All business logic lives in `torry-core`.
//!
//! # Rules for this crate
//!
//! 1. **No business logic** — delegate everything to `torry_core`.
//! 2. **Keep functions small** — convert types, call core, return result.
//! 3. **One `#[napi]` fn** per core function that needs JS exposure.
//! 4. **Error conversion** — map `torry_core::TorryError` to `napi::Error`.

use napi_derive::napi;

/// Returns the version of the underlying Rust core library.
#[napi]
pub fn get_core_version() -> String {
    torry_core::version().to_string()
}

/// Adds two numbers via the Rust core.
///
/// Pipeline verification function. Proves the entire chain works:
/// `TypeScript → NAPI → torry-core → NAPI → TypeScript`
#[napi]
pub fn add(a: i32, b: i32) -> i32 {
    torry_core::download::add(a, b)
}
