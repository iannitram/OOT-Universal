#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
probe_dir="$repo_root/tools/probes/shared-library"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/oot-shared-library.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

${CC:-cc} -std=c11 -fPIC -Wall -Wextra -Werror \
  -shared "$probe_dir/library.c" -o "$work_dir/liboot_probe.so"
${CC:-cc} -std=c11 -Wall -Wextra -Werror \
  "$probe_dir/loader.c" -ldl -o "$work_dir/oot-shared-loader"
"$work_dir/oot-shared-loader" "$work_dir/liboot_probe.so"
