# October 28 integration candidate and usability work

Candidate: `/home/bry/Projects/Lemon-Studio-safety-2026-10-28`, detached at fork
`6ee0bbf2d7340d6cef3f9fa4cf2354b23188eef0`. It combines upstream main
`db6b14393306655fa34fedb27cdefe145d17e9f2` with the fixes below. The upstream merge remains uncommitted,
with conflicts resolved. The two `linux-windows` changes through
`3aad4dabbd2db690f8bb1163b19fe4f881b7d4f6` were ported and adapted separately, not made a second merge parent.
No branch promotion, push or installation was performed. The canonical checkout and installed app are unchanged.

## Safety and compatibility

- Original `LMD1` and empty/default Lemon Delay slots use the preserved legacy DSP. Modern Fruity Delay 3 and modern
  states retain upstream's decoder. State/dispatch regressions and impulse fingerprints at two rates cover this.
- A scoped sampler export guard counts overlapping exports, including unwind/drop. One completed export cannot
  disable another export's disk-reader wait mode. The strict disk/RAM export comparison remains in place.
- Experimental spectral splitting runs as one background job, with progress/cancel UI. New jobs wait for loading,
  exporting and recording to finish. Cancellation stays pending until cleanup completes. A changed project, path,
  tempo or source rejects the result instead of overwriting subsequent edits.
- Decode limits are 32 MiB of encoded input and 1,000,000 source frames (about 20.8 seconds at 48 kHz). The complete
  source is processed, not only its playlist trim: export a short source file first. Only the requested 2/4/6/8 output
  buffers are allocated. This is a conservative short-source backend, not whole-song AI separation.
- Every run creates a unique directory and exclusively creates its outputs. All WAVs are flushed/synced before any
  project insertion. Failure/cancel/stale completion cleans up only that job's files. Existing audio is never truncated.
  Undo includes appended channels, playlist state and original muting; output files remain available for Redo.
- New clips preserve position, length, trim, raw fields and channel routing/settings. Fixed frequency, transient and
  Mid/Side rules still determine the layers. Reconstruction is not evidence of clean vocal isolation or processed-mix
  parity after nonlinear channel effects.

Regressions cover collisions, second-write failure, complete channel Undo/Redo, saved/reopened channels, source/project
changes, cancellation, exhausted playlist tracks, bounded decoding and the original delay states.

## Upstream work retained

System-device mixer inputs, input monitoring/routing and dialogs; automation range/LFO playback and editing; Filter,
Flanger and Phaser with factory presets; Chorus/Stereo Shaper corrections; native knob reset; mixer-fader wheel;
scrollbar-end zoom; and older FLP clip record sizes. Fork instruments, MCP, creative editing tools and plugin-picker
favorites remain present. The missing External input reply handler was restored during semantic review. Automation
regressions verify the modeled LFO and its saved round trip, following upstream's new header parsing.

## Hyprland

The default remains XWayland with docked plugin editors. `OPENSTUDIO_WAYLAND=1` remains an opt-in to native Wayland
with detached editors. Host scaling and the fork's browser-edge/window constraints remain intact.

Detached frames center in the current work area and advertise dialog type. Release-time focus and remembered internal
channel-window position follow `linux-windows`. Hyprland border/shadow/animation updates target a specific frame, rather
than all XWayland windows in the process. On Hyprland frames use `lemonstudio-frame-<XID>` as their WM_CLASS; the main
window's class is unchanged, and the instance name/icon remain Lemon's. Custom detached-window class rules may need
that pattern. No compositor configuration was edited.

IPC runs off the UI thread, with bounded connection/read/write waits and response size. Drag events are coalesced;
unavailable IPC falls back to X11 movement. Fake-socket tests cover slow/oversized replies and exact frame selection,
including exclusion of an XWayland main window and another process.

## Browser

Samples, presets and folders have persistent inline stars. Plugin stars share existing plugin-picker favorites. The
Favorites tab supports search and expanding folders. Starring does not audition or drag a sample. Labels reserve room
for the star, including during marquee scrolling. Favorites use `browser_favorites_v1` in the existing settings store.

