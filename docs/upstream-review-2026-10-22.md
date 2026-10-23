# Gemini work and upstream comparison — October 22, 2026

> Follow-up: Bry approved the plan. Repairs and a separately tested integration candidate are now saved; see
> [the implementation and acceptance record](upstream-integration-2026-10-22.md). The audit below is the
> pre-repair snapshot. The installed application remains unchanged.

## Bottom line

Keep the fork's useful workflow additions and original instruments, but repair the compatibility gaps before
another integration. Prefer upstream's newer recording, export, MIDI and shutdown implementations instead of
maintaining competing versions. The wavetable synth is a promising first instrument, not a finished Serum-class
replacement. Upstream also has an unmerged control-plane branch that overlaps our MCP ambitions substantially.

This was a source and automated-test audit, not a live desktop/listening acceptance session. No merge, branch,
commit, installation, client restart or music-project edit was performed. Only this report and linked vault notes
are added to the reviewed checkout/vault. External probes and logs are outside both.

## Frozen comparison

Both remotes were fetched successfully during this review.

| Item | Verified snapshot |
|---|---|
| Fork checkout | `/home/bry/Projects/Lemon-Studio`, `main`, initially clean |
| Fork / origin main | `e3b8ba4da9178c220ea2bc2d86a361a44368aab3` |
| Upstream | `https://github.com/ItsAlphaNeon/Lemon-Studio` |
| Upstream main | `cf77bd052b4b50cdddfff3d87d6aeaaf604e7064` |
| Common ancestor | `6e9c3b9d7c7cfa5a31dc2dc087358ba04eebc96e` |
| Divergence | 25 fork-only / 42 upstream-only commits, including merges |
| New upstream main since ancestor | 82 changed files, 13,421 insertions / 877 deletions |
| Non-mutating merge preview | Eight conflicted files; 24 files changed on both sides |

The installed executable and cached release executable have identical SHA-256:
`c771f4391bb26357329029885695cb6cd6ede1dead473c22e753f2bb29579302`.
The installed file predates the final commit timestamp, but contains the new device-strip strings. This supports
that the feature is in the installed build; neither timestamps nor matching cached bytes prove the exact source
revision used. Tests below exercise current checked-out source, not that installed executable.

## What changed, and whose work it is

The recent first-parent history contains both newly developed features and upstream integrations. Commit authors
alone do not identify which assistant wrote a change; the grouping below follows the user-described work period
and actual diffs.

| Work | Assessment | Recommendation |
|---|---|---|
| `c354a16`: search, Ray, assistant bridge, tools | Saves earlier Codex-era work; not all new Gemini implementation | Preserve plugin identities, searchable destinations and MCP safety |
| `5bee769`, `e1e4b0e`: upstream integrations | Brought themes, channel replacement, ASIO/WASAPI/low-latency work, native Reverb/Limiter into the fork | Keep upstream ownership; do not reimplement or list these as still missing |
| `427a52d` through `503e03d`: Lemon Wavetable | Real DSP, UI, plugin and app integration; capability claims outrun implementation | Keep as experimental; fix MIDI/real-time behavior and add integrated persistence tests |
| `b6a4f94`: pattern mixer focus | Filters visible inserts from the pattern's note channels; does not mute the rest of the mix | Keep; useful, small, distinct improvement |
| `e917af4`: hierarchical Escape | A useful UI navigation addition with a targeted regression | Keep; reconcile with upstream's new dialogs/windows |
| `5cc6e40`: audio-clip editing | Gain overlay/reset and reverse affordances are useful; reverse has incorrect Undo coverage and shared-source behavior | Keep the UX, repair semantics/history |
| `e3b8ba4`: bottom device strip | Helpful channel → instrument → mixer effects overview | Keep the concept; correct action wiring and unsupported macro presentation |

### Wavetable: implemented versus planned

Implemented: one wavetable oscillator, three factory tables, 32 polyphonic voices, up to 16 unison voices,
phase-warp modes, stereo filter, two envelopes, one LFO, two fixed modulation paths, native editor, and
CLAP/VST3/built-in registration. Sixteen module tests pass.

