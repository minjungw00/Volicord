#!/usr/bin/env python3
"""Normalize bounded Phase 8 facts from Codex rollout JSONL and canonical bundles.

Raw rollout and bundle content is local evidence.  This module intentionally keeps
prompt text and tool payloads only long enough to correlate an observed user turn
with the product's canonical Source; callers receive identities, ordering, paths,
and booleans rather than source bodies or arbitrary tool output.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re
import shlex
from typing import Any


MAX_CAPTURE_BYTES = 64 * 1024 * 1024
MAX_CAPTURE_EVENTS = 200_000
MAX_PATHS = 256
MAX_USER_MESSAGE_CONTENT_ITEMS = 256
MAX_USER_TURN_TEXT_CHARS = 1 << 20
MAX_MCP_CONTENT_RESULT_CHARS = 2 << 20
MAX_FILE_CHANGE_BODY_CHARS = 8 << 20
ACTIVATION_CONTEXT_MARKERS = (
    "Volicord is active for this explicitly authorized repository.",
    "Start project-scoped repository work with project_resolve",
    "workflow.required_next_action",
)
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
                path
                for path, _change_type, _body_sha256, _move_path in changes
                if not generated_repository_path(path)
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


@dataclass(frozen=True)
class ParsedCustomCall:
    tool_name: str
    arguments: Any
    output_mode: str


@dataclass(frozen=True)
class ParsedMcpWrapper:
    operation: str
    arguments: dict[str, Any]


class JsLiteralParser:
    """Parse only JSON-like JavaScript literals used by current dogfood calls."""

    def __init__(self, source: str):
        self.source = source
        self.offset = 0

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
    r"\A\s*const\s+(?P<variable>[A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*await\s+"
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


def parse_static_exec_command_list(value: str) -> tuple[dict[str, Any], ...] | None:
    """Parse a bounded comma-separated list of literal exec_command calls."""
    offset = 0
    result: list[dict[str, Any]] = []
    call_prefix = re.compile(r"tools\.exec_command\s*\(")
    while True:
        while offset < len(value) and value[offset] in " \t\r\n,":
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
            arguments = JsLiteralParser(value[argument_start:cursor]).parse()
        except EvidenceError:
            return None
        if not isinstance(arguments, dict):
            return None
        result.append(arguments)
        offset = cursor + 1
        while offset < len(value) and value[offset] in " \t\r\n":
            offset += 1
        if offset < len(value) and value[offset] != ",":
            return None


def indexed_promise_output_mode(forward: str, variable: str) -> str | None:
    identifier = r"[A-Za-z_$][A-Za-z0-9_$]*"
    escaped_variable = re.escape(variable)
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
        arguments = parse_static_exec_command_list(promise_match.group("calls"))
        mode = indexed_promise_output_mode(
            promise_match.group("forward").strip(), promise_match.group("variable")
        )
        if arguments is None or mode is None:
            return None
        return ParsedCustomCall("exec_command", arguments, mode)

    match = ASSIGNED_CALL.fullmatch(value)
    if match is None:
        return None
    variable = re.escape(match.group("variable"))
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
        rf"text\s*\(\s*{variable}\.output\s*\)\s*;\s*"
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
        ):
            correlated_fields = set(parsed_fields)
    template_exit_forward = re.fullmatch(
        rf"text\s*\(\s*{variable}\.output\s*\)\s*;\s*"
        rf"text\s*\(\s*`exit=\$\{{{variable}\.exit_code\}}`\s*\)\s*;",
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
        arguments = JsLiteralParser(match.group("argument")).parse()
    except EvidenceError:
        return None
    if not isinstance(arguments, dict):
        return None
    mode = (
        "result"
        if result_forward is not None
        else "correlated_split"
        if correlated_fields is not None and "session_id" not in correlated_fields
        else "correlated_session"
        if correlated_fields is not None
        else "template_exit"
        if template_exit_forward is not None
        else "output"
    )
    return ParsedCustomCall(tool_name, arguments, mode)


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
    if not isinstance(ok, dict) or not isinstance(ok.get("isError"), bool):
        return operation, arguments, {}, "failed", "malformed_mcp_completion"
    structured = ok.get("structuredContent")
    if not isinstance(structured, dict):
        return operation, arguments, {}, "failed", "malformed_mcp_completion"
    if ok["isError"]:
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
    structured = result.get("structuredContent")
    if structured is not None and not isinstance(structured, dict):
        return str(call_id), operation, arguments, {}, "failed", "malformed_mcp_completion"
    content_state, content_structured = current_mcp_content_result(result, is_error)
    if content_state == "malformed":
        return str(call_id), operation, arguments, {}, "failed", "malformed_mcp_completion"
    if isinstance(structured, dict) and isinstance(content_structured, dict):
        if structured != content_structured:
            raise EvidenceError("Codex MCP structured result representations conflict")
    elif structured is None:
        structured = content_structured
    if not isinstance(structured, dict):
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


def current_mcp_content_result(
    result: dict[str, Any], is_error: bool
) -> tuple[str, dict[str, Any] | None]:
    """Read only the current serialized CallToolResult content envelope.

    Direct tool text is not searched for JSON. The supported fallback is one text
    block whose complete text is a CallToolResult object, containing one text
    block whose complete text is the structured product result.
    """
    content = result.get("content")
    if not isinstance(content, list):
        return "malformed", None
    if not content:
        return "absent", None
    if (
        len(content) != 1
        or not isinstance(content[0], dict)
        or set(content[0]) != {"type", "text"}
        or content[0].get("type") != "text"
        or not isinstance(content[0].get("text"), str)
        or len(content[0]["text"]) > MAX_MCP_CONTENT_RESULT_CHARS
    ):
        return "malformed" if result.get("structuredContent") is None else "absent", None
    try:
        envelope = json.loads(content[0]["text"])
    except json.JSONDecodeError:
        return "malformed" if result.get("structuredContent") is None else "absent", None
    if not isinstance(envelope, dict) or set(envelope) not in (
        {"content", "isError"},
        {"content", "structuredContent", "isError"},
    ):
        return "malformed" if result.get("structuredContent") is None else "absent", None
    envelope_content = envelope.get("content")
    if (
        envelope.get("isError") is not is_error
        or not isinstance(envelope_content, list)
        or len(envelope_content) != 1
        or not isinstance(envelope_content[0], dict)
        or set(envelope_content[0]) != {"type", "text"}
        or envelope_content[0].get("type") != "text"
        or not isinstance(envelope_content[0].get("text"), str)
        or len(envelope_content[0]["text"]) > MAX_MCP_CONTENT_RESULT_CHARS
    ):
        return "malformed", None
    try:
        text_structured = json.loads(envelope_content[0]["text"])
    except json.JSONDecodeError:
        return "malformed", None
    if not isinstance(text_structured, dict):
        return "malformed", None
    envelope_structured = envelope.get("structuredContent")
    if envelope_structured is not None:
        if not isinstance(envelope_structured, dict):
            return "malformed", None
        if envelope_structured != text_structured:
            raise EvidenceError("Codex MCP content result representations conflict")
    return "valid", text_structured


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
    value: Any, *, includes_session_id: bool = False
) -> tuple[str, int, int | None] | None:
    parts = custom_output_parts(value)
    if parts is None or len(parts) != 3:
        return None
    header = CUSTOM_OUTPUT_HEADER.fullmatch(parts[0])
    if header is None or header.group("body"):
        return None
    try:
        status = json.loads(parts[2])
    except json.JSONDecodeError:
        return None
    required_keys = {"exit_code", "session_id"} if includes_session_id else {"exit_code"}
    if (
        not isinstance(status, dict)
        or len(status) > 16
        or not required_keys <= set(status)
    ):
        return None
    if "session_id" in status and status["session_id"] is not None and (
        isinstance(status["session_id"], bool)
        or not isinstance(status["session_id"], int)
    ):
        return None
    exit_code = status["exit_code"]
    if isinstance(exit_code, bool) or not isinstance(exit_code, int) or not 0 <= exit_code <= 2_147_483_647:
        return None
    session_id = status.get("session_id")
    return parts[1], exit_code, session_id


def custom_template_command_result(value: Any) -> tuple[str, int] | None:
    parts = custom_output_parts(value)
    if parts is None or len(parts) != 3:
        return None
    header = CUSTOM_OUTPUT_HEADER.fullmatch(parts[0])
    status = re.fullmatch(r"exit=([0-9]+)", parts[2])
    if header is None or header.group("body") or status is None:
        return None
    exit_code = int(status.group(1))
    return (parts[1], exit_code) if exit_code <= 2_147_483_647 else None


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
    expected_base = 0 if mode == "indexed_suffix_zero" else 1
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
        parsed = json.loads(body)
    except json.JSONDecodeError:
        return None
    return parsed if isinstance(parsed, dict) else None


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
    fresh_user_thread: bool
    repository_scoped_activation_observed: bool
    task_sequences: tuple[int, ...]
    completed_task_sequences: tuple[int, ...]
    compacted_sequences: tuple[int, ...]
    user_turns: tuple[UserTurn, ...]
    tool_calls: tuple[ToolCall, ...]
    evidence_transport_state: str
    evidence_transport_issues: tuple[EvidenceTransportIssue, ...]
    path_observations: tuple[PathObservation, ...]
    commands: tuple[CommandObservation, ...]

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
        matches = [turn for turn in self.user_turns if turn.turn_id == call.turn_id]
        return matches[0] if len(matches) == 1 else None

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
        lexer = shlex.shlex(value, posix=True, punctuation_chars=";&|<>")
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
        if token in {"&&", "||", ";", "|"}:
            if not current:
                return []
            segments.append(tuple(current))
            current = []
        else:
            current.append(token)
    if not current:
        return []
    segments.append(tuple(current))
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


def command_is_repository_inspection(value: Any) -> bool:
    inspection_programs = {
        "cat",
        "fd",
        "find",
        "grep",
        "head",
        "ls",
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
    argvs = command_argvs(value)
    return bool(argvs) and all(
        bool(argv)
        and (
            Path(argv[0]).name.lower() in inspection_programs
            or (Path(argv[0]).name.lower() == "cd" and len(argv) == 2)
            or (
                Path(argv[0]).name.lower() == "git"
                and len(argv) >= 2
                and argv[1].lower() in git_inspections
            )
        )
        for argv in argvs
    )


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
    raw_bytes = path.read_bytes()
    try:
        lines = raw_bytes.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        raise EvidenceError("Codex capture is not UTF-8 JSONL") from error
    if not lines or len(lines) > MAX_CAPTURE_EVENTS:
        raise EvidenceError("Codex capture has no events or exceeds the event bound")

    events: list[dict[str, Any]] = []
    for line in lines:
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise EvidenceError("Codex capture contains invalid JSONL") from error
        if not isinstance(value, dict):
            raise EvidenceError("Codex capture event is not an object")
        events.append(value)

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
        source != "vscode"
        or originator != "codex_vscode"
        or not nonempty(cli_version)
        or not nonempty(thread_source)
    ):
        raise EvidenceError("Codex session source is unsupported")
    git = meta.get("git") if isinstance(meta.get("git"), dict) else {}
    git_revision = git.get("commit_hash") if nonempty(git.get("commit_hash")) else None

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
    repository_scoped_activation_observed = False

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
            content = payload.get("content")
            if isinstance(content, list) and len(content) <= 32:
                developer_text = "\n".join(
                    item.get("text", "")
                    for item in content
                    if isinstance(item, dict)
                    and item.get("type") in {"input_text", "output_text"}
                    and isinstance(item.get("text"), str)
                )
                if all(marker in developer_text for marker in ACTIVATION_CONTEXT_MARKERS):
                    repository_scoped_activation_observed = True
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
                        or not isinstance(item.get("stdout"), str)
                        or not isinstance(item.get("stderr"), str)
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
        elif envelope == "response_item" and payload_type == "custom_tool_call":
            parsed = (
                parse_custom_call(payload.get("input"))
                if payload.get("name") == "exec" and payload.get("status") == "completed"
                else None
            )
            call_id = payload.get("call_id")
            metadata = payload.get("internal_chat_message_metadata_passthrough")
            turn_id = metadata.get("turn_id") if isinstance(metadata, dict) else None
            if parsed is not None and nonempty(call_id) and nonempty(turn_id):
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
    for call_id, (sequence, turn_id, parsed) in sorted(
        calls.items(), key=lambda item: item[1][0]
    ):
        completion = completions.get(call_id)
        if completion is None or completion[0] <= sequence or completion[1] != turn_id:
            continue
        completion_sequence, _, raw_output = completion
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
                continue
            pending = pending_commands.get(session_id_value)
            if pending is None:
                continue
            result_session_id = result.get("session_id")
            if result_session_id is not None and result_session_id != session_id_value:
                raise EvidenceError("Codex command continuation identity conflicts")
            output = result.get("output")
            exit_code = result.get("exit_code")
            if not isinstance(output, str) or (
                exit_code is not None
                and (
                    isinstance(exit_code, bool)
                    or not isinstance(exit_code, int)
                    or not 0 <= exit_code <= 2_147_483_647
                )
            ):
                continue
            pending["output"] += output
            pending["completion_sequence"] = completion_sequence
            if isinstance(exit_code, int):
                commands.append(
                    CommandObservation(
                        pending["sequence"],
                        completion_sequence,
                        pending["turn_id"],
                        0,
                        pending["arguments"],
                        exit_code,
                        "exited",
                        pending["output"],
                        not pending["output"].strip(),
                        f"process_session:{session_id_value}",
                        "completed",
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
            if parsed.output_mode.startswith("indexed_"):
                indexed = custom_indexed_command_results(
                    raw_output, parsed.output_mode, len(arguments)
                )
                normalized_results = indexed
            else:
                body = custom_output_body(raw_output)
                if body is None:
                    continue
                correlated = (
                    custom_correlated_command_result(
                        raw_output,
                        includes_session_id=parsed.output_mode == "correlated_session",
                    )
                    if parsed.output_mode in {"correlated_split", "correlated_session"}
                    else custom_template_command_result(raw_output)
                    if parsed.output_mode == "template_exit"
                    else None
                )
                result = (
                    custom_output_object(raw_output)
                    if parsed.output_mode == "result"
                    else None
                )
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
                    continue
                if not isinstance(output, str) or (
                    exit_code is not None
                    and (
                        isinstance(exit_code, bool)
                        or not isinstance(exit_code, int)
                        or not 0 <= exit_code <= 2_147_483_647
                    )
                ):
                    continue
                normalized_results = [(output, exit_code)]
            if normalized_results is None or len(normalized_results) != len(arguments):
                continue
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
                        group_index,
                        arguments_value,
                        exit_code,
                        "exited" if isinstance(exit_code, int) else None,
                        output,
                        not output.strip(),
                        f"custom_call:{call_id}:{group_index}",
                        "completed" if isinstance(exit_code, int) else "indeterminate",
                    )
                )

    for session_id_value, pending in pending_commands.items():
        commands.append(
            CommandObservation(
                pending["sequence"],
                pending["completion_sequence"],
                pending["turn_id"],
                0,
                pending["arguments"],
                None,
                None,
                pending["output"],
                not pending["output"].strip(),
                f"process_session:{session_id_value}",
                "indeterminate",
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
    tool_calls = merge_tool_call_evidence(tool_call_evidence)
    commands.sort(key=lambda value: (value.sequence, value.group_index))
    if any(value.turn_id not in known_turn_ids for value in raw_path_observations):
        raise EvidenceError("Codex file change refers to an unknown turn identity")
    path_observations = merge_path_observation_evidence(raw_path_observations)

    user_turns = normalize_user_turn_evidence(user_turn_evidence, known_turn_ids)
    fresh_user_thread = (
        thread_source == "user"
        and meta.get("forked_from_id") in {None, ""}
    )
    return CodexCapture(
        source_sha256=sha256_bytes(raw_bytes),
        session_id=str(session_id),
        cwd=cwd,
        git_revision=git_revision,
        source=str(source),
        originator=str(originator),
        cli_version=str(cli_version),
        thread_source=str(thread_source),
        fresh_user_thread=fresh_user_thread,
        repository_scoped_activation_observed=repository_scoped_activation_observed,
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
    if envelope.get("format_version") != 7:
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
    goals = recall_result.get("goals")
    if not isinstance(goals, list) or not goals or not all(nonempty(value) for value in goals):
        return None
    identities: list[str] = []
    for goal in goals:
        matches = [
            row.get("id")
            for row in bundle.rows("context_items")
            if row.get("role") == "goal" and row.get("statement") == goal and nonempty(row.get("id"))
        ]
        if len(matches) != 1:
            return None
        identities.append(str(matches[0]))
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
