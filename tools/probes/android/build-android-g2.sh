#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ndk_root="${ANDROID_NDK_HOME:-${ANDROID_NDK_LATEST_HOME:-}}"
if [[ -z "$ndk_root" || "$(uname -s)-$(uname -m)" != "Linux-x86_64" ]]; then
  echo "Set ANDROID_NDK_HOME to an installed NDK on Linux x86_64." >&2
  exit 2
fi
toolchain="$ndk_root/toolchains/llvm/prebuilt/linux-x86_64/bin"
clang="$toolchain/aarch64-linux-android24-clang"
readelf="$toolchain/llvm-readelf"
if [[ ! -x "$clang" || ! -x "$readelf" ]]; then
  echo "NDK is missing the Android ARM64 API 24 compiler or llvm-readelf." >&2
  exit 2
fi
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/includes"
python3 "$repo_root/tools/probes/g2/probe.py" --emit-dir "$tmp/includes"
for candidate in relink longshot; do
  executable="$tmp/$candidate"
  "$clang" -std=c11 -Wall -Wextra -Werror -Wl,--no-undefined -pie \
    -I "$tmp/includes/$candidate" "$repo_root/tools/probes/g2/fixture.c" -o "$executable"
  header="$("$readelf" -h "$executable")"
  printf '%s\n' "$header"
  grep -Eq 'Class:[[:space:]]+ELF64' <<<"$header"
  grep -Eq 'Machine:[[:space:]]+AArch64' <<<"$header"
  program_headers="$("$readelf" --program-headers "$executable")"
  grep -F '[Requesting program interpreter: /system/bin/linker64]' <<<"$program_headers"
  echo "$candidate: Android ARM64 API 24 fixture linked and verified"
done