Selecting audio shows an asynchronous waveform above file details, with duration, sample rate and click-to-preview.
Replay starts at the beginning; scrubbing and playback-position display are not implemented. A single worker coalesces
selections and cancels stale work. Eight small summaries are cached by file size/modification time. PCM WAV/AIFF files
are scanned in bounded chunks across the whole file. Compressed sources use the bounded decoder and display an
unavailable message if too large. Painting never decodes audio.

## Live clip gain and styling

The user's recording `/home/bry/Videos/Recordings/recording_20261028_222932.mp4` shows the bottom-center playlist clip
gain handle. The UI already marked gain edits dirty during a drag; continuing playback copied voices with their old
gain. A DC-audio regression reproduced that failure before the fix.

Sounding clips now retain their clip identity even at unity gain. Continuing an engine refreshes gain targets for the
corresponding internal/hosted voices, preserving sample position, age, fades and late-start declick. A short ramp avoids
an abrupt gain discontinuity. Other overlapping clips keep their levels. The regression verifies both sampler paths,
the overlapping-clip isolation, no retrigger, and restoration after Undo's project update.

The handle has a theme-aware pill/grip, restrained guide line and larger dB badge. Tiny clips omit the badge instead of
clamping against an invalid width. UI regressions retain one history step per drag and check narrow-clip painting.

## Validation and evidence

Evidence: `/home/bry/Projects/lemon-safety-2026-10-28`.
Build/bundle cache: `/home/bry/Projects/lemon-review-2026-10-28/target`.

The final workspace test run reports **1,829 passed, zero failures, 50 ignored**, including the live-gain and narrow-clip
regressions. The all-target Linux build passes. Eight Python MCP tests passed. All 22 products were rebuilt into 44
CLAP/VST3 bundles, without installation, for the final candidate. Each loads/enumerates in an isolated child with unique
IDs per format. CLAP/VST3 host tests pass with the bundles present.

Two normally ignored XWayland tests passed using unmapped test parents: sampler scale/page retention and Ray docking,
scaling and editing. Browser tests cover favorite persistence, shared plugin favorites, search, newest selection,
changed-file invalidation and star hit testing. Offscreen captures use generated fixture audio and play no sound.

All five original v4 delays now load. An 8.05-second built-in-only render of a project copy completed with Ray and legacy
effects. No audio device opened. Original v4 SHA-256:
`123652133bf9109f5669911b13a3da107045a4ab0333bad76970b1ef9bd216c2`.
Installed executable SHA-256:
`0ff116393e4b0f50ee22007efb5305312ef9d9cb49ee9c47b161d4904943c47b`.

The final v4 short render is byte-identical before/after the live-gain change, SHA-256
`d44e4fa8975c40a77b3e22b3b225b653375e5a9cd263525ac612b3f4e3b01c66`. This comparison covers that excerpt with
no interactive gain edits. Offscreen screenshots: `browser-sample-preview.png`, `browser-favorites.png` and
`clip-gain-drag.png` in the evidence directory.

The first offline run lacked other-platform dependency downloads needed by notices; fetching the locked dependencies
resolved it. Obsolete Favorites-disabled/raw automation-header assertions were updated to implemented behavior and the
upstream LFO model. No audio comparisons were weakened. The live-gain regression failed before the gain fix and passed
after it.

Ignored/conditional FL reference/corpus tests, physical input devices, mixed-monitor detached movement, listening
acceptance and Windows/macOS builds remain separate gates. A short render is not full-song parity or OOM endurance
testing. No AI separation model was downloaded or evaluated.

## Recovery and next step

Executable: `/home/bry/Projects/lemon-review-2026-10-28/target/debug/lemonstudio`.
Use isolated settings, disabled startup plugin scanning and a project copy for an authorized preview. A persistent GUI
must use an independent user service, rather than a short-lived shell's background process.

Safety-only backup: stash `bde6b39396d8be1246477089485d2ed256e38b14`, plus the external patch/new-file archive in the
evidence directory. Do not reapply it over the completed candidate. The October 22 candidate's unfinished merge and
installed artifacts remain preserved. Review the combined staged candidate before a separately authorized
commit/promotion/push/installation. Future real stem separation should reuse the job/transaction pipeline with an
independently evaluated model backend and process resource limits.
