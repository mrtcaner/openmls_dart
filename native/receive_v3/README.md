# Native receive frame3

Implementation: `rust/src/native_receive_v3.rs`. Additive Android JNI v3 and
Apple C v3 entry points call this same core. Existing v2 exports, version
reporting and wire behavior remain unchanged. Export availability alone does
not establish Android/Apple consumer parity.

The12-byte header is `KMLS`, big-endian u16 version3, u8 operation, zero flags,
big-endian u32 payload length. Operations1 application,2 member Commit and3
Welcome retain the exact v2 request payload and semantic core. Operation4 is
explicit external Commit receive. No auto-detection/downgrade of version2 input.

All payloads use the existing deterministic CBOR subset: ascending unsigned map
keys, shortest integer/length encodings, definite containers, no tags/floats/text.
The same borrowed preflight and snapshot decoder enforce12MiB request,8MiB result,
6MiB snapshot/batch,4096 rows,4096-byte keys,2MiB values,1MiB MLS message,
nonempty AAD<=16KiB,256 leaves, depth6 and32768 items. Product limits are separate.

## External request payload

Exact ten-entry map:

| Key | Value |
| --- | --- |
|0|profile2|
|1|group ID,16 bytes|
|2|incarnation,1..128 bytes|
|3|external Commit bytes|
|4|exact expected authenticated AAD bytes|
|5|installation-local base-state SHA256,32 bytes|
|6|retention2 or4|
|7|accepted cryptographic authority map below|
|8|exact superseded internal-pending binding, or null|
|9|existing storage-format1 snapshot encoding|

Authority is an eleven-entry map:0 candidate ID (1..128 bytes);1 submission
command ID (1..128 bytes);2 authenticated authorized-delta digest (32 bytes);
3 previous expected roster state;4 resulting expected roster state;5 removed
leaf;6 resulting-roster author;7 input GroupInfo SHA256;8 resulting GroupInfo
SHA256;9 Commit SHA256;10 AAD SHA256. Existing leaf/expected-state encodings are
reused unchanged. Incarnation is taken from outer key2, not separately repeated.
ID encoding is caller-owned and must be canonical/identical across calls.

Pending binding is null or an exact ten-entry map:0 group ID;1 incarnation;
2 transition command ID;3 transition context digest;4 author;5 previous state;
6 Commit SHA256;7 AAD SHA256;8 optional Welcome SHA256;9 optional GroupInfo SHA256.
Optional hashes use CBOR null. This is the existing member pending binding, not
an external candidate or a new private-state backup.

Before cloning secret rows the decoder checks profile, retention, field bounds,
matching group IDs/incarnation, resulting epoch=previous+1, exact Commit/AAD
hashes and pending binding scope. The N2 core then checks the local snapshot/base,
actual previous roster, exact pending loser, full private MLS processing, exact
authenticated AAD, resulting-roster sender, authorized delta and resulting state.
Own external acceptance uses candidate promotion, not this survivor receive path.

## Read-only pending inspection (operation5, frozen 2026-10-17)

Header remains frame3, operation byte5. Request is an exact seven-entry map:
0 profile2; 1 group ID (16 bytes); 2 incarnation (1..128 bytes);
3 current expected roster state (`{0:groupIdBytes,1:epochU64,2:digestBytes}`);
4 installation-local current base-state SHA256 (32 bytes); 5 retention (2 or4);
6 storage-format1 snapshot (same scoped encoding/bounds as operation4).
The expected roster's group ID must equal field1. Public profile, lengths,
retention and scope are validated before cloning snapshot rows. The existing
`runtime::inspect_pending`/`load` validates base, incarnation, roster/epoch,
configuration, storage format and native/local pending consistency.

Success payload is exactly `{0:3,1:false,2:{0:pendingBindingOrNull}}`.
The binding reuses the exact ten-field operation4 pending encoding, including
nullable Welcome and GroupInfo hashes. Null means validated state has no pending
Commit; it never substitutes for a validation error. No Commit/GroupInfo bytes,
plaintext, mutation batch or additional result fields are returned.
Failure is unchanged `{0:3,1:false,3:{3:errorCode}}`, operation5 in the header
when recognized. Existing typed mechanism errors and all bounds remain unchanged.

This is installation-local inspection, not server authority or permission to
discard. Caller keeps the SAME current snapshot/base and transaction/lifecycle/
group writer fence from inspection through operation4 receive and atomic batch
application. Pass the full returned binding unchanged as expected superseded
pending; still correlate the authorized losing transition in caller logic.
Do not parse opaque storage, infer hashes from publication policy, or weaken
full equality. Concurrent/stale state requires a fresh fenced read, not a retry
with fabricated fields. The existing v3 JNI/C entry points carry operation5;
no new export or version bump. v2 and operations1–4 are unchanged.

## Receive result payload

Outer shape matches v2 with version3: `{0:3,1:false,2:success}` or
`{0:3,1:false,3:{3:errorCode}}`. Failure has no success, batch or plaintext.
Mechanism error codes are the existing shared MlsErrorCode values.

Operations1..3 preserve their v2 success payloads. Operation4 success is
`{0:transitionResult,1:supersededPendingOrNull}`. transitionResult reuses the
member-Commit result encoding:0 authenticated sender (from RESULT roster);
1 previous roster;2 resulting roster;3 resulting local-state SHA256;
4 complete storage-format1 batch;5 message epoch (=previous epoch);
6 effective retention. No Welcome, KeyPackage or plaintext is returned for op4.

The package never applies a batch. Caller owns the transaction, live-state CAS,
cross-process serialization, lifecycle/account/installation fencing and durable
receipt/dedup state. On success apply the entire batch atomically or discard it.

## Authority and memory boundary

Framing and hashes are not authorization. Caller authenticates the server
receipt and all business scope/identity/command relationships before invoking
native. Submission command ID is distinct from preparation command ID. The
context digest is the authenticated authorized-delta digest, not a receipt hash.
The package carries these values; it does not parse an application-specific AAD
to extract/compare its internal business fields or reconstruct mailbox/delta
serialization. Exact whole-AAD equality and hash checks remain mandatory.

No production JSON or new MLS implementation. v2 encoders/decoders/core are
reused, with only crate-internal visibility changes to shared helpers. Parsed
snapshot entries and returned native batch entries retain existing zeroization;
temporary encoding buffers are zeroizing. Borrowed request bytes and returned
response bytes still belong to the caller, which must wipe/free them. No group
is retained between calls. Platform copy/allocation/free tests remain separate.

## Current proof and remaining work

Tests compare complete operation4 transition/batch bytes against typed N2,
with and without a pending loser; reject malformed/mismatched frames without
batch/plaintext; and verify selected v2 golden results remain byte-identical
while frame3 member results differ only in envelope version. A separate opt-in
test checks exact carriage of externally supplied frozen AAD/context bytes;
it does not independently implement the application's business encoders.

The bounded Apple artifact harness compares one real external success (including
the complete batch) and one base-mismatch failure against core-produced bytes.
The opt-in fixture exporter first checks the success against the typed N2 API.
Fixtures contain synthetic private test state and stay in ignored build evidence.
Run `make native-receive-v3-adapter-fixture`, `make build ARGS=--lib`, then
`make native-receive-v3-apple-artifact`. C callers use
`include/openmls_receive_v3.h` and free returned buffers exactly once.
Android adds `nativeExecuteReceiveV3` and `nativeContractVersionV3` on the
existing mechanical class; `nativeContractVersion` remains 2.

Consumer integration, JNI device execution and iOS framework/extension runtime
validation remain separate gates. No released or full mobile parity claim is
made by this bounded host artifact proof.
