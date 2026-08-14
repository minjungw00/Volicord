#!/usr/bin/env python3
"""Run one bounded authenticated Codex turn against an isolated Volicord MCP registration."""

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
TOOL_NAMES = {"project_health", "recall"}


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
                    "--setup-codex",
                ],
                env,
            )
            registration = json.loads(run([codex, "mcp", "get", "volicord", "--json"], env).stdout)
            if str(prefix / "bin" / "volicord-mcp") not in json.dumps(registration):
                raise RuntimeError("isolated Codex registration points at the wrong MCP executable")
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
        except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
            return report_blocked("isolated setup could not reach the authenticated turn", error=str(error))

        project_id = initialized["project_id"]
        prompt = (
            "Use the registered Volicord MCP server's read-only Project health capability for "
            f"Project {project_id}. Do not run shell commands and do not infer the result. "
            "You must call the Volicord product tool using its advertised input schema, then "
            "briefly report the returned connection and capability state."
        )
        command = [
            codex,
            "--ask-for-approval",
            "never",
            "--config",
            'mcp_servers.volicord.tools.project_health.approval_mode="approve"',
            "exec",
            "--ephemeral",
            "--json",
            "--sandbox",
            "read-only",
            "--skip-git-repo-check",
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
        if not calls:
            print(
                json.dumps(
                    {
                        "status": "failed",
                        "reason": "Codex completed without a successful observable Volicord product-tool call",
                        "selected_product_tool_calls": [
                            {"server": server, "tool": tool}
                            for server, tool in sorted(set(selected_calls))
                        ],
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
                    "isolated_registration": True,
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
