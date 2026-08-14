#!/usr/bin/env python3
"""Non-fail-fast V11 rehearsal through installed CLI and MCP boundaries."""

from __future__ import annotations

import argparse
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
import time
from typing import Any


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
) -> dict[str, Any]:
    auth = Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex"))) / "auth.json"
    isolated_auth = Path(env["CODEX_HOME"]) / "auth.json"
    if codex is None:
        return step("environment_blocked", "Codex CLI is unavailable")
    if not auth.is_file():
        return step("environment_blocked", "Codex authentication is unavailable")
    shutil.copy2(auth, isolated_auth)
    isolated_auth.chmod(0o600)
    prompt = (
        "Use the registered Volicord MCP server's project_health tool for Project "
        f"{project_id}. Do not run shell commands. Report its returned connection and capability state."
    )
    result = recorder.run(
        "authenticated-codex",
        [
            codex, "--ask-for-approval", "never", "--config",
            'mcp_servers.volicord.tools.project_health.approval_mode="approve"',
            "exec", "--ephemeral", "--json", "--sandbox", "read-only",
            "--skip-git-repo-check", "-C", str(repository), prompt,
        ],
        env,
        timeout=180,
    )
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
    if not project_id:
        for name in REQUIRED_STEPS[3:]:
            steps[name] = step("skipped", "Project initialization failed")
        steps["codex_mcp_connection"] = step("skipped", "Project initialization failed")
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
    codex_result = authenticated_codex(recorder, codex, env, repository, project_id)
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

    try:
        host = Mcp(mcp_binary, env)
        catalog = host.initialize()
        candidate_view, candidate_ok = host.tool("candidate_inspect", {"project_id": project_id})
        frontier, frontier_ok = host.tool("inquiry_frontier", {"project_id": project_id})
        host.close()
        mutation_names = {tool["name"] for tool in catalog} & {
            "candidate_collect", "candidate_promote", "candidate_dismiss", "candidate_expire"
        }
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        candidate_view, candidate_ok, frontier, frontier_ok, mutation_names = {"error": str(error)}, False, {}, False, set()
    candidate_count = len(candidate_view.get("candidates", [])) if candidate_ok and candidate_view else 0
    steps["candidate_boundary"] = step(
        "unsupported" if candidate_ok and not mutation_names and candidate_count == 0 else "partial",
        "analysis produced no inspectable Candidate and the supported tool catalog has no collection/promotion/disposition operation",
        inspection=candidate_view, candidate_mutation_tools=sorted(mutation_names),
    )
    question_count = len(frontier.get("questions", [])) if frontier_ok and frontier else 0
    steps["inquiry_decision"] = step(
        "unsupported" if frontier_ok and question_count == 0 else "partial",
        "no public path promoted a material Question, so an explicit staged Decision could not be exercised",
        frontier=frontier,
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

    expiration = str(time.time_ns() // 1_000 + 600_000_000)
    target_effect = target_root / "guarded-target.txt"
    target_effect.write_text("controlled V11 effect target\n", encoding="utf-8")
    request, request_op = cli_json(
        recorder, "guarded-request", cli, env, "guarded", "request", project_id,
        "destructive-delete", "delete controlled V11 target", str(target_effect),
        "remove the controlled temporary file", "bounded destructive deletion", expiration,
        f"path:{target_effect}",
    )
    wrong, wrong_op = (None, {"exit_code": None})
    confirmed = None
    if request:
        wrong, wrong_op = cli_json(
            recorder, "guarded-mismatch", cli, env, "guarded", "confirm",
            request["confirmation_request_identity"], str(request["request_revision"]),
            "sha256:" + "0" * 64, "codex", "v11", "Confirm exact controlled deletion",
        )
        try:
            host = Mcp(mcp_binary, env)
            host.initialize()
            confirmed, confirmed_ok = host.tool("guarded_interaction", {
                "confirmation_request_id": request["confirmation_request_identity"],
                "request_revision": request["request_revision"],
                "effect_fingerprint": request["effect_fingerprint"],
                "decision": "confirm",
                "user_turn": "Confirm the exact controlled V11 deletion",
            })
            host.close()
        except (OSError, RuntimeError, ValueError, json.JSONDecodeError):
            confirmed_ok = False
    else:
        confirmed_ok = False
    steps["guarded_boundary"] = step(
        "unsupported",
        "exact request/Source-linked confirmation is public, but no CLI/MCP/viewer dispatch operation exists to exercise completion, failure, indeterminate outcome, or reuse rejection",
        request=request, request_operation=request_op, mismatched_confirmation_exit=wrong_op.get("exit_code"),
        exact_confirmation=confirmed, exact_confirmation_ok=confirmed_ok,
        controlled_target_remains=target_effect.exists(), public_tools=mcp_evidence.get("tool_names", []),
    )

    checkpoint_source, source_op = cli_json(
        recorder, "checkpoint-source", cli, env, "canonical", "user-source", project_id,
        "codex", "v11", "Record the source-grounded V11 checkpoint",
    )
    checkpoint_value, checkpoint_op = (None, {"exit_code": None})
    if checkpoint_source:
        checkpoint_value, checkpoint_op = cli_json(
            recorder, "checkpoint", cli, env, "checkpoint", "record", project_id, "handoff",
            checkpoint_source["identity"], f"Rehearse {target_kind}", "Resume in a new session",
        )
    steps["checkpoint"] = step(
        "passed" if checkpoint_value else "failed",
        "source-grounded Checkpoint was recorded" if checkpoint_value else
        "the public CLI accepted the Checkpoint command shape but the domain operation failed",
        source=checkpoint_source, source_operation=source_op, checkpoint=checkpoint_value, operation=checkpoint_op,
    )
    recall_before, recall_op = cli_json(recorder, "recall", cli, env, "recall", project_id)
    try:
        restarted = Mcp(mcp_binary, env)
        restarted.initialize()
        recall_after, recall_after_ok = restarted.tool("recall", {"project_id": project_id})
        restart_cleanup = restarted.close()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        recall_after, recall_after_ok, restart_cleanup = {"error": str(error)}, False, {}
    steps["restart_recall"] = step(
        "partial" if recall_after_ok else "failed",
        "new MCP process kept Recall usable, but required Checkpoint and Decision context is incomplete",
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
    portability_ok = exported and clone_result["exit_code"] == 0 and imported and bound
    steps["portable_clone"] = step(
        "passed" if portability_ok else "failed", "portable bundle imported and explicitly rebound in another clone",
        export=exported, export_operation=export_op, clone_operation=clone_result,
        import_result=imported, import_operation=import_op, binding=bound, bind_operation=bind_op,
    )

    if portability_ok:
        source_a, _ = cli_json(recorder, "diverge-a-source", cli, env, "canonical", "user-source", project_id, "codex", "clone-a", "Independent A")
        source_b, _ = cli_json(recorder, "diverge-b-source", cli, env, "canonical", "user-source", project_id, "codex", "clone-b", "Independent B", runtime=clone_runtime)
        bundle_a = target_root / "a.volicord.json"
        bundle_b = target_root / "b.volicord.json"
        cli_json(recorder, "diverge-a-export", cli, env, "portable", "export", project_id, str(bundle_a))
        cli_json(recorder, "diverge-b-export", cli, env, "portable", "export", project_id, str(bundle_b), runtime=clone_runtime)
        merged, merge_op = cli_json(recorder, "independent-merge", cli, env, "portable", "import", str(bundle_b))
    else:
        source_a = source_b = merged = None
        merge_op = {"exit_code": None}
    steps["divergent_conflict"] = step(
        "unsupported",
        "independent divergence can be imported, but supported user surfaces expose neither same-record conflict creation nor conflict inspection/resolution",
        clone_a_source=source_a, clone_b_source=source_b, independent_merge=merged, merge_operation=merge_op,
    )

    deletion_authorization, _ = cli_json(
        recorder, "deletion-authorization", cli, env, "canonical", "user-source", project_id,
        "codex", "v11", "Authorize deletion of the disposable V11 Source",
    )
    disposable_source, _ = cli_json(
        recorder, "disposable-source", cli, env, "canonical", "user-source", project_id,
        "codex", "v11", "Disposable V11 source content",
    )
    deletion = None
    if deletion_authorization and disposable_source:
        deletion, deletion_op = cli_json(
            recorder, "forget-source", cli, env, "canonical", "forget", project_id,
            "source", disposable_source["identity"], deletion_authorization["identity"],
        )
    else:
        deletion_op = {"exit_code": None}
    steps["correction_supersession_deletion"] = step(
        "unsupported",
        "deletion is public and was exercised, but no public Candidate/Question journey created a correctable Context Item or supersedable Decision",
        deletion_authorization=deletion_authorization, disposable_source=disposable_source,
        deletion=deletion, deletion_operation=deletion_op,
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
    steps["provider_failure"] = step(
        "unsupported",
        "local-only operation with an unconfigured provider is observable, but no public operation requests background semantic dispatch to exercise an unavailable provider and recovery",
        privacy=privacy, privacy_operation=privacy_op, local_analysis_remained=analysis_ok,
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
    fake = {
        "schema_version": 1,
        "repositories": [
            {"class": name, "steps": {key: step("skipped", "self-check") for key in REQUIRED_STEPS}}
            for name in ("volicord", "small-python", "polyglot-medium")
        ],
    }
    validate_result(fake)
    print(json.dumps({"status": "passed", "required_steps": len(REQUIRED_STEPS), "polyglot_hash": tree_hash(POLYGLOT_FIXTURE)}, indent=2))
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
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "self-check":
        return self_check()
    if args.command == "preflight":
        return preflight(args)
    return run(args)


if __name__ == "__main__":
    raise SystemExit(main())
