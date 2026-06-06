import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:openmls/openmls.dart';

import '../utils.dart';

Uint8List _testKey() {
  final rng = Random.secure();
  return Uint8List.fromList(List.generate(32, (_) => rng.nextInt(256)));
}

/// Demonstrates the experimental X-Wing post-quantum ciphersuite
/// (hybrid ML-KEM-768 + X25519), preceded by a classical-suite regression
/// check to show the hybrid crypto provider leaves existing suites unchanged.
Future<void> runPostQuantumDemo() async {
  printHeader('Post-Quantum (X-Wing) Demo');

  // 1. Classical suite still works end-to-end with the hybrid provider.
  await _lifecycle(
    MlsCiphersuite.mls128DhkemX25519Aes128GcmSha256Ed25519,
    'classical',
    1,
  );

  // 2. The X-Wing post-quantum lifecycle.
  await _lifecycle(
    MlsCiphersuite.mls256XwingChacha20Poly1305Sha256Ed25519,
    'X-Wing',
    2,
  );

  print('');
  print('Both lifecycles completed — X-Wing verified on this platform.');
  print('');
  print('Note: X-Wing is experimental (no IANA codepoint, OpenMLS-only');
  print('interop). See the README "Post-Quantum Support" section.');
}

Future<void> _lifecycle(MlsCiphersuite cs, String label, int step) async {
  final config = MlsGroupConfig.defaultConfig(ciphersuite: cs);

  final aliceClient = await MlsEngine.create(
    dbPath: ':memory:',
    encryptionKey: _testKey(),
  );
  final aliceKeyPair = MlsSignatureKeyPair.generate(ciphersuite: cs);
  final aliceSigner = serializeSigner(
    ciphersuite: cs,
    privateKey: aliceKeyPair.privateKey(),
    publicKey: aliceKeyPair.publicKey(),
  );

  final bobClient = await MlsEngine.create(
    dbPath: ':memory:',
    encryptionKey: _testKey(),
  );
  final bobKeyPair = MlsSignatureKeyPair.generate(ciphersuite: cs);
  final bobSigner = serializeSigner(
    ciphersuite: cs,
    privateKey: bobKeyPair.privateKey(),
    publicKey: bobKeyPair.publicKey(),
  );

  // Alice creates the group.
  final group = await aliceClient.createGroup(
    config: config,
    signerBytes: aliceSigner,
    credentialIdentity: utf8.encode('alice-$label'),
    signerPublicKey: aliceKeyPair.publicKey(),
  );
  final groupId = group.groupId;

  // Round-trip: stored group reports the requested ciphersuite.
  final storedCs = await aliceClient.groupCiphersuite(groupIdBytes: groupId);
  printStep(step, '$label lifecycle', [
    'Ciphersuite: $cs',
    'Group ID: ${bytesToHex(groupId, maxLength: 32)}',
    'Ciphersuite round-trip OK: ${storedCs == cs}',
  ]);

  // Bob joins, both sides exchange an encrypted message.
  final bobKp = await bobClient.createKeyPackage(
    ciphersuite: cs,
    signerBytes: bobSigner,
    credentialIdentity: utf8.encode('bob-$label'),
    signerPublicKey: bobKeyPair.publicKey(),
  );
  final addResult = await aliceClient.addMembers(
    groupIdBytes: groupId,
    signerBytes: aliceSigner,
    keyPackagesBytes: [bobKp.keyPackageBytes],
  );
  await aliceClient.mergePendingCommit(groupIdBytes: groupId);
  await bobClient.joinGroupFromWelcome(
    config: config,
    welcomeBytes: addResult.welcome,
    signerBytes: bobSigner,
  );

  const messageText = 'Hello from a post-quantum group!';
  final encrypted = await aliceClient.createMessage(
    groupIdBytes: groupId,
    signerBytes: aliceSigner,
    message: utf8.encode(messageText),
  );
  final processed = await bobClient.processMessage(
    groupIdBytes: groupId,
    messageBytes: encrypted.ciphertext,
  );
  final decrypted = utf8.decode(processed.applicationMessage!);
  print('   Bob joined and decrypted: "$decrypted"');
  print('   Match: ${decrypted == messageText}');
  print('');
}
