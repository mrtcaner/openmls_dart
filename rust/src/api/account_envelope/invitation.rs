use unicode_normalization::{IsNormalized, UnicodeNormalization, is_nfc_quick};
use zeroize::Zeroizing;

use super::unicode17_casefold::{FULL_DEFAULT_CASE_FOLD, UNICODE_VERSION};

use super::types::{
    AccountEnvelopeErrorCodeV1, AccountEnvelopeErrorV1, AccountEnvelopePaddingClassV1,
    AccountEnvelopeResult, ContextInvitationAuthorityV1, ContextInvitationPreviewV1,
    FORMAT_VERSION_V1, INVITATION_CANONICAL_ENVELOPE_MAX_BYTES, INVITATION_ENVELOPE_MAX_BYTES,
    INVITATION_HEADER_BYTES, INVITATION_INNER_MAX_BYTES, INVITATION_TAG_MAX_BYTES,
    INVITATION_TAG_MAX_CODE_POINTS, INVITATION_TAG_MAX_COUNT, INVITATION_TITLE_MAX_BYTES,
    INVITATION_TITLE_MAX_CODE_POINTS, KEY_BYTES, SIGNATURE_BYTES,
};

const CONTEXT_INVITATION_PURPOSE_V1: u8 = 1;

pub(super) struct ParsedInvitationEnvelopeV1<'a> {
    pub authority: ContextInvitationAuthorityV1,
    pub canonical_header: [u8; INVITATION_HEADER_BYTES],
    pub encapsulation: [u8; KEY_BYTES],
    pub ciphertext: &'a [u8],
    pub signature: [u8; SIGNATURE_BYTES],
}

pub(super) fn canonicalize_and_encode_preview(
    preview: &ContextInvitationPreviewV1,
) -> AccountEnvelopeResult<Vec<u8>> {
    if preview.tags.len() > INVITATION_TAG_MAX_COUNT {
        return Err(bounds_exceeded());
    }
    let title = preview
        .title
        .as_deref()
        .map(|value| {
            normalize_bounded_text(
                value,
                INVITATION_TITLE_MAX_CODE_POINTS,
                INVITATION_TITLE_MAX_BYTES,
            )
        })
        .transpose()?;
    let mut tags = Vec::with_capacity(preview.tags.len());
    for tag in &preview.tags {
        tags.push(normalize_bounded_text(
            tag,
            INVITATION_TAG_MAX_CODE_POINTS,
            INVITATION_TAG_MAX_BYTES,
        )?);
    }
    reject_folded_duplicates(&tags)?;

    let estimated = 2_usize
        .checked_add(title.as_ref().map_or(0, |value| 2 + value.len()))
        .and_then(|total| {
            tags.iter()
                .try_fold(total, |sum, value| sum.checked_add(2 + value.len()))
        })
        .ok_or_else(bounds_exceeded)?;
    if estimated > INVITATION_INNER_MAX_BYTES {
        return Err(bounds_exceeded());
    }
    let mut output = Vec::with_capacity(estimated);
    match title {
        Some(value) => {
            output.push(1);
            write_bounded_string(&mut output, &value)?;
        }
        None => output.push(0),
    }
    output.push(tags.len() as u8);
    for tag in tags {
        write_bounded_string(&mut output, &tag)?;
    }
    Ok(output)
}

pub(super) fn decode_canonical_preview(
    input: &[u8],
) -> AccountEnvelopeResult<ContextInvitationPreviewV1> {
    if input.len() > INVITATION_INNER_MAX_BYTES {
        return Err(plaintext_schema_invalid());
    }
    let mut cursor = PreviewCursor::new(input);
    let title_range = match cursor.read_u8()? {
        0 => None,
        1 => Some(
            cursor
                .read_string_range(INVITATION_TITLE_MAX_CODE_POINTS, INVITATION_TITLE_MAX_BYTES)?,
        ),
        _ => return Err(plaintext_schema_invalid()),
    };
    let tag_count = cursor.read_u8()? as usize;
    if tag_count > INVITATION_TAG_MAX_COUNT {
        return Err(plaintext_schema_invalid());
    }
    let mut tag_ranges = Vec::with_capacity(tag_count);
    for _ in 0..tag_count {
        tag_ranges.push(
            cursor.read_string_range(INVITATION_TAG_MAX_CODE_POINTS, INVITATION_TAG_MAX_BYTES)?,
        );
    }
    if !cursor.is_finished() {
        return Err(plaintext_schema_invalid());
    }

    let title_str = title_range
        .as_ref()
        .map(|range| canonical_str(&input[range.clone()]))
        .transpose()?;
    let mut tag_strs = Vec::with_capacity(tag_ranges.len());
    for range in &tag_ranges {
        tag_strs.push(canonical_str(&input[range.clone()])?);
    }
    reject_folded_duplicate_refs(&tag_strs)?;

    // All validation is complete before allocating caller-visible plaintext.
    Ok(ContextInvitationPreviewV1 {
        title: title_str.map(ToOwned::to_owned),
        tags: tag_strs.into_iter().map(ToOwned::to_owned).collect(),
    })
}

