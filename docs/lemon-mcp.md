# Lemon Studio local MCP — first milestone

An opt-in bridge into Lemon's existing project, history, plugin host and export engine. It does not add an LLM,
call a model API, need an API key, or listen on a network port. Normal launches do not expose a control socket.
Linux is the validated target; Windows has no bridge transport in this milestone and macOS is unverified.

## Start a scratch session

Use an absolute, private workspace with an existing parent directory. Do not enable an assistant on your only
copy of a valuable project. Example from the canonical checkout, after building the updated app:

```sh
mkdir -p /home/bry/Projects/lemon-sessions
lemonstudio --assistant-headless /home/bry/Projects/lemon-sessions/my-new-sketch
```

The root and its `assets`, `exports` and `checkpoints` subdirectories must be real directories owned by you,
without group/other access (normally mode 0700). The app creates them if missing. One session owns each workspace.
Headless mode starts a fresh scratch project with the built-in plugins and isolated settings; it does not play
live audio. Run it in its own terminal; interrupt that process when finished. An interrupted headless process
can leave `control.sock`: confirm the owning process has ended before removing that stale socket. Startup never
deletes an existing socket automatically. Graceful GUI close removes its own socket.

For an interactive window, explicitly launch:

```sh
lemonstudio --assistant /home/bry/Projects/lemon-sessions/my-gui-session
```

This keeps normal window/backend selection, including existing XWayland docking. The title shows `[Assistant]`.
Stop playback and close dialogs before editing. Writes are refused if the open project path is outside the
opted-in workspace. Closing the opted-in app disconnects it; ordinary desktop launches stay disconnected.
There is not yet an in-app connection toggle or workspace/session picker.

## MCP adapter and client connection

The stdlib-only Python adapter uses newline-delimited JSON-RPC on stdio. Standard output is protocol only.
It exposes three read tools by default; `--allow-writes` explicitly enables the five mutation tools.

```sh
python3 /home/bry/Projects/Lemon-Studio/tools/lemon-mcp/server.py \
  --workspace /home/bry/Projects/lemon-sessions/my-new-sketch
```

Example **optional** Codex configuration, not automatically installed by this feature:

```toml
[mcp_servers.lemon_studio]
command = "python3"
args = ["/home/bry/Projects/Lemon-Studio/tools/lemon-mcp/server.py", "--workspace", "/home/bry/Projects/lemon-sessions/my-gui-session"]
default_tools_approval_mode = "writes"
tool_timeout_sec = 20
```

