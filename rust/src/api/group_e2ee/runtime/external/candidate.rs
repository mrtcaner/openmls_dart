//! Local opaque envelope1. Not encryption: caller uses authenticated encrypted
//! custody and retains the independent package-issued hash with lifecycle CAS.
use super::*;
use minicbor::{Decoder, Encoder};
pub(super) const MAX_BYTES: usize = 12 * 1024 * 1024;
pub(super) struct Candidate {
    pub candidate_id: Vec<u8>,
    pub incarnation_id: Vec<u8>,
    pub transition: MlsTransitionContext,
    pub lifecycle: Vec<u8>,
    pub retention: u32,
    pub proof: public_core::SuccessorRequest,
    pub previous: MlsRosterSummaryV1,
    pub resulting: MlsRosterSummaryV1,
    pub private_digest: Vec<u8>,
    pub rows: Entries,
}
impl Candidate {
    pub(super) fn binding(&self) -> Result<MlsExternalCommitBindingV1, Error> {
        let author = self
            .resulting
            .leaves
            .iter()
            .find(|l| {
                l.credential_identity == self.proof.authorized_new_identity
                    && l.signature_public_key == self.proof.authorized_new_signature_key
            })
            .cloned()
            .ok_or(Error::ExternalCandidateInvalid)?;
        Ok(MlsExternalCommitBindingV1 {
            candidate_id: self.candidate_id.clone(),
            incarnation_id: self.incarnation_id.clone(),
            transition: self.transition.clone(),
            previous_state: state(&self.previous),
            old_leaf: local_leaf(&self.proof.authorized_old),
            author,
            input_group_info_sha256: self.proof.base.sha256.clone(),
            commit_sha256: self.proof.commit_sha256.clone(),
            aad_sha256: hash(&self.proof.expected_aad),
            resulting_group_info_sha256: self.proof.reported_next_sha256.clone(),
        })
    }
    pub(super) fn info(
        &self,
        candidate_hash: Vec<u8>,
    ) -> Result<ExternalCommitCandidateInfo, Error> {
        Ok(ExternalCommitCandidateInfo {
            binding: self.binding()?,
            previous_roster: self.previous.clone(),
            proposed_resulting_roster: self.resulting.clone(),
            commit: self.proof.commit.clone(),
            group_info: self.proof.reported_next_group_info.clone(),
            candidate_sha256: candidate_hash,
            local_lifecycle_context_sha256: self.lifecycle.clone(),
            effective_retention: self.retention,
        })
    }
}
type E<'a> = Encoder<&'a mut Vec<u8>>;
fn put_roster(e: &mut E<'_>, r: &MlsRosterSummaryV1) {
    e.array(4)
        .unwrap()
        .bytes(&r.group_id)
        .unwrap()
        .u64(r.epoch)
        .unwrap()
        .bytes(&r.digest_sha256)
        .unwrap()
        .array(r.leaves.len() as u64)
        .unwrap();
    for l in &r.leaves {
        e.array(3)
            .unwrap()
            .u32(l.leaf_index)
            .unwrap()
            .bytes(&l.credential_identity)
            .unwrap()
            .bytes(&l.signature_public_key)
            .unwrap();
    }
}
pub(super) fn encode(c: &Candidate) -> Result<Zeroizing<Vec<u8>>, Error> {
    bounded_entries(&c.rows.0)?;
    let proof =
        public_codec::encode_request(&public_codec::Request::Successor(Box::new(c.proof.clone())))
            .map_err(public_error)?;
    let mut out = Zeroizing::new(Vec::new());
    let mut e = Encoder::new(&mut *out);
    e.array(12)
        .unwrap()
        .u16(1)
        .unwrap()
        .bytes(&c.candidate_id)
        .unwrap()
        .bytes(&c.incarnation_id)
        .unwrap()
        .bytes(&c.transition.command_id)
        .unwrap()
        .bytes(&c.transition.context_sha256)
        .unwrap()
        .bytes(&c.lifecycle)
        .unwrap()
        .u32(c.retention)
        .unwrap()
        .bytes(&proof)
        .unwrap();
    put_roster(&mut e, &c.previous);
    put_roster(&mut e, &c.resulting);
    e.bytes(&c.private_digest)
        .unwrap()
        .array(c.rows.0.len() as u64)
        .unwrap();
    for r in &c.rows.0 {
        e.array(2)
            .unwrap()
            .bytes(&r.key)
            .unwrap()
            .bytes(&r.value)
            .unwrap();
    }
    if out.len() > MAX_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}
