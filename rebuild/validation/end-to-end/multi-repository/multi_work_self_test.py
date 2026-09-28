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
                          "work_history": history[:1]},
        "status_after": copy.deepcopy(view), "mcp_understanding": copy.deepcopy(view),
        "canonical_after_a": {"records": before},
        "canonical_after_new_work": {"records": records},
        "cross_work_rejection": rejection,
    }
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
        ("duplicate Work", lambda e, p: e["work"]["C"].update(goal_id=e["work"]["B"]["goal_id"])),
        ("lost A history", lambda e, p: e["status_after"]["work_history"].pop(0)),
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
