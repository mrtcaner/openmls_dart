//! Versioned native receive contract shared by Android and Apple wrappers.
//!
//! The wire format is deliberately independent from Flutter Rust Bridge. A
//! small fixed header wraps a strict deterministic-CBOR subset. Every request
//! passes a non-allocating structural/canonical preflight before owned values
//! are decoded.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_snapshot_budget_is_checked_before_value_allocation() {
        let value = Zeroizing::new(vec![7; NATIVE_RECEIVE_STORAGE_VALUE_MAX_BYTES]);
        let mut encoded = Zeroizing::new(Vec::new());
        let mut e = Encoder::new(&mut *encoded);
        e.map(2)
            .unwrap()
            .u8(0)
            .unwrap()
            .u32(1)
            .unwrap()
            .u8(1)
            .unwrap()
            .array(3)
            .unwrap();
        for key in [b"a", b"b", b"c"] {
            e.map(3)
                .unwrap()
                .u8(0)
                .unwrap()
                .bytes(key)
                .unwrap()
                .u8(1)
                .unwrap()
                .bytes(&value)
                .unwrap()
                .u8(2)
                .unwrap()
                .null()
                .unwrap();
        }
        let mut d = Decoder::new(&encoded);
        assert_eq!(
            decode_storage_snapshot(&mut d, None),
            Err(MlsErrorCode::LimitExceeded)
        );
        // Only the snapshot envelope/count was consumed. The borrowed scan
        // rejected key+value aggregate overflow before constructing entries.
        assert_eq!(d.position(), 5);
    }

    #[test]
    fn v2_preflight_rejects_noncanonical_and_unbounded_containers() {
        for payload in [
            vec![0xbf, 0xff],                   // indefinite map
            vec![0xa1, 0x18, 0x00, 0x00],       // non-minimal map key
            vec![0x9a, 0xff, 0xff, 0xff, 0xff], // unbounded array
            vec![0xc0, 0x00],                   // tag
        ] {
            let frame =
                encode_frame_with_max(1, payload, NATIVE_RECEIVE_REQUEST_MAX_BYTES).unwrap();
            let result = execute_native_receive_v2(&frame);
            let mut d = Decoder::new(&result[FRAME_HEADER_BYTES..]);
            assert_eq!(d.map().unwrap(), Some(3));
            assert_eq!((d.u8().unwrap(), d.u16().unwrap()), (0, 2));
            assert_eq!(d.u8().unwrap(), 1);
            assert!(!d.bool().unwrap());
            assert_eq!(d.u8().unwrap(), 3);
            assert_eq!(d.map().unwrap(), Some(1));
            assert_eq!(d.u8().unwrap(), 3);
            assert!(matches!(d.u16().unwrap(), 5 | 6));
            assert_eq!(d.position(), result.len() - FRAME_HEADER_BYTES);
        }
    }
}

use std::panic::{AssertUnwindSafe, catch_unwind};

use minicbor::data::Type;
use minicbor::{Decoder, Encoder};
use zeroize::{Zeroize, Zeroizing};

use crate::api::group_e2ee::{
    MlsErrorCode, MlsExpectedRosterStateV1, MlsReceiveKind, MlsRosterLeafV1, MlsRosterSummaryV1,
    runtime,
};
use crate::api::storage::{MlsStorageBatch, MlsStorageEntry};

pub const NATIVE_RECEIVE_CONTRACT_VERSION: u16 = 2;
pub const NATIVE_RECEIVE_PROFILE_V2: u16 = 2;
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
pub enum NativeReceiveOperationV2 {
    Application = 1,
    Commit = 2,
    Welcome = 3,
}

impl NativeReceiveOperationV2 {
    fn from_u8(value: u8) -> Result<Self, NativeReceiveErrorCodeV2> {
        match value {
            1 => Ok(Self::Application),
            2 => Ok(Self::Commit),
            3 => Ok(Self::Welcome),
            _ => Err(NativeReceiveErrorCodeV2::UnsupportedOperation),
        }
    }
}

