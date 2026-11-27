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
| G2 ARM64 recompiler output | Probe candidate C++/Python recompilers against a synthetic program before selection. Repeat the probe with the pinned base before integration. Compile emitted C with Clang for x86-64 and ARM64; identify AVX2 intrinsics, pointer/address assumptions, threading/TLS and host-specific calls. | Synthetic generated code links and runs on desktop ARM64; mobile linking probe; list of required portable replacements. Emitting C alone proves no portability. |
| G3 Rust ABI/runtime contract | Extract the selected base's public calling conventions and host-service categories without committing game-derived analysis. Design fixed-width C types, opaque handles, ownership and error behavior. | Synthetic C caller links against a Rust runtime stub, tests callbacks and memory, and shuts down without unwinding over FFI. |
| G4 Native graphics path | Determine shader input/translation stages, mesh-shader dependence, texture formats and synchronization semantics. Probe Vulkan on Adreno and Metal on Apple silicon with synthetic shaders. | One representative draw and compute operation per backend; explicit fallback or blocker for unsupported mesh/features. A Vulkan wrapper cannot implement a Metal shader translator. |
| G5 Per-device build and signing | Android: prove an app-accessible local compiler/toolchain and output loader, memory/storage budget and lifecycle recovery. iOS: prove per-device artifact creation on the Mac, linking into a signed app and cable install. | Synthetic local game-library build and load on Android; signed synthetic C/Rust app on an iPhone/iPad. Resolve the spec's wording conflict before implementation: M6 says each device builds, while its iOS platform row uses the Mac toolchain. No remote build hub is implied. |
| G6 Shared integrations and UI | Inspect actual save-relay/launcher/FSR APIs and licenses at pinned revisions; choose UI and controller backends after desktop/mobile focus tests. Slint is a candidate, not a selection. | Small runnable integration probes, compatibility/licensing record and UI ADR; no copying whole sibling applications. |

The iOS path must establish code-signing, application sandbox access, local game-library inclusion, signature-expiry reporting and re-sign workflow on a free Apple ID. Do not assume desktop-style dynamic loading, on-device compilation, JIT entitlements or unattended signing. App Store distribution is not a 1.0 requirement; App Store review rules are not evidence that cable-installed development signing will or will not work.

## Base code

Choose the base after the owner benchmark. Pin the approved peer revision and inventory its recompiler, host interfaces, applicable tests and dependency licenses. Keep copy identification, asset validation, build orchestration, runtime services, renderer contracts, saves and launcher state reusable across every platform.

The foundation pins Rust and includes a synthetic C/Rust ABI probe, shared Cargo contracts, a pure copy identity gate and a synthetic staged build. Platform helpers and the G2 probe extend this synthetic evidence. Real copy inspection and asset validation, selected-recompiler integration, runtime services and game boot still need implementation.

