use openmls_rust_crypto::RustCrypto;
use openmls_traits::{
    crypto::OpenMlsCrypto,
    types::{HpkeAeadType, HpkeConfig, HpkeKdfType, HpkeKemType, SignatureScheme},
};
use zeroize::Zeroizing;

use super::codec::{decode_private_bundle, encode_private_bundle};
use super::crypto::{hpke_round_trip_for_test, private_bundle_from_test_material};
use super::invitation::default_case_fold;
use super::types::{
    AccountEnvelopeErrorCodeV1, AccountEnvelopePaddingClassV1,
    AccountEnvelopePrivateBundleAuthorityV1, PRIVATE_BUNDLE_ACTIVE_BYTES,
    PRIVATE_BUNDLE_RETIRED_BYTES, PUBLIC_BUNDLE_NO_TRANSITION_BYTES, PUBLIC_BUNDLE_ROTATION_BYTES,
    PrivateBundleStateV1, SELF_SIGNED_CANDIDATE_BYTES,
};
use super::*;

const ACCOUNT_ID: [u8; 16] = [
    0x01, 0x9a, 0x12, 0x34, 0x5a, 0x67, 0x70, 0x01, 0x80, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
];
const ROOT_INSTALLATION_ID: [u8; 16] = [
    0x01, 0x9a, 0x12, 0x34, 0x5a, 0x68, 0x70, 0x02, 0x80, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
];
const RECIPIENT_ACCOUNT_ID: [u8; 16] = [
    0x01, 0x9a, 0x12, 0x34, 0x5a, 0x69, 0x70, 0x03, 0x80, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
];
const RECIPIENT_ROOT_ID: [u8; 16] = [
    0x01, 0x9a, 0x12, 0x34, 0x5a, 0x6a, 0x70, 0x04, 0x80, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
];
const ENVELOPE_ID: [u8; 16] = [
    0x01, 0x9a, 0x12, 0x34, 0x5a, 0x6b, 0x70, 0x05, 0x80, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
];
const INVITE_ID: [u8; 16] = [
    0x01, 0x9a, 0x12, 0x34, 0x5a, 0x6c, 0x70, 0x06, 0x80, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
];

fn authority(generation: u64) -> AccountEnvelopePrivateBundleAuthorityV1 {
    AccountEnvelopePrivateBundleAuthorityV1 {
        account_id: ACCOUNT_ID,
        generation,
        root_installation_id: ROOT_INSTALLATION_ID,
        root_authority_generation: 1,
    }
}

fn generated(generation: u64) -> GenerateAccountEnvelopeKeyBundleResultV1 {
    generate_key_bundle_v1(authority(generation)).expect("test key generation must succeed")
}

fn recipient_authority(generation: u64) -> AccountEnvelopePrivateBundleAuthorityV1 {
    AccountEnvelopePrivateBundleAuthorityV1 {
        account_id: RECIPIENT_ACCOUNT_ID,
        generation,
        root_installation_id: RECIPIENT_ROOT_ID,
        root_authority_generation: 1,
    }
}

fn initial_public_bundle(
    authority: AccountEnvelopePrivateBundleAuthorityV1,
    private_bundle: &[u8],
) -> Vec<u8> {
    let result = create_self_signed_public_bundle_v1(
        authority.account_id,
        1,
        AccountEnvelopeActivationKindV1::Initial,
        None,
        0,
        authority,
        private_bundle.to_vec(),
    )
    .expect("initial public bundle must succeed");
    let SelfSignedPublicBundleResultV1::CanonicalPublicBundle(bundle) = result else {
        panic!("initial bundle must be complete")
    };
    bundle
}

fn invitation_authority(
    padding_class: AccountEnvelopePaddingClassV1,
) -> ContextInvitationAuthorityV1 {
    ContextInvitationAuthorityV1 {
        envelope_id: ENVELOPE_ID,
        invite_id: INVITE_ID,
        sender_account_id: ACCOUNT_ID,
        sender_generation: 1,
        recipient_account_id: RECIPIENT_ACCOUNT_ID,
        recipient_generation: 1,
        authority_attempt: 1,
        relay_slot_version: 1,
        server_created_at_unix_ms: 1_788_000_000_000,
        server_expires_at_unix_ms: 1_788_086_400_000,
        padding_class,
    }
}

