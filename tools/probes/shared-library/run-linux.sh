#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"
probe_dir="$repo_root/tools/probes/shared-library"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/oot-shared-library.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

rustc_version="$(rustc --version | awk '{print $2}')"
if [[ "$rustc_version" != "1.100.0" ]]; then
  echo "expected Rust 1.100.0, found ${rustc_version}" >&2
  exit 1
fi

cargo build --locked --manifest-path tools/probes/abi/Cargo.toml \
  --release --target-dir "$work_dir/target"

"${CC:-cc}" -std=c11 -fPIC -Wall -Wextra -Werror \
  -Itools/probes/abi -Wl,--no-undefined -shared "$probe_dir/library.c" "$probe_dir/bridge_library.c" \
  -o "$work_dir/liboot_probe.so"
nm -D --undefined-only "$work_dir/liboot_probe.so" > "$work_dir/shared-undefined-symbols.txt"
if grep -q 'oot_abi_probe_' "$work_dir/shared-undefined-symbols.txt"; then
  echo "shared library must call the host runtime table instead of linking Rust ABI symbols" >&2
  exit 1
fi
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror \
  "$probe_dir/loader.c" -ldl -o "$work_dir/oot-shared-loader"
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -Itools/probes/abi \
  "$probe_dir/bridge_loader.c" "$probe_dir/runtime_bridge.c" \
  "$work_dir/target/release/liboot_abi_probe.a" -ldl -lpthread -lm \
  -o "$work_dir/oot-shared-bridge-loader"
"$work_dir/oot-shared-loader" "$work_dir/liboot_probe.so"
if "$work_dir/oot-shared-loader" "$work_dir/missing.so"; then
  echo "loader unexpectedly accepted a missing library" >&2
  exit 1
fi
"$work_dir/oot-shared-bridge-loader" "$work_dir/liboot_probe.so"
