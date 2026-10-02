"""Current internal V11 target coverage and bounded lifecycle evidence contract."""
from __future__ import annotations

import hashlib
import json
from typing import Any


SCHEMA_VERSION = 2
TECHNICAL_CONTRACT = "v11-work-continuity-1"
WORKLOAD_IDENTITY = "three-target-installed-journey-volicord-three-work-1"
TARGETS = ("volicord", "small-python", "polyglot-medium")
COMMON_STEPS = (
    "clean_install", "codex_mcp_connection", "project_binding",
    "repository_analysis", "source_grounded_understanding", "candidate_boundary",
    "inquiry_decision", "ordinary_work", "guarded_boundary", "checkpoint",
    "restart_recall", "portable_clone", "divergent_conflict",
    "correction_supersession_deletion", "document_outputs", "provider_failure",
    "parser_failure", "derived_index_recovery",
)
VOLICORD_EXTRA = ("multi_work_continuity",)


def required_steps_for_target(target: str) -> tuple[str, ...]:
    if target not in TARGETS:
        raise ValueError(f"unknown V11 target: {target}")
    return COMMON_STEPS + (VOLICORD_EXTRA if target == "volicord" else ())


def required_counts() -> dict[str, int]:
    return {target: len(required_steps_for_target(target)) for target in TARGETS}


def required_total() -> int:
    return sum(required_counts().values())


def _digest(value: Any) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def _id(value: Any) -> bool:
    return isinstance(value, str) and 1 <= len(value) <= 64 and all(
        character.isalnum() or character in "-_" for character in value)


def _exact(value: Any, names: set[str]) -> bool:
    return isinstance(value, dict) and set(value) == names


def _purpose_digest(value: Any) -> str | None:
    if not isinstance(value, list) or not value:
        return None
    meaning = []
    for row in value:
        if not isinstance(row, dict) or not isinstance(row.get("statement"), str) or not row["statement"].strip():
            return None
        sources = row.get("source_ids", row.get("source_basis"))
        if (not isinstance(sources, list) or not sources
                or any(not _id(source) for source in sources)):
            return None
        meaning.append([row["statement"], sorted(sources)])
    return _digest(sorted(meaning))


def _view(value: dict[str, Any] | None) -> dict[str, Any]:
    value = value or {}
    rows = value.get("work_history", [])
    return {
        "project_id": value.get("project_id"),
        "purpose_sha256": _purpose_digest(value.get("project_purpose")),
        "history": sorted([{
            "work_id": row.get("work_item_id"),
            "state": row.get("state"),
            "checkpoint_ids": sorted(row.get("checkpoint_ids", [])),
            "source_ids": sorted(row.get("source_ids", row.get("source_basis", []))),
        } for row in rows if isinstance(row, dict)], key=lambda row: str(row["work_id"])),
        "current_work_ids": sorted(row.get("work_item_id") for row in value.get("current_work", [])
                                   if isinstance(row, dict)),
        "remaining_work_ids": sorted(row.get("work_item_id") for row in value.get("remaining_work", [])
                                     if isinstance(row, dict)),
    }


def _restart_read(value: dict[str, Any] | None, goal_id: str, decision_id: str) -> dict[str, Any]:
    value = value or {}
    goals = [item for item in value.get("goal_basis", [])
             if isinstance(item, dict) and item.get("identity") == goal_id]
    decisions = [item for item in value.get("decisions", [])
                 if isinstance(item, dict) and item.get("identity") == decision_id]
    goal = goals[0] if len(goals) == 1 else {}
    decision = decisions[0] if len(decisions) == 1 else {}
    checkpoint = value.get("checkpoint") or {}
    return {
        "project_id": value.get("project_id"),
        "goal_id": goal.get("identity"),
        "goal_source_ids": sorted(goal.get("source_ids", [])),
        "decision_id": decision.get("identity"),
        "decision_revision": decision.get("revision"),
        "decision_work_scope": decision.get("work_scope"),
        "checkpoint_id": checkpoint.get("identity"),
        "checkpoint_revision": checkpoint.get("revision"),
        "checkpoint_work_id": checkpoint.get("work_item_id"),
        "checkpoint_decision_ids": sorted(checkpoint.get("applied_decisions", [])),
    }


