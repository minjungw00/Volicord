"""Qualification authority negative controls; no naturalistic passage claims."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import machine_findings as m
import qualitative_review as review
import qualitative_review_self_test as fixtures
import qualification_policy as policy


class PolicyTests(unittest.TestCase):
    def setUp(self):
        self.prep = fixtures.preparation()
        self.specs = review.criterion_specs(self.prep["index"], self.prep["rubric"])
        self.agent = fixtures.completed(self.prep)
        self.human_prep = fixtures.preparation("human")
        self.human_prep["reviewer"]["run_id"] = "b" * 32
        self.human = fixtures.completed(self.human_prep)
        self.evaluation = {"run_id": "a" * 64, "cycles": [{"repository_class": "volicord", "cycle": 1, "findings": []}]}
        self.technical = {"state": "passed", "candidate_head": "a" * 40}

    def result(self, reviews=None, technical=None):
        return policy.combine(self.evaluation, self.specs, reviews if reviews is not None else [self.agent, self.human], technical or self.technical)

    def test_agent_covers_semantics_human_only_targeted_observations(self):
        value = self.result([self.agent])
        self.assertTrue(value["qualitative_review"]["resolved_criteria"])
        self.assertTrue(all('/live_viewer/' in c or c.endswith('/decision_comprehension_when_applicable')
            for c in value["qualitative_review"]["human_escalations"]))
        self.assertEqual(value["replacement_qualification"], "unresolved")
        self.assertFalse(value["phase_9_ready"])
        required = set(value["qualitative_review"]["human_escalations"])
        for i, a in enumerate(self.human["assessments"]):
            if a["criterion_id"] not in required:
                self.human["assessments"][i] = review.observation(a["criterion_id"])
        review.validate_value(self.human_prep, "d" * 64, self.human)
        self.assertEqual(self.result()["replacement_qualification"], "qualified")
        self.assertFalse(self.result()["phase_9_ready"])

    def test_hard_integrity_neither_agent_nor_human_can_override(self):
        self.evaluation["cycles"][0]["findings"] = [m.finding("raw_hash", "confirmed_violation", {"mismatch": True})]
        for reviews in ([self.agent], [self.human], [self.agent, self.human]):
            self.assertEqual(self.result(reviews)["replacement_qualification"], "blocked")
        self.assertEqual(policy.combine(self.evaluation, self.specs, [], self.technical, evidence_validity="invalid")["replacement_qualification"], "blocked")

    def test_indeterminate_is_unresolved_and_valid_agent_evidence_can_clarify(self):
        finding = m.finding("appropriate_inquiry_outcome", "indeterminate", {"observed": "ambiguous"})
        self.evaluation["cycles"][0]["findings"] = [finding]
        value = self.result()
        self.assertEqual(value["replacement_qualification"], "unresolved")
        cid = 'volicord-1/appropriate_inquiry_outcome'
        self.prep["index"]["machine_findings"][cid] = {"sample_id": "volicord-1", "finding": finding}
        self.prep["binding"]["machine_evaluation"] = {"run_id": self.evaluation["run_id"], "sha256": "f" * 64}
        self.agent["binding"] = copy.deepcopy(self.prep["binding"])
        a = next(a for a in self.agent["assessments"] if a["criterion_id"].endswith('/correct_no_question_behavior'))
        a["machine_relationships"] = [{"finding_id": cid, "relationship": "clarifies_indeterminate", "reasoning": "Bounded source-backed observation resolves the classifier ambiguity."}]
        review.validate_value(self.prep, "d" * 64, self.agent)
        self.assertEqual(self.result()["replacement_qualification"], "qualified")
        a["assessment"] = "insufficient_evidence"
        a["machine_relationships"] = []
        self.assertEqual(self.result()["replacement_qualification"], "unresolved")

    def test_insufficient_and_missing_technical_gate_cannot_pass(self):
        for a in self.agent["assessments"]:
            a["assessment"] = "insufficient_evidence"
        self.assertEqual(self.result([self.agent])["replacement_qualification"], "unresolved")
        self.assertEqual(self.result([self.human], {"state": "not_provided"})["replacement_qualification"], "unresolved")
        self.assertEqual(self.result([self.human], {"state": "failed"})["replacement_qualification"], "blocked")

    def test_conflict_requires_explicit_targeted_human_resolution(self):
        cid = self.agent["assessments"][0]["criterion_id"]
        negative = copy.deepcopy(self.agent)
        negative["reviewer"]["run_id"] = "c" * 32
        negative["assessments"][0]["assessment"] = "violated"
        result = self.result([self.agent, negative, self.human])
        self.assertIn(cid, result["qualitative_review"]["human_escalations"])
        self.human["resolves_review_runs"] = {cid: [self.agent["reviewer"]["run_id"], negative["reviewer"]["run_id"]]}
        self.assertEqual(self.result([self.agent, negative, self.human])["replacement_qualification"], "qualified")
        self.agent["resolves_review_runs"] = self.human["resolves_review_runs"]
        with self.assertRaisesRegex(ValueError, "only human"):
            review.validate_value(self.prep, "d" * 64, self.agent)

    def test_approval_refuses_missing_required_review(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'qualification.json'
            path.write_text(json.dumps({"replacement_qualification": "unresolved"}))
            with patch.object(policy, 'verify_qualification'):
                with self.assertRaisesRegex(ValueError, 'cannot replace'):
                    policy.approve(path, Path(directory) / 'approval', operator='operator', statement='approve-phase-9')
            self.assertFalse((Path(directory) / 'approval').exists())


class FileBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from review_operations_self_test import WorkflowTests
        WorkflowTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_real_evidence_reviews_result_revalidation_and_mismatch(self):
        import campaign
        import review_operations as ops
        from review_operations_self_test import insufficient_draft, snapshot
        before = snapshot(self.root)
        target = self.parent / 'incomplete-review'
        ops.prepare(self.root, target, reviewer_kind='agent', session_id='separate-review', evaluation_path=self.evaluation)
        insufficient_draft(target)
        ops.record(target, target / 'draft.json')
        output = self.parent / 'qualification'
        with patch.object(campaign.harness, 'real_session_evidence', side_effect=AssertionError('naturalistic rerun')):
            value = policy.qualify(self.root, self.evaluation, output,
                candidate=campaign.load_campaign(self.root)['candidate_head'], review_roots=[target])
            self.assertNotEqual(value['replacement_qualification'], 'qualified')
            self.assertEqual(value['technical_gate']['state'], 'not_provided')
            self.assertEqual(policy.verify_qualification(output / 'qualification.json'), value)
            with self.assertRaisesRegex(ValueError, 'cannot replace'):
                policy.approve(output / 'qualification.json', self.parent / 'approval', operator='operator', statement='approve-phase-9')
            with self.assertRaisesRegex(ValueError, 'candidate mismatch'):
                policy.qualify(self.root, self.evaluation, self.parent / 'wrong-candidate', candidate='0' * 40)
        self.assertEqual(snapshot(self.root), before)
        changed = copy.deepcopy(value)
        changed['qualitative_review']['unresolved_criteria'] = []
        changed['run_id'] = m.digest({k: v for k, v in changed.items() if k != 'run_id'})
        (output / 'qualification.json').chmod(0o600)
        (output / 'qualification.json').write_bytes(ops.encoded(changed))
        with self.assertRaises(ValueError):
            policy.verify_qualification(output / 'qualification.json')

    def test_agent_cannot_supply_human_observation(self):
        import review_operations as ops
        with self.assertRaisesRegex(ValueError, 'agent preparation'):
            ops.prepare(self.root, self.parent / 'false-human', reviewer_kind='agent', session_id='review',
                human_observations=self.parent / 'missing.json')


def run_contract_tests():
    result = unittest.TextTestRunner().run(unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(cls) for cls in (PolicyTests, FileBoundaryTests)))
    if not result.wasSuccessful():
        raise AssertionError('qualification policy regressions failed')


if __name__ == '__main__':
    unittest.main()
