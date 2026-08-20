//! Package-owned account-envelope cryptographic core.
//!
//! This module is deliberately separate from MLS. It does not read or mutate an
//! MLS group, KeyPackage, credential, exporter, storage snapshot, or native
//! receive operation. The public surface is the stateless, high-level
//! [`AccountEnvelopeCrypto`] facade; codec and provider details stay internal.

pub mod bridge;
mod codec;
mod crypto;
mod invitation;
pub mod types;
mod unicode17_casefold;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub use bridge::{
    AccountEnvelopeCrypto, AccountEnvelopePrivateBundleAuthorityInputV1,
    AccountEnvelopePublicBundleCandidateKindV1, AccountEnvelopePublicBundleCandidateV1,
    AccountEnvelopePublicBundleSummaryOutputV1, AccountEnvelopeSuccessorAuthorizationV1,
    ContextInvitationAuthorityInputV1, ContextInvitationPreviewInputV1,
    ContextInvitationPreviewOutputV1, ExpectedContextInvitationAuthorityInputV1,
    GenerateAccountEnvelopeKeyBundleOutputV1, VerifyAccountEnvelopeContinuityOutputV1,
};
pub use types::{
    AccountEnvelopeActivationKindV1, AccountEnvelopeContinuityDispositionV1,
    AccountEnvelopeErrorCodeV1, AccountEnvelopeErrorV1, AccountEnvelopePaddingClassV1,
    AccountEnvelopeResetReasonV1,
};

pub(crate) use types::{
    AccountEnvelopeContinuityResponseV1, AccountEnvelopePrivateBundleAuthorityV1,
    AccountEnvelopePublicBundleSummaryV1, AuthorizeSuccessorPublicBundleResultV1,
    ContextInvitationAuthorityV1, ContextInvitationPreviewV1, ExpectedContextInvitationAuthorityV1,
    GenerateAccountEnvelopeKeyBundleResultV1, OpenContextInvitationPreviewResultV1,
    SealContextInvitationPreviewResultV1, SelfSignedPublicBundleResultV1,
    VerifyContinuityResponseResultV1,
};

use codec::{
    ParsedPublicBundleV1, PublicBundleTbsV1, decode_private_bundle, encode_complete_public_bundle,
    encode_private_bundle, encode_public_bundle_tbs, encode_rotation_candidate,
    parse_complete_public_bundle, parse_rotation_candidate,
};
use crypto::{
    BUNDLE_DIGEST_DOMAIN_V1, BUNDLE_SELF_SIGNATURE_DOMAIN_V1,
    BUNDLE_TRANSITION_SIGNATURE_DOMAIN_V1, ENVELOPE_SIGNATURE_DOMAIN_V1, generate_private_bundle,
    sign_domain_message, validate_private_bundle_crypto, verify_domain_signature,
};
use invitation::{
    canonicalize_and_encode_preview, decode_canonical_preview, encode_invitation_envelope,
    encode_invitation_header, parse_invitation_envelope,
};
use types::{AccountEnvelopeResult, DecodedPrivateBundleV1, PrivateBundleStateV1};

pub(crate) fn generate_key_bundle_v1(
    authority: AccountEnvelopePrivateBundleAuthorityV1,
) -> AccountEnvelopeResult<GenerateAccountEnvelopeKeyBundleResultV1> {
    authority.validate()?;
    let bundle = generate_private_bundle(authority)?;
    Ok(GenerateAccountEnvelopeKeyBundleResultV1 {
        private_bundle: Zeroizing::new(encode_private_bundle(&bundle)),
    })
}

