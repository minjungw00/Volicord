"""Finite machine certainty/disposition policy; no review or passage authority.

Existing determinate failed checks remain blocking pending the later heuristic
capacity audit. Only explicit uncertainty/missing-observation bases are unresolved.
"""
from enum import StrEnum
import hashlib
import json
import re


class Status(StrEnum):
    PASS = "confirmed_pass"
    VIOLATION = "confirmed_violation"
    INDETERMINATE = "indeterminate"
    NOT_OBSERVED = "not_observed"
    NOT_APPLICABLE = "not_applicable"


class Disposition(StrEnum):
    HARD = "hard_blocking"
    REVIEW = "qualitative_review_required"
    ADVISORY = "advisory"


POLICY_VERSION = 1
# Integrity uncertainty cannot admit evidence either. Review cannot waive it.
INTEGRITY_RULES = frozenset({
    "candidate_binding", "campaign_inventory", "session_mapping", "activation_identity",
    "raw_hash", "destination_collision", "realization_binding", "project_binding",
    "privacy_credential_integrity",
})
BEHAVIOR_RULES = frozenset({
    "repository_scoped_activation", "naturalistic_prompt_integrity", "plain_task_goal_linkage",
    "grounded_pre_work_repository_baseline", "engineering_choice_discovery",
    "pre_write_materiality_work_authority", "learning_participation", "learning_deliberation_order",
    "learning_not_canonical_decision", "learning_interruption_precision", "behavior_classification",
    "appropriate_inquiry_outcome", "unnecessary_question_repetition", "hidden_material_discovery_order",
    "recorded_user_owned_authority", "meaningful_ordinary_changes", "source_grounded_checkpoint",
    "decision_provenance_when_required", "distinct_work_and_resume_invocations",
    "fresh_resume_without_prior_context", "repository_bound_project_resolution",
    "recall_precedes_inspection_and_continuation", "resume_pre_work_repository_baseline",
    "resume_materiality_work_authority", "recall_matches_checkpoint_decision_and_context",
    "learning_recall_continuity", "resolved_material_question_not_reasked",
    "meaningful_recalled_continuation", "canonical_bundle_and_provenance",
    "generated_document_outputs", "static_viewer_snapshot", "bounded_runtime_and_activation_evidence",
    "work_turn_lifecycle", "resume_contract", "evaluation_execution",
})


def disposition(rule, status):
    status = Status(status)
    if rule not in INTEGRITY_RULES | BEHAVIOR_RULES:
        raise ValueError("unregistered machine policy rule")
    if status in {Status.PASS, Status.NOT_APPLICABLE}:
        if rule in INTEGRITY_RULES and status == Status.NOT_APPLICABLE:
            raise ValueError("required integrity cannot be not applicable")
        return Disposition.ADVISORY
    if rule in INTEGRITY_RULES or status == Status.VIOLATION:
        return Disposition.HARD
    return Disposition.REVIEW


def finding(rule, status, basis):
    if not isinstance(basis, dict) or not basis:
        raise ValueError("machine finding requires inspectable basis")
    value = {"check": rule, "status": Status(status).value,
        "disposition": disposition(rule, status).value,
        "policy_owner": "machine_findings", "policy_version": POLICY_VERSION, "basis": basis}
    validate_finding(value)
    return value


def validate_finding(value):
    if not isinstance(value, dict) or set(value) != {
        "check", "status", "disposition", "policy_owner", "policy_version", "basis"}:
        raise ValueError("invalid machine finding shape")
    if (value["policy_owner"] != "machine_findings" or value["policy_version"] != POLICY_VERSION
        or value["disposition"] != disposition(value["check"], value["status"])
        or not isinstance(value["basis"], dict) or not value["basis"]):
        raise ValueError("inconsistent machine finding policy or basis")