fn bad(_: minicbor::decode::Error) -> Error {
    Error::ExternalCandidateInvalid
}
fn array(d: &mut Decoder<'_>, n: u64) -> Result<(), Error> {
    if d.array().map_err(bad)? != Some(n) {
        Err(Error::ExternalCandidateInvalid)
    } else {
        Ok(())
    }
}
fn bytes<'a>(d: &mut Decoder<'a>, min: usize, max: usize) -> Result<&'a [u8], Error> {
    let b = d.bytes().map_err(bad)?;
    if !(min..=max).contains(&b.len()) {
        Err(Error::LimitExceeded)
    } else {
        Ok(b)
    }
}
fn roster(d: &mut Decoder<'_>) -> Result<MlsRosterSummaryV1, Error> {
    array(d, 4)?;
    let group_id = bytes(d, 16, 16)?.to_vec();
    let epoch = d.u64().map_err(bad)?;
    let digest_sha256 = bytes(d, 32, 32)?.to_vec();
    let n = d
        .array()
        .map_err(bad)?
        .ok_or(Error::ExternalCandidateInvalid)?;
    if !(1..=256).contains(&n) {
        return Err(Error::LimitExceeded);
    }
    let mut leaves = Vec::with_capacity(n as usize);
    for _ in 0..n {
        array(d, 3)?;
        let l = MlsRosterLeafV1 {
            leaf_index: d.u32().map_err(bad)?,
            credential_identity: bytes(d, 45, 45)?.to_vec(),
            signature_public_key: bytes(d, 32, 32)?.to_vec(),
        };
        leaf_bounds(&l)?;
        leaves.push(l);
    }
    if validate_canonical_leaves(&leaves).is_err()
        || roster_digest(&group_id, epoch, &leaves).map_err(|_| Error::ExternalCandidateInvalid)?
            != digest_sha256
    {
        return Err(Error::ExternalCandidateInvalid);
    }
    Ok(MlsRosterSummaryV1 {
        group_id,
        epoch,
        digest_sha256,
        leaves,
    })
}
pub(super) fn read(input: &[u8], expected_hash: &[u8]) -> Result<Candidate, Error> {
    if input.is_empty() || input.len() > MAX_BYTES {
        return Err(Error::LimitExceeded);
    }
    fixed32(expected_hash)?;
    if hash(input) != expected_hash {
        return Err(Error::ExternalCandidateMismatch);
    }
    let mut d = Decoder::new(input);
    array(&mut d, 12)?;
    if d.u16().map_err(bad)? != 1 {
        return Err(Error::ExternalCandidateInvalid);
    }
    let candidate_id = bytes(&mut d, 1, 128)?.to_vec();
    let incarnation_id = bytes(&mut d, 1, 128)?.to_vec();
    let transition = MlsTransitionContext {
        command_id: bytes(&mut d, 1, 128)?.to_vec(),
        context_sha256: bytes(&mut d, 32, 32)?.to_vec(),
    };
    let lifecycle = bytes(&mut d, 32, 32)?.to_vec();
    let retention = d.u32().map_err(bad)?;
    validate_retention(retention)?;
    let public_codec::Request::Successor(proof) =
        public_codec::decode_request(bytes(&mut d, 1, public_codec::REQUEST_MAX_BYTES)?)
            .map_err(public_error)?
    else {
        return Err(Error::ExternalCandidateInvalid);
    };
    let previous = roster(&mut d)?;
    let resulting = roster(&mut d)?;
    let private_digest = bytes(&mut d, 32, 32)?.to_vec();
    let n = d
        .array()
        .map_err(bad)?
        .ok_or(Error::ExternalCandidateInvalid)?;
    if n > 4096 {
        return Err(Error::LimitExceeded);
    }
    // Borrowed first pass checks aggregate secret-row budget before any row clone.
    let mut scan = d.clone();
    let mut total = 0usize;
    for _ in 0..n {
        array(&mut scan, 2)?;
        let key = bytes(&mut scan, 1, 4096)?;
        let value = bytes(&mut scan, 0, 2 * 1024 * 1024)?;
        total = total
            .checked_add(key.len() + value.len() + if is_global_key(key) { 0 } else { 16 })
            .ok_or(Error::LimitExceeded)?;
        if total > 6 * 1024 * 1024 {
            return Err(Error::LimitExceeded);
        }
    }
    if scan.position() != input.len() {
        return Err(Error::ExternalCandidateInvalid);
    }
    let mut rows = Entries(Vec::with_capacity(n as usize));
    for _ in 0..n {
        array(&mut d, 2)?;
        let key = bytes(&mut d, 1, 4096)?.to_vec();
        let value = bytes(&mut d, 0, 2 * 1024 * 1024)?.to_vec();
        let group_id = if is_global_key(&key) {
            None
        } else {
            Some(resulting.group_id.clone())
        };
        rows.0.push(MlsStorageEntry {
            key,
            value,
            group_id,
        });
    }
    if rows.0.windows(2).any(|w| w[0].key >= w[1].key) {
        return Err(Error::ExternalCandidateInvalid);
    }
    let c = Candidate {
        candidate_id,
        incarnation_id,
        transition,
        lifecycle,
        retention,
        proof: *proof,
        previous,
        resulting,
        private_digest,
        rows,
    };
    if encode(&c)?.as_slice() != input {
        return Err(Error::NoncanonicalEncoding);
    }
    validate(&c)?;
    Ok(c)
}
fn validate(c: &Candidate) -> Result<(), Error> {
    let b = c.binding()?;
    validate_binding(&b)?;
    if c.proof.base.profile != public_core::PROFILE
        || c.proof.base.expected.group_id != c.previous.group_id
        || c.proof.base.expected.epoch != c.previous.epoch
        || c.proof.base.expected.roster_sha256 != c.previous.digest_sha256
    {
        return Err(Error::ExternalCandidateInvalid);
    }
    if hash(&c.proof.base.group_info) != b.input_group_info_sha256
        || hash(&c.proof.commit) != b.commit_sha256
        || hash(&c.proof.reported_next_group_info) != b.resulting_group_info_sha256
    {
        return Err(Error::ExternalCandidateInvalid);
    }
    bounded_aad(&c.proof.expected_aad)?;
    validate_delta(&c.previous, &c.resulting, &b.old_leaf, &b.author)?;
    // Trust is the independently retained hash of package-issued bytes, NOT an
    // attacker-supplied self-hash. Do not redo time-dependent base leaf validity
    // during durable receipt/restart recovery after a previously valid prepare.
    let loaded = load(Context {
        group_id: c.resulting.group_id.clone(),
        incarnation_id: c.incarnation_id.clone(),
        expected_current: state(&c.resulting),
        expected_base_sha256: c.private_digest.clone(),
        expected_retention: c.retention,
        entries: c.rows.0.clone(),
        storage_format: 1,
    })?;
    if !loaded.group.is_active()
        || loaded.local.pending.is_some()
        || loaded.local.join_epoch != c.resulting.epoch
        || loaded.group.own_leaf_index().u32() != b.author.leaf_index
    {
        return Err(Error::ExternalCandidateInvalid);
    }
    Ok(())
}
