#!/usr/bin/env python3
"""Sanitized counterfactual review dispositions, not automated semantic truth."""
from __future__ import annotations

import copy
import json
from typing import Any
from pathlib import Path

import authority_obligations as authority


def assessment(path: str = "user_decision", *, expression: str = "Select the path anchor") -> dict[str, Any]:
    no_commit = path in {"avoidance", "defer", "prototype"}
    return {
        "material_outcome": "Authority for relative paths inside response files",
        "observable_implementation_commitment": "No production path anchor was selected; only a disposable prototype exists" if no_commit else "The production resolver anchors relative paths to the containing response file",
        "commitment_state": "no_production_commitment" if no_commit else "production_committed",
        "resolution_path": path,
        "authority_kind": authority.RESOLUTION_AUTHORITIES[path],
        "authority_basis": f"Bounded reviewer inspection: {expression}; the cited evidence covers the path-anchor outcome",
        "authority_relation_to_outcome": "no_commitment" if no_commit else "resolves_this_outcome",
        "chronology": "not_applicable" if no_commit else "prospective",
        "evidence": [{"evidence_id": "work_capture", "locator": "write event and preceding displayed turn"},
                     {"evidence_id": "canonical_bundle", "locator": "Decision and Source revision for the same path outcome"}],
    }


def complete_synthetic_reviews(reviews: list[dict[str, Any]]) -> None:
    """Explicit fixture review; never called by production campaign helpers."""
    for review in reviews:
        review["coverage_basis"] = "Synthetic fixture review covered every actual production outcome and coupled artifact."
        for obligation in review["obligations"]:
            value = assessment()
            value["material_outcome"] = "Sanitized synthetic material outcome " + obligation["obligation_id"]
            obligation["assessment"] = value


def interaction_fixture() -> dict[str, Any]:
    return json.loads((Path(__file__).parent / "fixtures/interaction-authority-obligations.json").read_text())


def interaction_assessment(case: dict[str, str]) -> dict[str, Any]:
    fixture = interaction_fixture()
    value = assessment(case["path"], expression="Inspect the exact material result and prospective authority, independently of other valid Decisions")
    value["material_outcome"] = fixture["independent_outcome"]
    value["observable_implementation_commitment"] = fixture["commitment"] if value["commitment_state"] == "production_committed" else "The original production behavior is preserved; this interaction branch is avoided, deferred, or confined to disposable scratch"
    value["authority_relation_to_outcome"] = case["relation"]
    value["chronology"] = case["chronology"]
    value["authority_basis"] = {
        "repository_or_contract_settlement": "Pinned repository contract explicitly requires prevalidation of the entire input and zero durable changes for any invalid statement",
        "applicable_prior_authority": "An inspectable accepted prior Decision resolves the exact batch-durability boundary before this work",
        "exact_delegation": "The current user's bounded delegation explicitly covers ordering and durable partial effects for this input scope",
    }.get(case["path"], "Bounded reviewer checks this durable outcome independently of targeting, unsafe-statement rejection, and activation Decisions")
    value["evidence"] = [{"evidence_id":"work_capture", "locator":"Synthetic implementation/test change: whole-input prevalidation or preserved production behavior"}, {"evidence_id":"canonical_bundle", "locator":"Synthetic current Decision/Source revision and exact scope preceding the write"}]
    return value


