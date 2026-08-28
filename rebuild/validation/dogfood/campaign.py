#!/usr/bin/env python3
"""Maintain private Phase 8 naturalistic-dogfood campaign evidence.

This module never starts a Codex session, grants repository trust, or reads
canonical SQLite contents.  It prepares and validates evidence produced by an
operator-owned campaign and reuses the maintained dogfood normalizer.
"""

from __future__ import annotations

import argparse
from collections import Counter
import copy
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from typing import Any, Callable

import harness
from codex_events import EvidenceError, command_is_repository_inspection, load_codex_capture


ROOT = Path(__file__).resolve().parents[3]
CLASSES = harness.CLASSES
BEHAVIOR_CLASSES = harness.BEHAVIOR_CLASSES
CYCLE_COUNT_BY_REPOSITORY = harness.CYCLE_COUNT_BY_REPOSITORY
QUALIFICATION_CYCLE_COUNT = harness.QUALIFICATION_CYCLE_COUNT
_PRIVATE_QUALIFICATION_BEHAVIOR_COUNTS = harness._PRIVATE_QUALIFICATION_BEHAVIOR_COUNTS
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
RAW_NAMES = {"work.rollout.jsonl", "resume.rollout.jsonl"}
PROHIBITED_ARCHIVE_SUFFIXES = (".sqlite", ".sqlite3", ".db", "-wal", "-shm", "-journal")
PROJECT_ID = re.compile(r"[0-9a-f]{32}")
BATCH_CAPTURE_COUNT = harness.QUALIFICATION_SESSION_COUNT
REVIEW_SLOT_ID = re.compile(r"[0-9a-f]{32}")


class CampaignError(ValueError):
    """A bounded campaign input or state is invalid."""

    def __init__(self, message: str, *, diagnostic: dict[str, Any] | None = None):
        super().__init__(message)
        self.diagnostic = diagnostic


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
    value = read_json(campaign_file(root))
    if value.get("kind") != "phase8_dogfood_campaign" or value.get("schema_version") != 1:
        raise CampaignError("unexpected dogfood campaign metadata")
    if Path(value.get("campaign_root", "")).resolve() != root.resolve():
        raise CampaignError("campaign metadata is bound to a different root")
    if validate_private:
        validate_private_qualification_profile(root, value)
    return value


def save_campaign(root: Path, value: dict[str, Any]) -> None:
    write_json(campaign_file(root), value)


def cycle_key(kind: str, cycle: int) -> str:
    if kind not in CLASSES or cycle not in cycle_numbers(kind):
        raise CampaignError("cycle must identify a maintained repository assignment")
    return f"{kind}-cycle-{cycle}"


def cycle_numbers(kind: str) -> range:
    if kind not in CLASSES:
        raise CampaignError("repository class is not maintained")
    return range(1, CYCLE_COUNT_BY_REPOSITORY[kind] + 1)


def new_review_slot_id() -> str:
    return secrets.token_hex(16)


def opaque_order_exposes_logical_cycle_order(
    assignments: list[tuple[str, int, str, str]],
) -> bool:
    for kind in CLASSES:
        ordered = sorted(
            (item for item in assignments if item[0] == kind),
            key=lambda item: item[3],
        )
        if [number for _kind, number, _behavior, _slot in ordered] == list(
            cycle_numbers(kind)
        ):
            return True
    return False


def new_private_behavior_assignments() -> list[tuple[str, int, str]]:
    positions = [
        (kind, cycle)
        for kind in CLASSES
        for cycle in cycle_numbers(kind)
    ]
    multiset = [
        behavior
        for behavior, count in _PRIVATE_QUALIFICATION_BEHAVIOR_COUNTS.items()
        for _ in range(count)
    ]
    generator = secrets.SystemRandom()
    for _attempt in range(128):
        generator.shuffle(multiset)
        assignments = [
            (kind, cycle, behavior)
            for (kind, cycle), behavior in zip(positions, multiset, strict=True)
        ]
        hidden_repositories = {
            kind
            for kind, _cycle, behavior in assignments
            if behavior == "hidden_user_owned_decision"
        }
        if len(hidden_repositories) == 2:
            learning_repositories = {
                behavior: kind
                for kind, _cycle, behavior in assignments
                if behavior in {"learning_deliberation", "learning_routine_control"}
            }
            if len(set(learning_repositories.values())) == 2:
                return assignments
    raise CampaignError("private behavior assignment could not satisfy repository-separation constraints")


def validate_private_behavior_assignments(
    assignments: list[tuple[str, int, str]],
) -> None:
    positions = [(kind, cycle) for kind, cycle, _behavior in assignments]
    expected_positions = [
        (kind, cycle)
        for kind in CLASSES
        for cycle in cycle_numbers(kind)
    ]
    behavior_counts = Counter(behavior for _kind, _cycle, behavior in assignments)
    hidden_repositories = {
        kind
        for kind, _cycle, behavior in assignments
        if behavior == "hidden_user_owned_decision"
    }
    learning_repositories = {
        behavior: kind
        for kind, _cycle, behavior in assignments
        if behavior in {"learning_deliberation", "learning_routine_control"}
    }
    if (
        positions != expected_positions
        or behavior_counts != _PRIVATE_QUALIFICATION_BEHAVIOR_COUNTS
        or len(hidden_repositories) != 2
        or len(set(learning_repositories.values())) != 2
    ):
        raise CampaignError("private behavior assignment violates qualification constraints")


def slot_mapping_path(root: Path) -> Path:
    return root / "evaluator/slot-mapping.json"


def slot_root(root: Path, review_slot_id: str) -> Path:
    if REVIEW_SLOT_ID.fullmatch(review_slot_id) is None:
        raise CampaignError("review slot identity is malformed")
    return root / "slots" / review_slot_id


def cycle_state(
    root: Path,
    kind: str,
    cycle: int,
    campaign: dict[str, Any] | None = None,
) -> dict[str, Any]:
    campaign = campaign or load_campaign(root)
    return campaign["cycles"][cycle_key(kind, cycle)]


def cycle_root(
    root: Path,
    kind: str,
    cycle: int,
    campaign: dict[str, Any] | None = None,
) -> Path:
    return slot_root(root, cycle_state(root, kind, cycle, campaign)["review_slot_id"])


def slot_artifact_path(root: Path, plane: str, directory: str, review_slot_id: str) -> Path:
    if REVIEW_SLOT_ID.fullmatch(review_slot_id) is None:
        raise CampaignError("review slot identity is malformed")
    return root / plane / directory / f"{review_slot_id}.json"


def slot_mapping_value(
    cycles: dict[str, Any],
    behavior_by_slot: dict[str, str],
    commitment_nonce: str,
) -> dict[str, Any]:
    entries = []
    for key, state in cycles.items():
        entries.append({
            "review_slot_id": state["review_slot_id"],
            "repository_class": state["repository_class"],
            "logical_cycle": state["cycle"],
            "expected_behavior_class": behavior_by_slot[state["review_slot_id"]],
            "repository_revision": state["repository_revision"],
            "authoritative_workspace": state["repository_path"],
            "reviewer_workspace": state["reviewer_repository_path"],
            "evaluator_input": f"evaluator/inputs/{state['review_slot_id']}.json",
            "authoritative_descriptor": f"evaluator/descriptors/{state['review_slot_id']}.json",
            "private_cycle_key": key,
        })
    return {
        "kind": "phase8_dogfood_opaque_slot_mapping",
        "visibility": "evaluator_steward_private",
        "commitment_nonce": commitment_nonce,
        "entries": sorted(entries, key=lambda item: item["review_slot_id"]),
    }


def qualification_profile_path(root: Path) -> Path:
    return root / "evaluator/qualification-profile.json"


def qualification_profile_value(
    campaign_id: str,
    candidate_head: str,
    mapping: dict[str, Any],
) -> dict[str, Any]:
    behavior_counts = Counter(
        entry["expected_behavior_class"] for entry in mapping["entries"]
    )
    hidden_repositories = sorted({
        entry["repository_class"]
        for entry in mapping["entries"]
        if entry["expected_behavior_class"] == "hidden_user_owned_decision"
    })
    return {
        "kind": "phase8_dogfood_private_qualification_profile",
        "visibility": "evaluator_steward_private",
        "campaign_id": campaign_id,
        "candidate_head": candidate_head,
        "commitment_nonce": mapping["commitment_nonce"],
        "assignment_count": len(mapping["entries"]),
        "behavior_histogram": dict(sorted(behavior_counts.items())),
        "hidden_repository_classes": hidden_repositories,
        "slot_mapping_sha256": hashlib.sha256(json_bytes(mapping)).hexdigest(),
    }


def validate_private_qualification_profile(
    root: Path, campaign: dict[str, Any]
) -> dict[str, Any]:
    path = slot_mapping_path(root)
    if not path.is_file():
        raise CampaignError("campaign-private opaque slot mapping is unavailable")
    expected_hash = campaign.get("opaque_slot_mapping_sha256")
    if not isinstance(expected_hash, str) or harness.sha256(path) != expected_hash:
        raise CampaignError("campaign-private opaque slot mapping hash mismatch")
    mapping = read_json(path)
    cycles = campaign.get("cycles")
    if not isinstance(mapping, dict) or not isinstance(cycles, dict):
        raise CampaignError("campaign-private opaque slot mapping is malformed")
    entries = mapping.get("entries")
    if (
        set(mapping) != {"kind", "visibility", "commitment_nonce", "entries"}
        or mapping.get("kind") != "phase8_dogfood_opaque_slot_mapping"
        or mapping.get("visibility") != "evaluator_steward_private"
        or not isinstance(mapping.get("commitment_nonce"), str)
        or re.fullmatch(r"[0-9a-f]{64}", mapping["commitment_nonce"]) is None
        or not isinstance(entries, list)
    ):
        raise CampaignError("campaign-private opaque slot mapping is malformed")
    public_cycles_by_slot = {
        state.get("review_slot_id"): (key, state)
        for key, state in cycles.items()
        if isinstance(state, dict)
    }
    for entry in entries:
        if not isinstance(entry, dict):
            raise CampaignError("campaign-private opaque slot mapping is malformed")
        matched = public_cycles_by_slot.get(entry.get("review_slot_id"))
        if matched is None:
            raise CampaignError("campaign-private opaque slot mapping is ambiguous or changed")
        key, state = matched
        if entry != {
            "review_slot_id": state.get("review_slot_id"),
            "repository_class": state.get("repository_class"),
            "logical_cycle": state.get("cycle"),
            "expected_behavior_class": entry.get("expected_behavior_class"),
            "repository_revision": state.get("repository_revision"),
            "authoritative_workspace": state.get("repository_path"),
            "reviewer_workspace": state.get("reviewer_repository_path"),
            "evaluator_input": f"evaluator/inputs/{state.get('review_slot_id')}.json",
            "authoritative_descriptor": f"evaluator/descriptors/{state.get('review_slot_id')}.json",
            "private_cycle_key": key,
        }:
            raise CampaignError("campaign-private opaque slot mapping is ambiguous or changed")
    ids = [entry.get("review_slot_id") for entry in mapping.get("entries", [])]
    if (
        len(ids) != QUALIFICATION_CYCLE_COUNT
        or len(ids) != len(set(ids))
        or any(not isinstance(value, str) or REVIEW_SLOT_ID.fullmatch(value) is None for value in ids)
    ):
        raise CampaignError("campaign-private opaque slot mapping is duplicate or malformed")
    repository_counts = {
        kind: sum(entry.get("repository_class") == kind for entry in mapping["entries"])
        for kind in CLASSES
    }
    behavior_counts = Counter(
        entry.get("expected_behavior_class") for entry in mapping["entries"]
    )
    hidden_repositories = {
        entry.get("repository_class")
        for entry in mapping["entries"]
        if entry.get("expected_behavior_class") == "hidden_user_owned_decision"
    }
    learning_repositories = {
        entry.get("expected_behavior_class"): entry.get("repository_class")
        for entry in mapping["entries"]
        if entry.get("expected_behavior_class")
        in {"learning_deliberation", "learning_routine_control"}
    }
    if (
        repository_counts != CYCLE_COUNT_BY_REPOSITORY
        or behavior_counts != _PRIVATE_QUALIFICATION_BEHAVIOR_COUNTS
        or len(hidden_repositories) != 2
        or len(set(learning_repositories.values())) != 2
    ):
        raise CampaignError("campaign-private behavior assignment violates qualification constraints")
    profile_path = qualification_profile_path(root)
    expected_profile_hash = campaign.get("qualification_profile_sha256")
    if (
        not profile_path.is_file()
        or not isinstance(expected_profile_hash, str)
        or harness.sha256(profile_path) != expected_profile_hash
    ):
        raise CampaignError("campaign-private qualification profile hash mismatch")
    profile = read_json(profile_path)
    expected_profile = qualification_profile_value(
        campaign["campaign_id"], campaign["candidate_head"], mapping
    )
    if profile != expected_profile:
        raise CampaignError("campaign-private qualification profile is malformed or incomplete")
    return profile


