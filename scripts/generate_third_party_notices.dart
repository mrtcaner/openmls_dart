import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:openmls/src/third_party_notices.dart';

import 'src/common.dart';

Future<void> main(List<String> arguments) async {
  final outputPath = _argumentValue(arguments, '--output');
  final checkPath = _argumentValue(arguments, '--check');
  final manifestPath =
      _argumentValue(arguments, '--manifest-path') ?? 'rust/Cargo.toml';
  if ((outputPath == null) == (checkPath == null)) {
    stderr.writeln(
      'Usage: (--output <path> | --check <path>) '
      '[--manifest-path rust/Cargo.toml]',
    );
    exitCode = 64;
    return;
  }

  final result = await Process.run('cargo', [
    'metadata',
    '--locked',
    '--format-version=1',
    '--manifest-path',
    manifestPath,
  ]);
  if (result.exitCode != 0) {
    stderr.write(result.stderr);
    exitCode = result.exitCode;
    return;
  }

  final metadata = jsonDecode(result.stdout as String) as Map<String, dynamic>;
  final rootId = metadata['resolve'] is Map<String, dynamic>
      ? (metadata['resolve'] as Map<String, dynamic>)['root'] as String?
      : null;
  final packages =
      (metadata['packages'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((package) => package['id'] != rootId)
          .toList()
        ..sort((left, right) {
          final leftKey = _packageKey(left);
          final rightKey = _packageKey(right);
          return leftKey.compareTo(rightKey);
        });

  final buffer = StringBuffer()
    ..writeln('openmls_frb third-party notices')
    ..writeln('===================================')
    ..writeln()
    ..writeln(
      'This deterministic file covers the complete Cargo dependency resolution '
      'for this source revision. A target may link only a subset.',
    )
    ..writeln();
  final licenseTexts = <String, ({String name, String text})>{};

  for (final package in packages) {
    final name = package['name'] as String;
    final version = package['version'] as String;
    final license = package['license'] as String? ?? 'not declared';
    final repository = package['repository'] as String?;
    final source = package['source'] as String? ?? 'path dependency';
    buffer
      ..writeln(
        '------------------------------------------------------------------------',
      )
      ..writeln('$name $version')
      ..writeln('License expression: $license')
      ..writeln('Source: $source');
    if (repository != null) buffer.writeln('Repository: $repository');

    final licenseFiles = _licenseFiles(package);
    if (licenseFiles.isEmpty) {
      buffer
        ..writeln()
        ..writeln(
          '[No license or notice text was found in the resolved package.]',
        )
        ..writeln();
      continue;
    }
    for (final file in licenseFiles) {
      final text = _normalizeNewlines(file.readAsStringSync()).trimRight();
      final digest = sha256.convert(utf8.encode(text)).toString();
      final textId = 'text-${digest.substring(0, 16)}';
      final name = _basename(file.path);
      final existing = licenseTexts[textId];
      if (existing == null || name.compareTo(existing.name) < 0) {
        licenseTexts[textId] = (name: name, text: text);
      }
      buffer
        ..writeln()
        ..writeln('License text: $textId ($name)');
    }
  }

  buffer
    ..writeln(
      '========================================================================',
    )
    ..writeln('Deduplicated license and notice texts')
    ..writeln(
      '========================================================================',
    )
    ..writeln();
  final textIds = licenseTexts.keys.toList()..sort();
  for (final textId in textIds) {
    final licenseText = licenseTexts[textId]!;
    buffer
      ..writeln(
        '------------------------------------------------------------------------',
      )
      ..writeln('$textId (${licenseText.name})')
      ..writeln(
        '------------------------------------------------------------------------',
      )
      ..writeln(licenseText.text)
      ..writeln();
  }

  final content = '${buffer.toString().trimRight()}\n';
  if (outputPath != null) {
    final output = File(outputPath);
    output.parent.createSync(recursive: true);
    output.writeAsStringSync(content);
    stdout.writeln(
      'Wrote ${packages.length} resolved packages to ${output.path}',
    );
    return;
  }

  final checkedFile = File(checkPath!);
  if (!checkedFile.existsSync()) {
    stderr.writeln('Missing committed notice asset: ${checkedFile.path}');
    exitCode = 1;
    return;
  }

  final committedBytes = checkedFile.readAsBytesSync();
  final generatedBytes = utf8.encode(content);
  final committedDigest = sha256.convert(committedBytes).toString();
  final generatedDigest = sha256.convert(generatedBytes).toString();
  final normalizedCheckPath = checkPath.replaceAll('\\', '/');
  final expectedAssetKey = 'packages/openmls/$normalizedCheckPath';
  final crateVersion = getCrateVersion();
  final errors = <String>[];

  if (!_bytesEqual(committedBytes, generatedBytes)) {
    errors.add(
      'Committed notice bytes do not match the locked Cargo resolution '
      '(committed $committedDigest, generated $generatedDigest).',
    );
  }
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

  if (errors.isNotEmpty) {
    for (final error in errors) {
      stderr.writeln(error);
    }
    exitCode = 1;
    return;
  }

  stdout.writeln(
    'Verified ${packages.length} resolved packages, openmls_frb '
    '$crateVersion, and SHA-256 $committedDigest.',
  );
}

String? _argumentValue(List<String> arguments, String name) {
  final index = arguments.indexOf(name);
  if (index == -1 || index + 1 >= arguments.length) return null;
  return arguments[index + 1];
}

String _packageKey(Map<String, dynamic> package) =>
    '${package['name']}\u0000${package['version']}\u0000${package['source'] ?? ''}';

List<File> _licenseFiles(Map<String, dynamic> package) {
  final explicitPath = package['license_file'] as String?;
  if (explicitPath != null) {
    final explicit = File(explicitPath);
    if (explicit.existsSync()) return [explicit];
  }

  final manifest = File(package['manifest_path'] as String);
  final packageRoot = manifest.parent;
  var files = _licensesIn(packageRoot);
  if (files.isNotEmpty) return files;

  final source = package['source'] as String? ?? '';
  if (!source.startsWith('git+')) return const [];

  var directory = packageRoot;
  for (var depth = 0; depth < 8; depth++) {
    if (File('${directory.path}/.git').existsSync() ||
        Directory('${directory.path}/.git').existsSync()) {
      files = _licensesIn(directory);
      return files;
    }
    final parent = directory.parent;
    if (parent.path == directory.path) break;
    directory = parent;
  }
  return const [];
}

List<File> _licensesIn(Directory directory) {
  if (!directory.existsSync()) return const [];
  final files = directory.listSync(followLinks: false).whereType<File>().where((
    file,
  ) {
    final name = _basename(file.path).toUpperCase();
    return name.startsWith('LICENSE') ||
        name.startsWith('LICENCE') ||
        name.startsWith('COPYING') ||
        name.startsWith('NOTICE') ||
        name == 'UNLICENSE';
  }).toList()..sort((left, right) => left.path.compareTo(right.path));
  return files;
}

String _basename(String path) => path.replaceAll('\\', '/').split('/').last;

String _normalizeNewlines(String value) =>
    value.replaceAll('\r\n', '\n').replaceAll('\r', '\n');

bool _bytesEqual(List<int> left, List<int> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
