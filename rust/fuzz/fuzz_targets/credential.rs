#![no_main]
//! Fuzz target for MLS credential deserialization.
//!
//! `MlsCredential::deserialize` TLS-parses attacker-controlled bytes — a peer's
//! credential arrives over the wire inside key packages and leaf nodes. This
//! target proves the parser never panics on malformed input (an `Err` return is
//! a success).

use libfuzzer_sys::fuzz_target;
use openmls_frb::api::credential::MlsCredential;

fuzz_target!(|data: &[u8]| {
    let _ = MlsCredential::deserialize(data.to_vec());
});
