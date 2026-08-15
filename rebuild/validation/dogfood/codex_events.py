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
import shlex
from typing import Any


MAX_CAPTURE_BYTES = 64 * 1024 * 1024
MAX_CAPTURE_EVENTS = 200_000
MAX_PATHS = 256
VOLICORD_NAMESPACE = "mcp__volicord"
VOLICORD_OPERATIONS = {
    "decision_record",
    "checkpoint_record",
    "recall",
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


def parsed_json_object(value: Any) -> dict[str, Any] | None:
    if isinstance(value, dict):
        return value
    if not isinstance(value, str):
        return None
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return None
    return parsed if isinstance(parsed, dict) else None


def structured_tool_result(value: Any) -> dict[str, Any] | None:
    parsed = parsed_json_object(value)
    if parsed is None:
        return None
    if parsed.get("isError") is True:
        return None
    structured = parsed.get("structuredContent")
    if isinstance(structured, dict):
        return structured
    result = parsed.get("result")
    if isinstance(result, dict):
        nested = result.get("structuredContent")
        if isinstance(nested, dict) and result.get("isError") is not True:
            return nested
    return parsed


@dataclass(frozen=True)
class UserTurn:
    sequence: int
    turn_id: str
    user_turn_id: str
    text: str


@dataclass(frozen=True)
class ToolCall:
    sequence: int
    completion_sequence: int
    turn_id: str
    call_id: str
    operation: str
    arguments: dict[str, Any]
    result: dict[str, Any]


@dataclass(frozen=True)
class PathObservation:
    sequence: int
    turn_id: str
    paths: tuple[str, ...]


@dataclass(frozen=True)
class CommandObservation:
    sequence: int
    turn_id: str
    parsed_command: Any
    output_was_empty: bool


@dataclass(frozen=True)
class CodexCapture:
    source_sha256: str
    session_id: str
    cwd: Path
    git_revision: str | None
    source: str
    thread_source: str
    fresh_user_thread: bool
    task_sequences: tuple[int, ...]
    compacted_sequences: tuple[int, ...]
    user_turns: tuple[UserTurn, ...]
    tool_calls: tuple[ToolCall, ...]
    path_observations: tuple[PathObservation, ...]
    commands: tuple[CommandObservation, ...]

    def calls(self, operation: str) -> list[ToolCall]:
        return [call for call in self.tool_calls if call.operation == operation]

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
            for call in self.calls("repository_understanding")
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
    thread_source = meta.get("thread_source")
    if source not in {"cli", "vscode"} or not nonempty(thread_source):
        raise EvidenceError("Codex session source is unsupported")
    git = meta.get("git") if isinstance(meta.get("git"), dict) else {}
    git_revision = git.get("commit_hash") if nonempty(git.get("commit_hash")) else None

    current_turn: str | None = None
    task_sequences: list[int] = []
    compacted_sequences: list[int] = []
    user_turns: list[UserTurn] = []
    calls: dict[str, tuple[int, str, str, dict[str, Any]]] = {}
    completions: dict[str, tuple[int, dict[str, Any]]] = {}
    path_observations: list[PathObservation] = []
    commands: list[CommandObservation] = []

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
        elif envelope == "response_item" and payload_type == "function_call":
            if payload.get("namespace") != VOLICORD_NAMESPACE:
                continue
            operation = normalize_operation(payload.get("name"))
            arguments = parsed_json_object(payload.get("arguments"))
            call_id = payload.get("call_id")
            metadata = payload.get("internal_chat_message_metadata_passthrough")
            turn_id = metadata.get("turn_id") if isinstance(metadata, dict) else None
            if operation and arguments is not None and nonempty(call_id) and nonempty(turn_id):
                calls[str(call_id)] = (sequence, str(turn_id), operation, arguments)
        elif envelope == "response_item" and payload_type == "function_call_output":
            call_id = payload.get("call_id")
            result = structured_tool_result(payload.get("output"))
            if nonempty(call_id) and result is not None:
                completions[str(call_id)] = (sequence, result)
        elif envelope == "event_msg" and payload_type == "patch_apply_end":
            turn_id = payload.get("turn_id")
            raw_paths = payload.get("changes")
            if payload.get("success") is not True or not nonempty(turn_id) or not isinstance(raw_paths, list):
                continue
            paths = [bounded_path(value, cwd) for value in raw_paths]
            if not paths or len(paths) > MAX_PATHS or any(value is None for value in paths):
                continue
            path_observations.append(
                PathObservation(sequence, str(turn_id), tuple(sorted(set(str(value) for value in paths))))
            )
        elif envelope == "event_msg" and payload_type == "exec_command_end":
            turn_id = payload.get("turn_id")
            if payload.get("exit_code") != 0 or not nonempty(turn_id):
                continue
            output = payload.get("aggregated_output")
            commands.append(
                CommandObservation(
                    sequence,
                    str(turn_id),
                    payload.get("parsed_cmd"),
                    isinstance(output, str) and not output.strip(),
                )
            )

    tool_calls = []
    for call_id, (sequence, turn_id, operation, arguments) in calls.items():
        completion = completions.get(call_id)
        if completion is None or completion[0] <= sequence:
            continue
        tool_calls.append(
            ToolCall(sequence, completion[0], turn_id, call_id, operation, arguments, completion[1])
        )
    tool_calls.sort(key=lambda value: value.sequence)

    fresh_user_thread = (
        thread_source == "user"
        and meta.get("forked_from_id") in {None, ""}
        and not compacted_sequences
    )
    return CodexCapture(
        source_sha256=sha256_bytes(raw_bytes),
        session_id=str(session_id),
        cwd=cwd,
        git_revision=git_revision,
        source=str(source),
        thread_source=str(thread_source),
        fresh_user_thread=fresh_user_thread,
        task_sequences=tuple(task_sequences),
        compacted_sequences=tuple(compacted_sequences),
        user_turns=tuple(user_turns),
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
    next_step = recall_result.get("next_step")
    if not nonempty(next_step):
        return None
    matches = [row for row in bundle.rows("checkpoints") if row.get("next_step") == next_step]
    return matches[0] if len(matches) == 1 else None


def recalled_decision_ids(recall_result: dict[str, Any]) -> list[str] | None:
    decisions = recall_result.get("decisions")
    if not isinstance(decisions, list):
        return None
    identities = [item.get("identity") for item in decisions if isinstance(item, dict)]
    if len(identities) != len(decisions) or not all(nonempty(value) for value in identities):
        return None
    return sorted(set(str(value) for value in identities))
