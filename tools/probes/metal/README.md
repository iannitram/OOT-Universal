# Native Metal compute and draw probe

This standalone probe compiles and links an authored Objective-C harness with
Apple Clang and the macOS Metal and Foundation frameworks. At runtime the
harness compiles embedded Metal compute and render shaders, dispatches four
compute threads, and checks the results in a shared host-readable Metal
buffer. It also creates a render pipeline and render pass, draws an authored
triangle into a private 64x64 RGBA8 target, then blits the texture into a shared
buffer and checks that the center is red and the top-left corner remains black.
Shader, pipeline, dispatch, allocation, rendering, and readback errors fail
the probe.

Run on macOS with Xcode command-line tools:

```sh
bash tools/probes/metal/run.sh
```

When macOS reports no Metal device, the probe prints `NO_METAL_DEVICE` and
exits 77 locally. GitHub Actions records the capability result in a notice and
step summary and completes the compile-and-link check. All other nonzero
results fail the script. A successful run reports the device, verified compute
values `0x13579bdf` through `0x13579be2`, and the red-center/black-corner render
result. Paravirtual device names are identified as paravirtual; all other names
remain unclassified, so the probe does not claim physical-GPU validation.

The hosted workflow requires `uname -m` to report `arm64`, proving that the
native Apple toolchain compiled and linked this probe on macOS ARM64. A hosted
runner without a Metal device proves only compilation and linking; it does not
prove dispatch, readback, presentation, renderer integration, iOS behavior, or
operation on an owner device. Hosted Apple Paravirtual execution is evidence
for that virtual device only, not for a physical GPU. No device result is
inferred from runner architecture.
