"""Mutation controls for bounded multi-Work expected/observed evidence."""
from __future__ import annotations

import copy


def fixture():
    project, purpose, decision = "p" * 32, "u" * 32, "d" * 32
    ids = {"A": "a" * 32, "B": "b" * 32, "C": "c" * 32}
    sources = {"A": "1" * 32, "B": "2" * 32, "C": "3" * 32}
    checkpoints = {"A": "4" * 32, "B": "5" * 32, "C": "6" * 32}
    work = {label: {"goal_id": ids[label], "source_id": sources[label],
                    "checkpoint_id": checkpoints[label]} for label in ids}
    work["A"]["decision_id"] = decision
    purpose_rows = [{"statement": "Preserve Project meaning", "source_ids": ["7" * 32]}]
    history = [
        {"work_item_id": ids[label], "source_ids": [sources[label]],
         "checkpoint_ids": [checkpoints[label]],
         "state": "inprogress" if label == "C" else "paused"}
        for label in ids
    ]
    view = {"project_id": project, "project_purpose": purpose_rows,
            "work_history": history, "current_work": [history[2]],
            "remaining_work": history[:2]}
    records = [
        {"kind": "contextitem", "identity": purpose, "revision": 1, "source_basis": ["7" * 32]},
        *[{"kind": "contextitem", "identity": ids[label], "revision": 1,
           "source_basis": [sources[label]]} for label in ids],
    ]
    before = copy.deepcopy(records[:2])
    rejection = {"accepted": False,
        "mismatch": {"checkpoint_work_item_id": ids["B"], "decision_id": decision,
                     "decision_work_item_id": ids["A"]},
        "before": {"records": copy.deepcopy(records[:3])},
        "after": {"records": copy.deepcopy(records[:3])}}
    evidence = {
        "project_id": project, "purpose_id": purpose, "work": work, "errors": [],
        "work_b": {"checkpoint": {"checkpoint_id": checkpoints["B"]}},
        "work_c": {"checkpoint": {"checkpoint_id": checkpoints["C"]}},
        "status_before": {"project_id": project, "project_purpose": purpose_rows,
                          "work_history": history[:1], "current_work": [], "remaining_work": history[:1]},
        "status_after": copy.deepcopy(view), "mcp_understanding": copy.deepcopy(view),
        "canonical_after_a": {"records": before},
        "canonical_after_new_work": {"records": records},
        "cross_work_rejection": rejection,
    }
    evidence["mcp_understanding"]["project_purpose"] = [{
        "context_item_id": purpose, "role": "projectpurpose",
        "statement": purpose_rows[0]["statement"],
        "source_basis": purpose_rows[0]["source_ids"],
    }]
    for row in evidence["mcp_understanding"]["work_history"]:
        row["source_basis"] = row.pop("source_ids")
    evidence["mcp_understanding"]["work_history"][2]["state"] = "in_progress"
    evidence["mcp_understanding"]["current_work"][0]["state"] = "in_progress"
    expected = {"goal_id": ids["A"], "goal_source_id": sources["A"],
                "checkpoint_id": checkpoints["A"], "decision_id": decision}
    return evidence, expected, copy.deepcopy(view)


def self_check(module):
    evidence, expected, portable = fixture()
    assert all(module.verify_rehearsal(evidence, expected, portable).values())
    mutations = (
        ("wrong project", lambda e, p: e["status_after"].update(project_id="wrong")),
        ("changed purpose", lambda e, p: e["status_after"].update(project_purpose=[])),
        ("changed MCP purpose", lambda e, p: e["mcp_understanding"].update(project_purpose=[])),
        ("duplicate Work", lambda e, p: e["work"]["C"].update(goal_id=e["work"]["B"]["goal_id"])),
        ("lost A history", lambda e, p: e["status_after"]["work_history"].pop(0)),
        ("lost MCP source basis", lambda e, p: e["mcp_understanding"]["work_history"][0].pop("source_basis")),
        ("wrong current Work", lambda e, p: e["status_after"].update(current_work=[e["status_after"]["work_history"][1]])),
        ("wrong portable history", lambda e, p: p["work_history"].pop(1)),
        ("cross-Work accepted", lambda e, p: e["cross_work_rejection"].update(accepted=True)),
        ("canonical mutation", lambda e, p: e["cross_work_rejection"]["after"]["records"].pop()),
        ("missing checkpoint", lambda e, p: e["work_b"].update(checkpoint=None)),
        ("missing original Source", lambda e, p: e["canonical_after_new_work"]["records"].pop(1)),
    )
    for label, mutate in mutations:
        changed, current, imported = copy.deepcopy(evidence), copy.deepcopy(expected), copy.deepcopy(portable)
        mutate(changed, imported)
        assert not all(module.verify_rehearsal(changed, current, imported).values()), label


