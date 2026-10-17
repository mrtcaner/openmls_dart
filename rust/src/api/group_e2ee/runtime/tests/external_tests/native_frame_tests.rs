//! Frame3 proof against the accepted N2 core, not consumer/JNI/Apple acceptance.
use super::*;
use crate::native_receive_v2 as v2;
use crate::native_receive_v3 as v3;
use minicbor::{Decoder, Encoder};
mod inspection_tests;

fn request(
    a: &Peer,
    p: &Held,
    pending: Option<MlsPendingCommitBinding>,
) -> v3::NativeReceiveRequestV3 {
    v3::NativeReceiveRequestV3::External(v3::NativeExternalReceiveRequestV3 {
        profile_id: 2,
        group_id: GID.to_vec(),
        incarnation_id: vec![8; 16],
        commit: p.info.commit.clone(),
        expected_aad: b"N2 opaque authenticated external AAD4".to_vec(),
        expected_base_group_state_sha256: context(a).expected_base_sha256.clone(),
        expected_retention: 4,
        acceptance: accepted(p),
        expected_superseded_pending: pending,
        storage: v2::NativeStorageSnapshotV2 {
            storage_format_version: 1,
            entries: a
                .rows
                .0
                .iter()
                .map(|r| v2::NativeStorageEntryV2 {
                    key: r.key.clone(),
                    value: r.value.clone(),
                    group_id: r.group_id.clone(),
                })
                .collect(),
        },
    })
}
fn error(frame: &[u8]) -> u16 {
    assert_eq!(&frame[..6], b"KMLS\0\x03");
    let mut d = Decoder::new(&frame[12..]);
    assert_eq!(d.map().unwrap(), Some(3));
    assert_eq!((d.u8().unwrap(), d.u16().unwrap()), (0, 3));
    assert_eq!(d.u8().unwrap(), 1);
    assert!(!d.bool().unwrap());
    assert_eq!(d.u8().unwrap(), 3);
    assert_eq!(d.map().unwrap(), Some(1));
    assert_eq!(d.u8().unwrap(), 3);
    let code = d.u16().unwrap();
    assert_eq!(d.position(), frame.len() - 12);
    code
}

