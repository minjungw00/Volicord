"""Raw-derived interaction facts; synthetic fixtures never qualify replacement."""
import copy
from pathlib import Path
import tempfile
import unittest

import harness as h
import interaction_diagnostics as diagnostics
import machine_findings as machine


class InteractionDiagnosticTests(unittest.TestCase):
    def observed(self, path, behavior, intent, label="A"):
        descriptor = h.real_session_fixture("volicord", label, "ab" * 20, path,
            materiality_obligations=behavior)
        descriptor["workload_intent"] = intent
        descriptor["learning_collaboration_statement"] = descriptor["work_user_task"] if intent == "learning_collaborative" else None
        work = h.load_codex_capture(path / f"volicord-{label}-work-events.jsonl")
        resume = h.load_codex_capture(path / f"volicord-{label}-resume-events.jsonl")
        bundle = h.load_canonical_bundle(path / f"volicord-{label}-context.bundle.json")
        return descriptor, work, resume, bundle

    def test_learning_intent_reports_actual_participation_and_resume(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.observed(Path(directory), "learning_deliberation", "learning_collaborative")
            result = diagnostics.work_summary(*args)
            self.assertGreater(result["counts"]["user_turn_count"], 2)
            self.assertGreater(result["counts"]["learning_deliberation_call_count"], 0)
            self.assertGreater(result["counts"]["current_host_response_count"], 0)
            self.assertEqual(result["counts"]["fresh_resume_count"], 1)
            self.assertGreater(result["counts"]["recall_call_count"], 0)
            self.assertEqual(result["counts"]["canonical_decision_count"], 0)
            self.assertTrue(result["frozen_learning_request_present"])
            self.assertTrue(any(p["state"] == "active" and p["verbatim_statement_in_first_task"]
                for s in result["sessions"] for p in s["learning_participation_observations"]))

    def test_question_activity_is_factual_even_in_routine_work(self):
        with tempfile.TemporaryDirectory() as directory:
            result = diagnostics.work_summary(*self.observed(Path(directory), "explicit_user_owned_decision", "routine_bounded"))
            self.assertEqual(result["counts"]["question_candidate_count"], 1)
            self.assertEqual(result["counts"]["promoted_question_count"], 1)
            self.assertEqual(result["counts"]["canonical_decision_count"], 1)
            self.assertGreater(result["counts"]["current_host_response_count"], 0)
            self.assertNotIn("status", result)

    def test_zero_questions_and_zero_learning_never_derive_a_hard_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            for label, intent in (("B", "decision_rich"), ("C", "routine_bounded")):
                path = Path(directory) / label
                path.mkdir()
                args = self.observed(path, "research_or_no_question", intent, label)
                result = diagnostics.work_summary(*args)
                self.assertEqual(result["counts"]["promoted_question_count"], 0)
                self.assertEqual(result["counts"]["learning_deliberation_call_count"], 0)
                before = copy.deepcopy(result)
                # Diagnostics are outside the finite finding inventory, with no count rules.
                findings = machine.from_observation({"checks": {}, "interaction_diagnostics": result})
                self.assertEqual(findings, [])
                self.assertEqual(result, before)
                descriptor = args[0]
                fields = {"kind", "producer", "journey_id", "repository_class", "work_slot_id",
                    "work_label", "repository_revision", "work_user_task", "fresh_resume_user_task",
                    "evidence", "workload_intent", "learning_collaboration_statement", "_evidence_directory"}
                natural = {key: value for key, value in descriptor.items() if key in fields}
                natural["contract"] = "naturalistic-observation-1"
                # Synthetic unit transport for the current closed descriptor;
                # this test never supplies measured campaign evidence.
                natural["evidence_purpose"] = "naturalistic"
                natural["fresh_resume_user_task"] = None
                natural["evidence"] = copy.deepcopy(natural["evidence"])
                natural["evidence"]["captures"].pop("resume", None)
                observation = h.real_session_evidence(natural, kind="volicord", cycle=label, repository_revision="ab" * 20)
                self.assertFalse(any(f["disposition"] == "hard_blocking" for f in machine.from_observation(observation)))

                summary = diagnostics.campaign_summary({label: result})
                self.assertEqual(summary["semantic_verdict"], "not_derived_from_counts")

    def test_missing_capture_is_unknown_and_aggregation_preserves_gap(self):
        result = diagnostics.work_summary({"workload_intent": "decision_rich"}, None, None, None)
        self.assertTrue(all(value is None for value in result["counts"].values()))
        self.assertTrue(all(value is None for value in diagnostics.campaign_summary({"B": result})["counts"].values()))


if __name__ == "__main__":
    unittest.main()
