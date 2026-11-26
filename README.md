# OOT-Universal

The Legend of Zelda: Ocarina of Time (Switch 2 remake) from my own copy, natively on SteamOS, Windows, Mac, iPhone/iPad and Android. No game files, keys or firmware are ever in this repo.

**Stage:** see `PIPELINE.md`.

## Start here

- `IDEA.md`: the idea, in my words.
- `SPEC.md`: what it must do, what it won't do, and what 1.0 means.
- `PEERS.md`: the two PC recompilations this repo builds on or learns from.
- `ROADMAP.md` and `docs/code-map.md`: the draft plan and current code layout.
- `AGENTS.md`: rules for anyone, person or agent, changing the code.
- `CHANGELOG.md`: what changed and when.

## Frame preflight

With Rust installed, run `cargo run -- path/to/file.zs` (multiple explicit file paths are accepted). The command exits 0 when each file has the expected frame structure, 1 on a file or framing error, and 2 when no paths are given. It examines one standard frame per file, without decompressing or checking the checksum, game version, or payload correctness; it is not a complete integrity check. It is standalone and not yet integrated into setup. Tests: `cargo test`; lint: `cargo clippy --all-targets -- -D warnings`.
