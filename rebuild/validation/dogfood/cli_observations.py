#!/usr/bin/env python3
"""Collect immutable, candidate-bound CLI usability observations by repository class."""
from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import signal
import subprocess
import tempfile
import time
from typing import Any, Callable


MAX_STREAM_BYTES = 1024 * 1024
MAX_ARTIFACT_BYTES = 32 * 1024 * 1024
COMMAND_TIMEOUT_SECONDS = 120
CRITERION_COMMANDS = {
    "discover_with_cli_help": ["--help"],
    "status_without_project_id": ["status"],
    "analyze_without_project_id": ["analyze"],
    "recall_without_project_id": ["recall"],
    "documents_without_project_id": ["document", "preview", "handoff-resume", "--language", "en"],
    "export_without_project_id": ["context", "export", "--output", "<observation-output>"],
    "doctor_without_project_id": ["doctor", "check"],
}


def campaign_api():
    import campaign
    return campaign


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def encoded(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def path_fingerprint(path: Path) -> str:
    return digest(str(path.resolve(strict=False)).encode())


def stream_value(path: Path) -> dict[str, Any]:
    require(path.is_file() and path.stat().st_size <= MAX_STREAM_BYTES,
            "CLI observation output is missing or exceeds the bounded stream limit")
    data = path.read_bytes()
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError("CLI observation output is not UTF-8") from error
    return {"encoding": "utf-8", "text": text, "bytes": len(data), "sha256": digest(data)}


def run_process(binary: Path, runtime: Path, repository: Path, user_argv: list[str],
                directory: Path, order: int, criterion: str | None) -> dict[str, Any]:
    directory.mkdir(parents=True)
    stdout_path, stderr_path = directory / "stdout", directory / "stderr"
    actual_user_argv = [str(directory / "portable-context.json") if value == "<observation-output>" else value
                        for value in user_argv]
    argv = [str(binary), "--runtime", str(runtime), *actual_user_argv]
    started_at, started = utc_now(), time.monotonic_ns()
    return_code: int | None = None
    termination: dict[str, Any] | str | None = None
    with stdout_path.open("xb") as stdout, stderr_path.open("xb") as stderr:
        try:
            process = subprocess.Popen(argv, cwd=repository, stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, start_new_session=True)
        except OSError as error:
            raise ValueError("CLI observation process could not be started") from error
        try:
            return_code = process.wait(timeout=COMMAND_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            termination = {"kind": "timeout", "seconds": COMMAND_TIMEOUT_SECONDS,
                           "cleanup": "process_group_terminated"}
    if termination is None:
        if return_code is not None and return_code >= 0:
            termination = "exited"
        else:
            number = -return_code if return_code is not None else None
            termination = {"kind": "signal", "number": number,
                "name": signal.Signals(number).name if number in signal.Signals._value2member_map_ else "UNKNOWN"}
            return_code = None
    logical_argv = ["volicord", "--runtime", "<isolated-runtime>", *user_argv]
    return {"order": order, "criterion": criterion,
        "command_identity": digest(encoded(logical_argv)), "argv": logical_argv,
        "raw_argv_sha256": digest(encoded(argv)), "working_directory": "<observation-workspace>",
        "working_directory_sha256": path_fingerprint(repository), "started_at": started_at,
        "ended_at": utc_now(), "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
        "stdout": stream_value(stdout_path), "stderr": stream_value(stderr_path),
        "exit_code": return_code, "termination": termination}


def git_revision(repository: Path) -> str:
    completed = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repository,
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        text=True, check=False)
    require(completed.returncode == 0 and re.fullmatch(r"[0-9a-f]{40}", completed.stdout.strip()) is not None,
            "CLI observation repository revision is unavailable")
    return completed.stdout.strip()


def validate_process(value: Any, criterion: str | None, expected_argv: list[str]) -> None:
    require(isinstance(value, dict) and set(value) == {"order", "criterion", "command_identity", "argv",
        "raw_argv_sha256", "working_directory", "working_directory_sha256", "started_at", "ended_at",
        "duration_ms", "stdout", "stderr", "exit_code", "termination"}, "incomplete CLI process evidence")
    logical = ["volicord", "--runtime", "<isolated-runtime>", *expected_argv]
    require(value["criterion"] == criterion and value["argv"] == logical
        and value["command_identity"] == digest(encoded(logical)), "CLI observation invocation changed")
    require("--project" not in value["argv"] and value["working_directory"] == "<observation-workspace>"
        and re.fullmatch(r"[0-9a-f]{64}", str(value["working_directory_sha256"])),
        "CLI observation is not repository-relative or leaked a private path")
    require(type(value["order"]) is int and value["order"] >= 1
        and isinstance(value["started_at"], str) and isinstance(value["ended_at"], str)
        and isinstance(value["duration_ms"], (int, float)) and value["duration_ms"] >= 0
        and re.fullmatch(r"[0-9a-f]{64}", str(value["raw_argv_sha256"])), "invalid CLI process identity/order")
    for name in ("stdout", "stderr"):
        stream = value[name]
        require(isinstance(stream, dict) and set(stream) == {"encoding", "text", "bytes", "sha256"}
            and stream["encoding"] == "utf-8" and isinstance(stream["text"], str)
            and stream["bytes"] == len(stream["text"].encode()) <= MAX_STREAM_BYTES
            and stream["sha256"] == digest(stream["text"].encode()), "CLI stream bytes/hash changed")
    exited = value["termination"] == "exited"
    terminated = isinstance(value["termination"], dict) and value["termination"].get("kind") in {"signal", "timeout"}
    require((exited and type(value["exit_code"]) is int and 0 <= value["exit_code"] <= 255)
        or (terminated and value["exit_code"] is None), "incomplete CLI exit or termination evidence")


def validate_value(value: Any, *, candidate_head: str, evidence_sha256: str,
                   revisions: dict[str, str], candidate_executable: dict[str, str]) -> dict[str, Any]:
    c = campaign_api()
    require(isinstance(value, dict) and set(value) == {"kind", "schema_version", "observation_run_id",
        "candidate_head", "evidence_set_sha256", "candidate_executable", "execution_root_identity",
        "created_at", "repository_observations", "naturalistic_campaign_mutated"},
        "invalid CLI observation artifact")
    require(value["kind"] == "phase8_cli_observation_set" and value["schema_version"] == 1
        and value["candidate_head"] == candidate_head and value["evidence_set_sha256"] == evidence_sha256,
        "CLI observation candidate/evidence binding mismatch")
    require(re.fullmatch(r"[0-9a-f]{32}", str(value["observation_run_id"]))
        and re.fullmatch(r"[0-9a-f]{32}", str(value["execution_root_identity"]))
        and isinstance(value["created_at"], str) and value["naturalistic_campaign_mutated"] is False,
        "invalid CLI observation identity")
    executable = value["candidate_executable"]
    require(isinstance(executable, dict) and set(executable) == {"name", "sha256", "path_sha256"}
        and executable["name"] == "volicord"
        and all(re.fullmatch(r"[0-9a-f]{64}", str(executable[k])) for k in ("sha256", "path_sha256")),
        "invalid candidate executable binding")
    require(executable == candidate_executable,
        "CLI observation executable does not match the campaign-bound candidate artifact")
    observations = value["repository_observations"]
    require(isinstance(observations, list) and [v.get("repository_class") for v in observations] == list(c.CLASSES),
        "CLI observations must cover the three ordered repository classes")
    for item in observations:
        kind = item["repository_class"]
        require(set(item) == {"repository_class", "repository_revision", "workspace", "runtime_home",
            "repository_revision_after", "repository_clean_after", "invocations"}
            and item["repository_revision"] == item["repository_revision_after"] == revisions[kind]
            and item["repository_clean_after"] is True, "CLI observation repository class/revision changed")
        for field, suffix in (("workspace", f"workspaces/{kind}/repository"),
                              ("runtime_home", f"runtimes/{kind}")):
            binding = item[field]
            require(isinstance(binding, dict) and set(binding) == {"identity", "path_basis", "absolute_path_sha256"}
                and re.fullmatch(r"[0-9a-f]{32}", str(binding["identity"]))
                and binding["path_basis"] == suffix
                and re.fullmatch(r"[0-9a-f]{64}", str(binding["absolute_path_sha256"])),
                "CLI observation workspace/runtime isolation binding changed")
        invocations = item["invocations"]
        expected = [(None, ["init", f"CLI Observation {kind}"]), *CRITERION_COMMANDS.items()]
        require(isinstance(invocations, list) and len(invocations) == len(expected),
                "CLI observation surface inventory is incomplete")
        for order, (process, (criterion, argv)) in enumerate(zip(invocations, expected), start=1):
            validate_process(process, criterion, argv)
            require(process["order"] == order, "CLI observation order changed")
    return value


def load(campaign_root: Path, observation_root: Path) -> tuple[dict[str, Any], bytes, bytes]:
    c = campaign_api()
    manifest = c.load_evidence_set(campaign_root)
    bound = manifest["candidate_artifacts"]["volicord"]
    candidate_executable = {"name": "volicord", "sha256": bound["sha256"],
        "path_sha256": path_fingerprint(Path(bound["path"]))}
    evidence_sha256 = c.harness.sha256(campaign_root / "evidence-set.json")
    observation_root = observation_root.resolve()
    require(not observation_root.is_relative_to(campaign_root.resolve()), "CLI observations must remain outside the campaign")
    data = (observation_root / "observations.json").read_bytes()
    receipt_data = (observation_root / "receipt.json").read_bytes()
    require(len(data) <= MAX_ARTIFACT_BYTES and len(receipt_data) <= MAX_ARTIFACT_BYTES,
            "CLI observation artifact exceeds its bound")
    receipt = json.loads(receipt_data)
    value = json.loads(data)
    revisions = {}
    for kind in c.CLASSES:
        found = {state["repository_revision"] for state in manifest["cycles"].values()
                 if state["repository_class"] == kind}
        require(len(found) == 1, "campaign repository class has inconsistent pinned revisions")
        revisions[kind] = found.pop()
    validate_value(value, candidate_head=manifest["candidate_head"], evidence_sha256=evidence_sha256,
                   revisions=revisions, candidate_executable=candidate_executable)
    require(receipt == {"kind": "phase8_cli_observation_receipt", "schema_version": 1,
        "observation_run_id": value["observation_run_id"], "candidate_head": value["candidate_head"],
        "evidence_set_sha256": evidence_sha256, "observations_sha256": digest(data)},
        "CLI observation bytes/hash receipt mismatch")
    return value, data, receipt_data


def collect(campaign_root: Path, output: Path, *,
            cloner: Callable[[Path, Path, str], None] | None = None,
            runner: Callable[..., dict[str, Any]] = run_process,
            revision_reader: Callable[[Path], str] = git_revision,
            clean_reader: Callable[[Path], bool] | None = None,
            run_id: str | None = None) -> dict[str, Any]:
    c = campaign_api()
    campaign_root, output = campaign_root.resolve(), output.absolute()
    require(not output.resolve().is_relative_to(campaign_root), "CLI observation output must remain outside the campaign")
    manifest = c.load_evidence_set(campaign_root)
    before = {name: (campaign_root / name).read_bytes() for name in ("campaign.json", "evidence-set.json", "evidence-inventory.json")}
    campaign = c.load_campaign(campaign_root)
    binary = Path(campaign["candidate_binary"]).resolve()
    c.verify_candidate_artifacts(campaign, ("volicord",))
    bound = campaign["candidate_artifacts"]["volicord"]
    candidate_executable = {"name": "volicord", "sha256": bound["sha256"],
        "path_sha256": path_fingerprint(Path(bound["path"]))}
    specs = c.repository_spec_map(c.read_json(campaign_root / "repository-input.json"))
    cloner = cloner or c.clone_repository
    clean_reader = clean_reader or c.harness.git_clean
    output.parent.mkdir(parents=True, exist_ok=True)
    execution_root = Path(tempfile.mkdtemp(prefix=".cli-observation-", dir=output.parent)).resolve()
    observation_run_id = run_id or secrets.token_hex(16)
    require(re.fullmatch(r"[0-9a-f]{32}", observation_run_id) is not None, "invalid CLI observation run identity")
    observations = []
    try:
        for kind in c.CLASSES:
            source = Path(specs[kind]["path"]).resolve()
            require(not source.is_relative_to(campaign_root), "measured campaign workspaces cannot be CLI observation sources")
            revisions = {state["repository_revision"] for state in manifest["cycles"].values()
                         if state["repository_class"] == kind}
            require(len(revisions) == 1, "campaign repository class has inconsistent pinned revisions")
            revision = revisions.pop()
            repository = execution_root / "workspaces" / kind / "repository"
            runtime = execution_root / "runtimes" / kind
            runtime.mkdir(parents=True)
            cloner(source, repository, revision)
            require(revision_reader(repository) == revision, "CLI observation clone revision mismatch")
            commands = [["init", f"CLI Observation {kind}"], *CRITERION_COMMANDS.values()]
            criteria = [None, *CRITERION_COMMANDS]
            invocations = []
            for order, (criterion, argv) in enumerate(zip(criteria, commands), start=1):
                with c.candidate_artifact_use(campaign, ("volicord",)):
                    invocations.append(runner(binary, runtime, repository, list(argv),
                        execution_root / "processes" / kind / f"{order:02d}", order, criterion))
            observations.append({"repository_class": kind, "repository_revision": revision,
                "workspace": {"identity": secrets.token_hex(16),
                    "path_basis": f"workspaces/{kind}/repository", "absolute_path_sha256": path_fingerprint(repository)},
                "runtime_home": {"identity": secrets.token_hex(16),
                    "path_basis": f"runtimes/{kind}", "absolute_path_sha256": path_fingerprint(runtime)},
                "repository_revision_after": revision_reader(repository),
                "repository_clean_after": clean_reader(repository), "invocations": invocations})
        value = {"kind": "phase8_cli_observation_set", "schema_version": 1,
            "observation_run_id": observation_run_id, "candidate_head": manifest["candidate_head"],
            "evidence_set_sha256": c.harness.sha256(campaign_root / "evidence-set.json"),
            "candidate_executable": candidate_executable, "execution_root_identity": secrets.token_hex(16),
            "created_at": utc_now(), "repository_observations": observations,
            "naturalistic_campaign_mutated": False}
        revisions = {item["repository_class"]: item["repository_revision"] for item in observations}
        validate_value(value, candidate_head=manifest["candidate_head"],
            evidence_sha256=value["evidence_set_sha256"], revisions=revisions,
            candidate_executable=candidate_executable)
        data = encoded(value)
        require(len(data) <= MAX_ARTIFACT_BYTES, "CLI observation artifact exceeds its bound")
        receipt = {"kind": "phase8_cli_observation_receipt", "schema_version": 1,
            "observation_run_id": observation_run_id, "candidate_head": manifest["candidate_head"],
            "evidence_set_sha256": value["evidence_set_sha256"], "observations_sha256": digest(data)}
        for name, original in before.items():
            require((campaign_root / name).read_bytes() == original, "CLI observation mutated immutable campaign state")
        import review_operations
        review_operations.publish_directory(output, {"observations.json": data, "receipt.json": encoded(receipt)})
        load(campaign_root, output)
        return {"state": "collected", "observation_root": str(output),
            "observation_run_id": observation_run_id, "candidate_head": manifest["candidate_head"],
            "repository_classes": list(c.CLASSES), "criterion_count": len(c.CLASSES) * len(CRITERION_COMMANDS),
            "observations_sha256": digest(data), "campaign_mutated": False}
    finally:
        shutil.rmtree(execution_root, ignore_errors=True)