/// Shared semantic codes: Dart and native failures cannot diverge.
pub type NativeReceiveErrorCodeV2 = MlsErrorCode;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeLeafAuthorityV2 {
    pub leaf_index: u32,
    pub credential_identity: Vec<u8>,
    pub signature_public_key: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeExpectedRosterStateV2 {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub digest_sha256: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRosterSummaryV2 {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub leaves: Vec<NativeLeafAuthorityV2>,
    pub digest_sha256: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeStorageEntryV2 {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub group_id: Option<Vec<u8>>,
}

impl Drop for NativeStorageSnapshotV2 {
    fn drop(&mut self) {
        zeroize_entries(&mut self.entries);
    }
}
impl Drop for NativeStorageEntryV2 {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeStorageSnapshotV2 {
    pub storage_format_version: u32,
    pub entries: Vec<NativeStorageEntryV2>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeStorageBatchV2 {
    pub storage_format_version: u32,
    pub upserts: Vec<NativeStorageEntryV2>,
    pub deletes: Vec<Vec<u8>>,
    pub deleted_group_ids: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub enum NativeReceiveRequestV2 {
    Process {
        operation: NativeReceiveOperationV2,
        profile_id: u16,
        group_id: Vec<u8>,
        message_bytes: Vec<u8>,
        expected_aad: Vec<u8>,
        expected_sender: NativeLeafAuthorityV2,
        expected_previous_state: NativeExpectedRosterStateV2,
        expected_resulting_state: NativeExpectedRosterStateV2,
        expected_base_group_state_sha256: Vec<u8>,
        storage: NativeStorageSnapshotV2,
        expected_message_state: NativeExpectedRosterStateV2,
        expected_retention: u32,
        expected_message_sha256: Vec<u8>,
        incarnation_id: Vec<u8>,
    },
    Welcome {
        profile_id: u16,
        welcome_bytes: Vec<u8>,
        ratchet_tree_bytes: Option<Vec<u8>>,
        signer_bytes: Vec<u8>,
        expected_local_leaf: NativeLeafAuthorityV2,
        expected_resulting_state: NativeExpectedRosterStateV2,
        expected_target_key_package_sha256: Vec<u8>,
        storage: NativeStorageSnapshotV2,
        retention: u32,
        incarnation_id: Vec<u8>,
        expected_welcome_sha256: Vec<u8>,
    },
}

impl Drop for NativeReceiveRequestV2 {
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
pub enum NativeReceiveSuccessV2 {
    Application {
        sender: NativeLeafAuthorityV2,
        previous_roster: NativeRosterSummaryV2,
        resulting_roster: NativeRosterSummaryV2,
        resulting_group_state_sha256: Vec<u8>,
        message_epoch: u64,
        plaintext: Vec<u8>,
        storage_batch: NativeStorageBatchV2,
        effective_retention: u32,
    },
    Commit {
        message_epoch: u64,
        sender: NativeLeafAuthorityV2,
        previous_roster: NativeRosterSummaryV2,
        resulting_roster: NativeRosterSummaryV2,
        resulting_group_state_sha256: Vec<u8>,
        storage_batch: NativeStorageBatchV2,
        effective_retention: u32,
    },
    Welcome {
        local_leaf: NativeLeafAuthorityV2,
        resulting_roster: NativeRosterSummaryV2,
        resulting_group_state_sha256: Vec<u8>,
        consumed_key_package_sha256: Vec<u8>,
        storage_batch: NativeStorageBatchV2,
        effective_retention: u32,
    },
}

impl NativeReceiveSuccessV2 {
    pub fn operation(&self) -> NativeReceiveOperationV2 {
        match self {
            Self::Application { .. } => NativeReceiveOperationV2::Application,
            Self::Commit { .. } => NativeReceiveOperationV2::Commit,
            Self::Welcome { .. } => NativeReceiveOperationV2::Welcome,
        }
    }
}

impl Drop for NativeReceiveSuccessV2 {
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
pub struct NativeReceiveOutcomeV2 {
    pub state_applied: bool,
    pub result: Option<NativeReceiveSuccessV2>,
    pub error: Option<NativeReceiveErrorCodeV2>,
}

impl NativeReceiveOutcomeV2 {
    pub fn success(result: NativeReceiveSuccessV2) -> Self {
        Self {
            state_applied: false,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(error: NativeReceiveErrorCodeV2) -> Self {
        Self {
            state_applied: false,
            result: None,
            error: Some(error),
        }
    }
}

pub fn decode_native_receive_request_v2(
    frame: &[u8],
) -> Result<NativeReceiveRequestV2, NativeReceiveErrorCodeV2> {
    let header = decode_header(frame, NATIVE_RECEIVE_REQUEST_MAX_BYTES)?;
    let operation = NativeReceiveOperationV2::from_u8(header.operation)?;
    preflight_cbor(header.payload)?;
    let mut decoder = Decoder::new(header.payload);
    let request = match operation {
        NativeReceiveOperationV2::Application | NativeReceiveOperationV2::Commit => {
            decode_process_request(&mut decoder, operation)?
        }
        NativeReceiveOperationV2::Welcome => decode_welcome_request(&mut decoder)?,
    };
    if decoder.position() != header.payload.len() {
        return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding);
    }
    Ok(request)
}

/// Encode one canonical request frame. This helper is used to generate shared
/// platform fixtures; native consumers may implement the same frozen codec.
pub fn encode_native_receive_request_v2(
    request: &NativeReceiveRequestV2,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    encode_native_receive_request_v2_with_aad_minimum(request, 1)
}

#[cfg(any(test, feature = "native-receive-fixtures"))]
pub(crate) fn encode_native_receive_request_v2_allow_empty_aad_fixture(
    request: &NativeReceiveRequestV2,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    encode_native_receive_request_v2_with_aad_minimum(request, 0)
}

fn encode_native_receive_request_v2_with_aad_minimum(
    request: &NativeReceiveRequestV2,
    minimum_aad_bytes: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    let mut payload = Zeroizing::new(Vec::new());
    let operation = match request {
        NativeReceiveRequestV2::Process {
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
            expected_message_state,
            expected_retention,
            expected_message_sha256,
            incarnation_id,
        } => {
            if *operation == NativeReceiveOperationV2::Welcome {
                return Err(NativeReceiveErrorCodeV2::UnsupportedOperation);
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
            let mut encoder = Encoder::new(&mut *payload);
            encoder
                .map(13)
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
            encoder.u8(9).map_err(encode_error)?;
            encode_expected_roster(&mut encoder, expected_message_state)?;
            validate_retention(*expected_retention)?;
            validate_exact_result_bytes(expected_message_sha256, SHA256_BYTES)?;
            validate_result_range(incarnation_id, 1, 128)?;
            encoder
                .u8(10)
                .and_then(|v| v.u32(*expected_retention))
                .and_then(|v| v.u8(11))
                .and_then(|v| v.bytes(expected_message_sha256))
                .and_then(|v| v.u8(12))
                .and_then(|v| v.bytes(incarnation_id))
                .map_err(encode_error)?;
            *operation
        }
        NativeReceiveRequestV2::Welcome {
            profile_id,
            welcome_bytes,
            ratchet_tree_bytes,
            signer_bytes,
            expected_local_leaf,
            expected_resulting_state,
            expected_target_key_package_sha256,
            storage,
            retention,
            incarnation_id,
            expected_welcome_sha256,
        } => {
            validate_profile(*profile_id)?;
            validate_result_range(welcome_bytes, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
            if let Some(tree) = ratchet_tree_bytes {
                validate_result_range(tree, 1, NATIVE_RECEIVE_RATCHET_TREE_MAX_BYTES)?;
            }
            validate_result_range(signer_bytes, 1, NATIVE_RECEIVE_SIGNER_MAX_BYTES)?;
            validate_exact_result_bytes(expected_target_key_package_sha256, SHA256_BYTES)?;
            let mut encoder = Encoder::new(&mut *payload);
            encoder
                .map(11)
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
            validate_retention(*retention)?;
            validate_result_range(incarnation_id, 1, 128)?;
            validate_exact_result_bytes(expected_welcome_sha256, SHA256_BYTES)?;
            encoder
                .u8(8)
                .and_then(|v| v.u32(*retention))
                .and_then(|v| v.u8(9))
                .and_then(|v| v.bytes(incarnation_id))
                .and_then(|v| v.u8(10))
                .and_then(|v| v.bytes(expected_welcome_sha256))
                .map_err(encode_error)?;
            NativeReceiveOperationV2::Welcome
        }
    };
    encode_frame_with_max(
        operation as u8,
        std::mem::take(&mut *payload),
        NATIVE_RECEIVE_REQUEST_MAX_BYTES,
    )
}

/// Execute one receive operation without retaining group state or touching a
/// caller database. The returned frame always has `stateApplied=false`.
///
/// Panics are contained before a native wrapper boundary. Callers must still
/// serialize calls that can mutate the same installation/group snapshot.
pub fn execute_native_receive_v2(frame: &[u8]) -> Vec<u8> {
    let operation = operation_hint(frame);
    let outcome = match catch_unwind(AssertUnwindSafe(|| {
        let request = decode_native_receive_request_v2(frame)?;
        execute_request(request)
    })) {
        Ok(Ok(result)) => NativeReceiveOutcomeV2::success(result),
        Ok(Err(error)) => NativeReceiveOutcomeV2::failure(error),
        Err(_) => NativeReceiveOutcomeV2::failure(NativeReceiveErrorCodeV2::InternalFailure),
    };
    match encode_native_receive_outcome_v2(operation, &outcome) {
        Ok(frame) => frame,
        Err(error) => {
            let failure = NativeReceiveOutcomeV2::failure(error);
            encode_native_receive_outcome_v2(operation, &failure)
                .unwrap_or_else(|_| encode_failure_fallback(operation))
        }
    }
}

pub(crate) fn encode_native_receive_failure_v2(
    operation: Option<NativeReceiveOperationV2>,
    error: NativeReceiveErrorCodeV2,
) -> Vec<u8> {
    let outcome = NativeReceiveOutcomeV2::failure(error);
    encode_native_receive_outcome_v2(operation, &outcome)
        .unwrap_or_else(|_| encode_failure_fallback(operation))
}

fn execute_request(
    mut request: NativeReceiveRequestV2,
) -> Result<NativeReceiveSuccessV2, NativeReceiveErrorCodeV2> {
    match &mut request {
        NativeReceiveRequestV2::Process {
            operation,
            group_id,
            message_bytes,
            expected_aad,
            expected_sender,
            expected_previous_state,
            expected_resulting_state,
            expected_base_group_state_sha256,
            storage,
            expected_message_state,
            expected_retention,
            expected_message_sha256,
            incarnation_id,
            ..
        } => {
            let kind = match operation {
                NativeReceiveOperationV2::Application => MlsReceiveKind::Application,
                NativeReceiveOperationV2::Commit => MlsReceiveKind::Commit,
                NativeReceiveOperationV2::Welcome => {
                    return Err(MlsErrorCode::UnsupportedOperation);
                }
            };
            let result = runtime::receive(runtime::ReceiveInput {
                context: runtime::Context {
                    group_id: std::mem::take(group_id),
                    incarnation_id: std::mem::take(incarnation_id),
                    expected_current: native_expected_into_mls(std::mem::take(
                        expected_previous_state,
                    )),
                    expected_base_sha256: std::mem::take(expected_base_group_state_sha256),
                    expected_retention: *expected_retention,
                    entries: std::mem::take(&mut storage.entries)
                        .into_iter()
                        .map(native_storage_entry_into_mls)
                        .collect(),
                    storage_format: storage.storage_format_version,
                },
                kind,
                wire: std::mem::take(message_bytes),
                wire_sha256: std::mem::take(expected_message_sha256),
                aad: std::mem::take(expected_aad),
                sender: native_leaf_into_mls(std::mem::take(expected_sender)),
                message_state: native_expected_into_mls(std::mem::take(expected_message_state)),
                resulting_state: native_expected_into_mls(std::mem::take(expected_resulting_state)),
            })?;
            let sender = mls_leaf_into_native(result.sender);
            let previous_roster = mls_roster_into_native(result.mutation.previous);
            let resulting_roster = mls_roster_into_native(result.mutation.resulting);
            let resulting_group_state_sha256 = result.mutation.resulting_digest;
            let effective_retention = result.mutation.retention;
            let storage_batch = mls_batch_into_native(result.mutation.batch);
            match kind {
                MlsReceiveKind::Application => Ok(NativeReceiveSuccessV2::Application {
                    sender,
                    previous_roster,
                    resulting_roster,
                    resulting_group_state_sha256,
                    message_epoch: result.message_epoch,
                    effective_retention,
                    storage_batch,
                    plaintext: result.plaintext.unwrap_or_default(),
                }),
                MlsReceiveKind::Commit => Ok(NativeReceiveSuccessV2::Commit {
                    sender,
                    previous_roster,
                    resulting_roster,
                    resulting_group_state_sha256,
                    message_epoch: result.message_epoch,
                    effective_retention,
                    storage_batch,
                }),
                MlsReceiveKind::Proposal => unreachable!(),
            }
        }
        NativeReceiveRequestV2::Welcome {
            welcome_bytes,
            ratchet_tree_bytes,
            signer_bytes,
            expected_local_leaf,
            expected_resulting_state,
            expected_target_key_package_sha256,
            storage,
            retention,
            incarnation_id,
            expected_welcome_sha256,
            ..
        } => {
            let result = runtime::join(runtime::JoinInput {
                config: runtime::profile(*retention)?,
                incarnation_id: std::mem::take(incarnation_id),
                signer: std::mem::take(signer_bytes),
                welcome: std::mem::take(welcome_bytes),
                welcome_sha256: std::mem::take(expected_welcome_sha256),
                tree: std::mem::take(ratchet_tree_bytes),
                resulting: native_expected_into_mls(std::mem::take(expected_resulting_state)),
                local_leaf: native_leaf_into_mls(std::mem::take(expected_local_leaf)),
                target_key_package_sha256: std::mem::take(expected_target_key_package_sha256),
                entries: std::mem::take(&mut storage.entries)
                    .into_iter()
                    .map(native_storage_entry_into_mls)
                    .collect(),
                storage_format: storage.storage_format_version,
            })?;
            Ok(NativeReceiveSuccessV2::Welcome {
                local_leaf: mls_leaf_into_native(result.local_leaf),
                resulting_roster: mls_roster_into_native(result.mutation.resulting),
                resulting_group_state_sha256: result.mutation.resulting_digest,
                consumed_key_package_sha256: result.consumed_key_package_sha256,
                effective_retention: result.mutation.retention,
                storage_batch: mls_batch_into_native(result.mutation.batch),
            })
        }
    }
}

fn native_leaf_into_mls(value: NativeLeafAuthorityV2) -> MlsRosterLeafV1 {
    MlsRosterLeafV1 {
        leaf_index: value.leaf_index,
        credential_identity: value.credential_identity,
        signature_public_key: value.signature_public_key,
    }
}
fn validate_retention(value: u32) -> Result<(), NativeReceiveErrorCodeV2> {
    crate::local_group_state::validate_retention(value)
}

fn native_expected_into_mls(value: NativeExpectedRosterStateV2) -> MlsExpectedRosterStateV1 {
    MlsExpectedRosterStateV1 {
        group_id: value.group_id,
        epoch: value.epoch,
        digest_sha256: value.digest_sha256,
    }
}

fn native_storage_entry_into_mls(mut value: NativeStorageEntryV2) -> MlsStorageEntry {
    MlsStorageEntry {
        key: std::mem::take(&mut value.key),
        value: std::mem::take(&mut value.value),
        group_id: std::mem::take(&mut value.group_id),
    }
}

fn mls_leaf_into_native(value: MlsRosterLeafV1) -> NativeLeafAuthorityV2 {
    NativeLeafAuthorityV2 {
        leaf_index: value.leaf_index,
        credential_identity: value.credential_identity,
        signature_public_key: value.signature_public_key,
    }
}

fn mls_roster_into_native(value: MlsRosterSummaryV1) -> NativeRosterSummaryV2 {
    NativeRosterSummaryV2 {
        group_id: value.group_id,
        epoch: value.epoch,
        leaves: value.leaves.into_iter().map(mls_leaf_into_native).collect(),
        digest_sha256: value.digest_sha256,
    }
}

fn mls_batch_into_native(value: MlsStorageBatch) -> NativeStorageBatchV2 {
    NativeStorageBatchV2 {
        storage_format_version: value.storage_format_version,
        upserts: value
            .upserts
            .into_iter()
            .map(|entry| NativeStorageEntryV2 {
                key: entry.key,
                value: entry.value,
                group_id: entry.group_id,
            })
            .collect(),
        deletes: value.deletes,
        deleted_group_ids: value.deleted_group_ids,
    }
}

fn operation_hint(frame: &[u8]) -> Option<NativeReceiveOperationV2> {
    frame
        .get(6)
        .and_then(|value| NativeReceiveOperationV2::from_u8(*value).ok())
}

fn encode_failure_fallback(operation: Option<NativeReceiveOperationV2>) -> Vec<u8> {
    let payload = [0xa3, 0x00, 0x02, 0x01, 0xf4, 0x03, 0xa1, 0x03, 0x18, 0xff];
    let mut frame = Vec::with_capacity(FRAME_HEADER_BYTES + payload.len());
    frame.extend_from_slice(FRAME_MAGIC);
    frame.extend_from_slice(&NATIVE_RECEIVE_CONTRACT_VERSION.to_be_bytes());
    frame.push(operation.map_or(0, |value| value as u8));
    frame.push(0);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    frame
}

pub fn encode_native_receive_outcome_v2(
    operation: Option<NativeReceiveOperationV2>,
    outcome: &NativeReceiveOutcomeV2,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    if outcome.state_applied || outcome.result.is_some() == outcome.error.is_some() {
        return Err(NativeReceiveErrorCodeV2::InternalFailure);
    }
    let actual_operation = outcome
        .result
        .as_ref()
        .map(NativeReceiveSuccessV2::operation)
        .or(operation);
    let mut payload = Zeroizing::new(Vec::new());
    let mut encoder = Encoder::new(&mut *payload);
    if let Some(result) = &outcome.result {
        encoder
            .map(3)
            .and_then(|encoder| encoder.u8(0))
            .and_then(|encoder| encoder.u16(NATIVE_RECEIVE_CONTRACT_VERSION))
            .and_then(|encoder| encoder.u8(1))
            .and_then(|encoder| encoder.bool(false))
            .and_then(|encoder| encoder.u8(2))
            .map_err(|_| NativeReceiveErrorCodeV2::InternalFailure)?;
        encode_success(&mut encoder, result)?;
    } else {
        let error = outcome
            .error
            .ok_or(NativeReceiveErrorCodeV2::InternalFailure)?;
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
            .map_err(|_| NativeReceiveErrorCodeV2::InternalFailure)?;
    }
    encode_frame(
        actual_operation.map_or(0, |value| value as u8),
        std::mem::take(&mut *payload),
    )
}

struct DecodedHeader<'a> {
    operation: u8,
    payload: &'a [u8],
}

fn decode_header(
    frame: &[u8],
    maximum: usize,
) -> Result<DecodedHeader<'_>, NativeReceiveErrorCodeV2> {
    if frame.len() > maximum {
        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
    }
    if frame.len() < FRAME_HEADER_BYTES || &frame[..4] != FRAME_MAGIC {
        return Err(NativeReceiveErrorCodeV2::InvalidFrame);
    }
    let version = u16::from_be_bytes([frame[4], frame[5]]);
    if version != NATIVE_RECEIVE_CONTRACT_VERSION {
        return Err(NativeReceiveErrorCodeV2::UnsupportedContractVersion);
    }
    if frame[7] != 0 {
        return Err(NativeReceiveErrorCodeV2::InvalidFrame);
    }
    let payload_len = u32::from_be_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
    if payload_len != frame.len() - FRAME_HEADER_BYTES {
        return Err(NativeReceiveErrorCodeV2::InvalidFrame);
    }
    Ok(DecodedHeader {
        operation: frame[6],
        payload: &frame[FRAME_HEADER_BYTES..],
    })
}

fn encode_frame(operation: u8, payload: Vec<u8>) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    encode_frame_with_max(operation, payload, NATIVE_RECEIVE_RESULT_MAX_BYTES)
}

fn encode_frame_with_max(
    operation: u8,
    mut payload: Vec<u8>,
    maximum: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    let frame_len = FRAME_HEADER_BYTES
        .checked_add(payload.len())
        .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
    if frame_len > maximum {
        payload.zeroize();
        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
    }
    let payload_len =
        u32::try_from(payload.len()).map_err(|_| NativeReceiveErrorCodeV2::LimitExceeded)?;
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
    operation: NativeReceiveOperationV2,
) -> Result<NativeReceiveRequestV2, NativeReceiveErrorCodeV2> {
    expect_map(decoder, 13)?;
    expect_key(decoder, 0)?;
    let profile_id = decoder
        .u16()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
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
    expect_key(decoder, 9)?;
    let expected_message_state = decode_expected_roster(decoder)?;
    expect_key(decoder, 10)?;
    let expected_retention = decoder
        .u32()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    validate_retention(expected_retention)?;
    expect_key(decoder, 11)?;
    let expected_message_sha256 = decode_exact_bytes(decoder, SHA256_BYTES)?;
    expect_key(decoder, 12)?;
    let incarnation_id = decode_bounded_bytes(decoder, 1, 128)?;
    Ok(NativeReceiveRequestV2::Process {
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
        expected_message_state,
        expected_retention,
        expected_message_sha256,
        incarnation_id,
    })
}

fn decode_welcome_request(
    decoder: &mut Decoder<'_>,
) -> Result<NativeReceiveRequestV2, NativeReceiveErrorCodeV2> {
    expect_map(decoder, 11)?;
    expect_key(decoder, 0)?;
    let profile_id = decoder
        .u16()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    validate_profile(profile_id)?;
    expect_key(decoder, 1)?;
    let welcome_bytes = decode_bounded_bytes(decoder, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
    expect_key(decoder, 2)?;
    let ratchet_tree_bytes = match decoder
        .datatype()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?
    {
        Type::Null => {
            decoder
                .null()
                .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
            None
        }
        Type::Bytes => Some(decode_bounded_bytes(
            decoder,
            1,
            NATIVE_RECEIVE_RATCHET_TREE_MAX_BYTES,
        )?),
        _ => return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding),
    };
    expect_key(decoder, 3)?;
    let mut signer_bytes = Zeroizing::new(decode_bounded_bytes(
        decoder,
        1,
        NATIVE_RECEIVE_SIGNER_MAX_BYTES,
    )?);
    expect_key(decoder, 4)?;
    let expected_local_leaf = decode_leaf(decoder)?;
    expect_key(decoder, 5)?;
    let expected_resulting_state = decode_expected_roster(decoder)?;
    expect_key(decoder, 6)?;
    let expected_target_key_package_sha256 = decode_exact_bytes(decoder, SHA256_BYTES)?;
    expect_key(decoder, 7)?;
    let storage = decode_storage_snapshot(decoder, None)?;
    expect_key(decoder, 8)?;
    let retention = decoder
        .u32()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    validate_retention(retention)?;
    expect_key(decoder, 9)?;
    let incarnation_id = decode_bounded_bytes(decoder, 1, 128)?;
    expect_key(decoder, 10)?;
    let expected_welcome_sha256 = decode_exact_bytes(decoder, SHA256_BYTES)?;
    Ok(NativeReceiveRequestV2::Welcome {
        profile_id,
        welcome_bytes,
        ratchet_tree_bytes,
        signer_bytes: std::mem::take(&mut *signer_bytes),
        expected_local_leaf,
        expected_resulting_state,
        expected_target_key_package_sha256,
        storage,
        retention,
        incarnation_id,
        expected_welcome_sha256,
    })
}

fn validate_profile(profile_id: u16) -> Result<(), NativeReceiveErrorCodeV2> {
    if profile_id == NATIVE_RECEIVE_PROFILE_V2 {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV2::UnsupportedProfile)
    }
}

fn decode_leaf(
    decoder: &mut Decoder<'_>,
) -> Result<NativeLeafAuthorityV2, NativeReceiveErrorCodeV2> {
    expect_map(decoder, 3)?;
    expect_key(decoder, 0)?;
    let leaf_index = decoder
        .u32()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    expect_key(decoder, 1)?;
    let credential_identity = decode_exact_bytes(decoder, PROFILE_CREDENTIAL_IDENTITY_BYTES)?;
    expect_key(decoder, 2)?;
    let signature_public_key = decode_exact_bytes(decoder, PROFILE_SIGNATURE_PUBLIC_KEY_BYTES)?;
    Ok(NativeLeafAuthorityV2 {
        leaf_index,
        credential_identity,
        signature_public_key,
    })
}

fn decode_expected_roster(
    decoder: &mut Decoder<'_>,
) -> Result<NativeExpectedRosterStateV2, NativeReceiveErrorCodeV2> {
    expect_map(decoder, 3)?;
    expect_key(decoder, 0)?;
    let group_id = decode_exact_bytes(decoder, PROFILE_GROUP_ID_BYTES)?;
    expect_key(decoder, 1)?;
    let epoch = decoder
        .u64()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    expect_key(decoder, 2)?;
    let digest_sha256 = decode_exact_bytes(decoder, SHA256_BYTES)?;
    Ok(NativeExpectedRosterStateV2 {
        group_id,
        epoch,
        digest_sha256,
    })
}

fn decode_storage_snapshot(
    decoder: &mut Decoder<'_>,
    expected_group_id: Option<&[u8]>,
) -> Result<NativeStorageSnapshotV2, NativeReceiveErrorCodeV2> {
    expect_map(decoder, 2)?;
    expect_key(decoder, 0)?;
    let storage_format_version = decoder
        .u32()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    if storage_format_version != 1 {
        return Err(NativeReceiveErrorCodeV2::StorageFormatMismatch);
    }
    expect_key(decoder, 1)?;
    let count = expect_array_max(decoder, NATIVE_RECEIVE_STORAGE_MAX_ENTRIES)?;
    // Check aggregate scope/bytes with borrowed slices before allocating any
    // snapshot values. A rejected final row must not overshoot the 6 MiB cap.
    let mut scan = decoder.clone();
    let mut total = 0usize;
    for _ in 0..count {
        expect_map(&mut scan, 3)?;
        expect_key(&mut scan, 0)?;
        let key = scan
            .bytes()
            .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
        validate_result_range(key, 1, NATIVE_RECEIVE_STORAGE_KEY_MAX_BYTES)?;
        expect_key(&mut scan, 1)?;
        let value = scan
            .bytes()
            .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
        validate_result_range(value, 0, NATIVE_RECEIVE_STORAGE_VALUE_MAX_BYTES)?;
        expect_key(&mut scan, 2)?;
        let group_bytes = match scan
            .datatype()
            .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?
        {
            Type::Null => {
                scan.null()
                    .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
                0
            }
            Type::Bytes => {
                let group = scan
                    .bytes()
                    .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
                validate_exact_result_bytes(group, PROFILE_GROUP_ID_BYTES)?;
                if expected_group_id != Some(group) {
                    return Err(NativeReceiveErrorCodeV2::InvalidStorageSnapshot);
                }
                group.len()
            }
            _ => return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding),
        };
        total = total
            .checked_add(key.len())
            .and_then(|n| n.checked_add(value.len()))
            .and_then(|n| n.checked_add(group_bytes))
            .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
        if total > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
            return Err(NativeReceiveErrorCodeV2::LimitExceeded);
        }
    }
    let mut entries = Vec::with_capacity(count);
    let mut total = 0usize;
    for _ in 0..count {
        let entry = decode_storage_entry(decoder)?;
        total = total
            .checked_add(entry.key.len())
            .and_then(|value| value.checked_add(entry.value.len()))
            .and_then(|value| value.checked_add(entry.group_id.as_ref().map_or(0, Vec::len)))
            .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
        if total > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
            zeroize_entries(&mut entries);
            return Err(NativeReceiveErrorCodeV2::LimitExceeded);
        }
        if let Some(group_id) = &entry.group_id {
            match expected_group_id {
                Some(expected) if group_id == expected => {}
                _ => {
                    zeroize_entries(&mut entries);
                    return Err(NativeReceiveErrorCodeV2::InvalidStorageSnapshot);
                }
            }
        }
        entries.push(entry);
    }
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
        zeroize_entries(&mut entries);
        return Err(NativeReceiveErrorCodeV2::InvalidStorageSnapshot);
    }
    Ok(NativeStorageSnapshotV2 {
        storage_format_version,
        entries,
    })
}

fn decode_storage_entry(
    decoder: &mut Decoder<'_>,
) -> Result<NativeStorageEntryV2, NativeReceiveErrorCodeV2> {
    expect_map(decoder, 3)?;
    expect_key(decoder, 0)?;
    let key = decode_bounded_bytes(decoder, 1, NATIVE_RECEIVE_STORAGE_KEY_MAX_BYTES)?;
    expect_key(decoder, 1)?;
    let mut value = Zeroizing::new(decode_bounded_bytes(
        decoder,
        0,
        NATIVE_RECEIVE_STORAGE_VALUE_MAX_BYTES,
    )?);
    expect_key(decoder, 2)?;
    let group_id = match decoder
        .datatype()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?
    {
        Type::Null => {
            decoder
                .null()
                .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
            None
        }
        Type::Bytes => Some(decode_exact_bytes(decoder, PROFILE_GROUP_ID_BYTES)?),
        _ => return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding),
    };
    Ok(NativeStorageEntryV2 {
        key,
        value: std::mem::take(&mut *value),
        group_id,
    })
}

fn encode_success(
    encoder: &mut Encoder<&mut Vec<u8>>,
    result: &NativeReceiveSuccessV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
    match result {
        NativeReceiveSuccessV2::Application {
            sender,
            previous_roster,
            resulting_roster,
            resulting_group_state_sha256,
            plaintext,
            message_epoch,
            storage_batch,
            effective_retention,
        } => {
            validate_result_bytes(plaintext, NATIVE_RECEIVE_PLAINTEXT_MAX_BYTES)?;
            validate_exact_result_bytes(resulting_group_state_sha256, SHA256_BYTES)?;
            encoder.map(8).map_err(encode_error)?;
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
            validate_retention(*effective_retention)?;
            encoder
                .u8(6)
                .and_then(|v| v.u64(*message_epoch))
                .map_err(encode_error)?;
            encoder
                .u8(7)
                .and_then(|v| v.u32(*effective_retention))
                .map_err(encode_error)?;
        }
        NativeReceiveSuccessV2::Commit {
            message_epoch,
            sender,
            previous_roster,
            resulting_roster,
            resulting_group_state_sha256,
            storage_batch,
            effective_retention,
        } => {
            validate_exact_result_bytes(resulting_group_state_sha256, SHA256_BYTES)?;
            encoder.map(7).map_err(encode_error)?;
            encode_pair_leaf(encoder, 0, sender)?;
            encode_pair_roster(encoder, 1, previous_roster)?;
            encode_pair_roster(encoder, 2, resulting_roster)?;
            encoder
                .u8(3)
                .and_then(|value| value.bytes(resulting_group_state_sha256))
                .map_err(encode_error)?;
            encoder.u8(4).map_err(encode_error)?;
            encode_storage_batch(encoder, storage_batch)?;
            validate_retention(*effective_retention)?;
            encoder
                .u8(5)
                .and_then(|v| v.u64(*message_epoch))
                .map_err(encode_error)?;
            encoder
                .u8(6)
                .and_then(|v| v.u32(*effective_retention))
                .map_err(encode_error)?;
        }
        NativeReceiveSuccessV2::Welcome {
            local_leaf,
            resulting_roster,
            resulting_group_state_sha256,
            consumed_key_package_sha256,
            storage_batch,
            effective_retention,
        } => {
            validate_exact_result_bytes(resulting_group_state_sha256, SHA256_BYTES)?;
            validate_exact_result_bytes(consumed_key_package_sha256, SHA256_BYTES)?;
            encoder.map(6).map_err(encode_error)?;
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
            validate_retention(*effective_retention)?;
            encoder
                .u8(5)
                .and_then(|v| v.u32(*effective_retention))
                .map_err(encode_error)?;
        }
    }
    Ok(())
}

fn encode_pair_leaf(
    encoder: &mut Encoder<&mut Vec<u8>>,
    key: u8,
    leaf: &NativeLeafAuthorityV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
    encoder.u8(key).map_err(encode_error)?;
    encode_leaf(encoder, leaf)
}

fn encode_leaf(
    encoder: &mut Encoder<&mut Vec<u8>>,
    leaf: &NativeLeafAuthorityV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
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
    roster: &NativeRosterSummaryV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
    encoder.u8(key).map_err(encode_error)?;
    encode_roster(encoder, roster)
}

fn encode_expected_roster(
    encoder: &mut Encoder<&mut Vec<u8>>,
    roster: &NativeExpectedRosterStateV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
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
    snapshot: &NativeStorageSnapshotV2,
    expected_group_id: Option<&[u8]>,
) -> Result<(), NativeReceiveErrorCodeV2> {
    if snapshot.storage_format_version != 1
        || snapshot.entries.len() > NATIVE_RECEIVE_STORAGE_MAX_ENTRIES
    {
        return Err(NativeReceiveErrorCodeV2::StorageFormatMismatch);
    }
    let mut entries = snapshot.entries.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err(NativeReceiveErrorCodeV2::InvalidStorageSnapshot);
    }
    let mut total = 0usize;
    for entry in &entries {
        total = add_entry_size(total, entry)?;
        let is_global = crate::snapshot_storage::is_global_key(&entry.key);
        if (is_global && entry.group_id.is_some())
            || (!is_global && entry.group_id.as_deref() != expected_group_id)
        {
            return Err(NativeReceiveErrorCodeV2::InvalidStorageSnapshot);
        }
    }
    if total > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
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
    roster: &NativeRosterSummaryV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
    validate_exact_result_bytes(&roster.group_id, PROFILE_GROUP_ID_BYTES)?;
    validate_exact_result_bytes(&roster.digest_sha256, SHA256_BYTES)?;
    if roster.leaves.is_empty() || roster.leaves.len() > NATIVE_RECEIVE_ROSTER_MAX_LEAVES {
        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
    }
    if roster
        .leaves
        .windows(2)
        .any(|pair| pair[0].leaf_index >= pair[1].leaf_index)
    {
        return Err(NativeReceiveErrorCodeV2::InternalFailure);
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
    batch: &NativeStorageBatchV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
    if batch.storage_format_version != 1
        || batch.upserts.len() > NATIVE_RECEIVE_STORAGE_MAX_ENTRIES
        || batch.deletes.len() > NATIVE_RECEIVE_STORAGE_MAX_ENTRIES
        || batch.deleted_group_ids.len() > 8
    {
        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
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
            .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
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
            .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
        encoder.bytes(group_id).map_err(encode_error)?;
    }
    if total > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
    }
    Ok(())
}

fn add_entry_size(
    current: usize,
    entry: &NativeStorageEntryV2,
) -> Result<usize, NativeReceiveErrorCodeV2> {
    validate_result_range(&entry.key, 1, NATIVE_RECEIVE_STORAGE_KEY_MAX_BYTES)?;
    validate_result_range(&entry.value, 0, NATIVE_RECEIVE_STORAGE_VALUE_MAX_BYTES)?;
    if let Some(group_id) = &entry.group_id {
        validate_exact_result_bytes(group_id, PROFILE_GROUP_ID_BYTES)?;
    }
    current
        .checked_add(entry.key.len())
        .and_then(|value| value.checked_add(entry.value.len()))
        .and_then(|value| value.checked_add(entry.group_id.as_ref().map_or(0, Vec::len)))
        .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)
}

