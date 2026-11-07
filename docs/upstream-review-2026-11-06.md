# Antigravity review and upstream update — November 6, 2026

## Compared revisions

- Fork main and origin/main: `ef57488ec5aff8cb1ac6ee88ed0a6a4e2736416b` (November 5).
- Freshly fetched upstream/main: `73e6389821c59ffd5d738f0ca43e926d5ab84a89` (November 5).
- Latest shared ancestor: `e357d8509a7d8fb04ce0642d549d32f19757709b`.
- Ancestry counts: 65 commits reachable only from the fork, 53 only from upstream main. The 65 include upstream feature-branch history and merge commits; they are not 65 independent custom features.
- Canonical checkout `/home/bry/Projects/Lemon-Studio` was clean and matched origin/main before this review.

The previous October 28 candidate is already committed as `161a21aa` and included in current main. Its old handoff's claim that its merge is still uncommitted is historical, not the current repository state. Subsequent integration commits are `baab2a46` and `54fb26e8`.

## Antigravity work: what is actually new

Commit authors use the owner's Git identity, so attribution here follows the user's description and the local first-parent history, not a claim that Git independently identifies the agent.

| Work | Assessment and decision |
| --- | --- |
| Upstream Patcher, native `.lemon`, multithreaded mixing, Slicer/Vocoder/Drum Sampler, track modes and Burn MIDI | Already integrated. Retain rather than redo those merges. |
| Zest and its Space presets, editor resizing, Linux typing keys over editors | Integrated from upstream feature branches. Valuable additions, not new downstream implementations of those features. Keep them when updating main. |
| Etched skin and wallpaper frost (`07f42bb6`) | This brings in upstream/etched-skin through `b8eec0c8`; it is not an independently authored fork skin. Wallpaper reads run off the UI thread and large pictures have a reduced cache. Retain as an optional skin; its drawing cost needs visual/performance acceptance with the newer animation system. |
| Piano row size, C octave badges, label defaults (`fc89d807`, `aced86e8`) | Useful custom presentation. Preserve the 22 px default row, readable octave badges and note/scale options. They complement the new upstream plugin-editor label fixes. |
| Clip fade tabs, shaded fade regions and value badges (`5e225e03`, `a66701de`) | Keep the visual improvements, adapting them to upstream's extracted handle painter and fade animation. See the hit-area issue below. |
| Real-time limits and scanner notification changes (`ef57488e`) | Preserve the direct-scheduler path's avoidance of a new RTTIME cap, the memlock soft-limit adjustment and scanner-only notification isolation. Combine with upstream's new refused-promotion counter. Raising the memlock allowance does not itself lock memory or prove an OOM/crash fix. |

## Review findings

### High: an earlier merge discarded the bounded Hyprland helper

At fork main, `plugin_window/hyprland.rs` is identical to upstream main, replacing the safer October 28 version. `plain()` selects every XWayland window with this PID. That assumes the main window is native Wayland, but `main::use_x11()` deliberately defaults to X11/XWayland when available, to support docking. Opening a detached frame can therefore change the main window's compositor properties too.

`Drag::start()` and `follow()` also synchronously query the compositor from X11 input handling, with 500 ms read timeouts and without the previous response/connect/write limits. A slow compositor can stall interaction; selecting by PID and pointer overlap can pick the wrong window.

**Addressed in the candidate:** restored the exact frame-class/PID/XWayland targeting, address validation, bounded IPC and background/coalesced drag worker from `161a21aa`. Adapted the current X11 call sites, including X11 fallback on worker failure. Preserved the newer resize and keyboard handling. The main window keeps its usual class; detached frames use `lemonstudio-frame-<xid>`. No Hyprland configuration was changed.

### Medium: the new fade tabs are taller than their clickable area

At fork main, `views/playlist.rs::fades` paints 13 px tall tabs, but `playlist.rs::handle_at` still accepts the old region centered 3 px below the top with a 7 px half-height. The visible bottom 2 px of a tab are outside that region. Clicking there can start a clip edit rather than a fade drag. `a66701de` restored the old hit test after `5e225e03` enlarged it, without reducing the painted tab.

This was a pre-existing usability issue, not introduced by the upstream merge. **Addressed after review:** drawing and clicking now share `playlist::fade_tab`, bounded by the clip's width/height. The old grab padding is retained; crossfade/tension regions and priority are unchanged. The existing interaction regression now drags both tabs from 12 px below the top, checking that the fade changes and Undo remains intact. All 1,172 app tests pass after this fix (`fade-app-tests.log`).

### Release blocker: five new image dependencies were omitted from notices

