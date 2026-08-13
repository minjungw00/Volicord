#!/usr/bin/env python3
"""Map Phase 5 subsystem acceptance to Production Rust tests and run them."""

from __future__ import annotations

import json
from pathlib import Path
import shlex
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[4]
MANIFEST = ROOT / "rebuild/validation/shared/fixture-manifest.json"

REQUIREMENT_TESTS = {
    "all_text_fixture_inventory": "maintained_fixtures_recognize_all_seven_gate_languages",
    "out_of_set_inventory_fallback": "out_of_set_text_language_keeps_inventory_with_honest_fallback",
    "path_independent_repository_identity": "snapshot_identity_and_serialization_are_path_independent_and_repeatable",
    "changed_content_repository_basis": "snapshot_identity_and_serialization_are_path_independent_and_repeatable",
    "seven_structural_adapters": "all_seven_adapters_satisfy_maintained_entities_ranges_and_relations",
    "stable_entity_range_relation_output": "same_snapshot_facts_and_serialization_are_deterministic",
    "polyglot_failure_isolation": "injected_adapter_failure_is_bounded_to_one_language",
    "incremental_unaffected_reuse": "changed_file_and_declared_dependent_reparse_without_whole_repository",
    "dependency_invalidation": "changed_file_and_declared_dependent_reparse_without_whole_repository",
    "build_context_invalidation": "manifest_change_uses_explicit_build_context_invalidation",
    "stale_range_navigation": "search_is_source_grounded_and_stale_ranges_are_not_current_navigation",
    "three_semantic_ecosystems": "three_production_ecosystems_publish_normalized_semantic_relations",
    "semantic_unavailable_preserves_structural": "unavailable_or_failed_adapter_cannot_publish_semantic_facts",
    "semantic_failed_publishes_no_fact": "injected_adapter_failure_publishes_no_semantic_fact_for_failed_language",
    "semantic_broken_build_remainder": "broken_dependency_is_partial_without_erasing_usable_remainder",
    "same_name_and_overload_targets": "overloads_are_distinct_and_calls_resolve_by_arity",
    "semantic_deterministic_rebuild": "restart_cache_rebuild_and_refresh_are_deterministic",
    "source_grounded_search_explanation": "explanation_search_and_canonical_links_preserve_authority_and_freshness",
    "fact_result_interpretation_separation": "fact_result_and_interpretation_are_distinct_types_and_classes",
    "analysis_no_canonical_mutation": "repository_analysis_has_no_canonical_mutation_authority",
    "derived_rebuild_preserves_canonical_correction": "explanation_search_and_canonical_links_preserve_authority_and_freshness",
    "coverage_excluded_failed_stale_states": "exclusions_binary_vendor_generated_and_ignored_scopes_remain_visible",
    "repository_source_project_basis_grounding": "repository_source_project_and_snapshot_basis_are_grounded",
    "dangling_canonical_targets_rejected": "dangling_canonical_targets_are_rejected",
    "cross_project_canonical_targets_rejected": "cross_project_canonical_targets_are_rejected",
    "wrong_canonical_revisions_rejected": "impossible_and_unknown_canonical_revisions_are_rejected",
    "historical_revisions_preserved": "historical_revisions_remain_grounded_after_non_semantic_correction",
    "correction_does_not_rebind_analysis": "analysis_refresh_does_not_rebind_historical_references",
    "supersession_does_not_redirect_analysis": "decision_supersession_does_not_redirect_existing_analysis",
    "source_snapshot_basis_preserved": "repository_source_project_and_snapshot_basis_are_grounded",
    "analysis_snapshot_read_side_revalidation": "persisted_analysis_snapshot_links_are_revalidated_on_read",
    "canonical_grounding_no_mutation": "canonical_grounding_validation_has_no_mutation_authority",
    "deterministic_grounded_reference_serialization": "grounded_reference_serialization_is_deterministic_and_current_only",
    "all_canonical_reference_ingress_grounded": "automatic_and_manual_reference_ingress_is_grounded_before_consumption",
}

EXPECTED_V01 = {
    "v01-java",
    "v01-python",
    "v01-javascript",
    "v01-typescript",
    "v01-c",
    "v01-cpp",
    "v01-rust",
    "v01-polyglot",
    "v01-out-of-set",
}
EXPECTED_V02 = {"v02-java-maven", "v02-typescript-node", "v02-rust-cargo"}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run(arguments: list[str], *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    print(f"$ {shlex.join(arguments)}", flush=True)
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        if capture:
            print(result.stdout, end="", file=sys.stdout)
            print(result.stderr, end="", file=sys.stderr)
        raise RuntimeError(
            f"command failed with exit {result.returncode}: {shlex.join(arguments)}"
        )
    return result


def main() -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    fixtures = manifest.get("fixtures", [])
    v01 = {item.get("id") for item in fixtures if item.get("validation_id") == "V01"}
    v02 = {item.get("id") for item in fixtures if item.get("validation_id") == "V02"}
    require(v01 == EXPECTED_V01, "V01 fixture matrix changed without acceptance mapping")
    require(v02 == EXPECTED_V02, "V02 ecosystem matrix changed without acceptance mapping")

    cargo = [
        "cargo",
        "test",
        "--manifest-path",
        "rebuild/Cargo.toml",
        "-p",
        "volicord-repository-intelligence",
        "--all-targets",
        "--all-features",
    ]
    catalog = run([*cargo, "--", "--list"], capture=True).stdout
    missing = sorted(
        test for test in set(REQUIREMENT_TESTS.values()) if f"{test}: test" not in catalog
    )
    require(not missing, f"mapped Production Rust tests are missing: {missing}")
    production_tests = sum(line.endswith(": test") for line in catalog.splitlines())
    run(cargo)
    print(
        json.dumps(
            {
                "mapped_requirements": len(REQUIREMENT_TESTS),
                "production_tests": production_tests,
                "structural_fixture_count": len(EXPECTED_V01) - 2,
                "polyglot_fixture_count": 1,
                "fallback_fixture_count": 1,
                "semantic_ecosystem_count": len(EXPECTED_V02),
                "status": "passed",
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
