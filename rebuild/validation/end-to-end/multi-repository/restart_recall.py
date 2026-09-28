"""V11's expected-versus-observed, bounded same-Work restart oracle."""

from __future__ import annotations

from typing import Any


def _record(inspection: dict[str, Any], kind: str, identity: str) -> dict[str, Any] | None:
    matches = [item for item in inspection.get("records", [])
               if item.get("kind") == kind and item.get("identity") == identity]
    return matches[0] if len(matches) == 1 else None


def canonical_evidence(inspection: dict[str, Any] | None) -> tuple[Any, ...] | None:
    """Identity/revision and provenance, independent of display text and ordering."""
    if not isinstance(inspection, dict) or inspection.get("read_only") is not True:
        return None
    records = inspection.get("records")
    if not isinstance(records, list) or not records:
        return None
    values = []
    for item in records:
        if not isinstance(item, dict) or not isinstance(item.get("identity"), str) or not isinstance(item.get("revision"), int):
            return None
        sources = item.get("source_basis")
        if not isinstance(sources, list) or any(not isinstance(source, str) for source in sources):
            return None
        values.append((item.get("kind"), item["identity"], item["revision"],
                       item.get("lifecycle_state"), item.get("statement_role"), tuple(sorted(sources))))
    return tuple(sorted(values)) if len({(value[0], value[1]) for value in values}) == len(values) else None


def expected_state(project_id: str, binding: dict[str, Any], initial_binding: dict[str, Any],
                   goal_statement: str, goal: dict[str, Any],
                   decision: dict[str, Any], decision_source_id: str,
                   checkpoint: dict[str, Any], analysis: dict[str, Any],
                   current_analysis: dict[str, Any], inspection: dict[str, Any], next_step: str) -> dict[str, Any]:
    """Build only from successful authoring results and a pre-restart canonical read."""
    goal_id = goal.get("context_item_id")
    decision_id = decision.get("identity")
    checkpoint_id = checkpoint.get("checkpoint_id")
    records = {
        "project": _record(inspection, "project", project_id),
        "goal": _record(inspection, "contextitem", goal_id),
        "goal_source": _record(inspection, "source", goal.get("source_id")),
        "decision": _record(inspection, "decision", decision_id),
        "decision_source": _record(inspection, "source", decision_source_id),
        "repository_source": _record(inspection, "source", current_analysis.get("repository_source_id")),
        "checkpoint": _record(inspection, "checkpoint", checkpoint_id),
    }
    if (canonical_evidence(inspection) is None or not all(records.values())
            or not all(isinstance(value, str) and value for value in
                       (project_id, goal_id, decision_id, decision_source_id, checkpoint_id, next_step))
            or not isinstance(binding.get("binding_id"), str) or not binding["binding_id"]
            or not isinstance(binding.get("revision"), int)
            or binding.get("canonical_repository_path") != initial_binding.get("path")
            or any(binding.get(key) != initial_binding.get(key) for key in
                   ("revision", "clone_identity", "worktree_identity"))
            or goal.get("project_id") != project_id
            or records["goal"].get("summary") != goal_statement
            or goal.get("role") != "goal" or goal.get("canonical_mutation") is not True
            or records["goal"].get("statement_role") != "goal"
            or records["goal"].get("revision") != goal.get("revision")
            or goal.get("source_id") not in records["goal"].get("source_basis", [])
            or records["decision"].get("revision") != decision.get("revision")
            or decision_source_id not in records["decision"].get("source_basis", [])
            or records["checkpoint"].get("revision") != checkpoint.get("revision")
            or records["checkpoint"].get("lifecycle_state") != "paused"
            or checkpoint.get("goal_context_id") != goal_id
            or decision_id not in checkpoint.get("applied_decision_ids", [])
            or checkpoint.get("baseline_analysis_snapshot_id") != analysis.get("analysis_snapshot_id")
            or not current_analysis.get("analysis_snapshot_id")
            or not current_analysis.get("repository_snapshot_id")):
        raise ValueError("V11 authoring and pre-restart canonical evidence disagree")
    return {
        "project_id": project_id, "binding": binding,
        "goal_id": goal_id, "goal_revision": goal["revision"],
        "goal_statement": records["goal"]["summary"], "goal_source_id": goal["source_id"],
        "goal_sources": records["goal"]["source_basis"],
        "decision_id": decision_id, "decision_revision": decision["revision"],
        "decision_source_id": decision_source_id,
        "decision_sources": records["decision"]["source_basis"],
        "checkpoint_id": checkpoint_id, "checkpoint_revision": checkpoint["revision"],
        "checkpoint_sources": records["checkpoint"]["source_basis"],
        "changed_paths": checkpoint["changed_paths"],
        "checkpoint_analysis_snapshot_id": checkpoint["current_analysis_snapshot_id"],
        "checkpoint_repository_snapshot_id": checkpoint["current_repository_snapshot_id"],
        "analysis_snapshot_id": current_analysis["analysis_snapshot_id"],
        "repository_snapshot_id": current_analysis["repository_snapshot_id"],
        "repository_source_id": current_analysis["repository_source_id"],
        "next_step": next_step, "canonical": canonical_evidence(inspection),
    }


