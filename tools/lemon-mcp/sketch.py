import argparse
from array import array
import json
import math
from pathlib import Path
import random
import selectors
import struct
import subprocess
import sys
import time
import uuid
import wave


class Client:
    def __init__(self, workspace):
        self.process = subprocess.Popen([sys.executable, str(Path(__file__).with_name("server.py")), "--workspace", str(workspace), "--allow-writes"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr, text=True, bufsize=1)
        self.counter = 0
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "lemon-sketch-smoke", "version": "0.1.0"}})
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        self.process.stdin.flush()

    def rpc(self, method, params):
        self.counter += 1
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.counter, "method": method, "params": params}) + "\n")
        self.process.stdin.flush()
        if not self.selector.select(20):
            raise RuntimeError("MCP response timed out; inspect session before retrying")
        response = json.loads(self.process.stdout.readline())
        if response.get("id") != self.counter or "error" in response:
            raise RuntimeError(response)
        return response["result"]

    def tool(self, name, arguments=None):
        result = self.rpc("tools/call", {"name": name, "arguments": arguments or {}})
        if result.get("isError"):
            raise RuntimeError(result["content"])
        return result["structuredContent"]

    def write(self, name, arguments):
        context = self.tool("lemon_get_session")
        return self.tool(name, {"session_id": context["session_id"], "expected_revision": context["revision"], "operation_id": str(uuid.uuid4()), **arguments})

    def close(self):
        self.process.stdin.close()
        self.process.wait(timeout=5)
        self.selector.close()


def asset_file(path, frames, rate=48000):
    peak = max(abs(value) for frame in frames for value in frame)
    gain = 0.65 / max(peak, 0.001)
    samples = array("h", (round(max(-1, min(1, value * gain)) * 32767) for frame in frames for value in frame))
    if sys.byteorder != "little":
        samples.byteswap()
    with path.open("xb") as destination:
        with wave.open(destination, "wb") as output:
            output.setnchannels(2)
            output.setsampwidth(2)
            output.setframerate(rate)
            output.writeframes(samples.tobytes())


def make_assets(directory):
    rate = 48000
    frequency = 261.625565
    generator = random.Random(1629)
    keys = []
    for frame in range(rate * 3):
        seconds = frame / rate
        attack = min(1, seconds / 0.006)
        harmonics = [(1, 0.62, 2.5), (2.001, 0.24, 3.8), (3.004, 0.10, 5.5), (4.008, 0.045, 7.5), (5.012, 0.02, 9)]
        left = sum(amplitude * math.sin(math.tau * frequency * ratio * seconds) * math.exp(-decay * seconds) for ratio, amplitude, decay in harmonics)
        right = sum(amplitude * math.sin(math.tau * frequency * ratio * 1.0004 * seconds) * math.exp(-decay * seconds) for ratio, amplitude, decay in harmonics)
        keys.append([left * attack, right * attack])
    dry = [frame[:] for frame in keys]
    for delay, amount in [(0.071, 0.14), (0.113, 0.10), (0.181, 0.07), (0.293, 0.04)]:
        offset = int(delay * rate)
        for frame in range(offset, len(keys)):
            keys[frame][0] += dry[frame - offset][1] * amount
            keys[frame][1] += dry[frame - offset][0] * amount
    asset_file(directory / "soft-keys.wav", keys)
    vowels = []
    weights = []
    for harmonic in range(1, 25):
        hz = harmonic * frequency
        first = sum(weight * math.exp(-((hz - center) / width) ** 2) for center, width, weight in [(750, 180, 1), (1150, 220, 0.6), (2500, 380, 0.35)])
        second = sum(weight * math.exp(-((hz - center) / width) ** 2) for center, width, weight in [(450, 140, 1), (800, 200, 0.6), (2500, 380, 0.2)])
        weights.append((harmonic, first / harmonic, second / harmonic))
    for frame in range(int(rate * 0.8)):
        seconds = frame / rate
        morph = (1 - math.cos(math.pi * min(seconds / 0.5, 1))) / 2
        envelope = min(1, seconds / 0.012) * math.exp(-seconds * 5) * min(1, (0.8 - seconds) / 0.04)
        value = sum((first * (1 - morph) + second * morph) * math.sin(math.tau * frequency * harmonic * seconds + 0.012 * math.sin(math.tau * 5 * seconds)) for harmonic, first, second in weights)
        breath = generator.uniform(-0.008, 0.008)
        vowels.append([(value + breath) * envelope, (value - breath) * envelope])
    asset_file(directory / "synthetic-vowels.wav", vowels)
    bass = []
    for frame in range(rate * 2):
        seconds = frame / rate
        phase = math.tau * frequency * seconds
        envelope = min(1, seconds / 0.005) * min(1, (2 - seconds) / 0.04) * math.exp(-seconds * 0.35)
        fundamental = math.sin(phase)
        grit = math.tanh(2.8 * (fundamental + 0.2 * math.sin(3 * phase) + 0.08 * math.sin(5 * phase)))
        value = (0.75 * fundamental + 0.25 * grit) * envelope
        bass.append([value, value])
    asset_file(directory / "rubber-bass.wav", bass)


