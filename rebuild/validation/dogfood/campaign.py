#!/usr/bin/env python3
"""Maintain private Phase 8 naturalistic-dogfood campaign evidence.

This module never starts a Codex session, grants repository trust, or reads
canonical SQLite contents.  It prepares and validates evidence produced by an
operator-owned campaign and reuses the maintained dogfood normalizer.
"""

from __future__ import annotations

import argparse
from collections import Counter
from contextlib import contextmanager
import copy
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import tarfile
import tempfile
import time
import tomllib
from typing import Any, Callable

import harness
import evidence_purpose
import workload_intents
import interaction_diagnostics
import cli_observations
import document_realization
import explanation_evidence
import machine_findings
import review_operations
import human_review
import result_lineage
import repository_state
import resource_observer
import collection_runs
from codex_events import EvidenceError, command_is_repository_inspection, load_codex_capture


ROOT = Path(__file__).resolve().parents[3]
CLASSES = harness.CLASSES
WORK_SLOTS_BY_REPOSITORY = harness.WORK_SLOTS_BY_REPOSITORY
QUALIFICATION_WORK_COUNT = harness.QUALIFICATION_WORK_COUNT
DOCUMENT_KINDS = (
    "project-architecture-guide",
    "decision-report",
    "implementation-plan",
    "handoff-resume",
)
DOCUMENT_FORMATS = (
    ("markdown", "md"),
    ("html", "html"),
)
MANAGED_STORES = (
    "canonical.sqlite3",
    "candidates.sqlite3",
    "privacy.sqlite3",
    "guarded.sqlite3",
    "forgetting.sqlite3",
)
RAW_NAMES = {"start.rollout.jsonl", "resume.rollout.jsonl"}
PROHIBITED_ARCHIVE_SUFFIXES = (".sqlite", ".sqlite3", ".db", "-wal", "-shm", "-journal")
PROJECT_ID = re.compile(r"[0-9a-f]{32}")
BATCH_CAPTURE_COUNT = harness.QUALIFICATION_SESSION_COUNT
# Optional closed command/result envelope observes status and HEAD without prescribing Git work.
GIT_STATE_CHECK = (
    "git --no-optional-locks -c core.fsmonitor=false status "
    "--porcelain=v1 --untracked-files=all && git rev-parse HEAD"
)
WORK_SLOT_ID = re.compile(r"journey-(?:volicord|small-python|polyglot-medium)-work-[abc]")
CANDIDATE_ARTIFACTS = ("volicord", "volicord-mcp", "volicord-viewer")


class CampaignError(ValueError):
    """A bounded campaign input or state is invalid."""

    def __init__(self, message: str, *, diagnostic: dict[str, Any] | None = None):
        super().__init__(message)
        self.diagnostic = diagnostic


class IntegrityError(CampaignError):
    def __init__(self, rule: str, error: Exception):
        super().__init__(str(error), diagnostic={"kind": "dogfood_integrity_rejection",
            "finding": machine_findings.finding(rule, "confirmed_violation",
                {"error_kind": type(error).__name__, "detail": str(error)}),
            "collection_state": "rejected", "qualification_state": "not_run"})


def integrity_check(rule, operation, *args):
    try:
        return operation(*args)
    except (CampaignError, EvidenceError, OSError, repository_state.StateError) as error:
        raise IntegrityError(rule, error) from error


RESUME_FAILURE_DOMAINS = {
    "recall_transport_incomplete": "evidence",
    "recall_identity_or_project_invalid": "evidence",
    "recall_operation_failed": "product_integration",
    "pre_recall_repository_access_or_order_violation": "behavior_contract",
    "baseline_invalid": "behavior_contract",
    "scope_or_authority_missing": "behavior_contract",
    "post_change_validation_missing": "behavior_contract",
    "terminal_validation_failed": "product_integration",
    "terminal_validation_indeterminate": "evidence",
    "validator_invariant_failure": "validation_internal",
}


class ResumeContractError(CampaignError):
    """Finite evidence basis, never routed by exception message text."""

    def __init__(self, basis: str):
        if basis not in RESUME_FAILURE_DOMAINS:
            raise AssertionError("unknown resume failure basis")
        self.basis = basis
        self.domain = RESUME_FAILURE_DOMAINS[basis]
        self.check = basis
        super().__init__(basis)


@dataclass(frozen=True)
class MappedRollout:
    source: Path
    capture: Any
    task_transport: harness.FrozenTaskTransportComparison


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read JSON: {path}") from error


def write_json(path: Path, value: Any) -> None:
    harness.write_json(path, value)


