# Universal build roadmap

## Audited position — 2026-11-26

Audited `main` at `fdd935716260547c91875617ccbb043bd940b959` ([PR #3](https://github.com/iannitram/OOT-Universal/pull/3)). This document expands the plan; it does not implement the proposed modules. The project remains in **Spec**. The spec and full base choice still await owner approval and the benchmark in [SPEC.md](SPEC.md).

**We are at a standalone setup helper, before a game-build prototype.** The only executable on main is the dependency-free Rust `oot-frame-check`. It adapts ProjectReLink's `tools/dump_check.py` frame walker at `54af18cda249233a9a9aafcddf5dc317cb3fad42`. It checks one standard Zstandard frame's structure, skips the payload, and neither decompresses nor verifies checksums, game identity or asset completeness. It is not called by setup because setup does not exist.

The full Relink and Longshot histories on `import/relink` and `import/longshot` are reference branches, not integrated code. No Longshot implementation was merged. [PEERS.md](PEERS.md) records exact imported-code provenance. There is no launcher, recompiler integration, Rust runtime, renderer, save-relay integration, application package or release on audited main.

### Evidence and limits

| Evidence | What it establishes | What it does not establish |
|---|---|---|
| `cargo test --locked`: four library tests and one CLI integration test | Synthetic frame metadata, truncation, invalid fields, block limits and basic CLI behavior | Asset decompression, full copy verification or game compatibility |
| PR #3 review: Linux build/fmt/strict Clippy; ten `zstd`-generated frames and malformed-input smoke checks | The small preflight works on those inputs | A permanent comprehensive codec test suite; smoke cases are not all checked-in tests |
| PR #3 review: local Windows MSVC, Apple silicon macOS, arm64 iOS and Android `cargo check` | The checker type-checks for those targets | Native execution, linking, packaging, renderer support or mobile game compilation |
| `.github/workflows/rust.yml` | Desktop Rust test/lint and Android/iOS target-check jobs are configured | Green hosted CI: [merged-main run](https://github.com/iannitram/OOT-Universal/actions/runs/45897135519) failed all six jobs before any steps, at GitHub billing |

Statuses below mean **present** (source exists), **partial** (only the named subset exists), **planned** (no implementation) or **blocked** (named prerequisite unresolved). A requirement is done only when its own acceptance checks pass; a Rust target check is not a platform port.

## Decisions and feasibility gates

These gates precede a broad runtime or UI rewrite. Record outcomes in numbered ADRs under `docs/adr/`, including a reproducible synthetic probe and target/tool versions. Proposed choices below are not installed dependencies or approved decisions.

| Gate | Work and tools | Required exit evidence |
|---|---|---|
| G1 Base and provenance | Benchmark both candidates on the owner devices using the spec's scene/settings/cap. Pin chosen source, dependencies, licenses and applicable upstream tests. Inspect `nx2/build.py`, generated-C assumptions, `host/`, `gpu/` and build manifests. | Owner base choice, recorded benchmark and ADR; file-level reuse ledger and compatibility boundary. Do not replace main with either import branch beforehand. |
| G2 ARM64 recompiler output | Run the selected C++/Python recompiler against a synthetic program. Compile emitted C with Clang for x86-64 and ARM64; identify AVX2 intrinsics, pointer/address assumptions, threading/TLS and host-specific calls. | Synthetic generated code links and runs on desktop ARM64; mobile linking probe; list of required portable replacements. Emitting C alone proves no portability. |
| G3 Rust ABI/runtime contract | Extract the selected base's public calling conventions and host-service categories without committing game-derived analysis. Design fixed-width C types, opaque handles, ownership and error behavior. | Synthetic C caller links against a Rust runtime stub, tests callbacks and memory, and shuts down without unwinding over FFI. |
| G4 Native graphics path | Determine shader input/translation stages, mesh-shader dependence, texture formats and synchronization semantics. Probe Vulkan on Adreno and Metal on Apple silicon with synthetic shaders. | One representative draw and compute operation per backend; explicit fallback or blocker for unsupported mesh/features. A Vulkan wrapper cannot implement a Metal shader translator. |
| G5 Per-device build and signing | Android: prove an app-accessible local compiler/toolchain and output loader, memory/storage budget and lifecycle recovery. iOS: prove per-device artifact creation on the Mac, linking into a signed app and cable install. | Synthetic local game-library build and load on Android; signed synthetic C/Rust app on an iPhone/iPad. Resolve the spec's wording conflict before implementation: M6 says each device builds, while its iOS platform row uses the Mac toolchain. No remote build hub is implied. |
| G6 Shared integrations and UI | Inspect actual save-relay/launcher/FSR APIs and licenses at pinned revisions; choose UI and controller backends after desktop/mobile focus tests. Slint is a candidate, not a selection. | Small runnable integration probes, compatibility/licensing record and UI ADR; no copying whole sibling applications. |

The iOS path must establish code-signing, application sandbox access, local game-library inclusion, signature-expiry reporting and re-sign workflow on a free Apple ID. Do not assume desktop-style dynamic loading, on-device compilation, JIT entitlements or unattended signing. App Store distribution is not a 1.0 requirement; App Store review rules are not evidence that cable-installed development signing will or will not work.

## Dependency-ordered work packages

`Uxx` identifies a build work package; `Mxx` remains the original Must in SPEC.md. These are planning IDs, not claims that implementation issues already exist. Create focused issues using the repository template as a package is selected, with exact paths, expected results and prerequisite issue links. Each package may need several separately reviewable PRs.

| Package | Depends on | Code and tools to build | Acceptance gate | Musts |
|---|---|---|---|---|
| U00 Build evidence and repository guardrails — partial | Existing checker | Restore hosted execution through account administration; retain current Cargo checks. Add versioned toolchain policy, synthetic-fixture conventions and release allowlist checks as release tooling arrives. | Desktop jobs execute and pass; mobile target checks pass; no game input or compiled game library in CI artifacts. Existing checker alone does not satisfy M4. | M4, M10 |
| U01 Portability and base decisions — planned | Owner access for G1; synthetic probes can start now | G1–G6 probes, dependency/license inventory and ADRs. Preserve the recompiler rather than rewrite it in Rust. | Recorded base decision and target feasibility evidence, or explicit blocked items and spec decision requests. | M6, M11–M13, M16–M18 |
| U02 Workspace and shared contracts — planned | U00 policy; G3 contract draft | Preserve current package/API; introduce workspace members incrementally. Define typed errors, private storage layout, versioned config and target capability records. Use Cargo; serialization library choice remains open. | Existing CLI stays working; contract serialization round trips; no game-dependent build scripts. | M4–M8, M10–M11 |
| U03 Copy/version gate and asset validation — partial helper only | U02; selected base metadata format from U01 | Rust copy inspector, exact program/update/build validation, bounded extraction, real Zstandard decompression/checksum verification where needed. Call the existing `check_frame` only for files matching its single-frame contract. | Fake copies accept only `0400486024DEC000` / `v131072` / `e0f3ccfa`; reject traversal, wrong identity and damaged required assets before installing anything. Preflight is not the acceptance authority. | M6–M7, M10 |
| U04 Local build orchestration — planned | U03, G1–G3; G5 for mobile | Rust job state machine invokes pinned peer recompiler and native C compiler; progress/cancellation, staging, reproducible input/tool stamps, invalidation and atomic promotion. Retain C++/Python only inside the base recompiler boundary. | Synthetic input builds a callable library; interruption leaves last good installation intact; changed input/toolchain invalidates cache; one real build per target later. | M6, M11, M19 |
| U05 Rust runtime and C ABI — planned | G3, U02; U04 for game boot | Memory/address translation, threads/synchronization, timing, filesystem/save calls, input, audio/video services and diagnostic/error boundary. Derive the exact required host-call inventory from the chosen base. | Synthetic ABI conformance and service tests; boot at each incremental replacement on owner copy. Do not remove an existing service until equivalent behavior is proved. | M9, M11 |
| U06 Render interface and Vulkan backend — planned | G4, U05 contracts | Rust render command/resource model; Vulkan binding and surface integration, shader translation, textures, barriers, pipeline cache and presentation. `ash` is a candidate. | Synthetic draw/compute/readback tests, validation clean, then owner first frame on Windows and SteamOS; Adreno feature audit. | M3, M9, M12, M14–M15, M18 |
| U07 Metal backend — planned | G4, U05 contracts; can parallel U06 after shared interface | Rust Metal bindings, shader translation to supported Metal input, texture conversions, resource synchronization and presentation. Evaluate bindings and shader tools against the probe, not API names alone. | Synthetic draw/compute/readback on Mac, then signed iOS first frame and lifecycle test; no Vulkan backend masquerading as native Metal. | M12, M16–M17 |
| U08 Saves and safe session lifecycle — planned | U02, G6; U05 filesystem boundary | Save-relay adapter/manifest, per-platform save roots, atomic snapshots, conflict and offline handling; sync before launch and after clean exit, preserve dirty state after crash. Save progress parser or explicit unavailable state. | Fake relay and filesystem tests; safe offline retry; backup/restore and version migration tests; owner save round trip on all six devices. Never sync caches, keys or game libraries. | M2, M5, M19 |
| U09 Controller-first launcher and settings — planned | U02, G6; wire U03–U08 as available | Continue, Setup, Settings, Saves, About; testable view models, setup progress/errors, controller navigation, resolution/upscaler/cap and mappings, optional extras off by default. UI framework chosen by ADR. | Synthetic saves and fake build/runner tests cover controller focus and lifecycle; saved settings reach runtime; owner navigation on every platform. | M5, M8 |
| U10 Desktop packaging and Steam integration — planned | U04–U06, U08–U09 | Linux/SteamOS bundle and Windows package, native paths, updater, controller mapping, Steam shortcut and owner-provided/permitted artwork. Add application release allowlists. | Clean installs/updates with saves intact; Game Mode hour on both handhelds; Windows controller hour; packages contain no game material. | M10, M14–M15, M19 |
| U11 Apple packaging and lifecycle — blocked by feasibility | G2, G5, U04–U05, U07–U09 | Apple silicon Mac app; target-specific C linking, Xcode SDK/signing, iOS bundle, cable install/re-sign support, sandbox imports, controller APIs and pause/resume. | Native Mac play; iPhone/iPad install, all mapped buttons, resume and re-sign after actual expiry. Mac-generated artifact remains private and per-device; no build hub or redistribution. | M6, M16–M17, M19 |
| U12 Android packaging and local build — blocked by feasibility | G2, G4–G5, U04–U06, U08–U09 | Android app shell/entry, SDK/NDK linkage, Gradle package, Storage Access Framework import, validated app-local compiler and generated-library loading; controller and surface lifecycle. | Adreno device builds from owner copy and plays; rotation/background/surface loss and storage pressure preserve state. Host NDK compilation alone is not on-device compilation. | M6, M18–M19 |
| U13 Upscaling, caches and frame pacing — planned | U06–U07; G6 for reused FSR code | FSR Rust-renderer integration on Vulkan, MetalFX on Apple, cache stamps keyed by backend/device/driver/shader/tool version, warm-up and invalidation, 30/60 caps and frame-time capture. | Native-resolution baseline before upscaling; 30-minute handheld soaks at target resolution; device-specific cache invalidation tests. Frame generation is not required for 1.0. | M3, M8, M13 |
| U14 Compatibility and completion — planned | Relevant playable packages U05–U13 | Chosen-base open-bug inventory at approval, one test/repro per defect, private playthrough journal, completion criteria and release checklist. | All known play-breaking issues fixed or owner-ruled nonreproducible; M1 100% playthrough and all device/save/performance checks complete. | M1–M3, M9–M10 |

Critical path: **U00 + U01 → U02/U03 → U04/U05 → U06/U07 → playable platform packages → U13/U14**. Saves and launcher work can proceed against fakes after shared contracts exist. Begin mobile feasibility probes during U01, not after desktop completion. No dates are assigned before those probes bound the work.

## Toolchains and delivery evidence

| Platform | Proposed development/build tools | Runtime integration still required | Release gate |
|---|---|---|---|
| Shared Rust | Rust/Cargo, rustfmt, Clippy; pinned recompiler Python and C++ toolchain after base selection; Clang/CMake/Ninja only where the actual build requires them | Rust crate graph, C ABI and target-neutral contracts | All core tests on Linux, Windows and macOS; no game-dependent CI |
| SteamOS/Linux x86-64 | Native C compiler/linker; Vulkan SDK/tools and validation layers in development; RenderDoc candidate for synthetic graphics diagnostics | Vulkan surfaces, audio/input, game-library loading and Steam registration | Both handhelds, native screen size, Game Mode controls and soak |
| Windows x86-64 | Rust MSVC target, Visual Studio Build Tools/Windows SDK or a verified compatible selected-base toolchain; Vulkan development tools | DLL loading, paths, input/audio and packaging | Native Windows build/install and controller hour |
| macOS ARM64 | Xcode command-line tools/SDK, Clang, Rust Apple silicon target, Metal compiler and Xcode GPU tooling | Native Metal, app paths and bundle lifecycle | Linked app and native owner session; no Rosetta |
| iOS/iPadOS ARM64 | Mac with Xcode/iOS SDK, Rust iOS target, provisioning and code signing; simulator separate from device | Signed bundle linkage, sandbox access, controller and foreground/background lifecycle | Physical device install, play and seven-day re-sign check |
| Android ARM64 | Android SDK/NDK, Gradle and `adb` for app development; Rust Android target; separately prove compiler usable on device | App entry/surface lifecycle, Vulkan, file picker, local build and library load | On-device build and Adreno play; APK host cross-build is insufficient |

UI/backend/compiler packages must be version-pinned when selected, with licenses and platform versions recorded. Candidate UI: Slint (sibling alignment); candidates for Rust Vulkan/native Metal access require G4/G6 probes. No additional library is currently declared in Cargo.toml.

## Requirement coverage and immediate queue

| Must | Work packages | Audited status |
|---|---|---|
| M1 100% completion | U14 | Planned |
| M2 one save everywhere | U08, U14 | Planned |
| M3 handheld frame rate | U06, U13, U14 | Planned |
| M4 verified merges | U00, all packages | Partial: workflow exists; hosted execution blocked |
| M5 Continue screen | U08–U09 | Planned |
| M6 per-device build | U01, U03–U04, U11–U12 | Partial: standalone frame helper only |
| M7 exact supported version | U03 | Planned |
| M8 settings and extras | U09, U13 | Planned |
| M9 known play-breaking bugs | U05–U07, U14 | Planned |
| M10 no game material | U00, U03, U10, U14 | Ongoing review; automated release gate planned |
| M11 Rust and C ABI | U01–U02, U04–U05 | Rust helper present; runtime/build integration planned |
| M12 native renderers | U06–U07 | Planned |
| M13 upscaling | U13 | Planned |
| M14 handheld Steam integration | U10 | Planned |
| M15 Windows controller play | U10 | Planned |
| M16 native Mac | U11 | Planned; feasibility gate open |
| M17 signed iOS/controller/re-sign | U11 | Planned; feasibility gate open |
| M18 Android local build/play | U12 | Planned; feasibility gate open |
| M19 safe updates | U04, U08, U10–U12 | Planned |

Next: (1) restore hosted CI execution (account action; no source fix established), (2) open bounded U01 probe issues, (3) run the owner base benchmark, (4) decide the ABI/storage/UI boundaries and introduce U02 contracts, (5) implement U03 acceptance checks before any real install. Synthetic work that does not depend on base choice can proceed while the owner benchmark is pending.

## References for proposed tools

Tool documentation was checked during this audit; it is guidance, not proof that our port works. [Rust iOS target documentation](https://doc.rust-lang.org/stable/rustc/platform-support/apple-ios.html) explains target/SDK requirements. [Android NDK CMake documentation](https://developer.android.com/ndk/guides/cmake) covers host-driven native app builds, not our unproved on-device compiler. [Slint platform/licensing FAQ](https://slint.dev/faqs) establishes it as a candidate to probe. Use exact selected versions in implementation ADRs.
