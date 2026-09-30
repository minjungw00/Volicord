"""Factual interaction inventory. Counts never supply semantic verdicts."""
from collections import Counter

COUNT_FIELDS = (
    "user_turn_count", "question_candidate_count", "promoted_question_count",
    "current_host_response_count", "canonical_decision_count",
    "materiality_review_call_count", "learning_deliberation_call_count",
    "fresh_resume_count", "recall_call_count",
)


def work_summary(descriptor, work, resume, bundle):
    sessions = []
    candidate_ids, question_ids, response_ids, observed_source_ids = set(), set(), set(), set()
    for role, capture in (("start", work), ("resume", resume)):
        if capture is None:
            continue
        calls = [c for c in capture.tool_calls if c.outcome == "succeeded"]
        participation, deliberations = [], []
        for call in calls:
            for field in ("source_id", "user_response_source_id"):
                value = call.result.get(field)
                if isinstance(value, str):
                    observed_source_ids.add(value)
            action = call.arguments.get("action")
            if call.operation == "candidate_manage" and action in {"submit_question", "submit_question_from_materiality"} and isinstance(call.result.get("candidate_id"), str):
                candidate_ids.add(call.result["candidate_id"])
            if action == "promote_question" and isinstance(call.result.get("question_id"), str):
                question_ids.add(call.result["question_id"])
            if call.operation == "materiality_review" and isinstance(call.arguments.get("learning_participation"), dict):
                value = call.arguments["learning_participation"]
                participation.append({"sequence": call.sequence, "action": action,
                    "state": value.get("state"), "user_turn_source_id": value.get("user_turn_source_id"),
                    "verbatim_statement_in_first_task": bool(isinstance(value.get("verbatim_statement"), str)
                        and value["verbatim_statement"] and value["verbatim_statement"] in descriptor.get("work_user_task", ""))})
            if call.operation == "learning_deliberation":
                deliberations.append({"sequence": call.sequence, "action": action,
                    "candidate_id": call.result.get("deliberation_candidate_id")})
            if call.operation == "decision_record" or (call.operation == "learning_deliberation" and isinstance(action, str) and action.startswith("respond_")):
                for turn in capture.user_turns:
                    if turn.text == call.arguments.get("user_turn"):
                        response_ids.add((capture.session_id, turn.turn_id, turn.user_turn_id))
        sessions.append({"role": role, "session_id": capture.session_id,
            "capture_sha256": capture.source_sha256, "user_turn_count": len(capture.user_turns),
            "successful_operation_counts": dict(sorted(Counter(c.operation for c in calls).items())),
            "operation_outcome_counts": dict(sorted(Counter(c.outcome for c in capture.tool_calls).items())),
            "learning_participation_observations": participation,
            "learning_deliberation_activity": deliberations,
            "recall_sequences": [c.sequence for c in calls if c.operation == "recall"],
            "fresh_user_thread": capture.fresh_user_thread})
    # Canonical sessions are inferred only from returned Source identities, never raw thread labels.
    canonical_sessions = {s.get("detail_two") for s in (bundle.rows("sources") if bundle else ())
        if s.get("id") in observed_source_ids and s.get("source_kind") == "current_host_user_turn"
        and isinstance(s.get("detail_two"), str)}
    learning_sources = {row["context_item_id"] for row in (bundle.rows("context_item_sources") if bundle else ())
        if row.get("source_id") in observed_source_ids}
    learning_contexts = sorted({row["id"] for row in (bundle.rows("context_items") if bundle else ())
        if row.get("id") in learning_sources and row.get("role") in {"learning", "preference"}})
    decisions = sorted({d["id"] for d in (bundle.rows("decisions") if bundle else ())
        if (bundle.one("sources", id=d.get("user_turn_source_id")) or {}).get("detail_two") in canonical_sessions})
    complete = work is not None and (descriptor.get("fresh_resume_user_task") is None or resume is not None)
    counts = {"user_turn_count": sum(s["user_turn_count"] for s in sessions),
        "question_candidate_count": len(candidate_ids), "promoted_question_count": len(question_ids),
        "current_host_response_count": len(response_ids),
        "canonical_decision_count": len(decisions) if bundle else None,
        "materiality_review_call_count": sum(s["successful_operation_counts"].get("materiality_review", 0) for s in sessions),
        "learning_deliberation_call_count": sum(s["successful_operation_counts"].get("learning_deliberation", 0) for s in sessions),
        "fresh_resume_count": int(resume is not None and resume.fresh_user_thread),
        "recall_call_count": sum(len(s["recall_sequences"]) for s in sessions)}
    if not complete:
        counts = {field: None for field in COUNT_FIELDS}
    return {"workload_intent": descriptor.get("workload_intent"), "counts": counts,
        "capture_coverage": "complete" if complete else "incomplete", "sessions": sessions,
        "question_candidate_ids": sorted(candidate_ids), "promoted_question_ids": sorted(question_ids),
        "canonical_decision_ids": decisions, "canonical_learning_context_ids": learning_contexts,
        "frozen_learning_request_present": bool(descriptor.get("learning_collaboration_statement")),
        "canonical_bundle_sha256": bundle.source_sha256 if bundle else None,
        "limits": "Successful normalized operations and observed identities only; absent capture counts are unknown. Canonical Decisions are scoped by observed current-host Sources. Follow-up turns without matched response operations remain user turns. No count establishes necessity, learning recognition, quality or adequacy."}


def campaign_summary(works):
    return {"kind": "dogfood_interaction_diagnostics", "work_summaries": works,
        "workload_intents": {slot: summary["workload_intent"] for slot, summary in sorted(works.items())},
        "counts": {field: (sum(w["counts"][field] for w in works.values())
            if all(w["counts"][field] is not None for w in works.values()) else None) for field in COUNT_FIELDS},
        "semantic_verdict": "not_derived_from_counts"}