def json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def atomic_write_bytes(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(content)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def relative(root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError as error:
        raise CampaignError("campaign artifact escaped the campaign root") from error


def campaign_file(root: Path) -> Path:
    return root / "campaign.json"


def load_campaign(root: Path, *, validate_private: bool = True) -> dict[str, Any]:
    if (root / "batch-publication.json").exists():
        raise CampaignError(
            "batch publication is incomplete; preserve its journal and create a new campaign identity",
            diagnostic={"kind": "phase8_dogfood_batch_publication_failure",
                        "domain": "evidence", "basis": "batch_publication_incomplete",
                        "qualification_state": "not_run", "outcome": "repair_required"},
        )
    value = read_json(campaign_file(root))
    if (value.get("kind") != "phase8_dogfood_campaign"
            or value.get("schema_version") != 9):
        raise CampaignError("unexpected dogfood campaign metadata")
    if Path(value.get("campaign_root", "")).resolve() != root.resolve():
        raise CampaignError("campaign metadata is bound to a different root")
    evidence_purpose.validate(value.get("evidence_purpose"))
    evidence_purpose.require_same(value, value["naturalistic_memory_evidence"])
    try:
        resource_observer.validate(value.get("naturalistic_memory_evidence", {}),
            value.get("candidate_artifacts", {}).get("volicord-mcp", {}).get("sha256"))
    except (ValueError, TypeError, KeyError) as error:
        raise CampaignError("invalid naturalistic MCP resource evidence") from error
    if value.get("live_evidence_obligations") != live_evidence_obligations(
            value.get("candidate_artifacts", {}), value["naturalistic_memory_evidence"]):
        raise CampaignError("live evidence obligation classification changed")
    return value


def require_current_candidate(candidate_head: str) -> None:
    if harness.git_head(ROOT) != candidate_head or not harness.git_clean(ROOT):
        raise CampaignError(
            "campaign mutation requires its bound candidate to be the current clean qualifying HEAD"
        )


def bind_candidate_artifacts(binary: Path) -> dict[str, dict[str, str]]:
    binary = binary.resolve()
    paths = {
        "volicord": binary,
        "volicord-mcp": binary.with_name("volicord-mcp").resolve(),
        "volicord-viewer": binary.with_name("volicord-viewer").resolve(),
    }
    bindings: dict[str, dict[str, str]] = {}
    for name in CANDIDATE_ARTIFACTS:
        path = paths[name]
        if not path.is_file() or not os.access(path, os.X_OK):
            raise CampaignError(f"candidate executable is unavailable: {name}")
        bindings[name] = {"path": str(path), "sha256": harness.sha256(path)}
    return bindings


def naturalistic_memory_evidence(
    candidate_artifacts: dict[str, dict[str, str]],
) -> dict[str, Any]:
    """Initial truthful state; observation is explicitly attached later."""
    return resource_observer.initial(candidate_artifacts)


def live_evidence_obligations(
    candidate_artifacts: dict[str, dict[str, str]], resource=None,
) -> dict[str, Any]:
    """Keep deterministic support distinct from required live/naturalistic evidence."""
    return {
        "kind": "dogfood_live_evidence_obligations",
        "schema_version": 1,
        "live_viewer": {
            "status": "required_human_observation",
            "locales": ["en", "ko"],
            "required_scope": [
                "accessibility",
                "decision_comprehension_when_applicable",
                "browser_input_and_paint_responsiveness",
            ],
            "static_html_may_substitute": False,
            "agent_review_may_substitute": False,
        },
        "viewer_performance": {
            "snapshot_export_request": "candidate_bound_monotonic_proxy_measured_during_collection",
            "browser_input_and_paint": "unmeasured_until_direct_live_observation",
            "proxy_may_be_relabelled_browser_latency": False,
        },
        "naturalistic_resource": resource if resource is not None else naturalistic_memory_evidence(candidate_artifacts),
    }


def verify_candidate_artifacts(
    campaign: dict[str, Any], names: tuple[str, ...] = CANDIDATE_ARTIFACTS
) -> dict[str, dict[str, str]]:
    bindings = campaign.get("candidate_artifacts")
    if (not isinstance(bindings, dict) or len(bindings) != len(CANDIDATE_ARTIFACTS)
            or set(bindings) != set(CANDIDATE_ARTIFACTS)):
        raise CampaignError("campaign candidate executable bindings are missing or malformed")
    if campaign.get("candidate_binary") != bindings.get("volicord", {}).get("path"):
        raise CampaignError("campaign candidate executable path binding changed")
    for name in names:
        if name not in CANDIDATE_ARTIFACTS:
            raise CampaignError("unknown candidate executable binding")
        binding = bindings.get(name)
        if (
            not isinstance(binding, dict)
            or set(binding) != {"path", "sha256"}
            or not isinstance(binding.get("path"), str)
            or not isinstance(binding.get("sha256"), str)
            or re.fullmatch(r"[0-9a-f]{64}", binding["sha256"]) is None
        ):
            raise CampaignError("campaign candidate executable binding is malformed")
        path = Path(binding["path"])
        if not path.is_absolute() or path.resolve(strict=False) != path:
            raise CampaignError("campaign candidate executable path binding changed")
        if not path.is_file() or not os.access(path, os.X_OK):
            raise CampaignError(f"candidate executable is unavailable: {name}")
        if harness.sha256(path) != binding["sha256"]:
            raise CampaignError(f"candidate executable content mismatch: {name}")
    return bindings


@contextmanager
def candidate_artifact_use(
    campaign: dict[str, Any], names: tuple[str, ...] = CANDIDATE_ARTIFACTS
):
    verify_candidate_artifacts(campaign, names)
    try:
        yield
    finally:
        verify_candidate_artifacts(campaign, names)


def load_campaign_for_mutation(
    root: Path, *, validate_private: bool = True
) -> dict[str, Any]:
    campaign = load_campaign(root, validate_private=validate_private)
    require_current_candidate(campaign.get("candidate_head", ""))
    return campaign


def save_campaign(root: Path, value: dict[str, Any]) -> None:
    write_json(campaign_file(root), value)


def journey_id(kind: str) -> str:
    try:
        return harness.journey_id(kind)
    except ValueError as error:
        raise CampaignError(str(error)) from error


def work_key(kind: str, work_label: str) -> str:
    try:
        return harness.work_slot_id(kind, work_label)
    except ValueError as error:
        raise CampaignError(str(error)) from error


def work_labels(kind: str) -> tuple[str, ...]:
    try:
        return harness.work_slots(kind)
    except ValueError as error:
        raise CampaignError(str(error)) from error


def session_roles(kind: str, work_label: str) -> tuple[str, ...]:
    try:
        return harness.session_roles(kind, work_label)
    except ValueError as error:
        raise CampaignError(str(error)) from error


def session_slot_id(kind: str, work_label: str, role: str) -> str:
    try:
        return harness.session_slot_id(kind, work_label, role)
    except ValueError as error:
        raise CampaignError(str(error)) from error


def revision_is_bound(repository: Path, baseline: str, observed: str | None) -> bool:
    """Known revisions corroborate pinned repository history; absence stays unknown."""
    if observed is None:
        return True  # cwd/canonical/integration binding remains required; Git metadata may be absent.
    if observed == baseline:
        return True
    if not re.fullmatch(r"[0-9a-f]{40}", str(observed or "")):
        return False
    completed = subprocess.run(
        ["git", "merge-base", "--is-ancestor", baseline, observed],
        cwd=repository,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return completed.returncode == 0


def committed_delta_paths(repository: Path, base: str, boundary: str) -> list[str]:
    """Exact net tree delta, including both rename leaves, without pathspec parsing."""
    return sorted(repository_state.path_text(path) for path in repository_state.git(
        repository, "diff", "--name-only", "-z", "--no-renames", "--no-ext-diff",
        "--no-textconv", base, boundary, "--").split(b"\0") if path)


def session_git_state_observations(capture: Any) -> list[dict[str, Any]]:
    """Optional structured status/HEAD evidence, never a completion requirement."""
    observations = []
    mutations = harness.meaningful_work_path_observations(capture)
    last_mutation = max((item.sequence for item in mutations), default=0)
    first_mutation = min((item.sequence for item in mutations), default=None)
    for command in capture.commands:
        value = command.parsed_command
        if not isinstance(value, dict) or value.get("cmd") != GIT_STATE_CHECK:
            continue
        workdir = value.get("workdir", str(capture.cwd))
        if (not isinstance(workdir, str)
                or Path(workdir).resolve(strict=False) != capture.cwd.resolve(strict=False)
                or set(value) - {"cmd", "workdir", "max_output_tokens", "yield_time_ms",
                    "login", "shell", "tty", "sandbox_permissions", "justification", "prefix_rule"}
                or value.get("shell") not in {None, "/bin/sh", "/bin/bash", "/bin/zsh"}):
            continue
        if (command.evidence_state != "completed" or type(command.exit_code) is not int
                or command.exit_code != 0 or command.termination != "exited"
                or command.completion_sequence < command.sequence):
            continue
        match = re.fullmatch(r"(.*)([0-9a-f]{40})\n", command.output, re.DOTALL)
        if match is None or any(len(line) < 4 or line[2] != " "
                                for line in match[1].splitlines()):
            continue
        terminal = (command.sequence > last_mutation and not any(
            other is not command and other.completion_sequence >= command.sequence
            for other in capture.commands))
        observations.append({"kind": "structured_git_status_and_head",
            "source_sha256": capture.source_sha256, "session_id": capture.session_id,
            "sequence": command.sequence, "completion_sequence": command.completion_sequence,
            "execution_identity": command.execution_identity,
            "command": GIT_STATE_CHECK, "head": match[2], "status_porcelain": match[1],
            "output_sha256": repository_state.digest(command.output.encode()),
            "exit_code": command.exit_code, "workspace_clean": not match[1],
            "before_observed_mutations": first_mutation is None or command.completion_sequence < first_mutation,
            "terminal_after_observed_activity": terminal,
            "boundary": "tracked_and_nonignored_untracked"})
    return observations


def git_path_correlation(repository: Path, base: str | None, end: str | None,
                         observed_paths: list[str]) -> dict[str, Any]:
    if base is None or end is None:
        return {"state": "not_computable", "committed_paths": [],
                "correlated_paths": [], "uncorrelated_paths": observed_paths}
    paths = committed_delta_paths(repository, base, end)
    return {"state": "computed", "committed_paths": paths,
            "correlated_paths": sorted(set(observed_paths) & set(paths)),
            "uncorrelated_paths": sorted(set(observed_paths) - set(paths))}


def collect_journey_git_evidence(
    campaign: dict[str, Any],
    mapped: dict[tuple[str, str, str], MappedRollout],
) -> dict[str, Any]:
    """Observe Git facts independently of canonical Work identity and completion."""
    evidence: dict[str, Any] = {}
    for kind in CLASSES:
        journey = campaign["journeys"][journey_id(kind)]
        repository = Path(journey["repository_path"])
        baseline = journey["repository_revision"]
        ordered_slots = [slot for slot in harness.current_session_slots() if slot[0] == kind]
        attestation, _patches = repository_state.observe(repository)
        final_revision = attestation["final_head"]
        # Known revisions must still belong to the pinned repository history.
        # Their chronological order and intervals are observations, not Work policy.
        for revision in [*(mapped[slot].capture.git_revision for slot in ordered_slots), final_revision]:
            if not revision_is_bound(repository, baseline, revision):
                raise IntegrityError("project_binding", CampaignError(
                    "observed journey HEAD does not descend from its pinned repository baseline"))
        sessions = []
        for index, slot in enumerate(ordered_slots):
            capture = mapped[slot].capture
            paths = sorted({path for item in harness.meaningful_work_path_observations(capture)
                            for path in item.paths})
            statuses = session_git_state_observations(capture)
            terminal = next((item for item in reversed(statuses)
                             if item["terminal_after_observed_activity"]), None)
            start_status = next((item for item in statuses if item["before_observed_mutations"]), None)
            checkpoints = capture.successful_calls("checkpoint_record") if hasattr(capture, "successful_calls") else []
            dirty = harness.checkpoint_pre_existing_dirty_paths(checkpoints[0]) if checkpoints else None
            next_head = (mapped[ordered_slots[index + 1]].capture.git_revision
                         if index + 1 < len(ordered_slots) else final_revision)
            sessions.append({"session_slot_id": session_slot_id(*slot),
                "work_label": slot[1], "role": slot[2], "session_id": capture.session_id,
                "source_sha256": capture.source_sha256, "start_head": capture.git_revision,
                "end_head": terminal["head"] if terminal else None,
                "pre_mutation_status": start_status, "end_status": terminal,
                "git_state_observations": statuses, "baseline_dirty_paths": dirty,
                "changed_paths": paths, "next_observation_head": next_head,
                "commits_to_next_observation": repository_state.commit_history(repository, capture.git_revision, next_head),
                "path_correlation_to_next_observation": git_path_correlation(repository, capture.git_revision, next_head, paths),
                "path_correlation_to_final": git_path_correlation(repository, capture.git_revision, final_revision, paths)})
        carryover = []
        for previous, current in zip(sessions, sessions[1:]):
            dirty = current["baseline_dirty_paths"]
            carryover.append({"from_session_slot_id": previous["session_slot_id"],
                "to_session_slot_id": current["session_slot_id"],
                "state": "observed_baseline" if dirty is not None else "not_observed",
                "next_baseline_dirty_paths": dirty,
                "previous_observed_paths_still_dirty": sorted(set(previous["changed_paths"]) & set(dirty)) if dirty is not None else None,
                "actor_or_hunk_attribution": False})
        evidence[journey_id(kind)] = {
            "disposition": "advisory", "work_identity_basis": False,
            "git_policy_compliance": "post_hoc_task_and_repository_authority_review",
            "baseline_revision": baseline,
            "repository_start_state": {"head": baseline, "workspace_clean": True, "basis": "pinned_clean_clone"},
            "session_git_observations": sessions, "dirty_carryover": carryover,
            "commits": repository_state.commit_history(repository, baseline, final_revision),
            "ordered_session_revisions": [{"session_slot_id": session_slot_id(*slot),
                "revision": mapped[slot].capture.git_revision} for slot in ordered_slots],
            "final_revision": final_revision, "workspace_clean": attestation["workspace_clean"],
            "repository_state": attestation,
        }
    return evidence


def verify_final_repository_states(root: Path, manifest: dict[str, Any], *, live: bool) -> None:
    """Live recheck only at publication; historical consumers verify retained bytes."""
    for entry in manifest["journey_final_evidence"]:
        lineage = entry["repository_revision_lineage"]
        binding = lineage["attestation_artifacts"]
        state = read_json(root / binding["state"])
        patches = {key: (root / binding[key]).read_bytes() for key in ("staged", "unstaged")}
        repository_state.verify_retained(state, patches)
        if state != lineage["repository_state"] or state["final_head"] != lineage["final_revision"]:
            raise CampaignError("journey repository-state binding changed")
        if "git_observations" in binding and read_json(root / binding["git_observations"]) != journey_git_projection(entry["journey_id"], lineage):
            raise CampaignError("journey Git observation binding changed")
        if live:
            repository_state.verify(Path(manifest["journeys"][entry["journey_id"]]["repository_path"]), state)


def journey_git_projection(identity: str, lineage: dict[str, Any]) -> dict[str, Any]:
    return {"kind": "dogfood_journey_git_observations", "schema_version": 1,
            "journey_id": identity, "observation": {key: value for key, value in lineage.items()
                if key not in {"repository_state", "attestation_artifacts"}}}


def verify_final_repository_states_for_publication(root: Path) -> None:
    verify_final_repository_states(root, read_json(root / "evidence-set.json"), live=True)


def verify_retained_repository_states(root: Path, manifest: dict[str, Any]) -> None:
    verify_final_repository_states(root, manifest, live=False)














def slot_root(root: Path, work_slot_id: str) -> Path:
    if WORK_SLOT_ID.fullmatch(work_slot_id) is None:
        raise CampaignError("review slot identity is malformed")
    return root / "slots" / work_slot_id


def work_state(
    root: Path,
    kind: str,
    work_label: str,
    campaign: dict[str, Any] | None = None,
) -> dict[str, Any]:
    campaign = campaign or load_campaign(root)
    return campaign["works"][work_key(kind, work_label)]


def work_root(
    root: Path,
    kind: str,
    work_label: str,
    campaign: dict[str, Any] | None = None,
) -> Path:
    return slot_root(root, work_state(root, kind, work_label, campaign)["work_slot_id"])


def slot_artifact_path(root: Path, plane: str, directory: str, work_slot_id: str) -> Path:
    if WORK_SLOT_ID.fullmatch(work_slot_id) is None:
        raise CampaignError("review slot identity is malformed")
    path = root / plane / directory / f"{work_slot_id}.json"
    for candidate in (root / plane, root / plane / directory, path):
        if candidate.is_symlink() or not candidate.resolve(strict=False).is_relative_to(root.resolve()):
            raise CampaignError("campaign-owned artifact path escaped the private root")
    return path












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
            relative(root, path) != name
            or not path.is_file()
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




def frozen_descriptor_path(root: Path, kind: str, work: str) -> Path:
    state = work_state(root, kind, work)
    return slot_artifact_path(root, "tasks", "descriptors", state["work_slot_id"])






















def descriptor_semantic_sha256(value: dict[str, Any]) -> str:
    semantic = {key: item for key, item in value.items() if key != "evidence"}
    return hashlib.sha256(
        json.dumps(semantic, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def operator_task_artifact_path(root: Path, work_slot_id: str, role: str) -> Path:
    if WORK_SLOT_ID.fullmatch(work_slot_id) is None or role not in {"start", "resume"}:
        raise CampaignError("sealed operator task identity is malformed")
    return root / "operator/tasks" / f"{work_slot_id}.{role}.txt"


def write_operator_task_artifacts(
    root: Path,
    state: dict[str, Any],
    descriptor: dict[str, Any],
) -> dict[str, dict[str, Any]]:
    semantic_sha256 = descriptor_semantic_sha256(descriptor)
    artifacts: dict[str, dict[str, Any]] = {}
    fields = [("start", "work_user_task")]
    if "resume" in session_roles(state["repository_class"], state["work_label"]):
        fields.append(("resume", "fresh_resume_user_task"))
    for role, field in fields:
        content = descriptor[field].encode("utf-8")
        path = operator_task_artifact_path(root, state["work_slot_id"], role)
        if path.exists():
            raise CampaignError(
                f"sealed operator task artifact already exists: {relative(root, path)}"
            )
        atomic_write_bytes(path, content)
        artifacts[role] = {
            "path": relative(root, path),
            "bytes": len(content),
            "sha256": hashlib.sha256(content).hexdigest(),
            "frozen_descriptor_sha256": semantic_sha256,
        }
    return artifacts


def verify_operator_task_artifacts(
    root: Path,
    state: dict[str, Any],
    descriptor: dict[str, Any],
) -> dict[str, dict[str, Any]]:
    artifacts = state.get("operator_task_artifacts")
    expected_roles = set(session_roles(state["repository_class"], state["work_label"]))
    if not isinstance(artifacts, dict) or set(artifacts) != expected_roles:
        raise CampaignError("sealed Work has no complete raw operator task artifacts")
    inventory = load_inventory(root).get("artifacts", {})
    semantic_sha256 = descriptor_semantic_sha256(descriptor)
    fields = [("start", "work_user_task")]
    if "resume" in expected_roles:
        fields.append(("resume", "fresh_resume_user_task"))
    for role, field in fields:
        expected_path = operator_task_artifact_path(root, state["work_slot_id"], role)
        record = artifacts.get(role)
        expected_content = descriptor[field].encode("utf-8")
        expected_sha256 = hashlib.sha256(expected_content).hexdigest()
        expected_name = relative(root, expected_path)
        if (
            not isinstance(record, dict)
            or record.get("path") != expected_name
            or record.get("bytes") != len(expected_content)
            or record.get("sha256") != expected_sha256
            or record.get("frozen_descriptor_sha256") != semantic_sha256
            or not expected_path.is_file()
            or expected_path.read_bytes() != expected_content
            or inventory.get(expected_name)
            != {"bytes": len(expected_content), "sha256": expected_sha256}
        ):
            raise CampaignError(f"sealed {role} operator task artifact binding changed")
    return artifacts








def assert_operator_artifacts_do_not_leak(root: Path) -> None:
    """Operator projections contain only frozen tasks and bounded execution aids."""
    forbidden = ("EVALUATOR_ONLY", "evaluator/qualification-profile",
                 "reviewer/provisional", "qualification_profile_truth")
    for path in sorted((root / "operator").rglob("*")):
        if not path.is_file() or path.suffix == ".txt":
            continue
        text = path.read_text(encoding="utf-8")
        if any(marker in text for marker in forbidden):
            raise CampaignError(
                f"operator-facing artifact exposes private review material: {relative(root, path)}"
            )


def render_operator_run_sheet(root: Path) -> Path:
    campaign = load_campaign(root)
    preparation = read_json(root / "preparation.json")
    resource_scope = preparation.get("resource_observation", "unspecified")
    technical_state = preparation.get("technical_gate", {"state": "not_provided"})["state"]
    resource_guidance = (
        "Resource observation was explicitly selected for characterization. Follow "
        "rebuild/validation/dogfood/resource-observation.md using start/attach, expect, stop "
        "and record-resources. Until actually started, evidence remains not_observed. "
        "Retain attempted partial/failed observations and actual coverage; selection is no measurement. "
        if resource_scope == "selected" else
        "Resource observation is not selected for this user-experience run. Evidence stays "
        "not_observed with null/unmeasured resource values; proceed directly to collect-batch "
        "without telemetry operations. Optional characterization is described in "
        "rebuild/validation/dogfood/resource-observation.md. "
        if resource_scope == "not_selected" else
        "Resource scope was not recorded by this preparation. Preserve any existing promise "
        "and record a scope change separately before omitting selected characterization. "
    )
    entries_by_repository: dict[str, list[str]] = {kind: [] for kind in CLASSES}
    sequence_complete = all(
        state.get("state") == "frozen" for state in campaign["works"].values()
    )
    for kind in CLASSES:
        states = [campaign["works"][work_key(kind, label)] for label in work_labels(kind)]
        for state in states:
            if not sequence_complete:
                continue
            work = state["work_label"]
            descriptor = read_json(frozen_descriptor_path(root, kind, work))
            work_slot_id = state["work_slot_id"]
            task_artifacts = verify_operator_task_artifacts(root, state, descriptor)
            for role in session_roles(kind, work):
                task = task_artifacts[role]
                entries_by_repository[kind].append(
                    f"### Session `{work_slot_id}.{role}`\n\n"
                    f"- Workload intent: `{state['workload_intent']}`\n"
                    f"- Repository: `{state['repository_path']}`\n"
                    f"- Runtime Home: `{state['runtime_home']}`\n"
                    f"- Capture destination: `{slot_root(root, work_slot_id) / 'evidence' / f'{role}.rollout.jsonl'}`\n"
                    f"- Frozen task artifact: `{root / task['path']}`\n"
                    f"- Frozen task SHA-256: `{task['sha256']}`\n\n"
                    "Copy the exact UTF-8 bytes from the raw `.txt` artifact. Do not copy or "
                    "retype the task from Markdown, and do not add, remove, escape, or normalize "
                    "any character. Use the already reviewed integration and same-context readiness. "
                    "Satisfy only missing repository/hook trust in the selected host. Start this task in its own fresh thread, send only the frozen task, "
                    "and preserve the raw rollout file. Do not run campaign collection between chats.\n"
                )
    entries = [
        f"## Repository `{kind}`\n\n" + "\n\n".join(entries_by_repository[kind])
        for kind in CLASSES
        if entries_by_repository[kind]
    ]
    path = root / "operator/RUN-SHEET.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "# Naturalistic Dogfood Operator Run Sheet\n\n"
        "This helper does not grant repository or hook trust and does not start Codex sessions. "
        "Use this operator material after all five Works and eight session tasks are frozen. "
        "The campaign steward runs `activate-all` to generate inspectable integration before hook review; "
        "unchanged owned integration is verified and reused. Activation never grants trust. The helper "
        "verifies the production-owned static MCP and SessionStart files, but that does not prove that "
        "Codex executed SessionStart; every raw session still requires runtime activation evidence. "
        "Use `linux-codex-integration/launch_readiness.py --inspect` with the frozen absolute CLI, "
        "Runtime/repository and executable hashes to inspect the actual hook and exact Runtime permission "
        "guidance. For VS Code use its shared trusted project config; CLI `--add-dir` configures only "
        "that CLI invocation. Satisfy missing trust/permission before a scoped actual-host smoke. "
        "Retain readiness stdout, numeric execution and current host context privately; reuse matching "
        "scope with `--context`, `--reuse-output` and `--reuse-execution`. A changed condition needs only "
        "that scoped smoke. Local/elevated success cannot prove another/default host channel. "
        "If trust or activation is uncertain, inspect it before sending any frozen task. Run all "
        "eight fresh start/resume chats, preserve their raw rollouts, and provide the eight files once "
        "to the steward. When explanations are needed, first run `prepare-explanations --campaign-root ROOT "
        "--rollout-directory RAW --language en --language ko` (optionally select --work/--decision), then "
        "have the authorized active host interpret each private Product plan and run `record-explanation "
        "--campaign-root ROOT --explanation-id ID --input RESPONSE`. Inspect progress with `inspect-explanations`. "
        "This uses actual `work explain prepare/record --work ID` and `decision explain prepare/record --decision ID` "
        "CLI interfaces; no Work MCP tool exists. Same-language prose may need generation too. "
        "Earlier measured absent/stale/current answers remain independent. Steward generation is post-session "
        "and proves no earlier use or adoption. No per-session generation quota applies. "
        "Finish any requested explanation lifecycle before cross-locale documents: run `prepare-document-realizations`, "
        "has an active host complete and fix the private drafts, and then runs `collect-batch`. "
        "Same-locale evidence uses `collect-batch` directly. No per-chat control-session collection is required.\n\n"
        "Follow the user's task and repository-owned Git policy. Dogfood requires no Work commit "
        "or clean Work boundary: zero or multiple commits, dirty carryover across distinct Works, "
        "and later commits containing several Works are collectible. Preserve unrelated changes. "
        "Git state and history remain factual review evidence and do not create or merge Work identity. "
        "An optional structured status/HEAD observation can use "
        f"`{GIT_STATE_CHECK}`; it is not required and dirty output is valid evidence. "
        "Commit-policy compliance is assessed against the actual task and repository authority in "
        "post-hoc review; the harness does not infer a commit obligation.\n\n"
        + resource_guidance
        + "Technical V11/resource rehearsal evidence remains separate from user-run resources.\n\n"
        + f"Prepared technical prerequisite: {technical_state}. "
        "Use applicable verified exact-candidate capsule/archive with qualify; preparation "
        "does not execute a gate or establish user/human outcomes. Missing or failed technical "
        "evidence remains a prerequisite issue.\n\n"
        + ("\n\n".join(entries) if entries else "No slots are frozen for operator use yet.\n"),
        encoding="utf-8",
    )
    return path


def run_checked(argv: list[str], *, cwd: Path = ROOT) -> dict[str, Any]:
    completed = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        raise CampaignError(f"campaign command failed with exit {completed.returncode}: {argv[0]}")
    try:
        return json.loads(completed.stdout) if completed.stdout.strip().startswith("{") else {}
    except json.JSONDecodeError as error:
        raise CampaignError(f"campaign command returned malformed JSON: {argv[0]}") from error


def shell_quote_path(path: Path) -> str:
    return "'" + str(path).replace("'", "'\\''") + "'"


def verify_static_codex_integration(
    repository: Path,
    runtime: Path,
    binary: Path,
    result: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Verify the production-owned repository integration after enable."""
    repository = repository.resolve()
    runtime = runtime.resolve()
    binary = binary.resolve()
    mcp = binary.with_name("volicord-mcp").resolve()
    config_path = repository / ".codex/config.toml"
    manifest_path = repository / ".codex/volicord-integration.json"
    expected_result = {
        "operation": "codex_enable",
        "repository": str(repository),
        "config": str(config_path),
        "mcp_server": "volicord",
        "mcp_executable": str(mcp),
        "runtime": str(runtime),
        "session_start_matcher": "^(startup|resume|clear|compact)$",
        "project_trust": "user_controlled",
    }
    if not binary.is_file() or not os.access(binary, os.X_OK) or not mcp.is_file() or not os.access(mcp, os.X_OK):
        raise CampaignError("Codex owned static integration is not bound to candidate-local executables")
    if result is not None and result != expected_result:
        raise CampaignError("Codex enable result does not match the owned static integration contract")
    try:
        manifest = read_json(manifest_path)
        config = tomllib.loads(config_path.read_text(encoding="utf-8"))
    except (CampaignError, OSError, tomllib.TOMLDecodeError) as error:
        raise CampaignError("Codex owned static integration artifacts are unavailable or malformed") from error
    expected_manifest = {
        "kind": "volicord_codex_repository_integration",
        "schema_version": 1,
        "repository": str(repository),
        "runtime": str(runtime),
        "volicord": str(binary),
        "volicord_mcp": str(mcp),
    }
    if any(manifest.get(field) != value for field, value in expected_manifest.items()):
        raise CampaignError("Codex ownership manifest is not bound to the exact candidate repository/runtime")
    if not isinstance(manifest.get("config_created"), bool) or not isinstance(
        manifest.get("excluded_paths"), list
    ):
        raise CampaignError("Codex ownership manifest has an unexpected owned-state shape")
    server = config.get("mcp_servers", {}).get("volicord")
    if server != {
        "command": str(mcp),
        "enabled": True,
        "required": True,
        "env": {"VOLICORD_RUNTIME_DIR": str(runtime)},
    }:
        raise CampaignError("Codex Volicord MCP entry is not bound to the exact candidate/runtime")
    expected_command = (
        f"{shell_quote_path(binary)} --runtime {shell_quote_path(runtime)} "
        f"--repository {shell_quote_path(repository)} codex hook"
    )
    expected_hook = {
        "matcher": "^(startup|resume|clear|compact)$",
        "hooks": [{
            "type": "command",
            "command": expected_command,
            "timeout": 5,
            "statusMessage": "Activating Volicord repository context",
            "additionalContextLimit": 2000,
        }],
    }
    session_start = config.get("hooks", {}).get("SessionStart")
    if not isinstance(session_start, list) or session_start.count(expected_hook) != 1:
        raise CampaignError("Codex SessionStart hook is not bound to the exact candidate/runtime")
    return {
        "status": "passed",
        "ownership_manifest": str(manifest_path),
        "repository_config": str(config_path),
        "mcp_entry": "volicord",
        "session_start_matcher": expected_hook["matcher"],
        "candidate_binary": str(binary),
        "candidate_mcp_binary": str(mcp),
        "runtime_home": str(runtime),
        "repository_and_hook_trust": "user_controlled_not_automated",
        "runtime_session_start_execution": "not_proven_by_static_verification",
    }


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
        raise CampaignError("disposable work repository clone failed")
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
        raise CampaignError("disposable work repository revision could not be pinned cleanly")


def load_frozen_descriptor(
    root: Path,
    kind: str,
    work: str,
    campaign: dict[str, Any] | None = None,
) -> tuple[Path, dict[str, Any]]:
    campaign = campaign or load_campaign(root)
    state = campaign["works"][work_key(kind, work)]
    path = frozen_descriptor_path(root, kind, work)
    if state.get("state") not in {"frozen", "evidence_collected", "resume_collected"} or not path.is_file():
        raise CampaignError("work requires a complete frozen task descriptor")
    descriptor = read_json(path)
    if descriptor.get("contract") != "naturalistic-observation-1":
        raise CampaignError("frozen task descriptor uses a retired contract")
    if any(
        field in descriptor for field in ("materiality_obligations", "evaluation_basis", "behavior_review")
    ):
        raise CampaignError("frozen task descriptor contains semantic admission fields")
    if descriptor_semantic_sha256(descriptor) != state.get("frozen_descriptor_sha256"):
        raise CampaignError("frozen task descriptor changed")
    if descriptor.get("workload_intent") != state.get("workload_intent"):
        raise CampaignError("frozen workload intent changed")
    evidence_purpose.require_same(campaign, descriptor)
    errors = harness.work_descriptor_errors(
        descriptor,
        candidate_revision=campaign["candidate_head"],
        target_repository=Path(state["repository_path"]),
        verify_provenance=True,
    )
    if errors:
        raise CampaignError("frozen task descriptor no longer qualifies: " + "; ".join(errors))
    verify_operator_task_artifacts(root, state, descriptor)
    return path, descriptor



























































def verify_frozen_campaign(root: Path, campaign: dict[str, Any], *, verify_executables: bool = True) -> None:
    """Verify the complete deterministic run sheet before any activation."""
    verify_inventory(root)
    if verify_executables:
        verify_candidate_artifacts(campaign)
    expected_works = {work_key(kind, label) for kind in CLASSES for label in work_labels(kind)}
    expected_journeys = {journey_id(kind) for kind in CLASSES}
    if set(campaign.get("works", {})) != expected_works or set(campaign.get("journeys", {})) != expected_journeys:
        raise CampaignError("campaign is missing a maintained journey or Work")
    preparation = read_json(root / "preparation.json")
    if preparation.get("candidate_head") != campaign["candidate_head"] or preparation.get("session_count") != BATCH_CAPTURE_COUNT:
        raise CampaignError("campaign preparation candidate or topology changed")
    task_hashes = preparation.get("task_sha256_by_session_slot")
    if not isinstance(task_hashes, dict) or len(task_hashes) != BATCH_CAPTURE_COUNT:
        raise CampaignError("campaign preparation lacks eight frozen tasks")
    specs = repository_spec_map(read_json(root / "repository-input.json"))
    slot_ids = []
    paths, runtimes = [], []
    for kind in CLASSES:
        journey = campaign["journeys"][journey_id(kind)]
        expected_revision = campaign["candidate_head"] if kind == "volicord" else specs[kind]["revision"]
        repository = root / "journeys" / journey_id(kind) / "repository"
        runtime = root / "journeys" / journey_id(kind) / "runtime"
        if (journey.get("repository_path") != str(repository.resolve())
                or journey.get("runtime_home") != str(runtime.resolve())
                or journey.get("repository_revision") != expected_revision
                or journey.get("work_slot_ids") != [work_key(kind, label) for label in work_labels(kind)]):
            raise CampaignError("journey workspace, Runtime Home or revision binding changed")
        if (repository / ".git").exists() and not journey.get("codex_enabled"):
            if harness.git_head(repository) != expected_revision or not harness.git_clean(repository):
                raise CampaignError("unactivated journey repository no longer matches its pinned revision")
        paths.append(journey["repository_path"])
        runtimes.append(journey["runtime_home"])
        for label in work_labels(kind):
            state = campaign["works"][work_key(kind, label)]
            if (state.get("state") != "frozen"
                    or state.get("journey_id") != journey_id(kind)
                    or state.get("repository_class") != kind
                    or state.get("work_label") != label
                    or state.get("repository_path") != journey["repository_path"]
                    or state.get("repository_revision") != expected_revision
                    or state.get("runtime_home") != journey["runtime_home"]
                    or state.get("session_slot_ids") != [
                        session_slot_id(kind, label, role) for role in session_roles(kind, label)]):
                raise CampaignError("frozen Work identity or topology changed")
            slot_ids.append(state["work_slot_id"])
            _path, descriptor = load_frozen_descriptor(root, kind, label, campaign)
            for role, artifact in state["operator_task_artifacts"].items():
                if task_hashes.get(session_slot_id(kind, label, role)) != artifact["sha256"]:
                    raise CampaignError("frozen task differs from preparation receipt")
    if len(set(slot_ids)) != QUALIFICATION_WORK_COUNT or len(set(paths)) != len(CLASSES) or len(set(runtimes)) != len(CLASSES):
        raise CampaignError("journey or Work slot identity is not isolated")


def activate_journey(root: Path, kind: str) -> dict[str, Any]:
    campaign = load_campaign_for_mutation(root)
    verify_frozen_campaign(root, campaign)
    state = campaign["journeys"][journey_id(kind)]
    repository = Path(state["repository_path"])
    binary = Path(campaign["candidate_binary"])
    manifest = repository / ".codex/volicord-integration.json"
    with candidate_artifact_use(campaign, ("volicord", "volicord-mcp")):
        if manifest.exists():
            # Preserve reviewed hook/config bytes. A changed owned route is a
            # blocker to repair explicitly, never silently disable/rebind it.
            result = None
            verification = verify_static_codex_integration(repository, Path(state["runtime_home"]), binary)
        else:
            result = run_checked([
                str(binary), "--runtime", state["runtime_home"], "--json",
                "--repository", str(repository), "codex", "enable",
            ])
            verification = verify_static_codex_integration(repository, Path(state["runtime_home"]), binary, result)
    state["codex_enabled"] = True
    save_campaign(root, campaign)
    return {
        "journey_id": state["journey_id"],
        "repository_class": kind,
        "enable_result": result,
        "integration_execution": "reused" if result is None else "generated",
        "static_verification": verification,
    }


def activate_all(root: Path) -> dict[str, Any]:
    campaign = load_campaign_for_mutation(root)
    verify_frozen_campaign(root, campaign)
    results = [activate_journey(root, kind) for kind in CLASSES]
    return {
        "kind": "phase8_dogfood_campaign_activation",
        "journey_count": len(results),
        "repository_and_hook_trust": "user_controlled_not_automated",
        "journeys": results,
    }


def frozen_tasks(task_manifest: Path) -> tuple[dict[tuple[str, str, str], bytes], dict[str, dict[str, Any]]]:
    """Read every operator-owned task before mutating the campaign."""
    value = read_json(task_manifest)
    if not isinstance(value, dict) or set(value) != {"tasks"} or not isinstance(value["tasks"], dict):
        raise CampaignError("task manifest must contain exactly the five Work task mappings")
    expected_works = {(kind, label) for kind in CLASSES for label in work_labels(kind)}
    if set(value["tasks"]) != {work_key(*key) for key in expected_works}:
        raise CampaignError("task manifest must contain exactly five maintained Work slots")
    tasks = {}
    sources = set()
    for kind, label in sorted(expected_works):
        roles = session_roles(kind, label)
        entry = value["tasks"][work_key(kind, label)]
        if not isinstance(entry, dict) or set(entry) != set(roles) | {"workload_intent", "learning_collaboration_statement"}:
            raise CampaignError("task manifest must contain the exact start/resume role topology")
        for role in roles:
            source = Path(entry[role]).resolve()
            if source in sources or not source.is_file():
                raise CampaignError("task source paths must be eight distinct existing files")
            sources.add(source)
            content = source.read_bytes()
            try:
                task = content.decode("utf-8")
            except UnicodeDecodeError as error:
                raise CampaignError("frozen task must be UTF-8") from error
            field = "work_user_task" if role == "start" else "fresh_resume_user_task"
            problem = harness.plain_user_task_error(task, field)
            if problem:
                raise CampaignError(problem)
            tasks[(kind, label, role)] = content
        errors = workload_intents.metadata_errors(work_key(kind, label), entry,
            tasks[(kind, label, "start")].decode("utf-8"))
        if errors:
            raise CampaignError("; ".join(errors))
    if len(tasks) != BATCH_CAPTURE_COUNT or any(
        len({content for (repository_class, _work, _role), content in tasks.items()
            if repository_class == kind})
        != sum(len(session_roles(kind, label)) for label in work_labels(kind))
        for kind in CLASSES
    ):
        raise CampaignError("frozen tasks must uniquely identify each role within its journey")
    return tasks, {slot: {field: entry[field] for field in
        ("workload_intent", "learning_collaboration_statement")}
        for slot, entry in value["tasks"].items()}


def prepare_campaign(
    root: Path, campaign_id: str, candidate_head: str,
    repository_input: Path, task_manifest: Path, *,
    candidate_binary: Path | None = None, enable: bool = False,
    cloner: Callable[[Path, Path, str], None] = clone_repository,
    purpose: str = evidence_purpose.NATURALISTIC,
    capsule_path: Path | None = None, archive_path: Path | None = None,
    observe_resources: bool = False,
) -> dict[str, Any]:
    root = root.resolve()
    if root.exists() and any(root.iterdir()):
        raise CampaignError("campaign root must be absent or empty")
    if enable:
        raise CampaignError("activate-all is the explicit activation operation")
    require_private_campaign_root(root)
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{2,80}", campaign_id):
        raise CampaignError("campaign identity must be a bounded filesystem-safe value")
    require_current_candidate(candidate_head)
    evidence_purpose.validate(purpose)
    import qualification_policy
    technical = qualification_policy.verify_technical(candidate_head, capsule_path, archive_path)
    if technical["state"] == "failed":
        raise CampaignError("campaign technical prerequisite failed")
    definition = harness.load_definition()
    raw_input = read_json(repository_input)
    specs = repository_spec_map(raw_input)
    tasks, selection = frozen_tasks(task_manifest)
    document_language = raw_input.get("document_language", "en")
    if not isinstance(document_language, str) or not document_language.strip() or document_language != document_language.strip() or len(document_language.encode("utf-8")) > 128:
        raise CampaignError("campaign document language must be bounded non-empty text")
    viewer_locale = raw_input.get("viewer_locale", "en")
    if viewer_locale not in {"en", "ko"}:
        raise CampaignError("campaign Viewer locale must be en or ko")
    _, identities = harness.load_repository_specs(repository_input, candidate_head, definition)
    if any(item["status"] != "passed" for item in identities):
        raise CampaignError("one or more source repository identities do not qualify")
    root.mkdir(parents=True, exist_ok=True)
    root.chmod(0o700)
    binary = candidate_binary.resolve() if candidate_binary else install_candidate(root)
    candidate_artifacts = bind_candidate_artifacts(binary)
    if technical["state"] == "passed":
        # Recheck the retained evidence with the actual installed executable binding.
        # Candidate equality remains mandatory even when binaries are identical.
        technical = qualification_policy.verify_technical(candidate_head, capsule_path, archive_path,
            candidate_artifacts=candidate_artifacts)
    realization_route = (document_realization.route(binary)
        if document_realization.required(document_language, viewer_locale) else None)
    journeys = {}
    for kind in CLASSES:
        spec = specs[kind]
        revision = candidate_head if kind == "volicord" else spec["revision"]
        identity = journey_id(kind)
        journey_root = root / "journeys" / identity
        journeys[identity] = {
            "journey_id": identity, "repository_class": kind,
            "repository_path": str((journey_root / "repository").resolve()),
            "repository_revision": revision,
            "runtime_home": str((journey_root / "runtime").resolve()),
            "work_slot_ids": [work_key(kind, label) for label in work_labels(kind)],
            "project_id": None, "codex_enabled": False,
        }
        (journey_root / "runtime").mkdir(parents=True)
        cloner(Path(spec["path"]).resolve(), Path(journeys[identity]["repository_path"]), revision)
    works = {}
    for kind, label in [(kind, label) for kind in CLASSES for label in work_labels(kind)]:
        slot = work_key(kind, label)
        journey = journeys[journey_id(kind)]
        state = {
            "journey_id": journey["journey_id"],
            "work_slot_id": work_key(kind, label), "repository_class": kind,
            "work_label": label, "workload_intent": selection[slot]["workload_intent"],
            "session_slot_ids": [session_slot_id(kind, label, role) for role in session_roles(kind, label)],
            "repository_path": journey["repository_path"],
            "repository_revision": journey["repository_revision"],
            "runtime_home": journey["runtime_home"],
            "state": "frozen", "frozen_descriptor_sha256": None,
            "operator_task_artifacts": None, "project_id": None,
            "codex_enabled": False,
        }
        descriptor = {
            "kind": "phase8_work_descriptor", "contract": "naturalistic-observation-1",
            "evidence_purpose": purpose,
            "producer": "volicord_phase8_codex_event_normalizer",
            **selection[slot],
            "journey_id": state["journey_id"], "repository_class": kind,
            "work_slot_id": state["work_slot_id"], "work_label": label,
            "repository_revision": state["repository_revision"],
            "work_user_task": tasks[(kind, label, "start")].decode("utf-8"),
            "fresh_resume_user_task": (
                tasks[(kind, label, "resume")].decode("utf-8")
                if "resume" in session_roles(kind, label) else None
            ),
        }
        state["frozen_descriptor_sha256"] = descriptor_semantic_sha256(descriptor)
        descriptor_path = slot_artifact_path(root, "tasks", "descriptors", slot)
        write_json(descriptor_path, descriptor)
        register_artifact(root, descriptor_path)
        state["operator_task_artifacts"] = write_operator_task_artifacts(root, state, descriptor)
        for record in state["operator_task_artifacts"].values():
            register_artifact(root, root / record["path"])
        works[state["work_slot_id"]] = state
        (slot_root(root, slot) / "evidence").mkdir(parents=True)
    campaign = {
        "kind": "phase8_dogfood_campaign", "schema_version": 9,
        "campaign_id": campaign_id, "campaign_root": str(root), "evidence_purpose": purpose,
        "candidate_head": candidate_head, "candidate_binary": str(binary),
        "candidate_artifacts": candidate_artifacts,
        "naturalistic_memory_evidence": resource_observer.initial(candidate_artifacts, purpose=purpose),
        "live_evidence_obligations": live_evidence_obligations(candidate_artifacts,
            resource_observer.initial(candidate_artifacts, purpose=purpose)),
        "document_language": document_language, "viewer_locale": viewer_locale,
        "document_realization_route": realization_route,
        "repository_input": relative(root, root / "repository-input.json"),
        "terminal_outcome": None, "collection_state": "pending",
        "evaluation_state": "not_run", "qualification_state": "not_run",
        "evidence_set": None, "journeys": journeys, "works": works,
    }
    write_json(root / "repository-input.json", raw_input)
    save_campaign(root, campaign)
    write_json(inventory_path(root), load_inventory(root))
    preparation = {
        "kind": "phase8_dogfood_campaign_preparation", "campaign_id": campaign_id, "evidence_purpose": purpose,
        "candidate_head": candidate_head, "candidate_worktree_clean": True,
        "repository_identities": identities,
        "journey_count": len(CLASSES), "work_count": QUALIFICATION_WORK_COUNT,
        "resume_pair_count": harness.QUALIFICATION_RESUME_PAIR_COUNT,
        "session_count": BATCH_CAPTURE_COUNT, "candidate_local_install": str(binary),
        "candidate_artifacts": candidate_artifacts,
        "technical_gate": technical,
        "resource_observation": "selected" if observe_resources else "not_selected",
        "naturalistic_memory_evidence": resource_observer.initial(candidate_artifacts, purpose=purpose),
        "live_evidence_obligations": live_evidence_obligations(candidate_artifacts,
            resource_observer.initial(candidate_artifacts, purpose=purpose)),
        "repository_trust": "user_controlled_not_automated",
        "workload_intents": workload_intents.WORKLOAD_INTENTS,
        "task_sha256_by_session_slot": {
            session_slot_id(*slot): hashlib.sha256(content).hexdigest()
            for slot, content in sorted(tasks.items())
        },
    }
    write_json(root / "preparation.json", preparation)
    run_sheet = render_operator_run_sheet(root)
    for path in (root / "repository-input.json", root / "preparation.json", run_sheet):
        register_artifact(root, path)
    verify_inventory(root)
    return preparation


def observed_project_ids(capture: Any) -> list[str]:
    if capture.transport_issues('project_resolve', 'project_initialize', 'recall'):
        raise CampaignError('Project identity transport is incomplete')
    observed = [call for operation in ('project_initialize', 'project_resolve', 'recall')
        for call in capture.successful_calls(operation)]
    # A successful lookup may establish absence before explicit initialization.
    # Only its exact no-identity result is exempt from identity validation.
    observed = [call for call in observed if not (call.operation == 'project_resolve'
        and call.result.get('status') == 'not_found' and 'project_id' not in call.result
        and 'project_id' not in call.arguments)]
    if any(call.operation == 'project_resolve' and call.result.get('status') == 'not_found'
            or PROJECT_ID.fullmatch(str(call.result.get('project_id', ''))) is None
            or call.operation != 'project_initialize' and call.arguments.get('project_id', call.result.get('project_id')) != call.result.get('project_id')
            for call in observed):
        raise CampaignError('Project identity is malformed or conflicting')
    values = {
        str(call.result.get("project_id"))
        for operation in ("project_initialize", "project_resolve", "recall")
        for call in capture.successful_calls(operation)
        if PROJECT_ID.fullmatch(str(call.result.get("project_id", "")))
    }
    return sorted(values)


def observed_work_item_ids(capture: Any, role: str) -> list[str]:
    """Use durable start evidence or the resume's canonical Recall basis."""
    checkpoint_ids = [call.arguments.get("goal_context_id")
        for call in capture.successful_calls("checkpoint_record")]
    if role == "start":
        ids = checkpoint_ids
    elif role == "resume":
        recalls = capture.successful_calls("recall")
        if not recalls or capture.transport_issues('recall'):
            raise CampaignError("fresh resume lacks complete canonical Recall evidence")
        import answer_observations
        if any(answer_observations.recall_identity_errors(call.result, call.arguments.get('project_id'))
                for call in recalls):
            raise CampaignError("fresh resume Recall identity is malformed or conflicting")
        ids = [call.result['checkpoint']['work_item_id'] for call in recalls] + checkpoint_ids
    else:
        raise CampaignError("unknown Work session role")
    if not ids or any(not isinstance(value, str) or PROJECT_ID.fullmatch(value) is None
            for value in ids) or len(set(ids)) != 1:
        raise CampaignError("Work identity is absent, malformed, or conflicting")
    return sorted(set(ids))


def update_activation_summary(root: Path, kind: str, work: str, **updates: Any) -> Path:
    path = work_root(root, kind, work) / "activation-summary.json"
    current = read_json(path) if path.exists() else {
        "kind": "phase8_dogfood_activation_summary",
        "repository_class": kind,
        "journey_id": journey_id(kind),
        "work_slot_id": work_key(kind, work),
        "repository_config_present": (Path(load_campaign(root)["works"][work_key(kind, work)]["repository_path"]) / ".codex/config.toml").is_file(),
        "repository_ownership_manifest_present": (Path(load_campaign(root)["works"][work_key(kind, work)]["repository_path"]) / ".codex/volicord-integration.json").is_file(),
        "start_session_start_activation_observed": None,
        "resume_session_start_activation_observed": (
            None if "resume" in session_roles(kind, work) else "not_applicable"
        ),
    }
    current.update(updates)
    write_json(path, current)
    return path


def inspect_resume(capture: Any, descriptor: dict[str, Any], state: dict[str, Any]) -> str:
    if (
        not capture.fresh_user_thread
        or not revision_is_bound(Path(state["repository_path"]),
            state["repository_revision"], capture.git_revision)
        or capture.cwd.resolve(strict=False) != Path(state["repository_path"]).resolve(strict=False)
        or not capture.user_turns
        or not harness.codex_user_turn_transport_identity_matches(
            capture.user_turns[0].text, descriptor["fresh_resume_user_task"]
        )
        or harness.activation_failure(capture) is not None
    ):
        raise ResumeContractError(
            "recall_identity_or_project_invalid"
        )
    if capture.session_id == state.get("start_session_id"):
        raise ResumeContractError("recall_identity_or_project_invalid")
    resolves = capture.successful_calls("project_resolve")
    recalls = capture.successful_calls("recall")
    checkpoints = capture.successful_calls("checkpoint_record")
    if capture.transport_issues('project_resolve'):
        raise ResumeContractError('recall_identity_or_project_invalid')
    if capture.transport_issues("recall"):
        raise ResumeContractError("recall_transport_incomplete")
    if not recalls and capture.calls("recall"):
        raise ResumeContractError("recall_operation_failed")
    if not resolves or not recalls or capture.successful_calls("project_initialize"):
        raise ResumeContractError(
            "recall_identity_or_project_invalid"
        )
    resolves = sorted(resolves, key=lambda call: call.sequence)
    recalls = sorted(recalls, key=lambda call: call.sequence)
    resolve, recall = resolves[0], recalls[0]
    if any(call.result.get("read_only") is not True or "checkpoint" not in call.result for call in recalls):
        raise ResumeContractError("recall_transport_incomplete")
    project_id = resolve.result.get("project_id")
    if (
        any(call.result.get('status') != 'found' or call.result.get('project_id') != project_id for call in resolves)
        or not PROJECT_ID.fullmatch(str(project_id or ""))
        or any(project_id != call.arguments.get('project_id') or project_id != call.result.get('project_id') for call in recalls)
        or project_id != state.get("project_id")
        or any(not any(r.completion_sequence < call.sequence for r in resolves) for call in recalls)
    ):
        raise ResumeContractError(
            "recall_identity_or_project_invalid"
        )
    try:
        resumed_work_ids = observed_work_item_ids(capture, "resume")
    except CampaignError as error:
        raise ResumeContractError("recall_identity_or_project_invalid") from error
    if state.get("work_item_id") is not None and resumed_work_ids != [state["work_item_id"]]:
        raise ResumeContractError("recall_identity_or_project_invalid")
    if any(
        command.sequence <= recall.completion_sequence and command_is_repository_inspection(command.parsed_command)
        for command in capture.commands
    ) or any(
        call.sequence <= recall.completion_sequence
        for call in capture.tool_calls
        if harness.repository_operation_is_inspection(call)
        or call.operation in {"inquiry_frontier", "checkpoint_record"}
    ) or any(item.sequence <= recall.completion_sequence for item in capture.path_observations):
        raise ResumeContractError("pre_recall_repository_access_or_order_violation")
    meaningful_changes = harness.meaningful_work_path_observations(capture)
    first_write = min(
        (item.sequence for item in meaningful_changes),
        default=None,
    )
    change_baseline_ok = bool(checkpoints) and all(
        harness.checkpoint_baseline_is_pre_work(
            capture,
            checkpoint,
            project_id=str(project_id),
            boundary_completion_sequence=recall.completion_sequence,
            first_write_sequence=first_write,
        )
        for checkpoint in checkpoints
    )
    recalled_checkpoint = recall.result.get("checkpoint")
    terminal_checkpoint = (
        max(checkpoints, key=lambda call: call.sequence) if checkpoints else None
    )
    baseline_id = (
        terminal_checkpoint.arguments.get("baseline_analysis_snapshot_id")
        if terminal_checkpoint is not None
        else None
    )
    goal_context_id = (
        terminal_checkpoint.arguments.get("goal_context_id")
        if terminal_checkpoint is not None
        else None
    )
    _, current_review, _ = harness.current_authority_frontier(
        capture, project_id=project_id, goal_context_id=goal_context_id,
        baseline_analysis_snapshot_id=baseline_id, before_sequence=first_write or 0,
    )
    scope_chronology = (
        harness.executable_scope_chronology(
            capture,
            project_id=project_id,
            review_candidate_id=current_review.result.get("review_candidate_id"),
            goal_context_id=goal_context_id,
            baseline_analysis_snapshot_id=baseline_id,
        )
        if current_review is not None
        else {"qualified": False, "executable_work_scope": None}
    )
    continuation = harness.resume_continuation_facts(
        capture,
        recall,
        checkpoint_work_state=(
            recalled_checkpoint.get("work_state")
            if isinstance(recalled_checkpoint, dict)
            else None
        ),
        recalled_work_state=(
            recalled_checkpoint.get("work_state")
            if isinstance(recalled_checkpoint, dict)
            else None
        ),
        common_identity_and_freshness_ok=(not checkpoints or change_baseline_ok),
        change_baseline_ok=change_baseline_ok and scope_chronology["qualified"],
        executable_work_scope=scope_chronology["executable_work_scope"],
        descriptor_scope_paths=descriptor.get("work_scope", {}).get(
            "affected_paths"
        ),
    )
    if continuation["mode"] is None:
        basis = continuation["failure_basis"]
        if first_write is not None and change_baseline_ok and not scope_chronology["qualified"]:
            basis = "scope_or_authority_missing"
        raise ResumeContractError(basis or "validator_invariant_failure")
    return str(project_id)


def default_export(binary: Path, runtime: Path, repository: Path, destination: Path) -> None:
    completed = subprocess.run(
        [
            str(binary), "--runtime", str(runtime), "--repository", str(repository),
            "context", "export", "--output", str(destination),
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0 or not destination.is_file():
        raise CampaignError("candidate context export failed")


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
        "start_session_start_activation_observed": work_activation,
        "resume_session_start_activation_observed": resume_activation,
        "content_included": False,
    }


def generate_document(
    binary: Path,
    runtime: Path,
    repository: Path,
    kind: str,
    format_name: str,
    destination: Path,
    language: str,
    locale: str,
) -> dict[str, Any]:
    recorder = harness.load_v11().Recorder(
        destination.parents[1] / "document-export-processes" / destination.name
    )
    process = recorder.run(
        f"{kind}-{format_name}",
        [
            str(binary), "--json", "--locale", locale,
            "--runtime", str(runtime), "--repository", str(repository),
            "document", "export", kind, "--format", format_name,
            "--output", str(destination), "--language", language,
        ],
        os.environ.copy(),
        cwd=repository,
    )
    result: dict[str, Any] = {"status": "failed", "_process_result": process}
    if process.get("spawn_error") is not None:
        result["basis"] = "supported document export could not be spawned"
        return result
    if process.get("termination") is not None:
        result["basis"] = "supported document export terminated before completion"
        return result
    if process.get("exit_code") != 0:
        result["basis"] = f"supported document export exited {process.get('exit_code')}"
        return result
    try:
        product = json.loads(Path(process["stdout"]).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError, TypeError, KeyError):
        result["basis"] = "supported document export returned invalid structured output"
        return result
    if not isinstance(product, dict):
        result["basis"] = "supported document export returned invalid structured output"
        return result
    result["product_result"] = {
        key: product.get(key)
        for key in (
            "operation", "outcome", "project_id", "kind", "format",
            "destination", "published", "reason",
        )
        if key in product
    }
    if product.get("outcome") == "unavailable":
        reason = product.get("reason")
        result["basis"] = (
            f"requested-language document unavailable: {reason[:384]}"
            if isinstance(reason, str) and reason.strip()
            else "requested-language document unavailable"
        )
        return result
    destination_value = product.get("destination")
    destination_matches = (
        isinstance(destination_value, str)
        and Path(destination_value).resolve(strict=False)
        == destination.resolve(strict=False)
    )
    if (
        product.get("operation") == "document_export"
        and product.get("kind") == kind
        and product.get("format") == format_name
        and destination_matches
        and destination.is_file()
    ):
        result["status"] = "passed"
        return result
    result["basis"] = (
        "supported document export returned mismatched evidence or did not publish "
        "the requested destination"
    )
    return result


def bounded_document_process_evidence(
    root: Path, process: dict[str, Any]
) -> dict[str, Any]:
    evidence = harness.bounded_process_result(process)
    for name in ("stdout", "stderr"):
        path = Path(process[name])
        evidence[name] = {
            "relative_evidence_path": relative(root, path),
            "bytes": path.stat().st_size,
            "sha256": harness.sha256(path),
        }
    result_path = Path(process["stdout"]).with_name("result.json")
    command_path = Path(process["stdout"]).with_name("command.json")
    evidence["result"] = {
        "relative_evidence_path": relative(root, result_path),
        "bytes": result_path.stat().st_size,
        "sha256": harness.sha256(result_path),
    }
    evidence["command"] = {
        "relative_evidence_path": relative(root, command_path),
        "bytes": command_path.stat().st_size,
        "sha256": harness.sha256(command_path),
    }
    return evidence


def generate_viewer_snapshot(
    binary: Path,
    runtime: Path,
    project_id: str,
    destination: Path,
    locale: str,
    language: str,
) -> dict[str, Any]:
    viewer = binary.with_name("volicord-viewer")
    completed = subprocess.run(
        [
            str(viewer),
            "--runtime", str(runtime),
            "--project", project_id,
            "--locale", locale,
            "--language", language,
            "--snapshot", str(destination.resolve()),
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode == 0 and destination.is_file():
        return {"status": "passed"}
    return {
        "status": "failed",
        "basis": (
            f"public Viewer snapshot export exited {completed.returncode}; "
            f"destination_present={destination.is_file()}"
        ),
    }


def collect_viewer_snapshot_evidence(
    root: Path,
    kind: str,
    work: str,
    binary: Path,
    runtime: Path,
    project_id: str,
    candidate_head: str,
    locale: str,
    language: str,
    snapshotter: Callable[[Path, Path, str, Path, str, str], dict[str, Any]],
) -> tuple[dict[str, Any], list[Path]]:
    destination = work_root(root, kind, work) / "evidence/viewer-snapshot.html"
    started = time.monotonic_ns()
    try:
        result = snapshotter(
            binary,
            runtime,
            project_id,
            destination,
            locale,
            language,
        )
    except (OSError, ValueError, CampaignError) as error:
        result = {
            "status": "failed",
            "basis": f"Viewer snapshot evidence adapter failed: {type(error).__name__}",
        }
    duration_ms = round((time.monotonic_ns() - started) / 1_000_000, 3)
    evidence: dict[str, Any] = {
        "kind": "phase8_viewer_snapshot_evidence_summary",
        "schema_version": 3,
        "status": "failed",
        "project_id": project_id,
        "candidate_head": candidate_head,
        "repository_class": kind,
        "work": work,
        "locale": locale,
        "requested_language": language,
        "navigation_responsiveness": {
            "duration_ms": duration_ms,
            "timing_source": "monotonic_candidate_bound_snapshot_export_request",
            "request_completed": result.get("status") in {"passed", "failed"},
            "scope": "Viewer snapshot export request; not browser input or paint latency",
            "measured": ["snapshot_export_request_completion", "snapshot_export_request_duration"],
            "unmeasured": ["browser_input_latency", "browser_paint_latency"],
        },
    }
    produced: list[Path] = []
    if result.get("status") == "passed" and destination.is_file():
        evidence.update({
            "status": "passed",
            "relative_evidence_path": relative(root, destination),
            "bytes": destination.stat().st_size,
            "sha256": harness.sha256(destination),
        })
        produced.append(destination)
    else:
        basis = result.get("basis") if isinstance(result, dict) else None
        evidence["basis"] = (
            basis[:512]
            if isinstance(basis, str) and basis.strip()
            else "public Viewer snapshot export did not produce usable evidence"
        )
    summary = work_root(root, kind, work) / "viewer-snapshot-summary.json"
    write_json(summary, evidence)
    produced.append(summary)
    return evidence, produced


def collect_document_evidence(
    root: Path,
    kind: str,
    work: str,
    binary: Path,
    runtime: Path,
    repository: Path,
    project_id: str,
    candidate_head: str,
    locale: str,
    language: str,
    documenter: Callable[[Path, Path, Path, str, str, Path, str, str], dict[str, Any]],
) -> tuple[dict[str, Any], list[Path]]:
    directory = work_root(root, kind, work) / "evidence/generated-documents"
    directory.mkdir(parents=True, exist_ok=True)
    documents: dict[str, Any] = {}
    produced: list[Path] = []
    for document_kind in DOCUMENT_KINDS:
        formats: dict[str, Any] = {}
        for format_name, suffix in DOCUMENT_FORMATS:
            destination = directory / f"{document_kind}.{suffix}"
            try:
                result = document_realization.generate(root, kind, work, project_id,
                    document_kind, format_name, destination) if document_realization.required(language, locale) else documenter(
                    binary,
                    runtime,
                    repository,
                    document_kind,
                    format_name,
                    destination,
                    language,
                    locale,
                )
            except IntegrityError:
                raise
            except (OSError, ValueError, CampaignError) as error:
                result = {
                    "status": "failed",
                    "basis": str(error)[:512] if isinstance(error, CampaignError)
                        else f"document evidence adapter failed: {type(error).__name__}",
                }
            process = result.pop("_process_result", None)
            process_evidence = (
                bounded_document_process_evidence(root, process)
                if isinstance(process, dict)
                else None
            )
            if isinstance(process, dict):
                stdout_path = Path(process["stdout"])
                produced.extend(
                    (
                        stdout_path,
                        Path(process["stderr"]),
                        stdout_path.with_name("command.json"),
                        stdout_path.with_name("result.json"),
                    )
                )
            product_result = result.get("product_result")
            if (
                result.get("status") == "passed"
                and isinstance(product_result, dict)
                and product_result.get("project_id") != project_id
            ):
                result = {
                    **result,
                    "status": "failed",
                    "basis": "document export result is associated with a different Project",
                }
            if result.get("status") == "passed" and destination.is_file():
                formats[format_name] = {
                    "status": "passed",
                    "relative_evidence_path": relative(root, destination),
                    "bytes": destination.stat().st_size,
                    "sha256": harness.sha256(destination),
                }
                if "provenance" in result:
                    formats[format_name]["realization_provenance"] = result["provenance"]
                produced.append(destination)
            else:
                basis = result.get("basis") if isinstance(result, dict) else None
                if not isinstance(basis, str) or not basis.strip():
                    basis = "supported document export did not produce usable evidence"
                formats[format_name] = {
                    "status": "failed",
                    "basis": basis[:512],
                }
            if process_evidence is not None:
                formats[format_name]["process_evidence"] = process_evidence
        document_status = (
            "passed"
            if all(formats[name]["status"] == "passed" for name, _suffix in DOCUMENT_FORMATS)
            else "failed"
        )
        documents[document_kind] = {"status": document_status, "formats": formats}
    summary = {
        "kind": "phase8_generated_document_evidence_summary",
        "schema_version": 1,
        "project_id": project_id,
        "candidate_head": candidate_head,
        "repository_class": kind,
        "work": work,
        "locale": locale,
        "language": language,
        "status": "passed" if all(item["status"] == "passed" for item in documents.values()) else "failed",
        "required_document_kinds": list(DOCUMENT_KINDS),
        "documents": documents,
    }
    return summary, produced


def write_operator_document_review_index(
    root: Path,
    kind: str,
    work: str,
    summary: dict[str, Any],
) -> Path:
    state = work_state(root, kind, work)
    work_slot_id = state["work_slot_id"]
    lines = [
        f"# Generated document review: {kind} slot {work_slot_id}",
        "",
        f"Language: `{summary['language']}`",
        "",
    ]
    for document_kind in DOCUMENT_KINDS:
        lines.extend((f"## {document_kind}", ""))
        formats = summary["documents"][document_kind]["formats"]
        for format_name, _suffix in DOCUMENT_FORMATS:
            evidence = formats[format_name]
            if evidence["status"] == "passed":
                lines.append(f"- {format_name}: `{evidence['relative_evidence_path']}`")
            else:
                lines.append(f"- {format_name}: unavailable ({evidence['basis']})")
        lines.append("")
    path = root / "operator/document-review" / f"{work_slot_id}.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")
    assert_operator_artifacts_do_not_leak(root)
    return path


def extract_resume_evidence(
    root: Path,
    kind: str,
    work: str,
    capture: Any,
    destination: Path,
    *,
    exporter: Callable[[Path, Path, Path, Path], None] = default_export,
    documenter: Callable[[Path, Path, Path, str, str, Path, str, str], dict[str, Any]] = generate_document,
    snapshotter: Callable[[Path, Path, str, Path, str, str], dict[str, Any]] = generate_viewer_snapshot,
    final_state: str = "resume_collected",
    integrity_only: bool = False,
) -> dict[str, Any]:
    campaign = load_campaign(root)
    key = work_key(kind, work)
    state = campaign["works"][key]
    descriptor_path, descriptor = load_frozen_descriptor(root, kind, work, campaign)
    project_id = state["project_id"] if integrity_only else inspect_resume(capture, descriptor, state)
    binary = Path(campaign["candidate_binary"])
    runtime = collection_runs.runtime_path(Path(state["runtime_home"]))
    repository = Path(state["repository_path"])
    bundle = work_root(root, kind, work) / "context.bundle.json"
    with candidate_artifact_use(campaign, ("volicord",)):
        exporter(binary, runtime, repository, bundle)
    try:
        canonical = harness.load_canonical_bundle(bundle)
    except (OSError, EvidenceError) as error:
        raise CampaignError("context export is not a supported canonical bundle") from error
    if canonical.project_id != project_id:
        raise CampaignError("portable bundle Project identity does not match the resume capture")
    descriptor["evidence"] = {
        "captures": {
            "work": {"file": relative(root, work_root(root, kind, work) / "evidence/start.rollout.jsonl"), "sha256": harness.sha256(work_root(root, kind, work) / "evidence/start.rollout.jsonl")},
            "resume": {"file": relative(root, destination), "sha256": harness.sha256(destination)},
        },
        "canonical_bundle": {"file": relative(root, bundle), "sha256": harness.sha256(bundle)},
    }
    errors = harness.work_descriptor_errors(descriptor)
    if errors:
        raise CampaignError("completed descriptor does not qualify: " + "; ".join(errors))
    summary_path = work_root(root, kind, work) / "runtime-summary.json"
    activation_path = update_activation_summary(
        root, kind, work, resume_session_start_activation_observed=True
    )
    activation = read_json(activation_path)
    write_json(
        summary_path,
        runtime_summary(
            runtime,
            Path(state["repository_path"]),
            bool(activation["start_session_start_activation_observed"]),
            True,
        ),
    )
    with candidate_artifact_use(campaign, ("volicord",)):
        document_result, document_paths = collect_document_evidence(
            root,
            kind,
            work,
            binary,
            runtime,
            repository,
            project_id,
            campaign["candidate_head"],
            campaign.get("viewer_locale", "en"),
            campaign.get("document_language", "en"),
            documenter,
        )
    document_summary = work_root(root, kind, work) / "documents-summary.json"
    write_json(document_summary, document_result)
    document_review_index = write_operator_document_review_index(
        root, kind, work, document_result
    )
    with candidate_artifact_use(campaign, ("volicord-viewer",)):
        snapshot_result, snapshot_paths = collect_viewer_snapshot_evidence(
            root,
            kind,
            work,
            binary,
            runtime,
            project_id,
            campaign["candidate_head"],
            campaign.get("viewer_locale", "en"),
            campaign.get("document_language", "en"),
            snapshotter,
        )
    snapshot_summary = work_root(root, kind, work) / "viewer-snapshot-summary.json"
    descriptor["evidence"].update({
        "runtime_summary": {
            "file": relative(root, summary_path),
            "sha256": harness.sha256(summary_path),
        },
        "activation_summary": {
            "file": relative(root, activation_path),
            "sha256": harness.sha256(activation_path),
        },
        "generated_documents": {
            "file": relative(root, document_summary),
            "sha256": harness.sha256(document_summary),
        },
        "viewer_snapshot": {
            "file": relative(root, snapshot_summary),
            "sha256": harness.sha256(snapshot_summary),
        },
    })
    write_json(descriptor_path, descriptor)
    state["state"] = final_state
    state["resume_session_id"] = capture.session_id
    state["bundle_sha256"] = harness.sha256(bundle)
    save_campaign(root, campaign)
    for path in (
        destination,
        bundle,
        descriptor_path,
        summary_path,
        activation_path,
        document_summary,
        document_review_index,
        *document_paths,
        *snapshot_paths,
    ):
        register_artifact(
            root,
            path,
            replace=path in {destination, descriptor_path, activation_path},
        )
    return {
        "kind": "phase8_dogfood_resume_intake",
        "outcome": "evidence_collected",
        "repository_class": kind,
        "work": work,
        "project_id": project_id,
        "resume_capture_sha256": capture.source_sha256,
        "canonical_bundle_sha256": harness.sha256(bundle),
        "descriptor_evidence_completed": True,
        "runtime_home_copied": False,
        "document_evidence": document_result,
        "viewer_snapshot_evidence": snapshot_result,
    }


def batch_rollout_paths(
    explicit_paths: list[Path] | None,
    rollout_directory: Path | None,
) -> list[Path]:
    if (explicit_paths is None) == (rollout_directory is None):
        raise CampaignError("collect-batch requires either eight raw rollouts or one directory")
    if rollout_directory is not None:
        directory = rollout_directory.resolve()
        if not directory.is_dir():
            raise CampaignError("batch rollout directory is unavailable")
        entries = sorted(directory.iterdir())
        if len(entries) != BATCH_CAPTURE_COUNT or not all(path.is_file() for path in entries):
            raise CampaignError("batch rollout directory must contain exactly eight files")
        paths = entries
    else:
        paths = [path.resolve() for path in explicit_paths or []]
        if len(paths) != BATCH_CAPTURE_COUNT:
            raise CampaignError("collect-batch requires exactly eight explicit raw rollouts")
    resolved = [path.resolve() for path in paths]
    if len(set(resolved)) != BATCH_CAPTURE_COUNT or not all(path.is_file() for path in resolved):
        raise CampaignError("batch rollout inputs must be eight distinct files")
    return resolved


def map_batch_rollouts(
    root: Path,
    raw_paths: list[Path],
) -> dict[tuple[str, str, str], MappedRollout]:
    campaign = load_campaign(root)
    verify_inventory(root)
    slots: dict[tuple[str, str, str], tuple[dict[str, Any], dict[str, Any]]] = {}
    for kind in CLASSES:
        for work in work_labels(kind):
            _descriptor_path, descriptor = load_frozen_descriptor(root, kind, work, campaign)
            state = campaign["works"][work_key(kind, work)]
            for role in session_roles(kind, work):
                slots[(kind, work, role)] = (state, descriptor)

    mapped: dict[tuple[str, str, str], MappedRollout] = {}
    sessions: dict[str, Path] = {}
    for path in raw_paths:
        try:
            capture = load_codex_capture(path)
        except (OSError, EvidenceError) as error:
            diagnostic = {
                "kind": "phase8_dogfood_batch_mapping_error",
                "source_file": str(path.resolve()),
                "source_sha256": harness.sha256(path) if path.is_file() else None,
                "candidate_count": 0,
                "reason": "capture_provenance_or_format_is_unsupported",
                "mismatch_reasons": ["provenance_or_capture_format_mismatch"],
            }
            raise CampaignError(
                "batch rollout maps to zero frozen task roles",
                diagnostic=diagnostic,
            ) from error
        if evidence_purpose.capture_purpose(capture) != campaign["evidence_purpose"]:
            raise CampaignError("capture authorship/purpose differs from campaign")
        provenance_matches = (
            capture.fresh_user_thread
            and bool(capture.user_turns)
        )
        if not nonempty_session_id(capture.session_id):
            raise CampaignError("batch rollout has no bounded session identity")
        if capture.session_id in sessions:
            raise CampaignError("batch rollout reuses a Codex session identity")
        sessions[capture.session_id] = path
        candidates = []
        candidate_transport: dict[
            tuple[str, str, str], harness.FrozenTaskTransportComparison
        ] = {}
        task_candidates = []
        revision_candidates = []
        workspace_candidates = []
        for slot, (state, descriptor) in slots.items():
            role = slot[2]
            task_field = "work_user_task" if role == "start" else "fresh_resume_user_task"
            comparison = harness.compare_frozen_task_transport(
                descriptor[task_field],
                capture.user_turns[0].text if capture.user_turns else None,
            )
            task_matches = comparison.equivalent
            revision_matches = revision_is_bound(
                Path(state["repository_path"]),
                state["repository_revision"],
                capture.git_revision,
            )
            workspace_matches = capture.cwd.resolve(strict=False) == Path(
                state["repository_path"]
            ).resolve(strict=False)
            if task_matches:
                task_candidates.append(slot)
            if revision_matches:
                revision_candidates.append(slot)
            if workspace_matches:
                workspace_candidates.append(slot)
            if provenance_matches and task_matches and revision_matches and workspace_matches:
                candidates.append(slot)
                candidate_transport[slot] = comparison
        if len(candidates) != 1:
            if not candidates:
                mismatch_reasons = []
                if not provenance_matches:
                    mismatch_reasons.append("provenance_mismatch")
                if not task_candidates:
                    mismatch_reasons.append("frozen_task_mismatch")
                if not revision_candidates:
                    mismatch_reasons.append("repository_revision_mismatch")
                if not workspace_candidates:
                    mismatch_reasons.append("workspace_mismatch")
                if not mismatch_reasons:
                    mismatch_reasons.append("task_revision_workspace_combination_mismatch")
                diagnostic = {
                    "kind": "phase8_dogfood_batch_mapping_error",
                    "source_file": str(path.resolve()),
                    "source_sha256": capture.source_sha256,
                    "candidate_count": 0,
                    "reason": "no_sealed_role_matches_capture_identity",
                    "mismatch_reasons": mismatch_reasons,
                }
                raise CampaignError(
                    "batch rollout maps to zero frozen task roles",
                    diagnostic=diagnostic,
                )
            matching_roles = [
                {
                    "work_slot_id": slots[slot][0]["work_slot_id"],
                    "role": slot[2],
                }
                for slot in candidates[:8]
            ]
            diagnostic = {
                "kind": "phase8_dogfood_batch_mapping_error",
                "source_file": str(path.resolve()),
                "source_sha256": capture.source_sha256,
                "candidate_count": len(candidates),
                "matching_opaque_roles": matching_roles,
                "matching_opaque_roles_truncated": len(candidates) > len(matching_roles),
                "reason": "multiple_sealed_roles_match_capture_identity",
            }
            raise CampaignError(
                "batch rollout maps to multiple frozen task roles",
                diagnostic=diagnostic,
            )
        slot = candidates[0]
        if slot in mapped:
            raise CampaignError("batch rollouts contain duplicate evidence for one frozen task role")
        mapped[slot] = MappedRollout(path, capture, candidate_transport[slot])
    missing = sorted(set(slots) - set(mapped))
    if missing:
        raise CampaignError("batch rollouts are missing one or more frozen task roles")
    collect_journey_git_evidence(campaign, mapped)
    return mapped


def nonempty_session_id(value: Any) -> bool:
    return (
        isinstance(value, str)
        and bool(value.strip())
        and value == value.strip()
        and len(value.encode("utf-8")) <= 512
    )


def activation_failure_diagnostic(
    source: Path,
    capture: Any,
    work_slot_id: str,
    role: str,
) -> dict[str, Any]:
    failure = harness.activation_failure(capture)
    if failure is None:
        raise AssertionError("activated capture cannot have an activation failure diagnostic")
    return {
        "kind": (
            "phase8_dogfood_missing_session_start_activation"
            if capture.activation_evidence_state == "absent"
            else "phase8_dogfood_invalid_session_start_activation"
        ),
        "classification": failure.classification,
        "activation_evidence_state": capture.activation_evidence_state,
        "failure_attribution": {"domain": failure.domain, "basis": failure.basis},
        "source_file": str(source.resolve()),
        "source_sha256": capture.source_sha256,
        "session_id": capture.session_id,
        "work_slot_id": work_slot_id,
        "role": role,
        "volicord_mcp_calls_observed": bool(capture.tool_calls),
        "runtime_session_start_activation_observed": False,
    }


FAILURE_DOMAINS = (
    "environment",
    "product_integration",
    "evidence",
    "behavior_contract",
    "validation_internal",
)
FAILURE_BASES = {
    *RESUME_FAILURE_DOMAINS,
    *harness.WORK_CAPTURE_FAILURE_CHECKS,
    *(failure.basis for failure in harness.ACTIVATION_FAILURES.values()),
    "required_evidence_transport_indeterminate",
    "maintained_work_behavior_contract_failed",
    "work_project_identity_unavailable",
    "resume_supported_evidence_collection_failed",
    "supported_evidence_incomplete",
    "validator_invariant_failure",
}


def bounded_failure_attribution(
    phase: str,
    domain: str,
    basis: str,
    failed_checks: list[str],
) -> dict[str, Any]:
    """Build one evaluator-safe failure attribution without private material."""

    if phase not in {"work", "resume"} or domain not in FAILURE_DOMAINS:
        raise ValueError("invalid Dogfood failure attribution domain")
    checks = sorted(set(failed_checks))
    if not checks or basis not in FAILURE_BASES:
        raise ValueError("invalid Dogfood failure attribution basis")
    return {
        "phase": phase,
        "domain": domain,
        "basis": basis,
        "failed_checks": checks,
    }


def aggregate_batch_failure_attribution(
    works: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    domain_occurrences: Counter[str] = Counter()
    domain_works: dict[str, set[tuple[str, str]]] = {
        domain: set() for domain in FAILURE_DOMAINS
    }
    check_occurrences: Counter[str] = Counter()
    check_works: dict[str, set[tuple[str, str]]] = {}
    for work in works:
        identity = (work["repository_class"], work["work"])
        for attribution in work["failure_attribution"]:
            domain = attribution["domain"]
            domain_occurrences[domain] += 1
            domain_works[domain].add(identity)
            for check in attribution["failed_checks"]:
                check_occurrences[check] += 1
                check_works.setdefault(check, set()).add(identity)
    domains = [
        {
            "domain": domain,
            "work_count": len(domain_works[domain]),
            "attribution_count": domain_occurrences[domain],
        }
        for domain in FAILURE_DOMAINS
        if domain_occurrences[domain]
    ]
    checks = [
        {
            "check": check,
            "work_count": len(check_works[check]),
            "occurrence_count": check_occurrences[check],
        }
        for check in sorted(check_occurrences)
    ]
    return domains, checks


def collect_batch(
    root: Path,
    raw_paths: list[Path],
    *,
    exporter: Callable[[Path, Path, Path, Path], None] = default_export,
    documenter: Callable[
        [Path, Path, Path, str, str, Path, str, str], dict[str, Any]
    ] = generate_document,
    snapshotter: Callable[[Path, Path, str, Path, str, str], dict[str, Any]] = generate_viewer_snapshot,
) -> dict[str, Any]:
    campaign = integrity_check("candidate_binding", load_campaign_for_mutation, root)
    integrity_check("candidate_binding", verify_candidate_artifacts, campaign)
    integrity_check("campaign_inventory", verify_frozen_campaign, root, campaign)
    for journey in campaign["journeys"].values():
        if not journey.get("codex_enabled"):
            raise CampaignError("candidate-owned Codex integration was not activated")
        integrity_check("candidate_binding", verify_static_codex_integration,
            Path(journey["repository_path"]), Path(journey["runtime_home"]),
            Path(campaign["candidate_binary"]))
    if campaign.get("terminal_outcome") is not None:
        raise CampaignError("campaign already stopped; create a new campaign identity")
    if any(state.get("state") != "frozen" for state in campaign["works"].values()):
        raise CampaignError("batch collection requires all five frozen Works")
    # Global identity mapping is read-only and complete before staging anything.
    mapped = integrity_check("session_mapping", map_batch_rollouts, root, raw_paths)
    integrity_check("realization_binding", explanation_evidence.require_ready, root, campaign, mapped)
    integrity_check("realization_binding", document_realization.require_batch_ready, root, campaign, mapped)
    for slot, rollout in mapped.items():
        failure = harness.activation_failure(rollout.capture)
        if failure is not None:
            raise CampaignError("required session activation is invalid",
                diagnostic=activation_failure_diagnostic(rollout.source, rollout.capture,
                    campaign["works"][work_key(*slot[:2])]["work_slot_id"], slot[2]))
    for (kind, work, role), rollout in mapped.items():
        destination = work_root(root, kind, work) / "evidence" / f"{role}.rollout.jsonl"
        if destination.exists() or rollout.source.resolve() == destination.resolve():
            raise IntegrityError("destination_collision", CampaignError("batch evidence destination must be absent and distinct from its source"))
    baseline = {name: (root / name).read_bytes() for name in
                ["campaign.json", "evidence-inventory.json"]}
    collection_run, collection_inputs, collection_producers = collection_runs.request(root, raw_paths, mode="live")
    stage = Path(tempfile.mkdtemp(prefix=".batch-intake-", dir=root))
    try:
        for name in load_inventory(root)["artifacts"]:
            copy_exact(root / name, stage / name)
        write_json(inventory_path(stage), load_inventory(root))
        staged_campaign = copy.deepcopy(campaign)
        staged_campaign["campaign_root"] = str(stage)
        save_campaign(stage, staged_campaign)
        collection_runs.retain(stage, collection_run, collection_inputs, collection_producers)
        for (kind, work, role), rollout in sorted(mapped.items()):
            destination = work_root(stage, kind, work) / "evidence" / f"{role}.rollout.jsonl"
            copy_exact(rollout.source, destination)
            if harness.sha256(destination) != rollout.capture.source_sha256:
                raise IntegrityError("raw_hash", CampaignError("raw capture changed after candidate-bound mapping"))
            register_artifact(stage, destination)
        with candidate_artifact_use(campaign):
            summary = normalize_batch(
                stage, mapped, exporter=exporter, documenter=documenter, snapshotter=snapshotter,
            )
        verify_inventory(stage)
        integrity_check("project_binding", verify_final_repository_states_for_publication, stage)
        staged_campaign = load_campaign(stage)
        staged_campaign["campaign_root"] = str(root)
        save_campaign(stage, staged_campaign)
        collection_runs.unchanged(collection_run, collection_inputs, collection_producers)
        # No supported capture/extraction failure reaches publication as an exception.
        publish_batch(root, stage, baseline)
        return summary
    finally:
        # Interrupted/failed rollback evidence stays available behind the journal.
        if not (root / "batch-publication.json").exists():
            shutil.rmtree(stage)


def publish_batch(root: Path, stage: Path, baseline: dict[str, bytes]) -> None:
    """Controlled publication with rollback and an explicit crash/read barrier.

    Raw captures and derived process logs retain their exact observed bytes.
    Only relative artifact references and campaign metadata name published files.
    """
    staged_campaign = read_json(stage / "campaign.json")
    require_current_candidate(staged_campaign["candidate_head"])
    verify_candidate_artifacts(staged_campaign)
    integrity_check("project_binding", verify_final_repository_states_for_publication, stage)
    verify_inventory(root)
    if any((root / name).read_bytes() != data for name, data in baseline.items()):
        raise CampaignError("campaign changed during batch evaluation")
    names = sorted(load_inventory(stage)["artifacts"])
    names += ["evidence-inventory.json", "campaign.json"]
    known = set(load_inventory(root)["artifacts"]) | {"evidence-inventory.json", "campaign.json"}
    if any((root / name).exists() and name not in known for name in names):
        raise IntegrityError("destination_collision", CampaignError("evidence destination collision during publication"))
    changed = [name for name in names if not (root / name).is_file()
               or harness.sha256(root / name) != harness.sha256(stage / name)]
    backup = stage / "publication-backup"
    originals: set[str] = set()
    for name in changed:
        if (root / name).exists():
            copy_exact(root / name, backup / name)
            originals.add(name)
    journal = root / "batch-publication.json"
    marker = {"kind": "phase8_dogfood_batch_publication", "state": "publishing",
              "candidate_head": read_json(stage / "campaign.json")["candidate_head"],
              "qualification_state": "not_run", "stage_directory": relative(root, stage),
              "artifacts": [{"path": name, "before_sha256": harness.sha256(backup / name) if name in originals else None,
                             "after_sha256": harness.sha256(stage / name)} for name in changed]}
    # Exclusive marker also rejects concurrent publication; readers fail closed.
    with journal.open("x", encoding="utf-8") as output:
        output.write(json.dumps(marker))
        output.flush()
        os.fsync(output.fileno())
    written: list[str] = []
    try:
        for name in changed:
            written.append(name)
            atomic_write_bytes(root / name, (stage / name).read_bytes())
        verify_candidate_artifacts(staged_campaign)
        integrity_check("project_binding", verify_final_repository_states_for_publication, stage)
    except BaseException:
        try:
            for name in reversed(written):
                if name in originals:
                    atomic_write_bytes(root / name, (backup / name).read_bytes())
                else:
                    (root / name).unlink(missing_ok=True)
        except BaseException:
            # The durable marker continues to block every campaign consumer.
            raise CampaignError(
                "batch publication rollback failed; journal requires inspection",
                diagnostic={"kind": "phase8_dogfood_batch_publication_failure",
                            "domain": "evidence", "basis": "batch_publication_incomplete",
                            "qualification_state": "not_run", "outcome": "repair_required"},
            )
        journal.unlink()
        raise
    journal.unlink()


def extract_batch_resume(root: Path, kind: str, work: str, *args: Any, **kwargs: Any) -> dict[str, Any]:
    """Rollback a failed derived extraction inside the unpublished batch stage."""
    before = {path for path in root.rglob("*") if path.is_file()}
    mutable = [campaign_file(root), inventory_path(root), frozen_descriptor_path(root, kind, work),
               work_root(root, kind, work) / "activation-summary.json"]
    saved = {path: path.read_bytes() for path in mutable if path.is_file()}
    try:
        return extract_resume_evidence(root, kind, work, *args, **kwargs)
    except BaseException:
        for path in (path for path in root.rglob("*") if path.is_file() and path not in before):
            path.unlink()
        for path, data in saved.items():
            path.write_bytes(data)
        raise


def work_capture_failure_result(kind: str, work: str, error: harness.WorkCaptureContractError) -> dict[str, Any]:
    return {"kind": "phase8_dogfood_work_intake", "outcome": "evidence_failed",
            "classification": "work_capture_contract_failure", "repository_class": kind,
            "work": work, "basis": error.basis, "failed_checks": [error.check]}


def resolve_batch_identities(root: Path, mapped):
    """Read immutable captures before any candidate-dependent materialization."""
    campaign = load_campaign(root)
    captures_by_work: dict[tuple[str, str], dict[str, Any]] = {}
    journey_projects: dict[str, str] = {}
    journey_work_ids: dict[str, list[str]] = {journey_id(kind): [] for kind in CLASSES}
    work_item_ids: dict[tuple[str, str], str] = {}

    # Resolve the complete identity graph before exporting any journey-final
    # projection. This prevents Work A's export from being mistaken for the
    # final state before later Volicord Works have been accounted for.
    for kind in CLASSES:
        for work_label in work_labels(kind):
            key = work_key(kind, work_label)
            state = campaign["works"][key]
            captures = {
                role: mapped[(kind, work_label, role)].capture
                for role in session_roles(kind, work_label)
            }
            start = captures["start"]
            start_projects = observed_project_ids(start)
            start_work_ids = integrity_check("project_binding", observed_work_item_ids, start, "start")
            if len(start_projects) != 1 or len(start_work_ids) != 1:
                raise IntegrityError(
                    "project_binding",
                    CampaignError("each Work start must expose one Project and one Work identity"),
                )
            project_id = start_projects[0]
            work_item_id = start_work_ids[0]
            identity = journey_id(kind)
            if identity in journey_projects and journey_projects[identity] != project_id:
                raise IntegrityError("project_binding", CampaignError("one journey resolved multiple Project identities"))
            journey_projects[identity] = project_id
            if work_item_id in journey_work_ids[identity]:
                raise IntegrityError("project_binding", CampaignError("distinct Work slots reused one Work identity"))
            journey_work_ids[identity].append(work_item_id)
            if "resume" in captures:
                resume_projects = observed_project_ids(captures["resume"])
                resume_work_ids = integrity_check("project_binding", observed_work_item_ids,
                    captures["resume"], "resume")
                if resume_projects != [project_id] or resume_work_ids != [work_item_id]:
                    raise IntegrityError(
                        "project_binding",
                        CampaignError("fresh resume must resolve the same Project and Work identity"),
                    )
            captures_by_work[(kind, work_label)] = captures
            work_item_ids[(kind, work_label)] = work_item_id
            state["project_id"] = project_id
            state["work_item_id"] = work_item_id
            state["start_session_id"] = start.session_id
            state["resume_session_id"] = captures.get("resume").session_id if "resume" in captures else None

    if len(set(journey_projects.values())) != len(CLASSES):
        raise IntegrityError("project_binding", CampaignError("repository journeys share a Project identity"))
    for identity, project_id in journey_projects.items():
        campaign["journeys"][identity]["project_id"] = project_id
    save_campaign(root, campaign)

    return campaign, captures_by_work, journey_projects, journey_work_ids, work_item_ids


def normalize_batch(
    root: Path, mapped: dict[tuple[str, str, str], MappedRollout], *,
    exporter=default_export, documenter=generate_document, snapshotter=generate_viewer_snapshot,
) -> dict[str, Any]:
    """Freeze observations and supported outputs without behavioral qualification."""
    campaign, captures_by_work, journey_projects, journey_work_ids, work_item_ids = resolve_batch_identities(root, mapped)

    revision_evidence = collect_journey_git_evidence(campaign, mapped)
    for identity, lineage in revision_evidence.items():
        repository = Path(campaign["journeys"][identity]["repository_path"])
        observed, patches = repository_state.observe(repository)
        if observed != lineage["repository_state"]:
            raise IntegrityError("project_binding", CampaignError("journey changed during collection"))
        directory = root / "journeys" / identity / "evidence"
        bindings = {"state": relative(root, directory / "repository-state.json"),
                    "staged": relative(root, directory / "staged.patch"),
                    "unstaged": relative(root, directory / "unstaged.patch"),
                    "git_observations": relative(root, directory / "git-observations.json")}
        write_json(root / bindings["state"], observed)
        write_json(root / bindings["git_observations"], journey_git_projection(identity, lineage))
        for key, data in patches.items():
            (root / bindings[key]).write_bytes(data)
        for name in bindings.values():
            register_artifact(root, root / name)
        lineage["attestation_artifacts"] = bindings
    work_entries: list[dict[str, Any]] = []
    journey_final_evidence: list[dict[str, Any]] = []
    journey_bundle_evidence: dict[str, dict[str, Any]] = {}
    for kind in CLASSES:
        identity = journey_id(kind)
        expected_work_ids = journey_work_ids[identity]
        for work_label in work_labels(kind):
            key = work_key(kind, work_label)
            captures = captures_by_work[(kind, work_label)]
            update_activation_summary(root, kind, work_label,
                start_session_start_activation_observed=True,
                resume_session_start_activation_observed=(True if "resume" in captures else "not_applicable"))
            register_artifact(root, work_root(root, kind, work_label) / "activation-summary.json", replace=True)
            extraction = {"outcome": "not_applicable", "basis": "no_resume_session_for_work"}
            if "resume" in captures:
                extraction = extract_batch_resume(root, kind, work_label, captures["resume"],
                    work_root(root, kind, work_label) / "evidence/resume.rollout.jsonl",
                    exporter=exporter, documenter=documenter, snapshotter=snapshotter,
                    integrity_only=True, final_state="evidence_collected")
            campaign = load_campaign(root)
            campaign["works"][key]["state"] = "evidence_collected"
            if work_label == "A":
                descriptor = read_json(frozen_descriptor_path(root, kind, work_label))
                bundle_binding = descriptor.get("evidence", {}).get("canonical_bundle")
                if not isinstance(bundle_binding, dict):
                    raise IntegrityError(
                        "project_binding",
                        CampaignError("journey-final canonical bundle binding is absent"),
                    )
                bundle = root / str(bundle_binding.get("file", ""))
                try:
                    canonical = harness.load_canonical_bundle(bundle)
                except (OSError, EvidenceError) as error:
                    raise IntegrityError("project_binding", error) from error
                checkpoints_by_work = {
                    work_item_id: sorted(
                        str(row["id"])
                        for row in canonical.rows("checkpoints")
                        if row.get("project_id") == canonical.project_id
                        and row.get("work_item_id") == work_item_id
                        and isinstance(row.get("id"), str)
                    )
                    for work_item_id in expected_work_ids
                }
                if (
                    canonical.project_id != journey_projects[identity]
                    or any(not checkpoints for checkpoints in checkpoints_by_work.values())
                ):
                    raise IntegrityError(
                        "project_binding",
                        CampaignError(
                            "journey-final canonical bundle does not retain every journey Work"
                        ),
                    )
                journey_bundle_evidence[identity] = {
                    "canonical_bundle": copy.deepcopy(bundle_binding),
                    "checkpoint_ids_by_work_item": checkpoints_by_work,
                }
                final_entry = {
                    "journey_id": identity,
                    "project_id": journey_projects[identity],
                    "represented_work_slot_ids": [
                        work_key(kind, label) for label in work_labels(kind)
                    ],
                    "ordered_work_item_ids": expected_work_ids,
                    "projection_source_work_slot_id": key,
                    "artifact_inventory": copy.deepcopy(descriptor["evidence"]),
                    "repository_revision_lineage": revision_evidence[identity],
                }
                journey_final_evidence.append(final_entry)

            if work_label != "A":
                descriptor_path = frozen_descriptor_path(root, kind, work_label)
                descriptor = read_json(descriptor_path)
                journey_descriptor = read_json(frozen_descriptor_path(root, kind, "A"))
                descriptor["evidence"] = copy.deepcopy(journey_descriptor["evidence"])
                start_path = work_root(root, kind, work_label) / "evidence/start.rollout.jsonl"
                descriptor["evidence"]["captures"] = {"work": {
                    "file": relative(root, start_path), "sha256": harness.sha256(start_path)}}
                activation_path = work_root(root, kind, work_label) / "activation-summary.json"
                descriptor["evidence"]["activation_summary"] = {
                    "file": relative(root, activation_path),
                    "sha256": harness.sha256(activation_path),
                }
                write_json(descriptor_path, descriptor)
                register_artifact(root, descriptor_path, replace=True)
            work_item_id = work_item_ids[(kind, work_label)]
            bundle_evidence = journey_bundle_evidence[identity]
            entry = {"journey_id": identity, "repository_class": kind,
                "work_slot_id": key, "work_label": work_label,
                "collection_state": "collected", "intake_state": "accepted",
                "qualification_state": "not_run", "failed_checks": [], "failure_attribution": [],
                "resume_evidence": extraction, "project_id": journey_projects[identity],
                "work_item_id": work_item_id,
                "canonical_evidence": {
                    **copy.deepcopy(bundle_evidence["canonical_bundle"]),
                    "checkpoint_ids": bundle_evidence["checkpoint_ids_by_work_item"][work_item_id],
                    "journey_final_shared_projection": True,
                },
                "sessions": {}}
            for role, capture in captures.items():
                entry["sessions"][role] = {"session_slot_id": session_slot_id(kind, work_label, role),
                    "session_id": capture.session_id,
                    "relative_evidence_path": relative(root, work_root(root, kind, work_label) / "evidence" / f"{role}.rollout.jsonl"),
                    "sha256": capture.source_sha256,
                    "provenance": capture.provenance_evidence(),
                    "turn_lifecycle": capture.turn_lifecycle.bounded_evidence(),
                    "task_transport_equivalence": mapped[(kind, work_label, role)].task_transport.bounded_evidence()}
            work_entries.append(entry)
    campaign = load_campaign(root)
    campaign["collection_state"] = "collected"
    campaign["evaluation_state"] = "not_run"
    campaign["qualification_state"] = "not_run"
    save_campaign(root, campaign)
    summary = {"kind": "phase8_dogfood_batch_intake_summary", "schema_version": 3,
        "candidate_head": campaign["candidate_head"], "collection_state": "collected",
        "intake_state": "accepted", "qualification_state": "not_run", "outcome": "evidence_collected",
        "failed_checks": [], "failure_attribution": [],
        "session_distinctness": {"status": "passed", "expected_count": BATCH_CAPTURE_COUNT,
            "observed_count": len(mapped)}, "journeys": copy.deepcopy(campaign["journeys"]),
        "works": work_entries,
        "journey_final_evidence": journey_final_evidence}
    write_json(root / "batch-intake-summary.json", summary)
    register_artifact(root, root / "batch-intake-summary.json")
    # The manifest closes over exact artifacts, excluding mutable inventory/campaign
    # metadata and all future evaluation runs. Its byte hash is its stable identity.
    manifest = {"kind": "dogfood_evidence_set", "schema_version": 9,
        "collection_run": {"path": "collection/run.json",
            "run_id": read_json(root / "collection/run.json")["run_id"],
            "sha256": harness.sha256(root / "collection/run.json")},
        "campaign_id": campaign["campaign_id"], "candidate_head": campaign["candidate_head"],
        "evidence_purpose": campaign["evidence_purpose"],
        "candidate_artifacts": copy.deepcopy(campaign["candidate_artifacts"]),
        "naturalistic_memory_evidence": copy.deepcopy(campaign["naturalistic_memory_evidence"]),
        "live_evidence_obligations": copy.deepcopy(campaign["live_evidence_obligations"]),
        "explanation_evidence": explanation_evidence.collection_index(root, mapped),
        "raw_inputs": document_realization.raw_binding(mapped),
        "journeys": copy.deepcopy(campaign["journeys"]),
        "works": copy.deepcopy(campaign["works"]),
        "work_evidence": copy.deepcopy(work_entries),
        "journey_final_evidence": copy.deepcopy(journey_final_evidence),
        "artifacts": copy.deepcopy(load_inventory(root)["artifacts"])}
    path = root / "evidence-set.json"
    write_json(path, manifest)
    register_artifact(root, path)
    campaign["evidence_set"] = {"path": "evidence-set.json", "sha256": harness.sha256(path)}
    save_campaign(root, campaign)
    return {**summary, "evidence_set": campaign["evidence_set"]}


def record_resources(root: Path, source: Path) -> dict[str, Any]:
    campaign = load_campaign_for_mutation(root)
    verify_candidate_artifacts(campaign)
    if campaign.get("collection_state") != "pending":
        raise CampaignError("resource attachment requires an uncollected campaign")
    data = source.read_bytes()
    if len(data) > 32 << 20:
        raise CampaignError("resource evidence exceeds bound")
    value = resource_observer.validate(json.loads(data),
        campaign["candidate_artifacts"]["volicord-mcp"]["sha256"])
    evidence_purpose.require_same(campaign, value)
    bindings = {(resource_observer.path_binding(Path(j["runtime_home"])),
        resource_observer.path_binding(Path(j["repository_path"]))) for j in campaign["journeys"].values()}
    if not {r["runtime_binding"] for r in value["runtimes"]} <= {r for r, _ in bindings}:
        raise CampaignError("observed Runtime expectation binding mismatch")
    if any((i["identity"]["runtime_binding"], i["identity"]["cwd_binding"]) not in bindings
            for i in value["instances"]):
        raise CampaignError("observed process Runtime/repository binding mismatch")
    destination = root / "resources/observation.json"
    if destination.exists():
        raise CampaignError("resource evidence is immutable; start a new campaign")
    destination.parent.mkdir(exist_ok=True)
    with destination.open("xb") as output:
        output.write(data)
    register_artifact(root, destination)
    campaign["naturalistic_memory_evidence"] = value
    campaign["live_evidence_obligations"] = live_evidence_obligations(campaign["candidate_artifacts"], value)
    save_campaign(root, campaign)
    return {"status": value["status"], "sample_count": value["measurement"]["sample_count"]}


def load_evidence_set(root: Path) -> dict[str, Any]:
    """Read-only identity verification; never repairs or upgrades older campaigns."""
    collection_runs.require_publication(root)
    campaign = load_campaign(root)
    verify_inventory(root)
    reference = campaign.get("evidence_set")
    if (campaign.get("collection_state") != "collected" or not isinstance(reference, dict)
        or reference.get("path") != "evidence-set.json"
        or reference.get("sha256") != harness.sha256(root / "evidence-set.json")):
        raise CampaignError("campaign has no intact immutable evidence set")
    manifest = read_json(root / "evidence-set.json")
    evidence_purpose.require_same(campaign, manifest, read_json(root / "preparation.json"))
    if (manifest.get("kind") != "dogfood_evidence_set" or manifest.get("schema_version") != 9
        or manifest.get("candidate_head") != campaign["candidate_head"]
        or manifest.get("campaign_id") != campaign["campaign_id"]
        or manifest.get("candidate_artifacts") != campaign.get("candidate_artifacts")
        or manifest.get("journeys") != campaign["journeys"]
        or manifest.get("works") != campaign["works"]
        or len(manifest.get("work_evidence", [])) != QUALIFICATION_WORK_COUNT
        or len(manifest.get("journey_final_evidence", [])) != len(CLASSES)
        or len(manifest.get("raw_inputs", [])) != BATCH_CAPTURE_COUNT):
        raise CampaignError("evidence-set identity or candidate binding mismatch")
    if (manifest.get("naturalistic_memory_evidence")
            != campaign.get("naturalistic_memory_evidence")):
        raise CampaignError("evidence-set naturalistic memory classification changed")
    if (manifest.get("naturalistic_memory_evidence")
                != campaign.get("naturalistic_memory_evidence")
                or manifest.get("live_evidence_obligations")
                != campaign.get("live_evidence_obligations")):
        raise CampaignError("evidence-set live evidence obligations changed")
    resource_observer.validate(manifest["naturalistic_memory_evidence"],
        manifest["candidate_artifacts"]["volicord-mcp"]["sha256"])
    resource_observer.validate_bindings(manifest["naturalistic_memory_evidence"], manifest["journeys"])
    if "resources/observation.json" in manifest["artifacts"]:
        if read_json(root / "resources/observation.json") != manifest["naturalistic_memory_evidence"]:
            raise CampaignError("resource artifact/manifest disagreement")
    elif manifest["naturalistic_memory_evidence"] != resource_observer.initial(manifest["candidate_artifacts"], purpose=manifest["evidence_purpose"]):
        raise CampaignError("attached resource evidence lacks immutable artifact")
    for name, binding in manifest["artifacts"].items():
        path = root / name
        if relative(root, path) != name or not path.is_file() or binding != {
            "bytes": path.stat().st_size, "sha256": harness.sha256(path)}:
            raise CampaignError("immutable evidence-set artifact changed")
    expected_slots = set(harness.current_session_slots())
    if {tuple(item.get("session_slot", [])) for item in manifest["raw_inputs"]} != expected_slots:
        raise CampaignError("evidence-set rollout coverage changed")
    sessions = set()
    for item in manifest["raw_inputs"]:
        kind, work, role = item["session_slot"]
        state = manifest["works"][work_key(kind, work)]
        raw = root / "slots" / state["work_slot_id"] / "evidence" / f"{role}.rollout.jsonl"
        if (not nonempty_session_id(item.get("session_id")) or item["session_id"] in sessions
            or item["session_id"] != state.get(f"{role}_session_id")
            or item.get("sha256") != harness.sha256(raw)):
            raise CampaignError("evidence-set raw session/hash binding changed")
        sessions.add(item["session_id"])
    preparation = read_json(root / "preparation.json")
    task_hashes = preparation.get("task_sha256_by_session_slot", {})
    if preparation.get("candidate_head") != campaign["candidate_head"] or len(task_hashes) != BATCH_CAPTURE_COUNT:
        raise CampaignError("evidence-set preparation receipt changed")
    for state in manifest["works"].values():
        kind, label = state["repository_class"], state["work_label"]
        _path, descriptor = load_frozen_descriptor(root, kind, label, campaign)
        verify_operator_task_artifacts(root, state, descriptor)
        for role, artifact in state["operator_task_artifacts"].items():
            if task_hashes.get(session_slot_id(kind, label, role)) != artifact["sha256"]:
                raise CampaignError("evidence-set frozen task differs from preparation receipt")
    mapped = {slot: type("RetainedRollout", (), {"capture": load_codex_capture(root / "slots" /
        manifest["works"][work_key(*slot[:2])]["work_slot_id"] / "evidence" / f"{slot[2]}.rollout.jsonl")})()
        for slot in harness.current_session_slots()}
    if any(evidence_purpose.capture_purpose(v.capture) != manifest["evidence_purpose"] for v in mapped.values()):
        raise CampaignError("retained capture authorship/purpose differs from campaign")
    if manifest.get("explanation_evidence") != explanation_evidence.collection_index(root, mapped):
        raise CampaignError("immutable explanation observation/lifecycle index changed")
    integrity_check("project_binding", verify_retained_repository_states, root, manifest)
    collection_runs.verify(root, manifest)
    return manifest


def evaluate_works(root: Path, manifest: dict[str, Any]) -> list[dict[str, Any]]:
    """One observation engine for intact evidence and incomplete historical diagnostics."""
    works = []
    for kind in CLASSES:
        for work in work_labels(kind):
            state = manifest["works"][work_key(kind, work)]
            descriptor_path = root / "tasks/descriptors" / f"{state['work_slot_id']}.json"
            descriptor = read_json(descriptor_path)
            descriptor["_journey_projection_context"] = {
                "candidate_head": manifest["candidate_head"],
                "journey": manifest["journeys"][journey_id(kind)],
                "final_entries": manifest.get("journey_final_evidence", []),
                "work_entries": manifest.get("work_evidence", []),
            }
            descriptor["_evidence_directory"] = str(root)
            descriptor["_evidence_file_sha256"] = harness.sha256(descriptor_path)
            observation = harness.real_session_evidence(descriptor, kind=kind, cycle=work,
                repository_revision=state["repository_revision"], candidate_revision=manifest["candidate_head"],
                target_repository=Path(state["repository_path"]))
            resume_pair = "resume" in session_roles(kind, work)
            if not resume_pair:
                for check in machine_findings.RESUME_RULES:
                    observation["checks"].pop(check, None)
            # The enclosing immutable observation retains every existing check/basis.
            observation.pop("machine_findings", None)
            works.append({"journey_id": journey_id(kind), "repository_class": kind,
                "work_slot_id": work_key(kind, work), "work": work, "resume_pair": resume_pair,
                "workload_intent": descriptor["workload_intent"],
                "observation": observation,
                "findings": machine_findings.from_observation(observation)})
    return works


def evaluate_journeys(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    """Derive structural continuity only from the immutable main-campaign manifest."""
    work_evidence = {item["work_slot_id"]: item for item in manifest["work_evidence"]}
    final_evidence = {item["journey_id"]: item for item in manifest["journey_final_evidence"]}
    project_ids = [manifest["journeys"][journey_id(kind)].get("project_id") for kind in CLASSES]
    workspace_ids = [manifest["journeys"][journey_id(kind)].get("repository_path") for kind in CLASSES]
    runtime_ids = [manifest["journeys"][journey_id(kind)].get("runtime_home") for kind in CLASSES]
    isolated = (
        len(set(project_ids)) == len(CLASSES)
        and len(set(workspace_ids)) == len(CLASSES)
        and len(set(runtime_ids)) == len(CLASSES)
        and all(project_ids)
    )

    def checked(passed: bool, basis: dict[str, Any]) -> dict[str, Any]:
        return {
            "status": (
                machine_findings.Status.PASS.value
                if passed
                else machine_findings.Status.VIOLATION.value
            ),
            "basis": basis,
        }

    result = []
    for kind in CLASSES:
        identity = journey_id(kind)
        journey = manifest["journeys"][identity]
        slots = [work_key(kind, label) for label in work_labels(kind)]
        entries = [work_evidence.get(slot) for slot in slots]
        final = final_evidence.get(identity, {})
        expected_project = journey.get("project_id")
        ordered_work_ids = [entry.get("work_item_id") if isinstance(entry, dict) else None for entry in entries]
        project_identity_ok = bool(expected_project) and final.get("project_id") == expected_project and all(
            isinstance(entry, dict) and entry.get("project_id") == expected_project for entry in entries
        )
        work_identity_ok = (
            all(isinstance(value, str) and value for value in ordered_work_ids)
            and len(set(ordered_work_ids)) == len(slots)
            and final.get("ordered_work_item_ids") == ordered_work_ids
            and final.get("represented_work_slot_ids") == slots
        )
        history_ok = all(
            isinstance(entry, dict)
            and entry.get("collection_state") == "collected"
            and isinstance(entry.get("canonical_evidence", {}).get("checkpoint_ids"), list)
            and bool(entry["canonical_evidence"]["checkpoint_ids"])
            and entry["canonical_evidence"].get("journey_final_shared_projection") is True
            for entry in entries
        )
        canonical_files = {
            entry.get("canonical_evidence", {}).get("file")
            for entry in entries if isinstance(entry, dict)
        }
        relation_ok = (
            project_identity_ok and work_identity_ok and history_ok
            and len(canonical_files) == 1 and None not in canonical_files
            and all(
                entry["canonical_evidence"].get("sha256")
                == entries[0]["canonical_evidence"].get("sha256")
                for entry in entries if isinstance(entry, dict)
            )
        )
        resume_slots = [entry for entry in entries if isinstance(entry, dict) and "resume" in entry.get("sessions", {})]
        resume_ok = (
            len(resume_slots) == 1
            and resume_slots[0].get("work_label") == harness.RESUME_WORK_SLOT_BY_REPOSITORY[kind]
            and set(resume_slots[0]["sessions"]) == {"start", "resume"}
            and resume_slots[0]["sessions"]["start"].get("session_id")
                != resume_slots[0]["sessions"]["resume"].get("session_id")
        )
        observation = {"evidence_class": "immutable_main_campaign_journey_structure", "checks": {
            "journey_project_identity": checked(project_identity_ok, {
                "journey_id": identity, "expected_project_id": expected_project,
                "work_project_ids": [entry.get("project_id") if isinstance(entry, dict) else None for entry in entries],
            }),
            "journey_work_identity": checked(work_identity_ok, {
                "journey_id": identity, "expected_work_slot_ids": slots,
                "ordered_work_item_ids": ordered_work_ids,
            }),
            "journey_work_history": checked(history_ok, {
                "journey_id": identity,
                "checkpoint_ids_by_work_slot": {
                    slot: (entry or {}).get("canonical_evidence", {}).get("checkpoint_ids", [])
                    for slot, entry in zip(slots, entries, strict=True)
                },
            }),
            "journey_relation_consistency": checked(relation_ok, {
                "journey_id": identity, "project_id": expected_project,
                "work_item_ids": ordered_work_ids, "shared_canonical_files": sorted(str(v) for v in canonical_files),
            }),
            "journey_resume_continuity": checked(resume_ok, {
                "journey_id": identity, "required_resume_work_slot_id": work_key(kind, "A"),
                "observed_resume_work_slot_ids": [entry["work_slot_id"] for entry in resume_slots],
            }),
            "journey_isolation": checked(isolated, {
                "project_ids": project_ids, "workspace_identities": workspace_ids,
                "runtime_identities": runtime_ids,
            }),
        }}
        observation["git_evidence"] = final_evidence[identity].get("repository_revision_lineage")
        result.append({"journey_id": identity, "repository_class": kind,
            "work_slot_ids": slots, "resume_work_slot_id": work_key(kind, "A"),
            "observation": observation,
            "findings": machine_findings.from_journey_observation(observation)})
    return result


def evaluate_campaign(root: Path, output: Path | None = None, previous: Path | None = None) -> dict[str, Any]:
    """Append a machine run over frozen evidence; no export, replay or qualification."""
    campaign = load_campaign(root)
    manifest = load_evidence_set(root)
    from evaluation_runs import policy_identity, historical_reference
    prior = historical_reference(previous, manifest["candidate_head"], campaign["evidence_set"]) if previous else None
    works = evaluate_works(root, manifest)
    journeys = evaluate_journeys(manifest)
    result = {"kind": "dogfood_machine_evaluation", "schema_version": 4,
        "candidate_head": manifest["candidate_head"], "evidence_set": campaign["evidence_set"],
        "evidence_purpose": manifest["evidence_purpose"],
        "evaluator_revision": harness.git_head(ROOT), "policy_version": machine_findings.POLICY_VERSION,
        "evaluator_files": {name: harness.sha256(Path(__file__).parent / name)
            for name in ("harness.py", "codex_events.py", "machine_findings.py", "machine-policy.json", "campaign.py",
                "authority_obligations.py", "document_realization.py", "identity_provenance.py", "evaluation_runs.py", "answer_observations.py", "explanation_evidence.py", "answer_projection.py", "review_captures.py", "review_explanations.py", "review_operations.py",
                "../shared/recorded_action_evidence.py",
                "evaluation.json", "collection_runs.py", "interaction_diagnostics.py", "workload_intents.py", "support_evidence.py", "evidence_purpose.py")},
        "policy": policy_identity(), "qualitative_review_runs": [], "previous_evaluation": prior,
        "run_nonce": secrets.token_hex(16), "collection_state": "collected",
        "evaluation_state": "produced", "qualification_state": "not_run", "works": works,
        "journeys": journeys,
        "interaction_diagnostics": interaction_diagnostics.campaign_summary({w["work_slot_id"]: w["observation"]["interaction_diagnostics"] for w in works}),
        "coverage": {"repository_journeys": 3, "work_items": 5, "resume_pairs": 3,
            "fresh_sessions": 8,
            "work_distribution": {"volicord": 3, "small-python": 1, "polyglot-medium": 1},
            "resume_repository_classes": sorted(CLASSES)},
        "finding_state": machine_findings.evaluation_state(
            [f for item in works + journeys for f in item["findings"]])}
    result["run_id"] = machine_findings.digest(result)
    machine_findings.validate_run(result)
    # Recheck input hashes after evaluation and before the controlled publication.
    load_evidence_set(root)
    if previous and historical_reference(previous, manifest["candidate_head"], campaign["evidence_set"]) != prior:
        raise CampaignError("previous evaluation changed during replay")
    destination = output or root.parent / (root.name + "-evaluations") / result["run_id"]
    if destination.resolve().is_relative_to(root.resolve()):
        raise CampaignError("evaluation output must be outside the immutable campaign")
    from evaluation_runs import publish
    result_name = str(publish(destination, result))
    return {"evaluation": result_name, "run_id": result["run_id"],
        "finding_state": result["finding_state"], "qualification_state": "not_run"}


def diagnose_campaign(root: Path, output: Path) -> dict[str, Any]:
    """Inspect a historical inventory without replay, repair, or qualification."""
    campaign = read_json(campaign_file(root))
    if (campaign.get("kind") != "phase8_dogfood_campaign"
            or campaign.get("schema_version") not in {1, 2, 3, 4, 5, 6, 7, 8, 9}
            or Path(campaign.get("campaign_root", "")).resolve() != root.resolve()):
        raise CampaignError("unexpected dogfood campaign metadata")
    verify_inventory(root)
    names = set(load_inventory(root)["artifacts"]) | {"campaign.json", "evidence-inventory.json"}
    names.update(relative(root, path) for path in (root / "raw-rollouts").glob("*.jsonl"))
    def snapshot():
        return {name: {"sha256": harness.sha256(root / name), "bytes": (root / name).stat().st_size}
            for name in sorted(names)}
    before = snapshot()
    result = {
        "kind": "dogfood_historical_campaign_diagnostic",
        "schema_version": 1,
        "historical_campaign_schema_version": campaign["schema_version"],
        "candidate_head": campaign.get("candidate_head"),
        "historical_campaign_sha256": before["campaign.json"]["sha256"],
        "historical_terminal_outcome": campaign.get("terminal_outcome"),
        "artifacts": before,
        "inspection_state": "identity_and_inventory_only",
        "qualification_state": "not_run",
        "replacement_pass_candidate": False,
        "phase_9_ready": False,
        "limitation": "Inventory inspection cannot create or upgrade collected campaign evidence.",
    }
    result["run_id"] = machine_findings.digest(result)
    if snapshot() != before:
        raise CampaignError("historical evidence changed during diagnostic inspection")
    if output.resolve().is_relative_to(root.resolve()):
        raise CampaignError("diagnostic output must be outside historical campaign")
    review_operations.publish_directory(output, {"diagnostic.json": json_bytes(result)})
    return {"diagnostic": str(output / "diagnostic.json"),
        "run_id": result["run_id"], "candidate_head": result["candidate_head"],
        "inspection_state": result["inspection_state"],
        "qualification_state": "not_run", "phase_9_ready": False}


def finalize_manifest(root: Path, output: Path | None = None) -> Path:
    campaign = load_campaign_for_mutation(root)
    verify_inventory(root)
    if campaign.get("terminal_outcome") is not None:
        raise CampaignError("a stopped campaign cannot produce a qualifying repository manifest")
    if campaign.get("collection_state") == "collected":
        load_evidence_set(root)
    specs = repository_spec_map(read_json(root / campaign["repository_input"]))
    repositories: list[dict[str, Any]] = []
    for kind in CLASSES:
        spec = specs[kind]
        real: dict[str, str] = {}
        for number in work_labels(kind):
            state = campaign["works"][work_key(kind, number)]
            if state["state"] not in {"resume_collected", "evidence_collected"}:
                raise CampaignError("all five Works must have collected evidence before finalization")
            descriptor, _ = load_frozen_descriptor(root, kind, number, campaign)
            real[work_key(kind, number)] = relative(root, descriptor)
        repositories.append({
            **{key: spec[key] for key in ("class", "path", "origin", "revision", "license_file", "license_spdx", "provider_source_path") if key in spec},
            "revision": campaign["candidate_head"] if kind == "volicord" else spec["revision"],
            "real_session_evidence": real,
        })
    destination = output.resolve() if output else root / "repositories.json"
    if destination.parent != root:
        raise CampaignError("repository manifest must remain at the campaign root")
    if destination.exists():
        if read_json(destination) != {"repositories": repositories}:
            raise CampaignError("repository manifest is immutable")
        return destination
    write_json(destination, {"repositories": repositories})
    register_artifact(
        root,
        destination,
        replace=relative(root, destination) in load_inventory(root)["artifacts"],
    )
    return destination


def safe_archive_artifact(name: str, *, include_raw: bool) -> bool:
    path = Path(name)
    if path.name == "realization-bindings.json":
        return False
    lowered = name.casefold()
    if path.name in RAW_NAMES:
        return include_raw
    if any(lowered.endswith(suffix) for suffix in PROHIBITED_ARCHIVE_SUFFIXES):
        return False
    if any(
        part
        in {
            "runtime",
            "install",
            "bootstrap-runtime",
            "derived",
            "document-export-processes",
            "realizer",
        }
        for part in path.parts
    ):
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


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="command", required=True)
    rehearse = sub.add_parser("rehearse-evidence", help="Run local Product-backed support; standalone receipt is diagnostic")
    rehearse.add_argument("--candidate-head", required=True)
    rehearse.add_argument("--output", required=True)
    rehearse.add_argument("--bin-dir")
    prepare = sub.add_parser("prepare")
    prepare.add_argument("--campaign-root", required=True)
    prepare.add_argument("--campaign-id", required=True)
    prepare.add_argument("--candidate-head", required=True)
    prepare.add_argument("--repositories", required=True)
    prepare.add_argument("--tasks", required=True, help="JSON mapping five Work slots to eight UTF-8 task files")
    prepare.add_argument("--gate-capsule", help="Retained exact-candidate technical capsule; verified without execution")
    prepare.add_argument("--gate-archive", help="Matching retained technical archive")
    prepare.add_argument("--observe-resources", action="store_true",
        help="Select optional operator characterization; starts no observer")
    activate = sub.add_parser("activate-journey")
    activate_every = sub.add_parser("activate-all")
    reprocess = sub.add_parser("reprocess-collection", help="Publish retained observations without mutating or rebinding their Product campaign")
    reprocess.add_argument("--campaign-root", required=True)
    reprocess.add_argument("--source-campaign-sha256", required=True)
    reprocess.add_argument("--rollout-directory", required=True)
    reprocess.add_argument("--rejected-attempt", action="append", default=[])
    reprocess.add_argument("--output", required=True)
    verify_collection = sub.add_parser("verify-collection", help="Independently verify a copied collection publication")
    verify_collection.add_argument("--campaign-root", required=True)
    collect_b = sub.add_parser("collect-batch")
    prepare_explanations = sub.add_parser("prepare-explanations", help="Prepare private post-session Work/Decision explanation evidence before document realization")
    record_explanation = sub.add_parser("record-explanation", help="Record an active-host response through the existing Product CLI and retain readback")
    inspect_explanations = sub.add_parser("inspect-explanations", help="Inspect immutable explanation lifecycle progress")
    prepare_documents = sub.add_parser("prepare-document-realizations")
    inspect_documents = sub.add_parser("inspect-document-realizations",
        help="Inspect published preparation and immutable realization counts without mutation")
    validate_document = sub.add_parser("validate-document-realization")
    record_document = sub.add_parser("record-document-realization")
    bind_document_provenance = sub.add_parser("bind-document-realization-provenance",
        help="Bind stronger host-recorded runtime identity into a mutable realization draft")
    diagnostic = sub.add_parser("diagnose", help="Read-only historical inventory diagnostic when a collection receipt is unavailable; never qualifies")
    diagnostic.add_argument("--campaign-root", required=True)
    diagnostic.add_argument("--output", required=True)
    evaluate = sub.add_parser("evaluate", help="Append a new evaluation of immutable evidence, including an older Product candidate")
    evaluate.add_argument("--campaign-root", required=True)
    evaluate.add_argument("--output", help="New immutable run directory outside the campaign")
    evaluate.add_argument("--previous-evaluation", help="Historical run to retain by identity/hash for comparison")
    qualify = sub.add_parser("qualify", help="Combine verified candidate technical gate and evidence-bound qualitative reviews")
    qualify.add_argument("--campaign-root", required=True)
    qualify.add_argument("--candidate-head", required=True)
    qualify.add_argument("--machine-evaluation", required=True)
    qualify.add_argument("--review-root", action="append", default=[])
    qualify.add_argument("--gate-capsule")
    qualify.add_argument("--gate-archive")
    qualify.add_argument("--output", required=True)
    approve = sub.add_parser("approve-phase-9", help="Explicit operator authorization of a complete qualification")
    approve.add_argument("--qualification", required=True)
    approve.add_argument("--operator", required=True)
    approve.add_argument("--authorization", choices=["approve-phase-9"], required=True)
    approve.add_argument("--output", required=True)
    validate_qualification = sub.add_parser("validate-qualification", help="Recheck all exact qualification inputs without mutation")
    validate_qualification.add_argument("--qualification", required=True)
    validate_approval = sub.add_parser("validate-approval", help="Verify immutable operator approval and all bound qualification inputs")
    validate_approval.add_argument("--approval", required=True)
    validate_approval.add_argument("--qualification", required=True)
    publish_lineage = sub.add_parser("publish-result-lineage",
        help="Publish a durable self-contained evaluation/review/qualification lineage")
    publish_lineage.add_argument("--campaign-root", required=True)
    publish_lineage.add_argument("--machine-evaluation", required=True)
    publish_lineage.add_argument("--review-root", action="append", default=[])
    publish_lineage.add_argument("--qualification", required=True)
    publish_lineage.add_argument("--approval")
    publish_lineage.add_argument("--output")
    verify_lineage = sub.add_parser("verify-result-lineage",
        help="Verify a copied result lineage without original staging paths")
    verify_lineage.add_argument("--lineage-root", required=True)
    finalize = sub.add_parser("finalize-manifest")
    package = sub.add_parser("package-review")
    prepare_qualitative = sub.add_parser("prepare-qualitative-review")
    inspect_agent = sub.add_parser("inspect-agent-review",
        help="Present one agent criterion and its bound evidence without suggesting a verdict")
    resources = sub.add_parser("record-resources", help="Attach immutable candidate-bound MCP observation")
    resources.add_argument("--campaign-root", required=True)
    resources.add_argument("--input", required=True)
    capture_human = sub.add_parser("capture-human-viewer-observations",
        help="Conversationally capture candidate-bound live Viewer observations")
    converse_human = sub.add_parser("converse-qualitative-review",
        help="Capture one human-owned criterion without hand-authoring review JSON")
    collect_cli = sub.add_parser("collect-cli-observations", help="Collect isolated repository-class CLI usability evidence")
    validate_qualitative = sub.add_parser("validate-qualitative-review")
    record_qualitative = sub.add_parser("record-qualitative-review")
    prepare_qualitative.add_argument("--campaign-root", required=True)
    prepare_qualitative.add_argument("--output", required=True)
    prepare_qualitative.add_argument("--reviewer-kind", choices=("agent", "human"), required=True)
    prepare_qualitative.add_argument("--review-session-id")
    prepare_qualitative.add_argument("--reviewer-identity", help="JSON file with explicit unverified identity claims")
    prepare_qualitative.add_argument("--machine-evaluation")
    prepare_qualitative.add_argument("--include-raw-rollouts", action="store_true",
        help="Use immutable raw Work/resume rollouts as inputs to bounded reviewer-safe capture projections")
    prepare_qualitative.add_argument("--human-observations", help="Candidate/evidence-bound direct human en/ko live accessibility observations")
    prepare_qualitative.add_argument("--cli-observations", help="Candidate/evidence-bound repository-class CLI observation directory")
    inspect_agent.add_argument("--review-root", required=True)
    inspect_agent.add_argument("--criterion-number", type=int, required=True)
    capture_human.add_argument("--campaign-root", required=True)
    capture_human.add_argument("--output", required=True)
    capture_human.add_argument("--viewer-context", type=Path, action="append", required=True,
        help="Browser display-context directory; supply actually inspected en and ko captures")
    converse_human.add_argument("--review-root", required=True)
    converse_human.add_argument("--criterion-number", type=int)
    converse_human.add_argument("--resolve-review-root", action="append", default=[])
    collect_cli.add_argument("--campaign-root", required=True)
    collect_cli.add_argument("--output", required=True)
    for operation in (validate_qualitative, record_qualitative):
        operation.add_argument("--review-root", required=True)
        operation.add_argument("--draft", required=True)
    activate.add_argument("--campaign-root", required=True)
    activate.add_argument("--repository-class", choices=CLASSES, required=True)
    activate_every.add_argument("--campaign-root", required=True)
    collect_b.add_argument("--campaign-root", required=True)
    batch_input = collect_b.add_mutually_exclusive_group(required=True)
    batch_input.add_argument("--raw-rollout", action="append")
    batch_input.add_argument("--rollout-directory")
    prepare_explanations.add_argument("--campaign-root", required=True)
    explanation_inputs = prepare_explanations.add_mutually_exclusive_group(required=True)
    explanation_inputs.add_argument("--raw-rollout", action="append")
    explanation_inputs.add_argument("--rollout-directory")
    prepare_explanations.add_argument("--language", action="append")
    prepare_explanations.add_argument("--work", action="append")
    prepare_explanations.add_argument("--decision", action="append")
    record_explanation.add_argument("--campaign-root", required=True)
    record_explanation.add_argument("--explanation-id", required=True)
    record_explanation.add_argument("--input", required=True)
    inspect_explanations.add_argument("--campaign-root", required=True)
    prepare_documents.add_argument("--campaign-root", required=True)
    inspect_documents.add_argument("--campaign-root", required=True)
    document_inputs = prepare_documents.add_mutually_exclusive_group(required=True)
    document_inputs.add_argument("--raw-rollout", action="append")
    document_inputs.add_argument("--rollout-directory")
    for command in (validate_document, record_document, bind_document_provenance):
        command.add_argument("--campaign-root", required=True)
        command.add_argument("--realization-id", required=True)
        command.add_argument("--draft", required=True)
    bind_document_provenance.add_argument("--runtime-rollout", required=True)
    for command in (validate_document, record_document):
        command.add_argument("--runtime-rollout")
    finalize.add_argument("--campaign-root", required=True)
    package.add_argument("--review-root", required=True)
    package.add_argument("--output", required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    if args.command == "rehearse-evidence":
        import sys
        argv = [sys.executable, "-B", str(Path(__file__).with_name("rehearsal.py")), "--candidate-head", args.candidate_head, "--output", args.output]
        if args.bin_dir:
            argv.extend(["--bin-dir", args.bin_dir])
        return subprocess.run(argv, check=False).returncode
    if args.command == "verify-result-lineage":
        print(json.dumps(result_lineage.verify(Path(args.lineage_root)), indent=2, sort_keys=True))
        return 0
    if args.command == "validate-approval":
        import qualification_policy
        print(json.dumps(qualification_policy.verify_approval(Path(args.approval), Path(args.qualification)), indent=2, sort_keys=True))
        return 0
    if args.command in {"approve-phase-9", "validate-qualification"}:
        import qualification_policy
        value = (qualification_policy.approve(Path(args.qualification), Path(args.output),
            operator=args.operator, statement=args.authorization) if args.command == "approve-phase-9"
            else qualification_policy.verify_qualification(Path(args.qualification)))
        print(json.dumps(value, indent=2, sort_keys=True))
        return 0
    root = Path(getattr(args, "campaign_root", None) or args.review_root).resolve()
    if args.command == "prepare":
        value = prepare_campaign(root, args.campaign_id, args.candidate_head,
            Path(args.repositories).resolve(), Path(args.tasks).resolve(),
            capsule_path=Path(args.gate_capsule) if args.gate_capsule else None,
            archive_path=Path(args.gate_archive) if args.gate_archive else None,
            observe_resources=args.observe_resources)
    elif args.command == "activate-journey":
        value = activate_journey(root, args.repository_class)
    elif args.command == "activate-all":
        value = activate_all(root)
    elif args.command in {"validate-document-realization", "record-document-realization"}:
        operation = document_realization.validate if args.command == "validate-document-realization" else document_realization.record
        value = operation(root, args.realization_id, Path(args.draft).resolve(),
            Path(args.runtime_rollout).resolve() if args.runtime_rollout else None)
    elif args.command == "bind-document-realization-provenance":
        value = document_realization.bind_runtime_provenance(root, args.realization_id,
            Path(args.draft).resolve(), Path(args.runtime_rollout).resolve())
    elif args.command == "inspect-document-realizations":
        value = document_realization.inspect_state(root)
    elif args.command == "record-explanation":
        value = explanation_evidence.record(root, args.explanation_id, Path(args.input).resolve())
    elif args.command == "inspect-explanations":
        value = explanation_evidence.inspect(root)
    elif args.command in {"collect-batch", "prepare-document-realizations", "prepare-explanations"}:
        paths = batch_rollout_paths(
            [Path(path) for path in args.raw_rollout] if args.raw_rollout else None,
            Path(args.rollout_directory) if args.rollout_directory else None,
        )
        value = (explanation_evidence.prepare(root, paths, languages=args.language,
                    work_ids=args.work, decision_ids=args.decision) if args.command == "prepare-explanations"
                 else document_realization.prepare(root, paths, progress=document_realization.stderr_progress)
                 if args.command == "prepare-document-realizations"
                 else collect_batch(root, paths))
    elif args.command == "reprocess-collection":
        value = collection_runs.reprocess(root, batch_rollout_paths(None, Path(args.rollout_directory)),
            Path(args.output), source_sha256=args.source_campaign_sha256,
            rejected=[Path(p) for p in args.rejected_attempt])
    elif args.command == "verify-collection":
        value = collection_runs.verify_publication(root)
    elif args.command == "diagnose":
        value = diagnose_campaign(root, Path(args.output))
    elif args.command == "evaluate":
        value = evaluate_campaign(root, Path(args.output) if args.output else None,
            Path(args.previous_evaluation) if args.previous_evaluation else None)
    elif args.command == "qualify":
        import qualification_policy
        value = qualification_policy.qualify(root, Path(args.machine_evaluation), Path(args.output),
            candidate=args.candidate_head, review_roots=[Path(p) for p in args.review_root],
            capsule_path=Path(args.gate_capsule) if args.gate_capsule else None,
            archive_path=Path(args.gate_archive) if args.gate_archive else None)
    elif args.command == "publish-result-lineage":
        value = result_lineage.publish(root, Path(args.machine_evaluation),
            [Path(path) for path in args.review_root], Path(args.qualification),
            Path(args.output) if args.output else None,
            Path(args.approval) if args.approval else None)
    elif args.command == "finalize-manifest":
        value = {"manifest": str(finalize_manifest(root))}
    elif args.command == "prepare-qualitative-review":
        value = review_operations.prepare(root, Path(args.output), reviewer_kind=args.reviewer_kind,
            session_id=args.review_session_id,
            identity=read_json(Path(args.reviewer_identity)) if args.reviewer_identity else None,
            evaluation_path=Path(args.machine_evaluation) if args.machine_evaluation else None,
            include_raw=args.include_raw_rollouts,
            human_observations=Path(args.human_observations) if args.human_observations else None,
            cli_observation_root=Path(args.cli_observations) if args.cli_observations else None)
    elif args.command == "inspect-agent-review":
        value = review_operations.inspect_agent_criterion(root, args.criterion_number)
    elif args.command == "record-resources":
        value = record_resources(root, Path(args.input))
    elif args.command == "capture-human-viewer-observations":
        value = human_review.capture_viewer_observations(root, Path(args.output), context_paths=args.viewer_context)
    elif args.command == "converse-qualitative-review":
        value = human_review.converse_one(root, criterion_number=args.criterion_number,
            resolve_review_roots=[Path(path).resolve() for path in args.resolve_review_root])
    elif args.command == "collect-cli-observations":
        value = cli_observations.collect(root, Path(args.output))
    elif args.command in {"validate-qualitative-review", "record-qualitative-review"}:
        operation = review_operations.validate if args.command == "validate-qualitative-review" else review_operations.record
        value = operation(root, Path(args.draft))
    else:
        value = review_operations.package_review(root, Path(args.output))
    print(json.dumps(value, indent=2, sort_keys=True))
    return 1 if args.command == "reprocess-collection" and value["collection_state"] != "collected" else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CampaignError, EvidenceError, OSError, ValueError) as error:
        output = {"status": "failed", "error": str(error)}
        if isinstance(error, CampaignError) and error.diagnostic is not None:
            output["diagnostic"] = error.diagnostic
        print(json.dumps(output, sort_keys=True))
        raise SystemExit(1)
