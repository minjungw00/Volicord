"""Positive Naturalistic interaction projection; raw evidence stays in Campaign.

Only the current schema/policy is supported. Omitted semantic bodies retain
digests, never a redacted reconstruction of repository or process content.
"""
from __future__ import annotations

import evidence_purpose
import codex_events as codex
import answer_projection
import answer_observations
import explanation_evidence

SCHEMA_VERSION = 2
POLICY = "naturalistic-review-capture-2"
MAX_BODY_BYTES = 1 << 20
MAX_PROJECTION_BYTES = 32 << 20
CAPTURE_SURFACES = {"work_capture", "resume_capture"}
SEMANTIC_ROLES = {"user_turn", "agent_message", "question_request", "volicord_operation"}
LIMITS = {"source_bytes": codex.MAX_CAPTURE_BYTES, "source_events": codex.MAX_CAPTURE_EVENTS,
          "body_bytes": MAX_BODY_BYTES, "projection_bytes": MAX_PROJECTION_BYTES}
# Scalar request/outcome fields accompany typed returned meaning; never generic payloads.
OPERATION_FIELDS = {"action", "project_id", "work_item_id", "source_id", "candidate_id",
    "question_id", "decision_id", "checkpoint_id", "deliberation_candidate_id",
    "user_response_source_id", "state", "status", "resolution", "outcome", "requested_language",
    "record_id", "record_kind", "identity", "revision", "expected_revision", "replayed",
    "context_item_id", "role", "canonical_mutation", "work_transition"}


def plane():
    # Keep the sensitive-payload policy in its existing owner.
    import review_operations
    return review_operations


def body_projection(value):
    ops = plane()
    encoded = ops.encoded(value)
    data = value.encode("utf-8") if isinstance(value, str) else encoded
    reason = ("sensitive_payload" if ops.review_artifact_has_sensitive_payload(encoded)
              else "body_limit" if len(data) > MAX_BODY_BYTES else None)
    return {"state": "omitted" if reason else "retained", "reason": reason,
            "source_body_encoding": "utf8_text" if isinstance(value, str) else "selected_json",
            "source_body_bytes": len(data), "source_body_sha256": ops.digest(data),
            "value": None if reason else value}


def daemon_recovery_context(payload, segments, capture):
    """Exclude only the current host-owned recovery transport, never user turns."""
    metadata = payload.get("internal_chat_message_metadata_passthrough")
    return (isinstance(metadata, dict)
        and metadata.get("content_item_kinds") == ["daemon_recovery.internal_context"]
        and metadata.get("turn_id") in {turn.turn_id for turn in capture.turn_lifecycle.turns}
        and len(segments) == 1
        and plane().re.fullmatch(
            r'<codex_internal_context source="daemon_recovery">\n.+\n</codex_internal_context>',
            segments[0], plane().re.DOTALL) is not None)