pub(crate) fn create_self_signed_public_bundle_v1(
    account_id: [u8; 16],
    generation: u64,
    activation_kind: AccountEnvelopeActivationKindV1,
    reset_reason: Option<AccountEnvelopeResetReasonV1>,
    previous_generation: u64,
    expected_local_private_bundle_authority: AccountEnvelopePrivateBundleAuthorityV1,
    private_bundle: Vec<u8>,
) -> AccountEnvelopeResult<SelfSignedPublicBundleResultV1> {
    expected_local_private_bundle_authority.validate()?;
    if account_id != expected_local_private_bundle_authority.account_id
        || generation != expected_local_private_bundle_authority.generation
    {
        return Err(authority_mismatch());
    }
    let private_bundle_bytes = Zeroizing::new(private_bundle);
    let decoded = decode_and_validate_private_bundle(
        &private_bundle_bytes,
        expected_local_private_bundle_authority,
        true,
    )?;
    let tbs = PublicBundleTbsV1 {
        account_id,
        generation,
        hpke_public_key: decoded.hpke_public_key,
        signature_public_key: decoded.signature_public_key,
        activation_kind,
        previous_generation,
        reset_reason,
    };
    validate_requested_shape(&tbs)?;
    let tbs_bytes = encode_public_bundle_tbs(&tbs);
    let signature_private_key = decoded
        .signature_private_key
        .as_ref()
        .ok_or_else(private_bundle_invalid)?;
    let self_signature = sign_domain_message(
        BUNDLE_SELF_SIGNATURE_DOMAIN_V1,
        &[&tbs_bytes],
        signature_private_key,
    )?;

    if activation_kind == AccountEnvelopeActivationKindV1::Rotation {
        Ok(
            SelfSignedPublicBundleResultV1::NonPublishableRotationCandidate(
                encode_rotation_candidate(&tbs_bytes, &self_signature),
            ),
        )
    } else {
        let bundle = encode_complete_public_bundle(&tbs_bytes, &self_signature, None);
        verify_canonical_public_bundle_v1(&bundle)?;
        Ok(SelfSignedPublicBundleResultV1::CanonicalPublicBundle(
            bundle,
        ))
    }
}

pub(crate) fn authorize_successor_public_bundle_v1(
    expected_previous_local_private_bundle_authority: AccountEnvelopePrivateBundleAuthorityV1,
    previous_private_bundle: Vec<u8>,
    self_signed_successor_public_bundle: Vec<u8>,
) -> AccountEnvelopeResult<AuthorizeSuccessorPublicBundleResultV1> {
    expected_previous_local_private_bundle_authority.validate()?;
    let previous_private_bytes = Zeroizing::new(previous_private_bundle);
    let mut previous = decode_and_validate_private_bundle(
        &previous_private_bytes,
        expected_previous_local_private_bundle_authority,
        true,
    )?;
    let candidate = parse_rotation_candidate(&self_signed_successor_public_bundle)?;
    verify_self_signature(&candidate)?;
    if candidate.tbs.account_id != previous.authority.account_id
        || candidate.tbs.previous_generation != previous.authority.generation
        || candidate.tbs.generation != previous.authority.generation + 1
    {
        return Err(authority_mismatch());
    }

    let previous_signature_private_key = previous
        .signature_private_key
        .as_ref()
        .ok_or_else(private_bundle_invalid)?;
    let transition_signature = sign_domain_message(
        BUNDLE_TRANSITION_SIGNATURE_DOMAIN_V1,
        &[&candidate.tbs_bytes, &candidate.self_signature],
        previous_signature_private_key,
    )?;
    verify_domain_signature(
        BUNDLE_TRANSITION_SIGNATURE_DOMAIN_V1,
        &[&candidate.tbs_bytes, &candidate.self_signature],
        &previous.signature_public_key,
        &transition_signature,
    )?;
    let authorized = encode_complete_public_bundle(
        &candidate.tbs_bytes,
        &candidate.self_signature,
        Some(&transition_signature),
    );
    verify_canonical_public_bundle_v1(&authorized)?;

    previous.state = PrivateBundleStateV1::RetiredDecryptOnly;
    if let Some(mut signing_key) = previous.signature_private_key.take() {
        use zeroize::Zeroize;
        signing_key.zeroize();
    }
    let retired = encode_private_bundle(&previous);

    Ok(AuthorizeSuccessorPublicBundleResultV1 {
        authorized_canonical_successor_public_bundle: authorized,
        retired_previous_private_bundle_candidate: Zeroizing::new(retired),
    })
}

