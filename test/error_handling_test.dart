import 'dart:convert';
import 'dart:typed_data';

import 'package:openmls/openmls.dart';
import 'package:test/test.dart';

import 'test_helpers.dart';

void main() {
  late MlsEngine alice;
  late TestIdentity aliceId;

  setUpAll(() async {
    await Openmls.init();
  });

  setUp(() async {
    alice = await createTestEngine();
    aliceId = TestIdentity.create('alice');
  });

  group('error handling', () {
    test('process malformed message throws', () async {
      final groupResult = await alice.createGroup(
        config: defaultConfig(),
        signerBytes: aliceId.signerBytes,
        credentialIdentity: aliceId.credentialIdentity,
        signerPublicKey: aliceId.publicKey,
      );

      expect(
        () => alice.processMessage(
          groupIdBytes: groupResult.groupId,
          messageBytes: Uint8List.fromList([0, 1, 2, 3]),
        ),
        throwsA(isA<Object>()),
      );
    });

    test('load non-existent group throws', () async {
      final fakeGroupId = Uint8List.fromList(utf8.encode('no-such-group'));

      expect(
        () => alice.groupIsActive(groupIdBytes: fakeGroupId),
        throwsA(isA<Object>()),
      );
    });

    test('extract group ID from invalid bytes throws', () {
      expect(
        () => mlsMessageExtractGroupId(
          messageBytes: Uint8List.fromList([0xFF, 0xFF]),
        ),
        throwsA(isA<Object>()),
      );
    });

    test('extract epoch from invalid bytes throws', () {
      expect(
        () => mlsMessageExtractEpoch(
          messageBytes: Uint8List.fromList([0xFF, 0xFF]),
        ),
        throwsA(isA<Object>()),
      );
    });

    test('content type from invalid bytes throws', () {
      expect(
        () => mlsMessageContentType(
          messageBytes: Uint8List.fromList([0xFF, 0xFF]),
        ),
        throwsA(isA<Object>()),
      );
    });

    // Regression: a malformed message must make the routing parsers return an
    // error, not abort the process. Input found by fuzzing (259 bytes).
    test('malformed message throws instead of aborting', () {
      final crash = Uint8List.fromList(const [
        0, 1, 0, 4, 6, 236, 0, 2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 55, 55, 55,
        55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 54, 55, 55, 55, 55, 55, 55, 55,
        55, 55, 58, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55,
        55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 55, 51, 55, 55, 55,
        55, 55, 55, 48, 48, 51, 48, 53, 52, 49, 50, 54, 54, 56, 49, 57, 55, 54,
        54, 48, 57, 57, 48, 54, 64, 0, 0, 0, 0, 0, 0, 0, 0, 64, 64, 64, 64, 64,
        64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
        64, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 64, 64, 64,
        64, 64, 64, 0, 2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 54, 52, 52, 55, 64,
        64, 64, 64, 66, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
        64, 144, 0, 194, 0, 0, 0, 64, 48, 55, 55, 55, 55, 54, 55, 55, 55, 55,
        55, 55, 55, 55, 55, 5, 0, 62, 0, 5, 0, 0, 53, 55, 49, 51, 49, 5,
      ]);

      expect(
        () => mlsMessageExtractGroupId(messageBytes: crash),
        throwsA(isA<Object>()),
      );
      expect(
        () => mlsMessageExtractEpoch(messageBytes: crash),
        throwsA(isA<Object>()),
      );
      expect(
        () => mlsMessageContentType(messageBytes: crash),
        throwsA(isA<Object>()),
      );
    });
  });
}
