#!/bin/sh
set -eu
repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
build_dir="$repository_root/.buildlog/external-resync-r0-20261016/n3b-artifacts"
mkdir -p "$build_dir"
clang -std=c11 -Wall -Wextra -Werror \
  -I "$repository_root/native/receive_v2/include" \
  -I "$repository_root/native/receive_v3/include" \
  "$repository_root/native/receive_v3/apple/NativeReceiveV3ArtifactHarness.c" \
  -L "$repository_root/rust/target/release" -lopenmls_frb \
  -Wl,-rpath,"$repository_root/rust/target/release" \
  -o "$build_dir/native_receive_v3_apple_harness"
"$build_dir/native_receive_v3_apple_harness" "$build_dir/fixtures"
