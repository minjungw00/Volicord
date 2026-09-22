#!/usr/bin/env python3
"""Bounded cross-boundary regressions for the Phase 8 remediation sequence."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import time


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
MANIFEST = ROOT / "rebuild/Cargo.toml"


def command(identity: str, coverage: list[str], argv: list[str], cwd: Path) -> dict:
    started = time.monotonic_ns()
    completed = subprocess.run(
        argv,
        cwd=cwd,
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        stdin=subprocess.DEVNULL,
        check=False,
    )
    return {
        "id": identity,
        "coverage": coverage,
        "argv": argv,
        "exit_code": completed.returncode,
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
    }


def main() -> int:
    python_cases = [
        "frontier_self_test.FrontierTests.test_mixed_validation_outcomes_preserve_scope_and_attribution",
        "frontier_self_test.FrontierTests.test_validation_baseline_regression_recovery_and_environment_states",
        "machine_findings_self_test.MachineFindingTests.test_required_validation_hard_blocker_requires_confirmed_attribution",
        "long_lived_project_self_test.LongLivedProjectTests.test_one_project_accumulates_distinct_work_over_time",
        "long_lived_project_self_test.LongLivedProjectTests.test_viewer_and_review_contract_distinguish_every_work_item",
        "long_lived_project_self_test.LongLivedProjectTests.test_fixture_routes_to_production_restart_portability_cli_and_viewer_tests",
        "long_lived_project_self_test.LongLivedProjectTests.test_deterministic_fixture_is_not_naturalistic_qualification_evidence",
        "review_operations_self_test.WorkflowTests.test_conversational_human_judgment_generates_reviewable_draft",
        "review_operations_self_test.WorkflowTests.test_human_controls_preserve_partial_reference_and_insufficient_semantics",
        "review_operations_self_test.WorkflowTests.test_same_as_english_requires_the_identical_criterion_and_rebinds_locale_evidence",
        "qualitative_review_self_test.ContractTests.test_viewer_usability_dimensions_and_machine_timing_remain_independent",
        "qualification_self_test.FileBoundaryTests.test_real_evidence_reviews_result_revalidation_and_mismatch",
    ]
    definitions = [
        ("dogfood-interfaces",
         ["mixed_validation_semantics", "recovered_and_confirmed_failure_attribution",
          "long_lived_project_fixture_support_only", "live_evidence_unresolved_boundaries",
          "conversational_human_review", "human_control_states", "exact_semantic_locale_reuse", "updated_ux_rubric",
          "durable_result_lineage"],
         [sys.executable, "-B", "-m", "unittest", "-v", *python_cases], HERE),
        ("project-work-read-consumers",
         ["project_purpose_work_separation", "multiple_work_items"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-operations",
          "--test", "multi_work_project", "multi_work_identity_survives_restart_portability_and_read_consumers",
          "--", "--exact"], ROOT),
        ("polyglot-current-work",
         ["polyglot_current_work_flow", "current_work_grounding"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-projections",
          "--test", "current_work_flow", "polyglot_"], ROOT),
        ("viewer-grounding-hierarchy",
         ["viewer_graph_hierarchy", "viewer_evidence_disclosure", "multiple_work_items"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-viewer",
          "--test", "viewer", "project_understanding", "--", "--nocapture"], ROOT),
        ("viewer-profile",
         ["viewer_snapshot_request_performance_evidence", "browser_input_paint_scope_guard",
          "viewer_audit_detail_ordering"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-viewer",
          "--test", "viewer", "representative_large_repository_page_is_deterministically_bounded",
          "--", "--exact", "--nocapture"], ROOT),
        ("multi-decision-independent",
         ["multi_decision_inquiry", "independent_question_authority"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-operations",
          "--test", "work_authority", "independent_user_owned_outcomes_cannot_share_question_authority",
          "--", "--exact"], ROOT),
        ("multi-decision-settled",
         ["multi_decision_inquiry", "settled_choice_no_redundant_question"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-operations",
          "--test", "work_authority", "settled_choice_does_not_manufacture_a_second_user_question",
          "--", "--exact"], ROOT),
        ("mcp-project-understanding",
         ["project_understanding_mcp", "decision_and_work_grounding"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-host",
          "--test", "mcp", "mcp_workflow_guides_material_question_to_explicit_decision_and_ready_work",
          "--", "--exact"], ROOT),
        ("mcp-learning-explanation",
         ["learning_explanation_context", "restart_candidate_inspection"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-host",
          "--test", "mcp", "installed_mcp_learning_deliberation_is_ordered_restartable_and_not_a_decision",
          "--", "--exact"], ROOT),
        ("document-language-fidelity",
         ["document_readability", "requested_language_realization"],
         ["cargo", "test", "--manifest-path", str(MANIFEST), "-p", "volicord-projections",
          "--test", "project_documents", "project_surface_and_four_documents_are_grounded_equivalent_and_read_only",
          "--", "--exact"], ROOT),
    ]
    results = [command(*definition) for definition in definitions]
    missing = sorted({item for result in results for item in result["coverage"]
                      if result["exit_code"] != 0})
    outcome = {
        "kind": "phase8_remediation_integration_regressions",
        "schema_version": 2,
        "status": "passed" if not missing else "failed",
        "checks": results,
        "failed_coverage": missing,
    }
    print(json.dumps(outcome, indent=2, sort_keys=True))
    return 0 if not missing else 1


if __name__ == "__main__":
    raise SystemExit(main())
