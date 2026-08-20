use zeroize::{Zeroize, Zeroizing};

pub(super) const FORMAT_VERSION_V1: u8 = 1;
pub(super) const PRIVATE_BUNDLE_ACTIVE_BYTES: usize = 187;
pub(super) const PRIVATE_BUNDLE_RETIRED_BYTES: usize = 155;
pub(super) const PRIVATE_BUNDLE_MAX_BYTES: usize = 256;
pub(super) const PUBLIC_BUNDLE_TBS_BYTES: usize = 107;
pub(super) const SELF_SIGNED_CANDIDATE_BYTES: usize = 171;
pub(super) const PUBLIC_BUNDLE_NO_TRANSITION_BYTES: usize = 172;
pub(super) const PUBLIC_BUNDLE_ROTATION_BYTES: usize = 236;
pub(super) const PUBLIC_BUNDLE_MAX_BYTES: usize = 256;
pub(super) const KEY_BYTES: usize = 32;
pub(super) const SIGNATURE_BYTES: usize = 64;
pub(super) const MAX_WEB_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;
pub(super) const INVITATION_HEADER_BYTES: usize = 115;
pub(super) const INVITATION_ENVELOPE_MAX_BYTES: usize = 4096;
pub(super) const INVITATION_CANONICAL_ENVELOPE_MAX_BYTES: usize = 2277;
pub(super) const INVITATION_INNER_MAX_BYTES: usize = 1817;
pub(super) const INVITATION_TITLE_MAX_CODE_POINTS: usize = 120;
pub(super) const INVITATION_TITLE_MAX_BYTES: usize = 512;
pub(super) const INVITATION_TAG_MAX_COUNT: usize = 10;
pub(super) const INVITATION_TAG_MAX_CODE_POINTS: usize = 32;
pub(super) const INVITATION_TAG_MAX_BYTES: usize = 128;
pub(super) const INVITATION_MAX_LIFETIME_MS: u64 = 7 * 24 * 60 * 60 * 1000;
pub(super) const CONTINUITY_MAX_LINKS: usize = 32;
pub(super) const CONTINUITY_MAX_CANONICAL_BYTES: usize = 65_536;

pub(super) const HPKE_KEM_ID: u16 = 0x0020;
pub(super) const HPKE_KDF_ID: u16 = 0x0001;
pub(super) const HPKE_AEAD_ID: u16 = 0x0001;
pub(super) const SIGNATURE_SCHEME_ID: u16 = 0x0807;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum AccountEnvelopeErrorCodeV1 {
    AuthorityMismatch = 1,
    UnsupportedVersion = 2,
    NonCanonicalEncoding = 3,
    SignatureInvalid = 4,
    HpkeOpenFailed = 5,
    PlaintextSchemaInvalid = 6,
    BoundsExceeded = 7,
    PrivateBundleInvalid = 8,
    InternalCryptoFailure = 255,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccountEnvelopeErrorV1 {
    pub code: AccountEnvelopeErrorCodeV1,
}

impl AccountEnvelopeErrorV1 {
    pub(super) const fn new(code: AccountEnvelopeErrorCodeV1) -> Self {
        Self { code }
    }
}

impl std::fmt::Display for AccountEnvelopeErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "account-envelope operation failed ({})",
            self.code as u16
        )
    }
}

impl std::error::Error for AccountEnvelopeErrorV1 {}

pub(super) type AccountEnvelopeResult<T> = Result<T, AccountEnvelopeErrorV1>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AccountEnvelopePrivateBundleAuthorityV1 {
    pub account_id: [u8; 16],
    pub generation: u64,
    pub root_installation_id: [u8; 16],
    pub root_authority_generation: u64,
}

impl AccountEnvelopePrivateBundleAuthorityV1 {
    pub(super) fn validate(&self) -> AccountEnvelopeResult<()> {
        if !is_canonical_uuid(&self.account_id)
            || !is_canonical_uuid(&self.root_installation_id)
            || self.generation == 0
            || self.generation > MAX_WEB_SAFE_INTEGER
            || self.root_authority_generation == 0
            || self.root_authority_generation > MAX_WEB_SAFE_INTEGER
        {
            return Err(AccountEnvelopeErrorV1::new(
                AccountEnvelopeErrorCodeV1::AuthorityMismatch,
            ));
        }
        Ok(())
    }
}

