# openmls_dart

MIT-licensed Dart/Flutter bindings for [OpenMLS](https://github.com/openmls/openmls), focused on an operation-scoped, caller-owned storage boundary.

This fork does not open or own an application database. The caller supplies opaque OpenMLS rows for one operation and receives one complete mutation batch. That lets an application commit MLS state in the same transaction as its own encrypted database records.

## Status

The supported API is intentionally small:

- create a KeyPackage;
- create a group;
- add members and produce a Commit/Welcome;
- join from a Welcome;
- create and process application or handshake messages;
- delete one group’s state;
- inspect routing fields on an MLS protocol message.

The former `MlsEngine` API and its embedded SQLCipher/IndexedDB storage were removed in the 2.0 native ABI line. This is a breaking change. Applications must not decode or manufacture the opaque storage rows.

## Platforms

| | Android | iOS | macOS | Linux | Windows | Web |
|---|---|---|---|---|---|---|
| **Support** | SDK 24+ | 13.0+ | 10.15+ | arm64, x64 | x64 | WASM |
| **Release targets** | arm64-v8a, armeabi-v7a, x86_64 | device arm64; simulator arm64, x86_64 | arm64, x86_64 | arm64, x86_64 | x86_64 | wasm32 |

Every native release must contain every target above. The build hook selects and verifies the matching archive automatically.

## Dependency

This fork is designed to be pinned to an exact reviewed Git commit:

```yaml
dependencies:
  openmls:
    git:
      url: https://github.com/mrtcaner/openmls_dart.git
      ref: <exact-reviewed-commit>
```

Do not follow a branch when persisted MLS state depends on the fork. The native archive version comes from `rust/Cargo.toml`; the build hook verifies its SHA256 before extraction.

## Initialization

```dart
import 'package:openmls/openmls.dart';

Future<void> main() async {
  await Openmls.init();
  // Call caller-owned storage operations here.
}
```

Flutter host tests resolve the library from Flutter’s generated `NativeAssetsManifest.json`; callers do not need to pass a build-directory path. `libraryPath` remains available for explicit development overrides.

## Caller-owned transaction boundary

Each operation follows the same rule:

1. Read the installation-global entries and the entries for the target group in one consistent snapshot.
2. Call exactly one `*WithStorage` function.
3. If it succeeds, atomically apply the entire returned `MlsStorageBatch` with the related application change.
4. If the application transaction fails, discard the batch and retry from a fresh snapshot.

`MlsStorageEntry.groupId == null` identifies installation-global OpenMLS state. Other rows belong to the given group. Preserve `mlsStorageFormatVersion()` exactly and reject unknown versions.

The public operations are:

- `createKeyPackageWithStorage()`
- `createGroupWithStorage()`
- `addMembersWithStorage()`
- `joinGroupFromWelcomeWithStorage()`
- `createMessageWithStorage()`
- `processMessageWithStorage()`
- `deleteGroupWithStorage()`

### Authentication boundaries

`addMembersWithStorage()` requires one expected Basic Credential identity for each KeyPackage and authenticates the supplied AAD in the resulting Commit. A mismatch returns no mutation batch.

`processMessageWithStorage()` requires caller-supplied expected AAD for application and handshake messages. It returns both `previousEpoch` and `resultingEpoch`; a processed Commit normally advances the latter.

An MLS Welcome has no equivalent application AAD field. Bind it to authenticated bootstrap metadata in the application protocol before calling `joinGroupFromWelcomeWithStorage()`.

### Ordering and rejection

Commits are epoch transitions and must be processed in order. If a required Commit is permanently unavailable, terminate that group incarnation and bootstrap a new one; later application ciphertext cannot repair the missing epoch state.

Creating an application message advances the sender ratchet in the returned batch. If the application commits that batch but later rejects the ciphertext, a receiver may skip the missing generation only within `senderRatchetMaxForwardDistance` and related group limits. Test the configured boundary in the consuming application.

## Build and test

Use the Makefile as the single entry point:

```bash
make setup
make codegen
make build
make rust-check
make rust-clippy
make test
make flutter-test ARGS="test/openmls_test.dart"
make analyze ARGS="--fatal-infos"
make build-android
make build-ios
make build-web
```

The release workflow builds Android, iOS, macOS, Linux, Windows, and Web archives. Each archive contains `THIRD_PARTY_NOTICES.txt`, generated deterministically from the locked Cargo resolution, and is covered by checksums and GitHub artifact provenance.

The same verified inventory ships as the Flutter package asset identified by
`openmlsThirdPartyNoticesAssetKey`. Consumers can load it through their existing
asset bundle and use `openmlsThirdPartyNoticesNativeVersion` and
`openmlsThirdPartyNoticesSha256` as a fail-closed release check. The package does
not import Flutter or register UI entries itself.

Run `make verify-third-party-notices` after changing the locked Rust dependency
graph or the native crate version. Future release archives copy this committed,
verified asset rather than generating a separate notice file.

The inventory covers every resolved Cargo package. For 42 packages in the
current 2.0.0 graph, no package-local license or notice text was available.
Those entries are identified explicitly in the file and still require license
review; packaging the inventory does not turn them into complete text coverage.

## Versioning

Three identities move independently:

- `rust/Cargo.toml` — native FRB ABI and archive version;
- the pinned OpenMLS git tag — protocol implementation version;
- `pubspec.yaml` — Dart package version.

Removing `MlsEngine` changes the Dart/FRB surface and therefore requires a new major native ABI release. Consumers should pin the exact commit that selects that release.

## Security

The caller owns encryption at rest, database serialization, rollback protection, backups, and atomic application of mutation batches. Opaque MLS entries can contain secret state; never log or inspect them. See [SECURITY.md](SECURITY.md) for the complete boundary and reporting policy.

## License

MIT. OpenMLS and transitive dependencies retain their own licenses; release archives include their notices.
