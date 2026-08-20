//! FRB API modules for openmls.

pub mod config;
pub mod credential;
pub mod group_e2ee;
pub mod init;
pub mod keys;
pub mod message;
pub mod storage;
pub mod types;

// Kept crate-private until the Phase 2 Flutter Rust Bridge contract is reviewed
// and generated. Phase 1 deliberately lands the cryptographic core without an
// accidental public primitive or component-key surface.
#[allow(dead_code)]
pub(crate) mod account_envelope;
pub(crate) mod support;
