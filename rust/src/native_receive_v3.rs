//! Additive native frame3. Operations1..3 reuse the exact v2 semantic core;
//! operation4 is the explicit external Commit path. AAD/business authority is
//! authenticated by the caller, not by this framing or its hashes.
use crate::api::external_resync::MlsExternalCommitAcceptanceV1;
use crate::api::group_e2ee::{
    MlsErrorCode as Error, MlsPendingCommitBinding, MlsTransitionContext, runtime,
};
use crate::native_receive_v2::{self as v2, *};
use minicbor::{Decoder, Encoder, data::Type};
use sha2::{Digest, Sha256};
use std::panic::{AssertUnwindSafe, catch_unwind};
use zeroize::{Zeroize, Zeroizing};

pub const NATIVE_RECEIVE_CONTRACT_VERSION: u16 = 3;
pub const EXTERNAL_COMMIT_OPERATION: u8 = 4;
pub const INSPECT_PENDING_OPERATION: u8 = 5;
const HEADER: usize = 12;

#[derive(Debug)]
pub struct NativeInspectPendingRequestV3 {
    pub profile_id: u16,
    pub group_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub expected_current: crate::api::group_e2ee::MlsExpectedRosterStateV1,
    pub expected_base_group_state_sha256: Vec<u8>,
    pub expected_retention: u32,
    pub storage: NativeStorageSnapshotV2,
}

#[derive(Debug)]
pub struct NativeExternalReceiveRequestV3 {
    pub profile_id: u16,
    pub group_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub commit: Vec<u8>,
    pub expected_aad: Vec<u8>,
    pub expected_base_group_state_sha256: Vec<u8>,
    pub expected_retention: u32,
    pub acceptance: MlsExternalCommitAcceptanceV1,
    pub expected_superseded_pending: Option<MlsPendingCommitBinding>,
    pub storage: NativeStorageSnapshotV2,
}
impl Drop for NativeExternalReceiveRequestV3 {
    fn drop(&mut self) {
        self.commit.zeroize();
        self.expected_aad.zeroize();
    }
}
#[derive(Debug)]
// Operation-scoped DTOs have bounded stack size; keep ownership explicit and
// avoid extra heap allocations solely to equalize enum variant sizes.
#[allow(clippy::large_enum_variant)]
pub enum NativeReceiveRequestV3 {
    Member(NativeReceiveRequestV2),
    External(NativeExternalReceiveRequestV3),
    InspectPending(NativeInspectPendingRequestV3),
}
#[allow(clippy::large_enum_variant)]
enum Success {
    Member(NativeReceiveSuccessV2),
    Pending(Option<MlsPendingCommitBinding>),
    External {
        transition: NativeReceiveSuccessV2,
        superseded: Option<MlsPendingCommitBinding>,
    },
}

