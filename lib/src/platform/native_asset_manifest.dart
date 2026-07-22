import 'dart:convert';

/// Resolve one absolute code-asset path from Flutter's test manifest.
///
/// A host `flutter test` manifest contains a single host target. Returning
/// null when more than one distinct path is present avoids guessing an ABI.
String? resolveFlutterTestNativeAssetPath(String manifestJson, String assetId) {
  try {
    final decoded = jsonDecode(manifestJson);
    if (decoded is! Map<String, dynamic>) return null;
    final nativeAssets = decoded['native-assets'];
    if (nativeAssets is! Map<String, dynamic>) return null;

    final paths = <String>{};
    for (final targetAssets in nativeAssets.values) {
      if (targetAssets is! Map<String, dynamic>) continue;
      final location = targetAssets[assetId];
      if (location is! List<dynamic> || location.length != 2) continue;
      if (location[0] != 'absolute' || location[1] is! String) continue;
      final path = location[1] as String;
      if (path.isNotEmpty) paths.add(path);
    }
    return paths.length == 1 ? paths.single : null;
  } on FormatException {
    return null;
  }
}
