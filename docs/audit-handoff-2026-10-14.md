# Lemon Studio: audit and upstream integration handoff

Prepared October 14, 2026, America/Denver.

> **October 15 continuation:** The current preference is an active downstream fork: `upstream/main` is the clean reference, and fork `main` should receive the tested upstream-plus-feature result. This supersedes this historical handoff's mirror-main recommendation. Current inspected tips are fork feature `36e483c` and upstream `ba78a03`, with two merge-conflict files (`Cargo.toml`, `app.rs`). Earlier upstream integration is already on the feature branch. The named `upstream` remote is now configured. Updated overlap decisions and validation risks are in `/home/bry/notes/projects/Lemon Studio/Fork vs Upstream - 2026-10-15.md`. No new merge or branch update has been performed in the continuation.

## Read this first

The completed audit reviewed feature commit `d56d419`, not the newer audit-fix commit `f41c56e`. Fetching the remotes today found both a new fork fix and new upstream UI work. Use the revisions below when continuing; do not mistake yesterday's test results for validation of today's branches.

Recommended next work: integrate the **latest feature branch with upstream main on a new integration branch**, use upstream's metronome/tempo-tapper system, preserve the fork's distinct workflow improvements and reviewed audit fixes, and verify the result before publishing it. Keep the fork's main branch a clean upstream baseline if the fork will continue tracking upstream.

Full audit on this workstation: `/home/bry/notes/projects/Lemon Studio/Architecture and Code Audit - 2026-10-13.md`. Original request and pasted roadmap: `/home/bry/notes/projects/Lemon Studio/Codex Prompt.md`. The full audit contains 24 findings, architecture/dependency mapping, a roadmap verification table, engine design recommendations, legal/provenance sources, an MCP threat/design review, and ordered cleanup tasks. This handoff includes the evidence and integration decisions needed to continue without the chat history.

## Exact repository state

Repository: `/home/bry/Projects/Lemon-Studio`.

| Ref | Revision at handoff | Meaning |
|---|---|---|
| Current detached `HEAD` | `d56d419dfc8dbfe43880c70b6a1f090a404703ff` | Originally audited feature commit |
| Local `main` and `origin/main` | `6cb4dc8c082cebab80e02a52bc1be589e4414e1d` | Old shared baseline |
| `origin/feat/fl-workflow-and-dariacore-fx` | `f41c56ee1dc8c3ef659c1427b9a9a90b18aea840` | Original feature plus newer audit-fix commit |
| Originally audited upstream main | `a1016318a71576395371b98818f21bb6c393a896` | Included macOS work |
| Freshly fetched `upstream/main` | `f1962a75709e23a658b3a07c04942ca37154fab9` | Adds UI batch including its own metronome and tempo tapper |

At this snapshot, upstream and the latest feature branch have **39 and 2 exclusive commits**, respectively. Their merge base is `6cb4dc8`. The fork's main branch can fast-forward to this upstream history. Recheck ancestry after fetching if continuing later.

`origin` is configured as `https://github.com/battlemountain2/Lemon-Studio.git`. An `upstream/main` tracking ref exists, but **no remote named `upstream` is configured**: it was fetched directly from `https://github.com/ItsAlphaNeon/Lemon-Studio.git` into that ref. Add the remote before using `git fetch upstream`.

The product checkout was clean before this handoff document was added. No product source was edited during the audit/handoff. No branch, merge commit, rebase, push or PR was created. Fetch updated remote refs; `git merge-tree` wrote a preview tree to Git's object database without changing the checkout or index.

## Completed work and evidence

- Reviewed the fork contribution first: 17 files, +1,050 / −29 lines at `d56d419` relative to the shared baseline. Added drums/template, limiter default, audition, picker double-click, cut toggle, metronome, DSP prototypes and default app selection; no new dependencies.
- Mapped the actual 22-package workspace and runtime threads. GUI export attaches hosts; `os-render` CLI does not. Most packages use edition 2024; `os-vst3` intentionally uses edition 2021.
- Inspected engine, transport queues, unsafe CLAP/VST3 hosting, scanning/crash protection, FLP container/model serialization, CI, dependencies, rename compatibility and project state handling.
- Ran `cargo test --workspace --locked --offline` at `d56d419`: **520 reported passes, zero failures, 23 explicitly ignored**. Fifteen reported corpus passes were no-op returns because `OPENSTUDIO_FLP_CORPUS` was absent. Optional bundle tests initially skipped before bundles existed.
- Built all three release CLAP/VST3 bundles using `CARGO_NET_OFFLINE=true cargo run -p bundle-plugins --locked --offline`, without installation.
- Re-ran `cargo test -p os-clap -p os-vst3 -p os-app --locked --offline -- --nocapture`: **16 + 23 + 142 reported passes**, with 21 app tests ignored. Bundles were present for this run. Environment-specific external-plugin checks were still conditional.
- Reproduced the bugs below with an independent executable using the public model/engine/FLP APIs.
- Fetched today's branches, inspected `f41c56e` changes and upstream metronome integration, and performed a merge preview identifying six conflicted files. This was source inspection, not a test run of the newer fix or merged result.

