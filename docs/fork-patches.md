# Maintaining the downstream fork

`upstream/main` is the unmodified reference from ItsAlphaNeon/Lemon-Studio. Fork `main` is intended to contain the
tested combination of upstream and downstream work, not to be a second upstream mirror. Preserve published history;
do not reset or force-push main to update it.

## Patch inventory

Lemon Ray's first playable laser milestone is documented in [lemon-ray.md](lemon-ray.md). It is a separate
downstream instrument with stable identities and wrapper-based persistence, not a replacement for Trio.

The opt-in [local MCP bridge](lemon-mcp.md) reuses upstream project/history/host/export paths. It is a bounded
scratch-session composition milestone, not a network service or a second editing engine.

| Area | Decision and ownership | Verification |
| --- | --- | --- |
| Metronome | Reuse upstream's engine, sounds and settings; no parallel downstream click engine. | Engine metronome and app transport tests. |
| Cut groups and channel settings | Use upstream `os-flp::project::write_misc` as the sole writer. Retain the fork's rack self-cut controls. | Create/apply round trips, missing-event insertion, unchanged-save idempotence. |
| Plugin hosting and UI | Keep upstream VST2, CLAP mono/wakeup work, plugin/editor and interface changes. | Workspace tests plus bundled VST3/CLAP hosting tests. These do not prove arbitrary third-party plugins work. |
| Safer disk saves | Retain the fork's synced temporary-file/rename approach in `os-app::app`, alongside upstream overwrite backups, autosave and data-folder selection. | App save and upstream backup tests. Unique temporary names, backup failure reporting and crash/durability testing remain open. |
| Live effect replacement | Keep the fork's effect identity/state check before transferring live DSP in `os-engine::engine`. | Name/state transfer tests and an audio-output delay-to-reverb replacement regression. |
| Note recording | Adopt upstream's shared recording/capture/MIDI state machine and whole-take history; remove the older downstream recording implementation. | Upstream recording regressions plus fork-template take Undo/Redo, failed-start note preservation and project-meter count-in coverage. Hardware acceptance remains pending. |
| Drums and effects | Retain downstream procedural drums/template, delay and distortion. Use upstream Reeverb while preserving the original downstream DSP for legacy `LMR1` states. | Engine/DSP unit tests, old-implementation impulse fingerprints, v4 slot restoration and actual wet/dry project renders. No claim of FL Studio sonic parity. |
| Linux scan environment | Retain downstream additional library lookup for scanner children. | Scanner tests. This is not a guarantee that the runtime loader can instantiate every plugin found by scanning. |
| Playlist double-click | Keep downstream picker double-click alongside upstream right-click pattern menus. | App/editor tests. |
| Default pattern names | Keep an explicitly confirmed `Pattern N` name so an empty new pattern appears in the picker without custom typing. Browsing and cancellation must not create patterns. | Checkmark hit-testing, repeated creation, Enter, cancel/Escape, clearing an existing name and FLP save/reopen regressions in `os-app::patterns`. |
| Shared history and editing | Use upstream's global undo/redo, step-edit chord handling, preferences and responsive toolbar instead of parallel downstream versions. Keep audible step preview without changing upstream's left-paint/right-erase drag behavior. | Workspace history, step-edit, toolbar and mouse-button regressions. Live recording is not yet an undoable take. |
| Plugin discovery | Retain upstream's plugin database, visibility/format preferences and destination commands, but render plugin choices in a bounded searchable list instead of unlimited columns. Match names, vendors, categories and formats; keep favorites first and category/vendor/alphabetical modes. | Thousand-plugin search/navigation tests, destination preservation, real click/keyboard selection, held-key release, submenu behavior and small/scaled-window bounds in both designs. See [plugin search](plugin-search.md). |
| Meter and tempo | Use upstream `Project::bar`, bar-beat BPM and derived quarter-note tick tempo. Preserve runtime invalid-input bounds without clamping the derived tick tempo to displayed-BPM limits; preserve raw tempo metadata in FLP loading. | Meter/count-in tests, derived-tempo clock regression and FLP tests. |
| Hyprland/XWayland scaling | Retain X11 docking. Linux plugin frames inherit the X11 host scale and docked editors follow scale changes, preserving their active page; docking requests reconcile stale scales. Native Wayland's separate plugin frames retain X-server scaling. | Scale-selection/validation unit tests and the explicit display-dependent `docked_editor_follows_xwayland_parent_scale` check, covering real Lemon Sampler geometry and docking at fractional/integer scales. |
| CI and bundles | Run inherited platform matrix on main, integrate, feat and fix branches as well as PRs. Lock nested plugin builds. | CI must actually run remotely before claiming Windows/macOS verification. |

