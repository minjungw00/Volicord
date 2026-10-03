"""Bounded rehearsal evidence contract; no Product execution or archive dependency."""
import hashlib
import json
from pathlib import Path
import re

def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()

def require(value, message):
    if not value:
        raise ValueError(message)

CONTRACT = "product-backed-dogfood-evidence-rehearsal-1"
FIXTURE = Path(__file__).with_name("fixtures") / "evidence-rehearsal.json"
PRODUCER_FILES = ("rehearsal.py", "rehearsal_support.py", "rehearsal_contract.py", "evidence_purpose.py", "campaign.py", "codex_events.py",
    "answer_observations.py", "explanation_evidence.py", "document_realization.py",
    "review_operations.py", "review_captures.py", "review_explanations.py",
    "qualification_policy.py", "result_lineage.py", "resource_observer.py", "harness.py",
    "evaluation.json", "evaluation_runs.py", "machine_findings.py", "machine-policy.json",
    "answer_projection.py", "support_evidence.py", "authority_obligations.py", "cli_observations.py",
    "qualitative_review.py", "workload_intents.py", "identity_provenance.py", "repository_state.py",
    "interaction_diagnostics.py", "../shared/recorded_action_evidence.py", "../../install.sh", "../../scripts/dogfood-campaign")


EXPECTED_INNER = "unresolved"
CONTROLS = ('duplicate_titles', 'contradictory_shared_answer', 'missing_explanation_plan', 'mismatched_explanation_response', 'review_projection_omission', 'changed_artifact', 'memory_binary_mismatch', 'absent_human_observations', 'rehearsal_not_measured', 'copied_semantic_rehash')
TOPOLOGY = {'repository_journeys': 3, 'work_items': 5, 'resume_pairs': 3, 'fresh_sessions': 8, 'work_distribution': {'volicord': 3, 'small-python': 1, 'polyglot-medium': 1}, 'resume_repository_classes': ['polyglot-medium', 'small-python', 'volicord']}

def identities():
    return {"contract": CONTRACT, "fixture_sha256": sha256(FIXTURE),
        "producer_sha256": {name: sha256(Path(__file__).parent / name)
            for name in PRODUCER_FILES}}


def validate_result(value, candidate, *, expected_identities=None):
    """Same predicate for gate consumption and independent bounded verification."""
    expected = expected_identities or identities()
    require(set(value) == {"kind", "candidate_head", "evidence_purpose", "contract", "fixture_sha256",
        "producer_sha256", "status", "teardown", "external_transmission", "operator_approval",
        "executables", "pipeline", "processes", "result_id", "gate_binding"}, "unexpected retained rehearsal content")
    require(set(value["executables"]) == {"volicord", "volicord-mcp", "volicord-viewer"}
        and all(re.fullmatch(r"[0-9a-f]{64}", v) for v in value["executables"].values()), "invalid executable identities")
    require(value.get("kind") == "dogfood_evidence_rehearsal"
        and value.get("candidate_head") == candidate and value.get("evidence_purpose") == "dogfood_rehearsal"
        and all(value.get(k) == v for k, v in expected.items())
        and value.get("status") == "passed" and value.get("teardown") == "completed"
        and value.get("external_transmission") == "none" and value.get("operator_approval") == "not_provided",
        "rehearsal identity/status boundary failed")
    require(value.get("result_id") == digest({k: v for k, v in value.items() if k != "result_id"}),
        "rehearsal result hash changed")
    validate_binding(value["gate_binding"])
    pipeline = value["pipeline"]
    require(set(pipeline) == {"evidence_set_sha256", "evaluation_run_id", "qualification_run_id",
        "expected_inner_verdict", "technical_evidence", "human_observations", "unresolved_criteria_count",
        "hard_findings", "copied_lineage_id", "copied_verification", "resource_sample_count", "topology",
        "measured_evidence_eligible", "controls", "executables"}, "unexpected pipeline content")
    for key in ("evidence_set_sha256", "evaluation_run_id", "qualification_run_id", "copied_lineage_id"):
        require(re.fullmatch(r"[0-9a-f]{64}", pipeline[key]), "invalid pipeline identity")
    require(pipeline["expected_inner_verdict"] == EXPECTED_INNER
        and pipeline["executables"] == value["executables"]
        and pipeline["hard_findings"] == []
        and type(pipeline["unresolved_criteria_count"]) is int
        and type(pipeline["resource_sample_count"]) is int
        and pipeline["technical_evidence"] == "not_provided" and pipeline["human_observations"] == "not_provided"
        and pipeline["unresolved_criteria_count"] > 0 and pipeline["copied_verification"] == "verified"
        and pipeline["resource_sample_count"] > 0 and pipeline["measured_evidence_eligible"] is False
        and pipeline["topology"] == TOPOLOGY
        and pipeline["controls"] == dict.fromkeys(CONTROLS, "passed"),
        "rehearsal pipeline outcome inconsistent")
    records = value.get("processes")
    require(isinstance(records, list) and len(records) >= 16 and len(records) <= 200
        and len({r["identity"] for r in records}) == len(records)
        and all(type(r["exit_code"]) is int and r["exit_code"] == 0 and r["termination"] == "exited" and type(r["duration_ns"]) is int and r["duration_ns"] >= 0
            for r in records), "missing/failed rehearsal process evidence")
    for record in records:
        require(set(record) == {"identity", "exit_code", "termination", "duration_ns", "stdout", "stderr"}
            and re.fullmatch(r"[a-z0-9-]{1,80}", record["identity"]), "unexpected process content")
        for stream in ("stdout", "stderr"):
            require(set(record[stream]) == {"bytes", "sha256"}
                and type(record[stream]["bytes"]) is int and record[stream]["bytes"] >= 0
                and re.fullmatch(r"[0-9a-f]{64}", record[stream]["sha256"]), "invalid process stream evidence")
    return value

