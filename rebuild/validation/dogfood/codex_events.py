#!/usr/bin/env python3
"""Normalize bounded Phase 8 facts from Codex rollout JSONL and canonical bundles.

Raw rollout and bundle content is local evidence.  This module intentionally keeps
prompt text and tool payloads only long enough to correlate an observed user turn
with the product's canonical Source; callers receive identities, ordering, paths,
and booleans rather than source bodies or arbitrary tool output.
"""

from __future__ import annotations

from dataclasses import dataclass, replace
import ast
import datetime as dt
import hashlib
import json
from pathlib import Path
import re
import shlex
from typing import Any, Callable


MAX_CAPTURE_BYTES = 64 * 1024 * 1024
MAX_CAPTURE_EVENTS = 200_000
MAX_PATHS = 256
MAX_USER_MESSAGE_CONTENT_ITEMS = 256
MAX_USER_TURN_TEXT_CHARS = 1 << 20
MAX_MCP_CONTENT_RESULT_CHARS = 2 << 20
MAX_FILE_CHANGE_BODY_CHARS = 8 << 20
# The production owner supplies the wire identity; prose is not a contract.
ACTIVATION_IDENTITY = (
    Path(__file__).resolve().parents[2]
    / "crates/volicord-operations/src/session_start_identity.txt"
).read_text(encoding="utf-8").rstrip("\n")
ACTIVATION_PREFIX, ACTIVATION_SUFFIX = ACTIVATION_IDENTITY.split("{binding}")


def activation_identity(cwd: Path, session_id: str) -> str:
    binding = hashlib.sha256(
        str(cwd).encode("utf-8") + b"\0" + session_id.encode("utf-8")
    ).hexdigest()
    return ACTIVATION_IDENTITY.replace("{binding}", binding)


def activation_evidence(text: str, cwd: Path, session_id: str) -> str:
    lines = [line for line in text.splitlines() if line.startswith(ACTIVATION_PREFIX)]
    if not lines:
        return "absent"
    if len(lines) != 1 or re.fullmatch(
        re.escape(ACTIVATION_PREFIX) + r"[0-9a-f]{64}" + re.escape(ACTIVATION_SUFFIX),
        lines[0],
    ) is None:
        return "malformed"
    return "valid" if lines[0] == activation_identity(cwd, session_id) else "binding_mismatch"


def message_text_segments(payload: dict[str, Any]) -> list[str] | None:
    content = payload.get("content")
    if not isinstance(content, list) or not 0 < len(content) <= MAX_USER_MESSAGE_CONTENT_ITEMS:
        return None
    if any(
        not isinstance(item, dict)
        or item.get("type") not in {"input_text", "output_text"}
        or not isinstance(item.get("text"), str)
        for item in content
    ):
        return None
    segments = [item["text"] for item in content]
    return segments if sum(map(len, segments)) <= MAX_USER_TURN_TEXT_CHARS else None


def host_setup_message(segments: list[str]) -> bool:
    """Recognize the bounded VS Code setup bundle, never arbitrary user prose.

    Require the environment block and whole known setup segments. An unfamiliar
    or mixed representation stays ambiguous unless a normalized user turn binds it.
    """
    environment = False
    for segment in segments:
        text = segment.strip()
        if re.fullmatch(r"<environment_context>.*</environment_context>", text, re.DOTALL):
            environment = True
        elif re.fullmatch(r"<recommended_plugins>.*</recommended_plugins>", text, re.DOTALL):
            continue
        elif re.fullmatch(
            r"# AGENTS\.md instructions for [^\n]+\n\s*<INSTRUCTIONS>.*</INSTRUCTIONS>",
            text, re.DOTALL,
        ):
            continue
        else:
            return False
    return environment


def session_activation_state(
    events: list[dict[str, Any]],
    identities: list[tuple[int, str]],
    user_turns: tuple[UserTurn, ...],
    work_sequences: set[int],
) -> str:
    """Classify visibility, separately from task_started transport bookkeeping."""
    for state in ("malformed", "binding_mismatch"):
        if any(value == state for _, value in identities):
            return state
    boundaries = {turn.sequence for turn in user_turns} | work_sequences
    user_texts = {turn.text for turn in user_turns}
    uncertain: set[int] = set()
    unreadable_developer_context = False
    for sequence, event in enumerate(events):
        payload = event.get("payload")
        if not isinstance(payload, dict):
            uncertain.add(sequence)
            continue
        envelope, kind = event.get("type"), payload.get("type")
        if envelope == "response_item" and kind == "message":
            role = payload.get("role")
            if role in {"user", "developer"}:
                segments = message_text_segments(payload)
                if segments is None:
                    uncertain.add(sequence)
                    unreadable_developer_context |= role == "developer"
                elif role == "user":
                    # The agent-visible copy can precede its user-message event.
                    if "".join(segments) in user_texts:
                        boundaries.add(sequence)
                    elif not host_setup_message(segments):
                        uncertain.add(sequence)
            elif role != "system":
                uncertain.add(sequence)
        elif envelope == "response_item":
            # Known project operations are definitive boundaries below. Unknown
            # agent activity must not silently become setup or an operator fault.
            uncertain.add(sequence)
        elif envelope == "event_msg":
            if kind not in {"task_started", "token_count"}:
                uncertain.add(sequence)
        elif envelope not in {"session_meta", "world_state", "turn_context", "token_usage_record"}:
            uncertain.add(sequence)
    # An unreadable context may contain a missing or conflicting identity. Its
    # position cannot turn an identity interpretation failure into setup blame.
    if unreadable_developer_context:
        return "indeterminate"
    first_work = min(boundaries, default=len(events))
    valid_sequences = [sequence for sequence, state in identities if state == "valid"]
    if not valid_sequences:
        # No identifiable work boundary or uninterpretable pre-work context does
        # not prove that the operator omitted activation.
        return "indeterminate" if not boundaries or any(
            sequence < first_work for sequence in uncertain
        ) else "absent"
    first_activation = min(valid_sequences)
    if first_work < first_activation:
        return "late"
    if not boundaries or any(sequence < first_activation for sequence in uncertain):
        return "indeterminate"
    return "valid"


VOLICORD_OPERATIONS = {
    "background_semantic_operation",
    "candidate_inspect",
    "candidate_manage",
    "canonical_inspect",
    "canonical_mutate",
    "project_initialize",
    "project_resolve",
    "project_health",
    "context_record",
    "decision_record",
    "document_preview",
    "engineering_choice_discovery",
    "guarded_interaction",
    "inquiry_frontier",
    "materiality_review",
    "learning_deliberation",
    "checkpoint_record",
    "privacy_status",
    "recall",
    "repository_analyze",
    "repository_understanding",
}


class EvidenceError(ValueError):
    """The referenced evidence does not have the supported bounded shape."""


def nonempty(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def normalize_operation(value: Any) -> str | None:
    if not nonempty(value):
        return None
    operation = str(value)
    if operation.startswith("volicord_"):
        operation = operation[len("volicord_") :]
    return operation if operation in VOLICORD_OPERATIONS else None


def bounded_path(value: Any, cwd: Path) -> str | None:
    if not nonempty(value) or len(value) > 4096:
        return None
    candidate = Path(value)
    if candidate.is_absolute():
        try:
            candidate = candidate.resolve(strict=False).relative_to(cwd.resolve(strict=False))
        except ValueError:
            return None
    if candidate.is_absolute() or ".." in candidate.parts or not candidate.parts:
        return None
    normalized = candidate.as_posix()
    if normalized in {"", "."} or any(part in {".git", ".local"} for part in candidate.parts):
        return None
    return normalized


def generated_repository_path(path: str) -> bool:
    return any(
        part
        in {
            "build",
            "dist",
            "target",
            "node_modules",
            ".cache",
            ".venv",
            ".ruff_cache",
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
        }
        for part in Path(path).parts
    )


def normalized_file_changes(
    value: Any, cwd: Path
) -> tuple[
    tuple[str, ...],
    tuple[tuple[str, str, str, str | None], ...],
] | None:
    """Normalize the bounded patch/FileChange change map for identity and paths."""
    if not isinstance(value, dict) or not value or len(value) > MAX_PATHS:
        return None
    changes: list[tuple[str, str, str, str | None]] = []
    for raw_path, raw_change in value.items():
        if not isinstance(raw_path, str) or not isinstance(raw_change, dict):
            return None
        path = bounded_path(raw_path, cwd)
        if path is None:
            if Path(raw_path).is_absolute():
                continue
            return None
        change_type = raw_change.get("type")
        move_path: str | None = None
        if change_type == "update":
            if (
                set(raw_change) != {"type", "unified_diff", "move_path"}
                or not isinstance(raw_change.get("unified_diff"), str)
                or not raw_change["unified_diff"]
                or len(raw_change["unified_diff"]) > MAX_FILE_CHANGE_BODY_CHARS
            ):
                return None
            raw_move_path = raw_change.get("move_path")
            if raw_move_path is not None:
                move_path = bounded_path(raw_move_path, cwd)
                if move_path is None:
                    return None
            body = raw_change["unified_diff"]
        elif change_type in {"add", "delete"}:
            if (
                set(raw_change) != {"type", "content"}
                or not isinstance(raw_change.get("content"), str)
                or len(raw_change["content"]) > MAX_FILE_CHANGE_BODY_CHARS
            ):
                return None
            body = raw_change["content"]
        else:
            return None
        changes.append((path, str(change_type), sha256_bytes(body.encode("utf-8")), move_path))
    changes.sort()
    paths = tuple(
        sorted(
            {
                leaf
                for path, _change_type, _body_sha256, move_path in changes
                for leaf in (path, move_path)
                if leaf is not None and not generated_repository_path(leaf)
            }
        )
    )
    return paths, tuple(changes)


def merge_path_observation_evidence(
    evidence: list[_PathObservationEvidence],
) -> tuple[PathObservation, ...]:
    """Deduplicate equivalent patch transports and reject identity conflicts."""
    by_identity: dict[tuple[str, str], _PathObservationEvidence] = {}
    for candidate in sorted(evidence, key=lambda value: value.sequence):
        identity = (candidate.turn_id, candidate.call_id)
        prior = by_identity.get(identity)
        if prior is None:
            by_identity[identity] = candidate
            continue
        if prior.paths != candidate.paths or prior.changes != candidate.changes:
            raise EvidenceError("Codex file-change representations conflict")
    return tuple(
        PathObservation(value.sequence, value.turn_id, value.paths)
        for value in sorted(by_identity.values(), key=lambda item: item.sequence)
        if value.paths
    )


@dataclass(frozen=True)
class UserTurn:
    sequence: int
    turn_id: str
    user_turn_id: str
    text: str


def strict_json(value: str) -> Any:
    """Reject duplicate object members and non-JSON constants as ambiguous evidence."""
    def object_pairs(pairs):
        result = {}
        for key, item in pairs:
            if key in result:
                raise ValueError("duplicate JSON member")
            result[key] = item
        return result

    def invalid_constant(_value):
        raise ValueError("non-JSON constant")

    return json.loads(value, object_pairs_hook=object_pairs, parse_constant=invalid_constant)


@dataclass(frozen=True)
class AsyncQuestionRequest:
    session_id: str
    turn_id: str
    call_id: str
    sequence: int
    completion_sequence: int
    questions: tuple[str, ...]


@dataclass(frozen=True)
class AsyncQuestionReply:
    call_id: str
    question_index: int
    question: str
    answer: str


def parse_async_question_replies(text: str) -> tuple[AsyncQuestionReply, ...] | None:
    """Only the complete current Codex envelope and its array-of-items JSON schema."""
    opening = "<send_user_message_question_reply>"
    closing = "</send_user_message_question_reply>"
    if not text.startswith(opening) or not text.endswith(closing):
        return None
    try:
        items = strict_json(text[len(opening):-len(closing)])
        if not isinstance(items, list) or not 0 < len(items) <= MAX_PATHS:
            return None
        replies = []
        identities = set()
        for item in items:
            if (not isinstance(item, dict)
                or set(item) != {"answer", "question", "questionItemId"}
                or not all(nonempty(item[field]) for field in item)):
                return None
            identity = strict_json(item["questionItemId"])
            if (not isinstance(identity, list) or len(identity) != 3
                or identity[0] != "request_user_input_async"
                or not nonempty(identity[1]) or type(identity[2]) is not int
                or not 0 <= identity[2] < MAX_PATHS):
                return None
            key = (identity[1], identity[2])
            if key in identities:
                return None
            identities.add(key)
            replies.append(AsyncQuestionReply(*key, item["question"], item["answer"]))
        return tuple(replies)
    except (ValueError, RecursionError):
        return None


@dataclass(frozen=True)
class _UserTurnEvidence:
    sequence: int
    turn_id: str
    client_id: str
    text: str
    item_id: str | None


def current_user_turn_evidence(
    payload: dict[str, Any], sequence: int, session_id: str
) -> _UserTurnEvidence | None:
    """Normalize the bounded ItemCompleted(UserMessage) rollout representation.

    Codex's UserMessageItem::message() concatenates ordered text inputs without a
    separator. Dogfood accepts that same all-text representation and rejects
    attachments or malformed content rather than guessing at prompt identity.
    """
    if payload.get("type") != "item_completed":
        return None
    item = payload.get("item")
    if not isinstance(item, dict) or item.get("type") != "UserMessage":
        return None
    thread_id = payload.get("thread_id")
    turn_id = payload.get("turn_id")
    item_id = item.get("id")
    client_id = item.get("client_id")
    content = item.get("content")
    if (
        thread_id != session_id
        or not nonempty(turn_id)
        or not nonempty(item_id)
        or not nonempty(client_id)
        or not isinstance(content, list)
        or not content
        or len(content) > MAX_USER_MESSAGE_CONTENT_ITEMS
    ):
        raise EvidenceError("Codex UserMessage item identity or content is malformed")
    segments: list[str] = []
    for segment in content:
        if (
            not isinstance(segment, dict)
            or segment.get("type") != "text"
            or not isinstance(segment.get("text"), str)
        ):
            raise EvidenceError("Codex UserMessage item has unsupported textual content")
        segments.append(segment["text"])
    text = "".join(segments)
    if not nonempty(text) or len(text) > MAX_USER_TURN_TEXT_CHARS:
        raise EvidenceError("Codex UserMessage item text is empty or exceeds the bound")
    return _UserTurnEvidence(
        sequence,
        str(turn_id),
        str(client_id),
        text,
        str(item_id),
    )


def normalize_user_turn_evidence(
    evidence: list[_UserTurnEvidence], known_turn_ids: set[str]
) -> tuple[UserTurn, ...]:
    by_transport: dict[tuple[str, str], _UserTurnEvidence] = {}
    current_item_transports: dict[str, tuple[str, str]] = {}
    for candidate in sorted(evidence, key=lambda value: value.sequence):
        if candidate.turn_id not in known_turn_ids:
            raise EvidenceError("Codex user turn refers to an unknown turn identity")
        transport = (candidate.turn_id, candidate.client_id)
        if candidate.item_id is not None:
            prior_transport = current_item_transports.get(candidate.item_id)
            if prior_transport is not None and prior_transport != transport:
                raise EvidenceError("Codex UserMessage item identity is reused across user turns")
            current_item_transports[candidate.item_id] = transport
        prior = by_transport.get(transport)
        if prior is None:
            by_transport[transport] = candidate
            continue
        if prior.text != candidate.text or (
            prior.item_id is not None
            and candidate.item_id is not None
            and prior.item_id != candidate.item_id
        ):
            raise EvidenceError("Codex user-turn representations conflict")
        if prior.item_id is None and candidate.item_id is not None:
            by_transport[transport] = _UserTurnEvidence(
                prior.sequence,
                prior.turn_id,
                prior.client_id,
                prior.text,
                candidate.item_id,
            )
    return tuple(
        UserTurn(value.sequence, value.turn_id, value.client_id, value.text)
        for value in sorted(by_transport.values(), key=lambda item: item.sequence)
    )


@dataclass(frozen=True)
class ToolCall:
    sequence: int
    completion_sequence: int
    turn_id: str
    call_id: str
    server: str
    operation: str
    arguments: dict[str, Any]
    result: dict[str, Any]
    outcome: str
    error: str | None
    transport_representations: tuple[str, ...]


@dataclass(frozen=True)
class _ToolCallEvidence:
    sequence: int
    completion_sequence: int
    turn_id: str
    call_id: str
    server: str
    operation: str
    arguments: dict[str, Any]
    result: dict[str, Any]
    outcome: str
    error: str | None
    representation: str


@dataclass(frozen=True)
class EvidenceTransportIssue:
    sequence: int
    turn_id: str
    call_id: str | None
    server: str
    operation: str | None
    reason: str

    def __post_init__(self) -> None:
        if not supported_transport_issue(vars(self)):
            raise EvidenceError("unsupported evidence transport issue")


def supported_transport_issue(issue: Any) -> bool:
    """The single producer/consumer contract for supported transport diagnostics."""
    return (
        isinstance(issue, dict)
        and set(issue) == {"sequence", "turn_id", "call_id", "server", "operation", "reason"}
        and type(issue["sequence"]) is int and issue["sequence"] >= 0
        and nonempty(issue["turn_id"]) and nonempty(issue["call_id"])
        and (
            issue["server"] == "volicord"
            and issue["operation"] in VOLICORD_OPERATIONS
            and issue["reason"] in {"malformed_mcp_completion", "unsupported_mcp_completion_status",
                                    "mcp_completion_status_mismatch"}
            or issue["server"] == "codex" and issue["operation"] is None
            and issue["reason"] in {"malformed_file_change", "malformed_exec_completion",
                                    "command_completion_indeterminate"}
        )
    )


@dataclass(frozen=True)
class PathObservation:
    sequence: int
    turn_id: str
    paths: tuple[str, ...]


@dataclass(frozen=True)
class _PathObservationEvidence:
    sequence: int
    turn_id: str
    call_id: str
    paths: tuple[str, ...]
    changes: tuple[tuple[str, str, str, str | None], ...]
    representation: str


@dataclass(frozen=True)
class CommandObservation:
    sequence: int
    completion_sequence: int
    turn_id: str
    group_index: int
    parsed_command: Any
    exit_code: int | None
    termination: str | None
    output: str
    output_was_empty: bool
    execution_identity: str | None = None
    evidence_state: str = "indeterminate"
    raw_call_id: str | None = None
    continuation_coordinates: tuple[tuple[str, str, int, int, int], ...] = ()
    signal_number: int | None = None
    output_state: str = 'unknown'


@dataclass(frozen=True)
class ExecutionWrapperObservation:
    """Bounded raw locator and normalization coverage, never a success assertion."""
    sequence: int
    completion_sequence: int | None
    turn_id: str | None
    call_id: str | None
    wrapper_sha256: str
    tool_names: tuple[str, ...]
    observed_call_count: int | None
    state: str
    reasons: tuple[str, ...]

    def __post_init__(self):
        if not supported_execution_wrapper(vars(self)):
            raise EvidenceError('unsupported execution coverage observation')


EXECUTION_LIMIT_REASONS = {'unsupported_wrapper_grammar', 'wrapper_identity_unresolvable',
    'wrapper_completion_unresolvable', 'command_completion_indeterminate',
    'continuation_result_unresolvable', 'continuation_launch_unobserved'}


def safe_execution_id(value):
    return value if isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9_-]{1,256}', value) else None


