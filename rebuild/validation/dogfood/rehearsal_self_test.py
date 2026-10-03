"""Isolated contract controls; never a real Product-backed rehearsal receipt."""
import copy
import json
from pathlib import Path
import unittest

import evidence_purpose as purpose
import rehearsal_contract as contract
import rehearsal  # Import the actual runner and its bounded workflow builders.


from rehearsal_test_support import passed_result


class ContractTests(unittest.TestCase):
    def test_actual_child_timeout_is_reaped_and_streams_retained(self):
        import os
        import subprocess
        import sys
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            processes = rehearsal.Processes(Path(directory) / 'processes', process_timeout=0.05)
            children = []
            spawn = subprocess.Popen
            def observed_spawn(*args, **kwargs):
                child = spawn(*args, **kwargs)
                children.append(child)
                return child
            with patch.object(rehearsal.subprocess, 'Popen', side_effect=observed_spawn):
                with self.assertRaises(subprocess.TimeoutExpired):
                    processes.run([sys.executable, '-c', 'import time; time.sleep(60)'])
            self.assertEqual(len(children), 1)
            self.assertIsNotNone(children[0].poll())
            with self.assertRaises(ChildProcessError): os.waitpid(children[0].pid, os.WNOHANG)
            self.assertEqual(processes.records[0]['termination'], 'signal')
            self.assertLess(processes.records[0]['exit_code'], 0)
            for name in ('stdout', 'stderr'):
                body = (processes.root / ('process-0000.' + name)).read_bytes()
                self.assertEqual(processes.records[0][name], rehearsal.explanation_evidence.binding(body))

    def test_independent_contract_matches_maintained_inputs(self):
        fixture = json.loads(contract.FIXTURE.read_bytes())
        self.assertEqual(fixture['expected_inner']['replacement_qualification'], contract.EXPECTED_INNER)
        self.assertEqual(tuple(fixture['controls']), contract.CONTROLS)
        self.assertEqual(json.loads(Path(__file__).with_name('evaluation.json').read_bytes())['qualification_policy']['campaign_topology'], contract.TOPOLOGY)

    def test_support_memory_and_obligations_use_the_same_purpose(self):
        import campaign
        import resource_observer
        artifacts = {"volicord-mcp": {"sha256": "a" * 64}}
        memory = resource_observer.initial(artifacts, purpose=purpose.REHEARSAL)
        obligations = campaign.live_evidence_obligations(artifacts, memory)
        self.assertEqual(obligations["naturalistic_resource"], memory)
        self.assertEqual(memory["process_ownership"], "test_support_owned_candidate_process")
        resource_observer.validate(memory, "a" * 64)

    def test_authored_provenance_cannot_lose_its_purpose(self):
        from types import SimpleNamespace
        for metadata in ({}, {"evidence_purpose": purpose.NATURALISTIC}):
            capture = SimpleNamespace(observed_metadata={"session_meta": {
                "source": "self_authored_test_support", "source_metadata": metadata}})
            with self.assertRaises(ValueError):
                purpose.capture_purpose(capture)

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
        for mutation in ("candidate", "fixture", "producer", "processes", "exit", "signal", "inner", "human", "samples", "controls", "approval", "teardown", "purpose", "binary", "raw"):
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
            elif mutation == "binary": value["executables"]["volicord-mcp"] = "0" * 64
            elif mutation == "raw": value["raw_source_body"] = "private source must stay local"
            elif mutation == "purpose": value["evidence_purpose"] = purpose.NATURALISTIC
            value["result_id"] = contract.digest({k: v for k, v in value.items() if k != "result_id"})
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                contract.validate_result(value, "a" * 40)

    def test_rehashed_temporal_claims_require_actual_retained_evidence(self):
        original = passed_result()
        for mutation in ('missing', 'labels_only', 'no_process', 'no_receipt', 'drop_history',
                'other_locale', 'old_final', 'authorization_as_support', 'unordered', 'gap_as_pass',
                'hide_hard_error', 'no_copy', 'missing_outcome'):
            value = copy.deepcopy(original)
            temporal = value['pipeline']['temporal_evidence']
            if mutation == 'missing': value['pipeline'].pop('temporal_evidence')
            elif mutation == 'labels_only': temporal['outcomes'] = dict.fromkeys(contract.TEMPORAL_CONTROLS, 'passed')
            elif mutation == 'no_process': temporal['regeneration']['correction_process'] = 'absent-process'
            elif mutation == 'no_receipt': temporal['regeneration']['lifecycles'][0]['stages'].pop('receipt')
            elif mutation == 'drop_history': temporal['regeneration']['lifecycles'].pop(0)
            elif mutation == 'other_locale': temporal['regeneration']['lifecycles'][0]['language'] = 'ko'
            elif mutation == 'old_final': temporal['regeneration']['lifecycles'][1]['goal_revision'] = 1
            elif mutation == 'authorization_as_support': temporal['recall']['supporting_sources'] = [temporal['recall']['authorization_source_id']]
            elif mutation == 'unordered': temporal['recall']['correction']['invocation_sequence'] = 5
            elif mutation == 'gap_as_pass': temporal['outcomes']['missing_temporal_evidence']['status'] = 'confirmed_pass'
            elif mutation == 'hide_hard_error': temporal['outcomes']['missing_temporal_evidence_with_violation']['error_classes'] = []
            elif mutation == 'no_copy': temporal['copied']['lifecycles'].popitem()
            elif mutation == 'missing_outcome': temporal['outcomes'].popitem()
            value['result_id'] = contract.digest({k: v for k, v in value.items() if k != 'result_id'})
            with self.subTest(mutation=mutation), self.assertRaises((ValueError, TypeError, KeyError)):
                contract.validate_result(value, 'a' * 40)


if __name__ == "__main__":
    unittest.main()