#[test]
fn generate_validates_authority_and_returns_fixed_active_frame() {
    let result = generated(1);
    assert_eq!(result.private_bundle.len(), PRIVATE_BUNDLE_ACTIVE_BYTES);
    let parsed = decode_private_bundle(&result.private_bundle).expect("frame must decode");
    assert_eq!(parsed.authority, authority(1));
    assert_eq!(parsed.state, PrivateBundleStateV1::ActiveFull);

    let invalid = AccountEnvelopePrivateBundleAuthorityV1 {
        generation: (1_u64 << 53),
        ..authority(1)
    };
    assert_eq!(
        generate_key_bundle_v1(invalid)
            .err()
            .map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::AuthorityMismatch)
    );
}

#[test]
fn initial_and_reset_bundles_have_canonical_shape_and_verify() {
    let generation_one = generated(1);
    let initial = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        1,
        AccountEnvelopeActivationKindV1::Initial,
        None,
        0,
        authority(1),
        generation_one.private_bundle.to_vec(),
    )
    .expect("initial bundle must succeed");
    let SelfSignedPublicBundleResultV1::CanonicalPublicBundle(initial) = initial else {
        panic!("initial must be complete")
    };
    assert_eq!(initial.len(), PUBLIC_BUNDLE_NO_TRANSITION_BYTES);
    let summary = verify_canonical_public_bundle_v1(&initial).expect("initial must verify");
    assert_eq!(summary.generation, 1);
    assert_eq!(
        summary.activation_kind,
        AccountEnvelopeActivationKindV1::Initial
    );
    assert_eq!(
        summary.digest_sha256,
        public_bundle_digest_v1(&initial).unwrap()
    );

    let generation_two = generated(2);
    let reset = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        2,
        AccountEnvelopeActivationKindV1::ContinuityReset,
        Some(AccountEnvelopeResetReasonV1::new(1).unwrap()),
        1,
        authority(2),
        generation_two.private_bundle.to_vec(),
    )
    .expect("reset bundle must succeed");
    let SelfSignedPublicBundleResultV1::CanonicalPublicBundle(reset) = reset else {
        panic!("reset must be complete")
    };
    assert_eq!(reset.len(), PUBLIC_BUNDLE_NO_TRANSITION_BYTES);
    let summary = verify_canonical_public_bundle_v1(&reset).expect("reset must verify");
    assert_eq!(summary.generation, 2);
    assert_eq!(
        summary.activation_kind,
        AccountEnvelopeActivationKindV1::ContinuityReset
    );
}

#[test]
fn rotation_is_nonpublishable_until_predecessor_authorizes_it() {
    let previous = generated(1);
    let successor = generated(2);
    let candidate = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        2,
        AccountEnvelopeActivationKindV1::Rotation,
        None,
        1,
        authority(2),
        successor.private_bundle.to_vec(),
    )
    .expect("candidate must succeed");
    let SelfSignedPublicBundleResultV1::NonPublishableRotationCandidate(candidate) = candidate
    else {
        panic!("rotation must remain a candidate")
    };
    assert_eq!(candidate.len(), SELF_SIGNED_CANDIDATE_BYTES);
    assert_eq!(
        verify_canonical_public_bundle_v1(&candidate)
            .err()
            .map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::NonCanonicalEncoding)
    );

    let result = authorize_successor_public_bundle_v1(
        authority(1),
        previous.private_bundle.to_vec(),
        candidate.clone(),
    )
    .expect("authorization must succeed");
    assert_eq!(
        result.authorized_canonical_successor_public_bundle.len(),
        PUBLIC_BUNDLE_ROTATION_BYTES
    );
    let summary =
        verify_canonical_public_bundle_v1(&result.authorized_canonical_successor_public_bundle)
            .expect("authorized rotation must verify");
    assert_eq!(summary.generation, 2);
    assert_eq!(
        summary.activation_kind,
        AccountEnvelopeActivationKindV1::Rotation
    );

    assert_eq!(
        result.retired_previous_private_bundle_candidate.len(),
        PRIVATE_BUNDLE_RETIRED_BYTES
    );
    let retired = decode_private_bundle(&result.retired_previous_private_bundle_candidate)
        .expect("retired frame must decode");
    assert_eq!(retired.state, PrivateBundleStateV1::RetiredDecryptOnly);
    assert!(retired.signature_private_key.is_none());

    let repeat = authorize_successor_public_bundle_v1(
        authority(1),
        previous.private_bundle.to_vec(),
        candidate,
    )
    .expect("exact retry must be deterministic");
    assert_eq!(
        repeat.authorized_canonical_successor_public_bundle,
        result.authorized_canonical_successor_public_bundle
    );
    assert_eq!(
        repeat.retired_previous_private_bundle_candidate.as_slice(),
        result.retired_previous_private_bundle_candidate.as_slice()
    );
}

