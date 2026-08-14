#!/usr/bin/env python3
"""Non-fail-fast V11 rehearsal through installed CLI and MCP boundaries."""

from __future__ import annotations

import argparse
import ast
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterator


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
INSTALLER = ROOT / "rebuild/install.sh"
SMALL_FIXTURE = ROOT / "rebuild/validation/repository-intelligence/polyglot-structural/fixtures/python"
POLYGLOT_FIXTURE = HERE / "fixtures/polyglot-medium"
DOCUMENT_KINDS = (
    "project-architecture-guide",
    "decision-report",
    "implementation-plan",
    "handoff-resume",
)
REQUIRED_STEPS = (
    "clean_install",
    "codex_mcp_connection",
    "project_binding",
    "repository_analysis",
    "source_grounded_understanding",
    "candidate_boundary",
    "inquiry_decision",
    "ordinary_work",
    "guarded_boundary",
    "checkpoint",
    "restart_recall",
    "portable_clone",
    "divergent_conflict",
    "correction_supersession_deletion",
    "document_outputs",
    "provider_failure",
    "parser_failure",
    "derived_index_recovery",
)
ALLOWED_STATUS = {
    "passed",
    "partial",
    "unsupported",
    "failed",
    "environment_blocked",
    "skipped",
}
PROVIDER_SOURCE_PATHS = {
    "volicord": "rebuild/Cargo.toml",
    "small-python": "src/greeter/__init__.py",
    "polyglot-medium": "system.json",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_hash(directory: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(
        item
        for item in directory.rglob("*")
        if item.is_file() and ".git" not in item.relative_to(directory).parts
    ):
        relative = path.relative_to(directory).as_posix().encode()
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


class Recorder:
    def __init__(self, root: Path):
        self.root = root
        self.sequence = 0

    def run(
        self,
        label: str,
        argv: list[str],
        env: dict[str, str],
        *,
        cwd: Path = ROOT,
        timeout: int = 300,
    ) -> dict[str, Any]:
        self.sequence += 1
        directory = self.root / "operations" / f"{self.sequence:03d}-{label}"
        directory.mkdir(parents=True)
        started_at = time.time_ns() // 1_000
        started = time.monotonic_ns()
        metadata = {
            "schema_version": 1,
            "argv": argv,
            "command": shlex.join(argv),
            "working_directory": str(cwd),
            "started_at_unix_micros": started_at,
        }
        write_json(directory / "command.json", metadata)
        termination = None
        spawn_error = None
        exit_code = None
        stdout = b""
        stderr = b""
        try:
            process = subprocess.Popen(
                argv,
                cwd=cwd,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            try:
                stdout, stderr = process.communicate(timeout=timeout)
            except subprocess.TimeoutExpired:
                process.terminate()
                try:
                    stdout, stderr = process.communicate(timeout=10)
                    termination = {"kind": "timeout_terminate"}
                except subprocess.TimeoutExpired:
                    process.kill()
                    stdout, stderr = process.communicate()
                    termination = {"kind": "timeout_kill"}
            if process.returncode >= 0:
                exit_code = process.returncode
            else:
                termination = {"kind": "signal", "number": -process.returncode}
        except OSError as error:
            spawn_error = f"{type(error).__name__}: {error}"
            stderr = (spawn_error + "\n").encode()
        (directory / "stdout.log").write_bytes(stdout)
        (directory / "stderr.log").write_bytes(stderr)
        result = {
            **metadata,
            "ended_at_unix_micros": time.time_ns() // 1_000,
            "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
            "exit_code": exit_code,
            "termination": termination,
            "spawn_error": spawn_error,
            "stdout": str(directory / "stdout.log"),
            "stderr": str(directory / "stderr.log"),
            "outcome": (
                "spawn_failed" if spawn_error else "terminated" if termination else
                "succeeded" if exit_code == 0 else "failed"
            ),
        }
        write_json(directory / "result.json", result)
        return result


def decoded(result: dict[str, Any]) -> str:
    return Path(result["stdout"]).read_text(encoding="utf-8")


def stderr_text(result: dict[str, Any]) -> str:
    return Path(result["stderr"]).read_text(encoding="utf-8")


def cli_json(
    recorder: Recorder,
    label: str,
    cli: Path,
    env: dict[str, str],
    *args: str,
    runtime: Path | None = None,
) -> tuple[dict[str, Any] | None, dict[str, Any]]:
    argv = [str(cli)]
    if runtime is not None:
        argv += ["--runtime", str(runtime)]
    result = recorder.run(label, argv + list(args), env)
    if result["exit_code"] != 0:
        return None, result
    try:
        value = json.loads(decoded(result))
    except json.JSONDecodeError:
        return None, result
    return value, result


class Mcp:
    def __init__(self, binary: Path, env: dict[str, str]):
        self.process = subprocess.Popen(
            [str(binary)], cwd=ROOT, env=env, text=True,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.request_id = 0

    def rpc(self, method: str, params: dict[str, Any]) -> dict[str, Any]:
        self.request_id += 1
        assert self.process.stdin is not None and self.process.stdout is not None
        message = {"jsonrpc": "2.0", "id": self.request_id, "method": method, "params": params}
        self.process.stdin.write(json.dumps(message, separators=(",", ":")) + "\n")
        self.process.stdin.flush()
        response = self.process.stdout.readline()
        if not response:
            raise RuntimeError(f"MCP ended while handling {method}")
        return json.loads(response)

    def initialize(self) -> list[dict[str, Any]]:
        initialized = self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}})
        if "error" in initialized:
            raise RuntimeError(str(initialized["error"]))
        catalog = self.rpc("tools/list", {})
        return catalog["result"]["tools"]

    def tool(self, name: str, arguments: dict[str, Any]) -> tuple[dict[str, Any] | None, bool]:
        response = self.rpc("tools/call", {"name": name, "arguments": arguments})
        result = response.get("result", {})
        return result.get("structuredContent"), result.get("isError") is False

    def close(self) -> dict[str, Any]:
        assert self.process.stdin is not None
        self.process.stdin.close()
        try:
            code = self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            code = self.process.wait()
        stderr = self.process.stderr.read() if self.process.stderr else ""
        return {"exit_code": code, "stderr": stderr}


def step(status: str, summary: str, **evidence: Any) -> dict[str, Any]:
    if status not in ALLOWED_STATUS:
        raise ValueError(f"invalid V11 status: {status}")
    return {"status": status, "summary": summary, "evidence": evidence}


def recall_meaning(value: dict[str, Any] | None) -> dict[str, Any] | None:
    if value is None:
        return None
    return {key: item for key, item in value.items() if key != "used_sources"}


def canonical_record(
    inspection: dict[str, Any] | None,
    kind: str,
    *,
    summary_contains: str | None = None,
    lifecycle_state: str | None = None,
) -> dict[str, Any] | None:
    records = inspection.get("records", []) if inspection else []
    return next(
        (
            record
            for record in records
            if record.get("kind") == kind
            and (summary_contains is None or summary_contains in record.get("summary", ""))
            and (lifecycle_state is None or record.get("lifecycle_state") == lifecycle_state)
        ),
        None,
    )


def unsupported_cli(*operations: dict[str, Any]) -> bool:
    attempted = [operation for operation in operations if operation.get("exit_code") is not None]
    return bool(attempted) and any(
        operation.get("exit_code") == 2
        or "unsupported" in stderr_text(operation).lower()
        for operation in attempted
    )


class AuthenticationCleanupError(RuntimeError):
    """The bounded authenticated Codex staging directory could not be removed."""


@contextmanager
def staged_codex_authentication(
    source_auth: Path,
    registered_codex_home: Path,
    retained_root: Path,
    *,
    staging_parent: Path | None = None,
) -> Iterator[Path]:
    """Yield a Codex home containing only the registered config and staged auth."""
    if staging_parent is not None:
        staging_parent.mkdir(parents=True, exist_ok=True)
    temporary = tempfile.TemporaryDirectory(
        prefix="volicord-v11-codex-auth-",
        dir=staging_parent,
    )
    staging_directory = Path(temporary.name)
    try:
        retained = retained_root.resolve()
        staged = staging_directory.resolve()
        if staged == retained or retained in staged.parents:
            raise RuntimeError("Codex authentication staging resolved inside retained V11 artifacts")

        codex_home = staging_directory / "codex-home"
        codex_home.mkdir(mode=0o700)
        registered_config = registered_codex_home / "config.toml"
        if not registered_config.is_file():
            raise FileNotFoundError("the isolated V11 Codex registration is unavailable")
        shutil.copyfile(registered_config, codex_home / "config.toml")
        (codex_home / "config.toml").chmod(0o600)
        shutil.copyfile(source_auth, codex_home / "auth.json")
        (codex_home / "auth.json").chmod(0o600)
        yield codex_home
    finally:
        try:
            temporary.cleanup()
        except OSError as error:
            raise AuthenticationCleanupError(
                f"temporary Codex authentication cleanup failed: {type(error).__name__}"
            ) from error
        if staging_directory.exists():
            raise AuthenticationCleanupError("temporary Codex authentication directory remains")


def git_revision(recorder: Recorder, repository: Path, env: dict[str, str]) -> str:
    result = recorder.run("git-revision", ["git", "rev-parse", "HEAD"], env, cwd=repository)
    return decoded(result).strip() if result["exit_code"] == 0 else "unavailable"


def prepare_repository(kind: str, destination: Path, recorder: Recorder, env: dict[str, str]) -> dict[str, str]:
    if kind == "volicord":
        result = recorder.run(
            "clone-volicord", ["git", "clone", "--quiet", "--no-hardlinks", str(ROOT), str(destination)], env
        )
        if result["exit_code"] != 0:
            raise RuntimeError(stderr_text(result))
    else:
        source = SMALL_FIXTURE if kind == "small-python" else POLYGLOT_FIXTURE
        shutil.copytree(source, destination)
        for argv in (
            ["git", "init", "--quiet"],
            ["git", "add", "."],
            ["git", "-c", "user.name=V11", "-c", "user.email=v11@example.invalid", "commit", "--quiet", "-m", "fixture"],
        ):
            result = recorder.run("fixture-git", argv, env, cwd=destination)
            if result["exit_code"] != 0:
                raise RuntimeError(stderr_text(result))
    return {"revision": git_revision(recorder, destination, env), "content_sha256": tree_hash(destination)}


def authenticated_codex(
    recorder: Recorder,
    codex: str | None,
    env: dict[str, str],
    repository: Path,
    project_id: str,
    retained_root: Path,
    *,
    staging_parent: Path | None = None,
    authentication_source: Path | None = None,
) -> dict[str, Any]:
    auth = authentication_source or (
        Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex"))) / "auth.json"
    )
    if codex is None:
        return step("environment_blocked", "Codex CLI is unavailable")
    if not auth.is_file():
        return step("environment_blocked", "Codex authentication is unavailable")
    prompt = (
        "Use the registered Volicord MCP server's project_health tool for Project "
        f"{project_id}. Do not run shell commands. Report its returned connection and capability state."
    )
    try:
        with staged_codex_authentication(
            auth,
            Path(env["CODEX_HOME"]),
            retained_root,
            staging_parent=staging_parent,
        ) as codex_home:
            result = recorder.run(
                "authenticated-codex",
                [
                    codex, "--ask-for-approval", "never", "--config",
                    'mcp_servers.volicord.tools.project_health.approval_mode="approve"',
                    "exec", "--ephemeral", "--json", "--sandbox", "read-only",
                    "--skip-git-repo-check", "-C", str(repository), prompt,
                ],
                env | {"CODEX_HOME": str(codex_home)},
                timeout=180,
            )
    except AuthenticationCleanupError as error:
        return step("failed", "temporary Codex authentication cleanup failed", error=str(error))
    except OSError as error:
        return step(
            "environment_blocked",
            "authenticated Codex material could not be staged",
            error=f"{type(error).__name__}: {error}",
        )
    if any(path.is_file() for path in retained_root.rglob("auth.json")):
        return step("failed", "authenticated Codex material remains in retained V11 artifacts")
    if result["exit_code"] != 0:
        return step("environment_blocked", "authenticated Codex turn did not complete", operation=result)
    calls = []
    for line in decoded(result).splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        encoded = json.dumps(event)
        if "mcp_tool_call" in encoded and "volicord" in encoded and "project_health" in encoded:
            calls.append(event)
    if not calls:
        return step("failed", "Codex completed without an observable Volicord project_health call", operation=result)
    return step("passed", "authenticated Codex selected the installed Volicord MCP tool", operation=result)


def rehearse_target(
    target_kind: str,
    run_root: Path,
    recorder: Recorder,
    base_env: dict[str, str],
    codex: str | None,
) -> dict[str, Any]:
    target_root = run_root / "work" / target_kind
    home = target_root / "home"
    prefix = target_root / "prefix"
    runtime = target_root / "runtime"
    repository = target_root / "repository"
    clone = target_root / "clone"
    clone_runtime = target_root / "clone-runtime"
    codex_home = home / ".codex"
    legacy = target_root / "legacy-runtime"
    for path in (home, codex_home, legacy):
        path.mkdir(parents=True, exist_ok=True)
    legacy_sentinel = legacy / "DO-NOT-READ"
    legacy_sentinel.write_text("legacy sentinel\n", encoding="utf-8")
    legacy_before = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))
    env = base_env | {
        "HOME": str(home),
        "XDG_DATA_HOME": str(home / ".local/share"),
        "CODEX_HOME": str(codex_home),
        "VOLICORD_RUNTIME_DIR": str(runtime),
        "VOLICORD_HOME": str(legacy),
        "PATH": f"{prefix / 'bin'}:{base_env.get('PATH', '')}",
    }
    identity = prepare_repository(target_kind, repository, recorder, env)
    steps: dict[str, dict[str, Any]] = {}

    install = recorder.run(
        "install", [str(INSTALLER), "--prefix", str(prefix), "--runtime-dir", str(runtime), "--setup-codex"], env
    )
    cli = prefix / "bin/volicord"
    mcp_binary = prefix / "bin/volicord-mcp"
    installed = install["exit_code"] == 0 and all(
        path.is_file() and path.stat().st_mode & stat.S_IXUSR
        for path in (cli, prefix / "bin/volicord-viewer", mcp_binary)
    )
    steps["clean_install"] = step(
        "passed" if installed else "failed",
        "isolated replacement install completed" if installed else "isolated install failed",
        operation=install,
    )
    if not installed:
        for name in REQUIRED_STEPS[1:]:
            steps[name] = step("skipped", "prerequisite clean installation failed")
        return {"class": target_kind, "identity": identity, "steps": steps}

    initialized, init_op = cli_json(
        recorder, "project-init", cli, env, "project", "init", f"V11 {target_kind}", "--repository", str(repository)
    )
    project_id = initialized.get("project_id") if initialized else None
    steps["project_binding"] = step(
        "passed" if project_id and initialized.get("binding", {}).get("path") == str(repository.resolve()) else "failed",
        "Project initialized with an explicit clone binding" if project_id else "Project initialization failed",
        operation=init_op, project_id=project_id,
    )
    project_prerequisite_status = "passed" if project_id else "skipped"
    if not project_id:
        for name in REQUIRED_STEPS[3:]:
            steps[name] = step(project_prerequisite_status, "Project initialization failed")
        steps["codex_mcp_connection"] = step(
            project_prerequisite_status,
            "Project initialization failed",
        )
        return {"class": target_kind, "identity": identity, "steps": steps}

    mcp_evidence: dict[str, Any] = {}
    try:
        host = Mcp(mcp_binary, env)
        catalog = host.initialize()
        health, health_ok = host.tool("project_health", {"project_id": project_id})
        mcp_evidence = {
            "tool_names": sorted(tool["name"] for tool in catalog),
            "health": health,
            "cleanup": host.close(),
        }
        direct_ok = health_ok and health and health.get("connection") == "connected"
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        direct_ok = False
        mcp_evidence = {"error": str(error)}
    codex_result = authenticated_codex(
        recorder, codex, env, repository, project_id, target_root
    )
    combined_status = "passed" if direct_ok and codex_result["status"] == "passed" else (
        "environment_blocked" if direct_ok and codex_result["status"] == "environment_blocked" else "failed"
    )
    steps["codex_mcp_connection"] = step(
        combined_status, "installed MCP connection and authenticated Codex probe evaluated",
        direct=mcp_evidence, authenticated=codex_result,
    )

    analysis, analysis_op = cli_json(recorder, "analyze", cli, env, "analyze", project_id)
    analysis_ok = analysis is not None and analysis.get("state") in {"succeeded", "partial"}
    steps["repository_analysis"] = step(
        "passed" if analysis_ok else "failed", "inventory/capability analysis returned structured coverage",
        operation=analysis_op, result=analysis,
    )
    try:
        host = Mcp(mcp_binary, env)
        host.initialize()
        understanding, understanding_ok = host.tool("repository_understanding", {"project_id": project_id})
        understanding_cleanup = host.close()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        understanding, understanding_ok, understanding_cleanup = {"error": str(error)}, False, {}
    steps["source_grounded_understanding"] = step(
        "passed" if understanding_ok and understanding and understanding.get("repository_map", {}).get("entity_count", 0) > 0 else "failed",
        "MCP repository understanding exposed source-bound entities and gaps",
        result=understanding, cleanup=understanding_cleanup,
    )

    candidate_tools = {"candidate_inspect", "candidate_manage", "inquiry_frontier", "decision_record", "canonical_inspect"}
    candidate_evidence: dict[str, Any] = {}
    inquiry_evidence: dict[str, Any] = {}
    candidate_status = "failed"
    inquiry_status = "failed"
    decision_id = None
    decision_revision = None
    decision_source_id = None
    try:
        host = Mcp(mcp_binary, env)
        catalog = host.initialize()
        catalog_names = {tool["name"] for tool in catalog}
        missing_candidate_tools = sorted(candidate_tools - catalog_names)
        if missing_candidate_tools:
            candidate_status = inquiry_status = "unsupported"
            candidate_evidence = {"missing_public_tools": missing_candidate_tools}
            inquiry_evidence = {"missing_public_tools": missing_candidate_tools}
        else:
            canonical_before, canonical_before_ok = host.tool("canonical_inspect", {"project_id": project_id})
            repository_source = canonical_record(canonical_before, "source", summary_contains="RepositorySnapshot")
            source_id = repository_source.get("identity") if repository_source else None
            submitted, submitted_ok = host.tool("candidate_manage", {
                "action": "submit_question",
                "project_id": project_id,
                "source_ids": [source_id] if source_id else [],
                "source_operation": "v11-integrated-repository-review",
                "repository_snapshot": analysis.get("repository_snapshot", "unknown") if analysis else "unknown",
                "research_state": "research_required",
                "research_state_basis": "repository structure must be inspected before asking for a user judgment",
                "retention_basis": "retain through the explicit V11 inquiry disposition",
                "bounded_summary": "Choose how this Project should preserve its local context boundary",
                "prompt": "Which context boundary should this Project use?",
                "why_now": "the integrated journey needs one material current-host Decision",
                "affected_scope": ["project-context"],
                "established_facts": ["The Project has a current local Analysis Snapshot"],
                "assumptions": ["The Project remains local-first"],
                "uncertainty": ["External augmentation may be evaluated separately"],
                "alternatives": [
                    {"key": "local", "label": "Local", "consequence": "Keep canonical context local"},
                    {"key": "remote", "label": "Remote", "consequence": "Require a separate provider boundary"},
                ],
                "recommendation_key": "local",
                "recommendation_rationale": "the accepted Project boundary is local-first",
                "trade_offs": ["Remote augmentation remains a separately authorized capability"],
                "known_limits": ["The configured external provider is intentionally unavailable"],
                "what_unlocks": ["the integrated Checkpoint and portability journey"],
                "materiality_rationale": "the choice changes durable context behavior",
                "duplicate_basis": "canonical inspection found no matching Question",
                "presentation_order": 1,
            })
            candidate_id = submitted.get("candidate_id") if submitted_ok and submitted else None
            after_submission, after_submission_ok = host.tool("candidate_inspect", {"project_id": project_id})
            frontier_before, frontier_before_ok = host.tool("inquiry_frontier", {"project_id": project_id})
            insufficient, insufficient_ok = host.tool("candidate_manage", {
                "action": "attach_repository_research",
                "project_id": project_id,
                "candidate_id": candidate_id,
                "capability": "structural",
                "coverage": "current repository declarations",
                "freshness": "current",
                "source_ids": [source_id] if source_id else [],
                "evidence_assessment": "insufficient",
                "limits": ["cross-component consequences still require review"],
            }) if candidate_id else (None, False)
            premature_ready, premature_ready_ok = host.tool("candidate_manage", {
                "action": "mark_research_ready", "project_id": project_id, "candidate_id": candidate_id,
            }) if candidate_id else (None, False)
            sufficient, sufficient_ok = host.tool("candidate_manage", {
                "action": "attach_repository_research",
                "project_id": project_id,
                "candidate_id": candidate_id,
                "capability": "structural",
                "coverage": "current repository structure and explicit Project boundary",
                "freshness": "current",
                "source_ids": [source_id] if source_id else [],
                "evidence_assessment": "sufficient",
                "limits": ["runtime-only external behavior remains excluded"],
            }) if candidate_id else (None, False)
            ready, ready_ok = host.tool("candidate_manage", {
                "action": "mark_research_ready", "project_id": project_id, "candidate_id": candidate_id,
            }) if candidate_id else (None, False)
            ready_inspection, ready_inspection_ok = host.tool("candidate_inspect", {"project_id": project_id})
            promoted, promoted_ok = host.tool("candidate_manage", {
                "action": "promote_question", "project_id": project_id, "candidate_id": candidate_id,
            }) if candidate_id else (None, False)
            question_id = promoted.get("question_id") if promoted_ok and promoted else None
            promoted_inspection, promoted_inspection_ok = host.tool(
                "candidate_inspect", {"project_id": project_id}
            )
            promoted_candidate = next(
                (
                    item
                    for item in (promoted_inspection or {}).get("candidates", [])
                    if item.get("identity") == candidate_id
                ),
                None,
            )
            frontier, frontier_ok = host.tool("inquiry_frontier", {"project_id": project_id})
            questions = frontier.get("questions", []) if frontier_ok and frontier else []
            displayed = next((question for question in questions if question.get("identity") == question_id), None)
            decision, decision_ok = host.tool("decision_record", {
                "project_id": project_id,
                "question_id": question_id,
                "question_revision": displayed.get("revision") if displayed else 0,
                "alternative_key": "local",
                "user_turn": "Choose the local Project context boundary",
                "user_rationale": "Keep canonical Project context local and authorize providers separately",
            }) if displayed else (None, False)
            canonical_after, canonical_after_ok = host.tool("canonical_inspect", {"project_id": project_id})
            decision_record = canonical_record(canonical_after, "decision", lifecycle_state="active")
            decision_id = decision_record.get("identity") if decision_record else None
            decision_revision = decision_record.get("revision") if decision_record else None
            decision_source_id = decision.get("user_response_source_id") if decision_ok and decision else None
            submitted_candidate = next(
                (item for item in (after_submission or {}).get("candidates", []) if item.get("identity") == candidate_id),
                None,
            )
            ready_candidate = next(
                (item for item in (ready_inspection or {}).get("candidates", []) if item.get("identity") == candidate_id),
                None,
            )
            candidate_ok = all([
                canonical_before_ok, source_id, submitted_ok,
                submitted and submitted.get("research_state") == "research_required",
                after_submission_ok, submitted_candidate and submitted_candidate.get("research_state") == "research_required",
                frontier_before_ok, not (frontier_before or {}).get("questions"),
                insufficient_ok,
                insufficient and insufficient.get("research_state") == "research_required",
                not premature_ready_ok, sufficient_ok,
                sufficient and sufficient.get("research_state") == "research_required",
                ready_ok, ready and ready.get("research_state") == "ready_to_ask",
                ready_inspection_ok, ready_candidate and ready_candidate.get("research_state") == "ready_to_ask",
                promoted_ok, question_id,
                promoted_inspection_ok,
                promoted_candidate and promoted_candidate.get("disposition", {}).get("state") == "promoted",
                promoted_candidate and promoted_candidate.get("promotion_target") == question_id,
            ])
            inquiry_ok = all([
                frontier_ok, displayed, decision_ok, decision and decision.get("all_succeeded") is True,
                canonical_after_ok, decision_id, decision_revision, decision_source_id,
            ])
            candidate_status = "passed" if candidate_ok else "failed"
            inquiry_status = "passed" if inquiry_ok else "failed"
            candidate_evidence = {
                "repository_source": repository_source,
                "submission": submitted,
                "inspection_after_submission": submitted_candidate,
                "frontier_after_submission": frontier_before,
                "insufficient_research": insufficient,
                "premature_ready_transition": premature_ready,
                "sufficient_research": sufficient,
                "ready_transition": ready,
                "ready_inspection": ready_candidate,
                "promotion": promoted,
                "promoted_disposition": promoted_candidate,
            }
            inquiry_evidence = {
                "frontier": frontier,
                "displayed_question": displayed,
                "decision": decision,
                "canonical_decision": decision_record,
            }
        candidate_cleanup = host.close()
        candidate_evidence["cleanup"] = candidate_cleanup
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        candidate_evidence = {"error": str(error), **candidate_evidence}
        inquiry_evidence = {"error": str(error), **inquiry_evidence}
    steps["candidate_boundary"] = step(
        candidate_status,
        "research-required Candidate stayed off the frontier until source-grounded research, readiness, inspection, and explicit promotion completed",
        **candidate_evidence,
    )
    steps["inquiry_decision"] = step(
        inquiry_status,
        "the promoted frontier Question received one explicit current-host response and produced an inspectable Decision",
        **inquiry_evidence,
    )

    guarded_before = sha256(runtime / "guarded.sqlite3")
    ordinary = repository / "v11-ordinary-work.txt"
    ordinary.write_text("ordinary repository work requires no Guarded confirmation\n", encoding="utf-8")
    guarded_after = sha256(runtime / "guarded.sqlite3")
    steps["ordinary_work"] = step(
        "passed" if ordinary.is_file() and guarded_before == guarded_after else "failed",
        "ordinary repository work completed without Guarded ceremony",
        changed_path=str(ordinary.relative_to(repository)), guarded_store_unchanged=guarded_before == guarded_after,
    )

    provider_source_path = PROVIDER_SOURCE_PATHS[target_kind]
    provider_opt_in_source, provider_source_op = cli_json(
        recorder, "provider-opt-in-source", cli, env, "canonical", "user-source", project_id,
        "cli", "cli", "Enable the bounded V11 background semantic provider scope",
    )
    provider_opt_in, provider_opt_in_op = (None, {"exit_code": None})
    if provider_opt_in_source:
        provider_opt_in, provider_opt_in_op = cli_json(
            recorder, "provider-opt-in", cli, env, "privacy", "enable", project_id,
            "v11-unavailable-provider", "v11-model", provider_opt_in_source["identity"], provider_source_path,
        )
    guarded_status = "failed"
    provider_status = "failed"
    guarded_evidence: dict[str, Any] = {
        "opt_in_source": provider_opt_in_source,
        "opt_in_source_operation": provider_source_op,
        "opt_in": provider_opt_in,
        "opt_in_operation": provider_opt_in_op,
    }
    provider_evidence: dict[str, Any] = {}
    required_provider_tools = {"background_semantic_operation", "guarded_interaction", "canonical_inspect", "repository_analyze"}
    try:
        provider_host = Mcp(mcp_binary, env)
        provider_catalog = provider_host.initialize()
        provider_tool_names = {tool["name"] for tool in provider_catalog}
        missing_provider_tools = sorted(required_provider_tools - provider_tool_names)
        if missing_provider_tools:
            guarded_status = provider_status = "unsupported"
            guarded_evidence["missing_public_tools"] = missing_provider_tools
            provider_evidence["missing_public_tools"] = missing_provider_tools
        elif provider_opt_in:
            expiration = time.time_ns() // 1_000 + 600_000_000
            prepare_arguments = {
                "action": "prepare",
                "project_id": project_id,
                "provider": "v11-unavailable-provider",
                "model": "v11-model",
                "purpose": "background semantic analysis",
                "requested_capability": "semantic",
                "source_paths": [provider_source_path],
                "expiration_unix_micros": expiration,
            }
            denied_preparation, denied_preparation_ok = provider_host.tool(
                "background_semantic_operation", prepare_arguments
            )
            denied_request = (denied_preparation or {}).get("guarded_request", {})
            denied, denied_ok = provider_host.tool("guarded_interaction", {
                "confirmation_request_id": denied_request.get("confirmation_request_id"),
                "request_revision": denied_request.get("request_revision"),
                "effect_fingerprint": denied_request.get("effect_fingerprint"),
                "decision": "deny",
                "user_turn": "Deny this exact V11 provider transmission",
            }) if denied_request else (None, False)
            denied_dispatch, denied_dispatch_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": denied_request.get("confirmation_request_id"),
                "request_revision": denied_request.get("request_revision"),
                "effect_fingerprint": denied_request.get("effect_fingerprint"),
            }) if denied_request else (None, False)

            prepared, prepared_ok = provider_host.tool("background_semantic_operation", prepare_arguments)
            request = (prepared or {}).get("guarded_request", {})
            provider_request = (prepared or {}).get("provider_request", {})
            inspected_request, inspected_request_ok = provider_host.tool("guarded_interaction", {
                "confirmation_request_id": request.get("confirmation_request_id"),
            }) if request else (None, False)
            mismatched, mismatched_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": "sha256:" + "0" * 64,
            }) if request else (None, False)
            missing, missing_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
            }) if request else (None, False)
            confirmed, confirmed_ok = provider_host.tool("guarded_interaction", {
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
                "decision": "confirm",
                "user_turn": "Confirm this exact filtered V11 provider transmission",
            }) if request else (None, False)
            dispatched, dispatched_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
            }) if request else (None, False)
            durable, durable_ok = provider_host.tool("background_semantic_operation", {
                "action": "inspect",
                "project_id": project_id,
                "operation_id": (dispatched or {}).get("operation_id"),
                "provider_request_id": provider_request.get("provider_request_id"),
            }) if dispatched_ok and dispatched else (None, False)
            reused, reused_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
            }) if request else (None, False)
            local_canonical, local_canonical_ok = provider_host.tool("canonical_inspect", {"project_id": project_id})
            local_structural, local_structural_ok = provider_host.tool(
                "repository_analyze", {"project_id": project_id, "excluded_paths": []}
            )
            provider_request_after = (durable or {}).get("provider_request", {})
            manifest = provider_request_after.get("manifest", [])
            guarded_ok = all([
                denied_preparation_ok,
                denied_preparation and denied_preparation.get("state") == "awaiting_exact_confirmation",
                denied_ok, denied and denied.get("decision") == "denied",
                not denied_dispatch_ok,
                prepared_ok, prepared and prepared.get("state") == "awaiting_exact_confirmation",
                prepared and prepared.get("dispatch_occurred") is False,
                inspected_request_ok,
                inspected_request and inspected_request.get("effect_fingerprint") == request.get("effect_fingerprint"),
                mismatched_ok,
                mismatched and mismatched.get("guarded_outcome", {}).get("rejection") == "mismatched",
                mismatched and mismatched.get("provider_request", {}).get("outcome") == "prepared",
                missing_ok,
                missing and missing.get("guarded_outcome", {}).get("rejection") == "missing",
                missing and missing.get("provider_request", {}).get("outcome") == "prepared",
                confirmed_ok, confirmed and confirmed.get("decision") == "confirmed",
                dispatched_ok,
                dispatched and dispatched.get("guarded_outcome", {}).get("confirmation_consumed") is True,
                durable_ok, durable == dispatched,
                not reused_ok,
                reused and "live provider preparation is unavailable" in reused.get("error", ""),
            ])
            provider_ok = all([
                durable_ok,
                provider_request_after.get("outcome") == "provider_unavailable",
                manifest,
                all(entry.get("transmission_outcome") == "not_transmitted" for entry in manifest),
                local_canonical_ok,
                local_structural_ok,
                local_structural and local_structural.get("state") in {"succeeded", "partial"},
            ])
            guarded_status = "passed" if guarded_ok else "failed"
            provider_status = "passed" if provider_ok else "failed"
            guarded_evidence.update({
                "denied_preparation": denied_preparation,
                "denial": denied,
                "dispatch_after_denial": denied_dispatch,
                "preparation": prepared,
                "exact_inspection": inspected_request,
                "mismatched_dispatch": mismatched,
                "missing_confirmation_dispatch": missing,
                "confirmation": confirmed,
                "dispatch_attempt": dispatched,
                "durable_inspection": durable,
                "reuse_attempt": reused,
            })
            provider_evidence = {
                "durable_inspection": durable,
                "local_canonical": local_canonical,
                "local_structural": local_structural,
            }
        elif unsupported_cli(provider_opt_in_op):
            guarded_status = provider_status = "unsupported"
        provider_cleanup = provider_host.close()
        guarded_evidence["cleanup"] = provider_cleanup
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        guarded_evidence["error"] = str(error)
        provider_evidence["error"] = str(error)
    steps["guarded_boundary"] = step(
        guarded_status,
        "the public provider operation enforced exact inspection, denial cleanup, pre-confirmation rejection, confirmation consumption, durable outcome inspection, and terminal reuse rejection",
        **guarded_evidence,
    )

    checkpoint_value, checkpoint_op = (None, {"exit_code": None})
    checkpoint_next_step = f"Resume the {target_kind} V11 journey in a new session"
    checkpoint_target = "next Codex session"
    if decision_source_id:
        checkpoint_value, checkpoint_op = cli_json(
            recorder, "checkpoint", cli, env, "checkpoint", "record", project_id, "handoff",
            decision_source_id, f"Rehearse {target_kind} after the explicit Decision",
            checkpoint_next_step, checkpoint_target,
        )
    checkpoint_status = "passed" if checkpoint_value else (
        "unsupported" if unsupported_cli(checkpoint_op) else "failed"
    )
    steps["checkpoint"] = step(
        checkpoint_status,
        "the Decision response Source grounded a Handoff Checkpoint with an explicit next-session target",
        decision_source_id=decision_source_id, handoff_target=checkpoint_target,
        checkpoint=checkpoint_value, operation=checkpoint_op,
    )
    recall_before, recall_op = cli_json(recorder, "recall", cli, env, "recall", project_id)
    try:
        restarted = Mcp(mcp_binary, env)
        restarted.initialize()
        recall_after, recall_after_ok = restarted.tool("recall", {"project_id": project_id})
        restart_cleanup = restarted.close()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        recall_after, recall_after_ok, restart_cleanup = {"error": str(error)}, False, {}
    restart_ok = all([
        checkpoint_value,
        recall_before,
        recall_before.get("active_decision_count", 0) >= 1,
        recall_before.get("next_step") == checkpoint_next_step,
        recall_after_ok,
        recall_after,
        len(recall_after.get("decisions", [])) >= 1,
        recall_after.get("next_step") == checkpoint_next_step,
    ])
    steps["restart_recall"] = step(
        "passed" if restart_ok else "failed",
        "a new MCP process recovered the integrated Decision and explicit Handoff next step",
        cli_recall=recall_before, cli_operation=recall_op, restarted_recall=recall_after, cleanup=restart_cleanup,
    )

    base_bundle = target_root / "base.volicord.json"
    exported, export_op = cli_json(recorder, "bundle-export", cli, env, "portable", "export", project_id, str(base_bundle))
    clone_result = recorder.run("clone-target", ["git", "clone", "--quiet", "--no-hardlinks", str(repository), str(clone)], env)
    imported, import_op = cli_json(recorder, "bundle-import", cli, env, "portable", "import", str(base_bundle), runtime=clone_runtime)
    bound, bind_op = (None, {"exit_code": None})
    if imported:
        bound, bind_op = cli_json(
            recorder, "clone-bind", cli, env, "project", "bind", project_id, str(clone), runtime=clone_runtime
        )
    portability_ok = bool(exported and clone_result["exit_code"] == 0 and imported and bound)
    portability_status = "passed" if portability_ok else (
        "unsupported" if unsupported_cli(export_op, import_op, bind_op) else "failed"
    )
    steps["portable_clone"] = step(
        portability_status, "portable bundle imported and explicitly rebound in another clone",
        export=exported, export_operation=export_op, clone_operation=clone_result,
        import_result=imported, import_operation=import_op, binding=bound, bind_operation=bind_op,
    )

    local_decision = incoming_decision = comparison = resolution = None
    source_a = source_b = None
    conflict_operations: list[dict[str, Any]] = []
    if portability_ok and decision_id:
        source_a, source_a_op = cli_json(
            recorder, "diverge-a-source", cli, env, "canonical", "user-source", project_id,
            "codex", "clone-a", "Choose the remote branch in clone A",
        )
        source_b, source_b_op = cli_json(
            recorder, "diverge-b-source", cli, env, "canonical", "user-source", project_id,
            "codex", "clone-b", "Retain the local branch in clone B", runtime=clone_runtime,
        )
        conflict_operations.extend([source_a_op, source_b_op])
        if source_a:
            local_decision, local_decision_op = cli_json(
                recorder, "diverge-a-decision", cli, env, "canonical", "supersede-decision",
                project_id, decision_id, source_a["identity"], "remote", "Clone A chooses remote augmentation",
            )
            conflict_operations.append(local_decision_op)
        if source_b:
            incoming_decision, incoming_decision_op = cli_json(
                recorder, "diverge-b-decision", cli, env, "canonical", "supersede-decision",
                project_id, decision_id, source_b["identity"], "local", "Clone B retains local context",
                runtime=clone_runtime,
            )
            conflict_operations.append(incoming_decision_op)
        bundle_a = target_root / "a.volicord.json"
        bundle_b = target_root / "b.volicord.json"
        bundle_a_value, bundle_a_op = cli_json(
            recorder, "diverge-a-export", cli, env, "portable", "export", project_id, str(bundle_a)
        )
        bundle_b_value, bundle_b_op = cli_json(
            recorder, "diverge-b-export", cli, env, "portable", "export", project_id, str(bundle_b),
            runtime=clone_runtime,
        )
        conflict_operations.extend([bundle_a_op, bundle_b_op])
        if bundle_a_value and bundle_b_value:
            comparison, comparison_op = cli_json(
                recorder, "divergent-compare", cli, env, "portable", "compare", str(bundle_b),
                "--base", str(base_bundle),
            )
            conflict_operations.append(comparison_op)
            if comparison and source_a:
                resolution, resolution_op = cli_json(
                    recorder, "divergent-resolution", cli, env, "portable", "resolve", str(bundle_b),
                    comparison["conflict_set_identity"], str(comparison["conflict_revision"]),
                    source_a["identity"], "context-branch", "--base", str(base_bundle),
                )
                conflict_operations.append(resolution_op)
    conflicts = comparison.get("conflicts", []) if comparison else []
    conflict_ok = all([
        source_a, source_b, local_decision, incoming_decision, comparison,
        comparison and comparison.get("requires_user_resolution") is True,
        any(conflict.get("class") in {"same_record_revision", "semantic_decision_conflict"} for conflict in conflicts),
        resolution,
        resolution and resolution.get("status") in {"resolved", "branched"},
        resolution and resolution.get("resolution_source_id") == source_a.get("identity") if source_a else False,
    ])
    conflict_status = "passed" if conflict_ok else (
        "unsupported" if unsupported_cli(*conflict_operations) else "failed"
    )
    steps["divergent_conflict"] = step(
        conflict_status,
        "both clones superseded the same integrated Decision, exposed the canonical conflict set, and explicitly created a context branch",
        clone_a_source=source_a, clone_b_source=source_b,
        clone_a_decision=local_decision, clone_b_decision=incoming_decision,
        comparison=comparison, resolution=resolution,
    )

    correction_authorization, correction_source_op = cli_json(
        recorder, "correction-authorization", cli, env, "canonical", "user-source", project_id,
        "codex", "v11-correction", "Correct the integrated Decision rationale",
    )
    corrected = None
    correction_op = {"exit_code": None}
    if correction_authorization and local_decision:
        corrected, correction_op = cli_json(
            recorder, "correct-decision", cli, env, "canonical", "correct-decision", project_id,
            local_decision["identity"], str(local_decision["revision"]), correction_authorization["identity"],
            "Remote augmentation Clone A chooses",
        )
    supersession_authorization, supersession_source_op = cli_json(
        recorder, "supersession-authorization", cli, env, "canonical", "user-source", project_id,
        "codex", "v11-supersession", "Return the integrated Decision to the local boundary",
    )
    superseded = None
    supersession_op = {"exit_code": None}
    if supersession_authorization and local_decision and corrected:
        superseded, supersession_op = cli_json(
            recorder, "supersede-corrected-decision", cli, env, "canonical", "supersede-decision",
            project_id, local_decision["identity"], supersession_authorization["identity"], "local",
            "Keep canonical context local after evaluating the explicit provider boundary",
        )
    deletion_authorization, deletion_source_op = cli_json(
        recorder, "deletion-authorization", cli, env, "canonical", "user-source", project_id,
        "codex", "v11", "Authorize deletion of the disposable V11 Source",
    )
    disposable_source, disposable_source_op = cli_json(
        recorder, "disposable-source", cli, env, "canonical", "user-source", project_id,
        "codex", "v11", "Disposable Source created by the integrated V11 journey",
    )
    deletion = None
    if deletion_authorization and disposable_source:
        deletion, deletion_op = cli_json(
            recorder, "forget-source", cli, env, "canonical", "forget", project_id,
            "source", disposable_source["identity"], deletion_authorization["identity"],
        )
    else:
        deletion_op = {"exit_code": None}
    canonical_after_mutations, canonical_after_mutations_op = cli_json(
        recorder, "canonical-after-mutations", cli, env, "canonical", "inspect", project_id
    )
    records_after_mutations = canonical_after_mutations.get("records", []) if canonical_after_mutations else []
    mutation_operations = [
        correction_source_op, correction_op, supersession_source_op, supersession_op,
        deletion_source_op, disposable_source_op, deletion_op, canonical_after_mutations_op,
    ]
    mutations_ok = all([
        corrected,
        corrected and corrected.get("identity") == local_decision.get("identity") if local_decision else False,
        corrected and corrected.get("revision") == 2,
        superseded,
        superseded and superseded.get("identity") != local_decision.get("identity") if local_decision else False,
        deletion,
        canonical_after_mutations,
        disposable_source and all(record.get("identity") != disposable_source.get("identity") for record in records_after_mutations),
        canonical_record(canonical_after_mutations, "decision", lifecycle_state="active") is not None,
    ])
    mutation_status = "passed" if mutations_ok else (
        "unsupported" if unsupported_cli(*mutation_operations) else "failed"
    )
    steps["correction_supersession_deletion"] = step(
        mutation_status,
        "the integrated Decision was corrected and superseded, and an integrated disposable Source was forgotten",
        correction_authorization=correction_authorization, correction=corrected,
        supersession_authorization=supersession_authorization, supersession=superseded,
        deletion_authorization=deletion_authorization, disposable_source=disposable_source,
        deletion=deletion, canonical_after=canonical_after_mutations,
    )

    canonical_before_docs = target_root / "before-documents.json"
    cli_json(recorder, "docs-before-bundle", cli, env, "portable", "export", project_id, str(canonical_before_docs))
    document_results = []
    for kind in DOCUMENT_KINDS:
        for format_name, suffix in (("markdown", "md"), ("html", "html")):
            destination = target_root / "documents" / f"{kind}.{suffix}"
            destination.parent.mkdir(parents=True, exist_ok=True)
            value, operation = cli_json(
                recorder, f"document-{kind}-{format_name}", cli, env, "documents", "export",
                project_id, kind, format_name, str(destination), "en",
            )
            document_results.append({"kind": kind, "format": format_name, "result": value, "operation": operation})
    canonical_after_docs = target_root / "after-documents.json"
    cli_json(recorder, "docs-after-bundle", cli, env, "portable", "export", project_id, str(canonical_after_docs))
    docs_ok = all(item["result"] for item in document_results) and canonical_before_docs.read_bytes() == canonical_after_docs.read_bytes()
    steps["document_outputs"] = step(
        "passed" if docs_ok else "failed", "all four Markdown and self-contained HTML outputs were published without canonical mutation",
        documents=document_results, canonical_unchanged=canonical_before_docs.read_bytes() == canonical_after_docs.read_bytes(),
    )

    privacy, privacy_op = cli_json(recorder, "privacy-status", cli, env, "privacy", "status", project_id)
    provider_evidence.update({"privacy": privacy, "privacy_operation": privacy_op})
    steps["provider_failure"] = step(
        provider_status,
        "the configured production adapter reported provider_unavailable without transmission while canonical inspection and local structural analysis remained usable",
        **provider_evidence,
    )

    malformed = repository / ("src/v11_broken.rs" if target_kind == "volicord" else "v11_broken.py")
    malformed.parent.mkdir(parents=True, exist_ok=True)
    malformed.write_text("fn broken( {\n" if malformed.suffix == ".rs" else "def broken(:\n", encoding="utf-8")
    parser_result, parser_op = cli_json(recorder, "parser-degradation", cli, env, "analyze", project_id)
    parser_status = "passed" if parser_result and parser_result.get("state") == "partial" and parser_result.get("failed_scopes") else "failed"
    steps["parser_failure"] = step(
        parser_status, "malformed language area was analyzed and required scoped failure/partial reporting",
        result=parser_result, operation=parser_op,
    )

    stored_at = Path(parser_result["stored_at"]) if parser_result and parser_result.get("stored_at") else None
    recall_pre_recovery, _ = cli_json(recorder, "recovery-recall-before", cli, env, "recall", project_id)
    if stored_at and stored_at.is_file():
        stored_at.write_bytes(b"{ controlled V11 derived corruption")
        degraded_health, health_op = cli_json(recorder, "corrupt-health", cli, env, "health", project_id)
        repaired, repair_op = cli_json(recorder, "derived-repair", cli, env, "repair", project_id, "derived-analysis")
        recall_post_recovery, _ = cli_json(recorder, "recovery-recall-after", cli, env, "recall", project_id)
        recovery_ok = (
            degraded_health and degraded_health.get("state") == "degraded" and repaired and
            repaired.get("state") in {"succeeded", "partial"} and
            recall_meaning(recall_pre_recovery) == recall_meaning(recall_post_recovery)
        )
    else:
        degraded_health = repaired = recall_post_recovery = None
        health_op = repair_op = {"exit_code": None}
        recovery_ok = False
    steps["derived_index_recovery"] = step(
        "passed" if recovery_ok else "failed", "controlled derived corruption was diagnosed and rebuilt without changing Recall",
        degraded_health=degraded_health, health_operation=health_op, repair=repaired,
        repair_operation=repair_op,
        canonical_recall_meaning_unchanged=(
            recall_meaning(recall_pre_recovery) == recall_meaning(recall_post_recovery)
        ),
        repository_source_refreshed=(
            recall_pre_recovery is not None and recall_post_recovery is not None and
            recall_pre_recovery.get("used_sources") != recall_post_recovery.get("used_sources")
        ),
    )

    legacy_after = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))
    return {
        "class": target_kind,
        "identity": identity,
        "project_id": project_id,
        "legacy_runtime_untouched": legacy_before == legacy_after,
        "steps": steps,
    }


