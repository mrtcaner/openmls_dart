# openmls - Claude Code Configuration

## Important Rules

**ALWAYS use Makefile commands.** Never call scripts or cargo directly. The Makefile is the single entry point for all operations.

**NEVER work directly on `main`.** Code, dependency, automation,
configuration, API, storage, security, and release changes start with a public
GitHub issue, continue on a non-`main` branch, and reach `main` through a pull
request. Documentation-only corrections may skip the issue, but still require a
branch and pull request. The live ruleset has no administrator bypass.

```bash
# Correct - pass arguments via ARGS variable
make build ARGS="--target aarch64-apple-darwin"
make codegen
make test
make analyze ARGS="--fatal-infos"

# Wrong - never do this
cargo build --release
flutter_rust_bridge_codegen generate
make build --target aarch64-apple-darwin  # make interprets --target as its own flag!
```

## Available Makefile Commands

### Setup
```bash
make setup                        # Full setup (FVM + Rust tools)
make setup-fvm                    # Install FVM + Flutter only
make setup-rust-tools             # Install Rust tools (cargo-audit, frb_codegen)
make setup-web                    # Install web build tools (wasm-pack)
make setup-android                # Install Android build tools (cargo-ndk)
make setup-mobile-rust-targets    # Install all Android and iOS Rust targets
```

### Code Generation
```bash
make codegen                      # Generate Dart bindings from Rust code
```

**Note:** `make codegen` automatically creates a `.skip_openmls_hook` marker file to prevent Build Hooks from downloading libraries during codegen. The marker is automatically removed after completion.

### Build
```bash
make build                              # Build for current platform (always release)
make build ARGS="--target <target>"     # Build for specific Rust target
make build-android                      # Build for Android (all ABIs)
make build-android ARGS="--target arm64-v8a"  # Build for specific Android ABI
make build-ios                              # Build iOS device arm64
make build-ios IOS_RUST_TARGET=aarch64-apple-ios-sim  # iOS simulator arm64
make build-web                          # Build WASM for web
make build-example-web                  # Build WASM + Flutter Web example
```

### Rust Quality
```bash
make rust-check                   # Check Rust code compiles
make rust-test                    # Run Rust unit tests
make rust-clippy                  # Lint Rust code with clippy (warnings = errors)
make rust-audit                   # Audit Rust dependencies for vulnerabilities
make rust-deny                    # Check advisories/licenses/sources (cargo-deny)
```

### Fuzzing
```bash
make setup-fuzz                   # One-time: install nightly + cargo-fuzz
make fuzz-list                    # List available fuzz targets
make fuzz-seed                    # Generate seed corpus (extend rust/fuzz/examples/gen_corpus.rs)
make fuzz ARGS="mls_message -- -max_total_time=60"  # Run a fuzz target
```

### Dart Quality
```bash
make test                                # Run all tests
make test ARGS="test/example_test.dart"  # Run specific test file
make coverage                            # Run tests with coverage report
make analyze                             # Run static analysis
make analyze ARGS="--fatal-infos"        # Strict analysis
make format                              # Format Dart code
make format-check                        # Check formatting without changes
make doc                                 # Generate documentation
```

### Utilities
```bash
make get                          # Get dependencies
make clean                        # Clean build artifacts (including rust/target)
make version                      # Show current crate version
make rust-update                  # Update Cargo.lock
make check-new-openmls-version  # Check for new upstream openmls version
make check-new-openmls-version ARGS="--update"  # Apply update
make check-template-updates       # Check for copier template updates
make check-targets                # Check deployment targets (iOS/macOS/Android)
make check-targets ARGS="--ios --set 14.0"  # Set iOS target everywhere
make update-changelog ARGS="--version vX.Y.Z"  # Update CHANGELOG with AI
make help                         # Show all available commands
```

## Project Overview

Dart Flutter Rust Bridge wrapper for openmls.