def recall_errors(expected: dict[str, Any], recall: dict[str, Any] | None) -> list[str]:
    if not isinstance(recall, dict):
        return ["recall missing"]
    errors = []
    def require(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)

    require(recall.get("read_only") is True, "recall read-only state")
    require(recall.get("project_id") == expected["project_id"], "Project identity")
    goals = [item for item in recall.get("goal_basis", []) if isinstance(item, dict)
             and item.get("identity") == expected["goal_id"]]
    require(len(goals) == 1, "Goal/Work identity")
    if len(goals) == 1:
        goal = goals[0]
        require(goal.get("role") == "goal" and goal.get("statement") == expected["goal_statement"], "Goal meaning")
        require(set(goal.get("source_ids", [])) == set(expected["goal_sources"]), "Goal Source basis")
    checkpoint = recall.get("checkpoint")
    require(isinstance(checkpoint, dict), "Checkpoint missing")
    if isinstance(checkpoint, dict):
        for key, value, label in (
            ("identity", expected["checkpoint_id"], "Checkpoint identity"),
            ("revision", expected["checkpoint_revision"], "Checkpoint revision"),
            ("work_item_id", expected["goal_id"], "Checkpoint Work association"),
            ("kind", "handoff", "Checkpoint kind"),
            ("work_state", "paused", "work state"),
            ("next_step", expected["next_step"], "Checkpoint next step"),
        ):
            require(checkpoint.get(key) == value, label)
        require(checkpoint.get("goal") == expected["goal_statement"], "Checkpoint Goal")
        require(set(checkpoint.get("applied_decisions", [])) == {expected["decision_id"]}, "Checkpoint Decision")
        require(set(checkpoint.get("changed_paths", [])) == set(expected["changed_paths"]), "Checkpoint changed paths")
        require(set(expected["checkpoint_sources"]) == set(checkpoint.get("source_basis", [])), "Checkpoint Source basis")
        require(checkpoint.get("verification") is not None and len(checkpoint["verification"]) == 1
                and checkpoint["verification"][0].get("state") == "not_run", "verification state")
        require(isinstance(checkpoint.get("user_review"), dict) and checkpoint["user_review"].get("state") == "not_requested", "review state")
        require(isinstance(checkpoint.get("user_acceptance"), dict) and checkpoint["user_acceptance"].get("state") == "not_requested", "acceptance state")
    decisions = [item for item in recall.get("decisions", []) if isinstance(item, dict)
                 and item.get("identity") == expected["decision_id"]]
    require(len(decisions) == 1, "Decision identity")
    if len(decisions) == 1:
        decision = decisions[0]
        require(decision.get("revision") == expected["decision_revision"], "Decision revision")
        require(decision.get("state") == "current", "Decision applicability")
        require(decision.get("work_scope") == {"kind": "project_wide"}, "Decision work scope")
        require(decision.get("chosen_alternative_key") == "local", "Decision choice")
        require(set(expected["decision_sources"]) <= set(decision.get("source_basis", [])), "Decision Source basis")
        require(expected["decision_source_id"] in decision.get("source_basis", []), "Decision response Source")
    require(recall.get("next_step") == expected["next_step"], "Recall next step")
    require(isinstance(recall.get("active_decision_count"), int) and recall["active_decision_count"] >= 1, "active Decision count")
    sources = recall.get("source_details", [])
    snapshots_section = recall.get("snapshots", [])
    omissions = recall.get("omissions", [])
    def transport_bounded(section: Any) -> bool:
        markers = [section] if isinstance(section, dict) else section if isinstance(section, list) else []
        return any(isinstance(item, dict) and isinstance(item.get("transport_omission"), dict)
                   and item["transport_omission"].get("reason") == "serialized_byte_budget"
                   and (item["transport_omission"].get("omitted_count", 0) > 0
                        or item["transport_omission"].get("exact_json_bytes", 0) > 0)
                   for item in markers)
    source_ids = {item.get("identity") for item in sources if isinstance(item, dict)} if isinstance(sources, list) else set()
    def bounded(identity: str, kind: str) -> bool:
        return any(isinstance(item, dict) and item.get("identity") == identity
                   and item.get("kind") == kind and item.get("reason") == "bound"
                   for item in omissions)
    for identity in (expected["goal_source_id"], expected["decision_source_id"], expected["repository_source_id"]):
        require(identity in source_ids or bounded(identity, "source")
                or transport_bounded(sources), "Source visibility or declared bound")
    snapshots = [item for item in snapshots_section if isinstance(item, dict)
                 and item.get("analysis_snapshot") == expected["analysis_snapshot_id"]]
    require(bool(snapshots) or bounded(expected["analysis_snapshot_id"], "analysis_snapshot")
            or transport_bounded(snapshots_section), "Analysis Snapshot provenance")
    if snapshots:
        require(snapshots[0].get("repository_snapshot") == expected["repository_snapshot_id"], "Repository Snapshot provenance")
        freshness = snapshots[0].get("freshness", {})
        require(isinstance(freshness, dict) and freshness.get("state") == "current"
                and freshness.get("repository_snapshot") == expected["repository_snapshot_id"], "Analysis freshness")
    repository_sources = [item for item in sources if isinstance(item, dict)
                          and item.get("identity") == expected["repository_source_id"]]
    if repository_sources:
        source = repository_sources[0]
        require(source.get("availability") == "available" and source.get("freshness") == "current"
                and isinstance(source.get("snapshot_basis"), str) and bool(source["snapshot_basis"]),
                "Repository Source provenance")
    return errors


