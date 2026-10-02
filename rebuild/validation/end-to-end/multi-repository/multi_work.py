"""Installed public-operation Volicord Work continuity rehearsal.

The caller supplies the installed CLI/MCP adapters. Evidence remains in the local
V11 result; a later contract projects only bounded identity observations.
"""
from __future__ import annotations

from pathlib import Path
from typing import Any


def rehearse(api: Any, *, project_id: str, repository: Path, cli: Path,
             mcp_binary: Path, env: dict[str, str], recorder: Any,
             expected_a: dict[str, Any], canonical_after_a: dict[str, Any],
             purpose_id: str) -> dict[str, Any]:
    evidence: dict[str, Any] = {"project_id": project_id, "purpose_id": purpose_id}
    errors: list[str] = []
    host = None

    def call(name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        value, ok = host.tool(name, arguments)
        if not ok or not isinstance(value, dict):
            raise ValueError(f"{name} rejected valid Work path: {value}")
        return value

    try:
        status_before, status_operation = api.cli_json(
            recorder, "multi-work-status-before", cli, env, "status", cwd=repository)
        evidence["status_before"] = status_before
        evidence["status_before_operation"] = status_operation
        if not status_before or status_before.get("project_id") != project_id:
            raise ValueError("CLI status before new Work is unavailable")
        host = api.Mcp(mcp_binary, env)
        host.initialize()
        work = {"A": {
            "goal_id": expected_a["goal_id"],
            "source_id": expected_a["goal_source_id"],
            "checkpoint_id": expected_a["checkpoint_id"],
            "decision_id": expected_a["decision_id"],
        }}
        for label in ("B", "C"):
            marker = f"v11-work-{label.lower()}.txt"
            statement = f"Record bounded {label} Work continuity in the same Project"
            goal = call("context_record", {
                "project_id": project_id, "role": "goal",
                "work_transition": "start_new", "user_turn": statement,
                "statement": statement,
            })
            goal_id, source_id = goal.get("context_item_id"), goal.get("source_id")
            if (goal.get("project_id") != project_id
                    or goal.get("work_transition") != "start_new"
                    or goal.get("canonical_mutation") is not True
                    or not goal_id or not source_id
                    or goal_id in {entry["goal_id"] for entry in work.values()}):
                raise ValueError(f"Work {label} did not create a distinct Goal/Source")
            baseline = call("repository_analyze", {
                "project_id": project_id, "excluded_paths": []})
            baseline_id = baseline.get("analysis_snapshot_id")
            repository_source = baseline.get("repository_source_id")
            if not baseline_id or not repository_source:
                raise ValueError(f"Work {label} has no fresh analysis baseline")
            choice_id = f"private-continuity-{label.lower()}"
            alternatives = ("inline", "helper")
            choice = {
                "choice_id": choice_id,
                "summary": f"Choose private {label} Work marker organization",
                "affected_scope": [marker],
                "alternatives": [{
                    "alternative_id": alternative,
                    "summary": f"Keep the {label} marker in a private {alternative} form",
                    "technical_consequences": ["The bounded marker preserves public Work behavior"],
                    "material_decomposition": {
                        "state": "materially_atomic",
                        "rationale": "The maintained fixture fixes public behavior and leaves no subordinate product fork.",
                        "residual_fork_closure": {
                            "interaction_comparisons": [],
                            "fixed_outcome": "The Work marker preserves the same public behavior",
                            "credible_implementations": [
                                "Direct private implementation", "Private helper implementation"],
                            "remaining_material_outcomes": [],
                            "source_basis": [repository_source],
                        },
                    },
                } for alternative in alternatives],
                "technical_consequences": ["Private organization varies without changing public Work behavior"],
                "source_ids": [repository_source],
                "effect_categories": ["implementation_internal"],
                "relationship": {"state": "independent"},
                "evidence_state": "sufficient",
            }
            discovery = call("engineering_choice_discovery", {
                "project_id": project_id,
                "goal_context_id": goal_id,
                "baseline_analysis_snapshot_id": baseline_id,
                "source_operation": f"V11 {label} Work private-marker inspection",
                "summary": choice["summary"],
                "choices": [choice],
                "interaction_review": api.outside_interactions([choice], [repository_source]),
                "material_boundary_review": api.material_boundary_review([choice], [repository_source]),
            })
            review = call("materiality_review", {
                "action": "record",
                "project_id": project_id,
                "engineering_choice_discovery_candidate_id": discovery["discovery_candidate_id"],
                "rationale": "The exact current Goal and source leave only private marker organization open.",
                "behavioral_context_basis": {
                    "context_item_ids": [],
                    "completeness_rationale": "This bounded Work has no consequential non-Goal Context.",
                },
                "learning_participation": {"state": "inactive"},
                "judgments": [{
                    "choice_id": choice_id,
                    "disposition": "agent_owned_implementation_choice",
                    "materially_varying_outcomes": ["Private marker organization"],
                    "contains_user_owned_outcome": False,
                    "user_owned_outcomes": [],
                    "ownership_rationale": "Both private arrangements preserve the same public Work result.",
                    "discretion_counterfactuals": [{
                        "choice_id": choice_id, "alternative_id": alternative,
                        "externally_observable": False,
                        "observation_rationale": "Public Work identity and history remain the same.",
                        "source_id": repository_source,
                        "source_supported_boundary": "The maintained fixture bounds this private marker.",
                    } for alternative in alternatives],
                    "bounded_implementation_discretion_rationale": "The change is limited to a private marker.",
                    "ownership_source_ids": [repository_source],
                    "alternative_accounting": api.alternative_accounting(
                        choice_id, list(alternatives), repository_source),
                    "basis_summary": "Only private marker organization varies.",
                    "authority_counterfactual": "Neither alternative changes a user-owned outcome.",
                    "learning_authority": {"state": "inactive"},
                    "learning_value": {"state": "routine", "rationale": "No learning participation was requested."},
                }],
            })
            ready = call("materiality_review", {
                "action": "inspect",
                "project_id": project_id,
                "review_candidate_id": review["review_candidate_id"],
                "goal_context_id": goal_id,
                "baseline_analysis_snapshot_id": baseline_id,
                "paths": [marker], "components": [], "work_contexts": [],
                "met_revisit_triggers": [],
                "coupled_artifact_review": api.coupled_artifact_review([marker]),
            })
            if (ready.get("workflow", {}).get("stage") != "ready_for_work"
                    or ready["workflow"].get("blocks_ordinary_work") is not False):
                raise ValueError(f"Work {label} materiality did not resolve")
            (repository / marker).write_text(f"{label} Work continuity marker\n", encoding="utf-8")
            checkpoint_args = {
                "verification_basis": {"state": "ordinary_change"},
                "project_id": project_id,
                "goal_context_id": goal_id,
                "baseline_analysis_snapshot_id": baseline_id,
                "kind": "handoff", "work_state": "in_progress" if label == "C" else "paused",
                "work_contexts": [], "verification": [{"state": "not_run"}],
                "next_step": f"Resume {label} Work", "handoff_to": "next Codex session",
            }
            if label == "B":
                before_rejection = call("canonical_inspect", {"project_id": project_id})
                rejected, accepted = host.tool("checkpoint_record", {
                    **checkpoint_args,
                    "applied_decision_ids": [expected_a["decision_id"]],
                })
                after_rejection = call("canonical_inspect", {"project_id": project_id})
                mismatch = (rejected or {}).get("details", {}).get("cause", {}).get(
                    "checkpoint_decision_work_mismatch")
                if (accepted or not isinstance(mismatch, dict)
                        or mismatch.get("checkpoint_work_item_id") != goal_id
                        or mismatch.get("decision_id") != expected_a["decision_id"]
                        or mismatch.get("decision_work_item_id") != expected_a["goal_id"]
                        or api.restart_recall.canonical_evidence(before_rejection)
                        != api.restart_recall.canonical_evidence(after_rejection)):
                    raise ValueError("cross-Work Decision misuse was not atomically rejected")
                evidence["cross_work_rejection"] = {
                    "accepted": accepted, "mismatch": mismatch,
                    "before": before_rejection, "after": after_rejection,
                }
            checkpoint = call("checkpoint_record", {
                **checkpoint_args, "applied_decision_ids": []})
            if (checkpoint.get("goal_context_id") != goal_id
                    or checkpoint.get("baseline_analysis_snapshot_id") != baseline_id
                    or checkpoint.get("changed_paths") != [marker]
                    or checkpoint.get("workflow", {}).get("disposition") != "checkpoint_recorded"):
                raise ValueError(f"Work {label} Checkpoint identity/scope is wrong")
            work[label] = {
                "goal_id": goal_id, "source_id": source_id,
                "checkpoint_id": checkpoint["checkpoint_id"],
                "baseline_analysis_snapshot_id": baseline_id,
                "marker": marker,
            }
            evidence[f"work_{label.lower()}"] = {
                "goal": goal, "baseline": baseline, "discovery": discovery,
                "review": review, "ready": ready, "checkpoint": checkpoint,
            }
        evidence["work"] = work
        evidence["canonical_after_a"] = canonical_after_a
        evidence["canonical_after_new_work"] = call("canonical_inspect", {"project_id": project_id})
        evidence["mcp_understanding"] = call("repository_understanding", {"project_id": project_id})
        status_after, status_operation = api.cli_json(
            recorder, "multi-work-status-after", cli, env, "status", cwd=repository)
        evidence["status_after"] = status_after
        evidence["status_after_operation"] = status_operation
        if not status_after or status_after.get("project_id") != project_id:
            raise ValueError("CLI status after new Work is unavailable")
    except (OSError, RuntimeError, ValueError, KeyError, TypeError) as error:
        errors.append(str(error))
    finally:
        if host is not None:
            evidence["cleanup"] = host.close()
    evidence["errors"] = errors
    return evidence


def verify_rehearsal(evidence: dict[str, Any], expected_a: dict[str, Any],
                     portable_status: dict[str, Any] | None) -> dict[str, bool]:
    """Compare authored identities to both local read surfaces and imported clone."""
    work = evidence.get("work", {})
    before = evidence.get("status_before") or {}
    after = evidence.get("status_after") or {}
    understanding = evidence.get("mcp_understanding") or {}
    canonical = evidence.get("canonical_after_new_work") or {}
    rejection = evidence.get("cross_work_rejection") or {}
    imported = portable_status or {}
    project_id = evidence.get("project_id")
    purpose_id = evidence.get("purpose_id")
    ids = [work.get(label, {}).get("goal_id") for label in ("A", "B", "C")]
    sources = [work.get(label, {}).get("source_id") for label in ("A", "B", "C")]
    checkpoints = [work.get(label, {}).get("checkpoint_id") for label in ("A", "B", "C")]
    def distinct(values: list[Any]) -> bool:
        return all(isinstance(value, str) and value for value in values) and len(set(values)) == 3
    def history(view: dict[str, Any]) -> bool:
        rows = view.get("work_history")
        if not isinstance(rows, list) or len(rows) != 3:
            return False
        by_id = {row.get("work_item_id"): row for row in rows if isinstance(row, dict)}
        if set(by_id) != set(ids):
            return False
        for label in ("A", "B", "C"):
            row = by_id[work[label]["goal_id"]]
            if (work[label]["checkpoint_id"] not in row.get("checkpoint_ids", [])
                    or work[label]["source_id"] not in row.get("source_ids", [])
                    or row.get("state") != ("in_progress" if label == "C" else "paused")):
                return False
        return True
    def current(view: dict[str, Any]) -> bool:
        return (
            [row.get("work_item_id") for row in view.get("current_work", [])] == [ids[2]]
            and {row.get("work_item_id") for row in view.get("remaining_work", [])}
            == set(ids[:2])
        )
    def purpose_meaning(view: dict[str, Any]) -> list[tuple[str, tuple[str, ...]]]:
        rows = view.get("project_purpose")
        if not isinstance(rows, list):
            return []
        meaning = []
        for row in rows:
            if not isinstance(row, dict) or not isinstance(row.get("statement"), str):
                return []
            sources = row.get("source_ids", row.get("source_basis"))
            if not isinstance(sources, list) or not sources:
                return []
            meaning.append((row["statement"], tuple(sorted(sources))))
        return sorted(meaning)
    purpose = before.get("project_purpose")
    records = canonical.get("records", [])
    record_ids = {(row.get("kind"), row.get("identity")) for row in records
                  if isinstance(row, dict)}
    earlier = evidence.get("canonical_after_a", {}).get("records", [])
    prior_records_retained = bool(earlier) and all(
        any(row.get("kind") == old.get("kind")
            and row.get("identity") == old.get("identity")
            and row.get("revision") == old.get("revision")
            and row.get("source_basis") == old.get("source_basis")
            for row in records)
        for old in earlier if isinstance(old, dict) and old.get("kind") != "project"
    )
    mismatch = rejection.get("mismatch") or {}
    return {
        "public_authoring": not evidence.get("errors")
            and all(evidence.get(f"work_{label.lower()}", {}).get("checkpoint")
                    for label in ("B", "C")),
        "project_identity": bool(project_id and all(
            view.get("project_id") == project_id
            for view in (before, after, understanding, imported))),
        "purpose_preserved": bool(purpose_id and ("contextitem", purpose_id) in record_ids
            and purpose and purpose == after.get("project_purpose")
            and purpose == imported.get("project_purpose")
            and purpose_meaning(before) == purpose_meaning(understanding)),
        "distinct_work_goal_source_checkpoint": distinct(ids)
            and distinct(sources) and distinct(checkpoints),
        "restart_a_retained": bool(
            ids[0] == expected_a.get("goal_id")
            and sources[0] == expected_a.get("goal_source_id")
            and checkpoints[0] == expected_a.get("checkpoint_id")
            and prior_records_retained),
        "cross_work_authority_rejected_without_mutation": bool(
            rejection.get("accepted") is False
            and mismatch.get("checkpoint_work_item_id") == ids[1]
            and mismatch.get("decision_id") == expected_a.get("decision_id")
            and mismatch.get("decision_work_item_id") == ids[0]
            and rejection.get("before", {}).get("records")
            == rejection.get("after", {}).get("records")),
        "cli_history_and_current": history(after) and current(after),
        "mcp_history_and_current": history(understanding) and current(understanding),
        "portable_history_and_current": history(imported) and current(imported),
    }
