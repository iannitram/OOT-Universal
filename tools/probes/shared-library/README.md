# Synthetic shared-library loader probe

The authored library exports a fixed-width `int32_t` transform and a
`uint32_t` version function. The loader takes exactly one explicit library path,
loads it with `dlopen`, resolves and calls the operation with `dlsym`, checks the
version, confirms a missing symbol is rejected, and closes the library. It uses only standard fixed-width C types and the POSIX dynamic-loader
interface; the same two C sources can be compiled with the Android NDK.

On Linux, run from the repository root:

```sh
bash tools/probes/shared-library/run-linux.sh
```

The script builds in a temporary directory and removes its outputs on exit.
Hosted Linux CI runs this script and verifies rejection of a missing library. From the repository root on Linux x86-64 with the NDK installed, run `bash tools/probes/android/build-android-shared-library.sh`. The Android workflow uses API 24 and checks ELF metadata, symbols and explicit loader linkage. Android device execution remains open.