def score(context):
    rows = len(context["channels"])
    by_name = {channel["name"].lower(): channel["row"] for channel in context["channels"]}
    kick, clap, hat, snare = (by_name[name] for name in ["kick", "clap", "hat", "snare"])
    keys, vowels, bass, ray = range(rows, rows + 4)
    operations = [{"op": "set_tempo", "bpm": 160}]
    for channel, volume, pan in [(kick, 0.62, 0), (clap, 0.23, 0.08), (hat, 0.16, -0.1), (snare, 0.38, 0)]:
        operations.append({"op": "set_channel_level", "channel": channel, "volume": volume, "pan": pan})
    for name, asset, volume, pan in [("Soft keys", "soft-keys.wav", 0.45, -0.12), ("Synthetic vowels", "synthetic-vowels.wav", 0.23, 0.14), ("Rubber bass", "rubber-bass.wav", 0.5, 0)]:
        operations.append({"op": "add_channel", "name": name, "instrument": "sampler", "asset": asset, "volume": volume, "pan": pan})
    operations.append({"op": "add_channel", "name": "Ray accents", "instrument": "ray", "preset": "Soft ray", "volume": 0.65, "pan": 0.12})
    voicings = [[55, 58, 62, 65], [51, 55, 58, 62], [46, 53, 58, 62], [53, 57, 60, 67]]
    roots = [31, 27, 34, 29]
    names = ["Haze", "Fracture", "Impact", "Afterglow"]
    pattern = max(item["number"] for item in context["patterns"]) + 1
    generator = random.Random(16016)
    for section in range(4):
        notes = {"Drums": [], "Keys": [], "Vowels": [], "Bass": [], "Ray": []}

        def add(part, channel, key, beat, duration=0.18, velocity=85):
            notes[part].append({"channel": channel, "key": key, "beat": beat, "duration": min(duration, 16 - beat), "velocity": velocity})

        for local_bar in range(4):
            bar = section * 4 + local_bar
            start = local_bar * 4
            chord = voicings[(bar // 2) % 4]
            for step in range(8):
                key = chord[[0, 2, 1, 3, 2, 1, 3, 2][step]]
                if section == 3 and step in (3, 7):
                    key += 12
                add("Keys", keys, key, start + step * 0.5, 0.72 if step == 0 else 0.36, 68 + generator.randrange(16))
            if bar >= 2:
                kicks = [0, 2.5] if section == 0 else [0, 0.75, 2.25, 2.75]
                if bar % 2 and section > 0:
                    kicks += [3.5]
                for beat in kicks:
                    add("Drums", kick, 60, start + beat, 0.2, 103 if beat == 0 else 88)
                for beat in [1, 3]:
                    add("Drums", snare, 60, start + beat, 0.18, 97)
                    if section > 0:
                        add("Drums", clap, 60, start + beat + 1 / 48, 0.18, 65)
            hat_step = 1 if section == 0 else 0.25
            for step in range(round(4 / hat_step)):
                add("Drums", hat, 60, start + step * hat_step, 0.08, (53 if step % 2 == 0 else 35) + generator.randrange(12))
            if local_bar == 3 and section > 0:
                for step, beat in enumerate([3.5, 3 + 2 / 3, 3 + 5 / 6]):
                    add("Drums", snare, 60, start + beat, 0.1, 40 + step * 12)
            if section >= 1:
                for beat, offset in [(0.25, 12), (0.75, 10), (2.5, 7)]:
                    add("Vowels", vowels, 60 + offset - (bar % 2) * 2, start + beat, 0.2, 76)
                if section == 3 and local_bar % 2:
                    for step in range(3):
                        add("Vowels", vowels, 67 + step, start + 3.25 + step * 0.25, 0.12, 63)
            if section >= 2:
                root = roots[(bar // 2) % 4]
                for beat, length, offset in [(0, 0.6, 0), (0.875, 0.35, 0), (1.75, 0.45, 12), (2.5, 0.6, 0), (3.375, 0.35, 7)]:
                    add("Bass", bass, root + offset, start + beat, length, 95 if offset == 0 else 82)
            if local_bar == 3 or (section >= 2 and local_bar == 0):
                add("Ray", ray, [76, 79, 72, 82][section], start + (0 if local_bar == 0 else 3), 0.85, 105)
        for track, part in enumerate(["Drums", "Keys", "Vowels", "Bass", "Ray"]):
            if not notes[part]:
                continue
            operations += [
                {"op": "create_pattern", "number": pattern, "name": f"{part} - {names[section]}", "bars": 4},
                {"op": "add_notes", "pattern": pattern, "notes": sorted(notes[part], key=lambda note: note["beat"])},
                {"op": "place_pattern", "pattern": pattern, "start_bar": section * 4, "track": track},
            ]
            pattern += 1
    return operations


def inspect_audio(path):
    data = path.read_bytes()
    cursor = 12
    audio = None
    rate = None
    channels = None
    while cursor + 8 <= len(data):
        tag, length = struct.unpack_from("<4sI", data, cursor)
        chunk = data[cursor + 8:cursor + 8 + length]
        if tag == b"fmt ":
            format_id, channels, rate = struct.unpack_from("<HHI", chunk)
            if format_id != 3:
                raise RuntimeError("Expected floating-point render")
        if tag == b"data":
            audio = array("f")
            audio.frombytes(chunk)
            if sys.byteorder != "little":
                audio.byteswap()
            break
        cursor += 8 + length + length % 2
    if audio is None or not all(math.isfinite(value) for value in audio):
        raise RuntimeError("Missing or non-finite audio")
    peak = max(abs(value) for value in audio)
    if not 0.01 < peak <= 1.00001:
        raise RuntimeError(f"Unexpected peak: {peak}")
    return {"seconds": len(audio) / channels / rate, "rate": rate, "channels": channels, "peak": peak, "peak_dbfs": 20 * math.log10(peak), "rms": math.sqrt(sum(value * value for value in audio) / len(audio))}


def main():
    parser = argparse.ArgumentParser(description="Create an original editable 16-bar sketch through the actual MCP stdio adapter")
    parser.add_argument("--workspace", required=True, type=Path)
    options = parser.parse_args()
    client = Client(options.workspace)
    try:
        catalog = client.rpc("tools/list", {})
        if len(catalog["tools"]) != 8:
            raise RuntimeError("Expected the enabled first-version tool catalog")
        context = client.tool("lemon_get_session")
        if not context["headless"] or context["clips"] or any(pattern["notes"] for pattern in context["patterns"]):
            raise RuntimeError("Use a fresh headless scratch workspace; refusing to touch an existing composition")
        make_assets(options.workspace / "assets")
        composed = client.write("lemon_apply_edit", {"operations": score(context)})
        musical = lambda session: {"bpm": session["bpm"], "patterns": session["patterns"], "clips": session["clips"], "channels": session["channels"]}
        undone = client.write("lemon_history", {"action": "undo"})
        if undone["clips"]:
            raise RuntimeError("Undo did not restore the empty arrangement")
        redone = client.write("lemon_history", {"action": "redo"})
        if musical(composed) != musical(redone):
            raise RuntimeError("Redo did not restore the musical data")
        client.write("lemon_transport", {"action": "stop", "pattern": 2})
        saved = client.write("lemon_save_copy", {"filename": "Grave Light - 16 bars.flp"})
        client.write("lemon_render", {"filename": "Grave Light - 16 bars.wav"})
        deadline = time.monotonic() + 180
        while True:
            status = client.tool("lemon_render_status")
            if status["state"] == "done":
                break
            if status["state"] == "error" or time.monotonic() > deadline:
                raise RuntimeError(status)
            time.sleep(0.2)
        report = {"title": "Grave Light", "bars": 16, "bpm": 160, "key": "G minor", "sections": ["Haze: bars 1-4", "Fracture: bars 5-8", "Impact: bars 9-12", "Afterglow: bars 13-16"], "flp": saved["path"], "wav": status["path"], "notes": sum(len(pattern["notes"]) for pattern in redone["patterns"]), "clips": len(redone["clips"]), "mcp_tools": [item["name"] for item in catalog["tools"]], "undo_redo_verified": True, "audio": inspect_audio(Path(status["path"])), "sources": "Original procedural samples, composed MIDI and built-in Ray. Synthetic vowels are not recorded or cloned vocals. No Leroy recordings sampled."}
        with (options.workspace / "exports" / "sketch-report.json").open("x") as output:
            json.dump(report, output, indent=2)
        print(json.dumps(report, indent=2))
    finally:
        client.close()


if __name__ == "__main__":
    main()
