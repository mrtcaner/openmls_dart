/// IO-specific platform implementations for native platforms.
library;

import 'dart:io';
import 'dart:isolate';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import 'native_asset_manifest.dart';

/// Whether we're running on web.
const bool kIsWeb = false;

/// Get a unique identifier for the current isolate.
int getIsolateId() => Isolate.current.hashCode;

/// Try to load library via native assets build hook.
///
/// The build hook (hook/build.dart) places the library in predictable locations:
/// - Flutter host tests: `UNIT_TEST_ASSETS/NativeAssetsManifest.json`
/// - JIT mode (dart run): .dart_tool/lib/
/// - AOT mode (dart build cli): bundle/lib/ (relative to executable)
///
/// Note: DynamicLibrary.open(assetId) with 'package:' URIs doesn't work
/// in Dart - it tries to open the URI as a literal file path. We must
/// resolve the actual file path ourselves.
// ignore: avoid_unused_constructor_parameters
ExternalLibrary? tryLoadNativeAsset(String assetId) {
  // Flutter builds a host-native-assets manifest for `flutter test`, but the
  // FRB dynamic loader cannot resolve a package asset ID on its own. Resolve
  // the exact absolute path Flutter generated instead of requiring callers to
  // pass a build-directory path.
  final testAssetsDirectory = Platform.environment['UNIT_TEST_ASSETS'];
  if (Platform.environment['FLUTTER_TEST'] == 'true' &&
      testAssetsDirectory != null &&
      testAssetsDirectory.isNotEmpty) {
    final manifest = File('$testAssetsDirectory/NativeAssetsManifest.json');
    if (manifest.existsSync()) {
      try {
        final path = resolveFlutterTestNativeAssetPath(
          manifest.readAsStringSync(),
          assetId,
        );
        if (path != null && File(path).existsSync()) {
          return ExternalLibrary.open(path);
        }
      } catch (_) {}
    }
  }

  final libraryName = getLibraryName();

  // 1. Try JIT mode location: .dart_tool/lib/
  // In JIT mode, the build hook copies the library to .dart_tool/lib/
  final jitLibPath = '.dart_tool/lib/$libraryName';
  if (File(jitLibPath).existsSync()) {
    try {
      return ExternalLibrary.open(File(jitLibPath).absolute.path);
    } catch (_) {}
  }

  // 2. Try AOT mode location: ../lib/ relative to executable
  // In AOT mode (dart build cli), library is in bundle/lib/
  // coverage:ignore-start
  try {
    final executableDir = File(Platform.resolvedExecutable).parent.path;
    final aotLibPath = '$executableDir/../lib/$libraryName';
    if (File(aotLibPath).existsSync()) {
      return ExternalLibrary.open(File(aotLibPath).absolute.path);
    }
  } catch (_) {}
  // coverage:ignore-end

  return null;
}

/// Load library from a file path.
// coverage:ignore-start
ExternalLibrary openLibraryFromPath(String path) {
  return ExternalLibrary.open(path);
}
// coverage:ignore-end

/// Get the platform-specific library name.
String getLibraryName() {
  if (Platform.isMacOS) {
    return 'libopenmls_frb.dylib';
  }
  // coverage:ignore-start
  if (Platform.isLinux) {
    return 'libopenmls_frb.so';
  }
  if (Platform.isWindows) {
    return 'openmls_frb.dll';
  }
  if (Platform.isAndroid) {
    return 'libopenmls_frb.so';
  }
  if (Platform.isIOS) {
    return 'libopenmls_frb.dylib';
  }
  throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
  // coverage:ignore-end
}
