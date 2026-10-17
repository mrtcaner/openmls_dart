//! Explicit external resync. Business authorization and durable custody are caller-owned.
use super::group_e2ee::*;
use super::storage::{MlsStorageBatch, MlsStorageEntry};

#[derive(Clone)]
pub struct MlsExternalPreparationV1 {
    pub candidate_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub transition: MlsTransitionContext,
    pub local_lifecycle_context_sha256: Vec<u8>,
    pub input_group_info: Vec<u8>,
    pub input_group_info_sha256: Vec<u8>,
    pub expected_previous_state: MlsExpectedRosterStateV1,
    pub expected_snapshot_signer: MlsRosterLeafV1,
    pub authorized_old_leaf: MlsRosterLeafV1,
    pub authorized_new_credential_identity: Vec<u8>,
    pub authorized_new_signature_public_key: Vec<u8>,
    pub aad: Vec<u8>,
    pub retention: u32,
}

/// Cryptographic subset extracted from authenticated canonical acceptance.
/// No account identity or server receipt signature is authenticated by this DTO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MlsExternalCommitBindingV1 {
    pub candidate_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub transition: MlsTransitionContext,
    pub previous_state: MlsExpectedRosterStateV1,
    pub old_leaf: MlsRosterLeafV1,
    /// Resulting-roster sender; NEVER interpreted as a previous-roster member.
    pub author: MlsRosterLeafV1,
    pub input_group_info_sha256: Vec<u8>,
    pub commit_sha256: Vec<u8>,
    pub aad_sha256: Vec<u8>,
    pub resulting_group_info_sha256: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MlsExternalCommitAcceptanceV1 {
    pub binding: MlsExternalCommitBindingV1,
    pub resulting_state: MlsExpectedRosterStateV1,
}
pub struct ExternalCommitCandidateInfo {
    pub binding: MlsExternalCommitBindingV1,
    pub previous_roster: MlsRosterSummaryV1,
    pub proposed_resulting_roster: MlsRosterSummaryV1,
    pub commit: Vec<u8>,
    pub group_info: Vec<u8>,
    pub candidate_sha256: Vec<u8>,
    pub local_lifecycle_context_sha256: Vec<u8>,
    pub effective_retention: u32,
}
pub struct PrepareExternalCommitResult {
    /// SECRET. Opaque inactive envelope1; encrypt at rest and wipe after use.
    /// This is not a live storage batch. Persist it with its independent hash.
    pub candidate_bytes: Vec<u8>,
    pub info: ExternalCommitCandidateInfo,
}
/// Current target snapshot. Supply global rows and any rows for this exact group;
/// existing target group rows reject promotion. Other groups are outside scope.
pub struct MlsExternalTargetContextV1 {
    pub group_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub local_lifecycle_context_sha256: Vec<u8>,
    pub storage_entries: Vec<MlsStorageEntry>,
    pub storage_format_version: u32,
}
pub struct AcceptExternalCommitResult {
    pub candidate_sha256: Vec<u8>,
    pub local_leaf: MlsRosterLeafV1,
    pub resulting_roster: MlsRosterSummaryV1,
    pub resulting_group_state_sha256: Vec<u8>,
    pub effective_retention: u32,
    pub storage_batch: MlsStorageBatch,
}
pub struct ProcessExternalCommitResult {
    pub authenticated_sender: MlsRosterLeafV1,
    pub previous_roster: MlsRosterSummaryV1,
    pub resulting_roster: MlsRosterSummaryV1,
    pub resulting_group_state_sha256: Vec<u8>,
    pub effective_retention: u32,
    pub superseded_pending: Option<MlsPendingCommitBinding>,
    pub storage_batch: MlsStorageBatch,
}
#[allow(clippy::large_enum_variant)]
pub enum PrepareExternalCommitOutcome {
    Success(PrepareExternalCommitResult),
    Failure(MlsErrorCode),
}
#[allow(clippy::large_enum_variant)]
pub enum InspectExternalCommitCandidateOutcome {
    Success(ExternalCommitCandidateInfo),
    Failure(MlsErrorCode),
}
#[allow(clippy::large_enum_variant)]
pub enum AcceptExternalCommitOutcome {
    Success(AcceptExternalCommitResult),
    Failure(MlsErrorCode),
}
#[allow(clippy::large_enum_variant)]
pub enum ProcessExternalCommitOutcome {
    Success(ProcessExternalCommitResult),
    Failure(MlsErrorCode),
}

/// Starts from isolated empty private state. No live caller storage is modified.
pub fn prepare_external_commit(
    input: MlsExternalPreparationV1,
    signer_bytes: Vec<u8>,
) -> PrepareExternalCommitOutcome {
    match lifecycle_guard(|| runtime::external::prepare(input, signer_bytes)) {
        Ok(v) => PrepareExternalCommitOutcome::Success(v),
        Err(e) => PrepareExternalCommitOutcome::Failure(e),
    }
}
/// Recover exact retry bytes from encrypted caller custody; never regenerate.
/// expected_candidate_sha256 is the independently retained preparation hash.
pub fn inspect_external_commit_candidate(
    candidate_bytes: Vec<u8>,
    expected_candidate_sha256: Vec<u8>,
) -> InspectExternalCommitCandidateOutcome {
    match lifecycle_guard(|| runtime::external::inspect(candidate_bytes, expected_candidate_sha256))
    {
        Ok(v) => InspectExternalCommitCandidateOutcome::Success(v),
        Err(e) => InspectExternalCommitCandidateOutcome::Failure(e),
    }
}
/// Validate exact acceptance and return a promotion batch, never apply it.
/// Caller CASes lifecycle/account/installation and absence in the SAME transaction.
pub fn accept_external_commit_with_storage(
    target: MlsExternalTargetContextV1,
    candidate_bytes: Vec<u8>,
    expected_candidate_sha256: Vec<u8>,
    acceptance: MlsExternalCommitAcceptanceV1,
) -> AcceptExternalCommitOutcome {
    match lifecycle_guard(|| {
        runtime::external::accept(
            target,
            candidate_bytes,
            expected_candidate_sha256,
            acceptance,
        )
    }) {
        Ok(v) => AcceptExternalCommitOutcome::Success(v),
        Err(e) => AcceptExternalCommitOutcome::Failure(e),
    }
}
/// Explicit accepted external Commit path. Existing member receive is unchanged.
/// If a pending internal Commit loses, supply its exact binding; it is cleared
/// only in the returned atomic batch. Own external events must reconcile candidates.
pub fn process_external_commit_with_storage(
    context: MlsGroupOperationContext,
    commit: Vec<u8>,
    expected_aad: Vec<u8>,
    acceptance: MlsExternalCommitAcceptanceV1,
    expected_superseded_pending: Option<MlsPendingCommitBinding>,
) -> ProcessExternalCommitOutcome {
    match lifecycle_guard(|| {
        runtime::external::receive(
            context.into(),
            commit,
            expected_aad,
            acceptance,
            expected_superseded_pending,
        )
    }) {
        Ok(v) => ProcessExternalCommitOutcome::Success(v),
        Err(e) => ProcessExternalCommitOutcome::Failure(e),
    }
}
