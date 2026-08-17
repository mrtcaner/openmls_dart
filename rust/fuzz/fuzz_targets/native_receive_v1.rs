#![no_main]
//! Fuzz the bounded native receive frame, canonical-CBOR preflight, and panic
//! containment. Every input must produce a framed outcome without unwinding.

use libfuzzer_sys::fuzz_target;
use openmls_frb::native_receive_v1::execute_native_receive_v1;

fuzz_target!(|data: &[u8]| {
    let _ = execute_native_receive_v1(data);
});
