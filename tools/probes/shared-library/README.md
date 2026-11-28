# Synthetic shared-library loader probe

The authored library retains its fixed-width transform and version exports for
the Android C-only loader. On Linux, the shared object adds the separate
`bridge_library.c` scenario. `bridge_loader.c` dynamically loads it and passes a
version 1 function table from `runtime_bridge.c`.
The host links this bridge to the standalone Rust ABI static library. The C
library calls the Rust create, callback, scratch read/write, and shutdown
functions only through that table; it also checks null and out-of-bounds
requests are rejected without corrupting scratch data. The shared library has
no unresolved Rust ABI symbols and no direct Rust dependency. The original
`loader.c` remains the Android loader and does not require Rust.

On Linux, run from the repository root:

```sh
bash tools/probes/shared-library/run-linux.sh
```

The script requires Rust 1.100.0, builds in a temporary directory, and removes
its outputs on exit. Hosted Linux CI runs both the original loader checks and
the C-library-to-Rust-ABI bridge scenario. From the repository root on Linux
x86-64 with the NDK installed, run
`bash tools/probes/android/build-android-shared-library.sh`. The Android
workflow uses API 24 and checks ELF metadata, symbols and explicit loader
linkage. Android device execution remains open.