#[test]
fn authority_mismatch_and_corruption_return_no_result() {
    let generated = generated(1);
    let wrong_authority = AccountEnvelopePrivateBundleAuthorityV1 {
        root_authority_generation: 2,
        ..authority(1)
    };
    let mismatch = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        1,
        AccountEnvelopeActivationKindV1::Initial,
        None,
        0,
        wrong_authority,
        generated.private_bundle.to_vec(),
    );
    assert_eq!(
        mismatch.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::AuthorityMismatch)
    );

    let mut corrupted = generated.private_bundle.to_vec();
    corrupted[90] ^= 1;
    let corrupt = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        1,
        AccountEnvelopeActivationKindV1::Initial,
        None,
        0,
        authority(1),
        corrupted,
    );
    assert_eq!(
        corrupt.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::PrivateBundleInvalid)
    );
}

#[test]
fn retired_frame_cannot_sign_and_reencoding_is_canonical() {
    let generated = generated(1);
    let mut parsed = decode_private_bundle(&generated.private_bundle).unwrap();
    parsed.state = PrivateBundleStateV1::RetiredDecryptOnly;
    parsed.signature_private_key = None;
    let retired = encode_private_bundle(&parsed);
    assert_eq!(retired.len(), PRIVATE_BUNDLE_RETIRED_BYTES);
    assert_eq!(
        encode_private_bundle(&decode_private_bundle(&retired).unwrap()),
        retired
    );

    let result = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        1,
        AccountEnvelopeActivationKindV1::Initial,
        None,
        0,
        authority(1),
        retired,
    );
    assert_eq!(
        result.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::PrivateBundleInvalid)
    );
}

#[test]
fn rfc_8032_ed25519_test_vector_one_matches_provider() {
    let private =
        decode_hex::<32>("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
    let public =
        decode_hex::<32>("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let expected_signature = decode_hex::<64>(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
         5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );
    let provider = RustCrypto::default();
    let signature = Zeroizing::new(
        provider
            .sign(SignatureScheme::ED25519, b"", &private)
            .expect("RFC signing must succeed"),
    );
    assert_eq!(signature.as_slice(), expected_signature);
    provider
        .verify_signature(SignatureScheme::ED25519, b"", &public, &signature)
        .expect("RFC signature must verify");
}

#[test]
fn rfc_9180_appendix_a1_recipient_key_derivation_matches_provider() {
    let ikm = Zeroizing::new(decode_hex::<32>(
        "6db9df30aa07dd42ee5e8181afdb977e538f5e1fec8a06223f33f7013e525037",
    ));
    let expected_public =
        decode_hex::<32>("3948cfe0ad1ddb695d780e59077195da6c56506b027329794ab02bca80815c4d");
    let expected_private = Zeroizing::new(decode_hex::<32>(
        "4612c550263fc8ad58375df3f557aac531d26850903e55a9f23f21d8534e8ac8",
    ));
    let provider = RustCrypto::default();
    let key_pair = provider
        .derive_hpke_keypair(
            HpkeConfig(
                HpkeKemType::DhKem25519,
                HpkeKdfType::HkdfSha256,
                HpkeAeadType::AesGcm128,
            ),
            ikm.as_slice(),
        )
        .expect("RFC 9180 DeriveKeyPair must succeed");
    assert_eq!(key_pair.public, expected_public);
    assert_eq!(&*key_pair.private, expected_private.as_slice());
}

#[test]
fn deterministic_material_proves_hpke_pair_and_private_codec() {
    let signature_private =
        decode_hex::<32>("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
    let signature_public =
        decode_hex::<32>("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let bundle = private_bundle_from_test_material(
        authority(1),
        [0x42; 32],
        signature_private,
        signature_public,
    )
    .expect("fixed material must derive");
    let opened = hpke_round_trip_for_test(
        &bundle.hpke_public_key,
        &bundle.hpke_private_key,
        b"phase-one-hpke-provider-check",
    )
    .expect("HPKE round trip must succeed");
    assert_eq!(opened, b"phase-one-hpke-provider-check");

    let encoded = encode_private_bundle(&bundle);
    assert_eq!(encoded.len(), PRIVATE_BUNDLE_ACTIVE_BYTES);
    let reparsed = decode_private_bundle(&encoded).expect("fixed frame must decode");
    assert_eq!(reparsed.authority, authority(1));
    assert_eq!(reparsed.hpke_public_key, bundle.hpke_public_key);
}

#[test]
fn bundle_parser_rejects_trailing_bytes_and_signature_changes() {
    let generated = generated(1);
    let bundle = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        1,
        AccountEnvelopeActivationKindV1::Initial,
        None,
        0,
        authority(1),
        generated.private_bundle.to_vec(),
    )
    .unwrap();
    let SelfSignedPublicBundleResultV1::CanonicalPublicBundle(bundle) = bundle else {
        unreachable!()
    };

    let mut trailing = bundle.clone();
    trailing.push(0);
    assert_eq!(
        verify_canonical_public_bundle_v1(&trailing)
            .err()
            .map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::NonCanonicalEncoding)
    );

    let mut changed = bundle;
    changed[120] ^= 1;
    assert_eq!(
        verify_canonical_public_bundle_v1(&changed)
            .err()
            .map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::SignatureInvalid)
    );
}

