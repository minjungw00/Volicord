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
                   current_analysis: dict[str, Any], inspection: dict[str, Any], next_step: str,
                   decision_work_scope: dict[str, Any] | None = None,
                   next_step_claims: list[list[str]] | None = None) -> dict[str, Any]:
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
        "decision_work_scope": decision_work_scope or {"kind": "project_wide"},
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
        "next_step": next_step, "next_step_claims": next_step_claims or [],
        "canonical": canonical_evidence(inspection),
    }


def shared_answer_errors(expected: dict[str, Any], recall: dict[str, Any]) -> list[str]:
    """Known-fixture direction and basis, not a general generated-prose grader.

    Expectations come from canonical authoring, never work_answers or the observed
    transport. Generated wording may vary within independently authored claim groups.
    No transport omission can substitute for this minimum continuation meaning.
    """
    errors = []
    def require(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)
    require(recall.get("next_step") == expected["next_step"], "top-level recorded next action")
    work = recall.get("selected_work")
    if not isinstance(work, dict):
        return errors + ["selected Work missing"]
    require(work.get("work_item_id") == expected["goal_id"], "selected Work identity")
    checkpoints = work.get("checkpoint_ids")
    require(isinstance(checkpoints, list) and expected["checkpoint_id"] in checkpoints, "selected Work Checkpoint scope")
    answers = work.get("answers")
    if not isinstance(answers, dict):
        return errors + ["Recall question answers"]
    facts, prose = answers.get("facts"), answers.get("prose")
    if not isinstance(facts, list) or not isinstance(prose, list):
        return errors + ["shared answer sections"]
    def question(items: list[Any], name: str) -> list[dict[str, Any]]:
        return [a for a in items if isinstance(a, dict) and a.get("question") == name]
    action_key = f"checkpoint:{expected['checkpoint_id']}@{expected['checkpoint_revision']}:next_step"
    goal_key = f"context_item:{expected['goal_id']}@{expected['goal_revision']}:statement"
    recorded = question(facts, "RecordedNextStep")
    require(len(recorded) == 1, "recorded next action answer")
    require(not question(facts, "NextStepAvailability"), "valid direction declared missing")
    require(not question(prose, "RecordedNextStep") and not question(facts, "NextStep"), "recorded action role/scope")
    if len(recorded) == 1:
        fact = recorded[0]
        require(fact.get("role") == "deterministic_facts", "recorded action role")
        require(fact.get("evidence_keys") == [action_key], "recorded action evidence key")
        require(fact.get("text") in [
            f"Recorded next action quotation (original language): {expected['next_step']}",
            f"기록된 다음 행동 인용 (원문 언어): {expected['next_step']}",
        ], "ordinary recorded next action meaning")
        basis = fact.get("recorded_action")
        require(isinstance(basis, dict), "recorded action basis missing")
        if isinstance(basis, dict):
            for field, value in (
                ("work_item_id", expected["goal_id"]), ("checkpoint_id", expected["checkpoint_id"]),
                ("revision", expected["checkpoint_revision"]), ("field", "next_step"),
                ("recorded_text", expected["next_step"]),
            ):
                require(basis.get(field) == value and (field != "revision" or type(basis.get(field)) is int), f"recorded action {field}")
            sources = basis.get("source_ids")
            require(isinstance(sources, list) and all(isinstance(s, str) for s in sources)
                    and sorted(sources) == sorted(expected["checkpoint_sources"]), "recorded action Source basis")
            statuses = basis.get("source_status")
            require(isinstance(statuses, list) and len(statuses) == len(expected["checkpoint_sources"])
                    and all(isinstance(s, dict) and isinstance(s.get("source_id"), str) for s in statuses)
                    and sorted(s["source_id"] for s in statuses) == sorted(expected["checkpoint_sources"]), "recorded action Source status scope")
            if isinstance(statuses, list):
                source_details = recall.get("source_details", [])
                for status in statuses:
                    if not isinstance(status, dict):
                        continue
                    visible = [s for s in source_details if isinstance(s, dict) and s.get("identity") == status.get("source_id")] if isinstance(source_details, list) else []
                    for source in visible:
                        for field in ("availability", "freshness", "snapshot_basis"):
                            if field in source:
                                require(status.get(field) == source[field], f"recorded action Source {field}")
    states = question(facts, "WorkState")
    require(len(states) == 1 and states[0].get("role") == "deterministic_facts"
            and states[0].get("text") in ("Work: paused", "작업: 일시 중지")
            and states[0].get("evidence_keys") == [f"checkpoint:{expected['checkpoint_id']}@{expected['checkpoint_revision']}:work_state"], "shared Work state basis")
    generated = question(prose, "NextStep")
    state = answers.get("explanation_state")
    require(state in ("current", "unavailable", "stale", "corrupt", "unsupported"), "explanation state")
    if state == "current":
        require(len(generated) == 1, "current next-step interpretation")
        provenance = answers.get("provenance")
        require(isinstance(provenance, dict), "generated provenance missing")
        if isinstance(provenance, dict):
            require(provenance.get("project_id") == expected["project_id"]
                    and provenance.get("subject") == {"kind": "work", "identity": expected["goal_id"]}, "generated Work/Project scope")
            evidence = provenance.get("evidence", [])
            for key, identity, revision, field, sources in (
                ("next_step", expected["checkpoint_id"], expected["checkpoint_revision"], "next_step", expected["checkpoint_sources"]),
                ("goal", expected["goal_id"], expected["goal_revision"], "statement", expected["goal_sources"]),
            ):
                matches = [e for e in evidence if isinstance(e, dict) and e.get("key") == key] if isinstance(evidence, list) else []
                require(len(matches) == 1 and matches[0].get("identity") == identity
                        and matches[0].get("revision") == revision and matches[0].get("field") == field
                        and isinstance(matches[0].get("sources"), list)
                        and all(isinstance(s, str) for s in matches[0]["sources"])
                        and sorted(matches[0]["sources"]) == sorted(sources), f"generated {key} basis")
        if len(generated) == 1:
            paragraph = generated[0]
            keys = paragraph.get("evidence_keys")
            require(paragraph.get("role") == "generated_interpretation"
                    and isinstance(keys, list) and "next_step" in keys, "generated next-step role/evidence")
            text = paragraph.get("text")
            claims = expected.get("next_step_claims", [])
            require(isinstance(text, str) and bool(text.strip()) and bool(claims)
                    and all(any(term.casefold() in text.casefold() for term in group) for group in claims), "known-fixture generated next-step meaning")
    else:
        require(not generated and answers.get("provenance") is None, "unusable generated direction revived")
        availability = question(prose, "ExplanationAvailability")
        require(len(availability) == 1 and availability[0].get("role") == "unavailable", "separate explanation availability")
        goals = question(facts, "RecordedGoal")
        require(len(goals) == 1 and goals[0].get("role") == "deterministic_facts"
                and goals[0].get("text") in (f"Recorded Goal quotation: {expected['goal_statement']}", f"기록된 목표 인용: {expected['goal_statement']}")
                and goals[0].get("evidence_keys") == [goal_key], "shared Goal revision basis")
    return errors


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
        require(decision.get("work_scope") == expected["decision_work_scope"], "Decision work scope")
        require(decision.get("chosen_alternative_key") == "local", "Decision choice")
        require(set(expected["decision_sources"]) <= set(decision.get("source_basis", [])), "Decision Source basis")
        require(expected["decision_source_id"] in decision.get("source_basis", []), "Decision response Source")
    errors.extend(shared_answer_errors(expected, recall))
    require(isinstance(recall.get("active_decision_count"), int) and recall["active_decision_count"] >= 1, "active Decision count")
    sources = recall.get("source_details", [])
    snapshots_section = recall.get("snapshots", [])
    omissions = recall.get("omissions", [])
    def transport_bounded(section: Any) -> bool:
        # Scope comes from the containing Source/snapshot field. Only the maintained
        # complete-field or stable-array-suffix marker has this permission.
        markers = [section] if isinstance(section, dict) else section if isinstance(section, list) else []
        for item in markers:
            marker = item.get("transport_omission") if isinstance(item, dict) else None
            if not isinstance(marker, dict) or marker.get("reason") != "serialized_byte_budget":
                continue
            suffix = (isinstance(section, list)
                      and marker.get("basis") == "same parent identity, field and stable input order; inspect the authoritative record"
                      and type(marker.get("omitted_count")) is int and marker["omitted_count"] > 0)
            whole = (isinstance(section, dict)
                     and marker.get("basis") == "inspect the complete field on the authoritative parent record"
                     and type(marker.get("exact_json_bytes")) is int and marker["exact_json_bytes"] > 0)
            if suffix or whole:
                return True
        return False
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
