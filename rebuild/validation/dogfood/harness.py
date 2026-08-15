#!/usr/bin/env python3
"""Run and sanitize the repository-owned Phase 8 repeated dogfood evaluation."""

from __future__ import annotations

import argparse
from collections import Counter
import datetime as dt
import hashlib
from html.parser import HTMLParser
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import socket
import subprocess
import sys
import time
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
DEFINITION = HERE / "evaluation.json"
V11_HARNESS = ROOT / "rebuild/validation/end-to-end/multi-repository/harness.py"
DECISION_REGISTER = ROOT / "rebuild/docs/design/open-decisions.md"
ALLOWED_STATUS = {
    "passed", "failed", "partial", "unsupported", "skipped", "environment_blocked"
}
CLASSES = ("volicord", "small-python", "polyglot-medium")
OFFICIAL_SUFFIXES = {
    ".java": "Java",
    ".py": "Python",
    ".js": "JavaScript",
    ".jsx": "JavaScript",
    ".ts": "TypeScript",
    ".tsx": "TypeScript",
    ".c": "C",
    ".h": "C",
    ".cc": "C++",
    ".cpp": "C++",
    ".cxx": "C++",
    ".hpp": "C++",
    ".rs": "Rust",
}
DOCUMENT_SUFFIXES = {".md", ".markdown", ".rst", ".adoc"}
IGNORED_PARTS = {".git", ".local", "target", "node_modules", "vendor", "dist", "build"}
SECRET_MARKERS = (
    "bearer ", "api_key", "api-key", "access_token", "refresh_token",
    "private prompt", "auth.json", "credential_content",
)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def command_output(cwd: Path, *argv: str) -> str | None:
    result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, check=False)
    return result.stdout.strip() if result.returncode == 0 else None


def git_head(path: Path) -> str | None:
    return command_output(path, "git", "rev-parse", "HEAD")


def git_clean(path: Path) -> bool:
    value = command_output(path, "git", "status", "--porcelain=v1", "--untracked-files=all")
    return value == ""


def directory_bytes(path: Path) -> int:
    total = 0
    if not path.exists():
        return 0
    for root, directories, files in os.walk(path):
        directories[:] = [name for name in directories if name not in {".git"}]
        for name in files:
            try:
                total += (Path(root) / name).stat().st_size
            except OSError:
                continue
    return total


def load_v11() -> Any:
    spec = importlib.util.spec_from_file_location("volicord_phase8_v11", V11_HARNESS)
    if spec is None or spec.loader is None:
        raise RuntimeError("the maintained V11 harness could not be loaded")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_definition() -> dict[str, Any]:
    value = json.loads(DEFINITION.read_text(encoding="utf-8"))
    if value.get("kind") != "phase8_dogfood_evaluation_definition":
        raise ValueError("unexpected Phase 8 evaluation definition kind")
    if set(value.get("status_values", [])) != ALLOWED_STATUS:
        raise ValueError("the Phase 8 status vocabulary is incomplete")
    if value.get("candidate_cycle_count") != 2:
        raise ValueError("Phase 8 requires exactly two independent cycles per repository")
    if tuple(value.get("repository_classes", {})) != CLASSES:
        raise ValueError("the Phase 8 repository class order changed")
    v11 = load_v11()
    if tuple(value.get("required_product_steps", [])) != tuple(v11.REQUIRED_STEPS):
        raise ValueError("Phase 8 no longer routes its product journey through maintained V11 steps")
    return value


def repository_files(path: Path) -> list[Path]:
    return [
        item
        for item in path.rglob("*")
        if item.is_file() and not any(part in IGNORED_PARTS for part in item.relative_to(path).parts)
    ]


