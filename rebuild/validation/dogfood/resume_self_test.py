"""Typed resume failures and numeric verification chronology regressions."""
from dataclasses import replace
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import campaign
import harness as h
from codex_events import EvidenceTransportIssue, command_is_repository_inspection


class ResumeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.descriptor = h.real_session_fixture("volicord", 1, "0" * 40, self.root)
        self.path = self.root / self.descriptor["evidence"]["captures"]["resume"]["file"]
        self.capture = h.load_codex_capture(self.path)
        self.state = {"repository_revision": "0" * 40, "repository_path": "/phase8/repository",
            "project_id": "01" * 16, "work_session_id": "other"}
        self.verification = next(c for c in self.capture.commands if not command_is_repository_inspection(c.parsed_command))

    def inspect(self, capture=None):
        return campaign.inspect_resume(capture or self.capture, self.descriptor, self.state)

    def failure(self, capture, basis, domain):
        with self.assertRaises(campaign.ResumeContractError) as raised:
            self.inspect(capture)
        self.assertEqual((raised.exception.basis, raised.exception.domain), (basis, domain))
        attribution = campaign.bounded_failure_attribution("resume", domain, basis, [raised.exception.check])
        self.assertEqual(attribution["failed_checks"], [basis])

    def test_large_bounded_recall_round_trips_through_capture_parser(self):
        events = [json.loads(line) for line in self.path.read_text().splitlines()]
        def enlarge(result):
            result["snapshots"] = [{"identity": f"{i:064x}", "known_limits": ["sanitized scope" * 30]} for i in range(100)]
            result["transport_omission"] = {"omitted_count": 900, "reason": "transport_budget"}
            self.assertLess(len(json.dumps(result, separators=(",", ":")).encode()), 80 * 1024)
        for event in events:
            payload = event.get("payload", {})
            if "recall-call" not in str(payload.get("call_id", "")):
                continue
            if payload.get("type") == "custom_tool_call_output":
                result = json.loads(payload["output"][1]["text"])
                enlarge(result)
                payload["output"][1]["text"] = json.dumps(result, separators=(",", ":"))
            elif payload.get("type") == "mcp_tool_call_end":
                enlarge(payload["result"]["Ok"]["structuredContent"])
        self.path.write_text("".join(json.dumps(event) + "\n" for event in events))
        capture = h.load_codex_capture(self.path)
        self.assertEqual(len(capture.successful_calls("recall")[0].result["snapshots"]), 100)
        self.assertEqual(self.inspect(capture), "01" * 16)

    def test_inspection_after_verification_including_indeterminate_transport(self):
        for command in ("git -C /phase8/repository diff --stat", "git status --short && git diff", "bash -lc 'git diff --stat'"):
            inspection = replace(self.verification, sequence=self.verification.completion_sequence + 1,
                completion_sequence=self.verification.completion_sequence + 2,
                parsed_command={"cmd": command}, exit_code=None, termination=None, evidence_state="indeterminate")
            self.assertEqual(self.inspect(replace(self.capture, commands=(*self.capture.commands, inspection))), "01" * 16)

    def test_second_edit_requires_retest(self):
        mutation = replace(self.capture.path_observations[-1], sequence=self.verification.completion_sequence + 1)
        changed = replace(self.capture, path_observations=(*self.capture.path_observations, mutation))
        self.failure(changed, "post_change_validation_missing", "behavior_contract")
        retest = replace(self.verification, sequence=mutation.sequence + 1, completion_sequence=mutation.sequence + 2)
        self.assertEqual(self.inspect(replace(changed, commands=(*changed.commands, retest))), "01" * 16)

    def test_text_manifest_mutation_also_requires_retest(self):
        mutation = replace(self.capture.path_observations[-1], paths=("requirements.txt",),
            sequence=self.verification.completion_sequence + 1)
        changed = replace(self.capture, path_observations=(*self.capture.path_observations, mutation))
        self.assertEqual(h.resume_continuation_facts(changed, changed.successful_calls("recall")[0],
            checkpoint_work_state="paused", recalled_work_state="paused", common_identity_and_freshness_ok=True,
            change_baseline_ok=True, executable_work_scope={"paths": ["src/resume.rs", "requirements.txt"]},
            descriptor_scope_paths=[])["failure_basis"], "post_change_validation_missing")

    def test_nonzero_and_indeterminate_validation_are_distinct(self):
        for command, basis, domain in (
            (replace(self.verification, exit_code=2), "terminal_validation_failed", "product_integration"),
            (replace(self.verification, exit_code=None, termination=None, evidence_state="indeterminate"), "terminal_validation_indeterminate", "evidence"),
        ):
            self.failure(replace(self.capture, commands=tuple(command if c is self.verification else c for c in self.capture.commands)), basis, domain)

    def test_recall_transport_and_identity_are_evidence_failures(self):
        recall = self.capture.successful_calls("recall")[0]
        issue = EvidenceTransportIssue(recall.sequence, recall.turn_id, recall.call_id, "volicord", "recall", "malformed_mcp_completion")
        self.failure(replace(self.capture, evidence_transport_issues=(issue,)), "recall_transport_incomplete", "evidence")
        malformed = replace(recall, result={"project_id": "01" * 16})
        bad_identity = replace(recall, result={**recall.result, "project_id": "ab" * 16})
        for call, basis in ((malformed, "recall_transport_incomplete"), (bad_identity, "recall_identity_or_project_invalid")):
            self.failure(replace(self.capture, tool_calls=tuple(call if c is recall else c for c in self.capture.tool_calls)), basis, "evidence")

    def test_mutating_commands_cannot_be_ignored_as_inspection(self):
        for command in ("sed -i s/a/b/ src/a.rs", "find . -delete", "git diff --output=report", "git status --short && python3 scripts/edit.py"):
            self.assertFalse(command_is_repository_inspection({"cmd": command}))

    def test_textual_success_report_is_not_verification(self):
        report = replace(self.verification, parsed_command={"cmd": "echo tests passed"})
        self.failure(replace(self.capture, commands=tuple(report if c is self.verification else c for c in self.capture.commands)),
            "post_change_validation_missing", "behavior_contract")

    def test_order_baseline_and_scope_have_separate_bases(self):
        recall = self.capture.successful_calls("recall")[0]
        early = replace(self.capture.path_observations[0], sequence=recall.sequence - 1)
        self.failure(replace(self.capture, path_observations=(early, *self.capture.path_observations)),
            "pre_recall_repository_access_or_order_violation", "behavior_contract")
        checkpoint = self.capture.successful_calls("checkpoint_record")[-1]
        wrong = replace(checkpoint, arguments={**checkpoint.arguments, "baseline_analysis_snapshot_id": "ff" * 32})
        self.failure(replace(self.capture, tool_calls=tuple(wrong if c is checkpoint else c for c in self.capture.tool_calls)),
            "baseline_invalid", "behavior_contract")
        self.failure(replace(self.capture, tool_calls=tuple(c for c in self.capture.tool_calls
            if not(c.operation == "materiality_review" and c.arguments.get("action") == "inspect"))),
            "scope_or_authority_missing", "behavior_contract")

    def test_unknown_failure_basis_is_an_internal_invariant(self):
        with self.assertRaises(AssertionError):
            campaign.ResumeContractError("unmaintained")
        error = campaign.ResumeContractError("validator_invariant_failure")
        self.assertEqual(error.domain, "validation_internal")


class LearningContinuityTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.descriptor = h.real_session_fixture("volicord", 1, "0" * 40, self.root,
            behavior_class="learning_deliberation")
        captures = self.descriptor["evidence"]["captures"]
        self.work = h.load_codex_capture(self.root / captures["work"]["file"])
        self.resume = h.load_codex_capture(self.root / captures["resume"]["file"])
        self.begin = next(c for c in self.work.calls("learning_deliberation") if c.arguments["action"] == "begin")
        self.frontier = min(c.sequence for c in h.meaningful_work_path_observations(self.work))
        self.recall = self.resume.successful_calls("recall")[0]
        self.item = self.recall.result["learning_context"][0]

    def trace(self, work=None):
        return h.learning_deliberation_trace(work or self.work,
            review_id=self.begin.arguments["review_candidate_id"],
            dimension_id=self.begin.arguments["dimension_id"], first_write_sequence=self.frontier)

    def recalled(self, items, health=None, basis=None, project=None):
        result = {**self.recall.result, "learning_context": items,
            "learning_context_health": health or {"state": "available"}}
        if project is not None:
            result["project_id"] = project
        recall = replace(self.recall, result=result)
        resume = replace(self.resume, tool_calls=tuple(recall if c is self.recall else c for c in self.resume.tool_calls))
        return h.learning_recall_facts(resume, "learning_deliberation", basis if basis is not None else self.trace()[1])[0]

    def with_other_deliberation(self, same_review=False):
        calls = []
        for call in self.work.calls("learning_deliberation"):
            arguments = {**call.arguments}
            result = {**call.result, "deliberation_candidate_id": "af" * 16}
            if arguments["action"] == "begin":
                arguments.update(review_candidate_id=self.begin.arguments["review_candidate_id"] if same_review else "ae" * 16,
                    dimension_id="other-dimension")
                result.update(materiality_review_candidate_id=arguments["review_candidate_id"], dimension_id="other-dimension")
            else:
                arguments["deliberation_candidate_id"] = "af" * 16
            calls.append(replace(call, call_id="other-" + call.call_id, arguments=arguments, result=result))
        return replace(self.work, tool_calls=(*self.work.tool_calls, *calls))

    def test_relevant_trace_among_multiple_completed_deliberations(self):
        for same_review in (False, True):
            work = self.with_other_deliberation(same_review)
            valid, basis = self.trace(work)
            self.assertTrue(valid, basis)
            self.assertEqual(basis["deliberation_candidate_id"], self.item["candidate_id"])
            self.assertEqual(basis["transition_count"], 4)
            baseline = work.successful_calls("repository_analyze")[0]
            self.assertTrue(h.work_blocker_behavior_observations(work, "learning_deliberation", baseline, self.frontier)[0])

    def test_full_evaluator_carries_selected_identity_into_flat_recall(self):
        work = self.with_other_deliberation()
        historical = {**self.item, "candidate_id": "af" * 16, "materiality_review_candidate_id": "ae" * 16}
        for items, expected in (([historical, self.item], "passed"), ([historical], "failed")):
            recall = replace(self.recall, result={**self.recall.result, "learning_context": items})
            resume = replace(self.resume, tool_calls=tuple(recall if c is self.recall else c for c in self.resume.tool_calls))
            loader = h.load_codex_capture
            work_name = self.descriptor["evidence"]["captures"]["work"]["file"]
            resume_name = self.descriptor["evidence"]["captures"]["resume"]["file"]
            with patch.object(h, "load_codex_capture", side_effect=lambda path:
                work if path.name == Path(work_name).name else resume if path.name == Path(resume_name).name else loader(path)):
                result = h.real_session_evidence(self.descriptor, kind="volicord", cycle=1, repository_revision="0" * 40)
            self.assertEqual(result["checks"]["learning_deliberation_order"], "passed")
            self.assertEqual(result["checks"]["learning_recall_continuity"], expected)

    def test_duplicate_relevant_begins_and_candidate_identity_are_ambiguous(self):
        for candidate_id in (self.item["candidate_id"], "af" * 16):
            duplicate = replace(self.begin, call_id="duplicate-begin",
                result={**self.begin.result, "deliberation_candidate_id": candidate_id})
            self.assertFalse(self.trace(replace(self.work, tool_calls=(*self.work.tool_calls, duplicate)))[0])
        other = self.with_other_deliberation()
        self.assertFalse(self.trace(replace(other, tool_calls=tuple(c for c in other.tool_calls if c is not self.begin)))[0])

    def test_orphan_cross_candidate_and_wrong_project_transitions_fail(self):
        work = self.with_other_deliberation()
        response = next(c for c in work.calls("learning_deliberation") if c.arguments["action"] == "respond_select")
        for arguments, result in (
            ({**response.arguments, "deliberation_candidate_id": "af" * 16}, response.result),
            (response.arguments, {**response.result, "deliberation_candidate_id": "af" * 16}),
            ({**response.arguments, "deliberation_candidate_id": "ff" * 16}, {**response.result, "deliberation_candidate_id": "ff" * 16}),
            ({**response.arguments, "project_id": "ff" * 16}, response.result),
        ):
            changed = replace(response, arguments=arguments, result=result)
            self.assertFalse(self.trace(replace(work, tool_calls=tuple(changed if c is response else c for c in work.tool_calls)))[0])
        orphan = replace(response, call_id="orphan", arguments={**response.arguments, "deliberation_candidate_id": "ff" * 16},
            result={**response.result, "deliberation_candidate_id": "ff" * 16})
        self.assertFalse(self.trace(replace(work, tool_calls=(*work.tool_calls, orphan)))[0])

    def test_selected_state_chronology_and_non_decision_invariants(self):
        complete = next(c for c in self.work.calls("learning_deliberation") if c.arguments["action"] == "complete")
        response = next(c for c in self.work.calls("learning_deliberation") if c.arguments["action"] == "respond_select")
        for original, changed in (
            (complete, replace(complete, completion_sequence=self.frontier + 1)),
            (complete, replace(complete, result={**complete.result, "canonical_decision": True})),
            (complete, replace(complete, result={**complete.result, "state": {"state": "feedback_provided"}})),
            (response, replace(response, arguments={**response.arguments, "user_turn": "Uncaptured answer"})),
            (self.begin, replace(self.begin, result={**self.begin.result, "recommendation": "Choose now"})),
        ):
            self.assertFalse(self.trace(replace(self.work, tool_calls=tuple(changed if c is original else c for c in self.work.tool_calls)))[0])

    def test_flat_recall_selects_exact_current_candidate_among_history(self):
        historical = {**self.item, "candidate_id": "af" * 16, "materiality_review_candidate_id": "ae" * 16,
            "baseline_analysis_snapshot_id": "bf" * 32}
        self.assertTrue(self.recalled([historical, self.item]))
        self.assertTrue(self.recalled([self.item, historical], {"state": "available", "omitted_count": 8}))
        self.assertFalse(self.recalled([historical]))
        self.assertFalse(self.recalled([self.item, self.item]))
        self.assertFalse(self.recalled([self.item], basis={}))
        self.assertFalse(self.recalled([self.item], project="ff" * 16))

    def test_recall_rejects_wrong_basis_or_nonterminal_canonical_item(self):
        for field in ("candidate_id", "goal_context_id", "baseline_analysis_snapshot_id",
            "engineering_choice_discovery_candidate_id", "materiality_review_candidate_id", "dimension_id"):
            self.assertFalse(self.recalled([{**self.item, field: "unrelated"}]), field)
        for field, value in (("canonical_decision", True), ("state", {"state": "awaiting_agent_feedback"}),
            ("state", {"state": "skipped"}), ("state", None)):
            self.assertFalse(self.recalled([{**self.item, field: value}]))
        self.assertFalse(self.recalled([{"learning_deliberation": self.item}]))

    def test_required_learning_evidence_cannot_be_withheld(self):
        for health in ({"state": "unavailable"}, {"state": "degraded", "withheld_count": 1}):
            for items in ([], [self.item]):
                self.assertFalse(self.recalled(items, health))
        self.assertFalse(self.recalled([], {"state": "available", "omitted_count": 1}))

    def test_non_deliberation_controls_keep_empty_context_precision(self):
        for behavior in ("learning_routine_control", "research_or_no_question"):
            self.assertFalse(h.learning_recall_facts(self.resume, behavior, None)[0])
            empty = replace(self.recall, result={**self.recall.result, "learning_context": []})
            resume = replace(self.resume, tool_calls=tuple(empty if c is self.recall else c for c in self.resume.tool_calls))
            self.assertTrue(h.learning_recall_facts(resume, behavior, None)[0])


def check_resume_regressions():
    result = unittest.TextTestRunner().run(unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(case)
        for case in (ResumeTests, LearningContinuityTests)))
    if not result.wasSuccessful():
        raise AssertionError("resume evidence regressions failed")


if __name__ == "__main__":
    check_resume_regressions()
