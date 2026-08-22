#!/usr/bin/env python3
"""Disposable end-to-end checks for the private dogfood campaign helper."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
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
    value.pop("evidence", None)
    source = campaign.evaluator_input_path(root, kind, cycle)
    campaign.write_json(source, value)
    campaign.seal_cycle(root, kind, cycle, source)


def exporter_from(source: Path):
    def export(_binary: Path, _runtime: Path, project_id: str, destination: Path) -> None:
        assert project_id == "01" * 16
        shutil.copyfile(source, destination)
    return export


def documenter(
    _binary: Path,
    _runtime: Path,
    project_id: str,
    kind: str,
    format_name: str,
    destination: Path,
    language: str,
) -> dict[str, object]:
    assert project_id == "01" * 16
    assert kind in campaign.DOCUMENT_KINDS
    assert format_name in {name for name, _suffix in campaign.DOCUMENT_FORMATS}
    assert language == "en"
    destination.write_text(f"{kind} {format_name} {language}\n", encoding="utf-8")
    return {"status": "passed"}


def failed_documenter(
    binary: Path,
    runtime: Path,
    project_id: str,
    kind: str,
    format_name: str,
    destination: Path,
    language: str,
) -> dict[str, object]:
    if kind == "implementation-plan":
        return {"status": "failed", "basis": "fixture document kind unavailable"}
    return documenter(binary, runtime, project_id, kind, format_name, destination, language)


def filtered_capture(source: Path, destination: Path, phrase: str) -> Path:
    lines = [line for line in source.read_text(encoding="utf-8").splitlines() if phrase not in line]
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return destination


def assert_observation_round_trip(root: Path) -> None:
    for scope, names in (
        ("manual", campaign.MANUAL_OBSERVATIONS),
        ("accessibility", campaign.ACCESSIBILITY_OBSERVATIONS),
    ):
        path = campaign.operator_observation_path(root, "volicord", 1, scope)
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


def assert_sealing_and_provenance(parent: Path, binary: Path) -> None:
    root = parent / "sealing-campaign"
    prepare(root, parent / "sealing-sources", binary)
    run_sheet = root / "operator/RUN-SHEET.md"
    initial = run_sheet.read_text(encoding="utf-8")
    assert "descriptor" not in initial.casefold()
    assert not list((root / "cycles").glob("*/descriptor.json"))
    try:
        campaign.activate_cycle(root, "volicord", 1)
    except campaign.CampaignError as error:
        assert "sealed evaluator descriptor" in str(error)
    else:
        raise AssertionError("unsealed cycle activated")
    try:
        campaign.collect_work(root, "volicord", 1, parent / "absent-rollout.jsonl")
    except campaign.CampaignError as error:
        assert "sealed evaluator descriptor" in str(error)
    else:
        raise AssertionError("unsealed cycle accepted work collection")

    descriptor, _work, _resume, _bundle = fixture_for(parent, "volicord", 1)
    descriptor.pop("_evidence_directory", None)
    descriptor.pop("_evidence_file_sha256", None)
    descriptor.pop("evidence", None)
    leaked = copy.deepcopy(descriptor)
    leaked["materiality_review"]["independent_review"]["basis"] = leaked["work_user_task"]
    leaked_path = parent / "leaked-evaluator-input.json"
    campaign.write_json(leaked_path, leaked)
    try:
        campaign.seal_cycle(root, "volicord", 1, leaked_path)
    except campaign.CampaignError as error:
        assert "evaluator-only material" in str(error)
    else:
        raise AssertionError("evaluator material entered the operator run sheet")

    target = parent / "pinned-target"
    target.mkdir()
    subprocess.run(["git", "init", "--quiet", str(target)], check=True)
    subprocess.run(["git", "-C", str(target), "config", "user.email", "fixture@example.invalid"], check=True)
    subprocess.run(["git", "-C", str(target), "config", "user.name", "Fixture"], check=True)
    contract = target / "CONTRACT.md"
    contract.write_text("pinned target contract\n", encoding="utf-8")
    subprocess.run(["git", "-C", str(target), "add", "CONTRACT.md"], check=True)
    subprocess.run(["git", "-C", str(target), "commit", "--quiet", "-m", "fixture"], check=True)
    target_revision = subprocess.run(
        ["git", "-C", str(target), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    candidate_revision = harness.git_head(campaign.ROOT)
    assert candidate_revision is not None
    base = copy.deepcopy(descriptor)
    base["repository_revision"] = target_revision
    owner = base["materiality_review"]["provenance_references"][0]
    target_reference = {
        "scope": "target_repository",
        "path": "CONTRACT.md",
        "sha256": hashlib.sha256(b"pinned target contract\n").hexdigest(),
        "repository_revision": target_revision,
    }
    base["materiality_review"]["provenance_references"].append(target_reference)
    assert not harness.cycle_descriptor_errors(
        base,
        candidate_revision=candidate_revision,
        target_repository=target,
        verify_provenance=True,
    )

    invalid_cases = []
    nonexistent_owner = copy.deepcopy(base)
    nonexistent_owner["materiality_review"]["provenance_references"][0]["path"] = "rebuild/docs/design/not-an-owner.md"
    invalid_cases.append(("current active architecture owner", nonexistent_owner))
    inactive_owner = copy.deepcopy(base)
    inactive_owner["materiality_review"]["provenance_references"][0]["path"] = "rebuild/docs/design/product-charter.md"
    inactive_owner["materiality_review"]["provenance_references"][0]["sha256"] = harness.sha256(campaign.ROOT / "rebuild/docs/design/product-charter.md")
    invalid_cases.append(("current active architecture owner", inactive_owner))
    stale_owner = copy.deepcopy(base)
    stale_owner["materiality_review"]["provenance_references"][0]["sha256"] = "00" * 32
    invalid_cases.append(("stale", stale_owner))
    traversal = copy.deepcopy(base)
    traversal["materiality_review"]["provenance_references"][1]["path"] = "../CONTRACT.md"
    invalid_cases.append(("safe relative path", traversal))
    missing = copy.deepcopy(base)
    missing["materiality_review"]["provenance_references"][1]["path"] = "MISSING.md"
    invalid_cases.append(("does not exist", missing))
    stale_target = copy.deepcopy(base)
    stale_target["materiality_review"]["provenance_references"][1]["sha256"] = "11" * 32
    invalid_cases.append(("stale", stale_target))
    wrong_revision = copy.deepcopy(base)
    wrong_revision["materiality_review"]["provenance_references"][1]["repository_revision"] = "22" * 20
    invalid_cases.append(("wrong pinned target revision", wrong_revision))
    for expected, value in invalid_cases:
        errors = harness.cycle_descriptor_errors(
            value,
            candidate_revision=candidate_revision,
            target_repository=target,
            verify_provenance=True,
        )
        assert any(expected in error for error in errors), (expected, errors)


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
            document_evidence = resume_result["document_evidence"]
            assert document_evidence["status"] == "passed"
            assert set(document_evidence["documents"]) == set(campaign.DOCUMENT_KINDS)
            for document_kind in campaign.DOCUMENT_KINDS:
                formats = document_evidence["documents"][document_kind]["formats"]
                assert set(formats) == {name for name, _suffix in campaign.DOCUMENT_FORMATS}
                for evidence in formats.values():
                    path = root / evidence["relative_evidence_path"]
                    assert evidence["status"] == "passed"
                    assert evidence["bytes"] == path.stat().st_size
                    assert evidence["sha256"] == harness.sha256(path)
            campaign.record_observation(
                root,
                kind,
                cycle,
                "manual",
                "document_fidelity_and_usefulness",
                "passed",
                "reviewed all four generated document kinds",
            )
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
        assert len([name for name in names if name.startswith("evaluator/descriptors/")]) == 6
        assert len([name for name in names if name.startswith("materiality-reviews/") and name.endswith(".json")]) == 7
        assert len([name for name in names if "/evidence/generated-documents/" in name]) == 48
        assert len([name for name in names if name.endswith("/documents-summary.json")]) == 6
        assert len([name for name in names if name.startswith("operator/document-review/")]) == 6
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


def assert_failed_document_kind_blocks_pass(parent: Path, binary: Path) -> None:
    root = parent / "failed-document-campaign"
    prepare(root, parent / "failed-document-sources", binary)
    descriptor, work, resume, bundle = fixture_for(parent, "volicord", 1)
    install_descriptor(root, "volicord", 1, descriptor)
    assert campaign.collect_work(root, "volicord", 1, work)["outcome"] == "resume_allowed"
    result = campaign.collect_resume(
        root,
        "volicord",
        1,
        resume,
        exporter=exporter_from(bundle),
        documenter=failed_documenter,
    )
    failed = result["document_evidence"]["documents"]["implementation-plan"]
    assert result["document_evidence"]["status"] == "failed"
    assert failed["status"] == "failed"
    assert all(item["status"] == "failed" for item in failed["formats"].values())
    try:
        campaign.record_observation(
            root,
            "volicord",
            1,
            "manual",
            "document_fidelity_and_usefulness",
            "passed",
            "reviewed all required documents",
        )
    except campaign.CampaignError as error:
        assert "all four document kinds" in str(error)
    else:
        raise AssertionError("failed document evidence allowed a passed fidelity observation")


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="volicord-dogfood-campaign-") as temporary:
        parent = Path(temporary)
        binary = parent / "candidate/bin/volicord"
        write_fake_binary(binary)
        assert_sealing_and_provenance(parent, binary)
        assert_blockers(parent, binary)
        assert_failed_document_kind_blocks_pass(parent, binary)
        assert_successful_campaign(parent, binary)
    print(json.dumps({
        "status": "passed",
        "checks": [
            "observation_schema_round_trip",
            "sealed_evaluator_operator_isolation",
            "typed_materiality_provenance_verification",
            "terminal_work_blocker_stops_collection",
            "missing_activation_operator_environment_invalid",
            "automatic_project_identity_and_bundle_export",
            "four_kind_markdown_html_document_evidence",
            "failed_document_kind_blocks_fidelity_pass",
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
