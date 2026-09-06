"""Sanitized transport regressions, also run by the maintained Dogfood self-test."""
from copy import deepcopy
import json
from pathlib import Path
import unittest

from codex_events import (
    EvidenceError, MAX_MCP_CONTENT_RESULT_CHARS, load_codex_capture,
    normalize_current_mcp_completion, normalize_mcp_completion,
)

HERE = Path(__file__).resolve().parent


class McpCompletionTests(unittest.TestCase):
    def setUp(self):
        self.payloads = [json.loads(line)["payload"] for line in
                         (HERE / "fixtures/legacy-content-wrapped-mcp.jsonl").read_text().splitlines()
                         if json.loads(line).get("payload", {}).get("type") == "mcp_tool_call_end"]

    def test_supported_representations_are_equivalent(self):
        for payload in self.payloads:
            wrapped = normalize_mcp_completion(payload)
            self.assertEqual(wrapped[3:], ("succeeded", None))
            envelope = json.loads(payload["result"]["Ok"]["content"][0]["text"])
            direct = deepcopy(payload)
            direct["result"]["Ok"] = envelope
            self.assertEqual(normalize_mcp_completion(direct), wrapped)
            both = deepcopy(payload)
            both["result"]["Ok"].update(isError=False, structuredContent=envelope["structuredContent"])
            self.assertEqual(normalize_mcp_completion(both), wrapped)
            current = {"type": "item_completed", "thread_id": "fixture-session", "turn_id": "fixture-turn",
                       "item": {"type": "McpToolCall", "id": payload["call_id"],
                                **payload["invocation"], "status": "completed", "result": both["result"]["Ok"]}}
            self.assertEqual(normalize_current_mcp_completion(current, "fixture-session")[1:], wrapped)

    def test_conflicting_direct_nested_and_inner_results_fail(self):
        for field, value in (("isError", True), ("structuredContent", {"conflict": True})):
            payload = deepcopy(self.payloads[0])
            payload["result"]["Ok"][field] = value
            with self.assertRaises(EvidenceError):
                normalize_mcp_completion(payload)
        payload = deepcopy(self.payloads[0])
        envelope = json.loads(payload["result"]["Ok"]["content"][0]["text"])
        envelope["content"][0]["text"] = '{"conflict":true}'
        payload["result"]["Ok"]["content"][0]["text"] = json.dumps(envelope)
        with self.assertRaises(EvidenceError):
            normalize_mcp_completion(payload)

    def test_malformed_and_arbitrary_text_fail_closed(self):
        for text in ("success", "{}", "[]", '{"isError":false',
                     json.dumps({"isError": "false", "structuredContent": {}}),
                     json.dumps({"isError": False, "structuredContent": {}, "content": [{"type": "text", "text": "{"}]}),
                     "x" * (MAX_MCP_CONTENT_RESULT_CHARS + 1)):
            payload = deepcopy(self.payloads[0])
            payload["result"]["Ok"]["content"][0]["text"] = text
            self.assertEqual(normalize_mcp_completion(payload)[4], "malformed_mcp_completion")
        for content in ([{"type": "image", "data": "ignored"}], [{"type": "text", "text": "{}"}] * 2):
            payload = deepcopy(self.payloads[0])
            payload["result"]["Ok"]["content"] = content
            self.assertEqual(normalize_mcp_completion(payload)[4], "malformed_mcp_completion")

    def test_errors_and_minimal_envelope(self):
        for error in (False, True):
            payload = deepcopy(self.payloads[0])
            payload["result"]["Ok"] = {"content": [{"type": "text", "text": json.dumps({
                "isError": error, "structuredContent": {"error": "fixture_error"},
            })}]}
            self.assertEqual(normalize_mcp_completion(payload)[3:],
                             ("failed", "fixture_error") if error else ("succeeded", None))
        payload["result"] = {"Err": "transport_error"}
        self.assertEqual(normalize_mcp_completion(payload)[3:], ("failed", "transport_error"))

    def test_capture_normalization(self):
        capture = load_codex_capture(HERE / "fixtures/legacy-content-wrapped-mcp.jsonl")
        self.assertEqual(capture.evidence_transport_issues, ())
        self.assertEqual([call.operation for call in capture.tool_calls], ["recall", "materiality_review"])
        self.assertTrue(all(call.outcome == "succeeded" for call in capture.tool_calls))


def check_capture_regressions():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(McpCompletionTests))
    if not result.wasSuccessful():
        raise AssertionError("MCP completion transport regressions failed")


if __name__ == "__main__":
    check_capture_regressions()