Not equivalent to the full architecture described in `docs/lemon-wavetable.md`: no complete dual-oscillator,
sub/noise, dual-filter, effects-rack or editable general modulation-matrix workflow. `mod_matrix.rs` is not wired
into the rendering engine. The WAV parser exists as `table::Wavetable::from_serum_wav`, but has no call site in the
app/editor; arbitrary table import and persistence are not an end-to-end user feature. Envelope `tension` is
stored but not used by the ADSR processing. The documentation's VST3 ID `LemonStudioWave01` also differs from the
actual stable ID `LemonStudioWave0`; correct documentation rather than renaming a released plugin identity.

Treat the spec as a roadmap, not a completed-feature checklist. Mipmapping alone is also not evidence of
alias-free warped/nonlinear output; spectral and listening validation remain necessary.

## Findings to repair before integration

### P1 — Older Lemon Reverb states no longer load

`crates/os-engine/src/effects/mod.rs:47` dispatches `Lemon Reverb` to the replacement native-format decoder in
`effects/reverb.rs:98`. Our earlier effect saved 20-byte `LMR1` states, but that decoder expects the newer integer
layout and returns `None`. `engine.rs:1100` then counts the slot as bypassed. Supporting the display name is not
the same as supporting its old state format.

**Reproduced on real saved work:** a read-only probe loaded the polished Grave Light v4 FLP and passed its actual
states to the current effect factory. Eight `LMR1` reverb slots were rejected: mixer tracks 2, 5, 6, 8, 9, 11, 12
and 13. The FLP was not changed. This demonstrates a playback compatibility regression, not loss of project bytes.

Repair: dispatch old state magic to the legacy implementation, retaining its actual sound, or implement and
validate an explicit migration. Do not silently substitute default Reeverb settings. Add old-project render and
state round-trip regressions. The old Reverb fixture disappeared from the versioned-state regression while
Delay/Distortion remained, which explains why the full suite misses this.

### P1 — Audio reverse records the wrong Undo snapshot

`crates/os-app/src/playlist.rs:3546` takes a playlist snapshot, then toggles
`channel.sound.fx.flags` and updates the sampler. But `playlist.rs:1110` captures clips, markers, tracks,
automations and stretch values only; restore does not restore sample-FX flags.

**Source-verified:** Undo cannot revert this change. Moreover the flag belongs to the source channel, so reversing
one clip reverses every clip using that channel, despite the clip-oriented success message. The added test
toggles the command twice rather than testing Undo, so it misses the bug.

Repair: include the affected channel/plugin state in the history transaction. Decide explicitly between
source-wide reverse (label it honestly) and independently reversible clip instances; test two clips sharing a
source plus Undo/Redo and save/reopen.

### P2 — Wavetable pitch-wheel centre is a semitone sharp

`plugins/lemon-wavetable/src/lib.rs:147` forwards NIH-plug's `[0, 1]` pitch bend unchanged to a DSP method expecting
`[-1, 1]` (`dsp.rs:262`). The dependency's `NoteEvent::MidiPitchBend` contract says `0.5` is neutral.

**Reproduced:** centre value `0.5` changes A4 from 440 Hz to **466.163762 Hz (+100 cents)**. Minimum bend produces
no downward bend. The event's channel is also ignored and the engine updates every active voice.

Repair: normalize at the plugin boundary, track bends per MIDI channel, and test both extremes, centre and
simultaneous channels, including newly started notes after a bend.

### P2 — Wavetable allocates on the real-time render path

`plugins/lemon-wavetable/src/dsp.rs:307` returns a fresh `Vec<Ended>`; its first push when a released voice ends
allocates from `Plugin::process`. **Reproduced with a counting allocator:** one heap allocation in the measured
render call ending one voice, after engine/table/output setup had finished.

Repair: bounded preallocated termination storage or callback output. Include allocator assertions on release,
voice stealing and maximum polyphony. The code also fabricates termination event timing from list position
(`lib.rs:128`) rather than the actual ending sample, and stealing a voice overwrites it without a corresponding
termination notification. These need dedicated lifecycle tests, not just finite-output tests.

### P2 — Device-strip selection target also starts a volume drag

