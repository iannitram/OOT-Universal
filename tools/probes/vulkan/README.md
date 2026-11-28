# Synthetic software Vulkan probes

These standalone Linux probes select a Vulkan physical device whose type is
`CPU`. The compute probe compiles `compute.comp`, dispatches one workgroup, and
verifies mapped storage-buffer readback. The draw probe compiles the authored
triangle vertex and fragment shaders, renders into an offscreen RGBA image,
copies it to mapped host-visible coherent memory, and checks both a covered
red center pixel and a black untouched corner. Both fail on tool errors,
missing CPU Vulkan capabilities, API errors, or mismatched readback. The
workflow selects Mesa Lavapipe and the probes independently confirm CPU-device
evidence before running.

From the repository root on Ubuntu 24.04, install `build-essential`,
`libvulkan-dev`, `mesa-vulkan-drivers`, and `glslc`, then run:

```sh
bash tools/probes/vulkan/run.sh
```

Expected output names the Vulkan CPU device, reports the verified compute
value `0x13579bdf`, and confirms the covered and background draw pixels.
`VK_ICD_FILENAMES` and `VK_DRIVER_FILES` select the x86-64 Lavapipe manifest.
These checks demonstrate software Vulkan compute and offscreen drawing with
mapped readback on hosted Linux only. They do not establish Adreno support,
renderer integration or presentation coverage, validation-layer cleanliness,
or device performance.
