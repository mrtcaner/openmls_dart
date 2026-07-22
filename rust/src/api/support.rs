//! Internal helpers shared by the caller-storage and wire-message APIs.

use openmls::prelude::tls_codec::{
    Deserialize as TlsDeserialize, DeserializeBytes as TlsDeserializeBytes, Error as TlsCodecError,
};
use openmls::prelude::*;
use openmls_traits::OpenMlsProvider;

use crate::snapshot_storage::SnapshotOpenMlsProvider;

/// Build a CredentialWithKey from a serialized credential or Basic identity.
pub(crate) fn build_credential_with_key(
    credential_identity: &[u8],
    signer_public_key: &[u8],
    credential_bytes: Option<&[u8]>,
) -> Result<CredentialWithKey, String> {
    let credential = match credential_bytes {
        Some(bytes) => Credential::tls_deserialize_exact_bytes(bytes)
            .map_err(|e| format!("Failed to deserialize credential: {e}"))?,
        None => BasicCredential::new(credential_identity.to_vec()).into(),
    };
    Ok(CredentialWithKey {
        credential,
        signature_key: SignaturePublicKey::from(signer_public_key.to_vec()),
    })
}

/// Parse exact MLS wire bytes without the upstream exact-byte panic path.
pub(crate) fn mls_message_from_exact_bytes(bytes: &[u8]) -> Result<MlsMessageIn, TlsCodecError> {
    let mut reader = bytes;
    let message = MlsMessageIn::tls_deserialize(&mut reader)?;
    if !reader.is_empty() {
        return Err(TlsCodecError::TrailingData);
    }
    Ok(message)
}

/// Load a group from one caller-supplied storage snapshot.
pub(crate) fn load_group(
    group_id: &[u8],
    provider: &SnapshotOpenMlsProvider,
) -> Result<MlsGroup, String> {
    let group_id = GroupId::from_slice(group_id);
    MlsGroup::load(provider.storage(), &group_id)
        .map_err(|e| format!("Failed to load group: {e}"))?
        .ok_or_else(|| "No group found in storage".to_string())
}