#[test]
fn invitation_round_trip_honors_each_authenticated_padding_class() {
    for (padding_class, expected_envelope_bytes) in [
        (AccountEnvelopePaddingClassV1::Bytes512, 741),
        (AccountEnvelopePaddingClassV1::Bytes1024, 1253),
        (AccountEnvelopePaddingClassV1::Bytes2048, 2277),
    ] {
        let sender = generated(1);
        let recipient = generate_key_bundle_v1(recipient_authority(1)).unwrap();
        let sender_public = initial_public_bundle(authority(1), &sender.private_bundle);
        let recipient_public =
            initial_public_bundle(recipient_authority(1), &recipient.private_bundle);
        let authority = invitation_authority(padding_class);
        let sealed = seal_context_invitation_preview_v1(
            authority,
            super::tests::authority(1),
            ContextInvitationPreviewV1 {
                title: Some("\u{3000}Cafe\u{301}\u{00a0}".to_owned()),
                tags: vec!["Rust".to_owned(), "Straße".to_owned()],
            },
            recipient_public,
            sender.private_bundle.to_vec(),
        )
        .expect("sealing must succeed");
        assert_eq!(sealed.canonical_envelope.len(), expected_envelope_bytes);
        let opened = verify_and_open_context_invitation_preview_v1(
            sealed.canonical_envelope,
            ExpectedContextInvitationAuthorityV1 {
                invitation: authority,
                local_root_installation_id: RECIPIENT_ROOT_ID,
                local_root_authority_generation: 1,
            },
            recipient.private_bundle.to_vec(),
            sender_public,
        )
        .expect("opening must succeed");
        assert_eq!(opened.preview.title.as_deref(), Some("Café"));
        assert_eq!(opened.preview.tags, ["Rust", "Straße"]);
    }
}

#[test]
fn invitation_rejects_folded_duplicate_tags_before_crypto_output() {
    let sender = generated(1);
    let recipient = generate_key_bundle_v1(recipient_authority(1)).unwrap();
    let recipient_public = initial_public_bundle(recipient_authority(1), &recipient.private_bundle);
    let result = seal_context_invitation_preview_v1(
        invitation_authority(AccountEnvelopePaddingClassV1::Bytes2048),
        authority(1),
        ContextInvitationPreviewV1 {
            title: None,
            tags: vec!["Straße".to_owned(), "STRASSE".to_owned()],
        },
        recipient_public,
        sender.private_bundle.to_vec(),
    );
    assert_eq!(
        result.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::PlaintextSchemaInvalid)
    );
}

