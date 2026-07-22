import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:openmls/openmls.dart';

void main() => runApp(const MyApp());

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'openmls Example',
    theme: ThemeData(
      colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
      useMaterial3: true,
    ),
    home: const _CallerStorageDemo(),
  );
}

class _CallerStorageDemo extends StatefulWidget {
  const _CallerStorageDemo();

  @override
  State<_CallerStorageDemo> createState() => _CallerStorageDemoState();
}

class _CallerStorageDemoState extends State<_CallerStorageDemo> {
  String _status = 'Ready';

  Future<void> _createKeyPackage() async {
    setState(() => _status = 'Creating…');
    try {
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
        credentialIdentity: utf8.encode('example-installation'),
        signerPublicKey: keyPair.publicKey(),
        storageEntries: const [],
        storageFormatVersion: mlsStorageFormatVersion(),
      );
      setState(
        () => _status =
            'Created ${result.keyPackageBytes.length} bytes and '
            '${result.storageBatch.upserts.length} caller-owned mutations.',
      );
    } catch (error) {
      setState(() => _status = 'Failed: $error');
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('openmls Example')),
    body: Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'This example creates one KeyPackage without opening a database. '
            'A real app atomically stores the returned opaque mutation batch.',
          ),
          const SizedBox(height: 24),
          FilledButton(
            onPressed: _createKeyPackage,
            child: const Text('Create KeyPackage'),
          ),
          const SizedBox(height: 16),
          Text(_status),
        ],
      ),
    ),
  );
}
