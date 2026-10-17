# Public validator binary contract 1

Status: coordinated integration contract, 2026-10-16; not released. Tracking #59.
Executable: `openmls_public_validator`, built from this package's Rust core via
`make build-public-validator`. Linux-musl amd64/arm64 packaging is a separate
gate. No database/network, secret inputs, JSON, retained process, or shell protocol.

## Framing and canonical encoding

Exactly one request on stdin, one response on stdout. Request maximum **4 MiB**,
response maximum **256 KiB**, including the 12-byte header. A consumer may apply
stricter product admission. Process memory, timeout, concurrency, cancellation and
reaping are caller responsibilities; a semaphore does not enforce memory limits.

Header: bytes0..3 ASCII `KMPV`; bytes4..5 version1 unsigned big-endian u16;
byte6 operation (1 snapshot, 2 external successor); byte7 direction (0 request,
1 response); bytes8..11 payload length unsigned big-endian u32. No trailing data.
No flags/reserved bits. Response echoes operation; malformed short requests use0.

Payload is deterministic CBOR via minicbor: definite arrays/byte strings only,
unsigned integers with shortest representation, no maps, text, tags, floats,
negative integers or indefinite items. Re-encoding a decoded request MUST equal
the original payload. Below `bN` means exactly N bytes, `u64` is an unsigned
64-bit integer, and every listed array has exactly the stated arity/order.

- `Leaf = [index:u32, credential:b45, signatureKey:b32]`; index0..255.
- `State = [groupId:b16, epoch:u64, rosterDigest:b32]`.
- `Roster = [groupId:b16, epoch:u64, rosterDigest:b32, leaves:[Leaf...]]`;
  leaves1..256, ascending unique indices, unique credentials and signature keys.
  Roster digest is existing roster-summary/v1, unchanged.
- `SnapshotRequest = [profile:u16, groupInfo:bytes, sha256:b32, expectedState:State,
  expectedSigner:Leaf]`.
- `VerifiedSnapshot = [profile:u16, sha256:b32, roster:Roster, signer:Leaf]`.

Operation1 payload is `SnapshotRequest`.

Operation2 payload is the nine-item array:
`[base:SnapshotRequest, commit:bytes, commitSha256:b32, expectedAAD:bytes,
authorizedOld:Leaf, authorizedNewCredential:b45, authorizedNewSignatureKey:b32,
reportedNextGroupInfo:bytes, reportedNextSha256:b32]`.

Response payload is `[requestSha256:b32, code:u16, result]`.
Code0 means success. Operation1 success result is `VerifiedSnapshot`.
Operation2 success result is `[previous:VerifiedSnapshot, commitSha256:b32,
aadSha256:b32, resulting:VerifiedSnapshot]`. Every failure has `result=null`;
no partially verified snapshot/batch/plaintext. SHA-256 covers the complete exact
request frame. Oversized input that cannot be fully retained returns code6,
operation0 and an all-zero request hash; it must never be accepted as success.

CLI returns exit0 for a well-delivered typed success or failure frame. I/O failure
returns exit74; caller must reject missing/truncated/noncanonical/mismatched
responses and nonzero exit. No logs or secret/raw request diagnostics on stdout.

## Validation semantics and limits

Profile2: MLS1.0 / suite1 (X25519/AES128GCM/SHA256/Ed25519), Basic credential45,
signature key32. GroupInfo must embed ratchet_tree and external_pub; no separate
tree artifact or optional missing-next behavior. Existing private retention and
wire-policy configuration cannot be proven from public GroupInfo; unchanged
private callers enforce them. GroupContext extensions remain empty in this
narrow profile; unsupported extensions fail closed.

Package per-artifact limit1MiB; AAD1..16384 bytes; active leaves<=256; encoded tree
nodes<=511, depth<=8; bounded capabilities/extensions/unmerged-leaf/path lists
are checked before OpenMLS deserialization allocations. Existing product limits
are independent and must not be widened merely because package headroom exists.
Valid100/256-leaf and sparse-tree fixture measurements remain required.

Snapshot authority uses the frozen publication signer/roster, not today's root
key. OpenMLS verifies leaf/tree structure, GroupInfo signature/hash and lifetime;
package checks exact expected coordinates/digest and signer leaf.

