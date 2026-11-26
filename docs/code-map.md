# Universal build code map

## What exists on audited main

Audit date: 2026-11-26. Baseline: `fdd935716260547c91875617ccbb043bd940b959`, merged peer-reuse PR #3. Read [the roadmap](../ROADMAP.md) for U00–U14 dependencies and M1–M19 acceptance criteria. Only the paths in this section are implemented. All paths and interfaces in later sections are **proposals**, not files, dependencies or stable APIs.

| Existing location | API/entry point | Behavior and limits |
|---|---|---|
| [`Cargo.toml`](../Cargo.toml), [`Cargo.lock`](../Cargo.lock) | `oot-frame-check` 0.1.0, edition 2021, GPL-3.0-only | Single library/binary package with no external dependencies. Not a workspace or application build. |
| [`src/lib.rs`](../src/lib.rs) | `check_frame<R: Read + Seek>(&mut R) -> io::Result<FrameInfo>` | Seeks to start, parses one standard Zstandard frame, checks block limits/bounds, optional checksum bytes and exact end. Skips payload; rejects concatenated/skippable frames. Returns framing results, not proof of decompressibility. |
| [`src/lib.rs`](../src/lib.rs) | `FrameInfo { content_size: Option<u64>, dictionary_id: Option<u32>, has_checksum: bool, blocks: u64 }` | Size/dictionary are declared metadata; checksum flag is presence only. Callers must not treat any field as authenticated. |
| [`src/lib.rs`](../src/lib.rs) | Private `bytes`, `invalid`; four unit tests | Little-endian reads and typed IO errors. Tests metadata, valid layout, truncation, reserved fields/trailing data and block/window limits. |
| [`src/main.rs`](../src/main.rs) | `main() -> ExitCode`; binary `oot-frame-check` | Accepts OS-string file paths, checks regular-file metadata, opens a buffered reader and checks each file. Exit 0 all pass, 1 any file/framing error, 2 no arguments. Prints local paths: do not automatically upload output as diagnostics. |
| [`tests/cli.rs`](../tests/cli.rs) | `checks_local_files_and_returns_failure_for_bad_input` | Synthetic local file integration test, not a game-copy test or all review smoke cases. |
| [`.github/workflows/rust.yml`](../.github/workflows/rust.yml) | `test`, `cross-check` jobs | Desktop fmt/test/Clippy on Linux/Windows/macOS; `cargo check` on Android arm64 and iOS device/simulator arm64. Hosted billing prevented any execution at the audited merge. No mobile linking/package tests. |
| [`PEERS.md`](../PEERS.md) | Imported-code ledger | Source commit and GPL credit for Rust frame walker. Full import branches are reference material; they are not source directories on main. |
| [`docs/adr/README.md`](adr/README.md) | ADR convention | No numbered architecture decision exists yet. Spec/base/UI decisions must not be inferred from proposed names below. |

Current invocation: `cargo run --locked -- path/to/frame.zs [another.zs]`. Current validation: `cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`. These check the existing package only.

## Proposed ownership and layout

Keep the current root package and CLI stable while introducing a Cargo workspace incrementally in U02. The `crates/` layout below is a proposed separation of responsibilities; split or combine crates in an ADR when actual coupling is understood. Do not generate empty crates simply to make the map appear implemented. Preserve the current helper API or provide an explicit migration if moved later.

