//! Shared enums for the caller-owned OpenMLS API.

use openmls::prelude::*;

/// MLS ciphersuite selection.
pub enum MlsCiphersuite {
    Mls128DhkemX25519Aes128gcmSha256Ed25519,
    Mls128DhkemX25519Chacha20poly1305Sha256Ed25519,
    Mls128DhkemP256Aes128gcmSha256P256,
}

/// Wire format policy for MLS messages.
pub enum MlsWireFormatPolicy {
    Plaintext,
    Ciphertext,
}

/// Type of a processed incoming message.
pub enum ProcessedMessageType {
    Application,
    Proposal,
    StagedCommit,
}

/// MLS proposal types.
pub enum MlsProposalType {
    Add,
    Remove,
    Update,
    PreSharedKey,
    Reinit,
    ExternalInit,
    GroupContextExtensions,
    Custom,
}

pub(crate) fn ciphersuite_to_native(ciphersuite: &MlsCiphersuite) -> Ciphersuite {
    match ciphersuite {
        MlsCiphersuite::Mls128DhkemX25519Aes128gcmSha256Ed25519 => {
            Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519
        }
        MlsCiphersuite::Mls128DhkemX25519Chacha20poly1305Sha256Ed25519 => {
            Ciphersuite::MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519
        }
        MlsCiphersuite::Mls128DhkemP256Aes128gcmSha256P256 => {
            Ciphersuite::MLS_128_DHKEMP256_AES128GCM_SHA256_P256
        }
    }
}

pub(crate) fn wire_format_to_native(wire_format: &MlsWireFormatPolicy) -> WireFormatPolicy {
    match wire_format {
        MlsWireFormatPolicy::Plaintext => PURE_PLAINTEXT_WIRE_FORMAT_POLICY,
        MlsWireFormatPolicy::Ciphertext => PURE_CIPHERTEXT_WIRE_FORMAT_POLICY,
    }
}

/// Returns the list of supported ciphersuites.
#[flutter_rust_bridge::frb(sync)]
pub fn supported_ciphersuites() -> Vec<MlsCiphersuite> {
    vec![
        MlsCiphersuite::Mls128DhkemX25519Aes128gcmSha256Ed25519,
        MlsCiphersuite::Mls128DhkemX25519Chacha20poly1305Sha256Ed25519,
        MlsCiphersuite::Mls128DhkemP256Aes128gcmSha256P256,
    ]
}
