//! KMPV/1 fixed-schema canonical CBOR, implemented with minicbor.
use super::*;
use minicbor::{Decoder, Encoder};
use sha2::{Digest, Sha256};
pub const VERSION: u16 = 1;
pub const REQUEST_MAX_BYTES: usize = 4 * 1024 * 1024;
pub const RESULT_MAX_BYTES: usize = 256 * 1024;
const HEADER: usize = 12;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Snapshot(SnapshotRequest),
    Successor(Box<SuccessorRequest>),
}
fn array(d: &mut Decoder<'_>, n: u64) -> Result<(), Error> {
    if d.array().map_err(|_| Error::InvalidFrame)? != Some(n) {
        return Err(Error::InvalidFrame);
    }
    Ok(())
}
fn bytes(d: &mut Decoder<'_>, min: usize, max: usize) -> Result<Vec<u8>, Error> {
    let b = d.bytes().map_err(|_| Error::InvalidFrame)?;
    if b.len() < min || b.len() > max {
        return Err(Error::LimitExceeded);
    }
    Ok(b.to_vec())
}
fn leaf(d: &mut Decoder<'_>) -> Result<Leaf, Error> {
    array(d, 3)?;
    let l = Leaf {
        index: d.u32().map_err(|_| Error::InvalidFrame)?,
        identity: bytes(d, 45, 45)?,
        signature_key: bytes(d, 32, 32)?,
    };
    public::validate_leaf(&l)?;
    Ok(l)
}
fn state(d: &mut Decoder<'_>) -> Result<ExpectedState, Error> {
    array(d, 3)?;
    Ok(ExpectedState {
        group_id: bytes(d, 16, 16)?,
        epoch: d.u64().map_err(|_| Error::InvalidFrame)?,
        roster_sha256: bytes(d, 32, 32)?,
    })
}
fn snapshot(d: &mut Decoder<'_>) -> Result<SnapshotRequest, Error> {
    array(d, 5)?;
    Ok(SnapshotRequest {
        profile: d.u16().map_err(|_| Error::InvalidFrame)?,
        group_info: bytes(d, 1, MAX_MLS_BYTES)?,
        sha256: bytes(d, 32, 32)?,
        expected: state(d)?,
        expected_signer: leaf(d)?,
    })
}
fn put_leaf(e: &mut Encoder<Vec<u8>>, l: &Leaf) {
    e.array(3)
        .unwrap()
        .u32(l.index)
        .unwrap()
        .bytes(&l.identity)
        .unwrap()
        .bytes(&l.signature_key)
        .unwrap();
}
fn put_state(e: &mut Encoder<Vec<u8>>, s: &ExpectedState) {
    e.array(3)
        .unwrap()
        .bytes(&s.group_id)
        .unwrap()
        .u64(s.epoch)
        .unwrap()
        .bytes(&s.roster_sha256)
        .unwrap();
}
fn put_snapshot_request(e: &mut Encoder<Vec<u8>>, r: &SnapshotRequest) {
    e.array(5)
        .unwrap()
        .u16(r.profile)
        .unwrap()
        .bytes(&r.group_info)
        .unwrap()
        .bytes(&r.sha256)
        .unwrap();
    put_state(e, &r.expected);
    put_leaf(e, &r.expected_signer);
}
fn payload(r: &Request) -> Vec<u8> {
    let mut e = Encoder::new(Vec::new());
    match r {
        Request::Snapshot(s) => put_snapshot_request(&mut e, s),
        Request::Successor(s) => {
            e.array(9).unwrap();
            put_snapshot_request(&mut e, &s.base);
            e.bytes(&s.commit)
                .unwrap()
                .bytes(&s.commit_sha256)
                .unwrap()
                .bytes(&s.expected_aad)
                .unwrap();
            put_leaf(&mut e, &s.authorized_old);
            e.bytes(&s.authorized_new_identity)
                .unwrap()
                .bytes(&s.authorized_new_signature_key)
                .unwrap()
                .bytes(&s.reported_next_group_info)
                .unwrap()
                .bytes(&s.reported_next_sha256)
                .unwrap();
        }
    };
    e.into_writer()
}
fn frame(operation: u8, response: bool, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(b"KMPV");
    out.extend_from_slice(&VERSION.to_be_bytes());
    out.push(operation);
    out.push(u8::from(response));
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}
/// Fixture/client encoder. Requests are validated by the decoder and strict core.
pub fn encode_request(r: &Request) -> Result<Vec<u8>, Error> {
    let p = payload(r);
    if p.len() > REQUEST_MAX_BYTES - HEADER {
        return Err(Error::LimitExceeded);
    }
    Ok(frame(
        match r {
            Request::Snapshot(_) => 1,
            Request::Successor(_) => 2,
        },
        false,
        &p,
    ))
}
pub fn decode_request(input: &[u8]) -> Result<Request, Error> {
    if input.len() > REQUEST_MAX_BYTES {
        return Err(Error::LimitExceeded);
    }
    if input.len() < HEADER || &input[..4] != b"KMPV" || input[7] != 0 {
        return Err(Error::InvalidFrame);
    }
    if u16::from_be_bytes([input[4], input[5]]) != VERSION {
        return Err(Error::UnsupportedVersion);
    }
    let n = u32::from_be_bytes([input[8], input[9], input[10], input[11]]) as usize;
    if n != input.len() - HEADER {
        return Err(Error::InvalidFrame);
    }
    let mut d = Decoder::new(&input[HEADER..]);
    let r = match input[6] {
        1 => Request::Snapshot(snapshot(&mut d)?),
        2 => {
            array(&mut d, 9)?;
            Request::Successor(Box::new(SuccessorRequest {
                base: snapshot(&mut d)?,
                commit: bytes(&mut d, 1, MAX_MLS_BYTES)?,
                commit_sha256: bytes(&mut d, 32, 32)?,
                expected_aad: bytes(&mut d, 1, MAX_AAD_BYTES)?,
                authorized_old: leaf(&mut d)?,
                authorized_new_identity: bytes(&mut d, 45, 45)?,
                authorized_new_signature_key: bytes(&mut d, 32, 32)?,
                reported_next_group_info: bytes(&mut d, 1, MAX_MLS_BYTES)?,
                reported_next_sha256: bytes(&mut d, 32, 32)?,
            }))
        }
        _ => return Err(Error::UnsupportedOperation),
    };
    if d.position() != n {
        return Err(Error::InvalidFrame);
    }
    if payload(&r) != input[HEADER..] {
        return Err(Error::NoncanonicalEncoding);
    }
    Ok(r)
}
fn put_verified(e: &mut Encoder<Vec<u8>>, v: &VerifiedSnapshot) {
    e.array(4)
        .unwrap()
        .u16(v.profile)
        .unwrap()
        .bytes(&v.sha256)
        .unwrap();
    e.array(4)
        .unwrap()
        .bytes(&v.roster.state.group_id)
        .unwrap()
        .u64(v.roster.state.epoch)
        .unwrap()
        .bytes(&v.roster.state.roster_sha256)
        .unwrap()
        .array(v.roster.leaves.len() as u64)
        .unwrap();
    for l in &v.roster.leaves {
        put_leaf(e, l);
    }
    put_leaf(e, &v.signer);
}
fn failure(operation: u8, digest: &[u8], error: Error) -> Vec<u8> {
    let mut e = Encoder::new(Vec::new());
    e.array(3)
        .unwrap()
        .bytes(digest)
        .unwrap()
        .u16(error as u16)
        .unwrap()
        .null()
        .unwrap();
    frame(operation, true, &e.into_writer())
}
/// One request, one response; no partial validated result on any failure.
pub fn execute(input: &[u8]) -> Vec<u8> {
    if input.len() > REQUEST_MAX_BYTES {
        return failure(0, &[0; 32], Error::LimitExceeded);
    }
    let digest = Sha256::digest(input);
    let op = input.get(6).copied().unwrap_or(0);
    let run = || -> Result<Vec<u8>, Error> {
        let request = decode_request(input)?;
        let mut e = Encoder::new(Vec::new());
        e.array(3).unwrap().bytes(&digest).unwrap().u16(0).unwrap();
        match request {
            Request::Snapshot(s) => put_verified(&mut e, &public::validate_snapshot(&s)?),
            Request::Successor(s) => {
                let v = public::validate_external_successor(&s)?;
                e.array(4).unwrap();
                put_verified(&mut e, &v.previous);
                e.bytes(&v.commit_sha256)
                    .unwrap()
                    .bytes(&v.aad_sha256)
                    .unwrap();
                put_verified(&mut e, &v.resulting);
            }
        }
        let result = frame(op, true, &e.into_writer());
        if result.len() > RESULT_MAX_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(result)
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
        .unwrap_or(Err(Error::InternalFailure))
    {
        Ok(r) => r,
        Err(err) => failure(op, &digest, err),
    }
}
