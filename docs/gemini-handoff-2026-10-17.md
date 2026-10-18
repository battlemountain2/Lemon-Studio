# Gemini handoff — compare the maintained fork with upstream

Historical checkpoint: Bry subsequently asked Codex to continue the work rather than delegate it. The completed
local integration and current next steps are in [the October 17 integration record](upstream-integration-2026-10-17.md).
The read-only instructions and revision/publication status below describe the earlier handoff, not the current task.

Prepared October 17, 2026, America/Denver. Start with assessment, not an automatic merge or another feature implementation.

## Bry's request

Compare our current Lemon Studio fork with current upstream, identify which remaining work upstream has already
handled, and say honestly which remaining tasks you can take on. Then explain what integrating upstream would look
like while keeping our useful features and the combined application installed on this PC.

Bry explicitly chose **XWayland with docked plugin windows on Hyprland**, not a native Wayland main window with
separate editors. Preserve that choice. Use plain language in the summary, with source evidence and a technical plan underneath.

Before changing code, answer:

1. What is already the same, what has upstream improved, and where is our version still stronger or unique?
2. Can you handle the remaining work? Which parts can you verify yourself, and which require Bry's controller,
   third-party plugins, another OS, actual mixed-DPI monitors or another missing resource?
3. What would integration look like: branches, conflicts, retained/replaced patches, testing, installation and rollback?
4. What is the smallest useful next step, and what approval or input do you need before doing it?

Do not promise compatibility based only on a clean textual merge or a green test count.

## Authoritative checkout and saved state

- Work in `/home/bry/Projects/Lemon-Studio`, on local `main`. Read applicable `AGENTS.md` files first.
- Latest runtime-code checkpoint: `76235c7277493ad2cf6b005484d44cd2190c1406`,
  `Fix docked plugin scaling under XWayland`. It contains the four previously uncommitted Hyprland/scaling files.
- This handoff is saved in a subsequent documentation commit. Use the actual local `HEAD` for a new comparison;
  `76235c7` identifies the executable's source checkpoint, not necessarily the newest documentation commit.
- Fetched GitHub fork `origin/main`: `058a16635dcf61b5a5275115d3138f18ef824435`, the upstream combination plus
  the default-pattern confirmation fix. The new XWayland checkpoint and this handoff are **local, not pushed**.
- Upstream main fetched October 17: `b6807951d64a37229a49f1c7522d4c7c1afb1a83`,
  `Merge pull request #62 from ItsAlphaNeon/options-implement`.
- Previously integrated upstream: `ba78a0391419557a8bdc6c7bea003facf38c4d28`.
- Earlier merge: `c9a95679504d3b469c47935ab7cbcfc047399fc3`, combining that upstream with fork feature
  `36e483c4c70b5cb4c5dbae2f0363a6d2788ccb0c`.
- At runtime-code checkpoint `76235c7`, the ancestry count against fetched upstream was **9 fork-only / 159 upstream-only
  commits**. That is a revision-specific count, not a count of independent features; the documentation commit adds a fork-only commit.
- Remotes: fetch origin `https://github.com/battlemountain2/Lemon-Studio.git`, push origin
  `git@github.com:battlemountain2/Lemon-Studio.git`, upstream `https://github.com/ItsAlphaNeon/Lemon-Studio.git`.
- Older `/home/bry/Lemon-Studio` is not the current combined checkout. Do not install from it.
- `/home/bry/Projects/Lemon-Studio-integration` is an older worktree on `integrate/upstream-2026-10-15`, at `c9a9567`.
  Its `target` directory is reused as a build cache; do not mistake that worktree's source for the latest main.
- Preserve the existing untracked `docs/audit-handoff-2026-10-14.md`. It is historical and is not staged by this handoff.

If you can only access GitHub, you do **not** have the latest XWayland code yet. Ask for this local checkout, the
checkpoint patch, or an authorized push rather than claiming to have reviewed code that is only on this PC.

## What has been completed

The patch inventory in `docs/fork-patches.md` is the starting ownership map. Independently check its claims.