def repository_identity(kind: str, spec: dict[str, Any], definition: dict[str, Any]) -> dict[str, Any]:
    path = Path(spec["path"]).resolve()
    head = git_head(path)
    origin = command_output(path, "git", "remote", "get-url", "origin")
    files = repository_files(path) if path.is_dir() else []
    languages = sorted({OFFICIAL_SUFFIXES[item.suffix.lower()] for item in files if item.suffix.lower() in OFFICIAL_SUFFIXES})
    documents = sum(item.suffix.lower() in DOCUMENT_SUFFIXES for item in files)
    license_path = path / spec.get("license_file", "") if spec.get("license_file") else None
    license_identity = {
        "spdx": spec.get("license_spdx"),
        "file": license_path.relative_to(path).as_posix() if license_path and license_path.is_file() else None,
        "sha256": sha256(license_path) if license_path and license_path.is_file() else None,
    }
    status = "passed"
    blockers: list[str] = []
    if not path.is_dir() or head is None:
        blockers.append("source repository or revision is unavailable")
    if head != spec.get("revision"):
        blockers.append("configured revision does not match the source repository HEAD")
    if origin != spec.get("origin"):
        blockers.append("configured origin does not match the source repository")
    if not git_clean(path):
        blockers.append("source repository is not clean")
    if license_identity["file"] is None and kind != "volicord":
        blockers.append("external repository license file is unavailable")
    fixture_root = (ROOT / "rebuild/validation").resolve()
    if path == fixture_root or fixture_root in path.parents:
        blockers.append("a maintained validation fixture cannot substitute for an actual repository")
    rules = definition["repository_classes"][kind]
    if kind == "volicord" and head != git_head(ROOT):
        blockers.append("Volicord repository revision is not the candidate HEAD")
    if kind == "small-python":
        if languages != ["Python"]:
            blockers.append("small repository is not a single official Python application")
        if len(files) > rules["maximum_files"]:
            blockers.append("small repository exceeds the bounded file ceiling")
    if kind == "polyglot-medium":
        if len(files) < rules["minimum_files"]:
            blockers.append("polyglot repository does not meet the medium file floor")
        if len(languages) < rules["minimum_official_structural_languages"]:
            blockers.append("polyglot repository has fewer than three official structural languages")
        if documents == 0:
            blockers.append("polyglot repository has no documentation")
    if blockers:
        status = "environment_blocked"
    return {
        "class": kind,
        "status": status,
        "origin": origin,
        "revision": head,
        "license": license_identity,
        "file_count": len(files),
        "documentation_file_count": documents,
        "official_structural_languages": languages,
        "blockers": blockers,
    }


def load_repository_specs(path: Path, candidate_head: str, definition: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], list[dict[str, Any]]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    repositories = value.get("repositories")
    if not isinstance(repositories, list):
        raise ValueError("repository manifest requires a repositories list")
    by_class = {item.get("class"): item for item in repositories if isinstance(item, dict)}
    if tuple(item.get("class") for item in repositories) != CLASSES or set(by_class) != set(CLASSES):
        raise ValueError("repository manifest must contain the three ordered Phase 8 classes")
    by_class["volicord"]["revision"] = candidate_head
    identities = [repository_identity(kind, by_class[kind], definition) for kind in CLASSES]
    return by_class, identities


def operation_duration(step_value: dict[str, Any], *keys: str) -> float | None:
    evidence = step_value.get("evidence", {})
    for key in keys:
        operation = evidence.get(key)
        if isinstance(operation, dict) and isinstance(operation.get("duration_ms"), (int, float)):
            return float(operation["duration_ms"])
    return None


def document_measurements(step_value: dict[str, Any]) -> tuple[list[float], list[int]]:
    durations: list[float] = []
    sizes: list[int] = []
    for item in step_value.get("evidence", {}).get("documents", []):
        operation = item.get("operation", {})
        result = item.get("result", {})
        if isinstance(operation.get("duration_ms"), (int, float)):
            durations.append(float(operation["duration_ms"]))
        if isinstance(result.get("bytes"), int):
            sizes.append(result["bytes"])
    return durations, sizes


def status_from_steps(step_statuses: dict[str, str]) -> str:
    values = list(step_statuses.values())
    for status in ("failed", "environment_blocked", "partial", "unsupported", "skipped"):
        if status in values:
            return status
    return "passed" if values else "failed"


def quality_observations(step_statuses: dict[str, str]) -> dict[str, dict[str, str]]:
    routes = {
        "context_recovery_accuracy": ("restart_recall",),
        "decision_repetition": ("inquiry_decision", "restart_recall"),
        "question_relevance": ("candidate_boundary", "inquiry_decision"),
        "decision_comprehension": ("inquiry_decision",),
        "source_grounding": ("source_grounded_understanding", "checkpoint", "document_outputs"),
        "capability_honesty": ("repository_analysis", "parser_failure", "provider_failure"),
        "coverage": ("repository_analysis", "source_grounded_understanding"),
        "memory_correctability": ("correction_supersession_deletion",),
        "interruption_cost": ("ordinary_work", "guarded_boundary"),
        "document_fidelity_and_usefulness": ("document_outputs",),
        "portability": ("portable_clone", "divergent_conflict"),
        "recovery": ("provider_failure", "parser_failure", "derived_index_recovery"),
    }
    result: dict[str, dict[str, str]] = {}
    for name, steps in routes.items():
        routed = {step: step_statuses.get(step, "skipped") for step in steps}
        status = status_from_steps(routed)
        if name in {"question_relevance", "decision_comprehension", "document_fidelity_and_usefulness", "interruption_cost"} and status == "passed":
            status = "partial"
        result[name] = {
            "status": status,
            "basis": ",".join(f"{key}:{value}" for key, value in routed.items()),
        }
    return result


class AccessibilityParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.html_lang: str | None = None
        self.heading_levels: list[int] = []
        self.controls = 0
        self.labels = 0
        self.links = 0
        self.viewport = False
        self.styles: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        if tag == "html":
            self.html_lang = attributes.get("lang")
        if tag in {"h1", "h2", "h3", "h4", "h5", "h6"}:
            self.heading_levels.append(int(tag[1]))
        if tag in {"input", "select", "textarea", "button"}:
            self.controls += 1
        if tag == "label":
            self.labels += 1
        if tag == "a" and attributes.get("href"):
            self.links += 1
        if tag == "meta" and attributes.get("name") == "viewport":
            self.viewport = True

    def handle_data(self, data: str) -> None:
        if ":root{" in data or "@media" in data or ":focus" in data:
            self.styles.append(data)


def parse_accessibility_html(content: str, *, expected_language: str | None) -> dict[str, Any]:
    parser = AccessibilityParser()
    parser.feed(content)
    style = "\n".join(parser.styles)
    heading_order = all(next_level <= current + 1 for current, next_level in zip(parser.heading_levels, parser.heading_levels[1:]))
    checks = {
        "keyboard_reachability": "passed" if parser.links + parser.controls > 0 else "partial",
        "visible_focus": "passed" if re.search(r":focus(?:-visible)?", style) else "partial",
        "not_color_only": "partial",
        "headings_and_labels": "passed" if parser.heading_levels and heading_order and parser.labels >= parser.controls else "partial",
        "narrow_and_zoomed_presentation": "partial" if parser.viewport else "failed",
        "document_html_language": "passed" if expected_language is None or parser.html_lang == expected_language else "failed",
    }
    return {
        "checks": checks,
        "html_language": parser.html_lang,
        "heading_count": len(parser.heading_levels),
        "control_count": parser.controls,
        "label_count": parser.labels,
        "viewport": parser.viewport,
    }


def exchange_http(address: str, target: str) -> str:
    host, port = address.rsplit(":", 1)
    with socket.create_connection((host, int(port)), timeout=5) as connection:
        connection.settimeout(5)
        request = f"GET {target} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        connection.sendall(request.encode("ascii"))
        chunks: list[bytes] = []
        while True:
            chunk = connection.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
    response = b"".join(chunks).decode("utf-8", errors="replace")
    return response.split("\r\n\r\n", 1)[1] if "\r\n\r\n" in response else response


def viewer_start_failure_status(message: str) -> str:
    lowered = message.lower()
    if "operation not permitted" in lowered or "permission denied" in lowered:
        return "environment_blocked"
    return "failed"


def live_viewer_accessibility(target_root: Path, project_id: str) -> dict[str, Any]:
    results: dict[str, Any] = {}
    for locale in ("en", "ko"):
        argv = [
            str(target_root / "prefix/bin/volicord-viewer"),
            "--runtime", str(target_root / "runtime"),
            "--project", project_id,
            "--bind", "127.0.0.1:0",
            "--locale", locale,
            "--level", "deep",
            "--language", locale,
        ]
        process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        try:
            assert process.stderr is not None
            startup = process.stderr.readline().strip()
            marker = "Volicord local viewer: http://"
            if not startup.startswith(marker):
                results[locale] = {
                    "status": viewer_start_failure_status(startup),
                    "reason": "viewer_start_failed",
                }
                continue
            address = startup[len(marker):].rstrip("/")
            content = exchange_http(address, "/?level=deep")
            results[locale] = {"status": "passed", **parse_accessibility_html(content, expected_language=locale)}
        except (AssertionError, OSError, ValueError) as error:
            results[locale] = {"status": "failed", "reason": type(error).__name__}
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
    checks: dict[str, str] = {}
    for locale_result in results.values():
        for name, status in locale_result.get("checks", {}).items():
            if checks.get(name) == "failed" or status == "failed":
                checks[name] = "failed"
            elif checks.get(name) == "partial" or status == "partial":
                checks[name] = "partial"
            else:
                checks[name] = status
    locale_status = status_from_steps({locale: value.get("status", "failed") for locale, value in results.items()})
    checks["korean_english_fixed_ui"] = locale_status
    return {"status": status_from_steps(checks), "checks": checks, "locales": results}


