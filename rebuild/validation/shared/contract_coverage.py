#!/usr/bin/env python3
"""Validate cross-owner contracts against real product-entrypoint tests."""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
import re
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
DEFAULT_MAPPING = Path(__file__).with_name("contract-coverage.json")
ENTRYPOINTS = {"local_operations", "cli", "mcp", "viewer"}


def validate_mapping(mapping: dict[str, Any], root: Path = ROOT) -> None:
    if mapping.get("schema_version") != 1:
        raise AssertionError("contract coverage schema_version must be 1")
    contracts = mapping.get("contracts")
    if not isinstance(contracts, list) or not contracts:
        raise AssertionError("contract coverage must contain contracts")
    identifiers: set[str] = set()
    for contract in contracts:
        identifier = contract.get("id")
        if not isinstance(identifier, str) or not identifier or identifier in identifiers:
            raise AssertionError("contract coverage IDs must be unique non-empty strings")
        identifiers.add(identifier)
        owners = contract.get("owners")
        if not isinstance(owners, list) or len(set(owners)) < 2:
            raise AssertionError(f"cross-owner contract {identifier} must name at least two owners")
        if contract.get("requires_product_entrypoint") is not True:
            raise AssertionError(f"cross-owner contract {identifier} must require product entrypoints")
        required = contract.get("required_entrypoints")
        if (
            not isinstance(required, list)
            or not required
            or len(required) != len(set(required))
            or not set(required) <= ENTRYPOINTS
        ):
            raise AssertionError(f"contract {identifier} has invalid required product entrypoints")
        evidence = contract.get("evidence")
        if not isinstance(evidence, list) or not evidence:
            raise AssertionError(f"contract {identifier} has no evidence")
        covered: set[str] = set()
        for item in evidence:
            if (
                not isinstance(item, list)
                or len(item) != 3
                or not all(isinstance(value, str) and value for value in item)
            ):
                raise AssertionError(f"contract {identifier} has malformed evidence")
            entrypoint, relative, test_name = item
            if entrypoint not in ENTRYPOINTS:
                raise AssertionError(
                    f"contract {identifier} uses internal-only evidence instead of a product entrypoint"
                )
            path = root / relative
            if not path.is_file() or not path.resolve().is_relative_to((root / "rebuild/crates").resolve()):
                raise AssertionError(f"contract {identifier} references invalid Production test source")
            source = path.read_text(encoding="utf-8")
            if re.search(rf"\bfn\s+{re.escape(test_name)}\s*\(", source) is None:
                raise AssertionError(f"contract {identifier} references missing test {test_name}")
            covered.add(entrypoint)
        missing = set(required) - covered
        if missing:
            raise AssertionError(
                f"contract {identifier} lacks required product entrypoint coverage: {sorted(missing)}"
            )


def validate_forgetting_observation(value: dict[str, Any]) -> None:
    if value.get("state") != "completed":
        raise AssertionError("forgetting did not reach completed")
    for field in (
        "canonical_absent",
        "related_candidate_absent",
        "related_derived_absent",
        "unrelated_candidate_present",
        "unrelated_derived_present",
        "candidate_cleanup_completed",
        "managed_derived_cleanup_completed",
        "residue_verified",
    ):
        if value.get(field) is not True:
            raise AssertionError(f"forgetting acceptance lacks {field}")


def validate_candidate_failure_observation(value: dict[str, Any]) -> None:
    if value.get("projection_health") not in {"degraded", "failed", "partial"}:
        raise AssertionError("Candidate dependency failure was coerced to healthy success")
    if value.get("candidate_dependency") not in {
        "unavailable",
        "unsupported",
        "corrupt",
        "repair_required",
        "failed",
    }:
        raise AssertionError("Candidate dependency root cause was not preserved")
    if value.get("issue_scope") != "candidate_inspection":
        raise AssertionError("Candidate dependency failure lacks affected scope")


