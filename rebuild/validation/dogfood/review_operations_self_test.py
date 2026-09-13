#!/usr/bin/env python3
"""Reviewer isolation, bounded packaging and append-only review operations."""
import copy
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import harness
import qualitative_review as q
import review_operations as ops


def snapshot(root):
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*") if p.is_file()}


def insufficient_draft(root):
    p, sha, _ = ops.load_package(root)
    value = q.template(p, sha)
    value["observation_scope"]["inspected_evidence"] = [s["sample_id"] + "-availability" for s in p["index"]["samples"]]
    for spec, finding in zip(q.criterion_specs(p["index"], p["rubric"]), value["assessments"]):
        finding.update(assessment="insufficient_evidence", reasoning="Only the bounded evidence availability inventory was inspected.",
            evidence=[{"evidence_id": spec["sample_id"] + "-availability", "locator": {"kind": "json_pointer", "value": "/unavailable_surfaces"}}],
            uncertainty="No substantive judgment has been established from actual observations.",
            counterevidence={"state": "not_observable", "reasoning": "Missing inspection limits both positive and contrary observations.", "evidence": []})
    (root / "draft.json").write_bytes(ops.encoded(value))
    return value


def assert_review_workflow(root, parent):
    """Also called by the maintained campaign self-test on its collected fixture."""
    before = snapshot(root)
    target = parent / "review-safe"
    result = ops.prepare(root, target, reviewer_kind="agent", session_id="review-session", run_id="a" * 32)
    assert result["state"] == "prepared"
    assert snapshot(root) == before
    p, _, package = ops.load_package(target)
    assert len(p["index"]["samples"]) == 8
    assert len([x for x in p["index"]["evidence"].values() if x["surface"] == "documents"]) == 64
    assert not any(name.startswith("private-rollouts/") for name in package["artifacts"])
    assert any(u["surface"] == "live_viewer_observation" for u in p["unavailable_surfaces"])
    assert ops.validate(target, target / "draft.json")["assessment_state"] == "not_reviewed"
    copy_target = parent / "review-safe-repeat"
    repeated = ops.prepare(root, copy_target, reviewer_kind="agent", session_id="review-session", run_id="a" * 32)
    assert repeated["package_id"] == result["package_id"]
    assert snapshot(copy_target) == snapshot(target)
    insufficient_draft(target)
    original = snapshot(target)
    with patch.object(c, "load_evidence_set", side_effect=AssertionError("preflight read campaign")), \
         patch.object(harness, "git_blob_bytes", side_effect=AssertionError("preflight read repository")), \
         patch.object(harness, "real_session_evidence", side_effect=AssertionError("semantic rerun")):
        preflight = ops.validate(target, target / "draft.json")
        assert preflight["assessment_state"] == "insufficient_evidence"
        assert preflight["mutation"] == "none"
        assert snapshot(target) == original
        recorded = ops.record(target, target / "draft.json")
    assert recorded["result"]["qualification_state"] == "not_run"
    assert recorded["result"]["phase_9_ready"] is False
    assert (target / "recorded/review.json").read_bytes() == original["draft.json"]
    fixed = snapshot(target)
    try:
        ops.record(target, target / "draft.json")
    except ValueError:
        pass
    else:
        raise AssertionError("review run overwritten")
    assert snapshot(target) == fixed
    archive = parent / "review-safe.tar.gz"
    ops.package_review(target, archive)
    with tarfile.open(archive, "r:gz") as opened:
        names = opened.getnames()
        assert not any(name.startswith(("evaluator/", "operator/", "reviewer/", "private-rollouts/")) for name in names)
        body = b"".join(opened.extractfile(m).read() for m in opened.getmembers())
        assert b"BATCH-PRIVATE-STORE" not in body and b"BATCH-PRIVATE-DERIVED" not in body
        assert "recorded/review.json" in names and "recorded/receipt.json" in names
    assert snapshot(root) == before
    return target


class WorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.parent = Path(cls.temp.name)
        binary = cls.parent / "bin/volicord"
        fixtures.write_fake_binary(binary)
        with patch.object(harness, "git_clean", return_value=True):
            cls.root, raw, bundles = fixtures.prepared_batch(cls.parent, "campaign", binary)
            c.collect_batch(cls.root, raw, exporter=fixtures.batch_exporter(bundles),
                documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)
            cls.evaluation_result = c.evaluate_campaign(cls.root)
        cls.evaluation = cls.root / cls.evaluation_result["evaluation"]

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def target(self):
        return self.parent / self._testMethodName

    def test_round_trip_isolated_deterministic_and_append_only(self):
        assert_review_workflow(self.root, self.parent)

    def test_raw_opt_in_and_machine_binding(self):
        target = self.target()
        before = snapshot(self.root)
        ops.prepare(self.root, target, reviewer_kind="agent", session_id="reviewer", evaluation_path=self.evaluation, include_raw=True)
        p, _, package = ops.load_package(target)
        self.assertEqual(p["binding"]["machine_evaluation"]["run_id"], self.evaluation_result["run_id"])
        self.assertEqual(len([n for n in package["artifacts"] if n.startswith("private-rollouts/")]), 16)
        self.assertEqual(len(p["index"]["machine_findings"]), 8 * len(harness.REAL_SESSION_CHECKS))
        self.assertEqual(snapshot(self.root), before)
        for entry in p["index"]["evidence"].values():
            if entry["surface"] == "work_capture":
                self.assertEqual((target / entry["path"]).read_bytes(), (self.root / entry["origin"]["path"]).read_bytes())

    def test_evaluator_private_answers_are_not_selected(self):
        manifest = copy.deepcopy(c.load_evidence_set(self.root))
        slot = next(iter(manifest["cycles"].values()))["review_slot_id"]
        name = f"evaluator/descriptors/{slot}.json"
        path = self.root / name
        descriptor = c.read_json(path)
        descriptor["evaluation_basis"]["private_expected_answer"] = "EVALUATOR-PRIVATE-ANSWER-SENTINEL"
        descriptor["behavior_review"]["independent_review"]["secret_instructions"] = "EVALUATOR-PRIVATE-INSTRUCTION-SENTINEL"
        data = ops.encoded(descriptor)
        manifest["artifacts"][name] = {"bytes": len(data), "sha256": ops.digest(data)}
        original = ops.bounded_read
        with patch.object(ops, "bounded_read", side_effect=lambda p, *args: data if p == path else original(p, *args)):
            files, index, unavailable = ops.select_evidence(self.root, manifest, None, include_raw=False)
        content = b"".join(files.values()) + ops.encoded(index) + ops.encoded(unavailable)
        self.assertNotIn(b"EVALUATOR-PRIVATE-ANSWER-SENTINEL", content)
        self.assertNotIn(b"EVALUATOR-PRIVATE-INSTRUCTION-SENTINEL", content)
        self.assertNotIn(b"private_expected_answer", content)
        self.assertIn(b"never instructions", ops.INSTRUCTIONS)

    def test_mismatched_and_mutated_evidence_rejected(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        p, _, _ = ops.load_package(target)
        name = next(iter(p["index"]["evidence"].values()))["path"]
        path = target / name
        path.chmod(0o600)
        path.write_bytes(path.read_bytes() + b"tamper")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            ops.validate(target, target / "draft.json")
        with self.assertRaises(ValueError):
            ops.prepare(self.root, self.parent / "wrong-run", reviewer_kind="human", evaluation_path=target / "draft.json")

    def test_invalid_preflight_is_read_only_and_frozen_kind_is_enforced(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="agent", session_id="reviewer")
        value = insufficient_draft(target)
        for edit in (lambda v: v["reviewer"].update(kind="human"),
                     lambda v: v["binding"]["evidence_set"].update(sha256="0" * 64),
                     lambda v: v["assessments"][0]["evidence"][0].update(evidence_id="absent"),
                     lambda v: v["assessments"][0]["evidence"][0].update(locator={"kind": "line", "value": 99999999})):
            invalid = copy.deepcopy(value)
            edit(invalid)
            (target / "draft.json").write_bytes(ops.encoded(invalid))
            before = snapshot(target)
            with self.assertRaises(ValueError):
                ops.validate(target, target / "draft.json")
            self.assertEqual(snapshot(target), before)
            with self.assertRaises(ValueError):
                ops.record(target, target / "draft.json")
            self.assertFalse((target / "recorded").exists())

    def test_historical_candidate_is_read_only_input(self):
        before = snapshot(self.root)
        with patch.object(harness, "git_head", return_value="0" * 40), patch.object(harness, "git_clean", return_value=False):
            result = ops.prepare(self.root, self.target(), reviewer_kind="human")
        self.assertEqual(result["state"], "prepared")
        self.assertEqual(snapshot(self.root), before)

    def test_preparation_failure_and_collision_leave_no_partial_run(self):
        target = self.target()
        with patch.object(ops.os, "rename", side_effect=OSError("injected publication failure")):
            with self.assertRaises(OSError):
                ops.prepare(self.root, target, reviewer_kind="human")
        self.assertFalse(target.exists())
        self.assertFalse(target.with_name(target.name + ".publication-lock").exists())
        ops.prepare(self.root, target, reviewer_kind="human")
        before = snapshot(target)
        with self.assertRaises(ValueError):
            ops.prepare(self.root, target, reviewer_kind="agent", session_id="reviewer")
        self.assertEqual(snapshot(target), before)

    def test_empty_review_and_inventory_bound_drafts_cannot_be_recorded(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        with self.assertRaisesRegex(ValueError, "empty draft"):
            ops.record(target, target / "draft.json")
        with self.assertRaisesRegex(ValueError, "mutable draft"):
            ops.validate(target, target / "preparation.json")

    def test_symlink_and_bounds_fail_closed(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        p, _, _ = ops.load_package(target)
        name = next(iter(p["index"]["evidence"].values()))["path"]
        path = target / name
        outside = self.parent / "outside.txt"
        outside.write_bytes(path.read_bytes())
        path.unlink()
        path.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, "escaped|symlink"):
            ops.validate(target, target / "draft.json")
        with patch.object(ops, "MAX_FILES", 1):
            with self.assertRaisesRegex(ValueError, "bounds"):
                ops.prepare(self.root, self.parent / "over-bound", reviewer_kind="human")

    def test_recording_failure_and_tampering_are_not_success(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        insufficient_draft(target)
        before = snapshot(target)
        with patch.object(ops.os, "rename", side_effect=OSError("injected recording failure")):
            with self.assertRaises(OSError):
                ops.record(target, target / "draft.json")
        self.assertEqual(snapshot(target), before)
        ops.record(target, target / "draft.json")
        path = target / "recorded/review.json"
        path.chmod(0o600)
        path.write_bytes(path.read_bytes() + b"\n")
        with self.assertRaisesRegex(ValueError, "recorded review hash"):
            ops.package_review(target, self.parent / "corrupt-review.tar.gz")

    def test_matching_evidence_set_is_required_for_machine_run(self):
        invalid = c.read_json(self.evaluation)
        invalid["evidence_set"]["sha256"] = "0" * 64
        invalid["run_id"] = ops.machine.digest({k: v for k, v in invalid.items() if k != "run_id"})
        data = ops.encoded(invalid)
        original_read, original_inventory = ops.bounded_read, c.load_inventory
        inventory = copy.deepcopy(original_inventory(self.root))
        inventory["artifacts"][self.evaluation.relative_to(self.root).as_posix()] = {"bytes": len(data), "sha256": ops.digest(data)}
        # Isolate the machine-input check without rewriting the actual campaign.
        with patch.object(ops, "bounded_read", side_effect=lambda p, *args: data if p == self.evaluation else original_read(p, *args)), \
             patch.object(c, "load_inventory", return_value=inventory), \
             patch.object(c, "load_evidence_set", return_value=c.read_json(self.root / "evidence-set.json")):
            with self.assertRaisesRegex(ValueError, "machine run evidence-set/candidate mismatch"):
                ops.prepare(self.root, self.target(), reviewer_kind="human", evaluation_path=self.evaluation)

    def test_hard_and_indeterminate_machine_cycles_are_reviewable(self):
        invalid = c.read_json(self.evaluation)
        observation = invalid["cycles"][0]["observation"]
        observation["checks"]["source_grounded_checkpoint"] = "failed"
        observation["checks"]["appropriate_inquiry_outcome"] = "partial"
        invalid["cycles"][0]["findings"] = ops.machine.from_observation(observation)
        invalid["finding_state"] = ops.machine.evaluation_state([f for cycle in invalid["cycles"] for f in cycle["findings"]])
        invalid["run_id"] = ops.machine.digest({k: v for k, v in invalid.items() if k != "run_id"})
        ops.machine.validate_run(invalid)
        files, index, _ = ops.select_evidence(self.root, c.load_evidence_set(self.root), invalid, include_raw=False)
        self.assertEqual(len(index["samples"]), 8)
        self.assertTrue(any(v["finding"]["disposition"] == "hard_blocking" for v in index["machine_findings"].values()))
        self.assertTrue(any(v["finding"]["disposition"] == "qualitative_review_required" for v in index["machine_findings"].values()))
        self.assertIn("evidence/machine-findings.json", files)

    def test_cli_preflight_and_record_share_validation(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        insufficient_draft(target)
        for operation in ("validate-qualitative-review", "record-qualitative-review"):
            result = subprocess.run(["python3", "-B", str(Path(c.__file__)), operation,
                "--review-root", str(target), "--draft", str(target / "draft.json")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertNotIn('"phase_9_ready": true', result.stdout)


def run_workflow_tests():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(WorkflowTests))
    if not result.wasSuccessful():
        raise AssertionError("qualitative review operations self-test failed")


if __name__ == "__main__":
    unittest.main()
