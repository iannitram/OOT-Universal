# Spec

**1.0 means:** I finish The Legend of Zelda: Ocarina of Time (Switch 2 remake) at 100% completion from my own copy on my AMD handheld, play it on my Strix Halo tablet, RTX 3090 Ti desktop, Mac, iPhone or iPad with a controller, and an Adreno Android device with a controller, and the same save loads on every one of them through save-relay.

**Approved by me:** not yet

A project is Spec'd only when the line above has a date and a link. Change a Must or a Won't after approval and the approval line goes back to "not yet" until I approve again.

This draft carries my answers from the interview of 2026-11-21 (listed at the end). One thing stays open for me: the base, which my benchmark decides. The licence is decided (GPL-3.0). See "Open for me".

## 1.0 in one sentence

See the top of this page. The version is the one both bases already support (see Supported versions).

## Base

Two candidates, the same game build. I benchmark both before approval and record the result here; the better one becomes the base, and the other stays a reference.

| Check (same scene, same settings, upscaler and frame generation off, 60 FPS cap) | Relink (pixl27/ProjectReLink) | Longshot (Recomper/Longshot) |
|---|---|---|
| Average frame rate, AMD handheld | | |
| Average frame rate, RTX 3090 Ti desktop | | |
| Crashes or hangs in a 30-minute soak, each device | | |
| Platforms the base builds for | Windows only (its README) | Windows and Linux, AppImage (its README) |

