//! Isolated external candidate and explicit private receiver. Never owns a DB.
use super::*;
use crate::api::external_resync::*;
use crate::external_resync::{self as public_core, codec as public_codec};
mod candidate;

fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
fn state(r: &MlsRosterSummaryV1) -> MlsExpectedRosterStateV1 {
    MlsExpectedRosterStateV1 {
        group_id: r.group_id.clone(),
        epoch: r.epoch,
        digest_sha256: r.digest_sha256.clone(),
    }
}
fn public_leaf(l: &MlsRosterLeafV1) -> public_core::Leaf {
    public_core::Leaf {
        index: l.leaf_index,
        identity: l.credential_identity.clone(),
        signature_key: l.signature_public_key.clone(),
    }
}
fn local_leaf(l: &public_core::Leaf) -> MlsRosterLeafV1 {
    MlsRosterLeafV1 {
        leaf_index: l.index,
        credential_identity: l.identity.clone(),
        signature_public_key: l.signature_key.clone(),
    }
}
fn local_roster(r: &public_core::Roster) -> MlsRosterSummaryV1 {
    MlsRosterSummaryV1 {
        group_id: r.state.group_id.clone(),
        epoch: r.state.epoch,
        digest_sha256: r.state.roster_sha256.clone(),
        leaves: r.leaves.iter().map(local_leaf).collect(),
    }
}
fn public_error(e: public_core::Error) -> Error {
    match e {
        public_core::Error::LimitExceeded => Error::LimitExceeded,
        public_core::Error::GroupMismatch => Error::GroupMismatch,
        public_core::Error::EpochMismatch => Error::PreviousEpochMismatch,
        public_core::Error::RosterMismatch => Error::PreviousRosterMismatch,
        public_core::Error::SignerMismatch => Error::SenderMismatch,
        public_core::Error::IdentityMismatch | public_core::Error::DeltaMismatch => {
            Error::ExternalDeltaMismatch
        }
        public_core::Error::AadMismatch => Error::AadMismatch,
        public_core::Error::HashMismatch => Error::WireHashMismatch,
        public_core::Error::UnsupportedProfile => Error::UnsupportedProfile,
        public_core::Error::InternalFailure => Error::InternalFailure,
        _ => Error::MlsProtocolRejected,
    }
}
fn id(bytes: &[u8]) -> Result<(), Error> {
    if !(1..=128).contains(&bytes.len()) {
        Err(Error::LimitExceeded)
    } else {
        Ok(())
    }
}
fn fixed32(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() != 32 {
        Err(Error::LimitExceeded)
    } else {
        Ok(())
    }
}
fn leaf_bounds(l: &MlsRosterLeafV1) -> Result<(), Error> {
    public_core::public::validate_leaf(&public_leaf(l)).map_err(public_error)
}
fn validate_binding(b: &MlsExternalCommitBindingV1) -> Result<(), Error> {
    id(&b.candidate_id)?;
    id(&b.incarnation_id)?;
    id(&b.transition.command_id)?;
    for h in [
        &b.transition.context_sha256,
        &b.previous_state.digest_sha256,
        &b.input_group_info_sha256,
        &b.commit_sha256,
        &b.aad_sha256,
        &b.resulting_group_info_sha256,
    ] {
        fixed32(h)?;
    }
    if b.previous_state.group_id.len() != 16 {
        return Err(Error::LimitExceeded);
    }
    leaf_bounds(&b.old_leaf)?;
    leaf_bounds(&b.author)
}
fn validate_delta(
    previous: &MlsRosterSummaryV1,
    resulting: &MlsRosterSummaryV1,
    old: &MlsRosterLeafV1,
    new: &MlsRosterLeafV1,
) -> Result<(), Error> {
    leaf_bounds(old)?;
    leaf_bounds(new)?;
    if !previous.leaves.contains(old)
        || previous.leaves.iter().any(|l| {
            l.signature_public_key == new.signature_public_key
                || (l != old && l.credential_identity == new.credential_identity)
        })
        || previous.leaves.len() != resulting.leaves.len()
        || previous.group_id != resulting.group_id
        || previous.epoch.checked_add(1) != Some(resulting.epoch)
        || previous
            .leaves
            .iter()
            .filter(|l| *l != old)
            .any(|l| !resulting.leaves.contains(l))
        || resulting.leaves.contains(old)
        || resulting.leaves.iter().filter(|l| *l == new).count() != 1
    {
        return Err(Error::ExternalDeltaMismatch);
    }
    validate_canonical_leaves(&resulting.leaves).map_err(|_| Error::ExternalDeltaMismatch)?;
    if roster_digest(&resulting.group_id, resulting.epoch, &resulting.leaves)
        .map_err(|_| Error::ExternalDeltaMismatch)?
        != resulting.digest_sha256
    {
        return Err(Error::ResultingRosterMismatch);
    }
    Ok(())
}

