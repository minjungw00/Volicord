"""Factual shared-answer invariants; generated prose requires semantic review.

Expected values must come from independent authoring/observation-time records,
never from the returned answer. This module has no fixture-specific task meaning.
"""


def transport_omission(value):
    if not isinstance(value, dict) or set(value) != {'transport_omission'}:
        return False
    marker = value['transport_omission']
    if not isinstance(marker, dict) or marker.get('reason') != 'serialized_byte_budget':
        return False
    bases = {
        'omitted_count': 'same parent identity, field and stable input order; inspect the authoritative record',
        'omitted_field_count': 'inspect the authoritative record at this identity',
        'exact_json_bytes': 'inspect the complete field on the authoritative parent record',
    }
    return any(type(marker.get(k)) is int and marker[k] > 0 and marker.get('basis') == basis
        for k, basis in bases.items())


def recorded_action_errors(expected, recall):
    errors = []
    def require(ok, message):
        if not ok:
            errors.append(message)
    require(recall.get("project_id") == expected["project_id"], "Project identity")
    has_action = isinstance(expected["next_step"], str) and bool(expected["next_step"].strip())
    require(recall.get("next_step") == (expected["next_step"] if has_action else None), "top-level recorded next action")
    work = recall.get("selected_work")
    if not isinstance(work, dict):
        return errors + ["selected Work missing"]
    require(work.get("work_item_id") == expected["goal_id"], "selected Work identity")
    checkpoints = work.get("checkpoint_ids")
    require(isinstance(checkpoints, list) and (expected["checkpoint_id"] in checkpoints
            if expected["checkpoint_id"] is not None else not checkpoints),
            "selected Work Checkpoint scope")
    answers = work.get("answers")
    if not isinstance(answers, dict):
        return errors + ["Recall question answers"]
    facts, prose = answers.get("facts"), answers.get("prose")
    if not isinstance(facts, list):
        return errors + ["shared answer sections"]
    if not isinstance(prose, list):
        if not transport_omission(prose):
            return errors + ["shared answer sections"]
        # Only the visible recorded action is checked here. The caller retains
        # the generated-prose observation gap; the V11 oracle still requires its
        # fixture-specific generated/availability sections independently.
        prose = []
    recorded = [p for p in facts if isinstance(p, dict) and p.get("question") == "RecordedNextStep"]
    if not has_action:
        unavailable = [p for p in facts if isinstance(p, dict) and p.get('question') == 'NextStepAvailability']
        require(not recorded and len(unavailable) == 1, 'genuine recorded action absence')
        require(not any(isinstance(p, dict) and p.get('recorded_action') is not None for p in facts + prose),
                'absent action has no recorded basis')
        if len(unavailable) == 1:
            require(unavailable[0].get('role') == 'unavailable' and unavailable[0].get('evidence_keys') == [],
                    'recorded action absence role/basis')
            require(unavailable[0].get('text') in (
                ('No next action is recorded in the latest Checkpoint.', '최신 Checkpoint에 다음 행동이 기록되지 않았습니다.')
                if expected['checkpoint_id'] is not None else
                ('No next action is recorded: this Work has no Checkpoint.', '다음 행동이 기록되지 않았습니다. 이 작업에는 Checkpoint가 없습니다.')),
                'recorded action absence meaning')
        require(not any(isinstance(p, dict) and p.get('question') == 'NextStep' for p in facts),
                'recorded action absence role/scope')
        return errors
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
