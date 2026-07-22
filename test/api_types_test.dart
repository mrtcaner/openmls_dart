import 'package:openmls/openmls.dart';
import 'package:openmls/src/rust/api/init.dart';
import 'package:test/test.dart';

void main() {
  setUpAll(Openmls.init);

  group('api/init', () {
    test('initialization state is visible to the bridge', () {
      expect(isOpenmlsInitialized(), isTrue);
    });
  });

  group('caller-storage value types', () {
    test('supported ciphersuites are explicit and stable', () {
      expect(
        supportedCiphersuites(),
        unorderedEquals(const [
          MlsCiphersuite.mls128DhkemX25519Aes128GcmSha256Ed25519,
          MlsCiphersuite.mls128DhkemX25519Chacha20Poly1305Sha256Ed25519,
          MlsCiphersuite.mls128DhkemP256Aes128GcmSha256P256,
        ]),
      );
    });

    test('group configuration has value equality', () {
      final first = MlsGroupConfig.defaultConfig(
        ciphersuite: MlsCiphersuite.mls128DhkemX25519Aes128GcmSha256Ed25519,
      );
      final second = MlsGroupConfig.defaultConfig(
        ciphersuite: MlsCiphersuite.mls128DhkemX25519Aes128GcmSha256Ed25519,
      );

      expect(first, second);
      expect(first.hashCode, second.hashCode);
    });
  });
}