### Key Features
- Flutter Rust Bridge integration for type-safe FFI
- Pre-built native libraries for all platforms
- Automated builds via GitHub Actions
- Web/WASM support

### Upstream Repository
- **openmls**: https://github.com/openmls/openmls

## Project Structure

```
openmls/
├── lib/                            # Dart library code
│   └── src/rust/                   # FRB-generated Dart bindings
├── rust/                           # Rust crate
│   ├── Cargo.toml                  # Rust dependencies + version
│   └── src/
│       ├── lib.rs                  # Crate entry point
│       ├── frb_generated.rs        # FRB-generated Rust code
│       └── api/                    # Your Rust API modules
├── scripts/                        # Utility scripts (use via Makefile!)
├── hook/                           # Dart Build Hook for library download
├── test/                           # Tests
├── Makefile                        # Entry point for all commands
├── pubspec.yaml                    # Package config
├── flutter_rust_bridge.yaml        # FRB configuration
└── .github/workflows/              # CI/CD workflows
```

## Development Workflow

### 1. Implement Rust API

Add your Rust functions in `rust/src/api/`:

```rust
// rust/src/api/greeting.rs
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
```

Register the module in `rust/src/api/mod.rs`:

```rust
pub mod greeting;
```

### 2. Generate Dart Bindings

```bash
make codegen
```

This generates Dart code in `lib/src/rust/`.

### 3. Build Native Library

```bash
# For current platform
make build

# For specific target
make build ARGS="--target aarch64-apple-darwin"
```

### 4. Run Tests

```bash
make test
```

## Release Flow (two stages)

Releasing is **two independent stages**, each with its own command and tag. The
`openmls_frb` native crate (`rust/Cargo.toml` version) and the `openmls`
Dart package (`pubspec.yaml` version) are versioned and released separately.

- **Automated openmls update PRs do NOT bump the `openmls_frb`
  crate version and do NOT build binaries** — they only update the dependency +
  CHANGELOG. Updates accumulate on `main` (CI builds from source and tests them).
- **The native build is triggered by pushing a `openmls_frb-<version>` tag**
  (created by `make release-frb`), not by pushing to `main`. The tag must equal the
  `rust/Cargo.toml` crate version (the workflow validates this).
- **Stage 1 must finish before stage 2** — the published Dart package's build hook
  downloads the precompiled `openmls_frb-<crate>` binary, so it must already
  exist before you tag the pub.dev release.

### Stage 1 — release the native crate

```bash
# From a clean, up-to-date main. You enter your signing passphrase during the
# command (commit + tag are signed; the terminal is inherited).
make release-frb ARGS="--version X.Y.Z"            # bump + commit + tag + push
make release-frb ARGS="--version X.Y.Z --no-push"  # local only
```

Bumps `rust/Cargo.toml`, stamps the CHANGELOG highlight, signs a commit + tag
`openmls_frb-X.Y.Z`, and pushes — which triggers the native build workflow.
Choose `X.Y.Z` by SemVer of the FFI surface (a non-empty `lib/src/rust/` codegen
diff since the last frb release means the wire signature moved).

### Stage 2 — release the Dart package

```bash
# After the stage-1 native build has finished. Same interactive signing flow.
make release ARGS="--version X.Y.Z"   # verify frb binary + bump + finalize
                                      # CHANGELOG + dry-run + signed commit/tag/push
```

Verifies the stage-1 `openmls_frb-<crate>` release exists, runs
`make publish-dry-run` (on the clean, pre-bump tree), bumps `pubspec.yaml`,
finalizes the CHANGELOG (`[Unreleased]` → `[X.Y.Z]` + compare links; no empty
`[Unreleased]` is left behind), then signs a commit + tag `vX.Y.Z` and pushes —
`publish.yml` publishes to pub.dev.

