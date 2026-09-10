"""Sanitized transport regressions, also run by the maintained Dogfood self-test."""
from copy import deepcopy
import json
from pathlib import Path
import unittest
import tempfile
from dataclasses import replace

from codex_events import (
    EvidenceError, MAX_MCP_CONTENT_RESULT_CHARS, load_codex_capture,
    normalize_current_mcp_completion, normalize_mcp_completion,
    parse_custom_call,
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


class CurrentExecutionTests(unittest.TestCase):
    def capture(self, body):
        events = [json.loads(line) for line in
                  (HERE / "fixtures/current-codex-execution-evidence.jsonl").read_text().splitlines()]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "capture.jsonl"
            path.write_text("".join(json.dumps(e) + "\n" for e in [*events[:3], *body]))
            return load_codex_capture(path)

    def command(self, source, parts, *, raw_output=None):
        metadata = {"turn_id": "sanitized-execution-turn"}
        return self.capture([
            {"type": "response_item", "payload": {"type": "custom_tool_call", "name": "exec",
             "status": "completed", "call_id": "test", "input": source,
             "internal_chat_message_metadata_passthrough": metadata}},
            {"type": "response_item", "payload": {"type": "custom_tool_call_output", "call_id": "test",
             "output": raw_output if raw_output is not None else [{"type": "input_text", "text": p} for p in
                        ["Script completed\nWall time 0.1 seconds\nOutput:\n", *parts]],
             "internal_chat_message_metadata_passthrough": metadata}},
        ])

    def test_optional_file_change_streams(self):
        item = {"type": "FileChange", "id": "file", "status": "completed",
                "changes": {"src/lib.rs": {"type": "update", "unified_diff": "@@\n-old\n+new\n", "move_path": None}}}
        for streams in ({}, {"stdout": ""}, {"stderr": ""}, {"stdout": "", "stderr": ""}):
            with self.subTest(streams=streams):
                capture = self.capture([{"type": "event_msg", "payload": {"type": "item_completed",
                    "thread_id": "sanitized-execution-session", "turn_id": "sanitized-execution-turn",
                    "item": {**item, **streams}}}])
                self.assertEqual([p.paths for p in capture.path_observations], [("src/lib.rs",)])
                self.assertFalse(capture.evidence_transport_issues)
        for mutation in ({"stdout": None}, {"stderr": 0}, {"changes": {}}, {"changes": {"src/lib.rs": {"type": "update"}}}):
            capture = self.capture([{"type": "event_msg", "payload": {"type": "item_completed",
                "thread_id": "sanitized-execution-session", "turn_id": "sanitized-execution-turn",
                "item": {**item, **mutation}}}])
            self.assertFalse(capture.path_observations)
            self.assertEqual(capture.evidence_transport_issues[0].reason, "malformed_file_change")

    def test_current_exit_templates_and_literal_workdir(self):
        for marker in ("exit=", "EXIT:", "EXIT ", "exit:", "\\nEXIT:"):
            with self.subTest(marker=marker):
                capture = self.command('const wd="/phase8/repository"; const r=await tools.exec_command({cmd:"rg --files",workdir:wd});'
                    + 'text(r.output); text(`' + marker + '${r.exit_code}`);', ["src/lib.rs", marker.replace("\\n", "\n") + "0"])
                self.assertEqual(len(capture.commands), 1)
                self.assertEqual(capture.commands[0].exit_code, 0)
                self.assertEqual(capture.commands[0].parsed_command["workdir"], "/phase8/repository")

    def test_destructured_promise_and_uncorrelated_completion(self):
        prefix = 'const [a,b]=await Promise.all([tools.exec_command({cmd:"rg --files"}),tools.exec_command({cmd:"git diff --check"})]);'
        capture = self.command(prefix + 'text(JSON.stringify({a,b}));',
            [json.dumps({"a": {"output": "src/lib.rs", "exit_code": 0}, "b": {"output": "", "exit_code": 2}})])
        self.assertEqual([c.exit_code for c in capture.commands], [0, 2])
        capture = self.command(prefix + 'text("files:"+a.output+"diff:"+b.output);', ["files:src/lib.rsdiff:"])
        self.assertEqual([c.evidence_state for c in capture.commands], ["indeterminate"] * 2)
        capture = self.command('const r=await tools.exec_command({cmd:"rg --files"});text(r.output);text(`exit=${r.exit_code}`);', ["src/lib.rs", "exit=undefined"])
        self.assertEqual(len(capture.commands), 1)
        self.assertIsNone(capture.commands[0].exit_code)

    def test_codex_issue_passes_blocker_validator(self):
        import harness as h
        from codex_events import EvidenceTransportIssue
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            head = h.git_head(h.ROOT)
            descriptor = h.real_session_fixture("volicord", 1, head, root, behavior_class="research_or_no_question")
            capture = load_codex_capture(root / descriptor["evidence"]["captures"]["work"]["file"])
            capture = replace(capture, path_observations=(), evidence_transport_issues=(
                EvidenceTransportIssue(10, capture.user_turns[0].turn_id, "file", "codex", None, "malformed_file_change"),))
            result = h.build_work_blocker_result(head, descriptor, "0" * 64, capture)
            self.assertEqual(result["classification"], "evidence_transport_failure")
            h.validate_blocker_result(result)

    def test_labeled_results_and_malformed_completion(self):
        prefix = 'const [a,b]=await Promise.all([tools.exec_command({cmd:"rg --files"}),tools.exec_command({cmd:"git diff --check"})]);'
        capture = self.command(prefix + 'text("files:"+JSON.stringify(a));text("diff:"+JSON.stringify(b));',
            ['files:{"output":"src/lib.rs","exit_code":0}', 'diff:{"output":"","exit_code":1}'])
        self.assertEqual([c.exit_code for c in capture.commands], [0, 1])
        for value in ('{"output":0,"exit_code":0}', '{"output":"","exit_code":true}', 'broken'):
            capture = self.command('const r=await tools.exec_command({cmd:"rg --files"});text(r);', [value])
            self.assertIsNone(capture.commands[0].exit_code)
            self.assertEqual(capture.evidence_transport_issues[0].reason, "malformed_exec_completion")

    def test_yielded_and_failed_cells_are_not_malformed_command_results(self):
        source = 'const r=await tools.exec_command({cmd:"rg --files"});text(r);'
        for status in ("running with cell ID 40", "failed"):
            for as_parts in (False, True):
                output = f"Script {status}\nWall time 31.0 seconds\nOutput:\nexit=0"
                if as_parts:
                    output = [{"type": "input_text", "text": output}]
                capture = self.command(source, [], raw_output=output)
                self.assertEqual(len(capture.commands), 1)
                self.assertIsNone(capture.commands[0].exit_code)
                self.assertEqual(capture.commands[0].evidence_state, "indeterminate")
                self.assertEqual(capture.evidence_transport_issues[0].reason, "command_completion_indeterminate")

    def test_observed_repository_output_does_not_invent_numeric_exit(self):
        import harness as h
        from codex_events import bounded_repository_observation, command_role
        output = "src/main.rs:12:struct Tags { output_json: bool }\nsrc/tags.rs:32:fn generate_tags(source: &str) -> Tags {"
        command = "rg -n 'Tags|generate_tags' src/main.rs src/tags.rs | head -120"
        capture = self.command('const r=await tools.exec_command(' + json.dumps({"cmd": command}) + ');text(r.output);', [output])
        observed = capture.commands[0]
        self.assertIsNone(observed.exit_code)
        self.assertEqual(observed.evidence_state, "indeterminate")
        self.assertTrue(bounded_repository_observation(observed, capture.cwd))
        evidence = h.repository_investigation_evidence(capture, after_sequence=0, before_sequence=1000)
        self.assertEqual(evidence["state"], "complete")
        validation = replace(observed, parsed_command={"cmd": "cargo test"}, output="test result: ok. 100 passed")
        self.assertFalse(h.meaningful_resume_validation(replace(capture, commands=(validation,)), 0)["qualified"])
        for cmd in ("echo '" + output + "'", "cat /tmp/report", "cat ../report", "cat rebuild/.local/report",
            "rg -n Tags /tmp/report", "rg -n Tags src --pre script", "rg -n Tags src | python3 script.py"):
            self.assertFalse(bounded_repository_observation(replace(observed, parsed_command={"cmd": cmd}), capture.cwd), cmd)
        for text in ("", "I inspected the repository.", "other/file.rs:12:struct Tags { output_json: bool }\nother/file.rs:32:fn generate_tags(source: &str) -> Tags {"):
            self.assertFalse(bounded_repository_observation(replace(observed, output=text), capture.cwd))
        for cmd in ("cat src/main.rs", "sed -n '1,20p' src/main.rs", "nl -ba src/main.rs"):
            self.assertTrue(bounded_repository_observation(replace(observed, parsed_command={"cmd": cmd},
                output="struct Tags { output_json: bool, errors: Vec<String> }\nfn generate_tags(source: &str) -> Tags {"), capture.cwd))
        for cmd in ("cargo test || true", "cargo test; true", "cargo test | cat", "cargo test\necho success",
            "sed -n '1,20p' src/a.rs -e 'w changed'", "find . -delete", "fd -x touch output", "sort -o output input"):
            self.assertEqual(command_role({"cmd": cmd}), "unknown", cmd)
        for cmd in ("cargo test", "env XDG_CACHE_HOME=/tmp/cache cargo test --lib", "python3 -m unittest",
            "rebuild/scripts/validate focused checks -- cargo clippy", "bash -lc 'cargo test && cargo clippy'"):
            self.assertEqual(command_role({"cmd": cmd}), "validation", cmd)
        for cmd in ("nl -ba src/main.rs | sed -n '1,20p;40,50p'", "nl -ba src/a.rs | sed -n '1,20p'\nnl -ba src/b.rs | sed -n '1,20p'\n"):
            self.assertEqual(command_role({"cmd": cmd}), "inspection", cmd)

    def test_dynamic_calls_remain_unsupported(self):
        for source in ('const wd=load("wd");const r=await tools.exec_command({cmd:"rg --files",workdir:wd});text(r);',
                       'const r=await tools.exec_command({cmd:makeCommand()});text(r);',
                       'const [a,b]=await Promise.all(commands.map(c=>tools.exec_command(c)));text(JSON.stringify({a,b}));'):
            self.assertIsNone(parse_custom_call(source))


def check_capture_regressions():
    suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(cls)
                               for cls in (McpCompletionTests, CurrentExecutionTests))
    result = unittest.TextTestRunner().run(suite)
    if not result.wasSuccessful():
        raise AssertionError("MCP completion transport regressions failed")


if __name__ == "__main__":
    check_capture_regressions()