fn encode_storage_entry(
    encoder: &mut Encoder<&mut Vec<u8>>,
    entry: &NativeStorageEntryV2,
) -> Result<(), NativeReceiveErrorCodeV2> {
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

fn validate_result_bytes(value: &[u8], maximum: usize) -> Result<(), NativeReceiveErrorCodeV2> {
    validate_result_range(value, 0, maximum)
}

fn validate_exact_result_bytes(
    value: &[u8],
    expected: usize,
) -> Result<(), NativeReceiveErrorCodeV2> {
    if value.len() == expected {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV2::InternalFailure)
    }
}

fn validate_result_range(
    value: &[u8],
    minimum: usize,
    maximum: usize,
) -> Result<(), NativeReceiveErrorCodeV2> {
    if value.len() < minimum || value.len() > maximum {
        Err(NativeReceiveErrorCodeV2::LimitExceeded)
    } else {
        Ok(())
    }
}

fn encode_error<E>(_error: minicbor::encode::Error<E>) -> NativeReceiveErrorCodeV2 {
    NativeReceiveErrorCodeV2::InternalFailure
}

fn expect_map(decoder: &mut Decoder<'_>, expected: usize) -> Result<(), NativeReceiveErrorCodeV2> {
    let length = decoder
        .map()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?
        .ok_or(NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    if length == expected as u64 {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding)
    }
}

