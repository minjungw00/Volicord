"""Deterministic mutations against the V11 restart producer's oracle functions."""

from __future__ import annotations

import copy

def fixture(oracle):
    project, goal, goal_source = "01" * 16, "02" * 16, "03" * 16
    decision, decision_source = "04" * 16, "05" * 16
    checkpoint, analysis, repository = "06" * 16, "07" * 16, "08" * 16
    repository_source = "09" * 16
    records = [
        {"kind": "project", "identity": project, "revision": 1, "lifecycle_state": "current", "statement_role": None, "summary": "Project", "source_basis": []},
        {"kind": "source", "identity": goal_source, "revision": 1, "lifecycle_state": "current", "statement_role": "source_basis", "summary": "Goal Source", "source_basis": [goal_source]},
        {"kind": "source", "identity": decision_source, "revision": 1, "lifecycle_state": "current", "statement_role": "source_basis", "summary": "Decision Source", "source_basis": [decision_source]},
        {"kind": "source", "identity": repository_source, "revision": 1, "lifecycle_state": "current", "statement_role": "source_basis", "summary": "Repository Source", "source_basis": [repository_source]},
        {"kind": "contextitem", "identity": goal, "revision": 1, "lifecycle_state": "current", "statement_role": "goal", "summary": "Do bounded work", "source_basis": [goal_source]},
        {"kind": "decision", "identity": decision, "revision": 1, "lifecycle_state": "active", "statement_role": "user_judgment", "summary": "local", "source_basis": [decision_source]},
        {"kind": "checkpoint", "identity": checkpoint, "revision": 1, "lifecycle_state": "paused", "statement_role": "source_grounded_checkpoint", "summary": "Do bounded work", "source_basis": [goal_source, decision_source]},
    ]
    canonical = {"read_only": True, "records": records}
    binding = {"binding_id": "b" * 32, "revision": 1, "clone_identity": "clone", "worktree_identity": "tree", "canonical_repository_path": "/tmp/work"}
    authored_goal = {"project_id": project, "context_item_id": goal, "source_id": goal_source, "revision": 1, "role": "goal", "canonical_mutation": True}
    authored_checkpoint = {"checkpoint_id": checkpoint, "revision": 1, "goal_context_id": goal,
                           "applied_decision_ids": [decision], "baseline_analysis_snapshot_id": analysis,
                           "current_analysis_snapshot_id": analysis, "current_repository_snapshot_id": repository,
                           "changed_paths": ["v11-ordinary-work.txt"]}
    expected = oracle.expected_state(project, binding,
        {"path": "/tmp/work", "revision": 1, "clone_identity": "clone", "worktree_identity": "tree"},
        "Do bounded work", authored_goal,
        {"identity": decision, "revision": 1}, decision_source, authored_checkpoint,
        {"analysis_snapshot_id": analysis},
        {"analysis_snapshot_id": analysis, "repository_snapshot_id": repository, "repository_source_id": repository_source},
        canonical, "Continue bounded work", next_step_claims=[["continue", "resume"], ["bounded"], ["work"]])
    recall = {
        "read_only": True, "project_id": project,
        "goal_basis": [{"identity": goal, "role": "goal", "statement": "Do bounded work", "source_ids": [goal_source]}],
        "checkpoint": {"identity": checkpoint, "revision": 1, "work_item_id": goal, "kind": "handoff",
                       "work_state": "paused", "next_step": "Continue bounded work", "goal": "Do bounded work",
                       "changed_paths": ["v11-ordinary-work.txt"], "applied_decisions": [decision],
                       "source_basis": [goal_source, decision_source], "verification": [{"state": "not_run"}],
                       "user_review": {"state": "not_requested"}, "user_acceptance": {"state": "not_requested"},
                       "recorded_at_unix_micros": 1},
        "decisions": [{"identity": decision, "revision": 1, "state": "current", "work_scope": {"kind": "project_wide"},
                       "chosen_alternative_key": "local", "source_basis": [decision_source]}],
        "selected_work": {"work_item_id": goal, "checkpoint_ids": [checkpoint], "answers": {
            "explanation_state": "unavailable", "provenance": None, "diagnostic": None,
            "prose": [{"question": "ExplanationAvailability", "role": "unavailable", "text": "Interpretation has not been generated", "evidence_keys": []}],
            "facts": [
                {"question": "RecordedNextStep", "role": "deterministic_facts",
                 "text": "Recorded next action quotation (original language): Continue bounded work",
                 "evidence_keys": [f"checkpoint:{checkpoint}@1:next_step"],
                 "recorded_action": {"work_item_id": goal, "checkpoint_id": checkpoint, "revision": 1,
                                     "field": "next_step", "recorded_text": "Continue bounded work",
                                     "source_ids": [goal_source, decision_source],
                                     "source_status": [{"source_id": sid, "availability": "available", "freshness": "current", "snapshot_basis": None} for sid in [goal_source, decision_source]]}},
                {"question": "WorkState", "role": "deterministic_facts", "text": "Work: paused", "evidence_keys": [f"checkpoint:{checkpoint}@1:work_state"]},
                {"question": "RecordedGoal", "role": "deterministic_facts", "text": "Recorded Goal quotation: Do bounded work", "evidence_keys": [f"context_item:{goal}@1:statement"]},
            ]}},
        "active_decision_count": 1, "next_step": "Continue bounded work",
        "source_details": [{"identity": goal_source}, {"identity": decision_source},
                           {"identity": repository_source, "availability": "available",
                            "freshness": "current", "snapshot_basis": "observed"}],
        "snapshots": [{"analysis_snapshot": analysis, "repository_snapshot": repository,
                       "freshness": {"state": "current", "repository_snapshot": repository}}], "omissions": [],
    }
    resolution = {"status": "found", "project_id": project, "binding": binding}
    continued = {"project_id": project, "context_item_id": goal, "source_id": goal_source,
                 "revision": 1, "work_transition": "continue", "canonical_mutation": False}
    return expected, recall, resolution, continued, canonical


