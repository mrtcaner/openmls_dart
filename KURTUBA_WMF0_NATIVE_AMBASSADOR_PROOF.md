# Kurtuba WMF0 Native Ambassador Proof

**Date:** 2026-08-17

**Status:** Proof complete; coordinated production contract approved; release blocked on evidence gates

**Base:** `openmls_frb-3.0.0`, storage format `1`

**Branch:** `codex/wmf0-ambassador-proof`

**Implementation commit:** `de8be8a84e4394b746c2e5cd7974b02ba63c0633`

## Decision

The native fast path is constructible without a second MLS implementation or
a second OpenMLS binary in either platform package.

One feature-gated Rust core implements the strict operation semantics. Android
uses a thin JNI transport compiled into `libopenmls_frb.so`. Apple uses a thin
C-compatible buffer transport compiled into the same `openmls_frb` dynamic
library that is packaged as an extension-safe framework. The Android and Apple
transport mechanics are intentionally different; their MLS semantics and
storage-format-1 result are the same.

This result authorizes contract design work only after owner review. It does
not freeze a public ABI, codec, package version, error wire representation, or
release design.

## Persistent contract review status

On 2026-08-17 the Flutter owner accepted the fork's proposed persistent native
receive contract in full. Chat then supplied its final product limits and
accepted the package boundary, including the exact KeyPackage binding below.
Package-owned implementation and release preparation are authorized. Publishing
a release remains prohibited until every platform, zeroization, fixture,
packaging, and signing gate in this document passes and the exact coordinates
are handed back.

The consumer-approved package proposal is:

- an additive `3.1.0` native-only receive surface, native contract version `1`,
  with storage format `1` and no required Dart API break;
- one typed Rust/OpenMLS receive core for Welcome, Commit, and application,
  called by mechanically thin Android and Apple transports;
- a 12-byte `KMLS` frame followed by deterministic, definite-length CBOR with
  a non-allocating bounds/canonical-form preflight before semantic decoding;
- exact profile/group/kind/sender/AAD/previous-resulting roster/base-digest
  validation where applicable, with no AAD or previous group digest invented
  for Welcome;
- exact Welcome KeyPackage consumption binding: Flutter correlates Chat's
  authenticated `targetKeyPackageId` to the retained publication record, while
  the package receives `expectedTargetKeyPackageSha256`, verifies the exact
  locally retained KeyPackage OpenMLS selects, and returns the derived
  `consumedKeyPackageSha256`;
- stable numeric typed errors originating inside Rust, with
  `stateApplied=false` for every outcome and no batch, plaintext, or resulting
  digest on failure;
- the complete caller-owned storage-format-1 batch, unchanged atomic apply-or-
  discard ownership, and explicit buffer zeroization/free rules; and
- no JSON, proof symbols, legacy string matching, package database, retained
  group, second MLS implementation, or duplicate OpenMLS binary in production.

The provisional fail-closed ceilings are 12 MiB per request, 8 MiB per result,
6 MiB per storage snapshot or batch, 4,096 snapshot/upsert/delete entries, a
256-leaf package roster, 1 MiB MLS/Welcome bytes, 2 MiB ratchet trees, 16 KiB
AAD, 256 KiB plaintext, 4 KiB storage keys, and 2 MiB storage values. Profile
`1` fixes Kurtuba's existing MLS 1.0, X25519/AES-128-GCM/SHA-256/Ed25519,
ciphertext, ratchet-tree, `maxPastEpochs=0`, padding `0`, sender-ratchet
`5/1000`, and zero-resumption-PSK configuration. Credential identities and
signature keys are exactly 45 and 32 bytes. Product release membership remains
independently capped at 100.

Chat's product admission limits are intentionally lower than those package
ceilings: 32 KiB application ciphertext, 2 KiB application/Commit AAD, 16 KiB
Commit, 16 KiB Welcome plus 16 KiB GroupInfo, 100 roster members, 192 KiB per
encoded ambassador operation, 256 KiB per response, and at most eight
operations. Chat and Flutter enforce these before the package call. The larger
package limits are implementation headroom and fail-closed defense only; they
do not authorize larger network or product inputs.

`targetKeyPackageId` remains Chat/Flutter correlation authority and never enters
the package frame. `expectedTargetKeyPackageSha256` is exactly 32 bytes in the
Welcome request. Before staging, the package reproduces OpenMLS's deterministic
selection of the first Welcome KeyPackage reference present in the supplied
snapshot, hashes that selected bundle's canonical public KeyPackage bytes, and
requires equality. A mismatch returns typed
`expected_key_package_mismatch`, `stateApplied=false`, and no batch, plaintext,
or resulting digest. This is necessary because multiple KeyPackages for one
installation can share the same Basic Credential identity and signing key;
leaf/signer/roster validation alone cannot distinguish which private init-key
bundle was consumed.

Flutter resolved the transport/packaging questions as follows:

1. Android uses the fixed app-owned class
   `app.kurtuba.openmls.OpenMlsNativeReceive`. It only passes and wipes byte
   arrays. Package-owned JNI symbols stay in the single existing
   `libopenmls_frb.so`; explicit keep rules and symbol/load tests are required.
2. Apple provisionally uses one `OpenMls.framework`/XCFramework and
   `openmls_receive_v1_execute/free/version`. Final product, module,
   install-name, and export spelling freeze during packaging implementation;
   semantics and frame bytes may not change, and the extension gets no copy.
3. The provisional ceilings require generated worst-case 256-leaf evidence,
   Android device evidence, and physical Notification Service Extension
   memory/time evidence. Failure lowers the limits or rejects the architecture;
   it never silently raises them or splits the atomic transaction.

Flutter also accepted the ownership boundary proven in commit `f53a05da`: load
the complete snapshot only after `BEGIN IMMEDIATE`; use the installation-level
MLS writer/lifecycle fence for Welcome and the exact-group writer plus lifecycle
fence for application/Commit; then commit the complete batch, resulting local
digest, deduplication, handoff, and settlement intent together. Package error
codes prove mechanism only. Flutter and Chat continue to own retry, recovery,
settlement, and user experience.

## Shared proof contract

The proof is behind Cargo feature `wmf0-proof`, disabled by default. None of its
functions has a Flutter Rust Bridge annotation.

The disposable JSON envelope has a 96 MiB outer limit checked before field
decoding, followed by the smaller per-field, row-count and 64 MiB total storage
snapshot bounds in the semantic core. Production extension limits must be much
smaller and contract-owned.

### Existing-group Commit or application input

- proof contract version `0`;
- expected closed kind: `commit` or `application`;
- exact MLS group ID and wire message bytes;
- exact expected authenticated AAD;
- expected authenticated member sender:
  - leaf index;
  - Basic Credential identity;
  - signature public key;
- canonical expected previous and resulting group ID, epoch and roster digest;
- expected installation-local `baseGroupStateSha256`;
- bounded storage-format-1 snapshot.

Processing order matters. The package validates input limits and storage
format, recomputes and checks the local base digest, authenticates and processes
the MLS message, checks AAD, rejects proposals or a wrong closed kind, checks
the member sender against the authenticated previous roster, and checks the
canonical resulting state. Only then does it expose a candidate batch.

### Welcome input

- proof contract version `0`;
- MLS group configuration and serialized private signer;
- exact Welcome and optional ratchet tree;
- canonical expected resulting group ID, epoch and roster digest;
- installation-global storage-format-1 rows only.

Welcome has no MLS authenticated AAD. The package proves possession of the
private KeyPackage state, opens the Welcome, checks the local signer and exact
resulting state, and rejects any pre-existing group rows. Chat remains
responsible for authenticating the bootstrap coordinates and retained Welcome
hash outside MLS.

### Success and failure

A success returns:

- authenticated closed kind;
- exact group ID;
- applicable sender leaf and previous epoch;
- resulting epoch;
- previous/resulting roster summaries and digests;
- resulting installation-local group-state SHA-256;
- complete storage-format-1 batch;
- application plaintext only for an application item.

`state_applied` is always `false`: the package never owns or claims to have
mutated caller storage. Flutter/native must apply the complete batch inside its
own transaction or discard it.

Every proof failure returns no result, storage batch, or plaintext and reports
one stable proof-level code. Covered categories include contract/limit/encoding,
storage format/snapshot/group state, base/group/epoch/roster/AAD/kind/sender,
local signer, invalid Welcome, MLS rejection, and contained panic.

Standalone MLS proposals are outside this closed ambassador operation.

## Memory ownership

Android:

- JNI copies the request into Rust and zeroizes that copy after processing.
- Rust serializes the result, JNI copies it into one Java byte array, and Rust
  immediately zeroizes its result allocation.
- Java/Kotlin owns the returned byte array and must call `nativeZeroize` in a
  `finally` block after parsing and persisting the allowed fields.
- The JVM may retain implementation-internal copies; the proof makes no
  native-only plaintext claim.

Apple:

- the wrapper copies the caller request and zeroizes the copy;
- the returned `Wmf0ProofBuffer` is owned by the caller;
- the caller must invoke `wmf0_proof_buffer_free` exactly once; it reconstructs
  and zeroizes the Rust allocation before freeing it;
- no Rust pointer is retained after the call.

The shared success type zeroizes application plaintext and batch upsert values
on Rust drop. The existing receive path was also hardened to wipe application
plaintext when AAD or resulting-roster validation fails.

## Shared vectors

The exact synthetic bundle is
`proof/wmf0/fixtures/wmf0_vectors_2026-08-17.json`.

It contains:

1. successful Welcome join;
2. successful application processing;
3. successful Commit processing;
4. wrong local base digest;
5. wrong application AAD;
6. application supplied as expected Commit;
7. wrong member sender signature key;
8. wrong resulting Commit roster digest;
9. wrong resulting Welcome epoch.