#[test]
#[ignore = "exports synthetic private test state for the bounded platform ABI harness"]
fn export_frame3_adapter_fixture() {
    let out = std::path::PathBuf::from(std::env::var("MLS_N3B_ADAPTER_FIXTURES").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let (a, b) = pair(4);
    let c = peer(2, 4);
    let p = held(&a, &b, &c, 33);
    let mut req = request(&a, &p, None);
    let frame = Zeroizing::new(v3::encode_native_receive_request_v3(&req).unwrap());
    let actual = Zeroizing::new(v3::execute_native_receive_v3(&frame));
    let direct = ext::receive(
        context(&a),
        p.info.commit.clone(),
        b"N2 opaque authenticated external AAD4".to_vec(),
        accepted(&p),
        None,
    )
    .unwrap();
    assert!(!direct.storage_batch.upserts.is_empty());
    assert_transition(&actual, direct, false);
    std::fs::write(out.join("external_success.request.bin"), &*frame).unwrap();
    std::fs::write(out.join("external_success.response.bin"), &*actual).unwrap();
    let v3::NativeReceiveRequestV3::External(ref mut r) = req else {
        panic!()
    };
    r.expected_base_group_state_sha256[0] ^= 1;
    let frame = Zeroizing::new(v3::encode_native_receive_request_v3(&req).unwrap());
    let actual = Zeroizing::new(v3::execute_native_receive_v3(&frame));
    assert_eq!(error(&actual), Error::BaseStateMismatch as u16);
    std::fs::write(out.join("external_wrong_base.request.bin"), &*frame).unwrap();
    std::fs::write(out.join("external_wrong_base.response.bin"), &*actual).unwrap();
    println!(
        "exported one successful real external batch and one exact base-mismatch failure; synthetic private test state only"
    );
}
fn assert_transition(frame: &[u8], expected_result: ProcessExternalCommitResult, pending: bool) {
    assert_eq!(&frame[..8], b"KMLS\0\x03\x04\0");
    let mut d = Decoder::new(&frame[12..]);
    assert_eq!(d.map().unwrap(), Some(3));
    assert_eq!((d.u8().unwrap(), d.u16().unwrap()), (0, 3));
    assert_eq!(d.u8().unwrap(), 1);
    assert!(!d.bool().unwrap());
    assert_eq!(d.u8().unwrap(), 2);
    assert_eq!(d.map().unwrap(), Some(2));
    assert_eq!(d.u8().unwrap(), 0);
    let start = d.position();
    d.skip().unwrap();
    let end = d.position();
    let result = v2::NativeReceiveSuccessV2::Commit {
        message_epoch: expected_result.previous_roster.epoch,
        sender: v2::mls_leaf_into_native(expected_result.authenticated_sender),
        previous_roster: v2::mls_roster_into_native(expected_result.previous_roster),
        resulting_roster: v2::mls_roster_into_native(expected_result.resulting_roster),
        resulting_group_state_sha256: expected_result.resulting_group_state_sha256,
        storage_batch: v2::mls_batch_into_native(expected_result.storage_batch),
        effective_retention: expected_result.effective_retention,
    };
    let mut wanted = Zeroizing::new(Vec::new());
    v2::encode_success(&mut Encoder::new(&mut *wanted), &result).unwrap();
    assert!(
        &frame[12 + start..12 + end] == wanted.as_slice(),
        "native transition/batch differs from typed N2 core"
    );
    assert_eq!(d.u8().unwrap(), 1);
    if pending {
        assert_eq!(d.datatype().unwrap(), minicbor::data::Type::Map);
        d.skip().unwrap();
    } else {
        d.null().unwrap();
    }
    assert_eq!(d.position(), frame.len() - 12);
}
#[test]
fn frame3_external_result_matches_n2_complete_batch_and_pending_loser() {
    let (mut a, b) = pair(4);
    let c = peer(2, 4);
    let p = held(&a, &b, &c, 33);
    for with_pending in [false, true] {
        let pending = if with_pending {
            let prepared = self_prepare(&a, 44);
            mutate(&mut a, &prepared.mutation);
            Some(prepared.binding)
        } else {
            None
        };
        let req = request(&a, &p, pending.clone());
        let frame = Zeroizing::new(v3::encode_native_receive_request_v3(&req).unwrap());
        let decoded = v3::decode_native_receive_request_v3(&frame).unwrap();
        let v3::NativeReceiveRequestV3::External(ref r) = decoded else {
            panic!()
        };
        assert_eq!(r.acceptance, accepted(&p));
        assert_eq!(r.expected_superseded_pending, pending);
        let round = Zeroizing::new(v3::encode_native_receive_request_v3(&decoded).unwrap());
        assert!(frame.as_slice() == round.as_slice());
        let direct = ext::receive(
            context(&a),
            p.info.commit.clone(),
            b"N2 opaque authenticated external AAD4".to_vec(),
            accepted(&p),
            pending,
        )
        .unwrap();
        let actual = Zeroizing::new(v3::execute_native_receive_v3(&frame));
        assert_transition(&actual, direct, with_pending);
    }
}
#[test]
fn frame3_failures_have_no_batch_or_plaintext_and_do_not_apply_state() {
    let (a, b) = pair(4);
    let c = peer(2, 4);
    let p = held(&a, &b, &c, 33);
    let before = context(&a).expected_base_sha256.clone();
    for (case, wanted) in [
        (0, Error::WireHashMismatch),
        (1, Error::AadMismatch),
        (2, Error::GroupMismatch),
        (3, Error::ResultingEpochMismatch),
        (4, Error::BaseStateMismatch),
        (5, Error::PreviousRosterMismatch),
        (6, Error::SenderMismatch),
        (7, Error::LimitExceeded),
        (8, Error::AadMismatch),
    ] {
        let mut req = request(&a, &p, None);
        let v3::NativeReceiveRequestV3::External(r) = &mut req else {
            panic!()
        };
        match case {
            0 => r.acceptance.binding.commit_sha256[0] ^= 1,
            1 => r.acceptance.binding.aad_sha256[0] ^= 1,
            2 => r.acceptance.resulting_state.group_id[0] ^= 1,
            3 => r.acceptance.resulting_state.epoch += 1,
            4 => r.expected_base_group_state_sha256[0] ^= 1,
            5 => r.acceptance.binding.previous_state.digest_sha256[0] ^= 1,
            6 => r.acceptance.binding.author.signature_public_key[0] ^= 1,
            7 => r.expected_aad.clear(),
            _ => {
                r.expected_aad.push(0);
                r.acceptance.binding.aad_sha256 = Sha256::digest(&r.expected_aad).to_vec();
            }
        }
        let frame = Zeroizing::new(v3::encode_unchecked_fixture(&req).unwrap());
        assert_eq!(
            error(&v3::execute_native_receive_v3(&frame)),
            wanted as u16,
            "case {case}"
        );
        assert_eq!(context(&a).expected_base_sha256, before);
    }
    let valid =
        Zeroizing::new(v3::encode_native_receive_request_v3(&request(&a, &p, None)).unwrap());
    for (offset, value, wanted) in [
        (5, 2, Error::UnsupportedContractVersion),
        (6, 6, Error::UnsupportedOperation),
        (7, 1, Error::InvalidFrame),
        (12, 0xbf, Error::NoncanonicalEncoding),
    ] {
        let mut bad = Zeroizing::new(valid.to_vec());
        bad[offset] = value;
        assert_eq!(error(&v3::execute_native_receive_v3(&bad)), wanted as u16);
    }
    let oversize = Zeroizing::new(vec![0; v2::NATIVE_RECEIVE_REQUEST_MAX_BYTES + 1]);
    assert_eq!(
        error(&v3::execute_native_receive_v3(&oversize)),
        Error::LimitExceeded as u16
    );
}
#[test]
fn frame3_member_operations_reuse_v2_results_without_changing_v2_frames() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../native/receive_v2/fixtures");
    for name in [
        "application_success",
        "application_historical",
        "commit_success",
        "welcome_success",
        "application_wrong_aad",
        "v1_frame_rejected",
    ] {
        let original =
            Zeroizing::new(std::fs::read(root.join(format!("{name}.request.bin"))).unwrap());
        let baseline =
            Zeroizing::new(std::fs::read(root.join(format!("{name}.response.bin"))).unwrap());
        assert!(
            v2::execute_native_receive_v2(&original).as_slice() == baseline.as_slice(),
            "v2 changed: {name}"
        );
        let mut upgraded = Zeroizing::new(original.to_vec());
        if upgraded[5] == 2 {
            upgraded[5] = 3;
        }
        let mut expected = Zeroizing::new(baseline.to_vec());
        assert_eq!(&expected[12..15], &[0xa3, 0, 2]);
        expected[5] = 3;
        expected[14] = 3;
        let actual = Zeroizing::new(v3::execute_native_receive_v3(&upgraded));
        assert!(
            actual.as_slice() == expected.as_slice(),
            "v3 member semantic drift: {name}"
        );
    }
}
fn hex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2));
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn uuid(v: &serde_json::Value) -> Vec<u8> {
    hex(&v.as_str().unwrap().replace('-', ""))
}
fn state_from_tuple(v: &serde_json::Value) -> MlsExpectedRosterStateV1 {
    MlsExpectedRosterStateV1 {
        group_id: hex(v[0].as_str().unwrap()),
        epoch: v[2].as_str().unwrap().parse().unwrap(),
        digest_sha256: hex(v[4].as_str().unwrap()),
    }
}
fn leaf_from_tuple(v: &serde_json::Value) -> MlsRosterLeafV1 {
    MlsRosterLeafV1 {
        leaf_index: v[0].as_str().unwrap().parse().unwrap(),
        credential_identity: hex(v[3].as_str().unwrap()),
        signature_public_key: hex(v[4].as_str().unwrap()),
    }
}
#[test]
#[ignore = "requires the separately owned frozen backend fixture, never a production JSON parser"]
fn frame3_carries_frozen_backend_aad_and_submission_context_exactly() {
    let path = std::env::var("MLS_N3B_BACKEND_FIXTURE").unwrap();
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    for (name, len, sha) in [
        (
            "delta",
            858,
            "5da013ed4890f35a943a09160723c70f4a7618f2390885cade5b7e6bb6cca83a",
        ),
        (
            "aad4",
            331,
            "9a1b607d081d23a17bfab90d373d7ba9ba09268c50e8c30e866687988e4a25ad",
        ),
        (
            "mailbox",
            4739,
            "89948881aa74785afe8e0bd5b64f6c18c7511528ad1409db3357ec1aa23da35a",
        ),
    ] {
        let b = hex(fixture[name]["expectedHex"].as_str().unwrap());
        assert_eq!(b.len(), len);
        assert_eq!(Sha256::digest(&b).to_vec(), hex(sha));
        bytes.push(b);
    }
    // These are supplied canonical bytes. This test does NOT reconstruct Chat's
    // delta/mailbox codecs or authenticate their business authority.
    let mailbox: serde_json::Value = serde_json::from_slice(&bytes[2]).unwrap();
    let auth = &mailbox[14][1];
    let previous = state_from_tuple(&auth[18]);
    let resulting = state_from_tuple(&auth[19]);
    let old_index: u32 = auth[13].as_str().unwrap().parse().unwrap();
    let old = auth[18][5]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l[0].as_str().unwrap().parse::<u32>().unwrap() == old_index)
        .unwrap();
    let source_index: u32 = mailbox[13][2].as_str().unwrap().parse().unwrap();
    let author = auth[19][5]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l[0].as_str().unwrap().parse::<u32>().unwrap() == source_index)
        .unwrap();
    let commit = b"public external Commit fixture, serialization only".to_vec();
    let acceptance = MlsExternalCommitAcceptanceV1 {
        binding: MlsExternalCommitBindingV1 {
            candidate_id: uuid(&auth[2]),
            incarnation_id: uuid(&auth[9]),
            transition: MlsTransitionContext {
                command_id: uuid(&auth[1]),
                context_sha256: hex(auth[17].as_str().unwrap()),
            },
            previous_state: previous.clone(),
            old_leaf: leaf_from_tuple(old),
            author: leaf_from_tuple(author),
            input_group_info_sha256: hex(auth[22].as_str().unwrap()),
            resulting_group_info_sha256: hex(auth[23].as_str().unwrap()),
            aad_sha256: hex(auth[25].as_str().unwrap()),
            commit_sha256: hex(auth[26].as_str().unwrap()),
        },
        resulting_state: resulting,
    };
    assert_eq!(
        acceptance.binding.transition.context_sha256,
        Sha256::digest(&bytes[0]).to_vec()
    );
    assert_eq!(
        acceptance.binding.transition.command_id,
        uuid(&fixture["mailbox"]["input"]["externalCommit"]["authorization"]["causationCommandId"])
    );
    let req = v3::NativeReceiveRequestV3::External(v3::NativeExternalReceiveRequestV3 {
        profile_id: 2,
        group_id: previous.group_id,
        incarnation_id: acceptance.binding.incarnation_id.clone(),
        commit,
        expected_aad: bytes[1].clone(),
        expected_base_group_state_sha256: vec![9; 32],
        expected_retention: 4,
        acceptance: acceptance.clone(),
        expected_superseded_pending: None,
        storage: v2::NativeStorageSnapshotV2 {
            storage_format_version: 1,
            entries: vec![],
        },
    });
    let frame = Zeroizing::new(v3::encode_native_receive_request_v3(&req).unwrap());
    let v3::NativeReceiveRequestV3::External(decoded) =
        v3::decode_native_receive_request_v3(&frame).unwrap()
    else {
        panic!()
    };
    assert_eq!(decoded.acceptance, acceptance);
    assert_eq!(decoded.expected_aad, bytes[1]);
    assert!(error(&v3::execute_native_receive_v3(&frame)) != 0); // synthetic bytes/empty private state are NOT valid MLS authorization.
}
