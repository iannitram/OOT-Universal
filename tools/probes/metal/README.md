# Native Metal compute probe

This standalone probe compiles and links an authored Objective-C harness with
Apple Clang and the macOS Metal and Foundation frameworks. At runtime the
harness compiles an embedded Metal compute shader, dispatches four threads,
and checks the results in a shared host-readable Metal buffer. Shader,
pipeline, dispatch, allocation, and readback errors fail the probe.

Run on macOS with Xcode command-line tools:

```sh
bash tools/probes/metal/run.sh
```

The executable returns 77 and prints `NO_METAL_DEVICE` when macOS reports no
Metal device. The script preserves that distinct result in the log and GitHub
step summary, while allowing CI to complete with the probe clearly reported as
not run. All other nonzero results fail the script. A successful run reports
the device and verified values `0x13579bdf` through `0x13579be2`.

The hosted workflow requires `uname -m` to report `arm64`, proving that the
native Apple toolchain compiled and linked this probe on macOS ARM64. A hosted
runner without a Metal device proves only compilation and linking; it does not
prove dispatch, mapped readback, presentation, renderer integration, iOS
behavior, or operation on an owner device. No device result is inferred from
the runner architecture.
