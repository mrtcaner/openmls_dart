//! Shared production lifecycle core. Public API/native wiring is a separate cut.
use super::*;
use crate::local_group_state::{LocalGroupState, PendingRecord, validate_retention};
use crate::snapshot_storage::SnapshotOpenMlsProvider;
use openmls::framing::errors::{MessageDecryptionError, SecretTreeError};
use zeroize::Zeroizing;
type Error = MlsErrorCode;

pub(crate) struct Context {
    pub group_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub expected_current: MlsExpectedRosterStateV1,
    pub expected_base_sha256: Vec<u8>,
    pub expected_retention: u32,
    pub entries: Vec<MlsStorageEntry>,
    pub storage_format: u32,
}
impl Drop for Context {
    fn drop(&mut self) {
        zeroize_entry_values(&mut self.entries);
    }
}
pub(crate) struct Mutation {
    pub previous: MlsRosterSummaryV1,
    pub resulting: MlsRosterSummaryV1,
    pub resulting_digest: Vec<u8>,
    pub retention: u32,
    pub batch: MlsStorageBatch,
}
pub(crate) struct Prepared {
    pub binding: MlsPendingCommitBinding,
    pub proposed: MlsRosterSummaryV1,
    pub commit: Vec<u8>,
    pub welcome: Option<Vec<u8>>,
    pub group_info: Option<Vec<u8>>,
    pub preparation_base: Vec<u8>,
    pub mutation: Mutation,
}
struct Entries(Vec<MlsStorageEntry>);
impl Drop for Entries {
    fn drop(&mut self) {
        zeroize_entry_values(&mut self.0);
    }
}
struct Loaded {
    provider: SnapshotOpenMlsProvider,
    group: MlsGroup,
    local: LocalGroupState,
    base: Vec<u8>,
}

