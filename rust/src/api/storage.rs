//! Operation-scoped MLS storage boundary.
//!
//! Callers provide opaque entries from their durable store. An operation runs
//! against an in-memory snapshot and returns one batch for the caller to apply
//! atomically. This module never opens or writes a database.

use std::collections::HashSet;

use openmls::prelude::tls_codec::Serialize as TlsSerialize;
use openmls::prelude::*;
use zeroize::Zeroize;

use super::config::MlsGroupConfig;
use super::engine::{build_credential_with_key, load_group, mls_message_from_exact_bytes};
use super::keys::signer_from_bytes;
use super::types::{MlsCiphersuite, MlsProposalType, ProcessedMessageType, ciphersuite_to_native};
use crate::encrypted_db::{StorageUpdates, is_global_key};
use crate::snapshot_storage::{SnapshotOpenMlsProvider, SnapshotStorageProvider};

/// Version of the opaque OpenMLS key/value representation used by this API.
///
/// This is intentionally independent from any caller-owned database schema.
pub const MLS_STORAGE_FORMAT_VERSION: u32 = 1;

/// One opaque OpenMLS storage row owned by the caller.
pub struct MlsStorageEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    /// `None` means installation-global state; otherwise this is group state.
    pub group_id: Option<Vec<u8>>,
}

/// All durable changes produced by one successful MLS operation.
pub struct MlsStorageBatch {
    pub upserts: Vec<MlsStorageEntry>,
    pub deletes: Vec<Vec<u8>>,
    pub deleted_group_ids: Vec<Vec<u8>>,
    pub storage_format_version: u32,
}

/// Key package bytes and the state changes that created them.
pub struct CreateKeyPackageWithStorageResult {
    pub key_package_bytes: Vec<u8>,
    pub storage_batch: MlsStorageBatch,
}

pub struct CreateGroupWithStorageResult {
    pub group_id: Vec<u8>,
    pub storage_batch: MlsStorageBatch,
}

pub struct AddMembersWithStorageResult {
    pub commit: Vec<u8>,
    pub welcome: Vec<u8>,
    pub group_info: Option<Vec<u8>>,
    pub storage_batch: MlsStorageBatch,
}

pub struct JoinGroupWithStorageResult {
    pub group_id: Vec<u8>,
    pub storage_batch: MlsStorageBatch,
}

pub struct CreateMessageWithStorageResult {
    pub ciphertext: Vec<u8>,
    pub storage_batch: MlsStorageBatch,
}

pub struct ProcessMessageWithStorageResult {
    pub message_type: ProcessedMessageType,
    pub sender_index: Option<u32>,
    pub epoch: u64,
    pub application_message: Option<Vec<u8>>,
    pub has_staged_commit: bool,
    pub has_proposal: bool,
    pub proposal_type: Option<MlsProposalType>,
    pub storage_batch: MlsStorageBatch,
}

/// Return the only storage format version accepted by this build.
#[flutter_rust_bridge::frb(sync)]
pub fn mls_storage_format_version() -> u32 {
    MLS_STORAGE_FORMAT_VERSION
}