| Area | Completed work / decision |
| --- | --- |
| Previous upstream integration | Retained upstream workspace members, VST2, CLAP mono/wakeup changes, UI work, live synchronization and shared metronome. Did not reset away the fork. |
| Cut-group persistence | Removed overlapping fork event writing; upstream `write_misc` is the sole writer. Missing-event insertion and unchanged-second-save tests cover the overlap. Retained self-cut controls. |
| Note recording | Kept recording, wait-for-input, count-in, blend and loop controls. Failed empty-SONG starts no longer clear notes. Count-in used the then-current meter API, with 3/4 and 6/8 tests. Still experimental. |
| Effects and drums | Kept procedural drum/template work and original delay/reverb/distortion. Retained effect identity/state checks before runtime DSP transfer, with an output-based delay-to-reverb regression. |
| Saving | Kept synced temporary-file/rename saves instead of writing directly over the project. Recovery/temporary-name hardening remains open. |
| Pattern creation | Confirming the default `Pattern N` name now creates a listed, persistent empty pattern without custom typing. Checkmark hit-testing, Enter, repeated creation, cancel and save/reopen are tested. Published in `058a166`. |
| XWayland scaling | Frames with an X11 owner inherit the main window scale; Linux docked editors now reopen on scale changes and retain their selected page. Docking requests reconcile stale scale. Native-Wayland ownerless X11 frames retain X-server scale. Invalid scales are handled safely. Saved in `76235c7`. |
| CI / provenance | Expanded branch CI triggers, locked nested plugin builds and documented exact exceptions for four fork-origin DSP source files instead of assigning them upstream authorship. Review against upstream's newer header checker; do not blanket-disable it or falsify attribution. |

The XWayland runtime patch is in `crates/os-app/src/plugin_window/x11.rs` and `crates/os-app/src/plugins.rs`;
its other changed files are `README.md` and `docs/fork-patches.md`. The scale test first failed at requested 150%
versus actual 100%; the stale-scale docking assertion also failed before its corresponding fix.

## Fresh verification and installed executable

Repeated October 17 against the saved runtime code:

- `cargo test --workspace --locked --offline`: **676 reported passes, zero failures, 26 ignored**.
- Explicit `docked_editor_follows_xwayland_parent_scale -- --ignored --nocapture`: **one pass**.
- The real Linux editor test embeds Lemon Sampler under an unmapped X11 parent, checks editor/area geometry at
  100%, 125%, 150% and 200%, checks actual X11 parentage and page preservation, and checks a docking request with
  a stale scale. It does not take desktop focus.
- October 15 all-target build, release installation and isolated startup checks passed. The app was observed by
  Hyprland as `xwayland: true`. These were not newly repeated as full desktop acceptance tests on October 17.
- Installed `/home/bry/.cargo/bin/lemonstudio` still matches the release artifact byte-for-byte:
  SHA-256 `7124fb4fe631fbdd0737a36d306fcce718e801414bd0b496ed9a70b86810f557`, 22,577,824 bytes.
- The launcher `/home/bry/.local/share/applications/lemonstudio.desktop` uses that executable's absolute path.
- Pre-scaling executable backup:
  `/home/bry/Projects/lemonstudio-backups/2026-10-15/before-hyprland-scaling/lemonstudio`.
  Earlier app/plugin backups are in the same dated backup directory.
- Fresh logs: `/home/bry/Projects/lemon-gemini-handoff-tests-2026-10-17.log` and
  `/home/bry/Projects/lemon-gemini-handoff-editor-2026-10-17.log`.
- Historical build/regression logs: `/home/bry/Projects/lemon-hyprland-*.log`.

Coverage limits: 15 reported corpus successes return without private FLP fixtures; other environment-dependent
checks are conditional. The ordinary run does not execute the 26 ignored checks. The separate display test only
covers one built-in editor and geometry/host-link behavior, not arbitrary plugins, audio playback, real compositor
detach/reparent transitions or text focus. The two physical monitors used 100% scaling during the earlier checks;
simulated scale changes do not certify real mixed-DPI migration. Windows/macOS remain unverified for this fork.
The GitHub Actions API still returned zero runs for remote checkpoint `058a166` on October 17.

## New upstream integration: what to examine first

Upstream advanced substantially: the diff from previously merged `ba78a03` to fetched `b680795` spans 248 files.
Do not reuse the October 15 two-conflict estimate or assume the old audit findings still describe current upstream.

Two concrete inspection leads, not a complete upstream audit:

- The latest model replaces the old `Project::time_signature` API with `Project::bar`; recent commits also change
  how tempo counts a bar's beats. Our count-in and recording code must be reconciled with the new model and tempo
  semantics, not merely renamed until it compiles. Check scheduler, seeking, exports and the 6/8 test expectations.
- In the fetched source, `Plugins::set_scale` still limits docked rescaling to Windows/macOS, and X11 frame creation
  still ignores `_scale` in favor of `conn.scale`. Our Linux repair appears relevant, but inspect the full current
  paths and tests before choosing retain/adapt/replace. An auto-merged X11 file is not proof of semantic compatibility.

A non-checkout preview was run at runtime checkpoint `76235c7`:

```bash
git merge-tree --write-tree --name-only 76235c7 upstream/main
```

