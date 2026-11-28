#!/usr/bin/env bash
set -euo pipefail

# Link the existing synthetic C/Rust ABI probe for Android ARM64. This creates
# an Android executable; running it requires an Android device or emulator.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
api_level="${ANDROID_API_LEVEL:-24}"
target="aarch64-linux-android"
ndk_root="${ANDROID_NDK_HOME:-${ANDROID_NDK_LATEST_HOME:-}}"

if [[ -z "$ndk_root" || ! -d "$ndk_root/toolchains/llvm/prebuilt" ]]; then
  echo "Set ANDROID_NDK_HOME to an installed Android NDK." >&2
  exit 2
fi
if [[ ! "$api_level" =~ ^[0-9]+$ ]]; then
  echo "ANDROID_API_LEVEL must be an integer (got '$api_level')." >&2
  exit 2
fi

if [[ "$(uname -s)-$(uname -m)" != "Linux-x86_64" ]]; then
  echo "This script currently supports the verified Linux x86_64 NDK host only." >&2
  exit 2
fi

toolchain="$ndk_root/toolchains/llvm/prebuilt/linux-x86_64/bin"
clang="$toolchain/${target}${api_level}-clang"
readelf="$toolchain/llvm-readelf"
if [[ ! -x "$clang" || ! -x "$readelf" ]]; then
  echo "NDK is missing the Android ARM64 API $api_level compiler or llvm-readelf." >&2
  exit 2
fi

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$clang"
cargo +1.100.0 build --locked --manifest-path "$repo_root/tools/probes/abi/Cargo.toml" \
  --release --target "$target" --target-dir "$repo_root/target"

output_dir="$repo_root/target/android-arm64"
mkdir -p "$output_dir"
"$clang" -std=c11 -Wall -Wextra -Werror -Wl,--no-undefined -Wl,-z,relro,-z,now -pie \
  -I"$repo_root/tools/probes/abi" \
  "$repo_root/tools/probes/abi/driver.c" \
  "$repo_root/target/$target/release/liboot_abi_probe.a" \
  -ldl -llog -lunwind -ldl -lm -lc -o "$output_dir/oot-abi-probe"

header="$("$readelf" -h "$output_dir/oot-abi-probe")"
printf '%s\n' "$header"
grep -Eq 'Class:[[:space:]]+ELF64' <<<"$header"
grep -Eq 'Machine:[[:space:]]+AArch64' <<<"$header"
echo "Android ARM64 API $api_level executable verified: $output_dir/oot-abi-probe"