fn expect_array_max(
    decoder: &mut Decoder<'_>,
    maximum: usize,
) -> Result<usize, NativeReceiveErrorCodeV2> {
    let length = decoder
        .array()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?
        .ok_or(NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    let length = usize::try_from(length).map_err(|_| NativeReceiveErrorCodeV2::LimitExceeded)?;
    if length > maximum {
        Err(NativeReceiveErrorCodeV2::LimitExceeded)
    } else {
        Ok(length)
    }
}

fn expect_key(decoder: &mut Decoder<'_>, expected: u8) -> Result<(), NativeReceiveErrorCodeV2> {
    let actual = decoder
        .u8()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    if actual == expected {
        Ok(())
    } else {
        Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding)
    }
}

fn decode_exact_bytes(
    decoder: &mut Decoder<'_>,
    expected: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    decode_bounded_bytes(decoder, expected, expected)
}

fn decode_bounded_bytes(
    decoder: &mut Decoder<'_>,
    minimum: usize,
    maximum: usize,
) -> Result<Vec<u8>, NativeReceiveErrorCodeV2> {
    let value = decoder
        .bytes()
        .map_err(|_| NativeReceiveErrorCodeV2::NoncanonicalEncoding)?;
    if value.len() < minimum || value.len() > maximum {
        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
    }
    Ok(value.to_vec())
}

