import 'dart:io';

import 'package:test/test.dart';

void main() {
  final workflow = File(
    '.github/workflows/build-openmls.yml',
  ).readAsStringSync();
  String step(String name) {
    final start = workflow.indexOf('      - name: $name\n');
    expect(start, isNonNegative, reason: 'missing step $name');
    final next = workflow.indexOf('\n      - name:', start + 1);
    return workflow.substring(start, next < 0 ? workflow.length : next);
  }

  test('native publication and attestation require a tag push', () {
    const guard = "if: github.event_name == 'push' && github.ref_type == 'tag'";
    for (final name in [
      'Attest build provenance',
      'Attach attestation bundle to release assets',
      'Create Release',
    ]) {
      expect(step(name), contains(guard), reason: name);
    }
    expect(
      RegExp(r'^\s+gh release create ', multiLine: true).allMatches(workflow),
      hasLength(1),
    );
    expect(step('Create Release'), contains('gh release create'));
  });

  test('manual native runs retain checked archives without publication', () {
    expect(workflow, contains('workflow_dispatch:'));
    final upload = step('Retain checked archives for review');
    expect(upload, contains('actions/upload-artifact@'));
    expect(upload, contains('path: release-archives/'));
    expect(upload, contains('if-no-files-found: error'));
    expect(upload, isNot(contains('if:')));
  });
}
