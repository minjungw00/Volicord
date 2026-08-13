#!/usr/bin/env python3
"""Run the clean Linux/Codex portion of the maintained V08 journey."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
INSTALLER = ROOT / "rebuild/install.sh"
EXPECTED_TOOLS = [
    "project_initialize",
    "project_health",
    "recall",
    "repository_understanding",
    "repository_analyze",
    "inquiry_frontier",
    "decision_record",
    "checkpoint_record",
    "canonical_inspect",
    "canonical_mutate",
    "candidate_inspect",
    "privacy_status",
    "document_preview",
    "guarded_interaction",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run(
    arguments: list[str], env: dict[str, str], *, expected: int = 0
) -> subprocess.CompletedProcess[str]:
    print(f"$ {shlex.join(arguments)}", flush=True)
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.stdout:
        print(result.stdout, end="", file=sys.stdout)
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode != expected:
        raise RuntimeError(
            f"expected exit {expected}, got {result.returncode}: {shlex.join(arguments)}"
        )
    return result


def rpc(
    process: subprocess.Popen[str], request_id: int, method: str, params: dict[str, Any]
) -> dict[str, Any]:
    require(process.stdin is not None and process.stdout is not None, "MCP pipes unavailable")
    message = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
    process.stdin.write(json.dumps(message, separators=(",", ":")) + "\n")
    process.stdin.flush()
    response_line = process.stdout.readline()
    require(bool(response_line), f"MCP host ended before responding to {method}")
    response = json.loads(response_line)
    require(response.get("id") == request_id, f"MCP response identity mismatch for {method}")
    require("error" not in response, f"MCP protocol error for {method}: {response}")
    return response


def tool(
    process: subprocess.Popen[str], request_id: int, name: str, arguments: dict[str, Any]
) -> dict[str, Any]:
    response = rpc(
        process,
        request_id,
        "tools/call",
        {"name": name, "arguments": arguments},
    )
    result = response["result"]
    require(result["isError"] is False, f"{name} failed: {result}")
    return result["structuredContent"]


def start_host(binary: Path, env: dict[str, str]) -> subprocess.Popen[str]:
    return subprocess.Popen(
        [str(binary)],
        cwd=ROOT,
        env=env,
        text=True,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def stop_host(process: subprocess.Popen[str]) -> None:
    require(process.stdin is not None, "MCP stdin unavailable")
    process.stdin.close()
    return_code = process.wait(timeout=10)
    stderr = process.stderr.read() if process.stderr is not None else ""
    require(return_code == 0, f"MCP host did not clean up at EOF: {return_code}: {stderr}")
    require(process.poll() == 0, "MCP host remains live after EOF")


def initialize_host(process: subprocess.Popen[str], request_id: int) -> list[str]:
    initialized = rpc(
        process,
        request_id,
        "initialize",
        {"protocolVersion": "2025-06-18", "capabilities": {}},
    )
    require(initialized["result"]["serverInfo"]["name"] == "volicord", "wrong MCP server")
    catalog = rpc(process, request_id + 1, "tools/list", {})["result"]["tools"]
    names = [entry["name"] for entry in catalog]
    require(names == EXPECTED_TOOLS, "high-level MCP catalog changed")
    return names


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    codex = shutil.which("codex")
    if codex is None:
        print(json.dumps({"status": "skipped", "reason": "real codex executable unavailable"}))
        return 77

    base_env = os.environ.copy()
    original_home = Path.home()
    with tempfile.TemporaryDirectory(prefix="volicord-v08-") as directory:
        temporary = Path(directory)
        home = temporary / "home"
        prefix = temporary / "prefix"
        runtime = temporary / "runtime"
        repository = temporary / "repository"
        legacy = temporary / "legacy-runtime"
        codex_home = home / ".codex"
        for path in (home, repository, legacy, codex_home):
            path.mkdir(parents=True)
        (repository / "README.md").write_text("# V08 clean repository\n", encoding="utf-8")
        legacy_sentinel = legacy / "DO-NOT-READ"
        legacy_sentinel.write_text("legacy sentinel\n", encoding="utf-8")
        legacy_before = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))

        env = base_env | {
            "HOME": str(home),
            "XDG_DATA_HOME": str(home / ".local/share"),
            "CODEX_HOME": str(codex_home),
            "VOLICORD_HOME": str(legacy),
            "PATH": f"{prefix / 'bin'}:{base_env.get('PATH', '')}",
        }
        env.setdefault("CARGO_HOME", str(original_home / ".cargo"))
        env.setdefault("RUSTUP_HOME", str(original_home / ".rustup"))
        require(not runtime.exists(), "replacement runtime existed before install")

        version = run([codex, "--version"], env).stdout.strip()
        require(version.startswith("codex-cli "), "unexpected Codex executable")
        install = run(
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
        require("Installed Volicord executables" in install.stdout, "install result missing")

        binaries = [prefix / "bin" / name for name in ("volicord", "volicord-viewer", "volicord-mcp")]
        for binary in binaries:
            mode = binary.stat().st_mode
            require(mode & stat.S_IXUSR != 0, f"binary is not owner-executable: {binary.name}")
            require(os.access(binary, os.X_OK), f"binary is not executable: {binary.name}")
            require(shutil.which(binary.name, path=env["PATH"]) == str(binary), f"PATH misses {binary.name}")
        runtime_files = {path.name for path in runtime.iterdir()}
        require(
            {"canonical.sqlite3", "candidates.sqlite3", "privacy.sqlite3", "guarded.sqlite3"}
            <= runtime_files,
            "clean replacement runtime schemas were not initialized",
        )

        codex_get = json.loads(run([codex, "mcp", "get", "volicord", "--json"], env).stdout)
        encoded_get = json.dumps(codex_get, sort_keys=True)
        require(str(prefix / "bin" / "volicord-mcp") in encoded_get, "Codex command mismatch")
        require(str(runtime) in encoded_get, "Codex runtime environment mismatch")
        codex_list = json.loads(run([codex, "mcp", "list", "--json"], env).stdout)
        require("volicord" in json.dumps(codex_list), "Codex did not discover Volicord")

        cli = prefix / "bin" / "volicord"
        initialized = json.loads(
            run(
                [str(cli), "project", "init", "V08 Project", "--repository", str(repository)],
                env,
            ).stdout
        )
        project_id = initialized["project_id"]
        require(initialized["binding"]["path"] == str(repository.resolve()), "Project binding mismatch")

        host = start_host(prefix / "bin" / "volicord-mcp", env)
        tool_names = initialize_host(host, 1)
        health = tool(host, 3, "project_health", {"project_id": project_id})
        require(health["connection"] == "connected", "MCP connection not reported connected")
        require(health["capability_state"] == "healthy", "clean Project is not healthy")
        recall = tool(host, 4, "recall", {"project_id": project_id})
        require(recall["project_id"] == project_id and recall["read_only"] is True, "Recall mismatch")
        checkpoint = tool(
            host,
            5,
            "checkpoint_record",
            {
                "project_id": project_id,
                "user_turn": "Pause the V08 clean integration journey",
                "goal": "Validate Linux and Codex integration",
                "next_step": "Restart the host",
                "known_limits": ["V11 remains independent"],
            },
        )
        require(checkpoint.get("checkpoint_id"), "Checkpoint call did not create identity")
        stop_host(host)

        restarted = start_host(prefix / "bin" / "volicord-mcp", env)
        initialize_host(restarted, 10)
        restarted_health = tool(restarted, 12, "project_health", {"project_id": project_id})
        require(restarted_health["capability_state"] == "healthy", "restart did not reconnect")
        stop_host(restarted)

        unavailable_repository = temporary / "repository-unavailable"
        repository.rename(unavailable_repository)
        degraded_host = start_host(prefix / "bin" / "volicord-mcp", env)
        initialize_host(degraded_host, 20)
        degraded = tool(degraded_host, 22, "project_health", {"project_id": project_id})
        require(degraded["connection"] == "connected", "degradation was misreported as disconnect")
        require(degraded["capability_state"] == "degraded", "missing repository not degraded")
        require(degraded["repository_available"] is False, "missing repository reported available")
        stop_host(degraded_host)
        unavailable_repository.rename(repository)

        try:
            subprocess.Popen([str(temporary / "missing-mcp")], env=env)
        except FileNotFoundError:
            connection_failure = "launch_failed"
        else:
            raise AssertionError("missing MCP executable unexpectedly launched")

        recall_before = json.loads(run([str(cli), "recall", project_id], env).stdout)
        canonical = runtime / "canonical.sqlite3"
        canonical_size = canonical.stat().st_size
        run(
            [
                str(INSTALLER),
                "--prefix",
                str(prefix),
                "--runtime-dir",
                str(runtime),
                "--uninstall",
            ],
            env,
        )
        require(not any(binary.exists() for binary in binaries), "uninstall left a product binary")
        require(canonical.exists() and canonical.stat().st_size == canonical_size, "uninstall changed canonical data")
        removed_registration = run([codex, "mcp", "get", "volicord", "--json"], env, expected=1)
        require(removed_registration.returncode == 1, "uninstall left Codex registration")

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
        recall_after = json.loads(run([str(cli), "recall", project_id], env).stdout)
        require(recall_after == recall_before, "reinstall changed canonical Recall")
        require(json.loads(run([codex, "mcp", "get", "volicord", "--json"], env).stdout), "registration missing after reinstall")

        legacy_after = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))
        require(legacy_after == legacy_before, "clean journey touched the legacy runtime sentinel")

        print(
            json.dumps(
                {
                    "binaries": [binary.name for binary in binaries],
                    "codex": version,
                    "codex_registration": "discovered",
                    "connection_failure": connection_failure,
                    "degraded_capability": degraded["capability_state"],
                    "legacy_runtime": "untouched",
                    "mcp_tools": len(tool_names),
                    "process_cleanup": "passed",
                    "project_id": project_id,
                    "reinstall_preserved_recall": True,
                    "runtime_schemas": sorted(runtime_files),
                    "status": "passed",
                },
                indent=2,
                sort_keys=True,
            )
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"V08 harness failed: {error}", file=sys.stderr)
        raise SystemExit(1)
