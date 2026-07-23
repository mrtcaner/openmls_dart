/// Dart wrapper for OpenMLS — a Rust implementation of the Messaging Layer Security (MLS) protocol (RFC 9420)
///
/// This is the main entry point for openmls.
///
/// ## Getting Started
///
/// Add this package to your `pubspec.yaml`:
///
/// ```yaml
/// dependencies:
///   openmls:
///     git:
///       url: https://github.com/mrtcaner/openmls_dart.git
///       ref: <exact-reviewed-commit>
/// ```
///
/// Pin an exact reviewed commit when persisted MLS state depends on this fork's
/// caller-owned storage format. See the repository README for version details.
///
/// Native libraries are downloaded automatically during build via Dart Build Hooks.
///
/// ## Usage
///
/// ```dart
/// import 'package:openmls/openmls.dart';
///
/// void main() async {
///   await Openmls.init();
///   // ... use openmls APIs
/// }
/// ```
///
/// ## Platform Support
///
/// - Linux (x86_64, arm64)
/// - macOS (arm64, x86_64)
/// - Windows (x86_64)
/// - Android (arm64-v8a, armeabi-v7a, x86_64)
/// - iOS (device arm64, simulator arm64/x86_64)
/// - Web (WASM)

library;

export 'src/openmls.dart';
export 'src/rust/api/config.dart';
export 'src/rust/api/credential.dart';
export 'src/rust/api/keys.dart';
export 'src/rust/api/message.dart';
export 'src/rust/api/storage.dart';
export 'src/rust/api/types.dart';
export 'src/security/secure_bytes.dart';
export 'src/security/secure_uint8list.dart';
export 'src/third_party_notices.dart';