### Reproduced observations at d56d419

| Audit ID | Observation | Regression acceptance criterion |
|---|---|---|
| F01 | An existing `Custom 808 Kick.wav`, 48 kHz / 32 frames, decoded as a generated 44.1 kHz / 24,255-frame kick. Missing `what-vocal.wav` returned a hat sample. | Normal paths decode their actual contents or fail; only explicit built-in IDs invoke procedural synthesis. |
| F02 | Different serialized delay states produced identical audio. FL-branded delay/reverb/distortion names were dispatched to generic defaults. | Distinct valid own-effect states change DSP; unsupported FL states remain preserved/unavailable unless an explicit adapter exists. |
| F03 | Channel cut/cut-by `(7,7)` became `(0,0)` after create/apply/serialize/reload, while `unsaved_parts` was empty. | New and existing project edits persist, and unsupported writes are surfaced before marking saved. |
| F05 | Replacing Delay with Reverb in the same live slot produced exactly the unchanged Delay's output for 100 compared ticks. | Compare actual impulse/output behavior and configuration changes, not just the displayed/stored effect name. |
| F08 | Zero BPM made the clock's samples-per-tick nonfinite and nominal size `usize::MAX`. | Invalid values fail or follow an explicitly documented safe policy; valid project timing remains consistent across all paths. |
| F09 | At 48 kHz, tick 200 under 138-BPM automation over a 130-BPM base was frame **43,492 sequentially versus 46,154 after seek**. | Sequential playback and seeking use the same tempo timeline and sample offsets. |

The probe source and raw logs were originally in `/tmp/lemon-audit-*` and **are no longer present**. Historical results above come from the completed audit/session; no claim is made that raw evidence files remain available. Rebuild permanent regression tests from these cases in the repository. Do not restore the original probes unchanged as a passing regression suite: they asserted the presence of bugs.

No private real-project corpus, measured FL reference specifications/renders, Windows/macOS runtime, external plugin matrix, fuzz campaign, advisory scan or real-time stress benchmark was available. `docs/` lacks the extensive specifications named in source comments. Linux builds and own-plugin tests do not establish cross-platform or FL fidelity.

## New fork fix: inspected, not independently validated

Commit `f41c56e` is already on the remote feature branch. Do not restart implementation from `d56d419` and lose it. Its diff changes 12 files, +416 / −116 lines.

| Finding | Source change present in f41c56e | Follow-up before calling it closed |
|---|---|---|
| F01: sample substitution | Explicit `builtin:` IDs; ordinary read failures propagate; channel-name drum fallback removed | Test real/missing filenames and directory substrings through GUI sampler and engine. |
| F02: fake effect compatibility | Removed FL delay/reverb/distortion aliases; added own-effect state encoding/decoding | Check malformed/unknown state version behavior, parameter persistence, and whether effects are discoverable/editable. |
| F03: cut persistence | Writes cut-group events on new channels and edits | Test actual app save/reopen and existing-file byte preservation. |
| F04: direct overwrite | Writes/syncs a temp file, then renames | Temp name is filename + PID and uses `File::create`; add unique exclusive creation, backup/recovery and fault-injection tests. Verify replacement on Windows and directory durability policy. |
| F05: stale DSP | Transfer now checks slot, name and saved state | New test only checks `FxSlot.name`; the original bug swaps the `fx` object, so the test needs an audible/DSP assertion. Preserve the fix during integration. |
| F08: invalid clock | Clamps tempo/PPQ and substitutes a default | This is partial numeric hardening, not comprehensive model validation. Check supported ranges and consistent use of project PPQ/tempo across subsystems; prefer visible errors for unsupported inputs. |
| F11: cut collisions | Searches unused groups before fallback | Verify representable group bounds, exhaustion, imported groups and toggle-off semantics. |
| F12: metronome | Sets switch before prefill, enables menu, derives beat/bar values from signature | Supersede the fork's implementation with upstream's unified metronome while preserving acceptance cases. |

