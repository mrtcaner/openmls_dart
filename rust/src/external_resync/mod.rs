//! Strict external-resync public core. No network, database, or retained state.

pub mod codec;
pub(crate) mod preflight;
pub mod public;
#[cfg(test)]
mod tests;

pub const PROFILE: u16 = 2;
pub const MAX_MLS_BYTES: usize = 1024 * 1024;
pub const MAX_AAD_BYTES: usize = 16 * 1024;
pub const MAX_LEAVES: usize = 256;
pub const MAX_TREE_NODES: usize = 511;

/// Public-validator mechanism errors; no strings or partial success are returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Error {
    InvalidFrame = 1,
    UnsupportedVersion = 2,
    UnsupportedProfile = 3,
    UnsupportedOperation = 4,
    NoncanonicalEncoding = 5,
    LimitExceeded = 6,
    HashMismatch = 10,
    GroupMismatch = 20,
    EpochMismatch = 21,
    RosterMismatch = 22,
    SignerMismatch = 23,
    IdentityMismatch = 24,
    DeltaMismatch = 25,
    AadMismatch = 26,
    DecodeRejected = 30,
    SignatureRejected = 31,
    TreeRejected = 32,
    LifetimeRejected = 33,
    ProtocolRejected = 34,
    MissingRequiredExtension = 35,
    SuccessorMismatch = 36,
    InternalFailure = 255,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaf {
    pub index: u32,
    pub identity: Vec<u8>,
    pub signature_key: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedState {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub roster_sha256: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roster {
    pub state: ExpectedState,
    pub leaves: Vec<Leaf>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotRequest {
    pub profile: u16,
    pub group_info: Vec<u8>,
    pub sha256: Vec<u8>,
    pub expected: ExpectedState,
    pub expected_signer: Leaf,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuccessorRequest {
    pub base: SnapshotRequest,
    pub commit: Vec<u8>,
    pub commit_sha256: Vec<u8>,
    pub expected_aad: Vec<u8>,
    pub authorized_old: Leaf,
    pub authorized_new_identity: Vec<u8>,
    pub authorized_new_signature_key: Vec<u8>,
    /// Untrusted pre-acceptance claim, not an authenticated resulting-state oracle.
    pub reported_next_group_info: Vec<u8>,
    pub reported_next_sha256: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedSnapshot {
    pub profile: u16,
    pub sha256: Vec<u8>,
    pub roster: Roster,
    pub signer: Leaf,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedSuccessor {
    pub previous: VerifiedSnapshot,
    pub commit_sha256: Vec<u8>,
    pub aad_sha256: Vec<u8>,
    pub resulting: VerifiedSnapshot,
}
