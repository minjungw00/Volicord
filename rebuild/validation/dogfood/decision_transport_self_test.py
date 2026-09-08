"""Current-host response provenance keeps raw identities and a narrow comparison."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import harness as h


class DecisionTransportTests(unittest.TestCase):
    def test_directional_terminal_transport_only(self):
        caller = "Choose stable_order; keep errors explicit."
        for raw in (caller, caller + "\r\n", caller + " \t\r\n", caller.replace("_", "\\_") + " \n"):
            comparison = h.compare_current_host_response_transport(caller, raw)
            self.assertTrue(comparison["equivalent"])
            self.assertEqual(comparison["raw_host_text_sha256"], hashlib.sha256(raw.encode()).hexdigest())
            self.assertEqual(comparison["caller_text_sha256"], hashlib.sha256(caller.encode()).hexdigest())
        for raw in (caller.lower(), caller.replace("explicit", "implicit"), caller.replace(";", ","),
            caller.replace("keep errors", "keep  errors"), " " + caller, caller + "\u00a0", caller + "\nAlso delete errors"):
            self.assertFalse(h.compare_current_host_response_transport(caller, raw)["equivalent"])
        self.assertFalse(h.compare_current_host_response_transport(caller + " ", caller)["equivalent"])
        self.assertFalse(h.compare_current_host_response_transport(r"Choose stable\_order", "Choose stable_order")["equivalent"])
        self.assertFalse(h.compare_frozen_task_transport(caller, caller + " ").equivalent)

    def test_actual_capture_and_canonical_source_linkage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for suffix, semantic_change, expected in ((" \t\r\n", False, True), ("", True, False)):
                descriptor = h.real_session_fixture("volicord", 1, "0" * 40, root)
                path = root / descriptor["evidence"]["captures"]["work"]["file"]
                capture = h.load_codex_capture(path)
                decision = capture.successful_calls("decision_record")[0]
                caller = decision.arguments["user_turn"]
                raw = caller.replace("concise", "verbose") if semantic_change else caller + suffix
                events = [json.loads(line) for line in path.read_text().splitlines()]
                changed = 0
                for event in events:
                    payload = event.get("payload", {})
                    if payload.get("type") == "user_message" and payload.get("message") == caller:
                        payload["message"] = raw
                        changed += 1
                self.assertEqual(changed, 1)
                path.write_text("".join(json.dumps(event) + "\n" for event in events))
                capture = h.load_codex_capture(path)
                bundle = h.load_canonical_bundle(root / descriptor["evidence"]["canonical_bundle"]["file"])
                facts = h.decision_facts(capture, bundle)
                self.assertEqual(facts[0], expected)
                checkpoint = h.terminal_checkpoint_call(capture)
                baseline = h.selected_checkpoint_baseline_call(capture, checkpoint)
                observations = h.work_blocker_behavior_observations(capture, "explicit_user_owned_decision", baseline,
                    min(x.sequence for x in h.meaningful_work_path_observations(capture)))
                self.assertEqual(observations[2], expected)
                if expected:
                    evidence = next(iter(facts[-1].values()))["current_host_response_transport"]
                    self.assertEqual(evidence["raw_capture_sha256"], h.sha256(path))
                    self.assertEqual(evidence["raw_host_text_sha256"], hashlib.sha256(raw.encode()).hexdigest())
                    self.assertEqual(evidence["canonical_source_text_sha256"], hashlib.sha256(caller.encode()).hexdigest())
                    self.assertGreater(evidence["terminal_ascii_whitespace_removed_count"], 0)


def check_decision_transport_regressions():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(DecisionTransportTests))
    if not result.wasSuccessful():
        raise AssertionError("Decision transport regressions failed")


if __name__ == "__main__":
    check_decision_transport_regressions()
