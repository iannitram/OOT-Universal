#!/usr/bin/env bash
set -euo pipefail

# Compile and link the shared-library loader probe for Android ARM64. This does
# not run the resulting executable; device execution is outside this link check.
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
  echo "This script currently supports the Linux x86_64 NDK host only." >&2
  exit 2
fi

toolchain="$ndk_root/toolchains/llvm/prebuilt/linux-x86_64/bin"
clang="$toolchain/${target}${api_level}-clang"
readelf="$toolchain/llvm-readelf"
nm="$toolchain/llvm-nm"
if [[ ! -x "$clang" || ! -x "$readelf" || ! -x "$nm" ]]; then
  echo "NDK is missing the Android ARM64 API $api_level compiler or LLVM inspection tools." >&2
  exit 2
fi

probe_dir="$repo_root/tools/probes/shared-library"
output_dir="$repo_root/target/android-arm64"
mkdir -p "$output_dir"
library="$output_dir/liboot_probe.so"
loader="$output_dir/oot-shared-loader"

"$clang" -std=c11 -Wall -Wextra -Werror -fPIC -shared \
  -Wl,--no-undefined -Wl,-z,relro,-z,now -Wl,-soname,liboot_probe.so \
  "$probe_dir/library.c" -o "$library"
"$clang" -std=c11 -Wall -Wextra -Werror -fPIE -pie \
  -Wl,--no-undefined -Wl,-z,relro,-z,now \
  "$probe_dir/loader.c" -ldl -o "$loader"

library_header="$("$readelf" -h "$library")"
printf '%s\n' "$library_header"
grep -Eq 'Class:[[:space:]]+ELF64' <<<"$library_header"
grep -Eq 'Machine:[[:space:]]+AArch64' <<<"$library_header"
grep -Eq 'Type:[[:space:]]+DYN' <<<"$library_header"
library_dynamic="$("$readelf" --dynamic "$library")"
grep -F 'Library soname: [liboot_probe.so]' <<<"$library_dynamic"
symbols="$("$nm" -D --defined-only "$library")"
grep -Eq '[[:space:]]oot_shared_transform$' <<<"$symbols"
grep -Eq '[[:space:]]oot_shared_version$' <<<"$symbols"

loader_header="$("$readelf" -h "$loader")"
printf '%s\n' "$loader_header"
grep -Eq 'Class:[[:space:]]+ELF64' <<<"$loader_header"
grep -Eq 'Machine:[[:space:]]+AArch64' <<<"$loader_header"
grep -Eq 'Type:[[:space:]]+DYN' <<<"$loader_header"
loader_program_headers="$("$readelf" --program-headers "$loader")"
grep -F '[Requesting program interpreter: /system/bin/linker64]' <<<"$loader_program_headers"
loader_dynamic="$("$readelf" --dynamic "$loader")"
grep -Eq 'Shared library: \[libdl\.so\]' <<<"$loader_dynamic"
if grep -Fq 'Shared library: [liboot_probe.so]' <<<"$loader_dynamic"; then
  echo "loader must not link directly to the synthetic library" >&2
  exit 1
fi

echo "Android ARM64 API $api_level library and loader link verified; no device execution performed."
