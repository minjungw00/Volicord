"""Evaluator-blind discovery, critique, and adjudication rules.

These checks establish provenance and an explicit decision for every proposal.
They cannot establish semantic completeness; post-reveal coverage still does that.
"""

from __future__ import annotations

import copy
import re
from typing import Any

import blind_dimensions
import harness


PROPOSAL_KINDS = {
    "missing_scope", "split_scope", "duplicate_scope", "reclassification",
    "authority", "applicability", "implementation_detail_inflation",
}
DISPOSITIONS = {"accept", "reject", "merge_equivalent", "partially_accept"}
IDENTITY = re.compile(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}\Z")
PROPOSAL_FIELDS = {
    "proposal_id", "proposal_kind", "target_dimension_ids", "outcome_scope",
    "basis", "provenance_reference_indices", "proposed_assessment",
}
DISPOSITION_FIELDS = {
    "proposal_id", "disposition", "basis", "provenance_reference_indices",
    "final_dimension_ids", "accepted_scope", "rejected_scope",
}
LINEAGE_FIELDS = {
    "dimension_id", "discovery_dimension_ids", "critic_proposal_ids",
    "relationship",
}


PRIVATE_KEYS = {"materiality_obligations", "evaluation_basis", "counterfactual_review",
    "evaluator_classification", "evaluator_recommendation", "qualification_profile",
    "qualification_profile_truth", "slot_to_behavior_mapping", "hidden_repository_classes",
    "behavior_histogram", "expected_question", "expected_decision", "repository_class",
    "journey_id", "work_slot_id", "work_label", "logical_cycle"}
PRIVATE_MARKERS = ("evaluator/qualification-profile", "evaluator/slot-mapping",
                   "evaluator/descriptors", "qualification_profile_truth")


def private_content_errors(value: Any) -> list[str]:
    if isinstance(value, dict):
        if set(value).intersection(PRIVATE_KEYS):
            return ["blind artifact contains evaluator or qualification-profile material"]
        return [error for item in value.values() for error in private_content_errors(item)]
    if isinstance(value, list):
        return [error for item in value for error in private_content_errors(item)]
    if isinstance(value, str) and any(marker in value.casefold() for marker in PRIVATE_MARKERS):
        return ["blind artifact references evaluator or qualification-profile material"]
    return []


def discovery_errors(value: Any, preparation: dict[str, Any], references: int) -> list[str]:
    if not isinstance(value, dict) or set(value) != set(harness.provisional_review_contract()["required_fields"]) - {"adjudication"}:
        return ["discovery requires the complete blind assessment fields"]
    if value.get("kind") != "phase8_blind_discovery" or value.get("reviewer_role") != "primary_blind_discovery":
        return ["discovery kind or role is invalid"]
    normalized = dict(value, kind="phase8_provisional_behavior_review",
                      reviewer_role="campaign_preparation_independent_reviewer")
    return private_content_errors(value) + harness.blind_first_review_errors(preparation, normalized, references)


