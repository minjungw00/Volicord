#!/usr/bin/env python3
"""Disposable end-to-end checks for the private dogfood campaign helper."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import shutil
import tarfile
import tempfile

import campaign
import harness


REVISION = "ab" * 20


def write_fake_binary(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    path.chmod(0o755)


def repository_input(path: Path, sources: Path) -> Path:
    repositories = []
    for kind in campaign.CLASSES:
        repository = sources / kind
        repository.mkdir(parents=True)
        (repository / "LICENSE").write_text("fixture license\n", encoding="utf-8")
        repositories.append({
            "class": kind,
            "path": str(repository),
            "origin": f"https://example.invalid/{kind}.git",
            "revision": REVISION,
            "license_file": "LICENSE",
            "license_spdx": "MIT",
            "provider_source_path": "src/provider.fixture",
        })
    path.write_text(json.dumps({"repositories": repositories}), encoding="utf-8")
    return path


def fake_identities() -> list[dict[str, object]]:
    return [
        {
            "class": kind,
            "status": "passed",
            "origin": f"https://example.invalid/{kind}.git",
            "revision": REVISION,
            "license": {"spdx": "MIT", "file": "LICENSE", "sha256": "00" * 32},
            "file_count": 120,
            "documentation_file_count": 1,
            "official_structural_languages": ["Python", "Rust", "JavaScript"],
            "blockers": [],
        }
        for kind in campaign.CLASSES
    ]


def prepare(root: Path, source_root: Path, binary: Path) -> None:
    source = repository_input(root.parent / f"{root.name}-repositories.json", source_root)
    original_clean = harness.git_clean
    original_specs = harness.load_repository_specs
    harness.git_clean = lambda _path: True
    harness.load_repository_specs = lambda _path, _head, _definition: (
        campaign.repository_spec_map(json.loads(source.read_text(encoding="utf-8"))),
        fake_identities(),
    )
    try:
        def fake_clone(_source: Path, destination: Path, _revision: str) -> None:
            destination.mkdir(parents=True)

        campaign.prepare_campaign(
            root,
            root.name,
            harness.git_head(campaign.ROOT),
            source,
            candidate_binary=binary,
            enable=False,
            cloner=fake_clone,
        )
    finally:
        harness.git_clean = original_clean
        harness.load_repository_specs = original_specs


def fixture_for(root: Path, kind: str, cycle: int) -> tuple[dict[str, object], Path, Path, Path]:
    fixture_root = root / "fixture-source" / f"{kind}-{cycle}"
    fixture_root.mkdir(parents=True, exist_ok=True)
    revision = harness.git_head(campaign.ROOT) if kind == "volicord" else REVISION
    assert revision is not None
    descriptor = harness.real_session_fixture(kind, cycle, revision, fixture_root)
    work = fixture_root / f"{kind}-{cycle}-work-events.jsonl"
    resume = fixture_root / f"{kind}-{cycle}-resume-events.jsonl"
    bundle = fixture_root / f"{kind}-{cycle}-context.bundle.json"
    return descriptor, work, resume, bundle


def install_descriptor(root: Path, kind: str, cycle: int, descriptor: dict[str, object]) -> None:
    value = copy.deepcopy(descriptor)
    value.pop("_evidence_directory", None)
    value.pop("_evidence_file_sha256", None)
    campaign.write_json(campaign.cycle_root(root, kind, cycle) / "descriptor.json", value)


def exporter_from(source: Path):
    def export(_binary: Path, _runtime: Path, project_id: str, destination: Path) -> None:
        assert project_id == "01" * 16
        shutil.copyfile(source, destination)
    return export


def documenter(_binary: Path, _runtime: Path, project_id: str, destination: Path) -> dict[str, object]:
    assert project_id == "01" * 16
    destination.write_text("<!doctype html><html lang='en'><title>Architecture</title></html>\n", encoding="utf-8")
    return {
        "status": "passed",
        "file": destination.name,
        "bytes": destination.stat().st_size,
        "sha256": harness.sha256(destination),
    }


def filtered_capture(source: Path, destination: Path, phrase: str) -> Path:
    lines = [line for line in source.read_text(encoding="utf-8").splitlines() if phrase not in line]
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return destination


def assert_observation_round_trip(root: Path) -> None:
    for scope, names in (
        ("manual", campaign.MANUAL_OBSERVATIONS),
        ("accessibility", campaign.ACCESSIBILITY_OBSERVATIONS),
    ):
        path = campaign.cycle_root(root, "volicord", 1) / f"observations/{scope}.json"
        template = campaign.read_json(path)
        assert set(template) == set(names)
        for name, value in template.items():
            assert harness.validate_observation_object(value, name) == (
                "skipped",
                "Not yet observed; replace with a bounded operator observation.",
            )
            recorded = campaign.record_observation(
                root, "volicord", 1, scope, name, "partial", f"bounded review for {name}"
            )
            assert harness.validate_observation_object(recorded, name)[0] == "partial"


def assert_blockers(parent: Path, binary: Path) -> None:
    blocker_root = parent / "blocker-campaign"
    prepare(blocker_root, parent / "blocker-sources", binary)
    descriptor, work, resume, _bundle = fixture_for(parent, "volicord", 1)
    install_descriptor(blocker_root, "volicord", 1, descriptor)
    broken = filtered_capture(work, parent / "missing-completions.jsonl", '"type":"mcp_tool_call_end"')
    result = campaign.collect_work(blocker_root, "volicord", 1, broken)
    assert result["outcome"] == "campaign_stop"
    try:
        campaign.collect_resume(blocker_root, "volicord", 1, resume)
    except campaign.CampaignError as error:
        assert "new campaign identity" in str(error)
    else:
        raise AssertionError("terminal work blocker did not stop resume collection")

    activation_root = parent / "activation-campaign"
    prepare(activation_root, parent / "activation-sources", binary)
    install_descriptor(activation_root, "volicord", 1, descriptor)
    missing = filtered_capture(work, parent / "missing-activation.jsonl", "Volicord is active because")
    invalid = campaign.collect_work(activation_root, "volicord", 1, missing)
    assert invalid["outcome"] == "operator_environment_invalid"
    assert invalid["classification"] == "operator_environment_setup_failure"


def assert_successful_campaign(parent: Path, binary: Path) -> None:
    root = parent / "successful-campaign"
    prepare(root, parent / "successful-sources", binary)
    assert_observation_round_trip(root)
    fixtures: dict[tuple[str, int], tuple[dict[str, object], Path, Path, Path]] = {}
    for kind in campaign.CLASSES:
        for cycle in (1, 2):
            descriptor, work, resume, bundle = fixture_for(parent, kind, cycle)
            fixtures[(kind, cycle)] = (descriptor, work, resume, bundle)
            install_descriptor(root, kind, cycle, descriptor)
            runtime = campaign.cycle_root(root, kind, cycle) / "runtime"
            (runtime / "canonical.sqlite3").write_bytes(b"PRIVATE-STORE-CONTENT")
            derived = runtime / "derived/analysis/project"
            derived.mkdir(parents=True)
            (derived / "private-analysis.json").write_bytes(b"PRIVATE-DERIVED-CONTENT")
            work_result = campaign.collect_work(root, kind, cycle, work)
            assert work_result["outcome"] == "resume_allowed"
            resume_result = campaign.collect_resume(
                root,
                kind,
                cycle,
                resume,
                exporter=exporter_from(bundle),
                documenter=documenter,
            )
            assert resume_result["project_id"] == "01" * 16
            assert resume_result["descriptor_evidence_completed"] is True
            summary_bytes = (campaign.cycle_root(root, kind, cycle) / "runtime-summary.json").read_bytes()
            assert b"canonical.sqlite3" in summary_bytes
            assert b"PRIVATE-STORE-CONTENT" not in summary_bytes
            assert b"PRIVATE-DERIVED-CONTENT" not in summary_bytes

    manifest = campaign.finalize_manifest(root)
    first = manifest.read_bytes()
    campaign.finalize_manifest(root)
    assert manifest.read_bytes() == first

    tampered = campaign.cycle_root(root, "small-python", 2) / "context.bundle.json"
    original = tampered.read_bytes()
    tampered.write_bytes(original + b"tamper")
    try:
        campaign.build_review_package(root, parent / "must-not-exist.tar.gz")
    except campaign.CampaignError as error:
        assert "hash mismatch" in str(error)
    else:
        raise AssertionError("tampered bundle was not detected")
    tampered.write_bytes(original)

    archive = campaign.build_review_package(root, parent / "review.tar.gz")
    with tarfile.open(archive, "r:gz") as opened:
        names = opened.getnames()
        assert len([name for name in names if name.endswith("/descriptor.json")]) == 6
        assert len([name for name in names if name.startswith("materiality-reviews/") and name.endswith(".json")]) == 7
        assert not any(Path(name).name in campaign.RAW_NAMES for name in names)
        assert not any(any(part in {"runtime", "install", "bootstrap-runtime", "derived"} for part in Path(name).parts) for name in names)
        assert not any(name.casefold().endswith(campaign.PROHIBITED_ARCHIVE_SUFFIXES) for name in names)
        body = b"".join(
            file.read()
            for member in opened.getmembers()
            if member.isfile() and (file := opened.extractfile(member)) is not None
        )
        assert b"PRIVATE-STORE-CONTENT" not in body
        assert b"PRIVATE-DERIVED-CONTENT" not in body

    raw_archive = campaign.build_review_package(
        root, parent / "review-with-raw.tar.gz", include_raw=True
    )
    with tarfile.open(raw_archive, "r:gz") as opened:
        assert len([name for name in opened.getnames() if Path(name).name in campaign.RAW_NAMES]) == 12


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="volicord-dogfood-campaign-") as temporary:
        parent = Path(temporary)
        binary = parent / "candidate/bin/volicord"
        write_fake_binary(binary)
        assert_blockers(parent, binary)
        assert_successful_campaign(parent, binary)
    print(json.dumps({
        "status": "passed",
        "checks": [
            "observation_schema_round_trip",
            "terminal_work_blocker_stops_collection",
            "missing_activation_operator_environment_invalid",
            "automatic_project_identity_and_bundle_export",
            "bounded_runtime_summary",
            "deterministic_manifest",
            "bounded_default_review_archive",
            "explicit_raw_rollout_archive_option",
            "evidence_hash_tamper_detection",
        ],
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