Facts from the two READMEs: Relink's README reports Steam Deck and SteamOS as untested; Longshot's README lists a SteamOS freeze in Kokiri Forest and the Great Deku Tree with a workaround (issue #817). Both need Vulkan 1.3 with mesh shaders and a CPU with AVX2.

## A copy, built our way

Once the base is approved, this repo becomes a copy (not a GitHub fork, R18) of the chosen base's code, with its licence and credit kept. The base's recompiler is a C++/Python build tool. It stays as it is and turns the game's own code into C on my machine, from my copy of the game. The game's C then calls our Rust runtime through a C ABI. Our launcher and tools are Rust. Release builds hold no game code: each device builds its game library from its own copy.

Our delta is most of the project:

| Delta | Upstream? |
|---|---|
| Our runtime in Rust, replacing the base's C++ runtime (M11) | rewritten, so the base's runtime stops syncing from upstream |
| Our renderer in Rust on each platform's native API: Vulkan on Windows, SteamOS and Android; Metal on macOS, iPhone and iPad (M12) | rewritten, so the base's renderer stops syncing from upstream |
| Continue screen launcher, Rust, controller-first (M5) | keep on ours |
| Each device builds its game library from my copy, no build hubs in 1.0 (M6) | keep on ours |
| Mac, iPhone/iPad and Android builds (M16 to M18): no peer has them yet (see PEERS.md) | our own work |
| Shared saves through save-relay (M2) | keep on ours |
| Steam shortcut and artwork for the handhelds (M14) | keep on ours |
| Correctness fixes (crashes, renderer bugs found on my devices) | bug reports upstream only at my word |

The base's recompiler and its tools keep syncing from upstream. The runtime and renderer do not, once rewritten. Upstream's own tests run unchanged where they still apply.

## Musts

Device checks are `needs-owner-device`: they need my machines and my copy.

| # | Must | Done looks like | Check |
|---|---|---|---|
| M1 | The game is finished at 100% completion on the AMD handheld | All main dungeons are done, Ganon is defeated, the credits roll, and every side quest, collectible and Gold Skulltula is collected, with no crash or hang | a playthrough log (`playthrough.jsonl`) across the whole run; needs-owner-device |
| M2 | One save everywhere | A save made on one device loads on each of the other five after a restart; the launcher pulls the save before the game starts and pushes it after it quits, syncing only the game's save folder | save-relay manifest tests; the round trip on all six devices |
| M3 | Frame rate on the handhelds | 60 FPS on the AMD handheld and the Strix Halo tablet at their own screen size; 30 FPS is the fallback and never the normal state | frame stats over a 30-minute soak |
| M4 | Every merge is verified in CI | `cargo test` passes on Linux, Windows and macOS for each merge to `main`; the Apple and Android targets cross-build; CI never holds game files, so it builds the runtime and the launcher, never the game library | CI green |
| M5 | A controller-first Continue screen on every device | The launcher shows the latest save, its file name, how far it is, the device it was played on and when, with one button to sync and play; first run is a setup driven by the controller; Settings, Saves and About are one button away | view-model tests with fake saves; on each device |
| M6 | Each device builds from my copy | On first setup each device builds its own game library from my copy of the game; no device takes a finished library from another | core test with a fake copy; one real build per device |
| M7 | Only the supported version runs | My copy is accepted only if it is program `0400486024DEC000`, update `v131072`, build `e0f3ccfa`; any other copy is refused before anything installs, with a plain message | core tests with fake copies |
| M8 | The launcher starts the game with my settings | Resolution, upscaler, frame cap and my controller mapping apply from the launcher on every device; the base's extras are present, off by default, and never required | start the game from the launcher; on each device |
| M9 | Known play-breaking bugs are fixed | Each crash or hang that the chosen base lists as open at approval is fixed here or upstream, or ruled not reproducible on my devices, one by one | each issue's own check |
| M10 | No game files, ever | Nothing from my copy, my keys, the compiled game library or game-binary analysis is in the repo, a release, a log, CI or an issue | review of every PR; the release file list |
| M11 | Our code is Rust | The launcher, the tools and the runtime ship as Rust; the base's recompiler stays as it is and emits C that calls the Rust runtime through a C ABI | `git ls-files` of our folders shows no shipped C or Python of ours outside the recompiler; the game boots at each step |
| M12 | A native renderer on each platform | Vulkan on Windows, SteamOS and Android; Metal on macOS, iPhone and iPad; each through Rust bindings | the game boots and draws on each device |
| M13 | Upscaling on each platform | FSR runs in the Rust renderer on the Vulkan platforms; the Apple platforms use MetalFX | a session on each Vulkan device; a session on the Mac |
| M14 | ON the AMD handheld and the Strix Halo tablet, THE LAUNCHER SHALL add itself to Steam with the game's artwork, so Game Mode opens the Continue screen, and THE GAME SHALL play on the built-in controls | Steam's library shows the artwork; an hour from Game Mode with no crash on each | on each handheld |
| M15 | ON the RTX 3090 Ti desktop, THE GAME SHALL play from the launcher on Windows with a controller | An hour of play, no crash | on the device |
| M16 | ON my Mac, THE LAUNCHER SHALL build and run the game natively on Apple silicon, with no Rosetta | The game plays on the Mac | on the device |
| M17 | ON an iPhone or iPad, THE APP SHALL install over a cable with my free Apple ID, play with an Xbox, PlayStation or MFi controller with every button mapped, and say when the 7-day signature is close to running out and re-sign in one step | The app installs and starts, a session plays on a controller, and it is re-signed after 7 days | on the device |
| M18 | ON an Adreno Android device, THE LAUNCHER SHALL build the game from my copy and play with a controller | A session with no crash | on the device |
| M19 | WHEN I update the launcher or the port, THE LAUNCHER SHALL back up my saves first and keep them | Saves intact after an update | core test with a fake build; on one device |

Touch controls are not a Must in 1.0: phones and tablets play with a paired controller.

## Won'ts (1.0)

- Any game data, keys, firmware or the compiled game library in this repo, its releases, CI, logs or issues, ever.
- Game versions other than program `0400486024DEC000`, update `v131072`, build `e0f3ccfa`.
- Online play, multiplayer and netplay.
- Build hubs, pairing a device to another for its build, or a fleet view: each device builds from my copy.
- Touch controls on phones and tablets.
- Rewriting the base's recompiler in Rust.
- Required mods or cheats. The base's optional extras (randomizer, cheats, save editor and the like) ship as they do upstream, off by default, never required.
- Frame rates above 60 FPS.
- An emulator route.

## Prior art

The upstream and the peer list are in [`PEERS.md`](PEERS.md) at the repo root.

## Problem

I want to play Ocarina of Time on the devices I actually use. The two PC recompilations run on Windows, and Longshot also on Linux; neither has a Mac, iPhone, iPad or Android build. A save made on one machine does not follow me to another. *(My draft from the facts; the reason in my own words is still to come.)*

## Solution

One Rust launcher and runtime on each of my six devices, opening on a Continue screen, in front of the chosen base's recompiled game. Each device builds its game library from my own copy. save-relay keeps one save across all of them.

## User stories

1. As the owner, I want to pick up any of my devices and continue the same save, so that I never copy saves by hand.
2. As the owner, I want to start the game from Game Mode on my handhelds, so that I never need the desktop.
3. As the owner, I want to play on my iPhone, iPad or Android device with a controller, so that I can play away from home.
4. As the owner, I want one button on any device to put me back where I left Link.
5. As a contributor, I want one language for everything of ours, so that a fix is a fix, and I can read the whole launcher.

## Implementation decisions

- **Base:** Relink or Longshot, chosen by the benchmark under Base. Both are the same game build (Supported versions).
- **Rust:** the launcher, the tools and the runtime are Rust, behind a C ABI that the base's recompiler's C calls. The recompiler stays C++/Python.
- **Renderer:** native per platform through Rust bindings: Vulkan on Windows, SteamOS and Android; Metal on macOS, iPhone and iPad.
- **Upscaler:** FSR written in the Rust renderer on Vulkan platforms. On Apple platforms, MetalFX. *(MetalFX is my draft pick; the interview did not cover Metal.)*
- **Frame rate:** 60 FPS target on the handhelds; 30 FPS as the fallback.
- **Builds:** each device builds its game library from my copy. No build hubs, no pairing, no fleet in 1.0.
- **Saves:** a save-relay manifest for the game names the game's save folder for each platform, so a save moves unchanged between devices.
- **Shared pieces** from the other -Universal repos, where they exist and their licence allows: save-relay (`iannitram/save-relay`), the launcher modules from `iannitram/totk-universal` (MIT), and `iannitram/FSR-Vulkan` for the FSR path (optional, never required).
- **Extras:** the base's optional extras stay, off by default.
- **Launcher UI toolkit:** to choose in the Roadmap. The sibling repos use Slint.

## Testing decisions

- Core logic (the version gate, copy checks, the build steps, the save manifest and round trip) is tested with fake game folders in `cargo test` on Linux, Windows and macOS.
- Launcher pages are tested through their view models: button presses in, page state out.
- Everything that runs the game on hardware is a device check (`needs-owner-device`).

## Platforms

| | SteamOS | Windows | macOS | iPhone / iPad | Android |
|---|---|---|---|---|---|
| 1.0 device | AMD handheld, Strix Halo tablet | RTX 3090 Ti desktop | a Mac with Apple silicon | an iPhone or iPad with a controller | an Adreno device with a controller |
| Game build | from my copy, on the device | from my copy, on the device | from my copy, on the device | from my copy, with the Apple toolchain on the Mac | from my copy, on the device |
| Renderer | Vulkan | Vulkan | Metal | Metal | Vulkan |
| Upscaler | FSR (Rust renderer) | FSR (Rust renderer) | MetalFX | MetalFX | FSR (Rust renderer) |
| Launcher | Rust, controller-first | Rust | Rust | the app itself | Rust |
| Signing | none | none | none | free Apple ID, every 7 days | not decided |

## Supported versions

| Version | State |
|---|---|
| Program `0400486024DEC000`, update `v131072`, build `e0f3ccfa` | my copy; the only one in 1.0 (both bases support it) |

## Open for me

1. **Base:** the benchmark above decides it. Which one is the base at approval?
2. **Licence (decided 2026-11-21):** GPL-3.0, the licence both bases already use. The LICENSE file is in the repo.

## Questions I answered (2026-11-21)

1. **Finish:** 100% completion on the AMD handheld.
2. **Extras:** ported as they ship, off by default.
3. **Online:** no, offline only.
4. **Base:** benchmark both on my devices, decide at approval.
5. **Upscaler:** native Vulkan FSR in the Rust renderer.
6. **Touch:** controller only in 1.0.
7. **Shared pieces:** reuse them from the other -Universal repos where they exist.
8. Earlier rounds: versions Relink's only; all five platforms in 1.0; saves through save-relay; Rust all the way, runtime included; each device builds from my copy; the six device families of the sibling repos; a Rust launcher, controller-first; the base's recompiler stays and its output calls the Rust runtime; native graphics per platform; a free Apple ID with a 7-day re-sign; 60 FPS on the handhelds with a 30 FPS fallback.

## Further notes

Mac, iPhone/iPad and Android are all in 1.0 with no peer build to start from; that is the largest risk in this spec and the Roadmap starts there.
