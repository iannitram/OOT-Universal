# Local validation and synthetic fixtures

## Rust checks

The repository pins Rust and Cargo to `1.100.0` in `rust-toolchain.toml`. Rustup
selects that release in the repository and the hosted workflow installs the same
release. From the repository root, reproduce the desktop CI checks with:

```sh
rustup show active-toolchain
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The hosted `cross-check` job also runs `cargo check --locked --target TARGET`
for `aarch64-linux-android`, `aarch64-apple-ios`, and
`aarch64-apple-ios-sim`. Run a target check locally after installing that Rust
target with `rustup target add TARGET`. The workflow assigns the iOS jobs to
macOS. For the current dependency-free crate, a Rust-only target check can
also run on Linux with the target standard library installed. Linking an
iOS app requires the Apple SDK and toolchain. These checks establish Rust
compilation only. Native linking, packaging, signing, installation and
runtime behavior require separate checks.

## Fixture and artifact rules

Tests, probes, and hosted jobs may use only synthetic inputs created for this
repository: minimal generated source programs, shaders, metadata, and fake
directory/archive layouts. They must not use or retain owner game libraries,
game binaries, extracted game assets, decompiled or generated game code, or
game-derived data. Do not put such material in the repository, test fixtures,
workflow caches, logs, or uploaded artifacts. Keep private local validation
evidence outside tracked files and CI artifacts; report only the check names,
tool versions, and pass/fail results in project records.

Synthetic checks demonstrate only the behavior exercised by their generated
inputs. They do not establish compatibility with an owner copy or prove a
playable build. Hosted billing is managed outside this repository; a billing
failure before workflow steps is not a source or test failure.