pub(crate) fn verify_canonical_public_bundle_v1(
    canonical_public_bundle: &[u8],
) -> AccountEnvelopeResult<AccountEnvelopePublicBundleSummaryV1> {
    let parsed = parse_complete_public_bundle(canonical_public_bundle)?;
    verify_self_signature(&parsed)?;
    Ok(AccountEnvelopePublicBundleSummaryV1 {
        account_id: parsed.tbs.account_id,
        generation: parsed.tbs.generation,
        hpke_public_key: parsed.tbs.hpke_public_key,
        signature_public_key: parsed.tbs.signature_public_key,
        activation_kind: parsed.tbs.activation_kind,
        previous_generation: parsed.tbs.previous_generation,
        reset_reason: parsed.tbs.reset_reason,
        digest_sha256: public_bundle_digest_v1(canonical_public_bundle)?,
    })
}

pub(crate) fn public_bundle_digest_v1(
    canonical_public_bundle: &[u8],
) -> AccountEnvelopeResult<[u8; 32]> {
    // Parsing first prevents noncanonical bytes from acquiring a package digest.
    let parsed = parse_complete_public_bundle(canonical_public_bundle)?;
    verify_self_signature(&parsed)?;
    let mut hash = Sha256::new();
    hash.update(BUNDLE_DIGEST_DOMAIN_V1);
    hash.update(canonical_public_bundle);
    Ok(hash.finalize().into())
}

pub(crate) fn verify_continuity_response_v1(
    pinned_public_bundle: Option<Vec<u8>>,
    continuity_response: AccountEnvelopeContinuityResponseV1,
    manual_reanchor_authorized: bool,
) -> AccountEnvelopeResult<VerifyContinuityResponseResultV1> {
    let response_bytes = continuity_response
        .public_bundles
        .iter()
        .try_fold(0_usize, |total, bundle| total.checked_add(bundle.len()))
        .ok_or_else(bounds_exceeded)?;
    if response_bytes > types::CONTINUITY_MAX_CANONICAL_BYTES {
        return Err(bounds_exceeded());
    }

    match pinned_public_bundle {
        None => {
            if continuity_response.public_bundles.len() != 1 || manual_reanchor_authorized {
                return Err(noncanonical());
            }
            continuity_result(
                AccountEnvelopeContinuityDispositionV1::FirstObservation,
                continuity_response
                    .public_bundles
                    .into_iter()
                    .next()
                    .ok_or_else(noncanonical)?,
            )
        }
        Some(pinned) => verify_continuity_from_pin(
            pinned,
            continuity_response.public_bundles,
            manual_reanchor_authorized,
        ),
    }
}