def from_observation(observation):
    """Keep original check status and full observation in the enclosing result."""
    findings = []
    work = observation.get("work_evidence_basis", {})
    continuation = observation.get("continuation_basis", {})
    for check, observed in sorted(observation["checks"].items()):
        status = {"passed": Status.PASS, "failed": Status.VIOLATION,
            "partial": Status.INDETERMINATE, "skipped": Status.NOT_OBSERVED}.get(observed)
        if status is None:
            raise ValueError("unknown underlying machine check status")
        reason = "existing_determinate_check" if observed in {"passed", "failed"} else "incomplete_observation"
        if not observation.get("capture_sha256", {}).get("work"):
            status, reason = Status.NOT_OBSERVED, "required_capture_not_observed"
        elif check == "hidden_material_discovery_order" and work.get("hidden_investigation_state") == "indeterminate":
            status, reason = Status.INDETERMINATE, "hidden_investigation_indeterminate"
        elif check in {"pre_write_materiality_work_authority", "appropriate_inquiry_outcome",
                       "recorded_user_owned_authority", "meaningful_ordinary_changes"} and work.get("exploration_state") == "indeterminate":
            status, reason = Status.INDETERMINATE, "exploration_indeterminate"
        elif check == "meaningful_recalled_continuation" and continuation.get("failure_basis") == "terminal_validation_indeterminate":
            status, reason = Status.INDETERMINATE, "terminal_validation_indeterminate"
        elif check == "engineering_choice_discovery" and observed == "failed":
            status, reason = Status.NOT_OBSERVED, "discovery_identity_not_observed"
        if check.startswith("learning_") and observation.get("behavior_class") not in {
            "learning_deliberation", "learning_routine_control"} and observed == "passed":
            status, reason = Status.NOT_APPLICABLE, "learning_not_required_for_behavior_class"
        findings.append(finding(check, status, {"observed_check_status": observed,
            "reason": reason, "observation_pointer": "/observation"}))
    return findings


def evaluation_state(findings):
    for value in findings:
        validate_finding(value)
    if any(v["disposition"] == Disposition.HARD for v in findings):
        return "hard_blocked"
    if any(v["disposition"] == Disposition.REVIEW for v in findings):
        return "review_required"
    return "observations_complete"


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def validate_run(value):
    if (value.get("kind") != "dogfood_machine_evaluation" or value.get("schema_version") != 2
        or value.get("qualification_state") != "not_run"
        or value.get("collection_state") != "collected"
        or value.get("policy_version") != POLICY_VERSION
        or value.get("evaluation_state") != "produced"
        or not isinstance(value.get("cycles"), list) or len(value["cycles"]) != 8):
        raise ValueError("invalid evaluation lifecycle")
    reference = value.get("evidence_set", {})
    if (reference.get("path") != "evidence-set.json"
        or not re.fullmatch(r"[0-9a-f]{64}", str(reference.get("sha256", "")))
        or not re.fullmatch(r"[0-9a-f]{40}", str(value.get("candidate_head", "")))
        or not re.fullmatch(r"[0-9a-f]{40}", str(value.get("evaluator_revision", "")))
        or not re.fullmatch(r"[0-9a-f]{32}", str(value.get("run_nonce", "")))):
        raise ValueError("invalid evaluation evidence/candidate binding")
    from evaluation_runs import policy_identity
    if (value.get("policy") != policy_identity() or value.get("qualitative_review_runs") != []
        or not isinstance(value.get("evaluator_files"), dict)):
        raise ValueError("evaluation policy or review identity mismatch")
    prior = value.get("previous_evaluation")
    if prior is not None and (not isinstance(prior, dict) or set(prior) != {"run_id", "sha256"}
        or any(not re.fullmatch(r"[0-9a-f]{64}", str(v)) for v in prior.values())):
        raise ValueError("invalid prior evaluation identity")
    expected_cycles = {(kind, cycle) for kind, count in
        (("volicord", 3), ("small-python", 3), ("polyglot-medium", 2)) for cycle in range(1, count + 1)}
    if {(c.get("repository_class"), c.get("cycle")) for c in value["cycles"]} != expected_cycles:
        raise ValueError("evaluation cycle coverage changed")
    findings = []
    for cycle in value["cycles"]:
        if set(cycle["observation"].get("checks", {})) != BEHAVIOR_RULES - {
            "work_turn_lifecycle", "resume_contract", "evaluation_execution"}:
            raise ValueError("evaluation silently omitted required checks")
        if cycle["findings"] != from_observation(cycle["observation"]):
            raise ValueError("findings do not preserve observed certainty/basis")
        findings.extend(cycle["findings"])
    if value.get("finding_state") != evaluation_state(findings):
        raise ValueError("evaluation disposition disagrees with findings")
    if value.get("run_id") != digest({k: v for k, v in value.items() if k != "run_id"}):
        raise ValueError("evaluation run identity changed")
