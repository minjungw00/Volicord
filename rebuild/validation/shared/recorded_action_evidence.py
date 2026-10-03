"""Factual shared-answer invariants; generated prose requires semantic review.

Expected values must come from independent authoring/observation-time records,
never from the returned answer. This module has no fixture-specific task meaning.
"""


def recorded_action_errors(expected, recall):
    errors = []
    def require(ok, message):
        if not ok:
            errors.append(message)
    require(recall.get("project_id") == expected["project_id"], "Project identity")
    require(recall.get("next_step") == expected["next_step"], "top-level recorded next action")
    work = recall.get("selected_work")
    if not isinstance(work, dict):
        return errors + ["selected Work missing"]
    require(work.get("work_item_id") == expected["goal_id"], "selected Work identity")
    checkpoints = work.get("checkpoint_ids")
    require(isinstance(checkpoints, list) and expected["checkpoint_id"] in checkpoints,
            "selected Work Checkpoint scope")
    answers = work.get("answers")
    if not isinstance(answers, dict):
        return errors + ["Recall question answers"]
    facts, prose = answers.get("facts"), answers.get("prose")
    if not isinstance(facts, list) or not isinstance(prose, list):
        return errors + ["shared answer sections"]
    recorded = [p for p in facts if isinstance(p, dict) and p.get("question") == "RecordedNextStep"]
    require(len(recorded) == 1, "recorded next action answer")
    require(not any(isinstance(p, dict) and p.get("question") in {"NextStep", "NextStepAvailability"}
                    for p in facts), "recorded action role/scope")
    require(not any(isinstance(p, dict) and p.get("question") == "RecordedNextStep" for p in prose),
            "recorded action role/scope")
    if len(recorded) != 1:
        return errors
    fact = recorded[0]
    require(fact.get("role") == "deterministic_facts", "recorded action role")
    key = f"checkpoint:{expected['checkpoint_id']}@{expected['checkpoint_revision']}:next_step"
    require(fact.get("evidence_keys") == [key], "recorded action evidence key")
    require(fact.get("text") in (
        f"Recorded next action quotation (original language): {expected['next_step']}",
        f"기록된 다음 행동 인용 (원문 언어): {expected['next_step']}"), "ordinary recorded next action meaning")
    basis = fact.get("recorded_action")
    if not isinstance(basis, dict):
        return errors + ["recorded action basis missing"]
    for field, value in (("work_item_id", expected["goal_id"]), ("checkpoint_id", expected["checkpoint_id"]),
                         ("revision", expected["checkpoint_revision"]), ("field", "next_step"),
                         ("recorded_text", expected["next_step"])):
        require(basis.get(field) == value and (field != "revision" or type(basis.get(field)) is int),
                f"recorded action {field}")
    if expected.get("checkpoint_sources") is not None:
        sources = expected["checkpoint_sources"]
        require(isinstance(basis.get("source_ids"), list) and all(isinstance(s, str) for s in basis["source_ids"])
                and sorted(basis["source_ids"]) == sorted(sources), "recorded action Source basis")
        statuses = basis.get("source_status")
        require(isinstance(statuses, list) and all(isinstance(s, dict) and isinstance(s.get("source_id"), str)
                    for s in statuses) and sorted(s["source_id"] for s in statuses) == sorted(sources),
                "recorded action Source status scope")
        if isinstance(statuses, list):
            for status in statuses:
                if not isinstance(status, dict):
                    continue
                for visible in recall.get("source_details", []) if isinstance(recall.get("source_details"), list) else []:
                    if isinstance(visible, dict) and visible.get("identity") == status.get("source_id"):
                        for field in ("availability", "freshness", "snapshot_basis"):
                            if field in visible:
                                require(status.get(field) == visible[field], f"recorded action Source {field}")
    return errors