pub(super) fn encode_invitation_header(
    authority: &ContextInvitationAuthorityV1,
) -> AccountEnvelopeResult<[u8; INVITATION_HEADER_BYTES]> {
    authority.validate()?;
    let mut output = [0_u8; INVITATION_HEADER_BYTES];
    let mut cursor = 0;
    write_fixed(&mut output, &mut cursor, &[FORMAT_VERSION_V1]);
    write_fixed(&mut output, &mut cursor, &authority.envelope_id);
    write_fixed(&mut output, &mut cursor, &[CONTEXT_INVITATION_PURPOSE_V1]);
    write_fixed(&mut output, &mut cursor, &authority.sender_account_id);
    write_fixed(
        &mut output,
        &mut cursor,
        &authority.sender_generation.to_be_bytes(),
    );
    write_fixed(&mut output, &mut cursor, &authority.recipient_account_id);
    write_fixed(
        &mut output,
        &mut cursor,
        &authority.recipient_generation.to_be_bytes(),
    );
    write_fixed(&mut output, &mut cursor, &authority.invite_id);
    write_fixed(
        &mut output,
        &mut cursor,
        &authority.authority_attempt.to_be_bytes(),
    );
    write_fixed(
        &mut output,
        &mut cursor,
        &authority.relay_slot_version.to_be_bytes(),
    );
    write_fixed(
        &mut output,
        &mut cursor,
        &authority.server_created_at_unix_ms.to_be_bytes(),
    );
    write_fixed(
        &mut output,
        &mut cursor,
        &authority.server_expires_at_unix_ms.to_be_bytes(),
    );
    write_fixed(&mut output, &mut cursor, &[authority.padding_class as u8]);
    debug_assert_eq!(cursor, INVITATION_HEADER_BYTES);
    Ok(output)
}

pub(super) fn encode_invitation_envelope(
    canonical_header: &[u8; INVITATION_HEADER_BYTES],
    encapsulation: &[u8; KEY_BYTES],
    ciphertext: &[u8],
    signature: &[u8; SIGNATURE_BYTES],
) -> AccountEnvelopeResult<Vec<u8>> {
    let ciphertext_len = u16::try_from(ciphertext.len()).map_err(|_| bounds_exceeded())?;
    let total = INVITATION_HEADER_BYTES
        .checked_add(KEY_BYTES + 2 + ciphertext.len() + SIGNATURE_BYTES)
        .ok_or_else(bounds_exceeded)?;
    if total > INVITATION_CANONICAL_ENVELOPE_MAX_BYTES {
        return Err(bounds_exceeded());
    }
    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(canonical_header);
    output.extend_from_slice(encapsulation);
    output.extend_from_slice(&ciphertext_len.to_be_bytes());
    output.extend_from_slice(ciphertext);
    output.extend_from_slice(signature);
    Ok(output)
}

