# Roadmap

The project remains in Spec. The owner will choose the base by running the benchmark in [SPEC.md](SPEC.md) before approving the spec. The rows track progress against the spec's Musts.

| Area | Status | Next evidence |
|---|---|---|
| M1 100% completion on the AMD handheld | Planned | Owner playthrough log |
| M2 Shared save on all six devices | Planned | save-relay manifest tests and owner round trips |
| M3 Handheld frame rate | Planned | Owner 30-minute soaks at native resolution |
| M4 CI and cross-builds | Partial | Hosted desktop Rust tests and mobile `cargo check` jobs are configured for the preflight. GitHub blocked hosted execution at billing. Runtime and launcher builds and mobile linking are planned. |
| M5 Controller-first Continue screen | Planned | View-model tests and owner device checks |
| M6 Local game-library build on each device | Partial | Standalone Zstandard frame preflight works on synthetic input. Wire it into local setup after base selection and build from the owner's copy on each device. |
| M7 Supported copy and version gate | Planned | Fake-copy tests for exact program, update and build IDs |
| M8 Launcher settings and optional extras | Planned | Launch on each device with the saved settings |
| M9 Known play-breaking bugs | Planned | Triage the chosen base's known failures against owner devices |
| M10 Repository and release exclusion of game data | Planned | Continue checking changes and add release file-list verification when releases exist |
| M11 Rust runtime and C ABI | Planned | Boot with selected recompiler output calling the Rust runtime |
| M12 Native Vulkan and Metal renderers | Planned | Boot and draw on each platform |
| M13 Platform upscaling | Planned | FSR sessions on Vulkan devices and a MetalFX session on Mac |
| M14 Steam shortcut, artwork and Game Mode | Planned | Owner checks on both handhelds |
| M15 Windows controller play | Planned | An hour on the RTX 3090 Ti desktop |
| M16 Native Apple silicon Mac build | Planned | Owner build and play session on Mac |
| M17 iPhone and iPad signing and controller play | Planned | Owner install, mapped controller session and re-sign |
| M18 Adreno Android build and controller play | Planned | Owner build and play session on Android |
| M19 Save backups across updates | Planned | Synthetic update test and device check |
