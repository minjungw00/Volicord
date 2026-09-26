"""Independent dimension coverage controls; synthetic data only."""
import copy
import json
import unittest

import harness


class BlindDimensionTests(unittest.TestCase):
    def setUp(self):
        self.obligations = ["learning_deliberation", "learning_routine_control"]
        self.assessments = harness.fixture_blind_assessments(self.obligations)
        self.provisional = {"kind": "phase8_provisional_behavior_review", "status": "recorded",
            "reviewer_role": "campaign_preparation_independent_reviewer", "review_slot_id": "1" * 32,
            "preparation_sha256": "a" * 64, "classification": "learning_deliberation",
            "materiality_conclusion": "no_user_owned_material_outcome", "material_outcome_unavoidable": False,
            "operator_prompt_does_not_disclose_material_outcome": None, "basis": "Pinned owner source basis.",
            "provenance_reference_indices": [0], "assessments": self.assessments}
        self.preparation = {"kind": "phase8_blind_review_preparation_reference",
            "review_slot_id": "1" * 32, "sha256": "a" * 64}
        self.comparison = {"status": "resolved_from_evidence", "provisional_classification": "learning_deliberation",
            "evaluator_classification": self.obligations, "disagreements": ["classification"],
            "resolution_basis": "Both independently assessed dimensions occur in this bounded fixture task.",
            "provenance_reference_indices": [0],
            "obligation_coverage": harness.fixture_obligation_coverage(self.assessments)}

    def errors(self):
        return harness.classification_comparison_errors(self.comparison, self.provisional, self.obligations, 1)

    def test_all_obligations_independently_discovered_before_reveal(self):
        self.assertEqual(harness.blind_first_review_errors(self.preparation, self.provisional, 1), [])
        self.assertEqual(self.errors(), [])

    def test_scalar_only_cannot_satisfy_multi_obligation_work(self):
        del self.provisional["assessments"]
        self.assertTrue(harness.blind_first_review_errors(self.preparation, self.provisional, 1))
        self.assertTrue(any("blind_coverage_gap" in e for e in self.errors()))

    def test_unseen_revealed_obligation_cannot_be_resolved_into_coverage(self):
        self.provisional["assessments"] = self.assessments[:1]
        self.assertTrue(any("blind_coverage_gap" in e for e in self.errors()))
        row = self.comparison["obligation_coverage"][1]
        row.update(dimension_id=None, status="blind_coverage_gap", reviewer_outcome_scope=None)
        self.assertTrue(any("blind_coverage_gap" in e for e in self.errors()))
        # Post-reveal mapping to a second unreviewed ID remains a gap.
        row.update(dimension_id="invented-after-reveal", status="resolved_from_evidence")
        self.assertTrue(any("blind_coverage_gap" in e for e in self.errors()))

    def test_dimension_reuse_and_scope_rewrite_cannot_manufacture_coverage(self):
        original = copy.deepcopy(self.comparison)
        row = self.comparison["obligation_coverage"][1]
        row["dimension_id"] = self.assessments[0]["dimension_id"]
        self.assertTrue(any("blind_coverage_gap" in e for e in self.errors()))
        self.comparison = original
        self.comparison["obligation_coverage"][0]["reviewer_outcome_scope"] = "newly invented outcome"
        self.assertTrue(any("blind_coverage_gap" in e for e in self.errors()))

    def test_fact_disagreement_on_fixed_dimension_preserves_provisional_bytes(self):
        item = self.assessments[0]
        item["classification"] = "repository_or_environment_fact"
        self.assertEqual(harness.blind_first_review_errors(self.preparation, self.provisional, 1), [])
        fixed = json.dumps(self.provisional, sort_keys=True).encode()
        row = self.comparison["obligation_coverage"][0]
        row["status"] = "resolved_from_evidence"
        row["basis"] = "The already inspected fork has deliberation value under the pinned participation source."
        self.assertEqual(self.errors(), [])
        self.assertEqual(json.dumps(self.provisional, sort_keys=True).encode(), fixed)

    def test_assessment_identity_bounds_and_typed_grounding(self):
        for field, bad in (("dimension_id", "../outside"), ("outcome_scope", ""),
                           ("classification", "invented"), ("provenance_reference_indices", [True])):
            proposed = copy.deepcopy(self.provisional)
            proposed["assessments"][0][field] = bad
            self.assertTrue(harness.blind_first_review_errors(self.preparation, proposed, 1))


def check_blind_dimension_regressions():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(BlindDimensionTests))
    if not result.wasSuccessful():
        raise AssertionError("blind dimension regressions failed")


if __name__ == "__main__":
    unittest.main()
