import 'package:test/test.dart';

import '../../scripts/src/check_updates.dart';

void main() {
  group('validateOpenMlsTag', () {
    test('accepts upstream stable and prerelease tags', () {
      expect(validateOpenMlsTag('openmls-v0.8.1'), 'openmls-v0.8.1');
      expect(validateOpenMlsTag('openmls-v1.2.3-rc.1'), 'openmls-v1.2.3-rc.1');
      expect(
        validateOpenMlsTag('openmls-v1.2.3-alpha-1'),
        'openmls-v1.2.3-alpha-1',
      );
    });

    test('rejects other prefixes and unsafe values', () {
      for (final value in [
        'v0.8.1',
        '0.8.1',
        'OpenMLS-v0.8.1',
        'openmls-v01.8.1',
        'openmls-v0.8',
        'openmls-v0.8.1+build',
        'openmls-v0.8.1/branch',
        'openmls-v0.8.1; echo unsafe',
        'openmls-v0.8.1\nunsafe',
      ]) {
        expect(
          () => validateOpenMlsTag(value),
          throwsFormatException,
          reason: value,
        );
      }
    });
  });
}