All six failure vectors returned the expected typed code with
`state_applied=false` and no result. Android and Apple produced the exact same
semantic report:
`proof/wmf0/fixtures/wmf0_expected_report_2026-08-17.json`.

SHA-256:

- input bundle: `50cb928b0ae8f247e5cd87c10785e7de5fe76307d4930c44645b2f3e86c6200b`;
- semantic report: `f0fcac563bae96d3b5cafd7e8d80c3e9363de808e1f4d0cbe7d7e254f7b003ec`.

## Platform findings

### Android

The final proof binary was one arm64 `libopenmls_frb.so`, not a companion MLS
library. It exported both the existing FRB receive symbol and these proof JNI
symbols:

- `nativeExecute`;
- `nativeExecuteVectorBundle`;
- `nativeZeroize`.

The exact bundle passed in a fresh `app_process` on an arm64-v8a Android 9 / API
28 AVD. The Java harness also verified that request and response arrays were all
zero after `nativeZeroize` and that malformed JSON returned `invalid_encoding`,
no result, and `state_applied=false`.

### Apple

The arm64 iOS library was packaged with install name:

`@rpath/Wmf0OpenMlsProof.framework/Wmf0OpenMlsProof`

An unsigned generic-device Xcode build succeeded with an app target and a
Notification Service Extension target. Both linked the same framework. The
finished app contained exactly one `Wmf0OpenMlsProof.framework` under the app's
`Frameworks` directory and none inside the `.appex`. The extension used
`@executable_path/../../Frameworks` and was compiled with extension-only API
checking.

The exact vectors executed through the same final `openmls_frb` Apple buffer
boundary on the arm64 macOS host. The proof did not run an unsigned extension
on a physical iOS device; it proves iOS compilation, symbol resolution,
embedding topology and extension linkage.

## Size evidence

These are arm64 release binaries built from the same source tree and options;
the only variable is `wmf0-proof`.

| Platform | 3.0 feature off | Proof enabled | Raw increase | Deflated increase |
|---|---:|---:|---:|---:|
| Android `.so` | 6,852,104 B | 7,289,296 B | 437,192 B (6.38%) | 114,207 B (6.12%) |
| iOS dylib | 4,755,248 B | 5,031,880 B | 276,632 B (5.82%) | 85,147 B (5.97%) |

These are proof costs, not a release forecast. JSON, vector generation and
diagnostic details are intentionally still linked. A production binary should
use a smaller bounded codec and omit vector-generation code.

## Runtime evidence

Times cover native JSON decode, one strict MLS operation, result validation and
JSON encoding. They exclude file reads and `System.load`/process startup. Each
sample used a fresh harness process, so first-sample variance is visible.

Android API 28 arm64, three samples:

| Operation | Samples |
|---|---|
| Welcome | 4.90 ms, 1.27 ms, 1.22 ms |
| Application | 1.78 ms, 1.65 ms, 1.68 ms |
| Commit | 7.10 ms, 4.48 ms, 4.92 ms |

Observed Android RSS growth was about 2.2 MiB for Welcome/application and 5.5
MiB for Commit. The nine-vector bundle took 10.1-18.9 ms and grew RSS by about
5.9 MiB.

The arm64 macOS Apple-boundary host showed similar warmed operation times:
about 1.3-2.5 ms for Welcome/application and 4.4-4.5 ms for Commit. It is not an
iOS extension runtime benchmark.

Flutter's independent local-store proof commit `25cbce56` supplies the relevant
transaction evidence. On API 29 with sqlite3mc WAL, holding `BEGIN IMMEDIATE`
for 750 ms allowed a concurrent Runner read to finish in 561 microseconds and
see no uncommitted rows; a competing writer waited 744,649 microseconds and
then saw the complete committed batch. Process death before commit rolled back
dedup, handoff and settlement together; death after commit exposed all three.

Therefore MLS processing time inside the transaction is writer-blocking, not
inherently WAL-reader-blocking. The crash boundary must not be split merely to
shorten the writer lock.

## What remains before production

Flutter and Chat have approved the proposed answers to the proof's codec,
typed-error, limits, shared-core, version, KeyPackage binding, and platform-
transport questions. Production still requires:

1. Complete production zeroization review, including upstream signer-key copies
   that do not currently expose a zeroizing drop implementation.
2. Build Android/iOS release automation, signing and package validation only
   after the public contract is approved.
3. Generate worst-case 256-leaf codec/storage fixtures and prove the provisional
   bounds before treating them as final.
4. Run device-level iOS extension execution and final app package/RSS/latency
   measurements in the eventual consumer integration.
5. Prove the production decoder, typed errors, shared Rust core, zeroization,
   Android ABI/R8 loading, Apple single-framework topology, archive notices,
   checksums, signing, and cross-platform semantic vectors in CI.

No production Chat/Flutter schema, generated contract, release, tag, or public
API was created by this proof or contract record.