`crates/os-app/src/views/device_strip.rs:136` gives the target-mixer pill `Action::Fader`, even though its tooltip
says Select. `app.rs:7499` selects the track **and starts `Drag::Fader`**; subsequent vertical pointer movement
changes that track's level (`app.rs:4736`). A plain stationary click does not itself change the volume.

Repair: use a selection-only action and test real pointer down/move/up routing, not just direct state methods.
Separately, the strip's Cutoff/Res controls write `channel.sound` fields (`knobs.rs:185`), not the displayed hosted
instrument's own parameters. Do not present them as universal plugin macros: bind supported parameters explicitly
or disable/label unsupported cases. Their audible behavior still needs desktop acceptance.

## What upstream main added, and where its approach is stronger

| Area | Added since our last upstream merge | Why prefer it / integration decision |
|---|---|---|
| Recording | Note takes, channel volume/pan recording, count-in, start-on-input, blend, loop recording, selection punching, score-log recovery, armed UI | Dedicated `record.rs`, playback-clock integration and whole-take history snapshots are stronger than the fork's older embedded recording path. Adopt as the one recording state machine. |
| Audio capture | One-click recording, system/ASIO input selection and source header | New capability; test actual Linux/PipeWire inputs and Undo with files, not just state. |
| MIDI | Settings page, controller links, clock input/output, device routing | More complete than our earlier basic MIDI ingress. Reconcile typing/MIDI recording instead of retaining duplicate event paths. |
| Export | WAV/FLAC/MP3/OGG form, quality/rate/tail options, split-track choices, empty-export handling, MP3 gapless | A real shared export job is stronger than our WAV-only path plus external preview conversion. Adapt the MCP wrapper to it. |
| Headless rendering | `lemonstudio --render project.flp out.wav`, optional rate/length/start/built-in-only flags | Replaces reliance on a special ignored render test. Keep our workspace/no-overwrite protections around it. |
| Shutdown | Audio thread joined before plugin libraries are swept | Direct safety improvement for native-plugin exit crashes; take early. |
| Editing/navigation | Real Strum/Arpeggiate dialogs, browser resizing/long-name scrolling/context menus, project picker, touch keyboard/pads, persisted window geometry | Complement our search/focus/dock features; reconcile shortcuts and Escape priorities. |
| Playback fixes | Loop/Stop/pause fixes, one-shot handling, WAV `smpl` loops, sampler waveform preview, pattern selection sets playlist source | Adopt fixes with fork regression coverage, especially pattern creation and audio reverse. |

“Stronger” above is based on inspected integration, state/history architecture and added regression coverage, not
an assertion that every new upstream feature is bug-free or tested on this computer. Upstream recording's
`open_take` records pattern and, in song mode, playlist snapshots together (`record.rs:560`); this directly
addresses our earlier recoverable-recording goal.

## Important upstream branches — not merged into main

| Branch / reviewed head | What is there | Recommendation |
|---|---|---|
| `control-plane` / `c9d0938abe7b50e2d6352a890e60baca04888b84` | Local JSON-RPC; GUI/headless modes; read/propose/edit; stable session IDs; batched atomic edits; shared UI history; mixer, effects, notes, clips, plugin parameters; proposal review; polling events | Most relevant next architectural step. Prefer an MCP adapter over this shared command layer rather than maintaining a separate assistant-only edit engine. Not a drop-in replacement today. |
| `lemon-delay` / `0d8c5e0192c62979e09b4d32fa9f9252e510cf6c` | Fruity Delay 3-compatible native DSP, editor, CLAP/VST3 plugin | More complete than our small ping-pong delay. Evaluate after main, retaining `LMD1` state decoding and old sound to avoid repeating the Reverb regression. |
| `older-projects` / `ce32c3bf352c3274b78bc34c5c4363e366df409c` | In-memory conversion of FL 20–24.1 layouts; converted projects open unnamed so Save does not overwrite their originals | Useful compatibility addition. Validate private corpus, routing and Save As; not a claim of universal legacy FL support. |
| `citrus` / `1c51adb60ec96a83b92d8b950fc3220f2f367373` | Six-operator FM/Sytrus-state synth | Complements Ray and Wavetable; not the same synth architecture or evidence that upstream rebuilt Serum better. Do not confuse it with the Citrus UI theme. |

