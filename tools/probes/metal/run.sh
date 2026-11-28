#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source_file="${root}/tools/probes/metal/compute.m"
build_dir="$(mktemp -d)"
trap 'rm -rf "${build_dir}"' EXIT

binary="${build_dir}/metal-compute-probe"
echo "Compiling authored Objective-C Metal harness with Apple Clang"
clang -std=gnu11 -Wall -Wextra -Werror -fobjc-arc -framework Foundation -framework Metal \
  "${source_file}" -o "${binary}"

status=0
"${binary}" || status=$?

if [[ ${status} -eq 77 ]]; then
  echo "NO_METAL_DEVICE: native Metal dispatch was unavailable; the harness compiled and linked, but no compute/readback was run."
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    printf '%s\n' '## Metal compute probe' '' 'NO_METAL_DEVICE (exit 77): native dispatch/readback was not run.' >> "${GITHUB_STEP_SUMMARY}"
    echo "::notice::Metal device unavailable after native ARM64 compile and link"
    exit 0
  fi
fi

exit "${status}"