def validate_result(result: dict[str, Any]) -> None:
    if result.get("schema_version") != 1:
        raise AssertionError("result schema_version must be 1")
    repositories = result.get("repositories")
    if not isinstance(repositories, list) or len(repositories) != 3:
        raise AssertionError("V11 result must contain three repositories")
    for repository in repositories:
        if set(repository.get("steps", {})) != set(REQUIRED_STEPS):
            raise AssertionError(f"incomplete V11 steps for {repository.get('class')}")
        for value in repository["steps"].values():
            if value.get("status") not in ALLOWED_STATUS:
                raise AssertionError("invalid per-step status")


def assert_required_steps_are_evidence_driven(
    source: str | None = None,
    required_steps: set[str] | None = None,
) -> None:
    tree = ast.parse(source if source is not None else Path(__file__).read_text(encoding="utf-8"))
    required = set(REQUIRED_STEPS) if required_steps is None else required_steps
    assigned: set[str] = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.Assign) or not isinstance(node.value, ast.Call):
            continue
        call = node.value
        if not isinstance(call.func, ast.Name) or call.func.id != "step" or not call.args:
            continue
        for target in node.targets:
            if not (
                isinstance(target, ast.Subscript)
                and isinstance(target.value, ast.Name)
                and target.value.id == "steps"
                and isinstance(target.slice, ast.Constant)
                and isinstance(target.slice.value, str)
            ):
                continue
            name = target.slice.value
            if name not in required:
                continue
            assigned.add(name)
            if isinstance(call.args[0], ast.Constant):
                raise AssertionError(
                    f"required production-backed step {name} has a permanently hard-coded status"
                )
    missing = required - assigned
    if missing:
        raise AssertionError(f"required steps have no evidence-driven assignment: {sorted(missing)}")


