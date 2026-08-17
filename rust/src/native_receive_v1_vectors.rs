//! Generator and replay validator for committed native receive v1 frames.
//!
//! This module is test/feature gated and never ships in release artifacts.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use minicbor::Decoder;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::api::config::MlsGroupConfig;
use crate::api::group_e2ee::{
    MlsAuthorizedKeyPackageV1, MlsAuthorizedOwnerV1, MlsExpectedRosterStateV1, MlsRosterLeafV1,
    MlsRosterSummaryV1, add_members_with_storage, create_group_with_storage,
    join_group_from_welcome_with_storage, mls_group_state_digest, process_message_with_storage,
};
use crate::api::keys::{MlsSignatureKeyPair, serialize_signer};
use crate::api::storage::{
    MLS_STORAGE_FORMAT_VERSION, MlsStorageBatch, MlsStorageEntry, create_key_package_with_storage,
    create_message_with_storage, zeroize_entry_values,
};
use crate::api::types::MlsCiphersuite;
use crate::native_receive_v1::{
    NATIVE_RECEIVE_CONTRACT_VERSION, NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES,
    NATIVE_RECEIVE_PROFILE_V1, NATIVE_RECEIVE_REQUEST_MAX_BYTES, NATIVE_RECEIVE_RESULT_MAX_BYTES,
    NATIVE_RECEIVE_ROSTER_MAX_LEAVES, NATIVE_RECEIVE_STORAGE_MAX_BYTES,
    NativeExpectedRosterStateV1, NativeLeafAuthorityV1, NativeReceiveErrorCodeV1,
    NativeReceiveOperationV1, NativeReceiveRequestV1, NativeStorageEntryV1,
    NativeStorageSnapshotV1, encode_native_receive_request_v1, execute_native_receive_v1,
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NativeReceiveV1VectorManifest {
    pub generated_date: String,
    pub contract_version: u16,
    pub profile_id: u16,
    pub storage_format_version: u32,
    pub synthetic_secrets_only: bool,
    pub limits_256: NativeReceiveV1LimitEvidence,
    pub vectors: Vec<NativeReceiveV1VectorRecord>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NativeReceiveV1LimitEvidence {
    pub roster_leaves: usize,
    pub welcome_wire_bytes: usize,
    pub application_ciphertext_bytes: usize,
    pub welcome: NativeReceiveV1OperationEvidence,
    pub application: NativeReceiveV1OperationEvidence,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NativeReceiveV1OperationEvidence {
    pub request_bytes: usize,
    pub response_bytes: usize,
    pub snapshot_entries: usize,
    pub snapshot_bytes: usize,
    pub batch_upserts: usize,
    pub batch_deletes: usize,
    pub batch_deleted_groups: usize,
    pub batch_bytes: usize,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NativeReceiveV1VectorRecord {
    pub id: String,
    pub operation: u8,
    pub request_file: String,
    pub request_sha256: String,
    pub response_file: String,
    pub response_sha256: String,
    pub expected_error_code: Option<u16>,
}

pub fn write_native_receive_v1_vectors(output: &Path) -> Result<(), String> {
    fs::create_dir_all(output).map_err(|error| format!("create fixture directory: {error}"))?;
    let (vectors, limits_256) = build_vectors()?;
    let mut records = Vec::with_capacity(vectors.len());
    for vector in vectors {
        let request = encode_native_receive_request_v1(&vector.request)
            .map_err(|error| format!("encode {} request: {error:?}", vector.id))?;
        let response = execute_native_receive_v1(&request);
        validate_response_shape(&response, vector.operation, vector.expected_error)?;
        let request_file = format!("{}.request.bin", vector.id);
        let response_file = format!("{}.response.bin", vector.id);
        fs::write(output.join(&request_file), &request)
            .map_err(|error| format!("write {request_file}: {error}"))?;
        fs::write(output.join(&response_file), &response)
            .map_err(|error| format!("write {response_file}: {error}"))?;
        records.push(NativeReceiveV1VectorRecord {
            id: vector.id,
            operation: vector.operation as u8,
            request_file,
            request_sha256: sha256_hex(&request),
            response_file,
            response_sha256: sha256_hex(&response),
            expected_error_code: vector.expected_error.map(|error| error as u16),
        });
    }
    let manifest = NativeReceiveV1VectorManifest {
        generated_date: "2026-08-17".to_string(),
        contract_version: NATIVE_RECEIVE_CONTRACT_VERSION,
        profile_id: NATIVE_RECEIVE_PROFILE_V1,
        storage_format_version: MLS_STORAGE_FORMAT_VERSION,
        synthetic_secrets_only: true,
        limits_256,
        vectors: records,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("encode fixture manifest: {error}"))?;
    fs::write(output.join("manifest.json"), manifest_bytes)
        .map_err(|error| format!("write fixture manifest: {error}"))?;
    Ok(())
}

pub fn validate_native_receive_v1_vectors(output: &Path) -> Result<(), String> {
    let manifest_bytes = fs::read(output.join("manifest.json"))
        .map_err(|error| format!("read fixture manifest: {error}"))?;
    let manifest: NativeReceiveV1VectorManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("decode fixture manifest: {error}"))?;
    if manifest.contract_version != NATIVE_RECEIVE_CONTRACT_VERSION
        || manifest.profile_id != NATIVE_RECEIVE_PROFILE_V1
        || manifest.storage_format_version != MLS_STORAGE_FORMAT_VERSION
        || !manifest.synthetic_secrets_only
    {
        return Err("fixture manifest authority does not match this build".to_string());
    }
    validate_limit_evidence(&manifest.limits_256)?;
    for vector in manifest.vectors {
        let request = fs::read(output.join(&vector.request_file))
            .map_err(|error| format!("read {}: {error}", vector.request_file))?;
        let expected_response = fs::read(output.join(&vector.response_file))
            .map_err(|error| format!("read {}: {error}", vector.response_file))?;
        if sha256_hex(&request) != vector.request_sha256
            || sha256_hex(&expected_response) != vector.response_sha256
        {
            return Err(format!("fixture {} digest mismatch", vector.id));
        }
        validate_limit_record(
            &manifest.limits_256,
            &vector.id,
            request.len(),
            expected_response.len(),
        )?;
        let actual_response = execute_native_receive_v1(&request);
        if actual_response != expected_response {
            return Err(format!("fixture {} response mismatch", vector.id));
        }
        let operation = NativeReceiveOperationV1::from_u8_for_vectors(vector.operation)?;
        let expected_error = vector.expected_error_code.map(error_from_u16).transpose()?;
        validate_response_shape(&actual_response, operation, expected_error)?;
    }
    Ok(())
}

struct GeneratedVector {
    id: String,
    operation: NativeReceiveOperationV1,
    request: NativeReceiveRequestV1,
    expected_error: Option<NativeReceiveErrorCodeV1>,
}

fn build_vectors() -> Result<(Vec<GeneratedVector>, NativeReceiveV1LimitEvidence), String> {
    let config = MlsGroupConfig::default_config(ciphersuite());
    let group_id = vec![0x51; 16];
    let alice_identity = identity(1);
    let bob_identity = identity(2);
    let charlie_identity = identity(3);
    let (alice_signer, alice_public) = signer()?;
    let (bob_signer, bob_public) = signer()?;
    let (charlie_signer, charlie_public) = signer()?;

    let created = create_group_with_storage(
        config.clone(),
        alice_signer.clone(),
        group_id.clone(),
        MlsAuthorizedOwnerV1 {
            expected_credential_identity: alice_identity.clone(),
            expected_signature_public_key: alice_public.clone(),
        },
        None,
        Vec::new(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let mut alice_entries = Vec::new();
    apply_batch(&mut alice_entries, &created.storage_batch);

    let bob_key_package = create_key_package_with_storage(
        ciphersuite(),
        bob_signer.clone(),
        bob_identity.clone(),
        bob_public.clone(),
        None,
        Vec::new(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let bob_key_package_sha256 = Sha256::digest(&bob_key_package.key_package_bytes).to_vec();
    let mut bob_global_entries = Vec::new();
    apply_batch(&mut bob_global_entries, &bob_key_package.storage_batch);

    let add_bob = add_members_with_storage(
        group_id.clone(),
        alice_signer.clone(),
        vec![MlsAuthorizedKeyPackageV1 {
            key_package_bytes: bob_key_package.key_package_bytes,
            expected_credential_identity: bob_identity.clone(),
            expected_signature_public_key: bob_public,
        }],
        b"fixture-add-bob-aad".to_vec(),
        expected(&created.resulting_roster),
        alice_entries.clone(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    apply_batch(&mut alice_entries, &add_bob.storage_batch);
    let two_member_roster = add_bob.resulting_roster.clone();
    let welcome_bytes = add_bob
        .welcome
        .clone()
        .ok_or_else(|| "fixture add did not produce Welcome".to_string())?;
    let alice_leaf = native_leaf(
        two_member_roster
            .leaves
            .iter()
            .find(|leaf| leaf.credential_identity == alice_identity)
            .ok_or_else(|| "fixture Alice leaf missing".to_string())?,
    );
    let bob_leaf = native_leaf(
        two_member_roster
            .leaves
            .iter()
            .find(|leaf| leaf.credential_identity == bob_identity)
            .ok_or_else(|| "fixture Bob leaf missing".to_string())?,
    );

    let welcome_request = || NativeReceiveRequestV1::Welcome {
        profile_id: NATIVE_RECEIVE_PROFILE_V1,
        welcome_bytes: welcome_bytes.clone(),
        ratchet_tree_bytes: None,
        signer_bytes: bob_signer.clone(),
        expected_local_leaf: bob_leaf.clone(),
        expected_resulting_state: native_expected(&two_member_roster),
        expected_target_key_package_sha256: bob_key_package_sha256.clone(),
        storage: native_snapshot(&bob_global_entries),
    };
    let mut vectors = vec![success(
        "welcome_success",
        NativeReceiveOperationV1::Welcome,
        welcome_request(),
    )];

    let mut wrong_key_package = welcome_request();
    if let NativeReceiveRequestV1::Welcome {
        expected_target_key_package_sha256,
        ..
    } = &mut wrong_key_package
    {
        expected_target_key_package_sha256[0] ^= 0xff;
    }
    vectors.push(failure(
        "welcome_wrong_key_package",
        NativeReceiveOperationV1::Welcome,
        wrong_key_package,
        NativeReceiveErrorCodeV1::ExpectedKeyPackageMismatch,
    ));
    let mut wrong_local_leaf = welcome_request();
    if let NativeReceiveRequestV1::Welcome {
        expected_local_leaf,
        ..
    } = &mut wrong_local_leaf
    {
        expected_local_leaf.leaf_index = expected_local_leaf.leaf_index.saturating_add(1);
    }
    vectors.push(failure(
        "welcome_wrong_local_leaf",
        NativeReceiveOperationV1::Welcome,
        wrong_local_leaf,
        NativeReceiveErrorCodeV1::LocalLeafMismatch,
    ));

    let joined = join_group_from_welcome_with_storage(
        config,
        welcome_bytes,
        None,
        bob_signer,
        expected(&two_member_roster),
        bob_global_entries,
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let mut bob_entries = Vec::new();
    apply_batch(&mut bob_entries, &joined.storage_batch);

    let application_aad = b"fixture-application-aad".to_vec();
    let application = create_message_with_storage(
        group_id.clone(),
        alice_signer.clone(),
        b"fixture plaintext".to_vec(),
        application_aad.clone(),
        alice_entries.clone(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    apply_batch(&mut alice_entries, &application.storage_batch);
    let bob_application_base = group_digest(&group_id, &bob_entries)?;
    let application_request = || NativeReceiveRequestV1::Process {
        operation: NativeReceiveOperationV1::Application,
        profile_id: NATIVE_RECEIVE_PROFILE_V1,
        group_id: group_id.clone(),
        message_bytes: application.ciphertext.clone(),
        expected_aad: application_aad.clone(),
        expected_sender: alice_leaf.clone(),
        expected_previous_state: native_expected(&two_member_roster),
        expected_resulting_state: native_expected(&two_member_roster),
        expected_base_group_state_sha256: bob_application_base.clone(),
        storage: native_snapshot(&bob_entries),
    };
    vectors.push(success(
        "application_success",
        NativeReceiveOperationV1::Application,
        application_request(),
    ));

    let mut wrong_base = application_request();
    if let NativeReceiveRequestV1::Process {
        expected_base_group_state_sha256,
        ..
    } = &mut wrong_base
    {
        expected_base_group_state_sha256[0] ^= 0xff;
    }
    vectors.push(failure(
        "application_wrong_base",
        NativeReceiveOperationV1::Application,
        wrong_base,
        NativeReceiveErrorCodeV1::BaseStateMismatch,
    ));
    let mut wrong_aad = application_request();
    if let NativeReceiveRequestV1::Process { expected_aad, .. } = &mut wrong_aad {
        *expected_aad = b"wrong-aad".to_vec();
    }
    vectors.push(failure(
        "application_wrong_aad",
        NativeReceiveOperationV1::Application,
        wrong_aad,
        NativeReceiveErrorCodeV1::AadMismatch,
    ));
    let mut wrong_sender = application_request();
    if let NativeReceiveRequestV1::Process {
        expected_sender, ..
    } = &mut wrong_sender
    {
        expected_sender.signature_public_key[0] ^= 0xff;
    }
    vectors.push(failure(
        "application_wrong_sender",
        NativeReceiveOperationV1::Application,
        wrong_sender,
        NativeReceiveErrorCodeV1::SenderMismatch,
    ));
    let mut wrong_roster = application_request();
    if let NativeReceiveRequestV1::Process {
        expected_resulting_state,
        ..
    } = &mut wrong_roster
    {
        expected_resulting_state.digest_sha256[0] ^= 0xff;
    }
    vectors.push(failure(
        "application_wrong_roster",
        NativeReceiveOperationV1::Application,
        wrong_roster,
        NativeReceiveErrorCodeV1::ResultingRosterMismatch,
    ));
    let mut wrong_kind = application_request();
    if let NativeReceiveRequestV1::Process { operation, .. } = &mut wrong_kind {
        *operation = NativeReceiveOperationV1::Commit;
    }
    vectors.push(failure(
        "application_wrong_kind",
        NativeReceiveOperationV1::Commit,
        wrong_kind,
        NativeReceiveErrorCodeV1::MessageKindMismatch,
    ));

    let processed_application = process_message_with_storage(
        group_id.clone(),
        application.ciphertext,
        application_aad,
        expected(&two_member_roster),
        expected(&two_member_roster),
        bob_entries.clone(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    apply_batch(&mut bob_entries, &processed_application.storage_batch);

    let charlie_key_package = create_key_package_with_storage(
        ciphersuite(),
        charlie_signer,
        charlie_identity.clone(),
        charlie_public.clone(),
        None,
        Vec::new(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let commit_aad = b"fixture-commit-aad".to_vec();
    let commit = add_members_with_storage(
        group_id.clone(),
        alice_signer,
        vec![MlsAuthorizedKeyPackageV1 {
            key_package_bytes: charlie_key_package.key_package_bytes,
            expected_credential_identity: charlie_identity,
            expected_signature_public_key: charlie_public,
        }],
        commit_aad.clone(),
        expected(&two_member_roster),
        alice_entries,
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let commit_request = NativeReceiveRequestV1::Process {
        operation: NativeReceiveOperationV1::Commit,
        profile_id: NATIVE_RECEIVE_PROFILE_V1,
        group_id: group_id.clone(),
        message_bytes: commit.commit,
        expected_aad: commit_aad,
        expected_sender: alice_leaf,
        expected_previous_state: native_expected(&two_member_roster),
        expected_resulting_state: native_expected(&commit.resulting_roster),
        expected_base_group_state_sha256: group_digest(&group_id, &bob_entries)?,
        storage: native_snapshot(&bob_entries),
    };
    vectors.push(success(
        "commit_success",
        NativeReceiveOperationV1::Commit,
        commit_request,
    ));
    let (mut limit_vectors, limit_evidence) = build_limit_vectors()?;
    vectors.append(&mut limit_vectors);
    Ok((vectors, limit_evidence))
}

fn build_limit_vectors() -> Result<(Vec<GeneratedVector>, NativeReceiveV1LimitEvidence), String> {
    const ROSTER_LEAVES: usize = 256;

    let config = MlsGroupConfig::default_config(ciphersuite());
    let group_id = vec![0x52; 16];
    let owner_identity = limit_identity(0);
    let target_identity = limit_identity(1);
    let (owner_signer, owner_public) = signer()?;
    let created = create_group_with_storage(
        config.clone(),
        owner_signer.clone(),
        group_id.clone(),
        MlsAuthorizedOwnerV1 {
            expected_credential_identity: owner_identity.clone(),
            expected_signature_public_key: owner_public,
        },
        None,
        Vec::new(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let mut owner_entries = Vec::new();
    apply_batch(&mut owner_entries, &created.storage_batch);

    let mut additions = Vec::with_capacity(ROSTER_LEAVES - 1);
    let mut target_signer = Vec::new();
    let mut target_public = Vec::new();
    let mut target_entries = Vec::new();
    let mut target_key_package_sha256 = Vec::new();
    for member in 1..ROSTER_LEAVES {
        let identity = limit_identity(member as u16);
        let (mut member_signer, member_public) = signer()?;
        let mut key_package = create_key_package_with_storage(
            ciphersuite(),
            member_signer.clone(),
            identity.clone(),
            member_public.clone(),
            None,
            Vec::new(),
            MLS_STORAGE_FORMAT_VERSION,
        )?;
        if member == 1 {
            target_signer = member_signer.clone();
            target_public = member_public.clone();
            target_key_package_sha256 = Sha256::digest(&key_package.key_package_bytes).to_vec();
            apply_batch(&mut target_entries, &key_package.storage_batch);
        }
        member_signer.zeroize();
        zeroize_entry_values(&mut key_package.storage_batch.upserts);
        additions.push(MlsAuthorizedKeyPackageV1 {
            key_package_bytes: key_package.key_package_bytes,
            expected_credential_identity: identity,
            expected_signature_public_key: member_public,
        });
    }

    let added = add_members_with_storage(
        group_id.clone(),
        owner_signer.clone(),
        additions,
        vec![0x41; 2048],
        expected(&created.resulting_roster),
        owner_entries.clone(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    if added.resulting_roster.leaves.len() != ROSTER_LEAVES {
        return Err("limit fixture did not create exactly 256 active leaves".to_string());
    }
    apply_batch(&mut owner_entries, &added.storage_batch);
    let welcome_bytes = added
        .welcome
        .clone()
        .ok_or_else(|| "limit fixture add did not produce Welcome".to_string())?;
    let target_leaf = native_leaf(
        added
            .resulting_roster
            .leaves
            .iter()
            .find(|leaf| {
                leaf.credential_identity == target_identity
                    && leaf.signature_public_key == target_public
            })
            .ok_or_else(|| "limit fixture target leaf missing".to_string())?,
    );

    let welcome_request = NativeReceiveRequestV1::Welcome {
        profile_id: NATIVE_RECEIVE_PROFILE_V1,
        welcome_bytes: welcome_bytes.clone(),
        ratchet_tree_bytes: None,
        signer_bytes: target_signer.clone(),
        expected_local_leaf: target_leaf,
        expected_resulting_state: native_expected(&added.resulting_roster),
        expected_target_key_package_sha256: target_key_package_sha256,
        storage: native_snapshot(&target_entries),
    };
    let welcome_frame = encode_native_receive_request_v1(&welcome_request)
        .map_err(|error| format!("encode 256-leaf Welcome request: {error:?}"))?;
    let welcome_response = execute_native_receive_v1(&welcome_frame);
    validate_response_shape(&welcome_response, NativeReceiveOperationV1::Welcome, None)?;
    let joined = join_group_from_welcome_with_storage(
        config,
        welcome_bytes.clone(),
        None,
        target_signer,
        expected(&added.resulting_roster),
        target_entries.clone(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let welcome_evidence = operation_evidence(
        welcome_frame.len(),
        welcome_response.len(),
        &target_entries,
        &joined.storage_batch,
    );
    apply_batch(&mut target_entries, &joined.storage_batch);

    let application = create_message_with_storage(
        group_id.clone(),
        owner_signer,
        vec![0x50; 29 * 1024],
        vec![0x42; 2048],
        owner_entries,
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let owner_leaf = native_leaf(
        added
            .resulting_roster
            .leaves
            .iter()
            .find(|leaf| leaf.credential_identity == owner_identity)
            .ok_or_else(|| "limit fixture owner leaf missing".to_string())?,
    );
    let application_request = NativeReceiveRequestV1::Process {
        operation: NativeReceiveOperationV1::Application,
        profile_id: NATIVE_RECEIVE_PROFILE_V1,
        group_id: group_id.clone(),
        message_bytes: application.ciphertext.clone(),
        expected_aad: vec![0x42; 2048],
        expected_sender: owner_leaf,
        expected_previous_state: native_expected(&added.resulting_roster),
        expected_resulting_state: native_expected(&added.resulting_roster),
        expected_base_group_state_sha256: group_digest(&group_id, &target_entries)?,
        storage: native_snapshot(&target_entries),
    };
    let application_frame = encode_native_receive_request_v1(&application_request)
        .map_err(|error| format!("encode 256-leaf application request: {error:?}"))?;
    let application_response = execute_native_receive_v1(&application_frame);
    validate_response_shape(
        &application_response,
        NativeReceiveOperationV1::Application,
        None,
    )?;
    let processed = process_message_with_storage(
        group_id,
        application.ciphertext.clone(),
        vec![0x42; 2048],
        expected(&added.resulting_roster),
        expected(&added.resulting_roster),
        target_entries.clone(),
        MLS_STORAGE_FORMAT_VERSION,
    )?;
    let application_evidence = operation_evidence(
        application_frame.len(),
        application_response.len(),
        &target_entries,
        &processed.storage_batch,
    );

    let evidence = NativeReceiveV1LimitEvidence {
        roster_leaves: ROSTER_LEAVES,
        welcome_wire_bytes: welcome_bytes.len(),
        application_ciphertext_bytes: application.ciphertext.len(),
        welcome: welcome_evidence,
        application: application_evidence,
    };
    Ok((
        vec![
            success(
                "welcome_256_leaves",
                NativeReceiveOperationV1::Welcome,
                welcome_request,
            ),
            success(
                "application_256_leaves",
                NativeReceiveOperationV1::Application,
                application_request,
            ),
        ],
        evidence,
    ))
}

fn operation_evidence(
    request_bytes: usize,
    response_bytes: usize,
    snapshot: &[MlsStorageEntry],
    batch: &MlsStorageBatch,
) -> NativeReceiveV1OperationEvidence {
    NativeReceiveV1OperationEvidence {
        request_bytes,
        response_bytes,
        snapshot_entries: snapshot.len(),
        snapshot_bytes: entries_bytes(snapshot),
        batch_upserts: batch.upserts.len(),
        batch_deletes: batch.deletes.len(),
        batch_deleted_groups: batch.deleted_group_ids.len(),
        batch_bytes: entries_bytes(&batch.upserts)
            + batch.deletes.iter().map(Vec::len).sum::<usize>()
            + batch.deleted_group_ids.iter().map(Vec::len).sum::<usize>(),
    }
}

fn validate_limit_evidence(evidence: &NativeReceiveV1LimitEvidence) -> Result<(), String> {
    if evidence.roster_leaves != NATIVE_RECEIVE_ROSTER_MAX_LEAVES
        || evidence.welcome_wire_bytes > NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES
        || evidence.application_ciphertext_bytes > NATIVE_RECEIVE_MLS_MESSAGE_MAX_BYTES
    {
        return Err("256-leaf fixture does not exercise the declared roster ceiling".to_string());
    }
    for operation in [&evidence.welcome, &evidence.application] {
        if operation.request_bytes > NATIVE_RECEIVE_REQUEST_MAX_BYTES
            || operation.response_bytes > NATIVE_RECEIVE_RESULT_MAX_BYTES
            || operation.snapshot_bytes > NATIVE_RECEIVE_STORAGE_MAX_BYTES
            || operation.batch_bytes > NATIVE_RECEIVE_STORAGE_MAX_BYTES
        {
            return Err("256-leaf fixture exceeds a native receive ceiling".to_string());
        }
    }
    Ok(())
}

fn validate_limit_record(
    evidence: &NativeReceiveV1LimitEvidence,
    id: &str,
    request_bytes: usize,
    response_bytes: usize,
) -> Result<(), String> {
    let expected = match id {
        "welcome_256_leaves" => Some(&evidence.welcome),
        "application_256_leaves" => Some(&evidence.application),
        _ => None,
    };
    if let Some(expected) = expected
        && (request_bytes != expected.request_bytes || response_bytes != expected.response_bytes)
    {
        return Err(format!("fixture {id} does not match its limit evidence"));
    }
    Ok(())
}

fn entries_bytes(entries: &[MlsStorageEntry]) -> usize {
    entries
        .iter()
        .map(|entry| {
            entry.key.len() + entry.value.len() + entry.group_id.as_ref().map_or(0, Vec::len)
        })
        .sum()
}

fn limit_identity(value: u16) -> Vec<u8> {
    let mut identity = vec![0x4b; 45];
    identity[..2].copy_from_slice(&value.to_be_bytes());
    identity
}

fn success(
    id: &str,
    operation: NativeReceiveOperationV1,
    request: NativeReceiveRequestV1,
) -> GeneratedVector {
    GeneratedVector {
        id: id.to_string(),
        operation,
        request,
        expected_error: None,
    }
}

fn failure(
    id: &str,
    operation: NativeReceiveOperationV1,
    request: NativeReceiveRequestV1,
    error: NativeReceiveErrorCodeV1,
) -> GeneratedVector {
    GeneratedVector {
        id: id.to_string(),
        operation,
        request,
        expected_error: Some(error),
    }
}

fn signer() -> Result<(Vec<u8>, Vec<u8>), String> {
    let key_pair = MlsSignatureKeyPair::generate(ciphersuite())?;
    let public = key_pair.public_key();
    let signer = serialize_signer(ciphersuite(), key_pair.private_key(), public.clone())?;
    Ok((signer, public))
}

fn ciphersuite() -> MlsCiphersuite {
    MlsCiphersuite::Mls128DhkemX25519Aes128gcmSha256Ed25519
}

fn identity(value: u8) -> Vec<u8> {
    vec![value; 45]
}

fn expected(roster: &MlsRosterSummaryV1) -> MlsExpectedRosterStateV1 {
    MlsExpectedRosterStateV1 {
        group_id: roster.group_id.clone(),
        epoch: roster.epoch,
        digest_sha256: roster.digest_sha256.clone(),
    }
}

fn native_expected(roster: &MlsRosterSummaryV1) -> NativeExpectedRosterStateV1 {
    NativeExpectedRosterStateV1 {
        group_id: roster.group_id.clone(),
        epoch: roster.epoch,
        digest_sha256: roster.digest_sha256.clone(),
    }
}

fn native_leaf(leaf: &MlsRosterLeafV1) -> NativeLeafAuthorityV1 {
    NativeLeafAuthorityV1 {
        leaf_index: leaf.leaf_index,
        credential_identity: leaf.credential_identity.clone(),
        signature_public_key: leaf.signature_public_key.clone(),
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

fn group_digest(group_id: &[u8], entries: &[MlsStorageEntry]) -> Result<Vec<u8>, String> {
    mls_group_state_digest(
        group_id.to_vec(),
        entries.to_vec(),
        MLS_STORAGE_FORMAT_VERSION,
    )
}

fn apply_batch(entries: &mut Vec<MlsStorageEntry>, batch: &MlsStorageBatch) {
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

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn validate_response_shape(
    frame: &[u8],
    operation: NativeReceiveOperationV1,
    expected_error: Option<NativeReceiveErrorCodeV1>,
) -> Result<(), String> {
    if frame.len() < 12 || &frame[..4] != b"KMLS" || frame[6] != operation as u8 || frame[7] != 0 {
        return Err("response frame header mismatch".to_string());
    }
    let payload_len = u32::from_be_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
    if payload_len != frame.len() - 12 {
        return Err("response frame length mismatch".to_string());
    }
    let mut decoder = Decoder::new(&frame[12..]);
    if decoder.map().map_err(|error| error.to_string())? != Some(3)
        || decoder.u8().map_err(|error| error.to_string())? != 0
        || decoder.u16().map_err(|error| error.to_string())? != NATIVE_RECEIVE_CONTRACT_VERSION
        || decoder.u8().map_err(|error| error.to_string())? != 1
        || decoder.bool().map_err(|error| error.to_string())?
    {
        return Err("response authority fields mismatch".to_string());
    }
    match expected_error {
        Some(expected) => {
            if decoder.u8().map_err(|error| error.to_string())? != 3
                || decoder.map().map_err(|error| error.to_string())? != Some(1)
                || decoder.u8().map_err(|error| error.to_string())? != 3
                || decoder.u16().map_err(|error| error.to_string())? != expected as u16
                || decoder.position() != frame.len() - 12
            {
                return Err("response typed error mismatch".to_string());
            }
        }
        None => {
            if decoder.u8().map_err(|error| error.to_string())? != 2 {
                return Err("response success field missing".to_string());
            }
            decoder.skip().map_err(|error| error.to_string())?;
            if decoder.position() != frame.len() - 12 {
                return Err("response success has trailing bytes".to_string());
            }
        }
    }
    Ok(())
}

impl NativeReceiveOperationV1 {
    fn from_u8_for_vectors(value: u8) -> Result<Self, String> {
        match value {
            1 => Ok(Self::Application),
            2 => Ok(Self::Commit),
            3 => Ok(Self::Welcome),
            _ => Err("fixture operation is unsupported".to_string()),
        }
    }
}

fn error_from_u16(value: u16) -> Result<NativeReceiveErrorCodeV1, String> {
    match value {
        13 => Ok(NativeReceiveErrorCodeV1::BaseStateMismatch),
        25 => Ok(NativeReceiveErrorCodeV1::ResultingRosterMismatch),
        26 => Ok(NativeReceiveErrorCodeV1::AadMismatch),
        27 => Ok(NativeReceiveErrorCodeV1::MessageKindMismatch),
        28 => Ok(NativeReceiveErrorCodeV1::SenderMismatch),
        29 => Ok(NativeReceiveErrorCodeV1::LocalLeafMismatch),
        35 => Ok(NativeReceiveErrorCodeV1::ExpectedKeyPackageMismatch),
        _ => Err(format!("fixture error code {value} is unsupported")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_vectors_replay_exactly() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../native/receive_v1/fixtures");
        validate_native_receive_v1_vectors(&path).unwrap();
    }
}
