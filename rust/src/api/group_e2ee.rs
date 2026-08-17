//! Strict variable-roster operations for caller-owned MLS storage.
//!
//! Preparation validates an authenticated base roster and exact authorized
//! delta, then returns OpenMLS-selected candidate state. Join and receive
//! operations validate already-canonical resulting roster authority before
//! returning any storage batch.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use openmls::prelude::tls_codec::Serialize as TlsSerialize;
use openmls::prelude::*;
use openmls_traits::OpenMlsProvider;
use openmls_traits::storage::StorageProvider;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::config::MlsGroupConfig;
use super::keys::{deserialize_signer_bytes, signer_from_bytes};
use super::storage::{
    MlsStorageBatch, MlsStorageEntry, batch_from_provider, provider_from_entries,
    validate_storage_entries, zeroize_entry_values,
};
use super::support::{build_credential_with_key, load_group, mls_message_from_exact_bytes};
use super::types::{MlsProposalType, ProcessedMessageType};
use crate::snapshot_storage::is_global_key;

const ROSTER_DOMAIN_V1: &[u8] = b"openmls_dart/roster-summary/v1\0";
const GROUP_STATE_DOMAIN_V1: &[u8] = b"openmls_dart/group-state/v1\0";
type RequestedAddition = (Vec<u8>, Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MlsRosterLeafV1 {
    pub leaf_index: u32,
    pub credential_identity: Vec<u8>,
    pub signature_public_key: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MlsRosterSummaryV1 {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub leaves: Vec<MlsRosterLeafV1>,
    pub digest_sha256: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MlsExpectedRosterStateV1 {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub digest_sha256: Vec<u8>,
}

pub struct MlsAuthorizedKeyPackageV1 {
    pub key_package_bytes: Vec<u8>,
    pub expected_credential_identity: Vec<u8>,
    pub expected_signature_public_key: Vec<u8>,
}

pub struct MlsAuthorizedRemovalV1 {
    pub leaf_index: u32,
    pub expected_credential_identity: Vec<u8>,
    pub expected_signature_public_key: Vec<u8>,
}

pub struct MlsAuthorizedOwnerV1 {
    pub expected_credential_identity: Vec<u8>,
    pub expected_signature_public_key: Vec<u8>,
}

pub struct MlsAuthorizedSelfV1 {
    pub leaf_index: u32,
    pub expected_credential_identity: Vec<u8>,
    pub expected_signature_public_key: Vec<u8>,
}

pub struct CreateGroupWithStorageResult {
    pub group_id: Vec<u8>,
    pub resulting_roster: MlsRosterSummaryV1,
    pub storage_batch: MlsStorageBatch,
}

pub struct JoinGroupWithStorageResult {
    pub group_id: Vec<u8>,
    pub resulting_roster: MlsRosterSummaryV1,
    pub storage_batch: MlsStorageBatch,
}

pub struct PreparedCommitWithStorageResult {
    pub commit: Vec<u8>,
    pub welcome: Option<Vec<u8>>,
    pub group_info: Option<Vec<u8>>,
    pub commit_sha256: Vec<u8>,
    pub previous_roster: MlsRosterSummaryV1,
    pub resulting_roster: MlsRosterSummaryV1,
    pub base_group_state_sha256: Vec<u8>,
    pub storage_batch: MlsStorageBatch,
}

pub struct ProcessMessageWithStorageResult {
    pub message_type: ProcessedMessageType,
    pub sender_index: Option<u32>,
    pub previous_epoch: u64,
    pub resulting_epoch: u64,
    pub application_message: Option<Vec<u8>>,
    pub has_staged_commit: bool,
    pub has_proposal: bool,
    pub proposal_type: Option<MlsProposalType>,
    pub previous_roster: MlsRosterSummaryV1,
    pub resulting_roster: MlsRosterSummaryV1,
    pub storage_batch: MlsStorageBatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StrictReceiveErrorKind {
    StorageFormatMismatch,
    InvalidStorageSnapshot,
    GroupStateUnavailable,
    ConfigurationMismatch,
    GroupMismatch,
    PreviousEpochMismatch,
    PreviousRosterMismatch,
    ResultingEpochMismatch,
    ResultingRosterMismatch,
    AadMismatch,
    MessageKindMismatch,
    LocalLeafMismatch,
    InvalidSigner,
    UnsupportedCredential,
    MlsDecodeRejected,
    WelcomeRejected,
    MlsProtocolRejected,
    ExpectedKeyPackageMismatch,
    InternalFailure,
}

#[derive(Debug)]
pub(crate) struct StrictReceiveError {
    pub kind: StrictReceiveErrorKind,
    pub detail: String,
}

pub(crate) struct StrictJoinGroupWithStorageResult {
    pub joined: JoinGroupWithStorageResult,
    pub local_leaf: MlsRosterLeafV1,
    pub consumed_key_package_sha256: Vec<u8>,
}

fn strict_receive_error(
    kind: StrictReceiveErrorKind,
    detail: impl Into<String>,
) -> StrictReceiveError {
    StrictReceiveError {
        kind,
        detail: detail.into(),
    }
}

/// Compute the canonical version-1 roster digest from caller-supplied fields.
#[flutter_rust_bridge::frb(sync)]
pub fn mls_roster_digest_v1(
    group_id: Vec<u8>,
    epoch: u64,
    leaves: Vec<MlsRosterLeafV1>,
) -> Result<Vec<u8>, String> {
    validate_canonical_leaves(&leaves)?;
    roster_digest(&group_id, epoch, &leaves)
}

/// Compute the local-only digest of the group rows in one caller snapshot.
///
/// Installation-global rows are validated but excluded. Rust's decoded copy
/// of each input value is zeroized before this function returns.
#[flutter_rust_bridge::frb(sync)]
pub fn mls_group_state_digest(
    group_id: Vec<u8>,
    mut storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<Vec<u8>, String> {
    let result =
        group_state_digest_from_entries(&group_id, &storage_entries, storage_format_version);
    zeroize_entry_values(&mut storage_entries);
    result
}

/// Create an owner-only group with an explicit server-issued group ID.
#[allow(clippy::too_many_arguments)]
pub fn create_group_with_storage(
    config: MlsGroupConfig,
    signer_bytes: Vec<u8>,
    explicit_group_id: Vec<u8>,
    expected_owner_authority: MlsAuthorizedOwnerV1,
    credential_bytes: Option<Vec<u8>>,
    mut storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<CreateGroupWithStorageResult, String> {
    if explicit_group_id.is_empty() {
        zeroize_entry_values(&mut storage_entries);
        return Err("Explicit MLS group ID must not be empty".to_string());
    }
    let provider = provider_from_entries(storage_entries, storage_format_version, None)?;
    let signer = signer_from_bytes(signer_bytes)?;
    ensure_signer_public_key(
        &signer,
        &expected_owner_authority.expected_signature_public_key,
    )?;
    let credential_with_key = build_credential_with_key(
        &expected_owner_authority.expected_credential_identity,
        &expected_owner_authority.expected_signature_public_key,
        credential_bytes.as_deref(),
    )?;

    signer
        .store(provider.storage())
        .map_err(|e| format!("Failed to store signer: {e}"))?;
    let group = MlsGroup::new_with_group_id(
        &provider,
        &signer,
        &config.to_create_config(),
        GroupId::from_slice(&explicit_group_id),
        credential_with_key,
    )
    .map_err(|e| format!("Failed to create group: {e}"))?;
    let resulting_roster = roster_from_group(&group)?;
    if resulting_roster.group_id != explicit_group_id {
        return Err("Created MLS group ID does not match the explicit group ID".to_string());
    }
    if resulting_roster.leaves.len() != 1
        || !leaf_matches_owner(&resulting_roster.leaves[0], &expected_owner_authority)
    {
        return Err("Created owner leaf does not match expected owner authority".to_string());
    }
    let storage_batch = batch_from_provider(provider, Some(explicit_group_id.clone()), Vec::new())?;
    Ok(CreateGroupWithStorageResult {
        group_id: explicit_group_id,
        resulting_roster,
        storage_batch,
    })
}

/// Add exact authorized members and return deferred candidate state.
pub fn add_members_with_storage(
    group_id: Vec<u8>,
    signer_bytes: Vec<u8>,
    additions: Vec<MlsAuthorizedKeyPackageV1>,
    aad: Vec<u8>,
    expected_previous_state: MlsExpectedRosterStateV1,
    mut storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<PreparedCommitWithStorageResult, String> {
    if additions.is_empty() {
        zeroize_entry_values(&mut storage_entries);
        return Err("Add-members transition must contain an addition".to_string());
    }
    prepare_membership_commit_with_storage(
        group_id,
        signer_bytes,
        additions,
        Vec::new(),
        aad,
        expected_previous_state,
        storage_entries,
        storage_format_version,
    )
}

/// Remove exact authorized members and return deferred candidate state.
pub fn remove_members_with_storage(
    group_id: Vec<u8>,
    signer_bytes: Vec<u8>,
    removals: Vec<MlsAuthorizedRemovalV1>,
    aad: Vec<u8>,
    expected_previous_state: MlsExpectedRosterStateV1,
    mut storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<PreparedCommitWithStorageResult, String> {
    if removals.is_empty() {
        zeroize_entry_values(&mut storage_entries);
        return Err("Remove-members transition must contain a removal".to_string());
    }
    prepare_membership_commit_with_storage(
        group_id,
        signer_bytes,
        Vec::new(),
        removals,
        aad,
        expected_previous_state,
        storage_entries,
        storage_format_version,
    )
}

/// Atomically replace members using one combined remove/add Commit.
#[allow(clippy::too_many_arguments)]
pub fn swap_members_with_storage(
    group_id: Vec<u8>,
    signer_bytes: Vec<u8>,
    removals: Vec<MlsAuthorizedRemovalV1>,
    additions: Vec<MlsAuthorizedKeyPackageV1>,
    aad: Vec<u8>,
    expected_previous_state: MlsExpectedRosterStateV1,
    mut storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<PreparedCommitWithStorageResult, String> {
    if removals.is_empty() || additions.is_empty() {
        zeroize_entry_values(&mut storage_entries);
        return Err("Swap-members transition requires removals and additions".to_string());
    }
    prepare_membership_commit_with_storage(
        group_id,
        signer_bytes,
        additions,
        removals,
        aad,
        expected_previous_state,
        storage_entries,
        storage_format_version,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_membership_commit_with_storage(
    group_id: Vec<u8>,
    signer_bytes: Vec<u8>,
    additions: Vec<MlsAuthorizedKeyPackageV1>,
    removals: Vec<MlsAuthorizedRemovalV1>,
    aad: Vec<u8>,
    expected_previous_state: MlsExpectedRosterStateV1,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<PreparedCommitWithStorageResult, String> {
    let (provider, base_group_state_sha256) =
        provider_with_base_digest(&group_id, storage_entries, storage_format_version)?;
    let signer = signer_from_bytes(signer_bytes)?;
    let mut group = load_group(&group_id, &provider)?;
    let previous_roster = roster_from_group(&group)?;
    validate_expected_roster(&previous_roster, &expected_previous_state, "previous")?;
    ensure_local_signer(&group, &signer)?;

    let removal_indices = validate_removals(&previous_roster, &removals)?;
    let (key_packages, requested_additions) =
        validate_additions(&provider, additions, &previous_roster, &removal_indices)?;

    group.set_aad(aad);
    let bundle = group
        .commit_builder()
        .consume_proposal_store(false)
        .propose_adds(key_packages)
        .propose_removals(removal_indices.iter().copied().map(LeafNodeIndex::new))
        .load_psks(provider.storage())
        .map_err(|e| format!("Failed to load roster Commit PSKs: {e}"))?
        .build(provider.rand(), provider.crypto(), &signer, |_| true)
        .map_err(|e| format!("Failed to build roster Commit: {e}"))?
        .stage_commit(&provider)
        .map_err(|e| format!("Failed to stage roster Commit: {e}"))?;
    group
        .merge_pending_commit(&provider)
        .map_err(|e| format!("Failed to merge candidate roster Commit: {e}"))?;

    let resulting_roster = roster_from_group(&group)?;
    validate_exact_delta(
        &previous_roster,
        &resulting_roster,
        &removal_indices,
        &requested_additions,
    )?;
    prepared_result(
        bundle,
        previous_roster,
        resulting_roster,
        base_group_state_sha256,
        provider,
        group_id,
    )
}

/// Prepare a local self-update without changing installation identity or key.
pub fn self_update_with_storage(
    group_id: Vec<u8>,
    signer_bytes: Vec<u8>,
    aad: Vec<u8>,
    expected_previous_state: MlsExpectedRosterStateV1,
    expected_self_authority: MlsAuthorizedSelfV1,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<PreparedCommitWithStorageResult, String> {
    let (provider, base_group_state_sha256) =
        provider_with_base_digest(&group_id, storage_entries, storage_format_version)?;
    let signer = signer_from_bytes(signer_bytes)?;
    let mut group = load_group(&group_id, &provider)?;
    let previous_roster = roster_from_group(&group)?;
    validate_expected_roster(&previous_roster, &expected_previous_state, "previous")?;
    validate_self_authority(&group, &previous_roster, &expected_self_authority)?;
    ensure_signer_public_key(
        &signer,
        &expected_self_authority.expected_signature_public_key,
    )?;

    group.set_aad(aad);
    let bundle = group
        .self_update(&provider, &signer, LeafNodeParameters::default())
        .map_err(|e| format!("Failed to prepare self-update Commit: {e}"))?;
    group
        .merge_pending_commit(&provider)
        .map_err(|e| format!("Failed to merge candidate self-update Commit: {e}"))?;
    let resulting_roster = roster_from_group(&group)?;
    let expected_epoch = previous_roster
        .epoch
        .checked_add(1)
        .ok_or_else(|| "MLS epoch cannot advance beyond u64::MAX".to_string())?;
    if resulting_roster.group_id != previous_roster.group_id
        || resulting_roster.epoch != expected_epoch
        || resulting_roster.leaves != previous_roster.leaves
    {
        return Err("Self-update changed roster authority unexpectedly".to_string());
    }
    prepared_result(
        bundle,
        previous_roster,
        resulting_roster,
        base_group_state_sha256,
        provider,
        group_id,
    )
}

/// Join a Welcome only when its installed state matches canonical authority.
pub fn join_group_from_welcome_with_storage(
    config: MlsGroupConfig,
    welcome_bytes: Vec<u8>,
    ratchet_tree_bytes: Option<Vec<u8>>,
    signer_bytes: Vec<u8>,
    expected_resulting_state: MlsExpectedRosterStateV1,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<JoinGroupWithStorageResult, String> {
    join_group_from_welcome_with_storage_typed(
        config,
        welcome_bytes,
        ratchet_tree_bytes,
        signer_bytes,
        expected_resulting_state,
        None,
        storage_entries,
        storage_format_version,
    )
    .map(|result| result.joined)
    .map_err(|error| error.detail)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn join_group_from_welcome_with_storage_typed(
    config: MlsGroupConfig,
    welcome_bytes: Vec<u8>,
    ratchet_tree_bytes: Option<Vec<u8>>,
    signer_bytes: Vec<u8>,
    expected_resulting_state: MlsExpectedRosterStateV1,
    expected_target_key_package_sha256: Option<&[u8]>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<StrictJoinGroupWithStorageResult, StrictReceiveError> {
    if storage_format_version != super::storage::MLS_STORAGE_FORMAT_VERSION {
        return Err(strict_receive_error(
            StrictReceiveErrorKind::StorageFormatMismatch,
            "Unsupported MLS storage format version",
        ));
    }
    let provider =
        provider_from_entries(storage_entries, storage_format_version, None).map_err(|detail| {
            strict_receive_error(StrictReceiveErrorKind::InvalidStorageSnapshot, detail)
        })?;
    let signer = deserialize_signer_bytes(signer_bytes)
        .map_err(|detail| strict_receive_error(StrictReceiveErrorKind::InvalidSigner, detail))?;
    let signature_scheme = SignatureScheme::try_from(signer.scheme).map_err(|_| {
        strict_receive_error(
            StrictReceiveErrorKind::InvalidSigner,
            "Signer contains an unsupported signature scheme",
        )
    })?;
    if signature_scheme
        != config
            .to_create_config()
            .ciphersuite()
            .signature_algorithm()
    {
        return Err(strict_receive_error(
            StrictReceiveErrorKind::ConfigurationMismatch,
            "Signer signature scheme does not match the required configuration profile",
        ));
    }
    provider
        .storage()
        .write_serialized_basic_signer(&signer.private, &signer.public, signature_scheme)
        .map_err(|e| {
            strict_receive_error(
                StrictReceiveErrorKind::InternalFailure,
                format!("Failed to store signer: {e}"),
            )
        })?;
    let welcome_message = mls_message_from_exact_bytes(&welcome_bytes).map_err(|e| {
        strict_receive_error(
            StrictReceiveErrorKind::MlsDecodeRejected,
            format!("Failed to deserialize welcome: {e}"),
        )
    })?;
    let welcome = match welcome_message.extract() {
        MlsMessageBodyIn::Welcome(welcome) => welcome,
        _ => {
            return Err(strict_receive_error(
                StrictReceiveErrorKind::MessageKindMismatch,
                "Message is not a Welcome",
            ));
        }
    };
    let consumed_key_package_sha256 = selected_key_package_sha256(&provider, &welcome)?;
    if let Some(expected) = expected_target_key_package_sha256
        && (expected.len() != 32 || expected != consumed_key_package_sha256)
    {
        return Err(strict_receive_error(
            StrictReceiveErrorKind::ExpectedKeyPackageMismatch,
            "Welcome selected a different retained KeyPackage",
        ));
    }
    let ratchet_tree: Option<RatchetTreeIn> = ratchet_tree_bytes
        .map(|bytes| {
            RatchetTreeIn::tls_deserialize_exact_bytes(&bytes).map_err(|e| {
                strict_receive_error(
                    StrictReceiveErrorKind::MlsDecodeRejected,
                    format!("Failed to deserialize ratchet tree: {e}"),
                )
            })
        })
        .transpose()?;
    let staged =
        StagedWelcome::new_from_welcome(&provider, &config.to_join_config(), welcome, ratchet_tree)
            .map_err(|e| {
                strict_receive_error(
                    StrictReceiveErrorKind::WelcomeRejected,
                    format!("Failed to process welcome: {e}"),
                )
            })?;
    let group = staged.into_group(&provider).map_err(|e| {
        strict_receive_error(
            StrictReceiveErrorKind::WelcomeRejected,
            format!("Failed to join group from welcome: {e}"),
        )
    })?;
    if group.ciphersuite() != config.to_create_config().ciphersuite()
        || group.configuration() != &config.to_join_config()
    {
        return Err(strict_receive_error(
            StrictReceiveErrorKind::ConfigurationMismatch,
            "Joined MLS group does not match the required configuration profile",
        ));
    }
    ensure_local_signer_public_key(&group, &signer.public).map_err(|detail| {
        strict_receive_error(StrictReceiveErrorKind::LocalLeafMismatch, detail)
    })?;
    let resulting_roster = roster_from_group(&group).map_err(|detail| {
        strict_receive_error(StrictReceiveErrorKind::UnsupportedCredential, detail)
    })?;
    let own_leaf_index = group.own_leaf_index().u32();
    let local_leaf = resulting_roster
        .leaves
        .iter()
        .find(|leaf| leaf.leaf_index == own_leaf_index)
        .cloned()
        .ok_or_else(|| {
            strict_receive_error(
                StrictReceiveErrorKind::LocalLeafMismatch,
                "Joined group does not contain its own authenticated leaf",
            )
        })?;
    validate_expected_roster_typed(&resulting_roster, &expected_resulting_state, false)?;
    let group_id = resulting_roster.group_id.clone();
    let storage_batch = batch_from_provider(provider, Some(group_id.clone()), Vec::new())
        .map_err(|detail| strict_receive_error(StrictReceiveErrorKind::InternalFailure, detail))?;
    Ok(StrictJoinGroupWithStorageResult {
        joined: JoinGroupWithStorageResult {
            group_id,
            resulting_roster,
            storage_batch,
        },
        local_leaf,
        consumed_key_package_sha256,
    })
}

/// Process a message only when both base and resulting roster authority match.
pub fn process_message_with_storage(
    group_id: Vec<u8>,
    message_bytes: Vec<u8>,
    expected_aad: Vec<u8>,
    expected_previous_state: MlsExpectedRosterStateV1,
    expected_resulting_state: MlsExpectedRosterStateV1,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<ProcessMessageWithStorageResult, String> {
    process_message_with_storage_typed(
        group_id,
        message_bytes,
        expected_aad,
        expected_previous_state,
        expected_resulting_state,
        None,
        storage_entries,
        storage_format_version,
    )
    .map_err(|error| error.detail)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_message_with_storage_typed(
    group_id: Vec<u8>,
    message_bytes: Vec<u8>,
    expected_aad: Vec<u8>,
    expected_previous_state: MlsExpectedRosterStateV1,
    expected_resulting_state: MlsExpectedRosterStateV1,
    expected_config: Option<&MlsGroupConfig>,
    storage_entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<ProcessMessageWithStorageResult, StrictReceiveError> {
    if storage_format_version != super::storage::MLS_STORAGE_FORMAT_VERSION {
        return Err(strict_receive_error(
            StrictReceiveErrorKind::StorageFormatMismatch,
            "Unsupported MLS storage format version",
        ));
    }
    let provider = provider_from_entries(storage_entries, storage_format_version, Some(&group_id))
        .map_err(|detail| {
            strict_receive_error(StrictReceiveErrorKind::InvalidStorageSnapshot, detail)
        })?;
    let mut group = load_group(&group_id, &provider).map_err(|detail| {
        strict_receive_error(StrictReceiveErrorKind::GroupStateUnavailable, detail)
    })?;
    if let Some(config) = expected_config
        && (group.ciphersuite() != config.to_create_config().ciphersuite()
            || group.configuration() != &config.to_join_config())
    {
        return Err(strict_receive_error(
            StrictReceiveErrorKind::ConfigurationMismatch,
            "Loaded MLS group does not match the required configuration profile",
        ));
    }
    let previous_roster = roster_from_group(&group).map_err(|detail| {
        strict_receive_error(StrictReceiveErrorKind::UnsupportedCredential, detail)
    })?;
    validate_expected_roster_typed(&previous_roster, &expected_previous_state, true)?;
    let message = mls_message_from_exact_bytes(&message_bytes)
        .map_err(|e| {
            strict_receive_error(
                StrictReceiveErrorKind::MlsDecodeRejected,
                format!("Failed to deserialize message: {e}"),
            )
        })?
        .try_into_protocol_message()
        .map_err(|e| {
            strict_receive_error(
                StrictReceiveErrorKind::MessageKindMismatch,
                format!("Not a protocol message: {e}"),
            )
        })?;
    let processed = group.process_message(&provider, message).map_err(|e| {
        strict_receive_error(
            StrictReceiveErrorKind::MlsProtocolRejected,
            format!("Failed to process message: {e}"),
        )
    })?;
    if processed.aad() != expected_aad {
        zeroize_processed_content(processed);
        return Err(strict_receive_error(
            StrictReceiveErrorKind::AadMismatch,
            "Message AAD does not match the expected AAD",
        ));
    }
    let sender_index = match processed.sender() {
        Sender::Member(index) => Some(index.u32()),
        _ => None,
    };
    let (message_type, mut application_message, has_staged_commit, has_proposal, proposal_type) =
        match processed.into_content() {
            ProcessedMessageContent::ApplicationMessage(message) => (
                ProcessedMessageType::Application,
                Some(message.into_bytes()),
                false,
                false,
                None,
            ),
            ProcessedMessageContent::StagedCommitMessage(commit) => {
                group.merge_staged_commit(&provider, *commit).map_err(|e| {
                    strict_receive_error(
                        StrictReceiveErrorKind::MlsProtocolRejected,
                        format!("Failed to merge staged commit: {e}"),
                    )
                })?;
                (ProcessedMessageType::StagedCommit, None, true, false, None)
            }
            ProcessedMessageContent::ProposalMessage(proposal) => {
                let proposal_type = proposal_type(proposal.proposal());
                group
                    .store_pending_proposal(provider.storage(), *proposal)
                    .map_err(|e| {
                        strict_receive_error(
                            StrictReceiveErrorKind::MlsProtocolRejected,
                            format!("Failed to store pending proposal: {e}"),
                        )
                    })?;
                (
                    ProcessedMessageType::Proposal,
                    None,
                    false,
                    true,
                    Some(proposal_type),
                )
            }
            _ => {
                return Err(strict_receive_error(
                    StrictReceiveErrorKind::MessageKindMismatch,
                    "Unknown processed message content type",
                ));
            }
        };
    let resulting_roster = roster_from_group(&group).map_err(|detail| {
        strict_receive_error(StrictReceiveErrorKind::UnsupportedCredential, detail)
    })?;
    if let Err(error) =
        validate_expected_roster_typed(&resulting_roster, &expected_resulting_state, false)
    {
        if let Some(message) = &mut application_message {
            message.zeroize();
        }
        return Err(error);
    }
    let previous_epoch = previous_roster.epoch;
    let resulting_epoch = resulting_roster.epoch;
    let storage_batch = batch_from_provider(provider, Some(group_id), Vec::new())
        .map_err(|detail| strict_receive_error(StrictReceiveErrorKind::InternalFailure, detail))?;
    Ok(ProcessMessageWithStorageResult {
        message_type,
        sender_index,
        previous_epoch,
        resulting_epoch,
        application_message,
        has_staged_commit,
        has_proposal,
        proposal_type,
        previous_roster,
        resulting_roster,
        storage_batch,
    })
}

fn selected_key_package_sha256(
    provider: &crate::snapshot_storage::SnapshotOpenMlsProvider,
    welcome: &Welcome,
) -> Result<Vec<u8>, StrictReceiveError> {
    for encrypted_group_secrets in welcome.secrets() {
        let key_package_ref = encrypted_group_secrets.new_member();
        let key_package_bundle: Option<KeyPackageBundle> = provider
            .storage()
            .key_package(&key_package_ref)
            .map_err(|error| {
                strict_receive_error(
                    StrictReceiveErrorKind::InvalidStorageSnapshot,
                    format!("Failed to read retained KeyPackage: {error}"),
                )
            })?;
        if let Some(bundle) = key_package_bundle {
            let encoded = bundle
                .key_package()
                .tls_serialize_detached()
                .map_err(|error| {
                    strict_receive_error(
                        StrictReceiveErrorKind::InternalFailure,
                        format!("Failed to serialize selected KeyPackage: {error}"),
                    )
                })?;
            return Ok(Sha256::digest(encoded).to_vec());
        }
    }
    Err(strict_receive_error(
        StrictReceiveErrorKind::WelcomeRejected,
        "Welcome has no KeyPackage present in the supplied snapshot",
    ))
}

fn validate_expected_roster_typed(
    actual: &MlsRosterSummaryV1,
    expected: &MlsExpectedRosterStateV1,
    previous: bool,
) -> Result<(), StrictReceiveError> {
    let label = if previous { "previous" } else { "resulting" };
    if expected.digest_sha256.len() != 32 {
        return Err(strict_receive_error(
            if previous {
                StrictReceiveErrorKind::PreviousRosterMismatch
            } else {
                StrictReceiveErrorKind::ResultingRosterMismatch
            },
            format!("Expected {label} roster digest must be 32 bytes"),
        ));
    }
    if actual.group_id != expected.group_id {
        return Err(strict_receive_error(
            StrictReceiveErrorKind::GroupMismatch,
            format!("{label} MLS group ID does not match expected authority"),
        ));
    }
    if actual.epoch != expected.epoch {
        return Err(strict_receive_error(
            if previous {
                StrictReceiveErrorKind::PreviousEpochMismatch
            } else {
                StrictReceiveErrorKind::ResultingEpochMismatch
            },
            format!("{label} MLS epoch does not match expected authority"),
        ));
    }
    if actual.digest_sha256 != expected.digest_sha256 {
        return Err(strict_receive_error(
            if previous {
                StrictReceiveErrorKind::PreviousRosterMismatch
            } else {
                StrictReceiveErrorKind::ResultingRosterMismatch
            },
            format!("{label} MLS roster digest does not match expected authority"),
        ));
    }
    Ok(())
}

fn zeroize_processed_content(processed: ProcessedMessage) {
    if let ProcessedMessageContent::ApplicationMessage(message) = processed.into_content() {
        let mut plaintext = message.into_bytes();
        plaintext.zeroize();
    }
}
fn roster_from_group(group: &MlsGroup) -> Result<MlsRosterSummaryV1, String> {
    let mut leaves = Vec::new();
    for member in group.members() {
        let credential = BasicCredential::try_from(member.credential)
            .map_err(|_| "MLS roster member does not contain a Basic Credential".to_string())?;
        leaves.push(MlsRosterLeafV1 {
            leaf_index: member.index.u32(),
            credential_identity: credential.identity().to_vec(),
            signature_public_key: member.signature_key,
        });
    }
    leaves.sort_by_key(|leaf| leaf.leaf_index);
    validate_canonical_leaves(&leaves)?;
    let group_id = group.group_id().as_slice().to_vec();
    let epoch = group.epoch().as_u64();
    let digest_sha256 = roster_digest(&group_id, epoch, &leaves)?;
    Ok(MlsRosterSummaryV1 {
        group_id,
        epoch,
        leaves,
        digest_sha256,
    })
}

fn validate_canonical_leaves(leaves: &[MlsRosterLeafV1]) -> Result<(), String> {
    let mut previous_index = None;
    let mut identities = HashSet::with_capacity(leaves.len());
    let mut signature_keys = HashSet::with_capacity(leaves.len());
    for leaf in leaves {
        if previous_index.is_some_and(|index| leaf.leaf_index <= index) {
            return Err("Roster leaves must have unique ascending leaf indexes".to_string());
        }
        previous_index = Some(leaf.leaf_index);
        if !identities.insert(leaf.credential_identity.clone()) {
            return Err("MLS roster contains a duplicate Basic Credential identity".to_string());
        }
        if !signature_keys.insert(leaf.signature_public_key.clone()) {
            return Err("MLS roster contains a duplicate signature-key binding".to_string());
        }
    }
    Ok(())
}

fn roster_digest(
    group_id: &[u8],
    epoch: u64,
    leaves: &[MlsRosterLeafV1],
) -> Result<Vec<u8>, String> {
    let mut hash = Sha256::new();
    hash.update(ROSTER_DOMAIN_V1);
    hash_len_prefixed(&mut hash, group_id)?;
    hash.update(epoch.to_be_bytes());
    hash.update(checked_u32(leaves.len(), "roster leaf count")?.to_be_bytes());
    for leaf in leaves {
        hash.update(leaf.leaf_index.to_be_bytes());
        hash_len_prefixed(&mut hash, &leaf.credential_identity)?;
        hash_len_prefixed(&mut hash, &leaf.signature_public_key)?;
    }
    Ok(hash.finalize().to_vec())
}

pub(crate) fn group_state_digest_from_entries(
    group_id: &[u8],
    entries: &[MlsStorageEntry],
    storage_format_version: u32,
) -> Result<Vec<u8>, String> {
    validate_storage_entries(entries, storage_format_version, Some(group_id))?;
    let mut group_rows: Vec<_> = entries
        .iter()
        .filter(|entry| !is_global_key(&entry.key))
        .collect();
    group_rows.sort_by(|left, right| left.key.cmp(&right.key));
    let mut hash = Sha256::new();
    hash.update(GROUP_STATE_DOMAIN_V1);
    hash.update(storage_format_version.to_be_bytes());
    hash_len_prefixed(&mut hash, group_id)?;
    hash.update(checked_u32(group_rows.len(), "group-state row count")?.to_be_bytes());
    for entry in group_rows {
        hash_len_prefixed(&mut hash, &entry.key)?;
        hash_len_prefixed(&mut hash, &entry.value)?;
    }
    Ok(hash.finalize().to_vec())
}

fn provider_with_base_digest(
    group_id: &[u8],
    mut entries: Vec<MlsStorageEntry>,
    storage_format_version: u32,
) -> Result<(crate::snapshot_storage::SnapshotOpenMlsProvider, Vec<u8>), String> {
    let digest = match group_state_digest_from_entries(group_id, &entries, storage_format_version) {
        Ok(digest) => digest,
        Err(error) => {
            zeroize_entry_values(&mut entries);
            return Err(error);
        }
    };
    let provider = provider_from_entries(entries, storage_format_version, Some(group_id))?;
    Ok((provider, digest))
}

fn checked_u32(value: usize, label: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{label} exceeds the version-1 encoding limit"))
}

fn hash_len_prefixed(hash: &mut Sha256, value: &[u8]) -> Result<(), String> {
    hash.update(checked_u32(value.len(), "byte string")?.to_be_bytes());
    hash.update(value);
    Ok(())
}

fn validate_expected_roster(
    actual: &MlsRosterSummaryV1,
    expected: &MlsExpectedRosterStateV1,
    label: &str,
) -> Result<(), String> {
    if expected.digest_sha256.len() != 32 {
        return Err(format!("Expected {label} roster digest must be 32 bytes"));
    }
    if actual.group_id != expected.group_id {
        return Err(format!(
            "{label} MLS group ID does not match expected authority"
        ));
    }
    if actual.epoch != expected.epoch {
        return Err(format!(
            "{label} MLS epoch does not match expected authority"
        ));
    }
    if actual.digest_sha256 != expected.digest_sha256 {
        return Err(format!(
            "{label} MLS roster digest does not match expected authority"
        ));
    }
    Ok(())
}

fn ensure_signer_public_key(
    signer: &openmls_basic_credential::SignatureKeyPair,
    expected: &[u8],
) -> Result<(), String> {
    if signer.public() != expected {
        return Err("Signer public key does not match expected installation authority".to_string());
    }
    Ok(())
}

fn ensure_local_signer(
    group: &MlsGroup,
    signer: &openmls_basic_credential::SignatureKeyPair,
) -> Result<(), String> {
    let own = group
        .member_at(group.own_leaf_index())
        .ok_or_else(|| "Local MLS leaf is missing".to_string())?;
    ensure_signer_public_key(signer, &own.signature_key)
}

fn ensure_local_signer_public_key(
    group: &MlsGroup,
    signer_public_key: &[u8],
) -> Result<(), String> {
    let own = group
        .member_at(group.own_leaf_index())
        .ok_or_else(|| "Local MLS leaf is missing".to_string())?;
    if signer_public_key != own.signature_key {
        return Err("Signer public key does not match expected installation authority".to_string());
    }
    Ok(())
}

fn leaf_matches_owner(leaf: &MlsRosterLeafV1, owner: &MlsAuthorizedOwnerV1) -> bool {
    leaf.credential_identity == owner.expected_credential_identity
        && leaf.signature_public_key == owner.expected_signature_public_key
}

fn validate_removals(
    previous: &MlsRosterSummaryV1,
    removals: &[MlsAuthorizedRemovalV1],
) -> Result<BTreeSet<u32>, String> {
    let previous_by_index: HashMap<_, _> = previous
        .leaves
        .iter()
        .map(|leaf| (leaf.leaf_index, leaf))
        .collect();
    let mut indices = BTreeSet::new();
    for removal in removals {
        if !indices.insert(removal.leaf_index) {
            return Err("Removal list contains a duplicate leaf index".to_string());
        }
        let leaf = previous_by_index
            .get(&removal.leaf_index)
            .ok_or_else(|| "Authorized removal leaf is not active".to_string())?;
        if leaf.credential_identity != removal.expected_credential_identity
            || leaf.signature_public_key != removal.expected_signature_public_key
        {
            return Err("Authorized removal does not match current leaf authority".to_string());
        }
    }
    Ok(indices)
}

fn validate_additions(
    provider: &crate::snapshot_storage::SnapshotOpenMlsProvider,
    additions: Vec<MlsAuthorizedKeyPackageV1>,
    previous: &MlsRosterSummaryV1,
    removals: &BTreeSet<u32>,
) -> Result<(Vec<KeyPackage>, Vec<RequestedAddition>), String> {
    let mut identities = HashSet::new();
    let mut signature_keys = HashSet::new();
    for leaf in previous
        .leaves
        .iter()
        .filter(|leaf| !removals.contains(&leaf.leaf_index))
    {
        identities.insert(leaf.credential_identity.clone());
        signature_keys.insert(leaf.signature_public_key.clone());
    }
    let mut key_packages = Vec::with_capacity(additions.len());
    let mut requested = Vec::with_capacity(additions.len());
    for addition in additions {
        if !identities.insert(addition.expected_credential_identity.clone()) {
            return Err("Addition would create a duplicate Basic Credential identity".to_string());
        }
        if !signature_keys.insert(addition.expected_signature_public_key.clone()) {
            return Err("Addition would create a duplicate signature-key binding".to_string());
        }
        let key_package = KeyPackageIn::tls_deserialize_exact_bytes(&addition.key_package_bytes)
            .map_err(|e| format!("Failed to deserialize key package: {e}"))?
            .validate(provider.crypto(), ProtocolVersion::Mls10)
            .map_err(|e| format!("Failed to validate key package: {e}"))?;
        let credential = BasicCredential::try_from(key_package.leaf_node().credential().clone())
            .map_err(|_| "Key package does not contain a Basic Credential".to_string())?;
        if credential.identity() != addition.expected_credential_identity {
            return Err(
                "Key package credential identity does not match expected authority".to_string(),
            );
        }
        if key_package.leaf_node().signature_key().as_slice()
            != addition.expected_signature_public_key
        {
            return Err("Key package signature key does not match expected authority".to_string());
        }
        requested.push((
            addition.expected_credential_identity,
            addition.expected_signature_public_key,
        ));
        key_packages.push(key_package);
    }
    Ok((key_packages, requested))
}

fn validate_exact_delta(
    previous: &MlsRosterSummaryV1,
    resulting: &MlsRosterSummaryV1,
    removals: &BTreeSet<u32>,
    additions: &[RequestedAddition],
) -> Result<(), String> {
    let expected_epoch = previous
        .epoch
        .checked_add(1)
        .ok_or_else(|| "MLS epoch cannot advance beyond u64::MAX".to_string())?;
    if resulting.group_id != previous.group_id || resulting.epoch != expected_epoch {
        return Err("Roster Commit produced an unexpected group ID or epoch".to_string());
    }
    let retained: BTreeMap<_, _> = previous
        .leaves
        .iter()
        .filter(|leaf| !removals.contains(&leaf.leaf_index))
        .map(|leaf| (leaf.leaf_index, leaf))
        .collect();
    let mut matched_additions = vec![false; additions.len()];
    for leaf in &resulting.leaves {
        if let Some(expected) = retained.get(&leaf.leaf_index) {
            if *expected != leaf {
                return Err("Roster Commit changed a retained leaf".to_string());
            }
            continue;
        }
        let matches: Vec<_> = additions
            .iter()
            .enumerate()
            .filter(|(_, (identity, signature_key))| {
                leaf.credential_identity == *identity && leaf.signature_public_key == *signature_key
            })
            .map(|(index, _)| index)
            .collect();
        if matches.len() != 1 || matched_additions[matches[0]] {
            return Err("Roster Commit produced an unauthorized or duplicate leaf".to_string());
        }
        matched_additions[matches[0]] = true;
    }
    let expected_leaf_count = retained
        .len()
        .checked_add(additions.len())
        .ok_or_else(|| "Resulting roster leaf count overflowed".to_string())?;
    if resulting.leaves.len() != expected_leaf_count
        || matched_additions.iter().any(|matched| !matched)
    {
        return Err("Roster Commit result is not the exact authorized delta".to_string());
    }
    Ok(())
}

fn validate_self_authority(
    group: &MlsGroup,
    previous: &MlsRosterSummaryV1,
    expected: &MlsAuthorizedSelfV1,
) -> Result<(), String> {
    if group.own_leaf_index().u32() != expected.leaf_index {
        return Err("Expected self leaf index does not match local MLS leaf".to_string());
    }
    let leaf = previous
        .leaves
        .iter()
        .find(|leaf| leaf.leaf_index == expected.leaf_index)
        .ok_or_else(|| "Expected self leaf is not active".to_string())?;
    if leaf.credential_identity != expected.expected_credential_identity
        || leaf.signature_public_key != expected.expected_signature_public_key
    {
        return Err("Expected self authority does not match local MLS leaf".to_string());
    }
    Ok(())
}

fn prepared_result(
    bundle: CommitMessageBundle,
    previous_roster: MlsRosterSummaryV1,
    resulting_roster: MlsRosterSummaryV1,
    base_group_state_sha256: Vec<u8>,
    provider: crate::snapshot_storage::SnapshotOpenMlsProvider,
    group_id: Vec<u8>,
) -> Result<PreparedCommitWithStorageResult, String> {
    let (commit, welcome, group_info) = bundle.into_messages();
    let commit = commit
        .tls_serialize_detached()
        .map_err(|e| format!("Failed to serialize Commit: {e}"))?;
    let welcome = welcome
        .map(|message| message.tls_serialize_detached())
        .transpose()
        .map_err(|e| format!("Failed to serialize Welcome: {e}"))?;
    let group_info = group_info
        .map(|message| message.tls_serialize_detached())
        .transpose()
        .map_err(|e| format!("Failed to serialize GroupInfo: {e}"))?;
    let commit_sha256 = Sha256::digest(&commit).to_vec();
    let storage_batch = batch_from_provider(provider, Some(group_id), Vec::new())?;
    Ok(PreparedCommitWithStorageResult {
        commit,
        welcome,
        group_info,
        commit_sha256,
        previous_roster,
        resulting_roster,
        base_group_state_sha256,
        storage_batch,
    })
}

fn proposal_type(proposal: &Proposal) -> MlsProposalType {
    match proposal {
        Proposal::Add(_) => MlsProposalType::Add,
        Proposal::Remove(_) => MlsProposalType::Remove,
        Proposal::Update(_) => MlsProposalType::Update,
        Proposal::PreSharedKey(_) => MlsProposalType::PreSharedKey,
        Proposal::ReInit(_) => MlsProposalType::Reinit,
        Proposal::ExternalInit(_) => MlsProposalType::ExternalInit,
        Proposal::GroupContextExtensions(_) => MlsProposalType::GroupContextExtensions,
        _ => MlsProposalType::Custom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::storage::MLS_STORAGE_FORMAT_VERSION;

    #[test]
    fn roster_digest_has_stable_vector() {
        let leaves = vec![
            MlsRosterLeafV1 {
                leaf_index: 0,
                credential_identity: b"alice".to_vec(),
                signature_public_key: vec![1, 2, 3],
            },
            MlsRosterLeafV1 {
                leaf_index: 2,
                credential_identity: b"bob".to_vec(),
                signature_public_key: vec![4, 5],
            },
        ];
        let digest = mls_roster_digest_v1(b"group-1".to_vec(), 7, leaves).unwrap();
        assert_eq!(digest.len(), 32);
        assert_eq!(
            digest,
            vec![
                9, 246, 46, 82, 150, 7, 150, 96, 19, 71, 245, 33, 23, 163, 89, 183, 82, 135, 15,
                56, 194, 29, 145, 186, 84, 150, 171, 94, 156, 188, 210, 59,
            ]
        );
    }

    #[test]
    fn roster_digest_rejects_noncanonical_or_duplicate_leaves() {
        let duplicate_identity = vec![
            MlsRosterLeafV1 {
                leaf_index: 1,
                credential_identity: b"same".to_vec(),
                signature_public_key: vec![1],
            },
            MlsRosterLeafV1 {
                leaf_index: 2,
                credential_identity: b"same".to_vec(),
                signature_public_key: vec![2],
            },
        ];
        assert!(mls_roster_digest_v1(vec![1], 0, duplicate_identity).is_err());

        let duplicate_signature_key = vec![
            MlsRosterLeafV1 {
                leaf_index: 1,
                credential_identity: b"one".to_vec(),
                signature_public_key: vec![9],
            },
            MlsRosterLeafV1 {
                leaf_index: 3,
                credential_identity: b"two".to_vec(),
                signature_public_key: vec![9],
            },
        ];
        assert!(mls_roster_digest_v1(vec![1], 0, duplicate_signature_key).is_err());

        let descending = vec![
            MlsRosterLeafV1 {
                leaf_index: 4,
                credential_identity: b"one".to_vec(),
                signature_public_key: vec![1],
            },
            MlsRosterLeafV1 {
                leaf_index: 2,
                credential_identity: b"two".to_vec(),
                signature_public_key: vec![2],
            },
        ];
        assert!(mls_roster_digest_v1(vec![1], 0, descending).is_err());
    }

    #[test]
    fn exact_delta_accepts_leaf_reuse_and_rejects_extras() {
        let leaf = |leaf_index, identity: &[u8], signature_public_key: &[u8]| MlsRosterLeafV1 {
            leaf_index,
            credential_identity: identity.to_vec(),
            signature_public_key: signature_public_key.to_vec(),
        };
        let summary = |epoch, leaves: Vec<MlsRosterLeafV1>| MlsRosterSummaryV1 {
            group_id: b"group".to_vec(),
            epoch,
            digest_sha256: roster_digest(b"group", epoch, &leaves).unwrap(),
            leaves,
        };
        let previous = summary(3, vec![leaf(0, b"alice", &[1]), leaf(2, b"bob", &[2])]);
        let resulting = summary(4, vec![leaf(0, b"alice", &[1]), leaf(2, b"charlie", &[3])]);
        let removals = BTreeSet::from([2]);
        let additions = vec![(b"charlie".to_vec(), vec![3])];
        assert!(validate_exact_delta(&previous, &resulting, &removals, &additions).is_ok());

        let with_extra = summary(
            4,
            vec![
                leaf(0, b"alice", &[1]),
                leaf(2, b"charlie", &[3]),
                leaf(4, b"mallory", &[4]),
            ],
        );
        assert!(validate_exact_delta(&previous, &with_extra, &removals, &additions).is_err());
    }

    #[test]
    fn group_state_digest_is_sorted_and_excludes_global_rows() {
        let row = |key: &[u8], value: &[u8], group: Option<&[u8]>| MlsStorageEntry {
            key: key.to_vec(),
            value: value.to_vec(),
            group_id: group.map(<[u8]>::to_vec),
        };
        let group_id = b"g";
        let first = vec![
            row(b"Tree-z", b"z", Some(group_id)),
            row(b"KeyPackage-a", b"ignored", None),
            row(b"GroupState-a", b"a", Some(group_id)),
        ];
        let second = vec![
            row(b"GroupState-a", b"a", Some(group_id)),
            row(b"Tree-z", b"z", Some(group_id)),
            row(b"KeyPackage-other", b"also ignored", None),
        ];
        assert_eq!(
            group_state_digest_from_entries(group_id, &first, MLS_STORAGE_FORMAT_VERSION).unwrap(),
            group_state_digest_from_entries(group_id, &second, MLS_STORAGE_FORMAT_VERSION).unwrap()
        );
    }
}