#[test]
fn case_folding_uses_full_unicode_17_data() {
    assert_eq!(super::unicode17_casefold::UNICODE_VERSION, (17, 0, 0));
    assert_eq!(
        super::unicode17_casefold::FULL_DEFAULT_CASE_FOLD.len(),
        1585
    );
    assert_eq!(default_case_fold("Straße"), "strasse");
    // BERIA ERFE casing was added in Unicode 17 and is absent from Unicode 16.
    assert_eq!(default_case_fold("\u{16EA0}"), "\u{16EBB}");
}

#[test]
fn invitation_expected_authority_and_signature_fail_closed() {
    let sender = generated(1);
    let recipient = generate_key_bundle_v1(recipient_authority(1)).unwrap();
    let sender_public = initial_public_bundle(authority(1), &sender.private_bundle);
    let recipient_public = initial_public_bundle(recipient_authority(1), &recipient.private_bundle);
    let authority = invitation_authority(AccountEnvelopePaddingClassV1::Bytes512);
    let sealed = seal_context_invitation_preview_v1(
        authority,
        super::tests::authority(1),
        ContextInvitationPreviewV1 {
            title: Some("Bound preview".to_owned()),
            tags: vec![],
        },
        recipient_public,
        sender.private_bundle.to_vec(),
    )
    .unwrap();

    let mut wrong = authority;
    wrong.authority_attempt = 2;
    let mismatch = verify_and_open_context_invitation_preview_v1(
        sealed.canonical_envelope.clone(),
        ExpectedContextInvitationAuthorityV1 {
            invitation: wrong,
            local_root_installation_id: RECIPIENT_ROOT_ID,
            local_root_authority_generation: 1,
        },
        recipient.private_bundle.to_vec(),
        sender_public.clone(),
    );
    assert_eq!(
        mismatch.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::AuthorityMismatch)
    );

    let mut tampered = sealed.canonical_envelope;
    let ciphertext_offset = super::types::INVITATION_HEADER_BYTES + 32 + 2;
    tampered[ciphertext_offset] ^= 1;
    let invalid = verify_and_open_context_invitation_preview_v1(
        tampered,
        ExpectedContextInvitationAuthorityV1 {
            invitation: authority,
            local_root_installation_id: RECIPIENT_ROOT_ID,
            local_root_authority_generation: 1,
        },
        recipient.private_bundle.to_vec(),
        sender_public,
    );
    assert_eq!(
        invalid.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::SignatureInvalid)
    );
}

#[test]
fn continuity_first_observation_and_zero_link_pin_are_explicit() {
    let generated = generated(1);
    let pinned = initial_public_bundle(authority(1), &generated.private_bundle);
    let first = verify_continuity_response_v1(
        None,
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: vec![pinned.clone()],
        },
        false,
    )
    .expect("first observation must verify self-signature");
    assert_eq!(
        first.disposition,
        AccountEnvelopeContinuityDispositionV1::FirstObservation
    );
    assert_eq!(first.verified_public_bundle, pinned);

    let unchanged = verify_continuity_response_v1(
        Some(first.verified_public_bundle.clone()),
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: vec![],
        },
        false,
    )
    .expect("zero-link response must retain the pin");
    assert_eq!(
        unchanged.disposition,
        AccountEnvelopeContinuityDispositionV1::PinnedUnchanged
    );
    assert_eq!(unchanged.verified_summary.generation, 1);
}

#[test]
fn continuity_verifies_consecutive_rotations_and_rejects_reordering() {
    let generation_one = generated(1);
    let pinned = initial_public_bundle(authority(1), &generation_one.private_bundle);
    let generation_two = generated(2);
    let rotation_two = authorized_rotation(&generation_one, &generation_two, 1, 2);
    let generation_three = generated(3);
    let rotation_three = authorized_rotation(&generation_two, &generation_three, 2, 3);

    let verified = verify_continuity_response_v1(
        Some(pinned.clone()),
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: vec![rotation_two.clone(), rotation_three.clone()],
        },
        false,
    )
    .expect("ordered rotation chain must verify");
    assert_eq!(
        verified.disposition,
        AccountEnvelopeContinuityDispositionV1::RotationChainVerified
    );
    assert_eq!(verified.verified_summary.generation, 3);

    let reordered = verify_continuity_response_v1(
        Some(pinned),
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: vec![rotation_three, rotation_two],
        },
        false,
    );
    assert_eq!(
        reordered.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::AuthorityMismatch)
    );
}

