#![no_main]
//! Fuzz target for the MLS protocol-message parsers.
//!
//! `mls_message_extract_group_id` / `_epoch` / `_content_type` each run
//! `MlsMessageIn::tls_deserialize_exact_bytes` on attacker-controlled wire bytes
//! (an incoming message straight off the network) to route it before
//! `process_message`. This target proves those parsers never panic on malformed
//! input — returning an `Err` is a success.

use libfuzzer_sys::fuzz_target;
use openmls_frb::api::message::{
    mls_message_content_type, mls_message_extract_epoch, mls_message_extract_group_id,
};

fuzz_target!(|data: &[u8]| {
    let _ = mls_message_extract_group_id(data.to_vec());
    let _ = mls_message_extract_epoch(data.to_vec());
    let _ = mls_message_content_type(data.to_vec());
});
