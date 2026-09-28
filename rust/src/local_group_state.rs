//! Package-owned local history/pending metadata, in caller-owned format-1 rows.
//! No database, global mutable state, secret-bearing rollback snapshot or resize.

use crate::api::group_e2ee::{
    MlsErrorCode as Error, MlsExpectedRosterStateV1, MlsPendingCommitBinding, MlsRosterLeafV1,
    MlsRosterSummaryV1, MlsTransitionContext, mls_roster_digest_v1,
};
use crate::snapshot_storage::SnapshotStorageProvider;
use minicbor::{Decoder, Encoder};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const SCHEMA: u16 = 1;
const PROFILE: u16 = 2;
const MAX_VALUE: usize = 2 * 1024 * 1024;
const MAX_WIRE: usize = 1024 * 1024;
const MAX_LEAVES: usize = 256;
const PREFIX: &[u8] = b"OpenMlsDart/1/";
pub(crate) type RetryBytes = (
    Zeroizing<Vec<u8>>,
    Option<Zeroizing<Vec<u8>>>,
    Option<Zeroizing<Vec<u8>>>,
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingRecord {
    pub binding: MlsPendingCommitBinding,
    pub proposed_roster: MlsRosterSummaryV1,
    pub preparation_base_sha256: Vec<u8>,
    pub upstream_pending_sha256: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalGroupState {
    pub group_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub retention: u32,
    pub join_epoch: u64,
    pub current: MlsRosterSummaryV1,
    pub history: Vec<MlsRosterSummaryV1>,
    pub pending: Option<PendingRecord>,
}

fn key(group: &[u8], name: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(PREFIX.len() + name.len() + 2 + group.len());
    key.extend_from_slice(PREFIX);
    key.extend_from_slice(name);
    key.extend_from_slice(&(group.len() as u16).to_be_bytes());
    key.extend_from_slice(group);
    key
}
fn fixed(value: &[u8], len: usize) -> Result<(), Error> {
    if value.len() == len {
        Ok(())
    } else {
        Err(Error::InvalidStorageSnapshot)
    }
}
fn opaque_id(value: &[u8]) -> Result<(), Error> {
    if (1..=128).contains(&value.len()) {
        Ok(())
    } else {
        Err(Error::LimitExceeded)
    }
}
pub(crate) fn validate_retention(retention: u32) -> Result<(), Error> {
    if matches!(retention, 2 | 4) {
        Ok(())
    } else {
        Err(Error::UnsupportedRetention)
    }
}
fn validate_leaf(leaf: &MlsRosterLeafV1) -> Result<(), Error> {
    fixed(&leaf.credential_identity, 45)?;
    fixed(&leaf.signature_public_key, 32)
}
fn validate_roster(roster: &MlsRosterSummaryV1) -> Result<(), Error> {
    fixed(&roster.group_id, 16)?;
    fixed(&roster.digest_sha256, 32)?;
    if roster.leaves.is_empty() || roster.leaves.len() > MAX_LEAVES {
        return Err(Error::LimitExceeded);
    }
    for leaf in &roster.leaves {
        validate_leaf(leaf)?;
    }
    let hash = mls_roster_digest_v1(roster.group_id.clone(), roster.epoch, roster.leaves.clone())
        .map_err(|_| Error::InvalidStorageSnapshot)?;
    if hash != roster.digest_sha256 {
        return Err(Error::InvalidStorageSnapshot);
    }
    Ok(())
}
fn expected(roster: &MlsRosterSummaryV1) -> MlsExpectedRosterStateV1 {
    MlsExpectedRosterStateV1 {
        group_id: roster.group_id.clone(),
        epoch: roster.epoch,
        digest_sha256: roster.digest_sha256.clone(),
    }
}

impl LocalGroupState {
    pub(crate) fn new(
        incarnation_id: Vec<u8>,
        retention: u32,
        current: MlsRosterSummaryV1,
    ) -> Result<Self, Error> {
        let value = Self {
            group_id: current.group_id.clone(),
            incarnation_id,
            retention,
            join_epoch: current.epoch,
            current,
            history: vec![],
            pending: None,
        };
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_retention(self.retention)?;
        fixed(&self.group_id, 16)?;
        opaque_id(&self.incarnation_id)?;
        validate_roster(&self.current)?;
        if self.current.group_id != self.group_id || self.join_epoch > self.current.epoch {
            return Err(Error::InvalidStorageSnapshot);
        }
        let count = (self.current.epoch - self.join_epoch).min(self.retention as u64) as usize;
        if self.history.len() != count {
            return Err(Error::InvalidStorageSnapshot);
        }
        for (index, roster) in self.history.iter().enumerate() {
            validate_roster(roster)?;
            if roster.group_id != self.group_id
                || roster.epoch != self.current.epoch - count as u64 + index as u64
            {
                return Err(Error::InvalidStorageSnapshot);
            }
        }
        if let Some(p) = &self.pending {
            let b = &p.binding;
            validate_roster(&p.proposed_roster)?;
            opaque_id(&b.transition.command_id)?;
            fixed(&b.transition.context_sha256, 32)?;
            for hash in [
                &b.commit_sha256,
                &b.aad_sha256,
                &p.preparation_base_sha256,
                &p.upstream_pending_sha256,
            ] {
                fixed(hash, 32)?;
            }
            for hash in [&b.welcome_sha256, &b.group_info_sha256]
                .into_iter()
                .flatten()
            {
                fixed(hash, 32)?;
            }
            if b.group_id != self.group_id
                || b.incarnation_id != self.incarnation_id
                || b.previous_state != expected(&self.current)
                || !self.current.leaves.contains(&b.author)
                || p.proposed_roster.group_id != self.group_id
                || self.current.epoch.checked_add(1) != Some(p.proposed_roster.epoch)
            {
                return Err(Error::PendingBindingMismatch);
            }
        }
        Ok(())
    }
    pub(crate) fn advance(&mut self, resulting: MlsRosterSummaryV1) -> Result<(), Error> {
        self.validate()?;
        validate_roster(&resulting)?;
        if resulting.group_id != self.group_id {
            return Err(Error::GroupMismatch);
        }
        if self.current.epoch.checked_add(1) != Some(resulting.epoch) {
            return Err(Error::RequiredCommitEpochMismatch);
        }
        self.history
            .push(std::mem::replace(&mut self.current, resulting));
        if self.history.len() > self.retention as usize {
            self.history.remove(0);
        }
        self.pending = None;
        self.validate()
    }
    pub(crate) fn message_roster(&self, epoch: u64) -> Result<&MlsRosterSummaryV1, Error> {
        self.validate()?;
        if epoch == self.current.epoch {
            return Ok(&self.current);
        }
        if epoch > self.current.epoch {
            return Err(Error::FutureMessageEpoch);
        }
        self.history
            .iter()
            .find(|r| r.epoch == epoch)
            .ok_or(Error::PastEpochUnavailable)
    }
    pub(crate) fn load(storage: &SnapshotStorageProvider, group: &[u8]) -> Result<Self, Error> {
        fixed(group, 16)?;
        let known =
            [b"state/".as_slice(), b"commit/", b"welcome/", b"info/"].map(|name| key(group, name));
        for row_key in storage.opaque_keys_with_prefix(b"OpenMlsDart/") {
            if !row_key.starts_with(PREFIX) {
                return Err(Error::UnsupportedLocalMetadataVersion);
            }
            if !known.contains(&row_key) {
                return Err(Error::InvalidStorageSnapshot);
            }
        }
        let bytes = storage
            .read_opaque(&key(group, b"state/"))
            .ok_or(Error::LocalMetadataMissing)?;
        let state = Self::decode(&bytes)?;
        if state.group_id != group {
            return Err(Error::GroupMismatch);
        }
        state.validate_retry_rows(storage)?;
        Ok(state)
    }
    pub(crate) fn save(&self, storage: &SnapshotStorageProvider) -> Result<(), Error> {
        let bytes = self.encode()?;
        storage.write_opaque(key(&self.group_id, b"state/"), bytes);
        Ok(())
    }
    pub(crate) fn write_retry_rows(
        &self,
        storage: &SnapshotStorageProvider,
        commit: Vec<u8>,
        welcome: Option<Vec<u8>>,
        info: Option<Vec<u8>>,
    ) -> Result<(), Error> {
        self.validate()?;
        let pending = self.pending.as_ref().ok_or(Error::PendingCommitMissing)?;
        let wires = [
            (
                b"commit/".as_slice(),
                Some(commit),
                Some(&pending.binding.commit_sha256),
            ),
            (
                b"welcome/".as_slice(),
                welcome,
                pending.binding.welcome_sha256.as_ref(),
            ),
            (
                b"info/".as_slice(),
                info,
                pending.binding.group_info_sha256.as_ref(),
            ),
        ];
        // Validate all before any mutation, even though the entire provider is
        // still temporary and only a final complete batch may cross the boundary.
        for (_, wire, hash) in &wires {
            validate_wire(wire.as_deref(), hash.map(Vec::as_slice))?;
        }
        for (name, wire, _) in wires {
            let key = key(&self.group_id, name);
            if let Some(wire) = wire {
                storage.write_opaque(key, wire)
            } else {
                storage.delete_opaque(&key)
            }
        }
        Ok(())
    }
    pub(crate) fn clear_retry_rows(&self, storage: &SnapshotStorageProvider) {
        for name in [
            b"commit/".as_slice(),
            b"welcome/".as_slice(),
            b"info/".as_slice(),
        ] {
            storage.delete_opaque(&key(&self.group_id, name));
        }
    }
    fn validate_retry_rows(&self, storage: &SnapshotStorageProvider) -> Result<(), Error> {
        let hashes = self
            .pending
            .as_ref()
            .map(|p| {
                [
                    Some(&p.binding.commit_sha256),
                    p.binding.welcome_sha256.as_ref(),
                    p.binding.group_info_sha256.as_ref(),
                ]
            })
            .unwrap_or([None, None, None]);
        for (name, hash) in [
            b"commit/".as_slice(),
            b"welcome/".as_slice(),
            b"info/".as_slice(),
        ]
        .into_iter()
        .zip(hashes)
        {
            let bytes = storage.read_opaque(&key(&self.group_id, name));
            validate_wire(bytes.as_deref().map(Vec::as_slice), hash.map(Vec::as_slice))?;
        }
        Ok(())
    }
    pub(crate) fn retry_bytes(
        &self,
        storage: &SnapshotStorageProvider,
    ) -> Result<RetryBytes, Error> {
        self.validate_retry_rows(storage)?;
        if self.pending.is_none() {
            return Err(Error::PendingCommitMissing);
        }
        Ok((
            storage
                .read_opaque(&key(&self.group_id, b"commit/"))
                .ok_or(Error::PendingCommitMissing)?,
            storage.read_opaque(&key(&self.group_id, b"welcome/")),
            storage.read_opaque(&key(&self.group_id, b"info/")),
        ))
    }
}
fn validate_wire(wire: Option<&[u8]>, hash: Option<&[u8]>) -> Result<(), Error> {
    match (wire, hash) {
        (None, None) => Ok(()),
        (Some(w), Some(h)) => {
            if w.is_empty() || w.len() > MAX_WIRE {
                return Err(Error::LimitExceeded);
            }
            if Sha256::digest(w).as_slice() != h {
                return Err(Error::PendingBindingMismatch);
            }
            Ok(())
        }
        _ => Err(Error::PendingBindingMismatch),
    }
}

// The metadata codec is deliberately bounded and independent of serde/OpenMLS
// private layouts. Fixed arrays carry the existing public v1 roster fields.
type Enc<'a> = Encoder<&'a mut Vec<u8>>;
fn encode_error(_: minicbor::encode::Error<std::convert::Infallible>) -> Error {
    Error::InternalFailure
}
fn bytes(e: &mut Enc<'_>, v: &[u8]) -> Result<(), Error> {
    e.bytes(v).map_err(encode_error)?;
    Ok(())
}
fn leaf_encode(e: &mut Enc<'_>, l: &MlsRosterLeafV1) -> Result<(), Error> {
    e.array(3)
        .map_err(encode_error)?
        .u32(l.leaf_index)
        .map_err(encode_error)?;
    bytes(e, &l.credential_identity)?;
    bytes(e, &l.signature_public_key)
}
fn expected_encode(e: &mut Enc<'_>, r: &MlsExpectedRosterStateV1) -> Result<(), Error> {
    e.array(3).map_err(encode_error)?;
    bytes(e, &r.group_id)?;
    e.u64(r.epoch).map_err(encode_error)?;
    bytes(e, &r.digest_sha256)
}
fn roster_encode(e: &mut Enc<'_>, r: &MlsRosterSummaryV1) -> Result<(), Error> {
    e.array(4).map_err(encode_error)?;
    bytes(e, &r.group_id)?;
    e.u64(r.epoch)
        .map_err(encode_error)?
        .array(r.leaves.len() as u64)
        .map_err(encode_error)?;
    for l in &r.leaves {
        leaf_encode(e, l)?;
    }
    bytes(e, &r.digest_sha256)
}
fn optional_hash_encode(e: &mut Enc<'_>, v: Option<&Vec<u8>>) -> Result<(), Error> {
    if let Some(v) = v {
        bytes(e, v)
    } else {
        e.null().map_err(encode_error)?;
        Ok(())
    }
}
impl LocalGroupState {
    fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut out = Vec::new();
        let mut e = Encoder::new(&mut out);
        e.array(9)
            .map_err(encode_error)?
            .u16(SCHEMA)
            .map_err(encode_error)?
            .u16(PROFILE)
            .map_err(encode_error)?;
        bytes(&mut e, &self.group_id)?;
        bytes(&mut e, &self.incarnation_id)?;
        e.u32(self.retention)
            .map_err(encode_error)?
            .u64(self.join_epoch)
            .map_err(encode_error)?;
        roster_encode(&mut e, &self.current)?;
        e.array(self.history.len() as u64).map_err(encode_error)?;
        for r in &self.history {
            roster_encode(&mut e, r)?;
        }
        if let Some(p) = &self.pending {
            let b = &p.binding;
            e.array(13).map_err(encode_error)?;
            bytes(&mut e, &b.group_id)?;
            bytes(&mut e, &b.incarnation_id)?;
            bytes(&mut e, &b.transition.command_id)?;
            bytes(&mut e, &b.transition.context_sha256)?;
            leaf_encode(&mut e, &b.author)?;
            expected_encode(&mut e, &b.previous_state)?;
            bytes(&mut e, &b.commit_sha256)?;
            bytes(&mut e, &b.aad_sha256)?;
            optional_hash_encode(&mut e, b.welcome_sha256.as_ref())?;
            optional_hash_encode(&mut e, b.group_info_sha256.as_ref())?;
            roster_encode(&mut e, &p.proposed_roster)?;
            bytes(&mut e, &p.preparation_base_sha256)?;
            bytes(&mut e, &p.upstream_pending_sha256)?;
        } else {
            e.null().map_err(encode_error)?;
        }
        if out.len() > MAX_VALUE {
            return Err(Error::LimitExceeded);
        }
        Ok(out)
    }
    fn decode(input: &[u8]) -> Result<Self, Error> {
        if input.len() > MAX_VALUE {
            return Err(Error::LimitExceeded);
        }
        // Shape checks bound every owned allocation; re-encoding below rejects
        // nonminimal integers, trailing bytes and every noncanonical alternative.
        let mut d = Decoder::new(input);
        array(&mut d, 9)?;
        if d.u16().map_err(decode_error)? != SCHEMA {
            return Err(Error::UnsupportedLocalMetadataVersion);
        }
        if d.u16().map_err(decode_error)? != PROFILE {
            return Err(Error::UnsupportedProfile);
        }
        let group_id = read_bytes(&mut d, 16, 16)?;
        let incarnation_id = read_bytes(&mut d, 1, 128)?;
        let retention = d.u32().map_err(decode_error)?;
        validate_retention(retention)?;
        let join_epoch = d.u64().map_err(decode_error)?;
        let current = roster_decode(&mut d)?;
        let count = count(&mut d, retention as usize)?;
        let mut history = Vec::with_capacity(count);
        for _ in 0..count {
            history.push(roster_decode(&mut d)?);
        }
        let pending = if d.datatype().map_err(decode_error)? == minicbor::data::Type::Null {
            d.null().map_err(decode_error)?;
            None
        } else {
            array(&mut d, 13)?;
            let binding = MlsPendingCommitBinding {
                group_id: read_bytes(&mut d, 16, 16)?,
                incarnation_id: read_bytes(&mut d, 1, 128)?,
                transition: MlsTransitionContext {
                    command_id: read_bytes(&mut d, 1, 128)?,
                    context_sha256: read_bytes(&mut d, 32, 32)?,
                },
                author: leaf_decode(&mut d)?,
                previous_state: expected_decode(&mut d)?,
                commit_sha256: read_bytes(&mut d, 32, 32)?,
                aad_sha256: read_bytes(&mut d, 32, 32)?,
                welcome_sha256: optional_hash_decode(&mut d)?,
                group_info_sha256: optional_hash_decode(&mut d)?,
            };
            Some(PendingRecord {
                binding,
                proposed_roster: roster_decode(&mut d)?,
                preparation_base_sha256: read_bytes(&mut d, 32, 32)?,
                upstream_pending_sha256: read_bytes(&mut d, 32, 32)?,
            })
        };
        if d.position() != input.len() {
            return Err(Error::NoncanonicalEncoding);
        }
        let state = Self {
            group_id,
            incarnation_id,
            retention,
            join_epoch,
            current,
            history,
            pending,
        };
        state.validate()?;
        if state.encode()?.as_slice() != input {
            return Err(Error::NoncanonicalEncoding);
        }
        Ok(state)
    }
}
fn decode_error(_: minicbor::decode::Error) -> Error {
    Error::InvalidStorageSnapshot
}
fn count(d: &mut Decoder<'_>, max: usize) -> Result<usize, Error> {
    let n = d
        .array()
        .map_err(decode_error)?
        .ok_or(Error::NoncanonicalEncoding)?;
    if n > max as u64 {
        return Err(Error::LimitExceeded);
    }
    Ok(n as usize)
}
fn array(d: &mut Decoder<'_>, n: usize) -> Result<(), Error> {
    if count(d, n)? == n {
        Ok(())
    } else {
        Err(Error::InvalidStorageSnapshot)
    }
}
fn read_bytes(d: &mut Decoder<'_>, min: usize, max: usize) -> Result<Vec<u8>, Error> {
    let v = d.bytes().map_err(decode_error)?;
    if v.len() < min || v.len() > max {
        return Err(Error::LimitExceeded);
    }
    Ok(v.to_vec())
}
fn leaf_decode(d: &mut Decoder<'_>) -> Result<MlsRosterLeafV1, Error> {
    array(d, 3)?;
    Ok(MlsRosterLeafV1 {
        leaf_index: d.u32().map_err(decode_error)?,
        credential_identity: read_bytes(d, 45, 45)?,
        signature_public_key: read_bytes(d, 32, 32)?,
    })
}
fn expected_decode(d: &mut Decoder<'_>) -> Result<MlsExpectedRosterStateV1, Error> {
    array(d, 3)?;
    Ok(MlsExpectedRosterStateV1 {
        group_id: read_bytes(d, 16, 16)?,
        epoch: d.u64().map_err(decode_error)?,
        digest_sha256: read_bytes(d, 32, 32)?,
    })
}
fn roster_decode(d: &mut Decoder<'_>) -> Result<MlsRosterSummaryV1, Error> {
    array(d, 4)?;
    let group_id = read_bytes(d, 16, 16)?;
    let epoch = d.u64().map_err(decode_error)?;
    let n = count(d, MAX_LEAVES)?;
    if n == 0 {
        return Err(Error::LimitExceeded);
    }
    let mut leaves = Vec::with_capacity(n);
    for _ in 0..n {
        leaves.push(leaf_decode(d)?);
    }
    Ok(MlsRosterSummaryV1 {
        group_id,
        epoch,
        leaves,
        digest_sha256: read_bytes(d, 32, 32)?,
    })
}
fn optional_hash_decode(d: &mut Decoder<'_>) -> Result<Option<Vec<u8>>, Error> {
    if d.datatype().map_err(decode_error)? == minicbor::data::Type::Null {
        d.null().map_err(decode_error)?;
        Ok(None)
    } else {
        Ok(Some(read_bytes(d, 32, 32)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::group_e2ee::group_state_digest_from_entries;
    use crate::api::storage::{MlsStorageEntry, batch_from_provider};
    use crate::snapshot_storage::SnapshotOpenMlsProvider;
    fn roster(epoch: u64) -> MlsRosterSummaryV1 {
        let group_id = vec![7; 16];
        let leaves = vec![MlsRosterLeafV1 {
            leaf_index: 0,
            credential_identity: vec![1; 45],
            signature_public_key: vec![2; 32],
        }];
        let digest_sha256 = mls_roster_digest_v1(group_id.clone(), epoch, leaves.clone()).unwrap();
        MlsRosterSummaryV1 {
            group_id,
            epoch,
            leaves,
            digest_sha256,
        }
    }
    #[test]
    fn roundtrip_and_retention_are_exact_after_restart() {
        for retention in [2, 4] {
            let mut s = LocalGroupState::new(vec![3; 16], retention, roster(5)).unwrap();
            for epoch in 6..12 {
                s.advance(roster(epoch)).unwrap();
                s = LocalGroupState::decode(&s.encode().unwrap()).unwrap();
            }
            assert_eq!(s.history.len(), retention as usize);
            assert_eq!(
                s.message_roster(11 - retention as u64).unwrap().epoch,
                11 - retention as u64
            );
            assert_eq!(
                s.message_roster(10 - retention as u64).unwrap_err(),
                Error::PastEpochUnavailable
            );
            assert_eq!(s.message_roster(12).unwrap_err(), Error::FutureMessageEpoch);
            let mut gap = s.clone();
            gap.history.remove(0);
            assert_eq!(gap.encode().unwrap_err(), Error::InvalidStorageSnapshot);
        }
    }
    #[test]
    fn metadata_codec_fails_closed_on_unbounded_or_noncanonical_input() {
        let s = LocalGroupState::new(vec![3; 16], 2, roster(1)).unwrap();
        let good = s.encode().unwrap();
        let mut bad = good.clone();
        bad.push(0);
        assert_eq!(
            LocalGroupState::decode(&bad).unwrap_err(),
            Error::NoncanonicalEncoding
        );
        let mut nonminimal = vec![0x98, 9];
        nonminimal.extend_from_slice(&good[1..]);
        assert_eq!(
            LocalGroupState::decode(&nonminimal).unwrap_err(),
            Error::NoncanonicalEncoding
        );
        let mut version = good.clone();
        version[1] = 2;
        assert_eq!(
            LocalGroupState::decode(&version).unwrap_err(),
            Error::UnsupportedLocalMetadataVersion
        );
        assert_eq!(
            LocalGroupState::new(vec![3], 0, roster(1)).unwrap_err(),
            Error::UnsupportedRetention
        );
        assert_eq!(
            LocalGroupState::decode(&vec![0; MAX_VALUE + 1]).unwrap_err(),
            Error::LimitExceeded
        );
        for end in 0..good.len() {
            assert!(LocalGroupState::decode(&good[..end]).is_err());
        }
    }
    #[test]
    fn metadata_is_group_scoped_digest_covered_and_in_complete_batch() {
        use openmls_traits::OpenMlsProvider;
        let s = LocalGroupState::new(vec![3; 16], 2, roster(1)).unwrap();
        let provider = SnapshotOpenMlsProvider::new(SnapshotStorageProvider::from_entries(vec![]));
        s.save(provider.storage()).unwrap();
        assert_eq!(
            LocalGroupState::load(provider.storage(), &s.group_id).unwrap(),
            s
        );
        let batch = batch_from_provider(provider, Some(s.group_id.clone()), vec![]).unwrap();
        assert_eq!(batch.storage_format_version, 1);
        assert_eq!(batch.upserts.len(), 1);
        assert_eq!(batch.upserts[0].group_id, Some(s.group_id.clone()));
        let first = group_state_digest_from_entries(&s.group_id, &batch.upserts, 1).unwrap();
        let mut s4 = s.clone();
        s4.retention = 4;
        let different = vec![MlsStorageEntry {
            key: batch.upserts[0].key.clone(),
            value: s4.encode().unwrap(),
            group_id: Some(s.group_id.clone()),
        }];
        assert_ne!(
            first,
            group_state_digest_from_entries(&s.group_id, &different, 1).unwrap()
        );
        let provider = SnapshotStorageProvider::from_entries(
            batch
                .upserts
                .into_iter()
                .map(|e| (e.key, e.value))
                .collect(),
        );
        assert_eq!(
            LocalGroupState::load(&provider, &s.group_id)
                .unwrap()
                .retention,
            2
        );
    }
    #[test]
    fn reserved_metadata_namespace_rejects_unknown_and_cross_group_rows() {
        let state = LocalGroupState::new(vec![3; 16], 2, roster(1)).unwrap();
        for (extra, error) in [
            (
                b"OpenMlsDart/2/state/future".to_vec(),
                Error::UnsupportedLocalMetadataVersion,
            ),
            (
                key(&state.group_id, b"unknown/"),
                Error::InvalidStorageSnapshot,
            ),
            (key(&[9; 16], b"state/"), Error::InvalidStorageSnapshot),
        ] {
            let storage = SnapshotStorageProvider::from_entries(vec![]);
            state.save(&storage).unwrap();
            storage.write_opaque(extra, vec![0]);
            assert_eq!(LocalGroupState::load(&storage, &state.group_id), Err(error));
        }
    }
    #[test]
    fn retry_rows_must_match_all_saved_hashes_and_presence() {
        let mut s = LocalGroupState::new(vec![3; 16], 2, roster(1)).unwrap();
        let commit = vec![8; 80];
        let welcome = vec![9; 100];
        s.pending = Some(PendingRecord {
            binding: MlsPendingCommitBinding {
                group_id: s.group_id.clone(),
                incarnation_id: s.incarnation_id.clone(),
                transition: MlsTransitionContext {
                    command_id: vec![4; 16],
                    context_sha256: vec![5; 32],
                },
                author: s.current.leaves[0].clone(),
                previous_state: expected(&s.current),
                commit_sha256: Sha256::digest(&commit).to_vec(),
                aad_sha256: vec![6; 32],
                welcome_sha256: Some(Sha256::digest(&welcome).to_vec()),
                group_info_sha256: None,
            },
            proposed_roster: roster(2),
            preparation_base_sha256: vec![10; 32],
            upstream_pending_sha256: vec![11; 32],
        });
        let storage = SnapshotStorageProvider::from_entries(vec![]);
        assert_eq!(
            s.write_retry_rows(&storage, commit.clone(), None, None)
                .unwrap_err(),
            Error::PendingBindingMismatch
        );
        s.write_retry_rows(&storage, commit.clone(), Some(welcome.clone()), None)
            .unwrap();
        s.save(&storage).unwrap();
        let loaded = LocalGroupState::load(&storage, &s.group_id).unwrap();
        let (c, w, i) = loaded.retry_bytes(&storage).unwrap();
        assert_eq!(*c, commit);
        assert_eq!(*w.unwrap(), welcome);
        assert!(i.is_none());
        storage.write_opaque(key(&s.group_id, b"commit/"), vec![4; 80]);
        assert_eq!(
            LocalGroupState::load(&storage, &s.group_id).unwrap_err(),
            Error::PendingBindingMismatch
        );
        s.clear_retry_rows(&storage);
        s.pending = None;
        s.save(&storage).unwrap();
        assert_eq!(LocalGroupState::load(&storage, &s.group_id).unwrap(), s);
    }
}