fn verify_continuity_from_pin(
    pinned: Vec<u8>,
    response_bundles: Vec<Vec<u8>>,
    manual_reanchor_authorized: bool,
) -> AccountEnvelopeResult<VerifyContinuityResponseResultV1> {
    let pinned_summary = verify_canonical_public_bundle_v1(&pinned)?;
    if manual_reanchor_authorized {
        if response_bundles.len() != 1 {
            return Err(noncanonical());
        }
        let reanchor = response_bundles
            .into_iter()
            .next()
            .ok_or_else(noncanonical)?;
        let summary = verify_canonical_public_bundle_v1(&reanchor)?;
        if summary.account_id != pinned_summary.account_id {
            return Err(authority_mismatch());
        }
        return Ok(VerifyContinuityResponseResultV1 {
            disposition: AccountEnvelopeContinuityDispositionV1::ManualReanchorVerified,
            verified_public_bundle: reanchor,
            verified_summary: summary,
        });
    }
    if response_bundles.is_empty() {
        return Ok(VerifyContinuityResponseResultV1 {
            disposition: AccountEnvelopeContinuityDispositionV1::PinnedUnchanged,
            verified_public_bundle: pinned,
            verified_summary: pinned_summary,
        });
    }
    if response_bundles.len() > types::CONTINUITY_MAX_LINKS {
        return Err(bounds_exceeded());
    }

    let response_len = response_bundles.len();
    let mut previous = parse_complete_public_bundle(&pinned)?;
    verify_self_signature(&previous)?;
    let mut current_bundle = pinned;
    for (index, bundle) in response_bundles.into_iter().enumerate() {
        let current = parse_complete_public_bundle(&bundle)?;
        verify_self_signature(&current)?;
        if current.tbs.account_id != previous.tbs.account_id
            || current.tbs.previous_generation != previous.tbs.generation
            || current.tbs.generation != previous.tbs.generation + 1
        {
            return Err(authority_mismatch());
        }
        match current.tbs.activation_kind {
            AccountEnvelopeActivationKindV1::Rotation => {
                let transition_signature = current
                    .transition_signature
                    .as_ref()
                    .ok_or_else(noncanonical)?;
                verify_domain_signature(
                    BUNDLE_TRANSITION_SIGNATURE_DOMAIN_V1,
                    &[&current.tbs_bytes, &current.self_signature],
                    &previous.tbs.signature_public_key,
                    transition_signature,
                )?;
            }
            AccountEnvelopeActivationKindV1::ContinuityReset => {
                // A reset is a new trust anchor, not a bridge from the old pin.
                // Return it alone for explicit caller acceptance; never accept a
                // prefix or silently continue through later links.
                return if index == 0 && response_len == 1 {
                    continuity_result(
                        AccountEnvelopeContinuityDispositionV1::ResetAnchorRequiresAcceptance,
                        bundle,
                    )
                } else {
                    Err(noncanonical())
                };
            }
            AccountEnvelopeActivationKindV1::Initial => return Err(noncanonical()),
        }
        current_bundle = bundle;
        previous = current;
    }
    continuity_result(
        AccountEnvelopeContinuityDispositionV1::RotationChainVerified,
        current_bundle,
    )
}

fn continuity_result(
    disposition: AccountEnvelopeContinuityDispositionV1,
    bundle: Vec<u8>,
) -> AccountEnvelopeResult<VerifyContinuityResponseResultV1> {
    let summary = verify_canonical_public_bundle_v1(&bundle)?;
    Ok(VerifyContinuityResponseResultV1 {
        disposition,
        verified_public_bundle: bundle,
        verified_summary: summary,
    })
}

