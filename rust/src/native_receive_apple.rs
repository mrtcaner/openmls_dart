//! Apple transport for native receive v2.
//!
//! One framework embeds this Rust implementation for both the foreground app
//! and its Notification Service Extension. The caller owns the returned
//! allocation and must free it exactly once with `openmls_receive_v2_free`.

use std::slice;

use zeroize::Zeroize;

use crate::native_receive_v2::{
    NATIVE_RECEIVE_CONTRACT_VERSION, NATIVE_RECEIVE_REQUEST_MAX_BYTES, NativeReceiveErrorCodeV2,
    encode_native_receive_failure_v2, execute_native_receive_v2,
};

#[repr(C)]
pub struct OpenMlsReceiveV2Buffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl OpenMlsReceiveV2Buffer {
    fn empty() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openmls_receive_v2_execute(
    request_data: *const u8,
    request_len: usize,
) -> OpenMlsReceiveV2Buffer {
    if request_len > NATIVE_RECEIVE_REQUEST_MAX_BYTES {
        return into_buffer(encode_native_receive_failure_v2(
            None,
            NativeReceiveErrorCodeV2::LimitExceeded,
        ));
    }
    if request_data.is_null() {
        if request_len != 0 {
            return into_buffer(encode_native_receive_failure_v2(
                None,
                NativeReceiveErrorCodeV2::InvalidFrame,
            ));
        }
        return into_buffer(execute_native_receive_v2(&[]));
    }

    // SAFETY: The caller promises a readable allocation for the duration of
    // this call. Rust copies it immediately and never retains the pointer.
    let mut request = unsafe { slice::from_raw_parts(request_data, request_len) }.to_vec();
    let response = execute_native_receive_v2(&request);
    request.zeroize();
    into_buffer(response)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openmls_receive_v2_free(buffer: OpenMlsReceiveV2Buffer) {
    if buffer.data.is_null() {
        return;
    }
    if buffer.capacity < buffer.len {
        return;
    }
    // SAFETY: Only a buffer returned by `into_buffer` may be passed here once.
    let mut bytes = unsafe { Vec::from_raw_parts(buffer.data, buffer.len, buffer.capacity) };
    bytes.zeroize();
}

#[unsafe(no_mangle)]
pub extern "C" fn openmls_receive_v2_version() -> u16 {
    NATIVE_RECEIVE_CONTRACT_VERSION
}

fn into_buffer(mut bytes: Vec<u8>) -> OpenMlsReceiveV2Buffer {
    if bytes.is_empty() {
        return OpenMlsReceiveV2Buffer::empty();
    }
    let buffer = OpenMlsReceiveV2Buffer {
        data: bytes.as_mut_ptr(),
        len: bytes.len(),
        capacity: bytes.capacity(),
    };
    std::mem::forget(bytes);
    buffer
}

/// Identical allocation layout, separate versioned entry points.
pub type OpenMlsReceiveV3Buffer = OpenMlsReceiveV2Buffer;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openmls_receive_v3_execute(
    request_data: *const u8,
    request_len: usize,
) -> OpenMlsReceiveV3Buffer {
    use crate::native_receive_v3::{encode_native_receive_failure_v3, execute_native_receive_v3};
    if request_len > NATIVE_RECEIVE_REQUEST_MAX_BYTES {
        return into_buffer(encode_native_receive_failure_v3(
            0,
            NativeReceiveErrorCodeV2::LimitExceeded,
        ));
    }
    if request_data.is_null() {
        return into_buffer(if request_len == 0 {
            execute_native_receive_v3(&[])
        } else {
            encode_native_receive_failure_v3(0, NativeReceiveErrorCodeV2::InvalidFrame)
        });
    }
    // SAFETY: Same caller readable-allocation contract as v2; never retain it.
    let mut request = unsafe { slice::from_raw_parts(request_data, request_len) }.to_vec();
    let response = execute_native_receive_v3(&request);
    request.zeroize();
    into_buffer(response)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openmls_receive_v3_free(buffer: OpenMlsReceiveV3Buffer) {
    // SAFETY: v3 returns the exact same owned allocation layout as v2. Caller
    // must supply one unmodified returned allocation exactly once.
    unsafe {
        openmls_receive_v2_free(buffer);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openmls_receive_v3_version() -> u16 {
    crate::native_receive_v3::NATIVE_RECEIVE_CONTRACT_VERSION
}

#[cfg(test)]
mod v3_adapter_tests {
    use super::*;
    use crate::native_receive_v3::{encode_native_receive_failure_v3, execute_native_receive_v3};

    // SAFETY: Each helper receives one unchanged, live adapter allocation,
    // compares only its initialized bytes, and frees it exactly once.
    unsafe fn assert_response(buffer: OpenMlsReceiveV3Buffer, expected: &[u8]) {
        assert!(!buffer.data.is_null());
        assert!(buffer.capacity >= buffer.len);
        assert_eq!(
            unsafe { slice::from_raw_parts(buffer.data, buffer.len) },
            expected
        );
        unsafe { openmls_receive_v3_free(buffer) };
    }

    #[test]
    fn v3_adapter_preserves_versions_and_failure_frames() {
        assert_eq!(openmls_receive_v2_version(), 2);
        assert_eq!(openmls_receive_v3_version(), 3);
        // SAFETY: The request is readable throughout each call; returned
        // allocations are consumed once by assert_response.
        unsafe {
            assert_response(
                openmls_receive_v3_execute(std::ptr::null(), 0),
                &execute_native_receive_v3(&[]),
            );
            assert_response(
                openmls_receive_v3_execute(std::ptr::null(), 1),
                &encode_native_receive_failure_v3(0, NativeReceiveErrorCodeV2::InvalidFrame),
            );
            // Oversize must fail before inspecting even a null input pointer.
            assert_response(
                openmls_receive_v3_execute(std::ptr::null(), NATIVE_RECEIVE_REQUEST_MAX_BYTES + 1),
                &encode_native_receive_failure_v3(0, NativeReceiveErrorCodeV2::LimitExceeded),
            );
            let mut request = *b"KMLS\x00\x02\x01\x00\x00\x00\x00\x00";
            let original = request;
            assert_response(
                openmls_receive_v3_execute(request.as_ptr(), request.len()),
                &execute_native_receive_v3(&request),
            );
            assert_eq!(
                request, original,
                "borrowed caller input must remain unchanged"
            );
            request.zeroize();
        }
    }
}
