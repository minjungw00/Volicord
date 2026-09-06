"""Bounded human judgments about material authority; never a text classifier.

Machine evidence establishes provenance and chronology of recorded operations.
A reviewer must establish whether that authority actually covers the observable
implementation commitment. The initial evaluator challenge is rebuttable.
"""
from __future__ import annotations

import hashlib
from typing import Any

MAX_TEXT_BYTES = 8192
RESOLUTION_AUTHORITIES = {
    "user_decision": "current_user_decision",
    "applicable_prior_authority": "applicable_prior_authority",
    "exact_delegation": "exact_delegation",
    "repository_or_contract_settlement": "repository_or_contract_settlement",
    "avoidance": "no_production_commitment",
    "defer": "no_production_commitment",
    "prototype": "no_production_commitment",
    "silent_commitment": "absent",
    "unresolved": "uncertain",
}
ASSESSMENT_FIELDS = {
    "material_outcome", "observable_implementation_commitment", "commitment_state",
    "resolution_path", "authority_kind", "authority_basis", "authority_relation_to_outcome",
    "chronology", "evidence",
}


def bounded_text(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip()) and len(value.encode("utf-8")) <= MAX_TEXT_BYTES


def assessment_contract() -> dict[str, Any]:
    return {
        "required_fields": sorted(ASSESSMENT_FIELDS),
        "resolution_authorities": RESOLUTION_AUTHORITIES,
        "commitment_states": ["production_committed", "no_production_commitment", "uncertain"],
        "authority_relations": ["resolves_this_outcome", "concern_disproved", "no_commitment", "does_not_resolve_this_outcome", "uncertain"],
        "chronology": ["prospective", "late", "not_applicable", "uncertain"],
        "evidence_fields": ["evidence_id", "locator"],
        "maximum_text_utf8_bytes": MAX_TEXT_BYTES,
        "semantic_judgment_owner": "bounded_campaign_human_review",
        "instruction": "Inspect the actual commitment and cited authority for this outcome. A different Question, trivial ceremony, recommendation or implementation preference is not its authority. Rebut the initial concern with stronger evidence, or record avoidance/defer/prototype without production commitment. Resolve initial_concern_reference against the private post-session descriptor and verify its SHA-256; review all other actual-work outcomes as well. Use additional_outcomes for further independent outcomes and coverage_basis to explain complete coverage of actual work, including tests, documents and other coupled artifacts. Evidence locators name exact call/turn, Decision revision, file/line or diff hunk in the immutable evidence index. No Question wording, answer, count or similarity is an oracle.",
    }


def review_basis(evaluation: dict[str, Any], behavior_review: dict[str, Any], captures: dict[str, Any], *, changed_paths: list[str], decision_ids: list[str], materiality: dict[str, Any]) -> dict[str, Any]:
    # Sanitized machine results retain private descriptor pointers, never the
    # evaluator wording. The post-session review package carries the descriptor.
    material_concerns = evaluation.get("possible_material_concerns")
    material_concerns = material_concerns if isinstance(material_concerns, list) else []
    concerns = [{"descriptor_field": f"evaluation_basis.possible_material_concerns[{index}]",
                 "sha256": hashlib.sha256(concern.encode("utf-8")).hexdigest()}
                for index, concern in enumerate(material_concerns) if bounded_text(concern)]
    independent = behavior_review.get("independent_review")
    counterfactual = independent.get("counterfactual_review") if isinstance(independent, dict) else None
    counterfactual = counterfactual if isinstance(counterfactual, dict) else {}
    specific = counterfactual.get("specific_unresolved_outcome")
    if bounded_text(specific):
        concerns.insert(0, {"descriptor_field": "behavior_review.independent_review.counterfactual_review.specific_unresolved_outcome",
                           "sha256": hashlib.sha256(specific.encode("utf-8")).hexdigest()})
    concerns.append({"scope": "all_other_material_outcomes_in_actual_work"})
    evidence_index = {
        name: {"kind": name, "sha256": digest}
        for name, digest in captures.items() if isinstance(digest, str)
    }
    references = behavior_review.get("provenance_references")
    references = references if isinstance(references, list) else []
    for index, reference in enumerate(references):
        if not isinstance(reference, dict):
            continue
        evidence_index[f"initial_authority_{index}"] = dict(reference)
    return {
        "state": "requires_bounded_human_review",
        "initial_challenge_is_rebuttable": True,
        "machine_proves_semantic_authority": False,
        "obligations": [{"obligation_id": f"material-outcome-{index + 1}", "initial_concern_reference": concern}
            for index, concern in enumerate(concerns)],
        "evidence_index": evidence_index,
        "implementation_observations": {"changed_paths": changed_paths, "decision_ids": decision_ids,
            "materiality_review_candidate_id": materiality.get("review_candidate_id"),
            "dimension_ids": materiality.get("dimension_ids", []),
            "pre_work_readiness": materiality.get("pre_work_readiness", {})},
    }