def binding_errors(expected: dict[str, Any], resolved: dict[str, Any] | None) -> list[str]:
    if not isinstance(resolved, dict) or resolved.get("status") != "found" or resolved.get("project_id") != expected["project_id"]:
        return ["Project resolution"]
    binding = resolved.get("binding")
    if not isinstance(binding, dict):
        return ["Project binding missing"]
    return [f"Project binding {key}" for key in (
        "binding_id", "revision", "clone_identity", "worktree_identity", "canonical_repository_path"
    ) if binding.get(key) != expected["binding"].get(key)]


def continuation_errors(expected: dict[str, Any], recalled: dict[str, Any],
                        continued: dict[str, Any] | None,
                        canonical_after: dict[str, Any] | None) -> list[str]:
    errors = []
    if not isinstance(continued, dict) or any((
        continued.get("project_id") != expected["project_id"],
        continued.get("context_item_id") != expected["goal_id"],
        continued.get("source_id") != expected["goal_source_id"],
        continued.get("revision") != expected["goal_revision"],
        continued.get("work_transition") != "continue",
        continued.get("canonical_mutation") is not False,
        not any(isinstance(item, dict) and item.get("identity") == expected["goal_id"]
                for item in recalled.get("goal_basis", [])),
    )):
        errors.append("continue did not preserve exact recalled Goal/Source")
    if canonical_evidence(canonical_after) != expected["canonical"]:
        errors.append("read/continue changed canonical identity/revision/provenance")
    return errors


def verify_restart(expected: dict[str, Any], cli_recall: dict[str, Any] | None,
                   mcp_recall: dict[str, Any] | None, resolved: dict[str, Any] | None,
                   canonical_after_read: dict[str, Any] | None,
                   continued: dict[str, Any] | None,
                   canonical_after_continue: dict[str, Any] | None) -> list[str]:
    """The producer's single verdict for both transports and same-Work continuation."""
    errors = [f"CLI: {error}" for error in recall_errors(expected, cli_recall)]
    errors.extend(f"MCP: {error}" for error in recall_errors(expected, mcp_recall))
    errors.extend(binding_errors(expected, resolved))
    if canonical_evidence(canonical_after_read) != expected["canonical"]:
        errors.append("read changed canonical identity/revision/provenance")
    if not isinstance(mcp_recall, dict):
        errors.append("MCP Recall unavailable for continue")
    else:
        errors.extend(continuation_errors(
            expected, mcp_recall, continued, canonical_after_continue
        ))
    return errors
