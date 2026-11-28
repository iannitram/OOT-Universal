#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

rustc_version="$(rustc --version | awk '{print $2}')"
if [[ "$rustc_version" != "1.100.0" ]]; then
  echo "expected Rust 1.100.0, found ${rustc_version}" >&2
  exit 1
fi

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/oot-ios-abi.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

verify_and_link() {
  local rust_target="$1"
  local sdk="$2"
  local clang_target="$3"
  local minimum_flag="$4"
  local expected_platform="$5"
  local output="$work_dir/abi-${sdk}"
  local sdk_path
  sdk_path="$(xcrun --sdk "$sdk" --show-sdk-path)"

  cargo build --locked --manifest-path tools/probes/abi/Cargo.toml \
    --release --target "$rust_target" --target-dir "$work_dir/target"

  xcrun --sdk "$sdk" clang \
    -target "$clang_target" \
    -arch arm64 \
    "$minimum_flag" \
    -isysroot "$sdk_path" \
    -std=c11 -Wall -Wextra -Werror \
    -Itools/probes/abi \
    tools/probes/abi/driver.c \
    "$work_dir/target/$rust_target/release/liboot_abi_probe.a" \
    -o "$output"

  local architectures
  architectures="$(xcrun lipo -archs "$output")"
  if [[ "$architectures" != "arm64" ]]; then
    echo "expected only arm64 in $output, found: $architectures" >&2
    exit 1
  fi

  local build_metadata
  build_metadata="$(xcrun vtool -show-build "$output")"
  if ! grep -Eq "platform[[:space:]]+$expected_platform([[:space:]]|$)" <<< "$build_metadata"; then
    echo "expected Mach-O build platform $expected_platform in $output" >&2
    exit 1
  fi
  echo "PASS: $sdk $rust_target arm64; LC_BUILD_VERSION platform $expected_platform"
}

verify_and_link aarch64-apple-ios iphoneos arm64-apple-ios13.0 \
  -miphoneos-version-min=13.0 IOS
verify_and_link aarch64-apple-ios-sim iphonesimulator \
  arm64-apple-ios13.0-simulator -mios-simulator-version-min=13.0 IOSSIMULATOR