pub(super) fn parse_invitation_envelope(
    input: &[u8],
) -> AccountEnvelopeResult<ParsedInvitationEnvelopeV1<'_>> {
    if input.len() > INVITATION_ENVELOPE_MAX_BYTES {
        return Err(bounds_exceeded());
    }
    if input.len() > INVITATION_CANONICAL_ENVELOPE_MAX_BYTES
        || input.len() < INVITATION_HEADER_BYTES + KEY_BYTES + 2 + SIGNATURE_BYTES
    {
        return Err(noncanonical());
    }
    let canonical_header: [u8; INVITATION_HEADER_BYTES] = input
        .get(..INVITATION_HEADER_BYTES)
        .ok_or_else(noncanonical)?
        .try_into()
        .map_err(|_| noncanonical())?;
    let authority = parse_invitation_header(&canonical_header)?;
    let encapsulation_start = INVITATION_HEADER_BYTES;
    let encapsulation_end = encapsulation_start + KEY_BYTES;
    let encapsulation = input[encapsulation_start..encapsulation_end]
        .try_into()
        .map_err(|_| noncanonical())?;
    let length_end = encapsulation_end + 2;
    let ciphertext_len = u16::from_be_bytes(
        input[encapsulation_end..length_end]
            .try_into()
            .map_err(|_| noncanonical())?,
    ) as usize;
    if ciphertext_len != authority.padding_class.ciphertext_bytes() {
        return Err(noncanonical());
    }
    let ciphertext_end = length_end
        .checked_add(ciphertext_len)
        .ok_or_else(noncanonical)?;
    let signature_end = ciphertext_end
        .checked_add(SIGNATURE_BYTES)
        .ok_or_else(noncanonical)?;
    if signature_end != input.len() {
        return Err(noncanonical());
    }
    let ciphertext = input
        .get(length_end..ciphertext_end)
        .ok_or_else(noncanonical)?;
    let signature = input[ciphertext_end..signature_end]
        .try_into()
        .map_err(|_| noncanonical())?;
    Ok(ParsedInvitationEnvelopeV1 {
        authority,
        canonical_header,
        encapsulation,
        ciphertext,
        signature,
    })
}

fn parse_invitation_header(
    input: &[u8; INVITATION_HEADER_BYTES],
) -> AccountEnvelopeResult<ContextInvitationAuthorityV1> {
    let mut cursor = HeaderCursor::new(input);
    if cursor.read_u8()? != FORMAT_VERSION_V1 {
        return Err(AccountEnvelopeErrorV1::new(
            AccountEnvelopeErrorCodeV1::UnsupportedVersion,
        ));
    }
    let envelope_id = cursor.read_array()?;
    if cursor.read_u8()? != CONTEXT_INVITATION_PURPOSE_V1 {
        return Err(noncanonical());
    }
    let authority = ContextInvitationAuthorityV1 {
        envelope_id,
        sender_account_id: cursor.read_array()?,
        sender_generation: cursor.read_u64()?,
        recipient_account_id: cursor.read_array()?,
        recipient_generation: cursor.read_u64()?,
        invite_id: cursor.read_array()?,
        authority_attempt: cursor.read_u64()?,
        relay_slot_version: cursor.read_u64()?,
        server_created_at_unix_ms: cursor.read_u64()?,
        server_expires_at_unix_ms: cursor.read_u64()?,
        padding_class: AccountEnvelopePaddingClassV1::from_u8(cursor.read_u8()?)?,
    };
    if !cursor.is_finished() || authority.validate().is_err() {
        return Err(noncanonical());
    }
    Ok(authority)
}

fn normalize_bounded_text(
    input: &str,
    max_code_points: usize,
    max_bytes: usize,
) -> AccountEnvelopeResult<Zeroizing<String>> {
    let trimmed = input.trim_matches(is_unicode_17_white_space);
    if trimmed.is_empty() {
        return Err(plaintext_schema_invalid());
    }
    debug_assert_eq!(unicode_normalization::UNICODE_VERSION, UNICODE_VERSION);
    let normalized: Zeroizing<String> = Zeroizing::new(trimmed.nfc().collect());
    if normalized.len() > max_bytes || normalized.chars().count() > max_code_points {
        return Err(bounds_exceeded());
    }
    Ok(normalized)
}

fn canonical_str(input: &[u8]) -> AccountEnvelopeResult<&str> {
    let value = std::str::from_utf8(input).map_err(|_| plaintext_schema_invalid())?;
    if value.is_empty()
        || value.starts_with(is_unicode_17_white_space)
        || value.ends_with(is_unicode_17_white_space)
        || is_nfc_quick(value.chars()) != IsNormalized::Yes
    {
        return Err(plaintext_schema_invalid());
    }
    Ok(value)
}

