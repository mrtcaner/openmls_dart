# Security

## Scope

This package is a Flutter Rust Bridge wrapper around OpenMLS. The fork is responsible for its FFI surface, operation sequencing, caller-storage boundary, native delivery hook, and release supply chain. OpenMLS and RustCrypto remain responsible for the underlying MLS and cryptographic implementations.

Physical side channels, fault injection, hardware flaws, a compromised host, and reliable erasure from Dart’s garbage-collected heap are outside this wrapper’s guarantees.

## Storage authority

The fork has one storage model: the application owns persistence.

The `*WithStorage` functions reconstruct temporary OpenMLS state from caller-supplied opaque entries, perform one operation, and return a complete `MlsStorageBatch`. They do not open a database or retain a second durable copy. The former `MlsEngine`, SQLCipher, IndexedDB, and WebCrypto storage implementation are not part of the lean ABI.

The caller must:

- encrypt MLS entries at rest and protect its database key;
- serialize operations for the same installation/group;
- read from a consistent snapshot;
- atomically apply the complete batch with the related application record, or discard both;
- retain installation-global rows (`groupId == null`) and group rows without editing their bytes;
- preserve and validate `mlsStorageFormatVersion()`;
- protect against database rollback and stale backup restoration;
- never decode, manufacture, or log opaque keys or values.

Snapshot values are zeroized in Rust when the temporary provider is dropped. Values also cross the FFI boundary, where Dart may copy them; Dart-side zeroization is defence in depth, not a guarantee.

## Authenticated context and identity

`addMembersWithStorage()` validates every supplied KeyPackage and requires its Basic Credential identity to match the corresponding trusted expected identity. It also authenticates caller-supplied AAD in the generated Commit.

`processMessageWithStorage()` compares the message AAD with caller-supplied expected AAD before returning state changes. Derive expected identities and AAD from authenticated application/backend context, not untrusted client metadata.

An MLS Welcome does not carry the same application AAD field. The application must bind a Welcome to authenticated bootstrap metadata—conversation identity, group incarnation, intended recipient/installation, and the related accepted membership transition—before joining.

These checks establish cryptographic binding; they do not replace application authorization or abuse policy.

## Ordering and recovery

MLS Commits are ordered epoch transitions. Process them in order. If a required Commit is unavailable after the application’s replay/retention window, later ciphertext cannot reconstruct the missing epoch. Terminate that group incarnation and bootstrap a new one.

`createMessageWithStorage()` advances sender state in its returned batch. If the caller commits that batch but terminally rejects the ciphertext, later messages can only bridge the generation gap within the group’s sender-ratchet limits. Consumers must test both the configured boundary and the first failure beyond it.

Do not replay successful mutation batches, process the same message twice, or restore stale MLS rows independently of related application state.

## Key handling

Never log signer bytes, private keys, storage values, plaintext, or derived secrets. Keep Dart-side copies short-lived. `SecureBytes` and `zeroize()` can reduce exposure but cannot prevent garbage-collector copies.

The APIs returning highest-risk material include:

- `MlsSignatureKeyPair.privateKey()`;
- `serializeSigner()`;
- `MlsStorageEntry.value` and every mutation batch.

X.509 construction/deserialization does not validate certificate chains, expiration, revocation, or trust anchors. Applications must perform that validation before trusting such credentials.

## Native delivery

The build hook downloads the exact `openmls_frb-<version>` archive selected by `rust/Cargo.toml`. It verifies the archive against the release checksum and fails closed when a checksum cannot be obtained. `OPENMLS_ALLOW_UNVERIFIED_DOWNLOAD=1` is only a legacy escape hatch and must not be used for production builds.

Every release archive:

- is built by the all-platform GitHub Actions matrix;
- has a SHA256 entry;
- is covered by GitHub Artifact Attestations/Sigstore provenance;
- contains deterministic `THIRD_PARTY_NOTICES.txt` attribution.

Verify an archive with:

```bash
gh attestation verify openmls_frb-<version>-<platform>.tar.gz \
  --repo mrtcaner/openmls_dart
```

The repository contains ruleset/environment definitions, but live GitHub protection must be verified rather than assumed. See [`.github/rulesets/README.md`](.github/rulesets/README.md).

## Build hardening

- Release builds use LTO, size optimization, stripping, and overflow checks for hand-written wrapper code.
- Hand-written Rust denies unsafe code; generated FRB code has the explicit exception.
- `make rust-audit` and `make rust-deny` check advisories, licenses, and sources.
- `make rust-clippy` treats warnings as errors.
- Untrusted wire parsers have cargo-fuzz targets under `rust/fuzz/`.

## Web

The Web build exposes the same caller-owned storage API through WASM. This fork does not encrypt IndexedDB or persist MLS state in the browser. The Web application owns storage encryption and must also use HTTPS, a strict Content Security Policy, and minimal trusted scripts to reduce XSS exposure.

## Review checklist

- No secrets, storage values, plaintext, or keys in logs/errors.
- Same-group operations are serialized.
- Batches and related application records commit atomically.
- Expected AAD and credential identities come from trusted context.
- Commit replay/recovery and group-incarnation replacement are defined.
- Terminally rejected sends are tested at sender-ratchet limits.
- Exact fork commit and native archive version are pinned.
- All six platform families remain in the release matrix.
- Release checksums, provenance, and notices are present.

## Reporting

Do not open a public issue for a vulnerability. Use [GitHub private vulnerability reporting](https://github.com/mrtcaner/openmls_dart/security/advisories/new) with reproduction details, affected versions/commits, platform, and impact.

For defects in OpenMLS itself, also consult the [upstream OpenMLS repository](https://github.com/openmls/openmls).