def agent_records(events, capture):
    """Recognize current item and response transports; reject conflicting copies.

    Item/response copies with the same immutable message identity are one record.
    Legacy event_msg.agent_message remains an independent ordered observation.
    Reasoning/analysis messages are never user-visible semantic conversation.
    """
    by_identity, independent = {}, []
    turns = {turn.turn_id for turn in capture.turn_lifecycle.turns}
    current_turn = None
    for sequence, event in enumerate(events):
        payload = event["payload"]
        kind, envelope = payload.get("type"), event["type"]
        if envelope == "event_msg" and kind == "task_started":
            current_turn = payload.get("turn_id")
        identity, text, questions, phase = None, None, None, None
        if envelope == "event_msg" and kind == "item_completed" and payload.get("item", {}).get("type") == "AgentMessage":
            item = payload["item"]
            if payload.get("thread_id") != capture.session_id or not codex.nonempty(item.get("id")):
                raise ValueError("review agent message session mismatch")
            current = payload.get("turn_id")
            identity, phase = item.get("id"), item.get("phase")
            content = item.get("content")
            if not isinstance(content, list) or len(content) > codex.MAX_USER_MESSAGE_CONTENT_ITEMS or any(
                not isinstance(part, dict) or part.get("type") != "Text" or not isinstance(part.get("text"), str)
                for part in content):
                raise ValueError("unsupported review agent message content")
            text = "".join(part["text"] for part in content)
            questions = item.get("questions")
            if questions is not None:
                # Current asynchronous question presentation is a bounded title/options list.
                if not isinstance(questions, list) or not 0 < len(questions) <= 256 or any(
                    not isinstance(q, dict) or set(q) != {"title", "options"}
                    or not isinstance(q["title"], str) or not isinstance(q["options"], list)
                    or len(q["options"]) > 256 or not all(isinstance(o, str) for o in q["options"])
                    for q in questions):
                    raise ValueError("unsupported review question presentation")
        elif envelope == "response_item" and kind == "message" and payload.get("role") == "assistant":
            if payload.get("channel") == "analysis":
                continue
            segments = codex.message_text_segments(payload)
            if segments is None:
                raise ValueError("unsupported review assistant message content")
            text, identity, phase = "".join(segments), payload.get("id"), payload.get("phase")
            metadata = payload.get("internal_chat_message_metadata_passthrough")
            current = metadata.get("turn_id") if isinstance(metadata, dict) else current_turn
        elif envelope == "event_msg" and kind == "agent_message":
            text = payload.get("message")
            if not isinstance(text, str):
                raise ValueError("unsupported review agent message content")
            current, phase = current_turn, payload.get("phase")
        else:
            continue
        if current not in turns or phase not in {None, "commentary", "final_answer", "final"}:
            raise ValueError("unsupported review agent message identity or phase")
        record = {"sequence": sequence, "turn_id": current, "message_id": identity,
                  "semantic_role": "agent_message", "phase": phase,
                  "source_sequences": [sequence], "body_value": {"text": text, "questions": questions}}
        if identity is None:
            independent.append(record)
            continue
        if not codex.nonempty(identity):
            raise ValueError("review agent message identity is malformed")
        prior = by_identity.get(identity)
        if prior:
            if prior["turn_id"] != current or prior["phase"] != phase or prior["body_value"]["text"] != text:
                raise ValueError("review agent message transports conflict")
            # The response representation has no question list; retain the item list.
            old_questions = prior["body_value"]["questions"]
            if old_questions is not None and questions is not None and old_questions != questions:
                raise ValueError("review agent question transports conflict")
            prior["body_value"]["questions"] = old_questions if questions is None else questions
            prior["source_sequences"].append(sequence)
        else:
            by_identity[identity] = record
    return [*by_identity.values(), *independent]


