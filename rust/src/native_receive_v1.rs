//! Versioned native receive contract shared by Android and Apple wrappers.
//!
//! The wire format is deliberately independent from Flutter Rust Bridge. A
//! small fixed header wraps a strict deterministic-CBOR subset. Every request
//! passes a non-allocating structural/canonical preflight before owned values
//! are decoded.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use minicbor::data::Type;
use minicbor::{Decoder, Encoder};
use zeroize::Zeroize;

use crate::api::config::MlsGroupConfig;
use crate::api::group_e2ee::{
    MlsExpectedRosterStateV1, MlsRosterLeafV1, MlsRosterSummaryV1, ProcessMessageWithStorageResult,
    StrictJoinGroupWithStorageResult, StrictReceiveErrorKind, group_state_digest_from_entries,
    join_group_from_welcome_with_storage_typed, process_message_with_storage_typed,
};
use crate::api::storage::{MlsStorageBatch, MlsStorageEntry, zeroize_entry_values};
use crate::api::types::{MlsCiphersuite, MlsWireFormatPolicy, ProcessedMessageType};

pub const NATIVE_RECEIVE_CONTRACT_VERSION: u16 = 1;
pub const NATIVE_RECEIVE_PROFILE_V1: u16 = 1;
pub const NATIVE_RECEIVE_REQUEST_MAX_BYTES: usize = 12 * 1024 * 1024;
pub const NATIVE_RECEIVE_RESULT_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const NATIVE_RECEIVE_STORAGE_MAX_BYTES: usize = 6 * 1024 * 1024;
pub const NATIVE_RECEIVE_STORAGE_MAX_ENTRIES: usize = 4096;
pub const NATIVE_RECEIVE_STORAGE_KEY_MAX_BYTES: usize = 4096;
pub const NATIVE_RECEIVE_STORAGE_VALUE_MAX_BYTES: usize = 2 * 1024 * 1024;
pub const NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES: usize = 1024 * 1024;
pub const NATIVE_RECEIVE_RATCHET_TREE_MAX_BYTES: usize = 2 * 1024 * 1024;
pub const NATIVE_RECEIVE_AAD_MAX_BYTES: usize = 16 * 1024;
pub const NATIVE_RECEIVE_SIGNER_MAX_BYTES: usize = 4096;
pub const NATIVE_RECEIVE_PLAINTEXT_MAX_BYTES: usize = 256 * 1024;
pub const NATIVE_RECEIVE_ROSTER_MAX_LEAVES: usize = 256;

const FRAME_HEADER_BYTES: usize = 12;
const FRAME_MAGIC: &[u8; 4] = b"KMLS";
const PROFILE_GROUP_ID_BYTES: usize = 16;
const PROFILE_CREDENTIAL_IDENTITY_BYTES: usize = 45;
const PROFILE_SIGNATURE_PUBLIC_KEY_BYTES: usize = 32;
const SHA256_BYTES: usize = 32;
const PREFLIGHT_MAX_DEPTH: usize = 6;
const PREFLIGHT_MAX_ITEMS: usize = 32_768;
const PREFLIGHT_MAX_MAP_ENTRIES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NativeReceiveOperationV1 {
    Application = 1,
    Commit = 2,
    Welcome = 3,
}

