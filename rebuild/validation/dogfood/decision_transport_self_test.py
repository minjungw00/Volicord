"""Current-host response provenance keeps raw identities and a narrow comparison."""
from copy import deepcopy
from dataclasses import replace
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

    def fixture(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        descriptor = h.real_session_fixture("volicord", 1, "0" * 40, root)
        capture = h.load_codex_capture(root / descriptor["evidence"]["captures"]["work"]["file"])
        bundle = h.load_canonical_bundle(root / descriptor["evidence"]["canonical_bundle"]["file"])
        return capture, bundle

    def async_response_fixture(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        descriptor = h.real_session_fixture("volicord", 1, "0" * 40, root)
        path = root / descriptor["evidence"]["captures"]["work"]["file"]
        events = [json.loads(line) for line in path.read_text().splitlines()]
        task = next(e["payload"]["turn_id"] for e in events
                    if e.get("payload", {}).get("type") == "task_started")
        joined = []
        for event in events:
            payload = event.get("payload", {})
            if payload.get("type") == "task_complete" and payload.get("turn_id") == task:
                continue
            if payload.get("type") == "task_started" and payload.get("turn_id") != task:
                continue
            if "turn_id" in payload:
                payload["turn_id"] = task
            joined.append(event)
        path.write_text("".join(json.dumps(e) + "\n" for e in joined))
        return h.load_codex_capture(path), h.load_canonical_bundle(
            root / descriptor["evidence"]["canonical_bundle"]["file"])

    def test_async_reply_in_same_task_has_exact_current_schema_authority(self):
        capture, bundle = self.async_response_fixture()
        decision = capture.successful_calls("decision_record")[0]
        self.assertEqual(len(capture.user_turns), 2)
        self.assertEqual(len({t.turn_id for t in capture.user_turns}), 1)
        self.assertEqual(len({t.user_turn_id for t in capture.user_turns}), 2)
        latest = capture.user_turns[-1]
        self.assertEqual(capture.turn_for_call(decision), latest)
        facts = h.decision_facts(capture, bundle)
        self.assertTrue(facts[0])
        transport = facts[-1][facts[1]]["current_host_response_transport"]
        self.assertEqual(transport["captured_user_turn_id"], latest.user_turn_id)
        self.assertEqual(transport["captured_turn_sequence"], latest.sequence)
        self.assertEqual(transport["raw_capture_sha256"], capture.source_sha256)

    def test_async_reply_cannot_reuse_old_future_cross_task_or_ambiguous_authority(self):
        capture, bundle = self.async_response_fixture()
        decision = capture.successful_calls("decision_record")[0]
        latest = capture.user_turns[-1]
        for turns in (
            capture.user_turns[:-1],
            (*capture.user_turns[:-1], replace(latest, sequence=decision.sequence + 1)),
            (*capture.user_turns[:-1], replace(latest, turn_id="other-task")),
            (*capture.user_turns, replace(latest, user_turn_id="ambiguous-client")),
            (*capture.user_turns, replace(latest, sequence=decision.sequence - 1,
                user_turn_id="newer-client", text="A new response replaces the earlier answer")),
        ):
            self.assertFalse(h.decision_facts(replace(capture, user_turns=turns), bundle)[0])
        for table in ("sources", "question_response_sources", "decisions",
                      "question_decision_history_witnesses", "question_revisions"):
            tables = deepcopy(bundle.tables)
            tables[table] = ()
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])
        for table, field, value in (
            ("question_response_sources", "question_revision", 99),
            ("question_decision_history_witnesses", "root_decision_id", "f" * 32),
            ("question_decision_history_witnesses", "response_source_id", "f" * 32),
            ("question_decision_history_witnesses", "response_authority", "agent"),
            ("decisions", "user_authority", "agent"),
            ("question_revisions", "material_scope", "malformed"),
        ):
            tables = deepcopy(bundle.tables)
            for row in tables[table]:
                row[field] = value
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])

    def test_internal_host_session_is_not_raw_codex_session(self):
        capture, bundle = self.fixture()
        self.assertTrue(h.decision_facts(capture, bundle)[0])
        tables = deepcopy(bundle.tables)
        for source in tables["sources"]:
            if source["source_kind"] == "current_host_user_turn":
                source["detail_two"] = "independent-host-adapter-session"
        changed = replace(bundle, tables=tables)
        self.assertTrue(h.decision_facts(capture, changed)[0])
        goal = capture.successful_calls("context_record")[0]
        self.assertTrue(h.goal_facts(capture, changed, capture.user_turns[0].text,
            goal.result["context_item_id"])[0])

    def test_exact_decision_links_reject_substitution(self):
        capture, bundle = self.fixture()
        decision = capture.successful_calls("decision_record")[0]
        for field, value in (("question_revision", 99), ("question_id", "ff" * 16),
            ("presentation_receipt_id", "unrelated-receipt"),
            ("user_turn", "The agent recommends stable behavior."),
            ("user_turn", "Keep concise output, please.")):
            with self.subTest(field=field, value=value):
                altered = replace(decision, arguments={**decision.arguments, field: value})
                changed = replace(capture, tool_calls=tuple(altered if c is decision else c for c in capture.tool_calls))
                self.assertFalse(h.decision_facts(changed, bundle)[0])
        for field, value in (("actor_kind", "agent"), ("detail_one", "other-host"),
            ("project_id", "ff" * 16), ("source_kind", "file"),
            ("locator", "unrelated user turn")):
            tables = deepcopy(bundle.tables)
            source = next(s for s in tables["sources"] if s["id"] == decision.result["user_response_source_id"])
            source[field] = value
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])
        for altered in (
            replace(decision, turn_id=capture.user_turns[0].turn_id),
            replace(decision, result={**decision.result, "user_response_source_id": "ff" * 16}),
        ):
            changed = replace(capture, tool_calls=tuple(altered if c is decision else c for c in capture.tool_calls))
            self.assertFalse(h.decision_facts(changed, bundle)[0])
        response = capture.turn_for_call(decision)
        changed = replace(capture, user_turns=tuple(replace(t, text="A different response") if t is response else t
            for t in capture.user_turns))
        self.assertFalse(h.decision_facts(changed, bundle)[0])
        self.assertFalse(h.decision_facts(replace(capture, tool_calls=(*capture.tool_calls, decision)), bundle)[0])
        for table in ("question_response_sources", "question_decision_history_witnesses", "decisions"):
            tables = deepcopy(bundle.tables)
            tables[table] = (*tables[table], *tables[table])
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])

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