| Package | Depends on | Code and tools to build | Acceptance gate | Musts |
|---|---|---|---|---|
| U00 Build evidence and repository guardrails — partial | Rust 1.100.0 pin; local-check and synthetic-fixture policy | Rustup and both hosted jobs use the same pinned release; reproduce desktop checks from `docs/validation/local-checks.md`. A [hosted matrix run](https://github.com/iannitram/OOT-Universal/actions/runs/45928137542) has passed; add release allowlist checks as release tooling arrives. | Local desktop checks pass; hosted desktop and mobile target checks execute and pass; no game input or compiled game library in CI artifacts. Existing checker alone does not satisfy M4. | M4, M10 |
| U01 Portability and base decisions — partial | Owner access for G1; synthetic probes can start now | Added the base-neutral C/Rust ABI smoke probe at `tools/probes/abi/`. The [G2 probe](tools/probes/g2/README.md) checks synthetic ADD, conditional branch and bounded store/load cases with both pinned candidate emitters on Windows x86-64 and hosted macOS ARM64, and compiles freestanding AArch64 objects. G1–G6 probes, dependency/license inventory and ADRs remain. Preserve the recompiler rather than rewrite it in Rust. | Native synthetic callback/ownership probe passes; x86-64 and AArch64 C translation units compile. The full lift pipeline, selected-base ABI, mobile linking, owner base choice and other target feasibility evidence remain pending. | M6, M11–M13, M16–M18 |
| U02 Workspace and shared contracts — partial | U00 policy; G3 contract draft | Workspace now preserves the root checker and adds dependency-free `oot-contracts` copy-identity syntax and target-profile types. Typed operational errors, private storage, versioned settings and serialization remain unimplemented. | Root CLI remains callable; workspace packages check/test/lint with no game-dependent build scripts. Contract persistence round trips remain open until a serialization contract is selected. | M4–M8, M10–M11 |
| U03 Copy/version gate and asset validation — partial helper only | U02; selected base metadata format from U01 | The pure identity gate checks exact program/update/build values; copy inspection, bounded extraction and real Zstandard decompression/checksum verification remain. Call the existing `check_frame` only for files matching its single-frame contract. | Fake copies accept only `0400486024DEC000` / `v131072` / `e0f3ccfa`; reject traversal, wrong identity and damaged required assets before installing anything. Preflight is not the acceptance authority. | M6–M7, M10 |
| U04 Local build orchestration — partial | U03, G1–G3; G5 for mobile | Added the bounded host-only synthetic probe in `crates/build/`: direct compiler invocation for a bundled C fixture, runnable self-check, private staging and same-filesystem generation publication. This synthetic route does not use an `oot-copy` token. Real build orchestration still needs the selected recompiler, G3 ABI and mobile gates. Retain C++/Python only inside the base recompiler boundary. | Synthetic input compiles and runs; failed/cancelled work preserves prior output. Real generated library, interruption of a running child, cache invalidation and target builds remain unproven. | M6, M11, M19 |
| U05 Rust runtime and C ABI — planned | G3, U02; U04 needed for game boot only | Start with the reference ABI stub for U04 synthetic linking. Then implement memory/address translation, threads/synchronization, timing, filesystem/save calls, input, audio/video services and diagnostic/error boundary. Derive the exact required host-call inventory from the chosen base. | Synthetic ABI conformance and service tests; boot at each incremental replacement on owner copy. Do not remove an existing service until equivalent behavior is proved. | M9, M11 |
| U06 Render interface and Vulkan backend — planned | G4, U05 contracts | Rust render command/resource model; Vulkan binding and surface integration, shader translation, textures, barriers, pipeline cache and presentation. `ash` is a candidate. | Synthetic draw/compute/readback tests, validation clean, then owner first frame on Windows and SteamOS; Adreno feature audit. | M3, M9, M12, M14–M15, M18 |
| U07 Metal backend — planned | G4, U05 contracts; can parallel U06 after shared interface | Rust Metal bindings, shader translation to supported Metal input, texture conversions, resource synchronization and presentation. Evaluate bindings and shader tools against the probe, not API names alone. | Synthetic draw/compute/readback on Mac, then signed iOS first frame and lifecycle test; no Vulkan backend masquerading as native Metal. | M12, M16–M17 |
| U08 Saves and safe session lifecycle — planned | U02, G6; U05 filesystem boundary | Save-relay adapter/manifest, per-platform save roots, atomic snapshots, conflict and offline handling; sync before launch and after clean exit, preserve dirty state after crash. Save progress parser or explicit unavailable state. | Fake relay and filesystem tests; safe offline retry; backup/restore and version migration tests; owner save round trip on all six devices. Never sync caches, keys or game libraries. | M2, M5, M19 |
| U09 Controller-first launcher and settings — planned | U02, G6; wire U03–U08 as available | Continue, Setup, Settings, Saves, About; testable view models, setup progress/errors, controller navigation, resolution/upscaler/cap and mappings, optional extras off by default. UI framework chosen by ADR. | Synthetic saves and fake build/runner tests cover controller focus and lifecycle; saved settings reach runtime; owner navigation on every platform. | M5, M8 |
| U13 Upscaling, caches and frame pacing — planned | U06–U07; G6 for reused FSR code | FSR Rust-renderer integration on Vulkan, MetalFX on Apple, cache stamps keyed by backend/device/driver/shader/tool version, warm-up and invalidation, 30/60 caps and frame-time capture. | Native-resolution baseline before upscaling; 30-minute handheld soaks at target resolution; device-specific cache invalidation tests. Frame generation is not required for 1.0. | M3, M8, M13 |
| U14 Compatibility and completion — planned | Relevant playable packages U05–U13 | Chosen-base open-bug inventory at approval, one test/repro per defect, private playthrough journal, completion criteria and release checklist. | All known play-breaking issues fixed or owner-ruled nonreproducible; M1 100% playthrough and all device/save/performance checks complete. | M1–M3, M9–M10 |

Uxx identifies a work package. Mxx refers to a requirement in [SPEC.md](SPEC.md). U10 is the shared desktop packaging package and has separate Windows and SteamOS checks. U11 has separate macOS and iOS checks.

Critical path: establish toolchain and portability evidence (U00–U01), then extend contracts and local setup (U02–U04), reach runtime and renderer milestones (U05–U07), and finish platform packages, performance, and the device acceptance checks. Saves and launcher work can proceed against fakes once shared contracts exist. Start mobile feasibility probes during U01.

## Android

| Package | Depends on | Code and tools to build | Acceptance gate | Musts |
|---|---|---|---|---|
| U12 Android packaging and local build — blocked by feasibility | G2, G4–G5, U04–U06, U08–U09 | Android app shell/entry, SDK/NDK linkage, Gradle package, Storage Access Framework import, validated app-local compiler and generated-library loading; controller and surface lifecycle. | Adreno device builds from owner copy and plays; rotation/background/surface loss and storage pressure preserve state. Host NDK compilation alone is not on-device compilation. | M6, M18–M19 |

- Prove the Android ARM64 path with a synthetic C/Rust library first. Use the Android SDK, NDK and Gradle for host builds and `adb` for installation and device logs. Separately prove the compiler and loader that can run on the phone under the app's storage and execution rules.
- Compare Android lifecycle and power handling with the [peer survey](PEERS.md#adjacent-apple-and-android-projects). BananaNative lists version 2.0 on its separate stack. Verify our supported update and local build path independently.
- Wire the imported BananaNative battery and thermal policy into the future local build scheduler at step boundaries. Integrate Storage Access Framework input, app-private staging, Adreno Vulkan capability checks, controller mapping, audio, and surface/background recovery. Measure storage, memory and build time on the target device.
- Gate delivery on an owner-copy build and successful play on Adreno. Confirm save preservation across process death and updates. A host-built APK and a Rust target check cover earlier stages.

## iOS and iPadOS

| Package | Depends on | Code and tools to build | Acceptance gate | Musts |
|---|---|---|---|---|
| U11 Apple packaging and lifecycle — blocked by feasibility | G2, G5, U04–U05, U07–U09 | Apple silicon Mac app; target-specific C linking, Xcode SDK/signing, iOS bundle, cable install/re-sign support, sandbox imports, controller APIs and pause/resume. | Native Mac play; iPhone/iPad install, all mapped buttons, resume and re-sign after actual expiry. Mac-generated artifact remains private and per-device; no build hub or redistribution. | M6, M16–M17, M19 |

- Use the imported nxnative macro check to audit the M1 compiler feature floor. Start with a synthetic Rust/C app linked with Xcode and the iOS SDK. Keep simulator and device targets separate. Prove physical-device installation and execution before adopting a game-library packaging approach.
- The [peer survey](PEERS.md#adjacent-apple-and-android-projects) includes a user-reported Xcode install with a free Apple ID on an iPhone 18 Pro running nxnative. Use that report as a feasibility lead and repeat the tests with our build and signing route.
- Resolve the M6 and iOS platform-row wording with the owner. The current platform row puts the Apple toolchain on the Mac. Prove local build, link, provisioning, cable installation, expiry reporting, and one-step re-signing with the owner's free Apple ID.
- Implement native Metal and MetalFX where supported, Game Controller input, sandboxed imports, and pause/resume and save recovery. Gate on mapped Xbox, PlayStation or MFi input, play on an iPhone or iPad, and re-signing after expiry.

## macOS

- U11 shares source with iOS and has its own desktop validation. Build and link for Apple silicon with Xcode command-line tools, SDK, Clang, and Rust's `aarch64-apple-darwin` target. Verify selected recompiler output and desktop host ABI without Rosetta.
- Use the imported NX2Apple Mach-O path check as one bundle validation step, then verify resolved libraries and signatures on a Mac. Bring up native Metal presentation and MetalFX after synthetic shader and resource tests. Add bundle paths, controller and audio support, game-library loading, and save-safe updates.
- Gate on an owner-copy build, installed native app, controller play and save round trip on an Apple silicon Mac.

## Windows

| Package | Depends on | Code and tools to build | Acceptance gate | Musts |
|---|---|---|---|---|
| U10 Desktop packaging and Steam integration — planned | U04–U06, U08–U09 | Linux/SteamOS bundle and Windows package, native paths, updater, controller mapping, Steam shortcut and owner-provided/permitted artwork. Add application release allowlists. | Clean installs/updates with saves intact; Game Mode hour on both handhelds; Windows controller hour; packages contain no game material. | M10, M14–M15, M19 |

- Use the pinned Rust MSVC toolchain, Visual Studio Build Tools and Windows SDK, or record a compatible selected-base toolchain. Validate DLL loading, wide paths, runtime redistributables, Vulkan capability errors, and controller/audio selection.
- Package the launcher and runtime with a clean install and rollback path. Verify accurate errors for missing toolchains or unsupported hardware. Gate on a native owner-copy build, settings reaching the runtime, save-safe update, and a controller play hour on the RTX 3090 Ti.

## Linux (SteamOS)

- U10 shares packaging ownership with Windows. Build with native Rust and the selected C/C++ toolchain, and validate Vulkan loader/driver behavior, paths, controller input and audio on actual SteamOS. Benchmark both peers under comparable settings before choosing the base.
- The RedTunic adaptation adds an XDG desktop Exec quoting helper on this branch. Desktop entry installation and Steam registration remain separate work. Evaluate Longshot's Linux build and AppImage tooling as source references. Check portability, package dependency boundaries and licensing independently of the peer renderer. Validate clean installs in the distribution format chosen for this project.
- Register the launcher and permitted artwork with Steam, preserve unrelated entries, and test Game Mode launch and built-in controls. Gate on both handhelds, a one-hour play check, 30-minute frame-time soaks at native screen size, and save/update recovery.

## Requirement coverage and immediate queue

| Must | Work packages | Audited status |
|---|---|---|
| M1 100% completion | U14 | Planned |
| M2 one save everywhere | U08, U14 | Planned |
| M3 handheld frame rate | U06, U13, U14 | Planned |
| M4 verified merges | U00, all packages | Partial: hosted matrix has passed; each merge still requires its checks |
| M5 Continue screen | U08–U09 | Planned |
| M6 per-device build | U01, U03–U04, U11–U12 | Partial: standalone frame helper only |
| M7 exact supported version | U03 | Partial: pure identity gate present; copy inspection remains planned |
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

Next: (1) continue bounded U01 feasibility probes, (2) run the owner base benchmark, (3) decide the ABI/storage/UI boundaries and extend U02 contracts, (4) implement U03 acceptance checks before any real install. Synthetic work that does not depend on base choice can proceed while the owner benchmark is pending.

## References for proposed tools

Tool documentation was checked during this audit; it is guidance, not proof that our port works. [Rust iOS target documentation](https://doc.rust-lang.org/stable/rustc/platform-support/apple-ios.html) explains target/SDK requirements. [Android NDK CMake documentation](https://developer.android.com/ndk/guides/cmake) covers host-driven native app builds, not our unproved on-device compiler. [Slint platform/licensing FAQ](https://slint.dev/faqs) establishes it as a candidate to probe. Use exact selected versions in implementation ADRs.
