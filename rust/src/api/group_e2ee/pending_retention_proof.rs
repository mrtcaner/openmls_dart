//! Disposable issue #54 proof. No exported API, production profile or storage migration.
//! The file store is a serial test caller, NOT a SQLite/cross-process lock implementation.
use super::*;
use crate::api::keys::{MlsSignatureKeyPair, serialize_signer};
use crate::api::storage::{create_key_package_with_storage, legacy_create_message_with_storage};
use crate::api::types::MlsCiphersuite;
use openmls::framing::errors::{MessageDecryptionError, SecretTreeError};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use zeroize::Zeroizing;
mod measurement;

const GID: &[u8] = &[0x54; 16];
const AAD: &[u8] = b"proof/54/authenticated-aad";
type Row = (Vec<u8>, Vec<u8>, Option<Vec<u8>>);
type Leaf = (u32, Vec<u8>, Vec<u8>);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Roster {
    epoch: u64,
    leaves: Vec<Leaf>,
    digest: Vec<u8>,
}
impl Roster {
    fn from(v: &MlsRosterSummaryV1) -> Self {
        assert_eq!(v.group_id, GID);
        Self {
            epoch: v.epoch,
            leaves: v
                .leaves
                .iter()
                .map(|x| {
                    (
                        x.leaf_index,
                        x.credential_identity.clone(),
                        x.signature_public_key.clone(),
                    )
                })
                .collect(),
            digest: v.digest_sha256.clone(),
        }
    }
    fn expected(&self) -> MlsExpectedRosterStateV1 {
        MlsExpectedRosterStateV1 {
            group_id: GID.to_vec(),
            epoch: self.epoch,
            digest_sha256: self.digest.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Binding {
    command: u64,
    incarnation: Vec<u8>,
    author: Leaf,
    commit: Vec<u8>,
    welcome: Option<Vec<u8>>,
    group_info: Option<Vec<u8>>,
    wire_sha256: Vec<u8>,
    pending_sha256: Vec<u8>,
    preparation_digest: Vec<u8>,
    previous: Roster,
    resulting: Roster,
}
#[derive(Clone, Serialize, Deserialize)]
struct Store {
    revision: u64,
    rows: Vec<Row>,
    roster: Roster,
    history: BTreeMap<u64, Roster>,
    pending: Option<Binding>,
    applied: Option<Vec<u8>>,
}
impl Drop for Store {
    fn drop(&mut self) {
        for row in &mut self.rows {
            row.1.zeroize();
        }
    }
}
impl Store {
    fn entries(&self) -> Vec<MlsStorageEntry> {
        self.rows
            .iter()
            .map(|x| MlsStorageEntry {
                key: x.0.clone(),
                value: x.1.clone(),
                group_id: x.2.clone(),
            })
            .collect()
    }
    fn digest(&self) -> Vec<u8> {
        group_state_digest_from_entries(GID, &self.entries(), 1).unwrap()
    }
    fn apply(&mut self, b: &MlsStorageBatch) {
        assert_eq!(b.storage_format_version, 1);
        for row in &mut self.rows {
            if b.deletes.contains(&row.0)
                || row
                    .2
                    .as_ref()
                    .is_some_and(|g| b.deleted_group_ids.contains(g))
                || b.upserts.iter().any(|u| u.key == row.0)
            {
                row.1.zeroize();
            }
        }
        self.rows.retain(|r| {
            !b.deletes.contains(&r.0)
                && !r
                    .2
                    .as_ref()
                    .is_some_and(|g| b.deleted_group_ids.contains(g))
                && !b.upserts.iter().any(|u| u.key == r.0)
        });
        self.rows.extend(
            b.upserts
                .iter()
                .map(|u| (u.key.clone(), u.value.clone(), u.group_id.clone())),
        );
        self.rows.sort_by(|a, b| a.0.cmp(&b.0));
    }
    fn advance(&mut self, r: &Roster, retain: u32) {
        self.history.insert(self.roster.epoch, self.roster.clone());
        self.roster = r.clone();
        self.history
            .retain(|e, _| *e >= r.epoch.saturating_sub(retain as u64));
    }
}
struct Peer {
    signer: Zeroizing<Vec<u8>>,
    identity: Vec<u8>,
    public: Vec<u8>,
    store: Store,
}
fn suite() -> MlsCiphersuite {
    MlsCiphersuite::Mls128DhkemX25519Aes128gcmSha256Ed25519
}
fn config(retain: u32) -> MlsGroupConfig {
    assert!([0, 2, 4].contains(&retain));
    let mut c = MlsGroupConfig::default_config(suite());
    c.max_past_epochs = retain;
    c
}
fn blank() -> Store {
    Store {
        revision: 0,
        rows: vec![],
        roster: Roster {
            epoch: 0,
            leaves: vec![],
            digest: vec![],
        },
        history: BTreeMap::new(),
        pending: None,
        applied: None,
    }
}
fn peer(id: u8) -> Peer {
    let k = MlsSignatureKeyPair::generate(suite()).unwrap();
    let public = k.public_key();
    let signer =
        Zeroizing::new(serialize_signer(suite(), k.private_key(), public.clone()).unwrap());
    Peer {
        signer,
        identity: vec![id; 45],
        public,
        store: blank(),
    }
}
fn package(p: &mut Peer) -> MlsAuthorizedKeyPackageV1 {
    let k = create_key_package_with_storage(
        suite(),
        p.signer.to_vec(),
        p.identity.clone(),
        p.public.clone(),
        None,
        p.store.entries(),
        1,
    )
    .unwrap();
    p.store.apply(&k.storage_batch);
    MlsAuthorizedKeyPackageV1 {
        key_package_bytes: k.key_package_bytes,
        expected_credential_identity: p.identity.clone(),
        expected_signature_public_key: p.public.clone(),
    }
}
fn fixture(count: usize, retain: u32) -> Vec<Peer> {
    let mut peers: Vec<_> = (0..count).map(|i| peer((i + 1) as u8)).collect();
    let owner = &mut peers[0];
    let c = legacy_create_group_with_storage(
        config(retain),
        owner.signer.to_vec(),
        GID.to_vec(),
        MlsAuthorizedOwnerV1 {
            expected_credential_identity: owner.identity.clone(),
            expected_signature_public_key: owner.public.clone(),
        },
        None,
        vec![],
        1,
    )
    .unwrap();
    owner.store.apply(&c.storage_batch);
    owner.store.roster = Roster::from(&c.resulting_roster);
    let additions = peers.iter_mut().skip(1).map(package).collect();
    let p = prepare(&peers[0], 1, additions, vec![], false).unwrap();
    peers[0].store = p.clone();
    let binding = p.pending.clone().unwrap();
    peers[0].store = settle(&p, &p.digest(), &binding, accepted(&binding), retain).unwrap();
    for peer in peers.iter_mut().skip(1) {
        let j = legacy_join_group_from_welcome_with_storage(
            config(retain),
            binding.welcome.clone().unwrap(),
            None,
            peer.signer.to_vec(),
            binding.resulting.expected(),
            peer.store.entries(),
            1,
        )
        .unwrap();
        peer.store.apply(&j.storage_batch);
        peer.store.roster = Roster::from(&j.resulting_roster);
    }
    peers
}
#[derive(Debug, PartialEq, Eq)]
enum ProofError {
    PendingExists,
    MissingPending,
    BindingMismatch,
    AcceptanceMismatch,
    BaseMismatch,
    Authority,
    Protocol,
    PastEpochUnavailable,
    GenerationTooOld,
    Replay,
    ForwardDistance,
    Inactive,
    PendingControl,
    NativeLimit,
}
fn pending_hash(store: &Store) -> Result<Vec<u8>, ProofError> {
    // Bind the exact persisted GroupState bytes, not a reserialization of a
    // StagedCommit containing unordered proposal sets/maps. This is a local
    // integrity token, not a cross-platform canonical protocol digest.
    let row = store
        .rows
        .iter()
        .find(|r| r.0.starts_with(b"GroupState") && r.2.as_deref() == Some(GID))
        .ok_or(ProofError::MissingPending)?;
    Ok(Sha256::digest(&row.1).to_vec())
}
fn prepare(
    peer: &Peer,
    command: u64,
    additions: Vec<MlsAuthorizedKeyPackageV1>,
    removals: Vec<MlsAuthorizedRemovalV1>,
    self_update: bool,
) -> Result<Store, ProofError> {
    let old = &peer.store;
    if old.pending.is_some() {
        return Err(ProofError::PendingExists);
    }
    let provider =
        provider_from_entries(old.entries(), 1, Some(GID)).map_err(|_| ProofError::Protocol)?;
    let mut group = load_group(GID, &provider).map_err(|_| ProofError::Protocol)?;
    if group.pending_commit().is_some() {
        return Err(ProofError::PendingExists);
    }
    if !group.is_active() {
        return Err(ProofError::Inactive);
    }
    let signer = signer_from_bytes(peer.signer.to_vec()).unwrap();
    ensure_local_signer(&group, &signer).map_err(|_| ProofError::Authority)?;
    let previous = roster_from_group(&group).unwrap();
    validate_expected_roster(&previous, &old.roster.expected(), "previous")
        .map_err(|_| ProofError::Authority)?;
    let removal_indices =
        validate_removals(&previous, &removals).map_err(|_| ProofError::Authority)?;
    let (keys, requested) = validate_additions(&provider, additions, &previous, &removal_indices)
        .map_err(|_| ProofError::Authority)?;
    group.set_aad(AAD.to_vec());
    let bundle = if self_update {
        group
            .self_update(&provider, &signer, LeafNodeParameters::default())
            .map_err(|_| ProofError::Protocol)?
    } else {
        group
            .commit_builder()
            .consume_proposal_store(false)
            .propose_adds(keys)
            .propose_removals(removal_indices.iter().copied().map(LeafNodeIndex::new))
            .load_psks(provider.storage())
            .map_err(|_| ProofError::Protocol)?
            .build(provider.rand(), provider.crypto(), &signer, |_| true)
            .map_err(|_| ProofError::Protocol)?
            .stage_commit(&provider)
            .map_err(|_| ProofError::Protocol)?
    };
    assert!(group.pending_commit().is_some());
    let (commit, welcome, info) = bundle.into_messages();
    let commit = commit.tls_serialize_detached().unwrap();
    let mut candidate = old.clone();
    candidate.apply(&batch_from_provider(provider, Some(GID.to_vec()), vec![]).unwrap());
    let pending_sha256 = pending_hash(&candidate)?;
    // Preview in a disposable provider only. Durable rows remain UNMERGED.
    let preview = provider_from_entries(candidate.entries(), 1, Some(GID)).unwrap();
    let mut group = load_group(GID, &preview).unwrap();
    group
        .merge_pending_commit(&preview)
        .map_err(|_| ProofError::Protocol)?;
    let resulting = roster_from_group(&group).unwrap();
    if self_update {
        if previous.leaves != resulting.leaves || resulting.epoch != previous.epoch + 1 {
            return Err(ProofError::Authority);
        }
    } else {
        validate_exact_delta(&previous, &resulting, &removal_indices, &requested)
            .map_err(|_| ProofError::Authority)?;
    }
    candidate.pending = Some(Binding {
        command,
        incarnation: vec![0x19; 16],
        author: old
            .roster
            .leaves
            .iter()
            .find(|l| l.1 == peer.identity)
            .unwrap()
            .clone(),
        wire_sha256: Sha256::digest(&commit).to_vec(),
        commit,
        welcome: welcome.map(|v| v.tls_serialize_detached().unwrap()),
        group_info: info.map(|v| v.tls_serialize_detached().unwrap()),
        pending_sha256,
        preparation_digest: old.digest(),
        previous: Roster::from(&previous),
        resulting: Roster::from(&resulting),
    });
    Ok(candidate)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AcceptedAuthority {
    group_id: Vec<u8>,
    incarnation: Vec<u8>,
    command: u64,
    author: Leaf,
    commit_sha256: Vec<u8>,
    aad_sha256: Vec<u8>,
    welcome_sha256: Option<Vec<u8>>,
    group_info_sha256: Option<Vec<u8>>,
    previous: Roster,
    resulting: Roster,
}
// Simulates a server accepting exactly the submitted coordinates, then returning
// a separate authenticated receipt. No server signature/authentication is claimed.
fn receipt(b: &Binding) -> AcceptedAuthority {
    AcceptedAuthority {
        group_id: GID.to_vec(),
        incarnation: b.incarnation.clone(),
        command: b.command,
        author: b.author.clone(),
        commit_sha256: Sha256::digest(&b.commit).to_vec(),
        aad_sha256: Sha256::digest(AAD).to_vec(),
        welcome_sha256: b.welcome.as_ref().map(|x| Sha256::digest(x).to_vec()),
        group_info_sha256: b.group_info.as_ref().map(|x| Sha256::digest(x).to_vec()),
        previous: b.previous.clone(),
        resulting: b.resulting.clone(),
    }
}
fn accepted(b: &Binding) -> Decision {
    Decision::Accepted(Box::new(receipt(b)))
}
#[derive(Clone)]
enum Decision {
    Accepted(Box<AcceptedAuthority>),
    Rejected,
    Unknown,
}
fn settle(
    store: &Store,
    expected_base: &[u8],
    binding: &Binding,
    decision: Decision,
    retain: u32,
) -> Result<Store, ProofError> {
    if store.digest() != expected_base {
        return Err(ProofError::BaseMismatch);
    }
    let pending = store.pending.as_ref().ok_or(ProofError::MissingPending)?;
    if pending != binding || Sha256::digest(&binding.commit).as_slice() != binding.wire_sha256 {
        return Err(ProofError::BindingMismatch);
    }
    let p = provider_from_entries(store.entries(), 1, Some(GID)).unwrap();
    let mut g = load_group(GID, &p).unwrap();
    if !g.is_active() {
        return Err(ProofError::Inactive);
    }
    if g.pending_commit().is_none() {
        return Err(ProofError::MissingPending);
    }
    if pending_hash(store)? != binding.pending_sha256
        || Roster::from(&roster_from_group(&g).unwrap()) != binding.previous
    {
        return Err(ProofError::BindingMismatch);
    }
    if matches!(decision, Decision::Unknown) {
        return Ok(store.clone());
    }
    let mut next = store.clone();
    match decision {
        Decision::Accepted(authority) => {
            if *authority != receipt(binding) {
                return Err(ProofError::AcceptanceMismatch);
            }
            g.merge_pending_commit(&p)
                .map_err(|_| ProofError::Protocol)?;
            if Roster::from(&roster_from_group(&g).unwrap()) != binding.resulting {
                return Err(ProofError::Authority);
            }
            next.advance(&binding.resulting, retain);
            next.applied = Some(binding.wire_sha256.clone());
        }
        Decision::Rejected => g
            .clear_pending_commit(p.storage())
            .map_err(|_| ProofError::Protocol)?,
        Decision::Unknown => unreachable!(),
    }
    next.apply(&batch_from_provider(p, Some(GID.to_vec()), vec![]).unwrap());
    next.pending = None;
    Ok(next)
}
fn send(p: &mut Peer, payload: &[u8]) -> Vec<u8> {
    assert!(
        p.store.pending.is_none(),
        "caller pauses author send while pending"
    );
    let r = legacy_create_message_with_storage(
        GID.to_vec(),
        p.signer.to_vec(),
        payload.to_vec(),
        AAD.to_vec(),
        p.store.entries(),
        1,
    )
    .unwrap();
    p.store.apply(&r.storage_batch);
    r.ciphertext
}
fn receive_commit(store: &Store, b: &Binding, retain: u32) -> Result<Store, ProofError> {
    if store.pending.is_some() {
        return Err(ProofError::PendingControl);
    }
    let r = legacy_process_message_with_storage(
        GID.to_vec(),
        b.commit.clone(),
        AAD.to_vec(),
        store.roster.expected(),
        b.resulting.expected(),
        store.entries(),
        1,
    )
    .map_err(|_| ProofError::Protocol)?;
    let mut next = store.clone();
    if !r.has_staged_commit || r.sender_index != Some(b.author.0) {
        return Err(ProofError::Authority);
    }
    next.apply(&r.storage_batch);
    next.advance(&b.resulting, retain);
    Ok(next)
}
fn receive_application(
    store: &Store,
    wire: &[u8],
    aad: &[u8],
    sender: &Leaf,
    message_epoch: u64,
    retain: u32,
) -> Result<(Store, Vec<u8>, MlsStorageBatch), ProofError> {
    let p = provider_from_entries(store.entries(), 1, Some(GID)).unwrap();
    let mut g = load_group(GID, &p).unwrap();
    if !g.is_active() {
        return Err(ProofError::Inactive);
    }
    if g.configuration() != &config(retain).to_join_config() {
        return Err(ProofError::Authority);
    }
    validate_expected_roster(
        &roster_from_group(&g).unwrap(),
        &store.roster.expected(),
        "current",
    )
    .map_err(|_| ProofError::Authority)?;
    let message = mls_message_from_exact_bytes(wire)
        .map_err(|_| ProofError::Protocol)?
        .try_into_protocol_message()
        .map_err(|_| ProofError::Protocol)?;
    if message.epoch().as_u64() != message_epoch {
        return Err(ProofError::Authority);
    }
    // Upstream's debug build asserts on AEAD failure. Mirror the released
    // native adapter's unwind boundary; a failed temporary provider is dropped.
    let processed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        g.process_message(&p, message)
    }))
    .map_err(|_| ProofError::Protocol)?
    .map_err(|e| match e {
        ProcessMessageError::ValidationError(ValidationError::NoPastEpochData) => {
            ProofError::PastEpochUnavailable
        }
        ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
            MessageDecryptionError::SecretTreeError(SecretTreeError::TooDistantInThePast),
        )) => {
            // OpenMLS also uses this variant when the entire epoch is absent.
            // Distinguish from ratchet age using locally maintained authority,
            // never error-string matching or a claim of ciphertext authenticity.
            if message_epoch < store.roster.epoch && !store.history.contains_key(&message_epoch) {
                ProofError::PastEpochUnavailable
            } else {
                ProofError::GenerationTooOld
            }
        }
        ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
            MessageDecryptionError::SecretTreeError(SecretTreeError::TooDistantInTheFuture),
        )) => ProofError::ForwardDistance,
        ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
            MessageDecryptionError::SecretTreeError(SecretTreeError::SecretReuseError),
        )) => ProofError::Replay,
        _ => ProofError::Protocol,
    })?;
    let historical = if store.roster.epoch == message_epoch {
        Some(&store.roster)
    } else {
        store.history.get(&message_epoch)
    };
    let valid = processed.aad() == aad
        && processed.epoch().as_u64() == message_epoch
        && processed.sender() == &Sender::Member(LeafNodeIndex::new(sender.0))
        && BasicCredential::try_from(processed.credential().clone())
            .map(|c| c.identity() == sender.1)
            .unwrap_or(false)
        && historical.is_some_and(|r| r.leaves.contains(sender));
    if !valid {
        zeroize_processed_content(processed);
        return Err(ProofError::Authority);
    }
    let plaintext = match processed.into_content() {
        ProcessedMessageContent::ApplicationMessage(a) => a.into_bytes(),
        _ => return Err(ProofError::Protocol),
    };
    let batch = batch_from_provider(p, Some(GID.to_vec()), vec![]).unwrap();
    let mut next = store.clone();
    next.apply(&batch);
    Ok((next, plaintext, batch))
}
fn proof_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../.buildlog/pending-commit-retention-20260927")
}
fn read_store(path: &Path) -> Store {
    let bytes = Zeroizing::new(fs::read(path).unwrap());
    serde_json::from_slice(&bytes).unwrap()
}
fn persist(
    path: &Path,
    expected_revision: Option<u64>,
    mut next: Store,
    crash: Option<&str>,
) -> Result<(), ProofError> {
    if path.exists() {
        let old = read_store(path);
        if Some(old.revision) != expected_revision {
            return Err(ProofError::BaseMismatch);
        }
        next.revision = old.revision + 1;
    } else if expected_revision.is_some() {
        return Err(ProofError::BaseMismatch);
    }
    let temp = path.with_extension("staged");
    let bytes = Zeroizing::new(serde_json::to_vec(&next).unwrap());
    let mut file = File::create(&temp).unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    if crash == Some("before") {
        std::process::exit(73)
    }
    fs::rename(&temp, path).unwrap();
    File::open(path.parent().unwrap())
        .unwrap()
        .sync_all()
        .unwrap();
    if crash == Some("after") {
        std::process::exit(73)
    }
    Ok(())
}
#[test]
fn proof_child() {
    let Ok(mode) = std::env::var("MLS_PROOF_CHILD") else {
        return;
    };
    let target = PathBuf::from(std::env::var("MLS_PROOF_TARGET").unwrap());
    let next = read_store(&target.with_extension("next"));
    let old = read_store(&target);
    persist(&target, Some(old.revision), next, Some(&mode)).unwrap();
}
#[test]
fn binding_rejection_ratchet_limits_and_required_controls() {
    for retain in [2, 4] {
        let mut peers = fixture(4, retain);
        let p = prepare(&peers[0], 401, vec![], vec![], true).unwrap();
        let b = p.pending.clone().unwrap();
        for variant in 0..6 {
            let mut wrong = b.clone();
            match variant {
                0 => wrong.wire_sha256[0] ^= 1,
                1 => wrong.commit[0] ^= 1,
                2 => wrong.incarnation[0] ^= 1,
                3 => wrong.author.2[0] ^= 1,
                4 => wrong.resulting.digest[0] ^= 1,
                _ => wrong.group_info = Some(vec![1]),
            }
            assert!(matches!(
                settle(&p, &p.digest(), &wrong, accepted(&b), retain),
                Err(ProofError::BindingMismatch)
            ));
        }
        peers[0].store = settle(&p, &p.digest(), &b, Decision::Rejected, retain).unwrap();
        // Independently supplied response fields, not a second local Binding.
        let valid_receipt = receipt(&b);
        for variant in 0..3 {
            let mut wrong = valid_receipt.clone();
            match variant {
                0 => wrong.command += 1,
                1 => wrong.commit_sha256[0] ^= 1,
                _ => wrong.aad_sha256[0] ^= 1,
            }
            assert!(matches!(
                settle(
                    &p,
                    &p.digest(),
                    &b,
                    Decision::Accepted(Box::new(wrong)),
                    retain
                ),
                Err(ProofError::AcceptanceMismatch)
            ));
        }
        // Response lost/unknown: retain exact pending wire, then use the same
        // serialized accepted receipt after reload; never prepare another Commit.
        let waiting = settle(&p, &p.digest(), &b, Decision::Unknown, retain).unwrap();
        let accepted_wire = serde_json::to_vec(&valid_receipt).unwrap();
        let replayed_receipt: AcceptedAuthority = serde_json::from_slice(&accepted_wire).unwrap();
        let reconciled = settle(
            &waiting,
            &waiting.digest(),
            &b,
            Decision::Accepted(Box::new(replayed_receipt)),
            retain,
        )
        .unwrap();
        assert_eq!(reconciled.applied.as_ref(), Some(&b.wire_sha256));
        assert_eq!(waiting.pending.as_ref().unwrap().commit, b.commit);
        let fresh = prepare(&peers[0], 402, vec![], vec![], true).unwrap();
        let fresh_binding = fresh.pending.clone().unwrap();
        assert_ne!(b.commit, fresh_binding.commit);
        // Recipient did not see the rejected Commit. Consumed outgoing handshake
        // generation is retained, and the later legitimate Commit still works.
        peers[0].store = settle(
            &fresh,
            &fresh.digest(),
            &fresh_binding,
            accepted(&fresh_binding),
            retain,
        )
        .unwrap();
        let future_wire = send(&mut peers[0], b"requires-commit");
        let owner = peers[0].store.roster.leaves[0].clone();
        let future_epoch = peers[0].store.roster.epoch;
        assert!(
            receive_application(
                &peers[1].store,
                &future_wire,
                AAD,
                &owner,
                future_epoch,
                retain
            )
            .is_err()
        );
        peers[1].store = receive_commit(&peers[1].store, &fresh_binding, retain).unwrap();
        receive_application(
            &peers[1].store,
            &future_wire,
            AAD,
            &owner,
            future_epoch,
            retain,
        )
        .unwrap();
        let sender = peers[0].store.roster.leaves[1].clone();
        let epoch = peers[0].store.roster.epoch;
        let mut wires = Vec::with_capacity(1002);
        for _ in 0..1002 {
            wires.push(send(&mut peers[1], b"ratchet-limit"));
        }
        let original = peers[0].store.clone();
        assert!(matches!(
            receive_application(&original, &wires[1001], AAD, &sender, epoch, retain),
            Err(ProofError::ForwardDistance)
        ));
        assert!(
            matches!(
                receive_application(&original, &wires[1001], AAD, &sender, epoch, retain),
                Err(ProofError::ForwardDistance)
            ),
            "rejection does not advance ratchet"
        );
        let advanced = receive_application(&original, &wires[6], AAD, &sender, epoch, retain)
            .unwrap()
            .0;
        assert!(matches!(
            receive_application(&advanced, &wires[0], AAD, &sender, epoch, retain),
            Err(ProofError::GenerationTooOld)
        ));
        assert!(matches!(
            receive_application(&advanced, &wires[6], AAD, &sender, epoch, retain),
            Err(ProofError::Replay)
        ));
        let mut tampered = wires[4].clone();
        let end = tampered.len() - 1;
        tampered[end] ^= 1;
        assert!(receive_application(&advanced, &tampered, AAD, &sender, epoch, retain).is_err());
        receive_application(&advanced, &wires[4], AAD, &sender, epoch, retain).unwrap();
    }
}
fn crash_apply(path: &Path, next: &Store, when: &str) {
    let bytes = Zeroizing::new(serde_json::to_vec(next).unwrap());
    fs::write(path.with_extension("next"), &bytes).unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "api::group_e2ee::pending_retention_proof::proof_child",
            "--nocapture",
        ])
        .env("MLS_PROOF_CHILD", when)
        .env("MLS_PROOF_TARGET", path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
}
#[test]
fn pending_lifecycle_reload_receive_merge_discard_crash_cas() {
    fs::create_dir_all(proof_dir()).unwrap();
    for retain in [2, 4] {
        let mut peers = fixture(4, retain);
        let before = peers[0].store.clone();
        let wire = send(&mut peers[1], b"before-pending");
        let sender = before.roster.leaves[1].clone();
        let prepared = prepare(&peers[0], 2, vec![], vec![], true).unwrap();
        let binding = prepared.pending.clone().unwrap();
        let path = proof_dir().join(format!("lifecycle-{retain}-{}.json", std::process::id()));
        persist(&path, None, before.clone(), None).unwrap();
        crash_apply(&path, &prepared, "before");
        assert_eq!(read_store(&path).digest(), before.digest());
        crash_apply(&path, &prepared, "after");
        let loaded = read_store(&path);
        assert_eq!(loaded.pending, Some(binding.clone()));
        peers[0].store = loaded.clone();
        assert!(matches!(
            prepare(&peers[0], 3, vec![], vec![], true),
            Err(ProofError::PendingExists)
        ));
        let (received, plain, _) =
            receive_application(&loaded, &wire, AAD, &sender, before.roster.epoch, retain).unwrap();
        assert_eq!(plain, b"before-pending");
        let dart = legacy_process_message_with_storage(
            GID.to_vec(),
            wire.clone(),
            AAD.to_vec(),
            loaded.roster.expected(),
            loaded.roster.expected(),
            loaded.entries(),
            1,
        )
        .unwrap();
        let mut dart_state = loaded.clone();
        dart_state.apply(&dart.storage_batch);
        assert_eq!(
            dart_state.digest(),
            received.digest(),
            "private receive agrees with released Dart-facing Rust core"
        );
        assert_eq!(
            dart.application_message.as_deref(),
            Some(b"before-pending".as_slice())
        );
        assert_ne!(received.digest(), loaded.digest());
        assert!(matches!(
            receive_application(&received, &wire, AAD, &sender, before.roster.epoch, retain),
            Err(ProofError::Replay)
        ));
        assert!(
            receive_application(
                &loaded,
                &wire,
                b"wrong",
                &sender,
                before.roster.epoch,
                retain
            )
            .is_err()
        );
        persist(&path, Some(loaded.revision), received.clone(), None).unwrap();
        assert!(matches!(
            persist(&path, Some(loaded.revision), loaded.clone(), None),
            Err(ProofError::BaseMismatch)
        ));
        let received = read_store(&path);
        assert!(matches!(
            settle(
                &received,
                &loaded.digest(),
                &binding,
                accepted(&binding),
                retain
            ),
            Err(ProofError::BaseMismatch)
        ));
        let unknown = settle(
            &received,
            &received.digest(),
            &binding,
            Decision::Unknown,
            retain,
        )
        .unwrap();
        assert_eq!(unknown.pending, received.pending);
        assert_eq!(unknown.digest(), received.digest());
        let mut wrong = binding.clone();
        wrong.command += 1;
        assert!(matches!(
            settle(
                &received,
                &received.digest(),
                &wrong,
                accepted(&binding),
                retain
            ),
            Err(ProofError::BindingMismatch)
        ));
        let rejected = settle(
            &received,
            &received.digest(),
            &binding,
            Decision::Rejected,
            retain,
        )
        .unwrap();
        assert!(rejected.pending.is_none());
        assert!(matches!(
            receive_application(&rejected, &wire, AAD, &sender, before.roster.epoch, retain),
            Err(ProofError::Replay)
        ));
        assert_ne!(
            rejected.digest(),
            before.digest(),
            "discard must not restore preparation snapshot"
        );
        let merged = settle(
            &received,
            &received.digest(),
            &binding,
            accepted(&binding),
            retain,
        )
        .unwrap();
        crash_apply(&path, &merged, "before");
        assert_eq!(read_store(&path).pending, Some(binding.clone()));
        crash_apply(&path, &merged, "after");
        let merged = read_store(&path);
        assert_eq!(merged.applied, Some(binding.wire_sha256.clone()));
        assert!(merged.pending.is_none());
        assert!(matches!(
            settle(
                &merged,
                &merged.digest(),
                &binding,
                accepted(&binding),
                retain
            ),
            Err(ProofError::MissingPending)
        ));
        assert!(matches!(
            receive_application(&merged, &wire, AAD, &sender, before.roster.epoch, retain),
            Err(ProofError::Replay)
        ));
        peers[0].store = merged;
        for p in peers.iter_mut().skip(1) {
            p.store = receive_commit(&p.store, &binding, retain).unwrap();
        }
        let wire = send(&mut peers[1], b"after-merge");
        let epoch = peers[0].store.roster.epoch;
        let (_, plain, _) =
            receive_application(&peers[0].store, &wire, AAD, &sender, epoch, retain).unwrap();
        assert_eq!(plain, b"after-merge");
        println!(
            "P1 retention={retain} reload/live-receive/replay/accepted/rejected/unknown/crash/CAS PASS"
        );
    }
}
