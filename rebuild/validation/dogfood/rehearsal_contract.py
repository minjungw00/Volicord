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
    "interaction_diagnostics.py", "../shared/recorded_action_evidence.py")


def identities():
    return {"contract": CONTRACT, "fixture_sha256": sha256(FIXTURE),
        "producer_sha256": {name: sha256(Path(__file__).parent / name)
            for name in PRODUCER_FILES}}


def validate_result(value, candidate, *, expected_identities=None):
    """Same predicate for gate consumption and independent bounded verification."""
    expected = expected_identities or identities()
    require(set(value) == {"kind", "candidate_head", "evidence_purpose", "contract", "fixture_sha256",
        "producer_sha256", "status", "teardown", "external_transmission", "operator_approval",
        "executables", "pipeline", "processes", "result_id"}, "unexpected retained rehearsal content")
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
    pipeline = value["pipeline"]
    require(set(pipeline) == {"evidence_set_sha256", "evaluation_run_id", "qualification_run_id",
        "expected_inner_verdict", "technical_evidence", "human_observations", "unresolved_criteria_count",
        "hard_findings", "copied_lineage_id", "copied_verification", "resource_sample_count", "topology",
        "measured_evidence_eligible", "controls"}, "unexpected pipeline content")
    for key in ("evidence_set_sha256", "evaluation_run_id", "qualification_run_id", "copied_lineage_id"):
        require(re.fullmatch(r"[0-9a-f]{64}", pipeline[key]), "invalid pipeline identity")
    require(pipeline["expected_inner_verdict"] == json.loads(FIXTURE.read_bytes())["expected_inner"]["replacement_qualification"]
        and pipeline["technical_evidence"] == "not_provided" and pipeline["human_observations"] == "not_provided"
        and pipeline["unresolved_criteria_count"] > 0 and pipeline["copied_verification"] == "verified"
        and pipeline["resource_sample_count"] > 0 and pipeline["measured_evidence_eligible"] is False
        and pipeline["topology"] == json.loads(Path(__file__).with_name("evaluation.json").read_bytes())["qualification_policy"]["campaign_topology"]
        and pipeline["controls"] == dict.fromkeys(json.loads(FIXTURE.read_bytes())["controls"], "passed"),
        "rehearsal pipeline outcome inconsistent")
    records = value.get("processes")
    require(isinstance(records, list) and len(records) >= 16 and len(records) <= 200
        and len({r["identity"] for r in records}) == len(records)
        and all(r["exit_code"] == 0 and r["termination"] == "exited" and r["duration_ns"] >= 0
            for r in records), "missing/failed rehearsal process evidence")
    for record in records:
        require(set(record) == {"identity", "exit_code", "termination", "duration_ns", "stdout", "stderr"}
            and re.fullmatch(r"[a-z0-9-]{1,80}", record["identity"]), "unexpected process content")
        for stream in ("stdout", "stderr"):
            require(set(record[stream]) == {"bytes", "sha256"}
                and type(record[stream]["bytes"]) is int and record[stream]["bytes"] >= 0
                and re.fullmatch(r"[0-9a-f]{64}", record[stream]["sha256"]), "invalid process stream evidence")
    return value