def sanitized_cycle(
    kind: str,
    cycle: int,
    raw: dict[str, Any],
    cycle_root: Path,
    duration_ms: float,
    manual_observations: dict[str, Any] | None = None,
) -> dict[str, Any]:
    steps = raw.get("steps", {})
    step_statuses = {name: value.get("status", "failed") for name, value in steps.items()}
    document_durations, document_sizes = document_measurements(steps.get("document_outputs", {}))
    target_root = cycle_root / "work" / kind
    bundle = target_root / "base.volicord.json"
    analysis_store = target_root / "runtime/analysis"
    metrics = {
        "cycle_duration_ms": round(duration_ms, 3),
        "inventory_analysis_duration_ms": operation_duration(steps.get("repository_analysis", {}), "operation"),
        "recall_duration_ms": operation_duration(steps.get("restart_recall", {}), "cli_operation"),
        "document_generation_duration_ms": document_durations,
        "repair_reindex_duration_ms": operation_duration(steps.get("derived_index_recovery", {}), "repair_operation"),
        "document_output_bytes": document_sizes,
        "portable_bundle_bytes": bundle.stat().st_size if bundle.is_file() else None,
        "runtime_home_bytes": directory_bytes(target_root / "runtime"),
        "derived_state_bytes": directory_bytes(analysis_store),
        "peak_memory_bytes": None,
        "peak_memory_status": "unsupported",
    }
    accessibility = (
        live_viewer_accessibility(target_root, raw["project_id"])
        if raw.get("project_id") and (target_root / "prefix/bin/volicord-viewer").is_file()
        else {"status": "skipped", "checks": {}, "reason": "viewer prerequisite failed"}
    )
    generated_html = sorted((target_root / "documents").glob("*.html"))
    generated_language = [
        parse_accessibility_html(path.read_text(encoding="utf-8"), expected_language="en")
        for path in generated_html
    ]
    accessibility.setdefault("checks", {})["document_html_language"] = (
        "passed"
        if generated_language
        and all(item["checks"]["document_html_language"] == "passed" for item in generated_language)
        else "failed"
    )
    accessibility["status"] = status_from_steps(accessibility["checks"])
    quality = quality_observations(step_statuses)
    subjective = {
        "question_relevance",
        "decision_comprehension",
        "interruption_cost",
        "document_fidelity_and_usefulness",
    }
    for name, observation in (manual_observations or {}).items():
        if name not in subjective or not isinstance(observation, dict):
            raise ValueError(f"unsupported manual Phase 8 observation: {name}")
        if observation.get("status") not in ALLOWED_STATUS:
            raise ValueError(f"manual Phase 8 observation has an invalid status: {name}")
        basis = observation.get("basis")
        if not isinstance(basis, str) or not basis.strip() or len(basis) > 500:
            raise ValueError(f"manual Phase 8 observation needs a bounded basis: {name}")
        quality[name] = {"status": observation["status"], "basis": basis}
    return {
        "cycle": cycle,
        "status": status_from_steps(step_statuses),
        "project_identity": raw.get("project_id"),
        "repository_revision": raw.get("identity", {}).get("revision"),
        "legacy_runtime_untouched": raw.get("legacy_runtime_untouched"),
        "step_statuses": step_statuses,
        "step_status_counts": dict(sorted(Counter(step_statuses.values()).items())),
        "quality_observations": quality,
        "measurements": metrics,
        "accessibility": accessibility,
    }


def aggregate_accessibility(repositories: list[dict[str, Any]]) -> dict[str, Any]:
    checks: dict[str, str] = {}
    for repository in repositories:
        for cycle in repository.get("cycles", []):
            for name, status in cycle.get("accessibility", {}).get("checks", {}).items():
                current = checks.get(name)
                if current == "failed" or status == "failed":
                    checks[name] = "failed"
                elif current == "partial" or status == "partial":
                    checks[name] = "partial"
                else:
                    checks[name] = status
    return {
        "status": status_from_steps(checks),
        "checks": checks,
        "limits": [
            "No standards certification or human-subject accessibility qualification was performed.",
            "Narrow and zoom behavior is assessed from live HTML structure and viewport metadata, not a browser layout engine.",
        ],
    }