pub(crate) fn seal_context_invitation_preview_v1(
    authority: ContextInvitationAuthorityV1,
    expected_local_private_bundle_authority: AccountEnvelopePrivateBundleAuthorityV1,
    preview: ContextInvitationPreviewV1,
    recipient_public_bundle: Vec<u8>,
    sender_private_bundle: Vec<u8>,
) -> AccountEnvelopeResult<SealContextInvitationPreviewResultV1> {
    authority.validate()?;
    expected_local_private_bundle_authority.validate()?;
    if expected_local_private_bundle_authority.account_id != authority.sender_account_id
        || expected_local_private_bundle_authority.generation != authority.sender_generation
    {
        return Err(authority_mismatch());
    }
    let sender_private_bytes = Zeroizing::new(sender_private_bundle);
    let sender = decode_and_validate_private_bundle(
        &sender_private_bytes,
        expected_local_private_bundle_authority,
        true,
    )?;
    let recipient = verify_canonical_public_bundle_v1(&recipient_public_bundle)?;
    if recipient.account_id != authority.recipient_account_id
        || recipient.generation != authority.recipient_generation
    {
        return Err(authority_mismatch());
    }
    let inner = Zeroizing::new(canonicalize_and_encode_preview(&preview)?);
    let plaintext_bytes = authority.padding_class.plaintext_bytes();
    if inner.len() > plaintext_bytes - 2 {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::BoundsExceeded,
        ));
    }
    let mut plaintext = Zeroizing::new(vec![0_u8; plaintext_bytes]);
    let inner_len = u16::try_from(inner.len())
        .map_err(|_| AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::BoundsExceeded))?;
    plaintext[..2].copy_from_slice(&inner_len.to_be_bytes());
    plaintext[2..2 + inner.len()].copy_from_slice(&inner);
    let canonical_header = encode_invitation_header(&authority)?;
    let (encapsulation, ciphertext) =
        crypto::hpke_seal_invitation(&recipient.hpke_public_key, &canonical_header, &plaintext)?;
    if ciphertext.len() != authority.padding_class.ciphertext_bytes() {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::InternalCryptoFailure,
        ));
    }
    let ciphertext_len = u16::try_from(ciphertext.len())
        .map_err(|_| AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::BoundsExceeded))?;
    let signature_private_key = sender
        .signature_private_key
        .as_ref()
        .ok_or_else(private_bundle_invalid)?;
    let signature = sign_domain_message(
        ENVELOPE_SIGNATURE_DOMAIN_V1,
        &[
            &canonical_header,
            &encapsulation,
            &ciphertext_len.to_be_bytes(),
            &ciphertext,
        ],
        signature_private_key,
    )?;
    Ok(SealContextInvitationPreviewResultV1 {
        canonical_envelope: encode_invitation_envelope(
            &canonical_header,
            &encapsulation,
            &ciphertext,
            &signature,
        )?,
    })
}

pub(crate) fn verify_and_open_context_invitation_preview_v1(
    envelope: Vec<u8>,
    expected_authority: ExpectedContextInvitationAuthorityV1,
    recipient_private_bundle: Vec<u8>,
    sender_public_bundle: Vec<u8>,
) -> AccountEnvelopeResult<OpenContextInvitationPreviewResultV1> {
    expected_authority.validate()?;
    let parsed = parse_invitation_envelope(&envelope)?;
    if parsed.authority != expected_authority.invitation {
        return Err(authority_mismatch());
    }
    let sender = verify_canonical_public_bundle_v1(&sender_public_bundle)?;
    if sender.account_id != parsed.authority.sender_account_id
        || sender.generation != parsed.authority.sender_generation
    {
        return Err(authority_mismatch());
    }
    let expected_recipient_authority = AccountEnvelopePrivateBundleAuthorityV1 {
        account_id: parsed.authority.recipient_account_id,
        generation: parsed.authority.recipient_generation,
        root_installation_id: expected_authority.local_root_installation_id,
        root_authority_generation: expected_authority.local_root_authority_generation,
    };
    let recipient_private_bytes = Zeroizing::new(recipient_private_bundle);
    let recipient = decode_and_validate_private_bundle(
        &recipient_private_bytes,
        expected_recipient_authority,
        false,
    )?;
    let ciphertext_len = u16::try_from(parsed.ciphertext.len())
        .map_err(|_| AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::BoundsExceeded))?;
    verify_domain_signature(
        ENVELOPE_SIGNATURE_DOMAIN_V1,
        &[
            &parsed.canonical_header,
            &parsed.encapsulation,
            &ciphertext_len.to_be_bytes(),
            parsed.ciphertext,
        ],
        &sender.signature_public_key,
        &parsed.signature,
    )?;
    let plaintext = crypto::hpke_open_invitation(
        &recipient.hpke_private_key,
        &parsed.canonical_header,
        &parsed.encapsulation,
        parsed.ciphertext,
    )?;
    let expected_plaintext_len = parsed.authority.padding_class.plaintext_bytes();
    if plaintext.len() != expected_plaintext_len || plaintext.len() < 2 {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::PlaintextSchemaInvalid,
        ));
    }
    let inner_len = u16::from_be_bytes([plaintext[0], plaintext[1]]) as usize;
    let inner_end = 2_usize.checked_add(inner_len).ok_or_else(|| {
        AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::PlaintextSchemaInvalid)
    })?;
    if inner_len > types::INVITATION_INNER_MAX_BYTES
        || inner_end > plaintext.len()
        || plaintext[inner_end..].iter().any(|byte| *byte != 0)
    {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::PlaintextSchemaInvalid,
        ));
    }
    let preview = decode_canonical_preview(&plaintext[2..inner_end])?;
    Ok(OpenContextInvitationPreviewResultV1 { preview })
}