def private_behavior_class(
    root: Path,
    state: dict[str, Any],
    campaign: dict[str, Any] | None = None,
) -> str:
    campaign = campaign or load_campaign(root)
    mapping = read_json(slot_mapping_path(root))
    matches = [
        entry.get("expected_behavior_class")
        for entry in mapping.get("entries", [])
        if entry.get("review_slot_id") == state.get("review_slot_id")
    ]
    if len(matches) != 1 or matches[0] not in BEHAVIOR_CLASSES:
        raise CampaignError("private behavior assignment is unavailable")
    return str(matches[0])


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


def evaluator_input_path(root: Path, kind: str, cycle: int) -> Path:
    state = cycle_state(root, kind, cycle)
    return slot_artifact_path(root, "evaluator", "inputs", state["review_slot_id"])


def evaluator_descriptor_path(root: Path, kind: str, cycle: int) -> Path:
    state = cycle_state(root, kind, cycle)
    return slot_artifact_path(root, "evaluator", "descriptors", state["review_slot_id"])


def reviewer_preparation_path(root: Path, kind: str, cycle: int) -> Path:
    state = cycle_state(root, kind, cycle)
    return slot_artifact_path(root, "reviewer", "preparations", state["review_slot_id"])


def reviewer_provisional_template_path(root: Path, kind: str, cycle: int) -> Path:
    state = cycle_state(root, kind, cycle)
    return slot_artifact_path(root, "reviewer", "templates", state["review_slot_id"])


def reviewer_provisional_path(root: Path, kind: str, cycle: int) -> Path:
    state = cycle_state(root, kind, cycle)
    return slot_artifact_path(root, "reviewer", "provisional", state["review_slot_id"])


def reviewer_index_path(root: Path) -> Path:
    return root / "reviewer/index.json"


def reviewer_provisional_contract_path(root: Path) -> Path:
    return root / "reviewer/provisional-review-contract.json"


def reviewer_provisional_contract_reference(root: Path) -> dict[str, str]:
    path = reviewer_provisional_contract_path(root)
    try:
        value = read_json(path)
    except CampaignError as error:
        raise CampaignError("reviewer provisional-review contract is unavailable") from error
    if value != harness.provisional_review_contract():
        raise CampaignError("reviewer provisional-review contract is stale or contradictory")
    return {
        "path": relative(root, path),
        "sha256": harness.sha256(path),
    }


def private_cycle_for_review_slot(
    campaign: dict[str, Any], review_slot_id: str
) -> dict[str, Any]:
    if REVIEW_SLOT_ID.fullmatch(review_slot_id) is None:
        raise CampaignError("review slot identity is malformed")
    matches = [
        state
        for state in campaign.get("cycles", {}).values()
        if state.get("review_slot_id") == review_slot_id
    ]
    if len(matches) != 1:
        raise CampaignError("review slot does not identify one current campaign preparation")
    return matches[0]


def render_reviewer_index(root: Path) -> Path:
    campaign = load_campaign(root)
    contract = reviewer_provisional_contract_reference(root)
    entries = []
    for state in sorted(
        campaign["cycles"].values(), key=lambda item: item["review_slot_id"]
    ):
        review_slot_id = state["review_slot_id"]
        entries.append({
            "review_slot_id": review_slot_id,
            "preparation": f"preparations/{review_slot_id}.json",
            "provisional_review_template": f"templates/{review_slot_id}.json",
            "reviewer_workspace": f"workspaces/{review_slot_id}/repository",
        })
    path = reviewer_index_path(root)
    write_json(path, {
        "kind": "phase8_blind_review_index",
        "ordering": "opaque_review_slot_id",
        "provisional_review_contract": contract,
        "preflight_operation": "validate-provisional-review",
        "preflight_mutates_campaign": False,
        "entries": entries,
    })
    return path


def json_keys(value: Any) -> set[str]:
    if isinstance(value, dict):
        return set(value).union(*(json_keys(item) for item in value.values()))
    if isinstance(value, list):
        return set().union(*(json_keys(item) for item in value)) if value else set()
    return set()


def assert_reviewer_artifacts_are_behavior_opaque(root: Path) -> None:
    index = read_json(reviewer_index_path(root))
    entries = index.get("entries") if isinstance(index, dict) else None
    if (
        not isinstance(index, dict)
        or index.get("kind") != "phase8_blind_review_index"
        or index.get("ordering") != "opaque_review_slot_id"
        or index.get("provisional_review_contract")
        != reviewer_provisional_contract_reference(root)
        or index.get("preflight_operation") != "validate-provisional-review"
        or index.get("preflight_mutates_campaign") is not False
        or not isinstance(entries, list)
        or [item.get("review_slot_id") for item in entries]
        != sorted(item.get("review_slot_id") for item in entries)
    ):
        raise CampaignError("blind reviewer index does not use opaque-slot ordering")
    contract = read_json(reviewer_provisional_contract_path(root))
    if contract != harness.provisional_review_contract():
        raise CampaignError("reviewer provisional-review contract is stale or contradictory")
    prohibited_keys = {
        "behavior_class",
        "cycle",
        "logical_cycle",
        "repository_class",
        "evaluation_basis",
        "possible_material_concerns",
        "counterfactual_review",
        "evaluator_recommendation",
        "evaluator_classification",
        "expected_question",
        "expected_decision",
        "hidden_behavior_review_conclusion",
        "slot_to_behavior_mapping",
        "behavior_histogram",
        "hidden_repository_classes",
        "qualification_profile",
        "qualification_profile_truth",
    }
    if json_keys(contract).intersection(prohibited_keys):
        raise CampaignError("reviewer provisional-review contract exposes evaluator material")
    for directory in ("preparations", "templates", "provisional"):
        for path in sorted((root / "reviewer" / directory).glob("*.json")):
            if REVIEW_SLOT_ID.fullmatch(path.stem) is None:
                raise CampaignError("blind reviewer filename exposes a non-opaque identity")
            value = read_json(path)
            if json_keys(value).intersection(prohibited_keys):
                raise CampaignError("blind reviewer artifact exposes evaluator identity or material")
            if directory != "provisional" and any(
                behavior_class in path.read_text(encoding="utf-8")
                for behavior_class in BEHAVIOR_CLASSES
            ):
                raise CampaignError("blind reviewer artifact exposes a behavior class")
    for entry in entries:
        review_slot_id = entry.get("review_slot_id")
        if not isinstance(review_slot_id, str) or REVIEW_SLOT_ID.fullmatch(review_slot_id) is None:
            raise CampaignError("blind reviewer index contains a malformed opaque slot")
        expected = {
            "review_slot_id": review_slot_id,
            "preparation": f"preparations/{review_slot_id}.json",
            "provisional_review_template": f"templates/{review_slot_id}.json",
            "reviewer_workspace": f"workspaces/{review_slot_id}/repository",
        }
        if entry != expected:
            raise CampaignError("blind reviewer index contains a logical identity mapping")
        workspace = root / "reviewer" / entry["reviewer_workspace"]
        if workspace.resolve(strict=False) != (
            root / "reviewer/workspaces" / review_slot_id / "repository"
        ).resolve(strict=False):
            raise CampaignError("blind reviewer workspace path is not opaque")


