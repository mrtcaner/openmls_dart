# Flutter example

This app demonstrates the Rust-owned `MlsEngine` API across the package's
supported Flutter platforms. Its screens cover group lifecycle, messages,
proposals, commits, queries, and advanced operations.

From the repository root, prepare dependencies and native code with `make
setup` and `make build`. Use `make build-example-web` for the JavaScript Web
build. The package's `flutter build web --wasm` limitation also applies here.

For the operation-scoped caller-owned storage boundary, see
[`test/external_storage_test.dart`](../test/external_storage_test.dart); this
Flutter example intentionally uses only one storage authority.