def critique_errors(value: Any, slot: str, preparation_sha: str,
                    discovery_sha: str, critic_run_id: str, discovery: dict[str, Any], references: int) -> list[str]:
    if not isinstance(value, dict) or set(value) != {
        "kind", "review_slot_id", "preparation_sha256", "discovery_sha256", "critic_run_id", "proposals"
    } or value.get("critic_run_id") != critic_run_id or value.get("kind") != "phase8_blind_completeness_critique" or value.get("review_slot_id") != slot \
            or value.get("preparation_sha256") != preparation_sha or value.get("discovery_sha256") != discovery_sha:
        return ["critique identity or immutable discovery binding is invalid"]
    proposals = value.get("proposals")
    if not isinstance(proposals, list) or len(proposals) > blind_dimensions.MAX_DIMENSIONS:
        return ["critique proposals must be a bounded collection"]
    discovered = {item["dimension_id"] for item in discovery["assessments"]}
    errors, identities = private_content_errors(value), set()
    for proposal in proposals:
        if not isinstance(proposal, dict) or set(proposal) != PROPOSAL_FIELDS:
            errors.append("critic proposal fields are invalid")
            continue
        identity = proposal["proposal_id"]
        if not isinstance(identity, str) or not IDENTITY.fullmatch(identity) or identity in identities:
            errors.append("critic proposal identity is invalid or duplicated")
        else:
            identities.add(identity)
        kind = proposal["proposal_kind"]
        targets = proposal["target_dimension_ids"]
        if not isinstance(kind, str) or kind not in PROPOSAL_KINDS or not isinstance(targets, list) or any(not isinstance(target, str) for target in targets) or len(targets) != len(set(targets)) \
                or any(target not in discovered for target in targets) or (kind != "missing_scope" and not targets):
            errors.append("critic proposal kind or target is invalid")
        if not blind_dimensions.bounded(proposal["outcome_scope"], harness.MAX_REVIEW_TEXT_BYTES) \
                or not blind_dimensions.bounded(proposal["basis"], harness.MAX_REVIEW_TEXT_BYTES) \
                or not blind_dimensions.references_valid(proposal["provenance_reference_indices"], references):
            errors.append("critic proposal needs bounded reviewer-visible reasoning and provenance")
        assessment = proposal["proposed_assessment"]
        if assessment is not None:
            errors.extend(blind_dimensions.assessment_errors(
                [assessment], harness.provisional_review_contract()["classification_rules"],
                references, harness.MAX_REVIEW_TEXT_BYTES))
    return errors


