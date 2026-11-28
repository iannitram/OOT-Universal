# Synthetic staged build probe

This crate is an isolated orchestration probe. It compiles only the bundled `fixtures/synthetic.c`, invokes the configured compiler directly from staging with fixed relative filenames, runs the resulting executable, and requires the exact self-test output before publishing. After the self-test succeeds, staging receives an `artifact.manifest` containing format version 1, fixed fixture ID `oot-build-probe.synthetic-c.v1`, executable byte length, and lowercase SHA-256. The record contains no host paths. `verify_generation` strictly validates the record and executable bytes with a 256-byte metadata cap and a 64 MiB executable hashing cap, returning typed metadata, size, digest, and I/O failures. Temporary work lives in a unique 0700 staging directory on Unix below the output root. Successful output is published with a same-filesystem directory rename to a new generation that later probe runs leave untouched; cancellation is checked before publication, and a failed or cancelled run attempts to remove its staging directory. Use a trusted, private output root and a trusted compiler. The compiler inherits the caller environment and may invoke other tools. The probe coordinates with other probe runs sharing the root, while unrelated writers have unrestricted access there.

`build_synthetic_snapshot(&BuildConfig, &OwnedSyntheticAcceptance, cancelled)` adds a separate synthetic path. It writes the snapshot's already-validated bytes as `fixture/manifest.bin` and `fixture/payload.bin` under the private staging directory, then compiles `fixtures/snapshot.c` with that directory as its current working directory. The C fixture compares every byte against the authored fixture, checks EOF after each expected byte sequence, and emits `synthetic-snapshot-ok` only on success. The two fixture files remain in the published generation for inspection. Its manifest uses fixture ID `oot-build-probe.snapshot-c.v1`; `verify_generation` accepts this ID and the original `oot-build-probe.synthetic-c.v1`. The verifier checks only manifest structure and the executable's size and SHA-256. The compiled snapshot fixture performs the asset readback check before publication.

Cancellation is checked before staging, before and after compilation, while the synthetic executable runs, and immediately before publication. The compiler is not interrupted. During execution, cancellation kills and reaps the direct child; descendants are not managed. Output is captured in temporary staging files and removed before publication, avoiding pipe backpressure.

Run the isolated crate checks from the repository root:

```sh
cargo fmt --manifest-path crates/build/Cargo.toml -- --check
cargo test --manifest-path crates/build/Cargo.toml
cargo clippy --manifest-path crates/build/Cargo.toml --all-targets -- -D warnings
```

Run an actual host compiler smoke build (Linux example, using an explicit compiler path):

```sh
cargo run --manifest-path crates/build/Cargo.toml --example smoke -- /usr/bin/cc /tmp/oot-synthetic-output
```

The hosted Windows job runs the same smoke example with Clang and an output path containing spaces, then runs the published executable. From the repository root, run `./tools/probes/build/run-windows-smoke.ps1` in PowerShell to reproduce it.

The job also runs a native regression test that requires a compiler. To run just that test, set `OOT_BUILD_TEST_COMPILER` to the absolute path of a compatible host compiler and run:

```sh
cargo test --locked -p oot-build-probe --test native_failures -- --ignored
```

The tests check that compiler exit, a staged executable failure, incorrect output, and cancellation after compilation, during execution, or before publication each preserve the prior generation and remove staging. Native Windows coverage verifies the record and rejects altered or truncated metadata and executable files. Tests also check that simultaneous successful builds publish two separate runnable generations to one output root.

No production game-build CLI is exposed.

The compiler must be a host compiler that accepts the fixed options and produces an executable runnable on the current host. This proves only a local synthetic C build and execution. It does not invoke a real game recompiler, consume an `oot-copy` token, build a game library, establish a selected-base interface, or prove mobile toolchain behavior.
