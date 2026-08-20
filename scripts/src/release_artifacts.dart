import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

const expectedReleaseArtifactFiles = <String, List<String>>{
  'openmls-android-arm64-v8a': <String>['libopenmls_frb.so'],
  'openmls-android-armeabi-v7a': <String>['libopenmls_frb.so'],
  'openmls-android-x86_64': <String>['libopenmls_frb.so'],
  'openmls-ios-device-arm64': <String>['libopenmls_frb.dylib'],
  'openmls-ios-simulator-arm64': <String>['libopenmls_frb.dylib'],
  'openmls-ios-simulator-x86_64': <String>['libopenmls_frb.dylib'],
  'openmls-linux-arm64': <String>['libopenmls_frb.so'],
  'openmls-linux-x86_64': <String>['libopenmls_frb.so'],
  'openmls-macos-arm64': <String>['libopenmls_frb.dylib'],
  'openmls-macos-x86_64': <String>['libopenmls_frb.dylib'],
  'openmls-wasm32': <String>['openmls_frb.js', 'openmls_frb_bg.wasm'],
  'openmls-windows-x86_64': <String>['openmls_frb.dll'],
};

const forbiddenReleaseMarkers = <String>[
  'account-envelope-fixtures/v1',
  'deterministic-prng',
  'emit_account_envelope',
  'fuzz_decode_account_envelope_v1',
  'hpke_round_trip_for_test',
  'HpkeTestRng',
  'native-receive-proof',
  'native_receive_v1_export_vectors',
  'private_bundle_from_test_material',
];

const _allowedPackagingFiles = <String>{'LICENSE', 'THIRD_PARTY_NOTICES.txt'};

List<Map<String, Object>> verifyReleaseArtifacts({
  required Directory artifactsDirectory,
  required String version,
  Directory? archivesDirectory,
}) {
  if (!RegExp(r'^\d+\.\d+\.\d+$').hasMatch(version)) {
    throw ArgumentError.value(version, 'version', 'must be X.Y.Z');
  }
  final expectedDirectories = expectedReleaseArtifactFiles.keys.toList()
    ..sort();
  final actualDirectories =
      artifactsDirectory
          .listSync(followLinks: false)
          .whereType<Directory>()
          .map((directory) => _basename(directory.path))
          .toList()
        ..sort();
  if (!_sameStrings(actualDirectories, expectedDirectories)) {
    throw StateError(
      'release artifact directory mismatch: expected $expectedDirectories, '
      'found $actualDirectories',
    );
  }

  final records = <Map<String, Object>>[];
  for (final directoryName in expectedDirectories) {
    final directory = Directory('${artifactsDirectory.path}/$directoryName');
    final expectedFiles = [...expectedReleaseArtifactFiles[directoryName]!]
      ..sort();
    final actualFiles =
        directory
            .listSync(followLinks: false)
            .whereType<File>()
            .map((file) => _basename(file.path))
            .toList()
          ..sort();
    final unexpectedFiles = actualFiles
        .where(
          (file) =>
              !expectedFiles.contains(file) &&
              !_allowedPackagingFiles.contains(file),
        )
        .toList();
    final missingFiles = expectedFiles
        .where((file) => !actualFiles.contains(file))
        .toList();
    if (missingFiles.isNotEmpty || unexpectedFiles.isNotEmpty) {
      throw StateError(
        '$directoryName file mismatch: missing $missingFiles, '
        'unexpected $unexpectedFiles',
      );
    }
    for (final fileName in expectedFiles) {
      final file = File('${directory.path}/$fileName');
      final bytes = file.readAsBytesSync();
      if (bytes.isEmpty) throw StateError('${file.path} is empty');
      rejectForbiddenReleaseMarkers(bytes, path: file.path);
      records.add(_record('binary', '$directoryName/$fileName', bytes));
    }
  }

  if (archivesDirectory != null) {
    final expectedArchives =
        expectedDirectories
            .map(
              (directory) =>
                  'openmls_frb-$version-${directory.substring('openmls-'.length)}.tar.gz',
            )
            .toList()
          ..sort();
    final actualArchives =
        archivesDirectory
            .listSync(followLinks: false)
            .whereType<File>()
            .map((file) => _basename(file.path))
            .where((name) => name.endsWith('.tar.gz'))
            .toList()
          ..sort();
    if (!_sameStrings(actualArchives, expectedArchives)) {
      throw StateError(
        'release archive mismatch: expected $expectedArchives, '
        'found $actualArchives',
      );
    }
    for (final archiveName in expectedArchives) {
      final bytes = File(
        '${archivesDirectory.path}/$archiveName',
      ).readAsBytesSync();
      if (bytes.isEmpty) throw StateError('$archiveName is empty');
      records.add(_record('archive', archiveName, bytes));
    }
  }

  records.sort(
    (left, right) =>
        (left['path']! as String).compareTo(right['path']! as String),
  );
  return records;
}