def supported_execution_wrapper(value):
    safe_id = lambda item: item is None or safe_execution_id(item) is not None
    return (isinstance(value, dict) and set(value) == {'sequence', 'completion_sequence', 'turn_id',
        'call_id', 'wrapper_sha256', 'tool_names', 'observed_call_count', 'state', 'reasons'}
        and type(value['sequence']) is int and value['sequence'] >= 0
        and (value['completion_sequence'] is None or type(value['completion_sequence']) is int
            and value['completion_sequence'] > value['sequence'])
        and safe_id(value['turn_id']) and safe_id(value['call_id'])
        and isinstance(value['wrapper_sha256'], str) and re.fullmatch(r'[0-9a-f]{64}', value['wrapper_sha256']) is not None
        and isinstance(value['tool_names'], (tuple, list)) and bool(value['tool_names'])
        and all(name in {'exec_command', 'write_stdin'} for name in value['tool_names'])
        and list(value['tool_names']) == sorted(set(value['tool_names']))
        and (value['observed_call_count'] is None or type(value['observed_call_count']) is int
            and 1 <= value['observed_call_count'] <= 16)
        and value['state'] in {'normalized', 'unsupported', 'indeterminate'}
        and isinstance(value['reasons'], (tuple, list))
        and all(reason in EXECUTION_LIMIT_REASONS for reason in value['reasons'])
        and list(value['reasons']) == sorted(set(value['reasons']))
        and (value['turn_id'] is not None and value['call_id'] is not None
            or 'wrapper_identity_unresolvable' in value['reasons'])
        and (value['state'] != 'normalized' or value['observed_call_count'] is not None
            and value['turn_id'] is not None and value['call_id'] is not None
            and value['completion_sequence'] is not None)
        and bool(value['reasons']) == (value['state'] != 'normalized'))


@dataclass(frozen=True)
class ParsedCustomCall:
    tool_name: str
    arguments: Any
    output_mode: str
    result_keys: tuple[str, ...] = ()


@dataclass(frozen=True)
class ParsedMcpWrapper:
    operation: str
    arguments: dict[str, Any]


class JsLiteralParser:
    """Parse only JSON-like JavaScript literals used by current dogfood calls."""

    def __init__(self, source: str, bindings: dict[str, str] | None = None):
        self.source = source
        self.offset = 0
        self.bindings = bindings or {}

    def parse(self) -> Any:
        value = self.value()
        self.whitespace()
        if self.offset != len(self.source):
            raise EvidenceError("tool argument contains unsupported JavaScript")
        return value

    def whitespace(self) -> None:
        while self.offset < len(self.source) and self.source[self.offset] in " \t\r\n":
            self.offset += 1

    def value(self) -> Any:
        self.whitespace()
        if self.offset >= len(self.source):
            raise EvidenceError("tool argument is incomplete")
        character = self.source[self.offset]
        if character == '"':
            return self.string()
        if character == "{":
            return self.object()
        if character == "[":
            return self.array()
        identifier = re.match(r"[A-Za-z_$][A-Za-z0-9_$]*", self.source[self.offset:])
        if identifier and identifier.group() in self.bindings:
            self.offset += len(identifier.group())
            return self.bindings[identifier.group()]
        for literal, value in (("true", True), ("false", False), ("null", None)):
            if self.source.startswith(literal, self.offset):
                self.offset += len(literal)
                return value
        number = re.match(r"-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?", self.source[self.offset :])
        if number is None:
            raise EvidenceError("tool argument is not a supported literal")
        token = number.group(0)
        self.offset += len(token)
        return float(token) if any(marker in token for marker in ".eE") else int(token)

    def string(self) -> str:
        start = self.offset
        self.offset += 1
        escaped = False
        while self.offset < len(self.source):
            character = self.source[self.offset]
            self.offset += 1
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                token = self.source[start : self.offset]
                try:
                    value = json.loads(token)
                except json.JSONDecodeError as error:
                    raise EvidenceError("tool argument string is invalid") from error
                if not isinstance(value, str):
                    raise EvidenceError("tool argument string is invalid")
                return value
            elif ord(character) < 32:
                raise EvidenceError("tool argument string contains a control character")
        raise EvidenceError("tool argument string is unterminated")

    def identifier(self) -> str:
        self.whitespace()
        match = re.match(r"[A-Za-z_$][A-Za-z0-9_$]*", self.source[self.offset :])
        if match is None:
            raise EvidenceError("tool object key is not static")
        self.offset += len(match.group(0))
        return match.group(0)

    def object(self) -> dict[str, Any]:
        self.offset += 1
        result: dict[str, Any] = {}
        self.whitespace()
        if self.offset < len(self.source) and self.source[self.offset] == "}":
            self.offset += 1
            return result
        while True:
            self.whitespace()
            key = self.string() if self.offset < len(self.source) and self.source[self.offset] == '"' else self.identifier()
            self.whitespace()
            if self.offset >= len(self.source) or self.source[self.offset] != ":":
                raise EvidenceError("tool object property is malformed")
            self.offset += 1
            if key in result:
                raise EvidenceError("tool object contains a duplicate property")
            result[key] = self.value()
            self.whitespace()
            if self.offset >= len(self.source):
                raise EvidenceError("tool object is unterminated")
            delimiter = self.source[self.offset]
            self.offset += 1
            if delimiter == "}":
                return result
            if delimiter != ",":
                raise EvidenceError("tool object delimiter is invalid")

    def array(self) -> list[Any]:
        self.offset += 1
        result: list[Any] = []
        self.whitespace()
        if self.offset < len(self.source) and self.source[self.offset] == "]":
            self.offset += 1
            return result
        while True:
            result.append(self.value())
            self.whitespace()
            if self.offset >= len(self.source):
                raise EvidenceError("tool array is unterminated")
            delimiter = self.source[self.offset]
            self.offset += 1
            if delimiter == "]":
                return result
            if delimiter != ",":
                raise EvidenceError("tool array delimiter is invalid")


ASSIGNED_CALL = re.compile(
    r"\A\s*const\s+(?P<variable>[A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*await\s+"
    r"tools\.(?P<tool>[A-Za-z_$][A-Za-z0-9_$]*)\s*\((?P<argument>.*?)\)\s*;\s*"
    r"(?P<forward>.*)\s*\Z",
    re.DOTALL,
)
PROMISE_ASSIGNED_CALL = re.compile(
    r"\A\s*const\s+(?P<variable>[A-Za-z_$][A-Za-z0-9_$]*|\[[A-Za-z0-9_$,\s]+\])\s*=\s*await\s+"
    r"Promise\.all\s*\(\s*\[(?P<calls>.*)\]\s*\)\s*;\s*(?P<forward>.*)\s*\Z",
    re.DOTALL,
)
MCP_WRAPPER = re.compile(
    r"\A\s*const\s+(?P<variable>[A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*await\s+"
    r"tools\.mcp__volicord__(?P<operation>[A-Za-z_$][A-Za-z0-9_$]*)\s*"
    r"\((?P<argument>.*?)\)\s*;(?P<forward>.*)\Z",
    re.DOTALL,
)
DIRECT_PATCH = re.compile(
    r"\A\s*text\s*\(\s*await\s+tools\.apply_patch\s*\((?P<argument>.*)\)\s*\)\s*;\s*\Z",
    re.DOTALL,
)
BOUND_PATCH = re.compile(
    r"\A\s*const\s+(?P<variable>[A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?P<literal>\"(?:\\.|[^\"\\])*\")\s*;\s*"
    r"text\s*\(\s*await\s+tools\.apply_patch\s*\(\s*(?P=variable)\s*\)\s*\)\s*;\s*\Z",
    re.DOTALL,
)


def parse_static_exec_command_list(value: str, bindings: dict[str, str] | None = None) -> tuple[dict[str, Any], ...] | None:
    """Parse a bounded comma-separated list of literal exec_command calls."""
    offset = 0
    result: list[dict[str, Any]] = []
    call_prefix = re.compile(r"tools\.exec_command\s*\(")
    while True:
        while offset < len(value) and value[offset] in " \t\r\n":
            offset += 1
        if offset == len(value):
            return tuple(result) if 2 <= len(result) <= 16 else None
        match = call_prefix.match(value, offset)
        if match is None:
            return None
        argument_start = match.end()
        cursor = argument_start
        quote = False
        escaped = False
        while cursor < len(value):
            character = value[cursor]
            if quote:
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    quote = False
            elif character == '"':
                quote = True
            elif character == ")":
                break
            cursor += 1
        if cursor >= len(value) or quote:
            return None
        try:
            arguments = JsLiteralParser(value[argument_start:cursor], bindings).parse()
        except EvidenceError:
            return None
        if not isinstance(arguments, dict):
            return None
        result.append(arguments)
        offset = cursor + 1
        while offset < len(value) and value[offset] in " \t\r\n":
            offset += 1
        if offset < len(value):
            if value[offset] != ",":
                return None
            offset += 1


def indexed_promise_output_mode(forward: str, variable: str) -> str | None:
    identifier = r"[A-Za-z_$][A-Za-z0-9_$]*"
    escaped_variable = re.escape(variable)
    # Observed current host: literal Promise results enumerated in input order.
    entries = re.fullmatch(
        rf"for\s*\(\s*const\s*\[\s*(?P<index>{identifier})\s*,\s*"
        rf"(?P<result>{identifier})\s*\]\s+of\s+{escaped_variable}\.entries\(\)\s*\)"
        rf"\s*\{{\s*text\s*\(\s*`---(?:FILE)?\$\{{(?P=index)\}} exit="
        rf"\$\{{(?P=result)\.exit_code\}}\\n\$\{{(?P=result)\.output\}}`\s*\)\s*;?\s*\}}\s*;?",
        forward, re.DOTALL,
    )
    if entries is not None:
        names = (variable, entries["index"], entries["result"])
        if len(set(names)) != 3 or any(n in {"text", "tools", "JSON", "Promise"} for n in names):
            return None
        return "indexed_entries_zero"
    suffix_zero = re.fullmatch(
        rf"{escaped_variable}\.forEach\s*\(\s*\(\s*(?P<result>{identifier})\s*,\s*"
        rf"(?P<index>{identifier})\s*\)\s*=>\s*text\s*\(\s*`"
        rf"[A-Za-z][A-Za-z0-9 _-]{{0,31}}\$\{{(?P=index)\}}\\n"
        rf"\$\{{(?P=result)\.output\}}\\nexit=\$\{{(?P=result)\.exit_code\}}`"
        rf"\s*\)\s*\)\s*;",
        forward,
        re.DOTALL,
    )
    if suffix_zero is not None:
        return "indexed_suffix_zero"
    prefix_one = re.fullmatch(
        rf"{escaped_variable}\.forEach\s*\(\s*\(\s*(?P<result>{identifier})\s*,\s*"
        rf"(?P<index>{identifier})\s*\)\s*=>\s*text\s*\(\s*`"
        rf"[A-Za-z][A-Za-z0-9 _-]{{0,31}}\$\{{(?P=index)\+1\}}\s+exit="
        rf"\$\{{(?P=result)\.exit_code\}}\\n\$\{{(?P=result)\.output\}}`"
        rf"\s*\)\s*\)\s*;",
        forward,
        re.DOTALL,
    )
    if prefix_one is not None:
        return "indexed_prefix_one"
    suffix_one = re.fullmatch(
        rf"for\s*\(\s*let\s+(?P<index>{identifier})\s*=\s*0\s*;\s*"
        rf"(?P=index)\s*<\s*{escaped_variable}\.length\s*;\s*(?P=index)\+\+\s*\)\s*\{{\s*"
        rf"text\s*\(\s*`[A-Za-z][A-Za-z0-9 _-]{{0,31}}\$\{{(?P=index)\+1\}}\\n"
        rf"\$\{{{escaped_variable}\[(?P=index)\]\.output\}}\\nEXIT\s+"
        rf"\$\{{{escaped_variable}\[(?P=index)\]\.exit_code\}}`\s*\)\s*;?\s*\}}\s*;?",
        forward,
        re.DOTALL,
    )
    return "indexed_suffix_one" if suffix_one is not None else None