def make_lifecycle_proof(evidence: dict[str, Any], restart: dict[str, Any]) -> dict[str, Any]:
    """Project local raw operation results to a small, sanitized relation proof."""
    work = evidence["work"]
    expected = restart["expected_state"]
    rejection = evidence["cross_work_rejection"]
    mismatch = rejection["mismatch"]
    records_before = rejection["before"]["records"]
    records_after = rejection["after"]["records"]
    canonical_before = evidence["canonical_after_a"]["records"]
    canonical_after = evidence["canonical_after_new_work"]["records"]
    prior = [[row.get("kind"), row.get("identity"), row.get("revision"),
              sorted(row.get("source_basis", []))] for row in canonical_before
             if row.get("kind") != "project"]
    retained = [[row.get("kind"), row.get("identity"), row.get("revision"),
                 sorted(row.get("source_basis", []))] for row in canonical_after
                if row.get("kind") != "project"]
    authored = {}
    for label in ("B", "C"):
        raw = evidence[f"work_{label.lower()}"]
        authored[label] = {
            "project_id": raw["goal"].get("project_id"),
            "goal_id": raw["goal"].get("context_item_id"),
            "goal_source_id": raw["goal"].get("source_id"),
            "transition": raw["goal"].get("work_transition"),
            "canonical_mutation": raw["goal"].get("canonical_mutation"),
            "baseline_id": raw["baseline"].get("analysis_snapshot_id"),
            "ready_stage": raw["ready"].get("workflow", {}).get("stage"),
            "checkpoint_id": raw["checkpoint"].get("checkpoint_id"),
            "checkpoint_goal_id": raw["checkpoint"].get("goal_context_id"),
            "checkpoint_baseline_id": raw["checkpoint"].get("baseline_analysis_snapshot_id"),
            "checkpoint_decision_ids": raw["checkpoint"].get("applied_decision_ids"),
            "checkpoint_disposition": raw["checkpoint"].get("workflow", {}).get("disposition"),
        }
    continuation = restart.get("continuation") or {}
    return {
        "project_id": evidence["project_id"],
        "purpose_id": evidence["purpose_id"],
        "work": {label: {
            "goal_id": work[label]["goal_id"],
            "source_id": work[label]["source_id"],
            "checkpoint_id": work[label]["checkpoint_id"],
        } for label in ("A", "B", "C")},
        "restart": {
            "expected": {
                "project_id": expected["project_id"],
                "goal_id": expected["goal_id"],
                "goal_source_id": expected["goal_source_id"],
                "goal_revision": expected["goal_revision"],
                "decision_id": expected["decision_id"],
                "decision_revision": expected["decision_revision"],
                "decision_work_scope": expected["decision_work_scope"],
                "checkpoint_id": expected["checkpoint_id"],
                "checkpoint_revision": expected["checkpoint_revision"],
            },
            "cli": _restart_read(restart.get("cli_recall"), expected["goal_id"], expected["decision_id"]),
            "mcp": _restart_read(restart.get("restarted_recall"), expected["goal_id"], expected["decision_id"]),
            "continuation": {
                "project_id": continuation.get("project_id"),
                "goal_id": continuation.get("context_item_id"),
                "source_id": continuation.get("source_id"),
                "revision": continuation.get("revision"),
                "transition": continuation.get("work_transition"),
                "canonical_mutation": continuation.get("canonical_mutation"),
            },
        },
        "authored": authored,
        "authority": {
            "decision_id": mismatch.get("decision_id"),
            "decision_work_id": mismatch.get("decision_work_item_id"),
            "attempted_work_id": mismatch.get("checkpoint_work_item_id"),
            "rejected": rejection.get("accepted") is False,
            "canonical_before_sha256": _digest(records_before),
            "canonical_after_sha256": _digest(records_after),
        },
        "retention": {
            "prior": sorted(prior),
            "after": sorted(retained),
        },
        "views": {
            "cli": _view(evidence.get("status_after")),
            "mcp": _view(evidence.get("mcp_understanding")),
            "portable": _view(evidence.get("portable_status")),
            "before": _view(evidence.get("status_before")),
        },
    }


