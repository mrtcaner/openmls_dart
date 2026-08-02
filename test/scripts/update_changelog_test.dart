import 'package:test/test.dart';

import '../../scripts/src/update_changelog.dart';

/// A released CHANGELOG with NO `## [Unreleased]` section — the normal state
/// right after `make release` finalized the previous version (it no longer
/// leaves an empty `## [Unreleased]` behind). The next native-update PR lands on
/// top of this shape, so `insertChangelogEntry` must create the section.
const _noUnreleased = '''
# Changelog

## [1.4.2] - 2026-07-20

### For Users

- Prior release

## [1.4.1] - 2026-07-14

- Older release

[Unreleased]: https://github.com/mrtcaner/openmls_dart/compare/v1.4.2...HEAD
[1.4.2]: https://github.com/mrtcaner/openmls_dart/compare/v1.4.1...v1.4.2
''';

/// The same CHANGELOG but with an in-progress `## [Unreleased]` section already
/// open (a second native update within the same release cycle).
const _withUnreleased = '''
# Changelog

## [Unreleased]

### For Users

#### ✨ Highlights

- **openmls_frb v1.5.2** — Rust FFI bindings

#### Changed

- Update openmls native library to v0.8.1

## [1.4.2] - 2026-07-20

- Prior release

[Unreleased]: https://github.com/mrtcaner/openmls_dart/compare/v1.4.2...HEAD
[1.4.2]: https://github.com/mrtcaner/openmls_dart/compare/v1.4.1...v1.4.2
''';

void main() {
  group('insertChangelogEntry', () {
    test('creates the [Unreleased] section when none exists', () {
      final result = insertChangelogEntry(
        currentChangelog: _noUnreleased,
        nativeHighlight: '**openmls v0.8.2** — protocol update',
        changed: '- Update openmls native library to v0.8.2',
      );

      final lines = result.split('\n');

      // Exactly one [Unreleased] heading is created (no duplication).
      expect(
        lines.where((l) => l.startsWith('## [Unreleased]')).length,
        equals(1),
      );

      // It sits above the topmost released version.
      final unreleasedIdx = lines.indexWhere(
        (l) => l.startsWith('## [Unreleased]'),
      );
      final firstVersionIdx = lines.indexWhere(
        (l) => l.startsWith('## [1.4.2]'),
      );
      expect(unreleasedIdx, greaterThanOrEqualTo(0));
      expect(unreleasedIdx, lessThan(firstVersionIdx));

      // The new entry landed inside the created section.
      expect(result, contains('**openmls v0.8.2** — protocol update'));
      expect(result, contains('- Update openmls native library to v0.8.2'));

      // The released sections and the footer link are preserved.
      expect(result, contains('## [1.4.2] - 2026-07-20'));
      expect(
        result,
        contains(
          '[Unreleased]: https://github.com/mrtcaner/openmls_dart/compare',
        ),
      );
    });

    test('inserts into the existing [Unreleased] without duplicating it', () {
      final result = insertChangelogEntry(
        currentChangelog: _withUnreleased,
        nativeHighlight: '**openmls v0.8.2** — protocol update',
        changed: '- Update openmls native library to v0.8.2',
      );

      // Still exactly one [Unreleased] heading — it was reused, not recreated.
      expect(
        result.split('\n').where((l) => l.startsWith('## [Unreleased]')).length,
        equals(1),
      );

      // The new entry is present alongside the pre-existing one.
      expect(result, contains('**openmls v0.8.2** — protocol update'));
      expect(result, contains('**openmls_frb v1.5.2**'));
    });

    test('creates For Users before an existing For Contributors block', () {
      const input = '''
# Changelog

## [Unreleased]

### For Contributors

#### Fixed

- CI fix

## [1.4.2] - 2026-07-20
''';
      final result = insertChangelogEntry(
        currentChangelog: input,
        nativeHighlight: '**openmls v0.8.2** — protocol update',
        changed: '- Update openmls native library to v0.8.2',
      );
      final lines = result.split('\n');

      expect(lines.where((line) => line == '### For Users'), hasLength(1));
      expect(
        lines.indexOf('### For Users'),
        lessThan(lines.indexOf('### For Contributors')),
      );
      expect(result, contains('- CI fix'));
    });

    test('never files the update under Changed (Breaking)', () {
      const input = '''
# Changelog

## [Unreleased]

### For Users

#### Changed (Breaking)

- Existing breaking change

#### Changed

- Existing routine change

## [1.4.2] - 2026-07-20
''';
      const update = '- Update openmls native library to v0.8.2';
      final result = insertChangelogEntry(
        currentChangelog: input,
        nativeHighlight: '**openmls v0.8.2** — protocol update',
        changed: update,
      );
      final lines = result.split('\n');

      expect(lines.where((line) => line == update), hasLength(1));
      expect(lines.indexOf(update), greaterThan(lines.indexOf('#### Changed')));
      expect(
        lines.indexOf('#### Changed'),
        greaterThan(lines.indexOf('#### Changed (Breaking)')),
      );
    });

    test('creates Changed between Changed (Breaking) and Fixed', () {
      const input = '''
# Changelog

## [Unreleased]

### For Users

#### Changed (Breaking)

- Existing breaking change

#### Fixed

- Existing fix

## [1.4.2] - 2026-07-20
''';
      final result = insertChangelogEntry(
        currentChangelog: input,
        nativeHighlight: '**openmls v0.8.2** — protocol update',
        changed: '- Update openmls native library to v0.8.2',
      );
      final lines = result.split('\n');
      final breaking = lines.indexOf('#### Changed (Breaking)');
      final changed = lines.indexOf('#### Changed');
      final fixed = lines.indexOf('#### Fixed');

      expect(changed, greaterThan(breaking));
      expect(changed, lessThan(fixed));
      expect(result, contains('- Existing fix'));
    });
  });
}
