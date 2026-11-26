# Code map

| Location | Status | Purpose |
|---|---|---|
| `src/lib.rs` | Present | Checks a single standard Zstandard frame's header, block boundaries and optional checksum presence; does not decompress or verify content or checksum. |
| `src/main.rs` | Present | Runs the preflight on explicit local file paths and returns failure if any cannot be checked. |
| Launcher, setup orchestration, version gate, recompiler integration, runtime and renderers | Planned | See SPEC.md and ROADMAP.md; no implementation or game build exists here yet. |

The checker is a standalone step available to future M6 setup. It is not wired to a launcher or M7 supported-version gate.