| Proposed path | Package | Responsibility / code to add | Depends on |
|---|---|---|---|
| `crates/contracts/src/{copy,build,settings,session,error}.rs` | U02 | `CopyIdentity`, `TargetProfile`, `BuildRequest`, versioned `Settings`, `SessionState`, typed `AppError`. Shared data and capabilities only, no toolkit/renderer dependency. | Workspace ADR |
| `crates/copy/src/{inspect,version,assets,extract}.rs` | U03 | Inspect provided copy, exact identity gate, asset-list validation, bounded decompression/checksum verification, safe extraction to staging. Existing frame helper is one optional early check. | contracts; chosen metadata format |
| `crates/build/src/{plan,tools,runner,cache,install}.rs` | U04 | Tool discovery/version verification, recompiler/C compilation plan, subprocess execution, cancellation, private cache and atomic promotion. Distinguish host from target toolchain. | contracts, copy, selected recompiler |
| `third_party/recompiler/` | U01/U04 | Proposed destination for approved, pinned base recompiler and required public support code; preserve license/NOTICE and applicable peer tests. Exact subset chosen after G1. | Base decision and provenance audit |
| `crates/runtime/src/{abi,memory,thread,time,fs,input,audio,video}.rs` | U05 | Rust implementation of required host services, address translation and lifetime control. Exact service list comes from G3; filenames do not imply the base is already compatible. | contracts and ABI ADR |
| `include/oot_runtime.h` | U05 | Public C ABI header: fixed-width types, opaque handles, callbacks and documented ownership/error rules. C header is an interface, not a new C runtime. | ABI contract |
| `crates/render/src/{commands,resources,capabilities,shader,cache}.rs` | U06/U07 | Backend-neutral render operations/resources, capability negotiation, shader input contract and cache identity. Avoid exposing Vulkan-only concepts to the runtime. | G4 and runtime contract |
| `crates/render-vulkan/src/{device,surface,commands,shader,pipeline,texture,sync,present}.rs` | U06 | Rust Vulkan implementation, selected bindings, shader translation/tool integration, texture support, barriers and swapchain recovery. | render, Vulkan probe |
| `crates/render-metal/src/{device,surface,commands,shader,pipeline,texture,sync,present}.rs` | U07 | Native Metal implementation, translation/compiler integration and lifecycle; no assumption that peer SPIR-V alone solves Metal. | render, Metal probe |
| `crates/saves/src/{manifest,relay,snapshot,conflict,progress}.rs` | U08 | save-relay adapter, only save-folder sync, atomic backup/restore, conflicts/offline recovery, safe progress extraction. Do not pretend raw save mtime is completion progress. | contracts, inspected save-relay API |
| `crates/session/src/{launch,lifecycle,update}.rs` | U08/U09 | Single owner of sync-before-play, process/in-process runtime ownership, exit/crash handling and backup-before-update. Do not launch on a failed version gate. | build, saves, runtime |
| `crates/launcher/src/{app,view_model,controller,settings}.rs`, `ui/` | U09 | Continue/setup/settings/saves/about, focus navigation, progress/cancel/retry and persisted launch settings. `ui/` format awaits toolkit ADR. | contracts; session via injectable services |
| `crates/platform/src/{paths,controller,audio,diagnostics}.rs` | U02/U05/U09 | Narrow platform adapters, sandbox storage, input/audio lifecycle and opt-in redacted diagnostics. Replace module topology if needed to avoid platform dependency cycles. | platform probes |
| `crates/steam/src/{shortcut,artwork}.rs` | U10 | Idempotent Steam registration, permitted artwork and Game Mode arguments; preserve unrelated library entries. | launcher and platform paths |
| `platform/{linux,windows,macos,ios,android}/` | U10 / U11 / U12 | Native build/package manifests and lifecycle entry points; signing references rather than credentials. Rust remains application logic; any mandatory platform glue needs a recorded M11 boundary decision. | Selected toolchains and playable components |
| `crates/upscale/src/{fsr,metalfx}.rs` | U13 | Renderer-owned integration hooks and capability/settings validation; actual API boundaries selected with backend owners. FSR source/license pin required before reuse. | native renderers and G6 |
| `tests/{contracts,copy,build,abi,saves,launcher,render}/` | U02–U14 | Proposed test areas; synthetic C caller/shaders/fake copies/relay and subprocess doubles. Do not store proprietary game fixtures. | Corresponding implementation |
| `tools/` and `docs/validation/` | U00/U14 | Rust release/validation tools and sanitized checklists/templates. Keep owner game libraries, game-binary analysis and raw local playthrough evidence outside tracked files/CI. | artifact policy |

## Proposed contracts for implementation briefs

These signatures describe responsibilities, not public APIs that can be called today. Final lifetimes, async policy and naming belong in U02/G3 ADRs. Keep interfaces small enough to test without game files or a renderer.

```rust
// contracts + copy: pure identity decision before installation side effects.
fn inspect_copy(root: &Path) -> Result<CopyIdentity, CopyError>;
fn validate_identity(found: &CopyIdentity) -> Result<SupportedIdentity, CopyError>;
fn validate_copy_assets(root: &Path, identity: &SupportedIdentity)
                        -> Result<SupportedCopy, CopyError>;

// build: explicit host/target tools, cancellation and a non-game diagnostic stream.
fn plan_build(copy: &SupportedCopy, target: &TargetProfile,
              tools: &Toolchain) -> Result<BuildPlan, BuildError>;
fn run_build(plan: &BuildPlan, events: &mut dyn BuildEvents,
             cancel: &Cancellation) -> Result<LocalArtifact, BuildError>;

// saves/session: no renderer dependency; fake implementations in tests.
trait SaveStore {
    fn pull(&mut self) -> Result<SaveSnapshot, SaveError>;
    fn push(&mut self, snapshot: &SaveSnapshot) -> Result<(), SaveError>;
}
fn backup_before_update(saves: &Path) -> Result<BackupReceipt, SaveError>;
```

A `SupportedCopy` token must follow successful exact identity and required-asset checks and bind the build to those validated inputs. Validate a private snapshot or revalidate the inputs before use so replacing the original files cannot bypass the checks. `FrameInfo` can never construct it alone. `LocalArtifact` must carry target, identity, recompiler/runtime ABI and tool/input stamps, and remain in private storage. Build/cache keys must invalidate when any required source/tool/config input changes.

