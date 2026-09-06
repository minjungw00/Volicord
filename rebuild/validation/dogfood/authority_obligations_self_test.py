#!/usr/bin/env python3
"""Sanitized counterfactual review dispositions, not automated semantic truth."""
from __future__ import annotations

import copy
import json
from typing import Any

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
    return results


if __name__ == "__main__":
    print(json.dumps(self_test(), sort_keys=True))