Start the matching Lemon session separately. Add `"--allow-writes"` to `args` when intentionally authorizing
composition tools. Keep client-side approval prompts enabled for writes. Restart/reconnect the client as needed
to discover the server; merely creating these files does not register tools in an already running chat.
No global Codex configuration is changed by the app or sketch script. Configuration fields follow the
[official Codex MCP documentation](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

The adapter implements initialize/initialized, ping, tools/list, tools/call and a `lemon://session` read resource.
The exercised protocol is 2025-11-25; older version negotiation is accepted but not independently certified.
See the official MCP [lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
[stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) and
[tool format](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

| Tool | Scope |
| --- | --- |
| `lemon_get_session` | Session/revision, tempo, channels, patterns, notes, clips and status. |
| `lemon_list_plugins` | Already available instruments/effects; no scanning or installation. |
| `lemon_render_status` | Latest asynchronous preview job. |
| `lemon_apply_edit` | Tempo, new sampler/Ray channels, channel level/pan, new patterns, notes and playlist clips. |
| `lemon_history` | One ordinary Undo or Redo, including an assistant batch. |
| `lemon_transport` | Play song/pattern or Stop in an opted-in GUI; headless can Stop/select only. |
| `lemon_save_copy` | A new FLP in `exports`, without changing the open path or overwriting files. |
| `lemon_render` | New 48 kHz stereo float WAV of the arranged song; poll status until done. |

Every write requires the current `session_id`, `expected_revision` and a unique `operation_id`. Read and review
the session before writing. Reuse the same ID **and identical arguments** for a retry; successful receipts are
cached for the last 64 operations in this process, not persisted forever. On timeout, inspect the session before
trying anything else. A queued request canceled or expired before processing cannot mutate the app. A request
already executing may finish even if the client disconnects.

Channels and playlist tracks are **zero-based**. Pattern numbers are **one-based**. Notes use quarter-note beats,
not 1/16 steps; clip positions are zero-based bars. Read `ppq`/`beats_per_bar`, rather than guessing tick values.
`asset` is relative to `assets`, not an arbitrary file path. First-version channels are WAV samplers and Ray's
three original presets. Arbitrary effect insertion, plugin parameter automation, MIDI import, deleting/replacing
patterns, recording, loading projects and general filesystem/shell actions are not exposed.

## Safety and limitations

- All model mutations happen on the app's main thread. The socket thread only validates framing, queues bounded
  requests and writes replies/audit metadata; it does not edit the audio graph from a network or audio callback.
- Validate each batch on a clone, save a new checkpoint, then record it as one step in upstream's normal history.
  Undo/Redo reload hosted plugin state. This is intentionally heavier than incremental editing and is not allowed
  during playback. Existing GUI history entry points capture hosted state before a full-project snapshot swaps.
- Limits: 1 MiB requests, 128 operations/batch, 64 channels, 128 patterns, 16,384 notes, 1,024 clips; new patterns
  and arrangements up to 64 bars, 40–240 BPM, WAV assets up to 32 MiB. Adapter replies cap at 4 MiB.
- Save/render accept simple new filenames only, publish exclusively, and never intentionally replace an existing
  output. The original open path/dirty status stays intact. Keep sample assets alongside the sketch: the FLP
  references their local paths and is not a self-contained archive.
- The Unix socket is private (0600); workspace directories are private and checked for symlinks/ownership. This
  is a local same-user trust boundary, not isolation from a malicious process running as you. No TCP, OAuth,
  remote server, arbitrary shell, scan or plugin install endpoint is supplied.
- Audit JSONL records tool name, operation ID, outcome and time, not the full score. Checkpoints/audio/settings
  are workspace-local. Read metadata is untrusted data, never instructions. Keep client permissions scoped too.
- Revision fingerprints cover the project model/tempo/pattern/path, not every transient knob movement in a
  hosted editor. Current hosted state is captured before batch/history changes. Use stopped scratch projects;
  this is not a complete collaborative synchronization protocol or arbitrary-project fidelity guarantee.

## Original 16-bar example

With a **fresh headless** session running, from the canonical checkout:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/lemon-mcp/sketch.py --workspace /home/bry/Projects/lemon-sessions/my-new-sketch
```

The script drives the actual MCP stdio adapter and private app socket, not a parallel FLP writer. It creates
original procedural keys, synthetic vowel textures and rubber bass assets, adds Ray accents and the fork's
built-in drums, arranges four four-bar sections at 160 BPM, verifies batch Undo/Redo, saves a new editable FLP,
and renders a preview plus JSON report. Synthetic vowels are not a singer clone or sampled recording. No Leroy
recordings or melodies are imported; the reference is only the broad contrast Bry described.

The script refuses non-headless or already composed sessions and refuses to replace its assets/outputs. It is
a deterministic example and end-to-end smoke test, not automatic music generation from arbitrary artist prompts.
Musical taste, listening acceptance, GUI focus/monitor transitions and broader client interoperability still
require hands-on testing.

## Validation commands

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/lemon-mcp -p 'test_*.py' -v
cargo test -p os-app --bin lemonstudio assistant --locked --offline -j 2
LEMON_RAY_BUNDLES=/path/to/bundled cargo test --workspace --locked --offline -j 2
cargo build --workspace --all-targets --locked --offline -j 2
```

Targeted coverage includes atomic rejection, stale sessions/revisions, repeat receipts, ordinary Undo/Redo,
framed Ray CLAP state/preset save/reload, FLP copy round trips, overwrite/path guards, headless playback refusal,
private directory/socket checks, expired requests, actual socket dispatch, MCP lifecycle/schema/stdio handling
and bounded deterministic score construction. Tests do not establish arbitrary external-plugin compatibility.

## Local completion record — October 18, 2026

- Canonical source remains `/home/bry/Projects/Lemon-Studio`, local main `e8f4070` plus saved working-tree
  changes. No commits, branches, pushes or upstream updates were made for this milestone.
- Eight focused Rust bridge/controller tests and eight Python protocol/score tests passed. Full Linux workspace
  tests: **1,007 reported passes, zero failures, 29 ignored**, with `LEMON_RAY_BUNDLES` explicitly set. Workspace
  all-target and release app builds passed, locked/offline, two jobs. Existing warnings remain. Fifteen private
  corpus cases and other conditional tests still need their fixtures/environment; these totals are not full
  device, third-party plugin, GUI-client or cross-platform coverage.
- End-to-end MCP run created `Grave Light - 16 bars.flp` and its WAV in
  `/home/bry/Projects/lemon-sessions/grave-light-16-2026-10-18/exports/`: 160 BPM, 543 notes, 17 clips, eight
  channels. Verified ordinary Undo/Redo through the stdio adapter and actual private socket.
- Separately enabled the existing ignored `render_check::render_project` test on the saved FLP. Reopened seven
  Samplers plus Ray correctly; the entire stereo PCM matched the original render **byte-for-byte**. Preview is
  24.379 seconds including tail, 48 kHz stereo, peak −5.16 dBFS. The template's Master Fruity Limiter is not a
  hosted effect in this built-in-only test; the measured output itself remains below full scale. No listening
  or artist-similarity claim is implied. Reports and an MP3 preview remain beside the project; keep `assets/`.
- The interrupted test failure was traced to raw JSON supplied where NIH CLAP state expects an eight-byte
  little-endian length prefix. Fixed the framing and added preset/save/reload assertions. It was an invalid
  requested allocation, not evidence that this sketch exhausts the workstation's RAM.
- Installed app SHA-256: `1667e96465e7dd2dee830c9e92c7217026a3448e6b30ed046524e72e083c4ef6`.
  Backup: `/home/bry/Projects/lemonstudio-backups/2026-10-18/before-lemon-mcp/`; previous app SHA-256:
  `6af4672a87f3a771d3ef29a25f6ca6edc73ecf42037c0d9153919b69cdb40e76`. All eight installed plugin modules,
  the desktop launcher and Cargo install metadata remain unchanged. The user's running GUI/project was not
  closed or replaced. To roll back, close Lemon and atomically restore the backup `lemonstudio` binary.
- Validation logs: `/home/bry/Projects/lemon-mcp-{tests,workspace-tests,all-targets,release,sketch,reopen-render}.log`.
  Scratch headless validation was stopped afterward; no background listener is intentionally left running.
- **Next:** optionally register this adapter in the chosen MCP client, then test an explicitly opted-in GUI
  scratch session's transport/focus/history with Bry. No native tools were registered in Codex configuration
  during this implementation. Keep arbitrary plugin/effect/automation expansion separate from that acceptance.

## Authorized client connection — October 18, 2026 follow-up

Bry then authorized connecting the adapter. Added only `[mcp_servers.lemon_studio]` to
`/home/bry/.codex/config.toml`, pointing at this checkout's Python adapter with `--allow-writes` and the
`grave-light-16-2026-10-18` workspace. All eight tools are allowlisted, with `default_tools_approval_mode = "writes"`;
existing MusicCraft, Obsidian and other configuration was preserved and compared after parsing. Config backup:
`/home/bry/Projects/lemonstudio-backups/2026-10-18/before-mcp-connection/codex-config.toml`.

Opened the saved sketch in an explicitly opted-in XWayland GUI with workspace-local settings. The configured
command passed initialize/catalog/session/plugin-list checks against that real GUI: eight channels, 543 notes,
17 clips, and unchanged FLP bytes. This check issued no edits or transport commands and did not interrupt
playback. Report: `exports/mcp-connection-check.json`. The GUI is intentionally left open for Bry, not a
headless daemon. To reopen it after closing:

```sh
OPENSTUDIO_USER_DIR=/home/bry/Projects/lemon-sessions/grave-light-16-2026-10-18/gui-settings \
  /home/bry/.cargo/bin/lemonstudio \
  --assistant /home/bry/Projects/lemon-sessions/grave-light-16-2026-10-18 \
  '/home/bry/Projects/lemon-sessions/grave-light-16-2026-10-18/exports/Grave Light - 16 bars.flp'
```

**Still required:** refresh/restart the `lemon_studio` server in Codex's MCP settings so a chat discovers the new
native tools. The current chat's tool inventory did not hot-load them during setup; configuration plus a direct
protocol check is not evidence of native tool invocation. Do not restart Codex or close Bry's app automatically.
Approved GUI edits/history and focus/editor acceptance remain to be tested after that refresh.

Bry likes the sketch but says it does not capture the intended Status Update Music sound. Treat it as a first
pass from broad descriptors, not reference analysis. Obtain a short user-provided reference passage and identify
the largest timbral/rhythmic mismatch before composing a second version; leave the original intact.