pub(crate) fn prepare(
    input: MlsExternalPreparationV1,
    signer_bytes: Vec<u8>,
) -> Result<PrepareExternalCommitResult, Error> {
    let signer_bytes = Zeroizing::new(signer_bytes);
    id(&input.candidate_id)?;
    id(&input.incarnation_id)?;
    id(&input.transition.command_id)?;
    fixed32(&input.transition.context_sha256)?;
    fixed32(&input.local_lifecycle_context_sha256)?;
    bounded_aad(&input.aad)?;
    validate_retention(input.retention)?;
    if signer_bytes.is_empty()
        || signer_bytes.len() > 4096
        || input.authorized_new_credential_identity.len() != 45
        || input.authorized_new_signature_public_key.len() != 32
    {
        return Err(Error::LimitExceeded);
    }
    let base = public_core::SnapshotRequest {
        profile: public_core::PROFILE,
        group_info: input.input_group_info,
        sha256: input.input_group_info_sha256,
        expected: public_core::ExpectedState {
            group_id: input.expected_previous_state.group_id,
            epoch: input.expected_previous_state.epoch,
            roster_sha256: input.expected_previous_state.digest_sha256,
        },
        expected_signer: public_leaf(&input.expected_snapshot_signer),
    };
    let verified = public_core::public::validate_snapshot(&base).map_err(public_error)?;
    let previous = local_roster(&verified.roster);
    leaf_bounds(&input.authorized_old_leaf)?;
    if !previous.leaves.contains(&input.authorized_old_leaf)
        || previous.leaves.iter().any(|l| {
            l.signature_public_key == input.authorized_new_signature_public_key
                || (l != &input.authorized_old_leaf
                    && l.credential_identity == input.authorized_new_credential_identity)
        })
    {
        return Err(Error::ExternalDeltaMismatch);
    }
    let signer = signer_from_bytes(signer_bytes.to_vec()).map_err(|_| Error::InvalidSigner)?;
    ensure_signer_public_key(&signer, &input.authorized_new_signature_public_key)
        .map_err(|_| Error::InvalidSigner)?;
    let credential = build_credential_with_key(
        &input.authorized_new_credential_identity,
        &input.authorized_new_signature_public_key,
        None,
    )
    .map_err(|_| Error::UnsupportedCredential)?;
    let MlsMessageBodyIn::GroupInfo(info) = mls_message_from_exact_bytes(&base.group_info)
        .map_err(|_| Error::MlsDecodeRejected)?
        .extract()
    else {
        return Err(Error::MlsDecodeRejected);
    };
    // Empty provider is the inactive namespace; external finalize's automatic
    // merge can never mutate a live caller snapshot here.
    let provider = provider_from_entries(vec![], 1, None).map_err(|_| Error::InternalFailure)?;
    let (group, bundle) = MlsGroup::external_commit_builder()
        .with_config(profile(input.retention)?.to_join_config())
        .with_resync_leaf(LeafNodeIndex::new(input.authorized_old_leaf.leaf_index))
        .with_aad(input.aad.clone())
        .build_group(&provider, info, credential)
        .map_err(|_| Error::MlsProtocolRejected)?
        .load_psks(provider.storage())
        .map_err(|_| Error::MlsProtocolRejected)?
        .build(provider.rand(), provider.crypto(), &signer, |_| true)
        .map_err(|_| Error::MlsProtocolRejected)?
        .finalize(&provider)
        .map_err(|_| Error::MlsProtocolRejected)?;
    let (commit, welcome, next) = bundle.into_messages();
    if welcome.is_some() {
        return Err(Error::InternalFailure);
    }
    let commit = commit
        .tls_serialize_detached()
        .map_err(|_| Error::InternalFailure)?;
    let next = next
        .ok_or(Error::MlsProtocolRejected)?
        .tls_serialize_detached()
        .map_err(|_| Error::InternalFailure)?;
    let proof = public_core::SuccessorRequest {
        base,
        commit_sha256: hash(&commit),
        commit,
        expected_aad: input.aad,
        authorized_old: public_leaf(&input.authorized_old_leaf),
        authorized_new_identity: input.authorized_new_credential_identity,
        authorized_new_signature_key: input.authorized_new_signature_public_key,
        reported_next_sha256: hash(&next),
        reported_next_group_info: next,
    };
    let checked = public_core::public::validate_external_successor(&proof).map_err(public_error)?;
    let resulting = roster_from_group(&group).map_err(|_| Error::UnsupportedCredential)?;
    if resulting != local_roster(&checked.resulting.roster) {
        return Err(Error::ResultingRosterMismatch);
    }
    let author = local_leaf(&checked.resulting.signer);
    validate_delta(&previous, &resulting, &input.authorized_old_leaf, &author)?;
    if group.own_leaf_index().u32() != author.leaf_index || group.pending_commit().is_some() {
        return Err(Error::InternalFailure);
    }
    LocalGroupState::new(
        input.incarnation_id.clone(),
        input.retention,
        resulting.clone(),
    )?
    .save(provider.storage())?;
    let private_digest = digest(&provider, &resulting.group_id)?;
    let mut rows = Entries(provider.storage().snapshot_entries(&resulting.group_id));
    bounded_entries(&rows.0)?;
    rows.0.sort_by(|a, b| a.key.cmp(&b.key));
    let c = candidate::Candidate {
        candidate_id: input.candidate_id,
        incarnation_id: input.incarnation_id,
        transition: input.transition,
        lifecycle: input.local_lifecycle_context_sha256,
        retention: input.retention,
        proof,
        previous,
        resulting,
        private_digest,
        rows,
    };
    let mut encoded = candidate::encode(&c)?;
    let candidate_hash = hash(&encoded);
    let info = c.info(candidate_hash)?;
    Ok(PrepareExternalCommitResult {
        candidate_bytes: std::mem::take(&mut *encoded),
        info,
    })
}

