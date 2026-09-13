import 'package:test/test.dart';

import '../../scripts/src/openmls_production_features.dart';

void main() {
  test('accepts a production OpenMLS graph without forbidden features', () {
    const tree = '''
openmls_frb v3.2.0|default
openmls v0.8.1 (https://example.invalid/openmls)|openmls_rust_crypto
openmls_basic_credential v0.5.0|default,test-utils
backtrace v0.3.76|default,std
''';

    expect(findForbiddenOpenMlsProductionFeatures(tree), isEmpty);
  });

  test('rejects only forbidden features enabled on openmls itself', () {
    const tree = '''
openmls v0.8.1 (https://example.invalid/openmls)|backtrace,openmls_rust_crypto,test-utils
openmls_basic_credential v0.5.0|default,test-utils
''';

    expect(findForbiddenOpenMlsProductionFeatures(tree), {
      'backtrace',
      'test-utils',
    });
  });

  test('fails closed when cargo output has no openmls package node', () {
    const tree = 'openmls_basic_credential v0.5.0|default,test-utils';

    expect(
      () => findForbiddenOpenMlsProductionFeatures(tree),
      throwsFormatException,
    );
  });
}
