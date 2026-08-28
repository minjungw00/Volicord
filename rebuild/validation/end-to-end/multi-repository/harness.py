#!/usr/bin/env python3
"""Non-fail-fast V11 rehearsal through installed CLI and MCP boundaries."""

from __future__ import annotations

import argparse
import ast
from contextlib import contextmanager
import hashlib
import html
import json
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import sqlite3
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
DECISION_REGISTER = ROOT / "rebuild/docs/design/open-decisions.md"
DECISION_REGISTER_PATH = "rebuild/docs/design/open-decisions.md"
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
OFFICIAL_REVISIT_ASSESSMENT = "reported_by_official_v11"
FAILED_REVISIT_ASSESSMENT = "official_v11_assessment_failed"
DECISION_HEADING = re.compile(r"^## \d+\. (Q[0-9]+(?:-[A-Z])?) —", re.MULTILINE)
DECISION_ID = re.compile(r"Q[0-9]+(?:-[A-Z])?")


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


def decision_revisit_assessment(
    source: str,
    *,
    source_sha256: str,
    source_path: str = DECISION_REGISTER_PATH,
) -> dict[str, Any]:
    """Read the maintained Decision register's explicit current trigger state."""
    headings = list(DECISION_HEADING.finditer(source))
    if not headings:
        raise ValueError("the accepted Decision register has no Question decisions")
    decision_ids = [match.group(1) for match in headings]
    if len(decision_ids) != len(set(decision_ids)):
        raise ValueError("the accepted Decision register repeats a Question decision ID")

    accepted_ids: list[str] = []
    for index, match in enumerate(headings):
        block_end = headings[index + 1].start() if index + 1 < len(headings) else len(source)
        block = source[match.end():block_end]
        statuses = re.findall(r"^- 상태: `([^`]+)`$", block, re.MULTILINE)
        if len(statuses) != 1:
            raise ValueError(f"Decision {match.group(1)} has no unambiguous maintained status")
        if statuses[0] == "accepted":
            accepted_ids.append(match.group(1))
            if block.count("- 재검토 조건:") != 1:
                raise ValueError(
                    f"accepted Decision {match.group(1)} has no unambiguous revisit-trigger section"
                )

    unresolved = re.findall(r"^- 미해결 필수 제품 질문: (.+)$", source, re.MULTILINE)
    if len(unresolved) != 1:
        raise ValueError("the Decision register has no unambiguous unresolved-question state")
    active: list[str]
    if unresolved[0].strip() == "없음":
        active = []
    else:
        active = DECISION_ID.findall(unresolved[0])
        normalized = re.sub(r"[` ,·]", "", unresolved[0])
        if not active or normalized != "".join(active):
            raise ValueError("the Decision register unresolved-question state is not a bounded ID list")
        if len(active) != len(set(active)) or any(value not in accepted_ids for value in active):
            raise ValueError("the Decision register names an unknown or duplicate active trigger")

    return {
        "decision_revisit_trigger_assessment": OFFICIAL_REVISIT_ASSESSMENT,
        "active_decision_revisit_triggers": active,
        "decision_revisit_trigger_source": {
            "kind": "accepted_decision_register",
            "path": source_path,
            "content_sha256": source_sha256,
            "assessed_decision_ids": accepted_ids,
            "assessed_decision_count": len(accepted_ids),
        },
    }


def read_decision_revisit_assessment(path: Path = DECISION_REGISTER) -> dict[str, Any]:
    source = path.read_text(encoding="utf-8")
    return decision_revisit_assessment(
        source,
        source_sha256=hashlib.sha256(source.encode("utf-8")).hexdigest(),
        source_path=path.relative_to(ROOT).as_posix(),
    )


def failed_decision_revisit_assessment(path: Path = DECISION_REGISTER) -> dict[str, Any]:
    try:
        source_hash = sha256(path)
    except OSError:
        source_hash = None
    return {
        "decision_revisit_trigger_assessment": FAILED_REVISIT_ASSESSMENT,
        "active_decision_revisit_triggers": None,
        "decision_revisit_trigger_source": {
            "kind": "accepted_decision_register",
            "path": DECISION_REGISTER_PATH,
            "content_sha256": source_hash,
            "assessed_decision_ids": [],
            "assessed_decision_count": 0,
        },
    }


def make_v11_result(
    *,
    validated_production_head: str,
    final_gate_artifact: str,
    duration_ms: float,
    repositories: list[dict[str, Any]],
    revisit_assessment: dict[str, Any],
) -> dict[str, Any]:
    statuses = [
        value["status"]
        for repository in repositories
        for value in repository.get("steps", {}).values()
    ]
    steps_passed = bool(statuses and set(statuses) == {"passed"})
    assessment_completed = (
        revisit_assessment.get("decision_revisit_trigger_assessment")
        == OFFICIAL_REVISIT_ASSESSMENT
        and isinstance(revisit_assessment.get("active_decision_revisit_triggers"), list)
    )
    no_active_triggers = revisit_assessment.get("active_decision_revisit_triggers") == []
    phase_8_ready = steps_passed and assessment_completed and no_active_triggers
    result = {
        "schema_version": 1,
        "validation_id": "V11",
        "validated_production_head": validated_production_head,
        "final_gate_artifact": final_gate_artifact,
        "duration_ms": duration_ms,
        "repositories": repositories,
        "counts": {status: statuses.count(status) for status in sorted(ALLOWED_STATUS)},
        "status": "passed" if phase_8_ready else "failed",
        **revisit_assessment,
        "phase_8_ready": phase_8_ready,
    }
    validate_result(result)
    return result


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


def contains_hangul(value: str) -> bool:
    return any("\uac00" <= character <= "\ud7a3" for character in value)


def viewer_project_understanding_evidence(
    snapshot: Path,
    understanding: dict[str, Any] | None,
) -> dict[str, Any]:
    """Inspect readable, grounded Viewer content without retaining the HTML body."""

    if not snapshot.is_file():
        return {
            "status": "failed",
            "checks": {"snapshot_available": False},
            "entity_count": 0,
            "explanation_count": 0,
            "diagram_count": 0,
            "grounded_relation_count": 0,
        }
    content = snapshot.read_text(encoding="utf-8")
    node_ids = set(re.findall(
        r'<g class="diagram-node" data-entity-id="([^"]+)"',
        content,
    ))
    relations = re.findall(
        r'<g class="diagram-edge" data-relation-id="([^"]+)" '
        r'data-relation-class="([^"]+)" data-source-entity="([^"]+)" '
        r'data-target-entity="([^"]+)"',
        content,
    )
    explanations = re.findall(
        r'<article class="deterministic-derived explanation-item"[^>]*>'
        r'<p>([^<]+)</p>',
        content,
    )
    repository_entities = (
        understanding.get("repository_map", {}).get("entities", [])
        if isinstance(understanding, dict)
        else []
    )
    named_entities = [
        entity.get("name")
        for entity in repository_entities
        if isinstance(entity, dict) and isinstance(entity.get("name"), str)
    ]
    grounded_relations = [
        relation
        for relation in relations
        if relation[0]
        and relation[2] in node_ids
        and relation[3] in node_ids
    ]
    checks = {
        "snapshot_available": True,
        "project_understanding_heading": (
            "Project Understanding" in content
            and "How the architecture and code connect" in content
        ),
        "readable_repository_entity": any(
            len(name.strip()) >= 2 and html.escape(name, quote=True) in content
            for name in named_entities
        ),
        "readable_grounded_explanation": any(
            len(explanation.strip()) >= 24 for explanation in explanations
        ),
        "fact_interpretation_distinction": all(
            marker in content
            for marker in (
                'data-statement-role="verified-fact"',
                'data-statement-role="deterministic-derived"',
                'data-statement-role="generated-interpretation"',
            )
        ),
        "grounded_architecture_diagram": (
            'data-diagram="architecture-topology"' in content
            and bool(node_ids)
            and bool(grounded_relations)
        ),
        "grounded_flow_diagram": 'data-diagram="flow-topology"' in content,
        "inspectable_explanation_basis": (
            'class="explanation-evidence"' in content
            and 'data-relation-id="' in content
        ),
    }
    return {
        "status": "passed" if all(checks.values()) else "failed",
        "checks": checks,
        "entity_count": len(node_ids),
        "explanation_count": len(explanations),
        "diagram_count": content.count('<figure class="grounded-diagram"'),
        "grounded_relation_count": len(grounded_relations),
    }


