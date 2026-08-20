use super::types::{
    AccountEnvelopeActivationKindV1, AccountEnvelopeErrorCodeV1, AccountEnvelopeErrorV1,
    AccountEnvelopePrivateBundleAuthorityV1, AccountEnvelopeResetReasonV1, AccountEnvelopeResult,
    DecodedPrivateBundleV1, FORMAT_VERSION_V1, HPKE_AEAD_ID, HPKE_KDF_ID, HPKE_KEM_ID, KEY_BYTES,
    MAX_WEB_SAFE_INTEGER, PRIVATE_BUNDLE_ACTIVE_BYTES, PRIVATE_BUNDLE_MAX_BYTES,
    PRIVATE_BUNDLE_RETIRED_BYTES, PUBLIC_BUNDLE_MAX_BYTES, PUBLIC_BUNDLE_NO_TRANSITION_BYTES,
    PUBLIC_BUNDLE_ROTATION_BYTES, PUBLIC_BUNDLE_TBS_BYTES, PrivateBundleStateV1,
    SELF_SIGNED_CANDIDATE_BYTES, SIGNATURE_BYTES, SIGNATURE_SCHEME_ID,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PublicBundleTbsV1 {
    pub account_id: [u8; 16],
    pub generation: u64,
    pub hpke_public_key: [u8; KEY_BYTES],
    pub signature_public_key: [u8; KEY_BYTES],
    pub activation_kind: AccountEnvelopeActivationKindV1,
    pub previous_generation: u64,
    pub reset_reason: Option<AccountEnvelopeResetReasonV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParsedPublicBundleV1 {
    pub tbs: PublicBundleTbsV1,
    pub tbs_bytes: [u8; PUBLIC_BUNDLE_TBS_BYTES],
    pub self_signature: [u8; SIGNATURE_BYTES],
    pub transition_signature: Option<[u8; SIGNATURE_BYTES]>,
}

pub(super) fn encode_private_bundle(bundle: &DecodedPrivateBundleV1) -> Vec<u8> {
    let expected_len = match bundle.state {
        PrivateBundleStateV1::ActiveFull => PRIVATE_BUNDLE_ACTIVE_BYTES,
        PrivateBundleStateV1::RetiredDecryptOnly => PRIVATE_BUNDLE_RETIRED_BYTES,
    };
    let mut output = Vec::with_capacity(expected_len);
    output.push(FORMAT_VERSION_V1);
    output.push(bundle.state.as_u8());
    output.extend_from_slice(&bundle.authority.account_id);
    output.extend_from_slice(&bundle.authority.generation.to_be_bytes());
    output.extend_from_slice(&bundle.authority.root_installation_id);
    output.extend_from_slice(&bundle.authority.root_authority_generation.to_be_bytes());
    output.extend_from_slice(&HPKE_KEM_ID.to_be_bytes());
    output.extend_from_slice(&HPKE_KDF_ID.to_be_bytes());
    output.extend_from_slice(&HPKE_AEAD_ID.to_be_bytes());
    output.extend_from_slice(&SIGNATURE_SCHEME_ID.to_be_bytes());
    output.extend_from_slice(&bundle.hpke_private_key);
    output.extend_from_slice(&bundle.hpke_public_key);
    match &bundle.signature_private_key {
        Some(private_key) => {
            output.push(1);
            output.extend_from_slice(private_key);
        }
        None => output.push(0),
    }
    output.extend_from_slice(&bundle.signature_public_key);
    debug_assert_eq!(output.len(), expected_len);
    output
}

pub(super) fn decode_private_bundle(input: &[u8]) -> AccountEnvelopeResult<DecodedPrivateBundleV1> {
    if input.len() > PRIVATE_BUNDLE_MAX_BYTES {
        return Err(private_bundle_invalid());
    }
    if input.len() != PRIVATE_BUNDLE_ACTIVE_BYTES && input.len() != PRIVATE_BUNDLE_RETIRED_BYTES {
        return Err(private_bundle_invalid());
    }

    let mut cursor = Cursor::new(input);
    if cursor.read_u8()? != FORMAT_VERSION_V1 {
        return Err(private_bundle_invalid());
    }
    let state = PrivateBundleStateV1::from_u8(cursor.read_u8()?)?;
    let authority = AccountEnvelopePrivateBundleAuthorityV1 {
        account_id: cursor.read_array()?,
        generation: cursor.read_u64()?,
        root_installation_id: cursor.read_array()?,
        root_authority_generation: cursor.read_u64()?,
    };
    authority.validate().map_err(|_| private_bundle_invalid())?;
    if cursor.read_u16()? != HPKE_KEM_ID
        || cursor.read_u16()? != HPKE_KDF_ID
        || cursor.read_u16()? != HPKE_AEAD_ID
        || cursor.read_u16()? != SIGNATURE_SCHEME_ID
    {
        return Err(private_bundle_invalid());
    }
    let hpke_private_key = cursor.read_array()?;
    let hpke_public_key = cursor.read_array()?;
    let signature_private_key = match cursor.read_u8()? {
        0 => None,
        1 => Some(cursor.read_array()?),
        _ => return Err(private_bundle_invalid()),
    };
    let signature_public_key = cursor.read_array()?;
    if !cursor.is_finished()
        || hpke_private_key.iter().all(|byte| *byte == 0)
        || hpke_public_key.iter().all(|byte| *byte == 0)
        || signature_public_key.iter().all(|byte| *byte == 0)
        || matches!(
            (state, signature_private_key.is_some()),
            (PrivateBundleStateV1::ActiveFull, false)
                | (PrivateBundleStateV1::RetiredDecryptOnly, true)
        )
    {
        return Err(private_bundle_invalid());
    }

    Ok(DecodedPrivateBundleV1 {
        state,
        authority,
        hpke_private_key,
        hpke_public_key,
        signature_private_key,
        signature_public_key,
    })
}

pub(super) fn encode_public_bundle_tbs(tbs: &PublicBundleTbsV1) -> [u8; PUBLIC_BUNDLE_TBS_BYTES] {
    let mut output = [0_u8; PUBLIC_BUNDLE_TBS_BYTES];
    let mut cursor = 0;
    write_bytes(&mut output, &mut cursor, &[FORMAT_VERSION_V1]);
    write_bytes(&mut output, &mut cursor, &tbs.account_id);
    write_bytes(&mut output, &mut cursor, &tbs.generation.to_be_bytes());
    write_bytes(&mut output, &mut cursor, &HPKE_KEM_ID.to_be_bytes());
    write_bytes(&mut output, &mut cursor, &HPKE_KDF_ID.to_be_bytes());
    write_bytes(&mut output, &mut cursor, &HPKE_AEAD_ID.to_be_bytes());
    write_bytes(&mut output, &mut cursor, &SIGNATURE_SCHEME_ID.to_be_bytes());
    write_bytes(&mut output, &mut cursor, &tbs.hpke_public_key);
    write_bytes(&mut output, &mut cursor, &tbs.signature_public_key);
    write_bytes(&mut output, &mut cursor, &[tbs.activation_kind as u8]);
    write_bytes(
        &mut output,
        &mut cursor,
        &tbs.previous_generation.to_be_bytes(),
    );
    write_bytes(
        &mut output,
        &mut cursor,
        &[tbs
            .reset_reason
            .map_or(0, AccountEnvelopeResetReasonV1::value)],
    );
    debug_assert_eq!(cursor, PUBLIC_BUNDLE_TBS_BYTES);
    output
}

pub(super) fn encode_complete_public_bundle(
    tbs_bytes: &[u8; PUBLIC_BUNDLE_TBS_BYTES],
    self_signature: &[u8; SIGNATURE_BYTES],
    transition_signature: Option<&[u8; SIGNATURE_BYTES]>,
) -> Vec<u8> {
    let capacity = if transition_signature.is_some() {
        PUBLIC_BUNDLE_ROTATION_BYTES
    } else {
        PUBLIC_BUNDLE_NO_TRANSITION_BYTES
    };
    let mut output = Vec::with_capacity(capacity);
    output.extend_from_slice(tbs_bytes);
    output.extend_from_slice(self_signature);
    match transition_signature {
        Some(signature) => {
            output.push(1);
            output.extend_from_slice(signature);
        }
        None => output.push(0),
    }
    debug_assert_eq!(output.len(), capacity);
    output
}

pub(super) fn encode_rotation_candidate(
    tbs_bytes: &[u8; PUBLIC_BUNDLE_TBS_BYTES],
    self_signature: &[u8; SIGNATURE_BYTES],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(SELF_SIGNED_CANDIDATE_BYTES);
    output.extend_from_slice(tbs_bytes);
    output.extend_from_slice(self_signature);
    output
}

pub(super) fn parse_complete_public_bundle(
    input: &[u8],
) -> AccountEnvelopeResult<ParsedPublicBundleV1> {
    if input.len() > PUBLIC_BUNDLE_MAX_BYTES
        || (input.len() != PUBLIC_BUNDLE_NO_TRANSITION_BYTES
            && input.len() != PUBLIC_BUNDLE_ROTATION_BYTES)
    {
        return Err(noncanonical());
    }
    let mut cursor = Cursor::new(input);
    let tbs_bytes = cursor.read_array()?;
    let tbs = parse_public_bundle_tbs(&tbs_bytes)?;
    let self_signature = cursor.read_array()?;
    let transition_signature = match cursor.read_u8()? {
        0 => None,
        1 => Some(cursor.read_array()?),
        _ => return Err(noncanonical()),
    };
    if !cursor.is_finished() {
        return Err(noncanonical());
    }
    validate_bundle_shape(&tbs, transition_signature.is_some())?;
    Ok(ParsedPublicBundleV1 {
        tbs,
        tbs_bytes,
        self_signature,
        transition_signature,
    })
}

pub(super) fn parse_rotation_candidate(
    input: &[u8],
) -> AccountEnvelopeResult<ParsedPublicBundleV1> {
    if input.len() != SELF_SIGNED_CANDIDATE_BYTES {
        return Err(noncanonical());
    }
    let mut cursor = Cursor::new(input);
    let tbs_bytes = cursor.read_array()?;
    let tbs = parse_public_bundle_tbs(&tbs_bytes)?;
    let self_signature = cursor.read_array()?;
    if !cursor.is_finished() || tbs.activation_kind != AccountEnvelopeActivationKindV1::Rotation {
        return Err(noncanonical());
    }
    validate_bundle_shape(&tbs, true)?;
    Ok(ParsedPublicBundleV1 {
        tbs,
        tbs_bytes,
        self_signature,
        transition_signature: None,
    })
}

fn parse_public_bundle_tbs(
    input: &[u8; PUBLIC_BUNDLE_TBS_BYTES],
) -> AccountEnvelopeResult<PublicBundleTbsV1> {
    let mut cursor = Cursor::new(input);
    if cursor.read_u8()? != FORMAT_VERSION_V1 {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::UnsupportedVersion,
        ));
    }
    let account_id = cursor.read_array()?;
    let generation = cursor.read_u64()?;
    if cursor.read_u16()? != HPKE_KEM_ID
        || cursor.read_u16()? != HPKE_KDF_ID
        || cursor.read_u16()? != HPKE_AEAD_ID
        || cursor.read_u16()? != SIGNATURE_SCHEME_ID
    {
        return Err(noncanonical());
    }
    let hpke_public_key = cursor.read_array()?;
    let signature_public_key = cursor.read_array()?;
    let activation_kind = AccountEnvelopeActivationKindV1::from_u8(cursor.read_u8()?)?;
    let previous_generation = cursor.read_u64()?;
    let reset_reason_raw = cursor.read_u8()?;
    if !cursor.is_finished()
        || !super::types::is_canonical_uuid(&account_id)
        || generation == 0
        || generation > MAX_WEB_SAFE_INTEGER
        || previous_generation > MAX_WEB_SAFE_INTEGER
        || hpke_public_key.iter().all(|byte| *byte == 0)
        || signature_public_key.iter().all(|byte| *byte == 0)
    {
        return Err(noncanonical());
    }
    let reset_reason = if reset_reason_raw == 0 {
        None
    } else {
        Some(AccountEnvelopeResetReasonV1::new(reset_reason_raw)?)
    };
    Ok(PublicBundleTbsV1 {
        account_id,
        generation,
        hpke_public_key,
        signature_public_key,
        activation_kind,
        previous_generation,
        reset_reason,
    })
}

