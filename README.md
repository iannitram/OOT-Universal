# OOT-Universal

The Legend of Zelda: Ocarina of Time (Switch 2 remake) from my own copy, natively on SteamOS, Windows, Mac, iPhone/iPad and Android. No game files, keys or firmware are ever in this repo.

**Stage:** see `PIPELINE.md`.

## Start here

- `IDEA.md`: the idea, in my words.
- `SPEC.md`: what it must do, what it won't do, and what 1.0 means.
- `PEERS.md`: the two PC recompilations this repo builds on or learns from.
- `ROADMAP.md`: audited progress, dependency-ordered universal build packages, tools and device gates.
- `docs/code-map.md`: existing APIs, proposed modules/contracts, data flow and tests.
- `AGENTS.md`: rules for anyone, person or agent, changing the code.
- `CHANGELOG.md`: what changed and when.

## Frame preflight

With Rust installed, run `cargo run --locked -- path/to/file.zs`. You can pass multiple regular files. The command exits 0 when each file passes the framing checks, 1 on a file or framing error, and 2 when no paths are given. The preflight examines one standard frame per file and skips the payload. Decompression, checksum verification, game version checks, and integration with setup are separate work.

Run `cargo test --locked` for tests and `cargo clippy --locked --all-targets -- -D warnings` for lint.