def cli_json(
    recorder: Recorder,
    label: str,
    cli: Path,
    env: dict[str, str],
    *args: str,
    runtime: Path | None = None,
    cwd: Path = ROOT,
) -> tuple[dict[str, Any] | None, dict[str, Any]]:
    argv = [str(cli), "--json"]
    if runtime is not None:
        argv += ["--runtime", str(runtime)]
    result = recorder.run(label, argv + list(args), env, cwd=cwd)
    if result["exit_code"] != 0:
        return None, result
    try:
        value = json.loads(decoded(result))
    except json.JSONDecodeError:
        return None, result
    return value, result


def qualify_candidate_dependency_failures(
    recorder: Recorder,
    cli: Path,
    mcp_binary: Path,
    env: dict[str, str],
    runtime: Path,
    project_id: str,
) -> tuple[bool, list[dict[str, Any]]]:
    path = runtime / "candidates.sqlite3"
    connection = sqlite3.connect(path)
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    connection.close()
    baseline = path.read_bytes()
    sidecars = [Path(f"{path}-wal"), Path(f"{path}-shm"), Path(f"{path}-journal")]

    def restore() -> None:
        if path.is_dir():
            path.rmdir()
        elif path.exists():
            path.unlink()
        path.write_bytes(baseline)
        path.chmod(0o600)
        for sidecar in sidecars:
            if sidecar.is_file():
                sidecar.unlink()

    observations: list[dict[str, Any]] = []
    try:
        for fault, expected in (
            ("unsupported", "unsupported"),
            ("corrupt", "corrupt"),
            ("unavailable", "unavailable"),
        ):
            restore()
            if fault == "unsupported":
                with sqlite3.connect(path) as connection:
                    connection.execute(
                        "UPDATE metadata SET value = '999' WHERE key = 'schema_version'"
                    )
            elif fault == "corrupt":
                with sqlite3.connect(path) as connection:
                    connection.execute("DROP TABLE candidates")
            else:
                path.unlink()
                path.mkdir()
            cli_result, cli_operation = cli_json(
                recorder, f"candidate-{fault}-cli", cli, env,
                "--project", project_id, "advanced", "candidates"
            )
            canonical, canonical_operation = cli_json(
                recorder, f"candidate-{fault}-canonical", cli, env,
                "--project", project_id, "advanced", "records", "list",
            )
            host = None
            cleanup: dict[str, Any] = {}
            try:
                host = Mcp(mcp_binary, env)
                host.initialize()
                mcp_result, mcp_ok = host.tool(
                    "candidate_inspect", {"project_id": project_id}
                )
            except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
                mcp_result, mcp_ok = {"error": str(error)}, False
            finally:
                if host is not None:
                    cleanup = host.close()
            issue_preserved = any(
                issue.get("scope") == "candidate_inspection"
                and expected in issue.get("kind", "")
                for issue in (mcp_result or {}).get("issues", [])
            )
            passed = all((
                cli_result is not None and cli_result.get("health") == "degraded",
                cli_result is not None and cli_result.get("candidates") == [],
                canonical is not None and bool(canonical.get("records")),
                mcp_ok,
                mcp_result is not None and mcp_result.get("health") == expected,
                mcp_result is not None and mcp_result.get("candidates") == [],
                issue_preserved,
                cleanup.get("exit_code") == 0,
            ))
            observations.append({
                "fault": fault,
                "status": "passed" if passed else "failed",
                "cli": cli_result,
                "cli_operation": cli_operation,
                "canonical_usable": canonical is not None and bool(canonical.get("records")),
                "canonical_operation": canonical_operation,
                "mcp": mcp_result,
                "cleanup": cleanup,
            })
    finally:
        restore()
    return all(item["status"] == "passed" for item in observations), observations


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
    lifecycle_state: str | None = None,
) -> dict[str, Any] | None:
    records = inspection.get("records", []) if inspection else []
    return next(
        (
            record
            for record in records
            if record.get("kind") == kind
            and (lifecycle_state is None or record.get("lifecycle_state") == lifecycle_state)
        ),
        None,
    )


