# Roadmap

The project remains in Spec; the spec and base choice have not been approved. The milestone numbers below follow SPEC.md and are a draft map, not a claim of delivery order or platform validation.

| Area | Status | Next evidence |
|---|---|---|
| M6 local setup preflight | Partial: standalone Zstandard frame structure checker | Integrate into the eventual local build flow; validate real user-supplied inputs on each device |
| M4 CI and cross-builds | Planned | Linux, Windows and macOS tests; Apple and Android cross-builds |
| M5–M10 launcher, copy/version gate, performance and saves | Planned | Fake-copy tests, then device tests; M7 version validation remains separate from frame structure |
| M11–M15 Rust runtime, renderers and graphics features | Planned | ABI integration, native renderer and device performance evidence |
| M16–M19 platform builds and update safety | Planned | Real device builds and sessions, save backup checks |

The benchmark in SPEC.md must decide the base. The preflight imports no base or game content and does not establish game compatibility.
