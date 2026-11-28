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
Hosted Linux CI runs this script and verifies rejection of a missing library. To build the same sources for Android,
use the NDK's `aarch64-linux-android<api>-clang` driver with
`-std=c11 -Wall -Wextra -Werror -fPIC -shared` for `library.c`, and compile
`loader.c` as an executable. Both Android and Linux provide `dlopen` through
their platform C library; Linux additionally links the loader with `-ldl`.
