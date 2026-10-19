# Lemon Ray — first playable milestone

Ray is an original, small FM/pitch-envelope instrument, separate from Lemon Trio and its project-file identity.
This milestone implements the approved **Sound Canvas** direction, not a Serum clone or an exact artist patch.

## Playing it

Restart the updated Lemon Studio, open the channel rack's `+`, search for **Lemon Ray**, and add it as an instrument.
Use steps, the piano roll, or the app's typing keyboard. It routes to the rack channel's assigned mixer insert and
then through the normal mixer routing and Master effects. Start at a low listening volume.

The first version is the **Laser** voice. Bass mode, glide, wavetable editing, sample import, vocal processing,
advanced modulation and built-in delay/reverb are deliberately not presented as working features.

| Control | Meaning |
| --- | --- |
| Sweep | Initial pitch above the played note, falling exponentially to that note; 0–48 semitones. |
| Length | One-shot pitch/amplitude tail, 60–1,200 ms. Holding a key does not sustain it indefinitely. |
| Bite | Sine phase modulation at a 2:1 ratio, from soft to metallic. |
| Body | Blends in a sine one octave below the swept carrier. |
| Output | Final level, −24 to 0 dB; default −12 dB. |

Arc zap, Soft ray and Glass ping are original starting patches. The preset name is derived from the five values;
manual edits show Custom. Previous/next or clicking the preset name changes patches. Presets use the same parameter
edit path as sliders, so their values are stored and can be automated.

Drag or scroll the horizontal sliders. Double-click, middle-click or Alt-click resets a slider. Right-click offers
Reset, numeric entry and (inside Lemon Studio) Create automation clip. Clicking a slider focuses it; Tab cycles the
five controls, arrows adjust, Home/End choose the limits and Backspace resets. Numeric entry supports the parameter's
displayed units. This custom canvas is not a screen-reader accessibility implementation.

## Visuals and audio

The pitch curve calls the **same pitch law** as the voice. Its playhead follows the youngest active voice; with a
chord it is not a graph of every voice. The lower display reads actual rendered mono output, not a decorative
oscillator simulation. It auto-scales for visibility and separately reports the captured peak in dBFS; silence is
flat. Its 512 samples are a short live window, not an FFT or a complete note history.

The engine has 32 fixed voice slots, a 3 ms smooth attack, a finite curved decay and a 12 ms note-off release.
Natural ends, releases, choke and voice stealing report voice termination to the host. Standard MIDI tuning uses
A4 = 440 Hz, with per-channel ±2-semitone bend and per-note pan/tuning. MIDI CC 123 releases a channel; CC 120
chokes it. Stop/reset clears voices, bends and the scope. An oldest-voice steal at the 32-voice limit can cut a tail.

Four-times oversampling, cascaded low-pass stages and a frequency-dependent modulation budget limit bright FM
content. Five-millisecond parameter smoothing reduces abrupt macro changes; a final soft bound limits each stereo
sample. This is not a claim of alias-free Serum-quality DSP. Very high notes/sweeps are frequency-limited, and
the first release intentionally prioritizes bounded CPU, clean ordinary releases and predictable behavior.

There are no per-voice heap allocations, file operations or blocking GUI locks in Ray's render loop. Scope transfer
uses bounded atomic storage; the GUI does not read or lock the engine. The existing non-blocking LiveEdits path
lets edits reach the audio while the host confirms them.

## Hosting and saved projects

- Built-in CLAP ID: `org.lemonstudio.lemon-ray`.
- VST3 class ID bytes: `LemonStudioRay01`; do not change either identity after release.
- Parameter IDs/order: `sweep`, `length`, `bite`, `body`, `output`; keep them stable for existing automation/state.
- Ray is linked into Lemon Studio and appears without a plugin scan or an installed bundle. Standalone CLAP/VST3
  builds are also included in `bundle-plugins`. Trio, Sampler and EQ identities and storage stay unchanged.