fn verify_self_signature(parsed: &ParsedPublicBundleV1) -> AccountEnvelopeResult<()> {
    verify_domain_signature(
        BUNDLE_SELF_SIGNATURE_DOMAIN_V1,
        &[&parsed.tbs_bytes],
        &parsed.tbs.signature_public_key,
        &parsed.self_signature,
    )
}

fn decode_and_validate_private_bundle(
    private_bundle: &[u8],
    expected_authority: AccountEnvelopePrivateBundleAuthorityV1,
    require_active: bool,
) -> AccountEnvelopeResult<DecodedPrivateBundleV1> {
    let decoded = decode_private_bundle(private_bundle)?;
    if decoded.authority != expected_authority {
        return Err(authority_mismatch());
    }
    if require_active && decoded.state != PrivateBundleStateV1::ActiveFull {
        return Err(private_bundle_invalid());
    }
    validate_private_bundle_crypto(&decoded)?;
    Ok(decoded)
}

fn validate_requested_shape(tbs: &PublicBundleTbsV1) -> AccountEnvelopeResult<()> {
    let valid = match tbs.activation_kind {
        AccountEnvelopeActivationKindV1::Initial => {
            tbs.generation == 1 && tbs.previous_generation == 0 && tbs.reset_reason.is_none()
        }
        AccountEnvelopeActivationKindV1::Rotation => {
            tbs.generation > 1
                && tbs.previous_generation == tbs.generation - 1
                && tbs.reset_reason.is_none()
        }
        AccountEnvelopeActivationKindV1::ContinuityReset => {
            tbs.generation > 1
                && tbs.previous_generation == tbs.generation - 1
                && tbs.reset_reason.is_some()
        }
    };
    if !valid {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::NonCanonicalEncoding,
        ));
    }
    Ok(())
}

fn authority_mismatch() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::AuthorityMismatch)
}

fn private_bundle_invalid() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::PrivateBundleInvalid)
}

fn bounds_exceeded() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::BoundsExceeded)
}

fn noncanonical() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::NonCanonicalEncoding)
}

/// Decoder-only entry point for the separate cargo-fuzz crate.
///
/// This feature is never enabled by normal or release builds. It intentionally
/// returns no parsed value and exposes no cryptographic primitive.
#[cfg(feature = "account-envelope-fuzzing")]
pub fn fuzz_decode_account_envelope_v1(input: &[u8]) {
    let Some((&selector, payload)) = input.split_first() else {
        return;
    };
    match selector % 6 {
        0 => {
            let _ = parse_complete_public_bundle(payload);
        }
        1 => {
            let _ = parse_rotation_candidate(payload);
        }
        2 => {
            let _ = decode_private_bundle(payload);
        }
        3 => {
            let _ = decode_canonical_preview(payload);
        }
        4 => {
            let _ = parse_invitation_envelope(payload);
        }
        _ => {
            if let Ok(text) = std::str::from_utf8(payload) {
                let _ = invitation::default_case_fold(text);
            }
        }
    }
}

#[cfg(test)]
mod tests;