def lifecycle_errors(proof: Any) -> list[str]:
    """Check relationships without trusting the producer's leaf status or check booleans."""
    try:
        return _lifecycle_errors(proof)
    except (AttributeError, KeyError, TypeError, ValueError):
        return ["malformed lifecycle proof"]


def _lifecycle_errors(proof: Any) -> list[str]:
    errors: list[str] = []
    if not _exact(proof, {"project_id", "purpose_id", "work", "restart", "authored", "authority",
                          "retention", "views"}):
        return ["lifecycle proof shape"]
    project = proof["project_id"]
    work = proof["work"]
    if not _id(project) or not _id(proof["purpose_id"]):
        errors.append("Project/Purpose identity")
    if not _exact(work, {"A", "B", "C"}):
        return errors + ["A/B/C Work coverage"]
    for label in ("A", "B", "C"):
        if not _exact(work[label], {"goal_id", "source_id", "checkpoint_id"}) or not all(
            _id(value) for value in work[label].values()):
            errors.append(f"Work {label} identity")
    if errors:
        return errors
    for field in ("goal_id", "source_id", "checkpoint_id"):
        if len({work[label][field] for label in work}) != 3:
            errors.append(f"duplicate {field}")
    restart = proof["restart"]
    if not _exact(restart, {"expected", "cli", "mcp", "continuation"}):
        return errors + ["restart proof shape"]
    expected = restart["expected"]
    if not _exact(expected, {"project_id", "goal_id", "goal_source_id", "goal_revision",
                              "decision_id", "decision_revision", "decision_work_scope",
                              "checkpoint_id", "checkpoint_revision"}):
        return errors + ["restart expected shape"]
    if (expected["project_id"] != project or expected["goal_id"] != work["A"]["goal_id"]
            or expected["goal_source_id"] != work["A"]["source_id"]
            or expected["checkpoint_id"] != work["A"]["checkpoint_id"]
            or expected["decision_work_scope"] != {
                "kind": "work_item", "work_item_id": work["A"]["goal_id"]}):
        errors.append("restart expected identity/scope")
    if not _id(expected["decision_id"]) or any(
        type(expected[key]) is not int or expected[key] < 1
        for key in ("goal_revision", "decision_revision", "checkpoint_revision")
    ):
        errors.append("restart revision/Decision")
    observed_keys = {"project_id", "goal_id", "goal_source_ids", "decision_id",
                     "decision_revision", "decision_work_scope", "checkpoint_id",
                     "checkpoint_revision", "checkpoint_work_id", "checkpoint_decision_ids"}
    for transport in ("cli", "mcp"):
        observed = restart[transport]
        if not _exact(observed, observed_keys):
            errors.append(f"{transport} restart shape")
            continue
        if (observed["project_id"] != project
                or observed["goal_id"] != work["A"]["goal_id"]
                or work["A"]["source_id"] not in observed["goal_source_ids"]
                or observed["decision_id"] != expected["decision_id"]
                or observed["decision_revision"] != expected["decision_revision"]
                or observed["decision_work_scope"] != expected["decision_work_scope"]
                or observed["checkpoint_id"] != expected["checkpoint_id"]
                or observed["checkpoint_revision"] != expected["checkpoint_revision"]
                or observed["checkpoint_work_id"] != work["A"]["goal_id"]
                or observed["checkpoint_decision_ids"] != [expected["decision_id"]]):
            errors.append(f"{transport} restart relation")
    continued = restart["continuation"]
    if (not _exact(continued, {"project_id", "goal_id", "source_id", "revision",
                                "transition", "canonical_mutation"})
            or continued != {
                "project_id": project, "goal_id": work["A"]["goal_id"],
                "source_id": work["A"]["source_id"], "revision": expected["goal_revision"],
                "transition": "continue", "canonical_mutation": False,
            }):
        errors.append("same-Work continuation")
    authored = proof["authored"]
    if not _exact(authored, {"B", "C"}):
        errors.append("new Work authoring coverage")
    else:
        keys = {"project_id", "goal_id", "goal_source_id", "transition", "canonical_mutation",
                "baseline_id", "ready_stage", "checkpoint_id", "checkpoint_goal_id",
                "checkpoint_baseline_id", "checkpoint_decision_ids", "checkpoint_disposition"}
        for label in ("B", "C"):
            row = authored[label]
            if (not _exact(row, keys)
                    or row.get("project_id") != project
                    or row.get("goal_id") != work[label]["goal_id"]
                    or row.get("goal_source_id") != work[label]["source_id"]
                    or row.get("transition") != "start_new"
                    or row.get("canonical_mutation") is not True
                    or not _id(row.get("baseline_id"))
                    or row.get("ready_stage") != "ready_for_work"
                    or row.get("checkpoint_id") != work[label]["checkpoint_id"]
                    or row.get("checkpoint_goal_id") != work[label]["goal_id"]
                    or row.get("checkpoint_baseline_id") != row.get("baseline_id")
                    or row.get("checkpoint_decision_ids") != []
                    or row.get("checkpoint_disposition") != "checkpoint_recorded"):
                errors.append(f"Work {label} public authoring/checkpoint")
        if authored["B"].get("baseline_id") == authored["C"].get("baseline_id"):
            errors.append("new Work baseline reused")
    authority = proof["authority"]
    if (not _exact(authority, {"decision_id", "decision_work_id", "attempted_work_id",
                               "rejected", "canonical_before_sha256", "canonical_after_sha256"})
            or authority.get("decision_id") != expected["decision_id"]
            or authority.get("decision_work_id") != work["A"]["goal_id"]
            or authority.get("attempted_work_id") != work["B"]["goal_id"]
            or authority.get("rejected") is not True
            or authority.get("canonical_before_sha256") != authority.get("canonical_after_sha256")
            or not isinstance(authority.get("canonical_before_sha256"), str)
            or len(authority["canonical_before_sha256"]) != 64):
        errors.append("cross-Work authority rejection/atomicity")
    retention = proof["retention"]
    if not _exact(retention, {"prior", "after"}) or not isinstance(retention["prior"], list) or not isinstance(retention["after"], list):
        errors.append("canonical retention shape")
    elif not retention["prior"] or not all(item in retention["after"] for item in retention["prior"]):
        errors.append("prior Work canonical history lost")
    if isinstance(retention, dict):
        for label in ("prior", "after"):
            rows = retention.get(label)
            if not isinstance(rows, list) or len(rows) > 128 or any(
                not isinstance(row, (list, tuple)) or len(row) != 4
                or not isinstance(row[0], str) or not row[0].replace("_", "").islower()
                or len(row[0]) > 32 or not _id(row[1])
                or type(row[2]) is not int or row[2] < 1
                or not isinstance(row[3], list) or len(row[3]) > 64
                or any(not _id(source) for source in row[3])
                for row in rows
            ):
                errors.append(f"{label} canonical identity bound")
    views = proof["views"]
    if not _exact(views, {"before", "cli", "mcp", "portable"}):
        return errors + ["read/portable views shape"]
    purpose_hash = views["before"].get("purpose_sha256") if isinstance(views["before"], dict) else None
    for name, view in views.items():
        if not _exact(view, {"project_id", "purpose_sha256", "history",
                              "current_work_ids", "remaining_work_ids"}):
            errors.append(f"{name} view shape")
            continue
        if (view["project_id"] != project or view["purpose_sha256"] != purpose_hash
                or not isinstance(view["purpose_sha256"], str)
                or len(view["purpose_sha256"]) != 64
                or view["purpose_sha256"] == _digest(None)):
            errors.append(f"{name} Project/Purpose")
        expected_labels = ("A",) if name == "before" else ("A", "B", "C")
        rows = view["history"]
        if not isinstance(rows, list) or len(rows) != len(expected_labels):
            errors.append(f"{name} Work history count")
            continue
        by_id = {row.get("work_id"): row for row in rows if isinstance(row, dict)}
        if len(by_id) != len(expected_labels) or set(by_id) != {
                work[label]["goal_id"] for label in expected_labels}:
            errors.append(f"{name} Work history identity")
            continue
        for label in expected_labels:
            row = by_id[work[label]["goal_id"]]
            state = "in_progress" if label == "C" else "paused"
            if (not _exact(row, {"work_id", "state", "checkpoint_ids", "source_ids"})
                    or row["state"] != state
                    or row["checkpoint_ids"] != [work[label]["checkpoint_id"]]
                    or not isinstance(row["source_ids"], list)
                    or work[label]["source_id"] not in row["source_ids"]
                    or len(row["source_ids"]) > 64
                    or any(not _id(source) for source in row["source_ids"])):
                errors.append(f"{name} Work {label} history/scope")
        current_ids = [] if name == "before" else [work["C"]["goal_id"]]
        remaining_ids = [work["A"]["goal_id"]] if name == "before" else sorted(
            [work["A"]["goal_id"], work["B"]["goal_id"]])
        if view["current_work_ids"] != current_ids or view["remaining_work_ids"] != remaining_ids:
            errors.append(f"{name} current/historical Work")
    return errors