def assert_required_step_policy_regressions() -> None:
    required = {"clean_install"}
    hard_coded_skip = 'steps["clean_install"] = step("skipped", "permanent")\n'
    try:
        assert_required_steps_are_evidence_driven(hard_coded_skip, required)
    except AssertionError as error:
        if "permanently hard-coded status" not in str(error):
            raise
    else:
        raise AssertionError("required step with hard-coded skipped status was accepted")

    evidence_conditional = (
        'steps["clean_install"] = step("passed" if observed else "failed", "observed")\n'
    )
    assert_required_steps_are_evidence_driven(evidence_conditional, required)

    dynamic_runtime_classification = (
        'runtime_status = "skipped" if prerequisite_failed else "environment_blocked"\n'
        'steps["clean_install"] = step(runtime_status, "observed runtime classification")\n'
    )
    assert_required_steps_are_evidence_driven(dynamic_runtime_classification, required)


def assert_authenticated_codex_lifecycle() -> None:
    synthetic_material = b'{"synthetic":"v11-auth-lifecycle"}\n'
    with tempfile.TemporaryDirectory(prefix="volicord-v11-auth-self-check-") as directory:
        root = Path(directory)
        retained = root / "retained"
        registered_codex_home = retained / "work" / "synthetic" / "home" / ".codex"
        registered_codex_home.mkdir(parents=True)
        (registered_codex_home / "config.toml").write_text(
            '[mcp_servers.volicord]\ncommand = "synthetic-volicord-mcp"\n',
            encoding="utf-8",
        )
        source_auth = root / "source-auth.json"
        source_auth.write_bytes(synthetic_material)
        repository = root / "repository"
        repository.mkdir()
        visibility_marker = root / "child-saw-auth"
        staging_parent = root / "ephemeral-auth"
        fake_codex = root / "synthetic-codex"
        fake_codex.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, pathlib, sys\n"
            "auth = pathlib.Path(os.environ['CODEX_HOME']) / 'auth.json'\n"
            "expected = b'{\\\"synthetic\\\":\\\"v11-auth-lifecycle\\\"}\\n'\n"
            "if not auth.is_file() or auth.read_bytes() != expected:\n"
            "    raise SystemExit(41)\n"
            "pathlib.Path(os.environ['V11_AUTH_VISIBILITY_MARKER']).write_text('visible\\n')\n"
            "if os.environ.get('V11_SYNTHETIC_CODEX_FAILURE') == '1':\n"
            "    raise SystemExit(19)\n"
            "print(json.dumps({'type':'mcp_tool_call','server':'volicord','tool':'project_health'}))\n",
            encoding="utf-8",
        )
        fake_codex.chmod(0o700)
        recorder = Recorder(retained)
        env = os.environ.copy() | {
            "CODEX_HOME": str(registered_codex_home),
            "V11_AUTH_VISIBILITY_MARKER": str(visibility_marker),
        }

        succeeded = authenticated_codex(
            recorder,
            str(fake_codex),
            env,
            repository,
            "synthetic-project",
            retained,
            staging_parent=staging_parent,
            authentication_source=source_auth,
        )
        if succeeded["status"] != "passed" or not visibility_marker.is_file():
            raise AssertionError("synthetic child could not use staged Codex authentication")
        if any(staging_parent.iterdir()):
            raise AssertionError("Codex authentication staging remains after successful execution")

        failed = authenticated_codex(
            recorder,
            str(fake_codex),
            env | {"V11_SYNTHETIC_CODEX_FAILURE": "1"},
            repository,
            "synthetic-project",
            retained,
            staging_parent=staging_parent,
            authentication_source=source_auth,
        )
        if failed["status"] != "environment_blocked":
            raise AssertionError("synthetic child failure was not handled")
        if any(staging_parent.iterdir()):
            raise AssertionError("Codex authentication staging remains after child failure")

        caught = False
        try:
            with staged_codex_authentication(
                source_auth,
                registered_codex_home,
                retained,
                staging_parent=staging_parent,
            ) as codex_home:
                if (codex_home / "auth.json").read_bytes() != synthetic_material:
                    raise AssertionError("staged authentication was unavailable during execution")
                raise RuntimeError("synthetic handled exception")
        except RuntimeError as error:
            if str(error) != "synthetic handled exception":
                raise
            caught = True
        if not caught or any(staging_parent.iterdir()):
            raise AssertionError("Codex authentication staging remains after handled exception")
        if list(retained.rglob("auth.json")):
            raise AssertionError("retained V11 artifacts contain synthetic authentication")
        for path in retained.rglob("*"):
            if path.is_file() and synthetic_material.rstrip() in path.read_bytes():
                raise AssertionError("retained V11 evidence contains synthetic authentication content")
        if source_auth.read_bytes() != synthetic_material:
            raise AssertionError("source Codex authentication was modified")


