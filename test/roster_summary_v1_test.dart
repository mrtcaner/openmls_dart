import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:openmls/openmls.dart';
import 'package:test/test.dart';

void main() {
  setUpAll(Openmls.init);

  test('roster-summary-v1 fixture matches Dart and Rust encoders', () {
    final leaves = [
      MlsRosterLeafV1(
        leafIndex: 0,
        credentialIdentity: Uint8List.fromList(utf8.encode('alice')),
        signaturePublicKey: Uint8List.fromList([1, 2, 3]),
      ),
      MlsRosterLeafV1(
        leafIndex: 2,
        credentialIdentity: Uint8List.fromList(utf8.encode('bob')),
        signaturePublicKey: Uint8List.fromList([4, 5]),
      ),
    ];
    final encoded = _encodeRosterV1(
      groupId: utf8.encode('group-1'),
      epoch: 7,
      leaves: leaves,
    );

    expect(
      _hex(encoded),
      '6f70656e6d6c735f646172742f726f737465722d73756d6d6172792f763100'
      '0000000767726f75702d310000000000000007000000020000000000000005'
      '616c696365000000030102030000000200000003626f62000000020405',
    );
    const expectedDigest =
        '09f62e52960796601347f52117a359b752870f38c21d91ba5496ab5e9cbcd23b';
    expect(_hex(sha256.convert(encoded).bytes), expectedDigest);
    expect(
      _hex(
        mlsRosterDigestV1(
          groupId: utf8.encode('group-1'),
          epoch: BigInt.from(7),
          leaves: leaves,
        ),
      ),
      expectedDigest,
    );
  });
}

Uint8List _encodeRosterV1({
  required List<int> groupId,
  required int epoch,
  required List<MlsRosterLeafV1> leaves,
}) {
  final bytes = BytesBuilder(copy: false)
    ..add(utf8.encode('openmls_dart/roster-summary/v1'))
    ..addByte(0)
    ..add(_u32(groupId.length))
    ..add(groupId)
    ..add(_u64(epoch))
    ..add(_u32(leaves.length));
  for (final leaf in leaves) {
    bytes
      ..add(_u32(leaf.leafIndex))
      ..add(_u32(leaf.credentialIdentity.length))
      ..add(leaf.credentialIdentity)
      ..add(_u32(leaf.signaturePublicKey.length))
      ..add(leaf.signaturePublicKey);
  }
  return bytes.takeBytes();
}

Uint8List _u32(int value) =>
    (ByteData(4)..setUint32(0, value)).buffer.asUint8List();

Uint8List _u64(int value) =>
    (ByteData(8)..setUint64(0, value)).buffer.asUint8List();

String _hex(List<int> bytes) =>
    bytes.map((byte) => byte.toRadixString(16).padLeft(2, '0')).join();