def adjudication_errors(final: Any, discovery: dict[str, Any], critique: dict[str, Any],
                        discovery_sha: str, critique_sha: str, adjudicator_run_id: str, references: int) -> list[str]:
    if not isinstance(final, dict) or not isinstance(final.get("adjudication"), dict):
        return ["final provisional requires blind adjudication"]
    adjudication = final["adjudication"]
    if set(adjudication) != {"discovery_sha256", "critique_sha256", "adjudicator_run_id", "dispositions", "lineage"} \
            or adjudication.get("discovery_sha256") != discovery_sha \
            or adjudication.get("critique_sha256") != critique_sha \
            or adjudication.get("adjudicator_run_id") != adjudicator_run_id:
        return ["adjudication predecessor hash binding is invalid"]
    dispositions, lineage = adjudication.get("dispositions"), adjudication.get("lineage")
    if not isinstance(dispositions, list) or len(dispositions) != len(critique["proposals"]) \
            or not isinstance(lineage, list) or not 1 <= len(lineage) <= blind_dimensions.MAX_DIMENSIONS:
        return ["adjudication requires bounded decisions and final lineage"]
    proposals = {item["proposal_id"]: item for item in critique["proposals"]}
    original = {item["dimension_id"]: item for item in discovery["assessments"]}
    final_items = {item["dimension_id"]: item for item in final.get("assessments", [])
                   if isinstance(item, dict) and isinstance(item.get("dimension_id"), str)}
    errors, decisions = private_content_errors(final), {}
    for row in dispositions:
        if not isinstance(row, dict) or set(row) != DISPOSITION_FIELDS:
            errors.append("adjudication disposition fields are invalid")
            continue
        identity = row["proposal_id"]
        if not isinstance(identity, str) or identity not in proposals or identity in decisions \
                or not isinstance(row["disposition"], str) or row["disposition"] not in DISPOSITIONS:
            errors.append("adjudication proposal decision is missing, duplicated, or invalid")
            continue
        decisions[identity] = row
        destinations = row["final_dimension_ids"]
        if not isinstance(destinations, list) or len(destinations) > blind_dimensions.MAX_DIMENSIONS or any(not isinstance(item, str) for item in destinations) or len(destinations) != len(set(destinations)) \
                or any(item not in final_items for item in destinations):
            errors.append("adjudication disposition references unknown final dimensions")
        if row["disposition"] == "reject" and destinations:
            errors.append("rejected critic proposal cannot enter final assessments")
        if row["disposition"] != "reject" and not destinations:
            errors.append("supported critic proposal requires explicit final disposition lineage")
        if row["disposition"] == "partially_accept":
            if not blind_dimensions.bounded(row["accepted_scope"], harness.MAX_REVIEW_TEXT_BYTES) or \
                    not blind_dimensions.bounded(row["rejected_scope"], harness.MAX_REVIEW_TEXT_BYTES):
                errors.append("partial acceptance must separate supported and unsupported scopes")
        elif row["accepted_scope"] is not None or row["rejected_scope"] is not None:
            errors.append("non-partial disposition must not claim a partial scope")
        if not blind_dimensions.bounded(row["basis"], harness.MAX_REVIEW_TEXT_BYTES) \
                or not blind_dimensions.references_valid(row["provenance_reference_indices"], references):
            errors.append("adjudication disposition needs reviewer-visible evidence")
    if set(decisions) != set(proposals):
        errors.append("every critic proposal requires one explicit adjudication")
    seen = set()
    for row in lineage:
        if not isinstance(row, dict) or set(row) != LINEAGE_FIELDS:
            errors.append("final dimension lineage fields are invalid")
            continue
        identity = row["dimension_id"]
        if not isinstance(identity, str) or identity not in final_items or identity in seen:
            errors.append("final dimension lineage is missing or duplicated")
            continue
        seen.add(identity)
        sources, critics = row["discovery_dimension_ids"], row["critic_proposal_ids"]
        if not isinstance(sources, list) or not isinstance(critics, list) \
                or len(sources) + len(critics) > blind_dimensions.MAX_DIMENSIONS \
                or any(not isinstance(item, str) for item in sources + critics) \
                or len(sources) != len(set(sources)) or len(critics) != len(set(critics)) \
                or any(item not in original for item in sources) \
                or any(item not in proposals for item in critics) or not (sources or critics):
            errors.append("final dimension has invalid predecessor lineage")
            continue
        if any(critic not in decisions or decisions[critic]["disposition"] == "reject"
               or identity not in decisions[critic]["final_dimension_ids"] for critic in critics):
            errors.append("final dimension cites a critic proposal without accepted disposition")
        relationship = row["relationship"]
        if not isinstance(relationship, str) or relationship not in {"discovery", "accepted_critic", "merged_equivalent"} \
                or (relationship == "discovery" and (critics or len(sources) != 1)) \
                or (relationship == "accepted_critic" and not critics) \
                or (relationship == "merged_equivalent" and len(sources) + len(critics) < 2):
            errors.append("final dimension relationship is inconsistent")
        if not critics and (len(sources) != 1 or final_items[identity] != original[sources[0]]):
            errors.append("unchallenged discovery dimension was changed without adjudication")
    if seen != set(final_items):
        errors.append("every final assessment requires inspectable lineage")
    if errors:
        return errors
    for identity, row in decisions.items():
        if row["disposition"] != "reject" and not all(
            identity in line.get("critic_proposal_ids", []) for line in lineage
            if line.get("dimension_id") in row["final_dimension_ids"]
        ):
            errors.append("accepted critic proposal lacks final lineage")
        if row["disposition"] == "merge_equivalent" and any(
            not any(line.get("dimension_id") == destination
                and line.get("relationship") == "merged_equivalent"
                and any(final_items[destination] == original[source]
                    for source in line["discovery_dimension_ids"])
                for line in lineage)
            for destination in row["final_dimension_ids"]
        ):
            errors.append("equivalent critic concern must merge into an unchanged discovery dimension")
    for identity in original:
        if not any(identity in line.get("discovery_dimension_ids", []) for line in lineage) \
                and not any(row["disposition"] in {"accept", "partially_accept"}
                            and identity in proposals[proposal_id]["target_dimension_ids"]
                            for proposal_id, row in decisions.items()):
            errors.append("discovery dimension disappeared without accepted critic challenge")
    return errors


def discovery_from_provisional(value: dict[str, Any]) -> dict[str, Any]:
    result = copy.deepcopy(value)
    result["kind"] = "phase8_blind_discovery"
    result["reviewer_role"] = "primary_blind_discovery"
    result.pop("adjudication", None)
    return result
