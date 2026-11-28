#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

mode="${1:-}"
if [[ $# -gt 1 || ( $# -eq 1 && "$mode" != "--g2" ) ]]; then
  echo "Usage: bash tools/probes/abi/link_ios.sh [--g2]" >&2
  exit 2
fi

rustc_version="$(rustc --version | awk '{print $2}')"
if [[ "$rustc_version" != "1.100.0" ]]; then
  echo "expected Rust 1.100.0, found ${rustc_version}" >&2
  exit 1
fi

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/oot-ios-abi.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

if [[ "$mode" == "--g2" ]]; then
  mkdir "$work_dir/g2-generated"
  python3 tools/probes/g2/probe.py --emit-dir "$work_dir/g2-generated"
fi

verify_macho() {
  local executable="$1"
  local expected_platform="$2"
  local architectures build_metadata
  architectures="$(xcrun lipo -archs "$executable")"
  if [[ "$architectures" != "arm64" ]]; then
    echo "expected only arm64 in $executable, found: $architectures" >&2
    exit 1
  fi
  build_metadata="$(xcrun vtool -show-build "$executable")"
  if ! grep -Eq "platform[[:space:]]+$expected_platform([[:space:]]|$)" <<< "$build_metadata"; then
    echo "expected Mach-O build platform $expected_platform in $executable" >&2
    exit 1
  fi
}

verify_and_link() {
  local rust_target="$1"
  local sdk="$2"
  local clang_target="$3"
  local expected_platform="$4"
  local output="$work_dir/abi-${sdk}"
  local sdk_path
  sdk_path="$(xcrun --sdk "$sdk" --show-sdk-path)"

  cargo build --locked --manifest-path tools/probes/abi/Cargo.toml \
    --release --target "$rust_target" --target-dir "$work_dir/target"

  xcrun --sdk "$sdk" clang \
    -target "$clang_target" \
    -arch arm64 \
    -isysroot "$sdk_path" \
    -std=c11 -Wall -Wextra -Werror \
    -Itools/probes/abi \
    tools/probes/abi/driver.c \
    "$work_dir/target/$rust_target/release/liboot_abi_probe.a" \
    -o "$output"

  verify_macho "$output" "$expected_platform"
  echo "PASS: $sdk $rust_target arm64; LC_BUILD_VERSION platform $expected_platform"

  if [[ "$mode" != "--g2" ]]; then
    return 0
  fi
  local candidate g2_output
  for candidate in relink longshot; do
    g2_output="$work_dir/g2-${candidate}-${sdk}"
    xcrun --sdk "$sdk" clang \
      -target "$clang_target" \
      -arch arm64 \
      -isysroot "$sdk_path" \
      -std=c11 -Wall -Wextra -Werror \
      -I"$work_dir/g2-generated/$candidate" \
      tools/probes/g2/fixture.c \
      -o "$g2_output"

    verify_macho "$g2_output" "$expected_platform"
    echo "PASS: G2 $candidate $sdk arm64; LC_BUILD_VERSION platform $expected_platform"
  done
}

verify_and_link aarch64-apple-ios iphoneos arm64-apple-ios13.0 IOS
verify_and_link aarch64-apple-ios-sim iphonesimulator arm64-apple-ios14.0-simulator IOSSIMULATOR
