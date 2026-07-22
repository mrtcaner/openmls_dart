//! openmls_frb - Rust bridge layer for openmls.
//!
//! Dart wrapper for OpenMLS — a Rust implementation of the Messaging Layer Security (MLS) protocol (RFC 9420)

#![allow(dead_code)]

mod snapshot_storage;
// The generated bridge legitimately needs unsafe. Everything hand-written is
// covered by `unsafe_code = "deny"` in Cargo.toml.
#[allow(unsafe_code)]
mod frb_generated;
mod utils;

pub mod api;

pub use utils::current_time;