def candidate_repository_source_basis(
    analysis: dict[str, Any] | None,
) -> list[str]:
    source_id = analysis.get("repository_source_id") if analysis else None
    return [source_id] if isinstance(source_id, str) and source_id else []


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
        "This is a bounded MCP connectivity probe, not repository work. Call only this "
        "repository's Volicord MCP server project_health tool for Project "
        f"{project_id}. Do not call project_resolve, Recall, context_record, "
        "repository_analyze, another tool, or the shell. Report the returned connection "
        "and capability state."
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
                    codex, "--dangerously-bypass-hook-trust", "--ask-for-approval", "never", "--config",
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
        "install", [str(INSTALLER), "--prefix", str(prefix), "--runtime-dir", str(runtime)], env
    )
    cli = prefix / "bin/volicord"
    mcp_binary = prefix / "bin/volicord-mcp"
    installation_only = install["exit_code"] == 0 and not (codex_home / "config.toml").exists()
    activation = recorder.run(
        "codex-enable",
        [str(cli), "--repository", str(repository), "codex", "enable"],
        env,
        cwd=repository,
    ) if installation_only else {"exit_code": None}
    codex_home.joinpath("config.toml").write_text(
        f'[projects.{json.dumps(str(repository.resolve()))}]\ntrust_level = "trusted"\n',
        encoding="utf-8",
    )
    installed = installation_only and activation["exit_code"] == 0 and all(
        path.is_file() and path.stat().st_mode & stat.S_IXUSR
        for path in (cli, prefix / "bin/volicord-viewer", mcp_binary)
    )
    steps["clean_install"] = step(
        "passed" if installed else "failed",
        "isolated replacement install completed" if installed else "isolated install failed",
        operation=install,
        repository_activation=activation,
        installation_created_global_registration=not installation_only,
    )
    if not installed:
        for name in REQUIRED_STEPS[1:]:
            steps[name] = step("skipped", "prerequisite clean installation failed")
        return {"class": target_kind, "identity": identity, "steps": steps}

    initialized, init_op = cli_json(
        recorder, "project-init", cli, env, "init", f"V11 {target_kind}", cwd=repository
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

    status_before, status_before_op = cli_json(
        recorder, "project-understanding-before-analysis", cli, env, "status", cwd=repository
    )
    analysis, analysis_op = cli_json(
        recorder, "analyze", cli, env, "analyze", cwd=repository
    )
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
    try:
        language_host = Mcp(mcp_binary, env)
        language_host.initialize()
        language_plan, language_plan_ok = language_host.tool(
            "document_preview",
            {
                "project_id": project_id,
                "kind": "handoff-resume",
                "format": "markdown",
                "language": "ko-KR",
                "locale": "en",
            },
        )
        plan = (language_plan or {}).get("plan", {})
        realization = {
            "plan_fingerprint": plan.get("plan_fingerprint"),
            "title": "프로젝트 인수인계와 작업 재개",
            "sections": [
                {
                    "key": section.get("key"),
                    "title": f"한국어 설명: {section.get('source_title', '')}",
                    "claims": [
                        {
                            "identity": claim.get("identity"),
                            "text": f"한국어 설명: {claim.get('source_text', '')}",
                        }
                        for claim in section.get("claims", [])
                    ],
                }
                for section in plan.get("sections", [])
            ],
            "generator": {
                "generator": "v11-current-host",
                "agent": "codex",
                "model": "deterministic-v11-realizer",
            },
        }
        realized_document, realized_ok = language_host.tool(
            "document_preview",
            {
                "project_id": project_id,
                "kind": "handoff-resume",
                "format": "markdown",
                "language": "ko-KR",
                "locale": "en",
                "realization": realization,
            },
        )
        language_cleanup = language_host.close()
        language_ok = bool(
            language_plan_ok
            and language_plan
            and language_plan.get("outcome") == "realization_required"
            and realized_ok
            and realized_document
            and realized_document.get("outcome") == "realized"
            and realized_document.get("requested_language") == "ko-KR"
            and contains_hangul(realized_document.get("content", ""))
        )
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        language_plan = realized_document = {"error": str(error)}
        language_cleanup = {}
        language_ok = False
    viewer_snapshot = target_root / "project-understanding.html"
    viewer_result, viewer_operation = cli_json(
        recorder,
        "project-understanding-viewer-snapshot",
        cli,
        env,
        "viewer",
        "export",
        "--output",
        str(viewer_snapshot),
        "--level",
        "working",
        "--language",
        "en",
        cwd=repository,
    )
    viewer_understanding = viewer_project_understanding_evidence(
        viewer_snapshot,
        understanding if isinstance(understanding, dict) else None,
    )
    status_ok = bool(
        status_before
        and status_before.get("operation") == "project_status"
        and isinstance(status_before.get("architecture"), dict)
        and isinstance(status_before.get("evidence"), dict)
    )
    steps["source_grounded_understanding"] = step(
        "passed"
        if understanding_ok
        and understanding
        and understanding.get("repository_map", {}).get("entity_count", 0) > 0
        and status_ok
        and viewer_result
        and viewer_snapshot.is_file()
        and viewer_understanding["status"] == "passed"
        and language_ok
        else "failed",
        "task-oriented CLI status, MCP understanding, and the Viewer snapshot exposed grounded Project Understanding",
        cli_status=status_before,
        cli_status_operation=status_before_op,
        mcp_result=understanding,
        cleanup=understanding_cleanup,
        viewer_result=viewer_result,
        viewer_operation=viewer_operation,
        viewer_understanding=viewer_understanding,
        requested_language_plan=language_plan,
        requested_language_realization=realized_document,
        requested_language_cleanup=language_cleanup,
    )

    candidate_tools = {
        "candidate_inspect", "candidate_manage", "inquiry_frontier", "decision_record",
        "canonical_inspect", "context_record", "repository_analyze",
        "engineering_choice_discovery", "materiality_review", "learning_deliberation",
        "checkpoint_record",
    }
    candidate_evidence: dict[str, Any] = {}
    inquiry_evidence: dict[str, Any] = {}
    candidate_status = "failed"
    inquiry_status = "failed"
    decision_id = None
    decision_revision = None
    decision_source_id = None
    ordinary = repository / "v11-ordinary-work.txt"
    ordinary_evidence: dict[str, Any] = {}
    ordinary_status = "failed"
    checkpoint_value: dict[str, Any] | None = None
    checkpoint_evidence: dict[str, Any] = {}
    checkpoint_status = "failed"
    checkpoint_next_step = f"Resume the {target_kind} V11 journey in a new session"
    checkpoint_target = "next Codex session"
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
            learning_active = target_kind == "small-python"
            technical_delegated = target_kind == "polyglot-medium"
            goal_text = (
                f"Teach me through the meaningful technical fork while rehearsing {target_kind} through the resolved self-guiding work-authority path"
                if learning_active
                else f"Rehearse {target_kind} through the resolved self-guiding work-authority path; I delegate the internal technical state representation to you"
                if technical_delegated
                else f"Rehearse {target_kind} through the resolved self-guiding work-authority path"
            )
            goal, goal_ok = host.tool("context_record", {
                "project_id": project_id,
                "user_turn": goal_text,
                "role": "goal",
                "statement": goal_text,
            })
            candidate_analysis, candidate_analysis_ok = host.tool(
                "repository_analyze", {"project_id": project_id, "excluded_paths": []}
            )
            source_ids = candidate_repository_source_basis(candidate_analysis)
            source_id = source_ids[0] if source_ids else None
            materiality_dimension_id = "project-context-boundary"
            technical_dimension_id = "technical-state-representation"
            discovery, discovery_ok = host.tool("engineering_choice_discovery", {
                "project_id": project_id,
                "goal_context_id": (goal or {}).get("context_item_id"),
                "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                    "analysis_snapshot_id"
                ),
                "source_operation": "V11 installed MCP engineering-choice discovery",
                "summary": "Discover durable context authority and an independent technical representation fork",
                "choices": [
                    {
                        "choice_id": materiality_dimension_id,
                        "summary": "Choose how this Project preserves its durable context boundary",
                        "affected_scope": ["project-context"],
                        "alternatives": [
                            {"alternative_id": "local", "summary": "Keep canonical context local", "technical_consequences": ["Canonical context remains locally controlled"]},
                            {"alternative_id": "remote", "summary": "Use provider-backed canonical context", "technical_consequences": ["Canonical behavior would depend on a separately authorized provider boundary"]},
                        ],
                        "technical_consequences": ["The outcome changes durable local versus provider-backed behavior"],
                        "source_ids": [source_id] if source_id else [],
                        "effect_categories": ["persistence_or_lifetime", "privacy_or_disclosure"],
                        "relationship": {"state": "independent"},
                        "evidence_state": "sufficient",
                    },
                    {
                        "choice_id": technical_dimension_id,
                        "summary": "Represent bounded state as ordered records or a keyed index",
                        "affected_scope": ["internal-state"],
                        "alternatives": [
                            {"alternative_id": "ordered-records", "summary": "Use ordered records", "technical_consequences": ["Simple deterministic iteration with bounded lookup"]},
                            {"alternative_id": "keyed-index", "summary": "Use a keyed index", "technical_consequences": ["Direct lookup with additional ordering and synchronization obligations"]},
                        ],
                        "technical_consequences": ["The representation changes invariant placement and maintenance cost"],
                        "source_ids": [source_id] if source_id else [],
                        "effect_categories": ["maintenance_or_support", "implementation_internal"],
                        "relationship": {"state": "independent"},
                        "evidence_state": "sufficient",
                    },
                ],
            })
            discovery_id = discovery.get("discovery_candidate_id") if discovery_ok and discovery else None
            learning_participation = (
                {
                    "state": "active",
                    "user_turn_source_id": (goal or {}).get("source_id"),
                    "verbatim_statement": "Teach me through the meaningful technical fork",
                }
                if learning_active
                else {"state": "inactive"}
            )
            technical_learning_value = (
                {
                    "state": "deliberation_worthy",
                    "rationale": "Invariant placement and representation costs are transferable across repositories.",
                    "consequence_significance": ["The representation determines where consistency invariants live"],
                    "transferable_principles": ["Choose representations that make invariants explicit"],
                    "non_obvious_trade_offs": ["Faster direct lookup adds ordering and synchronization obligations"],
                }
                if learning_active
                else {
                    "state": "routine",
                    "rationale": "Normal mode keeps the bounded internal representation agent-owned and non-interrupting.",
                }
            )
            review, review_ok = host.tool("materiality_review", {
                "action": "record",
                "project_id": project_id,
                "engineering_choice_discovery_candidate_id": discovery_id,
                "rationale": "The durable Project context boundary requires explicit user authority.",
                "learning_participation": learning_participation,
                "judgments": [
                    {
                        "choice_id": materiality_dimension_id,
                        "disposition": "unresolved_user_owned_outcome",
                        "basis_summary": "Repository evidence establishes the boundary but cannot choose it",
                        "learning_value": {"state": "routine", "rationale": "User authority uses Inquiry even when learning is active."},
                    },
                    {
                        "choice_id": technical_dimension_id,
                        "disposition": (
                            "delegated_implementation_choice"
                            if technical_delegated
                            else "agent_owned_implementation_choice"
                        ),
                        "basis_summary": "The representation is explicitly delegated in this Goal and does not change the user-owned context boundary."
                        if technical_delegated
                        else "The representation is agent-owned and does not change the user-owned context boundary.",
                        **(
                            {
                                "delegation_statement": "I delegate the internal technical state representation to you",
                                "delegated_scope": ["internal-state"],
                            }
                            if technical_delegated
                            else {}
                        ),
                        "learning_value": technical_learning_value,
                    },
                ],
            }) if discovery_id else (None, False)
            review_candidate_id = (
                review.get("review_candidate_id") if review_ok and review else None
            )
            review_readiness = review
            review_readiness_ok = review_ok
            if technical_delegated and review_candidate_id:
                review_readiness, review_readiness_ok = host.tool("materiality_review", {
                    "action": "inspect",
                    "project_id": project_id,
                    "goal_context_id": (goal or {}).get("context_item_id"),
                    "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                        "analysis_snapshot_id"
                    ),
                    "work_contexts": ["internal-state"],
                })
            candidate_research_analysis, candidate_research_analysis_ok = host.tool(
                "repository_analyze", {"project_id": project_id, "excluded_paths": []}
            )
            research_source_ids = candidate_repository_source_basis(
                candidate_research_analysis
            )
            research_source_id = (
                research_source_ids[0] if research_source_ids else None
            )
            submitted, submitted_ok = host.tool("candidate_manage", {
                "action": "submit_question_from_materiality",
                "project_id": project_id,
                "review_candidate_id": review_candidate_id,
                "dimension_id": materiality_dimension_id,
                "research_state": "research_required",
                "research_state_basis": "repository structure must be inspected before asking for a user judgment",
                "retention_basis": "retain through the explicit V11 inquiry disposition",
                "bounded_summary": "Choose how this Project should preserve its local context boundary",
                "prompt": "Which context boundary should this Project use?",
                "why_now": "the integrated journey needs one material current-host Decision",
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
                "duplicate_basis": "canonical inspection found no matching Question",
                "presentation_order": 1,
            }) if review_candidate_id else (None, False)
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
                "source_ids": [research_source_id] if research_source_id else [],
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
                "source_ids": [research_source_id] if research_source_id else [],
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
            resolved_review, resolved_review_ok = host.tool("materiality_review", {
                "action": "revise",
                "project_id": project_id,
                "review_candidate_id": review_candidate_id,
                "rationale": "The explicit current-host Decision resolves the Project context boundary.",
                "learning_participation": learning_participation,
                "judgments": [
                    {
                        "choice_id": materiality_dimension_id,
                        "disposition": "unresolved_user_owned_outcome",
                        "resolution_decision_id": decision_id,
                        "basis_summary": "The explicit current-host Decision supplies current authority",
                        "learning_value": {"state": "routine", "rationale": "The user-owned outcome remains on the canonical Inquiry path."},
                    },
                    {
                        "choice_id": technical_dimension_id,
                        "disposition": (
                            "delegated_implementation_choice"
                            if technical_delegated
                            else "agent_owned_implementation_choice"
                        ),
                        "basis_summary": "The representation remains explicitly delegated independently from the canonical Decision."
                        if technical_delegated
                        else "The representation remains agent-owned independently from the canonical Decision.",
                        **(
                            {
                                "delegation_statement": "I delegate the internal technical state representation to you",
                                "delegated_scope": ["internal-state"],
                            }
                            if technical_delegated
                            else {}
                        ),
                        "learning_value": technical_learning_value,
                    },
                ],
            }) if review_candidate_id and decision_id else (None, False)
            resolved_review_readiness = resolved_review
            resolved_review_readiness_ok = resolved_review_ok
            if technical_delegated and resolved_review_ok and resolved_review:
                resolved_review_readiness, resolved_review_readiness_ok = host.tool(
                    "materiality_review",
                    {
                        "action": "inspect",
                        "project_id": project_id,
                        "goal_context_id": (goal or {}).get("context_item_id"),
                        "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                            "analysis_snapshot_id"
                        ),
                        "work_contexts": ["internal-state"],
                    },
                )
            learning_evidence: dict[str, Any] = {"active": learning_active}
            work_ready_review = resolved_review_readiness
            work_ready_review_ok = resolved_review_readiness_ok
            if learning_active and resolved_review_ok and resolved_review:
                begun, begun_ok = host.tool("learning_deliberation", {
                    "action": "begin",
                    "project_id": project_id,
                    "review_candidate_id": review_candidate_id,
                    "dimension_id": technical_dimension_id,
                    "source_operation": "V11 installed MCP learning deliberation",
                    "problem": "Which state representation makes the invariant easiest to preserve?",
                    "established_facts": ["The state set is bounded and deterministic ordering is required"],
                })
                deliberation_id = begun.get("deliberation_candidate_id") if begun_ok and begun else None
                responded, responded_ok = host.tool("learning_deliberation", {
                    "action": "respond_select",
                    "project_id": project_id,
                    "deliberation_candidate_id": deliberation_id,
                    "user_turn": "Choose ordered records because deterministic invariant inspection matters more than direct lookup.",
                    "user_rationale": "Deterministic invariant inspection matters more than direct lookup.",
                    "selections": [{"choice_id": technical_dimension_id, "alternative_id": "ordered-records"}],
                }) if deliberation_id else (None, False)
                feedback, feedback_ok = host.tool("learning_deliberation", {
                    "action": "feedback",
                    "project_id": project_id,
                    "deliberation_candidate_id": deliberation_id,
                    "feedback": "Ordered records keep deterministic inspection explicit; bounded lookup avoids making the linear scan operationally significant.",
                    "recommendation_selections": [{"choice_id": technical_dimension_id, "alternative_id": "ordered-records"}],
                    "recommendation_rationale": "The bounded size makes direct indexing unnecessary while deterministic order supports auditing.",
                }) if responded_ok else (None, False)
                completed, completed_ok = host.tool("learning_deliberation", {
                    "action": "complete",
                    "project_id": project_id,
                    "deliberation_candidate_id": deliberation_id,
                }) if feedback_ok else (None, False)
                work_ready_review = completed
                work_ready_review_ok = completed_ok
                learning_evidence = {
                    "active": True,
                    "begin": begun,
                    "response": responded,
                    "feedback": feedback,
                    "completion": completed,
                    "ordered": bool(
                        begun_ok
                        and begun
                        and begun.get("state", {}).get("state") == "awaiting_initial_response"
                        and not begun.get("rounds")
                        and responded_ok
                        and responded
                        and responded.get("state", {}).get("state") == "awaiting_agent_feedback"
                        and feedback_ok
                        and feedback
                        and feedback.get("state", {}).get("state") == "feedback_provided"
                        and completed_ok
                        and completed
                        and completed.get("state", {}).get("state") == "completed"
                        and completed.get("canonical_decision") is False
                    ),
                }
            canonical_after_learning, canonical_after_learning_ok = host.tool(
                "canonical_inspect", {"project_id": project_id}
            )
            decision_records_after_learning = [
                record
                for record in (canonical_after_learning or {}).get("records", [])
                if record.get("kind") == "decision"
            ]
            learning_evidence["canonical_decision_count"] = len(
                decision_records_after_learning
            )
            guarded_before = sha256(runtime / "guarded.sqlite3")
            if (
                work_ready_review_ok
                and work_ready_review
                and work_ready_review.get("workflow", {}).get("stage") == "ready_for_work"
                and work_ready_review.get("workflow", {}).get("blocks_ordinary_work") is False
            ):
                ordinary.write_text(
                    "ordinary repository work requires no Guarded confirmation\n",
                    encoding="utf-8",
                )
            guarded_after = sha256(runtime / "guarded.sqlite3")
            ordinary_status = (
                "passed"
                if ordinary.is_file() and guarded_before == guarded_after
                else "failed"
            )
            ordinary_evidence = {
                "changed_path": str(ordinary.relative_to(repository)),
                "guarded_store_unchanged": guarded_before == guarded_after,
                "resolved_materiality_review": resolved_review,
                "resolved_materiality_review_readiness": resolved_review_readiness,
                "learning_deliberation": learning_evidence,
            }
            checkpoint_value, checkpoint_ok = host.tool("checkpoint_record", {
                "project_id": project_id,
                "goal_context_id": (goal or {}).get("context_item_id"),
                "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                    "analysis_snapshot_id"
                ),
                "kind": "handoff",
                "work_state": "paused",
                "applied_decision_ids": [decision_id] if decision_id else [],
                "work_contexts": ["internal-state"] if technical_delegated else [],
                "verification": [{"state": "not_run"}],
                "next_step": checkpoint_next_step,
                "known_limits": [
                    "The configured external provider is intentionally unavailable"
                ],
                "handoff_to": checkpoint_target,
            }) if ordinary_status == "passed" else (None, False)
            checkpoint_status = (
                "passed"
                if checkpoint_ok
                and checkpoint_value
                and checkpoint_value.get("workflow", {}).get("disposition")
                == "checkpoint_recorded"
                and checkpoint_value.get("changed_paths")
                == [str(ordinary.relative_to(repository))]
                else "failed"
            )
            checkpoint_evidence = {
                "goal": goal,
                "baseline": candidate_analysis,
                "materiality_review": review,
                "materiality_review_readiness": review_readiness,
                "resolved_materiality_review": resolved_review,
                "resolved_materiality_review_readiness": resolved_review_readiness,
                "decision_source_id": decision_source_id,
                "handoff_target": checkpoint_target,
                "checkpoint": checkpoint_value,
                "mcp_call_succeeded": checkpoint_ok,
            }
            submitted_candidate = next(
                (item for item in (after_submission or {}).get("candidates", []) if item.get("identity") == candidate_id),
                None,
            )
            ready_candidate = next(
                (item for item in (ready_inspection or {}).get("candidates", []) if item.get("identity") == candidate_id),
                None,
            )
            candidate_ok = all([
                canonical_before_ok,
                goal_ok,
                goal,
                (goal or {}).get("workflow", {}).get("stage")
                == "repository_baseline",
                candidate_analysis_ok,
                source_id,
                discovery_ok,
                discovery_id,
                discovery
                and discovery.get("workflow", {}).get("stage")
                == "materiality_review",
                review_ok,
                review,
                review_readiness_ok,
                review_readiness,
                (review_readiness or {}).get("workflow", {}).get("stage")
                == "question_candidate",
                (review_readiness or {}).get("workflow", {}).get("required_next_action")
                == {
                    "tool": "candidate_manage",
                    "action": "submit_question_from_materiality",
                },
                candidate_research_analysis_ok,
                research_source_id,
                submitted_ok,
                submitted and submitted.get("research_state") == "research_required",
                submitted and submitted.get("review_candidate_id") == review_candidate_id,
                submitted and submitted.get("dimension_id") == materiality_dimension_id,
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
                resolved_review_ok,
                resolved_review,
                resolved_review_readiness_ok,
                resolved_review_readiness,
                (resolved_review_readiness or {}).get("workflow", {}).get("stage")
                == ("learning_deliberation" if learning_active else "ready_for_work"),
                (not learning_active or learning_evidence.get("ordered") is True),
                work_ready_review_ok,
                work_ready_review,
                (work_ready_review or {}).get("workflow", {}).get("stage")
                == "ready_for_work",
                canonical_after_learning_ok,
                len(decision_records_after_learning) == 1,
            ])
            candidate_status = "passed" if candidate_ok else "failed"
            inquiry_status = "passed" if inquiry_ok else "failed"
            candidate_evidence = {
                "repository_analysis": candidate_analysis,
                "repository_source_id": source_id,
                "engineering_choice_discovery": discovery,
                "candidate_research_analysis": candidate_research_analysis,
                "candidate_research_source_id": research_source_id,
                "goal": goal,
                "materiality_review": review,
                "materiality_review_readiness": review_readiness,
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
                "resolved_materiality_review": resolved_review,
                "resolved_materiality_review_readiness": resolved_review_readiness,
                "learning_deliberation": learning_evidence,
                "canonical_after_learning": canonical_after_learning,
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

    steps["ordinary_work"] = step(
        ordinary_status,
        "ordinary repository work completed without Guarded ceremony",
        **ordinary_evidence,
    )

    provider_source_path = PROVIDER_SOURCE_PATHS[target_kind]
    provider_opt_in_source, provider_source_op = cli_json(
        recorder, "provider-opt-in-source", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "cli", "--session", "cli", "--text",
        "Enable the bounded V11 background semantic provider scope",
    )
    provider_opt_in, provider_opt_in_op = (None, {"exit_code": None})
    if provider_opt_in_source:
        provider_opt_in, provider_opt_in_op = cli_json(
            recorder, "provider-opt-in", cli, env, "--project", project_id,
            "privacy", "enable", "v11-unavailable-provider", "v11-model",
            "--source", provider_opt_in_source["identity"], "--scope", provider_source_path,
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

    steps["checkpoint"] = step(
        checkpoint_status,
        "the exact Goal, pre-work Analysis Snapshot, resolved Materiality Review, and explicit Decision grounded a Handoff Checkpoint",
        **checkpoint_evidence,
    )
    recall_before, recall_op = cli_json(
        recorder, "recall", cli, env, "recall", cwd=repository
    )
    try:
        restarted = Mcp(mcp_binary, env)
        restarted.initialize()
        recall_after, recall_after_ok = restarted.tool("recall", {"project_id": project_id})
        restart_cleanup = restarted.close()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        recall_after, recall_after_ok, restart_cleanup = {"error": str(error)}, False, {}
    recalled_learning = (recall_after or {}).get("learning_context", [])
    learning_recall_ok = (
        any(
            item.get("learning_deliberation", {}).get("state", {}).get("state")
            == "completed"
            and item.get("learning_deliberation", {}).get("canonical_decision") is False
            for item in recalled_learning
        )
        if target_kind == "small-python"
        else not recalled_learning
    )
    restart_ok = all([
        checkpoint_value,
        recall_before,
        recall_before.get("active_decision_count", 0) >= 1,
        recall_before.get("next_step") == checkpoint_next_step,
        recall_after_ok,
        recall_after,
        len(recall_after.get("decisions", [])) >= 1,
        recall_after.get("next_step") == checkpoint_next_step,
        recall_after.get("learning_context_health", {}).get("state") == "available",
        learning_recall_ok,
    ])
    steps["restart_recall"] = step(
        "passed" if restart_ok else "failed",
        "a new MCP process recovered the integrated Decision, bounded learning context, and explicit Handoff next step",
        cli_recall=recall_before, cli_operation=recall_op, restarted_recall=recall_after, cleanup=restart_cleanup,
    )

    base_bundle = target_root / "base.volicord.json"
    exported, export_op = cli_json(
        recorder, "bundle-export", cli, env, "context", "export", "--output", str(base_bundle), cwd=repository
    )
    clone_result = recorder.run("clone-target", ["git", "clone", "--quiet", "--no-hardlinks", str(repository), str(clone)], env)
    imported, import_op = cli_json(
        recorder, "bundle-import", cli, env, "context", "import", "--input", str(base_bundle), runtime=clone_runtime
    )
    bound, bind_op = (None, {"exit_code": None})
    if imported:
        bound, bind_op = cli_json(
            recorder, "clone-bind", cli, env, "--project", project_id,
            "--repository", str(clone), "bind", runtime=clone_runtime
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
            recorder, "diverge-a-source", cli, env, "--project", project_id,
            "advanced", "records", "source", "--host", "codex", "--session", "clone-a",
            "--text", "Choose the remote branch in clone A",
        )
        source_b, source_b_op = cli_json(
            recorder, "diverge-b-source", cli, env, "--project", project_id,
            "advanced", "records", "source", "--host", "codex", "--session", "clone-b",
            "--text", "Retain the local branch in clone B", runtime=clone_runtime,
        )
        conflict_operations.extend([source_a_op, source_b_op])
        if source_a:
            local_decision, local_decision_op = cli_json(
                recorder, "diverge-a-decision", cli, env, "--project", project_id,
                "advanced", "records", "supersede-decision", decision_id,
                "--source", source_a["identity"], "--alternative", "remote",
                "--rationale", "Clone A chooses remote augmentation",
            )
            conflict_operations.append(local_decision_op)
        if source_b:
            incoming_decision, incoming_decision_op = cli_json(
                recorder, "diverge-b-decision", cli, env, "--project", project_id,
                "advanced", "records", "supersede-decision", decision_id,
                "--source", source_b["identity"], "--alternative", "local",
                "--rationale", "Clone B retains local context",
                runtime=clone_runtime,
            )
            conflict_operations.append(incoming_decision_op)
        bundle_a = target_root / "a.volicord.json"
        bundle_b = target_root / "b.volicord.json"
        bundle_a_value, bundle_a_op = cli_json(
            recorder, "diverge-a-export", cli, env, "--project", project_id,
            "context", "export", "--output", str(bundle_a)
        )
        bundle_b_value, bundle_b_op = cli_json(
            recorder, "diverge-b-export", cli, env, "--project", project_id,
            "context", "export", "--output", str(bundle_b),
            runtime=clone_runtime,
        )
        conflict_operations.extend([bundle_a_op, bundle_b_op])
        if bundle_a_value and bundle_b_value:
            comparison, comparison_op = cli_json(
                recorder, "divergent-compare", cli, env, "context", "compare",
                "--input", str(bundle_b), "--base", str(base_bundle),
            )
            conflict_operations.append(comparison_op)
            if comparison and source_a:
                resolution, resolution_op = cli_json(
                    recorder, "divergent-resolution", cli, env, "context", "resolve",
                    "--input", str(bundle_b), "--conflict-set", comparison["conflict_set_identity"],
                    "--revision", str(comparison["conflict_revision"]),
                    "--source", source_a["identity"], "--mode", "context-branch",
                    "--base", str(base_bundle),
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
        recorder, "correction-authorization", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11-correction",
        "--text", "Correct the integrated Decision rationale",
    )
    corrected = None
    correction_op = {"exit_code": None}
    if correction_authorization and local_decision:
        corrected, correction_op = cli_json(
            recorder, "correct-decision", cli, env, "--project", project_id,
            "advanced", "records", "correct-decision", local_decision["identity"],
            "--revision", str(local_decision["revision"]),
            "--source", correction_authorization["identity"],
            "--text", "Remote augmentation Clone A chooses",
        )
    supersession_authorization, supersession_source_op = cli_json(
        recorder, "supersession-authorization", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11-supersession",
        "--text", "Return the integrated Decision to the local boundary",
    )
    superseded = None
    supersession_op = {"exit_code": None}
    if supersession_authorization and local_decision and corrected:
        superseded, supersession_op = cli_json(
            recorder, "supersede-corrected-decision", cli, env, "--project", project_id,
            "advanced", "records", "supersede-decision", local_decision["identity"],
            "--source", supersession_authorization["identity"], "--alternative", "local",
            "--rationale", "Keep canonical context local after evaluating the explicit provider boundary",
        )
    deletion_authorization, deletion_source_op = cli_json(
        recorder, "deletion-authorization", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11",
        "--text", "Authorize deletion of the disposable V11 Source",
    )
    disposable_source, disposable_source_op = cli_json(
        recorder, "disposable-source", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11",
        "--text", "Disposable Source created by the integrated V11 journey",
    )
    cleanup_control_source, cleanup_control_source_op = cli_json(
        recorder, "cleanup-control-source", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11",
        "--text", "Unrelated Source that must survive the V11 forgetting journey",
    )
    cleanup_control_state = target_root / "forgetting-control-state.json"
    cleanup_control_result = target_root / "forgetting-control-result.json"
    fixture_control_command = [
        "cargo", "test", "--manifest-path", "rebuild/Cargo.toml", "-p", "volicord-operations",
        "--test", "v11_fixture_control", "--all-features", "--", "--exact",
        "seed_and_inspect_v11_forgetting_control", "--nocapture",
    ]
    if disposable_source and cleanup_control_source:
        fixture_env = env | {
            "VOLICORD_V11_RUNTIME": str(runtime),
            "VOLICORD_V11_PROJECT": project_id,
            "VOLICORD_V11_RELATED_SOURCE": disposable_source["identity"],
            "VOLICORD_V11_UNRELATED_SOURCE": cleanup_control_source["identity"],
            "VOLICORD_V11_FIXTURE_ACTION": "seed",
            "VOLICORD_V11_CONTROL_OUTPUT": str(cleanup_control_state),
        }
        cleanup_seed_op = recorder.run(
            "seed-forgetting-controls", fixture_control_command, fixture_env
        )
    else:
        fixture_env = env
        cleanup_seed_op = {"exit_code": None}
    deletion = None
    if deletion_authorization and disposable_source and cleanup_seed_op.get("exit_code") == 0:
        deletion, deletion_op = cli_json(
            recorder, "forget-source", cli, env, "--project", project_id,
            "advanced", "records", "forget", "source", disposable_source["identity"],
            "--source", deletion_authorization["identity"],
        )
    else:
        deletion_op = {"exit_code": None}
    canonical_after_mutations, canonical_after_mutations_op = cli_json(
        recorder, "canonical-after-mutations", cli, env,
        "--project", project_id, "advanced", "records", "list"
    )
    if deletion and cleanup_control_state.is_file():
        cleanup_inspect_op = recorder.run(
            "inspect-forgetting-controls",
            fixture_control_command,
            fixture_env | {
                "VOLICORD_V11_FIXTURE_ACTION": "inspect",
                "VOLICORD_V11_CONTROL_STATE": str(cleanup_control_state),
                "VOLICORD_V11_CONTROL_OUTPUT": str(cleanup_control_result),
            },
        )
        cleanup_control = (
            json.loads(cleanup_control_result.read_text(encoding="utf-8"))
            if cleanup_inspect_op["exit_code"] == 0 and cleanup_control_result.is_file()
            else None
        )
    else:
        cleanup_inspect_op = {"exit_code": None}
        cleanup_control = None
    records_after_mutations = canonical_after_mutations.get("records", []) if canonical_after_mutations else []
    mutation_operations = [
        correction_source_op, correction_op, supersession_source_op, supersession_op,
        deletion_source_op, disposable_source_op, cleanup_control_source_op, cleanup_seed_op,
        deletion_op, canonical_after_mutations_op, cleanup_inspect_op,
    ]
    mutations_ok = all([
        corrected,
        corrected and corrected.get("identity") == local_decision.get("identity") if local_decision else False,
        corrected and corrected.get("revision") == 2,
        superseded,
        superseded and superseded.get("identity") != local_decision.get("identity") if local_decision else False,
        deletion,
        deletion and deletion.get("state") == "completed",
        deletion and deletion.get("candidate_cleanup_completed") is True,
        deletion and deletion.get("managed_derived_cleanup_completed") is True,
        deletion and deletion.get("residue_verified") is True,
        canonical_after_mutations,
        disposable_source and all(record.get("identity") != disposable_source.get("identity") for record in records_after_mutations),
        cleanup_control_source and any(record.get("identity") == cleanup_control_source.get("identity") for record in records_after_mutations),
        cleanup_control and cleanup_control.get("related_candidate_absent") is True,
        cleanup_control and cleanup_control.get("related_derived_absent") is True,
        cleanup_control and cleanup_control.get("unrelated_candidate_present") is True,
        cleanup_control and cleanup_control.get("unrelated_derived_present") is True,
        canonical_record(canonical_after_mutations, "decision", lifecycle_state="active") is not None,
    ])
    mutation_status = "passed" if mutations_ok else (
        "unsupported" if unsupported_cli(*mutation_operations) else "failed"
    )
    steps["correction_supersession_deletion"] = step(
        mutation_status,
        "the integrated Decision was corrected and superseded, and public forgetting removed linked Candidate/managed Derived controls while preserving unrelated controls after restart",
        correction_authorization=correction_authorization, correction=corrected,
        supersession_authorization=supersession_authorization, supersession=superseded,
        deletion_authorization=deletion_authorization, disposable_source=disposable_source,
        unrelated_control_source=cleanup_control_source, cleanup_seed_operation=cleanup_seed_op,
        deletion=deletion, canonical_after=canonical_after_mutations,
        cleanup_restart_inspection=cleanup_control, cleanup_inspection_operation=cleanup_inspect_op,
    )

    canonical_before_docs = target_root / "before-documents.json"
    cli_json(
        recorder, "docs-before-bundle", cli, env, "context", "export", "--output", str(canonical_before_docs), cwd=repository
    )
    document_results = []
    for kind in DOCUMENT_KINDS:
        for format_name, suffix in (("markdown", "md"), ("html", "html")):
            destination = target_root / "documents" / f"{kind}.{suffix}"
            destination.parent.mkdir(parents=True, exist_ok=True)
            value, operation = cli_json(
                recorder, f"document-{kind}-{format_name}", cli, env,
                "document", "export", kind, "--format", format_name,
                "--output", str(destination), "--language", "en", cwd=repository,
            )
            document_results.append({"kind": kind, "format": format_name, "result": value, "operation": operation})
    canonical_after_docs = target_root / "after-documents.json"
    cli_json(
        recorder, "docs-after-bundle", cli, env, "context", "export", "--output", str(canonical_after_docs), cwd=repository
    )
    published_documents = all(
        item["result"]
        and item["result"].get("outcome") != "unavailable"
        and Path(item["result"]["destination"]).is_file()
        and Path(item["result"]["destination"]).stat().st_size > 0
        for item in document_results
    )
    docs_ok = (
        published_documents
        and language_ok
        and canonical_before_docs.read_bytes() == canonical_after_docs.read_bytes()
    )
    steps["document_outputs"] = step(
        "passed" if docs_ok else "failed", "all four Markdown and self-contained HTML outputs were published, and the current host realized the requested Korean body without canonical mutation",
        documents=document_results,
        published_documents=published_documents,
        requested_language_body_realized=language_ok,
        canonical_unchanged=canonical_before_docs.read_bytes() == canonical_after_docs.read_bytes(),
    )

    privacy, privacy_op = cli_json(
        recorder, "privacy-status", cli, env, "privacy", "status", cwd=repository
    )
    provider_evidence.update({"privacy": privacy, "privacy_operation": privacy_op})
    steps["provider_failure"] = step(
        provider_status,
        "the configured production adapter reported provider_unavailable without transmission while canonical inspection and local structural analysis remained usable",
        **provider_evidence,
    )

    malformed = repository / ("src/v11_broken.rs" if target_kind == "volicord" else "v11_broken.py")
    malformed.parent.mkdir(parents=True, exist_ok=True)
    malformed.write_text("fn broken( {\n" if malformed.suffix == ".rs" else "def broken(:\n", encoding="utf-8")
    parser_result, parser_op = cli_json(
        recorder, "parser-degradation", cli, env, "analyze", cwd=repository
    )
    parser_status = "passed" if parser_result and parser_result.get("state") == "partial" and parser_result.get("failed_scopes") else "failed"
    steps["parser_failure"] = step(
        parser_status, "malformed language area was analyzed and required scoped failure/partial reporting",
        result=parser_result, operation=parser_op,
    )

    stored_at = Path(parser_result["stored_at"]) if parser_result and parser_result.get("stored_at") else None
    recall_pre_recovery, _ = cli_json(
        recorder, "recovery-recall-before", cli, env, "recall", cwd=repository
    )
    if stored_at and stored_at.is_file():
        stored_at.write_bytes(b"{ controlled V11 derived corruption")
        degraded_health, health_op = cli_json(
            recorder, "corrupt-health", cli, env, "doctor", "check", cwd=repository
        )
        repaired, repair_op = cli_json(
            recorder, "derived-repair", cli, env, "doctor", "repair", cwd=repository
        )
        recall_post_recovery, _ = cli_json(
            recorder, "recovery-recall-after", cli, env, "recall", cwd=repository
        )
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

    try:
        candidate_failure_ok, candidate_failure_evidence = qualify_candidate_dependency_failures(
            recorder, cli, mcp_binary, env, runtime, project_id
        )
    except (OSError, RuntimeError, ValueError, sqlite3.Error, json.JSONDecodeError) as error:
        candidate_failure_ok = False
        candidate_failure_evidence = [{"status": "failed", "error": str(error)}]
    steps["candidate_boundary"]["evidence"]["dependency_failure_qualification"] = (
        candidate_failure_evidence
    )
    if not candidate_failure_ok:
        steps["candidate_boundary"]["status"] = "failed"
        steps["candidate_boundary"]["summary"] = (
            "Candidate lifecycle passed, but dependency failure honesty qualification failed"
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
    if [repository.get("class") for repository in repositories] != [
        "volicord", "small-python", "polyglot-medium"
    ]:
        raise AssertionError("V11 result has the wrong repository target contract")
    statuses: list[str] = []
    for repository in repositories:
        if set(repository.get("steps", {})) != set(REQUIRED_STEPS):
            raise AssertionError(f"incomplete V11 steps for {repository.get('class')}")
        for value in repository["steps"].values():
            if value.get("status") not in ALLOWED_STATUS:
                raise AssertionError("invalid per-step status")
            statuses.append(value["status"])
    expected_counts = {status: statuses.count(status) for status in sorted(ALLOWED_STATUS)}
    if result.get("counts") != expected_counts:
        raise AssertionError("V11 result status counts do not match repository steps")
    assessment = result.get("decision_revisit_trigger_assessment")
    triggers = result.get("active_decision_revisit_triggers")
    source = result.get("decision_revisit_trigger_source")
    if assessment not in {OFFICIAL_REVISIT_ASSESSMENT, FAILED_REVISIT_ASSESSMENT}:
        raise AssertionError("V11 result has an invalid revisit-trigger assessment state")
    if assessment == OFFICIAL_REVISIT_ASSESSMENT:
        if not isinstance(triggers, list) or not all(
            isinstance(value, str) and DECISION_ID.fullmatch(value) for value in triggers
        ):
            raise AssertionError("official V11 revisit triggers are not a bounded ID list")
        if len(triggers) != len(set(triggers)):
            raise AssertionError("official V11 revisit triggers are duplicated")
    elif triggers is not None:
        raise AssertionError("unassessable V11 revisit evidence must not become an empty list")
    if not isinstance(source, dict):
        raise AssertionError("V11 result is missing Decision-register source identity")
    if source.get("kind") != "accepted_decision_register" or source.get("path") != DECISION_REGISTER_PATH:
        raise AssertionError("V11 result has the wrong Decision-register source identity")
    source_hash = source.get("content_sha256")
    if source_hash is not None and not re.fullmatch(r"[0-9a-f]{64}", source_hash):
        raise AssertionError("V11 result has an invalid Decision-register digest")
    assessed_ids = source.get("assessed_decision_ids")
    if not isinstance(assessed_ids, list) or source.get("assessed_decision_count") != len(assessed_ids):
        raise AssertionError("V11 result has incomplete assessed Decision identities")
    if not all(isinstance(value, str) and DECISION_ID.fullmatch(value) for value in assessed_ids):
        raise AssertionError("V11 result has invalid assessed Decision identities")
    if len(assessed_ids) != len(set(assessed_ids)):
        raise AssertionError("V11 result repeats an assessed Decision identity")
    if assessment == OFFICIAL_REVISIT_ASSESSMENT and (
        not assessed_ids
        or source_hash is None
        or any(value not in assessed_ids for value in triggers)
    ):
        raise AssertionError("official V11 revisit evidence is not grounded in its Decision source")
    if result.get("phase_8_ready") is True and (
        result.get("status") != "passed"
        or assessment != OFFICIAL_REVISIT_ASSESSMENT
        or triggers != []
    ):
        raise AssertionError("V11 phase readiness requires a completed no-trigger assessment")
    if triggers and result.get("phase_8_ready") is not False:
        raise AssertionError("an active Decision revisit trigger cannot be Phase 8 ready")
    if (result.get("status") == "passed") != (result.get("phase_8_ready") is True):
        raise AssertionError("V11 result status and Phase 8 readiness disagree")
    if result.get("status") not in {"passed", "failed"}:
        raise AssertionError("V11 result has an invalid aggregate status")
    if result.get("status") == "passed":
        for repository in repositories:
            authenticated = (
                repository["steps"]["codex_mcp_connection"]
                .get("evidence", {})
                .get("authenticated", {})
            )
            if authenticated.get("status") != "passed":
                raise AssertionError("passed V11 result lacks authenticated target evidence")


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
            '[projects."/synthetic/repository"]\ntrust_level = "trusted"\n',
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
            "prompt = sys.argv[-1]\n"
            "if 'bounded MCP connectivity probe, not repository work' not in prompt:\n"
            "    raise SystemExit(42)\n"
            "if 'Do not call project_resolve' not in prompt or 'project_health' not in prompt:\n"
            "    raise SystemExit(43)\n"
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


def assert_candidate_repository_source_contract() -> None:
    source_id = "0f" * 16
    analysis = {
        "repository_source_id": source_id,
        "analysis_snapshot_id": "1a" * 32,
        "repository_snapshot_id": "2b" * 32,
    }
    human_facing_inspection = {
        "records": [{
            "kind": "source",
            "identity": source_id,
            "summary": "Repository snapshot at a readable revision",
        }]
    }
    summary = human_facing_inspection["records"][0]["summary"]
    if "RepositorySnapshot" in summary:
        raise AssertionError("synthetic display summary retained the old machine token")
    if candidate_repository_source_basis(analysis) != [source_id]:
        raise AssertionError("Candidate submission did not use structured repository Source identity")
    if candidate_repository_source_basis(None):
        raise AssertionError("missing analysis fabricated a repository Source identity")


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


def assert_current_materiality_review_contract(source: str) -> None:
    required_by_action = {
        "record": {
            "action",
            "project_id",
            "engineering_choice_discovery_candidate_id",
            "rationale",
            "learning_participation",
            "judgments",
        },
        "revise": {
            "action",
            "project_id",
            "review_candidate_id",
            "rationale",
            "learning_participation",
            "judgments",
        },
    }
    obsolete_judgment_fields = {
        "dimension_id",
        "discovered_choice_ids",
        "summary",
        "affected_scope",
        "material_consequences",
        "observable_signals",
        "basis",
    }
    found: set[str] = set()
    for node in ast.walk(ast.parse(source)):
        if not (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "tool"
            and len(node.args) >= 2
            and isinstance(node.args[0], ast.Constant)
            and node.args[0].value == "materiality_review"
            and isinstance(node.args[1], ast.Dict)
        ):
            continue
        payload = {
            key.value: value
            for key, value in zip(node.args[1].keys, node.args[1].values)
            if isinstance(key, ast.Constant) and isinstance(key.value, str)
        }
        action_node = payload.get("action")
        if not isinstance(action_node, ast.Constant) or action_node.value not in required_by_action:
            continue
        action = action_node.value
        found.add(action)
        payload_fields = set(payload)
        if payload_fields != required_by_action[action]:
            raise AssertionError(
                f"V11 {action} materiality payload does not match the current public contract: "
                f"{sorted(payload_fields)}"
            )
        judgments = payload["judgments"]
        if not isinstance(judgments, ast.List) or not judgments.elts:
            raise AssertionError(f"V11 {action} materiality payload lost caller-owned judgments")
        for judgment in judgments.elts:
            if not isinstance(judgment, ast.Dict):
                raise AssertionError(f"V11 {action} materiality judgment is not inspectable")
            judgment_fields = {
                key.value
                for key in judgment.keys
                if isinstance(key, ast.Constant) and isinstance(key.value, str)
            }
            if not {"choice_id", "disposition", "basis_summary", "learning_value"} <= judgment_fields:
                raise AssertionError(f"V11 {action} materiality judgment lost required authority fields")
            retained = sorted(judgment_fields & obsolete_judgment_fields)
            if retained:
                raise AssertionError(
                    f"V11 {action} materiality judgment retained discovery-owned fields: {retained}"
                )
    if found != set(required_by_action):
        raise AssertionError(f"V11 materiality journey is incomplete: {sorted(found)}")


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
    source = Path(__file__).read_text(encoding="utf-8")
    assert_current_materiality_review_contract(source)
    obsolete_pairs = {
        ("project", "init"),
        ("canonical", "user-source"),
        ("portable", "export"),
        ("documents", "export"),
        ("checkpoint", "record"),
        ("advanced", "checkpoint"),
    }
    for node in ast.walk(ast.parse(source)):
        if not (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "cli_json"
        ):
            continue
        literal_arguments = [
            argument.value
            for argument in node.args
            if isinstance(argument, ast.Constant) and isinstance(argument.value, str)
        ]
        for pair in zip(literal_arguments, literal_arguments[1:]):
            if pair in obsolete_pairs:
                raise AssertionError(f"V11 retained an obsolete CLI vector: {pair}")
    for current in (
        'argv = [str(cli), "--json"]',
        '"viewer",',
        '"document_preview",',
        '"ko-KR",',
        'contains_hangul(realized_document.get("content", ""))',
        'viewer_understanding = viewer_project_understanding_evidence(',
        'viewer_understanding["status"] == "passed"',
        'provider_request_after.get("outcome") == "provider_unavailable"',
        '"local_canonical": local_canonical',
        '"action": "submit_question_from_materiality"',
        'discovery, discovery_ok = host.tool("engineering_choice_discovery"',
        '"delegated_implementation_choice"',
        'review, review_ok = host.tool("materiality_review"',
        '"work_contexts": ["internal-state"]',
        'candidate_research_analysis, candidate_research_analysis_ok = host.tool(',
        'resolved_review, resolved_review_ok = host.tool("materiality_review"',
        'begun, begun_ok = host.tool("learning_deliberation"',
        'recall_after.get("learning_context_health", {}).get("state") == "available"',
        'checkpoint_value, checkpoint_ok = host.tool("checkpoint_record"',
    ):
        if current not in source:
            raise AssertionError(f"V11 lost a current public-journey contract: {current}")
    with tempfile.TemporaryDirectory(prefix="volicord-v11-viewer-contract-") as directory:
        viewer_contract = Path(directory) / "project-understanding.html"
        viewer_contract.write_text(
            '<!doctype html><html lang="en"><body><h1>Project Understanding</h1>'
            '<h2>How the architecture and code connect</h2>'
            '<span data-statement-role="verified-fact">Verified fact</span>'
            '<span data-statement-role="deterministic-derived">Deterministic explanation</span>'
            '<span data-statement-role="generated-interpretation">Generated interpretation</span>'
            '<p>Service</p><div class="grounded-explanations">'
            '<article class="deterministic-derived explanation-item" '
            'data-explanation-kind="component-role"><p>Service owns a grounded and readable '
            'repository component responsibility.</p><details class="explanation-evidence">'
            '<summary>Inspect evidence basis</summary></details></article></div>'
            '<figure class="grounded-diagram" data-diagram="architecture-topology">'
            '<g class="diagram-node" data-entity-id="service"></g>'
            '<g class="diagram-node" data-entity-id="client"></g>'
            '<g class="diagram-edge" data-relation-id="calls" '
            'data-relation-class="structural" data-source-entity="client" '
            'data-target-entity="service"></g></figure>'
            '<figure class="grounded-diagram" data-diagram="flow-topology"></figure>'
            '</body></html>',
            encoding="utf-8",
        )
        viewer_contract_result = viewer_project_understanding_evidence(
            viewer_contract,
            {"repository_map": {"entities": [{"name": "Service"}]}},
        )
        if viewer_contract_result["status"] != "passed":
            raise AssertionError("grounded Viewer Project Understanding did not qualify")
        viewer_contract.write_text(
            viewer_contract.read_text(encoding="utf-8").replace(
                'class="diagram-edge"',
                'class="ungrounded-edge"',
            ),
            encoding="utf-8",
        )
        if viewer_project_understanding_evidence(
            viewer_contract,
            {"repository_map": {"entities": [{"name": "Service"}]}},
        )["status"] != "failed":
            raise AssertionError("ungrounded Viewer diagram qualified")
    assert_candidate_repository_source_contract()
    assert_authenticated_codex_lifecycle()
    assert_credential_retention_audit()
    assessment = read_decision_revisit_assessment()
    if assessment["active_decision_revisit_triggers"]:
        raise AssertionError("the maintained Decision register has an active revisit trigger")
    active_source = DECISION_REGISTER.read_text(encoding="utf-8").replace(
        "- 미해결 필수 제품 질문: 없음",
        "- 미해결 필수 제품 질문: Q3",
        1,
    )
    active_assessment = decision_revisit_assessment(
        active_source,
        source_sha256=hashlib.sha256(active_source.encode("utf-8")).hexdigest(),
    )
    if active_assessment["active_decision_revisit_triggers"] != ["Q3"]:
        raise AssertionError("active Decision revisit trigger was not assessed")
    malformed_source = DECISION_REGISTER.read_text(encoding="utf-8").replace(
        "- 미해결 필수 제품 질문: 없음",
        "- 미해결 필수 제품 질문: unresolved prose",
        1,
    )
    try:
        decision_revisit_assessment(
            malformed_source,
            source_sha256=hashlib.sha256(malformed_source.encode("utf-8")).hexdigest(),
        )
    except ValueError:
        pass
    else:
        raise AssertionError("unassessable Decision revisit evidence was accepted")
    fake_repositories = [
        {"class": name, "steps": {key: step("skipped", "self-check") for key in REQUIRED_STEPS}}
        for name in ("volicord", "small-python", "polyglot-medium")
    ]
    make_v11_result(
        validated_production_head="0" * 40,
        final_gate_artifact="/synthetic/final.json",
        duration_ms=0.0,
        repositories=fake_repositories,
        revisit_assessment=assessment,
    )
    active_result = make_v11_result(
        validated_production_head="0" * 40,
        final_gate_artifact="/synthetic/final.json",
        duration_ms=0.0,
        repositories=[
            {"class": name, "steps": {key: step("passed", "self-check") for key in REQUIRED_STEPS}}
            for name in ("volicord", "small-python", "polyglot-medium")
        ],
        revisit_assessment=active_assessment,
    )
    if active_result["phase_8_ready"] is not False or active_result["status"] != "failed":
        raise AssertionError("active Decision revisit trigger did not block Phase 8")
    print(json.dumps({
        "status": "passed",
        "required_steps": len(REQUIRED_STEPS),
        "evidence_driven_steps": len(REQUIRED_STEPS),
        "required_step_policy_regressions": "passed",
        "candidate_structured_repository_source_regression": "passed",
        "self_guiding_work_authority_checkpoint_path": "passed",
        "viewer_project_understanding_contract": "passed",
        "authentication_lifecycle": "passed",
        "credential_retention_audit": "passed",
        "decision_revisit_trigger_assessment": "passed",
        "active_decision_revisit_trigger_regression": "passed",
        "unassessable_decision_revisit_regression": "passed",
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
    try:
        revisit_assessment = read_decision_revisit_assessment()
    except (OSError, ValueError):
        revisit_assessment = failed_decision_revisit_assessment()
    result = make_v11_result(
        validated_production_head=args.validated_head,
        final_gate_artifact=str(Path(args.final_artifact).resolve()),
        duration_ms=round((time.monotonic_ns() - started) / 1_000_000, 3),
        repositories=repositories,
        revisit_assessment=revisit_assessment,
    )
    write_json(output / "result.json", result)
    print(json.dumps({
        "status": result["status"], "phase_8_ready": result["phase_8_ready"],
        "result": str(output / "result.json"), "counts": result["counts"],
        "decision_revisit_trigger_assessment": result["decision_revisit_trigger_assessment"],
        "active_decision_revisit_triggers": result["active_decision_revisit_triggers"],
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
