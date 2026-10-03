"""Isolated contract controls; never a real Product-backed rehearsal receipt."""
import copy
import json
from pathlib import Path
import unittest

import evidence_purpose as purpose
import rehearsal_contract as contract
import rehearsal  # Import the actual runner and its bounded workflow builders.


def passed_result(candidate="a" * 40):
    """Explicit fake execution owner for orchestration/portable contract tests."""
    fixture = json.loads(contract.FIXTURE.read_bytes())
    value = {"kind": "dogfood_evidence_rehearsal", "candidate_head": candidate,
        "evidence_purpose": purpose.REHEARSAL, **contract.identities(), "status": "passed",
        "teardown": "completed", "external_transmission": "none", "operator_approval": "not_provided",
        "executables": dict.fromkeys(("volicord", "volicord-mcp", "volicord-viewer"), "b" * 64),
        "pipeline": {"evidence_set_sha256": "c" * 64, "evaluation_run_id": "d" * 64,
            "qualification_run_id": "e" * 64, "expected_inner_verdict": fixture["expected_inner"]["replacement_qualification"],
            "technical_evidence": "not_provided", "human_observations": "not_provided",
            "unresolved_criteria_count": 200, "hard_findings": ["journey-volicord-work-a/required_validation_execution"],
            "copied_lineage_id": "f" * 64, "copied_verification": "verified", "resource_sample_count": 3,
            "topology": json.loads(Path(__file__).with_name("evaluation.json").read_bytes())["qualification_policy"]["campaign_topology"],
            "measured_evidence_eligible": False, "controls": dict.fromkeys(fixture["controls"], "passed")},
        "processes": [{"identity": "support-process-" + str(i), "exit_code": 0,
            "termination": "exited", "duration_ns": 100,
            "stdout": {"bytes": 1, "sha256": "1" * 64}, "stderr": {"bytes": 0, "sha256": "2" * 64}}
            for i in range(16)]}
    value["result_id"] = contract.digest(value)
    return value


class ContractTests(unittest.TestCase):
    def test_support_memory_and_obligations_use_the_same_purpose(self):
        import campaign
        import resource_observer
        artifacts = {"volicord-mcp": {"sha256": "a" * 64}}
        memory = resource_observer.initial(artifacts, purpose=purpose.REHEARSAL)
        obligations = campaign.live_evidence_obligations(artifacts, memory)
        self.assertEqual(obligations["naturalistic_resource"], memory)
        self.assertEqual(memory["process_ownership"], "test_support_owned_candidate_process")
        resource_observer.validate(memory, "a" * 64)

    def test_closed_purpose_and_no_measured_approval(self):
        for value in (None, "support", "vscode"):
            with self.assertRaises(ValueError):
                purpose.validate(value)
        with self.assertRaises(ValueError):
            purpose.require_measured({"evidence_purpose": purpose.REHEARSAL})
        with self.assertRaises(ValueError):
            purpose.require_same({"evidence_purpose": purpose.REHEARSAL}, {"evidence_purpose": purpose.NATURALISTIC})

    def test_passed_claim_requires_pipeline_execution_and_inner_limits(self):
        original = passed_result()
        contract.validate_result(original, "a" * 40)
        for mutation in ("candidate", "fixture", "producer", "processes", "exit", "signal", "inner", "human", "samples", "controls", "approval", "teardown", "purpose"):
            value = copy.deepcopy(original)
            if mutation == "candidate": value["candidate_head"] = "0" * 40
            elif mutation == "fixture": value["fixture_sha256"] = "0" * 64
            elif mutation == "producer": value["producer_sha256"]["rehearsal.py"] = "0" * 64
            elif mutation == "processes": value["processes"] = []
            elif mutation == "exit": value["processes"][0]["exit_code"] = 1
            elif mutation == "signal": value["processes"][0]["termination"] = "signal"
            elif mutation == "inner": value["pipeline"]["expected_inner_verdict"] = "qualified"
            elif mutation == "human": value["pipeline"]["human_observations"] = "passed"
            elif mutation == "samples": value["pipeline"]["resource_sample_count"] = 0
            elif mutation == "controls": value["pipeline"]["controls"].popitem()
            elif mutation == "approval": value["operator_approval"] = "approved"
            elif mutation == "teardown": value["teardown"] = "pending"
            elif mutation == "purpose": value["evidence_purpose"] = purpose.NATURALISTIC
            value["result_id"] = contract.digest({k: v for k, v in value.items() if k != "result_id"})
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                contract.validate_result(value, "a" * 40)


if __name__ == "__main__":
    unittest.main()