def project(data, *, origin, role, session_id, candidate_head, evidence_set_sha256):
    ops = plane()
    if role not in {"start", "resume"} or origin != {
        "kind": "evidence_set_member", "path": origin.get("path"),
        "raw_bytes": len(data), "raw_sha256": ops.digest(data)}:
        raise ValueError("review capture raw source binding mismatch")
    capture = codex.parse_codex_capture(data)
    if capture.session_id != session_id:
        raise ValueError("review capture session binding mismatch")
    events = codex.capture_events(data)
    records = []
    for turn in capture.user_turns:
        records.append({"sequence": turn.sequence, "source_sequences": [turn.sequence],
            "turn_id": turn.turn_id, "user_turn_id": turn.user_turn_id,
            "semantic_role": "user_turn", "body_value": turn.text})
    # Unnormalized actual user prose must never be silently excluded as setup.
    user_texts = {turn.text for turn in capture.user_turns}
    for event in events:
        p = event["payload"]
        if event["type"] == "response_item" and p.get("type") == "message" and p.get("role") == "user":
            segments = codex.message_text_segments(p)
            if segments is None or ("".join(segments) not in user_texts
                    and not codex.host_setup_message(segments)
                    and not daemon_recovery_context(p, segments, capture)):
                raise ValueError("unsupported unnormalized review user interaction")
        if event["type"] == "event_msg" and p.get("type") == "user_message" and not any(
            t.text == p.get("message") and t.user_turn_id == p.get("client_id") for t in capture.user_turns):
            raise ValueError("unsupported review user interaction")
    records.extend(agent_records(events, capture))
    request_sequences = {r.sequence for r in capture.async_question_requests}
    for sequence, event in enumerate(events):
        p = event["payload"]
        if event["type"] == "response_item" and p.get("type") == "function_call" and p.get("name") == "request_user_input_async":
            if sequence not in request_sequences:
                raise ValueError("unsupported review question request transport")
    for request in capture.async_question_requests:
        questions = codex.strict_json(events[request.sequence]["payload"]["arguments"])["questions"]
        records.append({"sequence": request.sequence, "source_sequences": [request.sequence, request.completion_sequence],
            "turn_id": request.turn_id, "call_id": request.call_id, "semantic_role": "question_request",
            "body_value": [{"title": q["title"], "options": q.get("options", [])} for q in questions]})
    def operation_record(sequence, completion, turn, call_id, operation, outcome, request, result, transport, language):
        # This is the normalized observation locator used by the collection index.
        # Invocation windows belong to the independently retained machine basis.
        records.append({"sequence": sequence, "source_sequences": sorted({sequence, completion}),
            "completion_sequence": completion, "turn_id": turn, "call_id": call_id,
            "semantic_role": "volicord_operation", "operation": operation, "outcome": outcome,
            "transport": transport, "requested_language": language,
            "body_value": {"request": {k: v for k, v in request.items() if k in OPERATION_FIELDS
                    and (v is None or type(v) in {str, int, bool})},
                "result": {k: v for k, v in result.items() if k in OPERATION_FIELDS
                    and (v is None or type(v) in {str, int, bool})} if isinstance(result, dict) else {},
                "returned_meaning": answer_projection.project(result, operation)
                    if operation in answer_projection.SCHEMAS else None}})
    for call in capture.tool_calls:
        operation_record(call.sequence, call.completion_sequence, call.turn_id, call.call_id,
            call.operation, call.outcome, call.arguments, call.result, "mcp",
            call.arguments.get("requested_language", "en"))
    cli_returns = [r for r in answer_observations.returned_recalls(capture) if r['transport'] == 'cli']
    cli_returns += explanation_evidence.measured_cli_operations(capture)
    for returned in cli_returns:
        operation = returned.get('operation', 'recall')
        operation_record(returned['sequence'], returned['completion_sequence'], returned['turn_id'],
            returned['call_id'], operation, 'success' if isinstance(returned['result'], dict) else 'unresolvable',
            {}, returned['result'], 'cli', returned['requested_language'])
    for turn in capture.turn_lifecycle.turns:
        records.append({"sequence": turn.start_sequence, "source_sequences": [turn.start_sequence],
            "turn_id": turn.turn_id, "semantic_role": "turn_boundary", "state": turn.state,
            "end_sequence": turn.end_sequence})
        if turn.end_sequence is not None:
            records.append({"sequence": turn.end_sequence, "source_sequences": [turn.end_sequence],
                "turn_id": turn.turn_id, "semantic_role": "turn_terminal", "state": turn.state})
    for sequence in capture.compacted_sequences:
        records.append({"sequence": sequence, "source_sequences": [sequence], "semantic_role": "context_compacted"})
    for command in capture.commands:
        records.append({"sequence": command.sequence, "source_sequences": sorted(set([command.sequence, command.completion_sequence])),
            "turn_id": command.turn_id, "semantic_role": "execution_fact",
            "completion_sequence": command.completion_sequence, "group_index": command.group_index,
            "command_role": codex.command_role(command.parsed_command),
            "normalized_command_sha256": ops.digest(ops.encoded(command.parsed_command)),
            "output_retention": "non_semantic_by_design",
            "exit_code": command.exit_code, "evidence_state": command.evidence_state,
            "termination": command.termination})
    for issue in capture.evidence_transport_issues:
        records.append({"sequence": issue.sequence, "source_sequences": [issue.sequence],
            "turn_id": issue.turn_id, "call_id": issue.call_id, "operation": issue.operation,
            "semantic_role": "transport_issue", "reason": issue.reason})
    for record in records:
        if "body_value" in record:
            record["body"] = body_projection(record.pop("body_value"))
    used = {sequence for record in records for sequence in record["source_sequences"]}
    records.sort(key=lambda r: (r["sequence"], r["semantic_role"]))
    excluded = [{"sequence": sequence, "reason": "non_semantic_by_design"}
                for sequence in range(len(events)) if sequence not in used]
    value = {"kind": "naturalistic_review_capture", "schema_version": SCHEMA_VERSION,
        "evidence_purpose": evidence_purpose.capture_purpose(capture),
        "policy": POLICY, "origin": origin, "session_id": session_id, "role": role,
        "candidate_head": candidate_head, "evidence_set_sha256": evidence_set_sha256,
        "fresh_user_thread": capture.fresh_user_thread, "limits": LIMITS,
        "source_record_count": len(events), "records": records, "excluded_records": excluded}
    value.update(counts(value))
    projected = ops.encoded(value)
    validate(projected)
    return projected, metadata(projected)