def validate_provider_failure_observation(value: dict[str, Any]) -> None:
    if value.get("configured") is not True:
        raise AssertionError("provider failure observation lacks configured transport")
    if value.get("transport_available") is not False:
        raise AssertionError("provider failure observation did not exercise unavailable transport")
    if value.get("outcome") != "provider_unavailable":
        raise AssertionError("configured unavailable transport lost its truthful outcome")
    if value.get("transmitted") is not False:
        raise AssertionError("unavailable provider observation claimed transmission")
    if value.get("commercial_semantic_provider_success") is not False:
        raise AssertionError("unavailable configured transport was reported as provider success")


def expect_rejected(action, message: str) -> None:
    try:
        action()
    except AssertionError:
        return
    raise AssertionError(message)


def self_test(mapping: dict[str, Any]) -> None:
    validate_mapping(mapping)
    internal_only = copy.deepcopy(mapping)
    internal_only["contracts"][0]["required_entrypoints"] = ["cli"]
    internal_only["contracts"][0]["evidence"] = [
        ["internal_primitive", "rebuild/crates/volicord-privacy/tests/privacy_boundary.rs", "canonical_forgetting_cleans_only_related_candidate_and_derived_content"]
    ]
    expect_rejected(
        lambda: validate_mapping(internal_only),
        "internal primitive-only coverage was accepted",
    )
    missing_entrypoint = copy.deepcopy(mapping)
    missing_entrypoint["contracts"][0]["evidence"] = missing_entrypoint["contracts"][0]["evidence"][:-1]
    expect_rejected(
        lambda: validate_mapping(missing_entrypoint),
        "missing required product entrypoint was accepted",
    )

    complete = {
        "state": "completed",
        "canonical_absent": True,
        "related_candidate_absent": True,
        "related_derived_absent": True,
        "unrelated_candidate_present": True,
        "unrelated_derived_present": True,
        "candidate_cleanup_completed": True,
        "managed_derived_cleanup_completed": True,
        "residue_verified": True,
    }
    validate_forgetting_observation(complete)
    for field, label in (
        ("related_candidate_absent", "canonical-only forgetting"),
        ("candidate_cleanup_completed", "ignored cleanup failure"),
        ("unrelated_candidate_present", "unrelated over-deletion"),
        ("residue_verified", "repair-required coercion"),
    ):
        broken = complete | {field: False}
        expect_rejected(
            lambda broken=broken: validate_forgetting_observation(broken),
            f"{label} was accepted",
        )
    repair_coercion = complete | {"state": "completed", "managed_derived_cleanup_completed": False}
    expect_rejected(
        lambda: validate_forgetting_observation(repair_coercion),
        "repair-required outcome was coerced to completed",
    )

    validate_candidate_failure_observation(
        {
            "projection_health": "degraded",
            "candidate_dependency": "corrupt",
            "issue_scope": "candidate_inspection",
        }
    )
    expect_rejected(
        lambda: validate_candidate_failure_observation(
            {
                "projection_health": "complete",
                "candidate_dependency": "available",
                "issue_scope": None,
                "candidates": [],
            }
        ),
        "Candidate error-to-empty conversion was accepted",
    )

    unavailable_provider = {
        "configured": True,
        "transport_available": False,
        "outcome": "provider_unavailable",
        "transmitted": False,
        "commercial_semantic_provider_success": False,
    }
    validate_provider_failure_observation(unavailable_provider)
    expect_rejected(
        lambda: validate_provider_failure_observation(
            unavailable_provider | {"commercial_semantic_provider_success": True}
        ),
        "configured unavailable provider was accepted as commercial success",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mapping", nargs="?", type=Path, default=DEFAULT_MAPPING)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    mapping = json.loads(args.mapping.read_text(encoding="utf-8"))
    if args.self_test:
        self_test(mapping)
        print("contract coverage self-test passed")
    else:
        validate_mapping(mapping)
        print(f"contract coverage passed: {len(mapping['contracts'])} cross-owner contract(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
