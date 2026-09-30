#!/usr/bin/env python3
"""Self-test the Phase 8 dogfood evaluation support boundary."""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
HARNESS = Path(__file__).resolve().parent / "harness.py"


def main() -> int:
    """Assert the one active Naturalistic contract and retained support engine."""
    import campaign
    import harness
    import machine_findings
    import qualitative_review
    import qualification_policy
    import review_operations

    definition = harness.load_definition()
    topology = definition["campaign_topology"]
    if (topology["journey_count"], topology["work_count"],
            topology["resume_pair_count"], topology["session_count"]) != (3, 5, 3, 8):
        raise AssertionError("current Naturalistic topology changed")
    if any(key in definition for key in (
            "qualification_profile_contract", "materiality_obligations",
            "materiality_obligation_minimums", "reconciliation_contract")):
        raise AssertionError("active definition retains a semantic admission profile")
    if any(key in definition["real_session_evidence"] for key in (
            "bounded_evaluation_basis", "behavior_review",
            "opaque_slot_contract", "behavior_specific_work_intake_contract")):
        raise AssertionError("active definition retains pre-execution semantic obligations")
    commands = set(campaign.parser()._subparsers._group_actions[0].choices)
    obsolete = {"prepare-review", "validate-discovery", "record-discovery",
        "prepare-critique", "validate-critique", "record-critique",
        "prepare-adjudication", "validate-provisional-review",
        "record-provisional-review", "reveal-qualification-profile",
        "prepare-reconciliation", "validate-reconciliation",
        "inspect-reconciliation", "seal-work"}
    if commands & obsolete or not {"prepare", "activate-all", "collect-batch",
            "evaluate", "prepare-qualitative-review", "qualify",
            "approve-phase-9"} <= commands:
        raise AssertionError("current campaign CLI exposes a superseded semantic gate")
    git_contract = definition["repository_state_attestation_contract"]
    if (git_contract["dirty_state_blocks_collection"] or git_contract["work_commit_required"]
            or git_contract["git_defines_work_identity"]
            or git_contract["optional_status_command"] != campaign.GIT_STATE_CHECK):
        raise AssertionError("Dogfood invented Git workflow admission or Work identity")
    prepare = campaign.parser()._subparsers._group_actions[0].choices["prepare"]
    if not any("--tasks" in action.option_strings and action.required
            for action in prepare._actions):
        raise AssertionError("preparation does not require frozen task input")
    if qualification_policy.contract() != definition["qualification_policy"]:
        raise AssertionError("qualification policy differs from active definition")
    if "campaign_control_coverage" in qualification_policy.contract():
        raise AssertionError("blind control coverage is still a qualification condition")
    if qualitative_review.STATES != definition["qualitative_review_contract"]["states"]:
        raise AssertionError("qualitative assessment states differ from the current definition")
    if len(set(qualitative_review.STATES)) != 6 or "not_observed" not in qualitative_review.STATES:
        raise AssertionError("not_observed is not a distinct review state")
    machine_findings.validate_policy()
    if machine_findings.disposition("raw_hash", "confirmed_violation") != machine_findings.Disposition.HARD:
        raise AssertionError("raw hash tampering lost hard authority")
    for status in machine_findings.Status:
        if machine_findings.disposition("git_history_observation", status) != machine_findings.Disposition.ADVISORY:
            raise AssertionError("Git observation became a machine admission requirement")
    if machine_findings.disposition("learning_deliberation_order", "confirmed_violation") == machine_findings.Disposition.HARD:
        raise AssertionError("semantic Learning observation became a hard machine gate")
    if review_operations.workflow_contract()["qualification_authority"] is not False:
        raise AssertionError("review recording gained qualification authority")
    result = subprocess.run([sys.executable, "-B", str(HARNESS), "self-test"],
        cwd=ROOT, check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    print(result.stdout, end="")
    if result.returncode != 0:
        raise RuntimeError(f"Phase 8 harness self-test failed with exit {result.returncode}: {result.stderr[-2000:]}")
    print("phase 8 dogfood assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