def credential_retention_audit(
    artifact_directory: Path,
    authentication_source: Path | None = None,
) -> dict[str, Any]:
    auth = authentication_source or (
        Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex"))) / "auth.json"
    )
    named_files = 0
    content_matches = 0
    scan_errors = 0
    try:
        credential = auth.read_bytes()
    except OSError:
        credential = b""
        scan_errors += 1
    needles = {credential, credential.rstrip()} - {b""}
    if not artifact_directory.is_dir():
        scan_errors += 1
    else:
        for path in artifact_directory.rglob("*"):
            if not path.is_file():
                continue
            if path.name == "auth.json":
                named_files += 1
            try:
                content = path.read_bytes()
            except OSError:
                scan_errors += 1
                continue
            if any(needle in content for needle in needles):
                content_matches += 1
    passed = named_files == 0 and content_matches == 0 and scan_errors == 0
    return {
        "kind": "v11_credential_retention_audit",
        "status": "passed" if passed else "failed",
        "auth_named_file_count": named_files,
        "credential_content_match_count": content_matches,
        "scan_error_count": scan_errors,
    }


def assert_credential_retention_audit() -> None:
    secret = b'{"synthetic":"credential-audit-secret"}\n'
    with tempfile.TemporaryDirectory(prefix="volicord-v11-credential-audit-") as directory:
        root = Path(directory)
        auth = root / "source-auth.json"
        auth.write_bytes(secret)
        clean = root / "clean"
        clean.mkdir()
        (clean / "result.json").write_text('{"status":"passed"}\n', encoding="utf-8")
        clean_result = credential_retention_audit(clean, auth)
        if clean_result["status"] != "passed" or secret.rstrip() in json.dumps(clean_result).encode():
            raise AssertionError("clean credential audit is not bounded and secret-free")
        leaked = root / "leaked"
        leaked.mkdir()
        (leaked / "auth.json").write_bytes(secret)
        leaked_result = credential_retention_audit(leaked, auth)
        if leaked_result["status"] != "failed":
            raise AssertionError("credential audit accepted retained authentication")


