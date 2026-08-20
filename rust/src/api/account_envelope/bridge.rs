//! Flutter Rust Bridge facade for the account-envelope core.
//!
//! The facade deliberately exposes purpose-specific operations and opaque
//! bundle bytes. It does not expose HPKE, Ed25519, hashing, component keys,
//! provider selection, retained native state, persistence, or networking.

use zeroize::Zeroizing;

use super::types::{
    AccountEnvelopeContinuityResponseV1, AccountEnvelopePaddingClassV1,
    AccountEnvelopePrivateBundleAuthorityV1, AccountEnvelopePublicBundleSummaryV1,
    ContextInvitationAuthorityV1, ContextInvitationPreviewV1, ExpectedContextInvitationAuthorityV1,
    is_canonical_uuid,
};
use super::{
    AccountEnvelopeActivationKindV1, AccountEnvelopeContinuityDispositionV1,
    AccountEnvelopeErrorV1, AccountEnvelopeResetReasonV1, SelfSignedPublicBundleResultV1,
};

/// Stateless high-level account-envelope cryptographic operations.
///
/// This zero-sized facade retains no key, protocol, network, or database state.
#[flutter_rust_bridge::frb(opaque)]
pub struct AccountEnvelopeCrypto {
    _private: (),
}

/// Authenticated authority expected for one opaque private bundle.
pub struct AccountEnvelopePrivateBundleAuthorityInputV1 {
    pub account_id: Vec<u8>,
    pub generation: u64,
    pub root_installation_id: Vec<u8>,
    pub root_authority_generation: u64,
}

/// Opaque generated private bundle. Component keys are never exposed.
pub struct GenerateAccountEnvelopeKeyBundleOutputV1 {
    pub private_bundle: Vec<u8>,
}

/// Whether returned public bytes are publishable or await predecessor approval.
pub enum AccountEnvelopePublicBundleCandidateKindV1 {
    CanonicalPublicBundle,
    NonPublishableRotationCandidate,
}

/// Self-signed public bundle bytes and their lifecycle classification.
pub struct AccountEnvelopePublicBundleCandidateV1 {
    pub kind: AccountEnvelopePublicBundleCandidateKindV1,
    pub bytes: Vec<u8>,
}

/// Complete successor bytes and an inactive predecessor-retirement candidate.
pub struct AccountEnvelopeSuccessorAuthorizationV1 {
    pub authorized_canonical_successor_public_bundle: Vec<u8>,
    pub retired_previous_private_bundle_candidate: Vec<u8>,
}

/// Public, self-signature-verified canonical bundle fields.
pub struct AccountEnvelopePublicBundleSummaryOutputV1 {
    pub account_id: Vec<u8>,
    pub generation: u64,
    pub hpke_public_key: Vec<u8>,
    pub signature_public_key: Vec<u8>,
    pub activation_kind: AccountEnvelopeActivationKindV1,
    pub previous_generation: u64,
    pub reset_reason: Option<AccountEnvelopeResetReasonV1>,
    pub digest_sha256: Vec<u8>,
}

/// Verified continuity result. Reset anchors still require caller acceptance.
pub struct VerifyAccountEnvelopeContinuityOutputV1 {
    pub disposition: AccountEnvelopeContinuityDispositionV1,
    pub verified_public_bundle: Vec<u8>,
    pub verified_summary: AccountEnvelopePublicBundleSummaryOutputV1,
}

/// Authenticated clear-header authority for a sealed invitation preview.
pub struct ContextInvitationAuthorityInputV1 {
    pub envelope_id: Vec<u8>,
    pub invite_id: Vec<u8>,
    pub sender_account_id: Vec<u8>,
    pub sender_generation: u64,
    pub recipient_account_id: Vec<u8>,
    pub recipient_generation: u64,
    pub authority_attempt: u64,
    pub relay_slot_version: u64,
    pub server_created_at_unix_ms: u64,
    pub server_expires_at_unix_ms: u64,
    pub padding_class: AccountEnvelopePaddingClassV1,
}

/// Authenticated open authority plus installation-local private-bundle binding.
pub struct ExpectedContextInvitationAuthorityInputV1 {
    pub invitation: ContextInvitationAuthorityInputV1,
    pub local_root_installation_id: Vec<u8>,
    pub local_root_authority_generation: u64,
}

/// Caller-provided invitation preview. Rust normalizes and bounds every field.
pub struct ContextInvitationPreviewInputV1 {
    pub title: Option<String>,
    pub tags: Vec<String>,
}

/// Authenticated, normalized invitation preview returned after successful open.
pub struct ContextInvitationPreviewOutputV1 {
    pub title: Option<String>,
    pub tags: Vec<String>,
}

