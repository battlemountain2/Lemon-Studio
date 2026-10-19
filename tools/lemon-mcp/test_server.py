import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import server


class ProtocolTests(unittest.TestCase):
    def ready(self, writes=False):
        instance = server.Server(Path("/not-used"), writes)
        initialized = instance.dispatch({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}})
        self.assertEqual(initialized["result"]["protocolVersion"], "2025-11-25")
        self.assertIsNone(instance.dispatch({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        return instance

    def test_read_only_default_and_explicit_write_catalog(self):
        readonly = self.ready()
        writable = self.ready(True)
        self.assertEqual(len(readonly.tools), 3)
        self.assertEqual(len(writable.tools), 8)
        result = readonly.dispatch({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "lemon_apply_edit", "arguments": {}}})
        self.assertEqual(result["error"]["code"], -32602)

    def test_lifecycle_notifications_and_unknown_methods(self):
        instance = server.Server(Path("/not-used"))
        self.assertIn("error", instance.dispatch({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        self.assertIsNone(instance.dispatch({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 1}}))
        instance = self.ready()
        self.assertEqual(instance.dispatch({"jsonrpc": "2.0", "id": 3, "method": "unknown"})["error"]["code"], -32601)

    def test_invalid_initialization_can_be_corrected_and_tool_name_is_validated(self):
        instance = server.Server(Path("/not-used"))
        result = instance.dispatch({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
        self.assertEqual(result["error"]["code"], -32602)
        self.assertFalse(instance.initialized)
        instance = self.ready()
        result = instance.dispatch({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": []}})
        self.assertEqual(result["error"]["code"], -32602)

    def test_schema_rejects_bool_nonfinite_extra_fields_and_bad_notes(self):
        for value in [True, float("nan"), float("inf"), 10 ** 400, -1, 300]:
            with self.assertRaises(ValueError):
                server.validate(value, server.number(0, 127, True))
        instance = self.ready(True)
        arguments = {"session_id": "s", "expected_revision": "r", "operation_id": "o", "operations": [{"op": "add_notes", "pattern": 1, "notes": [{"channel": 0, "key": 140, "beat": 0, "duration": 1, "velocity": 90}]}]}
        result = instance.dispatch({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "lemon_apply_edit", "arguments": arguments}})
        self.assertTrue(result["result"]["isError"])
        with self.assertRaises(ValueError):
            server.validate({"unexpected": 1}, server.object_schema({}))

    def test_tool_output_resource_and_errors_are_mcp_content(self):
        instance = self.ready()
        with patch("server.bridge_call", return_value={"session_id": "scratch", "revision": "abc"}):
            result = instance.dispatch({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "lemon_get_session"}})
            self.assertFalse(result["result"]["isError"])
            self.assertEqual(result["result"]["structuredContent"]["revision"], "abc")
            resource = instance.dispatch({"jsonrpc": "2.0", "id": 3, "method": "resources/read", "params": {"uri": "lemon://session"}})
            self.assertEqual(json.loads(resource["result"]["contents"][0]["text"])["session_id"], "scratch")
        with patch("server.bridge_call", side_effect=OSError("Not connected")):
            result = instance.dispatch({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "lemon_get_session"}})
            self.assertTrue(result["result"]["isError"])

    def test_stdio_contains_only_json_rpc_and_parse_errors(self):
        messages = b'{bad json}\n{"jsonrpc":"2.0","id":1,"method":"ping"}\n'
        result = subprocess.run([sys.executable, str(Path(server.__file__)), "--workspace", "/not-used"], input=messages, capture_output=True, check=True)
        lines = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual(lines[0]["error"]["code"], -32700)
        self.assertEqual(lines[1]["result"], {})

    def test_socket_permissions_and_symlinks_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "control.sock").write_text("not a socket")
            with self.assertRaises(ValueError):
                server.bridge_call(workspace, "lemon_get_session", {})
            os.chmod(workspace, 0o755)
            with self.assertRaises(ValueError):
                server.bridge_call(workspace, "lemon_get_session", {})


if __name__ == "__main__":
    unittest.main()