def detect_revisit_triggers(repositories: list[dict[str, Any]], accessibility: dict[str, Any]) -> list[dict[str, str]]:
    triggers: list[dict[str, str]] = []
    failed_checks = sorted(name for name, status in accessibility.get("checks", {}).items() if status == "failed")
    if failed_checks:
        triggers.append({"decision_id": "Q5", "basis": "generated/viewer HTML accessibility blocker: " + ", ".join(failed_checks)})
    for repository in repositories:
        for cycle in repository.get("cycles", []):
            if cycle.get("step_statuses", {}).get("inquiry_decision") == "failed":
                triggers.append({"decision_id": "Q1", "basis": f"{repository['class']} cycle {cycle['cycle']} inquiry/Decision failure"})
            if cycle.get("step_statuses", {}).get("repository_analysis") == "failed":
                triggers.append({"decision_id": "Q2", "basis": f"{repository['class']} cycle {cycle['cycle']} repository analysis failure"})
    unique = {(item["decision_id"], item["basis"]): item for item in triggers}
    return [unique[key] for key in sorted(unique)]


def sanitize_check(value: Any) -> None:
    encoded = json.dumps(value, sort_keys=True).lower()
    if any(marker in encoded for marker in SECRET_MARKERS):
        raise ValueError("sanitized result contains a prohibited secret/private marker")
    if re.search(r"/(?:home|tmp)/[^\s\"']+", encoded):
        raise ValueError("sanitized result contains an absolute local path")


def validate_result(result: dict[str, Any], definition: dict[str, Any]) -> None:
    if result.get("kind") != "phase8_dogfood_result":
        raise ValueError("unexpected dogfood result kind")
    if not re.fullmatch(r"[0-9a-f]{40}", result.get("candidate_head", "")):
        raise ValueError("dogfood result has no exact candidate HEAD")
    repositories = result.get("repositories", [])
    if [item.get("class") for item in repositories] != list(CLASSES):
        raise ValueError("dogfood result does not contain the three ordered repository classes")
    for repository in repositories:
        if len(repository.get("cycles", [])) != definition["candidate_cycle_count"]:
            raise ValueError("dogfood result does not contain two cycles per repository")
        for cycle in repository["cycles"]:
            if set(cycle.get("step_statuses", {})) != set(definition["required_product_steps"]):
                raise ValueError("dogfood cycle silently dropped a maintained product step")
            statuses = set(cycle["step_statuses"].values())
            if not statuses <= ALLOWED_STATUS:
                raise ValueError("dogfood cycle contains an unknown status")
    if result.get("replacement_pass_candidate") is True:
        if result.get("status") != "passed" or result.get("blockers"):
            raise ValueError("replacement pass cannot have a non-pass status or blocker")
        if result.get("decision_revisit", {}).get("observed_active_triggers"):
            raise ValueError("replacement pass cannot have an active Decision revisit trigger")
        if result.get("candidate_worktree") != {"clean_before": True, "clean_after": True}:
            raise ValueError("replacement pass requires a clean candidate throughout dogfood")
        if result.get("fixture_regression", {}).get("status") != "passed":
            raise ValueError("replacement pass requires current structural/fallback regression")
        if result.get("accessibility", {}).get("status") != "passed":
            raise ValueError("replacement pass requires passed accessibility evaluation")
        for repository in repositories:
            if repository.get("status") != "passed" or not repository.get("independent_fresh_runtime_cycles"):
                raise ValueError("replacement pass requires two independent passed repository cycles")
            for cycle in repository["cycles"]:
                if cycle.get("status") != "passed":
                    raise ValueError("replacement pass contains a non-pass dogfood cycle")
                if any(
                    observation.get("status") != "passed"
                    for observation in cycle.get("quality_observations", {}).values()
                ):
                    raise ValueError("replacement pass contains an unqualified quality observation")
    sanitize_check(result)


def aggregate_status(repositories: list[dict[str, Any]], regression: dict[str, Any], accessibility: dict[str, Any], blockers: list[str]) -> str:
    statuses = [regression.get("status", "failed"), accessibility.get("status", "failed")]
    statuses.extend(repository.get("status", "failed") for repository in repositories)
    if "failed" in statuses:
        return "failed"
    if "environment_blocked" in statuses:
        return "environment_blocked"
    for status in ("partial", "unsupported", "skipped"):
        if status in statuses:
            return status
    return "environment_blocked" if blockers else "passed"


