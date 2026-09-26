"""Campaign-owned steward staging regressions, with deterministic fixtures."""
import copy
import io
from contextlib import redirect_stderr
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import harness


class ReconciliationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.root = self.parent / "campaign"
        self.binary = self.parent / "candidate/bin/volicord"
        fixtures.write_fake_binary(self.binary)
        self.clean = patch.object(harness, "git_clean", return_value=True)
        self.clean.start()
        self.addCleanup(self.clean.stop)

    def prepare(self, *, missing_dimension=False, fact_disagreement=False):
        fixtures.prepare(self.root, self.parent / "sources", self.binary)
        for kind, work, _obligations in fixtures.TEST_ASSIGNMENTS:
            descriptor, *_ = fixtures.fixture_for(self.parent / f"fixture-{kind}-{work}",
                kind, work, campaign_root=self.root)
            if kind == "volicord" and work == "C":
                independent = descriptor["behavior_review"]["independent_review"]
                if missing_dimension:
                    independent["provisional_review"]["assessments"].pop()
                if fact_disagreement:
                    dimension = independent["provisional_review"]["assessments"][0]
                    dimension["classification"] = "repository_or_environment_fact"
                    comparison = independent["classification_comparison"]
                    comparison["obligation_coverage"][0]["status"] = "resolved_from_evidence"
                    comparison["provisional_classification"] = harness.blind_dimensions.classifications(independent["provisional_review"])
                    comparison["disagreements"] = ["classification"]
                    comparison["status"] = "resolved_from_evidence"
            fixtures.record_descriptor_review(self.root, kind, work, descriptor)
        c.reveal_qualification_profile(self.root, c.load_campaign(self.root)["candidate_head"])

    def paths(self):
        return c.reconciliation_paths(self.root, c.work_state(self.root, "volicord", "C"))

    def test_private_mutable_draft_validated_exact_bytes_create_only_seal(self):
        self.prepare()
        fixed = c.reviewer_provisional_path(self.root, "volicord", "C")
        original = fixed.read_bytes()
        result = c.prepare_reconciliation(self.root, "volicord", "C")
        draft, receipt = self.paths()
        self.assertTrue(draft.is_relative_to(self.root / "evaluator/reconciliation"))
        self.assertEqual(result["visibility"], "steward_private")
        self.assertNotIn(c.relative(self.root, draft), c.load_inventory(self.root)["artifacts"])
        with self.assertRaisesRegex(c.CampaignError, "validated reconciliation"):
            c.seal_work(self.root, "volicord", "C")
        value = c.read_json(draft)
        value["behavior_review"]["independent_review"]["basis"] += " STEWARD_PRIVATE_RECONCILIATION_SENTINEL"
        c.write_json(draft, value)
        c.validate_reconciliation(self.root, "volicord", "C")
        self.assertEqual(c.inspect_reconciliation(self.root, "volicord", "C")["state"], "validated")
        draft.write_bytes(draft.read_bytes() + b"\n")
        self.assertEqual(c.inspect_reconciliation(self.root, "volicord", "C")["state"], "stale_validation")
        with self.assertRaisesRegex(c.CampaignError, "validated reconciliation"):
            c.seal_work(self.root, "volicord", "C")
        c.validate_reconciliation(self.root, "volicord", "C")
        c.seal_work(self.root, "volicord", "C")
        sealed = c.evaluator_descriptor_path(self.root, "volicord", "C")
        sealed_bytes = sealed.read_bytes()
        self.assertEqual(fixed.read_bytes(), original)
        self.assertIn(c.relative(self.root, receipt), c.load_inventory(self.root)["artifacts"])
        self.assertNotIn(c.relative(self.root, draft), c.load_inventory(self.root)["artifacts"])
        with self.assertRaises(c.CampaignError):
            c.seal_work(self.root, "volicord", "C")
        self.assertEqual(sealed.read_bytes(), sealed_bytes)
        for plane in ("operator", "reviewer"):
            for path in (self.root / plane).rglob("*.json"):
                self.assertNotIn(b"STEWARD_PRIVATE_RECONCILIATION_SENTINEL", path.read_bytes())
        self.assertNotIn("STEWARD_PRIVATE_RECONCILIATION_SENTINEL", (self.root / "operator/RUN-SHEET.md").read_text())
        draft.unlink()
        self.assertEqual(c.inspect_reconciliation(self.root, "volicord", "C")["state"], "sealed")
        c.verify_inventory(self.root)

    def test_external_or_symlinked_descriptor_and_forged_validation_rejected(self):
        self.prepare()
        c.prepare_reconciliation(self.root, "volicord", "C")
        draft, receipt = self.paths()
        external = self.parent / "external.json"
        external.write_bytes(draft.read_bytes())
        with self.assertRaises(TypeError):
            c.seal_work(self.root, "volicord", "C", external)
        with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            c.parser().parse_args(["seal-work", "--campaign-root", str(self.root),
                "--repository-class", "volicord", "--work", "C", "--descriptor", str(external)])
        original = draft.read_bytes()
        draft.unlink()
        draft.symlink_to(external)
        with self.assertRaisesRegex(c.CampaignError, "private root"):
            c.validate_reconciliation(self.root, "volicord", "C")
        draft.unlink()
        draft.write_bytes(original)
        c.write_json(receipt, {"kind": "phase8_reconciliation_validation", "status": "passed"})
        with self.assertRaisesRegex(c.CampaignError, "validated reconciliation"):
            c.seal_work(self.root, "volicord", "C")

    def test_unseen_revealed_obligation_blocks_validation_and_sealing(self):
        self.prepare(missing_dimension=True)
        fixed = c.reviewer_provisional_path(self.root, "volicord", "C")
        original = fixed.read_bytes()
        c.prepare_reconciliation(self.root, "volicord", "C")
        with self.assertRaisesRegex(c.CampaignError, "blind_coverage_gap"):
            c.validate_reconciliation(self.root, "volicord", "C")
        with self.assertRaisesRegex(c.CampaignError, "blind_coverage_gap"):
            c.seal_work(self.root, "volicord", "C")
        self.assertEqual(fixed.read_bytes(), original)
        self.assertFalse(self.paths()[1].exists())

    def test_evidence_resolves_fixed_dimension_without_rewriting_provisional(self):
        self.prepare(fact_disagreement=True)
        fixed = c.reviewer_provisional_path(self.root, "volicord", "C")
        original = fixed.read_bytes()
        c.prepare_reconciliation(self.root, "volicord", "C")
        c.validate_reconciliation(self.root, "volicord", "C")
        c.seal_work(self.root, "volicord", "C")
        self.assertEqual(fixed.read_bytes(), original)

    def test_pre_reveal_preparation_and_reprepare_refused(self):
        fixtures.prepare(self.root, self.parent / "sources", self.binary)
        with self.assertRaises(c.CampaignError):
            c.prepare_reconciliation(self.root, "volicord", "C")
        self.assertFalse((self.root / "evaluator/reconciliation").exists())


def check_reconciliation_regressions():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(ReconciliationTests))
    if not result.wasSuccessful():
        raise AssertionError("reconciliation regressions failed")


if __name__ == "__main__":
    unittest.main()
