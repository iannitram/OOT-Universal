# G2 synthetic instruction probe

This runs the vendored per-instruction AArch64-to-C emitters from both pinned candidate import commits on project-authored `ADD`, `CBZ`, `STR` and `LDR` encodings. Git reads the two source files from the local Git object database into a temporary directory. Donor source remains in the import branches; this probe commits no donor code. No game copy, image, or generated game code is used. Temporary emitters and generated C are removed after the probe exits. Fetch the import branches first if the pinned commits are not locally available.

Requirements: Python 3, Git, a working host Clang C/C++ toolchain and an AArch64-capable Clang backend. On Windows the C++ toolchain must have a working C++ standard library and linker. Pass explicit executables with `--clang` and `--clangxx` if they are not on PATH. Run from the repository root:

```sh
python tools/probes/g2/probe.py
```

For each candidate, the probe checks the emitter flags and runs the emitted C natively. It checks ADD normal and wrapping results, both CBZ destinations using a minimal host dispatch, and an eight-byte store/load round trip. The project-authored memory helpers use bounded byte offsets and little-endian encoding, and the fixture checks out-of-bounds reads and writes. The emitted branch sets the guest PC and returns; the host fixture dispatches only its two known destinations. The probe also compiles the fixture as a freestanding `aarch64-none-elf` object. A nonzero exit indicates failure. Windows x86-64 execution and ARM64 object compilation passed with Python 3.12.10 and Clang 23.1.5 for both candidates.

These four instructions do not exercise the full candidate lift pipeline or its host runtime. FP/vector operations, realistic guest address mapping, threading/TLS, broad instruction coverage, mobile linking and selected-base compatibility remain open. The [hosted macOS job](../../../.github/workflows/g2.yml) confirms the ARM64 host architecture and executes the fixture there. The freestanding object compile remains a separate check. The imported emitters contain GPL-licensed code; the probe does not copy it into the repository.