F09 tempo-aware seek remains a follow-up: the reviewed fix does not change `seek_tick` or `clips_under_way`. Other major open work includes CLAP bus negotiation/request_process, plugin state failure handling, PDC/sidechains, tail/export contracts, global undo/recovery, and corpus/test honesty. Consult F01–F24 in the full audit for detail.

## What upstream brings

The original 26-commit gap already included macOS plugin loading, editors/docking, text rendering, bundles, native menu integration and macOS CI. Today's gap also includes:

- Shared metronome switch, sound selection/settings and tests.
- Tempo tapper with click feedback, toolbar/menu/shortcut integration and test recording of click requests.
- Auto/continuous scrolling and enable-groups controls.
- Song position slider interactions.
- Piano event-panel splitter and close-window commands.
- Unused-channel selection and unused-audio-clip purge tools.

Retain this upstream UI/platform work. The fork's sine-click metronome duplicates upstream's now broader feature and would introduce two sources of truth if mechanically retained.

## Merge preview and conflict decisions

Executed with Git 2.56.0:

```bash
git merge-tree --write-tree --name-only upstream/main origin/feat/fl-workflow-and-dariacore-fx
```

The command returned exit status **1**, meaning merge conflicts, with preview tree `0652be660a4509dd77a46f46750bf65c776f8cd9`. This is not a merge commit and is not suitable for checking out/building as a resolved result. The working tree/index stayed untouched. The actual merge may show different hunk orientation because the preview used upstream as its first parent.

| Conflicted file | Resolution policy |
|---|---|
| `Cargo.toml` | Keep upstream's workspace members, including `tools/bundle-app`, and retain fork `default-members = ["crates/os-app"]`. Keep the app's `default-run`. |
| `crates/os-app/src/app.rs` | Keep upstream metronome action/control/settings and new AutoScroll/EnableGroups/SongPos integration. Retain fork template, audition/picker improvements and atomic-save work after review. Remove duplicate metronome methods/state/actions/tests. |
| `crates/os-app/src/transport.rs` | Keep upstream `Arc<metronome::Switch>` attachment before prefill and its NoteOn/TapClick path. Tempo tapping can create a live stream from an empty project. Remove fork boolean command/state and extra Session::start argument; update every call site and rebuild path consistently. |
| `crates/os-app/src/views/menus.rs` | Keep upstream label constants, `mark_of`, metronome sound/settings menus and newer controls; remove duplicate string-based dispatch/mark logic. |
| `crates/os-app/src/views/toolbar.rs` | Keep upstream layout/control placement; it already has the metronome button. Fork's `i == 0` handling would duplicate/misplace the metronome after the upstream button shift. |
| `crates/os-engine/src/engine.rs` | Keep upstream `Option<Metronome>`, `beats_per_accent`, `TapClicks`, shared-switch API, master-bus rendering and tests. Retain fork effect identity/state transfer fix and reviewed numeric validation. Remove old sine-oscillator fields/constructor values/render block/boolean API and adapt fork tests. |

**Cleanly merged text can still be wrong.** The preview automatically retains some fork sine-click/render/transfer code around upstream changes. Selecting upstream at conflict markers alone leaves obsolete references and can produce duplicate clicks or compile errors. Search and reconcile the entire metronome path, including `continue_from`, render placement, export, live-only streams and tests. The fork clock file can also merge cleanly while changing timing semantics.

Upstream's design attaches the shared switch inside Session::start before prefill, keeps previews/live notes free of regular metronome clicks, and provides a separate tap-click path. Preserve those behaviors. Add meter cases such as 3/4 and 6/8 rather than assuming either implementation is complete by inspection.

Do not resolve whole files with “ours” or “theirs”: that loses either upstream platform/UI work or independent fork fixes.

## Suggested integration runbook

These commands are **for the next implementation step**; branch changes, merge, commit and push have not been executed. Stop if the working tree contains work you have not accounted for. This handoff itself is currently an uncommitted document: preserve it outside the checkout or include it in an explicitly authorized documentation commit before starting a merge.

1. Read applicable instructions, inspect status/remotes and fetch both histories. Configure the upstream remote once; it is currently absent:

```bash
cd /home/bry/Projects/Lemon-Studio
git status --short
git remote -v
git remote add upstream https://github.com/ItsAlphaNeon/Lemon-Studio.git
git fetch upstream
git fetch origin
git log --oneline --left-right upstream/main...origin/feat/fl-workflow-and-dariacore-fx
```

2. Verify the fork main is still an ancestor, then advance local main with a fast-forward only:

```bash
git merge-base --is-ancestor origin/main upstream/main
git switch main
git merge --ff-only upstream/main
```

