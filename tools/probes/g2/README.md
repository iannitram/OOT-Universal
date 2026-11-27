# G2 synthetic instruction probe

This runs the vendored per-instruction AArch64-to-C emitter from both pinned candidate import commits using the project-authored encoding `0x91001c00` (`add x0, x0, #7`). Git reads the two source files from the local Git object database into a temporary directory. Donor source remains in the import branches; this probe commits no donor code. No game copy, image, or generated game code is used. The temporary emitter binaries and C are removed after the probe exits. Fetch the import branches first if the pinned commits are not locally available.

Requirements: Python 3, Git, a working host Clang C/C++ toolchain and an AArch64-capable Clang backend. On Windows the C++ toolchain must have a working C++ standard library and linker. Pass explicit executables with `--clang` and `--clangxx` if they are not on PATH. Run from the repository root:

```sh
python tools/probes/g2/probe.py
```

For each candidate the probe builds its emitter, rejects an unhandled or terminating translation, executes the emitted C natively with normal and wrapping inputs, then cross-compiles it as a freestanding `aarch64-none-elf` object. A nonzero exit indicates failure. On Windows x86-64 this passed with Python 3.12.10 and Clang 23.1.5; both candidates produced passing native checks and ARM64 objects.

This is one integer instruction, not the full candidate lift pipeline. The minimal context models only the register used by this instruction. It does not test branches, FP/vector operations, memory mapping, threading/TLS, host runtime dependencies, instruction coverage, or a real game. AArch64 execution on desktop ARM64, mobile linking, the selected base's full translator and its host-specific dependencies remain G2 gates. The imported emitters contain GPL-licensed code; the probe does not copy it into the repository.