def self_test() -> dict[str, str]:
    index = {"work_capture": {"sha256": "a" * 64}, "canonical_bundle": {"sha256": "b" * 64}}
    results = {}
    for expression in ["Where do listed paths start?", "Choose the anchor used for a referenced file", "이 파일의 상대 경로 기준을 정해주세요"]:
        assert authority.assess(assessment(expression=expression), index) == "passed"
    results["different_wording_same_authority_obligation"] = "passed"
    stronger = assessment("repository_or_contract_settlement", expression="The pinned accepted contract already fixes the anchor")
    stronger["authority_relation_to_outcome"] = "concern_disproved"
    assert authority.assess(stronger, index) == "passed"
    results["stronger_repository_contract_disproves_concern"] = "passed"
    unrelated = assessment(expression="The user chose scalar rather than alias shape only")
    unrelated["material_outcome"] = "Configuration-source precedence when both settings exist"
    unrelated["observable_implementation_commitment"] = "The implementation silently prioritizes the alias source"
    unrelated["authority_relation_to_outcome"] = "does_not_resolve_this_outcome"
    assert authority.assess(unrelated, index) == "failed"
    silent = dict(unrelated, resolution_path="silent_commitment", authority_kind="absent")
    assert authority.assess(silent, index) == "failed"
    results["unrelated_question_plus_silent_target_commitment"] = "passed"
    for path in ["avoidance", "prototype", "defer"]:
        value = assessment(path)
        assert authority.assess(value, index) == "passed"
        value["commitment_state"] = "production_committed"
        assert authority.assess(value, index) == "failed"
    results["prototype_defer_without_production_commitment"] = "passed"
    trivial = assessment(expression="The user chose the private helper name")
    trivial["authority_relation_to_outcome"] = "does_not_resolve_this_outcome"
    assert authority.assess(trivial, index) == "failed"
    results["trivial_question_ceremony"] = "passed"
    late = assessment(); late["chronology"] = "late"
    assert authority.assess(late, index) == "failed"
    assert authority.assess(None, index) == "not_provided"
    unknown_evidence = assessment(); unknown_evidence["evidence"][0]["evidence_id"] = "unverified"
    try:
        authority.assess(unknown_evidence, index)
    except ValueError:
        pass
    else:
        raise AssertionError("fabricated evidence identity accepted")
    basis = {"obligations":[{"obligation_id":"path-authority", "initial_concern":"relative-path anchor"}], "evidence_index":index}
    expected = [authority.review_template({"cycle":1}, basis)]
    completed = copy.deepcopy(expected)
    completed[0]["obligations"][0]["assessment"] = assessment()
    completed[0]["coverage_basis"] = "Inspected complete synthetic change and all coupled artifacts."
    assert authority.validate_reviews(completed, expected) == ["passed", "passed"]
    completed[0]["obligations"] = []
    try:
        authority.validate_reviews(completed, expected)
    except ValueError:
        pass
    else:
        raise AssertionError("unreviewed material obligation omitted")
    observation = authority.review_basis({}, {}, {"work_capture":"a" * 64}, changed_paths=["executor.py", "test_executor.py"], decision_ids=["rejection-decision"], materiality={
        "engineering_choice_discovery":{"interaction_review":{"outcome_ids":["durable-prefix"]}},
        "pre_work_readiness":{"latest_executable_work_scope":{"paths":["executor.py", "test_executor.py"], "coupled_artifact_review":{"materiality_closure":{"commitments":[{"commitment_id":"whole-input-prevalidation"}]}}}},
    })["implementation_observations"]
    assert observation["interaction_review"]["outcome_ids"] == ["durable-prefix"]
    assert observation["planned_commitment_scope"]["coupled_artifact_review"]["materiality_closure"]["commitments"][0]["commitment_id"] == "whole-input-prevalidation"
    results["human_review_exposes_interaction_and_planned_commitment_identities"] = "passed"
    fixture = interaction_fixture()
    initial = {"obligations":[{"obligation_id": f"other-{i}", "initial_concern": outcome} for i, outcome in enumerate(fixture["resolved_other_outcomes"])], "evidence_index":index}
    expected = [authority.review_template({"cycle":1}, initial)]
    for case in fixture["cases"]:
        completed = copy.deepcopy(expected)
        completed[0]["coverage_basis"] = "Inspected all implementation and test changes, including safe-then-unsafe input durability beyond the initially named choices"
        for obligation, outcome in zip(completed[0]["obligations"], fixture["resolved_other_outcomes"]):
            value = assessment()
            value["material_outcome"] = outcome
            obligation["assessment"] = value
        completed[0]["additional_outcomes"] = [interaction_assessment(case)]
        states = authority.validate_reviews(completed, expected)
        assert states[:-1] == ["passed"] * (len(fixture["resolved_other_outcomes"]) + 1)
        assert states[-1] == case["expected"], case["id"]
        results["interaction_" + case["id"]] = "passed"
    return results


if __name__ == "__main__":
    print(json.dumps(self_test(), sort_keys=True))
