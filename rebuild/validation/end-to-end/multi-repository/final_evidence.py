"""Exact Final leaf evidence and its current gate-parent binding for official V11."""
from __future__ import annotations

import hashlib
import importlib.util
from importlib.machinery import SourceFileLoader
import json
import os
from pathlib import Path
from typing import Any, Sequence

BINDING_PATH_ENV = "VOLICORD_GATE_FINAL_BINDING"
BINDING_HASH_ENV = "VOLICORD_GATE_FINAL_BINDING_SHA256"


def exact_final_passed(summary: dict[str, Any], final_commands: Sequence[Sequence[str]]) -> bool:
    commands = summary.get("commands")
    return bool(
        summary.get("outcome") == "succeeded"
        and type(summary.get("failure_count")) is int and summary["failure_count"] == 0
        and type(summary.get("command_count")) is int
        and summary["command_count"] == len(final_commands)
        and isinstance(commands, list) and len(commands) == len(final_commands)
        and all(isinstance(value, dict)
                and value.get("argv") == list(expected)
                and value.get("outcome") == "succeeded"
                and type(value.get("exit_code")) is int and value["exit_code"] == 0
                and "termination" in value and value["termination"] is None
                and "spawn_error" in value and value["spawn_error"] is None
                for value, expected in zip(commands, final_commands))
    )


def maintained_final_commands(root: Path) -> tuple[tuple[str, ...], ...]:
    # The runner remains the sole owner of the ordered command vector.
    path = root / "rebuild/scripts/validate"
    spec = importlib.util.spec_from_loader("v11_final_owner", SourceFileLoader("v11_final_owner", str(path)))
    assert spec is not None and spec.loader is not None
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    return runner.FINAL_COMMANDS


def issue_gate_binding(gate_directory: Path, candidate_head: str, final_path: Path) -> dict[str, str]:
    binding = {
        "kind": "current_gate_final_binding",
        "gate_invocation": gate_directory.name,
        "gate_directory": str(gate_directory.resolve()),
        "parent_pid": os.getpid(),
        "candidate_head": candidate_head,
        "final_artifact": str(final_path.resolve()),
        "final_sha256": hashlib.sha256(final_path.read_bytes()).hexdigest(),
    }
    path = gate_directory / "final-binding.json"
    path.write_text(json.dumps(binding, sort_keys=True) + "\n", encoding="utf-8")
    return {BINDING_PATH_ENV: str(path.resolve()),
            BINDING_HASH_ENV: hashlib.sha256(path.read_bytes()).hexdigest()}


def read_gate_final(root: Path, candidate_head: str, final_path: Path) -> dict[str, Any]:
    path_value = os.environ.get(BINDING_PATH_ENV)
    if not path_value:
        raise ValueError("official V11 requires the current gate parent's Final binding")
    path = Path(path_value).resolve()
    body = path.read_bytes()
    if hashlib.sha256(body).hexdigest() != os.environ.get(BINDING_HASH_ENV):
        raise ValueError("current gate Final binding bytes changed")
    binding = json.loads(body)
    if (not isinstance(binding, dict) or binding.get("kind") != "current_gate_final_binding"
        or binding.get("parent_pid") != os.getppid()
        or binding.get("candidate_head") != candidate_head
        or binding.get("final_artifact") != str(final_path.resolve())
        or Path(binding.get("gate_directory", "")).resolve() != path.parent
        or binding.get("gate_invocation") != path.parent.name
        or not path.is_relative_to(root / "rebuild/.local/validation")):
        raise ValueError("Final evidence does not belong to this gate parent and exact candidate")
    if not final_path.resolve().is_relative_to(root / "rebuild/.local/validation"):
        raise ValueError("Final artifact must be retained by the maintained validation runner")
    final_body = final_path.read_bytes()
    if hashlib.sha256(final_body).hexdigest() != binding.get("final_sha256"):
        raise ValueError("current gate Final artifact bytes changed")
    summary = json.loads(final_body)
    if (not isinstance(summary, dict) or summary.get("schema_version") != 1
        or summary.get("working_directory") != str(root.resolve())
        or not exact_final_passed(summary, maintained_final_commands(root))):
        raise ValueError("current gate Final artifact has invalid command evidence")
    for command in summary["commands"]:
        directory = Path(command["artifact_directory"])
        if (directory.resolve().parent != final_path.resolve().parent
            or Path(command["stdout"]).resolve() != directory.resolve() / "stdout.log"
            or Path(command["stderr"]).resolve() != directory.resolve() / "stderr.log"):
            raise ValueError("Final leaf paths do not belong to the bound aggregate")
        persisted = json.loads((directory / "result.json").read_text(encoding="utf-8"))
        if (persisted != {key: value for key, value in command.items() if key != "artifact_directory"}
            or not Path(command["stdout"]).is_file() or not Path(command["stderr"]).is_file()):
            raise ValueError("Final command evidence does not match preserved runner leaves")
    return binding
