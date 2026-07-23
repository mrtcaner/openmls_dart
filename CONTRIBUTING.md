# Contributing

Contributions are welcome.

## Repository workflow

Every change must be made on a non-`main` branch and merged through a pull
request. The live GitHub ruleset blocks direct pushes, force pushes, and
deletion of `main` for every actor, including administrators.

Open a public GitHub issue before starting any change to code, dependencies,
automation, configuration, APIs, storage formats, security behavior, or release
layout. The issue records the reason and intended scope before implementation.
Documentation-only corrections may skip the issue, but they still require a
branch and pull request. Security vulnerabilities are the exception: report
them privately as described below.

## Setup

Prerequisites are Git, Dart 3.10+, Flutter through FVM, Rust 1.88+, and `make`.

```bash
git clone https://github.com/mrtcaner/openmls_dart.git
cd openmls_dart
make setup
```

Use the Makefile for every project operation. Do not invoke Cargo, code-generation scripts, or release scripts directly.

## Architecture

The fork has one persistence boundary:

```text
caller snapshot -> SnapshotStorageProvider -> one OpenMLS operation
                <- complete MlsStorageBatch <- diffed temporary state
```

Important files:

| Path | Purpose |
|---|---|
| `rust/src/api/storage.rs` | Public caller-owned operations |
| `rust/src/snapshot_storage.rs` | Temporary OpenMLS storage provider and diff |
| `rust/src/api/support.rs` | Internal parsing/credential/group helpers |
| `rust/src/api/message.rs` | Storage-free message routing helpers |
| `lib/src/platform/` | Native library loading, including Flutter host tests |
| `hook/build.dart` | Verified release download and CodeAsset registration |
| `.github/workflows/build-openmls.yml` | All-platform native release |

Do not reintroduce `MlsEngine`, an embedded database, or a second storage authority as an incidental feature. Do not decode or edit opaque storage entries outside the provider implementation.

## Development loop

```bash
make get
make codegen
make build
make rust-check
make rust-clippy
make test
make flutter-test ARGS="test/openmls_test.dart"
make analyze ARGS="--fatal-infos"
make format-check
```

Generated FRB files under `lib/src/rust/` and `rust/src/frb_generated.rs` must be committed whenever `make codegen` changes them.

Tests for caller storage should verify both successful batches and fail-without-batch behavior. Security-sensitive cases include:

- unknown storage-format versions and duplicate keys;
- expected Basic Credential identity mismatches;
- application and Commit AAD mismatches;
- pre-merge versus resulting epoch reporting;
- discarded mutation batches;
- a committed-but-undelivered sender generation at and beyond configured limits;
- ordered Commit processing and unrecoverable missing epochs.

## Platform and delivery changes

The supported release is one complete matrix: Android, iOS, macOS, Linux, Windows, and Web. A change is not release-ready if only the developer’s host/mobile targets work.

For build-hook changes, test:

- a clean build and a cached build;
- no `File modified during build. Build must be rerun.` warning;
- automatic `flutter test` native loading without `libraryPath`;
- iOS device versus simulator cache identity;
- checksum failure behavior;
- Web version refresh behavior.

Run `make third-party-notices ARGS="--output <path>"` twice and compare the files when changing Cargo resolution or attribution logic.

## Pull requests

Keep each pull request narrow. Include:

- the public issue/decision it implements, or identify it as an exempt
  documentation-only correction;
- compatibility and security reasoning;
- exact commands run and their results;
- relevant before/after binary or app size measurements;
- confirmation that unrelated consumer repositories were not modified.

Breaking Dart/FRB changes require a major native ABI version. Releases use signed commits/tags and immutable GitHub assets; never overwrite an existing release to repair it.

## Security

Do not put secrets, signer bytes, opaque MLS state, plaintext, or keys in tests/logs. Report vulnerabilities through [GitHub private vulnerability reporting](https://github.com/mrtcaner/openmls_dart/security/advisories/new), not a public issue.
