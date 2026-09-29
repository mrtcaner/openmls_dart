#!/bin/sh
set -eu
repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
build_dir="$repository_root/.buildlog/native-receive-v2-apple"
mkdir -p "$build_dir"
clang -std=c11 -Wall -Wextra -Werror \
  -I "$repository_root/native/receive_v2/include" \
  "$repository_root/native/receive_v2/apple/NativeReceiveV2Harness.c" \
  -L "$repository_root/rust/target/release" -lopenmls_frb \
  -Wl,-rpath,"$repository_root/rust/target/release" \
  -o "$build_dir/native_receive_v2_apple_harness"
"$build_dir/native_receive_v2_apple_harness" "$repository_root/native/receive_v2/fixtures"
