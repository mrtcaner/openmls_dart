import 'dart:convert';

import 'package:openmls/src/platform/native_asset_manifest.dart';
import 'package:test/test.dart';

void main() {
  const assetId = 'package:openmls/openmls';

  test('resolves the one host-test absolute asset', () {
    final manifest = jsonEncode({
      'format-version': [1, 0, 0],
      'native-assets': {
        'macos_arm64': {
          assetId: ['absolute', '/tmp/libopenmls_frb.dylib'],
        },
      },
    });

    expect(
      resolveFlutterTestNativeAssetPath(manifest, assetId),
      '/tmp/libopenmls_frb.dylib',
    );
  });

  test('rejects unsupported and malformed locations', () {
    expect(resolveFlutterTestNativeAssetPath('{', assetId), isNull);
    expect(
      resolveFlutterTestNativeAssetPath(
        jsonEncode({
          'native-assets': {
            'macos_arm64': {
              assetId: ['system', 'openmls_frb'],
            },
          },
        }),
        assetId,
      ),
      isNull,
    );
  });

  test('does not guess when a manifest contains distinct target paths', () {
    final manifest = jsonEncode({
      'native-assets': {
        'macos_arm64': {
          assetId: ['absolute', '/tmp/arm64/libopenmls_frb.dylib'],
        },
        'macos_x64': {
          assetId: ['absolute', '/tmp/x64/libopenmls_frb.dylib'],
        },
      },
    });

    expect(resolveFlutterTestNativeAssetPath(manifest, assetId), isNull);
  });
}
