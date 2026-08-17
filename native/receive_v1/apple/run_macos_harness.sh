#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
build_dir=$(mktemp -d)
library="$repository_root/rust/target/release/libopenmls_frb.dylib"

cleanup() {
  rm -rf "$build_dir"
}
trap cleanup EXIT INT TERM

cargo build --release --manifest-path "$repository_root/rust/Cargo.toml"
clang -std=c11 -Wall -Wextra -Werror \
  -I "$repository_root/native/receive_v1/include" \
  "$repository_root/native/receive_v1/apple/NativeReceiveV1Harness.c" \
  -L "$repository_root/rust/target/release" -lopenmls_frb \
  -Wl,-rpath,"$repository_root/rust/target/release" \
  -o "$build_dir/native_receive_v1_apple_harness"
"$build_dir/native_receive_v1_apple_harness" \
  "$repository_root/native/receive_v1/fixtures"
