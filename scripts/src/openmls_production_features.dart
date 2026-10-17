/// Validation for the features enabled on the production `openmls` crate.
library;

/// Features that must never be enabled on the production `openmls` graph.
const forbiddenOpenMlsProductionFeatures = {'backtrace', 'test-utils'};

/// Returns forbidden features enabled on `openmls` in `cargo tree` output.
///
/// [cargoTreeOutput] must be produced with `--prefix none --format {p}|{f}`.
/// Other crates' features, including `openmls_basic_credential`, are ignored.
/// Throws if the graph activates the provider covered by lockfile-only audit
/// exceptions, or if the expected OpenMLS node is missing.
Set<String> findForbiddenOpenMlsProductionFeatures(String cargoTreeOutput) {
  final found = <String>{};
  var sawOpenMls = false;

  for (final line in cargoTreeOutput.split('\n')) {
    final separator = line.indexOf('|');
    if (separator < 0) continue;

    final package = line.substring(0, separator);
    if (package.startsWith('libcrux-kem v')) {
      throw const FormatException(
        'Production dependency graph contains libcrux-kem; re-review the '
        'lockfile-only audit exceptions before enabling this provider.',
      );
    }
    if (!package.startsWith('openmls v')) continue;
    sawOpenMls = true;

    final features = line
        .substring(separator + 1)
        .split(',')
        .map((feature) => feature.trim())
        .where((feature) => feature.isNotEmpty);
    found.addAll(features.where(forbiddenOpenMlsProductionFeatures.contains));
  }

  if (!sawOpenMls) {
    throw const FormatException(
      'Cargo feature graph did not contain an openmls package node.',
    );
  }
  return found;
}