impl AccountEnvelopeCrypto {
    #[flutter_rust_bridge::frb(sync)]
    pub fn generate_key_bundle_v1(
        account_id: Vec<u8>,
        generation: u64,
        root_installation_id: Vec<u8>,
        root_authority_generation: u64,
    ) -> Result<GenerateAccountEnvelopeKeyBundleOutputV1, AccountEnvelopeErrorV1> {
        let authority = private_authority(
            account_id,
            generation,
            root_installation_id,
            root_authority_generation,
        )?;
        let mut result = super::generate_key_bundle_v1(authority)?;
        Ok(GenerateAccountEnvelopeKeyBundleOutputV1 {
            private_bundle: take_zeroizing(&mut result.private_bundle),
        })
    }

    #[flutter_rust_bridge::frb(sync)]
    #[allow(clippy::too_many_arguments)]
    pub fn create_self_signed_public_bundle_v1(
        account_id: Vec<u8>,
        generation: u64,
        activation_kind: AccountEnvelopeActivationKindV1,
        reset_reason: Option<AccountEnvelopeResetReasonV1>,
        previous_generation: u64,
        expected_local_private_bundle_authority: AccountEnvelopePrivateBundleAuthorityInputV1,
        private_bundle: Vec<u8>,
    ) -> Result<AccountEnvelopePublicBundleCandidateV1, AccountEnvelopeErrorV1> {
        let mut private_bundle = Zeroizing::new(private_bundle);
        let account_id = exact_uuid(account_id)?;
        let expected = expected_local_private_bundle_authority.try_into_core()?;
        let result = super::create_self_signed_public_bundle_v1(
            account_id,
            generation,
            activation_kind,
            reset_reason,
            previous_generation,
            expected,
            take_zeroizing(&mut private_bundle),
        )?;
        Ok(match result {
            SelfSignedPublicBundleResultV1::CanonicalPublicBundle(bytes) => {
                AccountEnvelopePublicBundleCandidateV1 {
                    kind: AccountEnvelopePublicBundleCandidateKindV1::CanonicalPublicBundle,
                    bytes,
                }
            }
            SelfSignedPublicBundleResultV1::NonPublishableRotationCandidate(bytes) => {
                AccountEnvelopePublicBundleCandidateV1 {
                    kind:
                        AccountEnvelopePublicBundleCandidateKindV1::NonPublishableRotationCandidate,
                    bytes,
                }
            }
        })
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn authorize_successor_public_bundle_v1(
        expected_previous_local_private_bundle_authority: AccountEnvelopePrivateBundleAuthorityInputV1,
        previous_private_bundle: Vec<u8>,
        self_signed_successor_public_bundle: Vec<u8>,
    ) -> Result<AccountEnvelopeSuccessorAuthorizationV1, AccountEnvelopeErrorV1> {
        let mut previous_private_bundle = Zeroizing::new(previous_private_bundle);
        let expected = expected_previous_local_private_bundle_authority.try_into_core()?;
        let mut result = super::authorize_successor_public_bundle_v1(
            expected,
            take_zeroizing(&mut previous_private_bundle),
            self_signed_successor_public_bundle,
        )?;
        Ok(AccountEnvelopeSuccessorAuthorizationV1 {
            authorized_canonical_successor_public_bundle: result
                .authorized_canonical_successor_public_bundle,
            retired_previous_private_bundle_candidate: take_zeroizing(
                &mut result.retired_previous_private_bundle_candidate,
            ),
        })
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn verify_continuity_response_v1(
        pinned_public_bundle: Option<Vec<u8>>,
        continuity_public_bundles: Vec<Vec<u8>>,
        manual_reanchor_authorized: bool,
    ) -> Result<VerifyAccountEnvelopeContinuityOutputV1, AccountEnvelopeErrorV1> {
        let result = super::verify_continuity_response_v1(
            pinned_public_bundle,
            AccountEnvelopeContinuityResponseV1 {
                public_bundles: continuity_public_bundles,
            },
            manual_reanchor_authorized,
        )?;
        Ok(VerifyAccountEnvelopeContinuityOutputV1 {
            disposition: result.disposition,
            verified_public_bundle: result.verified_public_bundle,
            verified_summary: result.verified_summary.into(),
        })
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn seal_context_invitation_preview_v1(
        authority: ContextInvitationAuthorityInputV1,
        expected_local_private_bundle_authority: AccountEnvelopePrivateBundleAuthorityInputV1,
        preview: ContextInvitationPreviewInputV1,
        recipient_public_bundle: Vec<u8>,
        sender_private_bundle: Vec<u8>,
    ) -> Result<Vec<u8>, AccountEnvelopeErrorV1> {
        let mut sender_private_bundle = Zeroizing::new(sender_private_bundle);
        let result = super::seal_context_invitation_preview_v1(
            authority.try_into_core()?,
            expected_local_private_bundle_authority.try_into_core()?,
            ContextInvitationPreviewV1 {
                title: preview.title,
                tags: preview.tags,
            },
            recipient_public_bundle,
            take_zeroizing(&mut sender_private_bundle),
        )?;
        Ok(result.canonical_envelope)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn verify_and_open_context_invitation_preview_v1(
        envelope: Vec<u8>,
        expected_authority: ExpectedContextInvitationAuthorityInputV1,
        recipient_private_bundle: Vec<u8>,
        sender_public_bundle: Vec<u8>,
    ) -> Result<ContextInvitationPreviewOutputV1, AccountEnvelopeErrorV1> {
        let mut recipient_private_bundle = Zeroizing::new(recipient_private_bundle);
        let result = super::verify_and_open_context_invitation_preview_v1(
            envelope,
            expected_authority.try_into_core()?,
            take_zeroizing(&mut recipient_private_bundle),
            sender_public_bundle,
        )?;
        let mut preview = result.preview;
        Ok(ContextInvitationPreviewOutputV1 {
            title: preview.title.take(),
            tags: std::mem::take(&mut preview.tags),
        })
    }
}

impl AccountEnvelopePrivateBundleAuthorityInputV1 {
    fn try_into_core(
        self,
    ) -> Result<AccountEnvelopePrivateBundleAuthorityV1, AccountEnvelopeErrorV1> {
        private_authority(
            self.account_id,
            self.generation,
            self.root_installation_id,
            self.root_authority_generation,
        )
    }
}

impl ContextInvitationAuthorityInputV1 {
    fn try_into_core(self) -> Result<ContextInvitationAuthorityV1, AccountEnvelopeErrorV1> {
        Ok(ContextInvitationAuthorityV1 {
            envelope_id: exact_uuid(self.envelope_id)?,
            invite_id: exact_uuid(self.invite_id)?,
            sender_account_id: exact_uuid(self.sender_account_id)?,
            sender_generation: self.sender_generation,
            recipient_account_id: exact_uuid(self.recipient_account_id)?,
            recipient_generation: self.recipient_generation,
            authority_attempt: self.authority_attempt,
            relay_slot_version: self.relay_slot_version,
            server_created_at_unix_ms: self.server_created_at_unix_ms,
            server_expires_at_unix_ms: self.server_expires_at_unix_ms,
            padding_class: self.padding_class,
        })
    }
}

impl ExpectedContextInvitationAuthorityInputV1 {
    fn try_into_core(self) -> Result<ExpectedContextInvitationAuthorityV1, AccountEnvelopeErrorV1> {
        Ok(ExpectedContextInvitationAuthorityV1 {
            invitation: self.invitation.try_into_core()?,
            local_root_installation_id: exact_uuid(self.local_root_installation_id)?,
            local_root_authority_generation: self.local_root_authority_generation,
        })
    }
}

impl From<AccountEnvelopePublicBundleSummaryV1> for AccountEnvelopePublicBundleSummaryOutputV1 {
    fn from(value: AccountEnvelopePublicBundleSummaryV1) -> Self {
        Self {
            account_id: value.account_id.to_vec(),
            generation: value.generation,
            hpke_public_key: value.hpke_public_key.to_vec(),
            signature_public_key: value.signature_public_key.to_vec(),
            activation_kind: value.activation_kind,
            previous_generation: value.previous_generation,
            reset_reason: value.reset_reason,
            digest_sha256: value.digest_sha256.to_vec(),
        }
    }
}

fn private_authority(
    account_id: Vec<u8>,
    generation: u64,
    root_installation_id: Vec<u8>,
    root_authority_generation: u64,
) -> Result<AccountEnvelopePrivateBundleAuthorityV1, AccountEnvelopeErrorV1> {
    let authority = AccountEnvelopePrivateBundleAuthorityV1 {
        account_id: exact_uuid(account_id)?,
        generation,
        root_installation_id: exact_uuid(root_installation_id)?,
        root_authority_generation,
    };
    authority.validate()?;
    Ok(authority)
}

fn exact_uuid(value: Vec<u8>) -> Result<[u8; 16], AccountEnvelopeErrorV1> {
    let value: [u8; 16] = value.try_into().map_err(|_| super::authority_mismatch())?;
    if !is_canonical_uuid(&value) {
        return Err(super::authority_mismatch());
    }
    Ok(value)
}

fn take_zeroizing(value: &mut Zeroizing<Vec<u8>>) -> Vec<u8> {
    std::mem::take(&mut **value)
}
