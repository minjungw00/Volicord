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
MAX_PHASE8_OBJECTIVE_BYTES = 1024
PHASE8_OBJECTIVE_PREFIX = "PHASE8_OBJECTIVE: "
GIT_REVISION = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
VOLICORD_OPERATIONS = {
    "background_semantic_operation",
    "candidate_inspect",
    "candidate_manage",
    "canonical_inspect",
    "canonical_mutate",
    "project_initialize",
    "project_health",
    "context_record",
    "decision_record",
    "document_preview",
    "guarded_interaction",
    "inquiry_frontier",
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


@dataclass(frozen=True)
class UserTurn:
    sequence: int
    turn_id: str
    user_turn_id: str
    text: str


@dataclass(frozen=True)
class Phase8Objective:
    repository_revision: str
    value: str
    sha256: str
    utf8_length: int


def phase8_objective_from_turns(
    turns: tuple[UserTurn, ...],
) -> tuple[str, Phase8Objective | None]:
    """Extract the one current objective envelope without returning its prompt."""
    occurrences = [
        (turn_index, line)
        for turn_index, turn in enumerate(turns)
        for line in turn.text.splitlines()
        if PHASE8_OBJECTIVE_PREFIX in line
    ]
    if not occurrences:
        return "missing", None
    if len(occurrences) != 1:
        return "duplicate", None
    turn_index, line = occurrences[0]
    if turn_index != 0 or not line.startswith(PHASE8_OBJECTIVE_PREFIX):
        return "malformed", None
    encoded = line[len(PHASE8_OBJECTIVE_PREFIX) :]
    try:
        envelope = json.loads(encoded)
    except json.JSONDecodeError:
        return "malformed", None
    if not isinstance(envelope, dict) or set(envelope) != {"repository_revision", "objective"}:
        return "malformed", None
    repository_revision = envelope.get("repository_revision")
    objective = envelope.get("objective")
    if (
        not isinstance(repository_revision, str)
        or GIT_REVISION.fullmatch(repository_revision) is None
        or not nonempty(objective)
        or objective != objective.strip()
        or any(ord(character) < 32 for character in objective)
    ):
        return "malformed", None
    encoded_objective = objective.encode("utf-8")
    if len(encoded_objective) > MAX_PHASE8_OBJECTIVE_BYTES:
        return "malformed", None
    return (
        "valid",
        Phase8Objective(
            repository_revision=repository_revision,
            value=objective,
            sha256=sha256_bytes(encoded_objective),
            utf8_length=len(encoded_objective),
        ),
    )


@dataclass(frozen=True)
class ToolCall:
    sequence: int
    completion_sequence: int
    turn_id: str
    call_id: str
    operation: str
    arguments: dict[str, Any]
    result: dict[str, Any]
    outcome: str
    error: str | None


@dataclass(frozen=True)
class PathObservation:
    sequence: int
    turn_id: str
    paths: tuple[str, ...]


@dataclass(frozen=True)
class CommandObservation:
    sequence: int
    completion_sequence: int
    turn_id: str
    parsed_command: Any
    exit_code: int | None
    termination: str | None
    output: str
    output_was_empty: bool


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

    match = ASSIGNED_CALL.fullmatch(value)
    if match is None:
        return None
    variable = re.escape(match.group("variable"))
    tool_name = match.group("tool")
    if tool_name != "exec_command":
        return None
    forward = match.group("forward").strip()
    result_forward = re.fullmatch(rf"text\s*\(\s*{variable}\s*\)\s*;", forward, re.DOTALL)
    output_forward = re.fullmatch(rf"text\s*\(\s*{variable}\.output\s*\)\s*;", forward, re.DOTALL)
    if result_forward is None and output_forward is None:
        return None
    try:
        arguments = JsLiteralParser(match.group("argument")).parse()
    except EvidenceError:
        return None
    if not isinstance(arguments, dict):
        return None
    mode = "result" if result_forward is not None else "output"
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
    """Normalize the current Codex MCP completion shape without reading wrapper output."""
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


CUSTOM_OUTPUT_HEADER = re.compile(
    r"\AScript completed\nWall time [0-9]+(?:\.[0-9]+)? seconds\nOutput:\n(?P<body>.*)\Z",
    re.DOTALL,
)


def custom_output_body(value: Any) -> str | None:
    if not isinstance(value, list) or not value or len(value) > 8:
        return None
    parts: list[str] = []
    for item in value:
        if not isinstance(item, dict) or item.get("type") != "input_text" or not isinstance(item.get("text"), str):
            return None
        parts.append(item["text"])
    match = CUSTOM_OUTPUT_HEADER.fullmatch("".join(parts))
    return match.group("body") if match is not None else None


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
    task_sequences: tuple[int, ...]
    compacted_sequences: tuple[int, ...]
    user_turns: tuple[UserTurn, ...]
    phase8_objective_status: str
    phase8_objective: Phase8Objective | None
    tool_calls: tuple[ToolCall, ...]
    path_observations: tuple[PathObservation, ...]
    commands: tuple[CommandObservation, ...]

    def calls(self, operation: str) -> list[ToolCall]:
        return [call for call in self.tool_calls if call.operation == operation]

    def successful_calls(self, operation: str) -> list[ToolCall]:
        return [call for call in self.calls(operation) if call.outcome == "succeeded"]

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
            for call in self.successful_calls("repository_understanding")
            if call.sequence > sequence
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


def command_argvs(value: Any) -> list[tuple[str, ...]]:
    if isinstance(value, str):
        parsed = split_command(value)
        return [parsed] if parsed else []
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
            parsed = split_command(command)
            if parsed:
                result.append(parsed)
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
    }
    git_inspections = {"diff", "grep", "log", "ls-files", "show", "status"}
    return any(
        bool(argv)
        and (
            Path(argv[0]).name.lower() in inspection_programs
            or (
                Path(argv[0]).name.lower() == "git"
                and len(argv) >= 2
                and argv[1].lower() in git_inspections
            )
        )
        for argv in command_argvs(value)
    )


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
    compacted_sequences: list[int] = []
    user_turns: list[UserTurn] = []
    calls: dict[str, tuple[int, str, ParsedCustomCall]] = {}
    completions: dict[str, tuple[int, str, Any]] = {}
    mcp_wrappers: dict[str, tuple[int, str, ParsedMcpWrapper]] = {}
    mcp_completions: list[tuple[int, str, dict[str, Any]]] = []
    raw_path_observations: list[tuple[int, str, tuple[str, ...]]] = []

    for sequence, event in enumerate(events):
        payload = event.get("payload")
        if not isinstance(payload, dict):
            continue
        envelope = event.get("type")
        payload_type = payload.get("type")
        if envelope == "event_msg" and payload_type == "task_started":
            turn_id = payload.get("turn_id")
            if nonempty(turn_id):
                current_turn = str(turn_id)
                task_sequences.append(sequence)
        elif envelope == "event_msg" and payload_type == "context_compacted":
            compacted_sequences.append(sequence)
        elif envelope == "event_msg" and payload_type == "user_message":
            message = payload.get("message")
            client_id = payload.get("client_id")
            if current_turn is not None and nonempty(message) and nonempty(client_id):
                user_turns.append(UserTurn(sequence, current_turn, str(client_id), str(message)))
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
            raw_paths = payload.get("changes")
            if (
                payload.get("success") is not True
                or payload.get("status") != "completed"
                or not nonempty(turn_id)
                or not isinstance(raw_paths, dict)
            ):
                continue
            paths = [bounded_path(value, cwd) for value in raw_paths]
            if not paths or len(paths) > MAX_PATHS or any(value is None for value in paths):
                continue
            raw_path_observations.append(
                (sequence, str(turn_id), tuple(sorted(set(str(value) for value in paths))))
            )

    tool_calls: list[ToolCall] = []
    commands: list[CommandObservation] = []
    completed_patches: list[tuple[int, int, str]] = []
    for call_id, (sequence, turn_id, parsed) in calls.items():
        completion = completions.get(call_id)
        if completion is None or completion[0] <= sequence or completion[1] != turn_id:
            continue
        completion_sequence, _, raw_output = completion
        if parsed.tool_name == "exec_command":
            body = custom_output_body(raw_output)
            if body is None:
                continue
            result = custom_output_object(raw_output) if parsed.output_mode == "result" else None
            output = result.get("output") if isinstance(result, dict) else body
            exit_code = result.get("exit_code") if isinstance(result, dict) else None
            if not isinstance(output, str) or (exit_code is not None and not isinstance(exit_code, int)):
                continue
            commands.append(
                CommandObservation(
                    sequence,
                    completion_sequence,
                    turn_id,
                    parsed.arguments,
                    exit_code,
                    "exited" if isinstance(exit_code, int) else None,
                    output,
                    not output.strip(),
                )
            )
        elif parsed.tool_name == "apply_patch" and custom_output_body(raw_output) is not None:
            completed_patches.append((sequence, completion_sequence, turn_id))

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
        tool_calls.append(
            ToolCall(
                sequence,
                sequence,
                turn_id,
                completion_call_id,
                operation,
                arguments,
                result,
                outcome,
                error,
            )
        )

    tool_calls.sort(key=lambda value: value.sequence)
    commands.sort(key=lambda value: value.sequence)
    path_observations = [
        PathObservation(sequence, turn_id, paths)
        for sequence, turn_id, paths in raw_path_observations
        if any(
            start < sequence < completion and patch_turn == turn_id
            for start, completion, patch_turn in completed_patches
        )
    ]

    fresh_user_thread = (
        thread_source == "user"
        and meta.get("forked_from_id") in {None, ""}
        and not compacted_sequences
    )
    objective_status, objective = phase8_objective_from_turns(tuple(user_turns))
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
        task_sequences=tuple(task_sequences),
        compacted_sequences=tuple(compacted_sequences),
        user_turns=tuple(user_turns),
        phase8_objective_status=objective_status,
        phase8_objective=objective,
        tool_calls=tuple(tool_calls),
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
    if envelope.get("format_version") != 6:
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


def relevant_context_ids(bundle: CanonicalBundle, recall_result: dict[str, Any]) -> list[str] | None:
    goals = recall_result.get("goals")
    if not isinstance(goals, list) or not goals or not all(nonempty(value) for value in goals):
        return None
    identities = []
    for goal in goals:
        matches = [
            row.get("id")
            for row in bundle.rows("context_items")
            if row.get("role") == "goal" and row.get("statement") == goal and nonempty(row.get("id"))
        ]
        if len(matches) != 1:
            return None
        identities.append(str(matches[0]))
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