def self_check(oracle) -> None:
    expected, recall, resolved, continued, canonical = fixture(oracle)
    assert not oracle.recall_errors(expected, recall)
    assert not oracle.binding_errors(expected, resolved)
    assert not oracle.continuation_errors(expected, recall, continued, canonical)
    assert not oracle.verify_restart(expected, recall, copy.deepcopy(recall),
                                     resolved, canonical, continued, canonical)

    # Ordering, local path, time, and a declared bound on a Source are presentation differences.
    variant = copy.deepcopy(recall)
    variant["source_details"].reverse()
    variant["checkpoint"]["source_basis"].reverse()
    variant["checkpoint"]["recorded_at_unix_micros"] = 987654321
    omitted_source = variant["source_details"].pop()["identity"]
    variant["omissions"] = [{"identity": omitted_source, "reason": "bound", "kind": "source"}]
    assert not oracle.recall_errors(expected, variant)
    bounded_snapshot = copy.deepcopy(recall)
    bounded_snapshot["snapshots"] = []
    bounded_snapshot["omissions"] = [{"identity": expected["analysis_snapshot_id"],
                                      "reason": "bound", "kind": "analysis_snapshot"}]
    assert not oracle.recall_errors(expected, bounded_snapshot)
    transport_bound = copy.deepcopy(recall)
    transport_bound["source_details"] = [{"transport_omission": {
        "reason": "serialized_byte_budget", "omitted_count": 3,
        "basis": "same parent identity, field and stable input order; inspect the authoritative record"}}]
    transport_bound["snapshots"] = [{"transport_omission": {
        "reason": "serialized_byte_budget", "omitted_count": 1,
        "basis": "same parent identity, field and stable input order; inspect the authoritative record"}}]
    assert not oracle.recall_errors(expected, transport_bound)
    reordered = copy.deepcopy(canonical)
    reordered["records"].reverse()
    reordered["records"][0]["summary"] = "Presentation changed"
    assert not oracle.continuation_errors(expected, recall, continued, reordered)

    # These facts are authored here from the fixture setup, never copied from a
    # product answer. Generated wording is allowed to vary in the known scope.
    current = copy.deepcopy(recall)
    current["selected_work"]["answers"].update(
        explanation_state="current",
        prose=[{"question": "NextStep", "role": "generated_interpretation",
                "text": "Resume the bounded work.", "evidence_keys": ["next_step"]}],
        provenance={"project_id": expected["project_id"],
                    "subject": {"kind": "work", "identity": list(bytes.fromhex(expected["goal_id"]))},
                    "evidence": [
                        {"key": "next_step", "identity": expected["checkpoint_id"], "revision": 1, "field": "next_step", "sources": expected["checkpoint_sources"]},
                        {"key": "goal", "identity": expected["goal_id"], "revision": 1, "field": "statement", "sources": expected["goal_sources"]},
                    ]})
    for positive in [recall, current]:
        assert not oracle.verify_restart(expected, positive, copy.deepcopy(positive), resolved, canonical, continued, canonical)
    for state in ("stale", "corrupt", "unsupported"):
        variant = copy.deepcopy(recall)
        variant["selected_work"]["answers"]["explanation_state"] = state
        variant["selected_work"]["answers"]["prose"][0]["text"] = "Prepare and regenerate the explanation."
        assert not oracle.verify_restart(expected, variant, copy.deepcopy(variant), resolved, canonical, continued, canonical)
    for text in ("Continue the bounded work in this session.", "Resume bounded work now."):
        variant = copy.deepcopy(current)
        variant["selected_work"]["answers"]["prose"][0]["text"] = text
        assert not oracle.recall_errors(expected, variant)
    reordered_answers = copy.deepcopy(recall)
    reordered_answers["selected_work"]["answers"]["facts"].reverse()
    assert not oracle.recall_errors(expected, reordered_answers)
    whole_bound = copy.deepcopy(recall)
    for field in ("source_details", "snapshots"):
        whole_bound[field] = {"transport_omission": {"reason": "serialized_byte_budget", "exact_json_bytes": 90000,
            "basis": "inspect the complete field on the authoritative parent record"}}
    assert not oracle.recall_errors(expected, whole_bound)

    def action(value):
        return next(a for a in value["selected_work"]["answers"]["facts"] if a["question"] == "RecordedNextStep")
    def unrelated(value):
        value["next_step"] = "Ship another Work's CSV service."
        action(value)["text"] = "Recorded next action quotation (original language): Ship another Work's CSV service."
        action(value)["recorded_action"]["recorded_text"] = value["next_step"]
    def other_work(value):
        unrelated(value)
        value["selected_work"]["work_item_id"] = "other Work"
        action(value)["recorded_action"]["work_item_id"] = "other Work"
    controls = (
        ("top-level recorded next action", recall, lambda v: v.update(next_step="Delete unrelated data")),
        ("selected Work identity", recall, other_work),
        ("ordinary recorded next action meaning", recall, lambda v: action(v).update(text="Ship another Work's CSV service")),
        ("top-level recorded next action", recall, unrelated),
        ("recorded action work_item_id", recall, lambda v: action(v)["recorded_action"].update(work_item_id="other Work")),
        ("recorded action checkpoint_id", recall, lambda v: action(v)["recorded_action"].update(checkpoint_id="other Checkpoint")),
        ("recorded action revision", recall, lambda v: action(v)["recorded_action"].update(revision=2)),
        ("recorded action revision", recall, lambda v: action(v)["recorded_action"].update(revision=True)),
        ("recorded action revision", recall, lambda v: action(v)["recorded_action"].pop("revision")),
        ("recorded action evidence key", recall, lambda v: action(v).update(evidence_keys=[])),
        ("recorded action evidence key", recall, lambda v: action(v).update(evidence_keys=["checkpoint:other@1:next_step"])),
        ("recorded action basis missing", recall, lambda v: action(v).pop("recorded_action")),
        ("recorded action field", recall, lambda v: action(v)["recorded_action"].update(field="state_change")),
        ("recorded action Source basis", recall, lambda v: action(v)["recorded_action"].update(source_ids=[])),
        ("recorded action Source status scope", recall, lambda v: action(v)["recorded_action"].update(source_status=[])),
        ("recorded action role", recall, lambda v: action(v).update(role="generated_interpretation")),
        ("top-level recorded next action", recall, lambda v: v.update(next_step="Prepare and regenerate the explanation.")),
        ("recorded next action answer", recall, lambda v: action(v).update(question="ExplanationAvailability")),
        ("shared Work state basis", recall, lambda v: v["selected_work"]["answers"]["facts"][1].update(evidence_keys=[])),
        ("shared Work state basis", recall, lambda v: v["selected_work"]["answers"]["facts"][1].update(text="Work: completed")),
        ("shared Goal revision basis", recall, lambda v: v["selected_work"]["answers"]["facts"][2].update(text="Recorded Goal quotation: Ship the other service")),
        ("shared Goal revision basis", recall, lambda v: v["selected_work"]["answers"]["facts"][2].update(evidence_keys=[])),
        ("selected Work Checkpoint scope", recall, lambda v: v["selected_work"].update(checkpoint_ids=["other"])),
        ("known-fixture generated next-step meaning", current, lambda v: v["selected_work"]["answers"]["prose"][0].update(text="Deploy another Work's CSV service.")),
        ("known-fixture generated next-step meaning", current, lambda v: v["selected_work"]["answers"]["prose"][0].update(text="Prepare and regenerate the explanation.")),
        ("generated next-step role/evidence", current, lambda v: v["selected_work"]["answers"]["prose"][0].update(evidence_keys=[])),
        ("generated next-step role/evidence", current, lambda v: v["selected_work"]["answers"]["prose"][0].update(evidence_keys=None)),
        ("generated next_step basis", current, lambda v: v["selected_work"]["answers"]["provenance"]["evidence"][0].update(sources=None)),
        ("recorded action role/scope", recall, lambda v: v["selected_work"]["answers"]["facts"].append({"question":"NextStep", "role":"deterministic_facts", "text":"Ship the other service"})),
        ("generated next_step basis", current, lambda v: v["selected_work"]["answers"]["provenance"]["evidence"][0].update(revision=2)),
        ("generated Work/Project scope", current, lambda v: v["selected_work"]["answers"]["provenance"].update(project_id="other Project")),
        ("generated Work/Project scope", current, lambda v: v["selected_work"]["answers"]["provenance"]["subject"].update(identity="other Work")),
        ("unusable generated direction revived", current, lambda v: v["selected_work"]["answers"].update(explanation_state="stale")),
        ("Source visibility or declared bound", recall, lambda v: v.update(source_details=[{"transport_omission": {"reason":"serialized_byte_budget", "omitted_count":3}}])),
        ("recorded action work_item_id", recall, lambda v: (action(v)["recorded_action"].update(work_item_id="other Work"), v.update(omissions=[{"transport_omission":{"reason":"serialized_byte_budget","omitted_count":1}}]))),
    )
    for message, positive, mutate in controls:
        broken = copy.deepcopy(positive)
        mutate(broken)
        assert message in oracle.recall_errors(expected, broken), message
        for cli, mcp, prefixes in ((broken, positive, ["CLI"]), (positive, broken, ["MCP"]), (broken, copy.deepcopy(broken), ["CLI", "MCP"])):
            errors = oracle.verify_restart(expected, cli, mcp, resolved, canonical, continued, canonical)
            assert all(f"{prefix}: {message}" in errors for prefix in prefixes), (message, prefixes, errors)

    mutations = (
        ("Project", lambda value: value.update(project_id="wrong")),
        ("Goal/Work", lambda value: value["goal_basis"][0].update(identity="wrong")),
        ("missing Goal", lambda value: value.update(goal_basis=[])),
        ("Checkpoint", lambda value: value["checkpoint"].update(identity="wrong")),
        ("Checkpoint Work", lambda value: value["checkpoint"].update(work_item_id="wrong")),
        ("missing Checkpoint", lambda value: value.update(checkpoint=None)),
        ("Decision", lambda value: value["decisions"][0].update(identity="wrong")),
        ("Decision revision", lambda value: value["decisions"][0].update(revision=2)),
        ("Decision scope", lambda value: value["decisions"][0].update(work_scope={"kind": "unresolved"})),
        ("Decision applicability", lambda value: value["decisions"][0].update(state="review_required")),
        ("Decision Source", lambda value: value["decisions"][0].update(source_basis=[])),
        ("Goal Source", lambda value: value["goal_basis"][0].update(source_ids=[])),
        ("missing Source detail", lambda value: value.update(source_details=[
            item for item in value["source_details"]
            if item["identity"] != expected["goal_source_id"]])),
        ("wrong Source omission", lambda value: (value.update(source_details=[
            item for item in value["source_details"]
            if item["identity"] != expected["goal_source_id"]]),
            value.update(omissions=[{"identity": expected["goal_source_id"],
                                     "reason": "bound", "kind": "decision"}]))),
        ("Checkpoint next step", lambda value: value["checkpoint"].update(next_step="wrong")),
        ("shared answers", lambda value: value["selected_work"].update(answers=None)),
        ("work state", lambda value: value["checkpoint"].update(work_state="completed")),
        ("verification", lambda value: value["checkpoint"].update(verification=[])),
        ("review", lambda value: value["checkpoint"]["user_review"].update(state="reviewed")),
        ("acceptance", lambda value: value["checkpoint"]["user_acceptance"].update(state="accepted")),
        ("analysis Source", lambda value: value.update(snapshots=[])),
        ("repository revision", lambda value: value["snapshots"][0].update(repository_snapshot="wrong")),
        ("analysis freshness", lambda value: value["snapshots"][0]["freshness"].update(state="stale")),
        ("Repository Source freshness", lambda value: next(item for item in value["source_details"]
            if item["identity"] == expected["repository_source_id"]).update(freshness="stale")),
    )
    for label, mutate in mutations:
        broken = copy.deepcopy(recall)
        mutate(broken)
        assert oracle.recall_errors(expected, broken), label
        for cli, mcp in ((broken, recall), (recall, broken), (broken, copy.deepcopy(broken))):
            assert oracle.verify_restart(expected, cli, mcp, resolved,
                                         canonical, continued, canonical), label
    bad_binding = copy.deepcopy(resolved)
    bad_binding["binding"]["binding_id"] = "wrong"
    assert oracle.binding_errors(expected, bad_binding)
    assert oracle.verify_restart(expected, recall, recall, bad_binding, canonical, continued, canonical)
    for field, replacement in (("context_item_id", "new goal"), ("source_id", "new source"),
                               ("revision", 2), ("canonical_mutation", True)):
        bad_continue = copy.deepcopy(continued)
        bad_continue[field] = replacement
        assert oracle.verify_restart(expected, recall, recall, resolved, canonical,
                                     bad_continue, canonical), field
    for kind in ("contextitem", "source", "decision", "checkpoint"):
        bad_canonical = copy.deepcopy(canonical)
        next(item for item in bad_canonical["records"] if item["kind"] == kind)["revision"] += 1
        assert oracle.verify_restart(expected, recall, recall, resolved, canonical,
                                     continued, bad_canonical), kind
    missing_source_revision = copy.deepcopy(canonical)
    next(item for item in missing_source_revision["records"] if item["kind"] == "source")["revision"] = 0
    assert oracle.verify_restart(expected, recall, recall, resolved, missing_source_revision,
                                 continued, canonical)
    assert oracle.verify_restart(expected, recall, recall, resolved, canonical,
                                 continued, missing_source_revision)
    bad_authored = copy.deepcopy(canonical)
    next(item for item in bad_authored["records"] if item["kind"] == "contextitem")["source_basis"] = []
    try:
        oracle.expected_state(expected["project_id"], expected["binding"],
            {"path": "/tmp/work", "revision": 1, "clone_identity": "clone", "worktree_identity": "tree"},
            "Do bounded work", {"project_id": expected["project_id"], "context_item_id": expected["goal_id"], "source_id": expected["goal_source_id"],
             "revision": 1, "role": "goal", "canonical_mutation": True},
            {"identity": expected["decision_id"], "revision": 1}, expected["decision_source_id"],
            {"checkpoint_id": expected["checkpoint_id"], "revision": 1, "goal_context_id": expected["goal_id"],
             "applied_decision_ids": [expected["decision_id"]],
             "baseline_analysis_snapshot_id": expected["analysis_snapshot_id"],
             "current_analysis_snapshot_id": expected["analysis_snapshot_id"],
             "current_repository_snapshot_id": expected["repository_snapshot_id"],
             "changed_paths": ["v11-ordinary-work.txt"]},
            {"analysis_snapshot_id": expected["analysis_snapshot_id"]},
            {"analysis_snapshot_id": expected["analysis_snapshot_id"],
             "repository_snapshot_id": expected["repository_snapshot_id"],
             "repository_source_id": expected["repository_source_id"]},
            bad_authored, expected["next_step"])
    except ValueError:
        pass
    else:
        raise AssertionError("pre-restart contradictory canonical basis was accepted")