fn validate_bundle_shape(
    tbs: &PublicBundleTbsV1,
    has_transition_signature: bool,
) -> AccountEnvelopeResult<()> {
    let valid = match tbs.activation_kind {
        AccountEnvelopeActivationKindV1::Initial => {
            tbs.generation == 1
                && tbs.previous_generation == 0
                && tbs.reset_reason.is_none()
                && !has_transition_signature
        }
        AccountEnvelopeActivationKindV1::Rotation => {
            tbs.generation > 1
                && tbs.previous_generation == tbs.generation - 1
                && tbs.reset_reason.is_none()
                && has_transition_signature
        }
        AccountEnvelopeActivationKindV1::ContinuityReset => {
            tbs.generation > 1
                && tbs.previous_generation == tbs.generation - 1
                && tbs.reset_reason.is_some()
                && !has_transition_signature
        }
    };
    if !valid {
        return Err(noncanonical());
    }
    Ok(())
}

fn write_bytes<const N: usize>(output: &mut [u8; N], cursor: &mut usize, value: &[u8]) {
    let end = *cursor + value.len();
    output[*cursor..end].copy_from_slice(value);
    *cursor = end;
}

struct Cursor<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    fn read_u8(&mut self) -> AccountEnvelopeResult<u8> {
        Ok(self.read_array::<1>()?[0])
    }

    fn read_u16(&mut self) -> AccountEnvelopeResult<u16> {
        Ok(u16::from_be_bytes(self.read_array()?))
    }

    fn read_u64(&mut self) -> AccountEnvelopeResult<u64> {
        Ok(u64::from_be_bytes(self.read_array()?))
    }

    fn read_array<const N: usize>(&mut self) -> AccountEnvelopeResult<[u8; N]> {
        let end = self.position.checked_add(N).ok_or_else(noncanonical)?;
        let bytes = self
            .input
            .get(self.position..end)
            .ok_or_else(noncanonical)?;
        let mut output = [0_u8; N];
        output.copy_from_slice(bytes);
        self.position = end;
        Ok(output)
    }

    fn is_finished(&self) -> bool {
        self.position == self.input.len()
    }
}

fn noncanonical() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::NonCanonicalEncoding)
}

fn private_bundle_invalid() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::PrivateBundleInvalid)
}
