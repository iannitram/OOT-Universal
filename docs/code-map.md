# Universal build code map

## What exists on audited main

Audit date: 2026-11-26. Baseline: `fdd935716260547c91875617ccbb043bd940b959`, merged peer-reuse PR #3. Read [the roadmap](../ROADMAP.md) for shared base work, Android, iOS/iPadOS, macOS, Windows and Linux/SteamOS gates. [The peer survey](../PEERS.md#platform-peer-survey--2026-11-26) records external evidence separately from code present here. This section inventories implemented paths at the audited baseline plus committed follow-up changes in this branch. All paths and interfaces in later sections are **proposals**, not files, dependencies or stable APIs.

| Existing location | API/entry point | Behavior and limits |
|---|---|---|
| [`Cargo.toml`](../Cargo.toml), [`Cargo.lock`](../Cargo.lock) | Workspace with root `oot-frame-check` and `oot-contracts`, edition 2021, GPL-3.0-only | Dependency-free Cargo workspace. The root package and CLI remain intact; this is not an application build. |
| [`crates/contracts/src/copy.rs`](../crates/contracts/src/copy.rs) | `CopyIdentity::new` and normalized accessors | Validates 16-hex program ID, `v` plus decimal version and 8-hex build ID; normalizes hexadecimal case and decimal leading zeroes. This is syntax validation only, not supported-version policy. |
| [`crates/contracts/src/target.rs`](../crates/contracts/src/target.rs) | `TargetProfile::new`, architecture/platform accessors, `family` | Keeps architecture distinct from platform and classifies desktop/mobile. Recognizes planned platform and probe combinations, including Linux ARM64 for portability probes. No ABI, device/simulator distinction or runtime capability guarantee. |
| [`src/lib.rs`](../src/lib.rs) | `check_frame<R: Read + Seek>(&mut R) -> io::Result<FrameInfo>` | Seeks to start, parses one standard Zstandard frame, checks block limits/bounds, optional checksum bytes and exact end. Skips payload; rejects concatenated/skippable frames. Returns framing results, not proof of decompressibility. |
| [`src/lib.rs`](../src/lib.rs) | `FrameInfo { content_size: Option<u64>, dictionary_id: Option<u32>, has_checksum: bool, blocks: u64 }` | Size/dictionary are declared metadata; checksum flag is presence only. Callers must not treat any field as authenticated. |
| [`src/lib.rs`](../src/lib.rs) | Private `bytes`, `invalid`; four unit tests | Little-endian reads and typed IO errors. Tests metadata, valid layout, truncation, reserved fields/trailing data and block/window limits. |
| [`src/main.rs`](../src/main.rs) | `main() -> ExitCode`; binary `oot-frame-check` | Accepts OS-string file paths, checks regular-file metadata, opens a buffered reader and checks each file. Exit 0 all pass, 1 any file/framing error, 2 no arguments. Prints local paths: do not automatically upload output as diagnostics. |
| [`tests/cli.rs`](../tests/cli.rs) | `checks_local_files_and_returns_failure_for_bad_input` | Synthetic local file integration test, not a game-copy test or all review smoke cases. |
| [`.github/workflows/rust.yml`](../.github/workflows/rust.yml), [`rust-toolchain.toml`](../rust-toolchain.toml) | `test`, `cross-check` jobs; Rustup toolchain selection | Both jobs use Rust 1.100.0. Desktop fmt/test/Clippy runs on Linux/Windows/macOS; `cargo check` runs on Android arm64 and iOS device/simulator arm64. Local reproduction and synthetic-fixture rules: [`docs/validation/local-checks.md`](validation/local-checks.md). Hosted billing is external; no mobile linking/package tests. |
| [`PEERS.md`](../PEERS.md) | Imported-code ledger | Source commit and GPL credit for Rust frame walker. Full import branches are reference material; they are not source directories on main. |
| [`docs/adr/README.md`](adr/README.md) | ADR convention | No numbered architecture decision exists yet. Spec/base/UI decisions must not be inferred from proposed names below. |

Current invocation: `cargo run --locked -- path/to/frame.zs [another.zs]`. Current validation: `cargo fmt --all --check`, `cargo test --locked --workspace`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and workspace target checks. Run them with pinned Rust 1.100.0; see [`docs/validation/local-checks.md`](validation/local-checks.md). No settings persistence or serialization format is implemented.

### Platform support in the open PR stack

- [`desktop_exec`](../crates/platform-support/src/desktop_exec.rs) generates one XDG Exec value for an absolute executable path. The example emits a synthetic desktop entry for validator checks. Desktop installation and Steam registration are planned.

- [`audit_m1_macros`](../crates/platform-support/src/apple_cpu.rs) evaluates Clang macro output for the donor’s Apple ARM64 M1 floor. Run `clang -target arm64-apple-ios -mcpu=apple-m1 -dM -E -x c - </dev/null | cargo run --locked -p oot-platform-support --example apple_cpu`. Use `arm64-apple-macos` for a macOS preprocessor check. SDK linking, signing and device support have separate gates.

## Proposed ownership and layout

The workspace keeps the current root package and CLI stable. Add workspace members as their responsibilities become concrete. The `crates/` layout below is a proposed separation of responsibilities; split or combine crates in an ADR when actual coupling is understood. Do not generate empty crates simply to make the map appear implemented. Preserve the current helper API or provide an explicit migration if moved later.

| Proposed path | Package | Responsibility / code to add | Depends on |
|---|---|---|---|
| `crates/contracts/src/{build,settings,session,error}.rs` | U02 | Add build requests, settings, session state and an operational error model. No toolkit/renderer dependency. | Workspace; G3 decisions for later contracts |
| `crates/copy/src/lib.rs` | U03 (identity gate) | Pure exact program/update/build identity validation over `CopyIdentity`, returning an opaque `SupportedIdentity` token or a field-specific mismatch. Does not inspect copies, validate assets, decompress, or extract. | contracts |
| `crates/build/` | U04 | Current synthetic probe compiles only its bundled C fixture with an explicit host compiler path, executes and validates it, and atomically publishes verified output from private staging. Direct process args; cancellation is checked around but does not interrupt a running child. No `oot-copy` token is consumed on this synthetic route. | U03 remains on the roadmap sequence; selected recompiler and real build integration are pending |
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
| `tools/probes/abi/` | U01/G3 | Base-neutral Rust static library, narrow C header, C caller and freestanding C translation unit; validates fixed-width call, callback and ownership/shutdown behavior only. Commands and evidence limits are in its README. | selected-base ABI extraction and owner/mobile gates remain pending |
| `tools/` and `docs/validation/` | U00/U14 | Planned Rust release allowlist checker and owner validation templates. The local-check guide is already listed above. | release tooling and artifact policy |

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