def parse_custom_call(value: Any) -> ParsedCustomCall | None:
    """Recognize one bounded current exec-cell call without evaluating JavaScript."""
    if not isinstance(value, str) or len(value.encode("utf-8")) > 64 * 1024:
        return None
    direct = parse_direct_execution_sequence(value)
    if direct is not None:
        return direct[0] if len(direct) == 1 else ParsedCustomCall('direct_sequence', direct, 'ordered_results')
    # Only immutable literal strings; no substitution inside strings or captured JS execution.
    bindings: dict[str, str] = {}
    declaration = re.compile(r'\s*const\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*("(?:\\.|[^"\\])*")\s*;')
    while match := declaration.match(value):
        if match[1] in bindings or match[1] in {"tools", "text", "JSON", "Promise"} or len(bindings) >= 16:
            return None
        try:
            bindings[match[1]] = JsLiteralParser(match[2]).parse()
        except EvidenceError:
            return None
        value = value[match.end():]
    if bindings and not re.search(r"tools\.exec_command\s*\(", value):
        # Preserve the existing literal-bound patch grammar below.
        if len(bindings) == 1:
            name, literal = next(iter(bindings.items()))
            value = f"const {name}={json.dumps(literal)};" + value
    bound_patch = BOUND_PATCH.fullmatch(value)
    if bound_patch is not None:
        try:
            patch = JsLiteralParser(bound_patch.group("literal")).parse()
        except EvidenceError:
            return None
        return ParsedCustomCall("apply_patch", patch, "patch") if isinstance(patch, str) else None
    direct_patch = DIRECT_PATCH.fullmatch(value)
    if direct_patch is not None:
        try:
            patch = JsLiteralParser(direct_patch.group("argument")).parse()
        except EvidenceError:
            return None
        return ParsedCustomCall("apply_patch", patch, "patch") if isinstance(patch, str) else None

    promise_match = PROMISE_ASSIGNED_CALL.fullmatch(value)
    if promise_match is not None:
        arguments = parse_static_exec_command_list(promise_match.group("calls"), bindings)
        variable = promise_match.group("variable")
        forward = promise_match.group("forward").strip()
        if variable.startswith("["):
            names = tuple(name.strip() for name in variable[1:-1].split(","))
            if (arguments is None or len(names) != len(arguments) or len(set(names)) != len(names)
                or any(not re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_$]*", name)
                       or name in bindings or name in {"tools", "text", "JSON", "Promise"} for name in names)):
                return None
            named = re.fullmatch(r"text\s*\(\s*JSON\.stringify\s*\(\s*\{(.*?)\}\s*\)\s*\)\s*;", forward, re.DOTALL)
            if named:
                fields = [field.strip().split(":") for field in named[1].split(",")]
                mapping = {field[0].strip(): field[-1].strip() for field in fields if len(field) in {1, 2}}
                if (len(mapping) == len(names) and set(mapping.values()) == set(names)
                    and len(fields) == len(names)
                    and all(re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_$]*", key) for key in mapping)):
                    keys = tuple(next(key for key, name in mapping.items() if name == n) for n in names)
                    return ParsedCustomCall("exec_command", arguments, "named_result", keys)
            labeled = re.findall(
                r'\s*text\s*\(\s*("(?:\\.|[^"\\])*")\s*\+\s*JSON\.stringify\s*\(\s*([A-Za-z_$][A-Za-z0-9_$]*)\s*\)\s*\)\s*;',
                forward,
            )
            if labeled:
                pattern = r'\s*text\s*\(\s*"(?:\\.|[^"\\])*"\s*\+\s*JSON\.stringify\s*\(\s*[A-Za-z_$][A-Za-z0-9_$]*\s*\)\s*\)\s*;'
                if (re.fullmatch(f"(?:{pattern})+", forward)
                    and tuple(name for _, name in labeled) == names):
                    return ParsedCustomCall("exec_command", arguments, "labeled_results",
                        tuple(JsLiteralParser(prefix).parse() for prefix, _ in labeled))
            # Bounded output-only concatenation has no provable numeric completion.
            literal = r'"(?:\\.|[^"\\])*"'
            reference = "(?:" + "|".join(re.escape(n) for n in names) + r")\.output"
            atom = f"(?:{literal}|{reference})"
            if re.fullmatch(rf"(?:\s*text\s*\(\s*{atom}(?:\s*\+\s*{atom})*\s*\)\s*;)+\s*", forward):
                return ParsedCustomCall("exec_command", arguments, "indeterminate")
            return None
        mode = indexed_promise_output_mode(
            forward, variable
        )
        if arguments is None or mode is None:
            return None
        return ParsedCustomCall("exec_command", arguments, mode)

    match = ASSIGNED_CALL.fullmatch(value)
    if match is None:
        return None
    variable = re.escape(match.group("variable"))
    if match.group("variable") in bindings or match.group("variable") in {"tools", "text", "JSON", "Promise"}:
        return None
    tool_name = match.group("tool")
    if tool_name not in {"exec_command", "write_stdin"}:
        return None
    forward = match.group("forward").strip()
    result_forward = re.fullmatch(
        rf"text\s*\(\s*(?:{variable}|JSON\.stringify\s*\(\s*{variable}\s*\))\s*\)\s*;",
        forward,
        re.DOTALL,
    )
    output_forward = re.fullmatch(rf"text\s*\(\s*{variable}\.output\s*\)\s*;", forward, re.DOTALL)
    correlated_projection = re.fullmatch(
        rf"(?P<split>text\s*\(\s*{variable}\.output\s*\)\s*;\s*)?"
        rf"text\s*\(\s*JSON\.stringify\s*\(\s*\{{(?P<fields>.*?)\}}\s*\)\s*\)\s*;",
        forward,
        re.DOTALL,
    )
    correlated_fields: set[str] | None = None
    if correlated_projection is not None:
        fields = [field.strip() for field in correlated_projection.group("fields").split(",")]
        parsed_fields: list[str] = []
        for field in fields:
            projection = re.fullmatch(
                rf"(?P<key>[A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*"
                rf"{variable}\.(?P<member>[A-Za-z_$][A-Za-z0-9_$]*)",
                field,
            )
            if projection is None or projection.group("key") != projection.group("member"):
                parsed_fields = []
                break
            parsed_fields.append(projection.group("key"))
        if (
            parsed_fields
            and len(parsed_fields) <= 16
            and len(parsed_fields) == len(set(parsed_fields))
            and "exit_code" in parsed_fields
            and set(parsed_fields) <= {"exit_code", "session_id", "output", "chunk_id",
                                       "wall_time_seconds", "original_token_count"}
        ):
            correlated_fields = set(parsed_fields)
    template_exit_forward = re.fullmatch(
        rf"(?:text\s*\(\s*{variable}\.output\s*\)\s*;\s*)?"
        rf"text\s*\(\s*`(?:\\n)?(?:exit=|exit_code=|exit:|EXIT:|EXIT |EXIT_CODE=)\$\{{{variable}\.exit_code\}}`\s*\)\s*;",
        forward,
        re.DOTALL,
    )
    if tool_name == "write_stdin" and result_forward is None:
        return None
    if all(
        item is None
        for item in (
            result_forward,
            output_forward,
            correlated_fields,
            template_exit_forward,
        )
    ):
        return None
    try:
        arguments = JsLiteralParser(match.group("argument"), bindings).parse()
    except EvidenceError:
        return None
    if not isinstance(arguments, dict):
        return None
    mode = (
        "result"
        if result_forward is not None
        else "projection"
        if correlated_fields is not None and correlated_projection.group("split") is None
        else "correlated_split"
        if correlated_fields is not None and "session_id" not in correlated_fields
        else "correlated_session"
        if correlated_fields is not None
        else "template_exit"
        if template_exit_forward is not None
        else "output"
    )
    return ParsedCustomCall(tool_name, arguments, mode, tuple(sorted(correlated_fields or ())))


def parse_direct_execution_sequence(value: str) -> tuple[ParsedCustomCall, ...] | None:
    """Closed sequential forwarding grammar, with one output item per call.

    No expressions, callbacks, comments, dynamic bindings or extra output. Parsing
    consumes literals and punctuation only; JavaScript is never executed.
    """
    prefix = re.compile(r'\s*text\s*\(\s*await\s+tools\.(exec_command|write_stdin)\s*\(')
    suffix = re.compile(r'\s*\)\s*\)\s*;')
    offset, calls = 0, []
    while offset < len(value) and value[offset:].strip():
        match = prefix.match(value, offset)
        if match is None or len(calls) >= 16:
            return None
        parser = JsLiteralParser(value[match.end():])
        try:
            arguments = parser.value()
        except (EvidenceError, RecursionError):
            return None
        closing = suffix.match(value, match.end() + parser.offset)
        if closing is None or not isinstance(arguments, dict):
            return None
        calls.append(ParsedCustomCall(match[1], arguments, 'result'))
        offset = closing.end()
    return tuple(calls) if calls else None


def execution_tool_references(source: Any) -> tuple[str, ...]:
    """Lexical references only, excluding strings, comments and regex bodies.

    This scan does not prove execution or interpret JS. It merely prevents a
    shell-bearing unsupported cell from being silently represented as absence.
    Template interpolation code is scanned, while template text is skipped.
    """
    if not isinstance(source, str):
        return ()
    tokens, found = [], set()
    identifier = re.compile(r'[A-Za-z_$][A-Za-z0-9_$]*')
    def push(token):
        tokens.append(token)
        if len(tokens) > 3:
            del tokens[0]
        if len(tokens) == 3 and tokens[:2] == ['tools', '.'] and tokens[2] in {'exec_command', 'write_stdin'}:
            found.add(tokens[2])
    offset, template_depth = 0, []
    in_template = False
    while offset < len(source):
        character = source[offset]
        if in_template:
            if character == '\\':
                offset += 2
                continue
            if character == '`':
                in_template = False
            elif source.startswith('${', offset):
                template_depth.append(0)
                in_template = False
                push(';')
                offset += 2
                continue
            offset += 1
            continue
        if character in ' \t\r\n':
            offset += 1
            continue
        if source.startswith('//', offset):
            end = source.find('\n', offset + 2)
            offset = end if end >= 0 else len(source)
            continue
        if source.startswith('/*', offset):
            end = source.find('*/', offset + 2)
            offset = end + 2 if end >= 0 else len(source)
            continue
        if character in '\"\'':
            quote = character
            offset += 1
            while offset < len(source):
                if source[offset] == '\\':
                    offset += 2
                elif source[offset] == quote:
                    offset += 1
                    break
                else:
                    offset += 1
            push('literal')
            continue
        if character == '`':
            in_template = True
            offset += 1
            push('literal')
            continue
        if character == '/' and (not tokens or tokens[-1] in {'=', '(', ',', '[', ':', ';', '{', 'return', '!', '>'}):
            # Regex lexical body, including escaped slash and character classes.
            offset += 1
            bracket = False
            while offset < len(source):
                if source[offset] == '\\':
                    offset += 2
                    continue
                if source[offset] == '[':
                    bracket = True
                elif source[offset] == ']':
                    bracket = False
                elif source[offset] == '/' and not bracket:
                    offset += 1
                    break
                offset += 1
            push('literal')
            continue
        if template_depth:
            if character == '{':
                template_depth[-1] += 1
            elif character == '}':
                if template_depth[-1] == 0:
                    template_depth.pop()
                    in_template = True
                    offset += 1
                    push(';')
                    continue
                template_depth[-1] -= 1
        match = identifier.match(source, offset)
        token = match[0] if match else character
        push(token)
        offset += len(token)
    return tuple(sorted(found))


def parse_mcp_wrapper(value: Any) -> ParsedMcpWrapper | None:
    """Parse only a single static Volicord invocation for completion correlation."""
    if not isinstance(value, str) or len(value.encode("utf-8")) > 64 * 1024:
        return None
    match = MCP_WRAPPER.fullmatch(value)
    if match is None or "tools.mcp__" in match.group("forward"):
        return None
    operation = normalize_operation(match.group("operation"))
    if operation is None:
        return None
    try:
        arguments = JsLiteralParser(match.group("argument")).parse()
    except EvidenceError:
        return None
    if not isinstance(arguments, dict):
        return None
    return ParsedMcpWrapper(operation, arguments)


def normalize_mcp_completion(
    payload: dict[str, Any],
) -> tuple[str | None, dict[str, Any] | None, dict[str, Any], str, str | None]:
    """Normalize the maintained legacy MCP completion representation."""
    invocation = payload.get("invocation")
    if not isinstance(invocation, dict) or invocation.get("server") != "volicord":
        return None, None, {}, "ignored", None
    operation = normalize_operation(invocation.get("tool"))
    arguments = invocation.get("arguments")
    if operation is None or not isinstance(arguments, dict):
        return operation, None, {}, "failed", "malformed_mcp_completion"
    result = payload.get("result")
    if not isinstance(result, dict) or len(result) != 1:
        return operation, arguments, {}, "failed", "malformed_mcp_completion"
    if "Err" in result:
        raw_error = result.get("Err")
        return operation, arguments, {}, "failed", str(raw_error) if raw_error is not None else "mcp_error"
    ok = result.get("Ok")
    if not isinstance(ok, dict):
        return operation, arguments, {}, "failed", "malformed_mcp_completion"
    is_error, structured = mcp_result_semantics(ok)
    if not isinstance(is_error, bool) or not isinstance(structured, dict):
        return operation, arguments, {}, "failed", "malformed_mcp_completion"
    if is_error:
        raw_error = structured.get("error")
        return operation, arguments, structured, "failed", str(raw_error) if nonempty(raw_error) else "mcp_error"
    return operation, arguments, structured, "succeeded", None


def normalize_current_mcp_completion(
    payload: dict[str, Any], session_id: str
) -> tuple[str | None, str | None, dict[str, Any] | None, dict[str, Any], str, str | None] | None:
    """Normalize ItemCompleted(McpToolCall) by shape and semantic result."""
    if payload.get("type") != "item_completed":
        return None
    item = payload.get("item")
    if not isinstance(item, dict) or item.get("type") != "McpToolCall":
        return None
    server = item.get("server")
    if server != "volicord":
        return None
    if payload.get("thread_id") != session_id or not nonempty(payload.get("turn_id")):
        raise EvidenceError("Codex McpToolCall thread or turn identity is malformed")
    call_id = item.get("id")
    operation = normalize_operation(item.get("tool"))
    arguments = item.get("arguments")
    if not nonempty(call_id):
        raise EvidenceError("Codex McpToolCall item identity is malformed")
    if operation is None:
        return None
    if not isinstance(arguments, dict):
        return str(call_id), operation, None, {}, "failed", "malformed_mcp_completion"

    status = item.get("status")
    result = item.get("result")
    if status not in {"completed", "failed"}:
        return str(call_id), operation, arguments, {}, "failed", "unsupported_mcp_completion_status"
    if not isinstance(result, dict):
        return str(call_id), operation, arguments, {}, "failed", "malformed_mcp_completion"
    is_error = result.get("isError")
    if not isinstance(is_error, bool):
        return str(call_id), operation, arguments, {}, "failed", "malformed_mcp_completion"
    if not isinstance(result.get("content"), list):
        return str(call_id), operation, arguments, {}, "failed", "malformed_mcp_completion"
    is_error, structured = mcp_result_semantics(result)
    if not isinstance(is_error, bool) or not isinstance(structured, dict):
        return str(call_id), operation, arguments, {}, "failed", "malformed_mcp_completion"
    if status == "completed" and not is_error:
        return str(call_id), operation, arguments, structured, "succeeded", None
    if status == "failed" and is_error:
        raw_error = structured.get("error")
        return (
            str(call_id),
            operation,
            arguments,
            structured,
            "failed",
            str(raw_error) if nonempty(raw_error) else "mcp_error",
        )
    return str(call_id), operation, arguments, structured, "failed", "mcp_completion_status_mismatch"


def mcp_result_semantics(result: dict[str, Any]) -> tuple[bool | None, dict[str, Any] | None]:
    """One bounded semantic decoder shared by legacy and ItemCompleted MCP.

    Text is only a fallback when the entire block is a CallToolResult object.
    Its optional product-text representation must parse and agree too. Ordinary
    display text alongside a direct structured result is not success evidence.
    """
    is_error = result.get("isError")
    structured = result.get("structuredContent")
    if ("isError" in result and not isinstance(is_error, bool)) or (
        structured is not None and not isinstance(structured, dict)
    ):
        return None, None
    content = result.get("content", [])
    if not isinstance(content, list):
        return None, None
    if content:
        if (len(content) != 1 or not isinstance(content[0], dict)
            or set(content[0]) != {"type", "text"} or content[0].get("type") != "text"
            or not isinstance(content[0].get("text"), str)
            or len(content[0]["text"]) > MAX_MCP_CONTENT_RESULT_CHARS):
            return None, None
        try:
            envelope = json.loads(content[0]["text"])
        except json.JSONDecodeError:
            # A purported serialized object that is truncated is not display prose.
            if structured is None or content[0]["text"].lstrip().startswith("{"):
                return None, None
            envelope = None
        if isinstance(envelope, dict) and set(envelope) & {"content", "structuredContent", "isError"}:
            if (set(envelope) not in (
                {"content", "isError"}, {"structuredContent", "isError"},
                {"content", "structuredContent", "isError"},
            ) or not isinstance(envelope.get("isError"), bool)):
                return None, None
            nested_error = envelope["isError"]
            if is_error is not None and is_error != nested_error:
                raise EvidenceError("Codex MCP error representations conflict")
            nested = envelope.get("structuredContent")
            if nested is not None and not isinstance(nested, dict):
                return None, None
            if "content" in envelope:
                inner = envelope["content"]
                if (not isinstance(inner, list) or len(inner) != 1
                    or not isinstance(inner[0], dict) or set(inner[0]) != {"type", "text"}
                    or inner[0].get("type") != "text" or not isinstance(inner[0].get("text"), str)
                    or len(inner[0]["text"]) > MAX_MCP_CONTENT_RESULT_CHARS):
                    return None, None
                try:
                    product = json.loads(inner[0]["text"])
                except json.JSONDecodeError:
                    return None, None
                if not isinstance(product, dict):
                    return None, None
                if nested is not None and nested != product:
                    raise EvidenceError("Codex MCP content result representations conflict")
                nested = product
            if not isinstance(nested, dict):
                return None, None
            if structured is not None and structured != nested:
                raise EvidenceError("Codex MCP structured result representations conflict")
            is_error, structured = nested_error, nested
    return is_error, structured


def merge_tool_call_evidence(evidence: list[_ToolCallEvidence]) -> tuple[ToolCall, ...]:
    """Deduplicate equivalent transports and reject identity conflicts."""
    by_identity: dict[tuple[str, str], _ToolCallEvidence] = {}
    representations: dict[tuple[str, str], set[str]] = {}
    for candidate in sorted(evidence, key=lambda value: value.sequence):
        identity = (candidate.server, candidate.call_id)
        prior = by_identity.get(identity)
        if prior is None:
            by_identity[identity] = candidate
            representations[identity] = {candidate.representation}
            continue
        if (
            prior.turn_id != candidate.turn_id
            or prior.operation != candidate.operation
            or prior.arguments != candidate.arguments
            or prior.result != candidate.result
            or prior.outcome != candidate.outcome
            or prior.error != candidate.error
        ):
            raise EvidenceError("Codex MCP completion representations conflict")
        representations[identity].add(candidate.representation)
    return tuple(
        ToolCall(
            value.sequence,
            value.completion_sequence,
            value.turn_id,
            value.call_id,
            value.server,
            value.operation,
            value.arguments,
            value.result,
            value.outcome,
            value.error,
            tuple(sorted(representations[identity])),
        )
        for identity, value in sorted(
            by_identity.items(), key=lambda item: item[1].sequence
        )
    )


CUSTOM_OUTPUT_HEADER = re.compile(
    r"\AScript completed\nWall time [0-9]+(?:\.[0-9]+)? seconds\nOutput:\n(?P<body>.*)\Z",
    re.DOTALL,
)


def custom_cell_without_command_completion(value: Any) -> bool:
    """A yielded or failed exec cell does not prove a command's exit status."""
    parts = custom_output_parts(value)
    body = value if isinstance(value, str) else "".join(parts) if parts else ""
    return re.fullmatch(
        r"Script (?:running with cell ID [A-Za-z0-9_-]{1,128}|failed)\n"
        r"Wall time [0-9]+(?:\.[0-9]+)? seconds\nOutput:\n.*",
        body, re.DOTALL,
    ) is not None


def custom_output_body(value: Any) -> str | None:
    parts = custom_output_parts(value)
    if parts is None:
        return None
    match = CUSTOM_OUTPUT_HEADER.fullmatch("".join(parts))
    return match.group("body") if match is not None else None


def custom_output_parts(value: Any) -> list[str] | None:
    if not isinstance(value, list) or not value or len(value) > 17:
        return None
    parts: list[str] = []
    for item in value:
        if not isinstance(item, dict) or item.get("type") != "input_text" or not isinstance(item.get("text"), str):
            return None
        parts.append(item["text"])
    return parts


def custom_correlated_command_result(
    value: Any, *, fields: tuple[str, ...]
) -> tuple[str, int | None, int | None] | None:
    parts = custom_output_parts(value)
    if parts is None or len(parts) != 3:
        return None
    header = CUSTOM_OUTPUT_HEADER.fullmatch(parts[0])
    if header is None or header.group("body"):
        return None
    try:
        status = strict_json(parts[2])
    except (ValueError, RecursionError):
        return None
    if (
        not isinstance(status, dict)
        or "exit_code" not in status
        or not set(status) <= set(fields)
    ):
        return None
    if "session_id" in status and status["session_id"] is not None and (
        isinstance(status["session_id"], bool)
        or not isinstance(status["session_id"], int)
    ):
        return None
    exit_code = status["exit_code"]
    if exit_code is not None and (type(exit_code) is not int or not 0 <= exit_code <= 2_147_483_647):
        return None
    session_id = status.get("session_id")
    return parts[1], exit_code, session_id


def custom_template_command_result(value: Any) -> tuple[str, int] | None:
    parts = custom_output_parts(value)
    if parts is None or len(parts) not in {2, 3}:
        return None
    header = CUSTOM_OUTPUT_HEADER.fullmatch(parts[0])
    status = re.fullmatch(r"\n?(?:exit=|exit_code=|exit:|EXIT:|EXIT |EXIT_CODE=)([0-9]+)", parts[-1])
    if header is None or header.group("body") or status is None:
        return None
    exit_code = int(status.group(1))
    return (parts[1] if len(parts) == 3 else "", exit_code) if exit_code <= 2_147_483_647 else None


def custom_indexed_command_results(
    value: Any, mode: str, count: int
) -> list[tuple[str, int]] | None:
    parts = custom_output_parts(value)
    if parts is None or len(parts) != count + 1:
        return None
    header = CUSTOM_OUTPUT_HEADER.fullmatch(parts[0])
    if header is None or header.group("body"):
        return None
    patterns = {
        "indexed_entries_zero": re.compile(
            r"---(?:FILE)?(?P<index>[0-9]+) exit=(?P<exit>[0-9]+)\n(?P<output>.*)\Z",
            re.DOTALL,
        ),
        "indexed_suffix_zero": re.compile(
            r"[A-Za-z][A-Za-z0-9 _-]{0,31}(?P<index>[0-9]+)\n"
            r"(?P<output>.*)\nexit=(?P<exit>[0-9]+)\Z",
            re.DOTALL,
        ),
        "indexed_prefix_one": re.compile(
            r"[A-Za-z][A-Za-z0-9 _-]{0,31}(?P<index>[0-9]+)\s+exit="
            r"(?P<exit>[0-9]+)\n(?P<output>.*)\Z",
            re.DOTALL,
        ),
        "indexed_suffix_one": re.compile(
            r"[A-Za-z][A-Za-z0-9 _-]{0,31}(?P<index>[0-9]+)\n"
            r"(?P<output>.*)\nEXIT\s+(?P<exit>[0-9]+)\Z",
            re.DOTALL,
        ),
    }
    pattern = patterns.get(mode)
    if pattern is None:
        return None
    expected_base = 0 if mode in {"indexed_suffix_zero", "indexed_entries_zero"} else 1
    results: list[tuple[str, int]] = []
    for position, part in enumerate(parts[1:]):
        match = pattern.fullmatch(part)
        if match is None or int(match.group("index")) != position + expected_base:
            return None
        exit_code = int(match.group("exit"))
        if exit_code > 2_147_483_647:
            return None
        results.append((match.group("output"), exit_code))
    return results


def custom_output_object(value: Any) -> dict[str, Any] | None:
    body = custom_output_body(value)
    if body is None:
        return None
    try:
        parsed = strict_json(body)
    except (ValueError, RecursionError):
        return None
    return parsed if isinstance(parsed, dict) else None


def forwarded_output_state(result, arguments):
    if not isinstance(result, dict) or not isinstance(result.get('output'), str):
        return 'missing'
    original = result.get('original_token_count')
    limit = arguments.get('max_output_tokens', 10000) if isinstance(arguments, dict) else 10000
    if (type(original) is int and type(limit) is int and original > limit > 0
            or result['output'].startswith('Warning: truncated output (original token count:')):
        return 'truncated'
    return 'retained'


@dataclass(frozen=True)
class WorkTurn:
    turn_id: str
    start_sequence: int
    end_sequence: int | None
    state: str


@dataclass(frozen=True)
class TurnLifecycle:
    state: str
    turns: tuple[WorkTurn, ...]
    issues: tuple[str, ...]

    @property
    def last_interruption(self) -> int:
        return max((turn.end_sequence for turn in self.turns
                    if turn.state == "interrupted" and turn.end_sequence is not None), default=-1)

    def bounded_evidence(self) -> dict[str, Any]:
        interrupted = [turn for turn in self.turns if turn.state == "interrupted"]
        return {
            "state": self.state, "turn_count": len(self.turns),
            "interrupted_turn_count": len(interrupted),
            "last_interruption_sequence": self.last_interruption,
            "terminal_completion_sequence": (
                self.turns[-1].end_sequence if self.turns and self.turns[-1].state == "completed" else None
            ),
            "interrupted_turns": [{"turn_id": turn.turn_id, "start_sequence": turn.start_sequence,
                                   "interruption_sequence": turn.end_sequence} for turn in interrupted[-32:]],
            "interrupted_turns_truncated": len(interrupted) > 32,
            "issues": list(self.issues),
        }

    def contains_completion(self, turn_id: str, start: int, end: int) -> bool:
        return any(turn.turn_id == turn_id and turn.start_sequence < start <= end
                   and (turn.end_sequence is None or end < turn.end_sequence)
                   for turn in self.turns)


def normalize_turn_lifecycle(events: list[dict[str, Any]]) -> TurnLifecycle:
    """Use ordered starts; identity-less completion closes only the active turn.

    A new distinct start interrupts an open turn. Duplicate starts, late or
    orphan completions and conflicting context are evidence, never repairs.
    """
    turns: list[WorkTurn] = []
    issues: list[str] = []
    seen: set[str] = set()
    active: int | None = None
    for sequence, event in enumerate(events):
        payload = event.get("payload")
        if not isinstance(payload, dict):
            continue
        if event.get("type") == "turn_context" and "turn_id" in payload:
            if active is None or payload["turn_id"] != turns[active].turn_id:
                issues.append("turn_context_conflict")
        if event.get("type") != "event_msg":
            continue
        kind = payload.get("type")
        identity = payload.get("turn_id")
        if kind == "task_started":
            if not nonempty(identity):
                issues.append("start_identity_missing")
                continue
            if identity in seen:
                issues.append("start_identity_reused")
                continue
            seen.add(identity)
            if active is not None:
                turns[active] = replace(turns[active], end_sequence=sequence, state="interrupted")
            turns.append(WorkTurn(identity, sequence, None, "incomplete"))
            active = len(turns) - 1
        elif kind in {"task_complete", "task_completed", "turn_aborted"}:
            if active is None:
                issues.append("terminal_without_active_turn")
                continue
            if "turn_id" in payload and identity != turns[active].turn_id:
                issues.append("terminal_identity_conflict")
                continue
            turns[active] = replace(turns[active], end_sequence=sequence,
                                    state="interrupted" if kind == "turn_aborted" else "completed")
            active = None
    state = (
        "indeterminate" if issues or not turns else
        "terminal_incomplete" if turns[-1].state != "completed" else
        "completed_after_interruption" if any(turn.state == "interrupted" for turn in turns) else
        "completed"
    )
    return TurnLifecycle(state, tuple(turns), tuple(sorted(set(issues))))


def capture_bounds(events):
    """Retain bounded host clock evidence; missing/backward clocks stay unknown."""
    values = []
    try:
        for event in events:
            value = dt.datetime.fromisoformat(event['timestamp'].replace('Z', '+00:00'))
            if value.tzinfo is None:
                return None
            values.append(value)
    except (KeyError, ValueError, TypeError, AttributeError):
        return None
    if not values or any(a > b for a,b in zip(values, values[1:])):
        return None
    return {'first': values[0].isoformat(), 'last': values[-1].isoformat(), 'last_sequence': len(events) - 1}


@dataclass(frozen=True)
class CodexCapture:
    source_sha256: str
    session_id: str
    cwd: Path
    git_revision: str | None
    source: str
    originator: str
    cli_version: str
    thread_source: str
    capture_format: str
    observed_metadata: dict[str, Any]
    fresh_user_thread: bool
    repository_scoped_activation_observed: bool
    activation_evidence_state: str
    turn_lifecycle: TurnLifecycle
    task_sequences: tuple[int, ...]
    completed_task_sequences: tuple[int, ...]
    compacted_sequences: tuple[int, ...]
    user_turns: tuple[UserTurn, ...]
    tool_calls: tuple[ToolCall, ...]
    evidence_transport_state: str
    evidence_transport_issues: tuple[EvidenceTransportIssue, ...]
    path_observations: tuple[PathObservation, ...]
    commands: tuple[CommandObservation, ...]
    async_question_requests: tuple[AsyncQuestionRequest, ...] = ()
    execution_wrappers: tuple[ExecutionWrapperObservation, ...] = ()

    def execution_evidence(self) -> dict[str, Any]:
        observations = [dict(vars(item), raw_capture_sha256=self.source_sha256,
            tool_names=list(item.tool_names), reasons=list(item.reasons))
            for item in self.execution_wrappers]
        return {'state': ('not_observed' if not observations and not self.commands
            else 'limited' if any(item.state != 'normalized' for item in self.execution_wrappers)
            or any(command.evidence_state != 'completed' for command in self.commands) else 'observed'),
            'normalized_command_count': len(self.commands),
            'completed_command_count': sum(command.evidence_state == 'completed' for command in self.commands),
            'failed_command_count': sum(command.evidence_state == 'completed' and command.exit_code != 0 for command in self.commands),
            'unsupported_wrapper_count': sum(item.state == 'unsupported' for item in self.execution_wrappers),
            'indeterminate_wrapper_count': sum(item.state == 'indeterminate' for item in self.execution_wrappers),
            'wrappers': observations}

    def provenance_evidence(self) -> dict[str, Any]:
        """Host-recorded observations; source/originator do not attest a UI."""
        import evidence_purpose
        purpose = evidence_purpose.capture_purpose(self)
        return {"host_family": "test_support" if purpose == evidence_purpose.REHEARSAL else "codex",
                "evidence_purpose": purpose, "capture_format": self.capture_format,
                "ui_surface": "unknown", "observed_metadata": self.observed_metadata}

    def calls(self, operation: str) -> list[ToolCall]:
        return [call for call in self.tool_calls if call.operation == operation]

    def successful_calls(self, operation: str) -> list[ToolCall]:
        return [call for call in self.calls(operation) if call.outcome == "succeeded"]

    def transport_issues(self, *operations: str) -> list[EvidenceTransportIssue]:
        expected = set(operations)
        return [
            issue
            for issue in self.evidence_transport_issues
            if not expected or issue.operation in expected
        ]

    def turn_for_call(self, call: ToolCall) -> UserTurn | None:
        # A task can contain multiple distinct user messages (for example an
        # asynchronous Question reply). The normalizer preserves their client
        # identities; task turn_id alone is not the response identity. Select
        # the unique latest message before invocation, never by matching text
        # or by searching past a more recent user message.
        preceding = [turn for turn in self.user_turns if turn.sequence < call.sequence]
        if not preceding:
            return None
        latest_sequence = max(turn.sequence for turn in preceding)
        matches = [turn for turn in preceding if turn.sequence == latest_sequence]
        return (matches[0] if len(matches) == 1
                and matches[0].turn_id == call.turn_id else None)

    def response_transport_for_call(
        self, call: ToolCall, compare: Callable[[Any, Any], dict[str, Any]],
    ) -> dict[str, Any]:
        """Verify selected-answer transport against independently captured host events.

        Caller request coordinates identify an item; they never authenticate it.
        Canonical provenance is checked separately by the evaluator.
        """
        turn = self.turn_for_call(call)
        caller = call.arguments.get("user_turn")
        reference = call.arguments.get("async_reply")
        result = {**compare(caller, None), "response_kind": "unverified",
            "failure_kind": "response_binding_invalid"}
        if turn is None:
            return result
        wrapped = (turn.text.lstrip().startswith("<")
            or "<send_user_message_question_reply>" in turn.text
            or "</send_user_message_question_reply>" in turn.text)
        if not wrapped:
            if reference is not None:
                return result
            comparison = compare(caller, turn.text)
            return {**comparison, "response_kind": "plain_user_message",
                "failure_kind": None if comparison["equivalent"] else "answer_text_mismatch"}
        result["raw_host_text_sha256"] = sha256_bytes(turn.text.encode("utf-8"))
        replies = parse_async_question_replies(turn.text)
        if replies is None:
            return result
        # Validate every item before selecting one. A valid selected item cannot
        # hide malformed/cross-request evidence elsewhere in the same envelope.
        bound = []
        for reply in replies:
            requests = [request for request in self.async_question_requests
                if request.call_id == reply.call_id]
            if len(requests) != 1:
                return result
            request = requests[0]
            if (request.session_id != self.session_id or request.turn_id != turn.turn_id
                or not request.sequence < request.completion_sequence < turn.sequence < call.sequence
                or reply.question_index >= len(request.questions)
                or not compare(request.questions[reply.question_index], reply.question)["equivalent"]):
                return result
            bound.append((reply, request))
        if call.operation == "decision_record":
            if (not isinstance(reference, dict)
                or set(reference) != {"request_call_id", "question_index"}
                or not nonempty(reference["request_call_id"])
                or type(reference["question_index"]) is not int
                or reference["question_index"] < 0):
                # Historical wrappers stay invalid and unchanged. Their matching
                # bytes establish a representation error, not fabricated consent.
                if caller == turn.text:
                    return {**result, "failure_kind": "raw_envelope_as_answer"}
                return result
            selected = [(reply, request) for reply, request in bound
                if (reply.call_id, reply.question_index) ==
                    (reference["request_call_id"], reference["question_index"])]
        else:
            # Non-Decision consumers retain their existing bounded contract.
            selected = [(reply, request) for reply, request in bound
                if compare(caller, reply.answer)["equivalent"]]
        if len(selected) != 1:
            return result
        reply, request = selected[0]
        comparison = compare(caller, reply.answer)
        return {**comparison, "response_kind": "async_question_reply",
            "failure_kind": (None if comparison["equivalent"] else
                "raw_envelope_as_answer" if caller == turn.text else "answer_text_mismatch"),
            "transport_equivalence_used": True,
            "answer_transport_equivalence_used": comparison["transport_equivalence_used"],
            "raw_host_text_sha256": sha256_bytes(turn.text.encode("utf-8")),
            "answer_text_sha256": sha256_bytes(reply.answer.encode("utf-8")),
            "async_request_call_id": request.call_id,
            "async_request_session_id": request.session_id,
            "async_request_turn_id": request.turn_id,
            "async_request_sequence": request.sequence,
            "async_request_completion_sequence": request.completion_sequence,
            "async_question_index": reply.question_index,
            "async_question_text_sha256": sha256_bytes(reply.question.encode("utf-8"))}

    def paths_before(self, sequence: int) -> list[str]:
        return sorted({path for item in self.path_observations if item.sequence < sequence for path in item.paths})

    def paths_after(self, sequence: int) -> list[str]:
        return sorted({path for item in self.path_observations if item.sequence > sequence for path in item.paths})

    def first_inspection_after(self, sequence: int) -> int | None:
        candidates = [
            call.sequence
            for call in self.tool_calls
            if call.sequence > sequence
            and repository_operation_is_inspection(call)
        ]
        candidates.extend(
            command.sequence
            for command in self.commands
            if command.sequence > sequence
            and command_is_repository_inspection(command.parsed_command)
        )
        return min(candidates) if candidates else None

    def clean_git_status_before(self, sequence: int) -> bool:
        return any(
            command.sequence < sequence
            and command.exit_code == 0
            and command.output_was_empty
            and command_is_clean_git_status(command.parsed_command)
            for command in self.commands
        )


def split_command(value: str) -> tuple[str, ...] | None:
    try:
        parsed = tuple(shlex.split(value))
    except ValueError:
        return None
    return parsed or None


def split_static_compound_command(value: str) -> list[tuple[str, ...]]:
    """Split only bounded static shell control forms for read-only classification."""
    try:
        lexer = shlex.shlex(value, posix=True, punctuation_chars=";&|<>\n")
        lexer.whitespace = " \t\r"
        lexer.whitespace_split = True
        tokens = list(lexer)
    except ValueError:
        return []
    if not tokens or len(tokens) > 128 or any(
        "$" in token or "`" in token or token in {"<", ">", ">>", "<<", "&"}
        for token in tokens
    ):
        return []
    segments: list[tuple[str, ...]] = []
    current: list[str] = []
    for token in tokens:
        if token and set(token) == {"\n"}:
            if current:
                segments.append(tuple(current))
                current = []
            continue
        if token in {"&&", "||", ";", "|"}:
            if not current:
                return []
            segments.append(tuple(current))
            current = []
        else:
            current.append(token)
    if current:
        segments.append(tuple(current))
    elif not tokens[-1].endswith("\n"):
        return []
    return segments if len(segments) <= 16 else []


def command_argvs(value: Any) -> list[tuple[str, ...]]:
    if isinstance(value, str):
        return split_static_compound_command(value)
    if isinstance(value, list):
        if value and all(isinstance(item, str) for item in value):
            return [tuple(value)]
        return [argv for child in value for argv in command_argvs(child)]
    if not isinstance(value, dict):
        return []

    result: list[tuple[str, ...]] = []
    for key in ("cmd", "command"):
        command = value.get(key)
        if isinstance(command, str):
            result.extend(split_static_compound_command(command))
    argv = value.get("argv")
    if isinstance(argv, list) and argv and all(isinstance(item, str) for item in argv):
        result.append(tuple(argv))
    program = value.get("program")
    arguments = value.get("args")
    if isinstance(program, str) and isinstance(arguments, list) and all(
        isinstance(item, str) for item in arguments
    ):
        result.append((program, *arguments))
    return result


def command_is_clean_git_status(value: Any) -> bool:
    return any(
        len(argv) >= 3
        and Path(argv[0]).name.lower() == "git"
        and argv[1].lower() == "status"
        and any(option == "--short" or option.startswith("--porcelain") for option in argv[2:])
        for argv in command_argvs(value)
    )


def bounded_find_source_report(value: Any) -> bool:
    raw = value.get("cmd") if isinstance(value, dict) else value
    return isinstance(raw, str) and re.fullmatch(
        r"find [A-Za-z0-9_./-]+ -maxdepth [1-9][0-9]? -type f -print "
        r"-exec sed -n '[0-9]+,[0-9]+p' \{\} \\;", raw
    ) is not None


def bounded_python_assertion_validation(value: Any) -> bool:
    """Recognize a closed assertion script transport, without executing Python.

    This is validation intent, not a read-only claim or inferred test outcome.
    Shell tails, dynamic execution, swallowed assertions and empty loops are
    excluded. The captured command must still prove numeric successful exit.
    """
    raw = value.get("cmd") if isinstance(value, dict) else value
    if not isinstance(raw, str) or len(raw) > 16_384:
        return False
    match = re.fullmatch(
        r"(?:command -v python3; )?(?:PYTHONPATH=[A-Za-z0-9_./:-]+ )?"
        r"python3 - <<'([A-Za-z_][A-Za-z0-9_]*)'\n(.+)\n\1\n?", raw, re.DOTALL)
    if match is None:
        return False
    try:
        tree = ast.parse(match[2])
    except (SyntaxError, ValueError, RecursionError):
        return False
    nodes = list(ast.walk(tree))
    if len(nodes) > 512 or not any(isinstance(n, ast.Assert) and isinstance(n.test, ast.Compare)
        and any(isinstance(v, (ast.Name, ast.Call)) for v in ast.walk(n.test)) for n in nodes):
        return False
    allowed_statements = (ast.Import, ast.ImportFrom, ast.Assign, ast.For, ast.Assert, ast.Expr)
    forbidden_names = {"eval", "exec", "compile", "open", "exit", "quit", "getattr", "setattr", "globals", "locals",
        "os", "sys", "subprocess", "shutil", "socket", "pathlib", "importlib", "builtins"}
    for node in nodes:
        if isinstance(node, ast.stmt) and not isinstance(node, allowed_statements):
            return False
        if isinstance(node, ast.Assign) and not all(isinstance(target, ast.Name) for target in node.targets):
            return False
        if isinstance(node, ast.Expr) and not (isinstance(node.value, ast.Call)
            and isinstance(node.value.func, ast.Name) and node.value.func.id == "print"):
            return False
        if isinstance(node, (ast.Lambda, ast.NamedExpr, ast.Await, ast.Yield, ast.YieldFrom)):
            return False
        if isinstance(node, (ast.Name, ast.Attribute)):
            name = node.id if isinstance(node, ast.Name) else node.attr
            if name.startswith("_") or name in forbidden_names:
                return False
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            modules = [a.name for a in node.names]
            if isinstance(node, ast.ImportFrom):
                modules.append(node.module or "")
            if any(m.split(".")[0] in forbidden_names or m.startswith("_") for m in modules):
                return False
    assigned_names = [target.id for node in nodes if isinstance(node, ast.Assign)
        for target in node.targets if isinstance(target, ast.Name)]
    nonempty_literals = {target.id for statement in tree.body if isinstance(statement, ast.Assign)
        and isinstance(statement.value, (ast.Dict, ast.List, ast.Tuple))
        and bool(statement.value.keys if isinstance(statement.value, ast.Dict) else statement.value.elts)
        for target in statement.targets if isinstance(target, ast.Name) and assigned_names.count(target.id) == 1}
    for node in nodes:
        if not isinstance(node, ast.For):
            continue
        iterable = node.iter
        if isinstance(iterable, ast.Call) and isinstance(iterable.func, ast.Attribute) and iterable.func.attr == "items" and not iterable.args and not iterable.keywords:
            iterable = iterable.func.value
        if not (isinstance(iterable, (ast.List, ast.Tuple)) and bool(iterable.elts)
            or isinstance(iterable, ast.Name) and iterable.id in nonempty_literals):
            return False
    return True


def command_is_repository_inspection(value: Any) -> bool:
    if bounded_find_source_report(value):
        return True
    inspection_programs = {
        "cat",
        "fd",
        "find",
        "grep",
        "head",
        "ls",
        "nl",
        "sort",
        "rg",
        "sed",
        "stat",
        "tail",
        "tree",
        "pwd",
        "wc",
    }
    git_inspections = {
        "diff",
        "grep",
        "log",
        "ls-files",
        "rev-parse",
        "show",
        "status",
    }
    def read_only(argv: tuple[str, ...], depth: int = 0) -> bool:
        if not argv or depth > 3:
            return False
        program = Path(argv[0]).name.lower()
        if program in {"sh", "bash", "zsh"}:
            parts = split_static_compound_command(argv[2]) if len(argv) == 3 and argv[1] in {"-c", "-lc"} else []
            return bool(parts) and all(read_only(part, depth + 1) for part in parts)
        if program == "git":
            args = list(argv[1:])
            while args:
                if args[0] == "-C" and len(args) >= 2:
                    args = args[2:]
                elif args[0] in {"--no-pager", "--literal-pathspecs", "--no-optional-locks"}:
                    args = args[1:]
                elif args[:2] == ["-c", "core.fsmonitor=false"]:
                    # The campaign's terminal status disables external fsmonitor.
                    args = args[2:]
                else:
                    break
            return bool(args) and args[0] in git_inspections and not any(
                arg == "--output" or arg.startswith("--output=") or arg in {"--ext-diff", "--textconv"}
                for arg in args[1:]
            )
        if program == "cd":
            return len(argv) == 2
        if program == "find":
            return not any(arg.startswith(("-exec", "-ok", "-delete", "-fprint", "-fls")) for arg in argv[1:])
        if program == "sed":
            # Closed line-range scripts, including stdin after nl. No -e/-f,
            # writes, execution scripts or trailing options can enter this path.
            return (len(argv) >= 3 and argv[1] == "-n"
                and re.fullmatch(r"[0-9]+(?:,[0-9]+|,\$)?p(?:;[0-9]+(?:,[0-9]+|,\$)?p)*", argv[2]) is not None
                and all(not arg.startswith("-") for arg in argv[3:]))
        if program == "nl":
            return len(argv) == 3 and argv[1] == "-ba" and not argv[2].startswith("-")
        if program == "sort":
            return not any(arg.startswith(("-o", "--output", "--compress-program", "--files0-from")) for arg in argv[1:])
        if program == "fd":
            return not any(arg in {"-x", "-X"} or arg.startswith("--exec") for arg in argv[1:])
        if program in {"rg", "grep"} and any(arg.startswith(("--pre", "--hostname-bin")) for arg in argv[1:]):
            return False
        return program in inspection_programs

    argvs = command_argvs(value)
    return bool(argvs) and all(read_only(argv) for argv in argvs)


def nonexecuting_validation_mode(program: str, args: list[str]) -> bool:
    """Recognize explicit reporting modes of known validators, not script intent.

    This disqualifies execution evidence; it does not promise absence of import,
    configuration or build side effects. Compile/static checks remain validation.
    Only Cargo's explicit test-harness arguments are inspected past `--`.
    """
    if program in {"python", "python3"}:
        if args[:1] == ["-B"]:
            args = args[1:]
        if args[:2] == ["-m", "pytest"]:
            program, args = "pytest", args[2:]
    if program == "cargo":
        if args and args[0].startswith("+"):
            args = args[1:]
        return bool(args) and args[0] == "test" and "--" in args \
            and "--list" in args[args.index("--") + 1:]
    options = args[:args.index("--")] if "--" in args else args
    if program == "pytest":
        return any(arg in {"--collect-only", "--collectonly", "--co", "--setup-plan",
            "--fixtures", "--funcargs", "--fixtures-per-test"} for arg in options)
    if program == "ctest":
        return any(arg in {"-N", "--show-only", "--list-presets"}
            or arg.startswith("--show-only=") for arg in options)
    if program == "ruff" and options[:1] == ["check"]:
        return any(arg in {"--show-files", "--show-settings"} for arg in options)
    if program == "go" and options[:1] in (["test"], ["vet"], ["build"]):
        # -args hands the remaining words to the test binary, not the Go driver.
        driver = options[:options.index("-args")] if "-args" in options else options
        return any(arg in {"-n", "-n=true"} for arg in driver) or (
            options[0] == "test" and any(
                arg.startswith(("-list=", "-test.list=")) and bool(arg.split("=", 1)[1])
                or arg in {"-list", "-test.list"} and i + 1 < len(options) and bool(options[i + 1])
                for i, arg in enumerate(options)))
    if program == "make":
        return any(arg in {"-n", "--just-print", "--dry-run", "--recon"} for arg in options)
    if program in {"gradle", "gradlew"}:
        return any(arg in {"-m", "--dry-run", "--task-graph"} for arg in options)
    return False


def command_role(value: Any, depth: int = 0) -> str:
    """Bounded roles; an unknown command cannot supply successful validation."""
    if depth > 3:
        return "unknown"
    if bounded_python_assertion_validation(value):
        return "validation"
    if command_is_repository_inspection(value):
        return "inspection"
    argvs = command_argvs(value)
    if not argvs:
        return "unknown"
    raw = value.get("cmd") if isinstance(value, dict) else value
    if (all(len(argv) >= 2 and argv[0] == "git" and argv[1] in {"add", "commit"}
            for argv in argvs)
            and (len(argvs) == 1 or isinstance(raw, str)
                and not any(c in raw for c in (";", "|", "\n")))):
        # Campaign operator commit housekeeping is known execution, never a
        # repository validator or evidence of committed content by itself.
        return "repository_maintenance"
    if all(argv and Path(argv[0]).name in {"echo", "printf", "true"} for argv in argvs):
        return "report"
    if len(argvs) != 1:
        # Only && propagates every validation failure. Pipelines, ; and || can
        # hide it behind another command's status, even with numeric exit 0.
        raw = value.get("cmd") if isinstance(value, dict) else value
        if not isinstance(raw, str) or any(c in raw for c in (";", "|", "\n")):
            return "unknown"
        roles = [command_role(list(argv), depth + 1) for argv in argvs]
        return "validation" if "validation" in roles and all(r in {"validation", "inspection"} for r in roles) else "unknown"
    argv = argvs[0]
    # Literal shell assignment words only. Expansion is never evaluated.
    while argv and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=[^$`\n]*", argv[0]):
        argv = argv[1:]
    if not argv:
        return "unknown"
    program = Path(argv[0]).name
    args = list(argv[1:])
    if program in {"sh", "bash", "zsh"}:
        return command_role(args[1], depth + 1) if len(args) == 2 and args[0] in {"-c", "-lc"} else "unknown"
    if program == "env":
        while args and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=[^\n]*", args[0]):
            args.pop(0)
        return command_role(args, depth + 1) if args and not args[0].startswith("-") else "unknown"
    if str(argv[0]).endswith("rebuild/scripts/validate"):
        return command_role(args[3:], depth + 1) if len(args) > 3 and args[0] == "focused" and args[2] == "--" else "unknown"
    # Help/version output is reporting, even when it names a test command.
    if program in {"cargo", "python", "python3", "pytest", "cargo-clippy",
        "npm", "pnpm", "yarn", "go", "make", "cmake", "ctest", "mvn", "gradle", "gradlew", "ruff", "sphinx-build"} \
        and any(arg in {"--help", "--version", "-h"} for arg in args):
        return "report"
    if nonexecuting_validation_mode(program, args):
        return "report"
    if program == "cargo":
        if args and args[0].startswith("+"):
            args.pop(0)
        valid = bool(args) and (args[0] in {"test", "check", "clippy", "build"}
            or args[0] == "fmt" and "--check" in args)
    elif program in {"python", "python3"}:
        if args and args[0] == "-B":
            args.pop(0)
        if len(args) >= 2 and args[:2] == ["-m", "ruff"]:
            return command_role(["ruff", *args[2:]], depth + 1)
        valid = (len(args) >= 2 and args[0] == "-m" and args[1] in {"unittest", "pytest", "compileall", "sphinx"}
            or bool(args) and Path(args[0]).name.endswith("_self_test.py")
            or len(args) == 2 and Path(args[0]).name == "harness.py" and args[1] == "self-test")
    elif program in {"pytest", "cargo-clippy"}:
        valid = True
    elif program == "ruff":
        valid = bool(args) and (args[0] == "check" or args[0] == "format" and "--check" in args)
    elif program == "sphinx-build":
        valid = bool(args)
    elif program in {"npm", "pnpm", "yarn"}:
        valid = bool(args) and (args[0] in {"test", "check", "lint", "build"}
            or len(args) >= 2 and args[0] == "run" and args[1] in {"test", "check", "lint", "build"})
    elif program == "go":
        valid = bool(args) and args[0] in {"test", "vet", "build"}
    elif program in {"make", "cmake", "ctest", "mvn", "gradle", "gradlew"}:
        valid = (program == "ctest" or any(a in {"test", "check", "verify", "--build"} for a in args))
    else:
        valid = False
    return "validation" if valid else "unknown"


def bounded_repository_observation(command: CommandObservation, cwd: Path) -> bool:
    """Prove observation, never execution success, from a closed source-read form.

    Only one reader, optionally piped through bounded head/sed, and explicit
    in-repository paths qualify. No shell wrappers, arbitrary producers, compound
    commands, stdin/files-from indirection or external/runtime paths are inferred.
    """
    value = command.parsed_command
    raw = value.get("cmd") if isinstance(value, dict) else None
    if not isinstance(raw, str) or not command_is_repository_inspection(value):
        return False
    argvs = command_argvs(value)
    if not argvs or len(argvs) > 2 or any(c in raw for c in (";", "&", "\n")):
        return False
    if len(argvs) == 2:
        filter_argv = argvs[1]
        if not (len(filter_argv) == 2 and filter_argv[0] == "head" and re.fullmatch(r"-[1-9][0-9]{0,3}", filter_argv[1])
            or len(filter_argv) == 3 and filter_argv[:2] == ("sed", "-n")
            and re.fullmatch(r"[0-9]+,[0-9]+p", filter_argv[2])):
            return False
    argv = argvs[0]
    program = argv[0]
    if program in {"cat", "nl"}:
        paths = list(argv[2:] if program == "nl" else argv[1:])
    elif program == "sed" and len(argv) >= 4:
        paths = list(argv[3:])
    elif program == "rg" and len(argv) >= 4 and argv[1] == "-n":
        paths = list(argv[3:])
        # A small explicit glob option is supported; every remaining argument
        # must be an actual path scope, not another option or stdin sentinel.
        if "-g" in paths:
            i = paths.index("-g")
            if i + 1 >= len(paths):
                return False
            del paths[i:i + 2]
    else:
        return False
    workdir_value = value.get("workdir", str(cwd))
    if not isinstance(workdir_value, str):
        return False
    workdir = Path(workdir_value)
    if not workdir.is_absolute() or ".." in workdir.parts or not workdir.is_relative_to(cwd):
        return False
    def repository_path(path: str) -> Path | None:
        candidate = Path(path)
        if not path or path.startswith("-") or any(part in {"..", ".git", ".local", "target", "__pycache__"} for part in candidate.parts):
            return None
        candidate = candidate if candidate.is_absolute() else workdir / candidate
        return candidate if candidate.is_relative_to(cwd) else None
    scopes = [repository_path(path) for path in paths]
    if not scopes or any(path is None for path in scopes):
        return False
    lines = [line for line in command.output.splitlines() if line.strip()]
    if program == "rg":
        matched = []
        for line in lines:
            match = re.fullmatch(r"(.+?):([1-9][0-9]*):(.+)", line)
            if not match:
                continue
            path = repository_path(match[1])
            if path is not None and any(path == scope or path.is_relative_to(scope) for scope in scopes):
                matched.append(match[3])
        lines = matched
    elif any(re.match(r"(?:cat|sed|nl):|Warning:|.*No such file or directory", line) for line in lines):
        return False
    return len(lines) >= 2 and sum(len(line.strip()) for line in lines) >= 64


def repository_operation_is_inspection(call: ToolCall) -> bool:
    """Recognize a successful Repository Intelligence evidence acquisition."""

    if call.outcome != "succeeded":
        return False
    if call.operation == "repository_analyze":
        project_id = call.arguments.get("project_id")
        return bool(
            nonempty(project_id)
            and call.result.get("project_id") == project_id
            and nonempty(call.result.get("analysis_snapshot_id"))
            and nonempty(call.result.get("repository_snapshot_id"))
            and nonempty(call.result.get("repository_source_id"))
        )
    if call.operation == "repository_understanding":
        return bool(
            nonempty(call.arguments.get("project_id"))
            and nonempty(call.result.get("health"))
            and isinstance(call.result.get("overview"), dict)
            and isinstance(call.result.get("repository_map"), dict)
            and call.result.get("read_only") is True
        )
    return False


def load_codex_capture(path: Path) -> CodexCapture:
    if not path.is_file() or path.stat().st_size > MAX_CAPTURE_BYTES:
        raise EvidenceError("Codex capture is absent or exceeds the bounded size")
    return parse_codex_capture(path.read_bytes())


def capture_events(raw_bytes: bytes) -> list[dict[str, Any]]:
    """Shared bounded JSONL decoder for normalization and reviewer projection."""
    if len(raw_bytes) > MAX_CAPTURE_BYTES:
        raise EvidenceError("Codex capture exceeds the bounded size")
    try:
        lines = raw_bytes.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        raise EvidenceError("Codex capture is not UTF-8 JSONL") from error
    if not lines or len(lines) > MAX_CAPTURE_EVENTS:
        raise EvidenceError("Codex capture has no events or exceeds the event bound")

    events: list[dict[str, Any]] = []
    for line in lines:
        try:
            value = strict_json(line)
        except ValueError as error:
            raise EvidenceError("Codex capture contains invalid JSONL") from error
        if not isinstance(value, dict):
            raise EvidenceError("Codex capture event is not an object")
        if not nonempty(value.get("type")) or not isinstance(value.get("payload"), dict):
            raise EvidenceError("Codex capture event envelope is malformed")
        events.append(value)
    return events


def parse_codex_capture(raw_bytes: bytes) -> CodexCapture:
    """Normalize the exact supplied bytes, without re-reading a mutable path."""
    events = capture_events(raw_bytes)
    meta_events = [event for event in events if event.get("type") == "session_meta"]
    if len(meta_events) != 1 or events[0].get("type") != "session_meta":
        raise EvidenceError("Codex rollout requires one leading session_meta event")
    meta = meta_events[0].get("payload")
    if not isinstance(meta, dict):
        raise EvidenceError("Codex session_meta payload is missing")
    session_id = meta.get("session_id")
    if not nonempty(session_id) or session_id != meta.get("id"):
        raise EvidenceError("Codex session identity is missing or inconsistent")
    cwd_value = meta.get("cwd")
    if not nonempty(cwd_value) or not Path(cwd_value).is_absolute():
        raise EvidenceError("Codex session cwd is not an absolute path")
    cwd = Path(cwd_value)
    source = meta.get("source")
    originator = meta.get("originator")
    cli_version = meta.get("cli_version")
    thread_source = meta.get("thread_source")
    if (
        not nonempty(source)
        or not nonempty(originator)
        or not nonempty(cli_version)
        or not nonempty(thread_source)
    ):
        raise EvidenceError("Codex session metadata is missing or malformed")
    git = meta.get("git")
    if git is None:
        git = {}
    if not isinstance(git, dict):
        raise EvidenceError("Codex Git metadata is malformed")
    git_revision = git.get("commit_hash")
    if git_revision is not None and (not isinstance(git_revision, str)
            or re.fullmatch(r"[0-9a-f]{40}", git_revision) is None):
        raise EvidenceError("Codex Git revision metadata is malformed")

    turn_lifecycle = normalize_turn_lifecycle(events)
    # Recognize the maintained Codex rollout schema, not a filename or UI label.
    # Admission separately requires a fresh user thread, exact first turn and
    # candidate-owned activation; metadata alone never qualifies a capture.
    if not turn_lifecycle.turns:
        raise EvidenceError("unsupported capture format: Codex task lifecycle is absent")
    observed_metadata = {
        "capture_bounds": capture_bounds(events),
        "mcp_invocations": {},
        "session_meta": {key: value for key, value in meta.items() if key in {
            "id", "session_id", "cwd", "source", "originator", "cli_version",
            "client_version", "thread_source", "source_metadata", "forked_from_id",
            "model", "model_provider", "provider", "git", "timestamp"}},
        "turn_context": [{key: value for key, value in event["payload"].items()
                          if key in {"turn_id", "cwd", "model", "model_provider", "provider"}}
                         for event in events if event.get("type") == "turn_context"
                         and isinstance(event.get("payload"), dict)],
    }
    current_turn: str | None = None
    task_sequences: list[int] = []
    completed_task_sequences: list[int] = []
    compacted_sequences: list[int] = []
    known_turn_ids: set[str] = set()
    user_turn_evidence: list[_UserTurnEvidence] = []
    calls: dict[str, tuple[int, str, ParsedCustomCall]] = {}
    completions: dict[str, tuple[int, str, Any]] = {}
    mcp_wrappers: dict[str, tuple[int, str, ParsedMcpWrapper]] = {}
    mcp_completions: list[tuple[int, str, dict[str, Any]]] = []
    current_mcp_completions: list[tuple[int, str, dict[str, Any]]] = []
    evidence_transport_issues: list[EvidenceTransportIssue] = []
    raw_path_observations: list[_PathObservationEvidence] = []
    activation_states: list[tuple[int, str]] = []
    async_calls: dict[str, list[tuple[int, dict[str, Any]]]] = {}
    async_outputs: dict[str, list[tuple[int, dict[str, Any]]]] = {}
    shell_wrappers = []

    for sequence, event in enumerate(events):
        payload = event.get("payload")
        if not isinstance(payload, dict):
            continue
        envelope = event.get("type")
        payload_type = payload.get("type")
        if (
            envelope == "response_item"
            and payload_type == "message"
            and payload.get("role") == "developer"
        ):
            segments = message_text_segments(payload)
            if segments is not None:
                state = activation_evidence("\n".join(segments), cwd, str(session_id))
                if state != "absent":
                    activation_states.append((sequence, state))
        if envelope == "event_msg" and payload_type == "task_started":
            turn_id = payload.get("turn_id")
            if nonempty(turn_id):
                current_turn = str(turn_id)
                known_turn_ids.add(str(turn_id))
                task_sequences.append(sequence)
        elif envelope == "event_msg" and payload_type in {"task_complete", "task_completed"}:
            completed_task_sequences.append(sequence)
        elif envelope == "event_msg" and payload_type == "context_compacted":
            compacted_sequences.append(sequence)
        elif envelope == "event_msg" and payload_type == "user_message":
            message = payload.get("message")
            client_id = payload.get("client_id")
            if current_turn is not None and nonempty(message) and nonempty(client_id):
                user_turn_evidence.append(
                    _UserTurnEvidence(
                        sequence,
                        current_turn,
                        str(client_id),
                        str(message),
                        None,
                    )
                )
        elif envelope == "event_msg" and payload_type == "item_completed":
            current_evidence = current_user_turn_evidence(payload, sequence, str(session_id))
            if current_evidence is not None:
                user_turn_evidence.append(current_evidence)
            current_mcp = normalize_current_mcp_completion(payload, str(session_id))
            if current_mcp is not None:
                turn_id = payload.get("turn_id")
                if nonempty(turn_id):
                    current_mcp_completions.append((sequence, str(turn_id), payload))
                    call_id, operation, _arguments, _result, _outcome, error = current_mcp
                    if error in {
                        "malformed_mcp_completion",
                        "unsupported_mcp_completion_status",
                        "mcp_completion_status_mismatch",
                    }:
                        evidence_transport_issues.append(
                            EvidenceTransportIssue(
                                sequence,
                                str(turn_id),
                                call_id,
                                "volicord",
                                operation,
                                str(error),
                            )
                        )
            item = payload.get("item")
            if isinstance(item, dict) and item.get("type") == "FileChange":
                turn_id = payload.get("turn_id")
                call_id = item.get("id")
                if (
                    payload.get("thread_id") != session_id
                    or not nonempty(turn_id)
                    or not nonempty(call_id)
                ):
                    raise EvidenceError("Codex FileChange item identity is malformed")
                if item.get("status") == "completed":
                    normalized_changes = normalized_file_changes(item.get("changes"), cwd)
                    if (
                        normalized_changes is None
                        or not isinstance(item.get("stdout", ""), str)
                        or not isinstance(item.get("stderr", ""), str)
                    ):
                        evidence_transport_issues.append(
                            EvidenceTransportIssue(
                                sequence,
                                str(turn_id),
                                str(call_id),
                                "codex",
                                None,
                                "malformed_file_change",
                            )
                        )
                    else:
                        paths, changes = normalized_changes
                        raw_path_observations.append(
                            _PathObservationEvidence(
                                sequence,
                                str(turn_id),
                                str(call_id),
                                paths,
                                changes,
                                "event_msg.item_completed.FileChange",
                            )
                        )
                elif item.get("status") != "failed":
                    evidence_transport_issues.append(
                        EvidenceTransportIssue(
                            sequence,
                            str(turn_id),
                            str(call_id),
                            "codex",
                            None,
                            "malformed_file_change",
                        )
                    )
        elif envelope == "response_item" and payload_type == "function_call" and payload.get("name") == "request_user_input_async":
            if nonempty(payload.get("call_id")):
                async_calls.setdefault(payload["call_id"], []).append((sequence, payload))
        elif envelope == "response_item" and payload_type == "function_call_output":
            if nonempty(payload.get("call_id")):
                async_outputs.setdefault(payload["call_id"], []).append((sequence, payload))
        elif envelope == "response_item" and payload_type == "custom_tool_call":
            parsed = (
                parse_custom_call(payload.get("input"))
                if payload.get("name") == "exec" and payload.get("status") == "completed"
                else None
            )
            call_id = payload.get("call_id")
            metadata = payload.get("internal_chat_message_metadata_passthrough")
            turn_id = metadata.get("turn_id") if isinstance(metadata, dict) else None
            referenced = execution_tool_references(payload.get('input')) if payload.get('name') == 'exec' else ()
            if referenced:
                shell_wrappers.append((sequence, turn_id, call_id, referenced,
                    sha256_bytes(payload['input'].encode('utf-8')), parsed))
            if parsed is not None and safe_execution_id(call_id) and safe_execution_id(turn_id):
                if str(call_id) in calls:
                    raise EvidenceError("Codex capture reuses a supported custom call identity")
                calls[str(call_id)] = (sequence, str(turn_id), parsed)
            wrapper = (
                parse_mcp_wrapper(payload.get("input"))
                if payload.get("name") == "exec" and payload.get("status") == "completed"
                else None
            )
            if wrapper is not None and nonempty(call_id) and nonempty(turn_id):
                if str(call_id) in mcp_wrappers:
                    raise EvidenceError("Codex capture reuses an MCP wrapper identity")
                mcp_wrappers[str(call_id)] = (sequence, str(turn_id), wrapper)
        elif envelope == "response_item" and payload_type == "custom_tool_call_output":
            call_id = payload.get("call_id")
            metadata = payload.get("internal_chat_message_metadata_passthrough")
            turn_id = metadata.get("turn_id") if isinstance(metadata, dict) else None
            if nonempty(call_id) and nonempty(turn_id):
                if str(call_id) in completions:
                    raise EvidenceError("Codex capture reuses a custom call output identity")
                completions[str(call_id)] = (sequence, str(turn_id), payload.get("output"))
        elif envelope == "event_msg" and payload_type == "mcp_tool_call_end":
            if current_turn is not None:
                mcp_completions.append((sequence, current_turn, payload))
        elif envelope == "event_msg" and payload_type == "patch_apply_end":
            turn_id = payload.get("turn_id")
            call_id = payload.get("call_id")
            if (
                payload.get("success") is not True
                or payload.get("status") != "completed"
                or not nonempty(turn_id)
                or not nonempty(call_id)
                or str(turn_id) not in known_turn_ids
            ):
                continue
            normalized_changes = normalized_file_changes(payload.get("changes"), cwd)
            if normalized_changes is None:
                continue
            paths, changes = normalized_changes
            raw_path_observations.append(
                _PathObservationEvidence(
                    sequence,
                    str(turn_id),
                    str(call_id),
                    paths,
                    changes,
                    "event_msg.patch_apply_end",
                )
            )

    tool_call_evidence: list[_ToolCallEvidence] = []
    commands: list[CommandObservation] = []
    pending_commands: dict[int, dict[str, Any]] = {}
    wrapper_limits: dict[str, set[str]] = {}
    def wrapper_limit(call_id, reason):
        wrapper_limits.setdefault(call_id, set()).add(reason)
    executions = []
    for call_id, (sequence, turn_id, parsed) in sorted(
        calls.items(), key=lambda item: item[1][0]
    ):
        completion = completions.get(call_id)
        if parsed.tool_name == 'direct_sequence':
            parts = custom_output_parts(completion[2]) if completion else None
            header = CUSTOM_OUTPUT_HEADER.fullmatch(parts[0]) if parts else None
            correlated = bool(parts and len(parts) == len(parsed.arguments) + 1
                and header and not header['body'])
            for index, child in enumerate(parsed.arguments):
                child_completion = (completion[0], completion[1],
                    [{'type': 'input_text', 'text': part} for part in (parts[0], parts[index + 1])]) if correlated else None
                executions.append((call_id, sequence, turn_id, child, child_completion, index))
        else:
            executions.append((call_id, sequence, turn_id, parsed, completion, 0))
    for call_id, sequence, turn_id, parsed, completion, wrapper_index in executions:
        if completion is None or completion[0] <= sequence or completion[1] != turn_id:
            wrapper_limit(call_id, 'wrapper_completion_unresolvable')
            if parsed.tool_name != "exec_command":
                continue
            completion = (sequence, turn_id, None)
        completion_sequence, _, raw_output = completion
        if parsed.tool_name == "exec_command" and custom_cell_without_command_completion(raw_output):
            # Keep the observed static command. Cell lifecycle and command
            # completion are independent; no later unrelated exit can certify it.
            raw_output = None
        if parsed.tool_name == "write_stdin":
            result = custom_output_object(raw_output)
            session_id_value = (
                parsed.arguments.get("session_id")
                if isinstance(parsed.arguments, dict)
                else None
            )
            if (
                isinstance(session_id_value, bool)
                or not isinstance(session_id_value, int)
                or not isinstance(result, dict)
            ):
                wrapper_limit(call_id, 'continuation_result_unresolvable')
                continue
            pending = pending_commands.get(session_id_value)
            if pending is None:
                wrapper_limit(call_id, 'continuation_launch_unobserved')
                continue
            result_session_id = result.get("session_id")
            if result_session_id is not None and type(result_session_id) is not int:
                wrapper_limit(call_id, 'continuation_result_unresolvable')
                continue
            if result_session_id is not None and result_session_id != session_id_value:
                raise EvidenceError("Codex command continuation identity conflicts")
            output = result.get("output")
            exit_code = result.get("exit_code")
            if not isinstance(output, str) or (
                exit_code is not None
                and (
                    isinstance(exit_code, bool)
                    or not isinstance(exit_code, int)
                    or not -2_147_483_647 <= exit_code <= 2_147_483_647
                )
            ):
                wrapper_limit(call_id, 'continuation_result_unresolvable')
                continue
            pending["output"] += output
            pending["completion_sequence"] = completion_sequence
            pending['continuations'].append((call_id, turn_id, sequence, completion_sequence, wrapper_index))
            output_state = forwarded_output_state(result, parsed.arguments)
            pending['output_state'] = 'truncated' if 'truncated' in {pending['output_state'], output_state} else output_state
            if isinstance(exit_code, int):
                commands.append(
                    CommandObservation(
                        pending["sequence"],
                        completion_sequence,
                        pending["turn_id"],
                        pending['group_index'],
                        pending["arguments"],
                        exit_code,
                        'signaled' if exit_code < 0 else 'exited',
                        pending["output"],
                        not pending["output"].strip(),
                        pending['execution_identity'],
                        "completed",
                        pending['raw_call_id'],
                        tuple(pending['continuations']),
                        -exit_code if exit_code < 0 else None,
                        pending['output_state'],
                    )
                )
                del pending_commands[session_id_value]
            continue
        if parsed.tool_name == "exec_command":
            arguments = (
                list(parsed.arguments)
                if isinstance(parsed.arguments, tuple)
                else [parsed.arguments]
            )
            normalized_results: list[tuple[str, int | None]] | None = None
            execution_session_id: int | None = None
            malformed = False
            output_states = ['unknown'] * len(arguments)
            if parsed.output_mode in {"named_result", "labeled_results"}:
                values = []
                if parsed.output_mode == "named_result":
                    result = custom_output_object(raw_output)
                    values = [result.get(key) for key in parsed.result_keys] if isinstance(result, dict) else []
                else:
                    parts = custom_output_parts(raw_output)
                    if (parts and len(parts) == len(arguments) + 1
                        and CUSTOM_OUTPUT_HEADER.fullmatch(parts[0])
                        and not CUSTOM_OUTPUT_HEADER.fullmatch(parts[0]).group("body")):
                        for prefix, part in zip(parsed.result_keys, parts[1:], strict=True):
                            try:
                                values.append(strict_json(part[len(prefix):]) if part.startswith(prefix) else None)
                            except (ValueError, RecursionError):
                                values.append(None)
                if len(values) == len(arguments) and all(isinstance(v, dict) for v in values):
                    output_states = [forwarded_output_state(value, argument) for value, argument in zip(values, arguments)]
                    normalized_results = []
                    for value in values:
                        output, code = value.get("output"), value.get("exit_code")
                        if not isinstance(output, str) or (code is not None and
                            (type(code) is not int or not 0 <= code <= 2_147_483_647)):
                            malformed = True
                            normalized_results.append(("", None))
                        elif value.get("session_id") is not None:
                            normalized_results.append((output, None))
                        else:
                            normalized_results.append((output, code))
                else:
                    malformed = raw_output is not None
            elif parsed.output_mode == "indeterminate":
                pass
            elif parsed.output_mode.startswith("indexed_"):
                indexed = custom_indexed_command_results(
                    raw_output, parsed.output_mode, len(arguments)
                )
                normalized_results = indexed
            else:
                body = custom_output_body(raw_output)
                correlated = (
                    custom_correlated_command_result(
                        raw_output,
                        fields=parsed.result_keys,
                    )
                    if parsed.output_mode in {"correlated_split", "correlated_session"}
                    else custom_template_command_result(raw_output)
                    if parsed.output_mode == "template_exit"
                    else None
                )
                result = (
                    custom_output_object(raw_output)
                    if parsed.output_mode in {"result", "projection"}
                    else None
                )
                if parsed.output_mode == "result" and result is None and raw_output is not None:
                    malformed = True
                if parsed.output_mode == 'result':
                    output_states = [forwarded_output_state(result, arguments[0])]
                if parsed.output_mode == "projection":
                    if isinstance(result, dict) and set(result) <= set(parsed.result_keys):
                        result = {"output": "", **result}
                    else:
                        result = None
                        malformed = raw_output is not None
                raw_session_id = (
                    result.get("session_id") if isinstance(result, dict) else None
                )
                if (
                    correlated is not None
                    and parsed.output_mode
                    in {"correlated_split", "correlated_session"}
                ):
                    raw_session_id = correlated[2]
                if isinstance(raw_session_id, int) and not isinstance(
                    raw_session_id, bool
                ):
                    execution_session_id = raw_session_id
                output = (
                    correlated[0]
                    if correlated is not None
                    else result.get("output")
                    if isinstance(result, dict)
                    else body
                )
                exit_code = (
                    correlated[1]
                    if correlated is not None
                    else result.get("exit_code")
                    if isinstance(result, dict)
                    else None
                )
                if parsed.output_mode in {
                    "correlated_split",
                    "correlated_session",
                    "template_exit",
                } and correlated is None:
                    exit_code = None
                if (raw_session_id is not None and type(raw_session_id) is not int) or not isinstance(output, str) or (
                    exit_code is not None
                    and (
                        isinstance(exit_code, bool)
                        or not isinstance(exit_code, int)
                        or not (-2_147_483_647 if parsed.output_mode == 'result' else 0) <= exit_code <= 2_147_483_647
                    )
                ):
                    malformed = raw_output is not None
                    output, exit_code = "", None
                normalized_results = [(output, exit_code)]
            if normalized_results is None or len(normalized_results) != len(arguments):
                normalized_results = [("", None) for _ in arguments]
            if execution_session_id is not None:
                if len(arguments) != 1 or normalized_results[0][1] is not None:
                    raise EvidenceError("Codex command launch identity is malformed")
                if execution_session_id in pending_commands:
                    raise EvidenceError("Codex command process identity is reused")
                pending_commands[execution_session_id] = {
                    "sequence": sequence,
                    "completion_sequence": completion_sequence,
                    "turn_id": turn_id,
                    "arguments": arguments[0],
                    "output": normalized_results[0][0],
                    'execution_identity': f'custom_call:{call_id}:{wrapper_index}',
                    'raw_call_id': call_id,
                    'group_index': wrapper_index,
                    'continuations': [],
                    'output_state': output_states[0],
                }
                continue
            for group_index, (arguments_value, normalized) in enumerate(
                zip(arguments, normalized_results, strict=True)
            ):
                output, exit_code = normalized
                commands.append(
                    CommandObservation(
                        sequence,
                        completion_sequence,
                        turn_id,
                        group_index + wrapper_index,
                        arguments_value,
                        exit_code,
                        ('signaled' if exit_code < 0 else 'exited') if isinstance(exit_code, int) else None,
                        output,
                        not output.strip(),
                        f"custom_call:{call_id}:{group_index + wrapper_index}",
                        "completed" if isinstance(exit_code, int) else "indeterminate",
                        call_id,
                        (),
                        -exit_code if isinstance(exit_code, int) and exit_code < 0 else None,
                        output_states[group_index],
                    )
                )

            if any(code is None for _, code in normalized_results):
                evidence_transport_issues.append(EvidenceTransportIssue(
                    sequence, turn_id, call_id, "codex", None,
                    "malformed_exec_completion" if malformed else "command_completion_indeterminate",
                ))

    for session_id_value, pending in pending_commands.items():
        commands.append(
            CommandObservation(
                pending["sequence"],
                pending["completion_sequence"],
                pending["turn_id"],
                pending['group_index'],
                pending["arguments"],
                None,
                None,
                pending["output"],
                not pending["output"].strip(),
                pending['execution_identity'],
                "indeterminate",
                pending['raw_call_id'],
                tuple(pending['continuations']),
                None,
                pending['output_state'],
            )
        )

    seen_mcp_call_ids: set[str] = set()
    correlated_wrapper_ids: set[str] = set()
    for sequence, turn_id, payload in mcp_completions:
        completion_call_id = payload.get("call_id")
        if not nonempty(completion_call_id):
            continue
        completion_call_id = str(completion_call_id)
        if completion_call_id in seen_mcp_call_ids:
            raise EvidenceError("Codex capture reuses an MCP completion identity")
        seen_mcp_call_ids.add(completion_call_id)
        operation, arguments, result, outcome, error = normalize_mcp_completion(payload)
        if operation is None or arguments is None or outcome == "ignored":
            continue
        if error == "malformed_mcp_completion":
            evidence_transport_issues.append(
                EvidenceTransportIssue(
                    sequence,
                    turn_id,
                    completion_call_id,
                    "volicord",
                    operation,
                    error,
                )
            )
        correlated = [
            (wrapper_id, wrapper)
            for wrapper_id, wrapper in mcp_wrappers.items()
            if wrapper[1] == turn_id
            and wrapper[0] < sequence
            and wrapper_id not in correlated_wrapper_ids
            and (
                wrapper_id not in completions
                or sequence < completions[wrapper_id][0]
            )
        ]
        if len(correlated) > 1:
            outcome = "failed"
            error = "ambiguous_mcp_wrapper_correlation"
        elif len(correlated) == 1:
            wrapper_id, wrapper_data = correlated[0]
            correlated_wrapper_ids.add(wrapper_id)
            wrapper = wrapper_data[2]
            if wrapper.operation == operation and wrapper.arguments == arguments:
                observed_metadata['mcp_invocations'][completion_call_id] = wrapper_data[0]
            if wrapper.operation != operation or wrapper.arguments != arguments:
                outcome = "failed"
                error = "mcp_wrapper_completion_mismatch"
        tool_call_evidence.append(
            _ToolCallEvidence(
                sequence,
                sequence,
                turn_id,
                completion_call_id,
                "volicord",
                operation,
                arguments,
                result,
                outcome,
                error,
                "event_msg.mcp_tool_call_end",
            )
        )

    for sequence, turn_id, payload in current_mcp_completions:
        normalized = normalize_current_mcp_completion(payload, str(session_id))
        if normalized is None:
            continue
        call_id, operation, arguments, result, outcome, error = normalized
        if call_id is None or operation is None or arguments is None:
            continue
        candidates = [wrapper for wrapper_id, wrapper in mcp_wrappers.items()
            if wrapper[1] == turn_id and wrapper[0] < sequence
            and (wrapper_id not in completions or sequence < completions[wrapper_id][0])
            and wrapper[2].operation == operation and wrapper[2].arguments == arguments]
        if len(candidates) == 1:
            observed_metadata['mcp_invocations'][call_id] = candidates[0][0]
        tool_call_evidence.append(
            _ToolCallEvidence(
                sequence,
                sequence,
                turn_id,
                call_id,
                "volicord",
                operation,
                arguments,
                result,
                outcome,
                error,
                "event_msg.item_completed.McpToolCall",
            )
        )

    if any(value.turn_id not in known_turn_ids for value in tool_call_evidence):
        raise EvidenceError("Codex MCP completion refers to an unknown turn identity")
    if any(value.turn_id not in known_turn_ids for value in evidence_transport_issues):
        raise EvidenceError("Codex MCP transport issue refers to an unknown turn identity")
    tool_calls = tuple(
        call if turn_lifecycle.contains_completion(call.turn_id, call.sequence, call.completion_sequence)
        else replace(call, outcome="failed", error="completion_outside_turn")
        for call in merge_tool_call_evidence(tool_call_evidence)
    )
    commands = [
        command if turn_lifecycle.contains_completion(command.turn_id, command.sequence, command.completion_sequence)
        else replace(command, exit_code=None, signal_number=None, termination=None, evidence_state="indeterminate")
        for command in commands
    ]
    commands.sort(key=lambda value: (value.sequence, value.group_index))
    execution_wrappers = []
    safe_id = safe_execution_id
    for sequence, turn_id, call_id, tools_referenced, wrapper_hash, parsed in shell_wrappers:
        completion = completions.get(call_id)
        related = [command for command in commands if command.raw_call_id == call_id
            or any(coordinate[0] == call_id for coordinate in command.continuation_coordinates)]
        reasons = set(wrapper_limits.get(call_id, ()))
        if parsed is None:
            state, count = 'unsupported', None
            reasons.add('unsupported_wrapper_grammar')
        else:
            count = len(parsed.arguments) if parsed.tool_name == 'direct_sequence' or isinstance(parsed.arguments, tuple) else 1
            if any(command.evidence_state != 'completed' for command in related):
                reasons.add('command_completion_indeterminate')
            state = 'indeterminate' if reasons or not related else 'normalized'
            if not related and not reasons:
                reasons.add('command_completion_indeterminate')
        if safe_id(call_id) is None or safe_id(turn_id) is None:
            state = 'unsupported'
            reasons.add('wrapper_identity_unresolvable')
        correlated_completion = (completion[0] if completion and completion[0] > sequence
            and completion[1] == turn_id else None)
        if correlated_completion is None:
            reasons.add('wrapper_completion_unresolvable')
            if state == 'normalized':
                state = 'indeterminate'
        execution_wrappers.append(ExecutionWrapperObservation(sequence, correlated_completion,
            safe_id(turn_id), safe_id(call_id), wrapper_hash, tools_referenced, count,
            state, tuple(sorted(reasons))))
    if any(value.turn_id not in known_turn_ids for value in raw_path_observations):
        raise EvidenceError("Codex file change refers to an unknown turn identity")
    path_observations = merge_path_observation_evidence(raw_path_observations)

    async_question_requests = []
    for call_id, requests in async_calls.items():
        outputs = async_outputs.get(call_id, [])
        if len(requests) != 1 or len(outputs) != 1:
            continue
        sequence, request = requests[0]
        completion_sequence, output = outputs[0]
        try:
            arguments = strict_json(request.get("arguments"))
            accepted = strict_json(output.get("output"))
            metadata = request.get("internal_chat_message_metadata_passthrough", {})
            output_metadata = output.get("internal_chat_message_metadata_passthrough", {})
            turn_id = metadata.get("turn_id")
            questions = arguments.get("questions")
            if (turn_id not in known_turn_ids or output_metadata.get("turn_id") != turn_id
                or not isinstance(accepted, dict) or set(accepted) != {"accepted"}
                or accepted["accepted"] is not True or set(arguments) != {"questions"}
                or not turn_lifecycle.contains_completion(turn_id, sequence, completion_sequence)
                or not isinstance(questions, list) or not 0 < len(questions) <= MAX_PATHS
                or not all(isinstance(q, dict) and set(q) <= {"title", "options"}
                    and nonempty(q.get("title")) and ("options" not in q
                        or isinstance(q["options"], list) and 0 < len(q["options"]) <= MAX_PATHS
                        and all(nonempty(option) for option in q["options"])) for q in questions)):
                continue
            async_question_requests.append(AsyncQuestionRequest(str(session_id), turn_id,
                call_id, sequence, completion_sequence, tuple(q["title"] for q in questions)))
        except (ValueError, TypeError, AttributeError, RecursionError):
            continue

    user_turns = normalize_user_turn_evidence(user_turn_evidence, known_turn_ids)
    fresh_user_thread = (
        thread_source == "user"
        and meta.get("forked_from_id") in {None, ""}
    )
    activation_state = session_activation_state(
        events,
        activation_states,
        user_turns,
        {call.sequence for call in tool_calls}
        | {command.sequence for command in commands}
        | {observation.sequence for observation in path_observations}
        | {sequence for sequence, _, _ in calls.values()}
        | {sequence for sequence, _, _ in mcp_wrappers.values()},
    )
    repository_scoped_activation_observed = activation_state == "valid"
    return CodexCapture(
        source_sha256=sha256_bytes(raw_bytes),
        session_id=str(session_id),
        cwd=cwd,
        git_revision=git_revision,
        source=str(source),
        originator=str(originator),
        cli_version=str(cli_version),
        thread_source=str(thread_source),
        capture_format="codex_rollout_jsonl",
        observed_metadata=observed_metadata,
        fresh_user_thread=fresh_user_thread,
        repository_scoped_activation_observed=repository_scoped_activation_observed,
        activation_evidence_state=activation_state,
        turn_lifecycle=turn_lifecycle,
        task_sequences=tuple(task_sequences),
        completed_task_sequences=tuple(completed_task_sequences),
        compacted_sequences=tuple(compacted_sequences),
        user_turns=user_turns,
        tool_calls=tool_calls,
        evidence_transport_state=(
            "indeterminate" if evidence_transport_issues else "complete"
        ),
        evidence_transport_issues=tuple(
            sorted(evidence_transport_issues, key=lambda value: value.sequence)
        ),
        path_observations=tuple(path_observations),
        commands=tuple(commands),
        async_question_requests=tuple(async_question_requests),
        execution_wrappers=tuple(execution_wrappers),
    )


@dataclass(frozen=True)
class CanonicalBundle:
    source_sha256: str
    project_id: str
    tables: dict[str, tuple[dict[str, Any], ...]]

    def rows(self, name: str) -> tuple[dict[str, Any], ...]:
        return self.tables.get(name, ())

    def one(self, name: str, **expected: Any) -> dict[str, Any] | None:
        matches = [
            row
            for row in self.rows(name)
            if all(row.get(key) == value for key, value in expected.items())
        ]
        return matches[0] if len(matches) == 1 else None


def portable_value(value: Any) -> Any:
    if not isinstance(value, dict) or value.get("type") not in {"null", "integer", "text", "bytes"}:
        raise EvidenceError("canonical bundle contains an invalid portable value")
    if value["type"] == "null":
        return None
    decoded = value.get("value")
    if value["type"] == "integer" and not isinstance(decoded, int):
        raise EvidenceError("canonical bundle integer is invalid")
    if value["type"] in {"text", "bytes"} and not isinstance(decoded, str):
        raise EvidenceError("canonical bundle string value is invalid")
    if value["type"] == "bytes":
        try:
            bytes.fromhex(decoded)
        except ValueError as error:
            raise EvidenceError("canonical bundle byte value is not hexadecimal") from error
    return decoded


def load_canonical_bundle(path: Path) -> CanonicalBundle:
    if not path.is_file() or path.stat().st_size > MAX_CAPTURE_BYTES:
        raise EvidenceError("canonical bundle is absent or exceeds the bounded size")
    raw_bytes = path.read_bytes()
    try:
        envelope = json.loads(raw_bytes)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise EvidenceError("canonical bundle is not JSON") from error
    if not isinstance(envelope, dict) or envelope.get("kind") != "volicord-context-bundle":
        raise EvidenceError("canonical evidence is not a Volicord context bundle")
    if envelope.get("format_version") != 9:
        raise EvidenceError("canonical bundle format is not the supported current version")
    payload = envelope.get("payload")
    if not isinstance(payload, dict) or sha256_bytes(canonical_json(payload)) != envelope.get("checksum"):
        raise EvidenceError("canonical bundle payload checksum is invalid")
    project_id = payload.get("project_id")
    raw_tables = payload.get("tables")
    if not nonempty(project_id) or not isinstance(raw_tables, list):
        raise EvidenceError("canonical bundle payload is incomplete")
    lineage = payload.get("lineage")
    semantic_state = {"project_id": project_id, "tables": raw_tables}
    if (
        not isinstance(lineage, dict)
        or sha256_bytes(canonical_json(semantic_state)) != lineage.get("history_basis")
        or not isinstance(lineage.get("common_base_basis"), str)
        or len(lineage["common_base_basis"]) != 64
        or any(character not in "0123456789abcdefABCDEF" for character in lineage["common_base_basis"])
    ):
        raise EvidenceError("canonical bundle lineage does not match its semantic state")

    tables: dict[str, tuple[dict[str, Any], ...]] = {}
    for table in raw_tables:
        if not isinstance(table, dict):
            raise EvidenceError("canonical bundle table is invalid")
        name = table.get("name")
        columns = table.get("columns")
        rows = table.get("rows")
        if not nonempty(name) or name in tables or not isinstance(columns, list) or not isinstance(rows, list):
            raise EvidenceError("canonical bundle table shape is invalid")
        if len(columns) != len(set(columns)) or not all(nonempty(column) for column in columns):
            raise EvidenceError("canonical bundle table columns are invalid")
        decoded_rows: list[dict[str, Any]] = []
        for row in rows:
            if not isinstance(row, list) or len(row) != len(columns):
                raise EvidenceError("canonical bundle row shape is invalid")
            decoded_rows.append(dict(zip(columns, (portable_value(value) for value in row), strict=True)))
        tables[str(name)] = tuple(decoded_rows)
    return CanonicalBundle(sha256_bytes(raw_bytes), str(project_id), tables)


def decode_string_blob(value: Any) -> list[str] | None:
    if not isinstance(value, str):
        return None
    try:
        raw = bytes.fromhex(value)
    except ValueError:
        return None
    if len(raw) < 8:
        return None
    count = int.from_bytes(raw[:8], "big")
    if count > MAX_PATHS:
        return None
    offset = 8
    result: list[str] = []
    for _ in range(count):
        if offset + 8 > len(raw):
            return None
        length = int.from_bytes(raw[offset : offset + 8], "big")
        offset += 8
        end = offset + length
        if end > len(raw):
            return None
        try:
            result.append(raw[offset:end].decode("utf-8"))
        except UnicodeDecodeError:
            return None
        offset = end
    return result if offset == len(raw) else None


def decoded_blob(value: Any) -> bytes | None:
    if not isinstance(value, str):
        return None
    try:
        return bytes.fromhex(value)
    except ValueError:
        return None


def framed_bytes(raw: bytes, offset: int) -> tuple[bytes, int] | None:
    if offset + 8 > len(raw):
        return None
    length = int.from_bytes(raw[offset : offset + 8], "big")
    start = offset + 8
    end = start + length
    return (raw[start:end], end) if end <= len(raw) else None


def decode_question_alternatives(value: Any) -> list[dict[str, str]] | None:
    """Decode the current portable Question alternative blob for bounded review."""
    raw = decoded_blob(value)
    if raw is None or len(raw) < 8:
        return None
    count = int.from_bytes(raw[:8], "big")
    if count > MAX_PATHS:
        return None
    offset = 8
    result: list[dict[str, str]] = []
    for _ in range(count):
        values: list[str] = []
        for _field in range(3):
            framed = framed_bytes(raw, offset)
            if framed is None:
                return None
            field, offset = framed
            try:
                values.append(field.decode("utf-8"))
            except UnicodeDecodeError:
                return None
        result.append(dict(zip(("key", "label", "consequence"), values, strict=True)))
    return result if offset == len(raw) else None


def decode_established_fact_statements(value: Any) -> list[str] | None:
    """Decode statements while validating the complete current portable fact blob."""
    raw = decoded_blob(value)
    if raw is None or len(raw) < 8:
        return None
    count = int.from_bytes(raw[:8], "big")
    if count > MAX_PATHS:
        return None
    offset = 8
    result: list[str] = []
    for _ in range(count):
        statement = framed_bytes(raw, offset)
        if statement is None:
            return None
        statement_bytes, offset = statement
        source_ids = framed_bytes(raw, offset)
        if source_ids is None:
            return None
        _, offset = source_ids
        if offset >= len(raw) or raw[offset] not in {0, 1}:
            return None
        has_capability = raw[offset] == 1
        offset += 1
        if has_capability:
            capability = framed_bytes(raw, offset)
            if capability is None:
                return None
            _, offset = capability
        freshness = framed_bytes(raw, offset)
        if freshness is None:
            return None
        _, offset = freshness
        try:
            result.append(statement_bytes.decode("utf-8"))
        except UnicodeDecodeError:
            return None
    return result if offset == len(raw) else None


def relevant_context_ids(bundle: CanonicalBundle, recall_result: dict[str, Any]) -> list[str] | None:
    goals = recall_result.get("goal_basis")
    if recall_result.get("project_id") != bundle.project_id or not isinstance(goals, list) or not goals:
        return None
    identities: list[str] = []
    for goal in goals:
        if not isinstance(goal, dict) or goal.get("role") != "goal":
            return None
        identity = goal.get("identity")
        item = bundle.one("context_items", id=identity, project_id=bundle.project_id)
        sources = [row.get("source_id") for row in sorted(bundle.rows("context_item_sources"),
            key=lambda row: row.get("position", -1)) if row.get("project_id") == bundle.project_id
            and row.get("context_item_id") == identity]
        if (not nonempty(identity) or item is None or item.get("role") != "goal"
                or goal.get("statement") != item.get("statement")
                or goal.get("source_ids") != sources or not sources):
            return None
        identities.append(identity)
    behavioral = recall_result.get("behaviorally_relevant_context")
    if not isinstance(behavioral, list):
        return None
    for projected in behavioral:
        if not isinstance(projected, dict):
            return None
        identity = projected.get("identity")
        role = projected.get("role")
        statement = projected.get("statement")
        source_ids = projected.get("source_ids")
        item = bundle.one(
            "context_items",
            id=identity,
            project_id=bundle.project_id,
        )
        canonical_source_ids = [
            row.get("source_id")
            for row in sorted(
                bundle.rows("context_item_sources"),
                key=lambda row: row.get("position")
                if isinstance(row.get("position"), int)
                else -1,
            )
            if row.get("project_id") == bundle.project_id
            and row.get("context_item_id") == identity
        ]
        if (
            item is None
            or role not in {"constraint", "preference", "learning"}
            or item.get("role") != role
            or item.get("statement") != statement
            or not isinstance(source_ids, list)
            or not source_ids
            or not all(nonempty(source_id) for source_id in source_ids)
            or source_ids != canonical_source_ids
        ):
            return None
        identities.append(str(identity))
    if len(identities) != len(set(identities)):
        return None
    return sorted(set(identities))


def recalled_checkpoint(bundle: CanonicalBundle, recall_result: dict[str, Any]) -> dict[str, Any] | None:
    checkpoint = recall_result.get("checkpoint")
    if not isinstance(checkpoint, dict) or not nonempty(checkpoint.get("identity")):
        return None
    return bundle.one(
        "checkpoints",
        project_id=bundle.project_id,
        id=checkpoint["identity"],
    )


def recalled_decision_ids(recall_result: dict[str, Any]) -> list[str] | None:
    decisions = recall_result.get("decisions")
    if not isinstance(decisions, list):
        return None
    identities = [item.get("identity") for item in decisions if isinstance(item, dict)]
    if len(identities) != len(decisions) or not all(nonempty(value) for value in identities):
        return None
    return sorted(set(str(value) for value in identities))
