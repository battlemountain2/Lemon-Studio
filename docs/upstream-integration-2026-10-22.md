# Tested integration candidate — October 22, 2026

## Authorized source publication

Bry approved the repair/integration plan after the source audit and subsequently authorized committing and pushing
the tested integration to fork main. Source publication is separate from release installation: the installed
application, plugin bundles, MCP client configuration and original music projects remain unchanged.

- Canonical publication target: `/home/bry/Projects/Lemon-Studio`, branch `main`.
- Combined candidate: `/home/bry/Projects/Lemon-Studio-upstream-2026-10-22`.
- At validation, the candidate was a detached worktree with an uncommitted merge: HEAD
  `e3b8ba4da9178c220ea2bc2d86a361a44368aab3`; merged upstream
  `cf77bd052b4b50cdddfff3d87d6aeaaf604e7064`.
- All eight merge conflicts are resolved. The shared recording/export/MIDI/shutdown paths use upstream's
  implementation; the old downstream recording state machine is removed from the candidate.
- Experimental upstream control-plane, Delay, Citrus and older-project branches are **not** merged.
- Logs, project copies, rendered controls and probe sources:
  `/home/bry/Projects/lemon-review-2026-10-22/`.

The desktop worktree tool could not create a checkout because this chat has no repository attachment, so the
separate worktree was created directly with Git at the approved source revision. No new branch was created.

## Repairs retained

### Legacy Reverb

Restored the original downstream Schroeder/Freeverb DSP under
`crates/os-engine/src/effects/legacy_reverb.rs`. Only legacy `LMR1` states select it; upstream Reeverb's native
state reader and plugin remain unchanged. Truncated and nonfinite legacy payloads are rejected rather than
silently replaced. The exact old processing algorithm is preserved, not an approximate parameter migration.

One-second stereo impulse fingerprints were generated from the old implementation at `e8f4070` and checked
against the repaired factory at both rates: 44.1 kHz `1589a91a6ca74fc6`, 48 kHz `c395ee7689154e1c`
(FNV-1a over little-endian float samples). Reset and legacy state round-trip tests pass.

### Reverse and device-strip behavior

Reverse now records the actual source-channel flags in shared history. Undo/Redo also queues the hosted sampler
to follow the restored settings. The UI explicitly calls it **Reverse source sample** and says it affects all
clips using that source. Selecting two clips with the same source toggles it once; the change survives FLP save/
reopen. This is intentionally source-wide, not a new per-instance destructive audio editor.

The device-strip mixer destination uses a selection action rather than a volume-fader drag. Its sampler filter
controls are shown only on sampler/audio-clip channels; hosted instruments direct users to their plugin editor.
Pointer-motion and unsupported-control regressions pass. The integrated sampler updates preserve upstream's
audio-clip loop policy. Sampler filter MIDI-link targets also serialize and reload correctly.

### Wavetable

Pitch wheel input is normalized from the actual NIH-plug contract: 0.5 is neutral, with independent per-channel
bend and correct bend on newly started notes. A4 at centre is now exactly 440 Hz.

Voice-ending notifications use a callback instead of an allocating vector and carry their actual sample offsets.
Voice stealing reports the displaced identity; note choke, All Notes Off and All Sound Off are handled. Output
buffers are overwritten/cleared rather than relying on the host's initial contents. A counting-allocator test
covers rendering, release, stealing and choke at 32 voices / 16 unison without heap allocations.

The actual VST3 identity remains `LemonStudioWave0`; its documentation was corrected instead of changing plugin
identity. Existing Wavetable and Ray project/state/audio round-trip tests pass. Wavetable remains an experimental
instrument foundation, not a completed Serum-equivalent feature set.

## Upstream integration decisions

- Use upstream's recoverable note/automation recording, input capture, MIDI devices/controller links/clock,
  multi-format export, native headless renderer and joined audio-thread shutdown.
- Preserve the fork's default drum/template/limiter behavior, default-pattern confirmation, searchable plugin
  destinations, Ray/Wavetable identities, effect identity/state handoff guard, safer saves, mixer focus,
  clip-edit affordances, device strip and XWayland scale repair.