The control plane's source explicitly enforces user-only Unix socket permissions, loopback TCP, a token handshake
on TCP, and malformed-JSON connection closure. Its history/proposal design is broader than ours. However its
default level is **edit**, headless saves write directly, IDs are not persistent across loads, and rendering is
not exposed through the protocol. Do not remove our opt-in behavior, workspace path restrictions, new-output-only
saves/renders, revision checks, operation receipts and approval policy just because the upstream protocol exists.
Re-evaluate proposal side effects and transactional failure tests during adoption. The branch is not current main;
main's later recording/export changes need reconciling with it too.

## Concrete merge plan

1. Repair old-Reverb compatibility, reverse Undo and Wavetable bend/allocations first. Add regression fixtures
   before broad integration; preserve current plugin IDs and opaque project states.
2. When authorized, create a separate integration worktree from tested fork main. Keep the currently installed
   app/plugins and project originals unchanged; record rollback hashes and source/bundle provenance.
3. Integrate the pinned latest upstream main as one coherent update. Use upstream recording/capture/MIDI/export/
   shutdown as the shared foundation; reapply our template, pattern-confirmation, plugin-search, Ray/Wavetable,
   mixer-focus, Escape, clip-UX, device-strip and XWayland improvements deliberately.
4. Resolve the eight preview conflicts: `README.md`, `app.rs`, `browser.rs`, `midi_in.rs`, `playlist.rs`,
   `views/menus.rs`, `views/toolbar.rs`, `docs/status.md` (source paths under `crates/os-app/src/`). Do not use
   blanket ours/theirs resolution on the app or playlist.
5. Audit auto-merges too: `main.rs`, `history.rs`, `knobs.rs`, `patterns.rs`, `plugins.rs`, engine/sample behavior
   and transport/export integration. Upstream replaces `render_check.rs` and changes the export API: our assistant
   calls must migrate to the new job/results interface while preserving asynchronous completion and safeguards.
   Eight resolved files alone do not mean a safe merge.
6. Validate existing v3/v4 FLPs on **copies**: plugin/state identity, native effects active, mixer routing, Undo/
   Redo, notes/samples, save/reopen, wet/dry render comparisons. Check real MIDI/audio recording, formats/export
   tails and app shutdown. Re-run the Hyprland docked-plugin scale/focus/popup checks with the new UI dock.
7. Only after acceptance, install a separately backed-up combined build. Evaluate the control-plane branch as a
   second integration; Delay/Citrus/legacy-file support remain separate decisions, not one giant experimental merge.

## Verification and gaps

- Fork workspace tests: **1,108 passed, zero failed, 35 ignored** using the current canonical target directory,
  isolated user settings, `--locked --offline -j 2`, and explicit Ray bundles.
- Python MCP tests: **8 passed**. They validate the adapter; they are not a fresh GUI tool transaction.
- Full workspace/all-targets Linux development build: **passed**, using `--locked --offline -j 2`; existing
  compiler warnings remain. This did not replace the installed release executable or plugins.
- External Rust probes: old 20-byte Reverb fixture rejected; all eight actual v4 legacy reverbs rejected;
  neutral pitch bend +100 cents; one allocation in voice-ending render call.
- Merge preview: `git merge-tree --write-tree main upstream/main`, exit 1 for the eight conflicts; no checkout or
  index changes. It creates only Git objects, not an installed/merged application.
- Logs and reproducible probe source: `/home/bry/Projects/lemon-review-2026-10-22/`.
- Ignored or environment-conditional tests are not evidence of private-corpus, live audio, plugin-window,
  cross-platform or hardware acceptance. Upstream and its experimental branches were source-reviewed, not built
  and tested as separate products here. No listening comparison or fresh live Hyprland UI regression was run.

The recommended next implementation milestone is **compatibility and history repairs, then current upstream main**,
not another large synth feature. This protects our existing music while gaining recording/export work we already
wanted, and leaves a clearer foundation for a better MCP.
