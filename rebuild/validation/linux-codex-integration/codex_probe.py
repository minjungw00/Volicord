#!/usr/bin/env python3
"""Run one plain repository task through isolated project-scoped Codex activation."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
INSTALLER = ROOT / "rebuild/install.sh"
TOOL_NAMES = {"project_resolve", "recall"}


def report_blocked(reason: str, **details: Any) -> int:
    print(json.dumps({"status": "blocked", "reason": reason, **details}, indent=2, sort_keys=True))
    return 77


def run(
    arguments: list[str], env: dict[str, str], *, expected: int = 0
) -> subprocess.CompletedProcess[str]:
    print(f"$ {shlex.join(arguments)}", flush=True)
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.stdout:
        print(result.stdout, end="", file=sys.stdout)
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode != expected:
        raise RuntimeError(f"expected exit {expected}, got {result.returncode}")
    return result


def tool_call(event: Any, *, require_success: bool = False) -> tuple[str, str] | None:
    if not isinstance(event, dict):
        return None
    item = event.get("item")
    candidates = [event, item] if isinstance(item, dict) else [event]
    for candidate in candidates:
        if candidate.get("type") != "mcp_tool_call":
            continue
        server = candidate.get("server") or candidate.get("server_name")
        tool = candidate.get("tool") or candidate.get("name") or candidate.get("tool_name")
        succeeded = candidate.get("status") == "completed" and candidate.get("error") is None
        if server == "volicord" and tool in TOOL_NAMES and (succeeded or not require_success):
            return str(server), str(tool)
    for value in event.values():
        if isinstance(value, (dict, list)):
            found = tool_call(value, require_success=require_success)
            if found is not None:
                return found
    return None


def repository_inspection(event: Any) -> bool:
    if isinstance(event, list):
        return any(repository_inspection(value) for value in event)
    if not isinstance(event, dict):
        return False
    kind = event.get("type")
    name = event.get("name") or event.get("tool_name")
    if kind in {"command_execution", "file_read", "file_search"}:
        return True
    if isinstance(name, str) and name in {
        "exec_command",
        "apply_patch",
        "read_file",
        "list_directory",
        "search_files",
    }:
        return True
    return any(
        repository_inspection(value)
        for value in event.values()
        if isinstance(value, (dict, list))
    )


def main() -> int:
    codex = shutil.which("codex")
    if codex is None:
        return report_blocked("installed Codex CLI is unavailable")
    source_codex_home = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))
    source_auth = source_codex_home / "auth.json"
    if not source_auth.is_file():
        return report_blocked("Codex authentication cannot be copied into the isolated probe home")

    with tempfile.TemporaryDirectory(prefix="volicord-v08-codex-turn-") as directory:
        temporary = Path(directory)
        home = temporary / "home"
        codex_home = home / ".codex"
        prefix = temporary / "prefix"
        runtime = temporary / "runtime"
        repository = temporary / "repository"
        codex_home.mkdir(parents=True)
        repository.mkdir()
        (repository / "README.md").write_text("# Codex product-tool probe\n", encoding="utf-8")
        run(["git", "-C", str(repository), "init", "--quiet"], os.environ.copy())
        isolated_auth = codex_home / "auth.json"
        shutil.copy2(source_auth, isolated_auth)
        isolated_auth.chmod(0o600)

        env = os.environ.copy() | {
            "HOME": str(home),
            "XDG_DATA_HOME": str(home / ".local/share"),
            "CODEX_HOME": str(codex_home),
            "VOLICORD_RUNTIME_DIR": str(runtime),
            "PATH": f"{prefix / 'bin'}:{os.environ.get('PATH', '')}",
        }
        env.setdefault("CARGO_HOME", str(Path.home() / ".cargo"))
        env.setdefault("RUSTUP_HOME", str(Path.home() / ".rustup"))

        try:
            version = run([codex, "--version"], env).stdout.strip()
            login_result = run([codex, "login", "status"], env)
            login = f"{login_result.stdout}\n{login_result.stderr}".strip()
            if "Logged in" not in login:
                return report_blocked("isolated Codex home is not authenticated", codex=version)
            run(
                [
                    str(INSTALLER),
                    "--prefix",
                    str(prefix),
                    "--runtime-dir",
                    str(runtime),
                ],
                env,
            )
            initialized = json.loads(
                run(
                    [
                        str(prefix / "bin" / "volicord"),
                        "project",
                        "init",
                        "Authenticated Codex Probe",
                        "--repository",
                        str(repository),
                    ],
                    env,
                ).stdout
            )
            run([str(prefix / "bin" / "volicord"), "codex", "enable", str(repository)], env)
            codex_home.joinpath("config.toml").write_text(
                f'[projects.{json.dumps(str(repository.resolve()))}]\ntrust_level = "trusted"\n',
                encoding="utf-8",
            )
        except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
            return report_blocked("isolated setup could not reach the authenticated turn", error=str(error))

        project_id = initialized["project_id"]
        prompt = "Summarize this repository's purpose and current work context. Do not make changes."
        command = [
            codex,
            "--dangerously-bypass-hook-trust",
            "--ask-for-approval",
            "never",
            "--config",
            'mcp_servers.volicord.tools.project_resolve.approval_mode="approve"',
            "--config",
            'mcp_servers.volicord.tools.recall.approval_mode="approve"',
            "exec",
            "--ephemeral",
            "--json",
            "--sandbox",
            "read-only",
            "-C",
            str(repository),
            prompt,
        ]
        print(f"$ {shlex.join(command)}", flush=True)
        started = time.monotonic_ns()
        process = subprocess.Popen(
            command,
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        termination: dict[str, Any] | None = None
        try:
            stdout, stderr = process.communicate(timeout=180)
        except subprocess.TimeoutExpired:
            process.terminate()
            try:
                stdout, stderr = process.communicate(timeout=10)
                termination = {"kind": "timeout-terminate"}
            except subprocess.TimeoutExpired:
                process.kill()
                stdout, stderr = process.communicate()
                termination = {"kind": "timeout-kill"}
        duration_ms = round((time.monotonic_ns() - started) / 1_000_000, 3)
        if stdout:
            print(stdout, end="", file=sys.stdout)
        if stderr:
            print(stderr, end="", file=sys.stderr)
        child_result = {
            "argv": command,
            "command": shlex.join(command),
            "duration_ms": duration_ms,
            "exit_code": process.returncode if process.returncode >= 0 else None,
            "termination": termination
            or (
                {"kind": "signal", "number": -process.returncode}
                if process.returncode < 0
                else None
            ),
        }
        if termination is not None or process.returncode != 0:
            return report_blocked(
                "the supported noninteractive Codex turn was environment-blocked",
                codex=version,
                child=child_result,
            )

        events: list[dict[str, Any]] = []
        for line in stdout.splitlines():
            try:
                value = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(value, dict):
                events.append(value)
        selected_calls = [found for event in events if (found := tool_call(event)) is not None]
        calls = [
            found
            for event in events
            if (found := tool_call(event, require_success=True)) is not None
        ]
        called_tools = [tool for _, tool in calls]
        first_resolve = next(
            (
                index
                for index, event in enumerate(events)
                if tool_call(event, require_success=True) == ("volicord", "project_resolve")
            ),
            None,
        )
        inspected_before_resolve = first_resolve is None or any(
            repository_inspection(event) for event in events[:first_resolve]
        )
        if called_tools[:2] != ["project_resolve", "recall"] or inspected_before_resolve:
            print(
                json.dumps(
                    {
                        "status": "failed",
                        "reason": "plain repository task did not enter Volicord through resolve then Recall",
                        "selected_product_tool_calls": [
                            {"server": server, "tool": tool}
                            for server, tool in sorted(set(selected_calls))
                        ],
                        "repository_inspection_before_resolve": inspected_before_resolve,
                        "child": child_result,
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
            return 1
        print(
            json.dumps(
                {
                    "status": "passed",
                    "codex": version,
                    "authenticated": True,
                    "project_scoped_activation": True,
                    "plain_repository_prompt": True,
                    "project_id": project_id,
                    "observed_product_tool_calls": [
                        {"server": server, "tool": tool}
                        for server, tool in sorted(set(calls))
                    ],
                    "child": child_result,
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