def run_fixture_regression(v11: Any, raw_root: Path, base_env: dict[str, str], definition: dict[str, Any]) -> dict[str, Any]:
    recorder = v11.Recorder(raw_root / "fixture-regression")
    command = definition["fixture_regression_command"]
    result = recorder.run("official-structural-and-fallback", command, base_env, cwd=ROOT, timeout=900)
    return {
        "status": "passed" if result.get("exit_code") == 0 else "failed",
        "command": command,
        "duration_ms": result.get("duration_ms"),
        "covers_official_structural_fixtures": True,
        "covers_out_of_set_fallback_fixture": True,
        "fixtures_are_not_real_repository_substitutes": True,
    }


def run_evaluation(args: argparse.Namespace) -> int:
    definition = load_definition()
    candidate_head = git_head(ROOT)
    if candidate_head is None or candidate_head != args.candidate_head:
        raise RuntimeError("candidate HEAD does not match --candidate-head")
    clean_before = git_clean(ROOT)
    specs, identities = load_repository_specs(Path(args.repositories), candidate_head, definition)
    output = Path(args.output_dir).resolve()
    if output.exists() and any(output.iterdir()):
        raise RuntimeError("Phase 8 output directory must be absent or empty")
    output.mkdir(parents=True, exist_ok=True)
    raw_root = output / "raw"
    raw_root.mkdir()
    v11 = load_v11()
    base_env = os.environ.copy()
    base_env.setdefault("CARGO_HOME", str(Path.home() / ".cargo"))
    base_env.setdefault("RUSTUP_HOME", str(Path.home() / ".rustup"))
    authorization = definition["codex_transmission"]["authorization_assertion"]
    codex_authorized = args.authorize_codex_transmission == authorization
    codex = shutil.which("codex") if codex_authorized else None
    original_prepare: Callable[..., Any] = v11.prepare_repository
    source_by_class = {kind: Path(specs[kind]["path"]).resolve() for kind in CLASSES}

    def prepare_actual(kind: str, destination: Path, recorder: Any, env: dict[str, str]) -> dict[str, str]:
        source = source_by_class[kind]
        result = recorder.run(
            "clone-actual-repository",
            ["git", "clone", "--quiet", "--no-hardlinks", str(source), str(destination)],
            env,
            timeout=300,
        )
        if result.get("exit_code") != 0:
            raise RuntimeError("actual repository disposable clone failed")
        return {
            "revision": v11.git_revision(recorder, destination, env),
            "content_sha256": v11.tree_hash(destination),
        }

    v11.prepare_repository = prepare_actual
    for kind in CLASSES:
        v11.PROVIDER_SOURCE_PATHS[kind] = specs[kind]["provider_source_path"]
    started_at = utc_now()
    started = time.monotonic_ns()
    repository_results: list[dict[str, Any]] = []
    try:
        for identity in identities:
            kind = identity["class"]
            cycles: list[dict[str, Any]] = []
            if identity["status"] == "passed":
                for cycle_number in range(1, definition["candidate_cycle_count"] + 1):
                    cycle_root = raw_root / f"{kind}-cycle-{cycle_number}"
                    cycle_root.mkdir()
                    recorder = v11.Recorder(cycle_root)
                    cycle_started = time.monotonic_ns()
                    try:
                        raw = v11.rehearse_target(kind, cycle_root, recorder, base_env, codex)
                    except (AssertionError, OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
                        raw = {
                            "class": kind,
                            "identity": {"revision": identity["revision"]},
                            "steps": {
                                name: v11.step("failed" if name == "clean_install" else "skipped", type(error).__name__)
                                for name in definition["required_product_steps"]
                            },
                        }
                    write_json(cycle_root / "raw-cycle-result.json", raw)
                    cycles.append(sanitized_cycle(
                        kind,
                        cycle_number,
                        raw,
                        cycle_root,
                        (time.monotonic_ns() - cycle_started) / 1_000_000,
                        specs[kind].get("manual_observations", {}).get(str(cycle_number)),
                    ))
            else:
                for cycle_number in range(1, definition["candidate_cycle_count"] + 1):
                    skipped = {name: "environment_blocked" for name in definition["required_product_steps"]}
                    cycles.append({
                        "cycle": cycle_number,
                        "status": "environment_blocked",
                        "project_identity": None,
                        "repository_revision": identity["revision"],
                        "legacy_runtime_untouched": None,
                        "step_statuses": skipped,
                        "step_status_counts": {"environment_blocked": len(skipped)},
                        "quality_observations": quality_observations(skipped),
                        "measurements": {
                            name: None for name in definition["measurements"]
                        },
                        "accessibility": {"status": "skipped", "checks": {}},
                    })
            cycle_status = status_from_steps({str(item["cycle"]): item["status"] for item in cycles})
            independent = len({item.get("project_identity") for item in cycles if item.get("project_identity")}) == len(cycles)
            if not independent and cycle_status == "passed":
                cycle_status = "failed"
            repository_results.append({
                **identity,
                "status": cycle_status if identity["status"] == "passed" else identity["status"],
                "independent_fresh_runtime_cycles": independent,
                "cycles": cycles,
            })
    finally:
        v11.prepare_repository = original_prepare
    regression = run_fixture_regression(v11, raw_root, base_env, definition)
    accessibility = aggregate_accessibility(repository_results)
    try:
        maintained_decisions = v11.read_decision_revisit_assessment(DECISION_REGISTER)
    except (OSError, ValueError):
        maintained_decisions = v11.failed_decision_revisit_assessment(DECISION_REGISTER)
    observed_triggers = detect_revisit_triggers(repository_results, accessibility)
    clean_after = git_clean(ROOT)
    blockers: list[str] = []
    blockers.extend(
        f"{identity['class']}: {blocker}" for identity in identities for blocker in identity["blockers"]
    )
    if not clean_before or not clean_after:
        blockers.append("candidate worktree was not clean for the complete dogfood run")
    if not codex_authorized:
        blockers.append("fresh Codex dogfood turns were not authorized by the exact bounded Phase 8 assertion")
    if maintained_decisions.get("decision_revisit_trigger_assessment") != v11.OFFICIAL_REVISIT_ASSESSMENT:
        blockers.append("the accepted Decision register could not be assessed")
    if maintained_decisions.get("active_decision_revisit_triggers"):
        blockers.append("the accepted Decision register reports an active revisit trigger")
    if observed_triggers:
        blockers.append("dogfood evidence activates one or more accepted Decision revisit triggers")
    if accessibility.get("status") != "passed":
        blockers.append("accessibility evaluation has a blocker or unqualified criterion")
    for repository in repository_results:
        if repository["status"] != "passed":
            blockers.append(f"{repository['class']} repeated journey did not pass")
        for cycle in repository["cycles"]:
            incomplete_quality = sorted(
                name
                for name, observation in cycle["quality_observations"].items()
                if observation.get("status") != "passed"
            )
            if incomplete_quality:
                blockers.append(
                    f"{repository['class']} cycle {cycle['cycle']} has unqualified quality observations: "
                    + ", ".join(incomplete_quality)
                )
    if regression["status"] != "passed":
        blockers.append("maintained structural/fallback regression did not pass")
    status = aggregate_status(repository_results, regression, accessibility, blockers)
    replacement_pass_candidate = status == "passed" and not blockers
    result = {
        "kind": "phase8_dogfood_result",
        "candidate_head": candidate_head,
        "definition_sha256": sha256(DEFINITION),
        "started_at": started_at,
        "ended_at": utc_now(),
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
        "environment": {
            "operating_system": platform.system(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "candidate_worktree": {"clean_before": clean_before, "clean_after": clean_after},
        "status": status,
        "replacement_pass_candidate": replacement_pass_candidate,
        "blockers": blockers,
        "repositories": repository_results,
        "fixture_regression": regression,
        "accessibility": accessibility,
        "privacy_and_transmission": {
            "codex_transmission_required": True,
            "codex_transmission_authorized": codex_authorized,
            "authorization_assertion_id": authorization if codex_authorized else None,
            "authenticated_codex_turn_count_expected": 6,
            "commercial_semantic_provider_success_claimed": False,
            "raw_source_in_sanitized_result": False,
            "credentials_in_sanitized_result": False,
        },
        "decision_revisit": {
            "maintained_assessment": maintained_decisions.get("decision_revisit_trigger_assessment"),
            "maintained_active_triggers": maintained_decisions.get("active_decision_revisit_triggers"),
            "observed_active_triggers": observed_triggers,
        },
        "known_limits": [
            "Question relevance, decision comprehension, interruption cost, and document usefulness include agent-observed rather than human-subject evidence.",
            "Peak memory is unsupported because the reused maintained child runner does not expose a reliable per-operation peak measurement.",
            "A successful unavailable-provider recovery path is not commercial semantic-provider qualification.",
        ],
        "raw_evidence_retention": "ignored_phase8_raw_state_only",
        "sanitization": {
            "source_bodies": "excluded",
            "private_prompts": "excluded",
            "command_logs": "excluded",
            "credentials_and_provider_payloads": "excluded",
            "local_absolute_paths": "excluded",
        },
    }
    validate_result(result, definition)
    write_json(output / "dogfood-result.json", result)
    print(json.dumps({
        "status": status,
        "replacement_pass_candidate": replacement_pass_candidate,
        "candidate_head": candidate_head,
        "blockers": blockers,
        "result": "dogfood-result.json",
    }, indent=2, sort_keys=True))
    return 0 if replacement_pass_candidate else 1


def self_test() -> int:
    definition = load_definition()
    fake_steps = {name: "passed" for name in definition["required_product_steps"]}
    fake_cycle = {
        "cycle": 1,
        "status": "passed",
        "project_identity": "0" * 32,
        "repository_revision": "0" * 40,
        "legacy_runtime_untouched": True,
        "step_statuses": fake_steps,
        "step_status_counts": {"passed": len(fake_steps)},
        "quality_observations": {
            name: {"status": "passed", "basis": "synthetic bounded observation"}
            for name in definition["quality_observations"]
        },
        "measurements": {},
        "accessibility": {"status": "passed", "checks": {"visible_focus": "passed"}},
    }
    repositories = []
    for index, kind in enumerate(CLASSES):
        first = {**fake_cycle, "cycle": 1, "project_identity": f"{index + 1:032x}"}
        second = {**fake_cycle, "cycle": 2, "project_identity": f"{index + 11:032x}"}
        repositories.append({
            "class": kind,
            "status": "passed",
            "independent_fresh_runtime_cycles": True,
            "cycles": [first, second],
        })
    result = {
        "kind": "phase8_dogfood_result",
        "candidate_head": "0" * 40,
        "status": "passed",
        "replacement_pass_candidate": True,
        "blockers": [],
        "repositories": repositories,
        "candidate_worktree": {"clean_before": True, "clean_after": True},
        "fixture_regression": {"status": "passed"},
        "accessibility": {"status": "passed"},
        "decision_revisit": {"observed_active_triggers": []},
    }
    validate_result(result, definition)
    blocked = json.loads(json.dumps(result))
    blocked["status"] = "environment_blocked"
    blocked["replacement_pass_candidate"] = False
    blocked["blockers"] = ["synthetic missing repository"]
    validate_result(blocked, definition)
    leaked = json.loads(json.dumps(blocked))
    leaked["private_prompt"] = "private prompt body"
    try:
        validate_result(leaked, definition)
    except ValueError:
        pass
    else:
        raise AssertionError("sanitizer accepted private prompt content")
    active = json.loads(json.dumps(result))
    active["decision_revisit"]["observed_active_triggers"] = [{"decision_id": "Q5"}]
    try:
        validate_result(active, definition)
    except ValueError:
        pass
    else:
        raise AssertionError("replacement pass accepted an active Decision revisit trigger")
    parsed = parse_accessibility_html("<!doctype html><html><head><meta name=\"viewport\" content=\"width=device-width\"></head><body><h1>x</h1><a href=\"/\">x</a></body></html>", expected_language="en")
    if parsed["checks"]["document_html_language"] != "failed":
        raise AssertionError("missing HTML language was not detected")
    if viewer_start_failure_status("cannot bind: Operation not permitted (os error 1)") != "environment_blocked":
        raise AssertionError("sandbox viewer bind denial was not preserved as environment-blocked")
    if viewer_start_failure_status("viewer rejected the project") != "failed":
        raise AssertionError("product viewer startup failure was not preserved as failed")
    print(json.dumps({
        "status": "passed",
        "definition_sha256": sha256(DEFINITION),
        "required_product_steps": len(definition["required_product_steps"]),
        "repository_classes": list(CLASSES),
        "two_cycle_contract": "passed",
        "non_pass_preservation": "passed",
        "sanitization_regressions": "passed",
        "decision_revisit_blocking": "passed",
        "accessibility_blocker_detection": "passed",
        "viewer_environment_blocking": "passed",
        "v11_reuse_route": str(V11_HARNESS.relative_to(ROOT)),
    }, indent=2, sort_keys=True))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    run = subparsers.add_parser("run")
    run.add_argument("--candidate-head", required=True)
    run.add_argument("--repositories", required=True)
    run.add_argument("--output-dir", required=True)
    run.add_argument("--authorize-codex-transmission")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "self-test":
        return self_test()
    return run_evaluation(args)


if __name__ == "__main__":
    raise SystemExit(main())