pub(crate) fn inspect(
    bytes: Vec<u8>,
    expected_hash: Vec<u8>,
) -> Result<ExternalCommitCandidateInfo, Error> {
    let bytes = Zeroizing::new(bytes);
    let c = candidate::read(&bytes, &expected_hash)?;
    c.info(expected_hash)
}
pub(crate) fn accept(
    target: MlsExternalTargetContextV1,
    bytes: Vec<u8>,
    expected_hash: Vec<u8>,
    acceptance: MlsExternalCommitAcceptanceV1,
) -> Result<AcceptExternalCommitResult, Error> {
    let bytes = Zeroizing::new(bytes);
    let mut target_rows = Entries(target.storage_entries);
    let mut c = candidate::read(&bytes, &expected_hash)?;
    validate_binding(&acceptance.binding)?;
    if acceptance.binding != c.binding()? || acceptance.resulting_state != state(&c.resulting) {
        return Err(Error::ExternalAcceptanceMismatch);
    }
    if target.group_id != c.resulting.group_id || target.incarnation_id != c.incarnation_id {
        return Err(Error::GroupMismatch);
    }
    if target.local_lifecycle_context_sha256 != c.lifecycle {
        return Err(Error::ExternalLifecycleMismatch);
    }
    if target.storage_format_version != 1 {
        return Err(Error::StorageFormatMismatch);
    }
    bounded_entries(&target_rows.0)?;
    validate_storage_entries(&target_rows.0, 1, Some(&target.group_id))
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    if target_rows.0.iter().any(|r| !is_global_key(&r.key)) {
        return Err(Error::ExternalTargetStateExists);
    }
    let provider = provider_from_entries(std::mem::take(&mut target_rows.0), 1, None)
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    for row in &c.rows.0 {
        if let Some(existing) = provider.storage().read_opaque(&row.key)
            && *existing != row.value
        {
            return Err(Error::InvalidStorageSnapshot);
        }
    }
    for row in &mut c.rows.0 {
        provider
            .storage()
            .write_opaque(std::mem::take(&mut row.key), std::mem::take(&mut row.value));
    }
    let resulting_digest = digest(&provider, &target.group_id)?;
    if resulting_digest != c.private_digest {
        return Err(Error::ExternalCandidateInvalid);
    }
    let mut batch = batch_from_provider(provider, Some(target.group_id), vec![])
        .map_err(|_| Error::InternalFailure)?;
    if let Err(e) = bounded_batch(&batch) {
        zeroize_entry_values(&mut batch.upserts);
        return Err(e);
    }
    Ok(AcceptExternalCommitResult {
        candidate_sha256: expected_hash,
        local_leaf: acceptance.binding.author,
        resulting_roster: c.resulting,
        resulting_group_state_sha256: resulting_digest,
        effective_retention: c.retention,
        storage_batch: batch,
    })
}