If ancestry fails, inspect new fork-main commits instead of resetting/force-pushing. Remote main publication is a separate step after verification. Keeping main as the upstream mirror makes later updates simpler.

3. Create a new integration branch from the **latest** remote feature tip. The name below is a proposal; keep the original feature branch intact:

```bash
git switch -c integrate/upstream-2026-10-14 origin/feat/fl-workflow-and-dariacore-fx
git merge --no-ff --no-commit upstream/main
```

The merge is expected to stop with conflicts. Apply the resolution table; retain the useful fork changes and replace its metronome with upstream's. A merge preserves both existing fork commits and upstream history without rewriting a shared feature branch. If you decide to abandon the attempt, `git merge --abort` restores the pre-merge state; preserve any extra work made during the attempt first.

4. Review all auto-merged files, stage deliberate resolutions, and check for leftover markers. In the merged source, boolean metronome commands and oscillator fields should be gone; legitimate shared-switch calls remain:

```bash
git diff --name-only --diff-filter=U
git diff --check
rg -n '^(<<<<<<<|=======|>>>>>>>)' Cargo.toml crates plugins tools
rg -n 'Cmd::Metronome|metronome_phase|metronome_freq|metronome_on|set_metronome' crates/os-app/src crates/os-engine/src
```

The marker search returns status 1 when nothing matches; that is expected. The last search is a review aid, not a rule that every occurrence is invalid. Inspect metronome tests/settings too.

5. Validate and record fresh results for the merged tree. Use logs in a durable directory under `/home/bry/Projects`, not `/tmp`:

```bash
cargo build --workspace --all-targets --locked
cargo test -p os-engine -p os-flp --locked
CARGO_NET_OFFLINE=true cargo run -p bundle-plugins --locked --offline
cargo test --workspace --locked
```

If dependencies are unavailable offline, run the bundle build with network access and record that prerequisite. `bundle-plugins` invokes nested Cargo builds that do not propagate `--locked`; check `Cargo.lock` afterwards. Avoid `--install` during validation. Missing corpus still needs explicit reporting; do not count no-op corpus passes as coverage. Use a real locally supplied corpus in a separate required conformance run when available.

Required behavioral checks: sample resolution; cut-group save/reopen; actual Delay-to-Reverb output after live replacement; first metronome downbeat, master mute, meter, toggle/reopen and export exclusion; tapper before any loaded project; upstream scrolling/group/position controls; plugin editor lifecycle; failed-save preservation. Reproduce tempo seek mismatch and track it as open if it remains. Windows/macOS CI plus manual device/editor checks are still necessary.

6. Inspect the staged result and commit the merge only after the resolutions/checks are reviewed. A prospective merge message is `Merge upstream main and reconcile metronome with fork fixes`. Publish the integration branch for review. If keeping fork main as an upstream mirror, publish the verified main fast-forward separately using a normal `git push origin main`. No force push is needed. If you eventually want feature work on fork main, merge the reviewed integration branch there deliberately; that changes the mirror policy.

## Architectural decisions to carry forward

- Safety and correct save/reopen come first. Preserve original unknown/plugin state; unsupported edits must not silently become a successful save.
- Treat exact plugin IDs, FLP identifiers, and legacy environment/config migration as compatibility contracts. Avoid a blanket `os-*`/OpenStudio rename.
- Build stable IDs, document transactions and global undo/redo before broad MCP mutation. Keep validation, parsing, state I/O and compilation out of the device callback.
- Port-aware routing/PDC and a consistent tempo/render timeline precede parallel mixing. Sidechain-as-zero-send is not an actual plugin sidechain input.
- Preserve CLAP's symbolic-thread semantics: serialized audio operations may run on the main OS thread. Do not flag UI flush alone as a specification violation; implement thread-check and lifecycle tests.
- Process scanning is crash containment, not a permissions sandbox. Linux runtime plugins remain in-process; Windows SEH is conditional and cannot undo heap corruption.
- Give original Lemon DSP distinct identities and documented state; keep FL compatibility aliases only where a validated adapter exists.
- Defer the large named FL-plugin clone backlog. Original small instruments/effects, reliable CLAP/VST3 hosting, recoverable editing and a complete Linux workflow have higher value.

## Next-agent starting instructions

Read this file and the full audit. Verify refs and existing edits. Start from latest remote feature `f41c56e` or its successor, not detached HEAD `d56d419`. Reconcile upstream's metronome across the six preview conflict files and surrounding auto-merged code, retaining separate fork fixes. Validate behavior and platform builds; do not claim the audit is fully resolved from passing counts alone. Update the handoff/audit tasks with actual revisions, tests and remaining risks. Commit/push only when directly authorized.
