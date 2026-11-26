# Synthetic C/Rust ABI probe

This base-neutral probe validates fixed-width `int32_t` arguments/results, a synchronous C callback, a NULL-initialized output slot, a borrowed context, Rust allocation plus caller-owned opaque pointer, null/error handling, and shutdown that frees the allocation and clears the caller's pointer. Callback context must outlive the probe; callbacks must not unwind. The Rust boundary catches Rust panics and returns `OOT_ABI_PANIC`, but cannot catch C exceptions or a panic originating in an `extern "C"` callback. The callback-failure enum is reserved; this probe has no callback failure channel.

This is **not** the selected game's ABI or a claim that either candidate recompiler is compatible. No game-derived signatures/data are used. It does not test pointer bounds for arbitrary invalid non-null addresses, concurrent calls, mobile SDK linking, or a selected-base owner workflow.

From the repository root, run the native Linux probe:

```sh
CARGO_TARGET_DIR=target cargo test --manifest-path tools/probes/abi/Cargo.toml --lib
CARGO_TARGET_DIR=target cargo build --manifest-path tools/probes/abi/Cargo.toml --release
cc -std=c11 -Wall -Wextra -Werror -Itools/probes/abi \
  tools/probes/abi/driver.c \
  target/release/liboot_abi_probe.a -ldl -lpthread -lm -o /tmp/oot-abi-probe
/tmp/oot-abi-probe
```

Compile the freestanding C translation unit for both architectures (requires Clang targets/sysroot headers; this source uses only compiler-provided integer headers):

```sh
clang --target=x86_64-unknown-linux-gnu -ffreestanding -fsyntax-only \
  -std=c11 -Wall -Wextra -Werror -Itools/probes/abi tools/probes/abi/layout.c
clang --target=aarch64-unknown-linux-gnu -ffreestanding -fsyntax-only \
  -std=c11 -Wall -Wextra -Werror -Itools/probes/abi tools/probes/abi/layout.c
```

A successful Clang command establishes only that the synthetic C declarations compile for that target. It does not link or run ARM64 code. No ARM64 execution was performed by this probe; no iOS/Android application link was attempted. Selected recompiler ABI extraction, chosen-base linking, and owner base/mobile feasibility gates remain pending.
