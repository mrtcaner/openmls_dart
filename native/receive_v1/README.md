# Native receive v1

Status: implementation/release preparation for issue #33. This surface is not
published until the platform, limit, fixture, zeroization, signing, and iOS
Notification Service Extension gates are complete.

This is one strict Rust/OpenMLS receive implementation with thin platform
transports. It never opens a database, retains a group object, applies caller
state, performs network work, or owns product policy. Every outcome has
`stateApplied=false`. A failure contains only a typed error—never plaintext, a
storage batch, or a resulting local digest.

## Frame

Every request/result starts with 12 bytes:

| Offset | Type | Meaning |
| --- | --- | --- |
| 0 | 4 bytes | ASCII `KMLS` |
| 4 | u16 BE | contract version `1` |
| 6 | u8 | operation: application `1`, Commit `2`, Welcome `3` |
| 7 | u8 | flags, exactly `0` |
| 8 | u32 BE | payload byte length |

The payload is deterministic CBOR: unsigned numeric map keys in ascending
order, definite containers, shortest integer/length encodings, and no unknown
keys, duplicates, tags, floats, indefinite values, or trailing bytes. Rust
performs a bounded nonallocating structural preflight before semantic decode.

Request maps:

- application/Commit keys `0..8`: profile ID, group ID, MLS wire bytes,
  expected AAD, expected authenticated sender leaf, canonical previous roster,
  canonical resulting roster, installation-local base-group-state SHA-256,
  complete storage-format-1 snapshot;
- Welcome keys `0..7`: profile ID, Welcome, optional ratchet tree, private
  signer, expected local leaf, canonical resulting roster,
  `expectedTargetKeyPackageSha256`, complete installation-global snapshot.

Roster authority is `{groupId, epoch, roster-v1 digest}`. A leaf is
`{leafIndex, 45-byte Basic Credential identity, 32-byte Ed25519 public key}`.
Welcome success additionally returns `consumedKeyPackageSha256`, derived from
the exact retained public KeyPackage selected by OpenMLS.

Success map keys are `0` contract version, `1` false, `2` typed operation
result. Failure uses keys `0` version, `1` false, `3: {3: numericErrorCode}`.
Storage batches contain format version, sorted upserts, sorted deletes, and up
to eight deleted group IDs. Callers atomically apply deletes, then upserts,
then group deletions together with deduplication/handoff/settlement records.

## Limits and product authority

Package fail-closed ceilings are 12 MiB request, 8 MiB result, 6 MiB snapshot
or batch, 4,096 storage entries/upserts/deletes, 1 MiB MLS/Welcome, 2 MiB tree,
16 KiB AAD, 4 KiB signer, 256 KiB plaintext, and 256 roster leaves.

These are implementation headroom, not product admission authority. Chat's
lower limits remain: 32 KiB application ciphertext, 2 KiB AAD, 16 KiB Commit,
16 KiB Welcome plus 16 KiB GroupInfo, roster 100, 192 KiB encoded operation,
256 KiB response, and eight operations per settlement.

## Platform ownership

Android uses the app-owned class in `android/OpenMlsNativeReceive.java`; its
fixed JNI symbols are compiled into the one existing `libopenmls_frb.so`. The
class only loads/passes/wipes byte arrays. Apply `android/consumer-rules.pro`.
The JVM may retain internal copies, so this is not a native-only plaintext
claim.

Apple uses the header in `include/openmls_receive_v1.h` from the single
`OpenMls.framework`/XCFramework shared by the app and extension. The caller
must invoke `openmls_receive_v1_free` exactly once for every returned buffer;
free reconstructs and zeroizes the complete Rust allocation.

Welcome calls require the installation writer/lifecycle fence. Application and
Commit calls require the exact-group writer plus lifecycle fence. Cross-process
serialization and the caller's SQLite transaction remain consumer authority.