- Unlike the legacy recreations, Ray has no FL-native state format. Its built-in instance is saved using the existing
  plugin wrapper with its CLAP state. The logical path `builtin/lemon-ray.clap` preserves the wrapper's format
  discriminator; it is not a file the host needs to open. Saving the app executable's path there would be decoded
  as VST2 by the existing wrapper reader. Reopening resolves the built-in instrument by format/name.
- The custom editor uses the shared X11/XWayland embedding and macOS window backend. Linux is the validated target;
  macOS is unverified. Windows currently exposes audio/parameters without this custom editor.
- New source files are exact downstream-only entries in the existing attribution-checker exception list. They do
  not assert upstream authorship or add invented contributor copyright headers.

## Verification and next work

Focused tests cover pitch/amplitude endpoints, standard tuning, multiple sample rates, bounded/finite output,
voice limits and identity/channel-specific release, real scope data/reset, editor-to-audio edits, keyboard/presets,
scaled painting, host note offsets, Stop, actual FLP save/reopen and re-save, and Insert/Master effect routing.
The ignored native-window test checks real XWayland parentage, embedded size and wheel edits at 1x, 1.25x, 1.5x
and 2x using an unmapped parent. It does not take over the user's project or desktop focus.

Run focused tests with `cargo test -p lemon-ray` and `cargo test -p os-app lemon_ray`.
`LEMON_RAY_BUNDLES=<bundled-dir>` enables the external CLAP/VST3 load/state/playback test; without it that test
returns without exercising bundles. `OS_SHOT_DIR=<dir>` writes editor snapshots and a short dry render.

Before expanding the instrument: audition low/high notes and repeated short notes, try the three patches in a real
project, check focus/transport and actual mixed-monitor transitions on Hyprland, and save/reopen that project.
Then refine the sound/scope readability and design Bass mode/glide as a separate milestone. Full modulation,
spectral analysis and effects come later, not as a wall of inactive controls.

## Local build/install record — October 17, 2026

- Canonical source: `/home/bry/Projects/Lemon-Studio`, based on local main `e8f4070`; Ray and the earlier plugin-search
  changes remain uncommitted/unpushed. No branch reset, push or change to the user's project files.
- Workspace/all-target build and release app build passed, locked/offline, with two build jobs.
- Rebuilt all four VST3/CLAP pairs using the existing bundle tool. Full workspace tests reported **999 passes,
  zero failures, 29 ignored**, with `LEMON_RAY_BUNDLES` explicitly set so Ray's external CLAP and VST3 test ran.
- Ray's ignored XWayland window test was separately enabled and passed on the real display; unmapped parent,
  four scales, host link, docked parentage and native wheel-to-state confirmation.
- Rendered/inspected idle and live Sound Canvas snapshots, including 1x, 1.5x and 2x. Artifact directory:
  `/home/bry/Projects/lemon-validation-2026-10-17/ray/`; logs: `/home/bry/Projects/lemon-ray-*.log`.
- Installed the new app and **only Ray's new pair**; the existing Sampler/EQ/Trio installations and launcher were
  not replaced. Installed-binary scanner probes recognize both Ray formats as instruments with the expected IDs.
- App SHA-256: `6af4672a87f3a771d3ef29a25f6ca6edc73ecf42037c0d9153919b69cdb40e76`.
- Ray CLAP and VST3 module SHA-256: `e669e984b286bbd86b0a49812182c54a1754e9824c8be3970113eb84ee9fa5d9`.
- Previous app, launcher and Cargo metadata: `/home/bry/Projects/lemonstudio-backups/2026-10-17/before-lemon-ray/`.
  Previous app SHA-256: `5f6dfe7318856e3acc20518a61ccd33e7d3d855e0dbd16f81b85062ea27f9c5f`.
  No Ray bundles existed before this install. For a full rollback, close Lemon, atomically restore the backup app
  and move the two newly installed Ray bundles out of their plugin folders into the backup directory.

The test total still includes fixture/environment-dependent early returns, including fifteen private FLP corpus
cases. It is not a claim that all ignored tests or arbitrary third-party plugins were exercised. Existing compiler
warnings remain. Listening, real-project desktop acceptance, mixed-monitor transitions and Windows/macOS validation
are not replaced by these tests. The user's app was not forcibly closed or relaunched.
