//! Apple transport for native receive v1.
//!
//! One framework embeds this Rust implementation for both the foreground app
//! and its Notification Service Extension. The caller owns the returned
//! allocation and must free it exactly once with `openmls_receive_v1_free`.

use std::slice;

use zeroize::Zeroize;

use crate::native_receive_v1::{
    NATIVE_RECEIVE_CONTRACT_VERSION, NATIVE_RECEIVE_REQUEST_MAX_BYTES, NativeReceiveErrorCodeV1,
    encode_native_receive_failure_v1, execute_native_receive_v1,
};

#[repr(C)]
pub struct OpenMlsReceiveV1Buffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl OpenMlsReceiveV1Buffer {
    fn empty() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openmls_receive_v1_execute(
    request_data: *const u8,
    request_len: usize,
) -> OpenMlsReceiveV1Buffer {
    if request_len > NATIVE_RECEIVE_REQUEST_MAX_BYTES {
        return into_buffer(encode_native_receive_failure_v1(
            None,
            NativeReceiveErrorCodeV1::LimitExceeded,
        ));
    }
    if request_data.is_null() {
        if request_len != 0 {
            return into_buffer(encode_native_receive_failure_v1(
                None,
                NativeReceiveErrorCodeV1::InvalidFrame,
            ));
        }
        return into_buffer(execute_native_receive_v1(&[]));
    }

    // SAFETY: The caller promises a readable allocation for the duration of
    // this call. Rust copies it immediately and never retains the pointer.
    let mut request = unsafe { slice::from_raw_parts(request_data, request_len) }.to_vec();
    let response = execute_native_receive_v1(&request);
    request.zeroize();
    into_buffer(response)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openmls_receive_v1_free(buffer: OpenMlsReceiveV1Buffer) {
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
pub extern "C" fn openmls_receive_v1_version() -> u16 {
    NATIVE_RECEIVE_CONTRACT_VERSION
}

fn into_buffer(mut bytes: Vec<u8>) -> OpenMlsReceiveV1Buffer {
    if bytes.is_empty() {
        return OpenMlsReceiveV1Buffer::empty();
    }
    let buffer = OpenMlsReceiveV1Buffer {
        data: bytes.as_mut_ptr(),
        len: bytes.len(),
        capacity: bytes.capacity(),
    };
    std::mem::forget(bytes);
    buffer
}