fn entry_bytes(entries: &[MlsStorageEntry]) -> Result<usize, Error> {
    if entries.len() > 4096 {
        return Err(Error::LimitExceeded);
    }
    let mut bytes = 0usize;
    for row in entries {
        if row.key.len() > 4096 || row.value.len() > 2 * 1024 * 1024 {
            return Err(Error::LimitExceeded);
        }
        bytes = bytes
            .checked_add(row.key.len())
            .and_then(|v| v.checked_add(row.value.len()))
            .and_then(|v| v.checked_add(row.group_id.as_ref().map_or(0, Vec::len)))
            .ok_or(Error::LimitExceeded)?;
    }
    if bytes > 6 * 1024 * 1024 {
        return Err(Error::LimitExceeded);
    }
    Ok(bytes)
}
fn bounded_entries(entries: &[MlsStorageEntry]) -> Result<(), Error> {
    entry_bytes(entries).map(|_| ())
}
fn bounded_batch(batch: &MlsStorageBatch) -> Result<(), Error> {
    let mut bytes = entry_bytes(&batch.upserts)?;
    if batch.deletes.len() > 4096 || !batch.deleted_group_ids.is_empty() {
        return Err(Error::LimitExceeded);
    }
    for key in &batch.deletes {
        if key.len() > 4096 {
            return Err(Error::LimitExceeded);
        }
        bytes = bytes.checked_add(key.len()).ok_or(Error::LimitExceeded)?;
    }
    if bytes > 6 * 1024 * 1024 {
        return Err(Error::LimitExceeded);
    }
    Ok(())
}
fn bounded_aad(aad: &[u8]) -> Result<(), Error> {
    if aad.is_empty() || aad.len() > 16 * 1024 {
        Err(Error::LimitExceeded)
    } else {
        Ok(())
    }
}
fn check_state(
    actual: &MlsRosterSummaryV1,
    expected: &MlsExpectedRosterStateV1,
    resulting: bool,
) -> Result<(), Error> {
    if actual.group_id != expected.group_id {
        return Err(Error::GroupMismatch);
    }
    if actual.epoch != expected.epoch {
        return Err(if resulting {
            Error::ResultingEpochMismatch
        } else {
            Error::PreviousEpochMismatch
        });
    }
    if actual.digest_sha256 != expected.digest_sha256 {
        return Err(if resulting {
            Error::ResultingRosterMismatch
        } else {
            Error::PreviousRosterMismatch
        });
    }
    Ok(())
}
pub(crate) fn profile(retention: u32) -> Result<MlsGroupConfig, Error> {
    validate_retention(retention)?;
    let mut c = MlsGroupConfig::default_config(
        super::super::types::MlsCiphersuite::Mls128DhkemX25519Aes128gcmSha256Ed25519,
    );
    c.max_past_epochs = retention;
    Ok(c)
}
fn check_configuration(group: &MlsGroup, retention: u32) -> Result<(), Error> {
    let p = profile(retention)?;
    if group.configuration() != &p.to_join_config()
        || group.ciphersuite() != p.to_create_config().ciphersuite()
    {
        Err(Error::ConfigurationMismatch)
    } else {
        Ok(())
    }
}
fn load(mut context: Context) -> Result<Loaded, Error> {
    let mut entries = Entries(std::mem::take(&mut context.entries));
    bounded_entries(&entries.0)?;
    if context.group_id.len() != 16
        || !(1..=128).contains(&context.incarnation_id.len())
        || context.expected_base_sha256.len() != 32
        || context.expected_current.digest_sha256.len() != 32
    {
        return Err(Error::LimitExceeded);
    }
    if context.storage_format != 1 {
        return Err(Error::StorageFormatMismatch);
    }
    validate_retention(context.expected_retention)?;
    let base = group_state_digest_from_entries(&context.group_id, &entries.0, 1)
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    if base != context.expected_base_sha256 {
        return Err(Error::BaseStateMismatch);
    }
    let provider =
        provider_from_entries(std::mem::take(&mut entries.0), 1, Some(&context.group_id))
            .map_err(|_| Error::InvalidStorageSnapshot)?;
    let group =
        load_group(&context.group_id, &provider).map_err(|_| Error::GroupStateUnavailable)?;
    let local = LocalGroupState::load(provider.storage(), &context.group_id)?;
    if local.incarnation_id != context.incarnation_id {
        return Err(Error::GroupMismatch);
    }
    if local.retention != context.expected_retention {
        return Err(Error::ConfigurationMismatch);
    }
    check_configuration(&group, local.retention)?;
    let current = roster_from_group(&group).map_err(|_| Error::UnsupportedCredential)?;
    if current != local.current {
        return Err(Error::InvalidStorageSnapshot);
    }
    check_state(&current, &context.expected_current, false)?;
    match (group.pending_commit(), local.pending.as_ref()) {
        (None, None) => {}
        (Some(_), Some(p)) => {
            if provider
                .storage()
                .pending_state_sha256(group.group_id())
                .map_err(|_| Error::InvalidStorageSnapshot)?
                != p.upstream_pending_sha256
            {
                return Err(Error::PendingBindingMismatch);
            }
        }
        _ => return Err(Error::InvalidStorageSnapshot),
    }
    Ok(Loaded {
        provider,
        group,
        local,
        base,
    })
}
fn digest(provider: &SnapshotOpenMlsProvider, gid: &[u8]) -> Result<Vec<u8>, Error> {
    let entries = Entries(provider.storage().snapshot_entries(gid));
    bounded_entries(&entries.0)?;
    group_state_digest_from_entries(gid, &entries.0, 1).map_err(|_| Error::InvalidStorageSnapshot)
}
fn finish(loaded: Loaded, previous: MlsRosterSummaryV1) -> Result<Mutation, Error> {
    loaded.local.save(loaded.provider.storage())?;
    let resulting_digest = digest(&loaded.provider, &loaded.local.group_id)?;
    let resulting = loaded.local.current;
    let retention = loaded.local.retention;
    let mut batch = batch_from_provider(loaded.provider, Some(resulting.group_id.clone()), vec![])
        .map_err(|_| Error::InternalFailure)?;
    if let Err(error) = bounded_batch(&batch) {
        zeroize_entry_values(&mut batch.upserts);
        return Err(error);
    }
    Ok(Mutation {
        previous,
        resulting,
        resulting_digest,
        retention,
        batch,
    })
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn create(
    config: MlsGroupConfig,
    signer_bytes: Vec<u8>,
    group_id: Vec<u8>,
    incarnation_id: Vec<u8>,
    owner: MlsAuthorizedOwnerV1,
    credential_bytes: Option<Vec<u8>>,
    entries: Vec<MlsStorageEntry>,
    storage_format: u32,
) -> Result<Mutation, Error> {
    let signer_bytes = Zeroizing::new(signer_bytes);
    let mut entries = Entries(entries);
    bounded_entries(&entries.0)?;
    if group_id.len() != 16
        || !(1..=128).contains(&incarnation_id.len())
        || signer_bytes.len() > 4096
        || owner.expected_credential_identity.len() != 45
        || owner.expected_signature_public_key.len() != 32
        || credential_bytes
            .as_ref()
            .is_some_and(|bytes| bytes.len() > 4096)
    {
        return Err(Error::LimitExceeded);
    }
    if storage_format != 1 {
        return Err(Error::StorageFormatMismatch);
    }
    validate_retention(config.max_past_epochs)?;
    let expected = profile(config.max_past_epochs)?;
    if config.to_create_config() != expected.to_create_config() {
        return Err(Error::ConfigurationMismatch);
    }
    let provider = provider_from_entries(std::mem::take(&mut entries.0), 1, None)
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    let signer = signer_from_bytes(signer_bytes.to_vec()).map_err(|_| Error::InvalidSigner)?;
    ensure_signer_public_key(&signer, &owner.expected_signature_public_key)
        .map_err(|_| Error::InvalidSigner)?;
    let credential = build_credential_with_key(
        &owner.expected_credential_identity,
        &owner.expected_signature_public_key,
        credential_bytes.as_deref(),
    )
    .map_err(|_| Error::UnsupportedCredential)?;
    signer
        .store(provider.storage())
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    let group = MlsGroup::new_with_group_id(
        &provider,
        &signer,
        &config.to_create_config(),
        GroupId::from_slice(&group_id),
        credential,
    )
    .map_err(|_| Error::MlsProtocolRejected)?;
    let current = roster_from_group(&group).map_err(|_| Error::UnsupportedCredential)?;
    if current.leaves.len() != 1 || !leaf_matches_owner(&current.leaves[0], &owner) {
        return Err(Error::LocalLeafMismatch);
    }
    let local = LocalGroupState::new(incarnation_id, config.max_past_epochs, current.clone())?;
    finish(
        Loaded {
            provider,
            group,
            local,
            base: vec![],
        },
        current,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare(
    context: Context,
    signer_bytes: Vec<u8>,
    transition: MlsTransitionContext,
    additions: Vec<MlsAuthorizedKeyPackageV1>,
    removals: Vec<MlsAuthorizedRemovalV1>,
    aad: Vec<u8>,
    self_authority: Option<MlsAuthorizedSelfV1>,
) -> Result<Prepared, Error> {
    let signer_bytes = Zeroizing::new(signer_bytes);
    bounded_aad(&aad)?;
    if signer_bytes.len() > 4096 || additions.len() > 256 || removals.len() > 256 {
        return Err(Error::LimitExceeded);
    }
    for addition in &additions {
        if addition.key_package_bytes.is_empty()
            || addition.key_package_bytes.len() > 1024 * 1024
            || addition.expected_credential_identity.len() != 45
            || addition.expected_signature_public_key.len() != 32
        {
            return Err(Error::LimitExceeded);
        }
    }
    if transition.command_id.is_empty()
        || transition.command_id.len() > 128
        || transition.context_sha256.len() != 32
    {
        return Err(Error::LimitExceeded);
    }
    let mut loaded = load(context)?;
    if !loaded.group.is_active() {
        return Err(Error::InactiveGroup);
    }
    if loaded.local.pending.is_some() {
        return Err(Error::PendingCommitExists);
    }
    let previous = loaded.local.current.clone();
    let signer = signer_from_bytes(signer_bytes.to_vec()).map_err(|_| Error::InvalidSigner)?;
    if previous
        .leaves
        .len()
        .saturating_sub(removals.len())
        .saturating_add(additions.len())
        > 256
    {
        return Err(Error::LimitExceeded);
    }
    ensure_local_signer(&loaded.group, &signer).map_err(|_| Error::InvalidSigner)?;
    let author = previous
        .leaves
        .iter()
        .find(|l| l.leaf_index == loaded.group.own_leaf_index().u32())
        .cloned()
        .ok_or(Error::LocalLeafMismatch)?;
    let removal_indices =
        validate_removals(&previous, &removals).map_err(|_| Error::SenderMismatch)?;
    let (keys, requested) =
        validate_additions(&loaded.provider, additions, &previous, &removal_indices)
            .map_err(|_| Error::MlsProtocolRejected)?;
    let aad_sha256 = Sha256::digest(&aad).to_vec();
    loaded.group.set_aad(aad);
    let is_self = self_authority.is_some();
    let bundle = if let Some(expected) = self_authority {
        if !keys.is_empty() || !removal_indices.is_empty() {
            return Err(Error::InvalidFrame);
        }
        validate_self_authority(&loaded.group, &previous, &expected)
            .map_err(|_| Error::LocalLeafMismatch)?;
        loaded
            .group
            .commit_builder()
            .consume_proposal_store(false)
            .force_self_update(true)
            .load_psks(loaded.provider.storage())
            .map_err(|_| Error::MlsProtocolRejected)?
            .build(
                loaded.provider.rand(),
                loaded.provider.crypto(),
                &signer,
                |_| true,
            )
            .map_err(|_| Error::MlsProtocolRejected)?
            .stage_commit(&loaded.provider)
            .map_err(|_| Error::MlsProtocolRejected)?
    } else {
        if keys.is_empty() && removal_indices.is_empty() {
            return Err(Error::InvalidFrame);
        }
        loaded
            .group
            .commit_builder()
            .consume_proposal_store(false)
            .propose_adds(keys)
            .propose_removals(removal_indices.iter().copied().map(LeafNodeIndex::new))
            .load_psks(loaded.provider.storage())
            .map_err(|_| Error::MlsProtocolRejected)?
            .build(
                loaded.provider.rand(),
                loaded.provider.crypto(),
                &signer,
                |_| true,
            )
            .map_err(|_| Error::MlsProtocolRejected)?
            .stage_commit(&loaded.provider)
            .map_err(|_| Error::MlsProtocolRejected)?
    };
    // Derive actual resulting authority in a disposable preview. Never return
    // that preview's full-state batch for delayed replacement of the live group.
    let mut preview_entries = Entries(
        loaded
            .provider
            .storage()
            .snapshot_entries(&previous.group_id),
    );
    let preview = provider_from_entries(
        std::mem::take(&mut preview_entries.0),
        1,
        Some(&previous.group_id),
    )
    .map_err(|_| Error::InternalFailure)?;
    let mut preview_group =
        load_group(&previous.group_id, &preview).map_err(|_| Error::InternalFailure)?;
    preview_group
        .merge_pending_commit(&preview)
        .map_err(|_| Error::MlsProtocolRejected)?;
    let proposed = roster_from_group(&preview_group).map_err(|_| Error::UnsupportedCredential)?;
    if is_self {
        if proposed.leaves != previous.leaves
            || previous.epoch.checked_add(1) != Some(proposed.epoch)
        {
            return Err(Error::ResultingRosterMismatch);
        }
    } else {
        validate_exact_delta(&previous, &proposed, &removal_indices, &requested)
            .map_err(|_| Error::ResultingRosterMismatch)?;
    }
    drop(preview_group);
    drop(preview);
    let (commit, welcome, info) = bundle.into_messages();
    let commit = commit
        .tls_serialize_detached()
        .map_err(|_| Error::InternalFailure)?;
    let welcome = welcome
        .map(|m| m.tls_serialize_detached())
        .transpose()
        .map_err(|_| Error::InternalFailure)?;
    let group_info = info
        .map(|m| m.tls_serialize_detached())
        .transpose()
        .map_err(|_| Error::InternalFailure)?;
    let binding = MlsPendingCommitBinding {
        group_id: previous.group_id.clone(),
        incarnation_id: loaded.local.incarnation_id.clone(),
        transition,
        author,
        previous_state: MlsExpectedRosterStateV1 {
            group_id: previous.group_id.clone(),
            epoch: previous.epoch,
            digest_sha256: previous.digest_sha256.clone(),
        },
        commit_sha256: Sha256::digest(&commit).to_vec(),
        aad_sha256,
        welcome_sha256: welcome.as_ref().map(|x| Sha256::digest(x).to_vec()),
        group_info_sha256: group_info.as_ref().map(|x| Sha256::digest(x).to_vec()),
    };
    let preparation_base = loaded.base.clone();
    loaded.local.pending = Some(PendingRecord {
        binding: binding.clone(),
        proposed_roster: proposed.clone(),
        preparation_base_sha256: preparation_base.clone(),
        upstream_pending_sha256: loaded
            .provider
            .storage()
            .pending_state_sha256(loaded.group.group_id())
            .map_err(|_| Error::InvalidStorageSnapshot)?,
    });
    loaded.local.write_retry_rows(
        loaded.provider.storage(),
        commit.clone(),
        welcome.clone(),
        group_info.clone(),
    )?;
    Ok(Prepared {
        binding,
        proposed,
        commit,
        welcome,
        group_info,
        preparation_base,
        mutation: finish(loaded, previous)?,
    })
}
pub(crate) fn merge(context: Context, acceptance: MlsCommitAcceptance) -> Result<Mutation, Error> {
    let mut loaded = load(context)?;
    if !loaded.group.is_active() {
        return Err(Error::InactiveGroup);
    }
    let pending = loaded
        .local
        .pending
        .as_ref()
        .ok_or(Error::PendingCommitMissing)?;
    let expected_result = MlsExpectedRosterStateV1 {
        group_id: pending.proposed_roster.group_id.clone(),
        epoch: pending.proposed_roster.epoch,
        digest_sha256: pending.proposed_roster.digest_sha256.clone(),
    };
    if acceptance.binding != pending.binding
        || acceptance.resulting_state != expected_result
        || acceptance.preparation_base_group_state_sha256 != pending.preparation_base_sha256
    {
        return Err(Error::AcceptanceBindingMismatch);
    }
    if loaded.group.own_leaf_index().u32() != pending.binding.author.leaf_index {
        return Err(Error::LocalLeafMismatch);
    }
    let proposed = pending.proposed_roster.clone();
    let previous = loaded.local.current.clone();
    loaded
        .group
        .merge_pending_commit(&loaded.provider)
        .map_err(|_| Error::MlsProtocolRejected)?;
    let resulting = roster_from_group(&loaded.group).map_err(|_| Error::UnsupportedCredential)?;
    if resulting != proposed {
        return Err(Error::ResultingRosterMismatch);
    }
    loaded.local.advance(resulting)?;
    loaded.local.clear_retry_rows(loaded.provider.storage());
    finish(loaded, previous)
}
pub(crate) fn discard(
    context: Context,
    binding: MlsPendingCommitBinding,
) -> Result<Mutation, Error> {
    let mut loaded = load(context)?;
    if !loaded.group.is_active() {
        return Err(Error::InactiveGroup);
    }
    let pending = loaded
        .local
        .pending
        .as_ref()
        .ok_or(Error::PendingCommitMissing)?;
    if binding != pending.binding {
        return Err(Error::PendingBindingMismatch);
    }
    loaded
        .group
        .clear_pending_commit(loaded.provider.storage())
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    loaded.local.pending = None;
    loaded.local.clear_retry_rows(loaded.provider.storage());
    let previous = loaded.local.current.clone();
    finish(loaded, previous)
}
pub(crate) fn inspect_pending(context: Context) -> Result<Option<Prepared>, Error> {
    let loaded = load(context)?;
    let Some(pending) = &loaded.local.pending else {
        return Ok(None);
    };
    let (commit, welcome, info) = loaded.local.retry_bytes(loaded.provider.storage())?;
    Ok(Some(Prepared {
        binding: pending.binding.clone(),
        proposed: pending.proposed_roster.clone(),
        commit: commit.to_vec(),
        welcome: welcome.map(|x| x.to_vec()),
        group_info: info.map(|x| x.to_vec()),
        preparation_base: pending.preparation_base_sha256.clone(),
        mutation: Mutation {
            previous: loaded.local.current.clone(),
            resulting: loaded.local.current.clone(),
            resulting_digest: loaded.base,
            retention: loaded.local.retention,
            batch: MlsStorageBatch {
                upserts: vec![],
                deletes: vec![],
                deleted_group_ids: vec![],
                storage_format_version: 1,
            },
        },
    }))
}

pub(crate) struct Joined {
    pub local_leaf: MlsRosterLeafV1,
    pub consumed_key_package_sha256: Vec<u8>,
    pub mutation: Mutation,
}
pub(crate) struct JoinInput {
    pub config: MlsGroupConfig,
    pub incarnation_id: Vec<u8>,
    pub signer: Vec<u8>,
    pub welcome: Vec<u8>,
    pub welcome_sha256: Vec<u8>,
    pub tree: Option<Vec<u8>>,
    pub resulting: MlsExpectedRosterStateV1,
    pub local_leaf: MlsRosterLeafV1,
    pub target_key_package_sha256: Vec<u8>,
    pub entries: Vec<MlsStorageEntry>,
    pub storage_format: u32,
}
fn map_strict(e: StrictReceiveErrorKind) -> Error {
    match e {
        StrictReceiveErrorKind::StorageFormatMismatch => Error::StorageFormatMismatch,
        StrictReceiveErrorKind::InvalidStorageSnapshot => Error::InvalidStorageSnapshot,
        StrictReceiveErrorKind::GroupStateUnavailable => Error::GroupStateUnavailable,
        StrictReceiveErrorKind::ConfigurationMismatch => Error::ConfigurationMismatch,
        StrictReceiveErrorKind::GroupMismatch => Error::GroupMismatch,
        StrictReceiveErrorKind::PreviousEpochMismatch => Error::PreviousEpochMismatch,
        StrictReceiveErrorKind::PreviousRosterMismatch => Error::PreviousRosterMismatch,
        StrictReceiveErrorKind::ResultingEpochMismatch => Error::ResultingEpochMismatch,
        StrictReceiveErrorKind::ResultingRosterMismatch => Error::ResultingRosterMismatch,
        StrictReceiveErrorKind::AadMismatch => Error::AadMismatch,
        StrictReceiveErrorKind::MessageKindMismatch => Error::MessageKindMismatch,
        StrictReceiveErrorKind::LocalLeafMismatch => Error::LocalLeafMismatch,
        StrictReceiveErrorKind::InvalidSigner => Error::InvalidSigner,
        StrictReceiveErrorKind::UnsupportedCredential => Error::UnsupportedCredential,
        StrictReceiveErrorKind::MlsDecodeRejected => Error::MlsDecodeRejected,
        StrictReceiveErrorKind::WelcomeRejected => Error::WelcomeRejected,
        StrictReceiveErrorKind::MlsProtocolRejected => Error::MlsProtocolRejected,
        StrictReceiveErrorKind::ExpectedKeyPackageMismatch => Error::ExpectedKeyPackageMismatch,
        StrictReceiveErrorKind::InternalFailure => Error::InternalFailure,
    }
}
pub(crate) fn join(input: JoinInput) -> Result<Joined, Error> {
    let signer = Zeroizing::new(input.signer);
    let mut entries = Entries(input.entries);
    bounded_entries(&entries.0)?;
    if input.storage_format != 1 {
        return Err(Error::StorageFormatMismatch);
    }
    if input.welcome.is_empty()
        || input.welcome.len() > 1024 * 1024
        || input
            .tree
            .as_ref()
            .is_some_and(|t| t.len() > 2 * 1024 * 1024)
        || signer.len() > 4096
        || !(1..=128).contains(&input.incarnation_id.len())
        || input.resulting.group_id.len() != 16
        || input.resulting.digest_sha256.len() != 32
        || input.local_leaf.credential_identity.len() != 45
        || input.local_leaf.signature_public_key.len() != 32
        || input.welcome_sha256.len() != 32
    {
        return Err(Error::LimitExceeded);
    }
    if Sha256::digest(&input.welcome).as_slice() != input.welcome_sha256 {
        return Err(Error::WireHashMismatch);
    }
    let p = profile(input.config.max_past_epochs)?;
    if p.to_create_config() != input.config.to_create_config() {
        return Err(Error::ConfigurationMismatch);
    }
    let mut joined = join_group_from_welcome_with_storage_typed(
        input.config.clone(),
        input.welcome,
        input.tree,
        signer.to_vec(),
        input.resulting,
        Some(&input.target_key_package_sha256),
        entries.0.clone(),
        1,
    )
    .map_err(|e| map_strict(e.kind))?;
    // Own every secret-bearing returned row before any subsequent fallible step.
    let mut joined_rows = Entries(std::mem::take(&mut joined.joined.storage_batch.upserts));
    if joined.local_leaf != input.local_leaf {
        return Err(Error::LocalLeafMismatch);
    }
    // Import the verified join's complete mutation into a provider whose initial
    // state is the original global snapshot. The final diff also includes metadata.
    let provider = provider_from_entries(std::mem::take(&mut entries.0), 1, None)
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    for key in &joined.joined.storage_batch.deletes {
        provider.storage().delete_opaque(key);
    }
    for row in &mut joined_rows.0 {
        provider
            .storage()
            .write_opaque(std::mem::take(&mut row.key), std::mem::take(&mut row.value));
    }
    let group =
        load_group(&joined.joined.group_id, &provider).map_err(|_| Error::GroupStateUnavailable)?;
    let current = joined.joined.resulting_roster;
    let local = LocalGroupState::new(
        input.incarnation_id,
        input.config.max_past_epochs,
        current.clone(),
    )?;
    Ok(Joined {
        local_leaf: joined.local_leaf,
        consumed_key_package_sha256: joined.consumed_key_package_sha256,
        mutation: finish(
            Loaded {
                provider,
                group,
                local,
                base: vec![],
            },
            current,
        )?,
    })
}
pub(crate) struct Sent {
    pub ciphertext: Vec<u8>,
    pub mutation: Mutation,
}
pub(crate) fn send(
    context: Context,
    signer_bytes: Vec<u8>,
    plaintext: Vec<u8>,
    aad: Vec<u8>,
) -> Result<Sent, Error> {
    let signer = Zeroizing::new(signer_bytes);
    let plaintext = Zeroizing::new(plaintext);
    bounded_aad(&aad)?;
    if signer.len() > 4096 {
        return Err(Error::LimitExceeded);
    }
    if plaintext.len() > 256 * 1024 {
        return Err(Error::LimitExceeded);
    }
    let mut loaded = load(context)?;
    if !loaded.group.is_active() {
        return Err(Error::InactiveGroup);
    }
    if loaded.local.pending.is_some() {
        return Err(Error::PendingCommitExists);
    }
    let signer = signer_from_bytes(signer.to_vec()).map_err(|_| Error::InvalidSigner)?;
    ensure_local_signer(&loaded.group, &signer).map_err(|_| Error::InvalidSigner)?;
    loaded.group.set_aad(aad);
    let ciphertext = loaded
        .group
        .create_message(&loaded.provider, &signer, &plaintext)
        .map_err(|_| Error::MlsProtocolRejected)?
        .tls_serialize_detached()
        .map_err(|_| Error::InternalFailure)?;
    if ciphertext.len() > 1024 * 1024 {
        return Err(Error::LimitExceeded);
    }
    let previous = loaded.local.current.clone();
    Ok(Sent {
        ciphertext,
        mutation: finish(loaded, previous)?,
    })
}
pub(crate) struct ReceiveInput {
    pub context: Context,
    pub kind: MlsReceiveKind,
    pub wire: Vec<u8>,
    pub wire_sha256: Vec<u8>,
    pub aad: Vec<u8>,
    pub sender: MlsRosterLeafV1,
    pub message_state: MlsExpectedRosterStateV1,
    pub resulting_state: MlsExpectedRosterStateV1,
}
pub(crate) struct Received {
    pub kind: MlsReceiveKind,
    pub message_epoch: u64,
    pub sender: MlsRosterLeafV1,
    pub plaintext: Option<Vec<u8>>,
    pub proposal_type: Option<MlsProposalType>,
    pub mutation: Mutation,
}
pub(crate) fn receive(input: ReceiveInput) -> Result<Received, Error> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| receive_inner(input)))
        .map_err(|_| Error::InternalFailure)?
}
fn receive_inner(input: ReceiveInput) -> Result<Received, Error> {
    let mut loaded = load(input.context)?;
    if !loaded.group.is_active() {
        return Err(Error::InactiveGroup);
    }
    bounded_aad(&input.aad)?;
    if input.wire.is_empty() || input.wire.len() > 1024 * 1024 {
        return Err(Error::LimitExceeded);
    }
    if Sha256::digest(&input.wire).as_slice() != input.wire_sha256 {
        return Err(Error::WireHashMismatch);
    }
    let message = mls_message_from_exact_bytes(&input.wire)
        .map_err(|_| Error::MlsDecodeRejected)?
        .try_into_protocol_message()
        .map_err(|_| Error::MessageKindMismatch)?;
    if message.group_id().as_slice() != loaded.local.group_id
        || input.message_state.group_id != loaded.local.group_id
    {
        return Err(Error::GroupMismatch);
    }
    let message_epoch = message.epoch().as_u64();
    if message_epoch != input.message_state.epoch {
        return Err(Error::MessageEpochMismatch);
    }
    let previous = loaded.local.current.clone();
    if input.kind != MlsReceiveKind::Application {
        if message_epoch != previous.epoch {
            return Err(Error::RequiredCommitEpochMismatch);
        }
        if loaded.local.pending.is_some() {
            return Err(Error::PendingCommitExists);
        }
    }
    let historical = loaded.local.message_roster(message_epoch)?;
    if historical.digest_sha256 != input.message_state.digest_sha256 {
        return Err(Error::MessageRosterMismatch);
    }
    if !historical.leaves.contains(&input.sender) {
        return Err(Error::SenderMismatch);
    }
    if input.kind != MlsReceiveKind::Commit {
        check_state(&previous, &input.resulting_state, true)?;
    }
    let processed = loaded
        .group
        .process_message(&loaded.provider, message)
        .map_err(|e| match e {
            ProcessMessageError::ValidationError(ValidationError::NoPastEpochData) => {
                Error::InvalidStorageSnapshot
            }
            ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
                MessageDecryptionError::SecretTreeError(SecretTreeError::TooDistantInThePast),
            )) => Error::GenerationTooOld,
            ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
                MessageDecryptionError::SecretTreeError(SecretTreeError::TooDistantInTheFuture),
            )) => Error::ForwardDistanceExceeded,
            ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
                MessageDecryptionError::SecretTreeError(SecretTreeError::SecretReuseError),
            )) => Error::Replay,
            _ => Error::MlsProtocolRejected,
        })?;
    if processed.aad() != input.aad {
        zeroize_processed_content(processed);
        return Err(Error::AadMismatch);
    }
    if processed.epoch().as_u64() != message_epoch
        || processed.sender() != &Sender::Member(LeafNodeIndex::new(input.sender.leaf_index))
        || BasicCredential::try_from(processed.credential().clone())
            .map(|c| c.identity() != input.sender.credential_identity)
            .unwrap_or(true)
    {
        zeroize_processed_content(processed);
        return Err(Error::SenderMismatch);
    }
    let mut plaintext: Option<Zeroizing<Vec<u8>>> = None;
    let mut proposal = None;
    match (input.kind, processed.into_content()) {
        (MlsReceiveKind::Application, ProcessedMessageContent::ApplicationMessage(m)) => {
            let bytes = Zeroizing::new(m.into_bytes());
            if bytes.len() > 256 * 1024 {
                return Err(Error::LimitExceeded);
            }
            plaintext = Some(bytes);
        }
        (MlsReceiveKind::Commit, ProcessedMessageContent::StagedCommitMessage(c)) => {
            loaded
                .group
                .merge_staged_commit(&loaded.provider, *c)
                .map_err(|_| Error::MlsProtocolRejected)?;
            let resulting =
                roster_from_group(&loaded.group).map_err(|_| Error::UnsupportedCredential)?;
            check_state(&resulting, &input.resulting_state, true)?;
            loaded.local.advance(resulting)?;
        }
        (MlsReceiveKind::Proposal, ProcessedMessageContent::ProposalMessage(p)) => {
            proposal = Some(proposal_type(p.proposal()));
            loaded
                .group
                .store_pending_proposal(loaded.provider.storage(), *p)
                .map_err(|_| Error::InvalidStorageSnapshot)?;
        }
        (_, ProcessedMessageContent::ApplicationMessage(m)) => {
            let _wipe = Zeroizing::new(m.into_bytes());
            return Err(Error::MessageKindMismatch);
        }
        _ => return Err(Error::MessageKindMismatch),
    }
    let mutation = finish(loaded, previous)?;
    Ok(Received {
        kind: input.kind,
        message_epoch,
        sender: input.sender,
        plaintext: plaintext.map(|mut p| std::mem::take(&mut *p)),
        proposal_type: proposal,
        mutation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::keys::{MlsSignatureKeyPair, serialize_signer};
    use crate::api::storage::create_key_package_with_storage;
    const GID: &[u8] = &[54; 16];
    const AAD: &[u8] = b"production-lifecycle-aad";
    struct Peer {
        signer: Zeroizing<Vec<u8>>,
        id: Vec<u8>,
        public: Vec<u8>,
        rows: Entries,
        roster: MlsRosterSummaryV1,
        retention: u32,
    }
    fn expected(r: &MlsRosterSummaryV1) -> MlsExpectedRosterStateV1 {
        MlsExpectedRosterStateV1 {
            group_id: r.group_id.clone(),
            epoch: r.epoch,
            digest_sha256: r.digest_sha256.clone(),
        }
    }
    fn peer(id: u8, retention: u32) -> Peer {
        let k = MlsSignatureKeyPair::generate(profile(retention).unwrap().ciphersuite).unwrap();
        let public = k.public_key();
        let signer = Zeroizing::new(
            serialize_signer(
                profile(retention).unwrap().ciphersuite,
                k.private_key(),
                public.clone(),
            )
            .unwrap(),
        );
        Peer {
            signer,
            id: vec![id; 45],
            public,
            rows: Entries(vec![]),
            roster: MlsRosterSummaryV1 {
                group_id: GID.to_vec(),
                epoch: 0,
                leaves: vec![],
                digest_sha256: vec![],
            },
            retention,
        }
    }
    fn apply(p: &mut Peer, b: &MlsStorageBatch) {
        let mut rows: BTreeMap<_, _> = std::mem::take(&mut p.rows.0)
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        for key in &b.deletes {
            if let Some(mut row) = rows.remove(key) {
                row.value.zeroize();
            }
        }
        for r in &b.upserts {
            if let Some(mut old) = rows.insert(r.key.clone(), r.clone()) {
                old.value.zeroize();
            }
        }
        p.rows.0 = rows.into_values().collect();
    }
    fn mutate(p: &mut Peer, m: &Mutation) {
        apply(p, &m.batch);
        p.roster = m.resulting.clone();
        assert_eq!(
            group_state_digest_from_entries(GID, &p.rows.0, 1).unwrap(),
            m.resulting_digest
        );
    }
    fn apply_cas(p: &mut Peer, base: &[u8], m: &Mutation) -> Result<(), Error> {
        if group_state_digest_from_entries(GID, &p.rows.0, 1).unwrap() != base {
            return Err(Error::BaseStateMismatch);
        }
        mutate(p, m);
        Ok(())
    }
    fn context(p: &Peer) -> Context {
        Context {
            group_id: GID.to_vec(),
            incarnation_id: vec![8; 16],
            expected_current: expected(&p.roster),
            expected_base_sha256: group_state_digest_from_entries(GID, &p.rows.0, 1).unwrap(),
            expected_retention: p.retention,
            entries: p.rows.0.clone(),
            storage_format: 1,
        }
    }
    fn acceptance(p: &Prepared) -> MlsCommitAcceptance {
        MlsCommitAcceptance {
            binding: p.binding.clone(),
            resulting_state: expected(&p.proposed),
            preparation_base_group_state_sha256: p.preparation_base.clone(),
        }
    }
    fn transition(n: u8) -> MlsTransitionContext {
        MlsTransitionContext {
            command_id: vec![n; 16],
            context_sha256: vec![n; 32],
        }
    }
    fn pair(retention: u32) -> (Peer, Peer) {
        pair_with_retention(retention, retention)
    }
    fn pair_with_retention(retention: u32, receiver_retention: u32) -> (Peer, Peer) {
        let mut a = peer(1, retention);
        let c = create(
            profile(retention).unwrap(),
            a.signer.to_vec(),
            GID.to_vec(),
            vec![8; 16],
            MlsAuthorizedOwnerV1 {
                expected_credential_identity: a.id.clone(),
                expected_signature_public_key: a.public.clone(),
            },
            None,
            vec![],
            1,
        )
        .unwrap();
        mutate(&mut a, &c);
        let mut b = peer(2, receiver_retention);
        let kp = create_key_package_with_storage(
            profile(retention).unwrap().ciphersuite,
            b.signer.to_vec(),
            b.id.clone(),
            b.public.clone(),
            None,
            vec![],
            1,
        )
        .unwrap();
        apply(&mut b, &kp.storage_batch);
        let kp_hash = Sha256::digest(&kp.key_package_bytes).to_vec();
        let p = prepare(
            context(&a),
            a.signer.to_vec(),
            transition(1),
            vec![MlsAuthorizedKeyPackageV1 {
                key_package_bytes: kp.key_package_bytes,
                expected_credential_identity: b.id.clone(),
                expected_signature_public_key: b.public.clone(),
            }],
            vec![],
            AAD.to_vec(),
            None,
        )
        .unwrap();
        mutate(&mut a, &p.mutation);
        let merged = merge(context(&a), acceptance(&p)).unwrap();
        mutate(&mut a, &merged);
        let welcome = p.welcome.unwrap();
        let j = join(JoinInput {
            config: profile(receiver_retention).unwrap(),
            incarnation_id: vec![8; 16],
            signer: b.signer.to_vec(),
            welcome_sha256: Sha256::digest(&welcome).to_vec(),
            welcome,
            tree: None,
            resulting: expected(&a.roster),
            local_leaf: a
                .roster
                .leaves
                .iter()
                .find(|l| l.credential_identity == b.id)
                .unwrap()
                .clone(),
            target_key_package_sha256: kp_hash,
            entries: b.rows.0.clone(),
            storage_format: 1,
        })
        .unwrap();
        mutate(&mut b, &j.mutation);
        (a, b)
    }
    fn self_prepare(p: &Peer, n: u8) -> Prepared {
        let leaf = p
            .roster
            .leaves
            .iter()
            .find(|l| l.credential_identity == p.id)
            .unwrap();
        prepare(
            context(p),
            p.signer.to_vec(),
            transition(n),
            vec![],
            vec![],
            AAD.to_vec(),
            Some(MlsAuthorizedSelfV1 {
                leaf_index: leaf.leaf_index,
                expected_credential_identity: leaf.credential_identity.clone(),
                expected_signature_public_key: leaf.signature_public_key.clone(),
            }),
        )
        .unwrap()
    }
    fn incoming(
        p: &Peer,
        wire: Vec<u8>,
        kind: MlsReceiveKind,
        sender: MlsRosterLeafV1,
        message_state: MlsExpectedRosterStateV1,
        resulting_state: MlsExpectedRosterStateV1,
    ) -> ReceiveInput {
        ReceiveInput {
            context: context(p),
            kind,
            wire_sha256: Sha256::digest(&wire).to_vec(),
            wire,
            aad: AAD.to_vec(),
            sender,
            message_state,
            resulting_state,
        }
    }
    #[test]
    fn native_v2_shared_vectors() {
        use crate::native_receive_v2::*;
        use serde_json::json;
        use std::{fs, path::PathBuf};
        let output =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../native/receive_v2/fixtures");
        let generating = std::env::var_os("MLS_EXPORT_NATIVE_V2").is_some();
        let hash = |bytes: &[u8]| -> String {
            Sha256::digest(bytes)
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect()
        };
        if !generating {
            let manifest: serde_json::Value = serde_json::from_slice(
                &fs::read(output.join("manifest.json"))
                    .expect("generate v2 fixtures through Makefile first"),
            )
            .unwrap();
            assert_eq!(manifest["contract_version"], 2);
            assert_eq!(manifest["profile_id"], 2);
            assert_eq!(manifest["storage_format_version"], 1);
            assert_eq!(manifest["synthetic_secrets_only"], true);
            for record in manifest["vectors"].as_array().unwrap() {
                let mut request = Zeroizing::new(
                    fs::read(output.join(record["request_file"].as_str().unwrap())).unwrap(),
                );
                let response = Zeroizing::new(
                    fs::read(output.join(record["response_file"].as_str().unwrap())).unwrap(),
                );
                assert_eq!(hash(&request), record["request_sha256"]);
                assert_eq!(hash(&response), record["response_sha256"]);
                assert_eq!(
                    execute_native_receive_v2(&request),
                    *response,
                    "{}",
                    record["id"]
                );
                request.zeroize();
            }
            return;
        }
        fn leaf(v: &MlsRosterLeafV1) -> NativeLeafAuthorityV2 {
            NativeLeafAuthorityV2 {
                leaf_index: v.leaf_index,
                credential_identity: v.credential_identity.clone(),
                signature_public_key: v.signature_public_key.clone(),
            }
        }
        fn roster(v: &MlsExpectedRosterStateV1) -> NativeExpectedRosterStateV2 {
            NativeExpectedRosterStateV2 {
                group_id: v.group_id.clone(),
                epoch: v.epoch,
                digest_sha256: v.digest_sha256.clone(),
            }
        }
        fn rows(v: &[MlsStorageEntry]) -> NativeStorageSnapshotV2 {
            NativeStorageSnapshotV2 {
                storage_format_version: 1,
                entries: v
                    .iter()
                    .map(|r| NativeStorageEntryV2 {
                        key: r.key.clone(),
                        value: r.value.clone(),
                        group_id: r.group_id.clone(),
                    })
                    .collect(),
            }
        }
        fn request(input: ReceiveInput) -> NativeReceiveRequestV2 {
            NativeReceiveRequestV2::Process {
                operation: if input.kind == MlsReceiveKind::Commit {
                    NativeReceiveOperationV2::Commit
                } else {
                    NativeReceiveOperationV2::Application
                },
                profile_id: 2,
                group_id: input.context.group_id.clone(),
                message_bytes: input.wire,
                expected_aad: input.aad,
                expected_sender: leaf(&input.sender),
                expected_previous_state: roster(&input.context.expected_current),
                expected_resulting_state: roster(&input.resulting_state),
                expected_base_group_state_sha256: input.context.expected_base_sha256.clone(),
                storage: rows(&input.context.entries),
                expected_message_state: roster(&input.message_state),
                expected_retention: input.context.expected_retention,
                expected_message_sha256: input.wire_sha256,
                incarnation_id: input.context.incarnation_id.clone(),
            }
        }
        let mut records = vec![];
        fs::create_dir_all(&output).unwrap();
        let mut write = |id: &str, mut bytes: Vec<u8>, error: Option<Error>| {
            let result = Zeroizing::new(execute_native_receive_v2(&bytes));
            let operation = bytes[6];
            if let Some(error) = error {
                let op = match operation {
                    1 => NativeReceiveOperationV2::Application,
                    2 => NativeReceiveOperationV2::Commit,
                    3 => NativeReceiveOperationV2::Welcome,
                    _ => panic!("invalid fixture kind"),
                };
                assert_eq!(
                    *result,
                    encode_native_receive_failure_v2(Some(op), error),
                    "{id}"
                );
            } else {
                let mut d = minicbor::Decoder::new(&result[12..]);
                assert_eq!(d.map().unwrap(), Some(3));
                assert_eq!(d.u8().unwrap(), 0);
                assert_eq!(d.u16().unwrap(), 2);
                assert_eq!(d.u8().unwrap(), 1);
                assert!(!d.bool().unwrap());
                assert_eq!(d.u8().unwrap(), 2, "fixture unexpectedly failed: {id}");
            }
            let request_file = format!("{id}.request.bin");
            let response_file = format!("{id}.response.bin");
            fs::write(output.join(&request_file), &bytes).unwrap();
            fs::write(output.join(&response_file), &*result).unwrap();
            records.push(json!({"id":id,"operation":operation,"request_file":request_file,
                "response_file":response_file,"request_sha256":hash(&bytes),"response_sha256":hash(&result),
                "expected_error_code":error.map(|e|e as u16)}));
            bytes.zeroize();
        };
        let mut a = peer(1, 2);
        let initialized = create(
            profile(2).unwrap(),
            a.signer.to_vec(),
            GID.to_vec(),
            vec![8; 16],
            MlsAuthorizedOwnerV1 {
                expected_credential_identity: a.id.clone(),
                expected_signature_public_key: a.public.clone(),
            },
            None,
            vec![],
            1,
        )
        .unwrap();
        mutate(&mut a, &initialized);
        let mut b = peer(2, 4);
        let kp = create_key_package_with_storage(
            profile(4).unwrap().ciphersuite,
            b.signer.to_vec(),
            b.id.clone(),
            b.public.clone(),
            None,
            vec![],
            1,
        )
        .unwrap();
        apply(&mut b, &kp.storage_batch);
        let kp_hash = Sha256::digest(&kp.key_package_bytes).to_vec();
        let add = prepare(
            context(&a),
            a.signer.to_vec(),
            transition(1),
            vec![MlsAuthorizedKeyPackageV1 {
                key_package_bytes: kp.key_package_bytes,
                expected_credential_identity: b.id.clone(),
                expected_signature_public_key: b.public.clone(),
            }],
            vec![],
            AAD.to_vec(),
            None,
        )
        .unwrap();
        mutate(&mut a, &add.mutation);
        let merged = merge(context(&a), acceptance(&add)).unwrap();
        mutate(&mut a, &merged);
        let welcome = add.welcome.unwrap();
        let mut welcome_request = NativeReceiveRequestV2::Welcome {
            profile_id: 2,
            welcome_bytes: welcome.clone(),
            expected_welcome_sha256: Sha256::digest(&welcome).to_vec(),
            ratchet_tree_bytes: None,
            signer_bytes: b.signer.to_vec(),
            expected_local_leaf: leaf(&a.roster.leaves[1]),
            expected_resulting_state: roster(&expected(&a.roster)),
            expected_target_key_package_sha256: kp_hash.clone(),
            storage: rows(&b.rows.0),
            retention: 4,
            incarnation_id: vec![8; 16],
        };
        write(
            "welcome_success",
            encode_native_receive_request_v2(&welcome_request).unwrap(),
            None,
        );
        if let NativeReceiveRequestV2::Welcome {
            expected_target_key_package_sha256,
            ..
        } = &mut welcome_request
        {
            expected_target_key_package_sha256[0] ^= 1;
        }
        write(
            "welcome_wrong_key_package",
            encode_native_receive_request_v2(&welcome_request).unwrap(),
            Some(Error::ExpectedKeyPackageMismatch),
        );
        let joined = join(JoinInput {
            config: profile(4).unwrap(),
            incarnation_id: vec![8; 16],
            signer: b.signer.to_vec(),
            welcome_sha256: Sha256::digest(&welcome).to_vec(),
            welcome,
            tree: None,
            resulting: expected(&a.roster),
            local_leaf: a.roster.leaves[1].clone(),
            target_key_package_sha256: kp_hash,
            entries: b.rows.0.clone(),
            storage_format: 1,
        })
        .unwrap();
        mutate(&mut b, &joined.mutation);
        let current = expected(&a.roster);
        let author = a.roster.leaves[0].clone();
        let app = send(
            context(&a),
            a.signer.to_vec(),
            b"shared v2 fixture".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        mutate(&mut a, &app.mutation);
        let base_request = request(incoming(
            &b,
            app.ciphertext.clone(),
            MlsReceiveKind::Application,
            author.clone(),
            current.clone(),
            current.clone(),
        ));
        let base_frame = Zeroizing::new(encode_native_receive_request_v2(&base_request).unwrap());
        write("application_success", base_frame.to_vec(), None);
        for (id, error) in [
            ("application_wrong_aad", Error::AadMismatch),
            ("application_wrong_sender", Error::SenderMismatch),
            ("application_wrong_roster", Error::MessageRosterMismatch),
            ("application_wrong_base", Error::BaseStateMismatch),
            ("application_wrong_hash", Error::WireHashMismatch),
            ("application_wrong_kind", Error::MessageKindMismatch),
            ("application_wrong_retention", Error::ConfigurationMismatch),
        ] {
            let mut changed = decode_native_receive_request_v2(&base_frame).unwrap();
            if let NativeReceiveRequestV2::Process {
                expected_aad,
                expected_sender,
                expected_message_state,
                expected_base_group_state_sha256,
                expected_message_sha256,
                operation,
                expected_retention,
                ..
            } = &mut changed
            {
                match error {
                    Error::AadMismatch => expected_aad.push(0),
                    Error::SenderMismatch => expected_sender.signature_public_key[0] ^= 1,
                    Error::MessageRosterMismatch => expected_message_state.digest_sha256[0] ^= 1,
                    Error::BaseStateMismatch => expected_base_group_state_sha256[0] ^= 1,
                    Error::WireHashMismatch => expected_message_sha256[0] ^= 1,
                    Error::MessageKindMismatch => *operation = NativeReceiveOperationV2::Commit,
                    Error::ConfigurationMismatch => *expected_retention = 2,
                    _ => unreachable!(),
                }
            }
            write(
                id,
                encode_native_receive_request_v2(&changed).unwrap(),
                Some(error),
            );
        }
        let mut empty = decode_native_receive_request_v2(&base_frame).unwrap();
        if let NativeReceiveRequestV2::Process { expected_aad, .. } = &mut empty {
            expected_aad.clear();
        }
        write(
            "application_empty_aad",
            encode_native_receive_request_v2_allow_empty_aad_fixture(&empty).unwrap(),
            Some(Error::LimitExceeded),
        );
        let mut old = base_frame.to_vec();
        old[5] = 1;
        write(
            "v1_frame_rejected",
            old,
            Some(Error::UnsupportedContractVersion),
        );
        let p = self_prepare(&a, 2);
        mutate(&mut a, &p.mutation);
        let commit_request = incoming(
            &b,
            p.commit.clone(),
            MlsReceiveKind::Commit,
            author.clone(),
            current.clone(),
            expected(&p.proposed),
        );
        write(
            "commit_success",
            encode_native_receive_request_v2(&request(incoming(
                &b,
                p.commit.clone(),
                MlsReceiveKind::Commit,
                author.clone(),
                current.clone(),
                expected(&p.proposed),
            )))
            .unwrap(),
            None,
        );
        let received = receive(commit_request).unwrap();
        mutate(&mut b, &received.mutation);
        write(
            "application_historical",
            encode_native_receive_request_v2(&request(incoming(
                &b,
                app.ciphertext,
                MlsReceiveKind::Application,
                author,
                current,
                expected(&b.roster),
            )))
            .unwrap(),
            None,
        );
        let manifest = json!({"generated_date":"2026-09-28","contract_version":2,"profile_id":2,
            "storage_format_version":1,"synthetic_secrets_only":true,"vectors":records});
        fs::write(
            output.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn public_pending_api_requires_immediate_persist_and_explicit_settlement() {
        fn public_context(p: &Peer) -> MlsGroupOperationContext {
            MlsGroupOperationContext {
                group_id: GID.to_vec(),
                incarnation_id: vec![8; 16],
                expected_current_state: expected(&p.roster),
                expected_base_group_state_sha256: group_state_digest_from_entries(
                    GID, &p.rows.0, 1,
                )
                .unwrap(),
                expected_retention: p.retention,
                storage_entries: p.rows.0.clone(),
                storage_format_version: 1,
            }
        }
        let (mut a, b) = pair(2);
        let before = a.roster.clone();
        let own = before.leaves[0].clone();
        let outcome = self_update_with_storage(
            public_context(&a),
            transition(29),
            a.signer.to_vec(),
            MlsAuthorizedSelfV1 {
                leaf_index: own.leaf_index,
                expected_credential_identity: own.credential_identity,
                expected_signature_public_key: own.signature_public_key,
            },
            AAD.to_vec(),
        );
        let SelfUpdateWithStorageOutcome::Success(p) = outcome else {
            panic!("prepare failed")
        };
        assert_eq!(p.previous_roster, before);
        assert_eq!(p.proposed_resulting_roster.epoch, before.epoch + 1);
        // This persists pending state, not the proposed epoch/roster.
        apply(&mut a, &p.storage_batch);
        assert_eq!(load(context(&a)).unwrap().local.current, before);
        let GetPendingCommitWithStorageOutcome::Success(Some(info)) =
            get_pending_commit_with_storage(public_context(&a))
        else {
            panic!("pending missing")
        };
        assert_eq!(info.commit, p.commit);
        assert_eq!(info.pending_binding, p.pending_binding);
        assert!(matches!(
            crate::api::storage::create_message_with_storage(
                public_context(&a),
                a.signer.to_vec(),
                vec![1],
                AAD.to_vec()
            ),
            crate::api::storage::CreateMessageWithStorageOutcome::Failure(
                Error::PendingCommitExists
            )
        ));
        let MergePendingCommitWithStorageOutcome::Success(m) = merge_pending_commit_with_storage(
            public_context(&a),
            MlsCommitAcceptance {
                binding: p.pending_binding.clone(),
                resulting_state: expected(&p.proposed_resulting_roster),
                preparation_base_group_state_sha256: p.preparation_base_group_state_sha256,
            },
        ) else {
            panic!("merge failed")
        };
        apply(&mut a, &m.storage_batch);
        a.roster = m.resulting_roster;
        assert!(matches!(
            get_pending_commit_with_storage(public_context(&a)),
            GetPendingCommitWithStorageOutcome::Success(None)
        ));
        assert!(matches!(
            discard_pending_commit_with_storage(public_context(&a), p.pending_binding),
            DiscardPendingCommitWithStorageOutcome::Failure(Error::PendingCommitMissing)
        ));
        let ProcessMessageWithStorageOutcome::Success(r) = process_message_with_storage(
            public_context(&b),
            MlsReceiveKind::Commit,
            p.commit.clone(),
            Sha256::digest(&p.commit).to_vec(),
            AAD.to_vec(),
            before.leaves[0].clone(),
            expected(&before),
            expected(&a.roster),
        ) else {
            panic!("public receive failed")
        };
        assert_eq!(r.message_epoch, before.epoch);
        assert_eq!(r.resulting_epoch, before.epoch + 1);
        assert_eq!(r.effective_retention, 2);
        assert!(r.application_message.is_none());
    }

    #[test]
    fn native_v2_preserves_current_and_historical_authority_and_errors() {
        use crate::native_receive_v2::*;
        fn leaf(v: &MlsRosterLeafV1) -> NativeLeafAuthorityV2 {
            NativeLeafAuthorityV2 {
                leaf_index: v.leaf_index,
                credential_identity: v.credential_identity.clone(),
                signature_public_key: v.signature_public_key.clone(),
            }
        }
        fn state(v: &MlsExpectedRosterStateV1) -> NativeExpectedRosterStateV2 {
            NativeExpectedRosterStateV2 {
                group_id: v.group_id.clone(),
                epoch: v.epoch,
                digest_sha256: v.digest_sha256.clone(),
            }
        }
        fn frame(v: &ReceiveInput) -> Vec<u8> {
            encode_native_receive_request_v2(&NativeReceiveRequestV2::Process {
                operation: match v.kind {
                    MlsReceiveKind::Application => NativeReceiveOperationV2::Application,
                    MlsReceiveKind::Commit => NativeReceiveOperationV2::Commit,
                    _ => unreachable!(),
                },
                profile_id: 2,
                group_id: v.context.group_id.clone(),
                message_bytes: v.wire.clone(),
                expected_aad: v.aad.clone(),
                expected_sender: leaf(&v.sender),
                expected_previous_state: state(&v.context.expected_current),
                expected_resulting_state: state(&v.resulting_state),
                expected_base_group_state_sha256: v.context.expected_base_sha256.clone(),
                expected_message_state: state(&v.message_state),
                expected_retention: v.context.expected_retention,
                expected_message_sha256: v.wire_sha256.clone(),
                incarnation_id: v.context.incarnation_id.clone(),
                storage: NativeStorageSnapshotV2 {
                    storage_format_version: 1,
                    entries: v
                        .context
                        .entries
                        .iter()
                        .map(|r| NativeStorageEntryV2 {
                            key: r.key.clone(),
                            value: r.value.clone(),
                            group_id: r.group_id.clone(),
                        })
                        .collect(),
                },
            })
            .unwrap()
        }
        fn check_success(frame: &[u8], epoch: u64, retention: u32, app: bool) {
            let encoded = execute_native_receive_v2(frame);
            assert_eq!(&encoded[..6], b"KMLS\0\x02");
            let mut d = minicbor::Decoder::new(&encoded[12..]);
            assert_eq!(d.map().unwrap(), Some(3));
            assert_eq!((d.u8().unwrap(), d.u16().unwrap()), (0, 2));
            assert_eq!(d.u8().unwrap(), 1);
            assert!(!d.bool().unwrap());
            assert_eq!(d.u8().unwrap(), 2);
            assert_eq!(d.map().unwrap(), Some(if app { 8 } else { 7 }));
            for key in 0..if app { 6 } else { 5 } {
                assert_eq!(d.u8().unwrap(), key);
                d.skip().unwrap();
            }
            assert_eq!(d.u8().unwrap(), if app { 6 } else { 5 });
            assert_eq!(d.u64().unwrap(), epoch);
            assert_eq!(d.u8().unwrap(), if app { 7 } else { 6 });
            assert_eq!(d.u32().unwrap(), retention);
            assert_eq!(d.position(), encoded.len() - 12);
        }
        let (mut a, mut b) = pair(2);
        let current = expected(&a.roster);
        let author = a.roster.leaves[0].clone();
        let old = send(
            context(&a),
            a.signer.to_vec(),
            b"historical native".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        mutate(&mut a, &old.mutation);
        let request = incoming(
            &b,
            old.ciphertext.clone(),
            MlsReceiveKind::Application,
            author.clone(),
            current.clone(),
            current.clone(),
        );
        let bytes = frame(&request);
        assert_eq!(
            encode_native_receive_request_v2(&decode_native_receive_request_v2(&bytes).unwrap())
                .unwrap(),
            bytes
        );
        check_success(&bytes, current.epoch, 2, true);
        let mut bad_version = bytes.clone();
        bad_version[5] = 1;
        assert_eq!(
            execute_native_receive_v2(&bad_version),
            encode_native_receive_failure_v2(
                Some(NativeReceiveOperationV2::Application),
                Error::UnsupportedContractVersion
            )
        );
        let mut bad = request;
        bad.wire_sha256[0] ^= 1;
        assert_eq!(
            execute_native_receive_v2(&frame(&bad)),
            encode_native_receive_failure_v2(
                Some(NativeReceiveOperationV2::Application),
                Error::WireHashMismatch
            )
        );
        let p = self_prepare(&a, 25);
        mutate(&mut a, &p.mutation);
        let commit = incoming(
            &b,
            p.commit.clone(),
            MlsReceiveKind::Commit,
            author.clone(),
            current.clone(),
            expected(&p.proposed),
        );
        check_success(&frame(&commit), current.epoch, 2, false);
        let received = receive(commit).unwrap();
        mutate(&mut b, &received.mutation);
        let historical = incoming(
            &b,
            old.ciphertext,
            MlsReceiveKind::Application,
            author,
            current.clone(),
            expected(&b.roster),
        );
        check_success(&frame(&historical), current.epoch, 2, true);
        let mut decoded = decode_native_receive_request_v2(&frame(&historical)).unwrap();
        if let NativeReceiveRequestV2::Process { expected_aad, .. } = &mut decoded {
            expected_aad.clear();
        }
        assert!(matches!(
            encode_native_receive_request_v2(&decoded),
            Err(Error::LimitExceeded)
        ));
        let empty_aad = encode_native_receive_request_v2_allow_empty_aad_fixture(&decoded).unwrap();
        assert_eq!(
            execute_native_receive_v2(&empty_aad),
            encode_native_receive_failure_v2(
                Some(NativeReceiveOperationV2::Application),
                Error::LimitExceeded
            )
        );
    }

    #[test]
    fn ratchet_limits_remain_five_and_one_thousand_without_advancing_on_failure() {
        let (mut a, b) = pair(2);
        let mut loaded = load(context(&b)).unwrap();
        let signer = signer_from_bytes(b.signer.to_vec()).unwrap();
        let mut first = None;
        let mut near = None;
        let mut far = None;
        // Generate a real out-of-window ciphertext without repeatedly copying
        // the sender snapshot. This is a protocol regression, not a benchmark.
        for generation in 0..=1001 {
            loaded.group.set_aad(AAD.to_vec());
            let wire = loaded
                .group
                .create_message(&loaded.provider, &signer, b"ratchet")
                .unwrap()
                .tls_serialize_detached()
                .unwrap();
            match generation {
                0 => first = Some(wire),
                7 => near = Some(wire),
                1001 => far = Some(wire),
                _ => {}
            }
        }
        let sender = b.roster.leaves[1].clone();
        let input = |peer: &Peer, wire| {
            incoming(
                peer,
                wire,
                MlsReceiveKind::Application,
                sender.clone(),
                expected(&b.roster),
                expected(&peer.roster),
            )
        };
        assert!(matches!(
            receive(input(&a, far.unwrap())),
            Err(Error::ForwardDistanceExceeded)
        ));
        let first = first.unwrap();
        // Rejection did not burn generation zero in the caller's state.
        assert!(receive(input(&a, first.clone())).is_ok());
        let received = receive(input(&a, near.unwrap())).unwrap();
        mutate(&mut a, &received.mutation);
        assert!(matches!(
            receive(input(&a, first)),
            Err(Error::GenerationTooOld)
        ));
    }

    #[test]
    fn stored_proposals_are_not_silently_committed_by_self_update() {
        let (mut a, mut b) = pair(2);
        // A valid current-epoch proposal from B removes A. Preparing A's own
        // authorized self-update must not implicitly adopt this proposal.
        let mut loaded = load(context(&b)).unwrap();
        let signer = signer_from_bytes(b.signer.to_vec()).unwrap();
        loaded.group.set_aad(AAD.to_vec());
        let (proposal, _) = loaded
            .group
            .propose_remove_member(&loaded.provider, &signer, LeafNodeIndex::new(0))
            .unwrap();
        let proposal = proposal.tls_serialize_detached().unwrap();
        let before = b.roster.clone();
        mutate(&mut b, &finish(loaded, before).unwrap());
        let received = receive(incoming(
            &a,
            proposal.clone(),
            MlsReceiveKind::Proposal,
            b.roster.leaves[1].clone(),
            expected(&b.roster),
            expected(&a.roster),
        ))
        .unwrap();
        assert!(matches!(
            received.proposal_type,
            Some(MlsProposalType::Remove)
        ));
        mutate(&mut a, &received.mutation);
        let p = self_prepare(&a, 4);
        assert_eq!(p.proposed.leaves, a.roster.leaves);
        mutate(&mut a, &p.mutation);
        assert!(matches!(
            receive(incoming(
                &a,
                proposal,
                MlsReceiveKind::Proposal,
                b.roster.leaves[1].clone(),
                expected(&b.roster),
                expected(&a.roster)
            )),
            Err(Error::PendingCommitExists)
        ));
        let merged = merge(context(&a), acceptance(&p)).unwrap();
        mutate(&mut a, &merged);
        let received = receive(incoming(
            &b,
            p.commit,
            MlsReceiveKind::Commit,
            p.binding.author,
            p.binding.previous_state,
            expected(&p.proposed),
        ))
        .unwrap();
        mutate(&mut b, &received.mutation);
        assert_eq!(a.roster, b.roster);
        assert!(load(context(&a)).unwrap().group.is_active());
    }

    #[test]
    fn receive_authority_failures_do_not_consume_application_generation() {
        let (mut a, b) = pair(2);
        let s = send(
            context(&b),
            b.signer.to_vec(),
            b"exact plaintext".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        let request = || {
            incoming(
                &a,
                s.ciphertext.clone(),
                MlsReceiveKind::Application,
                b.roster.leaves[1].clone(),
                expected(&b.roster),
                expected(&a.roster),
            )
        };
        let mut bad = request();
        bad.aad.push(0);
        assert!(matches!(receive(bad), Err(Error::AadMismatch)));
        let mut bad = request();
        bad.wire_sha256[0] ^= 1;
        assert!(matches!(receive(bad), Err(Error::WireHashMismatch)));
        let mut bad = request();
        bad.kind = MlsReceiveKind::Commit;
        assert!(matches!(receive(bad), Err(Error::MessageKindMismatch)));
        let mut bad = request();
        bad.sender = a.roster.leaves[0].clone();
        assert!(matches!(receive(bad), Err(Error::SenderMismatch)));
        let mut bad = request();
        bad.resulting_state.epoch += 1;
        assert!(matches!(receive(bad), Err(Error::ResultingEpochMismatch)));
        let received = receive(request()).unwrap();
        assert_eq!(
            received.plaintext.as_deref(),
            Some(b"exact plaintext".as_slice())
        );
        mutate(&mut a, &received.mutation);
    }

    #[test]
    fn atomic_swap_installs_replacement_and_revokes_removed_local_group() {
        let (mut a, mut b) = pair(2);
        // Same installation identity may be atomically replaced with a distinct
        // signature key only when the exact old leaf is explicitly removed.
        let mut c = peer(2, 4);
        let kp = create_key_package_with_storage(
            profile(4).unwrap().ciphersuite,
            c.signer.to_vec(),
            c.id.clone(),
            c.public.clone(),
            None,
            vec![],
            1,
        )
        .unwrap();
        apply(&mut c, &kp.storage_batch);
        let kp_hash = Sha256::digest(&kp.key_package_bytes).to_vec();
        let old_leaf = b.roster.leaves[1].clone();
        let p = prepare(
            context(&a),
            a.signer.to_vec(),
            transition(8),
            vec![MlsAuthorizedKeyPackageV1 {
                key_package_bytes: kp.key_package_bytes,
                expected_credential_identity: c.id.clone(),
                expected_signature_public_key: c.public.clone(),
            }],
            vec![MlsAuthorizedRemovalV1 {
                leaf_index: old_leaf.leaf_index,
                expected_credential_identity: old_leaf.credential_identity,
                expected_signature_public_key: old_leaf.signature_public_key,
            }],
            AAD.to_vec(),
            None,
        )
        .unwrap();
        assert_eq!(p.proposed.leaves.len(), 2);
        assert!(
            !p.proposed
                .leaves
                .iter()
                .any(|leaf| leaf.signature_public_key == b.public)
        );
        mutate(&mut a, &p.mutation);
        let mut bad = acceptance(&p);
        bad.preparation_base_group_state_sha256[0] ^= 1;
        assert!(matches!(
            merge(context(&a), bad),
            Err(Error::AcceptanceBindingMismatch)
        ));
        let received = receive(incoming(
            &b,
            p.commit.clone(),
            MlsReceiveKind::Commit,
            p.binding.author.clone(),
            p.binding.previous_state.clone(),
            expected(&p.proposed),
        ))
        .unwrap();
        mutate(&mut b, &received.mutation);
        assert!(matches!(
            send(context(&b), b.signer.to_vec(), vec![1], AAD.to_vec()),
            Err(Error::InactiveGroup)
        ));
        let merged = merge(context(&a), acceptance(&p)).unwrap();
        mutate(&mut a, &merged);
        let welcome = p.welcome.unwrap();
        let make_join = || JoinInput {
            config: profile(4).unwrap(),
            incarnation_id: vec![8; 16],
            signer: c.signer.to_vec(),
            welcome: welcome.clone(),
            welcome_sha256: Sha256::digest(&welcome).to_vec(),
            tree: None,
            resulting: expected(&a.roster),
            local_leaf: a
                .roster
                .leaves
                .iter()
                .find(|l| l.signature_public_key == c.public)
                .unwrap()
                .clone(),
            target_key_package_sha256: kp_hash.clone(),
            entries: c.rows.0.clone(),
            storage_format: 1,
        };
        let mut bad = make_join();
        bad.target_key_package_sha256[0] ^= 1;
        assert!(matches!(join(bad), Err(Error::ExpectedKeyPackageMismatch)));
        let joined = join(make_join()).unwrap();
        assert_eq!(joined.consumed_key_package_sha256, kp_hash);
        {
            use crate::native_receive_v2::*;
            let input = make_join();
            let leaf = |v: &MlsRosterLeafV1| NativeLeafAuthorityV2 {
                leaf_index: v.leaf_index,
                credential_identity: v.credential_identity.clone(),
                signature_public_key: v.signature_public_key.clone(),
            };
            let row = |r: &MlsStorageEntry| NativeStorageEntryV2 {
                key: r.key.clone(),
                value: r.value.clone(),
                group_id: r.group_id.clone(),
            };
            let request = NativeReceiveRequestV2::Welcome {
                profile_id: 2,
                welcome_bytes: input.welcome,
                ratchet_tree_bytes: input.tree,
                signer_bytes: input.signer,
                expected_local_leaf: leaf(&input.local_leaf),
                expected_resulting_state: NativeExpectedRosterStateV2 {
                    group_id: input.resulting.group_id,
                    epoch: input.resulting.epoch,
                    digest_sha256: input.resulting.digest_sha256,
                },
                expected_target_key_package_sha256: input.target_key_package_sha256,
                storage: NativeStorageSnapshotV2 {
                    storage_format_version: 1,
                    entries: input.entries.iter().map(row).collect(),
                },
                retention: 4,
                incarnation_id: input.incarnation_id,
                expected_welcome_sha256: input.welcome_sha256,
            };
            let frame = encode_native_receive_request_v2(&request).unwrap();
            let expected_result =
                NativeReceiveOutcomeV2::success(NativeReceiveSuccessV2::Welcome {
                    local_leaf: leaf(&joined.local_leaf),
                    resulting_roster: NativeRosterSummaryV2 {
                        group_id: joined.mutation.resulting.group_id.clone(),
                        epoch: joined.mutation.resulting.epoch,
                        digest_sha256: joined.mutation.resulting.digest_sha256.clone(),
                        leaves: joined.mutation.resulting.leaves.iter().map(leaf).collect(),
                    },
                    resulting_group_state_sha256: joined.mutation.resulting_digest.clone(),
                    consumed_key_package_sha256: joined.consumed_key_package_sha256.clone(),
                    effective_retention: 4,
                    storage_batch: NativeStorageBatchV2 {
                        storage_format_version: 1,
                        upserts: joined.mutation.batch.upserts.iter().map(row).collect(),
                        deletes: joined.mutation.batch.deletes.clone(),
                        deleted_group_ids: vec![],
                    },
                });
            assert_eq!(
                execute_native_receive_v2(&frame),
                encode_native_receive_outcome_v2(
                    Some(NativeReceiveOperationV2::Welcome),
                    &expected_result
                )
                .unwrap()
            );
            let mut mismatch = decode_native_receive_request_v2(&frame).unwrap();
            if let NativeReceiveRequestV2::Welcome {
                expected_target_key_package_sha256,
                ..
            } = &mut mismatch
            {
                expected_target_key_package_sha256[0] ^= 1;
            }
            assert_eq!(
                execute_native_receive_v2(&encode_native_receive_request_v2(&mismatch).unwrap()),
                encode_native_receive_failure_v2(
                    Some(NativeReceiveOperationV2::Welcome),
                    Error::ExpectedKeyPackageMismatch
                )
            );
        }
        mutate(&mut c, &joined.mutation);
        let sent = send(
            context(&a),
            a.signer.to_vec(),
            b"replacement".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        let received = receive(incoming(
            &c,
            sent.ciphertext,
            MlsReceiveKind::Application,
            a.roster.leaves[0].clone(),
            expected(&a.roster),
            expected(&c.roster),
        ))
        .unwrap();
        assert_eq!(
            received.plaintext.as_deref(),
            Some(b"replacement".as_slice())
        );
    }

    #[test]
    fn durable_pending_receives_live_then_exact_acceptance_merges() {
        for retention in [2, 4] {
            let (mut a, mut b) = pair(retention);
            let p = self_prepare(&a, 2);
            mutate(&mut a, &p.mutation);
            assert!(matches!(
                send(
                    context(&a),
                    a.signer.to_vec(),
                    b"blocked-author".to_vec(),
                    AAD.to_vec()
                ),
                Err(Error::PendingCommitExists)
            ));
            let retry = inspect_pending(context(&a)).unwrap().unwrap();
            assert_eq!(retry.commit, p.commit);
            assert_eq!(retry.binding, p.binding);
            // Other members continue sending; this is not a group-wide barrier.
            let s = send(
                context(&b),
                b.signer.to_vec(),
                b"live".to_vec(),
                AAD.to_vec(),
            )
            .unwrap();
            mutate(&mut b, &s.mutation);
            let sender = b
                .roster
                .leaves
                .iter()
                .find(|l| l.credential_identity == b.id)
                .unwrap()
                .clone();
            let old = expected(&a.roster);
            let stale = context(&a);
            let stale_digest = stale.expected_base_sha256.clone();
            let wire = s.ciphertext;
            let r = receive(incoming(
                &a,
                wire.clone(),
                MlsReceiveKind::Application,
                sender.clone(),
                old.clone(),
                old.clone(),
            ))
            .unwrap();
            assert_eq!(r.plaintext.as_deref(), Some(b"live".as_slice()));
            mutate(&mut a, &r.mutation);
            let stale_result = merge(stale, acceptance(&p)).unwrap();
            assert_eq!(
                apply_cas(&mut a, &stale_digest, &stale_result),
                Err(Error::BaseStateMismatch),
                "detached compute cannot see caller writes; atomic compare/apply must reject its stale batch"
            );
            let mut wrong = acceptance(&p);
            wrong.binding.transition.command_id[0] ^= 1;
            assert!(matches!(
                merge(context(&a), wrong),
                Err(Error::AcceptanceBindingMismatch)
            ));
            let m = merge(context(&a), acceptance(&p)).unwrap();
            mutate(&mut a, &m);
            assert!(inspect_pending(context(&a)).unwrap().is_none());
            assert!(matches!(
                receive(incoming(
                    &a,
                    wire,
                    MlsReceiveKind::Application,
                    sender,
                    old,
                    expected(&a.roster)
                )),
                Err(Error::Replay)
            ));
            let c = receive(incoming(
                &b,
                p.commit,
                MlsReceiveKind::Commit,
                p.binding.author,
                p.binding.previous_state,
                expected(&p.proposed),
            ))
            .unwrap();
            mutate(&mut b, &c.mutation);
            let next = send(
                context(&b),
                b.signer.to_vec(),
                b"next".to_vec(),
                AAD.to_vec(),
            )
            .unwrap();
            let sender = b
                .roster
                .leaves
                .iter()
                .find(|l| l.credential_identity == b.id)
                .unwrap()
                .clone();
            receive(incoming(
                &a,
                next.ciphertext,
                MlsReceiveKind::Application,
                sender,
                expected(&b.roster),
                expected(&a.roster),
            ))
            .unwrap();
        }
    }
    #[test]
    fn definitive_discard_preserves_live_ratchets_and_config_is_fixed() {
        let (mut a, mut b) = pair(2);
        let p = self_prepare(&a, 2);
        mutate(&mut a, &p.mutation);
        let s = send(
            context(&b),
            b.signer.to_vec(),
            b"keep-replay-state".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        mutate(&mut b, &s.mutation);
        let sender = b.roster.leaves[1].clone();
        let msg = expected(&b.roster);
        let r = receive(incoming(
            &a,
            s.ciphertext.clone(),
            MlsReceiveKind::Application,
            sender.clone(),
            msg.clone(),
            expected(&a.roster),
        ))
        .unwrap();
        mutate(&mut a, &r.mutation);
        let d = discard(context(&a), p.binding).unwrap();
        mutate(&mut a, &d);
        assert!(matches!(
            receive(incoming(
                &a,
                s.ciphertext,
                MlsReceiveKind::Application,
                sender,
                msg,
                expected(&a.roster)
            )),
            Err(Error::Replay)
        ));
        let mut wrong = context(&a);
        wrong.expected_retention = 4;
        assert!(matches!(
            inspect_pending(wrong),
            Err(Error::ConfigurationMismatch)
        ));
        let mut stale = context(&a);
        stale.expected_base_sha256[0] ^= 1;
        assert!(matches!(
            inspect_pending(stale),
            Err(Error::BaseStateMismatch)
        ));
        let mut no_metadata = context(&a);
        no_metadata
            .entries
            .retain(|r| !r.key.starts_with(b"OpenMlsDart/"));
        no_metadata.expected_base_sha256 =
            group_state_digest_from_entries(GID, &no_metadata.entries, 1).unwrap();
        assert!(matches!(
            inspect_pending(no_metadata),
            Err(Error::LocalMetadataMissing)
        ));
    }
    #[test]
    fn parallel_snapshots_require_caller_cas_then_safe_recomputation() {
        let (mut a, mut b) = pair(2);
        let sender = b.roster.leaves[1].clone();
        let roster = expected(&b.roster);
        let one = send(
            context(&b),
            b.signer.to_vec(),
            b"one".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        mutate(&mut b, &one.mutation);
        let two = send(
            context(&b),
            b.signer.to_vec(),
            b"two".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        let base = context(&a).expected_base_sha256.clone();
        let first = incoming(
            &a,
            one.ciphertext,
            MlsReceiveKind::Application,
            sender.clone(),
            roster.clone(),
            roster.clone(),
        );
        let second = incoming(
            &a,
            two.ciphertext.clone(),
            MlsReceiveKind::Application,
            sender.clone(),
            roster.clone(),
            roster.clone(),
        );
        let left = std::thread::spawn(|| receive(first));
        let right = std::thread::spawn(|| receive(second));
        let left = left.join().unwrap().unwrap();
        let right = right.join().unwrap().unwrap();
        apply_cas(&mut a, &base, &left.mutation).unwrap();
        assert_eq!(
            apply_cas(&mut a, &base, &right.mutation),
            Err(Error::BaseStateMismatch)
        );
        let retry = receive(incoming(
            &a,
            two.ciphertext,
            MlsReceiveKind::Application,
            sender,
            roster.clone(),
            roster,
        ))
        .unwrap();
        assert_eq!(retry.plaintext.as_deref(), Some(b"two".as_slice()));
        let base = context(&a).expected_base_sha256.clone();
        apply_cas(&mut a, &base, &retry.mutation).unwrap();
    }
    #[test]
    fn mixed_local_retention_historical_keys_and_required_commits() {
        let (mut a, mut b) = pair_with_retention(2, 4);
        let old = expected(&a.roster);
        let owner = a.roster.leaves[0].clone();
        let receiver = a.roster.leaves[1].clone();
        let old_a = send(
            context(&a),
            a.signer.to_vec(),
            b"retained-by-four".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        mutate(&mut a, &old_a.mutation);
        let old_b = send(
            context(&b),
            b.signer.to_vec(),
            b"expired-at-two".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        mutate(&mut b, &old_b.mutation);
        for command in 3..6 {
            let p = self_prepare(&a, command);
            mutate(&mut a, &p.mutation);
            let m = merge(context(&a), acceptance(&p)).unwrap();
            mutate(&mut a, &m);
            let r = receive(incoming(
                &b,
                p.commit,
                MlsReceiveKind::Commit,
                p.binding.author,
                p.binding.previous_state,
                expected(&p.proposed),
            ))
            .unwrap();
            mutate(&mut b, &r.mutation);
        }
        assert_eq!(a.roster, b.roster);
        assert_eq!((a.retention, b.retention), (2, 4));
        let mut wrong_key = owner.clone();
        wrong_key.signature_public_key[0] ^= 1;
        assert!(matches!(
            receive(incoming(
                &b,
                old_a.ciphertext.clone(),
                MlsReceiveKind::Application,
                wrong_key,
                old.clone(),
                expected(&b.roster)
            )),
            Err(Error::SenderMismatch)
        ));
        let valid = receive(incoming(
            &b,
            old_a.ciphertext,
            MlsReceiveKind::Application,
            owner,
            old.clone(),
            expected(&b.roster),
        ))
        .unwrap();
        assert_eq!(valid.message_epoch, old.epoch);
        assert_eq!(valid.mutation.resulting, b.roster);
        assert_eq!(
            valid.plaintext.as_deref(),
            Some(b"retained-by-four".as_slice())
        );
        assert!(matches!(
            receive(incoming(
                &a,
                old_b.ciphertext,
                MlsReceiveKind::Application,
                receiver,
                old,
                expected(&a.roster)
            )),
            Err(Error::PastEpochUnavailable)
        ));
        let p = self_prepare(&a, 6);
        mutate(&mut a, &p.mutation);
        let m = merge(context(&a), acceptance(&p)).unwrap();
        mutate(&mut a, &m);
        let future = send(
            context(&a),
            a.signer.to_vec(),
            b"needs-control".to_vec(),
            AAD.to_vec(),
        )
        .unwrap();
        assert!(matches!(
            receive(incoming(
                &b,
                future.ciphertext,
                MlsReceiveKind::Application,
                a.roster.leaves[0].clone(),
                expected(&a.roster),
                expected(&b.roster)
            )),
            Err(Error::FutureMessageEpoch)
        ));
    }
}
