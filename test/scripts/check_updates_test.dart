import 'package:test/test.dart';

import '../../scripts/src/check_updates.dart';
import '../../scripts/src/common.dart';

const _prefix = 'openmls-v';

void main() {
  group('validateUpstreamTag', () {
    test('accepts stable and prerelease tags with the configured prefix', () {
      expect(validateUpstreamTag('${_prefix}0.8.1'), '${_prefix}0.8.1');
      expect(
        validateUpstreamTag('${_prefix}1.2.3-rc.1'),
        '${_prefix}1.2.3-rc.1',
      );
    });

    test('accepts the tag recorded in rust/Cargo.toml', () {
      expect(() => validateUpstreamTag(getUpstreamVersion()), returnsNormally);
    });

    test(
      'rejects other prefixes, noncanonical versions, and unsafe values',
      () {
        for (final value in [
          'v0.8.1',
          '0.8.1',
          'OpenMLS-v0.8.1',
          'openmls-v01.8.1',
          'openmls-v0.8',
          'openmls-v0.8.1+build',
          'openmls-v0.8.1/branch',
          'openmls-v0.8.1; echo unsafe',
          r'openmls-v0.8.1$(whoami)',
          'openmls-v0.8.1`id`',
          'openmls-v0.8.1/../evil',
          'openmls-v0.8.1\nunsafe',
          'openmls-v0.8.1\n',
        ]) {
          expect(
            () => validateUpstreamTag(value),
            throwsFormatException,
            reason: value,
          );
        }
      },
    );

    test('names the rejected source in the error', () {
      expect(
        () => validateUpstreamTag('nope', source: '--version argument'),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('--version argument'),
          ),
        ),
      );
    });
  });
}
