//! Focused N2 journeys, reusing the existing strict runtime peer fixtures.
mod native_frame_tests;
mod resource_tests;
use super::super::external as ext;
use super::*;
use crate::api::external_resync::*;
use std::io::{Read, Write};
struct Held {
    bytes: Zeroizing<Vec<u8>>,
    info: ExternalCommitCandidateInfo,
}
fn input(a: &Peer, old: &Peer, new: &Peer, n: u8) -> MlsExternalPreparationV1 {
    let own = a
        .roster
        .leaves
        .iter()
        .find(|l| l.credential_identity == a.id)
        .unwrap()
        .clone();
    let export = export_group_info(context(a), a.signer.to_vec(), own).unwrap();
    MlsExternalPreparationV1 {
        candidate_id: vec![n; 16],
        incarnation_id: vec![8; 16],
        transition: transition(n),
        local_lifecycle_context_sha256: vec![77; 32],
        input_group_info: export.group_info,
        input_group_info_sha256: export.group_info_sha256,
        expected_previous_state: expected(&a.roster),
        expected_snapshot_signer: export.signer,
        authorized_old_leaf: a
            .roster
            .leaves
            .iter()
            .find(|l| l.credential_identity == old.id)
            .unwrap()
            .clone(),
        authorized_new_credential_identity: new.id.clone(),
        authorized_new_signature_public_key: new.public.clone(),
        aad: b"N2 opaque authenticated external AAD4".to_vec(),
        retention: new.retention,
    }
}
fn held(a: &Peer, old: &Peer, new: &Peer, n: u8) -> Held {
    let result = ext::prepare(input(a, old, new, n), new.signer.to_vec()).unwrap();
    Held {
        bytes: Zeroizing::new(result.candidate_bytes),
        info: result.info,
    }
}
fn accepted(c: &Held) -> MlsExternalCommitAcceptanceV1 {
    MlsExternalCommitAcceptanceV1 {
        binding: c.info.binding.clone(),
        resulting_state: expected(&c.info.proposed_resulting_roster),
    }
}
fn target(p: &Peer) -> MlsExternalTargetContextV1 {
    MlsExternalTargetContextV1 {
        group_id: GID.to_vec(),
        incarnation_id: vec![8; 16],
        local_lifecycle_context_sha256: vec![77; 32],
        storage_entries: p.rows.0.clone(),
        storage_format_version: 1,
    }
}
fn promote(p: &mut Peer, c: &Held) {
    let v = ext::accept(
        target(p),
        c.bytes.to_vec(),
        c.info.candidate_sha256.clone(),
        accepted(c),
    )
    .unwrap();
    apply(p, &v.storage_batch);
    p.roster = v.resulting_roster;
    assert_eq!(
        context(p).expected_base_sha256,
        v.resulting_group_state_sha256
    );
}
fn external_receive(a: &mut Peer, c: &Held, superseded: Option<MlsPendingCommitBinding>) {
    let result = ext::receive(
        context(a),
        c.info.commit.clone(),
        b"N2 opaque authenticated external AAD4".to_vec(),
        accepted(c),
        superseded.clone(),
    )
    .unwrap();
    assert_eq!(result.superseded_pending, superseded);
    apply(a, &result.storage_batch);
    a.roster = result.resulting_roster;
    assert_eq!(
        context(a).expected_base_sha256,
        result.resulting_group_state_sha256
    );
}
fn app(from: &mut Peer, to: &mut Peer, payload: &[u8]) {
    let previous = expected(&from.roster);
    let author = from
        .roster
        .leaves
        .iter()
        .find(|l| l.credential_identity == from.id)
        .unwrap()
        .clone();
    let sent = send(
        context(from),
        from.signer.to_vec(),
        payload.to_vec(),
        AAD.to_vec(),
    )
    .unwrap();
    mutate(from, &sent.mutation);
    let result = receive(incoming(
        to,
        sent.ciphertext,
        MlsReceiveKind::Application,
        author,
        previous,
        expected(&to.roster),
    ))
    .unwrap();
    assert!(result.plaintext.as_deref() == Some(payload));
    mutate(to, &result.mutation);
}
#[test]
fn candidate_acceptance_restart_retry_and_same_identity_rotations() {
    let (mut a, b) = pair(4);
    let mut c = peer(2, 4);
    let base = context(&a).expected_base_sha256.clone();
    let candidate = held(&a, &b, &c, 31);
    assert_eq!(context(&a).expected_base_sha256, base);
    assert!(c.rows.0.is_empty());
    assert_eq!(
        candidate.info.binding.old_leaf.credential_identity,
        candidate.info.binding.author.credential_identity
    );
    assert_ne!(
        candidate.info.binding.old_leaf.signature_public_key,
        candidate.info.binding.author.signature_public_key
    );
    let inspected = ext::inspect(
        candidate.bytes.to_vec(),
        candidate.info.candidate_sha256.clone(),
    )
    .unwrap();
    assert!(inspected.commit == candidate.info.commit);
    assert_eq!(inspected.binding, candidate.info.binding);
    // Real fresh process parses only serialized candidate + independently retained hash.
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("api::group_e2ee::runtime::tests::external_tests::candidate_restart_child")
        .env("MLS_N2_RESTART_CHILD", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&candidate.info.candidate_sha256).unwrap();
        stdin.write_all(&candidate.bytes).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "fresh-process candidate validation failed"
    );
    // Canonical acceptance lost-reply retry while target absent returns exact state,
    // never regenerates a Commit or advances a sender ratchet.
    let first = ext::accept(
        target(&c),
        candidate.bytes.to_vec(),
        candidate.info.candidate_sha256.clone(),
        accepted(&candidate),
    )
    .unwrap();
    let retry = ext::accept(
        target(&c),
        candidate.bytes.to_vec(),
        candidate.info.candidate_sha256.clone(),
        accepted(&candidate),
    )
    .unwrap();
    assert_eq!(
        first.resulting_group_state_sha256,
        retry.resulting_group_state_sha256
    );
    assert_eq!(
        first.storage_batch.upserts.len(),
        retry.storage_batch.upserts.len()
    );
    assert!(
        first
            .storage_batch
            .upserts
            .iter()
            .zip(&retry.storage_batch.upserts)
            .all(|(a, b)| a.key == b.key && a.value == b.value)
    );
    promote(&mut c, &candidate);
    external_receive(&mut a, &candidate, None);
    app(&mut c, &mut a, b"replacement to survivor");
    app(&mut a, &mut c, b"survivor to replacement");
    let advanced = context(&c).expected_base_sha256.clone();
    assert!(matches!(
        ext::accept(
            target(&c),
            candidate.bytes.to_vec(),
            candidate.info.candidate_sha256.clone(),
            accepted(&candidate)
        ),
        Err(Error::ExternalTargetStateExists)
    ));
    assert_eq!(context(&c).expected_base_sha256, advanced);
    let mut d = peer(2, 4);
    let second = held(&a, &c, &d, 32);
    assert_eq!(second.info.binding.old_leaf.signature_public_key, c.public);
    promote(&mut d, &second);
    external_receive(&mut a, &second, None);
    app(&mut d, &mut a, b"second stable-identity replacement");
    app(&mut a, &mut d, b"second reply");
}
#[test]
fn candidate_restart_child() {
    if std::env::var_os("MLS_N2_RESTART_CHILD").is_none() {
        return;
    }
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take((12 * 1024 * 1024 + 33) as u64)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() > 32);
    let expected = bytes[..32].to_vec();
    let result = ext::inspect(bytes[32..].to_vec(), expected).unwrap();
    assert_eq!(
        result.binding.old_leaf.credential_identity,
        result.binding.author.credential_identity
    );
    assert_ne!(
        result.binding.old_leaf.signature_public_key,
        result.binding.author.signature_public_key
    );
}
#[test]
fn candidates_fail_closed_and_never_overwrite_unrelated_state() {
    let (a, b) = pair(4);
    let mut c = peer(2, 4);
    let p = held(&a, &b, &c, 33);
    // Self-hashes here reach malformed parsing only; real callers retain the
    // independent package-issued hash, not a recomputed hash of unknown bytes.
    let mut d = minicbor::Decoder::new(&p.bytes);
    assert_eq!(d.array().unwrap(), Some(12));
    for _ in 0..11 {
        d.skip().unwrap();
    }
    let mut too_many = Zeroizing::new(p.bytes[..d.position()].to_vec());
    minicbor::Encoder::new(&mut *too_many).array(4097).unwrap();
    assert!(matches!(
        ext::inspect(too_many.to_vec(), Sha256::digest(&*too_many).to_vec()),
        Err(Error::LimitExceeded)
    ));
    let mut noncanonical = Zeroizing::new(p.bytes.to_vec());
    noncanonical.splice(0..1, [0x98, 12]);
    assert!(matches!(
        ext::inspect(
            noncanonical.to_vec(),
            Sha256::digest(&*noncanonical).to_vec()
        ),
        Err(Error::NoncanonicalEncoding)
    ));
    let mut corrupted = Zeroizing::new(p.bytes.to_vec());
    *corrupted.last_mut().unwrap() ^= 1;
    assert!(matches!(
        ext::inspect(corrupted.to_vec(), p.info.candidate_sha256.clone()),
        Err(Error::ExternalCandidateMismatch)
    ));
    assert!(matches!(
        ext::inspect(vec![0], Sha256::digest([0]).to_vec()),
        Err(Error::ExternalCandidateInvalid)
    ));
    for kind in 0..8 {
        let mut receipt = accepted(&p);
        match kind {
            0 => receipt.binding.candidate_id[0] ^= 1,
            1 => receipt.binding.transition.context_sha256[0] ^= 1,
            2 => receipt.binding.input_group_info_sha256[0] ^= 1,
            3 => receipt.binding.resulting_group_info_sha256[0] ^= 1,
            4 => receipt.binding.author.signature_public_key[0] ^= 1,
            5 => receipt.binding.previous_state.epoch += 1,
            6 => receipt.resulting_state.digest_sha256[0] ^= 1,
            _ => receipt.binding.aad_sha256[0] ^= 1,
        }
        assert!(matches!(
            ext::accept(
                target(&c),
                p.bytes.to_vec(),
                p.info.candidate_sha256.clone(),
                receipt
            ),
            Err(Error::ExternalAcceptanceMismatch)
        ));
    }
    let mut t = target(&c);
    t.group_id[0] ^= 1;
    assert!(matches!(
        ext::accept(
            t,
            p.bytes.to_vec(),
            p.info.candidate_sha256.clone(),
            accepted(&p)
        ),
        Err(Error::GroupMismatch)
    ));
    let mut t = target(&c);
    t.local_lifecycle_context_sha256[0] ^= 1;
    assert!(matches!(
        ext::accept(
            t,
            p.bytes.to_vec(),
            p.info.candidate_sha256.clone(),
            accepted(&p)
        ),
        Err(Error::ExternalLifecycleMismatch)
    ));
    assert!(matches!(
        ext::prepare(input(&a, &b, &c, 40), a.signer.to_vec()),
        Err(Error::InvalidSigner)
    ));
    let mut bad = input(&a, &b, &c, 40);
    bad.expected_previous_state.epoch += 1;
    assert!(matches!(
        ext::prepare(bad, c.signer.to_vec()),
        Err(Error::PreviousEpochMismatch)
    ));
    c.rows.0.push(MlsStorageEntry {
        key: b"KeyPackage-unrelated".to_vec(),
        value: vec![91; 16],
        group_id: None,
    });
    let target_snapshot = target(&c);
    c.rows.0.push(MlsStorageEntry {
        key: b"OtherGroup-row".to_vec(),
        value: vec![92; 16],
        group_id: Some(vec![99; 16]),
    });
    // Caller loads only this target+global scope. Supplying another scope rejects.
    assert!(matches!(
        ext::accept(
            target(&c),
            p.bytes.to_vec(),
            p.info.candidate_sha256.clone(),
            accepted(&p)
        ),
        Err(Error::InvalidStorageSnapshot)
    ));
    let promoted = ext::accept(
        target_snapshot,
        p.bytes.to_vec(),
        p.info.candidate_sha256.clone(),
        accepted(&p),
    )
    .unwrap();
    assert!(promoted.storage_batch.deletes.is_empty());
    assert!(
        promoted
            .storage_batch
            .upserts
            .iter()
            .all(|r| r.group_id.as_deref() == Some(GID) || r.group_id.is_none())
    );
    apply(&mut c, &promoted.storage_batch);
    assert!(
        c.rows
            .0
            .iter()
            .any(|r| r.key == b"KeyPackage-unrelated" && r.value == vec![91; 16])
    );
    assert!(
        c.rows
            .0
            .iter()
            .any(|r| r.key == b"OtherGroup-row" && r.value == vec![92; 16])
    );
}
#[test]
fn external_winner_discards_only_exact_internal_loser_and_keeps_old_ratchets() {
    let (mut a, mut b) = pair(4);
    let mut c = peer(2, 4);
    let candidate = held(&a, &b, &c, 34);
    let old_state = expected(&b.roster);
    let old_author = b.roster.leaves[1].clone();
    let first = send(
        context(&b),
        b.signer.to_vec(),
        b"old one".to_vec(),
        AAD.to_vec(),
    )
    .unwrap();
    mutate(&mut b, &first.mutation);
    let second = send(
        context(&b),
        b.signer.to_vec(),
        b"old two".to_vec(),
        AAD.to_vec(),
    )
    .unwrap();
    mutate(&mut b, &second.mutation);
    let pending = self_prepare(&a, 35);
    mutate(&mut a, &pending.mutation);
    let seen = receive(incoming(
        &a,
        first.ciphertext.clone(),
        MlsReceiveKind::Application,
        old_author.clone(),
        old_state.clone(),
        expected(&a.roster),
    ))
    .unwrap();
    mutate(&mut a, &seen.mutation);
    let before = context(&a).expected_base_sha256.clone();
    assert!(matches!(
        ext::receive(
            context(&a),
            candidate.info.commit.clone(),
            b"N2 opaque authenticated external AAD4".to_vec(),
            accepted(&candidate),
            None
        ),
        Err(Error::PendingBindingMismatch)
    ));
    let mut bad_receipt = accepted(&candidate);
    bad_receipt.resulting_state.epoch += 1;
    assert!(matches!(
        ext::receive(
            context(&a),
            candidate.info.commit.clone(),
            b"N2 opaque authenticated external AAD4".to_vec(),
            bad_receipt,
            Some(pending.binding.clone())
        ),
        Err(Error::ResultingEpochMismatch)
    ));
    assert_eq!(context(&a).expected_base_sha256, before);
    assert!(inspect_pending(context(&a)).unwrap().is_some());
    external_receive(&mut a, &candidate, Some(pending.binding));
    promote(&mut c, &candidate);
    assert!(inspect_pending(context(&a)).unwrap().is_none());
    assert!(matches!(
        receive(incoming(
            &a,
            first.ciphertext,
            MlsReceiveKind::Application,
            old_author.clone(),
            old_state.clone(),
            expected(&a.roster)
        )),
        Err(Error::Replay)
    ));
    let old = receive(incoming(
        &a,
        second.ciphertext,
        MlsReceiveKind::Application,
        old_author,
        old_state,
        expected(&a.roster),
    ))
    .unwrap();
    assert!(old.plaintext.as_deref() == Some(b"old two"));
    mutate(&mut a, &old.mutation);
    app(&mut c, &mut a, b"new sender after losing pending");
}
#[test]
fn internal_winner_and_wrong_external_authority_do_not_mutate() {
    let (mut a, b) = pair(4);
    let c = peer(2, 4);
    let candidate = held(&a, &b, &c, 36);
    let base = context(&a).expected_base_sha256.clone();
    let mut wrong = accepted(&candidate);
    wrong.binding.author.signature_public_key[0] ^= 1;
    assert!(matches!(
        ext::receive(
            context(&a),
            candidate.info.commit.clone(),
            b"N2 opaque authenticated external AAD4".to_vec(),
            wrong,
            None
        ),
        Err(Error::SenderMismatch)
    ));
    assert!(matches!(
        ext::receive(
            context(&a),
            candidate.info.commit.clone(),
            b"wrong".to_vec(),
            accepted(&candidate),
            None
        ),
        Err(Error::AadMismatch)
    ));
    assert!(matches!(
        ext::receive(
            context(&b),
            candidate.info.commit.clone(),
            b"N2 opaque authenticated external AAD4".to_vec(),
            accepted(&candidate),
            None
        ),
        Err(Error::ExternalOwnCommit)
    ));
    assert_eq!(context(&a).expected_base_sha256, base);
    let pending = self_prepare(&a, 37);
    mutate(&mut a, &pending.mutation);
    let winning = merge(context(&a), acceptance(&pending)).unwrap();
    mutate(&mut a, &winning);
    let advanced = context(&a).expected_base_sha256.clone();
    assert!(matches!(
        ext::receive(
            context(&a),
            candidate.info.commit.clone(),
            b"N2 opaque authenticated external AAD4".to_vec(),
            accepted(&candidate),
            None
        ),
        Err(Error::PreviousEpochMismatch)
    ));
    assert_eq!(context(&a).expected_base_sha256, advanced);
}
