# Command-line example

This package demonstrates the Rust-owned `MlsEngine` API from a standalone Dart
program in `bin/main.dart`. It covers key-package creation, group setup, member
admission, Welcome processing, encrypted messages, and commit processing.

Prepare the repository with `make setup` and `make build`, then validate the
library and examples with `make test`. For the caller-owned storage boundary,
see [`test/external_storage_test.dart`](../test/external_storage_test.dart).
