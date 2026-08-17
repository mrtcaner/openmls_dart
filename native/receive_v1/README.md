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

Stable numeric errors:

| Code | Meaning | Code | Meaning |
| ---: | --- | ---: | --- |
| 1 | invalid frame | 2 | unsupported contract version |
| 3 | unsupported profile | 4 | unsupported operation |
| 5 | noncanonical encoding | 6 | limit exceeded |
| 10 | storage format mismatch | 11 | invalid storage snapshot |
| 12 | group state unavailable | 13 | local base state mismatch |
| 20 | configuration mismatch | 21 | group mismatch |
| 22 | previous epoch mismatch | 23 | previous roster mismatch |
| 24 | resulting epoch mismatch | 25 | resulting roster mismatch |
| 26 | AAD mismatch | 27 | message kind mismatch |
| 28 | sender mismatch | 29 | local leaf mismatch |
| 30 | invalid signer | 31 | unsupported credential |
| 32 | MLS decode rejected | 33 | Welcome rejected |
| 34 | MLS protocol rejected | 35 | expected KeyPackage mismatch |
| 255 | contained internal failure | | |

## Limits and product authority

Package fail-closed ceilings are 12 MiB request, 8 MiB result, 6 MiB snapshot
or batch, 4,096 storage entries/upserts/deletes, 1 MiB MLS/Welcome, 2 MiB tree,
16 KiB AAD, 4 KiB signer, 256 KiB plaintext, and 256 roster leaves.

These are implementation headroom, not product admission authority. Chat's
lower limits remain: 32 KiB application ciphertext, 2 KiB AAD, 16 KiB Commit,
16 KiB Welcome plus 16 KiB GroupInfo, roster 100, 192 KiB encoded operation,
256 KiB response, and eight operations per resolve response/page.

## Platform ownership

Android uses the app-owned class in `android/OpenMlsNativeReceive.java`; its
fixed JNI symbols are compiled into the one existing `libopenmls_frb.so`. The
class only loads/passes/wipes byte arrays. Apply `android/consumer-rules.pro`.
The JVM may retain internal copies, so this is not a native-only plaintext
claim.

Apple uses the header in `include/openmls_receive_v1.h`. The three C symbols
are compiled into the same `libopenmls_frb.dylib` that carries the FRB symbols;
the Dart code-assets pipeline converts that one dylib into the app framework.
The Notification Service Extension must link that same generated framework.
Do not add a companion `OpenMls.framework` or a second Rust/OpenMLS binary.
Final generated product/module/install-name spelling remains open until the
actual extension packaging gate. The caller must invoke
`openmls_receive_v1_free` exactly once for every returned buffer; free
reconstructs and zeroizes the complete Rust allocation.

Welcome calls require the installation writer/lifecycle fence. Application and
Commit calls require the exact-group writer plus lifecycle fence. Cross-process
serialization and the caller's SQLite transaction remain consumer authority.

## Shared vectors and harnesses

`fixtures/manifest.json` records thirteen synthetic canonical binary
request/result pairs: Welcome, application and Commit success; wrong
KeyPackage hash, local leaf, base digest, empty/mismatched AAD, sender,
resulting roster and kind; plus successful Welcome and application operations
at the package's 256-leaf ceiling. Rust replays the committed bytes exactly in
its test suite.

- `android/run_avd_harness.sh <serial>` compiles the fixed Java class, loads the
  existing arm64 `libopenmls_frb.so`, compares all thirteen results
  byte-for-byte, and verifies Java request/result array wiping.
- `apple/run_macos_harness.sh` compiles the C header boundary, compares all
  thirteen results, and frees every returned Rust allocation through the public
  API.

Evidence on 2026-08-17: Android arm64 API 28 and the macOS Apple boundary both
reported `13 passed=true`; the iOS arm64 device target built and exported
`openmls_receive_v1_execute`, `openmls_receive_v1_free`, and
`openmls_receive_v1_version`. Simulator/physical Notification Service
Extension execution and final framework naming are still release gates.

The generated 256-leaf fixture stays far below every package ceiling. Welcome
used an 88,669-byte request, 299,738-byte response, 2,037-byte input snapshot,
and 277,126-byte output batch. Application used a 311,320-byte request,
82,252-byte response, 276,979-byte snapshot, and 7,773-byte batch. Its MLS
ciphertext is 31,895 bytes, below Chat's independent 32 KiB admission limit.
On the API-28 arm64 AVD, Welcome/application took 28,860/14,561 microseconds
with process high-water RSS reaching 83,148 KiB. The macOS Apple boundary took
29,222/11,885 microseconds with high-water RSS reaching 10,764,288 bytes. These
are constructibility measurements, not production service-level guarantees.

Same-toolchain stripped release binaries were compared with signed tag
`openmls_frb-3.0.0` (`280752b`):

| Target | 3.0.0 bytes | Native receive v1 bytes | Delta |
| --- | ---: | ---: | ---: |
| macOS arm64 | 4,794,800 | 4,937,856 | +143,056 (+3.0%) |
| iOS arm64 device | 4,754,984 | 4,913,320 | +158,336 (+3.3%) |
| Android arm64 | 6,853,464 | 7,108,344 | +254,880 (+3.7%) |
| Android x86_64 | 6,250,600 | 6,458,832 | +208,232 (+3.3%) |
