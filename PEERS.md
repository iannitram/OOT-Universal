# Peers

The projects doing the same work as this repo: the static recompilations of *The Legend of Zelda: Ocarina of Time* (Switch 2 remake) that run natively on a PC from your own copy. The machine-read list is the `peer:` lines in [SPEC.md](SPEC.md#prior-art), under `## Prior art`; this page has the notes. Named by me 2026-11-21 ("link them").

**My ranking:** Relink is the best performance for me; Longshot is second, with the most contributors.

| Peer | Licence | What to take |
|---|---|---|
| pixl27/ProjectReLink (Relink) | GPL-3.0 | Best performance for me. A native Windows build from the Switch 2 remake, with a launcher and shader and pipeline-cache work. Branches to watch: `build-speed`, `fsr4`, `hdr-present`, the shader branches. Base candidate. |
| Recomper/Longshot (Longshot) | GPL-3.0 | Second on performance, and the most contributors (103 on GitHub). Windows and Linux builds, an in-game menu, frame generation and upscaling. Its README says it is a fork of Hookshot by NotRexed. |

Neither peer's README names a Mac, iPhone/iPad or Android build. Those routes are still to find; the weekly peer review will look for them.

**Licences.** Both peers are GPL-3.0. This repo is GPL-3.0 too (my answer, 2026-11-21; LICENSE file added), so their code may reach `main`, with credit. Until a base is approved, the import branches hold their code unchanged, and nothing from them goes into `main`.

## Import branches

| Branch | Source | Source commit |
|---|---|---|
| `import/relink` | pixl27/ProjectReLink, `main` | 16cba0845075868d211c63674156fa37401d852e |
| `import/longshot` | Recomper/Longshot, `main` | fa2cc03aa5c2369dea53856d4b11bdb9d0299b3b |

Each branch carries its peer's full history as fetched from GitHub; nothing is edited. The peers' other branches are not imported.

## Best base candidates

1. **pixl27/ProjectReLink** (GPL-3.0): the best performance for me; the base to start from.
2. **Recomper/Longshot** (GPL-3.0): second on performance, the biggest contributor base; the second source to compare against.