Successor base and old-to-new mapping are authenticated by the caller. The caller
MUST establish that the new credential/key is the current authorized successor
of the exact old participant/membership; native does not authenticate accounts.
Generation need not be old+1. The credential may remain identical to the exactly
authorized removed leaf (normal stable-identity installation rotation). The new
signing key MUST differ from every previous key, including the removed leaf's.
Neither credential nor signing key may collide with any unchanged leaf. Caller
still authenticates the same business membership and authorized new generation.
This profile requires exactly one ExternalInit and one Remove of the specified old leaf, no
other proposals or references, unchanged unaffected leaves INCLUDING indices,
same leaf count and epoch+1. New index is derived by OpenMLS, not predicted by Chat.

`reportedNext` is a preacceptance CLAIM, not canonical expected-result authority.
The validator processes the public external Commit and independently derives
the successor. Next GroupInfo must match full resulting group context/tree and
confirmation bytes, with signer equal to the authorized new path leaf. Result
roster is returned for canonicalization. Output hash belongs in acceptance
binding, NOT in the originating Commit AAD (which would be circular). Exact AAD
is caller-authenticated opaque bytes; the package does not reinterpret app AAD
versions. Existing member/historical AAD encodings are unaffected.

Public processing cannot validate secret-dependent confirmation MAC or prove
external_pub derives from private epoch secrets. Internal encrypted Commits are
not accepted by operation2. Existing private receivers retain full MLS checks.
Backend must authenticate the response process/artifact, bind its request hash,
then recheck hashes/authority under transaction locks before canonical publication.

## Error codes

1 invalid_frame; 2 unsupported_version; 3 unsupported_profile;
4 unsupported_operation; 5 noncanonical_encoding; 6 limit_exceeded;
10 hash_mismatch; 20 group_mismatch; 21 epoch_mismatch; 22 roster_mismatch;
23 signer_mismatch; 24 identity_mismatch; 25 delta_mismatch; 26 aad_mismatch;
30 decode_rejected; 31 signature_rejected; 32 tree_rejected;
33 lifetime_rejected; 34 protocol_rejected; 35 missing_required_extension;
36 successor_mismatch; 255 internal_failure.

Mechanism errors do not authorize retry, recovery, revocation or admission.
Backend and UI own those policy decisions. No lifetime bypass is provided.

## Version separation

Additive native4.1.0 release candidate; this public protocol1 is independent of
mobile native receive frame3, unchanged active crypto/profile2,
storage1/metadata1/roster-digest1/state-digest1, and external candidate envelope1.
App client capability and database schema versions are not package protocol fields.
No production release has been published for this candidate.

## Caller-storage current GroupInfo export

Rust `export_group_info_with_storage(context, signer_bytes, expected_local_leaf)`
uses the existing `MlsGroupOperationContext` and `MlsRosterLeafV1`. It returns
`ExportGroupInfoWithStorageOutcome::{Success, Failure}` using existing lifecycle
`MlsErrorCode` values (a separate namespace from this executable's error codes).
Success contains `group_info`, `group_info_sha256`, `current_roster`, `signer`,
`base_group_state_sha256` and `effective_retention`. There is no mutation batch.
The implementation also verifies that the provider produced no changes.

Absent, inactive, pending-Commit, wrong incarnation/profile/retention, stale base,
wrong current epoch/roster and nonlocal signer/leaf fail closed. Exported GroupInfo
includes the tree and external public key and passes the same public profile,
including strict lifetime checks. Public validation rejection maps to lifecycle
`MlsProtocolRejected`, except limit/profile/internal errors retain their existing
specific lifecycle codes. No metadata is manufactured for legacy/missing state.
Caller owns snapshot fencing and canonical publication; successful export does
not grant business authority or imply the snapshot is still current later.
Generated Dart wiring exposes this API through the same strict caller-owned
storage boundary.

## Synthetic shared fixtures and host replay

`native/public_validator_v1/fixtures/manifest.json` records eleven synthetic binary
request/response pairs and lowercase hexadecimal SHA-256 hashes. No private
signer/storage bytes are retained. Valid one- and two-leaf fixtures use Commit-origin
leaves (no KeyPackage lifetime), so replay does not require clock overrides. Dynamic
sparse multi-leaf and expired-leaf tests still enforce lifetime policy unchanged.

- `make export-public-validator-fixtures`: explicitly regenerate random synthetic
  fixtures; never part of routine test execution.
- `make test-public-validator`: replay fixed vectors and focused parser/protocol tests.
- `make test-public-validator-host`: build the host executable and compare every
  real stdin/stdout result byte-for-byte; require exit0 and empty stderr.
  Captured results are under ignored `.buildlog/public-validator-host/`.

Host replay is not evidence of Linux musl/Android/Apple packaging or peak memory.
Bounded Linux and 100/256-leaf measurements were completed separately; they do
not establish mobile extension execution or production rollout acceptance.
