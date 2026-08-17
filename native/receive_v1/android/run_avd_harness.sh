#!/bin/sh
set -eu

serial=${1:?adb serial required}
repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
sdk_root=${ANDROID_HOME:-"$HOME/Library/Android/sdk"}
d8=$(find "$sdk_root/build-tools" -mindepth 2 -maxdepth 2 -type f -name d8 | sort | tail -1)
library="$repository_root/rust/target/aarch64-linux-android/release/libopenmls_frb.so"
fixtures="$repository_root/native/receive_v1/fixtures"
build_dir=$(mktemp -d)
remote_dir="/data/local/tmp/openmls-receive-v1-$$"

cleanup() {
  rm -rf "$build_dir"
  adb -s "$serial" shell rm -rf "$remote_dir" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

test -x "$d8"
test -f "$library"
test -f "$fixtures/manifest.json"

javac -d "$build_dir/classes" \
  "$repository_root/native/receive_v1/android/OpenMlsNativeReceive.java" \
  "$repository_root/native/receive_v1/android/NativeReceiveV1Harness.java"
jar --create --file "$build_dir/harness.jar" -C "$build_dir/classes" .
"$d8" --min-api 28 --output "$build_dir" "$build_dir/harness.jar"

adb -s "$serial" shell mkdir -p "$remote_dir/fixtures"
adb -s "$serial" push "$build_dir/classes.dex" "$remote_dir/classes.dex" >/dev/null
adb -s "$serial" push "$library" "$remote_dir/libopenmls_frb.so" >/dev/null
adb -s "$serial" push "$fixtures/." "$remote_dir/fixtures/" >/dev/null
adb -s "$serial" shell \
  "CLASSPATH=$remote_dir/classes.dex app_process /system/bin app.kurtuba.openmls.NativeReceiveV1Harness $remote_dir/libopenmls_frb.so $remote_dir/fixtures"