def validate_binding(value):
    if value is None:
        return
    require(isinstance(value, dict) and set(value) == {'gate_invocation', 'final_sha256', 'binding_sha256'}
        and re.fullmatch(r'[A-Za-z0-9_.-]{1,160}', str(value['gate_invocation']))
        and all(re.fullmatch(r'[0-9a-f]{64}', str(value[k])) for k in ('final_sha256', 'binding_sha256')),
        'invalid bounded gate binding')


def validate_retained_result(value, candidate, expected):
    """Failure receipts retain only closed identities/counts; messages stay private."""
    if value.get('status') == 'passed':
        return validate_result(value, candidate, expected_identities=expected)
    common = {'kind', 'candidate_head', 'evidence_purpose', 'contract', 'fixture_sha256',
        'producer_sha256', 'status', 'teardown', 'external_transmission', 'operator_approval',
        'processes', 'result_id', 'gate_binding', 'failure_kind'}
    require(common <= set(value) <= common | {'executables', 'pipeline'}, 'unexpected failed rehearsal content')
    require(value['kind'] == 'dogfood_evidence_rehearsal' and value['candidate_head'] == candidate
        and value['evidence_purpose'] == 'dogfood_rehearsal' and value['status'] == 'failed'
        and value['teardown'] in {'completed', 'failed'} and value['external_transmission'] == 'none'
        and value['operator_approval'] == 'not_provided' and all(value[k] == v for k, v in expected.items())
        and re.fullmatch(r'[A-Za-z]{1,80}', str(value['failure_kind']))
        and value['result_id'] == digest({k: v for k, v in value.items() if k != 'result_id'}),
        'invalid failed rehearsal receipt')
    validate_binding(value['gate_binding'])
    require(isinstance(value['processes'], list) and len(value['processes']) <= 200, 'invalid failed processes')
    for record in value['processes']:
        require(set(record) == {'identity', 'exit_code', 'termination', 'duration_ns', 'stdout', 'stderr'}
            and re.fullmatch(r'[a-z0-9-]{1,80}', str(record['identity']))
            and type(record['exit_code']) is int and record['termination'] in {'exited', 'signal', 'timeout', 'interrupted'}
            and type(record['duration_ns']) is int and record['duration_ns'] >= 0, 'invalid failed process')
        for name in ('stdout', 'stderr'):
            stream = record[name]
            require(set(stream) == {'bytes', 'sha256'} and type(stream['bytes']) is int and stream['bytes'] >= 0
                and re.fullmatch(r'[0-9a-f]{64}', str(stream['sha256'])), 'invalid failed stream')
    if 'executables' in value:
        require(set(value['executables']) == {'volicord', 'volicord-mcp', 'volicord-viewer'}
            and all(re.fullmatch(r'[0-9a-f]{64}', str(v)) for v in value['executables'].values()),
            'invalid failed executable identity')
    if 'pipeline' in value:
        # Only an already completed pipeline can precede a later continuity failure.
        positive = {k: v for k, v in value.items() if k != 'failure_kind'}
        positive.update(status='passed', teardown='completed')
        positive['result_id'] = digest({k: v for k, v in positive.items() if k != 'result_id'})
        validate_result(positive, candidate, expected_identities=expected)
    return value


