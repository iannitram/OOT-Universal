# Synthetic C/Rust ABI probe

This base-neutral probe validates fixed-width `int32_t` arguments/results, a synchronous C callback, a NULL-initialized output slot, a borrowed context, Rust allocation plus caller-owned opaque pointer, null/error handling, and shutdown that frees the allocation and clears the caller's pointer. It also provides 64 zero-initialized scratch bytes per handle with fixed-width `uint64_t` offset/count copy operations. Reads and writes reject invalid ranges before copying; zero-length copies accept a null buffer, rejected reads preserve their destination, nonzero buffers cannot overlap the probe allocation, and separate handles have separate scratch areas. The C header specifies ownership, pointer alignment, and which calls may overlap. The borrowed context must remain valid until shutdown, and callbacks must return normally. The Rust boundary catches Rust panics and returns `OOT_ABI_PANIC`, but cannot catch C exceptions or a panic originating in an `extern "C"` callback. The callback-failure enum is reserved; this probe has no callback failure channel.

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

Run the same native C/Rust ABI driver with the pinned Rust toolchain on macOS
ARM64 or Linux x86-64 using `bash tools/probes/abi/run_native.sh`. The hosted
[`abi-native.yml`](../../.github/workflows/abi-native.yml) workflow asserts each
runner architecture, links with strict C warnings and executes the driver. The
Linux run supplies the Rust static library's system link dependencies; macOS
uses its native linker defaults.

Compile the freestanding C translation unit for both architectures (requires Clang with both targets and its compiler-provided integer headers):

```sh
clang --target=x86_64-unknown-linux-gnu -ffreestanding -c \
  -std=c11 -Wall -Wextra -Werror -Itools/probes/abi tools/probes/abi/layout.c \
  -o /tmp/oot-abi-probe-x86_64.o
clang --target=aarch64-unknown-linux-gnu -ffreestanding -c \
  -std=c11 -Wall -Wextra -Werror -Itools/probes/abi tools/probes/abi/layout.c \
  -o /tmp/oot-abi-probe-aarch64.o
readelf -h /tmp/oot-abi-probe-x86_64.o /tmp/oot-abi-probe-aarch64.o
```

Each Clang command emits a relocatable ELF object. The headers report ELF64 and the corresponding x86-64 or AArch64 machine. It does not link or run ARM64 code. The hosted link probes exercise the same driver and static library with the mobile SDKs. From the repository root, use `bash tools/probes/abi/link_ios.sh` on a Mac with Xcode and the two Rust iOS targets. Use `bash tools/probes/android/build-android-arm64.sh` on Linux x86-64 with the Android NDK and the Rust Android ARM64 target. Set `ANDROID_NDK_HOME` to the NDK directory. Android defaults to API 24.

The iOS script verifies the Mach-O platform and architecture for device and simulator separately. The Android script checks ELF64, AArch64, and the Android dynamic loader. These are SDK link checks. Device execution, signing, app-local compilation, and selected-base linking remain open.
