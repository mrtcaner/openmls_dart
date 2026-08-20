import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

import '../../scripts/src/release_artifacts.dart';

void main() {
  late Directory temporary;

  setUp(() {
    temporary = Directory.systemTemp.createTempSync('release_artifacts_test');
  });
  tearDown(() => temporary.deleteSync(recursive: true));

  test('verifies exact all-platform inventory and writes stable report', () {
    final artifacts = _createArtifactInventory(temporary);
    final archives = Directory('${temporary.path}/archives')..createSync();
    for (final directory in expectedReleaseArtifactFiles.keys) {
      final suffix = directory.substring('openmls-'.length);
      File(
        '${archives.path}/openmls_frb-3.2.0-$suffix.tar.gz',
      ).writeAsBytesSync(<int>[1, 2, 3]);
    }

    final records = verifyReleaseArtifacts(
      artifactsDirectory: artifacts,
      archivesDirectory: archives,
      version: '3.2.0',
    );
    expect(records, hasLength(25));
    final paths = records.map((record) => record['path']! as String).toList();
    expect(paths, orderedEquals([...paths]..sort()));

    final baselineArtifacts = _createArtifactInventory(
      temporary,
      directoryName: 'baseline-artifacts',
      bytes: <int>[1, 2, 3],
    );
    final baselineArchives = Directory('${temporary.path}/baseline-archives')
      ..createSync();
    for (final directory in expectedReleaseArtifactFiles.keys) {
      final suffix = directory.substring('openmls-'.length);
      File(
        '${baselineArchives.path}/openmls_frb-3.1.0-$suffix.tar.gz',
      ).writeAsBytesSync(<int>[1, 2]);
    }
    final baselineRecords = verifyReleaseArtifacts(
      artifactsDirectory: baselineArtifacts,
      archivesDirectory: baselineArchives,
      version: '3.1.0',
    );
    final recordsWithDeltas = addReleaseArtifactSizeDeltas(
      current: records,
      baseline: baselineRecords,
      currentVersion: '3.2.0',
      baselineVersion: '3.1.0',
    );
    expect(recordsWithDeltas.first['baselineBytes'], 3);
    expect(recordsWithDeltas.first['deltaBytes'], 1);
    expect(recordsWithDeltas.first['deltaPercent'], 33.33);

    final report = File('${temporary.path}/report.json');
    writeReleaseArtifactReport(
      target: report,
      version: '3.2.0',
      records: recordsWithDeltas,
      baselineVersion: '3.1.0',
      sourceRevision: 'abc123',
    );
    final decoded =
        jsonDecode(report.readAsStringSync()) as Map<String, Object?>;
    expect(decoded['format'], 'openmls_dart/release-artifact-report/v1');
    expect(decoded['sourceRevision'], 'abc123');
    expect(decoded['baselineVersion'], '3.1.0');
    expect(decoded['artifacts'], hasLength(25));
  });

  test('rejects a missing or extra platform artifact', () {
    final artifacts = _createArtifactInventory(temporary);
    Directory(
      '${artifacts.path}/openmls-linux-arm64',
    ).deleteSync(recursive: true);
    expect(
      () => verifyReleaseArtifacts(
        artifactsDirectory: artifacts,
        version: '3.2.0',
      ),
      throwsStateError,
    );

    _writeArtifact(artifacts, 'unexpected-platform', 'library.bin', <int>[1]);
    expect(
      () => verifyReleaseArtifacts(
        artifactsDirectory: artifacts,
        version: '3.2.0',
      ),
      throwsStateError,
    );
  });

  test('rejects test-only and deterministic markers in release bytes', () {
    final bytes = utf8.encode('prefix fuzz_decode_account_envelope_v1 suffix');
    expect(
      () => rejectForbiddenReleaseMarkers(bytes, path: 'libopenmls_frb.so'),
      throwsStateError,
    );
  });
}

Directory _createArtifactInventory(
  Directory parent, {
  String directoryName = 'artifacts',
  List<int> bytes = const <int>[1, 2, 3, 4],
}) {
  final artifacts = Directory('${parent.path}/$directoryName')..createSync();
  for (final entry in expectedReleaseArtifactFiles.entries) {
    for (final file in entry.value) {
      _writeArtifact(artifacts, entry.key, file, bytes);
    }
  }
  return artifacts;
}

void _writeArtifact(
  Directory root,
  String directory,
  String file,
  List<int> bytes,
) {
  File('${root.path}/$directory/$file')
    ..createSync(recursive: true)
    ..writeAsBytesSync(bytes);
}
