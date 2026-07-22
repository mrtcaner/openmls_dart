# openmls - MLS Protocol for Dart

[![pub package](https://img.shields.io/pub/v/openmls.svg)](https://pub.dev/packages/openmls)
[![CI](https://github.com/mrtcaner/openmls_dart/actions/workflows/test.yml/badge.svg)](https://github.com/mrtcaner/openmls_dart/actions/workflows/test.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Dart](https://img.shields.io/badge/dart-%3E%3D3.10.0-brightgreen.svg)](https://dart.dev)
[![Flutter](https://img.shields.io/badge/flutter-%3E%3D3.38.0-blue.svg)](https://flutter.dev)
[![openmls](https://img.shields.io/badge/openmls-v0.8.1-orange.svg)](https://github.com/openmls/openmls)
[![native bridge](https://img.shields.io/badge/openmls__frb-1.5.4-purple.svg)](https://github.com/mrtcaner/openmls_dart/releases/tag/openmls_frb-1.5.4)
[![coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/mrtcaner/8ff33d8b0975fa90bc0e9249a70e5b17/raw/coverage.json)](https://github.com/mrtcaner/openmls_dart/actions/workflows/test.yml)

Dart bindings for [OpenMLS](https://github.com/openmls/openmls), providing a Rust implementation of the Messaging Layer Security (MLS) protocol ([RFC 9420](https://www.rfc-editor.org/rfc/rfc9420.html)) for secure group messaging.

## Platform Support

|             | Android | iOS   | macOS  | Linux      | Windows | Web |
|-------------|---------|-------|--------|------------|---------|-----|
| **Support** | SDK 24+ | 13.0+ | 10.15+ | arm64, x64 | x64     | WASM |
| **Arch**    | arm64, armv7, x64 | device arm64; simulator arm64, x64 | arm64, x64 | arm64, x64 | x64 | wasm32 |

## Features

- **MLS Protocol (RFC 9420)**: Secure group messaging with forward secrecy and post-compromise security
- **Group Key Agreement**: Efficient tree-based group key agreement (TreeKEM)
- **Two Storage Modes**: Rust-owned encrypted storage through `MlsEngine`, or
  operation-scoped caller-owned state for integration with an existing
  transaction authority
- **Basic & X.509 Credentials**: Support for both credential types in the
  `MlsEngine` API; caller-owned member admission currently validates Basic
  Credential identities
- **Flutter & CLI Support**: Works with Flutter apps and standalone Dart CLI applications
- **Automatic Builds**: Native libraries downloaded automatically via build hooks
- **High Performance**: Direct Rust integration via Flutter Rust Bridge

## Implementation Status

| Category | Status | Description |
|----------|:------:|-------------|
| Group Lifecycle | Done | Create, join (Welcome, external commit), leave, inspect |
| Member Management | Done | Add, remove, swap members |
| Messaging | Done | Encrypt/decrypt application messages with AAD |
| Proposals | Done | Add, remove, self-update, PSK, custom, group context extensions |
| Commits | Done | Pending proposals, flexible commit, merge/clear |
| Key Packages | Done | Create with options (lifetime, last-resort) |
| Credentials | Done | Basic and X.509 credential types |
| State Queries | Done | Members, epoch, extensions, ratchet tree, group info, PSK export |
| Rust-owned storage | Done | `MlsEngine` with SQLCipher on native and Web Crypto on WASM |
| Caller-owned storage | Done | Versioned opaque entries and atomic mutation batches; the host owns encryption and transactions |

<details>
<summary>API overview</summary>

**Key Packages**: `createKeyPackage`, `createKeyPackageWithOptions`

**Group Lifecycle**: `createGroup`, `createGroupWithBuilder`, `joinGroupFromWelcome`, `joinGroupFromWelcomeWithOptions`, `inspectWelcome`, `joinGroupExternalCommit`, `joinGroupExternalCommitV2`

**State Queries**: `groupId`, `groupEpoch`, `groupIsActive`, `groupMembers`, `groupCiphersuite`, `groupOwnIndex`, `groupCredential`, `groupExtensions`, `groupPendingProposals`, `groupHasPendingProposals`, `groupMemberAt`, `groupMemberLeafIndex`, `groupOwnLeafNode`, `groupConfirmationTag`, `exportRatchetTree`, `exportGroupInfo`, `exportSecret`, `exportGroupContext`, `getPastResumptionPsk`

**Mutations**: `addMembers`, `addMembersWithoutUpdate`, `removeMembers`, `selfUpdate`, `selfUpdateWithNewSigner`, `swapMembers`, `leaveGroup`, `leaveGroupViaSelfRemove`

**Proposals**: `proposeAdd`, `proposeRemove`, `proposeSelfUpdate`, `proposeExternalPsk`, `proposeGroupContextExtensions`, `proposeCustomProposal`, `proposeRemoveMemberByCredential`

**Commit/Merge**: `commitToPendingProposals`, `mergePendingCommit`, `clearPendingCommit`, `clearPendingProposals`, `setConfiguration`, `updateGroupContextExtensions`, `flexibleCommit`

**Messages**: `createMessage`, `processMessage`, `processMessageWithInspect`, `mlsMessageExtractGroupId`, `mlsMessageExtractEpoch`, `mlsMessageContentType`

**Caller-owned storage**: `mlsStorageFormatVersion`,
`createKeyPackageWithStorage`, `createGroupWithStorage`,
`addMembersWithStorage`, `joinGroupFromWelcomeWithStorage`,
`createMessageWithStorage`, `processMessageWithStorage`,
`deleteGroupWithStorage`, `MlsStorageEntry`, `MlsStorageBatch`

</details>

## Installation

The caller-owned storage API and its matching native bridge are currently
consumed from this fork at an exact reviewed commit:

```yaml
dependencies:
  openmls:
    git:
      url: https://github.com/mrtcaner/openmls_dart.git
      ref: 84b389d03fcefc2f1c0b209b9690950fdf7d4d11
```

Do not use an unpinned branch for an application that persists MLS state. The
pub.dev Dart package remains version `1.4.2`; the fork changes documented here
have not been published as a newer Dart package.

| Component | Pinned version |
|-----------|----------------|
| Dart package metadata | `1.4.2` |
| Native bridge and release assets | `openmls_frb-1.5.4` |
| Upstream OpenMLS | `openmls-v0.8.1` |

Native libraries are downloaded automatically during build via Dart build hooks.

**No Rust required** for end users - precompiled binaries are downloaded from GitHub Releases.
Downloads require a matching SHA256 entry from this fork's public release.

## Usage: Rust-owned encrypted storage

```dart
import 'dart:convert';
import 'dart:typed_data';
import 'package:openmls/openmls.dart';

void main() async {
  // Initialize the library
  await Openmls.init();

  // Create an MlsEngine with encrypted storage.
  // - Native: SQLCipher database at the given file path
  // - Web: IndexedDB with AES-256-GCM encryption via Web Crypto API
  // Use ":memory:" for ephemeral in-memory storage (testing).
  final encryptionKey = Uint8List(32); // 32-byte key — store in platform secure storage!
  final engine = await MlsEngine.create(
    dbPath: ':memory:',
    encryptionKey: encryptionKey,
  );

  // Generate signing key pair
  final ciphersuite = MlsCiphersuite.mls128DhkemX25519Aes128GcmSha256Ed25519;
  final keyPair = MlsSignatureKeyPair.generate(ciphersuite: ciphersuite);
  final signerBytes = serializeSigner(
    ciphersuite: ciphersuite,
    privateKey: keyPair.privateKey(),
    publicKey: keyPair.publicKey(),
  );

  // Create a group
  final config = MlsGroupConfig.defaultConfig(ciphersuite: ciphersuite);
  final group = await engine.createGroup(
    config: config,
    signerBytes: signerBytes,
    credentialIdentity: utf8.encode('alice'),
    signerPublicKey: keyPair.publicKey(),
  );
  print('Created group: ${group.groupId}');

  // Close engine (releases DB connection and encryption key resources)
  await engine.close();

  // Clean up FRB runtime (optional, for CLI apps exiting)
  Openmls.cleanup();
}
```

## Storage Modes

Choose one durable authority for an application. Do not combine both modes for
the same MLS state.

### `MlsEngine`: Rust-owned encrypted database

`MlsEngine` owns persistence and encrypts its database automatically:

| Platform | Backend | Encryption |
|----------|---------|------------|
| Native (iOS, Android, macOS, Linux, Windows) | SQLCipher | AES-256 full-database encryption |
| Web (WASM) | IndexedDB | AES-256-GCM per-value encryption via `crypto.subtle` |

```dart
// Create engine with a 32-byte encryption key.
// Store the key in platform secure storage (Keychain, Android Keystore, etc.)
final engine = await MlsEngine.create(
  dbPath: 'mls_data.db',    // file path on native, IDB name on web
  encryptionKey: myKey,       // 32-byte AES-256 key
);

// All operations go through the engine
final group = await engine.createGroup(...);
await engine.addMembers(...);

// Close the engine to release the DB connection and encryption key resources.
// After close, all operations fail with "MlsEngine is closed".
// Useful for screen lock / app background scenarios.
await engine.close();

// Re-create from secure storage on unlock
final engine2 = await MlsEngine.create(dbPath: 'mls_data.db', encryptionKey: myKey);
```

On WASM, the encryption key is imported as a **non-extractable `CryptoKey`** via the Web Crypto API. Raw key bytes are zeroized from WASM memory immediately after import.

### Caller-owned storage and transactions

The top-level `*WithStorage` functions do not open a database or retain a group
object. Each call accepts opaque `MlsStorageEntry` values and the current
`mlsStorageFormatVersion()`. A successful mutation returns one
`MlsStorageBatch`; the caller must atomically apply the entire batch with its
related application state, or discard both. A failed call returns no batch.

This mode is intended for applications that already have an encrypted database
and need MLS changes to share that database's transaction. The host is
responsible for encryption at rest, serialized writes, rollback, backup policy,
and avoiding logs or unnecessary copies of opaque values.

Release `openmls_frb-1.5.4` adds two receive/admission checks to this boundary:

- `processMessageWithStorage(expectedAad: ...)` rejects an authenticated AAD
  mismatch before returning a storage batch.
- `addMembersWithStorage(expectedCredentialIdentities: ...)` requires each
  validated KeyPackage to contain a Basic Credential with the corresponding
  expected identity. Count, credential-type, and identity mismatches fail before
  a member-add commit is created.

See [`test/external_storage_test.dart`](test/external_storage_test.dart) for a
complete create/add/join/message/commit flow that recreates the provider from
caller state for every operation and tests discard, mismatch, and retry.

The caller-owned boundary deliberately has a smaller surface than `MlsEngine`.
It currently does not expose member removal, member swapping, arbitrary
proposals, external commits, or explicit retirement of an abandoned KeyPackage.
Do not decode or manufacture opaque storage rows to simulate a missing
operation.

## Known Limitations

### Web: `flutter build web --wasm` (dart2wasm) is not supported

This package works with the standard `flutter build web` (dart2js) target. It does **not** currently work when the host app is compiled with `flutter build web --wasm` / `flutter run -d chrome --wasm` (dart2wasm). Calls to the Rust side fail with:

```
Type 'JSValue' is not a subtype of type 'List<dynamic>' in type cast
```

This is an upstream limitation in [`flutter_rust_bridge`](https://github.com/fzyzcjy/flutter_rust_bridge) — its generated Dart decoders rely on implicit JS-array casts that work on dart2js but fail under dart2wasm. The pattern is hardcoded in FRB's codegen templates, so it affects every FRB-based Dart package, not just this one. Tracking upstream: [flutter_rust_bridge#2575](https://github.com/fzyzcjy/flutter_rust_bridge/issues/2575).

| Command | Status |
|---------|--------|
| `flutter run -d chrome` | Works (dart2js) |
| `flutter build web` | Works (dart2js) |
| `flutter run -d chrome --wasm` | Not supported (dart2wasm) |
| `flutter build web --wasm` | Not supported (dart2wasm) |

The Rust core of openmls ships as a `.wasm` module in both modes — `--wasm` only changes what the *Dart* code compiles to. Crypto performance and functionality are equivalent.

## Building from Source

### For End Users

**No setup required!** Precompiled native libraries are downloaded automatically from GitHub Releases during `flutter build`.

### For Contributors / Source Builds

If you want to build from source (or precompiled binaries are not available):

- [Flutter](https://flutter.dev/) 3.38+
- [FVM](https://fvm.app/) (optional, for version management)
- **Rust toolchain** (1.88+):
  - [rustup](https://rustup.rs/) - Rust toolchain installer
  - `cargo` - Rust package manager (installed with rustup)

### Setup

```bash
# Clone the repository
git clone https://github.com/mrtcaner/openmls_dart.git
cd openmls_dart

# Install FVM and dependencies
make setup

# Generate Dart bindings
make codegen

# Build native library
make build

# Run tests
make test

# See all available commands
make help
```

### Developing Rust API

1. Add your Rust functions in `rust/src/api/`:

```rust
// rust/src/api/greeting.rs
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
```

2. Register the module in `rust/src/api/mod.rs`:

```rust
pub mod greeting;
```

3. Generate Dart bindings:

```bash
make codegen
```

4. Build and test:

```bash
make build
make test
```

### Building Native Libraries

Native libraries are pre-built and downloaded automatically via build hooks.
If you need to build them locally:

```bash
# Build for current platform
make build

# Build with specific target
make build ARGS="--target aarch64-apple-darwin"

# Build for Android
make build-android

# Build for Web (WASM)
make build-web
```

## CI / Version Management

```bash
# Check for new openmls versions
make check-new-openmls-version

# Check for new copier template versions
make check-template-updates

# Check deployment target consistency (iOS/macOS/Android)
make check-targets

# Update Cargo.lock dependencies
make rust-update

# Generate AI-powered changelog entry (requires AI_MODELS_TOKEN)
make update-changelog ARGS="--version v1.0.0"
```

The CI automatically checks for new openmls releases daily and creates PRs with:
- Updated `pubspec.yaml` and version badges
- Updated `Cargo.lock` (if successful)
- Regenerated FRB bindings (if successful)
- AI-generated CHANGELOG entry (if `AI_MODELS_TOKEN` secret is configured)

It also checks for copier template updates daily and creates notification PRs with changelog and update instructions.


## Architecture

```
┌─────────────────────────────────────────────────┐
│          OpenMLS (Rust crate)                    │  Core MLS implementation
├─────────────────────────────────────────────────┤
│  MlsEngine + EncryptedDb  OR  *WithStorage API  │  One selected storage mode
├─────────────────────────────────────────────────┤
│       rust/src/api/*.rs (Rust wrappers)         │  FRB-annotated functions
├─────────────────────────────────────────────────┤
│      lib/src/rust/*.dart (FRB generated)        │  Auto-generated Dart API
├─────────────────────────────────────────────────┤
│           Your Dart application code            │  Owns integration boundary
└─────────────────────────────────────────────────┘
```

## Security Notes

**Key Properties:**
- **MLS Protocol (RFC 9420)** - Standardized group key agreement with forward secrecy and post-compromise security
- **Rust Implementation** - All cryptographic operations run in Rust using OpenMLS with the RustCrypto backend
- **Encryption at Rest** - `MlsEngine` uses SQLCipher or Web Crypto; callers of
  `*WithStorage` must provide equivalent protection in their own durable store
- **Web Crypto on WASM** - Encryption key stored as non-extractable `CryptoKey` via `crypto.subtle` — raw bytes never persist in WASM memory
- **Memory Safety** - Rust's ownership model prevents memory-related vulnerabilities
- **No `unsafe` code** in the wrapper layer (except `Send + Sync` for `CryptoKey` on single-threaded WASM)

**Best Practices:**
- Keep the library updated to the latest version
- Store the 32-byte encryption key in platform secure storage (Keychain, Android Keystore, `flutter_secure_storage`)
- Never log or expose serialized key material (`signer.serialize()`, private keys)
- Use `SecureBytes.wrap()` or `.zeroize()` for sensitive data (serialized keys, shared secrets) — see [SECURITY.md](SECURITY.md)
- Process MLS messages in order to maintain group state consistency
- **Web deployment:** Enable strict CSP headers (`script-src 'self'`) and serve over HTTPS

See [SECURITY.md](SECURITY.md) for full security guidelines.

## Acknowledgements

This library would not be possible without [OpenMLS](https://github.com/openmls/openmls), which provides the underlying Rust implementation of the MLS protocol.

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting issues or pull requests.

For major changes, please open an issue first to discuss what you would like to change.

## Security

See [SECURITY.md](SECURITY.md) for security policy and reporting vulnerabilities.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Related Projects

- [OpenMLS](https://github.com/openmls/openmls) - The underlying Rust MLS library
- [RFC 9420](https://www.rfc-editor.org/rfc/rfc9420.html) - The Messaging Layer Security (MLS) Protocol
- [Flutter Rust Bridge](https://cjycode.com/flutter_rust_bridge/) - Dart/Flutter <-> Rust binding generator