- Retain upstream browser resizing alongside assistant history capture, and combine playlist handle/picker
  state with upstream ruler-selection state.
- Escape navigation recognizes the new Touch/Strum/Arpeggiate windows, with preview cancellation routed through
  their normal close actions.
- Port the MCP export call/result to upstream's shared export Job. Retain workspace restrictions, session/revision
  checks, operation receipts, checkpoints, ordinary Undo, async status and new-file-only publication.
- Reject assistant edits while Record is armed or waiting, during count-in and while modal dialogs are open.
  Headless assistant startup explicitly suppresses opening an audio device.
- Keep the existing downstream-attribution policy: watermark's exact-path fork inventory includes the restored
  legacy module and new real-time regression; other source-header validation remains active.

A new upstream long-clip export test assumed a unity-gain template. Its fixture now explicitly removes default
effects so it checks clip timing/loop behavior without being altered by the fork's intentional master limiter.
Global record-shortcut tests likewise remove editor focus so playlist/piano-roll shortcuts keep their intended
priority. Production template or shortcut behavior was not weakened to make the tests pass.

## Verification

- **1,213 workspace tests passed; zero failed; 37 ignored**, with isolated settings, explicit Ray bundles,
  `--locked --offline -j 2`.
- Full Linux workspace/all-targets check and build passed. The first offline check needed uncached upstream
  export dependencies; a locked online check fetched them, and subsequent validation ran offline.
- **Eight Python MCP tests passed.**
- Two explicitly run ignored XWayland checks passed on this Wayland session's X11 display: Lemon Sampler
  docking/scaling and Ray docking/scaling/real parameter edits. Both include 1x, 1.25x, 1.5x and 2x scales and
  use unmapped test parents, not the user's Lemon window.
- Real stdio MCP → private socket → app acceptance passed in a fresh scratch workspace: eight tools, batched
  composition, Undo/Redo, FLP save, async WAV completion, and rejection of stale revisions, traversal and
  overwriting both saved projects and renders. The temporary test processes were stopped.
- Read/apply/reopen checks on v3 and v4 copies preserve notes, arrangements, slot states, slot switches/mix and
  routes; the original source-file bytes remain unchanged. All eight v4 legacy Reverb states now load.
- Full headless renders: v3 25.7667 seconds; v4 53.984375 seconds. V4 peak is -5.676 dBFS.
- V4's original FLP and its saved/reopened copy produce **byte-identical WAVs** in the candidate:
  SHA-256 `83ff02f9938a8448aaa680cc30b3c99c10eb9c7db0ad417b71fcb192ce3173b1`.
- Bypassing only the eight legacy reverbs changes the rendered audio (difference RMS 0.00351732, -49.076 dBFS).
  This verifies that restoration affects the real mix, not just a decoder return value.

These are technical regressions and selected real-project checks, not listening approval, microphone/controller
hardware certification, complete private-corpus coverage or Windows/macOS validation. No user-visible interactive
session or installed-release acceptance was claimed.

## Preview and release gate

Candidate development executable:
`/home/bry/Projects/Lemon-Studio/target/debug/lemonstudio`.

Its SHA-256 at verification:
`e6f613d18b25d842956304ab14f77b773bf2af225707148d44512c16dcf16fb1`.

Installed executable SHA-256 remains unchanged:
`c771f4391bb26357329029885695cb6cd6ede1dead473c22e753f2bb29579302`.

The target directory is shared: rebuilding from another checkout can replace the verified debug binary.
Continue combined work from the October 22 integration worktree or canonical main after source promotion,
not an older integration checkout.

A prepared preview launcher is
`/home/bry/Projects/lemon-review-2026-10-22/run-integration-preview.sh`.
It uses isolated settings, disables plugin auto-install/rebuild, and opens the test copy of v4 by default.
It has not been launched. Save any currently open work before trying a second audio application instance.

Bry authorized the merge commit and publication of this tested source to main. Next, audition/use the candidate
with Bry and test real capture/controller input, shortcuts and window focus before any separately authorized,
backed-up release installation. Evaluate the experimental control-plane branch for the MCP only as a separate
follow-up, preserving current safety boundaries.
