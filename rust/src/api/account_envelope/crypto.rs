use openmls_rust_crypto::RustCrypto;
use openmls_traits::{
    crypto::OpenMlsCrypto,
    random::OpenMlsRand,
    types::{HpkeAeadType, HpkeCiphertext, HpkeConfig, HpkeKdfType, HpkeKemType, SignatureScheme},
};
use zeroize::Zeroizing;

use super::types::{
    AccountEnvelopeErrorCodeV1, AccountEnvelopeErrorV1, AccountEnvelopePrivateBundleAuthorityV1,
    AccountEnvelopeResult, DecodedPrivateBundleV1, KEY_BYTES, PrivateBundleStateV1,
    SIGNATURE_BYTES,
};

pub(super) const BUNDLE_SELF_SIGNATURE_DOMAIN_V1: &[u8] =
    b"openmls_dart/account-envelope/bundle-self-signature/v1\0";
pub(super) const BUNDLE_TRANSITION_SIGNATURE_DOMAIN_V1: &[u8] =
    b"openmls_dart/account-envelope/bundle-transition-signature/v1\0";
pub(super) const BUNDLE_DIGEST_DOMAIN_V1: &[u8] =
    b"openmls_dart/account-envelope/bundle-digest/v1\0";
pub(super) const ENVELOPE_SIGNATURE_DOMAIN_V1: &[u8] =
    b"openmls_dart/account-envelope/envelope-signature/v1\0";
const HPKE_CONTEXT_DOMAIN_V1: &[u8] = b"openmls_dart/account-envelope/hpke-context/v1\0";
const KEY_SELF_TEST_INFO_V1: &[u8] = b"openmls_dart/account-envelope/key-self-test/v1\0";
const KEY_SELF_TEST_AAD_V1: &[u8] = b"account-envelope-key-pair";
const KEY_SELF_TEST_PLAINTEXT_V1: &[u8] = b"ok";
const SIGNATURE_TRANSCRIPT_MAX_BYTES: usize = 4096;

pub(super) fn generate_private_bundle(
    authority: AccountEnvelopePrivateBundleAuthorityV1,
) -> AccountEnvelopeResult<DecodedPrivateBundleV1> {
    let provider = RustCrypto::default();
    let ikm = Zeroizing::new(
        provider
            .random_vec(KEY_BYTES)
            .map_err(|_| internal_crypto_failure())?,
    );
    let hpke_key_pair = provider
        .derive_hpke_keypair(fixed_hpke_config(), ikm.as_slice())
        .map_err(|_| internal_crypto_failure())?;
    let hpke_private = Zeroizing::new(hpke_key_pair.private.to_vec());
    let hpke_public = hpke_key_pair.public;

    let (signature_private, signature_public) = provider
        .signature_key_gen(SignatureScheme::ED25519)
        .map_err(|_| internal_crypto_failure())?;
    let signature_private = Zeroizing::new(signature_private);

    let bundle = DecodedPrivateBundleV1 {
        state: PrivateBundleStateV1::ActiveFull,
        authority,
        hpke_private_key: exact_key(&hpke_private)?,
        hpke_public_key: exact_key(&hpke_public)?,
        signature_private_key: Some(exact_key(&signature_private)?),
        signature_public_key: exact_key(&signature_public)?,
    };
    validate_private_bundle_crypto(&bundle)?;
    Ok(bundle)
}

#[cfg(test)]
pub(super) fn private_bundle_from_test_material(
    authority: AccountEnvelopePrivateBundleAuthorityV1,
    hpke_ikm: [u8; KEY_BYTES],
    signature_private_key: [u8; KEY_BYTES],
    signature_public_key: [u8; KEY_BYTES],
) -> AccountEnvelopeResult<DecodedPrivateBundleV1> {
    let provider = RustCrypto::default();
    let hpke_ikm = Zeroizing::new(hpke_ikm);
    let hpke_key_pair = provider
        .derive_hpke_keypair(fixed_hpke_config(), hpke_ikm.as_slice())
        .map_err(|_| internal_crypto_failure())?;
    let hpke_private = Zeroizing::new(hpke_key_pair.private.to_vec());
    let bundle = DecodedPrivateBundleV1 {
        state: PrivateBundleStateV1::ActiveFull,
        authority,
        hpke_private_key: exact_key(&hpke_private)?,
        hpke_public_key: exact_key(&hpke_key_pair.public)?,
        signature_private_key: Some(signature_private_key),
        signature_public_key,
    };
    validate_private_bundle_crypto(&bundle)?;
    Ok(bundle)
}

pub(super) fn validate_private_bundle_crypto(
    bundle: &DecodedPrivateBundleV1,
) -> AccountEnvelopeResult<()> {
    let provider = RustCrypto::default();
    if let Some(signature_private_key) = &bundle.signature_private_key {
        let signature = Zeroizing::new(
            provider
                .sign(
                    SignatureScheme::ED25519,
                    KEY_SELF_TEST_INFO_V1,
                    signature_private_key,
                )
                .map_err(|_| private_bundle_invalid())?,
        );
        provider
            .verify_signature(
                SignatureScheme::ED25519,
                KEY_SELF_TEST_INFO_V1,
                &bundle.signature_public_key,
                signature.as_slice(),
            )
            .map_err(|_| private_bundle_invalid())?;
    }

    let ciphertext = provider
        .hpke_seal(
            fixed_hpke_config(),
            &bundle.hpke_public_key,
            KEY_SELF_TEST_INFO_V1,
            KEY_SELF_TEST_AAD_V1,
            KEY_SELF_TEST_PLAINTEXT_V1,
        )
        .map_err(|_| private_bundle_invalid())?;
    let opened = Zeroizing::new(
        provider
            .hpke_open(
                fixed_hpke_config(),
                &ciphertext,
                &bundle.hpke_private_key,
                KEY_SELF_TEST_INFO_V1,
                KEY_SELF_TEST_AAD_V1,
            )
            .map_err(|_| private_bundle_invalid())?,
    );
    if opened.as_slice() != KEY_SELF_TEST_PLAINTEXT_V1 {
        return Err(private_bundle_invalid());
    }
    Ok(())
}