fn zeroize_entries(entries: &mut [NativeStorageEntryV2]) {
    for entry in entries {
        entry.value.zeroize();
    }
}

fn preflight_cbor(input: &[u8]) -> Result<(), NativeReceiveErrorCodeV2> {
    let mut scanner = CborPreflight {
        input,
        position: 0,
        items: 0,
    };
    scanner.value(0)?;
    if scanner.position != input.len() {
        return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding);
    }
    Ok(())
}

struct CborPreflight<'a> {
    input: &'a [u8],
    position: usize,
    items: usize,
}

impl CborPreflight<'_> {
    fn value(&mut self, depth: usize) -> Result<(), NativeReceiveErrorCodeV2> {
        if depth > PREFLIGHT_MAX_DEPTH {
            return Err(NativeReceiveErrorCodeV2::LimitExceeded);
        }
        self.items = self
            .items
            .checked_add(1)
            .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
        if self.items > PREFLIGHT_MAX_ITEMS {
            return Err(NativeReceiveErrorCodeV2::LimitExceeded);
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
                    usize::try_from(length).map_err(|_| NativeReceiveErrorCodeV2::LimitExceeded)?;
                if length > NATIVE_RECEIVE_STORAGE_MAX_BYTES {
                    return Err(NativeReceiveErrorCodeV2::LimitExceeded);
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
                        return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding);
                    }
                    let key = self.argument(key_initial & 0x1f)?;
                    if previous.is_some_and(|value| key <= value) {
                        return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding);
                    }
                    previous = Some(key);
                    self.items = self
                        .items
                        .checked_add(1)
                        .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
                    if self.items > PREFLIGHT_MAX_ITEMS {
                        return Err(NativeReceiveErrorCodeV2::LimitExceeded);
                    }
                    self.value(depth + 1)?;
                }
            }
            7 if matches!(additional, 20..=22) => {}
            _ => return Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding),
        }
        Ok(())
    }

    fn container_length(
        &mut self,
        additional: u8,
        maximum: usize,
    ) -> Result<usize, NativeReceiveErrorCodeV2> {
        let length = usize::try_from(self.argument(additional)?)
            .map_err(|_| NativeReceiveErrorCodeV2::LimitExceeded)?;
        if length > maximum {
            Err(NativeReceiveErrorCodeV2::LimitExceeded)
        } else {
            Ok(length)
        }
    }

    fn argument(&mut self, additional: u8) -> Result<u64, NativeReceiveErrorCodeV2> {
        match additional {
            value @ 0..=23 => Ok(u64::from(value)),
            24 => {
                let value = u64::from(self.byte()?);
                if value < 24 {
                    Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            25 => {
                let bytes = self.bytes::<2>()?;
                let value = u64::from(u16::from_be_bytes(bytes));
                if value <= u64::from(u8::MAX) {
                    Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            26 => {
                let bytes = self.bytes::<4>()?;
                let value = u64::from(u32::from_be_bytes(bytes));
                if value <= u64::from(u16::MAX) {
                    Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            27 => {
                let value = u64::from_be_bytes(self.bytes::<8>()?);
                if value <= u64::from(u32::MAX) {
                    Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding)
                } else {
                    Ok(value)
                }
            }
            _ => Err(NativeReceiveErrorCodeV2::NoncanonicalEncoding),
        }
    }

    fn byte(&mut self) -> Result<u8, NativeReceiveErrorCodeV2> {
        let value = *self
            .input
            .get(self.position)
            .ok_or(NativeReceiveErrorCodeV2::InvalidFrame)?;
        self.position += 1;
        Ok(value)
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], NativeReceiveErrorCodeV2> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
        let bytes = self
            .input
            .get(self.position..end)
            .ok_or(NativeReceiveErrorCodeV2::InvalidFrame)?;
        self.position = end;
        bytes
            .try_into()
            .map_err(|_| NativeReceiveErrorCodeV2::InvalidFrame)
    }

    fn advance(&mut self, length: usize) -> Result<(), NativeReceiveErrorCodeV2> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(NativeReceiveErrorCodeV2::LimitExceeded)?;
        if end > self.input.len() {
            return Err(NativeReceiveErrorCodeV2::InvalidFrame);
        }
        self.position = end;
        Ok(())
    }
}