## Source attribution

The downstream-only files `crates/os-engine/src/drums.rs`, `crates/os-engine/src/effects/delay.rs`,
`crates/os-engine/src/effects/distortion.rs` and `crates/os-engine/src/effects/reverb.rs` came from the fork's
feature history, not the newly merged upstream implementation. Do not stamp upstream authorship onto them.
`tools/watermark` names those exact paths, Ray's five new source files, and the local bridge/adapter's six new source
and test files as exceptions to its upstream-specific header requirement. It validates that each path still exists
and continues checking every other source. These exceptions do not change the project license or remove existing notices.

## Update workflow

The latest local candidate and its release gates are documented in
[the October 22 integration record](upstream-integration-2026-10-22.md). Bry authorized source publication after
validation; installation and live hardware/listening acceptance remain separate release gates.

1. Fetch both remotes and review upstream changes in modules listed above.
2. Start an integration branch from the latest combined main (the initial integration starts from the latest feature
   branch, `36e483c`, to include work absent from old fork main). Merge upstream without rewriting history.
3. Resolve behavior, not just conflict markers. Compare auto-merged save/event writing, plugin state transfer and
   recording input paths. Prefer a stronger upstream equivalent rather than maintaining duplicate implementations.
4. Build all targets, build plugin bundles, then run workspace tests with the bundles present. Inspect test skips;
   FLP corpus tests need private fixtures and ignored hardware/UI tests need separate runs. Smoke-test the UI and
   retain a rollback binary before installing.
5. Promote only a validated result to fork main. Publish without force-push when authorized; check the remote platform
   matrix before claiming cross-platform success. Keep new features small and independently testable.

Local Linux validation is not Windows/macOS certification, and synthetic round trips are not private-project corpus
coverage. Keep experimental recording clearly labeled until its timing, loops, source identity and undo are covered.

## Initial integration verification — October 15, 2026

Combined downstream `36e483c4c70b5cb4c5dbae2f0363a6d2788ccb0c` with upstream
`ba78a0391419557a8bdc6c7bea003facf38c4d28` on `integrate/upstream-2026-10-15`. Linux checks:

- `cargo build --workspace --all-targets --locked --offline`: passed.
- `CARGO_NET_OFFLINE=true cargo run -p bundle-plugins --locked --offline`: built all three VST3/CLAP pairs.
- `cargo test --workspace --locked --offline`: 669 reported passes, zero failures, 25 ignored. Includes 229 app,
  105 engine, 17 CLAP and 24 VST3 unit tests. Fifteen reported corpus successes return without fixtures when
  `OPENSTUDIO_FLP_CORPUS` is unset; environment-dependent third-party tests are also conditional. These totals
  are not evidence of corpus, external-plugin or ignored device-test coverage.
- `cargo build -p os-app --release --locked --offline`: passed; release and debug mixer screenshots matched.
- Native debug app mapped a desktop window with isolated test settings and no logged startup errors. This was
  an opening/render smoke test, not a complete physical MIDI/audio/editor acceptance run.

Windows/macOS CI and physical MIDI/controller validation remain unverified for this downstream merge.

## Follow-up integration — October 17, 2026

Merged upstream `b6807951d64a37229a49f1c7522d4c7c1afb1a83` into the saved combined fork at `bc755a8`, preserving
history and the XWayland repair. See [the comparison, validation record and ordered next steps](upstream-integration-2026-10-17.md).
Linux workspace validation reported 980 passes, zero failures and 28 ignored, plus a separately passed real
XWayland docked-editor scaling regression. Fifteen corpus cases remain conditional without private fixtures.
Rebuilt and installed the app and all three VST3/CLAP pairs; this does not certify third-party plugins or other OSes.
