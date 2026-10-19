import unittest

import server
import sketch


class SketchTests(unittest.TestCase):
    def test_original_score_fits_schema_and_sixteen_bars(self):
        context = {"channels": [{"name": name, "row": row} for row, name in enumerate(["Kick", "Clap", "Hat", "Snare"])], "patterns": [{"number": 1}]}
        operations = sketch.score(context)
        self.assertEqual(operations, sketch.score(context))
        self.assertLessEqual(len(operations), 128)
        lengths = {}
        clips = []
        notes = []
        for operation in operations:
            server.validate(operation, server.OPERATIONS)
            if operation["op"] == "create_pattern":
                lengths[operation["number"]] = operation["bars"]
            if operation["op"] == "place_pattern":
                clips.append(operation)
            if operation["op"] == "add_notes":
                for note in operation["notes"]:
                    self.assertLessEqual(note["beat"] + note["duration"], lengths[operation["pattern"]] * 4)
                    self.assertLess(note["channel"], 8)
                    notes.append(note)
        self.assertEqual(max(clip["start_bar"] + lengths[clip["pattern"]] for clip in clips), 16)
        self.assertEqual({clip["track"] for clip in clips}, set(range(5)))
        self.assertGreater(len(notes), 400)


if __name__ == "__main__":
    unittest.main()
