use super::Error;
use super::*;
use crate::api::group_e2ee::{MlsRosterLeafV1, mls_roster_digest_v1};
use openmls::prelude::{tls_codec::*, *};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto as Provider;
use openmls_traits::OpenMlsProvider;
use sha2::{Digest, Sha256};
const CS: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
fn h(b: &[u8]) -> Vec<u8> {
    Sha256::digest(b).to_vec()
}
fn keys(n: u8) -> (SignatureKeyPair, CredentialWithKey) {
    let k = SignatureKeyPair::new(CS.signature_algorithm()).unwrap();
    let c = CredentialWithKey {
        credential: BasicCredential::new(vec![n; 45]).into(),
        signature_key: k.public().into(),
    };
    (k, c)
}
fn cfg() -> MlsGroupCreateConfig {
    MlsGroupCreateConfig::builder()
        .ciphersuite(CS)
        .use_ratchet_tree_extension(true)
        .build()
}
fn request(g: &MlsGroup, p: &impl OpenMlsProvider, k: &SignatureKeyPair) -> SnapshotRequest {
    let group_info = g
        .export_group_info(p.crypto(), k, true)
        .unwrap()
        .tls_serialize_detached()
        .unwrap();
    let mut leaves: Vec<_> = g
        .members()
        .map(|m| MlsRosterLeafV1 {
            leaf_index: m.index.u32(),
            credential_identity: BasicCredential::try_from(m.credential)
                .unwrap()
                .identity()
                .to_vec(),
            signature_public_key: m.signature_key,
        })
        .collect();
    leaves.sort_by_key(|l| l.leaf_index);
    let owner = leaves
        .iter()
        .find(|l| l.leaf_index == g.own_leaf_index().u32())
        .unwrap();
    SnapshotRequest {
        profile: PROFILE,
        sha256: h(&group_info),
        group_info,
        expected: ExpectedState {
            group_id: g.group_id().as_slice().to_vec(),
            epoch: g.epoch().as_u64(),
            roster_sha256: mls_roster_digest_v1(
                g.group_id().as_slice().to_vec(),
                g.epoch().as_u64(),
                leaves.clone(),
            )
            .unwrap(),
        },
        expected_signer: Leaf {
            index: owner.leaf_index,
            identity: owner.credential_identity.clone(),
            signature_key: owner.signature_public_key.clone(),
        },
    }
}
fn fixture() -> SuccessorRequest {
    let p = Provider::default();
    let (k, c) = keys(1);
    let mut g =
        MlsGroup::new_with_group_id(&p, &k, &cfg(), GroupId::from_slice(&[7; 16]), c).unwrap();
    // Commit-origin leaf avoids time-dependent lifetime in persistent public vectors.
    g.self_update(&p, &k, LeafNodeParameters::default())
        .unwrap();
    g.merge_pending_commit(&p).unwrap();
    let base = request(&g, &p, &k);
    let old = base.expected_signer.clone();
    successor(base, old)
}
fn successor(base: SnapshotRequest, old: Leaf) -> SuccessorRequest {
    let p2 = Provider::default();
    let (k2, mut c2) = keys(2);
    let new_identity = old.identity.clone();
    c2.credential = BasicCredential::new(new_identity.clone()).into();
    let aad = b"synthetic-authorized-input-snapshot-and-delta".to_vec();
    let MlsMessageBodyIn::GroupInfo(info) = MlsMessageIn::tls_deserialize_exact(&base.group_info)
        .unwrap()
        .extract()
    else {
        panic!()
    };
    let (_, bundle) = MlsGroup::external_commit_builder()
        .with_resync_leaf(LeafNodeIndex::new(old.index))
        .with_config(
            MlsGroupJoinConfig::builder()
                .use_ratchet_tree_extension(true)
                .build(),
        )
        .with_aad(aad.clone())
        .build_group(&p2, info, c2)
        .unwrap()
        .load_psks(p2.storage())
        .unwrap()
        .build(p2.rand(), p2.crypto(), &k2, |_| true)
        .unwrap()
        .finalize(&p2)
        .unwrap();
    let (commit, _, next) = bundle.into_messages();
    let commit = commit.tls_serialize_detached().unwrap();
    let next = next.unwrap().tls_serialize_detached().unwrap();
    SuccessorRequest {
        base,
        commit_sha256: h(&commit),
        commit,
        expected_aad: aad,
        authorized_old: old,
        authorized_new_identity: new_identity,
        authorized_new_signature_key: k2.public().to_vec(),
        reported_next_sha256: h(&next),
        reported_next_group_info: next,
    }
}
fn code(frame: &[u8]) -> u16 {
    assert_eq!(&frame[..4], b"KMPV");
    let mut d = minicbor::Decoder::new(&frame[12..]);
    assert_eq!(d.array().unwrap(), Some(3));
    d.bytes().unwrap();
    let c = d.u16().unwrap();
    if c != 0 {
        d.null().unwrap();
        assert_eq!(d.position(), frame.len() - 12);
    }
    c
}
fn followup(r: &SuccessorRequest) -> SuccessorRequest {
    let v = public::validate_external_successor(r).unwrap().resulting;
    let base = SnapshotRequest {
        profile: PROFILE,
        group_info: r.reported_next_group_info.clone(),
        sha256: r.reported_next_sha256.clone(),
        expected: v.roster.state,
        expected_signer: v.signer.clone(),
    };
    successor(base, v.signer)
}
// Both retained leaves are Commit-origin, so checked-in fixtures do not expire.
// The first replacement removes the sole KeyPackage-origin leaf normally; no
// clock/lifetime policy override is used to build or replay these vectors.
fn multileaf_fixture() -> SuccessorRequest {
    let p = Provider::default();
    let (k, c) = keys(1);
    let mut g =
        MlsGroup::new_with_group_id(&p, &k, &cfg(), GroupId::from_slice(&[7; 16]), c).unwrap();
    g.self_update(&p, &k, LeafNodeParameters::default())
        .unwrap();
    g.merge_pending_commit(&p).unwrap();
    let other = Provider::default();
    let (s, c) = keys(3);
    let kp = KeyPackage::builder().build(CS, &other, &s, c).unwrap();
    g.add_members(&p, &k, &[kp.key_package().clone()]).unwrap();
    g.merge_pending_commit(&p).unwrap();
    let base = request(&g, &p, &k);
    let old = Leaf {
        index: 1,
        identity: vec![3; 45],
        signature_key: s.public().to_vec(),
    };
    followup(&successor(base, old))
}
#[test]
fn stable_credential_survives_consecutive_fresh_key_replacements() {
    let first = fixture();
    let second = followup(&first);
    assert_eq!(first.authorized_old.identity, first.authorized_new_identity);
    assert_eq!(
        second.authorized_old.identity,
        first.authorized_old.identity
    );
    assert_eq!(
        second.authorized_new_identity,
        first.authorized_old.identity
    );
    assert_ne!(
        first.authorized_old.signature_key,
        first.authorized_new_signature_key
    );
    assert_eq!(
        second.authorized_old.signature_key,
        first.authorized_new_signature_key
    );
    assert_ne!(
        second.authorized_new_signature_key,
        first.authorized_new_signature_key
    );
    let v = public::validate_external_successor(&second).unwrap();
    assert_eq!(
        v.resulting.roster.state.epoch,
        first.base.expected.epoch + 2
    );
    let mut reused = second.clone();
    reused.authorized_new_signature_key = second.authorized_old.signature_key;
    assert_eq!(
        public::validate_external_successor(&reused),
        Err(Error::IdentityMismatch)
    );
}
#[test]
fn strict_snapshot_and_successor() {
    let r = fixture();
    assert_eq!(
        public::validate_snapshot(&r.base).unwrap().roster.state,
        r.base.expected
    );
    let v = public::validate_external_successor(&r).unwrap();
    assert_eq!(v.resulting.roster.state.epoch, r.base.expected.epoch + 1);
    assert_eq!(
        v.resulting.signer.signature_key,
        r.authorized_new_signature_key
    );
    let mut bad = r.clone();
    bad.authorized_new_signature_key[0] ^= 1;
    assert_eq!(
        public::validate_external_successor(&bad),
        Err(Error::SignerMismatch)
    );
    let mut bad = r.clone();
    bad.authorized_old.signature_key[0] ^= 1;
    assert_eq!(
        public::validate_external_successor(&bad),
        Err(Error::IdentityMismatch)
    );
    let mut bad = r.clone();
    bad.expected_aad.push(0);
    assert_eq!(
        public::validate_external_successor(&bad),
        Err(Error::AadMismatch)
    );
    let mut bad = r.clone();
    bad.reported_next_group_info = r.base.group_info.clone();
    bad.reported_next_sha256 = h(&bad.reported_next_group_info);
    assert!(public::validate_external_successor(&bad).is_err());
    let mut bad = r.base.clone();
    *bad.group_info.last_mut().unwrap() ^= 1;
    bad.sha256 = h(&bad.group_info);
    assert_eq!(
        public::validate_snapshot(&bad),
        Err(Error::SignatureRejected)
    );
}
#[test]
fn codec_is_canonical_fail_closed_and_request_bound() {
    let r = codec::Request::Successor(Box::new(fixture()));
    let f = codec::encode_request(&r).unwrap();
    assert_eq!(codec::decode_request(&f).unwrap(), r);
    let out = codec::execute(&f);
    assert_eq!(code(&out), 0);
    let mut d = minicbor::Decoder::new(&out[12..]);
    d.array().unwrap();
    assert_eq!(d.bytes().unwrap(), h(&f));
    let mut bad = f.clone();
    bad.push(0);
    assert_eq!(code(&codec::execute(&bad)), Error::InvalidFrame as u16);
    let mut bad = f.clone();
    bad[4] = 1;
    assert_eq!(
        code(&codec::execute(&bad)),
        Error::UnsupportedVersion as u16
    );
    let mut bad = f.clone();
    bad[6] = 3;
    assert_eq!(
        code(&codec::execute(&bad)),
        Error::UnsupportedOperation as u16
    );
    let mut snapshot = codec::encode_request(&codec::Request::Snapshot(fixture().base)).unwrap();
    assert_eq!(snapshot[13], 2);
    snapshot.splice(13..14, [0x18, 2]);
    let n = (snapshot.len() - 12) as u32;
    snapshot[8..12].copy_from_slice(&n.to_be_bytes());
    assert_eq!(
        code(&codec::execute(&snapshot)),
        Error::NoncanonicalEncoding as u16
    );
    assert_eq!(
        code(&codec::execute(&vec![0; codec::REQUEST_MAX_BYTES + 1])),
        Error::LimitExceeded as u16
    );
}
#[test]
fn bounds_and_expiry_are_not_bypassed() {
    let r = fixture();
    let mut bad = r.base.clone();
    bad.group_info.resize(MAX_MLS_BYTES + 1, 0);
    assert_eq!(public::validate_snapshot(&bad), Err(Error::LimitExceeded));
    let p = Provider::default();
    let (k, c) = keys(8);
    let expired =
        Lifetime::tls_deserialize_exact([0u64.to_be_bytes(), 1u64.to_be_bytes()].concat()).unwrap();
    let config = MlsGroupCreateConfig::builder()
        .ciphersuite(CS)
        .use_ratchet_tree_extension(true)
        .lifetime(expired)
        .build();
    let g = MlsGroup::new_with_group_id(&p, &k, &config, GroupId::from_slice(&[8; 16]), c).unwrap();
    assert_eq!(
        public::validate_snapshot(&request(&g, &p, &k)),
        Err(Error::LifetimeRejected)
    );
}
#[test]
fn export_interop_vectors_when_requested() {
    if std::env::var_os("MLS_EXPORT_PUBLIC_VALIDATOR").is_none() {
        return;
    }
    let r = fixture();
    let mut cases = vec![
        ("snapshot_ok", codec::Request::Snapshot(r.base.clone()), 0),
        (
            "external_successor_ok",
            codec::Request::Successor(Box::new(r.clone())),
            0,
        ),
    ];
    cases.push((
        "stable_credential_second_replacement",
        codec::Request::Successor(Box::new(followup(&r))),
        0,
    ));
    let multi = multileaf_fixture();
    let previous = public::validate_snapshot(&multi.base).unwrap();
    let retained = previous
        .roster
        .leaves
        .iter()
        .find(|l| **l != multi.authorized_old)
        .unwrap();
    cases.push((
        "stable_credential_multileaf",
        codec::Request::Successor(Box::new(multi.clone())),
        0,
    ));
    let mut collision = multi.clone();
    collision.authorized_new_identity = retained.identity.clone();
    cases.push((
        "unchanged_identity_collision",
        codec::Request::Successor(Box::new(collision)),
        Error::IdentityMismatch as u16,
    ));
    let mut collision = multi;
    collision.authorized_new_signature_key = retained.signature_key.clone();
    cases.push((
        "unchanged_key_collision",
        codec::Request::Successor(Box::new(collision)),
        Error::IdentityMismatch as u16,
    ));
    let mut reused = r.clone();
    reused.authorized_new_signature_key = r.authorized_old.signature_key.clone();
    cases.push((
        "reused_old_key",
        codec::Request::Successor(Box::new(reused)),
        Error::IdentityMismatch as u16,
    ));
    let mut bad = r.clone();
    bad.authorized_new_signature_key[0] ^= 1;
    cases.push((
        "wrong_new_key",
        codec::Request::Successor(Box::new(bad)),
        Error::SignerMismatch as u16,
    ));
    let mut bad = r.clone();
    bad.authorized_old.index += 1;
    cases.push((
        "wrong_old_leaf",
        codec::Request::Successor(Box::new(bad)),
        Error::IdentityMismatch as u16,
    ));
    let mut bad = r.clone();
    bad.expected_aad.clear();
    cases.push((
        "empty_aad",
        codec::Request::Successor(Box::new(bad)),
        Error::LimitExceeded as u16,
    ));
    let mut bad = r.base.clone();
    *bad.group_info.last_mut().unwrap() ^= 1;
    bad.sha256 = h(&bad.group_info);
    cases.push((
        "bad_signature",
        codec::Request::Snapshot(bad),
        Error::SignatureRejected as u16,
    ));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../native/public_validator_v1/fixtures");
    std::fs::create_dir_all(&dir).unwrap();
    let mut manifest = Vec::new();
    for (name, r, expected) in cases {
        let request = codec::encode_request(&r).unwrap();
        let response = codec::execute(&request);
        assert_eq!(code(&response), expected, "{name}");
        std::fs::write(dir.join(format!("{name}.request.bin")), &request).unwrap();
        std::fs::write(dir.join(format!("{name}.response.bin")), &response).unwrap();
        manifest.push(serde_json::json!({"name":name,"code":expected,"requestSha256":hex_hash(&request),"responseSha256":hex_hash(&response)}));
    }
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}
fn hex_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
fn shared_public_vectors_replay() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../native/public_validator_v1/fixtures");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    let cases = manifest.as_array().unwrap();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let request = std::fs::read(dir.join(format!("{name}.request.bin"))).unwrap();
        let response = std::fs::read(dir.join(format!("{name}.response.bin"))).unwrap();
        assert_eq!(hex_hash(&request), case["requestSha256"].as_str().unwrap());
        assert_eq!(
            hex_hash(&response),
            case["responseSha256"].as_str().unwrap()
        );
        assert_eq!(codec::execute(&request), response, "{name}");
        assert_eq!(u64::from(code(&response)), case["code"].as_u64().unwrap());
    }
}
#[test]
fn sparse_multileaf_successor_preserves_unaffected_indices() {
    let p = Provider::default();
    let (k, c) = keys(1);
    let mut g =
        MlsGroup::new_with_group_id(&p, &k, &cfg(), GroupId::from_slice(&[7; 16]), c).unwrap();
    let mut packages = Vec::new();
    for n in 3..=5 {
        let other = Provider::default();
        let (s, c) = keys(n);
        packages.push(
            KeyPackage::builder()
                .build(CS, &other, &s, c)
                .unwrap()
                .key_package()
                .clone(),
        );
    }
    g.add_members(&p, &k, &packages).unwrap();
    g.merge_pending_commit(&p).unwrap();
    g.remove_members(&p, &k, &[LeafNodeIndex::new(1)]).unwrap();
    g.merge_pending_commit(&p).unwrap();
    let base = request(&g, &p, &k);
    let old_member = g.members().find(|m| m.index.u32() == 2).unwrap();
    let old = Leaf {
        index: 2,
        identity: BasicCredential::try_from(old_member.credential)
            .unwrap()
            .identity()
            .to_vec(),
        signature_key: old_member.signature_key,
    };
    let r = successor(base, old.clone());
    let v = public::validate_external_successor(&r).unwrap();
    assert_eq!(
        v.previous
            .roster
            .leaves
            .iter()
            .map(|l| l.index)
            .collect::<Vec<_>>(),
        vec![0, 2, 3]
    );
    assert_eq!(v.resulting.roster.leaves.len(), 3);
    for leaf in v.previous.roster.leaves.iter().filter(|l| **l != old) {
        assert!(v.resulting.roster.leaves.contains(leaf));
    }
    assert!(!v.resulting.roster.leaves.contains(&old));
    assert_eq!(v.resulting.signer.identity, old.identity);
    let mut collision = r.clone();
    collision.authorized_new_identity = v.previous.roster.leaves[0].identity.clone();
    assert_eq!(
        public::validate_external_successor(&collision),
        Err(Error::IdentityMismatch)
    );
    let mut collision = r.clone();
    collision.authorized_new_signature_key = v.previous.roster.leaves[0].signature_key.clone();
    assert_eq!(
        public::validate_external_successor(&collision),
        Err(Error::IdentityMismatch)
    );
    // Allocation is derived, not an input oracle; the earlier blank slot is reused.
    assert_eq!(v.resulting.signer.index, 1);
    let mut wrong = r.clone();
    wrong.authorized_old = v.previous.roster.leaves[2].clone();
    // With the wrong removal authority, the stable credential now collides
    // with an unchanged leaf; reject before inspecting the Commit's proposals.
    assert_eq!(
        public::validate_external_successor(&wrong),
        Err(Error::IdentityMismatch)
    );
}
#[test]
fn truncated_tls_and_malformed_cbor_fail_without_result() {
    let r = fixture();
    for bytes in [&r.base.group_info, &r.commit] {
        for n in [0, 1, 4, 8, bytes.len() / 2, bytes.len() - 1] {
            let result = if std::ptr::eq(bytes, &r.commit) {
                preflight::external_commit(&bytes[..n])
            } else {
                preflight::group_info(&bytes[..n])
            };
            assert!(result.is_err(), "truncation {n}");
        }
    }
    let frame = codec::encode_request(&codec::Request::Snapshot(r.base)).unwrap();
    let failure = |payload: &[u8]| {
        let mut f = frame[..12].to_vec();
        f[8..12].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        f.extend_from_slice(payload);
        assert_ne!(code(&codec::execute(&f)), 0);
    };
    for payload in [
        vec![0x9f, 0xff],
        vec![0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        vec![
            0x85, 2, 0x5b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ],
        vec![0x85, 2, 0x5f, 0xff],
    ] {
        failure(&payload);
    }
    for n in [0, 1, 4, 11, 12, frame.len() / 2, frame.len() - 1] {
        assert_ne!(code(&codec::execute(&frame[..n])), 0);
    }
}