fn header(input: &[u8]) -> Result<(u8, &[u8]), Error> {
    if input.len() > NATIVE_RECEIVE_REQUEST_MAX_BYTES {
        return Err(Error::LimitExceeded);
    }
    if input.len() < HEADER || &input[..4] != b"KMLS" {
        return Err(Error::InvalidFrame);
    }
    if u16::from_be_bytes([input[4], input[5]]) != 3 {
        return Err(Error::UnsupportedContractVersion);
    }
    if input[7] != 0
        || u32::from_be_bytes(input[8..12].try_into().unwrap()) as usize != input.len() - HEADER
    {
        return Err(Error::InvalidFrame);
    }
    if !(1..=5).contains(&input[6]) {
        return Err(Error::UnsupportedOperation);
    }
    Ok((input[6], &input[HEADER..]))
}
fn frame(op: u8, mut payload: Vec<u8>, maximum: usize) -> Result<Vec<u8>, Error> {
    if payload.len() > maximum - HEADER {
        payload.zeroize();
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(b"KMLS");
    out.extend_from_slice(&3u16.to_be_bytes());
    out.extend_from_slice(&[op, 0]);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.append(&mut payload);
    Ok(out)
}
fn id(bytes: &[u8]) -> Result<(), Error> {
    validate_result_range(bytes, 1, 128)
}
fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
fn leaf(l: &crate::api::group_e2ee::MlsRosterLeafV1) -> Result<(), Error> {
    if l.leaf_index > 255 {
        return Err(Error::LimitExceeded);
    }
    validate_exact_result_bytes(&l.credential_identity, 45)?;
    validate_exact_result_bytes(&l.signature_public_key, 32)
}
fn state(s: &crate::api::group_e2ee::MlsExpectedRosterStateV1) -> Result<(), Error> {
    validate_exact_result_bytes(&s.group_id, 16)?;
    validate_exact_result_bytes(&s.digest_sha256, 32)
}
fn validate(r: &NativeExternalReceiveRequestV3) -> Result<(), Error> {
    validate_profile(r.profile_id)?;
    validate_retention(r.expected_retention)?;
    validate_exact_result_bytes(&r.group_id, 16)?;
    id(&r.incarnation_id)?;
    validate_exact_result_bytes(&r.expected_base_group_state_sha256, 32)?;
    validate_result_range(&r.commit, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
    validate_result_range(&r.expected_aad, 1, NATIVE_RECEIVE_AAD_MAX_BYTES)?;
    let b = &r.acceptance.binding;
    id(&b.candidate_id)?;
    id(&b.transition.command_id)?;
    validate_exact_result_bytes(&b.transition.context_sha256, 32)?;
    state(&b.previous_state)?;
    state(&r.acceptance.resulting_state)?;
    leaf(&b.old_leaf)?;
    leaf(&b.author)?;
    for h in [
        &b.input_group_info_sha256,
        &b.resulting_group_info_sha256,
        &b.commit_sha256,
        &b.aad_sha256,
    ] {
        validate_exact_result_bytes(h, 32)?;
    }
    if b.incarnation_id != r.incarnation_id
        || b.previous_state.group_id != r.group_id
        || r.acceptance.resulting_state.group_id != r.group_id
    {
        return Err(Error::GroupMismatch);
    }
    if b.previous_state.epoch.checked_add(1) != Some(r.acceptance.resulting_state.epoch) {
        return Err(Error::ResultingEpochMismatch);
    }
    if hash(&r.commit) != b.commit_sha256 {
        return Err(Error::WireHashMismatch);
    }
    if hash(&r.expected_aad) != b.aad_sha256 {
        return Err(Error::AadMismatch);
    }
    if let Some(p) = &r.expected_superseded_pending {
        validate_pending(p)?;
        if p.group_id != r.group_id
            || p.incarnation_id != r.incarnation_id
            || p.previous_state != b.previous_state
        {
            return Err(Error::PendingBindingMismatch);
        }
    }
    Ok(())
}
fn validate_pending(p: &MlsPendingCommitBinding) -> Result<(), Error> {
    validate_exact_result_bytes(&p.group_id, 16)?;
    id(&p.incarnation_id)?;
    id(&p.transition.command_id)?;
    validate_exact_result_bytes(&p.transition.context_sha256, 32)?;
    leaf(&p.author)?;
    state(&p.previous_state)?;
    for h in [&p.commit_sha256, &p.aad_sha256] {
        validate_exact_result_bytes(h, 32)?;
    }
    for h in [&p.welcome_sha256, &p.group_info_sha256]
        .into_iter()
        .flatten()
    {
        validate_exact_result_bytes(h, 32)?;
    }
    Ok(())
}
fn validate_inspection(r: &NativeInspectPendingRequestV3) -> Result<(), Error> {
    validate_profile(r.profile_id)?;
    validate_retention(r.expected_retention)?;
    validate_exact_result_bytes(&r.group_id, 16)?;
    id(&r.incarnation_id)?;
    state(&r.expected_current)?;
    validate_exact_result_bytes(&r.expected_base_group_state_sha256, 32)?;
    if r.group_id != r.expected_current.group_id {
        return Err(Error::GroupMismatch);
    }
    Ok(())
}
fn put_state(
    e: &mut Encoder<&mut Vec<u8>>,
    s: &crate::api::group_e2ee::MlsExpectedRosterStateV1,
) -> Result<(), Error> {
    encode_expected_roster(
        e,
        &NativeExpectedRosterStateV2 {
            group_id: s.group_id.clone(),
            epoch: s.epoch,
            digest_sha256: s.digest_sha256.clone(),
        },
    )
}
fn put_leaf(
    e: &mut Encoder<&mut Vec<u8>>,
    l: &crate::api::group_e2ee::MlsRosterLeafV1,
) -> Result<(), Error> {
    encode_leaf(e, &mls_leaf_into_native(l.clone()))
}
fn get_state(
    d: &mut Decoder<'_>,
) -> Result<crate::api::group_e2ee::MlsExpectedRosterStateV1, Error> {
    Ok(native_expected_into_mls(decode_expected_roster(d)?))
}
fn get_leaf(d: &mut Decoder<'_>) -> Result<crate::api::group_e2ee::MlsRosterLeafV1, Error> {
    Ok(native_leaf_into_mls(decode_leaf(d)?))
}
fn get_id(d: &mut Decoder<'_>) -> Result<Vec<u8>, Error> {
    decode_bounded_bytes(d, 1, 128)
}
fn get_hash(d: &mut Decoder<'_>) -> Result<Vec<u8>, Error> {
    decode_exact_bytes(d, 32)
}
fn optional_hash(d: &mut Decoder<'_>) -> Result<Option<Vec<u8>>, Error> {
    if d.datatype().map_err(|_| Error::NoncanonicalEncoding)? == Type::Null {
        d.null().map_err(|_| Error::NoncanonicalEncoding)?;
        Ok(None)
    } else {
        Ok(Some(get_hash(d)?))
    }
}
fn put_optional_hash(e: &mut Encoder<&mut Vec<u8>>, h: &Option<Vec<u8>>) -> Result<(), Error> {
    match h {
        Some(v) => {
            e.bytes(v).map_err(encode_error)?;
        }
        None => {
            e.null().map_err(encode_error)?;
        }
    }
    Ok(())
}
fn put_pending(
    e: &mut Encoder<&mut Vec<u8>>,
    p: &Option<MlsPendingCommitBinding>,
) -> Result<(), Error> {
    let Some(p) = p else {
        e.null().map_err(encode_error)?;
        return Ok(());
    };
    validate_pending(p)?;
    e.map(10)
        .and_then(|e| e.u8(0))
        .and_then(|e| e.bytes(&p.group_id))
        .and_then(|e| e.u8(1))
        .and_then(|e| e.bytes(&p.incarnation_id))
        .and_then(|e| e.u8(2))
        .and_then(|e| e.bytes(&p.transition.command_id))
        .and_then(|e| e.u8(3))
        .and_then(|e| e.bytes(&p.transition.context_sha256))
        .and_then(|e| e.u8(4))
        .map_err(encode_error)?;
    put_leaf(e, &p.author)?;
    e.u8(5).map_err(encode_error)?;
    put_state(e, &p.previous_state)?;
    e.u8(6)
        .and_then(|e| e.bytes(&p.commit_sha256))
        .and_then(|e| e.u8(7))
        .and_then(|e| e.bytes(&p.aad_sha256))
        .and_then(|e| e.u8(8))
        .map_err(encode_error)?;
    put_optional_hash(e, &p.welcome_sha256)?;
    e.u8(9).map_err(encode_error)?;
    put_optional_hash(e, &p.group_info_sha256)
}
fn get_pending(d: &mut Decoder<'_>) -> Result<Option<MlsPendingCommitBinding>, Error> {
    if d.datatype().map_err(|_| Error::NoncanonicalEncoding)? == Type::Null {
        d.null().map_err(|_| Error::NoncanonicalEncoding)?;
        return Ok(None);
    }
    expect_map(d, 10)?;
    expect_key(d, 0)?;
    let group_id = decode_exact_bytes(d, 16)?;
    expect_key(d, 1)?;
    let incarnation_id = get_id(d)?;
    expect_key(d, 2)?;
    let command_id = get_id(d)?;
    expect_key(d, 3)?;
    let context_sha256 = get_hash(d)?;
    expect_key(d, 4)?;
    let author = get_leaf(d)?;
    expect_key(d, 5)?;
    let previous_state = get_state(d)?;
    expect_key(d, 6)?;
    let commit_sha256 = get_hash(d)?;
    expect_key(d, 7)?;
    let aad_sha256 = get_hash(d)?;
    expect_key(d, 8)?;
    let welcome_sha256 = optional_hash(d)?;
    expect_key(d, 9)?;
    let group_info_sha256 = optional_hash(d)?;
    Ok(Some(MlsPendingCommitBinding {
        group_id,
        incarnation_id,
        transition: MlsTransitionContext {
            command_id,
            context_sha256,
        },
        author,
        previous_state,
        commit_sha256,
        aad_sha256,
        welcome_sha256,
        group_info_sha256,
    }))
}
fn put_acceptance(
    e: &mut Encoder<&mut Vec<u8>>,
    a: &MlsExternalCommitAcceptanceV1,
) -> Result<(), Error> {
    let b = &a.binding;
    e.map(11)
        .and_then(|e| e.u8(0))
        .and_then(|e| e.bytes(&b.candidate_id))
        .and_then(|e| e.u8(1))
        .and_then(|e| e.bytes(&b.transition.command_id))
        .and_then(|e| e.u8(2))
        .and_then(|e| e.bytes(&b.transition.context_sha256))
        .and_then(|e| e.u8(3))
        .map_err(encode_error)?;
    put_state(e, &b.previous_state)?;
    e.u8(4).map_err(encode_error)?;
    put_state(e, &a.resulting_state)?;
    e.u8(5).map_err(encode_error)?;
    put_leaf(e, &b.old_leaf)?;
    e.u8(6).map_err(encode_error)?;
    put_leaf(e, &b.author)?;
    for (key, bytes) in [
        (7, &b.input_group_info_sha256),
        (8, &b.resulting_group_info_sha256),
        (9, &b.commit_sha256),
        (10, &b.aad_sha256),
    ] {
        e.u8(key)
            .and_then(|e| e.bytes(bytes))
            .map_err(encode_error)?;
    }
    Ok(())
}
fn get_acceptance(
    d: &mut Decoder<'_>,
    incarnation: &[u8],
) -> Result<MlsExternalCommitAcceptanceV1, Error> {
    use crate::api::external_resync::MlsExternalCommitBindingV1;
    expect_map(d, 11)?;
    expect_key(d, 0)?;
    let candidate_id = get_id(d)?;
    expect_key(d, 1)?;
    let command_id = get_id(d)?;
    expect_key(d, 2)?;
    let context_sha256 = get_hash(d)?;
    expect_key(d, 3)?;
    let previous_state = get_state(d)?;
    expect_key(d, 4)?;
    let resulting_state = get_state(d)?;
    expect_key(d, 5)?;
    let old_leaf = get_leaf(d)?;
    expect_key(d, 6)?;
    let author = get_leaf(d)?;
    expect_key(d, 7)?;
    let input_group_info_sha256 = get_hash(d)?;
    expect_key(d, 8)?;
    let resulting_group_info_sha256 = get_hash(d)?;
    expect_key(d, 9)?;
    let commit_sha256 = get_hash(d)?;
    expect_key(d, 10)?;
    let aad_sha256 = get_hash(d)?;
    Ok(MlsExternalCommitAcceptanceV1 {
        binding: MlsExternalCommitBindingV1 {
            candidate_id,
            incarnation_id: incarnation.to_vec(),
            transition: MlsTransitionContext {
                command_id,
                context_sha256,
            },
            previous_state,
            old_leaf,
            author,
            input_group_info_sha256,
            resulting_group_info_sha256,
            commit_sha256,
            aad_sha256,
        },
        resulting_state,
    })
}
pub fn encode_native_receive_request_v3(
    request: &NativeReceiveRequestV3,
) -> Result<Vec<u8>, Error> {
    encode_request(request, true)
}
#[cfg(test)]
pub(crate) fn encode_unchecked_fixture(request: &NativeReceiveRequestV3) -> Result<Vec<u8>, Error> {
    encode_request(request, false)
}
fn encode_request(request: &NativeReceiveRequestV3, check: bool) -> Result<Vec<u8>, Error> {
    if let NativeReceiveRequestV3::InspectPending(r) = request {
        if check {
            validate_inspection(r)?;
        }
        let mut bytes = Zeroizing::new(Vec::new());
        let mut e = Encoder::new(&mut *bytes);
        e.map(7)
            .and_then(|e| e.u8(0))
            .and_then(|e| e.u16(r.profile_id))
            .and_then(|e| e.u8(1))
            .and_then(|e| e.bytes(&r.group_id))
            .and_then(|e| e.u8(2))
            .and_then(|e| e.bytes(&r.incarnation_id))
            .and_then(|e| e.u8(3))
            .map_err(encode_error)?;
        put_state(&mut e, &r.expected_current)?;
        e.u8(4)
            .and_then(|e| e.bytes(&r.expected_base_group_state_sha256))
            .and_then(|e| e.u8(5))
            .and_then(|e| e.u32(r.expected_retention))
            .and_then(|e| e.u8(6))
            .map_err(encode_error)?;
        encode_storage_snapshot(&mut e, &r.storage, Some(&r.group_id))?;
        return frame(
            INSPECT_PENDING_OPERATION,
            std::mem::take(&mut *bytes),
            NATIVE_RECEIVE_REQUEST_MAX_BYTES,
        );
    }
    let NativeReceiveRequestV3::External(r) = request else {
        let NativeReceiveRequestV3::Member(r) = request else {
            unreachable!()
        };
        let mut frame = v2::encode_native_receive_request_v2(r)?;
        frame[4..6].copy_from_slice(&3u16.to_be_bytes());
        return Ok(frame);
    };
    if check {
        validate(r)?;
    }
    let mut bytes = Zeroizing::new(Vec::new());
    let mut e = Encoder::new(&mut *bytes);
    e.map(10)
        .and_then(|e| e.u8(0))
        .and_then(|e| e.u16(r.profile_id))
        .and_then(|e| e.u8(1))
        .and_then(|e| e.bytes(&r.group_id))
        .and_then(|e| e.u8(2))
        .and_then(|e| e.bytes(&r.incarnation_id))
        .and_then(|e| e.u8(3))
        .and_then(|e| e.bytes(&r.commit))
        .and_then(|e| e.u8(4))
        .and_then(|e| e.bytes(&r.expected_aad))
        .and_then(|e| e.u8(5))
        .and_then(|e| e.bytes(&r.expected_base_group_state_sha256))
        .and_then(|e| e.u8(6))
        .and_then(|e| e.u32(r.expected_retention))
        .and_then(|e| e.u8(7))
        .map_err(encode_error)?;
    put_acceptance(&mut e, &r.acceptance)?;
    e.u8(8).map_err(encode_error)?;
    put_pending(&mut e, &r.expected_superseded_pending)?;
    e.u8(9).map_err(encode_error)?;
    encode_storage_snapshot(&mut e, &r.storage, Some(&r.group_id))?;
    frame(
        4,
        std::mem::take(&mut *bytes),
        NATIVE_RECEIVE_REQUEST_MAX_BYTES,
    )
}
pub fn decode_native_receive_request_v3(input: &[u8]) -> Result<NativeReceiveRequestV3, Error> {
    let (op, payload) = header(input)?;
    preflight_cbor(payload)?;
    let mut d = Decoder::new(payload);
    let request = match op {
        1 | 2 => NativeReceiveRequestV3::Member(decode_process_request(
            &mut d,
            if op == 1 {
                NativeReceiveOperationV2::Application
            } else {
                NativeReceiveOperationV2::Commit
            },
        )?),
        3 => NativeReceiveRequestV3::Member(decode_welcome_request(&mut d)?),
        4 => {
            expect_map(&mut d, 10)?;
            expect_key(&mut d, 0)?;
            let profile_id = d.u16().map_err(|_| Error::NoncanonicalEncoding)?;
            expect_key(&mut d, 1)?;
            let group_id = decode_exact_bytes(&mut d, 16)?;
            expect_key(&mut d, 2)?;
            let incarnation_id = get_id(&mut d)?;
            expect_key(&mut d, 3)?;
            let commit = decode_bounded_bytes(&mut d, 1, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES)?;
            expect_key(&mut d, 4)?;
            let expected_aad = decode_bounded_bytes(&mut d, 1, NATIVE_RECEIVE_AAD_MAX_BYTES)?;
            expect_key(&mut d, 5)?;
            let expected_base_group_state_sha256 = get_hash(&mut d)?;
            expect_key(&mut d, 6)?;
            let expected_retention = d.u32().map_err(|_| Error::NoncanonicalEncoding)?;
            expect_key(&mut d, 7)?;
            let acceptance = get_acceptance(&mut d, &incarnation_id)?;
            expect_key(&mut d, 8)?;
            let expected_superseded_pending = get_pending(&mut d)?;
            let mut r = NativeExternalReceiveRequestV3 {
                profile_id,
                group_id,
                incarnation_id,
                commit,
                expected_aad,
                expected_base_group_state_sha256,
                expected_retention,
                acceptance,
                expected_superseded_pending,
                storage: NativeStorageSnapshotV2::default(),
            };
            validate(&r)?; // Reject inconsistent public authority before cloning secret storage.
            expect_key(&mut d, 9)?;
            r.storage = decode_storage_snapshot(&mut d, Some(&r.group_id))?;
            NativeReceiveRequestV3::External(r)
        }
        5 => {
            expect_map(&mut d, 7)?;
            expect_key(&mut d, 0)?;
            let profile_id = d.u16().map_err(|_| Error::NoncanonicalEncoding)?;
            expect_key(&mut d, 1)?;
            let group_id = decode_exact_bytes(&mut d, 16)?;
            expect_key(&mut d, 2)?;
            let incarnation_id = get_id(&mut d)?;
            expect_key(&mut d, 3)?;
            let expected_current = get_state(&mut d)?;
            expect_key(&mut d, 4)?;
            let expected_base_group_state_sha256 = get_hash(&mut d)?;
            expect_key(&mut d, 5)?;
            let expected_retention = d.u32().map_err(|_| Error::NoncanonicalEncoding)?;
            let mut r = NativeInspectPendingRequestV3 {
                profile_id,
                group_id,
                incarnation_id,
                expected_current,
                expected_base_group_state_sha256,
                expected_retention,
                storage: NativeStorageSnapshotV2::default(),
            };
            validate_inspection(&r)?;
            expect_key(&mut d, 6)?;
            r.storage = decode_storage_snapshot(&mut d, Some(&r.group_id))?;
            NativeReceiveRequestV3::InspectPending(r)
        }
        _ => unreachable!(),
    };
    if d.position() != payload.len() {
        return Err(Error::NoncanonicalEncoding);
    }
    Ok(request)
}
fn execute(request: NativeReceiveRequestV3) -> Result<Success, Error> {
    match request {
        NativeReceiveRequestV3::Member(r) => Ok(Success::Member(v2::execute_request(r)?)),
        NativeReceiveRequestV3::InspectPending(mut r) => {
            let context = runtime::Context {
                group_id: r.group_id,
                incarnation_id: r.incarnation_id,
                expected_current: r.expected_current,
                expected_base_sha256: r.expected_base_group_state_sha256,
                expected_retention: r.expected_retention,
                entries: std::mem::take(&mut r.storage.entries)
                    .into_iter()
                    .map(native_storage_entry_into_mls)
                    .collect(),
                storage_format: r.storage.storage_format_version,
            };
            Ok(Success::Pending(
                runtime::inspect_pending(context)?.map(|p| p.binding),
            ))
        }
        NativeReceiveRequestV3::External(mut r) => {
            let context = runtime::Context {
                group_id: std::mem::take(&mut r.group_id),
                incarnation_id: std::mem::take(&mut r.incarnation_id),
                expected_current: r.acceptance.binding.previous_state.clone(),
                expected_base_sha256: std::mem::take(&mut r.expected_base_group_state_sha256),
                expected_retention: r.expected_retention,
                entries: std::mem::take(&mut r.storage.entries)
                    .into_iter()
                    .map(native_storage_entry_into_mls)
                    .collect(),
                storage_format: r.storage.storage_format_version,
            };
            let result = runtime::external::receive(
                context,
                std::mem::take(&mut r.commit),
                std::mem::take(&mut r.expected_aad),
                r.acceptance.clone(),
                r.expected_superseded_pending.take(),
            )?;
            Ok(Success::External {
                superseded: result.superseded_pending,
                transition: NativeReceiveSuccessV2::Commit {
                    message_epoch: result.previous_roster.epoch,
                    sender: mls_leaf_into_native(result.authenticated_sender),
                    previous_roster: mls_roster_into_native(result.previous_roster),
                    resulting_roster: mls_roster_into_native(result.resulting_roster),
                    resulting_group_state_sha256: result.resulting_group_state_sha256,
                    effective_retention: result.effective_retention,
                    storage_batch: mls_batch_into_native(result.storage_batch),
                },
            })
        }
    }
}
fn encode_result(op: u8, result: Result<Success, Error>) -> Result<Vec<u8>, Error> {
    let mut bytes = Zeroizing::new(Vec::new());
    let mut e = Encoder::new(&mut *bytes);
    e.map(3)
        .and_then(|e| e.u8(0))
        .and_then(|e| e.u16(3))
        .and_then(|e| e.u8(1))
        .and_then(|e| e.bool(false))
        .map_err(encode_error)?;
    match result {
        Ok(Success::Pending(ref pending)) => {
            e.u8(2)
                .and_then(|e| e.map(1))
                .and_then(|e| e.u8(0))
                .map_err(encode_error)?;
            put_pending(&mut e, pending)?;
        }
        Ok(Success::Member(ref r)) => {
            e.u8(2).map_err(encode_error)?;
            encode_success(&mut e, r)?;
        }
        Ok(Success::External {
            ref transition,
            ref superseded,
        }) => {
            e.u8(2)
                .and_then(|e| e.map(2))
                .and_then(|e| e.u8(0))
                .map_err(encode_error)?;
            encode_success(&mut e, transition)?;
            e.u8(1).map_err(encode_error)?;
            put_pending(&mut e, superseded)?;
        }
        Err(error) => {
            e.u8(3)
                .and_then(|e| e.map(1))
                .and_then(|e| e.u8(3))
                .and_then(|e| e.u16(error as u16))
                .map_err(encode_error)?;
        }
    }
    frame(
        op,
        std::mem::take(&mut *bytes),
        NATIVE_RECEIVE_RESULT_MAX_BYTES,
    )
}
pub(crate) fn encode_native_receive_failure_v3(op: u8, error: Error) -> Vec<u8> {
    encode_result(if (1..=5).contains(&op) { op } else { 0 }, Err(error)).unwrap_or_else(|_| {
        let mut out = b"KMLS\0\x03\0\0\0\0\0\x0a".to_vec();
        out.extend_from_slice(&[0xa3, 0, 3, 1, 0xf4, 3, 0xa1, 3, 0x18, 0xff]);
        out
    })
}
pub fn execute_native_receive_v3(input: &[u8]) -> Vec<u8> {
    let op = input
        .get(6)
        .copied()
        .filter(|op| (1..=5).contains(op))
        .unwrap_or(0);
    let result = catch_unwind(AssertUnwindSafe(|| {
        execute(decode_native_receive_request_v3(input)?)
    }))
    .unwrap_or(Err(Error::InternalFailure));
    encode_result(op, result).unwrap_or_else(|error| encode_native_receive_failure_v3(op, error))
}
