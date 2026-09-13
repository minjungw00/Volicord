"""Certainty/policy and immutable evaluation regressions; synthetic evidence only."""
from copy import deepcopy
from argparse import Namespace
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import harness
import machine_findings as m


class MachineFindingTests(unittest.TestCase):
    def test_status_and_disposition_are_independent(self):
        for status in m.Status:
            value = m.finding("meaningful_recalled_continuation", status, {"sequence": 42})
            m.validate_finding(value)
            self.assertEqual(value["status"], status)
        violation = m.finding("required_validation_execution", m.Status.VIOLATION, {"exit_code": 1})
        unknown = m.finding("meaningful_recalled_continuation", m.Status.INDETERMINATE, {"exit_code": None})
        absent = m.finding("meaningful_recalled_continuation", m.Status.NOT_OBSERVED, {"commands": []})
        self.assertEqual(violation["disposition"], m.Disposition.HARD)
        self.assertEqual(unknown["disposition"], m.Disposition.REVIEW)
        self.assertEqual(absent["disposition"], m.Disposition.REVIEW)
        self.assertNotEqual(unknown["status"], absent["status"])
        for rule in m.INTEGRITY_RULES:
            for status in (m.Status.VIOLATION, m.Status.INDETERMINATE, m.Status.NOT_OBSERVED):
                self.assertEqual(m.disposition(rule, status), m.Disposition.HARD)
        with self.assertRaises(ValueError):
            m.disposition("unregistered", m.Status.PASS)

    def test_finite_authority_audit_preserves_procedural_signals(self):
        m.validate_policy()
        for rule in ("naturalistic_prompt_integrity", "hidden_material_discovery_order", "meaningful_ordinary_changes",
            "repository_bound_project_resolution", "recall_precedes_inspection_and_continuation", "static_viewer_snapshot",
            "generated_document_outputs", "learning_deliberation_order", "decision_provenance_when_required"):
            self.assertEqual(m.disposition(rule, "confirmed_violation"), m.Disposition.REVIEW)
            self.assertTrue(m.POLICY["rules"][rule]["owner"])
            self.assertTrue(m.POLICY["rules"][rule]["rationale"])
        for rule in ("unnecessary_question_repetition", "procedure_invocation_counts", "learning_interruption_precision"):
            self.assertEqual(m.disposition(rule, "confirmed_violation"), m.Disposition.ADVISORY)
        observed = {"checks": {"static_viewer_snapshot": "failed", "generated_document_outputs": "failed"},
            "capture_sha256": {"work": "a" * 64}}
        before = deepcopy(observed)
        findings = m.from_observation(observed)
        self.assertEqual(observed, before)
        self.assertTrue(all(f["disposition"] == m.Disposition.REVIEW for f in findings))
        self.assertNotIn("human_observed", str(findings))

    def test_impossible_combinations_rejected(self):
        valid = m.finding("raw_hash", m.Status.VIOLATION, {"actual": "different"})
        for key, value in (("disposition", "qualitative_review_required"), ("status", "passed"),
                           ("policy_version", 99), ("basis", {})):
            invalid = {**valid, key: value}
            with self.assertRaises(ValueError):
                m.validate_finding(invalid)
        with self.assertRaises(ValueError):
            m.finding("raw_hash", m.Status.NOT_APPLICABLE, {"claim": "waived"})

    def test_underlying_indeterminate_and_absent_are_never_violation_or_pass(self):
        value = {"checks": {"meaningful_recalled_continuation": "failed"},
            "capture_sha256": {"work": "a" * 64},
            "continuation_basis": {"failure_basis": "terminal_validation_indeterminate",
                "terminal_validation": {"exit_code": None}}}
        before = deepcopy(value)
        findings = m.from_observation(value)
        self.assertEqual(findings[0]["status"], "indeterminate")
        self.assertEqual(m.evaluation_state(findings), "review_required")
        self.assertEqual(value, before)
        del value["capture_sha256"]
        self.assertEqual(m.from_observation(value)[0]["status"], "not_observed")
        value["capture_sha256"] = {"work": "a" * 64}
        value["continuation_basis"]["failure_basis"] = "terminal_validation_failed"
        self.assertEqual(m.from_observation(value)[0]["status"], "confirmed_violation")

    def test_append_only_evaluation_reads_exact_evidence_set(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(harness, "git_clean", return_value=True):
            parent = Path(directory)
            binary = parent / "bin/volicord"
            fixtures.write_fake_binary(binary)
            root, captures, bundles = fixtures.prepared_batch(parent, "evaluation", binary)
            c.collect_batch(root, captures, exporter=fixtures.batch_exporter(bundles),
                documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)
            evidence = c.load_evidence_set(root)
            frozen = {name: (root / name).read_bytes() for name in evidence["artifacts"]}
            identity = (root / "evidence-set.json").read_bytes()
            first = c.evaluate_campaign(root)
            first_bytes = (root / first["evaluation"]).read_bytes()
            result = c.read_json(root / first["evaluation"])
            m.validate_run(result)
            self.assertEqual(len(result["cycles"]), 8)
            self.assertEqual(result["evidence_set"]["sha256"], harness.sha256(root / "evidence-set.json"))
            before_campaign = {p: p.read_bytes() for p in root.rglob("*") if p.is_file()}
            with patch.object(harness, "git_head", return_value="c" * 40):
                second = c.evaluate_campaign(root, previous=Path(first["evaluation"]))
            self.assertEqual({p: p.read_bytes() for p in before_campaign}, before_campaign)
            later = c.read_json(Path(second["evaluation"]))
            self.assertEqual(later["candidate_head"], result["candidate_head"])
            self.assertNotEqual(later["candidate_head"], later["evaluator_revision"])
            self.assertEqual(later["previous_evaluation"]["run_id"], first["run_id"])
            with self.assertRaises(ValueError):
                c.evaluate_campaign(root, output=Path(second["evaluation"]).parent)
            self.assertNotEqual(first["run_id"], second["run_id"])
            diagnostic_before = {p: p.read_bytes() for p in root.rglob("*") if p.is_file()}
            diagnostic = c.diagnose_campaign(root, parent / "diagnostic")
            self.assertEqual({p: p.read_bytes() for p in diagnostic_before}, diagnostic_before)
            self.assertEqual(diagnostic["candidate_head"], result["candidate_head"])
            self.assertFalse(diagnostic["phase_9_ready"])
            with self.assertRaises(ValueError):
                c.diagnose_campaign(root, parent / "diagnostic")
            self.assertEqual((root / first["evaluation"]).read_bytes(), first_bytes)
            self.assertEqual((root / "evidence-set.json").read_bytes(), identity)
            self.assertEqual({name: (root / name).read_bytes() for name in frozen}, frozen)
            self.assertIsNone(c.load_campaign(root)["terminal_outcome"])
            self.assertEqual(c.load_campaign(root)["qualification_state"], "not_run")
            from evaluation_runs import load
            with patch.object(harness, "real_session_evidence", side_effect=AssertionError("semantic evaluation reran")):
                self.assertEqual(load(Path(first["evaluation"])), result)
            outside = parent / "unregistered-evaluation.json"
            outside.write_bytes(first_bytes)
            with self.assertRaises(ValueError):
                load(outside)
            invalid = deepcopy(result)
            invalid["qualification_state"] = "passed"
            invalid["run_id"] = m.digest({k: v for k, v in invalid.items() if k != "run_id"})
            with self.assertRaises(ValueError):
                m.validate_run(invalid)
            # Even updating the mutable inventory cannot legitimize changed frozen bytes.
            name = next(name for name in frozen if name.endswith("work.rollout.jsonl"))
            (root / name).write_bytes(frozen[name] + b"\n")
            c.register_artifact(root, root / name, replace=True)
            with self.assertRaises(c.CampaignError):
                c.evaluate_campaign(root)
            self.assertEqual(len(list((root.parent / (root.name + "-evaluations")).glob("*/evaluation.json"))), 2)


if __name__ == "__main__":
    unittest.main()
