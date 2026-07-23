import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:openmls/openmls.dart';
import 'package:test/test.dart';

void main() {
  test('bundled notice metadata matches the committed asset', () {
    const relativePath = 'assets/THIRD_PARTY_NOTICES.txt';
    final notice = File(relativePath);

    expect(notice.existsSync(), isTrue, reason: 'notice asset must be present');
    expect(openmlsThirdPartyNoticesAssetKey, 'packages/openmls/$relativePath');
    expect(
      sha256.convert(notice.readAsBytesSync()).toString(),
      openmlsThirdPartyNoticesSha256,
    );
  });

  test('bundled notice metadata matches the selected native release', () {
    final cargoToml = File('rust/Cargo.toml').readAsStringSync();
    final version = RegExp(
      r'^version\s*=\s*"([^"]+)"',
      multiLine: true,
    ).firstMatch(cargoToml);

    expect(version, isNotNull);
    expect(openmlsThirdPartyNoticesNativeVersion, version!.group(1));
  });
}