def dependencies(root):
    base = root / 'rebuild/validation/dogfood'
    paths = {'fixture': base / 'fixtures/evidence-rehearsal.json',
        **{name: base / name for name in PRODUCER_FILES}}
    return {'contract': CONTRACT, 'inputs': {name: {'path': path.resolve().relative_to(root.resolve()).as_posix(),
        'sha256': sha256(path), 'status': 'available'} for name, path in paths.items()}}


def expected_from_dependencies(value):
    require(set(value) == {'contract', 'inputs'} and value['contract'] == CONTRACT,
        'rehearsal dependencies are missing')
    inputs = value['inputs']
    require(set(inputs) == {'fixture', *PRODUCER_FILES}, 'rehearsal dependency inventory changed')
    for name, entry in inputs.items():
        path = (Path('rebuild/validation/dogfood') / ('fixtures/evidence-rehearsal.json' if name == 'fixture' else name))
        # Resolve lexical ../shared without consulting a source checkout.
        from posixpath import normpath
        require(entry == {'path': normpath(str(path)), 'sha256': entry.get('sha256'), 'status': 'available'}
            and isinstance(entry['sha256'], str) and re.fullmatch(r'[0-9a-f]{64}', entry['sha256']),
            'rehearsal dependency identity malformed')
    return {'contract': CONTRACT, 'fixture_sha256': inputs['fixture']['sha256'],
        'producer_sha256': {name: inputs[name]['sha256'] for name in PRODUCER_FILES}}


def not_run_stage():
    return {'contract': CONTRACT, 'status': 'not_run', 'invocation_count': 0,
        'result_sha256': None, 'result': None, 'execution': None}