def self_check() -> int:
    if platform.system() != "Linux":
        raise AssertionError("V11 is qualified only on Linux")
    if not SMALL_FIXTURE.is_dir() or not POLYGLOT_FIXTURE.is_dir():
        raise AssertionError("required repository fixtures are missing")
    if len(list(POLYGLOT_FIXTURE.rglob("*"))) < 16:
        raise AssertionError("polyglot fixture is no longer medium-sized")
    suffixes = {path.suffix for path in POLYGLOT_FIXTURE.rglob("*") if path.is_file()}
    if not {".java", ".py", ".ts", ".md"} <= suffixes:
        raise AssertionError("polyglot fixture lost three languages or documentation")
    assert_required_steps_are_evidence_driven()
    assert_required_step_policy_regressions()
    assert_authenticated_codex_lifecycle()
    assert_credential_retention_audit()
    fake = {
        "schema_version": 1,
        "repositories": [
            {"class": name, "steps": {key: step("skipped", "self-check") for key in REQUIRED_STEPS}}
            for name in ("volicord", "small-python", "polyglot-medium")
        ],
    }
    validate_result(fake)
    print(json.dumps({
        "status": "passed",
        "required_steps": len(REQUIRED_STEPS),
        "evidence_driven_steps": len(REQUIRED_STEPS),
        "required_step_policy_regressions": "passed",
        "authentication_lifecycle": "passed",
        "credential_retention_audit": "passed",
        "polyglot_hash": tree_hash(POLYGLOT_FIXTURE),
    }, indent=2))
    return 0


