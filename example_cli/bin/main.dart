import 'dart:convert';

import 'package:openmls/openmls.dart';

Future<void> main() async {
  await Openmls.init();
  const suite = MlsCiphersuite.mls128DhkemX25519Aes128GcmSha256Ed25519;
  final keyPair = MlsSignatureKeyPair.generate(ciphersuite: suite);
  final signer = serializeSigner(
    ciphersuite: suite,
    privateKey: keyPair.privateKey(),
    publicKey: keyPair.publicKey(),
  );
  final result = await createKeyPackageWithStorage(
    ciphersuite: suite,
    signerBytes: signer,
    credentialIdentity: utf8.encode('cli-example-installation'),
    signerPublicKey: keyPair.publicKey(),
    storageEntries: const [],
    storageFormatVersion: mlsStorageFormatVersion(),
  );

  print('KeyPackage bytes: ${result.keyPackageBytes.length}');
  print('Opaque mutations to commit: ${result.storageBatch.upserts.length}');
  Openmls.cleanup(dispose: true);
}