def bounded_execution(value):
    streams = {}
    for key in ('stdout', 'stderr'):
        path = value.get(key)
        if not path:
            streams[key] = None
        else:
            try:
                body = Path(path).read_bytes()
                streams[key] = {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
            except OSError:
                streams[key] = None
    return {'exit_code': value.get('exit_code'), 'wrapper_exit_code': value.get('wrapper_exit_code'),
        'termination': value.get('termination'), 'spawn_error': value.get('spawn_error') is not None,
        'duration_ms': value.get('duration_ms'), 'started_at': value.get('started_at'),
        'ended_at': value.get('ended_at'), **streams}


def stage_evidence(result, execution, path, candidate, final_hash, invocation, dependency):
    retained = None
    try:
        if path.is_file() and json.loads(path.read_bytes()) == result:
            retained = validate_retained_result(result, candidate, expected_from_dependencies(dependency))
    except (ValueError, OSError, TypeError, KeyError, AttributeError):
        pass
    evidence = {'contract': CONTRACT, 'status': 'failed', 'invocation_count': 1,
        'result_sha256': sha256(path) if retained is not None else None,
        'result': retained, 'execution': bounded_execution(execution)}
    if execution.get('spawn_error') is not None:
        evidence['status'] = 'environment_blocked'
        return evidence
    try:
        require(type(execution.get('exit_code')) is int and execution.get('exit_code') == 0
            and type(execution.get('wrapper_exit_code')) is int and execution.get('wrapper_exit_code') == 0
            and execution.get('termination') is None, 'rehearsal process failed')
        require(path.is_file() and json.loads(path.read_bytes()) == result, 'rehearsal result is missing or changed')
        validate_result(result, candidate, expected_identities=expected_from_dependencies(dependency))
        binding = result['gate_binding']
        require(binding['gate_invocation'] == invocation and binding['final_sha256'] == final_hash,
            'rehearsal is not bound to this gate Final')
        evidence['status'] = 'passed'
        validate_stage({'dogfood_rehearsal': evidence, 'validated_candidate_head': candidate,
            'gate_invocation': invocation, 'final_summary_sha256': final_hash,
            'dependency_snapshot': {'dogfood_rehearsal': dependency},
            'final_aggregate': {'status': 'succeeded'}, 'contract_coverage_execution': {'status': 'passed'},
            'phase_8_ready': False})
    except (ValueError, OSError, TypeError, KeyError, AttributeError):
        evidence['status'] = 'failed'
    return evidence


def validate_stage(capsule):
    stage = capsule.get('dogfood_rehearsal')
    require(isinstance(stage, dict) and set(stage) == set(not_run_stage())
        and stage['contract'] == CONTRACT, 'mandatory rehearsal stage is missing or malformed')
    expected = expected_from_dependencies(capsule.get('dependency_snapshot', {}).get('dogfood_rehearsal', {}))
    require(isinstance(capsule.get('gate_invocation'), str) and re.fullmatch(r'[A-Za-z0-9_.-]{1,160}', capsule['gate_invocation']), 'gate invocation identity missing')
    status = stage['status']
    require(status in {'not_run', 'passed', 'failed', 'environment_blocked'}, 'invalid rehearsal stage status')
    later = capsule.get('live_provider_qualification', {}).get('status', 'not_run') != 'not_run' or capsule.get('official_v11', {}).get('status', 'not_run') != 'not_run'
    if status == 'not_run':
        require(stage == not_run_stage() and not later and capsule.get('phase_8_ready') is not True,
            'not-run rehearsal contains later-stage evidence')
        return False
    require(type(stage['invocation_count']) is int and stage['invocation_count'] == 1,
        'rehearsal must execute exactly once')
    execution = stage['execution']
    require(isinstance(execution, dict) and set(execution) == {'exit_code', 'wrapper_exit_code', 'termination', 'spawn_error', 'duration_ms', 'started_at', 'ended_at', 'stdout', 'stderr'},
        'rehearsal execution evidence is missing')
    require(capsule.get('final_aggregate', {}).get('status') == 'succeeded'
        and capsule.get('contract_coverage_execution', {}).get('status') == 'passed',
        'rehearsal ran before passed Final/mapped tests')
    require(type(execution['spawn_error']) is bool and isinstance(execution['duration_ms'], (int, float))
        and execution['duration_ms'] >= 0, 'invalid rehearsal execution observations')
    from datetime import datetime
    require(datetime.fromisoformat(execution['ended_at']) >= datetime.fromisoformat(execution['started_at']),
        'rehearsal execution timestamps reversed')
    for name in ('stdout', 'stderr'):
        stream = execution[name]
        require(isinstance(stream, dict) and set(stream) == {'bytes', 'sha256'}
            and type(stream['bytes']) is int and stream['bytes'] >= 0
            and isinstance(stream['sha256'], str) and re.fullmatch(r'[0-9a-f]{64}', stream['sha256']),
            'missing rehearsal execution stream identity')
    if status != 'passed':
        require((status == 'environment_blocked') is execution['spawn_error'], 'rehearsal failure classification changed')
        require(not later and capsule.get('phase_8_ready') is not True
            and capsule.get('blocking_classification') in {'dogfood_rehearsal_failed', 'dogfood_rehearsal_environment_blocked'},
            'failed rehearsal cannot permit external stages/readiness')
        require((stage['result'] is None) == (stage['result_sha256'] is None), 'failed result presence changed')
        if stage['result'] is not None:
            validate_retained_result(stage['result'], capsule['validated_candidate_head'], expected)
            require(stage['result_sha256'] == hashlib.sha256((json.dumps(stage['result'], indent=2, sort_keys=True) + '\n').encode()).hexdigest(), 'failed result hash changed')
        require(stage['result_sha256'] is None or re.fullmatch(r'[0-9a-f]{64}', str(stage['result_sha256'])),
            'invalid failed rehearsal result identity')
        return False
    require(type(execution['exit_code']) is int and execution['exit_code'] == 0
        and type(execution['wrapper_exit_code']) is int and execution['wrapper_exit_code'] == 0
        and execution['termination'] is None and execution['spawn_error'] is False,
        'passed rehearsal has unsuccessful execution')
    result = stage['result']
    validate_result(result, capsule['validated_candidate_head'], expected_identities=expected)
    require(stage['result_sha256'] == hashlib.sha256((json.dumps(result, indent=2, sort_keys=True) + '\n').encode()).hexdigest(),
        'retained rehearsal result hash changed')
    binding = result['gate_binding']
    require(isinstance(binding, dict) and set(binding) == {'gate_invocation', 'final_sha256', 'binding_sha256'}
        and isinstance(binding['gate_invocation'], str) and re.fullmatch(r'[A-Za-z0-9_.-]{1,160}', binding['gate_invocation'])
        and binding['gate_invocation'] == capsule['gate_invocation']
        and binding['final_sha256'] == capsule.get('final_summary_sha256')
        and re.fullmatch(r'[0-9a-f]{64}', str(binding['binding_sha256'])), 'rehearsal gate-parent binding missing')
    return True