/// Create a key package without performing any durable write.
///
/// On success, the caller must apply the complete returned batch or discard it.
/// On failure, no batch is returned.
pub fn create_key_package_with_storage(
    ciphersuite: MlsCiphersuite,
    signer_bytes: Vec<u8>,
    credential_identity: Vec<u8>,
    signer_public_key: Vec<u8>,
    credential_bytes: Option<Vec<u8>>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<CreateKeyPackageWithStorageResult, String> {
    let provider = provider_from_entries(storage_entries, storage_format_version, None)?;
    let signer = signer_from_bytes(signer_bytes)?;
    let credential_with_key = build_credential_with_key(
        &credential_identity,
        &signer_public_key,
        credential_bytes.as_deref(),
    )?;

    let key_package_bundle = KeyPackage::builder()
        .build(
            ciphersuite_to_native(&ciphersuite),
            &provider,
            &signer,
            credential_with_key,
        )
        .map_err(|e| format!("Failed to create key package: {e}"))?;

    let key_package_bytes = key_package_bundle
        .key_package()
        .tls_serialize_detached()
        .map_err(|e| format!("Failed to serialize key package: {e}"))?;
    let storage_batch = batch_from_provider(provider, None, Vec::new())?;

    Ok(CreateKeyPackageWithStorageResult {
        key_package_bytes,
        storage_batch,
    })
}

/// Create a group from caller-owned global state without writing a database.
#[allow(clippy::too_many_arguments)]
pub fn create_group_with_storage(
    config: MlsGroupConfig,
    signer_bytes: Vec<u8>,
    credential_identity: Vec<u8>,
    signer_public_key: Vec<u8>,
    group_id: Option<Vec<u8>>,
    credential_bytes: Option<Vec<u8>>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<CreateGroupWithStorageResult, String> {
    let provider = provider_from_entries(storage_entries, storage_format_version, None)?;
    let signer = signer_from_bytes(signer_bytes)?;
    let credential_with_key = build_credential_with_key(
        &credential_identity,
        &signer_public_key,
        credential_bytes.as_deref(),
    )?;
    let create_config = config.to_create_config();

    signer
        .store(provider.storage())
        .map_err(|e| format!("Failed to store signer: {e}"))?;

    let group = if let Some(group_id) = group_id {
        MlsGroup::new_with_group_id(
            &provider,
            &signer,
            &create_config,
            GroupId::from_slice(&group_id),
            credential_with_key,
        )
    } else {
        MlsGroup::new(&provider, &signer, &create_config, credential_with_key)
    }
    .map_err(|e| format!("Failed to create group: {e}"))?;

    let group_id = group.group_id().as_slice().to_vec();
    let storage_batch = batch_from_provider(provider, Some(group_id.clone()), Vec::new())?;

    Ok(CreateGroupWithStorageResult {
        group_id,
        storage_batch,
    })
}

/// Add members and merge the pending commit against caller-owned group state.
///
/// Each validated KeyPackage must contain a Basic Credential whose identity
/// exactly matches the corresponding caller-supplied expected identity. A
/// mismatch fails before group state changes are returned.
pub fn add_members_with_storage(
    group_id: Vec<u8>,
    signer_bytes: Vec<u8>,
    key_packages_bytes: Vec<Vec<u8>>,
    expected_credential_identities: Vec<Vec<u8>>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<AddMembersWithStorageResult, String> {
    let provider = provider_from_entries(storage_entries, storage_format_version, Some(&group_id))?;
    let signer = signer_from_bytes(signer_bytes)?;

    if key_packages_bytes.len() != expected_credential_identities.len() {
        return Err(format!(
            "Key package count ({}) does not match expected credential identity count ({})",
            key_packages_bytes.len(),
            expected_credential_identities.len()
        ));
    }

    let mut group = load_group(&group_id, &provider)?;

    let mut key_packages = Vec::with_capacity(key_packages_bytes.len());
    for (key_package_bytes, expected_credential_identity) in key_packages_bytes
        .into_iter()
        .zip(expected_credential_identities)
    {
        let key_package = KeyPackageIn::tls_deserialize_exact_bytes(&key_package_bytes)
            .map_err(|e| format!("Failed to deserialize key package: {e}"))?
            .validate(provider.crypto(), ProtocolVersion::Mls10)
            .map_err(|e| format!("Failed to validate key package: {e}"))?;
        let credential = BasicCredential::try_from(key_package.leaf_node().credential().clone())
            .map_err(|_| "Key package does not contain a Basic Credential".to_string())?;
        if credential.identity() != expected_credential_identity {
            return Err(
                "Key package credential identity does not match the expected identity".to_string(),
            );
        }
        key_packages.push(key_package);
    }

    let (commit, welcome, group_info) = group
        .add_members(&provider, &signer, &key_packages)
        .map_err(|e| format!("Failed to add members: {e}"))?;
    group
        .merge_pending_commit(&provider)
        .map_err(|e| format!("Failed to merge pending commit: {e}"))?;

    let commit = commit
        .tls_serialize_detached()
        .map_err(|e| format!("Failed to serialize commit: {e}"))?;
    let welcome = welcome
        .tls_serialize_detached()
        .map_err(|e| format!("Failed to serialize welcome: {e}"))?;
    let group_info = group_info
        .map(|message| message.tls_serialize_detached())
        .transpose()
        .map_err(|e| format!("Failed to serialize group info: {e}"))?;
    let storage_batch = batch_from_provider(provider, Some(group_id), Vec::new())?;

    Ok(AddMembersWithStorageResult {
        commit,
        welcome,
        group_info,
        storage_batch,
    })
}

/// Join a group from a Welcome using caller-owned global state.
pub fn join_group_from_welcome_with_storage(
    config: MlsGroupConfig,
    welcome_bytes: Vec<u8>,
    ratchet_tree_bytes: Option<Vec<u8>>,
    signer_bytes: Vec<u8>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<JoinGroupWithStorageResult, String> {
    let provider = provider_from_entries(storage_entries, storage_format_version, None)?;
    let signer = signer_from_bytes(signer_bytes)?;

    signer
        .store(provider.storage())
        .map_err(|e| format!("Failed to store signer: {e}"))?;

    let welcome_message = mls_message_from_exact_bytes(&welcome_bytes)
        .map_err(|e| format!("Failed to deserialize welcome: {e}"))?;
    let welcome = match welcome_message.extract() {
        MlsMessageBodyIn::Welcome(welcome) => welcome,
        _ => return Err("Message is not a Welcome".to_string()),
    };
    let ratchet_tree: Option<RatchetTreeIn> = ratchet_tree_bytes
        .map(|bytes| {
            RatchetTreeIn::tls_deserialize_exact_bytes(&bytes)
                .map_err(|e| format!("Failed to deserialize ratchet tree: {e}"))
        })
        .transpose()?;

    let staged =
        StagedWelcome::new_from_welcome(&provider, &config.to_join_config(), welcome, ratchet_tree)
            .map_err(|e| format!("Failed to process welcome: {e}"))?;
    let group = staged
        .into_group(&provider)
        .map_err(|e| format!("Failed to join group from welcome: {e}"))?;
    let group_id = group.group_id().as_slice().to_vec();
    let storage_batch = batch_from_provider(provider, Some(group_id.clone()), Vec::new())?;

    Ok(JoinGroupWithStorageResult {
        group_id,
        storage_batch,
    })
}

/// Create an application message and return its sender-state changes.
pub fn create_message_with_storage(
    group_id: Vec<u8>,
    signer_bytes: Vec<u8>,
    message: Vec<u8>,
    aad: Option<Vec<u8>>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<CreateMessageWithStorageResult, String> {
    let provider = provider_from_entries(storage_entries, storage_format_version, Some(&group_id))?;
    let signer = signer_from_bytes(signer_bytes)?;
    let mut group = load_group(&group_id, &provider)?;

    if let Some(aad) = aad {
        group.set_aad(aad);
    }

    let ciphertext = group
        .create_message(&provider, &signer, &message)
        .map_err(|e| format!("Failed to create message: {e}"))?
        .tls_serialize_detached()
        .map_err(|e| format!("Failed to serialize message: {e}"))?;
    let storage_batch = batch_from_provider(provider, Some(group_id), Vec::new())?;

    Ok(CreateMessageWithStorageResult {
        ciphertext,
        storage_batch,
    })
}

/// Process an application, proposal, or commit message against caller state.
///
/// When `expected_aad` is present, the authenticated message AAD must match it
/// byte-for-byte. A mismatch returns no storage batch.
pub fn process_message_with_storage(
    group_id: Vec<u8>,
    message_bytes: Vec<u8>,
    expected_aad: Option<Vec<u8>>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<ProcessMessageWithStorageResult, String> {
    let provider = provider_from_entries(storage_entries, storage_format_version, Some(&group_id))?;
    let mut group = load_group(&group_id, &provider)?;
    let message = mls_message_from_exact_bytes(&message_bytes)
        .map_err(|e| format!("Failed to deserialize message: {e}"))?
        .try_into_protocol_message()
        .map_err(|e| format!("Not a protocol message: {e}"))?;
    let processed = group
        .process_message(&provider, message)
        .map_err(|e| format!("Failed to process message: {e}"))?;

    if let Some(expected_aad) = expected_aad
        && processed.aad() != expected_aad
    {
        return Err("Message AAD does not match the expected AAD".to_string());
    }

    let sender_index = match processed.sender() {
        Sender::Member(index) => Some(index.u32()),
        _ => None,
    };
    let epoch = group.epoch().as_u64();
    let (message_type, application_message, has_staged_commit, has_proposal, proposal_type) =
        match processed.into_content() {
            ProcessedMessageContent::ApplicationMessage(message) => (
                ProcessedMessageType::Application,
                Some(message.into_bytes()),
                false,
                false,
                None,
            ),
            ProcessedMessageContent::StagedCommitMessage(commit) => {
                group
                    .merge_staged_commit(&provider, *commit)
                    .map_err(|e| format!("Failed to merge staged commit: {e}"))?;
                (ProcessedMessageType::StagedCommit, None, true, false, None)
            }
            ProcessedMessageContent::ProposalMessage(proposal) => {
                let proposal_type = match proposal.proposal() {
                    Proposal::Add(_) => MlsProposalType::Add,
                    Proposal::Remove(_) => MlsProposalType::Remove,
                    Proposal::Update(_) => MlsProposalType::Update,
                    Proposal::PreSharedKey(_) => MlsProposalType::PreSharedKey,
                    Proposal::ReInit(_) => MlsProposalType::Reinit,
                    Proposal::ExternalInit(_) => MlsProposalType::ExternalInit,
                    Proposal::GroupContextExtensions(_) => MlsProposalType::GroupContextExtensions,
                    _ => MlsProposalType::Custom,
                };
                group
                    .store_pending_proposal(provider.storage(), *proposal)
                    .map_err(|e| format!("Failed to store pending proposal: {e}"))?;
                (
                    ProcessedMessageType::Proposal,
                    None,
                    false,
                    true,
                    Some(proposal_type),
                )
            }
            _ => return Err("Unknown processed message content type".to_string()),
        };
    let storage_batch = batch_from_provider(provider, Some(group_id), Vec::new())?;

    Ok(ProcessMessageWithStorageResult {
        message_type,
        sender_index,
        epoch,
        application_message,
        has_staged_commit,
        has_proposal,
        proposal_type,
        storage_batch,
    })
}

/// Delete a group and represent the complete group removal in one batch.
pub fn delete_group_with_storage(
    group_id: Vec<u8>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<MlsStorageBatch, String> {
    let provider = provider_from_entries(storage_entries, storage_format_version, Some(&group_id))?;
    let mut group = load_group(&group_id, &provider)?;
    group
        .delete(provider.storage())
        .map_err(|e| format!("Failed to delete group: {e}"))?;

    batch_from_provider(provider, Some(group_id.clone()), vec![group_id])
}

pub(crate) fn provider_from_entries(
    mut entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
    expected_group_id: Option<&[u8]>,
) -> Result<SnapshotOpenMlsProvider, String> {
    if storage_format_version != MLS_STORAGE_FORMAT_VERSION {
        zeroize_entry_values(&mut entries);
        return Err(format!(
            "Unsupported MLS storage format version {storage_format_version}; expected {MLS_STORAGE_FORMAT_VERSION}"
        ));
    }

    let mut seen_keys = HashSet::with_capacity(entries.len());
    let validation = entries.iter().try_for_each(|entry| {
        if !seen_keys.insert(entry.key.clone()) {
            return Err("Duplicate MLS storage key in snapshot".to_string());
        }

        if is_global_key(&entry.key) {
            if entry.group_id.is_some() {
                return Err("Global MLS storage entry must not have a group ID".to_string());
            }
        } else {
            let expected = expected_group_id.ok_or_else(|| {
                "Group-scoped MLS storage entry supplied to a global operation".to_string()
            })?;
            if entry.group_id.as_deref() != Some(expected) {
                return Err("MLS storage entry belongs to a different group".to_string());
            }
        }
        Ok(())
    });
    if let Err(error) = validation {
        zeroize_entry_values(&mut entries);
        return Err(error);
    }

    let snapshot_entries = entries
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect();

    Ok(SnapshotOpenMlsProvider::new(
        SnapshotStorageProvider::from_entries(snapshot_entries),
    ))
}

pub(crate) fn batch_from_provider(
    provider: SnapshotOpenMlsProvider,
    group_id: Option<Vec<u8>>,
    deleted_group_ids: Vec<Vec<u8>>,
) -> Result<MlsStorageBatch, String> {
    batch_from_updates(
        provider.into_storage().into_updates(),
        group_id,
        deleted_group_ids,
    )
}

fn batch_from_updates(
    mut updates: StorageUpdates,
    group_id: Option<Vec<u8>>,
    deleted_group_ids: Vec<Vec<u8>>,
) -> Result<MlsStorageBatch, String> {
    if group_id.is_none()
        && updates
            .upserts
            .iter()
            .any(|(key, _value)| !is_global_key(key))
    {
        for (_key, value) in &mut updates.upserts {
            value.zeroize();
        }
        return Err("MLS operation produced group state without a group ID".to_string());
    }

    let mut upserts = Vec::with_capacity(updates.upserts.len());
    for (key, value) in updates.upserts {
        let entry_group_id = if is_global_key(&key) {
            None
        } else {
            Some(group_id.clone().expect("group ID prechecked above"))
        };
        upserts.push(MlsStorageEntry {
            key,
            value,
            group_id: entry_group_id,
        });
    }

    upserts.sort_by(|left, right| left.key.cmp(&right.key));
    let mut deletes = updates.deletes;
    deletes.sort();

    Ok(MlsStorageBatch {
        upserts,
        deletes,
        deleted_group_ids,
        storage_format_version: MLS_STORAGE_FORMAT_VERSION,
    })
}

fn zeroize_entry_values(entries: &mut [MlsStorageEntry]) {
    for entry in entries {
        entry.value.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &[u8], group_id: Option<&[u8]>) -> MlsStorageEntry {
        MlsStorageEntry {
            key: key.to_vec(),
            value: b"value".to_vec(),
            group_id: group_id.map(<[u8]>::to_vec),
        }
    }

    #[test]
    fn rejects_unknown_storage_format() {
        let result = provider_from_entries(Vec::new(), 99, None);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .contains("Unsupported MLS storage format")
        );
    }

    #[test]
    fn rejects_duplicate_keys() {
        let result = provider_from_entries(
            vec![entry(b"KeyPackage-a", None), entry(b"KeyPackage-a", None)],
            MLS_STORAGE_FORMAT_VERSION,
            None,
        );
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Duplicate MLS storage key"));
    }

    #[test]
    fn validates_global_and_group_scope() {
        assert!(
            provider_from_entries(
                vec![
                    entry(b"KeyPackage-a", None),
                    entry(b"Tree-a", Some(b"group-a"))
                ],
                MLS_STORAGE_FORMAT_VERSION,
                Some(b"group-a"),
            )
            .is_ok()
        );

        assert!(
            provider_from_entries(
                vec![entry(b"KeyPackage-a", Some(b"group-a"))],
                MLS_STORAGE_FORMAT_VERSION,
                Some(b"group-a"),
            )
            .is_err()
        );

        assert!(
            provider_from_entries(
                vec![entry(b"Tree-a", Some(b"group-b"))],
                MLS_STORAGE_FORMAT_VERSION,
                Some(b"group-a"),
            )
            .is_err()
        );
    }

    #[test]
    fn batch_is_scoped_and_sorted() {
        let batch = batch_from_updates(
            StorageUpdates {
                upserts: vec![
                    (b"Tree-z".to_vec(), b"group".to_vec()),
                    (b"KeyPackage-a".to_vec(), b"global".to_vec()),
                ],
                deletes: vec![b"z".to_vec(), b"a".to_vec()],
            },
            Some(b"group-a".to_vec()),
            Vec::new(),
        )
        .unwrap();

        assert_eq!(batch.upserts[0].key, b"KeyPackage-a");
        assert_eq!(batch.upserts[0].group_id, None);
        assert_eq!(batch.upserts[1].key, b"Tree-z");
        assert_eq!(
            batch.upserts[1].group_id.as_deref(),
            Some(b"group-a".as_slice())
        );
        assert_eq!(batch.deletes, vec![b"a".to_vec(), b"z".to_vec()]);
        assert_eq!(batch.storage_format_version, MLS_STORAGE_FORMAT_VERSION);
    }

    #[test]
    fn group_update_requires_a_group_id() {
        let result = batch_from_updates(
            StorageUpdates {
                upserts: vec![(b"Tree-a".to_vec(), b"value".to_vec())],
                deletes: Vec::new(),
            },
            None,
            Vec::new(),
        );
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("without a group ID"));
    }
}
