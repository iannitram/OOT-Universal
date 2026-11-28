# Synthetic software Vulkan compute probe

This standalone Linux probe compiles the authored `compute.comp` shader to
SPIR-V, selects a Vulkan physical device whose type is `CPU`, dispatches one
compute workgroup, and verifies the result through a mapped host-visible,
host-coherent storage buffer. It fails on tool errors, missing CPU compute
devices, Vulkan API errors, or a mismatched readback. The workflow explicitly
selects Mesa Lavapipe's ICD and the probe independently checks CPU-device
evidence before dispatch.

From the repository root on Ubuntu 24.04, install `build-essential`,
`libvulkan-dev`, `mesa-vulkan-drivers`, and `glslc`, then run:

```sh
bash tools/probes/vulkan/run.sh
```

Expected output names the Vulkan CPU device and reports the verified value
`0x13579bdf`. `VK_ICD_FILENAMES` and `VK_DRIVER_FILES` select the x86-64
Lavapipe manifest. The check demonstrates software Vulkan compute and mapped
readback on hosted Linux only. It does not establish Adreno support, renderer
integration, draw or presentation coverage, validation-layer cleanliness, or
device performance.