fn reject_folded_duplicates(values: &[Zeroizing<String>]) -> AccountEnvelopeResult<()> {
    let refs: Vec<&str> = values.iter().map(|value| value.as_str()).collect();
    reject_folded_duplicate_refs(&refs)
}

fn reject_folded_duplicate_refs(values: &[&str]) -> AccountEnvelopeResult<()> {
    let mut folded = Vec::<Zeroizing<String>>::with_capacity(values.len());
    for value in values {
        let candidate = Zeroizing::new(default_case_fold(value));
        if folded
            .iter()
            .any(|existing| existing.as_str() == candidate.as_str())
        {
            return Err(plaintext_schema_invalid());
        }
        folded.push(candidate);
    }
    Ok(())
}

pub(super) fn default_case_fold(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match FULL_DEFAULT_CASE_FOLD.binary_search_by_key(&character, |(source, _)| *source) {
            Ok(index) => output.extend(FULL_DEFAULT_CASE_FOLD[index].1.iter().copied()),
            Err(_) => output.push(character),
        }
    }
    output
}

fn write_bounded_string(output: &mut Vec<u8>, value: &str) -> AccountEnvelopeResult<()> {
    let length = u16::try_from(value.len()).map_err(|_| bounds_exceeded())?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_fixed<const N: usize>(output: &mut [u8; N], cursor: &mut usize, value: &[u8]) {
    let end = *cursor + value.len();
    output[*cursor..end].copy_from_slice(value);
    *cursor = end;
}

fn is_unicode_17_white_space(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

struct PreviewCursor<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> PreviewCursor<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    fn read_u8(&mut self) -> AccountEnvelopeResult<u8> {
        let value = *self
            .input
            .get(self.position)
            .ok_or_else(plaintext_schema_invalid)?;
        self.position += 1;
        Ok(value)
    }

    fn read_string_range(
        &mut self,
        max_code_points: usize,
        max_bytes: usize,
    ) -> AccountEnvelopeResult<std::ops::Range<usize>> {
        let length_end = self
            .position
            .checked_add(2)
            .ok_or_else(plaintext_schema_invalid)?;
        let length_bytes: [u8; 2] = self
            .input
            .get(self.position..length_end)
            .ok_or_else(plaintext_schema_invalid)?
            .try_into()
            .map_err(|_| plaintext_schema_invalid())?;
        self.position = length_end;
        let length = u16::from_be_bytes(length_bytes) as usize;
        if length == 0 || length > max_bytes {
            return Err(plaintext_schema_invalid());
        }
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(plaintext_schema_invalid)?;
        let bytes = self
            .input
            .get(self.position..end)
            .ok_or_else(plaintext_schema_invalid)?;
        let value = std::str::from_utf8(bytes).map_err(|_| plaintext_schema_invalid())?;
        if value.chars().count() > max_code_points {
            return Err(plaintext_schema_invalid());
        }
        let range = self.position..end;
        self.position = end;
        Ok(range)
    }

    fn is_finished(&self) -> bool {
        self.position == self.input.len()
    }
}

struct HeaderCursor<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> HeaderCursor<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    fn read_u8(&mut self) -> AccountEnvelopeResult<u8> {
        Ok(self.read_array::<1>()?[0])
    }

    fn read_u64(&mut self) -> AccountEnvelopeResult<u64> {
        Ok(u64::from_be_bytes(self.read_array()?))
    }

    fn read_array<const N: usize>(&mut self) -> AccountEnvelopeResult<[u8; N]> {
        let end = self.position.checked_add(N).ok_or_else(noncanonical)?;
        let value = self
            .input
            .get(self.position..end)
            .ok_or_else(noncanonical)?;
        let output = value.try_into().map_err(|_| noncanonical())?;
        self.position = end;
        Ok(output)
    }

    fn is_finished(&self) -> bool {
        self.position == self.input.len()
    }
}

fn bounds_exceeded() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::BoundsExceeded)
}

fn noncanonical() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::NonCanonicalEncoding)
}

fn plaintext_schema_invalid() -> AccountEnvelopeErrorV1 {
    AccountEnvelopeErrorV1::new(AccountEnvelopeErrorCodeV1::PlaintextSchemaInvalid)
}