For the C ABI, use a generated or audited header and synthetic C caller. Specify ABI version negotiation, pointer bounds, alignment, ownership and callback lifetime. Do not pass Rust-owned `String`, `Vec` or unwinding panics across C. Test invalid handles, partial initialization and double shutdown. Do not invent exported symbols until the selected recompiler's calling convention has been examined.

The render interface needs resource creation/destruction, upload, draw/compute, synchronization, present, capabilities and recoverable surface/device loss. Share semantics and tests; each backend owns its native resources. Shader translation, guest graphics-state interpretation and texture conversion are explicit engineering tasks, not automatic results of choosing bindings.

## End-to-end flow and failure ownership

1. **Import and validate (U03):** owner chooses local input → inspect exact supported identity → stage selected inputs privately → validate required assets. Reject before installation on unsupported input. Never mutate the source copy.
2. **Build (U04):** verify target capabilities/toolchain → recompile locally → compile/link against versioned runtime ABI → verify target and ABI metadata → promote the staged result atomically. In synthetic tests, separately load and call a fixture artifact through the stub ABI. On cancel/error preserve the last good installation and saves.
3. **Prepare play (U08/U09):** resolve relay state/conflicts/offline behavior → pull saves → apply validated settings/controller mapping → open runtime. Renderer and runtime receive only the handles/paths they need.
4. **Play (U05–U07/U13):** generated C → Rust ABI/services → native render/audio/input backend. No game code is available in CI; automate against synthetic ABI programs and shaders.
5. **Exit/update (U08/U10–U12):** flush and snapshot saves → push or record pending sync → dispose resources. On updates, back up saves before replacement, migrate config separately and allow rollback. Mobile suspension is not a clean desktop process exit.

## Peer reuse boundaries

| Source | Available evidence | Next action / limit |
|---|---|---|
| Relink `tools/dump_check.py` at `54af18c…` | Adapted `check()` is already in `src/lib.rs`; GPL attribution is in source and PEERS.md. | Reuse existing Rust helper; add decompression and identity checks in U03 rather than claim the helper supplies them. |
| Relink reference branch at `16cba084…`: `nx2/build.py`, `nx2/version.py`, `host/`, `gpu/` | Paths inspected in stored `origin/import/relink` tree during audit; this is an older snapshot, not a new live import. | Compare with selected live revision, inspect dependencies/provenance, extract contracts and recompiler subset after G1. `host/` and `gpu/` are reference implementations, not our promised Rust implementation. |
| Longshot reference branch at `fa2cc03a…`: `builder/`, `nx2/build.py`, `nx2/bake.py`, `BAKE.md` | Stored snapshot shows runtime-coupled builder and shader-bake tooling. No Longshot code integrated on main. | Reassess when renderer/cache contracts exist. Do not import caches or game-derived shader material to save a build step. |
| save-relay, totk-universal launcher modules, FSR-Vulkan | Mentioned in SPEC.md; integration APIs and exact reusable files not audited here. | G6 must pin revisions, check licenses and run adapter probes. These are reuse candidates, not installed modules or guaranteed savings. |

## Test and delivery matrix

| Boundary | Automated proof to add | Owner/device proof |
|---|---|---|
| Copy/version/assets | Fake directory/archive metadata; wrong IDs; missing/corrupt assets; extraction path and size limits; checksum/decompression failures | Accepted owner copy on every build route |
| Builder/cache/install | Fake recompiler/compiler; nonzero exit, cancel, restart, disk-full and cache invalidation; synthetic C library link/load | Real local compile time, RAM/storage and recovery on each device route |
| ABI/runtime | C/Rust conformance, memory boundaries, thread shutdown, filesystem sandbox and service mocks | Incremental boot and stable game behavior |
| Graphics | Synthetic shader compile, draw/compute/readback, cache corruption/invalidation, capability fallback; CI only on suitable GPU runners | Real first frame, correctness, driver/device loss and thermal/frame-time soaks |
| Saves/session/update | Fake relay conflicts/offline retry, crash timing, atomic backup/restore and migrations | Restart and cross-device save round trips |
| Launcher/platform | View-model controller actions, focus traversal, setting validation, fake progress/errors and packaging manifests | Controller-first use, Game Mode, mobile background/resume, signing expiry/re-sign |
| Releases | Allowlisted open-source binaries/resources, licenses, no private inputs or generated game libraries; clean install/update tests | Full six-device acceptance and private M1 completion log |

Hosted desktop tests must actually execute before M4 can be complete. Add real mobile linking and packaging with the relevant SDK/toolchain; retain `cargo check` as a cheap earlier stage. A simulator, target check, synthetic first frame and owner gameplay are separate evidence levels. The current GitHub billing failure needs account action, not an invented source workaround.

## Keeping these maps accurate

An implementation PR updates its U-package and M-requirement status, exact files/APIs, provenance and validation evidence in the same diff. Move only the delivered subset from planned to present. Add an issue's exact expected results and dependency links before coding. Keep tool/framework choices provisional until an ADR and runnable probe justify them. Re-audit the baseline SHA after new imports; do not treat this document's proposed directory tree as existing code.
