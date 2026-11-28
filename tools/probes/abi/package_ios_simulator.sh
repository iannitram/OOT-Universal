#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

rustc_version="$(rustc --version | awk '{print $2}')"
if [[ "$rustc_version" != "1.100.0" ]]; then
  echo "expected Rust 1.100.0, found ${rustc_version}" >&2
  exit 1
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) ;;
  *)
    echo "expected a macOS ARM64 runner, found $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac

output_app="${1:-$repo_root/target/ios-simulator/OotAbiSimulator.app}"
mkdir -p "$(dirname "$output_app")"
output_parent="$(cd "$(dirname "$output_app")" && pwd)"
output_app="$output_parent/$(basename "$output_app")"
if [[ -e "$output_app" || -L "$output_app" ]]; then
  echo "refusing to replace existing app bundle: $output_app" >&2
  exit 1
fi

work_dir="$(mktemp -d "$output_parent/.oot-ios-simulator.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

sdk_path="$(xcrun --sdk iphonesimulator --show-sdk-path)"
rust_target="aarch64-apple-ios-sim"
clang_target="arm64-apple-ios14.0-simulator"
app_name="OotAbiSimulator"
app_dir="$work_dir/$app_name.app"
mkdir -p "$app_dir"

cargo build --locked --manifest-path tools/probes/abi/Cargo.toml \
  --release --target "$rust_target" --target-dir "$work_dir/target"

xcrun --sdk iphonesimulator clang \
  -target "$clang_target" -arch arm64 -isysroot "$sdk_path" \
  -std=c11 -Wall -Wextra -Werror -Dmain=oot_abi_driver_main \
  -Itools/probes/abi -c tools/probes/abi/driver.c \
  -o "$work_dir/abi_driver.o"

xcrun --sdk iphonesimulator clang \
  -target "$clang_target" -arch arm64 -isysroot "$sdk_path" \
  -fobjc-arc -Wall -Wextra -Werror \
  tools/probes/abi/ios-simulator/AppDelegate.m "$work_dir/abi_driver.o" \
  "$work_dir/target/$rust_target/release/liboot_abi_probe.a" \
  -framework UIKit -framework Foundation -o "$app_dir/$app_name"

cp tools/probes/abi/ios-simulator/Info.plist "$app_dir/Info.plist"
plutil -lint "$app_dir/Info.plist"

plist_executable="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app_dir/Info.plist")"
if [[ "$plist_executable" != "$app_name" || ! -x "$app_dir/$plist_executable" ]]; then
  echo "Info.plist executable does not resolve to the app's executable" >&2
  exit 1
fi

architectures="$(xcrun lipo -archs "$app_dir/$plist_executable")"
if [[ "$architectures" != "arm64" ]]; then
  echo "expected only arm64 in app executable, found: $architectures" >&2
  exit 1
fi
build_metadata="$(xcrun vtool -show-build "$app_dir/$plist_executable")"
if ! grep -Eq 'platform[[:space:]]+IOSSIMULATOR([[:space:]]|$)' <<< "$build_metadata"; then
  echo "expected Mach-O build platform IOSSIMULATOR in app executable" >&2
  exit 1
fi
echo "PASS: app executable is arm64 Mach-O for IOSSIMULATOR"

codesign --force --sign - --timestamp=none "$app_dir"
codesign --verify --strict --verbose=2 "$app_dir"
signature_details="$(codesign --display --verbose=4 "$app_dir" 2>&1)"
if ! grep -q '^Signature=adhoc$' <<< "$signature_details"; then
  echo "expected an ad hoc app signature" >&2
  exit 1
fi
echo "PASS: ad hoc signature verified"
echo "PASS: executable matches Info.plist CFBundleExecutable"
mv -n "$app_dir" "$output_app"
if [[ -e "$app_dir" ]]; then
  echo "refusing to replace app bundle created during packaging: $output_app" >&2
  exit 1
fi
echo "Built iOS Simulator app: $output_app"
