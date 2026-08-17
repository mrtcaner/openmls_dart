//! Android transport for native receive v1.
//!
//! The app-owned Java/Kotlin class is a mechanical byte-array shim. All MLS,
//! validation, framing, and error semantics remain in the Rust core.

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass};
use jni::sys::{jbyteArray, jint};
use zeroize::Zeroize;

use crate::native_receive_v1::{
    NATIVE_RECEIVE_CONTRACT_VERSION, NATIVE_RECEIVE_REQUEST_MAX_BYTES, NativeReceiveErrorCodeV1,
    encode_native_receive_failure_v1, execute_native_receive_v1,
};

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_kurtuba_openmls_OpenMlsNativeReceive_nativeExecuteReceiveV1(
    env: JNIEnv,
    _class: JClass,
    request: JByteArray,
) -> jbyteArray {
    let Ok(request_len) = env.get_array_length(&request) else {
        return std::ptr::null_mut();
    };
    if request_len as usize > NATIVE_RECEIVE_REQUEST_MAX_BYTES {
        return response_to_java(
            &env,
            encode_native_receive_failure_v1(None, NativeReceiveErrorCodeV1::LimitExceeded),
        );
    }
    let mut request_bytes = match env.convert_byte_array(&request) {
        Ok(bytes) => bytes,
        Err(_) => return std::ptr::null_mut(),
    };
    let response = execute_native_receive_v1(&request_bytes);
    request_bytes.zeroize();
    response_to_java(&env, response)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_kurtuba_openmls_OpenMlsNativeReceive_nativeZeroize(
    env: JNIEnv,
    _class: JClass,
    bytes: JByteArray,
) {
    let Ok(length) = env.get_array_length(&bytes) else {
        return;
    };
    let zeros = vec![0_i8; length as usize];
    let _ = env.set_byte_array_region(&bytes, 0, &zeros);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_kurtuba_openmls_OpenMlsNativeReceive_nativeContractVersion(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    jint::from(NATIVE_RECEIVE_CONTRACT_VERSION)
}

fn response_to_java(env: &JNIEnv, mut response: Vec<u8>) -> jbyteArray {
    let java_response = env.byte_array_from_slice(&response);
    response.zeroize();
    java_response
        .map(JByteArray::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