def review_template(sample: dict[str, Any], basis: dict[str, Any]) -> dict[str, Any]:
    return {"sample": sample, "review_basis": basis,
        "obligations": [{"obligation_id": obligation["obligation_id"], "assessment": None}
            for obligation in basis["obligations"]],
        "additional_outcomes": [], "coverage_basis": None}


def assess(value: Any, evidence_index: dict[str, Any]) -> str:
    if value is None:
        return "not_provided"
    if not isinstance(value, dict) or set(value) != ASSESSMENT_FIELDS:
        raise ValueError("authority obligation requires the closed bounded assessment shape")
    if any(not bounded_text(value[field]) for field in (
        "material_outcome", "observable_implementation_commitment", "authority_basis")):
        raise ValueError("authority obligation requires concrete outcome, commitment and bounded authority reasoning")
    path = value["resolution_path"]
    if not isinstance(path, str) or path not in RESOLUTION_AUTHORITIES or value["authority_kind"] != RESOLUTION_AUTHORITIES[path]:
        raise ValueError("authority kind must describe the actual resolution path")
    if value["commitment_state"] not in assessment_contract()["commitment_states"] or value["authority_relation_to_outcome"] not in assessment_contract()["authority_relations"] or value["chronology"] not in assessment_contract()["chronology"]:
        raise ValueError("unsupported authority obligation disposition")
    evidence = value["evidence"]
    if not isinstance(evidence, list) or not evidence or len(evidence) > 64:
        raise ValueError("authority obligation requires bounded inspectable evidence")
    ids = set()
    for reference in evidence:
        if not isinstance(reference, dict) or set(reference) != {"evidence_id", "locator"} or not isinstance(reference["evidence_id"], str) or reference["evidence_id"] not in evidence_index or not bounded_text(reference["locator"]):
            raise ValueError("authority evidence must locate a fact in the candidate-bound evidence index")
        ids.add(reference["evidence_id"])
    # Every disposition must inspect actual execution. Initial reviewer concern
    # or a canonical Decision alone says nothing about implementation fidelity.
    if "work_capture" not in ids:
        raise ValueError("authority obligation must inspect the actual work capture")
    if path in {"user_decision", "applicable_prior_authority"} and "canonical_bundle" not in ids:
        raise ValueError("Decision/prior authority requires canonical evidence as well as actual work")
    if path == "repository_or_contract_settlement" and not ("canonical_bundle" in ids or any(name.startswith("initial_authority_") for name in ids)):
        raise ValueError("repository/contract settlement requires inspectable Source or owner evidence")
    if path in {"silent_commitment", "unresolved"}:
        return "failed"
    if path in {"avoidance", "defer", "prototype"}:
        return "passed" if (
            value["commitment_state"] == "no_production_commitment"
            and value["authority_relation_to_outcome"] == "no_commitment"
            and value["chronology"] == "not_applicable"
        ) else "failed"
    return "passed" if (
        value["commitment_state"] == "production_committed"
        and value["authority_relation_to_outcome"] in {"resolves_this_outcome", "concern_disproved"}
        and value["chronology"] == "prospective"
    ) else "failed"


def validate_reviews(reviews: Any, expected: list[dict[str, Any]]) -> list[str]:
    if not isinstance(reviews, list) or len(reviews) != len(expected):
        raise ValueError("human review must account for every material authority obligation in every cycle")
    states = []
    for review, template in zip(reviews, expected):
        if not isinstance(review, dict) or set(review) != {"sample", "review_basis", "obligations", "additional_outcomes", "coverage_basis"} or review["sample"] != template["sample"] or review["review_basis"] != template["review_basis"]:
            raise ValueError("authority obligation review must retain its immutable candidate/cycle evidence basis")
        obligations = review["obligations"]
        if not isinstance(obligations, list) or any(not isinstance(item, dict) or set(item) != {"obligation_id", "assessment"} for item in obligations):
            raise ValueError("invalid authority obligation list")
        if [item["obligation_id"] for item in obligations] != [item["obligation_id"] for item in template["obligations"]]:
            raise ValueError("authority obligations cannot be dropped, duplicated or replaced by another Question")
        additional = review["additional_outcomes"]
        if not isinstance(additional, list) or len(additional) > 64:
            raise ValueError("additional material outcomes require bounded individual assessments")
        coverage = review["coverage_basis"]
        if coverage is not None and not bounded_text(coverage):
            raise ValueError("coverage basis must explain how all actual material outcomes were accounted for")
        states.append("not_provided" if coverage is None else "passed")
        states.extend(assess(item["assessment"], review["review_basis"]["evidence_index"]) for item in obligations)
        states.extend(assess(item, review["review_basis"]["evidence_index"]) for item in additional)
    return states