def capsule_technical_errors(official: Any, outcomes: Any = None) -> list[str]:
    """Apply the current technical identity to a sanitized gate capsule."""
    if not isinstance(official, dict):
        return ["official V11 shape"]
    status = official.get("status")
    count = official.get("required_step_count")
    counts = official.get("status_counts")
    if status == "not_run":
        if (official.get("schema_version") is not None
                or official.get("technical_contract") is not None
                or official.get("workload_identity") is not None
                or official.get("required_by_target") not in ({}, None)
                or official.get("multi_work_continuity") is not None):
            return ["not-run V11 carries current evidence"]
        return []
    errors = []
    if (official.get("schema_version") != SCHEMA_VERSION
            or official.get("technical_contract") != TECHNICAL_CONTRACT
            or official.get("workload_identity") != WORKLOAD_IDENTITY):
        errors.append("current V11 schema/contract/workload identity")
    if official.get("required_by_target") != required_counts():
        errors.append("target-specific V11 coverage")
    if type(count) is not int or count != required_total():
        errors.append("V11 required total")
    if not isinstance(counts, dict) or sum(value for value in counts.values()
            if type(value) is int) != required_total():
        errors.append("V11 observed total")
    proof = official.get("multi_work_continuity")
    if proof is not None:
        errors.extend(lifecycle_errors(proof))
    if status == "passed" and proof is None:
        errors.append("passed V11 lacks multi-Work proof")
    if status == "passed" and isinstance(counts, dict):
        if counts.get("passed") != required_total() or any(
            key != "passed" and value != 0 for key, value in counts.items()
        ):
            errors.append("passed V11 leaf statuses contradict coverage")
    if outcomes is not None and proof is not None:
        if (not isinstance(outcomes, list)
                or [item.get("target") for item in outcomes if isinstance(item, dict)] != list(TARGETS)
                or any(not _id(item.get("project_id")) for item in outcomes)
                or not isinstance(proof, dict)
                or outcomes[0].get("project_id") != proof.get("project_id")):
            errors.append("lifecycle Project differs from authenticated target evidence")
    return errors
