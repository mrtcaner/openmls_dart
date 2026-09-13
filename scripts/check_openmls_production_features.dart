#!/usr/bin/env dart

/// Fails when the resolved production graph enables unsafe OpenMLS features.
library;

import 'dart:io';

import 'src/openmls_production_features.dart';

Future<void> main(List<String> arguments) async {
  final manifestPath = _argumentValue(arguments, '--manifest-path');
  if (manifestPath == null || arguments.length != 2) {
    stderr.writeln(
      'Usage: dart scripts/check_openmls_production_features.dart '
      '--manifest-path <Cargo.toml>',
    );
    exitCode = 64;
    return;
  }

  final result = await Process.run('cargo', [
    'tree',
    '--locked',
    '--manifest-path',
    manifestPath,
    '--target',
    'all',
    '--edges',
    'normal,build',
    '--prefix',
    'none',
    '--format',
    '{p}|{f}',
  ]);
  if (result.exitCode != 0) {
    stderr
      ..writeln('cargo tree failed with exit code ${result.exitCode}:')
      ..write(result.stderr);
    exitCode = result.exitCode;
    return;
  }

  try {
    final forbidden = findForbiddenOpenMlsProductionFeatures(
      result.stdout as String,
    );
    if (forbidden.isNotEmpty) {
      final sorted = forbidden.toList()..sort();
      stderr.writeln(
        'Forbidden production OpenMLS features are enabled: '
        '${sorted.join(', ')}',
      );
      exitCode = 1;
      return;
    }
  } on FormatException catch (error) {
    stderr.writeln(error.message);
    exitCode = 1;
    return;
  }

  stdout.writeln('Production OpenMLS feature graph is safe.');
}

String? _argumentValue(List<String> arguments, String name) {
  final index = arguments.indexOf(name);
  if (index < 0 || index + 1 >= arguments.length) return null;
  return arguments[index + 1];
}