def preflight(args: argparse.Namespace) -> int:
    final = json.loads(Path(args.final_artifact).read_text(encoding="utf-8"))
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=True).stdout.strip()
    status = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, text=True, capture_output=True, check=True).stdout
    dirty_paths = [line[3:] for line in status.splitlines() if len(line) > 3]
    allowed_dirty = bool(args.allow_validation_changes) and all(
        path.startswith("rebuild/validation/") for path in dirty_paths
    )
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", args.validated_head, head], cwd=ROOT
    ).returncode == 0
    production_diff = subprocess.run(
        [
            "git", "diff", "--quiet", args.validated_head, head, "--",
            "rebuild/crates", "rebuild/Cargo.toml", "rebuild/Cargo.lock",
            "rebuild/install.sh", "rebuild/docs/design",
        ],
        cwd=ROOT,
    ).returncode
    passed = (
        platform.system() == "Linux" and final.get("outcome") == "succeeded" and
        final.get("failure_count") == 0 and ancestor and production_diff == 0 and
        (not status or allowed_dirty)
    )
    print(json.dumps({
        "status": "passed" if passed else "failed", "head": head,
        "validated_head": args.validated_head, "validated_head_is_ancestor": ancestor,
        "production_diff_empty": production_diff == 0, "worktree_clean": not status,
        "allowed_validation_changes": allowed_dirty, "dirty_paths": dirty_paths,
        "final_artifact": str(Path(args.final_artifact).resolve()), "final_outcome": final.get("outcome"),
        "required_tools": {name: shutil.which(name) for name in ("cargo", "git", "codex")},
    }, indent=2, sort_keys=True))
    return 0 if passed else 1


