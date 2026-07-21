//! openmls_frb - Rust bridge layer for openmls.
//!
//! Dart wrapper for OpenMLS — a Rust implementation of the Messaging Layer Security (MLS) protocol (RFC 9420)

#![allow(dead_code)]

mod encrypted_db;
mod snapshot_storage;
// The FRB-generated bridge and a couple of hand-written modules
// (encrypted_db's WASM `unsafe impl Send/Sync`) legitimately need unsafe;
// they carry their own `#[allow(unsafe_code)]` / `#![allow(unsafe_code)]`.
// Everything else is covered by `unsafe_code = "deny"` in Cargo.toml.
#[allow(unsafe_code)]
mod frb_generated;
mod utils;

pub mod api;

pub use utils::current_time;
