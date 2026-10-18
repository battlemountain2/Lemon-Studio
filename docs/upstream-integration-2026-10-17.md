# Upstream integration and next steps — October 17, 2026

Bry asked Codex to finish the current work, compare and integrate upstream, then plan the next feature work.
The chosen Linux direction remains **XWayland with docked plugin editors**, not a native Wayland main window.

## Revision boundary

- Starting combined fork: `bc755a8`, including XWayland runtime checkpoint `76235c7` and the default-pattern fix.
- Integrated upstream: `b6807951d64a37229a49f1c7522d4c7c1afb1a83` from `ItsAlphaNeon/Lemon-Studio`.
- Previously integrated upstream: `ba78a0391419557a8bdc6c7bea003facf38c4d28`.
- There were 159 upstream-only and 10 fork-only commits at the starting checkpoint. These are ancestry counts,
  not independent feature counts.
- Integration worktree: `/home/bry/Projects/Lemon-Studio-upstream-2026-10-17`, branch
  `integrate/upstream-2026-10-17`. The maintained product checkout is `/home/bry/Projects/Lemon-Studio`.
- This integration is local. No push or remote cross-platform CI run was performed for it.

## Comparison: reuse upstream, retain useful differences

| Area | What overlaps / which implementation to use | Integration decision |
| --- | --- | --- |
| Undo and editing | Upstream now supplies shared history/redo and step-edit chord handling. This is broader than the fork's earlier editor-local history. | Reuse `os-app::history` and `step_edit`; do not create another history system. Live recording is **not** an undoable take yet. |
| Saving and recovery | Upstream adds overwrite copies, timed autosave, backup locations and project/data-folder settings. The fork's synced temporary-file/rename save protects a different failure boundary. | Combine them in `App::write_project`; keep upstream save-size notices and backup behavior. Neither implementation proves crash durability or reliable recovery from every error. |
| Pattern creation | Upstream lists the currently selected/placed empty pattern, even before it has notes. This does not replace persisting an explicitly confirmed default name. | Keep the fork's checkmark/Enter fix; confirmed `Pattern N` stays listed after selecting another pattern and survives saving. Preserve upstream's source-listing behavior. |
| Linux plugin scaling | Upstream's new display/PPI settings overlap with scale selection, but do not replace the fork's X11 host/editor scale reconciliation. | Keep the docked scaling fix and stale-scale docking repair. Use upstream's shorter plugin-open lock scope to avoid holding the instance while painting/raising its frame. |
| Plugin preferences | Upstream adds saved dock/detach choices and a bulk editor-mode command. | Keep these and apply the fork's scale reconciliation when an editor docks. |
| Tempo and meter | Upstream replaces `time_signature` with `Project::bar`; BPM counts a bar's beats and playback derives quarter-note tick tempo. | Follow that model, including count-in. At 120 BPM both 3/4 and 6/8 now count one beat every 0.5 seconds. Runtime safety bounds must allow derived tempos outside the displayed 10–999 BPM range. FLP loading preserves raw stored tempo. |
| Toolbar and rack | Upstream adds adaptive toolbar layout, high-visibility rendering and mouse-button preferences. | Keep those, wire the active fork recording controls into them, and retain audible step preview. Painting follows upstream's left-on/right-off semantics, even when the initial press toggles a step. |
| Metronome and channel settings | These were already shared with upstream in the previous integration. | Continue using upstream's metronome and sole cut-group event writer. Keep the fork's self-cut controls and persistence tests. |
| Original fork capabilities | Procedural Kick/Clap/Hat/Snare template, delay/reverb/distortion, live note recording, picker double-click and effect DSP identity checks remain downstream differences. | Retain them. Recording stays experimental; synthetic drums/effects are not claims of FL Studio sonic parity. |
| Source attribution | Upstream's updated checker supports multiple copyright holders and shared-source notices. | Keep it; retain only the four exact fork-origin DSP file exceptions, with existence checks. Do not invent upstream authorship. |

The nine text conflicts were README, app, plugins, rack, toolbar, engine, sample tests, FLP project tests and the
header checker. A compiling merge initially exposed four behavioral/fixture regressions: drag painting, confirmed
empty-pattern persistence, the six-file loading fixture and the four-drum default-template expectation. All four
were reconciled rather than dropping either implementation wholesale.

## Local verification

Using `/home/bry/Projects/Lemon-Studio-integration/target` only as a shared build cache:

- `cargo build --workspace --all-targets --locked --offline`: passed.
- `CARGO_NET_OFFLINE=true cargo run -p bundle-plugins --locked --offline`: rebuilt all three VST3/CLAP pairs.
- `cargo test --workspace --locked --offline` **after rebuilding bundles**: 980 reported passes, zero failures,
  28 ignored, including 470 app and 122 engine unit tests.