def run(args: argparse.Namespace) -> int:
    output = Path(args.output_dir).resolve()
    if output.exists():
        raise RuntimeError("V11 output directory already exists")
    output.mkdir(parents=True)
    recorder = Recorder(output)
    base_env = os.environ.copy()
    base_env.setdefault("CARGO_HOME", str(Path.home() / ".cargo"))
    base_env.setdefault("RUSTUP_HOME", str(Path.home() / ".rustup"))
    started = time.monotonic_ns()
    repositories = []
    for target in ("volicord", "small-python", "polyglot-medium"):
        try:
            repositories.append(rehearse_target(target, output, recorder, base_env, shutil.which("codex")))
        except (AssertionError, OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
            repositories.append({
                "class": target,
                "identity": {},
                "steps": {name: step("failed" if name == "clean_install" else "skipped", str(error)) for name in REQUIRED_STEPS},
            })
    statuses = [value["status"] for repository in repositories for value in repository["steps"].values()]
    result = {
        "schema_version": 1,
        "validation_id": "V11",
        "validated_production_head": args.validated_head,
        "final_gate_artifact": str(Path(args.final_artifact).resolve()),
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
        "repositories": repositories,
        "counts": {status: statuses.count(status) for status in sorted(ALLOWED_STATUS)},
        "status": "passed" if statuses and set(statuses) == {"passed"} else "failed",
        "phase_8_ready": bool(statuses and set(statuses) == {"passed"}),
    }
    validate_result(result)
    write_json(output / "result.json", result)
    print(json.dumps({
        "status": result["status"], "phase_8_ready": result["phase_8_ready"],
        "result": str(output / "result.json"), "counts": result["counts"],
    }, indent=2, sort_keys=True))
    return 0 if result["status"] == "passed" else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-check")
    for name in ("preflight", "run"):
        child = subparsers.add_parser(name)
        child.add_argument("--validated-head", required=True)
        child.add_argument("--final-artifact", required=True)
        if name == "preflight":
            child.add_argument("--allow-validation-changes", action="store_true")
        if name == "run":
            child.add_argument("--output-dir", required=True)
    audit = subparsers.add_parser("credential-audit")
    audit.add_argument("--artifact-dir", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "self-check":
        return self_check()
    if args.command == "preflight":
        return preflight(args)
    if args.command == "credential-audit":
        result = credential_retention_audit(Path(args.artifact_dir).resolve())
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result["status"] == "passed" else 1
    return run(args)


if __name__ == "__main__":
    raise SystemExit(main())