The live fork's **Protect main branch** ruleset has been active since
2026-07-23 with no bypass actors: all `main` updates require a pull request, and
direct pushes, force pushes, and deletion are blocked. The other committed
rulesets and `native-build` environment protections have not yet been verified
as live. Do not claim the release path is fully gated until those settings are
applied and verified. See `.github/rulesets/README.md`.

## Native Library Version

Three versions move independently:

- `rust/Cargo.toml` `[package].version` is the `openmls_frb` native bridge and
  release asset version.
- OpenMLS dependency git tags are pinned in `rust/Cargo.toml`.
- `pubspec.yaml` `version` is the Dart package version.

There is no `native_version` field in `pubspec.yaml`.

To check/update the version:
```bash
make check-new-openmls-version              # Check for updates
make check-new-openmls-version ARGS="--update"  # Apply update
make rust-update                    # Update Cargo.lock after version bump
make update-changelog ARGS="--version openmls-vX.Y.Z"  # Generate AI changelog entry
```

### AI-Powered Changelog

The `update-changelog` command uses GitHub Models API to analyze release notes and generate changelog entries. Requires `AI_MODELS_TOKEN` environment variable:

```bash
# Get token from https://github.com/settings/tokens (Models → Read only)
AI_MODELS_TOKEN=xxx make update-changelog ARGS="--version v1.0.0"
```

## Supported Platforms

| Platform | Rust Target | Library |
|----------|------------|---------|
| Linux x86_64 | x86_64-unknown-linux-gnu | libopenmls_frb.so |
| Linux arm64 | aarch64-unknown-linux-gnu | libopenmls_frb.so |
| macOS arm64 | aarch64-apple-darwin | libopenmls_frb.dylib |
| macOS x86_64 | x86_64-apple-darwin | libopenmls_frb.dylib |
| Windows x86_64 | x86_64-pc-windows-msvc | openmls_frb.dll |
| iOS device | aarch64-apple-ios | libopenmls_frb.dylib |
| iOS simulator arm64 | aarch64-apple-ios-sim | libopenmls_frb.dylib |
| iOS simulator x86_64 | x86_64-apple-ios | libopenmls_frb.dylib |
| Android arm64 | aarch64-linux-android | libopenmls_frb.so |
| Android arm32 | armv7-linux-androideabi | libopenmls_frb.so |
| Android x86_64 | x86_64-linux-android | libopenmls_frb.so |
| Web (WASM) | wasm32-unknown-unknown | openmls_frb.wasm |


## Security Considerations

> **Important:** See [SECURITY.md](SECURITY.md) for full security policy and best practices.

### Supply Chain Security
- All native libraries are built from source in GitHub Actions
- SHA256 checksums verify downloaded libraries
- Pin to specific upstream releases

### Code Review Checklist
1. No hardcoded keys or secrets
2. Memory properly freed after use
3. Sensitive data zeroed before freeing
4. No timing side-channels

## Storage Architecture

This fork exposes one storage authority: the caller. The removed `MlsEngine`
and embedded SQLCipher/IndexedDB implementation must not be reintroduced without
an explicit, reviewed architecture decision and a new breaking ABI release.

### How it works

```
1. caller entries        → Vec<(key, value)> → HashMap (initial + current clone)
2. OpenMLS operates      → reads/writes on `current` HashMap
3. into_updates(provider)→ diff(initial, current) → complete mutation batch
4. Drop                  → zeroize() all values in both HashMaps → memory freed
```

No MLS snapshot is retained by Rust between calls.

### Key files

| File | Purpose |
|------|---------|
| `rust/src/snapshot_storage.rs` | SnapshotStorageProvider (HashMap-based StorageProvider impl) |
| `rust/src/api/storage.rs` | Caller-owned operation boundary (opaque entries → mutation batch) |
| `rust/src/api/support.rs` | Internal parsing/credential/group-loading helpers |
| `rust/src/api/message.rs` | Standalone protocol-message routing helpers |

### Scalability