#[test]
fn continuity_reset_requires_acceptance_and_manual_reanchor_is_separate() {
    let generation_one = generated(1);
    let pinned = initial_public_bundle(authority(1), &generation_one.private_bundle);
    let generation_two = generated(2);
    let reset = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        2,
        AccountEnvelopeActivationKindV1::ContinuityReset,
        Some(AccountEnvelopeResetReasonV1::new(2).unwrap()),
        1,
        authority(2),
        generation_two.private_bundle.to_vec(),
    )
    .unwrap();
    let SelfSignedPublicBundleResultV1::CanonicalPublicBundle(reset) = reset else {
        unreachable!()
    };

    let observed = verify_continuity_response_v1(
        Some(pinned.clone()),
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: vec![reset.clone()],
        },
        false,
    )
    .expect("valid reset anchor must be recognized");
    assert_eq!(
        observed.disposition,
        AccountEnvelopeContinuityDispositionV1::ResetAnchorRequiresAcceptance
    );
    assert_eq!(observed.verified_summary.generation, 2);

    let reanchored = verify_continuity_response_v1(
        Some(pinned),
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: vec![reset],
        },
        true,
    )
    .expect("explicit manual reanchor must be reported separately");
    assert_eq!(
        reanchored.disposition,
        AccountEnvelopeContinuityDispositionV1::ManualReanchorVerified
    );
}

#[test]
fn continuity_rejects_more_than_32_links_before_parsing_them() {
    let generated = generated(1);
    let pinned = initial_public_bundle(authority(1), &generated.private_bundle);
    let result = verify_continuity_response_v1(
        Some(pinned.clone()),
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: vec![pinned; 33],
        },
        false,
    );
    assert_eq!(
        result.err().map(|error| error.code),
        Some(AccountEnvelopeErrorCodeV1::BoundsExceeded)
    );
}

#[test]
fn continuity_accepts_exactly_32_consecutive_rotation_links() {
    let mut previous = generated(1);
    let pinned = initial_public_bundle(authority(1), &previous.private_bundle);
    let mut links = Vec::with_capacity(32);
    for generation in 2..=33 {
        let successor = generated(generation);
        links.push(authorized_rotation(
            &previous,
            &successor,
            generation - 1,
            generation,
        ));
        previous = successor;
    }
    let verified = verify_continuity_response_v1(
        Some(pinned),
        AccountEnvelopeContinuityResponseV1 {
            public_bundles: links,
        },
        false,
    )
    .expect("the exact 32-link protocol limit must verify");
    assert_eq!(verified.verified_summary.generation, 33);
    assert_eq!(
        verified.disposition,
        AccountEnvelopeContinuityDispositionV1::RotationChainVerified
    );
}

fn authorized_rotation(
    previous: &GenerateAccountEnvelopeKeyBundleResultV1,
    successor: &GenerateAccountEnvelopeKeyBundleResultV1,
    previous_generation: u64,
    successor_generation: u64,
) -> Vec<u8> {
    let candidate = create_self_signed_public_bundle_v1(
        ACCOUNT_ID,
        successor_generation,
        AccountEnvelopeActivationKindV1::Rotation,
        None,
        previous_generation,
        authority(successor_generation),
        successor.private_bundle.to_vec(),
    )
    .expect("rotation candidate must succeed");
    let SelfSignedPublicBundleResultV1::NonPublishableRotationCandidate(candidate) = candidate
    else {
        panic!("rotation must remain a candidate")
    };
    authorize_successor_public_bundle_v1(
        authority(previous_generation),
        previous.private_bundle.to_vec(),
        candidate,
    )
    .expect("predecessor must authorize successor")
    .authorized_canonical_successor_public_bundle
}

fn decode_hex<const N: usize>(input: &str) -> [u8; N] {
    let compact: String = input
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    assert_eq!(compact.len(), N * 2);
    let mut output = [0_u8; N];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16).unwrap();
    }
    output
}
