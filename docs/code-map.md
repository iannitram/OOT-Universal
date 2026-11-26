# Code map

| Location | Status | Purpose |
|---|---|---|
| [`src/lib.rs`](../src/lib.rs), `check_frame<R: Read + Seek>()` and `FrameInfo` | Present | Reads a single frame from the start of seekable input. Checks the header, block sizes and boundaries, and space for an optional checksum. Returns declared content size, dictionary ID, checksum presence, and block count. It skips the compressed payload. |
| [`src/main.rs`](../src/main.rs), `oot-frame-check` | Present | Runs `check_frame` for explicit local regular files. Exits 0 on success, 1 on any file or framing error, and 2 when no inputs are supplied. |
| [`src/lib.rs`](../src/lib.rs), unit tests | Present | Synthetic frames exercise metadata, valid block layout, truncation, reserved fields, trailing data, and the block-size limit. |
| [`tests/cli.rs`](../tests/cli.rs), integration test | Present | Runs the compiled command with synthetic local files and checks its exit status and errors. |
| [`.github/workflows/rust.yml`](../.github/workflows/rust.yml) | Present | Desktop Rust checks and Android and iOS target checks are configured. GitHub blocked hosted execution at billing. |
| Launcher, M6 setup orchestration, M7 version gate, recompiler integration, runtime and renderers | Planned | See the draft [roadmap](../ROADMAP.md). The preflight stands alone. |