pub(super) fn sign_domain_message(
    domain: &[u8],
    message_parts: &[&[u8]],
    private_key: &[u8; KEY_BYTES],
) -> AccountEnvelopeResult<[u8; SIGNATURE_BYTES]> {
    let provider = RustCrypto::default();
    let transcript = transcript(domain, message_parts)?;
    let signature = Zeroizing::new(
        provider
            .sign(SignatureScheme::ED25519, &transcript, private_key)
            .map_err(|_| internal_crypto_failure())?,
    );
    exact_signature(&signature)
}

pub(super) fn verify_domain_signature(
    domain: &[u8],
    message_parts: &[&[u8]],
    public_key: &[u8; KEY_BYTES],
    signature: &[u8; SIGNATURE_BYTES],
) -> AccountEnvelopeResult<()> {
    let provider = RustCrypto::default();
    let transcript = transcript(domain, message_parts)?;
    provider
        .verify_signature(SignatureScheme::ED25519, &transcript, public_key, signature)
        .map_err(|_| AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::SignatureInvalid))
}

#[cfg(test)]
pub(super) fn hpke_round_trip_for_test(
    public_key: &[u8; KEY_BYTES],
    private_key: &[u8; KEY_BYTES],
    plaintext: &[u8],
) -> AccountEnvelopeResult<Vec<u8>> {
    let provider = RustCrypto::default();
    let ciphertext: HpkeCiphertext = provider
        .hpke_seal(
            fixed_hpke_config(),
            public_key,
            KEY_SELF_TEST_INFO_V1,
            KEY_SELF_TEST_AAD_V1,
            plaintext,
        )
        .map_err(|_| internal_crypto_failure())?;
    provider
        .hpke_open(
            fixed_hpke_config(),
            &ciphertext,
            private_key,
            KEY_SELF_TEST_INFO_V1,
            KEY_SELF_TEST_AAD_V1,
        )
        .map_err(|_| AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::HpkeOpenFailed))
}

pub(super) fn hpke_seal_invitation(
    recipient_public_key: &[u8; KEY_BYTES],
    canonical_header: &[u8],
    plaintext: &[u8],
) -> AccountEnvelopeResult<([u8; KEY_BYTES], Vec<u8>)> {
    let provider = RustCrypto::default();
    let sealed = provider
        .hpke_seal(
            fixed_hpke_config(),
            recipient_public_key,
            HPKE_CONTEXT_DOMAIN_V1,
            canonical_header,
            plaintext,
        )
        .map_err(|_| internal_crypto_failure())?;
    let encapsulation = exact_key(sealed.kem_output.as_slice())?;
    Ok((encapsulation, sealed.ciphertext.into()))
}

pub(super) fn hpke_open_invitation(
    recipient_private_key: &[u8; KEY_BYTES],
    canonical_header: &[u8],
    encapsulation: &[u8; KEY_BYTES],
    ciphertext: &[u8],
) -> AccountEnvelopeResult<Zeroizing<Vec<u8>>> {
    let provider = RustCrypto::default();
    let sealed = HpkeCiphertext {
        kem_output: encapsulation.to_vec().into(),
        ciphertext: ciphertext.to_vec().into(),
    };
    provider
        .hpke_open(
            fixed_hpke_config(),
            &sealed,
            recipient_private_key,
            HPKE_CONTEXT_DOMAIN_V1,
            canonical_header,
        )
        .map(Zeroizing::new)
        .map_err(|_| AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::HpkeOpenFailed))
}

fn transcript(domain: &[u8], parts: &[&[u8]]) -> AccountEnvelopeResult<Vec<u8>> {
    let total = parts.iter().try_fold(domain.len(), |total, part| {
        total
            .checked_add(part.len())
            .ok_or_else(|| AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::BoundsExceeded))
    })?;
    if total > SIGNATURE_TRANSCRIPT_MAX_BYTES {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::BoundsExceeded,
        ));
    }
    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(domain);
    for part in parts {
        output.extend_from_slice(part);
    }
    Ok(output)
}

fn fixed_hpke_config() -> HpkeConfig {
    HpkeConfig(
        HpkeKemType::DhKem25519,
        HpkeKdfType::HkdfSha256,
        HpkeAeadType::AesGcm128,
    )
}

fn exact_key(input: &[u8]) -> AccountEnvelopeResult<[u8; KEY_BYTES]> {
    input.try_into().map_err(|_| internal_crypto_failure())
}

fn exact_signature(input: &[u8]) -> AccountEnvelopeResult<[u8; SIGNATURE_BYTES]> {
    input.try_into().map_err(|_| internal_crypto_failure())
}

fn private_bundle_invalid() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::PrivateBundleInvalid)
}

fn internal_crypto_failure() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::InternalCryptoFailure)
}