void writeReleaseArtifactReport({
  required File target,
  required String version,
  required List<Map<String, Object>> records,
  String? baselineVersion,
  String? sourceRevision,
}) {
  final report = <String, Object>{
    'format': 'openmls_dart/release-artifact-report/v1',
    'version': version,
    if (baselineVersion != null) 'baselineVersion': baselineVersion,
    if (sourceRevision != null && sourceRevision.isNotEmpty)
      'sourceRevision': sourceRevision,
    'artifacts': records,
  };
  target.parent.createSync(recursive: true);
  target.writeAsStringSync(
    '${const JsonEncoder.withIndent('  ').convert(report)}\n',
  );
}

List<Map<String, Object>> addReleaseArtifactSizeDeltas({
  required List<Map<String, Object>> current,
  required List<Map<String, Object>> baseline,
  required String currentVersion,
  required String baselineVersion,
}) {
  if (currentVersion == baselineVersion) {
    throw ArgumentError('current and baseline versions must differ');
  }
  final baselineByPath = <String, Map<String, Object>>{
    for (final record in baseline) record['path']! as String: record,
  };
  return current.map((record) {
    final path = record['path']! as String;
    final baselinePath = record['kind'] == 'archive'
        ? path.replaceFirst(
            'openmls_frb-$currentVersion-',
            'openmls_frb-$baselineVersion-',
          )
        : path;
    final baselineRecord = baselineByPath[baselinePath];
    if (baselineRecord == null || baselineRecord['kind'] != record['kind']) {
      throw StateError('no matching baseline artifact for $path');
    }
    final bytes = record['bytes']! as int;
    final baselineBytes = baselineRecord['bytes']! as int;
    if (baselineBytes == 0) throw StateError('$baselinePath is empty');
    final deltaBytes = bytes - baselineBytes;
    final deltaPercent = deltaBytes * 100 / baselineBytes;
    return <String, Object>{
      ...record,
      'baselineBytes': baselineBytes,
      'deltaBytes': deltaBytes,
      'deltaPercent': double.parse(deltaPercent.toStringAsFixed(2)),
    };
  }).toList();
}

void rejectForbiddenReleaseMarkers(Uint8List bytes, {required String path}) {
  for (final marker in forbiddenReleaseMarkers) {
    if (_indexOf(bytes, ascii.encode(marker)) >= 0) {
      throw StateError('$path contains forbidden release marker "$marker"');
    }
  }
}

Map<String, Object> _record(String kind, String path, Uint8List bytes) =>
    <String, Object>{
      'kind': kind,
      'path': path,
      'bytes': bytes.length,
      'sha256': sha256.convert(bytes).toString(),
    };

int _indexOf(Uint8List haystack, List<int> needle) {
  if (needle.isEmpty) return 0;
  for (var start = 0; start <= haystack.length - needle.length; start++) {
    var matches = true;
    for (var index = 0; index < needle.length; index++) {
      if (haystack[start + index] != needle[index]) {
        matches = false;
        break;
      }
    }
    if (matches) return start;
  }
  return -1;
}

bool _sameStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

String _basename(String path) =>
    path.replaceAll('\\', '/').split('/').where((part) => part.isNotEmpty).last;
