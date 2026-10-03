"""Finite authority policy over preserved observations; no automatic semantic oracle."""
from enum import StrEnum
import hashlib
import json
import re
from pathlib import Path


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


POLICY = json.loads(Path(__file__).with_name("machine-policy.json").read_text())
POLICY_VERSION = POLICY["revision"]
FACT_RULES = frozenset({"shared_answer_integrity", "recorded_decision_integrity", "measured_project_identity",
    "measured_session_provenance", "required_validation_execution", "procedure_invocation_counts", "projection_evidence_identity"})
GIT_RULES = frozenset({"git_history_observation"})
# Integrity uncertainty cannot admit evidence either. Review cannot waive it.
INTEGRITY_RULES = frozenset({
    "candidate_binding", "campaign_inventory", "session_mapping", "activation_identity",
    "raw_hash", "destination_collision", "realization_binding", "project_binding",
    "privacy_credential_integrity",
    "journey_project_identity", "journey_work_identity", "journey_work_history",
    "journey_relation_consistency", "journey_resume_continuity", "journey_isolation",
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
RESUME_RULES = frozenset({
    "distinct_work_and_resume_invocations", "fresh_resume_without_prior_context",
    "repository_bound_project_resolution", "recall_precedes_inspection_and_continuation",
    "resume_pre_work_repository_baseline", "resume_materiality_work_authority",
    "recall_matches_checkpoint_decision_and_context", "learning_recall_continuity",
    "resolved_material_question_not_reasked", "meaningful_recalled_continuation",
})
WORK_OBSERVATION_RULES = BEHAVIOR_RULES - {
    "work_turn_lifecycle", "resume_contract", "evaluation_execution"
}
EXPECTED_WORKS = {
    ("volicord", "A"), ("volicord", "B"), ("volicord", "C"),
    ("small-python", "A"), ("polyglot-medium", "A"),
}
EXPECTED_JOURNEYS = {"volicord", "small-python", "polyglot-medium"}


def disposition(rule, status):
    status = Status(status)
    if rule not in POLICY["rules"]:
        raise ValueError("unregistered machine policy rule")
    if status in {Status.PASS, Status.NOT_APPLICABLE}:
        if rule in INTEGRITY_RULES and status == Status.NOT_APPLICABLE:
            raise ValueError("required integrity cannot be not applicable")
        return Disposition.ADVISORY
    policy = POLICY["rules"][rule]
    return Disposition(policy["authority"] if status == Status.VIOLATION else policy["uncertainty"])


def finding(rule, status, basis):
    if not isinstance(basis, dict) or not basis:
        raise ValueError("machine finding requires inspectable basis")
    value = {"check": rule, "status": Status(status).value,
        "disposition": disposition(rule, status).value,
        "policy_owner": POLICY["rules"][rule]["owner"], "policy_version": POLICY_VERSION, "basis": basis}
    validate_finding(value)
    return value


def validate_finding(value):
    if not isinstance(value, dict) or set(value) != {
        "check", "status", "disposition", "policy_owner", "policy_version", "basis"}:
        raise ValueError("invalid machine finding shape")
    if (value["policy_owner"] != POLICY["rules"].get(value["check"], {}).get("owner") or value["policy_version"] != POLICY_VERSION
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
        if check.startswith("learning_"):
            participation = observation.get("learning_basis", {}).get("participation_state")
            if participation == "active":
                status, reason = Status.INDETERMINATE, "runtime_learning_active_requires_post_hoc_review"
            elif participation == "inactive":
                status, reason = Status.NOT_OBSERVED, "runtime_learning_participation_not_active"
            else:
                status, reason = Status.INDETERMINATE, "runtime_learning_participation_uncertain"
        findings.append(finding(check, status, {"observed_check_status": observed,
            "reason": reason, "observation_pointer": "/observation"}))
    for rule, fact in sorted(observation.get("machine_facts", {}).items()):
        if rule not in FACT_RULES:
            raise ValueError("unknown audited fact rule")
        findings.append(finding(rule, fact["status"], fact["basis"]))
    return findings


def from_journey_observation(observation):
    findings = []
    for rule in sorted(INTEGRITY_RULES & {
        "journey_project_identity", "journey_work_identity", "journey_work_history",
        "journey_relation_consistency", "journey_resume_continuity", "journey_isolation",
    }):
        item = observation.get("checks", {}).get(rule)
        if not isinstance(item, dict) or set(item) != {"status", "basis"}:
            raise ValueError("journey observation omitted structural continuity evidence")
        findings.append(finding(rule, item["status"], item["basis"]))
    git_evidence = observation.get("git_evidence")
    findings.append(finding("git_history_observation",
        Status.PASS if git_evidence else Status.NOT_OBSERVED,
        {"git_evidence": git_evidence,
         "work_identity_basis": False,
         "git_policy_compliance": "post_hoc_task_and_repository_authority_review"}))
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


def validate_run(value, *, require_current_policy=True):
    if not isinstance(value, dict) or set(value) != {"kind", "schema_version", "candidate_head", "evidence_set",
        "evaluator_revision", "policy_version", "evaluator_files", "policy", "qualitative_review_runs",
        "previous_evaluation", "run_nonce", "collection_state", "evaluation_state", "qualification_state",
        "works", "journeys", "interaction_diagnostics", "coverage", "finding_state", "run_id"}:
        raise ValueError("invalid machine evaluation shape")
    if (value.get("kind") != "dogfood_machine_evaluation" or value.get("schema_version") != 4
        or value.get("qualification_state") != "not_run"
        or value.get("collection_state") != "collected"
        or value.get("policy_version") != POLICY_VERSION
        or value.get("evaluation_state") != "produced"
        or not isinstance(value.get("works"), list) or len(value["works"]) != 5
        or not isinstance(value.get("journeys"), list) or len(value["journeys"]) != 3):
        raise ValueError("invalid evaluation lifecycle")
    reference = value.get("evidence_set", {})
    if (reference.get("path") != "evidence-set.json"
        or not re.fullmatch(r"[0-9a-f]{64}", str(reference.get("sha256", "")))
        or not re.fullmatch(r"[0-9a-f]{40}", str(value.get("candidate_head", "")))
        or not re.fullmatch(r"[0-9a-f]{40}", str(value.get("evaluator_revision", "")))
        or not re.fullmatch(r"[0-9a-f]{32}", str(value.get("run_nonce", "")))):
        raise ValueError("invalid evaluation evidence/candidate binding")
    from evaluation_runs import policy_identity
    recorded_policy = value.get("policy")
    if (not isinstance(recorded_policy, dict) or set(recorded_policy) != {"revision", "sha256"}
        or recorded_policy["revision"] != policy_identity()["revision"]
        or not re.fullmatch(r"[0-9a-f]{64}", str(recorded_policy["sha256"]))
        or (require_current_policy and recorded_policy != policy_identity())
        or value.get("qualitative_review_runs") != []
        or not isinstance(value.get("evaluator_files"), dict)):
        raise ValueError("evaluation policy or review identity mismatch")
    prior = value.get("previous_evaluation")
    if prior is not None and (not isinstance(prior, dict) or set(prior) != {"run_id", "sha256"}
        or any(not re.fullmatch(r"[0-9a-f]{64}", str(v)) for v in prior.values())):
        raise ValueError("invalid prior evaluation identity")
    if {(w.get("repository_class"), w.get("work")) for w in value["works"]} != EXPECTED_WORKS:
        raise ValueError("evaluation Work coverage changed")
    if {j.get("repository_class") for j in value["journeys"]} != EXPECTED_JOURNEYS:
        raise ValueError("evaluation journey coverage changed")
    coverage = value.get("coverage")
    if not isinstance(coverage, dict) or coverage != {
        "repository_journeys": 3,
        "work_items": 5,
        "resume_pairs": 3,
        "fresh_sessions": 8,
        "work_distribution": {"volicord": 3, "small-python": 1, "polyglot-medium": 1},
        "resume_repository_classes": ["polyglot-medium", "small-python", "volicord"],
    }:
        raise ValueError("evaluation topology coverage changed")
    import interaction_diagnostics
    import workload_intents
    if {w["work_slot_id"]: w.get("workload_intent") for w in value["works"]} != workload_intents.WORKLOAD_INTENTS:
        raise ValueError("evaluation workload intent coverage changed")
    expected_diagnostics = interaction_diagnostics.campaign_summary({w["work_slot_id"]: w["observation"]["interaction_diagnostics"] for w in value["works"]})
    if value.get("interaction_diagnostics") != expected_diagnostics:
        raise ValueError("campaign diagnostics differ from Work facts")
    findings = []
    for work in value["works"]:
        expected_checks = WORK_OBSERVATION_RULES if work.get("resume_pair") else WORK_OBSERVATION_RULES - RESUME_RULES
        if set(work["observation"].get("checks", {})) != expected_checks:
            raise ValueError("evaluation silently omitted required checks")
        if set(work["observation"].get("machine_facts", {})) != FACT_RULES:
            raise ValueError("evaluation omitted audited integrity/execution facts")
        if work["findings"] != from_observation(work["observation"]):
            raise ValueError("findings do not preserve observed certainty/basis")
        findings.extend(work["findings"])
    for journey in value["journeys"]:
        if journey["findings"] != from_journey_observation(journey["observation"]):
            raise ValueError("journey findings do not preserve structural evidence")
        findings.extend(journey["findings"])
    if value.get("finding_state") != evaluation_state(findings):
        raise ValueError("evaluation disposition disagrees with findings")
    if value.get("run_id") != digest({k: v for k, v in value.items() if k != "run_id"}):
        raise ValueError("evaluation run identity changed")


def review_groups(rule):
    return set(POLICY["rules"][rule]["review_groups"])


def validate_policy():
    if set(POLICY["rules"]) != INTEGRITY_RULES | BEHAVIOR_RULES | FACT_RULES | GIT_RULES:
        raise ValueError("finite policy coverage changed")
    for value in POLICY["rules"].values():
        if (set(value) != {"authority", "uncertainty", "owner", "rationale", "review_groups"}
            or not value["owner"] or not value["rationale"]):
            raise ValueError("policy requires a rationale and owner for every rule")
        for key in ("authority", "uncertainty"):
            Disposition(value[key])
        if "qualitative_review_required" in {value["authority"], value["uncertainty"]} and not value["review_groups"]:
            raise ValueError("reviewable rule has no semantic jurisdiction")


validate_policy()


def work_findings(failed_checks, indeterminate_checks):
    return [finding(POLICY["work_observations"][check],
        Status.INDETERMINATE if check in indeterminate_checks else Status.VIOLATION,
        {"work_observation_check": check, "indeterminate": check in indeterminate_checks}) for check in failed_checks]