pub(super) fn is_canonical_uuid(value: &[u8; 16]) -> bool {
    let version = value[6] >> 4;
    value.iter().any(|byte| *byte != 0)
        && (value[8] & 0b1100_0000) == 0b1000_0000
        && (1..=8).contains(&version)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AccountEnvelopeActivationKindV1 {
    Initial = 1,
    Rotation = 2,
    ContinuityReset = 3,
}

impl AccountEnvelopeActivationKindV1 {
    pub(super) fn from_u8(value: u8) -> AccountEnvelopeResult<Self> {
        match value {
            1 => Ok(Self::Initial),
            2 => Ok(Self::Rotation),
            3 => Ok(Self::ContinuityReset),
            _ => Err(AccountEnvelopeErrorV1::new(
                AccountEnvelopeErrorCodeV1::NonCanonicalEncoding,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AccountEnvelopeResetReasonV1 {
    IdentityReset = 1,
    ChatStoreReset = 2,
    RootInstallationMove = 3,
    RootOrDatabaseKeyLoss = 4,
    AccountRecovery = 5,
    Compromise = 6,
}

impl AccountEnvelopeResetReasonV1 {
    pub(crate) fn new(value: u8) -> AccountEnvelopeResult<Self> {
        match value {
            1 => Ok(Self::IdentityReset),
            2 => Ok(Self::ChatStoreReset),
            3 => Ok(Self::RootInstallationMove),
            4 => Ok(Self::RootOrDatabaseKeyLoss),
            5 => Ok(Self::AccountRecovery),
            6 => Ok(Self::Compromise),
            _ => Err(AccountEnvelopeErrorV1::new(
                AccountEnvelopeErrorCodeV1::NonCanonicalEncoding,
            )),
        }
    }

    pub(super) const fn value(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrivateBundleStateV1 {
    ActiveFull,
    RetiredDecryptOnly,
}

impl PrivateBundleStateV1 {
    pub(super) const fn as_u8(self) -> u8 {
        match self {
            Self::ActiveFull => 1,
            Self::RetiredDecryptOnly => 2,
        }
    }

    pub(super) fn from_u8(value: u8) -> AccountEnvelopeResult<Self> {
        match value {
            1 => Ok(Self::ActiveFull),
            2 => Ok(Self::RetiredDecryptOnly),
            _ => Err(AccountEnvelopeErrorV1::new(
                AccountEnvelopeErrorCodeV1::PrivateBundleInvalid,
            )),
        }
    }
}

pub(super) struct DecodedPrivateBundleV1 {
    pub state: PrivateBundleStateV1,
    pub authority: AccountEnvelopePrivateBundleAuthorityV1,
    pub hpke_private_key: [u8; KEY_BYTES],
    pub hpke_public_key: [u8; KEY_BYTES],
    pub signature_private_key: Option<[u8; KEY_BYTES]>,
    pub signature_public_key: [u8; KEY_BYTES],
}

impl Drop for DecodedPrivateBundleV1 {
    fn drop(&mut self) {
        self.hpke_private_key.zeroize();
        if let Some(private_key) = &mut self.signature_private_key {
            private_key.zeroize();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AccountEnvelopePublicBundleSummaryV1 {
    pub account_id: [u8; 16],
    pub generation: u64,
    pub hpke_public_key: [u8; KEY_BYTES],
    pub signature_public_key: [u8; KEY_BYTES],
    pub activation_kind: AccountEnvelopeActivationKindV1,
    pub previous_generation: u64,
    pub reset_reason: Option<AccountEnvelopeResetReasonV1>,
    pub digest_sha256: [u8; 32],
}

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct GenerateAccountEnvelopeKeyBundleResultV1 {
    pub private_bundle: Zeroizing<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelfSignedPublicBundleResultV1 {
    CanonicalPublicBundle(Vec<u8>),
    NonPublishableRotationCandidate(Vec<u8>),
}

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct AuthorizeSuccessorPublicBundleResultV1 {
    pub authorized_canonical_successor_public_bundle: Vec<u8>,
    pub retired_previous_private_bundle_candidate: Zeroizing<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AccountEnvelopePaddingClassV1 {
    Bytes512 = 1,
    Bytes1024 = 2,
    Bytes2048 = 3,
}

impl AccountEnvelopePaddingClassV1 {
    pub(super) fn from_u8(value: u8) -> AccountEnvelopeResult<Self> {
        match value {
            1 => Ok(Self::Bytes512),
            2 => Ok(Self::Bytes1024),
            3 => Ok(Self::Bytes2048),
            _ => Err(AccountEnvelopeErrorV1::new(
                AccountEnvelopeErrorCodeV1::NonCanonicalEncoding,
            )),
        }
    }

    pub(super) const fn plaintext_bytes(self) -> usize {
        match self {
            Self::Bytes512 => 512,
            Self::Bytes1024 => 1024,
            Self::Bytes2048 => 2048,
        }
    }

    pub(super) const fn ciphertext_bytes(self) -> usize {
        self.plaintext_bytes() + 16
    }
}

pub(crate) struct ContextInvitationPreviewV1 {
    pub title: Option<String>,
    pub tags: Vec<String>,
}

impl Drop for ContextInvitationPreviewV1 {
    fn drop(&mut self) {
        if let Some(title) = &mut self.title {
            title.zeroize();
        }
        for tag in &mut self.tags {
            tag.zeroize();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContextInvitationAuthorityV1 {
    pub envelope_id: [u8; 16],
    pub invite_id: [u8; 16],
    pub sender_account_id: [u8; 16],
    pub sender_generation: u64,
    pub recipient_account_id: [u8; 16],
    pub recipient_generation: u64,
    pub authority_attempt: u64,
    pub relay_slot_version: u64,
    pub server_created_at_unix_ms: u64,
    pub server_expires_at_unix_ms: u64,
    pub padding_class: AccountEnvelopePaddingClassV1,
}

impl ContextInvitationAuthorityV1 {
    pub(super) fn validate(&self) -> AccountEnvelopeResult<()> {
        let lifetime = self
            .server_expires_at_unix_ms
            .checked_sub(self.server_created_at_unix_ms)
            .ok_or_else(authority_mismatch)?;
        if !is_canonical_uuid(&self.envelope_id)
            || !is_canonical_uuid(&self.invite_id)
            || !is_canonical_uuid(&self.sender_account_id)
            || !is_canonical_uuid(&self.recipient_account_id)
            || self.sender_generation == 0
            || self.sender_generation > MAX_WEB_SAFE_INTEGER
            || self.recipient_generation == 0
            || self.recipient_generation > MAX_WEB_SAFE_INTEGER
            || self.authority_attempt == 0
            || self.authority_attempt > MAX_WEB_SAFE_INTEGER
            || self.relay_slot_version != 1
            || self.server_created_at_unix_ms == 0
            || self.server_created_at_unix_ms > MAX_WEB_SAFE_INTEGER
            || self.server_expires_at_unix_ms > MAX_WEB_SAFE_INTEGER
            || lifetime == 0
            || lifetime > INVITATION_MAX_LIFETIME_MS
        {
            return Err(authority_mismatch());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExpectedContextInvitationAuthorityV1 {
    pub invitation: ContextInvitationAuthorityV1,
    pub local_root_installation_id: [u8; 16],
    pub local_root_authority_generation: u64,
}

impl ExpectedContextInvitationAuthorityV1 {
    pub(super) fn validate(&self) -> AccountEnvelopeResult<()> {
        self.invitation.validate()?;
        if !is_canonical_uuid(&self.local_root_installation_id)
            || self.local_root_authority_generation == 0
            || self.local_root_authority_generation > MAX_WEB_SAFE_INTEGER
        {
            return Err(authority_mismatch());
        }
        Ok(())
    }
}

pub(crate) struct SealContextInvitationPreviewResultV1 {
    pub canonical_envelope: Vec<u8>,
}

pub(crate) struct OpenContextInvitationPreviewResultV1 {
    pub preview: ContextInvitationPreviewV1,
}

pub(crate) struct AccountEnvelopeContinuityResponseV1 {
    /// Complete canonical public bundles in oldest-to-newest order.
    ///
    /// With no pin, this contains exactly the current observed bundle. With a
    /// pin, zero bundles means unchanged and 1..=32 bundles are successor
    /// candidates. The caller owns transport encoding; the package validates
    /// each canonical bundle and transition.
    pub public_bundles: Vec<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountEnvelopeContinuityDispositionV1 {
    FirstObservation,
    PinnedUnchanged,
    RotationChainVerified,
    ResetAnchorRequiresAcceptance,
    ManualReanchorVerified,
}

pub(crate) struct VerifyContinuityResponseResultV1 {
    pub disposition: AccountEnvelopeContinuityDispositionV1,
    /// Self-signature/transition-verified bundle. A reset disposition remains
    /// only a candidate until the caller explicitly accepts that trust anchor.
    pub verified_public_bundle: Vec<u8>,
    pub verified_summary: AccountEnvelopePublicBundleSummaryV1,
}

fn authority_mismatch() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::AuthorityMismatch)
}
