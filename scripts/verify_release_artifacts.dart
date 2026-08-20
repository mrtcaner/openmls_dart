#!/usr/bin/env dart

import 'dart:io';

import 'src/common.dart';
import 'src/release_artifacts.dart';

void main(List<String> arguments) {
  try {
    final artifactsPath = _requiredValue(arguments, '--artifacts');
    final version = _requiredValue(arguments, '--version');
    final archivesPath = _optionalValue(arguments, '--archives');
    final reportPath = _optionalValue(arguments, '--report');
    final baselineArtifactsPath = _optionalValue(
      arguments,
      '--baseline-artifacts',
    );
    final baselineArchivesPath = _optionalValue(
      arguments,
      '--baseline-archives',
    );
    final baselineVersion = _optionalValue(arguments, '--baseline-version');
    if ((archivesPath == null) != (reportPath == null)) {
      throw const FormatException(
        '--archives and --report must be supplied together',
      );
    }
    final baselineValues = <String?>[
      baselineArtifactsPath,
      baselineArchivesPath,
      baselineVersion,
    ];
    if (baselineValues.any((value) => value == null) &&
        baselineValues.any((value) => value != null)) {
      throw const FormatException(
        '--baseline-artifacts, --baseline-archives, and --baseline-version '
        'must be supplied together',
      );
    }
    var records = verifyReleaseArtifacts(
      artifactsDirectory: Directory(artifactsPath),
      version: version,
      archivesDirectory: archivesPath == null ? null : Directory(archivesPath),
    );
    if (baselineVersion != null) {
      final baseline = verifyReleaseArtifacts(
        artifactsDirectory: Directory(baselineArtifactsPath!),
        version: baselineVersion,
        archivesDirectory: Directory(baselineArchivesPath!),
      );
      records = addReleaseArtifactSizeDeltas(
        current: records,
        baseline: baseline,
        currentVersion: version,
        baselineVersion: baselineVersion,
      );
    }
    if (reportPath != null) {
      writeReleaseArtifactReport(
        target: File(reportPath),
        version: version,
        records: records,
        baselineVersion: baselineVersion,
        sourceRevision: Platform.environment['GITHUB_SHA'],
      );
      logSuccess('Wrote verified artifact report to $reportPath.');
    } else {
      logSuccess('Verified ${records.length} release artifact files.');
    }
  } on Object catch (error) {
    logError('$error');
    exitCode = 1;
  }
}

String _requiredValue(List<String> arguments, String flag) {
  final value = _optionalValue(arguments, flag);
  if (value == null) throw FormatException('missing $flag <value>');
  return value;
}

String? _optionalValue(List<String> arguments, String flag) {
  final index = arguments.indexOf(flag);
  if (index == -1) return null;
  if (index + 1 >= arguments.length || arguments[index + 1].startsWith('--')) {
    throw FormatException('missing value after $flag');
  }
  return arguments[index + 1];
}
