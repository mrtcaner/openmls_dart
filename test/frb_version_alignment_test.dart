import 'dart:io';

import 'package:test/test.dart';

void main() {
  test('Dart, Rust, and codegen use one exact FRB version', () {
    final cargoVersion = _capture(
      File('rust/Cargo.toml').readAsStringSync(),
      RegExp(r'^flutter_rust_bridge\s*=\s*"=([^"]+)"\s*$', multiLine: true),
      'exact flutter_rust_bridge version in rust/Cargo.toml',
    );
    final dartVersion = _capture(
      File('pubspec.yaml').readAsStringSync(),
      RegExp(r'^  flutter_rust_bridge:\s*([0-9][^\s#]*)\s*$', multiLine: true),
      'exact flutter_rust_bridge version in pubspec.yaml',
    );
    final codegenVersion = _capture(
      File('Makefile').readAsStringSync(),
      RegExp(r'^FRB_CODEGEN_VERSION\s*\?=\s*([^\s#]+)\s*$', multiLine: true),
      'FRB_CODEGEN_VERSION in Makefile',
    );

    expect(dartVersion, cargoVersion);
    expect(codegenVersion, cargoVersion);
  });
}

String _capture(String source, RegExp pattern, String description) {
  final match = pattern.firstMatch(source);
  expect(match, isNotNull, reason: 'Missing $description');
  return match!.group(1)!;
}
