"""Explicit fake execution owners for tests; never imported by production runners."""
import hashlib
import json
from pathlib import Path
import runpy
from types import SimpleNamespace

contract = SimpleNamespace(**runpy.run_path(str(Path(__file__).with_name("rehearsal_contract.py"))))
ROOT = Path(__file__).resolve().parents[3]

def temporal_fixture():
    """Synthetic portable facts for contract tests; never Product receipts."""
    artifact = {'bytes': 100, 'sha256': '1' * 64}
    sources = ['04' * 16]
    lives = []
    for language, base in [('en', 1), ('ko', 3)]:
        for revision in (1, 2):
            lives.append({'identity': f'{base + revision - 1:032x}',
                'publication_role': 'historical' if revision == 1 else 'final',
                'selected_identity': f'{base + 1:032x}', 'language': language,
                'goal_revision': revision, 'goal_sources': sources,
                'before_state': 'unavailable' if revision == 1 else 'stale', 'after_state': 'current',
                'plan_fingerprint': 'sha256:' + str(revision) * 64,
                'stages': {name: dict(artifact) for name in ('attempt','preparation','response','record','after','receipt')}})
    observations = [{'transport': 'mcp', 'call_id': f'call-{i}', 'sequence': 10 + 10*i,
        'completion_sequence': 10 + 10*i, 'invocation_sequence': 9 + 10*i,
        'raw_capture_sha256': 'a' * 64, 'status': 'confirmed_pass',
        'goal_revision': revision, 'basis_sha256': 'b' * 64} for i,revision in enumerate((1,2,2))]
    correction = {'transport': 'mcp', 'call_id': 'correction', 'sequence': 16, 'completion_sequence': 16,
        'invocation_sequence': 15, 'kind': 'correct_context', 'raw_capture_sha256': 'a' * 64,
        'session_id': 'support-only', 'expected_revision': 1, 'actual_revision': 2,
        'authorization_source_id': '05' * 16, 'supporting_sources_changed': False, 'transition': 'successful_correction'}
    outcomes = {}
    for name,expected in contract.TEMPORAL_CONTROLS.items():
        recall = expected[0] == 'naturalistic_observation'
        statuses = []
        revisions = []
        if recall:
            statuses = ['confirmed_pass'] * 3
            revisions = [1,2,2]
            if name == 'post_correction_old_revision': statuses[1] = 'confirmed_violation'
            elif name == 'future_correction_scope': statuses[0] = 'confirmed_violation'
            elif name.startswith('missing_temporal'):
                statuses[1:] = [expected[2], 'indeterminate']
                revisions = [1,None,None]
        outcomes[name] = dict(zip(('consumer','check','status','disposition'),expected)) | {
            'basis_sha256': contract.digest({'lifecycles':lives}) if name == 'historical_explanation_regeneration' else 'c'*64,
            'finding': dict(artifact), 'artifacts': [v['stages']['receipt'] for v in lives] if name == 'historical_explanation_regeneration' else [dict(artifact)],
            'observation_statuses': statuses, 'goal_revisions': revisions,
            'error_classes': ['generated goal revision basis'] if name in {'post_correction_old_revision', 'future_correction_scope'} else (
                ['recorded action'] if name == 'missing_temporal_evidence_with_violation' else []),
            'rejection_sha256': None if recall or name == 'historical_explanation_regeneration' else 'd' * 64}
    return {'regeneration': {'work_slot_id': 'journey-polyglot-medium-work-a', 'project_id': '02'*16,
        'work_item_id': '03'*16, 'authorization_source_id': '05'*16, 'correction_process': 'support-process-0',
        'correction_receipt': dict(artifact), 'lifecycles': lives},
        'recall': {'work_slot_id':'journey-small-python-work-a', 'supporting_sources': sources,
            'authorization_source_id':'05'*16, 'observations': observations, 'correction': correction},
        'outcomes':outcomes, 'copied':{'lifecycles': {v['identity']:dict(artifact) for v in lives},
            'recall_capture': dict(artifact)}}


