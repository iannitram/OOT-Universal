# Synthetic staged build probe

This crate is an isolated orchestration probe. It compiles only the bundled `fixtures/synthetic.c`, invokes the configured compiler directly from staging with fixed relative filenames, runs the resulting executable, and requires the exact self-test output before publishing. Temporary work lives in a unique 0700 staging directory on Unix below the output root. Successful output is published with a same-filesystem directory rename to a new generation that later probe runs leave untouched; a failed or cancelled run attempts to remove its staging directory. Use a trusted, private output root and a trusted compiler. The compiler inherits the caller environment and may invoke other tools. The probe coordinates with other probe runs sharing the root, while unrelated writers have unrestricted access there.

Cancellation is checked before staging, before compilation, after compilation, and after execution. `Command::output` waits for a child and does not interrupt it while running; cancellation requested during a compiler or executable run is observed only after that child exits. A child that hangs therefore also delays cancellation.

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

No production game-build CLI is exposed.

The compiler must be a host compiler that accepts the fixed options and produces an executable runnable on the current host. This proves only a local synthetic C build and execution. It does not invoke a real game recompiler, consume an `oot-copy` token, build a game library, establish a selected-base interface, or prove mobile toolchain behavior.