- The ignored `plugins::linux_tests::docked_editor_follows_xwayland_parent_scale` was separately run and passed
  on the actual XWayland server. It checks real Lemon Sampler frame/editor geometry at 1x, 1.25x, 1.5x and 2x,
  X11 parentage, active-page preservation and stale-scale docking. Its parent is unmapped; it does not take desktop focus.
- `cargo build -p os-app --release --locked --offline`: passed.
- Release mixer screenshot rendered and was visually inspected. The four-drum rack and upstream toolbar/mixer render.
- Release startup with isolated settings mapped a `lemonstudio` window that Hyprland reported as `xwayland: true`,
  on a silent temporary special workspace; no startup errors were logged. The probe ended after ten seconds.
- `git diff upstream/main --check`: passed for downstream changes. The full imported patch has two pre-existing
  upstream trailing-whitespace lines (notices text and a preferences test fixture); these were left unchanged.

The 980 count includes fifteen corpus cases that return without private fixtures when `OPENSTUDIO_FLP_CORPUS` is
unset. Other third-party/environment tests may also return early. Ignored hardware/UI tests were not all enabled.
These results do not establish physical MIDI timing, arbitrary third-party plugin compatibility, mixed-DPI monitor
transitions, Windows/macOS support or full real-project round trips. Existing compiler warnings remain.

Validation logs are `/home/bry/Projects/lemon-upstream-2026-10-17-*.log`; screenshot, startup log and compositor
window evidence are under `/home/bry/Projects/lemon-validation-2026-10-17/`, outside the repository and vault.

## Installed build and rollback

Installed `/home/bry/.cargo/bin/lemonstudio` matches the tested release byte for byte:

`3afa247c53623e454d198b7d43a1037f0df4eafd7e82dc7bb7fff57335aa874d`

Installed Lemon Sampler, Lemon Parametric EQ and Lemon Trio in both `~/.vst3` and `~/.clap`; every installed bundle
was compared to its freshly built counterpart. The desktop entry still uses the absolute installed executable.
No real projects, user settings or global Hyprland configuration were changed. No existing Lemon session was killed.

Rollback snapshot: `/home/bry/Projects/lemonstudio-backups/2026-10-17/before-upstream-b680795/`.
It contains the previous executable, installed Lemon plugins, desktop entry and Cargo installation metadata.
Previous executable SHA-256: `7124fb4fe631fbdd0737a36d306fcce718e801414bd0b496ed9a70b86810f557`.
To roll back, close Lemon first, restore `bin/lemonstudio` to `~/.cargo/bin/lemonstudio`, and restore this snapshot's
Lemon VST3/CLAP bundles to their corresponding per-user folders. Do not replace unrelated third-party plugins.
The Cargo metadata is historical evidence; restoring it blindly could discard later installations.

## Ordered next steps

1. **Finish desktop acceptance before new features.** On this installed build, make a small beat, confirm default
   pattern names, edit a plugin text field, click back into the rack, then check typing notes, Space/Ctrl+Space,
   Ctrl+S, menus above docked editors, file dialogs and sample drag/drop. Save a disposable copy, reopen and export.
   Move an editor between the real monitors if available. Report an exact failing action rather than changing global
   compositor settings or promising all XWayland behavior is finished. Linux plugin-to-host shortcut forwarding
   and external sample drops still need explicit coverage; upstream preferences alone do not implement them.
2. **Make recording recoverable.** Reuse upstream's history for one undoable take, including replace mode, armed
   mid-playback, stop/pause and redo. Add explicit regressions before advertising recording as reliable.
3. **Fix recording timing/source boundaries.** Capture callback timestamps and device/channel identity, reconcile
   overlapping typing/MIDI notes and notes held across loops, then verify with Bry's actual controller. UI-poll
   timestamps and key-only release matching remain limitations.
4. **Harden save failure recovery.** Unique exclusive temporary names, failed-backup reporting and crash/durability
   checks are still needed. Autosave is useful, not permission to overwrite original FLP projects.
5. **Publish and verify the platform matrix when authorized.** Push normally, never force-push; inspect real Linux,
   Windows and macOS results. A successful local Linux run is not a remote CI result.
6. **Then choose Bry's next idea.** Prefer one small feature with clear acceptance checks and low overlap with
   upstream. Continue merging upstream on an integration branch, reviewing the patch inventory and installing only
   after validation and a rollback snapshot.

The earlier transient low-memory warning has not been reproduced and is not marked fixed. The separate 32-bit
Synth1/Yabridge scan watchdog failure is not proof of memory pressure and is not resolved by this integration.
