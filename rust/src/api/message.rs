//! Standalone MLS wire-message inspection helpers.

use openmls::prelude::ContentType;

use super::support::mls_message_from_exact_bytes;

/// Extract the group ID from an MLS protocol message.
#[flutter_rust_bridge::frb(sync)]
pub fn mls_message_extract_group_id(message_bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let message = mls_message_from_exact_bytes(&message_bytes)
        .map_err(|e| format!("Failed to deserialize message: {e}"))?;
    let protocol_message = message
        .try_into_protocol_message()
        .map_err(|e| format!("Not a protocol message: {e}"))?;
    Ok(protocol_message.group_id().as_slice().to_vec())
}

/// Extract the epoch from an MLS protocol message.
#[flutter_rust_bridge::frb(sync)]
pub fn mls_message_extract_epoch(message_bytes: Vec<u8>) -> Result<u64, String> {
    let message = mls_message_from_exact_bytes(&message_bytes)
        .map_err(|e| format!("Failed to deserialize message: {e}"))?;
    let protocol_message = message
        .try_into_protocol_message()
        .map_err(|e| format!("Not a protocol message: {e}"))?;
    Ok(protocol_message.epoch().as_u64())
}

/// Return `application`, `proposal`, or `commit` for a protocol message.
#[flutter_rust_bridge::frb(sync)]
pub fn mls_message_content_type(message_bytes: Vec<u8>) -> Result<String, String> {
    let message = mls_message_from_exact_bytes(&message_bytes)
        .map_err(|e| format!("Failed to deserialize message: {e}"))?;
    let protocol_message = message
        .try_into_protocol_message()
        .map_err(|e| format!("Not a protocol message: {e}"))?;
    let content_type = match protocol_message.content_type() {
        ContentType::Application => "application",
        ContentType::Proposal => "proposal",
        ContentType::Commit => "commit",
    };
    Ok(content_type.to_string())
}