Exit 1 indicated conflicts; preview tree `e7b8298f099d92d696499f5bcec25c420308b5a0`. This creates Git preview objects,
not a branch merge or a changed working tree/index. Full output:
`/home/bry/Projects/lemon-gemini-merge-preview-2026-10-17.log`.

Nine conflicted files:

- `README.md`
- `crates/os-app/src/app.rs`
- `crates/os-app/src/plugins.rs`
- `crates/os-app/src/rack.rs`
- `crates/os-app/src/views/toolbar.rs`
- `crates/os-engine/src/engine.rs`
- `crates/os-engine/src/sample.rs`
- `crates/os-flp/src/project.rs`
- `tools/watermark/src/lib.rs`

Auto-merged files still need review, especially X11 windows, pattern confirmation, clock/timing, MIDI input,
scanner behavior and effects. Recompute the preview if either branch moves.

## Remaining work to assess, in user-priority order

1. **Hyprland/XWayland usability:** plugin keyboard focus and transport/save shortcuts without stealing text entry;
   popup/dialog stacking and clipping; tiled/floating main windows; detached/re-docked editors; two-monitor movement;
   actual fractional-DPI setups; Linux sample drag/drop, starting with Lemon Sampler.
2. **Recording safety:** whole-take undo, reliable replace-mode recovery, callback timestamps instead of UI-consumption
   timing, note/device/channel identity and notes held across loop boundaries. Check what upstream now provides first.
3. **Project safety:** temporary-file uniqueness, write/rename failure tests, backups/recovery and clear missing-sample
   behavior rather than silently substituting audio. Earlier audit findings need fresh reproduction, not blind transplanting.
4. **Host/engine correctness:** remaining tempo/seek issues, state-load/save failure propagation, broader port/sidechain
   layouts and plugin compatibility. Separate issues already improved upstream from still-open ones.
5. **Verification:** real FLP corpus, representative third-party editors and physical MIDI/controller checks; working
   remote Windows/Linux/macOS CI. State explicitly what your environment cannot prove.

Separate observation: the 32-bit Synth1 Yabridge scanner child aborted while its 64-bit counterpart scanned
successfully. Bry also reported a transient memory warning inside Lemon that was no longer reproducible; system
memory was healthy when checked. The warning's connection to the bridge failure is unconfirmed. Do not claim it was
fixed, disable all plugins or change Wine/Hyprland globally based only on this observation.

## Requested review output and integration proposal

Produce a table with **feature/fix, our implementation, upstream equivalent, which is stronger and why,
retain/adapt/replace/drop, and verification needed**. Include code locations and distinguish observations from inference.
Say which remaining tasks you can handle, a confidence level and a concrete test plan; name missing resources.

Then answer Bry directly: **“What would integrating current upstream look like while preserving my useful features
in the binary installed on my PC?”** Cover these decisions:

- Keep `upstream/main` as the clean reference and fork `main` as the tested combined product.
- Propose an integration branch from latest local main, not the old feature branch or old integration worktree.
- Preserve published ancestry; no reset-to-upstream, force-push or whole-file “ours/theirs” conflict resolution.
- Explain conflict groups and auto-merge risks, including the meter/tempo refactor and source-header changes.
- Prefer stronger upstream equivalents where verified, carrying valuable behavior tests over rather than maintaining duplicates.
- Specify the staged build, bundle, test and manual acceptance gates before promotion.
- Explain that merging source alone does not update the installed executable. Installation must use the combined
  canonical source after verification, with a backup, byte comparison and a user-controlled restart after saving work.
- Describe normal push/CI verification and rollback. Do not call a locally successful merge cross-platform certified.

Read-only/reproducible starting commands:

```bash
cd /home/bry/Projects/Lemon-Studio
git status --short
git log -5 --oneline
git fetch origin
git fetch upstream
git rev-list --left-right --count HEAD...upstream/main
git log --oneline --left-right HEAD...upstream/main
git diff upstream/main...HEAD --stat
git diff HEAD...upstream/main --stat
git merge-tree --write-tree --name-only HEAD upstream/main
export CARGO_TARGET_DIR=/home/bry/Projects/Lemon-Studio-integration/target
cargo test --workspace --locked --offline
cargo test -p os-app --locked --offline docked_editor_follows_xwayland_parent_scale -- --ignored --nocapture
```

The merge-preview exit 1 is expected for conflicts. Offline commands require the cached dependencies. Before any
approved future integration is accepted, also build all targets, build the three VST3/CLAP plugin pairs, rerun tests
with those bundles present, inspect skips and perform the relevant hardware/editor checks.

**Assessment first:** report your comparison, feasibility assessment and proposed integration plan. Ask Bry whether
to proceed with that plan before creating branches, merging, committing, pushing, installing, closing his app,
changing settings or doing the remaining feature work. Do not send anything to upstream on Bry's behalf.
