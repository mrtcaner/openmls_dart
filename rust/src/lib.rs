//! openmls_frb - Rust bridge layer for openmls.
//!
//! Dart wrapper for OpenMLS — a Rust implementation of the Messaging Layer Security (MLS) protocol (RFC 9420)

#![allow(dead_code)]

mod snapshot_storage;
mod local_group_state;
// The generated bridge legitimately needs unsafe. Everything hand-written is
// covered by `unsafe_code = "deny"` in Cargo.toml.
#[allow(unsafe_code)]
mod frb_generated;
mod utils;

pub mod api;

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
mod native_receive_android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
#[allow(unsafe_code)]
mod native_receive_apple;
#[cfg(any(test, feature = "native-receive-fixtures"))]
/// cbindgen:ignore
pub mod native_receive_v1;
pub mod native_receive_v2;
#[cfg(any(test, feature = "native-receive-fixtures"))]
/// cbindgen:ignore
pub mod native_receive_v1_vectors;
pub use utils::current_time;