The ratchet tree is the only entry scaling with members (~500 bytes per member). A 10,000-member group = ~10 MB peak memory during a single operation. MLS protocol itself (O(N) commit processing) is the bottleneck, not our storage pattern. For groups >50K members, MLS RFC recommends fan-out (subgroups).

### Security properties

- Plaintext in memory only during single-digit milliseconds per operation
- Both HashMaps zeroized on Drop (`snapshot_storage.rs`)
- Security profile identical to Wire's direct-DB approach (both must hold plaintext while OpenMLS operates)

### Why an opaque snapshot boundary

1. It lets the application keep one encrypted database authority.
2. It decouples the application schema from OpenMLS internals.
3. It makes the application transaction the atomic commit/rollback boundary.
4. It avoids a second database engine and its native dependencies.

### Caller-owned storage

Functions in `rust/src/api/storage.rs` reconstruct a provider from caller-owned
opaque entries for one operation and return one complete mutation batch. They do
not open a database or retain state. The host owns encryption at rest, serialized
writes, atomic apply/discard, rollback, and backup policy. Keep this surface
operation-scoped and do not decode or manufacture its opaque rows.

Before extending the API, add an explicit operation and tests instead of
attempting to emulate missing behavior by editing storage entries. A storage
format change requires a reviewed version bump and migration contract for
callers; there is no fork-owned database migration layer.

## FVM (Flutter Version Management)

This project uses FVM for consistent Flutter/Dart versions.

**Version:** Flutter 3.44.6

FVM is automatically installed by `make setup`.

## Windows Users

On Windows, install `make` first:
- Chocolatey: `choco install make`
- Scoop: `scoop install make`
- Or use Git Bash / WSL

## Changelog Format

Each release is a `## [X.Y.Z] - YYYY-MM-DD` heading split into **audience-scoped**
sections. Keep this structure so entries stay consistent across releases.

```markdown
## [X.Y.Z] - YYYY-MM-DD

### For Users

#### ✨ Highlights

- **<headline>** — short description (mark breaking ones **(breaking)**)
- **openmls vX.Y.Z** — ... (state "unchanged this release" if it didn't move)
- **openmls_frb vX.Y.Z** — Rust FFI bindings

#### Changed (Breaking)

- **<summary>** — what broke. Include an **Action required:** note.

#### Changed

- **<summary>** — non-breaking behavior/API change

#### Security

- **<summary>** — security-relevant, user-observable change

#### Fixed

- **<summary>** — bug fix

### For Contributors

#### Added

- **<summary>** — internal tooling only (fuzzing, cargo-deny, scripts, …)

#### Changed

- **<summary>** — CI / lints / build config / template adoption
```

Rules:
- **`### For Users`** = anything a consumer of the published package can observe
  (public API, runtime behavior, the native binary, the build hook). A change is
  "For Users" even if it feels internal when a consumer sees it at build/run time
  (e.g. `overflow-checks` in the shipped binary).
- **`### For Contributors`** = changes that do NOT affect the published package's
  behavior (CI, dev tooling, lints, fuzzing, cargo-deny, build scripts, template
  adoption).
- Every bullet starts with a **bold summary** + em-dash, then the detail.
- Omit any section/subsection with no entries. Order subsections as shown
  (Highlights → Changed (Breaking) → Changed → Security → Fixed).
- Released sections are immutable; edit the top pending version until release.

## Publishing Checklist

```bash
# 1. Run quality checks
make analyze
make test
make format-check

# 2. Update version in pubspec.yaml
# 3. Update CHANGELOG.md

# 4. Dry run
make publish-dry-run

# 5. Create annotated tag and push (CI will publish)
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

## Claude Skills

Claude Code skills available in this project (invoke with `/<skill>` or used automatically by Claude):

| Skill | Description |
|-------|-------------|
| `add-db-migration` | Add a new database migration to EncryptedDb (schema/data format changes) |
| `release-package` | Prepare a new version for publication to pub.dev |
| `update-template` | Update copier template to latest version |
