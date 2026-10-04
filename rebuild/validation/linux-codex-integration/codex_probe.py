#!/usr/bin/env python3
"""Exercise trusted activation and explicit shell readiness in an isolated host probe."""

from __future__ import annotations

import launch_readiness
import argparse
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
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", required=True, help="Exact model for the authorized Codex turn")
    arguments = parser.parse_args()
    if not arguments.model.strip():
        parser.error("--model must not be empty")
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
                        "--json",
                        "--repository",
                        str(repository),
                        "init",
                        "Authenticated Codex Probe",
                    ],
                    env,
                ).stdout
            )
            run([
                str(prefix / "bin" / "volicord"),
                "--repository", str(repository), "codex", "enable",
            ], env)
            codex_home.joinpath("config.toml").write_text(
                f'[projects.{json.dumps(str(repository.resolve()))}]\ntrust_level = "trusted"\n',
                encoding="utf-8",
            )
        except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
            return report_blocked("isolated setup could not reach the authenticated turn", error=str(error))

        shadow = temporary / "conflicting bin"; shadow.mkdir()
        (shadow / "volicord").write_text("#!/bin/sh\nexit 64\n")
        (shadow / "volicord").chmod(0o755)
        env["PATH"] = str(shadow) + ":" + env["PATH"]
        project_id = initialized["project_id"]
        probe_command = [sys.executable, "-B", str(Path(launch_readiness.__file__).resolve()),
            "--binary", str(prefix / "bin/volicord"), "--runtime", str(runtime),
            "--repository", str(repository), "--cli-sha256", launch_readiness.digest(prefix / "bin/volicord"),
            "--mcp-sha256", launch_readiness.digest(prefix / "bin/volicord-mcp")]
        prompt = "Follow the trusted SessionStart guidance: call the Volicord MCP tool project_resolve for this repository and then the MCP tool recall before inspecting or executing repository operations. Then check local Volicord launch readiness by running " + shlex.join(probe_command) + ". Summarize this repository's purpose and current work context. Do not make changes or inspect source files outside this isolated probe repository. Do not inspect credentials or environments; execute the specified readiness command without reading its source."

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
            "--model",
            arguments.model,
            "--ephemeral",
            "--json",
            "--sandbox",
            "workspace-write",
            "--add-dir", str(runtime),
            "-C",
            str(repository),
            prompt,
        ]
        repository_before = {str(p.relative_to(repository)):launch_readiness.digest(p) for p in repository.rglob("*") if p.is_file() and ".git" not in p.parts}
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
        repository_after = {str(p.relative_to(repository)):launch_readiness.digest(p) for p in repository.rglob("*") if p.is_file() and ".git" not in p.parts}
        if repository_before != repository_after:
            return report_blocked("read-only probe task changed repository files", child=child_result)
        registrations = [json.loads(p.read_bytes()) for p in (runtime / "observations/mcp").glob("*.json")]
        expected_mcp = launch_readiness.digest(prefix / "bin/volicord-mcp")
        if not registrations or any(r.get("executable_sha256") != expected_mcp
                or r.get("executable_path") != str((prefix / "bin/volicord-mcp").resolve())
                or r.get("runtime_binding") != launch_readiness.hashlib.sha256(str(runtime.resolve()).encode()).hexdigest()
                or r.get("cwd_binding") != launch_readiness.hashlib.sha256(str(repository.resolve()).encode()).hexdigest()
                for r in registrations):
            return report_blocked("actual MCP lifecycle binding missing or mismatched", child=child_result)
        # Require actual host command completion and read-only Product result, not parent PATH.
        shell_proofs = []
        for event in events:
            item = event.get("item", {})
            if (item.get("type") == "command_execution" and item.get("status") == "completed"
                and item.get("exit_code") == 0 and "launch_readiness.py" in item.get("command", "")):
                for line in item.get("aggregated_output", "").splitlines():
                    try:
                        proof = json.loads(line)
                    except ValueError:
                        continue
                    if (proof.get("status") == "ready" and proof.get("project_id") == project_id
                        and proof.get("cli_sha256") == launch_readiness.digest(prefix / "bin/volicord")
                        and proof.get("mcp_sha256") == launch_readiness.digest(prefix / "bin/volicord-mcp")
                        and proof.get("invoked_executable") == str((prefix / "bin/volicord").resolve())):
                        shell_proofs.append(proof)
        if not shell_proofs:
            return report_blocked("actual host tool-shell launch-readiness proof missing", child=child_result)
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
                        "reason": "explicit host probe did not enter Volicord through resolve then Recall",
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
                    "runtime_access": "workspace_write_with_exact_isolated_runtime",
                    "repository_unchanged": True,
                    "mcp_lifecycle_sha256": expected_mcp,
                    "project_scoped_activation": True,
                    "probe_prompt": "explicit_read_only_launch_probe_not_naturalistic_task",
                    "actual_host_tool_shell": shell_proofs,
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
