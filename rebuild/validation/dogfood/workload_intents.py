"""Frozen task-selection metadata; never semantic expected answers."""

WORKLOAD_INTENTS = {
    "journey-volicord-work-a": "learning_collaborative",
    "journey-volicord-work-b": "decision_rich",
    "journey-volicord-work-c": "routine_bounded",
    "journey-small-python-work-a": "exploratory_debugging",
    "journey-polyglot-medium-work-a": "cross_stack_integration",
}


def contract():
    return {"mapping": WORKLOAD_INTENTS,
        "selection_authority": "ordinary_user_intent_not_expected_semantic_outcome",
        "learning_request": "nonempty_verbatim_first_task_excerpt; meaning_reviewed_post_hoc",
        "question_count_requirement": False, "learning_call_count_requirement": False}


def metadata_errors(slot, value, task):
    errors = []
    if value.get("workload_intent") != WORKLOAD_INTENTS.get(slot) or slot not in WORKLOAD_INTENTS:
        errors.append("Work must carry its required workload intent")
    statement = value.get("learning_collaboration_statement")
    if WORKLOAD_INTENTS.get(slot) == "learning_collaborative":
        if not (isinstance(statement, str) and statement.strip()
                and len(statement.encode("utf-8")) <= 4096
                and isinstance(task, str) and statement in task):
            errors.append("Learning intent requires an explicit verbatim first-task request excerpt")
    elif statement is not None:
        errors.append("Learning request excerpt belongs only to the learning/collaborative Work")
    return errors