A fresh `cargo test --workspace --locked --offline -j2` against the unmodified fork stops at `notices::tests::every_crate_shipped_is_listed`. Missing entries are `byteorder-lite 0.1.0`, `image-webp 0.2.4`, `quick-error 2.0.1`, `zune-core 0.5.3` and `zune-jpeg 0.5.15`, introduced with the wallpaper work. Baseline log: `fork-baseline-tests.log`. This is an existing check failure, not a merge conflict.

**Addressed in the candidate:** regenerated notices with the repository's generator and a private copy of cargo-about 0.9.3 under the evidence directory. Kept the existing ISC notice rather than accepting the newer generator's placeholder copyright. All three notices tests pass after that adjustment.

### Release gap: old successful checks do not certify these later merges

The October 28 test/bundle/render evidence applies to `161a21aa`'s source, not automatically to the November 5 fork or this new combined tree. Run fresh checks. Listening, real input devices, mixed-monitor movement and Windows/macOS remain separate acceptance gates. Scanner notification suppression is not crash isolation or evidence of lower memory use.

## What latest upstream main improves

| Area | Change to adopt | Relationship to our work |
| --- | --- | --- |
| Playback | Keep one playback stream while the device is open; Play/Stop/live notes no longer repeatedly start streams (`9bf7925a`). | A stronger lifecycle implementation; retain our live clip-gain refresh inside the engine it drives. |
| Effects/performance | Stop decoding already-hosted mixer slots every tick (`a17c41e5`); half-size FFT for the same monitor spectrum (`c20bec86`). | Direct reductions in repeated work. Prefer upstream's implementations. |
| Pause/Stop | Hosted instruments release and ring out, repeated held keys receive matching offs, busy/missed notes cannot fire after pause (`61457ad5`, `06257674`, `4f9ad0f9`). | Better explicit tail/note lifecycle coverage; preserve it alongside legacy effect state compatibility. |
| Plugin editors | Fix fractional-scale captions, control overlap, typed values/units, stored-value drawing, wheel steps and text clipping across multiple plugins. | Complements the fork's main piano/playlist styling. Use upstream's focused fixes. |
| Mixer | Group fader drag, Alt-click fader/pan/separation reset. | More complete mixer interaction; retain our pattern-focus badge and downstream controls. |
| Motion | Four animation levels, easing, hover fades, clip delete/drag feedback, menu/palette transitions and transparency options. | The main source conflict with Etched, custom clip handles and the command palette. Merge both behaviors explicitly. |

## Candidate merge and conflict decisions

Reused the existing clean safety checkout at `/home/bry/Projects/Lemon-Studio-safety-2026-10-28`. Its previous `161a21aa` checkpoint remains in Git history. Candidate HEAD is detached at fork `ef57488e`; MERGE_HEAD is pinned upstream `73e63898`. The merge is not committed or published.

Git reported six conflicted files:

- `app.rs`: retain both Etched debug state and motion state; combine transparent-grid and animation/transparency restrictions on pixel-scroll shortcuts; retain both command-palette toggling and playback-scroll querying; retain assistant capture plus chosen-row animation; include browser-drag Escape handling.
- `main.rs`: register both command-palette and motion modules.
- `rt.rs`: retain fallback-only RTTIME capping and upstream refused-promotion diagnostics.
- `views/mixer.rs`: animate the plus icon inside its visibility scope; retain the pattern-focus badge while animating layout text.
- `views/mod.rs`: draw motion ghosts and keep the command-palette modal above them.
- `views/playlist.rs`: keep shaded fades, tabs, gain grip and value badges in upstream's extracted handle painter; use its hover fading; keep the adjusted-gain guide when not hovered.

Additionally restored the two Hyprland implementation/call-site files described above. Auto-merged engine and transport code requires fresh regression coverage; a clean textual merge alone is insufficient. Browser favorites/waveforms, Ray/Wavetable, MCP, legacy LMD1 delays and the spectral-job safety pipeline remain present.

## Branches beyond main: keep separate for now

- `atomic-save` (`325b9bce`): overlaps our atomic project save. Upstream's helper additionally preserves symlinks and permissions and syncs the containing directory on Unix. Prefer a single shared helper with those improvements in a separate reviewed change; keep backup/error behavior. It still needs qualification rather than treating its commit description as a power-loss guarantee.
- `pipewire-stream-name` (`5b1af90b`): useful small Linux polish for recognizable stream names/icons.
- `browser-my-projects` (`b87fe72d`): useful browser addition that should be combined with our stars/preview panel and folder filtering.
- `playback-holds-the-stream` (`cbc80019`): further pause/tail lifecycle work beyond current main; evaluate after the main update instead of stacking another transport change immediately.
- `lemon-features` and `smith`: native format feature declarations and an Ogun-compatible additive instrument. Keep separate until their persistence/compatibility and DSP/editor coverage are reviewed.
- Do not use `cutting-edge` as a wholesale replacement for the maintained fork.

