"""Deterministic mutations against the V11 restart producer's oracle functions."""

from __future__ import annotations

import copy

def fixture(oracle):
    project, goal, goal_source = "p" * 32, "g" * 32, "s" * 32
    decision, decision_source = "d" * 32, "u" * 32
    checkpoint, analysis, repository = "c" * 32, "a" * 32, "r" * 32
    repository_source = "t" * 32
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
        canonical, "Continue bounded work")
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
        "reason": "serialized_byte_budget", "omitted_count": 3}}]
    transport_bound["snapshots"] = [{"transport_omission": {
        "reason": "serialized_byte_budget", "omitted_count": 1}}]
    assert not oracle.recall_errors(expected, transport_bound)
    reordered = copy.deepcopy(canonical)
    reordered["records"].reverse()
    reordered["records"][0]["summary"] = "Presentation changed"
    assert not oracle.continuation_errors(expected, recall, continued, reordered)

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
        ("next step", lambda value: value.update(next_step="wrong")),
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
        # Identical corruption in both transports cannot turn either into expected state.
        assert oracle.verify_restart(expected, broken, copy.deepcopy(broken), resolved,
                                     canonical, continued, canonical), f"both transports: {label}"
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
