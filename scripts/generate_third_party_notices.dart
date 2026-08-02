#!/usr/bin/env dart

/// Generate or verify the bundled native dependency notice inventory.
///
/// The inventory is derived from the locked normal and build dependency graph
/// for every release target. Development-only dependencies are excluded.
library;

import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:openmls/src/third_party_notices.dart';

import 'src/common.dart';
import 'src/third_party_notices.dart';

const _defaultNoticePath = 'assets/THIRD_PARTY_NOTICES.txt';

Future<void> main(List<String> arguments) async {
  if (arguments.contains('--help') || arguments.contains('-h')) {
    _printUsage();
    return;
  }

  final outputPath = _argumentValue(arguments, '--output');
  final checkRequested = arguments.contains('--check');
  final checkPath = checkRequested
      ? (_argumentValue(arguments, '--check') ?? _defaultNoticePath)
      : null;
  final manifestPath =
      _argumentValue(arguments, '--manifest-path') ?? 'rust/Cargo.toml';

  if (outputPath != null && checkRequested) {
    stderr.writeln('Choose either --output or --check, not both.');
    exitCode = 64;
    return;
  }
  if (arguments.contains('--output') && outputPath == null) {
    stderr.writeln('--output requires a path.');
    exitCode = 64;
    return;
  }

  try {
    logStep(
      'Resolving linked crates across ${releaseTargets.length} release '
      'targets...',
    );
    final generated = generateNotices(
      manifestPath: manifestPath,
      packageName: 'openmls',
      crateName: getCrateName(),
      onProgress: (target) => logInfo('  $target'),
    );

    if (checkRequested) {
      _verify(generated, checkPath!);
      return;
    }

    final target = File(outputPath ?? _defaultNoticePath);
    target.parent.createSync(recursive: true);
    target.writeAsStringSync(generated);
    final digest = sha256.convert(utf8.encode(generated));
    logSuccess(
      'Wrote ${target.path} '
      '(${(generated.length / 1024).toStringAsFixed(1)} KiB, SHA-256 $digest)',
    );
  } on Exception catch (error) {
    stderr.writeln(error);
    exitCode = 2;
  }
}

void _verify(String generated, String checkPath) {
  final checkedFile = File(checkPath);
  final errors = <String>[];
  final drift = describeDrift(generated: generated, committed: checkedFile);
  if (drift != null) errors.add(drift);

  if (checkedFile.existsSync()) {
    final committedDigest = sha256
        .convert(checkedFile.readAsBytesSync())
        .toString();
    final normalizedPath = checkPath.replaceAll('\\', '/');
    final expectedAssetKey = 'packages/openmls/$normalizedPath';
    final crateVersion = getCrateVersion();

    if (openmlsThirdPartyNoticesAssetKey != expectedAssetKey) {
      errors.add(
        'Asset key is "$openmlsThirdPartyNoticesAssetKey"; '
        'expected "$expectedAssetKey".',
      );
    }
    if (openmlsThirdPartyNoticesNativeVersion != crateVersion) {
      errors.add(
        'Notice native version is "$openmlsThirdPartyNoticesNativeVersion"; '
        'rust/Cargo.toml is "$crateVersion".',
      );
    }
    if (openmlsThirdPartyNoticesSha256 != committedDigest) {
      errors.add(
        'Notice SHA-256 is "$openmlsThirdPartyNoticesSha256"; '
        'committed asset is "$committedDigest".',
      );
    }
  }

  if (errors.isNotEmpty) {
    for (final error in errors) {
      stderr.writeln(error);
    }
    exitCode = 1;
    return;
  }

  final digest = sha256.convert(checkedFile.readAsBytesSync());
  logSuccess(
    'Verified ${checkedFile.path}, openmls_frb ${getCrateVersion()}, '
    'and SHA-256 $digest.',
  );
}

String? _argumentValue(List<String> arguments, String name) {
  final index = arguments.indexOf(name);
  if (index == -1 || index + 1 >= arguments.length) return null;
  final value = arguments[index + 1];
  return value.startsWith('--') ? null : value;
}

void _printUsage() {
  stdout.writeln('''
Generate or verify the bundled third-party notice inventory.

Usage:
  make third-party-notices
  make third-party-notices ARGS="--output <path>"
  make verify-third-party-notices

Options:
  --output <path>       Write to a path (defaults to $_defaultNoticePath)
  --check [path]        Verify a path (defaults to $_defaultNoticePath)
  --manifest-path <p>   Cargo manifest (defaults to rust/Cargo.toml)
  --help, -h            Show this help
''');
}
