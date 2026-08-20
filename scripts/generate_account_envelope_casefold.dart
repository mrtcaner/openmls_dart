import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';

const _expectedHeader = '# CaseFolding-17.0.0.txt';
const _expectedSha256 =
    'ff8d8fefbf123574205085d6714c36149eb946d717a0c585c27f0f4ef58c4183';

void main(List<String> arguments) {
  if (arguments.length != 2) {
    stderr.writeln(
      'Usage: generate_account_envelope_casefold.dart '
      '<CaseFolding-17.0.0.txt> <output.rs>',
    );
    exitCode = 64;
    return;
  }

  final inputBytes = File(arguments[0]).readAsBytesSync();
  final inputDigest = sha256.convert(inputBytes).toString();
  if (inputDigest != _expectedSha256) {
    stderr.writeln(
      'CaseFolding source SHA-256 is $inputDigest; expected $_expectedSha256.',
    );
    exitCode = 65;
    return;
  }
  final input = const LineSplitter().convert(utf8.decode(inputBytes));
  if (input.isEmpty || input.first != _expectedHeader) {
    stderr.writeln('Input is not the frozen Unicode 17.0.0 case-fold file.');
    exitCode = 65;
    return;
  }

  final mappings = <int, List<int>>{};
  for (final rawLine in input) {
    final data = rawLine.split('#').first.trim();
    if (data.isEmpty) continue;
    final fields = data.split(';').map((value) => value.trim()).toList();
    if (fields.length < 3 || (fields[1] != 'C' && fields[1] != 'F')) {
      continue;
    }
    final source = int.parse(fields[0], radix: 16);
    final target = fields[2]
        .split(RegExp(r'\s+'))
        .where((value) => value.isNotEmpty)
        .map((value) => int.parse(value, radix: 16))
        .toList(growable: false);
    if (target.isEmpty || target.length > 3 || mappings.containsKey(source)) {
      stderr.writeln('Unexpected mapping at U+${_hex(source)}.');
      exitCode = 65;
      return;
    }
    mappings[source] = target;
  }

  final sorted = mappings.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  final output = StringBuffer()
    ..writeln(
      '// Generated from Unicode 17.0.0 CaseFolding.txt statuses C and F.',
    )
    ..writeln('// Source SHA-256:')
    ..writeln(
      '// ff8d8fefbf123574205085d6714c36149eb946d717a0c585c27f0f4ef58c4183',
    )
    ..writeln('// Unicode data is used under the Unicode License v3.')
    ..writeln(
      '// Regenerate through `make generate-account-envelope-casefold`.',
    )
    ..writeln()
    ..writeln('pub(super) const UNICODE_VERSION: (u8, u8, u8) = (17, 0, 0);')
    ..writeln(
      'pub(super) const FULL_DEFAULT_CASE_FOLD: &[(char, &[char])] = &[',
    );
  for (final entry in sorted) {
    final target = entry.value
        .map((value) => "'\\u{${_hex(value)}}'")
        .join(', ');
    output.writeln("    ('\\u{${_hex(entry.key)}}', &[$target]),");
  }
  output.writeln('];');
  File(arguments[1]).writeAsStringSync(output.toString());
  stdout.writeln('Generated ${sorted.length} Unicode 17 case-fold mappings.');
}

String _hex(int value) => value.toRadixString(16).toUpperCase().padLeft(4, '0');
