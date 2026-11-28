#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
probe_dir="${root}/tools/probes/vulkan"
build_dir="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/oot-vulkan-probe"
build_dir="$(mktemp -d "${build_dir}.XXXXXX")"
trap 'rm -rf "${build_dir}"' EXIT

icd="/usr/share/vulkan/icd.d/lvp_icd.x86_64.json"
if [[ ! -r "${icd}" ]]; then
  icd="/usr/share/vulkan/icd.d/lvp_icd.json"
fi
if [[ ! -r "${icd}" ]]; then
  echo "Lavapipe ICD manifest not found in /usr/share/vulkan/icd.d" >&2
  exit 1
fi
export VK_ICD_FILENAMES="${icd}"
export VK_DRIVER_FILES="${icd}"

glslc --target-env=vulkan1.0 "${probe_dir}/compute.comp" -o "${build_dir}/compute.spv"
cc -std=c11 -O2 -Wall -Wextra -Werror \
  "${probe_dir}/probe.c" -lvulkan -o "${build_dir}/vulkan-compute-probe"
"${build_dir}/vulkan-compute-probe" "${build_dir}/compute.spv"
