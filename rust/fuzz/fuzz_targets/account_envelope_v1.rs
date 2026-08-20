#![no_main]

use libfuzzer_sys::fuzz_target;
use openmls_frb::api::account_envelope::fuzz_decode_account_envelope_v1;

fuzz_target!(|data: &[u8]| {
    fuzz_decode_account_envelope_v1(data);
});