## Validation and next steps

Fresh baseline and candidate results are recorded in `/home/bry/Projects/lemon-review-2026-11-06`:

- Unmodified fork baseline: failed at the five missing notices, as described above. This stopped that run before all workspace tests executed; do not call the baseline wholly green.
- Combined candidate: `cargo test --workspace --exclude notices --locked --offline -j2` passed, reporting 2,592 passes, zero failures and 69 ignored. The separately run notices suite passes all three tests: 2,595 reported passes across those two runs. App portion: 1,172 passed, zero failures, 49 ignored. Logs: `candidate-tests.log` and `notices-tests.log`.
- These include exact-window targeting and bounded/slow-reply Hyprland tests, Etched full/partial repaint equality, command-palette behavior, live gain on sounding internal/hosted clip voices, upstream note-release handling and the preserved compatibility/safety regressions.
- Production app build: `cargo build -p os-app --bin lemonstudio --locked --offline -j2` passed, and passed again after the fade fix (`fade-app-build.log`). Existing compiler warnings remain; this is not a warning-free or cross-platform qualification.
- Two normally ignored real XWayland checks passed separately, sequentially, using unmapped test parents: Lemon Sampler host-scale/page retention and Ray docking/scaling/editing. Log: `xwayland-tests.log`. No existing user windows were moved or resized.
- Clip-gain drag/narrow-clip render test rerun for a capture: passed. Visually inspected `screenshots/clip-gain-drag.png`; the themed gain grip/badge and custom tabs remain visible after the painter extraction. This is offscreen synthetic rendering with no audio played.
- After the fade fix, the complete app suite passed again: 1,172 passed, zero failures, 49 ignored. Its drag regression covers both lower tab edges; the existing narrow-clip, gain, crossfade, repaint and history cases also pass.
- Rebuilt all 27 products listed by `bundle-plugins` into 54 CLAP/VST3 bundles, without installation (`bundle-build.log`). Copies are saved under the dated evidence directory's `bundled/`. All 54 copies load/enumerate in separate scanner children, with unique IDs within each format; the final preview executable performed that scan (`bundle-scan-final.log`, `bundle-smoke.json`).
- With the fresh bundles present, all 31 CLAP and 35 VST3 host tests pass (`fresh-bundle-host-tests.log`). Conditional third-party cases still do not prove arbitrary installed plugins work.
- Ignored and conditionally skipped corpus/device/third-party-plugin cases are not certified by these counts. Windows/macOS CI, physical input, listening and mixed-monitor detached movement remain release gates.

The final preview executable is `/home/bry/Projects/lemon-review-2026-11-06/bin/lemonstudio-preview`, SHA-256 `283d176a18626f364db96a4eef9ff5915788076fe55b62d9da33ccadc17b5dcb`. The initial review binary remains saved separately as `bin/lemonstudio`. The final upstream main ref was checked again and still names `73e63898`.

On November 6 at 17:03 MST, launched the final executable through `lemon-review-preview-20261106.service`, with a persistent memory/exit monitor. Verified the running PID's executable hash matches the above, and Hyprland reports its mapped main window as XWayland. The original project has not been opened for editing: the preview uses `project-copies/Grave Light - preview.flp`, a byte-identical copy, with all eleven source sample paths present. Its separate settings contain a valid cache of only the 54 explicitly scanned bundles and disable startup scanning, so it does not trigger first-run external/Windows plugin discovery.

The preview exited after 64.48 seconds with exit code 0, no signal, an empty error log and observed peak app RSS of 189,228 KiB (about 185 MiB). The user confirmed they closed it. Evidence is in `preview-la8_cuca/`. The user subsequently accepted the overall preview experience with “Everything feels pretty good.” Long-session memory stability, physical inputs, individual third-party plugins and mixed-monitor detached movement retain their separate qualification status.

The accepted source combination merges fork `ef57488e` and upstream `73e63898`, retaining the reviewed conflict resolutions, Hyprland protections, dependency notices and fade-tab fix. The user's preview acceptance completes the local source-promotion gate. The merge commit and publication status are recorded in the external `candidate-state.json` evidence manifest. Windows/macOS CI and the additional hardware/plugin qualifications above remain to be checked separately.

The installed executable observed at this checkpoint has SHA-256 `c62c0191d2aa4a633bbe393c9c2e96e56cc247b4d6213ca310450e7b9d9df5e7`. The accepted executable is available separately at `bin/lemonstudio-preview` under the dated evidence directory. The monitor saves memory samples and eventual exit status independently of its sampling interval.
