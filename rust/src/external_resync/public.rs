use super::Error;
use super::*;
use crate::api::support::mls_message_from_exact_bytes;
use openmls::prelude::*;
use openmls::treesync::errors::LeafNodeValidationError;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
fn check_hash(bytes: &[u8], expected: &[u8]) -> Result<(), Error> {
    if expected.len() != 32 {
        return Err(Error::LimitExceeded);
    }
    if hash(bytes) != expected {
        return Err(Error::HashMismatch);
    }
    Ok(())
}
pub(crate) fn validate_leaf(l: &Leaf) -> Result<(), Error> {
    if l.index >= MAX_LEAVES as u32 || l.identity.len() != 45 || l.signature_key.len() != 32 {
        return Err(Error::LimitExceeded);
    }
    Ok(())
}
fn roster(g: &PublicGroup) -> Result<Roster, Error> {
    let mut leaves = Vec::new();
    let mut identities = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for member in g.members() {
        if leaves.len() >= MAX_LEAVES {
            return Err(Error::LimitExceeded);
        }
        let l = Leaf {
            index: member.index.u32(),
            identity: BasicCredential::try_from(member.credential)
                .map_err(|_| Error::UnsupportedProfile)?
                .identity()
                .to_vec(),
            signature_key: member.signature_key,
        };
        validate_leaf(&l)?;
        if !identities.insert(l.identity.clone()) || !keys.insert(l.signature_key.clone()) {
            return Err(Error::IdentityMismatch);
        }
        leaves.push(l);
    }
    leaves.sort_by_key(|l| l.index);
    let group_id = g.group_id().as_slice().to_vec();
    let epoch = g.group_context().epoch().as_u64();
    let mut h = Sha256::new();
    h.update(b"openmls_dart/roster-summary/v1\0");
    let lp = |h: &mut Sha256, b: &[u8]| {
        h.update((b.len() as u32).to_be_bytes());
        h.update(b);
    };
    lp(&mut h, &group_id);
    h.update(epoch.to_be_bytes());
    h.update((leaves.len() as u32).to_be_bytes());
    for l in &leaves {
        h.update(l.index.to_be_bytes());
        lp(&mut h, &l.identity);
        lp(&mut h, &l.signature_key);
    }
    Ok(Roster {
        state: ExpectedState {
            group_id,
            epoch,
            roster_sha256: h.finalize().to_vec(),
        },
        leaves,
    })
}
fn load(bytes: &[u8], expected_hash: &[u8]) -> Result<(PublicGroup, VerifiedSnapshot), Error> {
    preflight::group_info(bytes)?;
    check_hash(bytes, expected_hash)?;
    let MlsMessageBodyIn::GroupInfo(info) = mls_message_from_exact_bytes(bytes)
        .map_err(|_| Error::DecodeRejected)?
        .extract()
    else {
        return Err(Error::DecodeRejected);
    };
    let tree = info
        .extensions()
        .ratchet_tree()
        .ok_or(Error::MissingRequiredExtension)?
        .ratchet_tree()
        .clone();
    let p = OpenMlsRustCrypto::default();
    let (g, verified) =
        PublicGroup::from_external(p.crypto(), p.storage(), tree, info, ProposalStore::new())
            .map_err(|e| match e {
                CreationFromExternalError::InvalidGroupInfoSignature => Error::SignatureRejected,
                CreationFromExternalError::UnknownSender => Error::SignerMismatch,
                CreationFromExternalError::LeafNodeValidation(
                    LeafNodeValidationError::Lifetime(_),
                ) => Error::LifetimeRejected,
                CreationFromExternalError::UnsupportedMlsVersion => Error::UnsupportedProfile,
                CreationFromExternalError::WriteToStorageError(_)
                | CreationFromExternalError::LibraryError(_) => Error::InternalFailure,
                _ => Error::TreeRejected,
            })?;
    let r = roster(&g)?;
    let signer = r
        .leaves
        .iter()
        .find(|l| l.index == verified.signer().u32())
        .ok_or(Error::SignerMismatch)?
        .clone();
    Ok((
        g,
        VerifiedSnapshot {
            profile: PROFILE,
            sha256: expected_hash.to_vec(),
            roster: r,
            signer,
        },
    ))
}
fn expected(actual: &ExpectedState, expected: &ExpectedState) -> Result<(), Error> {
    if expected.group_id.len() != 16 || expected.roster_sha256.len() != 32 {
        return Err(Error::LimitExceeded);
    }
    if actual.group_id != expected.group_id {
        return Err(Error::GroupMismatch);
    }
    if actual.epoch != expected.epoch {
        return Err(Error::EpochMismatch);
    }
    if actual.roster_sha256 != expected.roster_sha256 {
        return Err(Error::RosterMismatch);
    }
    Ok(())
}
fn snapshot_inner(r: &SnapshotRequest) -> Result<(PublicGroup, VerifiedSnapshot), Error> {
    if r.profile != PROFILE {
        return Err(Error::UnsupportedProfile);
    }
    validate_leaf(&r.expected_signer)?;
    let (g, v) = load(&r.group_info, &r.sha256)?;
    expected(&v.roster.state, &r.expected)?;
    if v.signer != r.expected_signer {
        return Err(Error::SignerMismatch);
    }
    Ok((g, v))
}
/// Pure public verification. Caller authenticates the expected publication authority.
pub fn validate_snapshot(r: &SnapshotRequest) -> Result<VerifiedSnapshot, Error> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        snapshot_inner(r).map(|(_, v)| v)
    }))
    .unwrap_or(Err(Error::InternalFailure))
}
pub fn validate_external_successor(r: &SuccessorRequest) -> Result<VerifiedSuccessor, Error> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| successor_inner(r)))
        .unwrap_or(Err(Error::InternalFailure))
}
fn successor_inner(r: &SuccessorRequest) -> Result<VerifiedSuccessor, Error> {
    if r.expected_aad.is_empty()
        || r.expected_aad.len() > MAX_AAD_BYTES
        || r.authorized_new_identity.len() != 45
        || r.authorized_new_signature_key.len() != 32
    {
        return Err(Error::LimitExceeded);
    }
    validate_leaf(&r.authorized_old)?;
    let (mut g, previous) = snapshot_inner(&r.base)?;
    if !previous.roster.leaves.contains(&r.authorized_old) {
        return Err(Error::IdentityMismatch);
    }
    if previous.roster.leaves.iter().any(|l| {
        // A stable participant credential may survive rotation of the exactly
        // authorized old leaf. No unchanged leaf may collide; the key is fresh.
        (l != &r.authorized_old && l.identity == r.authorized_new_identity)
            || l.signature_key == r.authorized_new_signature_key
    }) {
        return Err(Error::IdentityMismatch);
    }
    preflight::external_commit(&r.commit)?;
    check_hash(&r.commit, &r.commit_sha256)?;
    let p = OpenMlsRustCrypto::default();
    let message = mls_message_from_exact_bytes(&r.commit)
        .map_err(|_| Error::DecodeRejected)?
        .try_into_protocol_message()
        .map_err(|_| Error::ProtocolRejected)?;
    if message.group_id() != g.group_id() {
        return Err(Error::GroupMismatch);
    }
    if message.epoch() != g.group_context().epoch() {
        return Err(Error::EpochMismatch);
    }
    let processed = g
        .process_message(p.crypto(), message)
        .map_err(|_| Error::ProtocolRejected)?;
    if processed.aad() != r.expected_aad {
        return Err(Error::AadMismatch);
    }
    if processed.sender() != &Sender::NewMemberCommit
        || BasicCredential::try_from(processed.credential().clone())
            .map_err(|_| Error::UnsupportedProfile)?
            .identity()
            != r.authorized_new_identity
    {
        return Err(Error::IdentityMismatch);
    }
    let ProcessedMessageContent::StagedCommitMessage(staged) = processed.into_content() else {
        return Err(Error::ProtocolRejected);
    };
    if staged
        .update_path_leaf_node()
        .ok_or(Error::ProtocolRejected)?
        .signature_key()
        .as_slice()
        != r.authorized_new_signature_key
    {
        return Err(Error::SignerMismatch);
    }
    let mut init = 0;
    let mut remove = 0;
    for q in staged.queued_proposals() {
        if q.proposal_or_ref_type() != ProposalOrRefType::Proposal {
            return Err(Error::DeltaMismatch);
        }
        match q.proposal() {
            Proposal::ExternalInit(_) => init += 1,
            Proposal::Remove(v) if v.removed().u32() == r.authorized_old.index => remove += 1,
            _ => return Err(Error::DeltaMismatch),
        }
    }
    if init != 1 || remove != 1 {
        return Err(Error::DeltaMismatch);
    }
    g.merge_commit(p.storage(), *staged)
        .map_err(|_| Error::InternalFailure)?;
    let derived = roster(&g)?;
    if derived.state.epoch
        != previous
            .roster
            .state
            .epoch
            .checked_add(1)
            .ok_or(Error::EpochMismatch)?
    {
        return Err(Error::EpochMismatch);
    }
    if derived.leaves.len() != previous.roster.leaves.len()
        || previous
            .roster
            .leaves
            .iter()
            .filter(|l| **l != r.authorized_old)
            .any(|l| !derived.leaves.contains(l))
        || derived.leaves.contains(&r.authorized_old)
    {
        return Err(Error::DeltaMismatch);
    }
    let new_leaf = derived
        .leaves
        .iter()
        .find(|l| {
            l.identity == r.authorized_new_identity
                && l.signature_key == r.authorized_new_signature_key
        })
        .ok_or(Error::DeltaMismatch)?;
    let (next, resulting) = load(&r.reported_next_group_info, &r.reported_next_sha256)?;
    if resulting.signer != *new_leaf {
        return Err(Error::SignerMismatch);
    }
    if resulting.roster != derived
        || g.group_context() != next.group_context()
        || g.export_ratchet_tree() != next.export_ratchet_tree()
        || g.confirmation_tag() != next.confirmation_tag()
    {
        return Err(Error::SuccessorMismatch);
    }
    Ok(VerifiedSuccessor {
        previous,
        commit_sha256: r.commit_sha256.clone(),
        aad_sha256: hash(&r.expected_aad),
        resulting,
    })
}