impl NativeReceiveOperationV1 {
    fn from_u8(value: u8) -> Result<Self, NativeReceiveErrorCodeV1> {
        match value {
            1 => Ok(Self::Application),
            2 => Ok(Self::Commit),
            3 => Ok(Self::Welcome),
            _ => Err(NativeReceiveErrorCodeV1::UnsupportedOperation),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum NativeReceiveErrorCodeV1 {
    InvalidFrame = 1,
    UnsupportedContractVersion = 2,
    UnsupportedProfile = 3,
    UnsupportedOperation = 4,
    NoncanonicalEncoding = 5,
    LimitExceeded = 6,
    StorageFormatMismatch = 10,
    InvalidStorageSnapshot = 11,
    GroupStateUnavailable = 12,
    BaseStateMismatch = 13,
    ConfigurationMismatch = 20,
    GroupMismatch = 21,
    PreviousEpochMismatch = 22,
    PreviousRosterMismatch = 23,
    ResultingEpochMismatch = 24,
    ResultingRosterMismatch = 25,
    AadMismatch = 26,
    MessageKindMismatch = 27,
    SenderMismatch = 28,
    LocalLeafMismatch = 29,
    InvalidSigner = 30,
    UnsupportedCredential = 31,
    MlsDecodeRejected = 32,
    WelcomeRejected = 33,
    MlsProtocolRejected = 34,
    ExpectedKeyPackageMismatch = 35,
    InternalFailure = 255,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeLeafAuthorityV1 {
    pub leaf_index: u32,
    pub credential_identity: Vec<u8>,
    pub signature_public_key: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeExpectedRosterStateV1 {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub digest_sha256: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRosterSummaryV1 {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub leaves: Vec<NativeLeafAuthorityV1>,
    pub digest_sha256: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeStorageEntryV1 {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub group_id: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeStorageSnapshotV1 {
    pub storage_format_version: u32,
    pub entries: Vec<NativeStorageEntryV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeStorageBatchV1 {
    pub storage_format_version: u32,
    pub upserts: Vec<NativeStorageEntryV1>,
    pub deletes: Vec<Vec<u8>>,
    pub deleted_group_ids: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub enum NativeReceiveRequestV1 {
    Process {
        operation: NativeReceiveOperationV1,
        profile_id: u16,
        group_id: Vec<u8>,
        message_bytes: Vec<u8>,
        expected_aad: Vec<u8>,
        expected_sender: NativeLeafAuthorityV1,
        expected_previous_state: NativeExpectedRosterStateV1,
        expected_resulting_state: NativeExpectedRosterStateV1,
        expected_base_group_state_sha256: Vec<u8>,
        storage: NativeStorageSnapshotV1,
    },
    Welcome {
        profile_id: u16,
        welcome_bytes: Vec<u8>,
        ratchet_tree_bytes: Option<Vec<u8>>,
        signer_bytes: Vec<u8>,
        expected_local_leaf: NativeLeafAuthorityV1,
        expected_resulting_state: NativeExpectedRosterStateV1,
        expected_target_key_package_sha256: Vec<u8>,
        storage: NativeStorageSnapshotV1,
    },
}

impl Drop for NativeReceiveRequestV1 {
    fn drop(&mut self) {
        match self {
            Self::Process {
                message_bytes,
                expected_aad,
                storage,
                ..
            } => {
                message_bytes.zeroize();
                expected_aad.zeroize();
                zeroize_entries(&mut storage.entries);
            }
            Self::Welcome {
                welcome_bytes,
                ratchet_tree_bytes,
                signer_bytes,
                storage,
                ..
            } => {
                welcome_bytes.zeroize();
                if let Some(tree) = ratchet_tree_bytes {
                    tree.zeroize();
                }
                signer_bytes.zeroize();
                zeroize_entries(&mut storage.entries);
            }
        }
    }
}

#[derive(Debug)]
pub enum NativeReceiveSuccessV1 {
    Application {
        sender: NativeLeafAuthorityV1,
        previous_roster: NativeRosterSummaryV1,
        resulting_roster: NativeRosterSummaryV1,
        resulting_group_state_sha256: Vec<u8>,
        plaintext: Vec<u8>,
        storage_batch: NativeStorageBatchV1,
    },
    Commit {
        sender: NativeLeafAuthorityV1,
        previous_roster: NativeRosterSummaryV1,
        resulting_roster: NativeRosterSummaryV1,
        resulting_group_state_sha256: Vec<u8>,
        storage_batch: NativeStorageBatchV1,
    },
    Welcome {
        local_leaf: NativeLeafAuthorityV1,
        resulting_roster: NativeRosterSummaryV1,
        resulting_group_state_sha256: Vec<u8>,
        consumed_key_package_sha256: Vec<u8>,
        storage_batch: NativeStorageBatchV1,
    },
}

impl NativeReceiveSuccessV1 {
    pub fn operation(&self) -> NativeReceiveOperationV1 {
        match self {
            Self::Application { .. } => NativeReceiveOperationV1::Application,
            Self::Commit { .. } => NativeReceiveOperationV1::Commit,
            Self::Welcome { .. } => NativeReceiveOperationV1::Welcome,
        }
    }
}

impl Drop for NativeReceiveSuccessV1 {
    fn drop(&mut self) {
        match self {
            Self::Application {
                plaintext,
                storage_batch,
                ..
            } => {
                plaintext.zeroize();
                zeroize_entries(&mut storage_batch.upserts);
            }
            Self::Commit { storage_batch, .. } | Self::Welcome { storage_batch, .. } => {
                zeroize_entries(&mut storage_batch.upserts);
            }
        }
    }
}

#[derive(Debug)]
pub struct NativeReceiveOutcomeV1 {
    pub state_applied: bool,
    pub result: Option<NativeReceiveSuccessV1>,
    pub error: Option<NativeReceiveErrorCodeV1>,
}

impl NativeReceiveOutcomeV1 {
    pub fn success(result: NativeReceiveSuccessV1) -> Self {
        Self {
            state_applied: false,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(error: NativeReceiveErrorCodeV1) -> Self {
        Self {
            state_applied: false,
            result: None,
            error: Some(error),
        }
    }
}

pub fn decode_native_receive_request_v1(
    frame: &[u8],
) -> Result<NativeReceiveRequestV1, NativeReceiveErrorCodeV1> {
    let header = decode_header(frame, NATIVE_RECEIVE_REQUEST_MAX_BYTES)?;
    let operation = NativeReceiveOperationV1::from_u8(header.operation)?;
    preflight_cbor(header.payload)?;
    let mut decoder = Decoder::new(header.payload);
    let request = match operation {
        NativeReceiveOperationV1::Application | NativeReceiveOperationV1::Commit => {
            decode_process_request(&mut decoder, operation)?
        }
        NativeReceiveOperationV1::Welcome => decode_welcome_request(&mut decoder)?,
    };
    if decoder.position() != header.payload.len() {
        return Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding);
    }
    Ok(request)
}

/// Encode one canonical request frame. This helper is used to generate shared
/// platform fixtures; native consumers may implement the same frozen codec.
pub fn encode_native_receive_request_v1(
    request: &NativeReceiveRequestV1,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    encode_native_receive_request_v1_with_aad_minimum(request, 1)
}

#[cfg(any(test, feature = "native-receive-fixtures"))]
pub(crate) fn encode_native_receive_request_v1_allow_empty_aad_fixture(
    request: &NativeReceiveRequestV1,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    encode_native_receive_request_v1_with_aad_minimum(request, 0)
}

fn encode_native_receive_request_v1_with_aad_minimum(
    request: &NativeReceiveRequestV1,
    minimum_aad_bytes: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    let mut payload = Vec::new();
    let operation = match request {
        NativeReceiveRequestV1::Process {
            operation,
            profile_id,
            group_id,
            message_bytes,
            expected_aad,
            expected_sender,
            expected_previous_state,
            expected_resulting_state,
            expected_base_group_state_sha256,
            storage,
        } => {
            if *operation == NativeReceiveOperationV1::Welcome {
                return Err(NativeReceiveErrorCodeV1::UnsupportedOperation);
            }
            validate_profile(*profile_id)?;
            validate_exact_result_bytes(group_id, PROFILE_GROUP_ID_BYTES)?;
            validate_result_range(message_bytes, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
            validate_result_range(
                expected_aad,
                minimum_aad_bytes,
                NATIVE_RECEIVE_AAD_MAX_BYTES,
            )?;
            validate_exact_result_bytes(expected_base_group_state_sha256, SHA256_BYTES)?;
            let mut encoder = Encoder::new(&mut payload);
            encoder
                .map(9)
                .and_then(|value| value.u8(0))
                .and_then(|value| value.u16(*profile_id))
                .and_then(|value| value.u8(1))
                .and_then(|value| value.bytes(group_id))
                .and_then(|value| value.u8(2))
                .and_then(|value| value.bytes(message_bytes))
                .and_then(|value| value.u8(3))
                .and_then(|value| value.bytes(expected_aad))
                .and_then(|value| value.u8(4))
                .map_err(encode_error)?;
            encode_leaf(&mut encoder, expected_sender)?;
            encoder.u8(5).map_err(encode_error)?;
            encode_expected_roster(&mut encoder, expected_previous_state)?;
            encoder.u8(6).map_err(encode_error)?;
            encode_expected_roster(&mut encoder, expected_resulting_state)?;
            encoder
                .u8(7)
                .and_then(|value| value.bytes(expected_base_group_state_sha256))
                .and_then(|value| value.u8(8))
                .map_err(encode_error)?;
            encode_storage_snapshot(&mut encoder, storage, Some(group_id))?;
            *operation
        }
        NativeReceiveRequestV1::Welcome {
            profile_id,
            welcome_bytes,
            ratchet_tree_bytes,
            signer_bytes,
            expected_local_leaf,
            expected_resulting_state,
            expected_target_key_package_sha256,
            storage,
        } => {
            validate_profile(*profile_id)?;
            validate_result_range(welcome_bytes, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
            if let Some(tree) = ratchet_tree_bytes {
                validate_result_range(tree, 1, NATIVE_RECEIVE_RATCHET_TREE_MAX_BYTES)?;
            }
            validate_result_range(signer_bytes, 1, NATIVE_RECEIVE_SIGNER_MAX_BYTES)?;
            validate_exact_result_bytes(expected_target_key_package_sha256, SHA256_BYTES)?;
            let mut encoder = Encoder::new(&mut payload);
            encoder
                .map(8)
                .and_then(|value| value.u8(0))
                .and_then(|value| value.u16(*profile_id))
                .and_then(|value| value.u8(1))
                .and_then(|value| value.bytes(welcome_bytes))
                .and_then(|value| value.u8(2))
                .map_err(encode_error)?;
            match ratchet_tree_bytes {
                Some(tree) => encoder.bytes(tree).map_err(encode_error)?,
                None => encoder.null().map_err(encode_error)?,
            };
            encoder
                .u8(3)
                .and_then(|value| value.bytes(signer_bytes))
                .and_then(|value| value.u8(4))
                .map_err(encode_error)?;
            encode_leaf(&mut encoder, expected_local_leaf)?;
            encoder.u8(5).map_err(encode_error)?;
            encode_expected_roster(&mut encoder, expected_resulting_state)?;
            encoder
                .u8(6)
                .and_then(|value| value.bytes(expected_target_key_package_sha256))
                .and_then(|value| value.u8(7))
                .map_err(encode_error)?;
            encode_storage_snapshot(&mut encoder, storage, None)?;
            NativeReceiveOperationV1::Welcome
        }
    };
    encode_frame_with_max(operation as u8, payload, NATIVE_RECEIVE_REQUEST_MAX_BYTES)
}

/// Execute one receive operation without retaining group state or touching a
/// caller database. The returned frame always has `stateApplied=false`.
///
/// Panics are contained before a native wrapper boundary. Callers must still
/// serialize calls that can mutate the same installation/group snapshot.
pub fn execute_native_receive_v1(frame: &[u8]) -> Vec<u8> {
    let operation = operation_hint(frame);
    let outcome = match catch_unwind(AssertUnwindSafe(|| {
        let request = decode_native_receive_request_v1(frame)?;
        execute_request(request)
    })) {
        Ok(Ok(result)) => NativeReceiveOutcomeV1::success(result),
        Ok(Err(error)) => NativeReceiveOutcomeV1::failure(error),
        Err(_) => NativeReceiveOutcomeV1::failure(NativeReceiveErrorCodeV1::InternalFailure),
    };
    match encode_native_receive_outcome_v1(operation, &outcome) {
        Ok(frame) => frame,
        Err(error) => {
            let failure = NativeReceiveOutcomeV1::failure(error);
            encode_native_receive_outcome_v1(operation, &failure)
                .unwrap_or_else(|_| encode_failure_fallback(operation))
        }
    }
}

pub(crate) fn encode_native_receive_failure_v1(
    operation: Option<NativeReceiveOperationV1>,
    error: NativeReceiveErrorCodeV1,
) -> Vec<u8> {
    let outcome = NativeReceiveOutcomeV1::failure(error);
    encode_native_receive_outcome_v1(operation, &outcome)
        .unwrap_or_else(|_| encode_failure_fallback(operation))
}

fn execute_request(
    mut request: NativeReceiveRequestV1,
) -> Result<NativeReceiveSuccessV1, NativeReceiveErrorCodeV1> {
    match &mut request {
        NativeReceiveRequestV1::Process {
            operation,
            profile_id: _,
            group_id,
            message_bytes,
            expected_aad,
            expected_sender,
            expected_previous_state,
            expected_resulting_state,
            expected_base_group_state_sha256,
            storage,
        } => execute_process(
            *operation,
            std::mem::take(group_id),
            std::mem::take(message_bytes),
            std::mem::take(expected_aad),
            std::mem::take(expected_sender),
            std::mem::take(expected_previous_state),
            std::mem::take(expected_resulting_state),
            std::mem::take(expected_base_group_state_sha256),
            std::mem::take(storage),
        ),
        NativeReceiveRequestV1::Welcome {
            profile_id: _,
            welcome_bytes,
            ratchet_tree_bytes,
            signer_bytes,
            expected_local_leaf,
            expected_resulting_state,
            expected_target_key_package_sha256,
            storage,
        } => execute_welcome(
            std::mem::take(welcome_bytes),
            std::mem::take(ratchet_tree_bytes),
            std::mem::take(signer_bytes),
            std::mem::take(expected_local_leaf),
            std::mem::take(expected_resulting_state),
            std::mem::take(expected_target_key_package_sha256),
            std::mem::take(storage),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_process(
    operation: NativeReceiveOperationV1,
    group_id: Vec<u8>,
    message_bytes: Vec<u8>,
    expected_aad: Vec<u8>,
    expected_sender: NativeLeafAuthorityV1,
    expected_previous_state: NativeExpectedRosterStateV1,
    expected_resulting_state: NativeExpectedRosterStateV1,
    expected_base_group_state_sha256: Vec<u8>,
    storage: NativeStorageSnapshotV1,
) -> Result<NativeReceiveSuccessV1, NativeReceiveErrorCodeV1> {
    if operation == NativeReceiveOperationV1::Welcome {
        return Err(NativeReceiveErrorCodeV1::UnsupportedOperation);
    }
    let storage_format_version = storage.storage_format_version;
    let mut source_entries = SensitiveMlsEntries(
        storage
            .entries
            .into_iter()
            .map(native_storage_entry_into_mls)
            .collect::<Vec<_>>(),
    );
    let actual_base =
        group_state_digest_from_entries(&group_id, &source_entries.0, storage_format_version)
            .map_err(|_| NativeReceiveErrorCodeV1::InvalidStorageSnapshot)?;
    if actual_base != expected_base_group_state_sha256 {
        return Err(NativeReceiveErrorCodeV1::BaseStateMismatch);
    }

    let mut resulting_entries = SensitiveMlsEntries(source_entries.0.clone());
    let storage_entries = std::mem::take(&mut source_entries.0);
    let profile = profile_config_v1();
    let mut processed = process_message_with_storage_typed(
        group_id.clone(),
        message_bytes,
        expected_aad,
        native_expected_into_mls(expected_previous_state),
        native_expected_into_mls(expected_resulting_state),
        Some(&profile),
        storage_entries,
        storage_format_version,
    )
    .map_err(|error| map_strict_receive_error(error.kind))?;

    let actual_operation = match processed.message_type {
        ProcessedMessageType::Application => NativeReceiveOperationV1::Application,
        ProcessedMessageType::StagedCommit => NativeReceiveOperationV1::Commit,
        ProcessedMessageType::Proposal => {
            zeroize_process_result(&mut processed);
            return Err(NativeReceiveErrorCodeV1::MessageKindMismatch);
        }
    };
    if actual_operation != operation {
        zeroize_process_result(&mut processed);
        return Err(NativeReceiveErrorCodeV1::MessageKindMismatch);
    }

    let authenticated_sender = authenticated_sender(&processed)
        .filter(|actual| mls_leaf_matches_native(actual, &expected_sender));
    let Some(authenticated_sender) = authenticated_sender else {
        zeroize_process_result(&mut processed);
        return Err(NativeReceiveErrorCodeV1::SenderMismatch);
    };

    apply_storage_batch(&mut resulting_entries.0, &processed.storage_batch);
    let resulting_group_state_sha256 =
        group_state_digest_from_entries(&group_id, &resulting_entries.0, storage_format_version)
            .map_err(|_| {
                zeroize_process_result(&mut processed);
                NativeReceiveErrorCodeV1::InternalFailure
            })?;

    let sender = mls_leaf_into_native(authenticated_sender);
    let plaintext = match operation {
        NativeReceiveOperationV1::Application => {
            let Some(plaintext) = processed.application_message.take() else {
                zeroize_process_result(&mut processed);
                return Err(NativeReceiveErrorCodeV1::InternalFailure);
            };
            Some(plaintext)
        }
        NativeReceiveOperationV1::Commit => {
            if processed.application_message.is_some() {
                zeroize_process_result(&mut processed);
                return Err(NativeReceiveErrorCodeV1::InternalFailure);
            }
            None
        }
        NativeReceiveOperationV1::Welcome => unreachable!("Welcome rejected above"),
    };
    let previous_roster = mls_roster_into_native(processed.previous_roster);
    let resulting_roster = mls_roster_into_native(processed.resulting_roster);
    let storage_batch = mls_batch_into_native(processed.storage_batch);
    match operation {
        NativeReceiveOperationV1::Application => Ok(NativeReceiveSuccessV1::Application {
            sender,
            previous_roster,
            resulting_roster,
            resulting_group_state_sha256,
            plaintext: plaintext.unwrap_or_default(),
            storage_batch,
        }),
        NativeReceiveOperationV1::Commit => Ok(NativeReceiveSuccessV1::Commit {
            sender,
            previous_roster,
            resulting_roster,
            resulting_group_state_sha256,
            storage_batch,
        }),
        NativeReceiveOperationV1::Welcome => Err(NativeReceiveErrorCodeV1::InternalFailure),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_welcome(
    welcome_bytes: Vec<u8>,
    ratchet_tree_bytes: Option<Vec<u8>>,
    signer_bytes: Vec<u8>,
    expected_local_leaf: NativeLeafAuthorityV1,
    expected_resulting_state: NativeExpectedRosterStateV1,
    expected_target_key_package_sha256: Vec<u8>,
    storage: NativeStorageSnapshotV1,
) -> Result<NativeReceiveSuccessV1, NativeReceiveErrorCodeV1> {
    let storage_format_version = storage.storage_format_version;
    let mut source_entries = SensitiveMlsEntries(
        storage
            .entries
            .into_iter()
            .map(native_storage_entry_into_mls)
            .collect::<Vec<_>>(),
    );
    let mut resulting_entries = SensitiveMlsEntries(source_entries.0.clone());
    let storage_entries = std::mem::take(&mut source_entries.0);
    let mut joined = join_group_from_welcome_with_storage_typed(
        profile_config_v1(),
        welcome_bytes,
        ratchet_tree_bytes,
        signer_bytes,
        native_expected_into_mls(expected_resulting_state),
        Some(&expected_target_key_package_sha256),
        storage_entries,
        storage_format_version,
    )
    .map_err(|error| map_strict_receive_error(error.kind))?;

    if !mls_leaf_matches_native(&joined.local_leaf, &expected_local_leaf) {
        zeroize_join_result(&mut joined);
        return Err(NativeReceiveErrorCodeV1::LocalLeafMismatch);
    }
    apply_storage_batch(&mut resulting_entries.0, &joined.joined.storage_batch);
    let resulting_group_state_sha256 = group_state_digest_from_entries(
        &joined.joined.group_id,
        &resulting_entries.0,
        storage_format_version,
    )
    .map_err(|_| {
        zeroize_join_result(&mut joined);
        NativeReceiveErrorCodeV1::InternalFailure
    })?;

    Ok(NativeReceiveSuccessV1::Welcome {
        local_leaf: mls_leaf_into_native(joined.local_leaf),
        resulting_roster: mls_roster_into_native(joined.joined.resulting_roster),
        resulting_group_state_sha256,
        consumed_key_package_sha256: joined.consumed_key_package_sha256,
        storage_batch: mls_batch_into_native(joined.joined.storage_batch),
    })
}

fn profile_config_v1() -> MlsGroupConfig {
    MlsGroupConfig {
        ciphersuite: MlsCiphersuite::Mls128DhkemX25519Aes128gcmSha256Ed25519,
        wire_format_policy: MlsWireFormatPolicy::Ciphertext,
        use_ratchet_tree_extension: true,
        max_past_epochs: 0,
        padding_size: 0,
        sender_ratchet_max_out_of_order: 5,
        sender_ratchet_max_forward_distance: 1000,
        number_of_resumption_psks: 0,
    }
}

fn map_strict_receive_error(kind: StrictReceiveErrorKind) -> NativeReceiveErrorCodeV1 {
    match kind {
        StrictReceiveErrorKind::StorageFormatMismatch => {
            NativeReceiveErrorCodeV1::StorageFormatMismatch
        }
        StrictReceiveErrorKind::InvalidStorageSnapshot => {
            NativeReceiveErrorCodeV1::InvalidStorageSnapshot
        }
        StrictReceiveErrorKind::GroupStateUnavailable => {
            NativeReceiveErrorCodeV1::GroupStateUnavailable
        }
        StrictReceiveErrorKind::ConfigurationMismatch => {
            NativeReceiveErrorCodeV1::ConfigurationMismatch
        }
        StrictReceiveErrorKind::GroupMismatch => NativeReceiveErrorCodeV1::GroupMismatch,
        StrictReceiveErrorKind::PreviousEpochMismatch => {
            NativeReceiveErrorCodeV1::PreviousEpochMismatch
        }
        StrictReceiveErrorKind::PreviousRosterMismatch => {
            NativeReceiveErrorCodeV1::PreviousRosterMismatch
        }
        StrictReceiveErrorKind::ResultingEpochMismatch => {
            NativeReceiveErrorCodeV1::ResultingEpochMismatch
        }
        StrictReceiveErrorKind::ResultingRosterMismatch => {
            NativeReceiveErrorCodeV1::ResultingRosterMismatch
        }
        StrictReceiveErrorKind::AadMismatch => NativeReceiveErrorCodeV1::AadMismatch,
        StrictReceiveErrorKind::MessageKindMismatch => {
            NativeReceiveErrorCodeV1::MessageKindMismatch
        }
        StrictReceiveErrorKind::LocalLeafMismatch => NativeReceiveErrorCodeV1::LocalLeafMismatch,
        StrictReceiveErrorKind::InvalidSigner => NativeReceiveErrorCodeV1::InvalidSigner,
        StrictReceiveErrorKind::UnsupportedCredential => {
            NativeReceiveErrorCodeV1::UnsupportedCredential
        }
        StrictReceiveErrorKind::MlsDecodeRejected => NativeReceiveErrorCodeV1::MlsDecodeRejected,
        StrictReceiveErrorKind::WelcomeRejected => NativeReceiveErrorCodeV1::WelcomeRejected,
        StrictReceiveErrorKind::MlsProtocolRejected => {
            NativeReceiveErrorCodeV1::MlsProtocolRejected
        }
        StrictReceiveErrorKind::ExpectedKeyPackageMismatch => {
            NativeReceiveErrorCodeV1::ExpectedKeyPackageMismatch
        }
        StrictReceiveErrorKind::InternalFailure => NativeReceiveErrorCodeV1::InternalFailure,
    }
}

fn native_expected_into_mls(value: NativeExpectedRosterStateV1) -> MlsExpectedRosterStateV1 {
    MlsExpectedRosterStateV1 {
        group_id: value.group_id,
        epoch: value.epoch,
        digest_sha256: value.digest_sha256,
    }
}

fn native_storage_entry_into_mls(value: NativeStorageEntryV1) -> MlsStorageEntry {
    MlsStorageEntry {
        key: value.key,
        value: value.value,
        group_id: value.group_id,
    }
}

fn mls_leaf_into_native(value: MlsRosterLeafV1) -> NativeLeafAuthorityV1 {
    NativeLeafAuthorityV1 {
        leaf_index: value.leaf_index,
        credential_identity: value.credential_identity,
        signature_public_key: value.signature_public_key,
    }
}

fn mls_roster_into_native(value: MlsRosterSummaryV1) -> NativeRosterSummaryV1 {
    NativeRosterSummaryV1 {
        group_id: value.group_id,
        epoch: value.epoch,
        leaves: value.leaves.into_iter().map(mls_leaf_into_native).collect(),
        digest_sha256: value.digest_sha256,
    }
}

fn mls_batch_into_native(value: MlsStorageBatch) -> NativeStorageBatchV1 {
    NativeStorageBatchV1 {
        storage_format_version: value.storage_format_version,
        upserts: value
            .upserts
            .into_iter()
            .map(|entry| NativeStorageEntryV1 {
                key: entry.key,
                value: entry.value,
                group_id: entry.group_id,
            })
            .collect(),
        deletes: value.deletes,
        deleted_group_ids: value.deleted_group_ids,
    }
}

fn authenticated_sender(result: &ProcessMessageWithStorageResult) -> Option<MlsRosterLeafV1> {
    let sender_index = result.sender_index?;
    result
        .previous_roster
        .leaves
        .iter()
        .find(|leaf| leaf.leaf_index == sender_index)
        .cloned()
}

fn mls_leaf_matches_native(actual: &MlsRosterLeafV1, expected: &NativeLeafAuthorityV1) -> bool {
    actual.leaf_index == expected.leaf_index
        && actual.credential_identity == expected.credential_identity
        && actual.signature_public_key == expected.signature_public_key
}

fn apply_storage_batch(entries: &mut Vec<MlsStorageEntry>, batch: &MlsStorageBatch) {
    let mut by_key: BTreeMap<Vec<u8>, MlsStorageEntry> = entries
        .drain(..)
        .map(|entry| (entry.key.clone(), entry))
        .collect();
    for key in &batch.deletes {
        if let Some(mut removed) = by_key.remove(key) {
            removed.value.zeroize();
        }
    }
    for upsert in &batch.upserts {
        if let Some(mut replaced) = by_key.insert(upsert.key.clone(), upsert.clone()) {
            replaced.value.zeroize();
        }
    }
    for deleted_group_id in &batch.deleted_group_ids {
        by_key.retain(|_, entry| {
            let keep = entry.group_id.as_deref() != Some(deleted_group_id.as_slice());
            if !keep {
                entry.value.zeroize();
            }
            keep
        });
    }
    *entries = by_key.into_values().collect();
}

fn zeroize_process_result(result: &mut ProcessMessageWithStorageResult) {
    if let Some(plaintext) = &mut result.application_message {
        plaintext.zeroize();
    }
    zeroize_entry_values(&mut result.storage_batch.upserts);
}

fn zeroize_join_result(result: &mut StrictJoinGroupWithStorageResult) {
    zeroize_entry_values(&mut result.joined.storage_batch.upserts);
}

struct SensitiveMlsEntries(Vec<MlsStorageEntry>);

impl Drop for SensitiveMlsEntries {
    fn drop(&mut self) {
        zeroize_entry_values(&mut self.0);
    }
}

fn operation_hint(frame: &[u8]) -> Option<NativeReceiveOperationV1> {
    frame
        .get(6)
        .and_then(|value| NativeReceiveOperationV1::from_u8(*value).ok())
}

fn encode_failure_fallback(operation: Option<NativeReceiveOperationV1>) -> Vec<u8> {
    let payload = [0xa3, 0x00, 0x01, 0x01, 0xf4, 0x03, 0xa1, 0x03, 0x18, 0xff];
    let mut frame = Vec::with_capacity(FRAME_HEADER_BYTES + payload.len());
    frame.extend_from_slice(FRAME_MAGIC);
    frame.extend_from_slice(&NATIVE_RECEIVE_CONTRACT_VERSION.to_be_bytes());
    frame.push(operation.map_or(0, |value| value as u8));
    frame.push(0);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    frame
}

pub fn encode_native_receive_outcome_v1(
    operation: Option<NativeReceiveOperationV1>,
    outcome: &NativeReceiveOutcomeV1,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    if outcome.state_applied || outcome.result.is_some() == outcome.error.is_some() {
        return Err(NativeReceiveErrorCodeV1::InternalFailure);
    }
    let actual_operation = outcome
        .result
        .as_ref()
        .map(NativeReceiveSuccessV1::operation)
        .or(operation);
    let mut payload = Vec::new();
    let mut encoder = Encoder::new(&mut payload);
    if let Some(result) = &outcome.result {
        encoder
            .map(3)
            .and_then(|encoder| encoder.u8(0))
            .and_then(|encoder| encoder.u16(NATIVE_RECEIVE_CONTRACT_VERSION))
            .and_then(|encoder| encoder.u8(1))
            .and_then(|encoder| encoder.bool(false))
            .and_then(|encoder| encoder.u8(2))
            .map_err(|_| NativeReceiveErrorCodeV1::InternalFailure)?;
        encode_success(&mut encoder, result)?;
    } else {
        let error = outcome
            .error
            .ok_or(NativeReceiveErrorCodeV1::InternalFailure)?;
        encoder
            .map(3)
            .and_then(|encoder| encoder.u8(0))
            .and_then(|encoder| encoder.u16(NATIVE_RECEIVE_CONTRACT_VERSION))
            .and_then(|encoder| encoder.u8(1))
            .and_then(|encoder| encoder.bool(false))
            .and_then(|encoder| encoder.u8(3))
            .and_then(|encoder| encoder.map(1))
            .and_then(|encoder| encoder.u8(3))
            .and_then(|encoder| encoder.u16(error as u16))
            .map_err(|_| NativeReceiveErrorCodeV1::InternalFailure)?;
    }
    encode_frame(actual_operation.map_or(0, |value| value as u8), payload)
}

struct DecodedHeader<'a> {
    operation: u8,
    payload: &'a [u8],
}

fn decode_header(
    frame: &[u8],
    maximum: usize,
) -> Result<DecodedHeader<'_>, NativeReceiveErrorCodeV1> {
    if frame.len() > maximum {
        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
    }
    if frame.len() < FRAME_HEADER_BYTES || &frame[..4] != FRAME_MAGIC {
        return Err(NativeReceiveErrorCodeV1::InvalidFrame);
    }
    let version = u16::from_be_bytes([frame[4], frame[5]]);
    if version != NATIVE_RECEIVE_CONTRACT_VERSION {
        return Err(NativeReceiveErrorCodeV1::UnsupportedContractVersion);
    }
    if frame[7] != 0 {
        return Err(NativeReceiveErrorCodeV1::InvalidFrame);
    }
    let payload_len = u32::from_be_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
    if payload_len != frame.len() - FRAME_HEADER_BYTES {
        return Err(NativeReceiveErrorCodeV1::InvalidFrame);
    }
    Ok(DecodedHeader {
        operation: frame[6],
        payload: &frame[FRAME_HEADER_BYTES..],
    })
}

fn encode_frame(operation: u8, payload: Vec<u8>) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    encode_frame_with_max(operation, payload, NATIVE_RECEIVE_RESULT_MAX_BYTES)
}

fn encode_frame_with_max(
    operation: u8,
    mut payload: Vec<u8>,
    maximum: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    let frame_len = FRAME_HEADER_BYTES
        .checked_add(payload.len())
        .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
    if frame_len > maximum {
        payload.zeroize();
        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
    }
    let payload_len =
        u32::try_from(payload.len()).map_err(|_| NativeReceiveErrorCodeV1::LimitExceeded)?;
    let mut frame = Vec::with_capacity(frame_len);
    frame.extend_from_slice(FRAME_MAGIC);
    frame.extend_from_slice(&NATIVE_RECEIVE_CONTRACT_VERSION.to_be_bytes());
    frame.push(operation);
    frame.push(0);
    frame.extend_from_slice(&payload_len.to_be_bytes());
    frame.append(&mut payload);
    Ok(frame)
}

fn decode_process_request(
    decoder: &mut Decoder<'_>,
    operation: NativeReceiveOperationV1,
) -> Result<NativeReceiveRequestV1, NativeReceiveErrorCodeV1> {
    expect_map(decoder, 9)?;
    expect_key(decoder, 0)?;
    let profile_id = decoder
        .u16()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    validate_profile(profile_id)?;
    expect_key(decoder, 1)?;
    let group_id = decode_exact_bytes(decoder, PROFILE_GROUP_ID_BYTES)?;
    expect_key(decoder, 2)?;
    let message_bytes = decode_bounded_bytes(decoder, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
    expect_key(decoder, 3)?;
    let expected_aad = decode_bounded_bytes(decoder, 1, NATIVE_RECEIVE_AAD_MAX_BYTES)?;
    expect_key(decoder, 4)?;
    let expected_sender = decode_leaf(decoder)?;
    expect_key(decoder, 5)?;
    let expected_previous_state = decode_expected_roster(decoder)?;
    expect_key(decoder, 6)?;
    let expected_resulting_state = decode_expected_roster(decoder)?;
    expect_key(decoder, 7)?;
    let expected_base_group_state_sha256 = decode_exact_bytes(decoder, SHA256_BYTES)?;
    expect_key(decoder, 8)?;
    let storage = decode_storage_snapshot(decoder, Some(&group_id))?;
    Ok(NativeReceiveRequestV1::Process {
        operation,
        profile_id,
        group_id,
        message_bytes,
        expected_aad,
        expected_sender,
        expected_previous_state,
        expected_resulting_state,
        expected_base_group_state_sha256,
        storage,
    })
}

fn decode_welcome_request(
    decoder: &mut Decoder<'_>,
) -> Result<NativeReceiveRequestV1, NativeReceiveErrorCodeV1> {
    expect_map(decoder, 8)?;
    expect_key(decoder, 0)?;
    let profile_id = decoder
        .u16()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    validate_profile(profile_id)?;
    expect_key(decoder, 1)?;
    let welcome_bytes = decode_bounded_bytes(decoder, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
    expect_key(decoder, 2)?;
    let ratchet_tree_bytes = match decoder
        .datatype()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?
    {
        Type::Null => {
            decoder
                .null()
                .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
            None
        }
        Type::Bytes => Some(decode_bounded_bytes(
            decoder,
            1,
            NATIVE_RECEIVE_RATCHET_TREE_MAX_BYTES,
        )?),
        _ => return Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding),
    };
    expect_key(decoder, 3)?;
    let signer_bytes = decode_bounded_bytes(decoder, 1, NATIVE_RECEIVE_SIGNER_MAX_BYTES)?;
    expect_key(decoder, 4)?;
    let expected_local_leaf = decode_leaf(decoder)?;
    expect_key(decoder, 5)?;
    let expected_resulting_state = decode_expected_roster(decoder)?;
    expect_key(decoder, 6)?;
    let expected_target_key_package_sha256 = decode_exact_bytes(decoder, SHA256_BYTES)?;
    expect_key(decoder, 7)?;
    let storage = decode_storage_snapshot(decoder, None)?;
    Ok(NativeReceiveRequestV1::Welcome {
        profile_id,
        welcome_bytes,
        ratchet_tree_bytes,
        signer_bytes,
        expected_local_leaf,
        expected_resulting_state,
        expected_target_key_package_sha256,
        storage,
    })
}

fn validate_profile(profile_id: u16) -> Result<(), NativeReceiveErrorCodeV1> {
    if profile_id == NATIVE_RECEIVE_PROFILE_V1 {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV1::UnsupportedProfile)
    }
}

fn decode_leaf(
    decoder: &mut Decoder<'_>,
) -> Result<NativeLeafAuthorityV1, NativeReceiveErrorCodeV1> {
    expect_map(decoder, 3)?;
    expect_key(decoder, 0)?;
    let leaf_index = decoder
        .u32()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    expect_key(decoder, 1)?;
    let credential_identity = decode_exact_bytes(decoder, PROFILE_CREDENTIAL_IDENTITY_BYTES)?;
    expect_key(decoder, 2)?;
    let signature_public_key = decode_exact_bytes(decoder, PROFILE_SIGNATURE_PUBLIC_KEY_BYTES)?;
    Ok(NativeLeafAuthorityV1 {
        leaf_index,
        credential_identity,
        signature_public_key,
    })
}

fn decode_expected_roster(
    decoder: &mut Decoder<'_>,
) -> Result<NativeExpectedRosterStateV1, NativeReceiveErrorCodeV1> {
    expect_map(decoder, 3)?;
    expect_key(decoder, 0)?;
    let group_id = decode_exact_bytes(decoder, PROFILE_GROUP_ID_BYTES)?;
    expect_key(decoder, 1)?;
    let epoch = decoder
        .u64()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    expect_key(decoder, 2)?;
    let digest_sha256 = decode_exact_bytes(decoder, SHA256_BYTES)?;
    Ok(NativeExpectedRosterStateV1 {
        group_id,
        epoch,
        digest_sha256,
    })
}

fn decode_storage_snapshot(
    decoder: &mut Decoder<'_>,
    expected_group_id: Option<&[u8]>,
) -> Result<NativeStorageSnapshotV1, NativeReceiveErrorCodeV1> {
    expect_map(decoder, 2)?;
    expect_key(decoder, 0)?;
    let storage_format_version = decoder
        .u32()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    if storage_format_version != 1 {
        return Err(NativeReceiveErrorCodeV1::StorageFormatMismatch);
    }
    expect_key(decoder, 1)?;
    let count = expect_array_max(decoder, NATIVE_RECEIVE_STORAGE_MAX_ENTRIES)?;
    let mut entries = Vec::with_capacity(count);
    let mut total = 0usize;
    for _ in 0..count {
        let entry = decode_storage_entry(decoder)?;
        total = total
            .checked_add(entry.key.len())
            .and_then(|value| value.checked_add(entry.value.len()))
            .and_then(|value| value.checked_add(entry.group_id.as_ref().map_or(0, Vec::len)))
            .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
        if total > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
            zeroize_entries(&mut entries);
            return Err(NativeReceiveErrorCodeV1::LimitExceeded);
        }
        if let Some(group_id) = &entry.group_id {
            match expected_group_id {
                Some(expected) if group_id == expected => {}
                _ => {
                    zeroize_entries(&mut entries);
                    return Err(NativeReceiveErrorCodeV1::InvalidStorageSnapshot);
                }
            }
        }
        entries.push(entry);
    }
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
        zeroize_entries(&mut entries);
        return Err(NativeReceiveErrorCodeV1::InvalidStorageSnapshot);
    }
    Ok(NativeStorageSnapshotV1 {
        storage_format_version,
        entries,
    })
}

fn decode_storage_entry(
    decoder: &mut Decoder<'_>,
) -> Result<NativeStorageEntryV1, NativeReceiveErrorCodeV1> {
    expect_map(decoder, 3)?;
    expect_key(decoder, 0)?;
    let key = decode_bounded_bytes(decoder, 1, NATIVE_RECEIVE_STORAGE_KEY_MAX_BYTES)?;
    expect_key(decoder, 1)?;
    let value = decode_bounded_bytes(decoder, 0, NATIVE_RECEIVE_STORAGE_VALUE_MAX_BYTES)?;
    expect_key(decoder, 2)?;
    let group_id = match decoder
        .datatype()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?
    {
        Type::Null => {
            decoder
                .null()
                .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
            None
        }
        Type::Bytes => Some(decode_exact_bytes(decoder, PROFILE_GROUP_ID_BYTES)?),
        _ => return Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding),
    };
    Ok(NativeStorageEntryV1 {
        key,
        value,
        group_id,
    })
}

fn encode_success(
    encoder: &mut Encoder<&mut Vec<u8>>,
    result: &NativeReceiveSuccessV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    match result {
        NativeReceiveSuccessV1::Application {
            sender,
            previous_roster,
            resulting_roster,
            resulting_group_state_sha256,
            plaintext,
            storage_batch,
        } => {
            validate_result_bytes(plaintext, NATIVE_RECEIVE_PLAINTEXT_MAX_BYTES)?;
            validate_exact_result_bytes(resulting_group_state_sha256, SHA256_BYTES)?;
            encoder.map(6).map_err(encode_error)?;
            encode_pair_leaf(encoder, 0, sender)?;
            encode_pair_roster(encoder, 1, previous_roster)?;
            encode_pair_roster(encoder, 2, resulting_roster)?;
            encoder
                .u8(3)
                .and_then(|value| value.bytes(resulting_group_state_sha256))
                .map_err(encode_error)?;
            encoder
                .u8(4)
                .and_then(|value| value.bytes(plaintext))
                .map_err(encode_error)?;
            encoder.u8(5).map_err(encode_error)?;
            encode_storage_batch(encoder, storage_batch)?;
        }
        NativeReceiveSuccessV1::Commit {
            sender,
            previous_roster,
            resulting_roster,
            resulting_group_state_sha256,
            storage_batch,
        } => {
            validate_exact_result_bytes(resulting_group_state_sha256, SHA256_BYTES)?;
            encoder.map(5).map_err(encode_error)?;
            encode_pair_leaf(encoder, 0, sender)?;
            encode_pair_roster(encoder, 1, previous_roster)?;
            encode_pair_roster(encoder, 2, resulting_roster)?;
            encoder
                .u8(3)
                .and_then(|value| value.bytes(resulting_group_state_sha256))
                .map_err(encode_error)?;
            encoder.u8(4).map_err(encode_error)?;
            encode_storage_batch(encoder, storage_batch)?;
        }
        NativeReceiveSuccessV1::Welcome {
            local_leaf,
            resulting_roster,
            resulting_group_state_sha256,
            consumed_key_package_sha256,
            storage_batch,
        } => {
            validate_exact_result_bytes(resulting_group_state_sha256, SHA256_BYTES)?;
            validate_exact_result_bytes(consumed_key_package_sha256, SHA256_BYTES)?;
            encoder.map(5).map_err(encode_error)?;
            encode_pair_leaf(encoder, 0, local_leaf)?;
            encode_pair_roster(encoder, 1, resulting_roster)?;
            encoder
                .u8(2)
                .and_then(|value| value.bytes(resulting_group_state_sha256))
                .map_err(encode_error)?;
            encoder
                .u8(3)
                .and_then(|value| value.bytes(consumed_key_package_sha256))
                .map_err(encode_error)?;
            encoder.u8(4).map_err(encode_error)?;
            encode_storage_batch(encoder, storage_batch)?;
        }
    }
    Ok(())
}

fn encode_pair_leaf(
    encoder: &mut Encoder<&mut Vec<u8>>,
    key: u8,
    leaf: &NativeLeafAuthorityV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    encoder.u8(key).map_err(encode_error)?;
    encode_leaf(encoder, leaf)
}

fn encode_leaf(
    encoder: &mut Encoder<&mut Vec<u8>>,
    leaf: &NativeLeafAuthorityV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    validate_exact_result_bytes(&leaf.credential_identity, PROFILE_CREDENTIAL_IDENTITY_BYTES)?;
    validate_exact_result_bytes(
        &leaf.signature_public_key,
        PROFILE_SIGNATURE_PUBLIC_KEY_BYTES,
    )?;
    encoder
        .map(3)
        .and_then(|value| value.u8(0))
        .and_then(|value| value.u32(leaf.leaf_index))
        .and_then(|value| value.u8(1))
        .and_then(|value| value.bytes(&leaf.credential_identity))
        .and_then(|value| value.u8(2))
        .and_then(|value| value.bytes(&leaf.signature_public_key))
        .map_err(encode_error)?;
    Ok(())
}

fn encode_pair_roster(
    encoder: &mut Encoder<&mut Vec<u8>>,
    key: u8,
    roster: &NativeRosterSummaryV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    encoder.u8(key).map_err(encode_error)?;
    encode_roster(encoder, roster)
}

fn encode_expected_roster(
    encoder: &mut Encoder<&mut Vec<u8>>,
    roster: &NativeExpectedRosterStateV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    validate_exact_result_bytes(&roster.group_id, PROFILE_GROUP_ID_BYTES)?;
    validate_exact_result_bytes(&roster.digest_sha256, SHA256_BYTES)?;
    encoder
        .map(3)
        .and_then(|value| value.u8(0))
        .and_then(|value| value.bytes(&roster.group_id))
        .and_then(|value| value.u8(1))
        .and_then(|value| value.u64(roster.epoch))
        .and_then(|value| value.u8(2))
        .and_then(|value| value.bytes(&roster.digest_sha256))
        .map_err(encode_error)?;
    Ok(())
}

fn encode_storage_snapshot(
    encoder: &mut Encoder<&mut Vec<u8>>,
    snapshot: &NativeStorageSnapshotV1,
    expected_group_id: Option<&[u8]>,
) -> Result<(), NativeReceiveErrorCodeV1> {
    if snapshot.storage_format_version != 1
        || snapshot.entries.len() > NATIVE_RECEIVE_STORAGE_MAX_ENTRIES
    {
        return Err(NativeReceiveErrorCodeV1::StorageFormatMismatch);
    }
    let mut entries = snapshot.entries.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err(NativeReceiveErrorCodeV1::InvalidStorageSnapshot);
    }
    let mut total = 0usize;
    for entry in &entries {
        total = add_entry_size(total, entry)?;
        let is_global = crate::snapshot_storage::is_global_key(&entry.key);
        if (is_global && entry.group_id.is_some())
            || (!is_global && entry.group_id.as_deref() != expected_group_id)
        {
            return Err(NativeReceiveErrorCodeV1::InvalidStorageSnapshot);
        }
    }
    if total > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
    }
    encoder
        .map(2)
        .and_then(|value| value.u8(0))
        .and_then(|value| value.u32(snapshot.storage_format_version))
        .and_then(|value| value.u8(1))
        .and_then(|value| value.array(entries.len() as u64))
        .map_err(encode_error)?;
    for entry in entries {
        encode_storage_entry(encoder, entry)?;
    }
    Ok(())
}

fn encode_roster(
    encoder: &mut Encoder<&mut Vec<u8>>,
    roster: &NativeRosterSummaryV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    validate_exact_result_bytes(&roster.group_id, PROFILE_GROUP_ID_BYTES)?;
    validate_exact_result_bytes(&roster.digest_sha256, SHA256_BYTES)?;
    if roster.leaves.is_empty() || roster.leaves.len() > NATIVE_RECEIVE_ROSTER_MAX_LEAVES {
        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
    }
    if roster
        .leaves
        .windows(2)
        .any(|pair| pair[0].leaf_index >= pair[1].leaf_index)
    {
        return Err(NativeReceiveErrorCodeV1::InternalFailure);
    }
    encoder
        .map(4)
        .and_then(|value| value.u8(0))
        .and_then(|value| value.bytes(&roster.group_id))
        .and_then(|value| value.u8(1))
        .and_then(|value| value.u64(roster.epoch))
        .and_then(|value| value.u8(2))
        .and_then(|value| value.array(roster.leaves.len() as u64))
        .map_err(encode_error)?;
    for leaf in &roster.leaves {
        encode_leaf(encoder, leaf)?;
    }
    encoder
        .u8(3)
        .and_then(|value| value.bytes(&roster.digest_sha256))
        .map_err(encode_error)?;
    Ok(())
}

fn encode_storage_batch(
    encoder: &mut Encoder<&mut Vec<u8>>,
    batch: &NativeStorageBatchV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    if batch.storage_format_version != 1
        || batch.upserts.len() > NATIVE_RECEIVE_STORAGE_MAX_ENTRIES
        || batch.deletes.len() > NATIVE_RECEIVE_STORAGE_MAX_ENTRIES
        || batch.deleted_group_ids.len() > 8
    {
        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
    }
    let mut total = 0usize;
    encoder
        .map(4)
        .and_then(|value| value.u8(0))
        .and_then(|value| value.u32(batch.storage_format_version))
        .and_then(|value| value.u8(1))
        .and_then(|value| value.array(batch.upserts.len() as u64))
        .map_err(encode_error)?;
    for entry in &batch.upserts {
        total = add_entry_size(total, entry)?;
        encode_storage_entry(encoder, entry)?;
    }
    encoder
        .u8(2)
        .and_then(|value| value.array(batch.deletes.len() as u64))
        .map_err(encode_error)?;
    for key in &batch.deletes {
        validate_result_range(key, 1, NATIVE_RECEIVE_STORAGE_KEY_MAX_BYTES)?;
        total = total
            .checked_add(key.len())
            .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
        encoder.bytes(key).map_err(encode_error)?;
    }
    encoder
        .u8(3)
        .and_then(|value| value.array(batch.deleted_group_ids.len() as u64))
        .map_err(encode_error)?;
    for group_id in &batch.deleted_group_ids {
        validate_exact_result_bytes(group_id, PROFILE_GROUP_ID_BYTES)?;
        total = total
            .checked_add(group_id.len())
            .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
        encoder.bytes(group_id).map_err(encode_error)?;
    }
    if total > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
    }
    Ok(())
}

fn add_entry_size(
    current: usize,
    entry: &NativeStorageEntryV1,
) -> Result<usize, NativeReceiveErrorCodeV1> {
    validate_result_range(&entry.key, 1, NATIVE_RECEIVE_STORAGE_KEY_MAX_BYTES)?;
    validate_result_range(&entry.value, 0, NATIVE_RECEIVE_STORAGE_VALUE_MAX_BYTES)?;
    if let Some(group_id) = &entry.group_id {
        validate_exact_result_bytes(group_id, PROFILE_GROUP_ID_BYTES)?;
    }
    current
        .checked_add(entry.key.len())
        .and_then(|value| value.checked_add(entry.value.len()))
        .and_then(|value| value.checked_add(entry.group_id.as_ref().map_or(0, Vec::len)))
        .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)
}

fn encode_storage_entry(
    encoder: &mut Encoder<&mut Vec<u8>>,
    entry: &NativeStorageEntryV1,
) -> Result<(), NativeReceiveErrorCodeV1> {
    encoder
        .map(3)
        .and_then(|value| value.u8(0))
        .and_then(|value| value.bytes(&entry.key))
        .and_then(|value| value.u8(1))
        .and_then(|value| value.bytes(&entry.value))
        .and_then(|value| value.u8(2))
        .map_err(encode_error)?;
    match &entry.group_id {
        Some(group_id) => {
            encoder.bytes(group_id).map_err(encode_error)?;
        }
        None => {
            encoder.null().map_err(encode_error)?;
        }
    }
    Ok(())
}

fn validate_result_bytes(value: &[u8], maximum: usize) -> Result<(), NativeReceiveErrorCodeV1> {
    validate_result_range(value, 0, maximum)
}

fn validate_exact_result_bytes(
    value: &[u8],
    expected: usize,
) -> Result<(), NativeReceiveErrorCodeV1> {
    if value.len() == expected {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV1::InternalFailure)
    }
}

fn validate_result_range(
    value: &[u8],
    minimum: usize,
    maximum: usize,
) -> Result<(), NativeReceiveErrorCodeV1> {
    if value.len() < minimum || value.len() > maximum {
        Err(NativeReceiveErrorCodeV1::LimitExceeded)
    } else {
        Ok(())
    }
}

fn encode_error<E>(_error: minicbor::encode::Error<E>) -> NativeReceiveErrorCodeV1 {
    NativeReceiveErrorCodeV1::InternalFailure
}

fn expect_map(decoder: &mut Decoder<'_>, expected: usize) -> Result<(), NativeReceiveErrorCodeV1> {
    let length = decoder
        .map()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?
        .ok_or(NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    if length == expected as u64 {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding)
    }
}

fn expect_array_max(
    decoder: &mut Decoder<'_>,
    maximum: usize,
) -> Result<usize, NativeReceiveErrorCodeV1> {
    let length = decoder
        .array()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?
        .ok_or(NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    let length = usize::try_from(length).map_err(|_| NativeReceiveErrorCodeV1::LimitExceeded)?;
    if length > maximum {
        Err(NativeReceiveErrorCodeV1::LimitExceeded)
    } else {
        Ok(length)
    }
}

fn expect_key(decoder: &mut Decoder<'_>, expected: u8) -> Result<(), NativeReceiveErrorCodeV1> {
    let actual = decoder
        .u8()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    if actual == expected {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding)
    }
}

fn decode_exact_bytes(
    decoder: &mut Decoder<'_>,
    expected: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    decode_bounded_bytes(decoder, expected, expected)
}

fn decode_bounded_bytes(
    decoder: &mut Decoder<'_>,
    minimum: usize,
    maximum: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV1> {
    let value = decoder
        .bytes()
        .map_err(|_| NativeReceiveErrorCodeV1::NoncanonicalEncoding)?;
    if value.len() < minimum || value.len() > maximum {
        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
    }
    Ok(value.to_vec())
}

fn zeroize_entries(entries: &mut [NativeStorageEntryV1]) {
    for entry in entries {
        entry.value.zeroize();
    }
}

fn preflight_cbor(input: &[u8]) -> Result<(), NativeReceiveErrorCodeV1> {
    let mut scanner = CborPreflight {
        input,
        position: 0,
        items: 0,
    };
    scanner.value(0)?;
    if scanner.position != input.len() {
        return Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding);
    }
    Ok(())
}

struct CborPreflight<'a> {
    input: &'a [u8],
    position: usize,
    items: usize,
}

impl CborPreflight<'_> {
    fn value(&mut self, depth: usize) -> Result<(), NativeReceiveErrorCodeV1> {
        if depth > PREFLIGHT_MAX_DEPTH {
            return Err(NativeReceiveErrorCodeV1::LimitExceeded);
        }
        self.items = self
            .items
            .checked_add(1)
            .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
        if self.items > PREFLIGHT_MAX_ITEMS {
            return Err(NativeReceiveErrorCodeV1::LimitExceeded);
        }
        let initial = self.byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        match major {
            0 => {
                self.argument(additional)?;
            }
            2 => {
                let length = self.argument(additional)?;
                let length =
                    usize::try_from(length).map_err(|_| NativeReceiveErrorCodeV1::LimitExceeded)?;
                if length > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
                    return Err(NativeReceiveErrorCodeV1::LimitExceeded);
                }
                self.advance(length)?;
            }
            4 => {
                let length =
                    self.container_length(additional, NATIVE_RECEIVE_STORAGE_MAX_ENTRIES)?;
                for _ in 0..length {
                    self.value(depth + 1)?;
                }
            }
            5 => {
                let length = self.container_length(additional, PREFLIGHT_MAX_MAP_ENTRIES)?;
                let mut previous = None;
                for _ in 0..length {
                    let key_initial = self.byte()?;
                    if key_initial >> 5 != 0 {
                        return Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding);
                    }
                    let key = self.argument(key_initial & 0x1f)?;
                    if previous.is_some_and(|value| key <= value) {
                        return Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding);
                    }
                    previous = Some(key);
                    self.items = self
                        .items
                        .checked_add(1)
                        .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
                    if self.items > PREFLIGHT_MAX_ITEMS {
                        return Err(NativeReceiveErrorCodeV1::LimitExceeded);
                    }
                    self.value(depth + 1)?;
                }
            }
            7 if matches!(additional, 20..=22) => {}
            _ => return Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding),
        }
        Ok(())
    }

    fn container_length(
        &mut self,
        additional: u8,
        maximum: usize,
    ) -> Result<usize, NativeReceiveErrorCodeV1> {
        let length = usize::try_from(self.argument(additional)?)
            .map_err(|_| NativeReceiveErrorCodeV1::LimitExceeded)?;
        if length > maximum {
            Err(NativeReceiveErrorCodeV1::LimitExceeded)
        } else {
            Ok(length)
        }
    }

    fn argument(&mut self, additional: u8) -> Result<u64, NativeReceiveErrorCodeV1> {
        match additional {
            value @ 0..=23 => Ok(u64::from(value)),
            24 => {
                let value = u64::from(self.byte()?);
                if value < 24 {
                    Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            25 => {
                let bytes = self.bytes::<2>()?;
                let value = u64::from(u16::from_be_bytes(bytes));
                if value <= u64::from(u8::MAX) {
                    Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            26 => {
                let bytes = self.bytes::<4>()?;
                let value = u64::from(u32::from_be_bytes(bytes));
                if value <= u64::from(u16::MAX) {
                    Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            27 => {
                let value = u64::from_be_bytes(self.bytes::<8>()?);
                if value <= u64::from(u32::MAX) {
                    Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            _ => Err(NativeReceiveErrorCodeV1::NoncanonicalEncoding),
        }
    }

    fn byte(&mut self) -> Result<u8, NativeReceiveErrorCodeV1> {
        let value = *self
            .input
            .get(self.position)
            .ok_or(NativeReceiveErrorCodeV1::InvalidFrame)?;
        self.position += 1;
        Ok(value)
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], NativeReceiveErrorCodeV1> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
        let bytes = self
            .input
            .get(self.position..end)
            .ok_or(NativeReceiveErrorCodeV1::InvalidFrame)?;
        self.position = end;
        bytes
            .try_into()
            .map_err(|_| NativeReceiveErrorCodeV1::InvalidFrame)
    }

    fn advance(&mut self, length: usize) -> Result<(), NativeReceiveErrorCodeV1> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(NativeReceiveErrorCodeV1::LimitExceeded)?;
        if end > self.input.len() {
            return Err(NativeReceiveErrorCodeV1::InvalidFrame);
        }
        self.position = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    use crate::api::group_e2ee::{
        MlsAuthorizedKeyPackageV1, MlsAuthorizedOwnerV1, add_members_with_storage,
        create_group_with_storage,
    };
    use crate::api::keys::{MlsSignatureKeyPair, serialize_signer};
    use crate::api::storage::{
        MLS_STORAGE_FORMAT_VERSION, create_key_package_with_storage, create_message_with_storage,
    };

    fn frame(operation: NativeReceiveOperationV1, payload: Vec<u8>) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend_from_slice(FRAME_MAGIC);
        frame.extend_from_slice(&NATIVE_RECEIVE_CONTRACT_VERSION.to_be_bytes());
        frame.push(operation as u8);
        frame.push(0);
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&payload);
        frame
    }

    fn leaf(encoder: &mut Encoder<&mut Vec<u8>>, index: u32) {
        encoder.map(3).unwrap();
        encoder.u8(0).unwrap().u32(index).unwrap();
        encoder.u8(1).unwrap().bytes(&[7; 45]).unwrap();
        encoder.u8(2).unwrap().bytes(&[8; 32]).unwrap();
    }

    fn expected(encoder: &mut Encoder<&mut Vec<u8>>, epoch: u64) {
        encoder.map(3).unwrap();
        encoder.u8(0).unwrap().bytes(&[1; 16]).unwrap();
        encoder.u8(1).unwrap().u64(epoch).unwrap();
        encoder.u8(2).unwrap().bytes(&[2; 32]).unwrap();
    }

    fn empty_storage(encoder: &mut Encoder<&mut Vec<u8>>) {
        encoder.map(2).unwrap();
        encoder.u8(0).unwrap().u32(1).unwrap();
        encoder.u8(1).unwrap().array(0).unwrap();
    }

    #[test]
    fn executes_real_welcome_application_and_commit_with_strict_authority() {
        let mut fixture = InitialFixture::new();

        let welcome =
            execute_request(fixture.welcome_request(fixture.bob_key_package_sha256.clone()))
                .expect("strict Welcome succeeds");
        let NativeReceiveSuccessV1::Welcome {
            local_leaf,
            resulting_roster,
            resulting_group_state_sha256,
            consumed_key_package_sha256,
            storage_batch,
        } = &welcome
        else {
            panic!("Welcome returned a different operation");
        };
        assert_eq!(local_leaf, &fixture.bob_leaf);
        assert_eq!(
            resulting_roster.digest_sha256,
            fixture.two_member_roster.digest_sha256
        );
        assert_eq!(consumed_key_package_sha256, &fixture.bob_key_package_sha256);
        apply_native_batch(&mut fixture.bob_entries, storage_batch);
        assert_eq!(
            resulting_group_state_sha256.as_slice(),
            group_state_digest_from_entries(
                &fixture.group_id,
                &fixture.bob_entries,
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap()
            .as_slice()
        );

        let application_aad = b"native-application-aad".to_vec();
        let application = create_message_with_storage(
            fixture.group_id.clone(),
            fixture.alice_signer.clone(),
            b"native hello".to_vec(),
            application_aad.clone(),
            fixture.alice_entries.clone(),
            MLS_STORAGE_FORMAT_VERSION,
        )
        .unwrap();
        apply_storage_batch(&mut fixture.alice_entries, &application.storage_batch);
        let bob_application_base = group_state_digest_from_entries(
            &fixture.group_id,
            &fixture.bob_entries,
            MLS_STORAGE_FORMAT_VERSION,
        )
        .unwrap();
        let application_result = execute_request(NativeReceiveRequestV1::Process {
            operation: NativeReceiveOperationV1::Application,
            profile_id: NATIVE_RECEIVE_PROFILE_V1,
            group_id: fixture.group_id.clone(),
            message_bytes: application.ciphertext,
            expected_aad: application_aad,
            expected_sender: fixture.alice_leaf.clone(),
            expected_previous_state: native_expected(&fixture.two_member_roster),
            expected_resulting_state: native_expected(&fixture.two_member_roster),
            expected_base_group_state_sha256: bob_application_base,
            storage: native_snapshot(&fixture.bob_entries),
        })
        .expect("strict application receive succeeds");
        let NativeReceiveSuccessV1::Application {
            plaintext,
            storage_batch,
            resulting_group_state_sha256,
            ..
        } = &application_result
        else {
            panic!("application returned a different operation");
        };
        assert_eq!(plaintext, b"native hello");
        apply_native_batch(&mut fixture.bob_entries, storage_batch);
        assert_eq!(
            resulting_group_state_sha256.as_slice(),
            group_state_digest_from_entries(
                &fixture.group_id,
                &fixture.bob_entries,
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap()
            .as_slice()
        );

        let (charlie_signer, charlie_public) = signer();
        let charlie_identity = identity(3);
        let charlie_key_package = create_key_package_with_storage(
            ciphersuite(),
            charlie_signer,
            charlie_identity.clone(),
            charlie_public.clone(),
            None,
            Vec::new(),
            MLS_STORAGE_FORMAT_VERSION,
        )
        .unwrap();
        let commit_aad = b"native-commit-aad".to_vec();
        let commit = add_members_with_storage(
            fixture.group_id.clone(),
            fixture.alice_signer,
            vec![MlsAuthorizedKeyPackageV1 {
                key_package_bytes: charlie_key_package.key_package_bytes,
                expected_credential_identity: charlie_identity,
                expected_signature_public_key: charlie_public,
            }],
            commit_aad.clone(),
            expected_roster(&fixture.two_member_roster),
            fixture.alice_entries,
            MLS_STORAGE_FORMAT_VERSION,
        )
        .unwrap();
        let bob_commit_base = group_state_digest_from_entries(
            &fixture.group_id,
            &fixture.bob_entries,
            MLS_STORAGE_FORMAT_VERSION,
        )
        .unwrap();
        let commit_result = execute_request(NativeReceiveRequestV1::Process {
            operation: NativeReceiveOperationV1::Commit,
            profile_id: NATIVE_RECEIVE_PROFILE_V1,
            group_id: fixture.group_id.clone(),
            message_bytes: commit.commit,
            expected_aad: commit_aad,
            expected_sender: fixture.alice_leaf,
            expected_previous_state: native_expected(&fixture.two_member_roster),
            expected_resulting_state: native_expected(&commit.resulting_roster),
            expected_base_group_state_sha256: bob_commit_base,
            storage: native_snapshot(&fixture.bob_entries),
        })
        .expect("strict Commit receive succeeds");
        let NativeReceiveSuccessV1::Commit {
            resulting_roster,
            storage_batch,
            resulting_group_state_sha256,
            ..
        } = &commit_result
        else {
            panic!("Commit returned a different operation");
        };
        assert_eq!(
            resulting_roster.digest_sha256,
            commit.resulting_roster.digest_sha256
        );
        apply_native_batch(&mut fixture.bob_entries, storage_batch);
        assert_eq!(
            resulting_group_state_sha256.as_slice(),
            group_state_digest_from_entries(
                &fixture.group_id,
                &fixture.bob_entries,
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap()
            .as_slice()
        );
    }

    #[test]
    fn rejects_wrong_welcome_key_package_hash_before_returning_state() {
        let fixture = InitialFixture::new();
        let mut wrong_hash = fixture.bob_key_package_sha256.clone();
        wrong_hash[0] ^= 0xff;
        let error = execute_request(fixture.welcome_request(wrong_hash)).unwrap_err();
        assert_eq!(error, NativeReceiveErrorCodeV1::ExpectedKeyPackageMismatch);
    }

    #[test]
    fn strict_mismatch_vectors_return_typed_failures() {
        let fixture = ApplicationFixture::new();

        let mut wrong_base = fixture.base_digest.clone();
        wrong_base[0] ^= 0xff;
        assert_eq!(
            execute_request(fixture.request(
                NativeReceiveOperationV1::Application,
                fixture.aad.clone(),
                fixture.sender.clone(),
                native_expected(&fixture.roster),
                wrong_base,
            ))
            .unwrap_err(),
            NativeReceiveErrorCodeV1::BaseStateMismatch
        );

        assert_eq!(
            execute_request(fixture.request(
                NativeReceiveOperationV1::Application,
                b"wrong-aad".to_vec(),
                fixture.sender.clone(),
                native_expected(&fixture.roster),
                fixture.base_digest.clone(),
            ))
            .unwrap_err(),
            NativeReceiveErrorCodeV1::AadMismatch
        );

        let mut wrong_sender = fixture.sender.clone();
        wrong_sender.signature_public_key[0] ^= 0xff;
        assert_eq!(
            execute_request(fixture.request(
                NativeReceiveOperationV1::Application,
                fixture.aad.clone(),
                wrong_sender,
                native_expected(&fixture.roster),
                fixture.base_digest.clone(),
            ))
            .unwrap_err(),
            NativeReceiveErrorCodeV1::SenderMismatch
        );

        let mut wrong_roster = native_expected(&fixture.roster);
        wrong_roster.digest_sha256[0] ^= 0xff;
        assert_eq!(
            execute_request(fixture.request(
                NativeReceiveOperationV1::Application,
                fixture.aad.clone(),
                fixture.sender.clone(),
                wrong_roster,
                fixture.base_digest.clone(),
            ))
            .unwrap_err(),
            NativeReceiveErrorCodeV1::ResultingRosterMismatch
        );

        assert_eq!(
            execute_request(fixture.request(
                NativeReceiveOperationV1::Commit,
                fixture.aad.clone(),
                fixture.sender.clone(),
                native_expected(&fixture.roster),
                fixture.base_digest.clone(),
            ))
            .unwrap_err(),
            NativeReceiveErrorCodeV1::MessageKindMismatch
        );

        let initial = InitialFixture::new();
        let mut wrong_leaf = initial.bob_leaf.clone();
        wrong_leaf.leaf_index = wrong_leaf.leaf_index.saturating_add(1);
        let mut request = initial.welcome_request(initial.bob_key_package_sha256.clone());
        if let NativeReceiveRequestV1::Welcome {
            expected_local_leaf,
            ..
        } = &mut request
        {
            *expected_local_leaf = wrong_leaf;
        }
        assert_eq!(
            execute_request(request).unwrap_err(),
            NativeReceiveErrorCodeV1::LocalLeafMismatch
        );
    }

    #[test]
    fn canonical_request_encoder_roundtrips_identically() {
        let fixture = InitialFixture::new();
        let request = fixture.welcome_request(fixture.bob_key_package_sha256.clone());
        let first = encode_native_receive_request_v1(&request).unwrap();
        let decoded = decode_native_receive_request_v1(&first).unwrap();
        let second = encode_native_receive_request_v1(&decoded).unwrap();
        assert_eq!(first, second);
        assert!(matches!(decoded, NativeReceiveRequestV1::Welcome { .. }));
    }

    #[test]
    fn rejects_empty_process_aad_at_codec_boundary() {
        let fixture = ApplicationFixture::new();
        let request = fixture.request(
            NativeReceiveOperationV1::Application,
            Vec::new(),
            fixture.sender.clone(),
            native_expected(&fixture.roster),
            fixture.base_digest.clone(),
        );

        assert_eq!(
            encode_native_receive_request_v1(&request).unwrap_err(),
            NativeReceiveErrorCodeV1::LimitExceeded
        );

        // The fixture-only encoder constructs canonical CBOR that deliberately
        // violates the semantic lower bound, exercising the production decoder.
        let frame = encode_native_receive_request_v1_allow_empty_aad_fixture(&request).unwrap();
        assert_eq!(
            decode_native_receive_request_v1(&frame).unwrap_err(),
            NativeReceiveErrorCodeV1::LimitExceeded
        );
        let expected_failure = encode_native_receive_outcome_v1(
            Some(NativeReceiveOperationV1::Application),
            &NativeReceiveOutcomeV1::failure(NativeReceiveErrorCodeV1::LimitExceeded),
        )
        .unwrap();
        assert_eq!(execute_native_receive_v1(&frame), expected_failure);
    }

    #[test]
    fn decodes_canonical_process_request() {
        let mut payload = Vec::new();
        let mut encoder = Encoder::new(&mut payload);
        encoder.map(9).unwrap();
        encoder.u8(0).unwrap().u16(1).unwrap();
        encoder.u8(1).unwrap().bytes(&[1; 16]).unwrap();
        encoder.u8(2).unwrap().bytes(&[3]).unwrap();
        encoder.u8(3).unwrap().bytes(&[4]).unwrap();
        encoder.u8(4).unwrap();
        leaf(&mut encoder, 0);
        encoder.u8(5).unwrap();
        expected(&mut encoder, 1);
        encoder.u8(6).unwrap();
        expected(&mut encoder, 1);
        encoder.u8(7).unwrap().bytes(&[5; 32]).unwrap();
        encoder.u8(8).unwrap();
        empty_storage(&mut encoder);

        let request = decode_native_receive_request_v1(&frame(
            NativeReceiveOperationV1::Application,
            payload,
        ))
        .unwrap();
        assert!(matches!(
            request,
            NativeReceiveRequestV1::Process {
                operation: NativeReceiveOperationV1::Application,
                ..
            }
        ));
    }

    #[test]
    fn rejects_noncanonical_map_key_order_before_decode() {
        let payload = vec![0xa2, 0x01, 0x01, 0x00, 0x01];
        let error = decode_native_receive_request_v1(&frame(
            NativeReceiveOperationV1::Application,
            payload,
        ))
        .unwrap_err();
        assert_eq!(error, NativeReceiveErrorCodeV1::NoncanonicalEncoding);
    }

    struct InitialFixture {
        group_id: Vec<u8>,
        alice_signer: Vec<u8>,
        alice_entries: Vec<MlsStorageEntry>,
        bob_signer: Vec<u8>,
        bob_entries: Vec<MlsStorageEntry>,
        bob_key_package_sha256: Vec<u8>,
        welcome_bytes: Vec<u8>,
        two_member_roster: MlsRosterSummaryV1,
        alice_leaf: NativeLeafAuthorityV1,
        bob_leaf: NativeLeafAuthorityV1,
    }

    struct ApplicationFixture {
        group_id: Vec<u8>,
        wire: Vec<u8>,
        aad: Vec<u8>,
        sender: NativeLeafAuthorityV1,
        roster: MlsRosterSummaryV1,
        base_digest: Vec<u8>,
        entries: Vec<MlsStorageEntry>,
    }

    impl ApplicationFixture {
        fn new() -> Self {
            let mut initial = InitialFixture::new();
            let welcome =
                execute_request(initial.welcome_request(initial.bob_key_package_sha256.clone()))
                    .unwrap();
            let NativeReceiveSuccessV1::Welcome { storage_batch, .. } = &welcome else {
                panic!("fixture Welcome returned a different operation");
            };
            apply_native_batch(&mut initial.bob_entries, storage_batch);
            let aad = b"mismatch-vector-aad".to_vec();
            let application = create_message_with_storage(
                initial.group_id.clone(),
                initial.alice_signer,
                b"mismatch vector".to_vec(),
                aad.clone(),
                initial.alice_entries,
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap();
            let base_digest = group_state_digest_from_entries(
                &initial.group_id,
                &initial.bob_entries,
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap();
            Self {
                group_id: initial.group_id,
                wire: application.ciphertext,
                aad,
                sender: initial.alice_leaf,
                roster: initial.two_member_roster,
                base_digest,
                entries: initial.bob_entries,
            }
        }

        fn request(
            &self,
            operation: NativeReceiveOperationV1,
            expected_aad: Vec<u8>,
            expected_sender: NativeLeafAuthorityV1,
            expected_resulting_state: NativeExpectedRosterStateV1,
            expected_base_group_state_sha256: Vec<u8>,
        ) -> NativeReceiveRequestV1 {
            NativeReceiveRequestV1::Process {
                operation,
                profile_id: NATIVE_RECEIVE_PROFILE_V1,
                group_id: self.group_id.clone(),
                message_bytes: self.wire.clone(),
                expected_aad,
                expected_sender,
                expected_previous_state: native_expected(&self.roster),
                expected_resulting_state,
                expected_base_group_state_sha256,
                storage: native_snapshot(&self.entries),
            }
        }
    }

    impl InitialFixture {
        fn new() -> Self {
            let group_id = vec![0x41; PROFILE_GROUP_ID_BYTES];
            let alice_identity = identity(1);
            let bob_identity = identity(2);
            let (alice_signer, alice_public) = signer();
            let (bob_signer, bob_public) = signer();

            let created = create_group_with_storage(
                profile_config_v1(),
                alice_signer.clone(),
                group_id.clone(),
                MlsAuthorizedOwnerV1 {
                    expected_credential_identity: alice_identity.clone(),
                    expected_signature_public_key: alice_public,
                },
                None,
                Vec::new(),
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap();
            let mut alice_entries = Vec::new();
            apply_storage_batch(&mut alice_entries, &created.storage_batch);

            let bob_key_package = create_key_package_with_storage(
                ciphersuite(),
                bob_signer.clone(),
                bob_identity.clone(),
                bob_public.clone(),
                None,
                Vec::new(),
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap();
            let bob_key_package_sha256 =
                Sha256::digest(&bob_key_package.key_package_bytes).to_vec();
            let mut bob_entries = Vec::new();
            apply_storage_batch(&mut bob_entries, &bob_key_package.storage_batch);

            let add_bob = add_members_with_storage(
                group_id.clone(),
                alice_signer.clone(),
                vec![MlsAuthorizedKeyPackageV1 {
                    key_package_bytes: bob_key_package.key_package_bytes,
                    expected_credential_identity: bob_identity.clone(),
                    expected_signature_public_key: bob_public,
                }],
                b"native-add-bob-aad".to_vec(),
                expected_roster(&created.resulting_roster),
                alice_entries.clone(),
                MLS_STORAGE_FORMAT_VERSION,
            )
            .unwrap();
            apply_storage_batch(&mut alice_entries, &add_bob.storage_batch);
            let two_member_roster = add_bob.resulting_roster;
            let alice_leaf = native_leaf(
                two_member_roster
                    .leaves
                    .iter()
                    .find(|leaf| leaf.credential_identity == alice_identity)
                    .unwrap(),
            );
            let bob_leaf = native_leaf(
                two_member_roster
                    .leaves
                    .iter()
                    .find(|leaf| leaf.credential_identity == bob_identity)
                    .unwrap(),
            );

            Self {
                group_id,
                alice_signer,
                alice_entries,
                bob_signer,
                bob_entries,
                bob_key_package_sha256,
                welcome_bytes: add_bob.welcome.unwrap(),
                two_member_roster,
                alice_leaf,
                bob_leaf,
            }
        }

        fn welcome_request(&self, key_package_sha256: Vec<u8>) -> NativeReceiveRequestV1 {
            NativeReceiveRequestV1::Welcome {
                profile_id: NATIVE_RECEIVE_PROFILE_V1,
                welcome_bytes: self.welcome_bytes.clone(),
                ratchet_tree_bytes: None,
                signer_bytes: self.bob_signer.clone(),
                expected_local_leaf: self.bob_leaf.clone(),
                expected_resulting_state: native_expected(&self.two_member_roster),
                expected_target_key_package_sha256: key_package_sha256,
                storage: native_snapshot(&self.bob_entries),
            }
        }
    }

    fn signer() -> (Vec<u8>, Vec<u8>) {
        let key_pair = MlsSignatureKeyPair::generate(ciphersuite()).unwrap();
        let public = key_pair.public_key();
        let signer =
            serialize_signer(ciphersuite(), key_pair.private_key(), public.clone()).unwrap();
        (signer, public)
    }

    fn ciphersuite() -> MlsCiphersuite {
        MlsCiphersuite::Mls128DhkemX25519Aes128gcmSha256Ed25519
    }

    fn identity(value: u8) -> Vec<u8> {
        vec![value; PROFILE_CREDENTIAL_IDENTITY_BYTES]
    }

    fn expected_roster(value: &MlsRosterSummaryV1) -> MlsExpectedRosterStateV1 {
        MlsExpectedRosterStateV1 {
            group_id: value.group_id.clone(),
            epoch: value.epoch,
            digest_sha256: value.digest_sha256.clone(),
        }
    }

    fn native_expected(value: &MlsRosterSummaryV1) -> NativeExpectedRosterStateV1 {
        NativeExpectedRosterStateV1 {
            group_id: value.group_id.clone(),
            epoch: value.epoch,
            digest_sha256: value.digest_sha256.clone(),
        }
    }

    fn native_leaf(value: &MlsRosterLeafV1) -> NativeLeafAuthorityV1 {
        NativeLeafAuthorityV1 {
            leaf_index: value.leaf_index,
            credential_identity: value.credential_identity.clone(),
            signature_public_key: value.signature_public_key.clone(),
        }
    }

    fn native_snapshot(entries: &[MlsStorageEntry]) -> NativeStorageSnapshotV1 {
        NativeStorageSnapshotV1 {
            storage_format_version: MLS_STORAGE_FORMAT_VERSION,
            entries: entries
                .iter()
                .map(|entry| NativeStorageEntryV1 {
                    key: entry.key.clone(),
                    value: entry.value.clone(),
                    group_id: entry.group_id.clone(),
                })
                .collect(),
        }
    }

    fn apply_native_batch(entries: &mut Vec<MlsStorageEntry>, batch: &NativeStorageBatchV1) {
        let mls_batch = MlsStorageBatch {
            storage_format_version: batch.storage_format_version,
            upserts: batch
                .upserts
                .iter()
                .map(|entry| MlsStorageEntry {
                    key: entry.key.clone(),
                    value: entry.value.clone(),
                    group_id: entry.group_id.clone(),
                })
                .collect(),
            deletes: batch.deletes.clone(),
            deleted_group_ids: batch.deleted_group_ids.clone(),
        };
        apply_storage_batch(entries, &mls_batch);
    }

    #[test]
    fn rejects_nonminimal_integer_before_decode() {
        let payload = vec![0xa1, 0x18, 0x00, 0x01];
        let error = decode_native_receive_request_v1(&frame(
            NativeReceiveOperationV1::Application,
            payload,
        ))
        .unwrap_err();
        assert_eq!(error, NativeReceiveErrorCodeV1::NoncanonicalEncoding);
    }

    #[test]
    fn failure_has_no_success_fields() {
        let encoded = encode_native_receive_outcome_v1(
            Some(NativeReceiveOperationV1::Welcome),
            &NativeReceiveOutcomeV1::failure(NativeReceiveErrorCodeV1::ExpectedKeyPackageMismatch),
        )
        .unwrap();
        assert_eq!(&encoded[..4], FRAME_MAGIC);
        assert_eq!(encoded[6], NativeReceiveOperationV1::Welcome as u8);
        assert!(encoded.len() < 64);
        let mut decoder = Decoder::new(&encoded[FRAME_HEADER_BYTES..]);
        assert_eq!(decoder.map().unwrap(), Some(3));
        assert_eq!(decoder.u8().unwrap(), 0);
        assert_eq!(decoder.u16().unwrap(), NATIVE_RECEIVE_CONTRACT_VERSION);
        assert_eq!(decoder.u8().unwrap(), 1);
        assert!(!decoder.bool().unwrap());
        assert_eq!(decoder.u8().unwrap(), 3);
        assert_eq!(decoder.map().unwrap(), Some(1));
        assert_eq!(decoder.u8().unwrap(), 3);
        assert_eq!(
            decoder.u16().unwrap(),
            NativeReceiveErrorCodeV1::ExpectedKeyPackageMismatch as u16
        );
        assert_eq!(decoder.position(), encoded.len() - FRAME_HEADER_BYTES);
    }
}