def counts(value):
    omitted = [r for r in value["records"] if r.get("body", {}).get("state") == "omitted"]
    partial = [r for r in value["records"] if r.get("body", {}).get("state") == "retained"
        and r['semantic_role'] == 'volicord_operation'
        and isinstance(r['body']['value'].get('returned_meaning'), dict)
        and not r['body']['value']['returned_meaning']['semantic_complete']]
    semantic = sum(r["semantic_role"] in SEMANTIC_ROLES for r in omitted) + len(partial)
    return {"retained_record_count": len(value["records"]) - len(omitted),
        "omitted_record_count": len(value["excluded_records"]) + len(omitted) + len(partial),
        "non_semantic_omission_count": len(value["excluded_records"]) + len(omitted) + len(partial) - semantic,
        "semantic_omission_count": semantic, "semantic_complete": semantic == 0}


def metadata(data):
    ops = plane()
    value = codex.strict_json(data.decode("utf-8"))
    return {"schema_version": SCHEMA_VERSION, "policy": POLICY,
        "review_bytes": len(data), "review_sha256": ops.digest(data),
        "limits": value["limits"], **counts(value)}


def validate(data):
    """Recheck projection shape, omission consistency and strict retained privacy.

    Package validation is source-independent. Reproduction and raw hash checking
    occur at preparation; an artifact hash alone is no authentication of provenance.
    """
    ops = plane()
    ops.require_review_artifact_safe(data)
    value = codex.strict_json(data.decode("utf-8"))
    required = {"evidence_purpose", "kind", "schema_version", "policy", "origin", "session_id", "role", "candidate_head",
        "evidence_set_sha256", "fresh_user_thread", "limits", "source_record_count", "records", "excluded_records",
        "retained_record_count", "omitted_record_count", "non_semantic_omission_count", "semantic_omission_count", "semantic_complete"}
    ops.review.require(isinstance(value, dict) and set(value) == required
        and value["kind"] == "naturalistic_review_capture" and value["schema_version"] == SCHEMA_VERSION
        and value["policy"] == POLICY and value["limits"] == LIMITS and len(data) <= MAX_PROJECTION_BYTES,
        "unsupported review capture projection")
    ops.review.require(type(value["source_record_count"]) is int and 0 < value["source_record_count"] <= codex.MAX_CAPTURE_EVENTS
        and isinstance(value["records"], list) and value["records"] and isinstance(value["excluded_records"], list),
        "malformed review capture records")
    evidence_purpose.validate(value["evidence_purpose"])
    origin = value["origin"]
    ops.review.require(isinstance(origin, dict) and set(origin) == {"kind", "path", "raw_bytes", "raw_sha256"}
        and origin["kind"] == "evidence_set_member" and type(origin["raw_bytes"]) is int
        and 0 < origin["raw_bytes"] <= LIMITS["source_bytes"]
        and isinstance(origin["raw_sha256"], str) and ops.re.fullmatch(r"[0-9a-f]{64}", origin["raw_sha256"])
        and value["role"] in {"start", "resume"} and type(value["fresh_user_thread"]) is bool
        and codex.nonempty(value["session_id"]) and len(value["session_id"]) <= 256
        and isinstance(value["candidate_head"], str) and ops.re.fullmatch(r"[0-9a-f]{40}", value["candidate_head"])
        and isinstance(value["evidence_set_sha256"], str) and ops.re.fullmatch(r"[0-9a-f]{64}", value["evidence_set_sha256"]),
        "malformed review capture origin/context binding")
    name = origin["path"]
    ops.review.require(isinstance(name, str) and name and not ops.Path(name).is_absolute()
        and all(part and part not in {".", ".."} for part in name.split("/")),
        "unsafe review capture origin member path")
    fields = {
        "user_turn": {"turn_id", "user_turn_id", "body"},
        "agent_message": {"turn_id", "message_id", "phase", "body"},
        "question_request": {"turn_id", "call_id", "body"},
        "volicord_operation": {"completion_sequence", "turn_id", "call_id", "operation", "outcome", "body", "transport", "requested_language"},
        "turn_boundary": {"turn_id", "state", "end_sequence"},
        "turn_terminal": {"turn_id", "state"},
        "context_compacted": set(),
        "execution_fact": {"turn_id", "completion_sequence", "group_index", "command_role",
            "normalized_command_sha256", "output_retention", "exit_code", "evidence_state", "termination"},
        "transport_issue": {"turn_id", "call_id", "operation", "reason"},
    }
    previous = -1
    for record in value["records"]:
        ops.review.require(isinstance(record, dict) and record.get("semantic_role") in fields
            and set(record) == {"sequence", "source_sequences", "semantic_role"} | fields[record["semantic_role"]]
            and type(record.get("sequence")) is int and previous <= record["sequence"] < value["source_record_count"]
            and isinstance(record.get("source_sequences"), list) and record["source_sequences"]
            and record["sequence"] == min(record["source_sequences"])
            and all(type(s) is int and 0 <= s < value["source_record_count"] for s in record["source_sequences"]),
            "malformed review capture coordinates")
        previous = record["sequence"]
        body = record.get("body")
        if record["semantic_role"] in SEMANTIC_ROLES | {"volicord_operation"}:
            ops.review.require(isinstance(body, dict) and set(body) == {
                "state", "reason", "source_body_encoding", "source_body_bytes", "source_body_sha256", "value"}
                and type(body["source_body_bytes"]) is int and body["source_body_bytes"] >= 0
                and body["source_body_encoding"] in {"utf8_text", "selected_json"}
                and isinstance(body["source_body_sha256"], str)
                and ops.re.fullmatch(r"[0-9a-f]{64}", body["source_body_sha256"]), "malformed review capture body")
            if body["state"] == "retained":
                ops.review.require(body == body_projection(body["value"]), "review capture retained body mismatch")
                role, retained = record["semantic_role"], body["value"]
                if role == "user_turn":
                    ops.review.require(isinstance(retained, str), "invalid projected user body")
                elif role == "agent_message":
                    ops.review.require(isinstance(retained, dict) and set(retained) == {"text", "questions"}
                        and isinstance(retained["text"], str), "invalid projected agent body")
                elif role == "question_request":
                    ops.review.require(isinstance(retained, list) and retained and all(
                        isinstance(q, dict) and set(q) == {"title", "options"}
                        and isinstance(q["title"], str) and isinstance(q["options"], list)
                        and all(isinstance(o, str) for o in q["options"]) for q in retained),
                        "invalid projected question body")
                elif role == "volicord_operation":
                    ops.review.require(isinstance(retained, dict) and set(retained) == {"request", "result", "returned_meaning"}
                        and all(isinstance(p, dict) and set(p) <= OPERATION_FIELDS and all(
                            v is None or type(v) in {str, int, bool} for v in p.values()) for p in (retained["request"], retained["result"])),
                        "invalid projected operation body")
                    ops.review.require(record['transport'] in {'mcp', 'cli'}
                        and isinstance(record['requested_language'], str), 'invalid operation transport/language')
                    meaning = retained['returned_meaning']
                    if record['operation'] in answer_projection.SCHEMAS:
                        answer_projection.validate(meaning)
                        ops.review.require(meaning['operation'] == record['operation'], 'returned operation scope mismatch')
                    else:
                        ops.review.require(meaning is None, 'unsupported returned meaning')
            else:
                ops.review.require(body["state"] == "omitted" and body["reason"] in {"sensitive_payload", "body_limit"}
                    and body["value"] is None, "invalid review capture omission")
    used = {s for r in value["records"] for s in r["source_sequences"]}
    ops.review.require(value["excluded_records"] == [{"sequence": s, "reason": "non_semantic_by_design"}
        for s in range(value["source_record_count"]) if s not in used]
        and all(type(value[key]) is type(expected) and value[key] == expected for key, expected in counts(value).items()),
        "review capture projection omission/count inconsistency")
    return value