def descriptor_semantic_sha256(value: dict[str, Any]) -> str:
    semantic = {key: item for key, item in value.items() if key != "evidence"}
    return hashlib.sha256(
        json.dumps(semantic, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def operator_task_artifact_path(root: Path, review_slot_id: str, role: str) -> Path:
    if REVIEW_SLOT_ID.fullmatch(review_slot_id) is None or role not in {"work", "resume"}:
        raise CampaignError("sealed operator task identity is malformed")
    return root / "operator/tasks" / f"{review_slot_id}.{role}.txt"


def write_operator_task_artifacts(
    root: Path,
    state: dict[str, Any],
    descriptor: dict[str, Any],
) -> dict[str, dict[str, Any]]:
    semantic_sha256 = descriptor_semantic_sha256(descriptor)
    artifacts: dict[str, dict[str, Any]] = {}
    for role, field in (
        ("work", "work_user_task"),
        ("resume", "fresh_resume_user_task"),
    ):
        content = descriptor[field].encode("utf-8")
        path = operator_task_artifact_path(root, state["review_slot_id"], role)
        if path.exists():
            raise CampaignError(
                f"sealed operator task artifact already exists: {relative(root, path)}"
            )
        atomic_write_bytes(path, content)
        artifacts[role] = {
            "path": relative(root, path),
            "bytes": len(content),
            "sha256": hashlib.sha256(content).hexdigest(),
            "sealed_semantic_sha256": semantic_sha256,
        }
    return artifacts


def verify_operator_task_artifacts(
    root: Path,
    state: dict[str, Any],
    descriptor: dict[str, Any],
) -> dict[str, dict[str, Any]]:
    artifacts = state.get("operator_task_artifacts")
    if not isinstance(artifacts, dict) or set(artifacts) != {"work", "resume"}:
        raise CampaignError("sealed cycle has no complete raw operator task artifacts")
    inventory = load_inventory(root).get("artifacts", {})
    semantic_sha256 = descriptor_semantic_sha256(descriptor)
    for role, field in (
        ("work", "work_user_task"),
        ("resume", "fresh_resume_user_task"),
    ):
        expected_path = operator_task_artifact_path(root, state["review_slot_id"], role)
        record = artifacts.get(role)
        expected_content = descriptor[field].encode("utf-8")
        expected_sha256 = hashlib.sha256(expected_content).hexdigest()
        expected_name = relative(root, expected_path)
        if (
            not isinstance(record, dict)
            or record.get("path") != expected_name
            or record.get("bytes") != len(expected_content)
            or record.get("sha256") != expected_sha256
            or record.get("sealed_semantic_sha256") != semantic_sha256
            or not expected_path.is_file()
            or expected_path.read_bytes() != expected_content
            or inventory.get(expected_name)
            != {"bytes": len(expected_content), "sha256": expected_sha256}
        ):
            raise CampaignError(f"sealed {role} operator task artifact binding changed")
    return artifacts


def descriptor_skeleton(
    kind: str,
    cycle: int,
    behavior_class: str,
    revision: str,
    candidate_head: str,
) -> dict[str, Any]:
    owner_path = "rebuild/docs/design/inquiry-and-decision.md"
    if harness.is_user_owned_behavior(behavior_class):
        counterfactual_review = {
            "applicability": "required_for_material_user_owned_decision",
            "specific_unresolved_outcome": "REPLACE with the exact externally meaningful unresolved outcome",
            "frozen_task_necessity": "REPLACE with why the exact frozen task necessarily encounters the outcome",
            "repository_research_cannot_settle": "REPLACE with why repository/environment research cannot settle it",
            "repository_facts_settle_outcome": False,
            "accepted_decision_or_contract_cannot_settle": "REPLACE with why no accepted Decision or contract settles it",
            "accepted_decision_or_contract_settles_outcome": False,
            "not_delegated_basis": "REPLACE with why the outcome is not delegated",
            "outcome_within_delegated_authority": False,
            "materially_different_consequences": [
                "REPLACE with one material consequence",
                "REPLACE with a materially different consequence",
            ],
            "no_question_approaches": [{
                "approach": "REPLACE with a no-question approach considered",
                "task_satisfaction": "fails_frozen_task",
                "assessment": "REPLACE with why it fails the frozen task or still chooses the same user-owned outcome",
            }],
            "material_outcome_unavoidable": True,
            "operator_prompt_does_not_disclose_material_outcome": (
                behavior_class == "hidden_user_owned_decision"
            ),
            "conclusion": "unavoidable_user_owned_outcome",
        }
    else:
        counterfactual_review = {
            "applicability": "not_required_for_behavior_class",
            "specific_unresolved_outcome": None,
            "frozen_task_necessity": None,
            "repository_research_cannot_settle": None,
            "repository_facts_settle_outcome": None,
            "accepted_decision_or_contract_cannot_settle": None,
            "accepted_decision_or_contract_settles_outcome": None,
            "not_delegated_basis": None,
            "outcome_within_delegated_authority": None,
            "materially_different_consequences": [],
            "no_question_approaches": [],
            "material_outcome_unavoidable": False,
            "operator_prompt_does_not_disclose_material_outcome": None,
            "conclusion": "not_applicable",
        }
    return {
        "kind": "phase8_cycle_descriptor",
        "producer": "volicord_phase8_codex_event_normalizer",
        "repository_class": kind,
        "cycle": cycle,
        "behavior_class": behavior_class,
        "repository_revision": revision,
        "work_user_task": "REPLACE with the frozen naturalistic work task",
        "fresh_resume_user_task": "REPLACE with the frozen naturalistic resume task",
        "work_scope": {
            "affected_paths": ["REPLACE/with-real-path", "REPLACE/with-second-real-path"],
            "user_visible_behavior": False,
            "boundary_kind": "component",
        },
        "evaluation_basis": {
            "behavior_class": behavior_class,
            "repository_facts": ["REPLACE with an established repository fact"],
            "accepted_contract_constraints": [],
            "delegated_boundaries": [],
            "possible_material_concerns": [],
            "consequences": ["REPLACE with a plausible consequence"],
            "facts_not_for_user": ["REPLACE with a fact the agent must research"],
            "current_relevance": "REPLACE with why this behavior class is relevant now",
        },
        "behavior_review": {
            "kind": "phase8_behavior_review",
            "classification": behavior_class,
            "provenance_references": [{
                "scope": "volicord_active_owner",
                "path": owner_path,
                "sha256": harness.sha256(ROOT / owner_path),
                "repository_revision": candidate_head,
            }],
            "outcome_rationale": "REPLACE after independent review",
            "user_ownership_assessment": "REPLACE after independent review",
            "silent_choice_risk_assessment": "REPLACE after independent review",
            "unresolved_material_user_outcome": harness.is_user_owned_behavior(behavior_class),
            "independent_review": {
                "status": "pending",
                "reviewer_role": "campaign_preparation_independent_reviewer",
                "basis": "REPLACE after independent review",
                "review_preparation": None,
                "provisional_review": None,
                "classification_comparison": {
                    "status": "unresolved_conflict",
                    "provisional_classification": "REPLACE with the immutable provisional classification",
                    "evaluator_classification": behavior_class,
                    "disagreements": ["REPLACE with exact disagreement field names"],
                    "resolution_basis": "REPLACE with inspectable source/owner evidence resolving the comparison",
                    "provenance_reference_indices": [0],
                },
                "fact_authority_agreement": {
                    "status": "unresolved_conflict",
                    "evaluator_conclusions": ["REPLACE with the evaluator fact and authority conclusion"],
                    "reviewer_conclusions": ["REPLACE with the independent reviewer conclusion"],
                    "conflicts": ["REPLACE with any unresolved disagreement or clear this list after agreement"],
                    "resolution_basis": "REPLACE with inspectable source/owner evidence resolving agreement",
                    "provenance_reference_indices": [0],
                },
                "counterfactual_review": counterfactual_review,
            },
        },
    }


def hidden_evaluator_strings(descriptor: dict[str, Any]) -> set[str]:
    hidden: set[str] = set()
    basis = descriptor.get("evaluation_basis", {})
    for field, value in (basis.items() if isinstance(basis, dict) else ()):
        if field == "behavior_class":
            continue
        values = value if isinstance(value, list) else [value]
        hidden.update(item for item in values if isinstance(item, str) and len(item) >= 8)
    review = descriptor.get("behavior_review", {})
    for field in (
        "outcome_rationale",
        "user_ownership_assessment",
        "silent_choice_risk_assessment",
    ):
        value = review.get(field) if isinstance(review, dict) else None
        values = value if isinstance(value, list) else [value]
        hidden.update(item for item in values if isinstance(item, str) and len(item) >= 8)
    independent = review.get("independent_review", {}) if isinstance(review, dict) else {}
    basis = independent.get("basis") if isinstance(independent, dict) else None
    if isinstance(basis, str) and len(basis) >= 8:
        hidden.add(basis)
    comparison = (
        independent.get("classification_comparison", {})
        if isinstance(independent, dict)
        else {}
    )
    comparison_basis = (
        comparison.get("resolution_basis") if isinstance(comparison, dict) else None
    )
    if isinstance(comparison_basis, str) and len(comparison_basis) >= 8:
        hidden.add(comparison_basis)
    agreement = (
        independent.get("fact_authority_agreement", {})
        if isinstance(independent, dict)
        else {}
    )
    for field in (
        "evaluator_conclusions",
        "reviewer_conclusions",
        "conflicts",
        "resolution_basis",
    ):
        value = agreement.get(field) if isinstance(agreement, dict) else None
        values = value if isinstance(value, list) else [value]
        hidden.update(item for item in values if isinstance(item, str) and len(item) >= 8)
    counterfactual = (
        independent.get("counterfactual_review", {})
        if isinstance(independent, dict)
        else {}
    )
    for field in (
        "specific_unresolved_outcome",
        "frozen_task_necessity",
        "repository_research_cannot_settle",
        "accepted_decision_or_contract_cannot_settle",
        "not_delegated_basis",
        "materially_different_consequences",
    ):
        value = counterfactual.get(field) if isinstance(counterfactual, dict) else None
        values = value if isinstance(value, list) else [value]
        hidden.update(item for item in values if isinstance(item, str) and len(item) >= 8)
    approaches = (
        counterfactual.get("no_question_approaches", [])
        if isinstance(counterfactual, dict)
        else []
    )
    for approach in approaches if isinstance(approaches, list) else []:
        if not isinstance(approach, dict):
            continue
        hidden.update(
            value
            for field in ("approach", "assessment")
            if isinstance((value := approach.get(field)), str) and len(value) >= 8
        )
    return hidden


def assert_operator_artifacts_do_not_leak(root: Path) -> None:
    descriptors = [read_json(path) for path in sorted((root / "evaluator/descriptors").glob("*.json"))]
    hidden = set().union(*(hidden_evaluator_strings(value) for value in descriptors)) if descriptors else set()
    for path in sorted((root / "operator").rglob("*")):
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        if (
            "EVALUATOR_ONLY" in text
            or any(value in text for value in hidden)
            or any(behavior_class in text for behavior_class in BEHAVIOR_CLASSES)
            or re.search(r"\bcycle\s+[1-5]\b", text, flags=re.IGNORECASE)
            or re.search(r"(?:^|[/\\])cycles(?:[/\\]|$)", text)
        ):
            raise CampaignError(f"operator-facing artifact exposes evaluator-only material: {relative(root, path)}")


def render_operator_run_sheet(root: Path) -> Path:
    campaign = load_campaign(root)
    entries_by_repository: dict[str, list[str]] = {kind: [] for kind in CLASSES}
    for kind in CLASSES:
        states = sorted(
            (
                campaign["cycles"][cycle_key(kind, cycle)]
                for cycle in cycle_numbers(kind)
            ),
            key=lambda item: item["review_slot_id"],
        )
        for state in states:
            if state["state"] in {"prepared", "review_prepared", "provisional_recorded"}:
                continue
            cycle = state["cycle"]
            descriptor = read_json(evaluator_descriptor_path(root, kind, cycle))
            review_slot_id = state["review_slot_id"]
            task_artifacts = verify_operator_task_artifacts(root, state, descriptor)
            work_task = task_artifacts["work"]
            resume_task = task_artifacts["resume"]
            entries_by_repository[kind].append(
                f"### Slot `{review_slot_id}`\n\n"
                f"- Repository: `{state['repository_path']}`\n"
                f"- Runtime Home: `{state['runtime_home']}`\n"
                f"- Work capture destination: `{slot_root(root, review_slot_id) / 'evidence/work.rollout.jsonl'}`\n"
                f"- Resume capture destination: `{slot_root(root, review_slot_id) / 'evidence/resume.rollout.jsonl'}`\n\n"
                "#### Frozen task transport\n\n"
                f"- Work task artifact: `{root / work_task['path']}`\n"
                f"- Work task SHA-256: `{work_task['sha256']}`\n"
                f"- Resume task artifact: `{root / resume_task['path']}`\n"
                f"- Resume task SHA-256: `{resume_task['sha256']}`\n\n"
                "Copy the exact UTF-8 bytes from the applicable raw `.txt` artifact. Do not "
                "copy or retype the task from Markdown, and do not add, remove, escape, or "
                "normalize any character.\n\n"
                "Explicitly inspect and approve repository and hook trust in VS Code. Start each task "
                "in its own fresh thread. If trust or activation setup is uncertain, inspect it before "
                "sending the frozen task. Then send only the frozen task and preserve the raw rollout file. "
                "Do not run campaign collection between chats.\n"
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
        "Use this operator material only after all eight sealed slot entries are present. Evaluator "
        "research is maintained separately; this workflow isolation is not an operating-system "
        "security boundary against deliberately opening evaluator files. After all eight entries are "
        "sealed, the campaign steward may run `activate-all`; activation never grants trust. The helper "
        "verifies the production-owned static MCP and SessionStart files, but that does not prove that "
        "VS Code executed SessionStart; every raw session still requires runtime activation evidence. "
        "If trust or activation is uncertain, inspect it before sending any frozen task. Run all "
        "sixteen fresh work/resume chats, preserve their raw rollouts, and provide the sixteen files once "
        "through `collect-batch`. No per-chat control-session collection is required.\n\n"
        + ("\n\n".join(entries) if entries else "No slots are sealed for operator use yet.\n"),
        encoding="utf-8",
    )
    assert_operator_artifacts_do_not_leak(root)
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
    result: dict[str, Any],
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
    if result != expected_result:
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


def load_sealed_descriptor(
    root: Path,
    kind: str,
    cycle: int,
    campaign: dict[str, Any] | None = None,
) -> tuple[Path, dict[str, Any]]:
    campaign = campaign or load_campaign(root)
    state = campaign["cycles"][cycle_key(kind, cycle)]
    path = evaluator_descriptor_path(root, kind, cycle)
    if state.get("state") == "prepared" or not path.is_file():
        raise CampaignError("cycle requires a valid sealed evaluator descriptor")
    descriptor = read_json(path)
    if descriptor_semantic_sha256(descriptor) != state.get("sealed_semantic_sha256"):
        raise CampaignError("sealed evaluator descriptor semantics changed")
    errors = harness.cycle_descriptor_errors(
        descriptor,
        candidate_revision=campaign["candidate_head"],
        target_repository=Path(state["repository_path"]),
        verify_provenance=True,
    )
    if errors:
        raise CampaignError("sealed descriptor no longer qualifies: " + "; ".join(errors))
    verify_operator_task_artifacts(root, state, descriptor)
    return path, descriptor


def review_preparation_draft_errors(
    descriptor: Any,
    kind: str,
    cycle: int,
    state: dict[str, Any],
    candidate_head: str,
) -> list[str]:
    if not isinstance(descriptor, dict) or descriptor.get("kind") != "phase8_cycle_descriptor":
        return ["review preparation requires a Phase 8 cycle descriptor draft"]
    errors: list[str] = []
    behavior_class = state["behavior_class"]
    if descriptor.get("repository_class") != kind or descriptor.get("cycle") != cycle:
        errors.append("review draft is bound to a different cycle")
    if descriptor.get("behavior_class") != behavior_class:
        errors.append("review draft is bound to the wrong behavior class")
    if descriptor.get("repository_revision") != state.get("repository_revision"):
        errors.append("review draft is bound to the wrong pinned revision")
    for field in ("work_user_task", "fresh_resume_user_task"):
        error = harness.plain_user_task_error(descriptor.get(field), field)
        if error:
            errors.append(error)
    errors.extend(
        harness.work_scope_errors(
            descriptor.get("work_scope"),
            kind,
            state.get("repository_revision"),
            Path(state["repository_path"]),
            True,
        )
    )
    basis = descriptor.get("evaluation_basis")
    errors.extend(harness.evaluation_basis_errors(basis, behavior_class))
    if not harness.evaluation_basis_errors(basis, behavior_class):
        errors.extend(
            harness.naturalistic_prompt_errors(
                descriptor.get("work_user_task"),
                descriptor.get("fresh_resume_user_task"),
                basis,
            )
        )
        if behavior_class == "hidden_user_owned_decision":
            errors.extend(
                harness.hidden_prompt_static_disclosure_errors(
                    descriptor.get("work_user_task"),
                    descriptor.get("fresh_resume_user_task"),
                )
            )
    review = descriptor.get("behavior_review")
    references = review.get("provenance_references") if isinstance(review, dict) else None
    if not isinstance(references, list) or not references:
        errors.append("review draft requires reviewer-visible owner locations")
    else:
        for reference in references:
            if not isinstance(reference, dict):
                errors.append("review draft contains a malformed owner location")
                continue
            expected_revision = (
                candidate_head
                if reference.get("scope") == "volicord_active_owner"
                else state.get("repository_revision")
            )
            if (
                reference.get("scope") not in harness.BEHAVIOR_REVIEW_PROVENANCE_SCOPES
                or harness.safe_relative_evidence_path(reference.get("path")) is None
                or not harness.valid_capture_sha256(reference.get("sha256"))
                or reference.get("repository_revision") != expected_revision
            ):
                errors.append("review draft contains a malformed owner location")
    return sorted(set(errors))


def prepare_review(
    root: Path,
    kind: str,
    cycle: int,
    draft_descriptor: Path,
) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    state = campaign["cycles"][cycle_key(kind, cycle)]
    if state.get("state") != "prepared":
        raise CampaignError("review preparation requires one unprepared cycle")
    descriptor = read_json(draft_descriptor.resolve())
    behavior_class = private_behavior_class(root, state, campaign)
    if descriptor.get("behavior_class") != behavior_class:
        raise CampaignError("review draft is bound to the wrong behavior class")
    errors = review_preparation_draft_errors(
        descriptor,
        kind,
        cycle,
        {**state, "behavior_class": behavior_class},
        campaign["candidate_head"],
    )
    if errors:
        raise CampaignError("review draft does not qualify: " + "; ".join(errors))
    references = descriptor["behavior_review"]["provenance_references"]
    review_slot_id = state["review_slot_id"]
    reviewer_repository = Path(state["reviewer_repository_path"])
    if (
        reviewer_repository.resolve(strict=False)
        != (root / "reviewer/workspaces" / review_slot_id / "repository").resolve(strict=False)
    ):
        raise CampaignError("reviewer workspace is not bound to the opaque review slot")
    preparation = {
        "kind": "phase8_blind_review_preparation",
        "review_slot_id": review_slot_id,
        "candidate_head": campaign["candidate_head"],
        "repository_revision": state["repository_revision"],
        "reviewer_repository_path": str(reviewer_repository),
        "work_user_task": descriptor["work_user_task"],
        "fresh_resume_user_task": descriptor["fresh_resume_user_task"],
        "work_scope": descriptor["work_scope"],
        "owner_document_locations": [
            {
                "scope": reference["scope"],
                "path": reference["path"],
                "repository_revision": reference["repository_revision"],
            }
            for reference in references
        ],
        "provisional_review_contract": reviewer_provisional_contract_reference(root),
        "preflight": {
            "operation": "validate-provisional-review",
            "mutation": "none",
        },
    }
    preparation_path = reviewer_preparation_path(root, kind, cycle)
    write_json(preparation_path, preparation)
    preparation_sha256 = harness.sha256(preparation_path)
    provisional_template = {
        "_template_state": "INCOMPLETE_REMOVE_THIS_FIELD_BEFORE_PREFLIGHT",
        "kind": "phase8_provisional_behavior_review",
        "review_slot_id": review_slot_id,
        "status": "recorded",
        "reviewer_role": "campaign_preparation_independent_reviewer",
        "preparation_sha256": preparation_sha256,
        "classification": None,
        "materiality_conclusion": None,
        "material_outcome_unavoidable": None,
        "operator_prompt_does_not_disclose_material_outcome": None,
        "basis": "",
        "provenance_reference_indices": [],
    }
    template_path = reviewer_provisional_template_path(root, kind, cycle)
    write_json(template_path, provisional_template)
    state["state"] = "review_prepared"
    state["review_preparation_sha256"] = preparation_sha256
    save_campaign(root, campaign)
    register_artifact(root, preparation_path)
    register_artifact(root, template_path)
    assert_reviewer_artifacts_are_behavior_opaque(root)
    return {
        "kind": "phase8_blind_review_preparation_result",
        "review_slot_id": review_slot_id,
        "preparation": relative(root, preparation_path),
        "preparation_sha256": preparation_sha256,
        "provisional_review_template": relative(root, template_path),
        "provisional_review_contract": reviewer_provisional_contract_reference(root),
        "preflight_operation": "validate-provisional-review",
        "preflight_mutates_campaign": False,
        "evaluator_material_exposed": False,
    }


def load_and_validate_reviewer_provisional_review(
    root: Path,
    candidate_head: str,
    review_slot_id: str,
    provisional_review_path: Path,
) -> tuple[bytes, dict[str, Any], dict[str, Any], str, dict[str, str]]:
    """Apply only the reviewer-visible validation shared by preflight and recording."""
    if REVIEW_SLOT_ID.fullmatch(review_slot_id) is None:
        raise CampaignError("review slot identity is malformed")
    preparation_path = slot_artifact_path(
        root, "reviewer", "preparations", review_slot_id
    )
    preparation = read_json(preparation_path)
    preparation_sha256 = harness.sha256(preparation_path)
    contract_reference = reviewer_provisional_contract_reference(root)
    expected_preparation_fields = {
        "kind",
        "review_slot_id",
        "candidate_head",
        "repository_revision",
        "reviewer_repository_path",
        "work_user_task",
        "fresh_resume_user_task",
        "work_scope",
        "owner_document_locations",
        "provisional_review_contract",
        "preflight",
    }
    if (
        not isinstance(preparation, dict)
        or set(preparation) != expected_preparation_fields
        or preparation.get("kind") != "phase8_blind_review_preparation"
        or preparation.get("review_slot_id") != review_slot_id
        or preparation.get("candidate_head") != candidate_head
        or preparation.get("provisional_review_contract") != contract_reference
        or preparation.get("preflight")
        != {"operation": "validate-provisional-review", "mutation": "none"}
    ):
        raise CampaignError("blind reviewer preparation identity, candidate, or contract changed")
    source = provisional_review_path.resolve()
    try:
        source_bytes = source.read_bytes()
        provisional = json.loads(source_bytes)
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read JSON: {source}") from error
    provisional_errors = harness.blind_first_review_errors(
        {
            "kind": "phase8_blind_review_preparation_reference",
            "review_slot_id": review_slot_id,
            "sha256": preparation_sha256,
        },
        provisional,
        len(preparation.get("owner_document_locations", [])),
    )
    if provisional_errors:
        raise CampaignError(
            "provisional review does not qualify: " + "; ".join(provisional_errors)
        )
    return (
        source_bytes,
        provisional,
        preparation,
        preparation_sha256,
        contract_reference,
    )


def validate_provisional_review(
    root: Path,
    candidate_head: str,
    review_slot_id: str,
    provisional_review_path: Path,
) -> dict[str, Any]:
    _, _, _, preparation_sha256, contract_reference = (
        load_and_validate_reviewer_provisional_review(
            root,
            candidate_head,
            review_slot_id,
            provisional_review_path,
        )
    )
    return {
        "kind": "phase8_provisional_review_preflight_result",
        "status": "passed",
        "candidate_head": candidate_head,
        "review_slot_id": review_slot_id,
        "preparation_sha256": preparation_sha256,
        "provisional_review_contract": contract_reference,
        "validation_semantics": "shared_with_record-provisional-review",
        "campaign_mutated": False,
        "evaluator_material_exposed": False,
        "qualification_profile_exposed": False,
    }


def record_provisional_review(
    root: Path,
    candidate_head: str,
    review_slot_id: str,
    provisional_review_path: Path,
) -> dict[str, Any]:
    campaign = load_campaign(root, validate_private=False)
    verify_inventory(root)
    if campaign.get("candidate_head") != candidate_head:
        raise CampaignError("provisional review is bound to a different campaign candidate")
    state = private_cycle_for_review_slot(campaign, review_slot_id)
    if state.get("state") != "review_prepared":
        raise CampaignError("provisional review requires one review-prepared opaque slot")
    (
        source_bytes,
        provisional,
        preparation,
        preparation_sha256,
        _contract_reference,
    ) = load_and_validate_reviewer_provisional_review(
        root,
        candidate_head,
        review_slot_id,
        provisional_review_path,
    )
    if (
        preparation.get("repository_revision") != state.get("repository_revision")
        or preparation.get("reviewer_repository_path")
        != state.get("reviewer_repository_path")
        or preparation_sha256 != state.get("review_preparation_sha256")
    ):
        raise CampaignError("blind reviewer preparation identity, candidate, or hash changed")
    provisional_destination = slot_artifact_path(
        root, "reviewer", "provisional", review_slot_id
    )
    source = provisional_review_path.resolve()
    if source == provisional_destination.resolve():
        raise CampaignError("provisional review input must remain separate until recording")
    if provisional_destination.exists():
        raise CampaignError("fixed provisional review already exists")
    source_sha256 = hashlib.sha256(source_bytes).hexdigest()
    inventory = load_inventory(root)
    artifact_name = relative(root, provisional_destination)
    artifacts = inventory.setdefault("artifacts", {})
    if artifact_name in artifacts:
        raise CampaignError("fixed provisional review is already inventory-bound")
    updated_inventory = copy.deepcopy(inventory)
    updated_inventory["artifacts"][artifact_name] = {
        "bytes": len(source_bytes),
        "sha256": source_sha256,
    }
    updated_campaign = copy.deepcopy(campaign)
    updated_state = private_cycle_for_review_slot(updated_campaign, review_slot_id)
    updated_state["state"] = "provisional_recorded"
    updated_state["provisional_review_sha256"] = source_sha256
    updated_campaign["provisional_count"] = sum(
        item.get("state") in {"provisional_recorded", "sealed"}
        for item in updated_campaign["cycles"].values()
    )

    campaign_path = campaign_file(root)
    evidence_path = inventory_path(root)
    original_campaign = campaign_path.read_bytes()
    original_inventory = evidence_path.read_bytes()
    assert_reviewer_artifacts_are_behavior_opaque(root)
    try:
        atomic_write_bytes(provisional_destination, source_bytes)
        atomic_write_bytes(evidence_path, json_bytes(updated_inventory))
        atomic_write_bytes(campaign_path, json_bytes(updated_campaign))
    except BaseException:
        provisional_destination.unlink(missing_ok=True)
        atomic_write_bytes(evidence_path, original_inventory)
        atomic_write_bytes(campaign_path, original_campaign)
        raise
    return {
        "kind": "phase8_dogfood_provisional_review_recorded",
        "review_slot_id": review_slot_id,
        "candidate_head": candidate_head,
        "state": "provisional_recorded",
        "provisional_review": artifact_name,
        "provisional_review_bytes": len(source_bytes),
        "provisional_review_sha256": source_sha256,
        "provisional_count": updated_campaign["provisional_count"],
        "evaluator_material_exposed": False,
        "qualification_profile_exposed": False,
    }


def verify_all_provisional_reviews_fixed(
    root: Path, campaign: dict[str, Any]
) -> None:
    states = list(campaign.get("cycles", {}).values())
    if (
        len(states) != QUALIFICATION_CYCLE_COUNT
        or campaign.get("provisional_count") != QUALIFICATION_CYCLE_COUNT
        or any(state.get("state") not in {"provisional_recorded", "sealed"} for state in states)
    ):
        raise CampaignError(
            "qualification profile and evaluator reveal require all eight provisional reviews"
        )
    inventory = load_inventory(root)
    for state in states:
        review_slot_id = state.get("review_slot_id")
        path = slot_artifact_path(root, "reviewer", "provisional", review_slot_id)
        entry = inventory.get("artifacts", {}).get(relative(root, path))
        if (
            not path.is_file()
            or harness.sha256(path) != state.get("provisional_review_sha256")
            or not isinstance(entry, dict)
            or entry.get("sha256") != state.get("provisional_review_sha256")
            or entry.get("bytes") != path.stat().st_size
        ):
            raise CampaignError("fixed provisional review hash or inventory binding changed")


def reveal_qualification_profile(root: Path, candidate_head: str) -> dict[str, Any]:
    campaign = load_campaign(root, validate_private=False)
    verify_inventory(root)
    if campaign.get("candidate_head") != candidate_head:
        raise CampaignError("qualification profile reveal is bound to a different candidate")
    if campaign.get("qualification_profile_state") != "hidden":
        raise CampaignError("qualification profile has already been revealed")
    verify_all_provisional_reviews_fixed(root, campaign)
    profile = validate_private_qualification_profile(root, campaign)
    campaign["qualification_profile_state"] = "revealed"
    campaign["qualification_profile_revealed_sha256"] = harness.sha256(
        qualification_profile_path(root)
    )
    save_campaign(root, campaign)
    return {
        "kind": "phase8_dogfood_qualification_profile_revealed",
        "candidate_head": candidate_head,
        "provisional_count": QUALIFICATION_CYCLE_COUNT,
        "qualification_profile_state": "revealed",
        "qualification_profile_sha256": harness.sha256(qualification_profile_path(root)),
        "assignment_count": profile["assignment_count"],
        "profile_validation": "passed",
        "evaluator_material_available": True,
    }


def seal_cycle(
    root: Path,
    kind: str,
    cycle: int,
    prepared_descriptor: Path,
) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    verify_all_provisional_reviews_fixed(root, campaign)
    if campaign.get("qualification_profile_state") != "revealed":
        raise CampaignError(
            "evaluator reveal requires the all-provisionals qualification-profile reveal"
        )
    state = campaign["cycles"][cycle_key(kind, cycle)]
    if state.get("state") != "provisional_recorded":
        raise CampaignError("cycle sealing requires one recorded provisional review")
    preparation_path = reviewer_preparation_path(root, kind, cycle)
    preparation = read_json(preparation_path)
    if (
        preparation.get("kind") != "phase8_blind_review_preparation"
        or preparation.get("review_slot_id") != state.get("review_slot_id")
        or harness.sha256(preparation_path) != state.get("review_preparation_sha256")
    ):
        raise CampaignError("blind reviewer preparation identity or hash changed")
    provisional_destination = reviewer_provisional_path(root, kind, cycle)
    provisional = read_json(provisional_destination)
    provisional_errors = harness.blind_first_review_errors(
        {
            "kind": "phase8_blind_review_preparation_reference",
            "review_slot_id": state["review_slot_id"],
            "sha256": state["review_preparation_sha256"],
        },
        provisional,
        len(preparation.get("owner_document_locations", [])),
    )
    if provisional_errors:
        raise CampaignError("provisional review does not qualify: " + "; ".join(provisional_errors))
    inventory_entry = load_inventory(root).get("artifacts", {}).get(
        relative(root, provisional_destination)
    )
    if (
        not provisional_destination.is_file()
        or harness.sha256(provisional_destination)
        != state.get("provisional_review_sha256")
        or not isinstance(inventory_entry, dict)
        or inventory_entry.get("sha256") != state.get("provisional_review_sha256")
        or inventory_entry.get("bytes") != provisional_destination.stat().st_size
    ):
        raise CampaignError("fixed provisional review hash or inventory binding changed")
    assert_reviewer_artifacts_are_behavior_opaque(root)

    descriptor = read_json(prepared_descriptor.resolve())
    if "evidence" in descriptor:
        raise CampaignError("evaluator-prepared descriptor must not contain collection evidence")
    if descriptor.get("repository_class") != kind or descriptor.get("cycle") != cycle:
        raise CampaignError("evaluator descriptor is bound to a different cycle")
    if descriptor.get("behavior_class") != private_behavior_class(root, state, campaign):
        raise CampaignError("evaluator descriptor is bound to the wrong behavior class")
    if descriptor.get("repository_revision") != state["repository_revision"]:
        raise CampaignError("evaluator descriptor is bound to the wrong pinned revision")
    if (
        preparation.get("work_user_task") != descriptor.get("work_user_task")
        or preparation.get("fresh_resume_user_task")
        != descriptor.get("fresh_resume_user_task")
        or preparation.get("work_scope") != descriptor.get("work_scope")
    ):
        raise CampaignError("sealed descriptor changed the blind reviewer preparation basis")
    references = descriptor.get("behavior_review", {}).get("provenance_references", [])
    independent = descriptor.get("behavior_review", {}).get("independent_review")
    if not isinstance(independent, dict):
        raise CampaignError("evaluator descriptor has no independent review comparison")
    supplied_provisional = independent.get("provisional_review")
    conclusion_fields = {
        "kind",
        "status",
        "reviewer_role",
        "classification",
        "materiality_conclusion",
        "material_outcome_unavoidable",
        "operator_prompt_does_not_disclose_material_outcome",
        "basis",
        "provenance_reference_indices",
    }
    if not isinstance(supplied_provisional, dict) or any(
        supplied_provisional.get(field) != provisional.get(field)
        for field in conclusion_fields
    ):
        raise CampaignError("final comparison cannot rewrite the fixed provisional review")
    independent["review_preparation"] = {
        "kind": "phase8_blind_review_preparation_reference",
        "review_slot_id": state["review_slot_id"],
        "sha256": state["review_preparation_sha256"],
    }
    independent["provisional_review"] = provisional
    errors = harness.cycle_descriptor_errors(
        descriptor,
        candidate_revision=campaign["candidate_head"],
        target_repository=Path(state["repository_path"]),
        verify_provenance=True,
    )
    if errors:
        raise CampaignError("evaluator descriptor does not qualify: " + "; ".join(errors))
    operator_text = f"{descriptor['work_user_task']}\n{descriptor['fresh_resume_user_task']}"
    if "EVALUATOR_ONLY" in operator_text or any(
        value in operator_text for value in hidden_evaluator_strings(descriptor)
    ):
        raise CampaignError("operator-facing task would expose evaluator-only material")
    destination = evaluator_descriptor_path(root, kind, cycle)
    if destination.exists():
        raise CampaignError("authoritative evaluator descriptor already exists")
    write_json(destination, descriptor)
    state["state"] = "sealed"
    state["sealed_semantic_sha256"] = descriptor_semantic_sha256(descriptor)
    state["operator_task_artifacts"] = write_operator_task_artifacts(
        root, state, descriptor
    )
    save_campaign(root, campaign)
    register_artifact(root, destination)
    for task in state["operator_task_artifacts"].values():
        register_artifact(root, root / task["path"])
    run_sheet = render_operator_run_sheet(root)
    register_artifact(root, run_sheet, replace=True)
    assert_reviewer_artifacts_are_behavior_opaque(root)
    return {
        "kind": "phase8_dogfood_sealed_cycle",
        "review_slot_id": state["review_slot_id"],
        "repository_revision": state["repository_revision"],
        "sealed_semantic_sha256": state["sealed_semantic_sha256"],
        "operator_task_artifacts": state["operator_task_artifacts"],
        "operator_run_sheet": relative(root, run_sheet),
    }


def activate_cycle(root: Path, kind: str, cycle: int) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    key = cycle_key(kind, cycle)
    state = campaign["cycles"][key]
    load_sealed_descriptor(root, kind, cycle, campaign)
    repository = Path(state["repository_path"])
    binary = Path(campaign["candidate_binary"])
    manifest = repository / ".codex/volicord-integration.json"
    if manifest.exists():
        run_checked([
            str(binary), "--runtime", state["runtime_home"], "--json",
            "--repository", str(repository), "codex", "disable",
        ])
    result = run_checked(
        [
            str(binary), "--runtime", state["runtime_home"], "--json",
            "--repository", str(repository), "codex", "enable",
        ]
    )
    verification = verify_static_codex_integration(
        repository,
        Path(state["runtime_home"]),
        binary,
        result,
    )
    state["codex_enabled"] = True
    campaign["active_cycle_by_repository"][kind] = cycle
    save_campaign(root, campaign)
    return {"enable_result": result, "static_verification": verification}


def activate_all(root: Path) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    for kind in CLASSES:
        for cycle in cycle_numbers(kind):
            load_sealed_descriptor(root, kind, cycle, campaign)
    results = []
    for kind in CLASSES:
        for cycle in cycle_numbers(kind):
            state = load_campaign(root)["cycles"][cycle_key(kind, cycle)]
            results.append({
                "repository_class": kind,
                "review_slot_id": state["review_slot_id"],
                "result": activate_cycle(root, kind, cycle),
            })
    return {
        "kind": "phase8_dogfood_campaign_activation",
        "cycle_count": len(results),
        "repository_and_hook_trust": "user_controlled_not_automated",
        "cycles": results,
    }


def prepare_campaign(
    root: Path,
    campaign_id: str,
    candidate_head: str,
    repository_input: Path,
    *,
    candidate_binary: Path | None = None,
    enable: bool = False,
    cloner: Callable[[Path, Path, str], None] = clone_repository,
    slot_id_factory: Callable[[], str] = new_review_slot_id,
    behavior_assignment_factory: Callable[[], list[tuple[str, int, str]]] = new_private_behavior_assignments,
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
    document_language = raw_input.get("document_language", "en")
    if (
        not isinstance(document_language, str)
        or not document_language.strip()
        or document_language != document_language.strip()
        or len(document_language.encode("utf-8")) > 128
    ):
        raise CampaignError("campaign document language must be bounded non-empty text")
    viewer_locale = raw_input.get("viewer_locale", "en")
    if viewer_locale not in {"en", "ko"}:
        raise CampaignError("campaign Viewer locale must be en or ko")
    _, identities = harness.load_repository_specs(repository_input, candidate_head, definition)
    failures = [item for item in identities if item["status"] != "passed"]
    if failures:
        raise CampaignError("one or more source repository identities do not qualify")
    private_behaviors = behavior_assignment_factory()
    validate_private_behavior_assignments(private_behaviors)
    assignments: list[tuple[str, int, str, str]] = []
    for _attempt in range(64):
        assignments = [
            (kind, number, behavior, slot_id_factory())
            for kind, number, behavior in private_behaviors
        ]
        review_slot_ids = [
            review_slot_id
            for _kind, _number, _behavior, review_slot_id in assignments
        ]
        if (
            len(review_slot_ids) != len(set(review_slot_ids))
            or any(
                not isinstance(review_slot_id, str)
                or REVIEW_SLOT_ID.fullmatch(review_slot_id) is None
                for review_slot_id in review_slot_ids
            )
        ):
            raise CampaignError(
                "opaque review slot generation produced a duplicate or malformed identity"
            )
        if not opaque_order_exposes_logical_cycle_order(assignments):
            break
    else:
        raise CampaignError("opaque review slot generation could not produce a blind order")
    root.mkdir(parents=True, exist_ok=True)
    root.chmod(0o700)
    binary = candidate_binary.resolve() if candidate_binary else install_candidate(root)
    if not binary.is_file():
        raise CampaignError("candidate binary is unavailable")
    cycles: dict[str, Any] = {}
    behavior_by_slot: dict[str, str] = {}
    for kind, number, behavior_class, review_slot_id in assignments:
        spec = specs[kind]
        revision = candidate_head if kind == "volicord" else spec["revision"]
        destination = slot_root(root, review_slot_id)
        reviewer_repository = root / "reviewer/workspaces" / review_slot_id / "repository"
        cycles[cycle_key(kind, number)] = {
            "review_slot_id": review_slot_id,
            "repository_class": kind,
            "cycle": number,
            "repository_path": str((destination / "repository").resolve()),
            "reviewer_repository_path": str(reviewer_repository.resolve()),
            "repository_revision": revision,
            "runtime_home": str((destination / "runtime").resolve()),
            "state": "prepared",
            "sealed_semantic_sha256": None,
            "operator_task_artifacts": None,
            "review_preparation_sha256": None,
            "provisional_review_sha256": None,
            "project_id": None,
            "codex_enabled": False,
        }
        behavior_by_slot[review_slot_id] = behavior_class
    for state in sorted(cycles.values(), key=lambda item: item["review_slot_id"]):
        review_slot_id = state["review_slot_id"]
        destination = slot_root(root, review_slot_id)
        (destination / "evidence").mkdir(parents=True)
        (destination / "runtime").mkdir()
        spec = specs[state["repository_class"]]
        cloner(
            Path(spec["path"]).resolve(),
            Path(state["repository_path"]),
            state["repository_revision"],
        )
        reviewer_repository = Path(state["reviewer_repository_path"])
        reviewer_repository.parent.mkdir(parents=True, exist_ok=True)
        cloner(
            Path(spec["path"]).resolve(),
            reviewer_repository,
            state["repository_revision"],
        )
        write_json(
            slot_artifact_path(root, "evaluator", "inputs", review_slot_id),
            descriptor_skeleton(
                state["repository_class"],
                state["cycle"],
                behavior_by_slot[review_slot_id],
                state["repository_revision"],
                candidate_head,
            ),
        )
    mapping = slot_mapping_value(cycles, behavior_by_slot, secrets.token_hex(32))
    write_json(slot_mapping_path(root), mapping)
    profile = qualification_profile_value(campaign_id, candidate_head, mapping)
    write_json(qualification_profile_path(root), profile)
    campaign = {
        "kind": "phase8_dogfood_campaign",
        "schema_version": 1,
        "campaign_id": campaign_id,
        "campaign_root": str(root),
        "candidate_head": candidate_head,
        "candidate_binary": str(binary),
        "document_language": document_language,
        "viewer_locale": viewer_locale,
        "repository_input": relative(root, root / "repository-input.json"),
        "terminal_outcome": None,
        "active_cycle_by_repository": {},
        "cycles": cycles,
        "opaque_slot_mapping_sha256": harness.sha256(slot_mapping_path(root)),
        "qualification_profile_sha256": harness.sha256(qualification_profile_path(root)),
        "qualification_profile_state": "hidden",
        "qualification_profile_revealed_sha256": None,
        "provisional_count": 0,
    }
    write_json(root / "repository-input.json", raw_input)
    save_campaign(root, campaign)
    write_json(inventory_path(root), load_inventory(root))
    reviewer_contract = reviewer_provisional_contract_path(root)
    write_json(reviewer_contract, harness.provisional_review_contract())
    reviewer_index = render_reviewer_index(root)
    assert_reviewer_artifacts_are_behavior_opaque(root)
    run_sheet = render_operator_run_sheet(root)
    if enable:
        raise CampaignError("cycles must be evaluator-sealed before activation")
    preparation = {
        "kind": "phase8_dogfood_campaign_preparation",
        "campaign_id": campaign_id,
        "candidate_head": candidate_head,
        "candidate_worktree_clean": True,
        "repository_identities": identities,
        "cycle_count": QUALIFICATION_CYCLE_COUNT,
        "candidate_local_install": str(binary),
        "repository_trust": "user_controlled_not_automated",
    }
    write_json(root / "preparation.json", preparation)
    for path in (
        root / "repository-input.json",
        slot_mapping_path(root),
        qualification_profile_path(root),
        reviewer_contract,
        reviewer_index,
        run_sheet,
        root / "preparation.json",
    ):
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
    if state["state"] != "sealed":
        raise CampaignError("work collection requires a valid sealed evaluator descriptor")
    descriptor_path, descriptor = load_sealed_descriptor(root, kind, cycle, campaign)
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
            campaign["candidate_head"],
            descriptor,
            harness.sha256(descriptor_path),
            capture,
            target_repository=Path(state["repository_path"]),
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
    for path in (destination, cycle_root(root, kind, cycle) / "work-intake.json", activation):
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
    checkpoints = capture.successful_calls("checkpoint_record")
    if len(resolves) != 1 or len(recalls) != 1 or capture.successful_calls("project_initialize"):
        raise CampaignError("resume must resolve one existing Project and must not initialize a replacement")
    if not checkpoints:
        raise CampaignError("resume must retain its pre-work analysis baseline in a Checkpoint")
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
    meaningful_changes = harness.meaningful_work_path_observations(capture)
    first_write = min(
        (item.sequence for item in meaningful_changes),
        default=None,
    )
    if not all(
        harness.checkpoint_baseline_is_pre_work(
            capture,
            checkpoint,
            project_id=str(project_id),
            boundary_completion_sequence=recall.completion_sequence,
            first_write_sequence=first_write,
        )
        for checkpoint in checkpoints
    ):
        raise CampaignError(
            "resume Checkpoint baseline identity, Project, Recall boundary, or pre-write ordering is invalid"
        )
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
        "work_session_start_activation_observed": work_activation,
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
) -> dict[str, Any]:
    completed = subprocess.run(
        [
            str(binary), "--runtime", str(runtime), "--repository", str(repository),
            "document", "export", kind, "--format", format_name,
            "--output", str(destination), "--language", language,
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
            f"supported document export exited {completed.returncode}; "
            f"destination_present={destination.is_file()}"
        ),
    }


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
            "--level", "deep",
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
    cycle: int,
    binary: Path,
    runtime: Path,
    project_id: str,
    candidate_head: str,
    locale: str,
    language: str,
    snapshotter: Callable[[Path, Path, str, Path, str, str], dict[str, Any]],
) -> tuple[dict[str, Any], list[Path]]:
    destination = cycle_root(root, kind, cycle) / "evidence/viewer-snapshot.html"
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
    evidence: dict[str, Any] = {
        "kind": "phase8_viewer_snapshot_evidence_summary",
        "schema_version": 1,
        "status": "failed",
        "project_id": project_id,
        "candidate_head": candidate_head,
        "repository_class": kind,
        "cycle": cycle,
        "locale": locale,
        "requested_language": language,
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
    summary = cycle_root(root, kind, cycle) / "viewer-snapshot-summary.json"
    write_json(summary, evidence)
    produced.append(summary)
    return evidence, produced


def collect_document_evidence(
    root: Path,
    kind: str,
    cycle: int,
    binary: Path,
    runtime: Path,
    repository: Path,
    language: str,
    documenter: Callable[[Path, Path, Path, str, str, Path, str], dict[str, Any]],
) -> tuple[dict[str, Any], list[Path]]:
    directory = cycle_root(root, kind, cycle) / "evidence/generated-documents"
    directory.mkdir(parents=True, exist_ok=True)
    documents: dict[str, Any] = {}
    produced: list[Path] = []
    for document_kind in DOCUMENT_KINDS:
        formats: dict[str, Any] = {}
        for format_name, suffix in DOCUMENT_FORMATS:
            destination = directory / f"{document_kind}.{suffix}"
            try:
                result = documenter(
                    binary,
                    runtime,
                    repository,
                    document_kind,
                    format_name,
                    destination,
                    language,
                )
            except (OSError, ValueError, CampaignError) as error:
                result = {
                    "status": "failed",
                    "basis": f"document evidence adapter failed: {type(error).__name__}",
                }
            if result.get("status") == "passed" and destination.is_file():
                formats[format_name] = {
                    "status": "passed",
                    "relative_evidence_path": relative(root, destination),
                    "bytes": destination.stat().st_size,
                    "sha256": harness.sha256(destination),
                }
                produced.append(destination)
            else:
                basis = result.get("basis") if isinstance(result, dict) else None
                if not isinstance(basis, str) or not basis.strip():
                    basis = "supported document export did not produce usable evidence"
                formats[format_name] = {
                    "status": "failed",
                    "basis": basis[:512],
                }
        document_status = (
            "passed"
            if all(formats[name]["status"] == "passed" for name, _suffix in DOCUMENT_FORMATS)
            else "failed"
        )
        documents[document_kind] = {"status": document_status, "formats": formats}
    summary = {
        "kind": "phase8_generated_document_evidence_summary",
        "schema_version": 1,
        "language": language,
        "status": "passed" if all(item["status"] == "passed" for item in documents.values()) else "failed",
        "required_document_kinds": list(DOCUMENT_KINDS),
        "documents": documents,
    }
    return summary, produced


def write_operator_document_review_index(
    root: Path,
    kind: str,
    cycle: int,
    summary: dict[str, Any],
) -> Path:
    state = cycle_state(root, kind, cycle)
    review_slot_id = state["review_slot_id"]
    lines = [
        f"# Generated document review: {kind} slot {review_slot_id}",
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
    path = root / "operator/document-review" / f"{review_slot_id}.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")
    assert_operator_artifacts_do_not_leak(root)
    return path


def extract_resume_evidence(
    root: Path,
    kind: str,
    cycle: int,
    capture: Any,
    destination: Path,
    *,
    exporter: Callable[[Path, Path, Path, Path], None] = default_export,
    documenter: Callable[[Path, Path, Path, str, str, Path, str], dict[str, Any]] = generate_document,
    snapshotter: Callable[[Path, Path, str, Path, str, str], dict[str, Any]] = generate_viewer_snapshot,
    final_state: str = "resume_collected",
) -> dict[str, Any]:
    campaign = load_campaign(root)
    key = cycle_key(kind, cycle)
    state = campaign["cycles"][key]
    descriptor_path, descriptor = load_sealed_descriptor(root, kind, cycle, campaign)
    project_id = inspect_resume(capture, descriptor, state)
    binary = Path(campaign["candidate_binary"])
    runtime = Path(state["runtime_home"])
    repository = Path(state["repository_path"])
    bundle = cycle_root(root, kind, cycle) / "context.bundle.json"
    exporter(binary, runtime, repository, bundle)
    try:
        canonical = harness.load_canonical_bundle(bundle)
    except (OSError, EvidenceError) as error:
        raise CampaignError("context export is not a supported canonical bundle") from error
    if canonical.project_id != project_id:
        raise CampaignError("portable bundle Project identity does not match the resume capture")
    descriptor["evidence"] = {
        "captures": {
            "work": {"file": relative(root, cycle_root(root, kind, cycle) / "evidence/work.rollout.jsonl"), "sha256": harness.sha256(cycle_root(root, kind, cycle) / "evidence/work.rollout.jsonl")},
            "resume": {"file": relative(root, destination), "sha256": harness.sha256(destination)},
        },
        "canonical_bundle": {"file": relative(root, bundle), "sha256": harness.sha256(bundle)},
    }
    errors = harness.cycle_descriptor_errors(descriptor)
    if errors:
        raise CampaignError("completed descriptor does not qualify: " + "; ".join(errors))
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
    document_result, document_paths = collect_document_evidence(
        root,
        kind,
        cycle,
        binary,
        runtime,
        repository,
        campaign.get("document_language", "en"),
        documenter,
    )
    document_summary = cycle_root(root, kind, cycle) / "documents-summary.json"
    write_json(document_summary, document_result)
    document_review_index = write_operator_document_review_index(
        root, kind, cycle, document_result
    )
    snapshot_result, snapshot_paths = collect_viewer_snapshot_evidence(
        root,
        kind,
        cycle,
        binary,
        runtime,
        project_id,
        campaign["candidate_head"],
        campaign.get("viewer_locale", "en"),
        campaign.get("document_language", "en"),
        snapshotter,
    )
    snapshot_summary = cycle_root(root, kind, cycle) / "viewer-snapshot-summary.json"
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
        "cycle": cycle,
        "project_id": project_id,
        "resume_capture_sha256": capture.source_sha256,
        "canonical_bundle_sha256": harness.sha256(bundle),
        "descriptor_evidence_completed": True,
        "runtime_home_copied": False,
        "document_evidence": document_result,
        "viewer_snapshot_evidence": snapshot_result,
    }


def collect_resume(
    root: Path,
    kind: str,
    cycle: int,
    raw_capture: Path,
    *,
    exporter: Callable[[Path, Path, Path, Path], None] = default_export,
    documenter: Callable[[Path, Path, Path, str, str, Path, str], dict[str, Any]] = generate_document,
    snapshotter: Callable[[Path, Path, str, Path, str, str], dict[str, Any]] = generate_viewer_snapshot,
) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    if campaign.get("terminal_outcome") is not None:
        raise CampaignError("later collection is blocked; create a new campaign identity")
    state = campaign["cycles"][cycle_key(kind, cycle)]
    if state["state"] != "work_collected":
        raise CampaignError("resume collection requires a resume_allowed work intake")
    destination = cycle_root(root, kind, cycle) / "evidence/resume.rollout.jsonl"
    copy_exact(raw_capture.resolve(), destination)
    try:
        capture = load_codex_capture(destination)
    except (OSError, EvidenceError) as error:
        raise CampaignError("resume rollout is not a supported normalized Codex capture") from error
    return extract_resume_evidence(
        root,
        kind,
        cycle,
        capture,
        destination,
        exporter=exporter,
        documenter=documenter,
        snapshotter=snapshotter,
    )


def batch_rollout_paths(
    explicit_paths: list[Path] | None,
    rollout_directory: Path | None,
) -> list[Path]:
    if (explicit_paths is None) == (rollout_directory is None):
        raise CampaignError("collect-batch requires either sixteen raw rollouts or one directory")
    if rollout_directory is not None:
        directory = rollout_directory.resolve()
        if not directory.is_dir():
            raise CampaignError("batch rollout directory is unavailable")
        entries = sorted(directory.iterdir())
        if len(entries) != BATCH_CAPTURE_COUNT or not all(path.is_file() for path in entries):
            raise CampaignError("batch rollout directory must contain exactly sixteen files")
        paths = entries
    else:
        paths = [path.resolve() for path in explicit_paths or []]
        if len(paths) != BATCH_CAPTURE_COUNT:
            raise CampaignError("collect-batch requires exactly sixteen explicit raw rollouts")
    resolved = [path.resolve() for path in paths]
    if len(set(resolved)) != BATCH_CAPTURE_COUNT or not all(path.is_file() for path in resolved):
        raise CampaignError("batch rollout inputs must be sixteen distinct files")
    return resolved


def map_batch_rollouts(
    root: Path,
    raw_paths: list[Path],
) -> dict[tuple[str, int, str], tuple[Path, Any]]:
    campaign = load_campaign(root)
    verify_inventory(root)
    slots: dict[tuple[str, int, str], tuple[dict[str, Any], dict[str, Any]]] = {}
    for kind in CLASSES:
        for cycle in cycle_numbers(kind):
            _descriptor_path, descriptor = load_sealed_descriptor(root, kind, cycle, campaign)
            state = campaign["cycles"][cycle_key(kind, cycle)]
            for role, field in (("work", "work_user_task"), ("resume", "fresh_resume_user_task")):
                slots[(kind, cycle, role)] = (state, descriptor)

    mapped: dict[tuple[str, int, str], tuple[Path, Any]] = {}
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
                "batch rollout maps to zero sealed cycle roles",
                diagnostic=diagnostic,
            ) from error
        provenance_matches = (
            capture.source == "vscode"
            and capture.originator == "codex_vscode"
            and capture.fresh_user_thread
            and bool(capture.user_turns)
        )
        if not nonempty_session_id(capture.session_id):
            raise CampaignError("batch rollout has no bounded session identity")
        if capture.session_id in sessions:
            raise CampaignError("batch rollout reuses a Codex session identity")
        sessions[capture.session_id] = path
        candidates = []
        task_candidates = []
        revision_candidates = []
        workspace_candidates = []
        for slot, (state, descriptor) in slots.items():
            role = slot[2]
            task_field = "work_user_task" if role == "work" else "fresh_resume_user_task"
            task_matches = bool(capture.user_turns) and harness.codex_user_turn_transport_identity_matches(
                capture.user_turns[0].text,
                descriptor[task_field],
            )
            revision_matches = capture.git_revision == state["repository_revision"]
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
                    "batch rollout maps to zero sealed cycle roles",
                    diagnostic=diagnostic,
                )
            matching_roles = [
                {
                    "review_slot_id": slots[slot][0]["review_slot_id"],
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
                "batch rollout maps to multiple sealed cycle roles",
                diagnostic=diagnostic,
            )
        slot = candidates[0]
        if slot in mapped:
            raise CampaignError("batch rollouts contain duplicate evidence for one sealed cycle role")
        mapped[slot] = (path, capture)
    missing = sorted(set(slots) - set(mapped))
    if missing:
        raise CampaignError("batch rollouts are missing one or more sealed cycle roles")
    return mapped


def nonempty_session_id(value: Any) -> bool:
    return (
        isinstance(value, str)
        and bool(value.strip())
        and value == value.strip()
        and len(value.encode("utf-8")) <= 512
    )


def missing_activation_diagnostic(
    source: Path,
    capture: Any,
    review_slot_id: str,
    role: str,
) -> dict[str, Any]:
    return {
        "kind": "phase8_dogfood_missing_session_start_activation",
        "classification": "operator_environment_setup_failure",
        "source_file": str(source.resolve()),
        "source_sha256": capture.source_sha256,
        "session_id": capture.session_id,
        "review_slot_id": review_slot_id,
        "role": role,
        "volicord_mcp_calls_observed": bool(capture.tool_calls),
        "runtime_session_start_activation_observed": False,
    }


def collect_batch(
    root: Path,
    raw_paths: list[Path],
    *,
    exporter: Callable[[Path, Path, Path, Path], None] = default_export,
    documenter: Callable[[Path, Path, Path, str, str, Path, str], dict[str, Any]] = generate_document,
    snapshotter: Callable[[Path, Path, str, Path, str, str], dict[str, Any]] = generate_viewer_snapshot,
) -> dict[str, Any]:
    campaign = load_campaign(root)
    verify_inventory(root)
    if campaign.get("terminal_outcome") is not None:
        raise CampaignError("campaign already stopped; create a new campaign identity")
    for state in campaign["cycles"].values():
        if state.get("state") != "sealed":
            raise CampaignError("batch collection requires all eight sealed cycles")

    mapped = map_batch_rollouts(root, raw_paths)
    for (kind, cycle, role), (source, _capture) in sorted(mapped.items()):
        destination = cycle_root(root, kind, cycle) / "evidence" / f"{role}.rollout.jsonl"
        if source.resolve() == destination.resolve():
            raise CampaignError("batch rollout source must be outside its sealed evidence destination")
        copy_exact(source, destination)
        register_artifact(root, destination)

    cycle_results: list[dict[str, Any]] = []
    has_product_blocker = False
    has_environment_invalid = False
    has_evidence_failure = False
    environment_invalid_diagnostics: list[dict[str, Any]] = []
    for kind in CLASSES:
        for cycle in cycle_numbers(kind):
            key = cycle_key(kind, cycle)
            state = campaign["cycles"][key]
            descriptor_path, descriptor = load_sealed_descriptor(root, kind, cycle, campaign)
            work_destination = cycle_root(root, kind, cycle) / "evidence/work.rollout.jsonl"
            resume_destination = cycle_root(root, kind, cycle) / "evidence/resume.rollout.jsonl"
            work_capture = mapped[(kind, cycle, "work")][1]
            resume_capture = mapped[(kind, cycle, "resume")][1]
            if (
                not work_capture.repository_scoped_activation_observed
                or not resume_capture.repository_scoped_activation_observed
            ):
                has_environment_invalid = True
            if not work_capture.repository_scoped_activation_observed:
                environment_invalid_diagnostics.append(missing_activation_diagnostic(
                    mapped[(kind, cycle, "work")][0],
                    work_capture,
                    state["review_slot_id"],
                    "work",
                ))
            if not resume_capture.repository_scoped_activation_observed:
                environment_invalid_diagnostics.append(missing_activation_diagnostic(
                    mapped[(kind, cycle, "resume")][0],
                    resume_capture,
                    state["review_slot_id"],
                    "resume",
                ))
            project_ids = observed_project_ids(work_capture)
            blocker: dict[str, Any] | None = None
            try:
                blocker = harness.build_work_blocker_result(
                    campaign["candidate_head"],
                    descriptor,
                    harness.sha256(descriptor_path),
                    work_capture,
                    target_repository=Path(state["repository_path"]),
                )
            except ValueError as error:
                if "has no machine-observable terminal work blocker" not in str(error):
                    raise CampaignError(str(error)) from error

            work_intake_path = cycle_root(root, kind, cycle) / "work-intake.json"
            if blocker is None and len(project_ids) == 1:
                work_intake = {
                    "kind": "phase8_dogfood_work_intake",
                    "outcome": "resume_allowed",
                    "repository_class": kind,
                    "cycle": cycle,
                    "project_id": project_ids[0],
                    "work_capture_sha256": work_capture.source_sha256,
                    "repository_scoped_activation_observed": True,
                }
                state["state"] = "work_collected"
            elif blocker is not None:
                work_intake = blocker
                blocker_path = cycle_root(root, kind, cycle) / "blocker-result.json"
                write_json(blocker_path, blocker)
                register_artifact(root, blocker_path)
                has_environment_invalid |= blocker["outcome"] == "operator_environment_invalid"
                has_product_blocker |= blocker["outcome"] == "campaign_stop"
                state["state"] = blocker["outcome"]
            else:
                work_intake = {
                    "kind": "phase8_dogfood_work_intake",
                    "outcome": "campaign_stop",
                    "classification": "product_work_session_blocker",
                    "repository_class": kind,
                    "cycle": cycle,
                    "basis": "qualifying work capture did not expose exactly one Project identity",
                }
                state["state"] = "campaign_stop"
                has_product_blocker = True
            if len(project_ids) == 1:
                state["project_id"] = project_ids[0]
            state["work_session_id"] = work_capture.session_id
            write_json(work_intake_path, work_intake)
            activation_path = update_activation_summary(
                root,
                kind,
                cycle,
                work_session_start_activation_observed=(
                    work_capture.repository_scoped_activation_observed
                ),
                resume_session_start_activation_observed=(
                    resume_capture.repository_scoped_activation_observed
                ),
            )
            save_campaign(root, campaign)
            register_artifact(root, work_intake_path)
            register_artifact(root, activation_path)

            resume_result: dict[str, Any]
            if len(project_ids) == 1:
                try:
                    resume_result = extract_resume_evidence(
                        root,
                        kind,
                        cycle,
                        resume_capture,
                        resume_destination,
                        exporter=exporter,
                        documenter=documenter,
                        snapshotter=snapshotter,
                        final_state=(
                            "resume_collected"
                            if blocker is None
                            else "batch_diagnostic_evidence_collected"
                        ),
                    )
                except (CampaignError, EvidenceError, OSError, ValueError) as error:
                    resume_result = {
                        "kind": "phase8_dogfood_resume_intake",
                        "outcome": "evidence_failed",
                        "repository_class": kind,
                        "cycle": cycle,
                        "basis": f"{type(error).__name__}: {str(error)[:384]}",
                        "resume_capture_sha256": resume_capture.source_sha256,
                    }
                    has_evidence_failure = True
            else:
                resume_result = {
                    "kind": "phase8_dogfood_resume_intake",
                    "outcome": "prerequisite_unavailable",
                    "repository_class": kind,
                    "cycle": cycle,
                    "basis": "work Project identity was unavailable",
                    "resume_capture_sha256": resume_capture.source_sha256,
                }
                has_evidence_failure = True
            evidence_complete = (
                resume_result.get("outcome") == "evidence_collected"
                and resume_result.get("document_evidence", {}).get("status") == "passed"
                and resume_result.get("viewer_snapshot_evidence", {}).get("status") == "passed"
            )
            has_evidence_failure |= not evidence_complete
            cycle_results.append({
                "repository_class": kind,
                "cycle": cycle,
                "behavior_class": descriptor["behavior_class"],
                "intake_state": (
                    "accepted"
                    if blocker is None and work_intake["outcome"] == "resume_allowed" and evidence_complete
                    else "rejected"
                ),
                "qualification_state": "not_run",
                "work": {
                    "outcome": work_intake["outcome"],
                    "session_id": work_capture.session_id,
                    "relative_evidence_path": relative(root, work_destination),
                    "sha256": work_capture.source_sha256,
                    "activation_observed": work_capture.repository_scoped_activation_observed,
                },
                "resume": {
                    "outcome": resume_result["outcome"],
                    "session_id": resume_capture.session_id,
                    "relative_evidence_path": relative(root, resume_destination),
                    "sha256": resume_capture.source_sha256,
                    "activation_observed": resume_capture.repository_scoped_activation_observed,
                    **(
                        {"basis": resume_result["basis"]}
                        if isinstance(resume_result.get("basis"), str)
                        else {}
                    ),
                },
                "project_id": state.get("project_id"),
                "supported_evidence_complete": evidence_complete,
                "terminal_work_failure_preserved": blocker is not None,
            })
            campaign = load_campaign(root)

    campaign["terminal_outcome"] = (
        "operator_environment_invalid"
        if has_environment_invalid
        else "campaign_stop"
        if has_product_blocker or has_evidence_failure
        else None
    )
    save_campaign(root, campaign)
    summary = {
        "kind": "phase8_dogfood_batch_intake_summary",
        "schema_version": 1,
        "candidate_head": campaign["candidate_head"],
        "environment_invalid_diagnostics": environment_invalid_diagnostics,
        "intake_state": "accepted" if all(item["intake_state"] == "accepted" for item in cycle_results) else "rejected",
        "qualification_state": "not_run",
        "outcome": (
            "evidence_collected"
            if all(item["intake_state"] == "accepted" for item in cycle_results)
            else "operator_environment_invalid"
            if has_environment_invalid
            else "campaign_stop"
        ),
        "session_distinctness": {
            "status": "passed",
            "expected_count": BATCH_CAPTURE_COUNT,
            "observed_count": len({
                mapped[slot][1].session_id for slot in mapped
            }),
        },
        "cycles": cycle_results,
        "later_evidence_cannot_restore_terminal_work_failure": True,
    }
    summary_path = root / "batch-intake-summary.json"
    write_json(summary_path, summary)
    register_artifact(root, summary_path)
    return summary


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
        for number in cycle_numbers(kind):
            state = campaign["cycles"][cycle_key(kind, number)]
            if state["state"] != "resume_collected":
                raise CampaignError("all eight cycles must have resume evidence before finalization")
            descriptor, _ = load_sealed_descriptor(root, kind, number, campaign)
            real[str(number)] = relative(root, descriptor)
        repositories.append({
            **{key: spec[key] for key in ("class", "path", "origin", "revision", "license_file", "license_spdx", "provider_source_path") if key in spec},
            "revision": campaign["candidate_head"] if kind == "volicord" else spec["revision"],
            "real_session_evidence": real,
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
        "document_language": campaign.get("document_language", "en"),
        "viewer_locale": campaign.get("viewer_locale", "en"),
        "terminal_outcome": campaign["terminal_outcome"],
        "cycles": {
            key: {
                field: value[field]
                for field in (
                    "repository_class",
                    "cycle",
                    "behavior_class",
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
        "operator/RUN-SHEET.md": (root / "operator/RUN-SHEET.md").read_bytes(),
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
        if name.startswith("evaluator/descriptors/") and name.endswith(".json"):
            descriptor = read_json(path)
            state = campaign["cycles"][cycle_key(
                descriptor["repository_class"], descriptor["cycle"]
            )]
            review_slot_id = state["review_slot_id"]
            review_name = f"behavior-reviews/{review_slot_id}.json"
            review_bytes = (json.dumps(descriptor["behavior_review"], indent=2, sort_keys=True) + "\n").encode()
            files[review_name] = review_bytes
            review_index.append({
                "review_slot_id": review_slot_id,
                "repository_class": descriptor["repository_class"],
                "logical_cycle": descriptor["cycle"],
                "expected_behavior_class": descriptor["behavior_class"],
                "derived_archive_entry": review_name,
                "authoritative_descriptor": name,
                "sha256": hashlib.sha256(review_bytes).hexdigest(),
                "blind_review_preparation": (
                    f"reviewer/preparations/{review_slot_id}.json"
                ),
                "provisional_review": (
                    f"reviewer/provisional/{review_slot_id}.json"
                ),
            })
    if len(review_index) != QUALIFICATION_CYCLE_COUNT:
        raise CampaignError(
            f"review package requires {QUALIFICATION_CYCLE_COUNT} completed descriptors "
            "and behavior reviews"
        )
    files["behavior-reviews/index.json"] = (
        json.dumps({"kind": "phase8_behavior_review_index", "reviews": review_index}, indent=2, sort_keys=True) + "\n"
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


def prepare_human_review(root: Path, automated_result_path: Path) -> Path:
    load_campaign(root)
    verify_inventory(root)
    automated_bytes = automated_result_path.read_bytes()
    try:
        automated_result = json.loads(automated_bytes)
    except json.JSONDecodeError as error:
        raise CampaignError("automated Dogfood result is malformed") from error
    destination = root / "operator/human-review.json"
    if destination.exists():
        raise CampaignError("campaign-level human review artifact already exists")
    try:
        artifact = harness.human_review_template(
            automated_result,
            hashlib.sha256(automated_bytes).hexdigest(),
        )
    except ValueError as error:
        raise CampaignError(str(error)) from error
    write_json(destination, artifact)
    return destination


def qualify_human_review(
    root: Path,
    automated_result_path: Path,
    human_review_path: Path,
    output: Path,
) -> Path:
    load_campaign(root)
    verify_inventory(root)
    if output.exists():
        raise CampaignError("qualified Dogfood result destination already exists")
    expected_review = (root / "operator/human-review.json").resolve()
    if human_review_path.resolve() != expected_review:
        raise CampaignError("human review must be the campaign-level operator artifact")
    automated_bytes = automated_result_path.read_bytes()
    try:
        automated_result = json.loads(automated_bytes)
        human_review = read_json(human_review_path)
    except json.JSONDecodeError as error:
        raise CampaignError("automated Dogfood result is malformed") from error
    try:
        qualified = harness.combine_human_review(
            automated_result,
            human_review,
            hashlib.sha256(automated_bytes).hexdigest(),
        )
    except ValueError as error:
        raise CampaignError(str(error)) from error
    if output.resolve().parent != root.resolve():
        raise CampaignError("qualified Dogfood result must remain at the campaign root")
    write_json(output, qualified)
    register_artifact(root, human_review_path)
    register_artifact(root, output)
    return output


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="command", required=True)
    prepare = sub.add_parser("prepare")
    prepare.add_argument("--campaign-root", required=True)
    prepare.add_argument("--campaign-id", required=True)
    prepare.add_argument("--candidate-head", required=True)
    prepare.add_argument("--repositories", required=True)
    prepare_reviewer = sub.add_parser("prepare-review")
    validate_provisional = sub.add_parser("validate-provisional-review")
    record_provisional = sub.add_parser("record-provisional-review")
    reveal_profile = sub.add_parser("reveal-qualification-profile")
    seal = sub.add_parser("seal-cycle")
    activate = sub.add_parser("activate-cycle")
    activate_every = sub.add_parser("activate-all")
    collect_w = sub.add_parser("collect-work")
    collect_r = sub.add_parser("collect-resume")
    collect_b = sub.add_parser("collect-batch")
    finalize = sub.add_parser("finalize-manifest")
    package = sub.add_parser("package-review")
    prepare_review = sub.add_parser("prepare-human-review")
    qualify_review = sub.add_parser("qualify-review")
    for command in (prepare_reviewer, seal, activate, collect_w, collect_r):
        command.add_argument("--campaign-root", required=True)
        command.add_argument("--repository-class", choices=CLASSES, required=True)
        command.add_argument(
            "--cycle",
            choices=range(1, max(CYCLE_COUNT_BY_REPOSITORY.values()) + 1),
            type=int,
            required=True,
        )
    prepare_reviewer.add_argument("--descriptor", required=True)
    for command in (validate_provisional, record_provisional):
        command.add_argument("--campaign-root", required=True)
        command.add_argument("--candidate-head", required=True)
        command.add_argument("--review-slot-id", required=True)
        command.add_argument("--provisional-review", required=True)
    reveal_profile.add_argument("--campaign-root", required=True)
    reveal_profile.add_argument("--candidate-head", required=True)
    seal.add_argument("--descriptor", required=True)
    collect_w.add_argument("--raw-rollout", required=True)
    collect_r.add_argument("--raw-rollout", required=True)
    activate_every.add_argument("--campaign-root", required=True)
    collect_b.add_argument("--campaign-root", required=True)
    batch_input = collect_b.add_mutually_exclusive_group(required=True)
    batch_input.add_argument("--raw-rollout", action="append")
    batch_input.add_argument("--rollout-directory")
    finalize.add_argument("--campaign-root", required=True)
    package.add_argument("--campaign-root", required=True)
    package.add_argument("--output", required=True)
    package.add_argument("--include-raw-rollouts", action="store_true")
    prepare_review.add_argument("--campaign-root", required=True)
    prepare_review.add_argument("--automated-result", required=True)
    qualify_review.add_argument("--campaign-root", required=True)
    qualify_review.add_argument("--automated-result", required=True)
    qualify_review.add_argument("--human-review", required=True)
    qualify_review.add_argument("--output", required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    root = Path(args.campaign_root).resolve()
    if args.command == "prepare":
        value = prepare_campaign(root, args.campaign_id, args.candidate_head, Path(args.repositories).resolve())
    elif args.command == "prepare-review":
        value = prepare_review(
            root,
            args.repository_class,
            args.cycle,
            Path(args.descriptor),
        )
    elif args.command == "validate-provisional-review":
        value = validate_provisional_review(
            root,
            args.candidate_head,
            args.review_slot_id,
            Path(args.provisional_review),
        )
    elif args.command == "record-provisional-review":
        value = record_provisional_review(
            root,
            args.candidate_head,
            args.review_slot_id,
            Path(args.provisional_review),
        )
    elif args.command == "reveal-qualification-profile":
        value = reveal_qualification_profile(root, args.candidate_head)
    elif args.command == "seal-cycle":
        value = seal_cycle(
            root,
            args.repository_class,
            args.cycle,
            Path(args.descriptor),
        )
    elif args.command == "activate-cycle":
        value = activate_cycle(root, args.repository_class, args.cycle)
    elif args.command == "activate-all":
        value = activate_all(root)
    elif args.command == "collect-work":
        value = collect_work(root, args.repository_class, args.cycle, Path(args.raw_rollout))
    elif args.command == "collect-resume":
        value = collect_resume(root, args.repository_class, args.cycle, Path(args.raw_rollout))
    elif args.command == "collect-batch":
        paths = batch_rollout_paths(
            [Path(path) for path in args.raw_rollout] if args.raw_rollout else None,
            Path(args.rollout_directory) if args.rollout_directory else None,
        )
        value = collect_batch(root, paths)
    elif args.command == "finalize-manifest":
        value = {"manifest": str(finalize_manifest(root))}
    elif args.command == "prepare-human-review":
        value = {
            "human_review": str(
                prepare_human_review(root, Path(args.automated_result).resolve())
            )
        }
    elif args.command == "qualify-review":
        value = {
            "result": str(
                qualify_human_review(
                    root,
                    Path(args.automated_result).resolve(),
                    Path(args.human_review).resolve(),
                    Path(args.output).resolve(),
                )
            )
        }
    else:
        value = {"archive": str(build_review_package(root, Path(args.output), include_raw=args.include_raw_rollouts))}
    print(json.dumps(value, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CampaignError, EvidenceError, OSError, ValueError) as error:
        output = {"status": "failed", "error": str(error)}
        if isinstance(error, CampaignError) and error.diagnostic is not None:
            output["diagnostic"] = error.diagnostic
        print(json.dumps(output, sort_keys=True))
        raise SystemExit(1)