def passed_result(candidate="a" * 40):
    """Explicit fake execution owner for orchestration/portable contract tests."""
    fixture = json.loads(contract.FIXTURE.read_bytes())
    value = {"kind": "dogfood_evidence_rehearsal", "candidate_head": candidate,
        "evidence_purpose": "dogfood_rehearsal", **contract.identities(), "status": "passed",
        "teardown": "completed", "external_transmission": "none", "operator_approval": "not_provided", "gate_binding": None,
        "executables": dict.fromkeys(("volicord", "volicord-mcp", "volicord-viewer"), "b" * 64),
        "pipeline": {"executables": dict.fromkeys(("volicord", "volicord-mcp", "volicord-viewer"), "b" * 64), "evidence_set_sha256": "c" * 64, "evaluation_run_id": "d" * 64,
            "qualification_run_id": "e" * 64, "expected_inner_verdict": fixture["expected_inner"]["replacement_qualification"],
            "technical_evidence": "not_provided", "human_observations": "not_provided",
            "unresolved_criteria_count": 200, "hard_findings": [],
            "copied_lineage_id": "f" * 64, "copied_verification": "verified", "resource_sample_count": 3,
            "topology": json.loads(Path(__file__).with_name("evaluation.json").read_bytes())["qualification_policy"]["campaign_topology"],
            "measured_evidence_eligible": False, "controls": dict.fromkeys(fixture["controls"], "passed"), "temporal_evidence": temporal_fixture()},
        "processes": [{"identity": "support-process-" + str(i), "exit_code": 0,
            "termination": "exited", "duration_ns": 100,
            "stdout": {"bytes": 1, "sha256": "1" * 64}, "stderr": {"bytes": 0, "sha256": "2" * 64}}
            for i in range(16)]}
    value["result_id"] = contract.digest(value)
    return value



def fake_owner(root, candidate, final_hash, invocation):
    """Produce test-only wrapper files as well as an independently consumed result."""
    root.mkdir(parents=True, exist_ok=True)
    result = passed_result(candidate)
    result['gate_binding'] = {'gate_invocation': invocation, 'final_sha256': final_hash,
        'binding_sha256': '9' * 64}
    result['result_id'] = contract.digest({k: v for k, v in result.items() if k != 'result_id'})
    path = root / 'result.json'
    path.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    invocation_dir = root.parent / 'dogfood-rehearsal-invocation'
    invocation_dir.mkdir(parents=True, exist_ok=True)
    stdout, stderr = invocation_dir / 'stdout.log', invocation_dir / 'stderr.log'
    stdout.write_bytes(b'Explicit fake execution owner\n'); stderr.write_bytes(b'')
    execution = {'argv': [str(ROOT / 'rebuild/validation/dogfood/rehearsal.py'),
        '--candidate-head', candidate, '--final-artifact', str(root.parent / 'final/summary.json'),
        '--output', str(root)], 'working_directory': str(ROOT),
        'started_at': '2026-08-21T00:00:02.000000+00:00',
        'ended_at': '2026-08-21T00:00:02.001000+00:00', 'duration_ms': 1.0,
        'outcome': 'succeeded', 'exit_code': 0, 'wrapper_exit_code': 0,
        'termination': None, 'spawn_error': None, 'stdout': str(stdout), 'stderr': str(stderr)}
    (invocation_dir / 'result.json').write_text(json.dumps(execution))
    return result, execution, path


def attach_stage(capsule, gate_dir, candidate, final_hash):
    capsule['gate_invocation'] = gate_dir.name
    capsule.setdefault('dependency_snapshot', {})['dogfood_rehearsal'] = contract.dependencies(ROOT)
    result, execution, path = fake_owner(gate_dir / 'dogfood-rehearsal', candidate, final_hash, gate_dir.name)
    capsule['final_aggregate'] = {'status': 'succeeded'}
    capsule['contract_coverage_execution'] = {'status': 'passed'}
    capsule['dogfood_rehearsal'] = contract.stage_evidence(result, execution, path, candidate,
        final_hash, gate_dir.name, capsule['dependency_snapshot']['dogfood_rehearsal'])
    return capsule['dogfood_rehearsal']
