#!/usr/bin/env python3
"""Maintain private Phase 8 naturalistic-dogfood campaign evidence.

This module never starts a Codex session, grants repository trust, or reads
canonical SQLite contents.  It prepares and validates evidence produced by an
operator-owned campaign and reuses the maintained dogfood normalizer.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
from typing import Any, Callable

import harness
from codex_events import EvidenceError, command_is_repository_inspection, load_codex_capture


ROOT = Path(__file__).resolve().parents[3]
CLASSES = harness.CLASSES
MANUAL_OBSERVATIONS = (
    "question_relevance",
    "decision_comprehension",
    "interruption_cost",
    "document_fidelity_and_usefulness",
)
ACCESSIBILITY_OBSERVATIONS = (
    "keyboard_reachability",
    "visible_focus",
    "not_color_only",
    "narrow_and_zoomed_presentation",
)
MANAGED_STORES = (
    "canonical.sqlite3",
    "candidates.sqlite3",
    "privacy.sqlite3",
    "guarded.sqlite3",
    "forgetting.sqlite3",
)
RAW_NAMES = {"work.rollout.jsonl", "resume.rollout.jsonl"}
PROHIBITED_ARCHIVE_SUFFIXES = (".sqlite", ".sqlite3", ".db", "-wal", "-shm", "-journal")
PROJECT_ID = re.compile(r"[0-9a-f]{32}")


class CampaignError(ValueError):
    """A bounded campaign input or state is invalid."""


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read JSON: {path}") from error


def write_json(path: Path, value: Any) -> None:
    harness.write_json(path, value)


def relative(root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError as error:
        raise CampaignError("campaign artifact escaped the campaign root") from error


def campaign_file(root: Path) -> Path:
    return root / "campaign.json"


def load_campaign(root: Path) -> dict[str, Any]:
    value = read_json(campaign_file(root))
    if value.get("kind") != "phase8_dogfood_campaign" or value.get("schema_version") != 1:
        raise CampaignError("unexpected dogfood campaign metadata")
    if Path(value.get("campaign_root", "")).resolve() != root.resolve():
        raise CampaignError("campaign metadata is bound to a different root")
    return value


def save_campaign(root: Path, value: dict[str, Any]) -> None:
    write_json(campaign_file(root), value)


def cycle_key(kind: str, cycle: int) -> str:
    if kind not in CLASSES or cycle not in {1, 2}:
        raise CampaignError("cycle must identify one maintained repository class and repetition")
    return f"{kind}-cycle-{cycle}"


def cycle_root(root: Path, kind: str, cycle: int) -> Path:
    return root / "cycles" / cycle_key(kind, cycle)


def inventory_path(root: Path) -> Path:
    return root / "evidence-inventory.json"


def load_inventory(root: Path) -> dict[str, Any]:
    path = inventory_path(root)
    if not path.exists():
        return {"kind": "phase8_dogfood_evidence_inventory", "schema_version": 1, "artifacts": {}}
    value = read_json(path)
    if value.get("kind") != "phase8_dogfood_evidence_inventory":
        raise CampaignError("unexpected evidence inventory")
    return value


def verify_inventory(root: Path) -> None:
    inventory = load_inventory(root)
    for name, expected in sorted(inventory.get("artifacts", {}).items()):
        path = root / name
        if (
            not path.is_file()
            or path.stat().st_size != expected.get("bytes")
            or harness.sha256(path) != expected.get("sha256")
        ):
            raise CampaignError(f"evidence hash mismatch: {name}")


def register_artifact(root: Path, path: Path, *, replace: bool = False) -> None:
    name = relative(root, path)
    inventory = load_inventory(root)
    artifacts = inventory.setdefault("artifacts", {})
    if name in artifacts and not replace:
        raise CampaignError(f"evidence artifact is already sealed: {name}")
    artifacts[name] = {"bytes": path.stat().st_size, "sha256": harness.sha256(path)}
    write_json(inventory_path(root), inventory)


def copy_exact(source: Path, destination: Path) -> None:
    if destination.exists():
        raise CampaignError(f"capture destination already exists: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with source.open("rb") as incoming, destination.open("xb") as outgoing:
        shutil.copyfileobj(incoming, outgoing)
    if harness.sha256(source) != harness.sha256(destination):
        raise CampaignError("raw capture copy did not preserve source bytes")


def descriptor_skeleton(kind: str, cycle: int, revision: str) -> dict[str, Any]:
    return {
        "kind": "phase8_cycle_descriptor",
        "producer": "volicord_phase8_codex_event_normalizer",
        "repository_class": kind,
        "cycle": cycle,
        "repository_revision": revision,
        "work_user_task": "REPLACE with the frozen naturalistic work task",
        "fresh_resume_user_task": "REPLACE with the frozen naturalistic resume task",
        "decision_oracle": {
            "work_task_materiality_basis": "REPLACE with the material choice implied by the work task",
            "user_owned_dimension": "REPLACE with the user-owned decision dimension",
            "established_repository_facts": ["REPLACE with an established repository fact"],
            "why_repository_inspection_cannot_decide": "REPLACE with why inspection cannot decide",
            "viable_alternatives": ["REPLACE alternative A", "REPLACE alternative B"],
            "recommendation": "REPLACE with the independently researched recommendation",
            "expected_choice": "REPLACE with the user choice expected by the evaluator",
            "material_consequence": "REPLACE with the user-visible consequence",
        },
        "materiality_review": {
            "kind": "phase8_materiality_review",
            "classification": "unassessed",
            "decision_dimension": "REPLACE with the user-owned decision dimension",
            "reviewed_active_owner_references": ["rebuild/docs/design/open-decisions.md"],
            "established_repository_facts": ["REPLACE with an established repository fact"],
            "why_repository_facts_do_not_determine_choice": "REPLACE after independent review",
            "why_no_accepted_contract_determines_choice": "REPLACE after independent review",
            "why_not_explicitly_delegated_implementation_choice": "REPLACE after independent review",
            "user_visible_material_consequence": "REPLACE with the user-visible consequence",
            "independent_review": {
                "status": "pending",
                "reviewer_role": "campaign_preparation_independent_reviewer",
                "basis": "REPLACE after independent review",
            },
        },
    }


def run_checked(argv: list[str], *, cwd: Path = ROOT) -> dict[str, Any]:
    completed = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        raise CampaignError(f"campaign command failed with exit {completed.returncode}: {argv[0]}")
    try:
        return json.loads(completed.stdout) if completed.stdout.strip().startswith("{") else {}
    except json.JSONDecodeError as error:
        raise CampaignError(f"campaign command returned malformed JSON: {argv[0]}") from error


def install_candidate(root: Path) -> Path:
    prefix = root / "install"
    bootstrap = root / "bootstrap-runtime"
    completed = subprocess.run(
        [str(ROOT / "rebuild/install.sh"), "--prefix", str(prefix), "--runtime-dir", str(bootstrap)],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0:
        raise CampaignError("candidate-local install failed")
    binary = prefix / "bin/volicord"
    if not binary.is_file():
        raise CampaignError("candidate-local install did not produce volicord")
    return binary


def repository_spec_map(value: Any) -> dict[str, dict[str, Any]]:
    repositories = value.get("repositories") if isinstance(value, dict) else None
    if not isinstance(repositories, list) or tuple(item.get("class") for item in repositories) != CLASSES:
        raise CampaignError("repository input must contain the three ordered maintained classes")
    return {item["class"]: dict(item) for item in repositories}


def require_private_campaign_root(root: Path) -> None:
    try:
        repository_relative = root.relative_to(ROOT)
    except ValueError:
        return
    completed = subprocess.run(
        ["git", "check-ignore", "--quiet", "--", repository_relative.as_posix()],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0:
        raise CampaignError("a campaign inside the repository must use a Git-ignored path")


def clone_repository(source: Path, destination: Path, revision: str) -> None:
    completed = subprocess.run(
        ["git", "clone", "--quiet", "--no-hardlinks", str(source), str(destination)],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0:
        raise CampaignError("disposable cycle repository clone failed")
    completed = subprocess.run(
        ["git", "checkout", "--quiet", "--detach", revision],
        cwd=destination,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if (
        completed.returncode != 0
        or harness.git_head(destination) != revision
        or not harness.git_clean(destination)
    ):
        raise CampaignError("disposable cycle repository revision could not be pinned cleanly")


def activate_cycle(root: Path, kind: str, cycle: int) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    key = cycle_key(kind, cycle)
    state = campaign["cycles"][key]
    repository = Path(state["repository_path"])
    binary = Path(campaign["candidate_binary"])
    manifest = repository / ".codex/volicord-integration.json"
    if manifest.exists():
        run_checked([str(binary), "--runtime", state["runtime_home"], "codex", "disable", str(repository)])
    result = run_checked(
        [str(binary), "--runtime", state["runtime_home"], "codex", "enable", str(repository)]
    )
    if result.get("project_trust") != "user_controlled":
        raise CampaignError("Codex enable did not preserve user-controlled trust")
    state["codex_enabled"] = True
    campaign["active_cycle_by_repository"][kind] = cycle
    save_campaign(root, campaign)
    return result


def prepare_campaign(
    root: Path,
    campaign_id: str,
    candidate_head: str,
    repository_input: Path,
    *,
    candidate_binary: Path | None = None,
    enable: bool = True,
    cloner: Callable[[Path, Path, str], None] = clone_repository,
) -> dict[str, Any]:
    root = root.resolve()
    if root.exists() and any(root.iterdir()):
        raise CampaignError("campaign root must be absent or empty")
    require_private_campaign_root(root)
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{2,80}", campaign_id):
        raise CampaignError("campaign identity must be a bounded filesystem-safe value")
    actual_head = harness.git_head(ROOT)
    if actual_head != candidate_head or not harness.git_clean(ROOT):
        raise CampaignError("candidate HEAD/worktree does not match a clean requested candidate")
    definition = harness.load_definition()
    raw_input = read_json(repository_input)
    specs = repository_spec_map(raw_input)
    _, identities = harness.load_repository_specs(repository_input, candidate_head, definition)
    failures = [item for item in identities if item["status"] != "passed"]
    if failures:
        raise CampaignError("one or more source repository identities do not qualify")
    root.mkdir(parents=True, exist_ok=True)
    root.chmod(0o700)
    binary = candidate_binary.resolve() if candidate_binary else install_candidate(root)
    if not binary.is_file():
        raise CampaignError("candidate binary is unavailable")
    cycles: dict[str, Any] = {}
    for kind in CLASSES:
        spec = specs[kind]
        revision = candidate_head if kind == "volicord" else spec["revision"]
        for number in (1, 2):
            key = cycle_key(kind, number)
            destination = cycle_root(root, kind, number)
            (destination / "evidence").mkdir(parents=True)
            (destination / "runtime").mkdir()
            (destination / "documents").mkdir()
            repository = destination / "repository"
            cloner(Path(spec["path"]).resolve(), repository, revision)
            write_json(destination / "descriptor.json", descriptor_skeleton(kind, number, revision))
            write_json(destination / "observations/manual.json", harness.observation_template(MANUAL_OBSERVATIONS))
            write_json(
                destination / "observations/accessibility.json",
                harness.observation_template(ACCESSIBILITY_OBSERVATIONS),
            )
            cycles[key] = {
                "repository_class": kind,
                "cycle": number,
                "repository_path": str(repository.resolve()),
                "repository_revision": revision,
                "runtime_home": str((destination / "runtime").resolve()),
                "state": "prepared",
                "project_id": None,
                "codex_enabled": False,
            }
    campaign = {
        "kind": "phase8_dogfood_campaign",
        "schema_version": 1,
        "campaign_id": campaign_id,
        "campaign_root": str(root),
        "candidate_head": candidate_head,
        "candidate_binary": str(binary),
        "repository_input": relative(root, root / "repository-input.json"),
        "terminal_outcome": None,
        "active_cycle_by_repository": {},
        "cycles": cycles,
    }
    write_json(root / "repository-input.json", raw_input)
    save_campaign(root, campaign)
    write_json(inventory_path(root), load_inventory(root))
    run_sheet = root / "RUN-SHEET.md"
    run_sheet.write_text(
        "# Naturalistic Dogfood Run Sheet\n\n"
        "This helper never grants repository or hook trust and never starts a Codex session.\n\n"
        "For each cycle: review and complete `descriptor.json`; run `activate-cycle`; explicitly "
        "trust the repository and hook in VS Code; start the frozen work task in a fresh thread; "
        "collect-work; only when it reports `resume_allowed`, start the frozen resume task in a "
        "distinct fresh thread; then collect-resume and record-observation. Finalize only after all "
        "six cycles are complete. Preserve raw rollouts separately unless explicit archive inclusion "
        "is required.\n",
        encoding="utf-8",
    )
    if enable:
        for kind in CLASSES:
            activate_cycle(root, kind, 1)
    preparation = {
        "kind": "phase8_dogfood_campaign_preparation",
        "campaign_id": campaign_id,
        "candidate_head": candidate_head,
        "candidate_worktree_clean": True,
        "repository_identities": identities,
        "cycle_count": 6,
        "candidate_local_install": str(binary),
        "repository_trust": "user_controlled_not_automated",
    }
    write_json(root / "preparation.json", preparation)
    for path in (root / "repository-input.json", run_sheet, root / "preparation.json"):
        register_artifact(root, path)
    return preparation


def observed_project_ids(capture: Any) -> list[str]:
    values = {
        str(call.result.get("project_id"))
        for operation in ("project_initialize", "project_resolve")
        for call in capture.successful_calls(operation)
        if PROJECT_ID.fullmatch(str(call.result.get("project_id", "")))
    }
    return sorted(values)


def update_activation_summary(root: Path, kind: str, cycle: int, **updates: Any) -> Path:
    path = cycle_root(root, kind, cycle) / "activation-summary.json"
    current = read_json(path) if path.exists() else {
        "kind": "phase8_dogfood_activation_summary",
        "repository_class": kind,
        "cycle": cycle,
        "repository_config_present": (Path(load_campaign(root)["cycles"][cycle_key(kind, cycle)]["repository_path"]) / ".codex/config.toml").is_file(),
        "repository_ownership_manifest_present": (Path(load_campaign(root)["cycles"][cycle_key(kind, cycle)]["repository_path"]) / ".codex/volicord-integration.json").is_file(),
        "work_session_start_activation_observed": None,
        "resume_session_start_activation_observed": None,
    }
    current.update(updates)
    write_json(path, current)
    return path


def collect_work(root: Path, kind: str, cycle: int, raw_capture: Path) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    if campaign.get("terminal_outcome") is not None:
        raise CampaignError("campaign already stopped; create a new campaign identity")
    key = cycle_key(kind, cycle)
    state = campaign["cycles"][key]
    if state["state"] != "prepared":
        raise CampaignError("work collection requires a prepared cycle")
    descriptor_path = cycle_root(root, kind, cycle) / "descriptor.json"
    descriptor = read_json(descriptor_path)
    errors = harness.cycle_descriptor_errors(descriptor)
    if errors:
        raise CampaignError("descriptor does not qualify: " + "; ".join(errors))
    destination = cycle_root(root, kind, cycle) / "evidence/work.rollout.jsonl"
    copy_exact(raw_capture.resolve(), destination)
    try:
        capture = load_codex_capture(destination)
    except (OSError, EvidenceError) as error:
        raise CampaignError("work rollout is not a supported normalized Codex capture") from error
    project_ids = observed_project_ids(capture)
    result: dict[str, Any]
    try:
        blocker = harness.build_work_blocker_result(
            campaign["candidate_head"], descriptor, harness.sha256(descriptor_path), capture
        )
    except ValueError as error:
        if "has no machine-observable terminal work blocker" not in str(error):
            raise CampaignError(str(error)) from error
        if len(project_ids) != 1:
            raise CampaignError("qualifying work capture must expose one Project identity")
        result = {
            "kind": "phase8_dogfood_work_intake",
            "outcome": "resume_allowed",
            "repository_class": kind,
            "cycle": cycle,
            "project_id": project_ids[0],
            "work_capture_sha256": capture.source_sha256,
            "repository_scoped_activation_observed": True,
        }
        state["state"] = "work_collected"
        state["project_id"] = project_ids[0]
        state["work_session_id"] = capture.session_id
    else:
        result = blocker
        state["state"] = blocker["outcome"]
        campaign["terminal_outcome"] = blocker["outcome"]
        write_json(cycle_root(root, kind, cycle) / "blocker-result.json", blocker)
        register_artifact(root, cycle_root(root, kind, cycle) / "blocker-result.json")
    write_json(cycle_root(root, kind, cycle) / "work-intake.json", result)
    activation = update_activation_summary(
        root,
        kind,
        cycle,
        work_session_start_activation_observed=capture.repository_scoped_activation_observed,
    )
    save_campaign(root, campaign)
    for path in (destination, descriptor_path, cycle_root(root, kind, cycle) / "work-intake.json", activation):
        register_artifact(root, path)
    return result


def inspect_resume(capture: Any, descriptor: dict[str, Any], state: dict[str, Any]) -> str:
    if (
        capture.source != "vscode"
        or capture.originator != "codex_vscode"
        or not capture.fresh_user_thread
        or capture.git_revision != state["repository_revision"]
        or not capture.user_turns
        or not harness.codex_user_turn_transport_identity_matches(
            capture.user_turns[0].text, descriptor["fresh_resume_user_task"]
        )
        or not capture.repository_scoped_activation_observed
    ):
        raise CampaignError("resume capture does not match the frozen fresh VS Code cycle contract")
    if capture.session_id == state.get("work_session_id"):
        raise CampaignError("resume capture must come from a distinct fresh session")
    resolves = capture.successful_calls("project_resolve")
    recalls = capture.successful_calls("recall")
    if len(resolves) != 1 or len(recalls) != 1 or capture.successful_calls("project_initialize"):
        raise CampaignError("resume must resolve one existing Project and must not initialize a replacement")
    resolve, recall = resolves[0], recalls[0]
    project_id = resolve.result.get("project_id")
    if (
        resolve.result.get("status") != "found"
        or not PROJECT_ID.fullmatch(str(project_id or ""))
        or project_id != recall.arguments.get("project_id")
        or project_id != recall.result.get("project_id")
        or project_id != state.get("project_id")
        or resolve.sequence >= recall.sequence
    ):
        raise CampaignError("resume Project resolution/Recall identity or ordering is invalid")
    if any(
        command.sequence < recall.sequence and command_is_repository_inspection(command.parsed_command)
        for command in capture.commands
    ) or any(
        call.sequence < recall.sequence
        for operation in ("repository_analyze", "inquiry_frontier", "checkpoint_record")
        for call in capture.calls(operation)
    ) or any(item.sequence < recall.sequence for item in capture.path_observations):
        raise CampaignError("resume inspected or changed the repository before Recall")
    return str(project_id)


def default_export(binary: Path, runtime: Path, project_id: str, destination: Path) -> None:
    completed = subprocess.run(
        [str(binary), "--runtime", str(runtime), "portable", "export", project_id, str(destination)],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0 or not destination.is_file():
        raise CampaignError("candidate portable export failed")


def runtime_summary(runtime: Path, repository: Path, work_activation: bool, resume_activation: bool) -> dict[str, Any]:
    managed: list[dict[str, Any]] = []
    for name in MANAGED_STORES:
        path = runtime / name
        if path.is_file():
            managed.append({"logical_name": name, "bytes": path.stat().st_size})
        for suffix in ("-wal", "-shm", "-journal"):
            sidecar = runtime / f"{name}{suffix}"
            if sidecar.is_file():
                managed.append({"logical_name": f"{name}{suffix}", "bytes": sidecar.stat().st_size})
    lock = runtime / "mutation.lock"
    if lock.is_file():
        managed.append({"logical_name": "mutation.lock", "bytes": lock.stat().st_size})
    return {
        "kind": "phase8_bounded_runtime_summary",
        "runtime_home_bytes": harness.directory_bytes(runtime),
        "derived_analysis_bytes": harness.directory_bytes(runtime / "derived/analysis"),
        "managed_file_inventory": sorted(managed, key=lambda item: item["logical_name"]),
        "repository_config_present": (repository / ".codex/config.toml").is_file(),
        "repository_ownership_manifest_present": (repository / ".codex/volicord-integration.json").is_file(),
        "work_session_start_activation_observed": work_activation,
        "resume_session_start_activation_observed": resume_activation,
        "content_included": False,
    }


def generate_document(binary: Path, runtime: Path, project_id: str, destination: Path) -> dict[str, Any]:
    completed = subprocess.run(
        [str(binary), "--runtime", str(runtime), "documents", "export", project_id,
         "project-architecture-guide", "html", str(destination), "en"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode == 0 and destination.is_file():
        return {"status": "passed", "file": destination.name, "bytes": destination.stat().st_size, "sha256": harness.sha256(destination)}
    return {"status": "skipped", "basis": "current document prerequisites or export were unavailable"}


def collect_resume(
    root: Path,
    kind: str,
    cycle: int,
    raw_capture: Path,
    *,
    exporter: Callable[[Path, Path, str, Path], None] = default_export,
    documenter: Callable[[Path, Path, str, Path], dict[str, Any]] = generate_document,
) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    if campaign.get("terminal_outcome") is not None:
        raise CampaignError("later collection is blocked; create a new campaign identity")
    key = cycle_key(kind, cycle)
    state = campaign["cycles"][key]
    if state["state"] != "work_collected":
        raise CampaignError("resume collection requires a resume_allowed work intake")
    destination = cycle_root(root, kind, cycle) / "evidence/resume.rollout.jsonl"
    copy_exact(raw_capture.resolve(), destination)
    try:
        capture = load_codex_capture(destination)
    except (OSError, EvidenceError) as error:
        raise CampaignError("resume rollout is not a supported normalized Codex capture") from error
    descriptor_path = cycle_root(root, kind, cycle) / "descriptor.json"
    descriptor = read_json(descriptor_path)
    project_id = inspect_resume(capture, descriptor, state)
    binary = Path(campaign["candidate_binary"])
    runtime = Path(state["runtime_home"])
    bundle = cycle_root(root, kind, cycle) / "context.bundle.json"
    exporter(binary, runtime, project_id, bundle)
    try:
        canonical = harness.load_canonical_bundle(bundle)
    except (OSError, EvidenceError) as error:
        raise CampaignError("portable export is not a supported canonical bundle") from error
    if canonical.project_id != project_id:
        raise CampaignError("portable bundle Project identity does not match the resume capture")
    descriptor["evidence"] = {
        "captures": {
            "work": {"file": "evidence/work.rollout.jsonl", "sha256": harness.sha256(cycle_root(root, kind, cycle) / "evidence/work.rollout.jsonl")},
            "resume": {"file": "evidence/resume.rollout.jsonl", "sha256": harness.sha256(destination)},
        },
        "canonical_bundle": {"file": "context.bundle.json", "sha256": harness.sha256(bundle)},
    }
    errors = harness.cycle_descriptor_errors(descriptor)
    if errors:
        raise CampaignError("completed descriptor does not qualify: " + "; ".join(errors))
    write_json(descriptor_path, descriptor)
    summary_path = cycle_root(root, kind, cycle) / "runtime-summary.json"
    activation_path = update_activation_summary(
        root, kind, cycle, resume_session_start_activation_observed=True
    )
    activation = read_json(activation_path)
    write_json(
        summary_path,
        runtime_summary(
            runtime,
            Path(state["repository_path"]),
            bool(activation["work_session_start_activation_observed"]),
            True,
        ),
    )
    document_path = cycle_root(root, kind, cycle) / "documents/project-architecture-guide.html"
    document_result = documenter(binary, runtime, project_id, document_path)
    document_summary = cycle_root(root, kind, cycle) / "documents-summary.json"
    write_json(document_summary, document_result)
    state["state"] = "resume_collected"
    state["resume_session_id"] = capture.session_id
    state["bundle_sha256"] = harness.sha256(bundle)
    save_campaign(root, campaign)
    for path in (destination, bundle, descriptor_path, summary_path, activation_path, document_summary):
        register_artifact(root, path, replace=path in {descriptor_path, activation_path})
    if document_result.get("status") == "passed":
        register_artifact(root, document_path)
    return {
        "kind": "phase8_dogfood_resume_intake",
        "outcome": "evidence_collected",
        "repository_class": kind,
        "cycle": cycle,
        "project_id": project_id,
        "resume_capture_sha256": capture.source_sha256,
        "canonical_bundle_sha256": harness.sha256(bundle),
        "descriptor_evidence_completed": True,
        "runtime_home_copied": False,
        "document_evidence": document_result,
    }


def record_observation(root: Path, kind: str, cycle: int, scope: str, name: str, status: str, basis: str) -> dict[str, str]:
    load_campaign(root)
    verify_inventory(root)
    names = MANUAL_OBSERVATIONS if scope == "manual" else ACCESSIBILITY_OBSERVATIONS if scope == "accessibility" else ()
    if name not in names:
        raise CampaignError("observation name is not permitted for the selected scope")
    observation = {"status": status, "basis": basis}
    harness.validate_observation_object(observation, name)
    path = cycle_root(root, kind, cycle) / f"observations/{scope}.json"
    values = read_json(path)
    values[name] = observation
    for item_name, item in values.items():
        harness.validate_observation_object(item, item_name)
    write_json(path, values)
    return observation


def finalize_manifest(root: Path, output: Path | None = None) -> Path:
    campaign = load_campaign(root)
    verify_inventory(root)
    if campaign.get("terminal_outcome") is not None:
        raise CampaignError("a stopped campaign cannot produce a qualifying repository manifest")
    specs = repository_spec_map(read_json(root / campaign["repository_input"]))
    repositories: list[dict[str, Any]] = []
    for kind in CLASSES:
        spec = specs[kind]
        real: dict[str, str] = {}
        manual: dict[str, Any] = {}
        accessibility: dict[str, Any] = {}
        for number in (1, 2):
            state = campaign["cycles"][cycle_key(kind, number)]
            if state["state"] != "resume_collected":
                raise CampaignError("all six cycles must have resume evidence before finalization")
            descriptor = cycle_root(root, kind, number) / "descriptor.json"
            if harness.cycle_descriptor_errors(read_json(descriptor)):
                raise CampaignError("manifest cannot reference an invalid completed descriptor")
            real[str(number)] = relative(root, descriptor)
            manual[str(number)] = read_json(cycle_root(root, kind, number) / "observations/manual.json")
            accessibility[str(number)] = read_json(cycle_root(root, kind, number) / "observations/accessibility.json")
            for values in (manual[str(number)], accessibility[str(number)]):
                for name, observation in values.items():
                    harness.validate_observation_object(observation, name)
        repositories.append({
            **{key: spec[key] for key in ("class", "path", "origin", "revision", "license_file", "license_spdx", "provider_source_path") if key in spec},
            "revision": campaign["candidate_head"] if kind == "volicord" else spec["revision"],
            "real_session_evidence": real,
            "manual_observations": manual,
            "accessibility_observations": accessibility,
        })
    destination = output.resolve() if output else root / "repositories.json"
    if destination.parent != root:
        raise CampaignError("repository manifest must remain at the campaign root")
    write_json(destination, {"repositories": repositories})
    register_artifact(
        root,
        destination,
        replace=relative(root, destination) in load_inventory(root)["artifacts"],
    )
    return destination


def safe_archive_artifact(name: str, *, include_raw: bool) -> bool:
    path = Path(name)
    lowered = name.casefold()
    if path.name in RAW_NAMES:
        return include_raw
    if any(lowered.endswith(suffix) for suffix in PROHIBITED_ARCHIVE_SUFFIXES):
        return False
    if any(part in {"runtime", "install", "bootstrap-runtime", "derived"} for part in path.parts):
        return False
    return True


def tar_info(name: str, size: int) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name)
    info.size = size
    info.mtime = 0
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    info.mode = 0o600
    return info


def build_review_package(root: Path, output: Path, *, include_raw: bool = False) -> Path:
    campaign = load_campaign(root)
    verify_inventory(root)
    manifest = root / "repositories.json"
    if not manifest.is_file():
        raise CampaignError("finalize the repository manifest before review packaging")
    campaign_projection = {
        "kind": campaign["kind"],
        "schema_version": campaign["schema_version"],
        "campaign_id": campaign["campaign_id"],
        "candidate_head": campaign["candidate_head"],
        "terminal_outcome": campaign["terminal_outcome"],
        "cycles": {
            key: {
                field: value[field]
                for field in (
                    "repository_class",
                    "cycle",
                    "repository_revision",
                    "state",
                    "project_id",
                    "bundle_sha256",
                )
                if field in value
            }
            for key, value in sorted(campaign["cycles"].items())
        },
        "repository_and_hook_trust": "user_controlled_not_automated",
    }
    preparation = read_json(root / "preparation.json")
    preparation["candidate_local_install"] = "campaign_private_install_completed"
    files: dict[str, bytes] = {
        "campaign.json": (json.dumps(campaign_projection, indent=2, sort_keys=True) + "\n").encode(),
        "preparation.json": (json.dumps(preparation, indent=2, sort_keys=True) + "\n").encode(),
        "repositories.json": manifest.read_bytes(),
        "evidence-inventory.json": inventory_path(root).read_bytes(),
        "RUN-SHEET.md": (root / "RUN-SHEET.md").read_bytes(),
    }
    inventory = load_inventory(root)
    review_index: list[dict[str, Any]] = []
    for name in sorted(inventory["artifacts"]):
        if name in files or name == "repository-input.json":
            continue
        if not safe_archive_artifact(name, include_raw=include_raw):
            continue
        path = root / name
        files[name] = path.read_bytes()
        if name.endswith("/descriptor.json"):
            descriptor = read_json(path)
            review_name = f"materiality-reviews/{cycle_key(descriptor['repository_class'], descriptor['cycle'])}.json"
            review_bytes = (json.dumps(descriptor["materiality_review"], indent=2, sort_keys=True) + "\n").encode()
            files[review_name] = review_bytes
            review_index.append({
                "repository_class": descriptor["repository_class"],
                "cycle": descriptor["cycle"],
                "derived_archive_entry": review_name,
                "authoritative_descriptor": name,
                "sha256": hashlib.sha256(review_bytes).hexdigest(),
            })
    if len(review_index) != 6:
        raise CampaignError("review package requires six completed descriptors and materiality reviews")
    files["materiality-reviews/index.json"] = (
        json.dumps({"kind": "phase8_materiality_review_index", "reviews": review_index}, indent=2, sort_keys=True) + "\n"
    ).encode()
    for name, content in files.items():
        lowered = content.lower()
        if any(marker.encode() in lowered for marker in harness.SECRET_MARKERS):
            raise CampaignError(f"review artifact contains a prohibited secret marker: {name}")
    for name in files:
        if not safe_archive_artifact(name, include_raw=include_raw):
            raise CampaignError(f"prohibited review artifact selected: {name}")
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise CampaignError("review archive destination already exists")
    with output.open("xb") as raw:
        with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                for name, content in sorted(files.items()):
                    archive.addfile(tar_info(name, len(content)), io.BytesIO(content))
    output.chmod(0o600)
    return output


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="command", required=True)
    prepare = sub.add_parser("prepare")
    prepare.add_argument("--campaign-root", required=True)
    prepare.add_argument("--campaign-id", required=True)
    prepare.add_argument("--candidate-head", required=True)
    prepare.add_argument("--repositories", required=True)
    activate = sub.add_parser("activate-cycle")
    collect_w = sub.add_parser("collect-work")
    collect_r = sub.add_parser("collect-resume")
    observe = sub.add_parser("record-observation")
    finalize = sub.add_parser("finalize-manifest")
    package = sub.add_parser("package-review")
    for command in (activate, collect_w, collect_r, observe):
        command.add_argument("--campaign-root", required=True)
        command.add_argument("--repository-class", choices=CLASSES, required=True)
        command.add_argument("--cycle", choices=(1, 2), type=int, required=True)
    collect_w.add_argument("--raw-rollout", required=True)
    collect_r.add_argument("--raw-rollout", required=True)
    observe.add_argument("--scope", choices=("manual", "accessibility"), required=True)
    observe.add_argument("--name", required=True)
    observe.add_argument("--status", choices=sorted(harness.ALLOWED_STATUS), required=True)
    observe.add_argument("--basis", required=True)
    finalize.add_argument("--campaign-root", required=True)
    package.add_argument("--campaign-root", required=True)
    package.add_argument("--output", required=True)
    package.add_argument("--include-raw-rollouts", action="store_true")
    return result


def main() -> int:
    args = parser().parse_args()
    root = Path(args.campaign_root).resolve()
    if args.command == "prepare":
        value = prepare_campaign(root, args.campaign_id, args.candidate_head, Path(args.repositories).resolve())
    elif args.command == "activate-cycle":
        value = activate_cycle(root, args.repository_class, args.cycle)
    elif args.command == "collect-work":
        value = collect_work(root, args.repository_class, args.cycle, Path(args.raw_rollout))
    elif args.command == "collect-resume":
        value = collect_resume(root, args.repository_class, args.cycle, Path(args.raw_rollout))
    elif args.command == "record-observation":
        value = record_observation(root, args.repository_class, args.cycle, args.scope, args.name, args.status, args.basis)
    elif args.command == "finalize-manifest":
        value = {"manifest": str(finalize_manifest(root))}
    else:
        value = {"archive": str(build_review_package(root, Path(args.output), include_raw=args.include_raw_rollouts))}
    print(json.dumps(value, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CampaignError, EvidenceError, OSError, ValueError) as error:
        print(json.dumps({"status": "failed", "error": str(error)}, sort_keys=True))
        raise SystemExit(1)
