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
  Darwin:arm64|Linux:x86_64) ;;
  *)
    echo "expected a native macOS ARM64 or Linux x86-64 runner, found $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/oot-native-abi.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

cargo build --locked --manifest-path tools/probes/abi/Cargo.toml \
  --release --target-dir "$work_dir/target"

case "$(uname -s)" in
  Darwin)
    cc -std=c11 -Wall -Wextra -Werror -Itools/probes/abi \
      tools/probes/abi/driver.c \
      "$work_dir/target/release/liboot_abi_probe.a" \
      -o "$work_dir/oot-abi-probe"
    ;;
  Linux)
    cc -std=c11 -Wall -Wextra -Werror -Itools/probes/abi \
      tools/probes/abi/driver.c \
      "$work_dir/target/release/liboot_abi_probe.a" \
      -ldl -lpthread -lm -o "$work_dir/oot-abi-probe"
    ;;
esac

"$work_dir/oot-abi-probe"
echo "PASS: native $(uname -s) $(uname -m) C/Rust ABI driver"