def contract_fixture(contract, project_id="p" * 32):
    """A coherent synthetic raw producer observation for contract mutation tests."""
    evidence, expected_ids, portable = fixture()
    old_project = evidence["project_id"]
    evidence["project_id"] = project_id
    for key in ("status_before", "status_after", "mcp_understanding"):
        evidence[key]["project_id"] = project_id
    portable["project_id"] = project_id
    evidence["portable_status"] = portable
    a, b, c = [evidence["work"][label] for label in ("A", "B", "C")]
    for label, baseline in (("B", "8" * 32), ("C", "9" * 32)):
        row = evidence["work"][label]
        evidence[f"work_{label.lower()}"] = {
            "goal": {"project_id": project_id, "context_item_id": row["goal_id"],
                     "source_id": row["source_id"], "work_transition": "start_new",
                     "canonical_mutation": True},
            "baseline": {"analysis_snapshot_id": baseline},
            "ready": {"workflow": {"stage": "ready_for_work"}},
            "checkpoint": {"checkpoint_id": row["checkpoint_id"],
                           "goal_context_id": row["goal_id"],
                           "baseline_analysis_snapshot_id": baseline,
                           "applied_decision_ids": [],
                           "workflow": {"disposition": "checkpoint_recorded"}},
        }
    scope = {"kind": "work_item", "work_item_id": a["goal_id"]}
    recall = {
        "project_id": project_id,
        "goal_basis": [{"identity": a["goal_id"], "source_ids": [a["source_id"]]}],
        "decisions": [{"identity": expected_ids["decision_id"], "revision": 1,
                       "work_scope": scope}],
        "checkpoint": {"identity": a["checkpoint_id"], "revision": 1,
                       "work_item_id": a["goal_id"],
                       "applied_decisions": [expected_ids["decision_id"]]},
    }
    restart = {
        "expected_state": {
            "project_id": project_id,
            "goal_id": a["goal_id"], "goal_source_id": a["source_id"],
            "goal_revision": 1,
            "decision_id": expected_ids["decision_id"],
            "decision_revision": 1, "decision_work_scope": scope,
            "checkpoint_id": a["checkpoint_id"], "checkpoint_revision": 1,
        },
        "cli_recall": copy.deepcopy(recall),
        "restarted_recall": copy.deepcopy(recall),
        "continuation": {
            "project_id": project_id, "context_item_id": a["goal_id"],
            "source_id": a["source_id"], "revision": 1,
            "work_transition": "continue", "canonical_mutation": False,
        },
    }
    proof = contract.make_lifecycle_proof(evidence, restart)
    assert not contract.lifecycle_errors(proof), contract.lifecycle_errors(proof)
    return evidence, restart, proof


def contract_self_check(contract):
    _, _, original = contract_fixture(contract)
    mutations = (
        ("wrong Project", lambda p: p["views"]["cli"].update(project_id="wrong")),
        ("wrong Work", lambda p: p["views"]["portable"]["history"][0].update(work_id="wrong")),
        ("wrong Decision", lambda p: p["restart"]["mcp"].update(decision_id="wrong")),
        ("wrong Checkpoint", lambda p: p["restart"]["cli"].update(checkpoint_id="wrong")),
        ("missing restart", lambda p: p["restart"].pop("mcp")),
        ("both transports wrong", lambda p: (
            p["restart"]["cli"].update(goal_id="wrong"),
            p["restart"]["mcp"].update(goal_id="wrong"))),
        ("continue made new Work", lambda p: p["restart"]["continuation"].update(goal_id=p["work"]["B"]["goal_id"])),
        ("duplicate A/B", lambda p: p["work"]["B"].update(goal_id=p["work"]["A"]["goal_id"])),
        ("lost prior history", lambda p: p["retention"].update(after=[])),
        ("changed purpose", lambda p: p["views"]["portable"].update(purpose_sha256="0" * 64)),
        ("missing MCP purpose", lambda p: p["views"]["mcp"].update(purpose_sha256=None)),
        ("wrong current Work", lambda p: p["views"]["cli"].update(current_work_ids=[p["work"]["B"]["goal_id"]])),
        ("cross-Work accepted", lambda p: p["authority"].update(rejected=False)),
        ("cross-Work mutated canonical", lambda p: p["authority"].update(canonical_after_sha256="0" * 64)),
        ("missing new baseline", lambda p: p["authored"]["C"].update(baseline_id=None)),
        ("passed without B checkpoint", lambda p: p["authored"]["B"].update(checkpoint_id=None)),
        ("unbounded user prose", lambda p: p["retention"]["after"].append(
            ["source", "/home/private/source", 1, []])),
    )
    for label, mutate in mutations:
        changed = copy.deepcopy(original)
        mutate(changed)
        assert contract.lifecycle_errors(changed), label
    assert contract.required_counts() == {
        target: len(contract.required_steps_for_target(target))
        for target in contract.TARGETS
    }
    assert len(contract.COMMON_STEPS) == 18
    assert not set(contract.VOLICORD_EXTRA) & set(contract.COMMON_STEPS)
    try:
        contract.required_steps_for_target("unknown")
    except ValueError:
        pass
    else:
        raise AssertionError("unknown V11 target accepted")