pub(crate) fn receive(
    context: Context,
    commit: Vec<u8>,
    aad: Vec<u8>,
    acceptance: MlsExternalCommitAcceptanceV1,
    superseded: Option<MlsPendingCommitBinding>,
) -> Result<ProcessExternalCommitResult, Error> {
    if commit.is_empty() || commit.len() > public_core::MAX_MLS_BYTES {
        return Err(Error::LimitExceeded);
    }
    bounded_aad(&aad)?;
    validate_binding(&acceptance.binding)?;
    let b = &acceptance.binding;
    if hash(&commit) != b.commit_sha256 {
        return Err(Error::WireHashMismatch);
    }
    if hash(&aad) != b.aad_sha256 {
        return Err(Error::AadMismatch);
    }
    let mut loaded = load(context)?;
    if !loaded.group.is_active() {
        return Err(Error::InactiveGroup);
    }
    let previous = loaded.local.current.clone();
    if loaded.local.incarnation_id != b.incarnation_id {
        return Err(Error::GroupMismatch);
    }
    check_state(&previous, &b.previous_state, false)?;
    if b.old_leaf.leaf_index == loaded.group.own_leaf_index().u32() {
        return Err(Error::ExternalOwnCommit);
    }
    if loaded.local.pending.as_ref().map(|p| &p.binding) != superseded.as_ref() {
        return Err(Error::PendingBindingMismatch);
    }
    // Discard only the identified loser on the CURRENT snapshot, never restore
    // preparation-time rows (which would rewind same-epoch message ratchets).
    if superseded.is_some() {
        loaded
            .group
            .clear_pending_commit(loaded.provider.storage())
            .map_err(|_| Error::InvalidStorageSnapshot)?;
        loaded.local.pending = None;
        loaded.local.clear_retry_rows(loaded.provider.storage());
    }
    crate::external_resync::preflight::external_commit(&commit).map_err(public_error)?;
    let wire = mls_message_from_exact_bytes(&commit)
        .map_err(|_| Error::MlsDecodeRejected)?
        .try_into_protocol_message()
        .map_err(|_| Error::MessageKindMismatch)?;
    if wire.group_id().as_slice() != previous.group_id {
        return Err(Error::GroupMismatch);
    }
    if wire.epoch().as_u64() != previous.epoch {
        return Err(Error::RequiredCommitEpochMismatch);
    }
    let message = loaded
        .group
        .process_message(&loaded.provider, wire)
        .map_err(|_| Error::MlsProtocolRejected)?;
    if message.sender() != &Sender::NewMemberCommit {
        return Err(Error::MessageKindMismatch);
    }
    if message.aad() != aad {
        return Err(Error::AadMismatch);
    }
    if BasicCredential::try_from(message.credential().clone())
        .map_err(|_| Error::UnsupportedCredential)?
        .identity()
        != b.author.credential_identity
    {
        return Err(Error::SenderMismatch);
    }
    let ProcessedMessageContent::StagedCommitMessage(staged) = message.into_content() else {
        return Err(Error::MessageKindMismatch);
    };
    if staged
        .update_path_leaf_node()
        .ok_or(Error::ExternalDeltaMismatch)?
        .signature_key()
        .as_slice()
        != b.author.signature_public_key
    {
        return Err(Error::SenderMismatch);
    }
    let mut init = 0;
    let mut removed = 0;
    for q in staged.queued_proposals() {
        if q.proposal_or_ref_type() != ProposalOrRefType::Proposal {
            return Err(Error::ExternalDeltaMismatch);
        }
        match q.proposal() {
            Proposal::ExternalInit(_) => init += 1,
            Proposal::Remove(r) if r.removed().u32() == b.old_leaf.leaf_index => removed += 1,
            _ => return Err(Error::ExternalDeltaMismatch),
        }
    }
    if init != 1 || removed != 1 {
        return Err(Error::ExternalDeltaMismatch);
    }
    loaded
        .group
        .merge_staged_commit(&loaded.provider, *staged)
        .map_err(|_| Error::MlsProtocolRejected)?;
    let resulting = roster_from_group(&loaded.group).map_err(|_| Error::UnsupportedCredential)?;
    validate_delta(&previous, &resulting, &b.old_leaf, &b.author)?;
    check_state(&resulting, &acceptance.resulting_state, true)?;
    if !loaded.group.is_active() {
        return Err(Error::InactiveGroup);
    }
    loaded.local.advance(resulting)?;
    let mutation = finish(loaded, previous)?;
    Ok(ProcessExternalCommitResult {
        authenticated_sender: acceptance.binding.author,
        previous_roster: mutation.previous,
        resulting_roster: mutation.resulting,
        resulting_group_state_sha256: mutation.resulting_digest,
        effective_retention: mutation.retention,
        superseded_pending: superseded,
        storage_batch: mutation.batch,
    })
}
