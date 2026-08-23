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
STRICT_FAKE = campaign.ROOT / "rebuild/validation/shared/strict_fake_volicord.py"


def write_fake_binary(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(STRICT_FAKE, path)
    path.chmod(0o755)
    viewer = path.with_name("volicord-viewer")
    viewer.write_text(
        "#!/bin/sh\n"
        "destination=\n"
        "for argument in \"$@\"; do destination=$argument; done\n"
        "printf '%s\\n' '<!doctype html><html lang=\"en\"><body data-viewer-mode=\"snapshot\">fixture</body></html>' > \"$destination\"\n",
        encoding="utf-8",
    )
    viewer.chmod(0o755)


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


def fixture_for(
    root: Path,
    kind: str,
    cycle: int,
    campaign_root: Path | None = None,
) -> tuple[dict[str, object], Path, Path, Path]:
    fixture_root = root / "fixture-source" / f"{kind}-{cycle}"
    fixture_root.mkdir(parents=True, exist_ok=True)
    revision = harness.git_head(campaign.ROOT) if kind == "volicord" else REVISION
    assert revision is not None
    repository = (
        campaign.cycle_root(campaign_root, kind, cycle) / "repository"
        if campaign_root is not None
        else None
    )
    descriptor = harness.real_session_fixture(
        kind,
        cycle,
        revision,
        fixture_root,
        repository_path=repository,
    )
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
    def export(_binary: Path, _runtime: Path, repository: Path, destination: Path) -> None:
        assert repository.name == "repository"
        shutil.copyfile(source, destination)
    return export


def documenter(
    _binary: Path,
    _runtime: Path,
    repository: Path,
    kind: str,
    format_name: str,
    destination: Path,
    language: str,
) -> dict[str, object]:
    assert repository.name == "repository"
    assert kind in campaign.DOCUMENT_KINDS
    assert format_name in {name for name, _suffix in campaign.DOCUMENT_FORMATS}
    assert language == "en"
    destination.write_text(f"{kind} {format_name} {language}\n", encoding="utf-8")
    return {"status": "passed"}


def failed_documenter(
    binary: Path,
    runtime: Path,
    repository: Path,
    kind: str,
    format_name: str,
    destination: Path,
    language: str,
) -> dict[str, object]:
    if kind == "implementation-plan":
        return {"status": "failed", "basis": "fixture document kind unavailable"}
    return documenter(binary, runtime, repository, kind, format_name, destination, language)


def snapshotter(
    _binary: Path,
    _runtime: Path,
    project_id: str,
    destination: Path,
    locale: str,
    language: str,
) -> dict[str, object]:
    assert project_id == "01" * 16
    assert locale == "en"
    assert language == "en"
    destination.write_text(
        '<!doctype html><html lang="en"><body data-viewer-mode="snapshot">fixture</body></html>\n',
        encoding="utf-8",
    )
    return {"status": "passed"}


def filtered_capture(source: Path, destination: Path, phrase: str) -> Path:
    lines = [line for line in source.read_text(encoding="utf-8").splitlines() if phrase not in line]
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return destination


def replaced_capture(source: Path, destination: Path, old: str, new: str) -> Path:
    text = source.read_text(encoding="utf-8")
    if old not in text:
        raise AssertionError(f"capture replacement source was absent: {old}")
    destination.write_text(text.replace(old, new), encoding="utf-8")
    return destination


def prepared_batch(
    parent: Path,
    name: str,
    binary: Path,
) -> tuple[Path, list[Path], dict[str, Path]]:
    root = parent / name
    prepare(root, parent / f"{name}-sources", binary)
    captures: list[Path] = []
    bundles: dict[str, Path] = {}
    for kind in campaign.CLASSES:
        for cycle in range(1, len(campaign.BEHAVIOR_CLASSES) + 1):
            descriptor, work, resume, bundle = fixture_for(
                parent / f"{name}-fixtures",
                kind,
                cycle,
                campaign_root=root,
            )
            install_descriptor(root, kind, cycle, descriptor)
            captures.extend((work, resume))
            bundles[campaign.cycle_key(kind, cycle)] = bundle
    return root, captures, bundles


def batch_exporter(bundles: dict[str, Path]):
    def export(_binary: Path, _runtime: Path, repository: Path, destination: Path) -> None:
        assert repository.name == "repository"
        shutil.copyfile(bundles[destination.parent.name], destination)

    return export


def assert_strict_cli_contract(parent: Path, binary: Path) -> None:
    runtime = parent / "strict-runtime"
    repository = parent / "strict-repository"
    repository.mkdir()
    current = [
        "--runtime", str(runtime), "--repository", str(repository),
        "codex", "enable",
    ]
    accepted = subprocess.run([str(binary), *current], text=True, capture_output=True, check=False)
    assert accepted.returncode == 0
    assert json.loads(accepted.stdout)["project_trust"] == "user_controlled"
    rejected = (
        ["portable", "export", "11" * 16, str(parent / "old.bundle")],
        ["documents", "export", "11" * 16, "handoff-resume", "markdown", str(parent / "old.md"), "en"],
        ["repair", "11" * 16, "derived-analysis"],
        ["reindex", "11" * 16],
        ["codex", "enable", str(repository)],
        ["codex", "--repository", str(repository), "enable"],
        ["unexpected"],
    )
    for argv in rejected:
        result = subprocess.run(
            [str(binary), "--runtime", str(runtime), *argv],
            text=True,
            capture_output=True,
            check=False,
        )
        assert result.returncode == 2, (argv, result.returncode, result.stdout, result.stderr)


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
    leaked["behavior_review"]["independent_review"]["basis"] = leaked["work_user_task"]
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
    owner = base["behavior_review"]["provenance_references"][0]
    target_reference = {
        "scope": "target_repository",
        "path": "CONTRACT.md",
        "sha256": hashlib.sha256(b"pinned target contract\n").hexdigest(),
        "repository_revision": target_revision,
    }
    base["behavior_review"]["provenance_references"].append(target_reference)
    assert not harness.cycle_descriptor_errors(
        base,
        candidate_revision=candidate_revision,
        target_repository=target,
        verify_provenance=True,
    )

    invalid_cases = []
    nonexistent_owner = copy.deepcopy(base)
    nonexistent_owner["behavior_review"]["provenance_references"][0]["path"] = "rebuild/docs/design/not-an-owner.md"
    invalid_cases.append(("current active architecture owner", nonexistent_owner))
    inactive_owner = copy.deepcopy(base)
    inactive_owner["behavior_review"]["provenance_references"][0]["path"] = "rebuild/docs/design/product-charter.md"
    inactive_owner["behavior_review"]["provenance_references"][0]["sha256"] = harness.sha256(campaign.ROOT / "rebuild/docs/design/product-charter.md")
    invalid_cases.append(("current active architecture owner", inactive_owner))
    stale_owner = copy.deepcopy(base)
    stale_owner["behavior_review"]["provenance_references"][0]["sha256"] = "00" * 32
    invalid_cases.append(("stale", stale_owner))
    traversal = copy.deepcopy(base)
    traversal["behavior_review"]["provenance_references"][1]["path"] = "../CONTRACT.md"
    invalid_cases.append(("safe relative path", traversal))
    missing = copy.deepcopy(base)
    missing["behavior_review"]["provenance_references"][1]["path"] = "MISSING.md"
    invalid_cases.append(("does not exist", missing))
    stale_target = copy.deepcopy(base)
    stale_target["behavior_review"]["provenance_references"][1]["sha256"] = "11" * 32
    invalid_cases.append(("stale", stale_target))
    wrong_revision = copy.deepcopy(base)
    wrong_revision["behavior_review"]["provenance_references"][1]["repository_revision"] = "22" * 20
    invalid_cases.append(("wrong pinned target revision", wrong_revision))
    for expected, value in invalid_cases:
        errors = harness.cycle_descriptor_errors(
            value,
            candidate_revision=candidate_revision,
            target_repository=target,
            verify_provenance=True,
        )
        assert any(expected in error for error in errors), (expected, errors)


def assert_batch_workflow(parent: Path, binary: Path) -> None:
    root, captures, bundles = prepared_batch(parent, "batch-campaign", binary)
    run_sheet = (root / "operator/RUN-SHEET.md").read_text(encoding="utf-8")
    assert "collect-batch" in run_sheet
    assert "collect-work" not in run_sheet
    assert "collect-resume" not in run_sheet

    mapped = campaign.map_batch_rollouts(root, list(reversed(captures)))
    assert len(mapped) == campaign.BATCH_CAPTURE_COUNT
    assert len({capture.session_id for _path, capture in mapped.values()}) == 24

    directory = parent / "unordered-rollouts"
    directory.mkdir()
    for index, source in enumerate(reversed(captures)):
        shutil.copyfile(source, directory / f"capture-{index:02}.jsonl")
    directory_paths = campaign.batch_rollout_paths(None, directory)
    assert len(campaign.map_batch_rollouts(root, directory_paths)) == 24
    try:
        campaign.map_batch_rollouts(root, captures[:-1])
    except campaign.CampaignError as error:
        assert "missing" in str(error)
    else:
        raise AssertionError("batch mapping accepted a missing rollout")

    work = next(path for path in captures if path.name == "volicord-1-work-events.jsonl")
    resume = next(path for path in captures if path.name == "volicord-1-resume-events.jsonl")
    duplicate_session = replaced_capture(
        resume,
        parent / "duplicate-session.jsonl",
        "volicord-resume-session-1",
        "volicord-work-session-1",
    )
    duplicate_inputs = [duplicate_session if path == resume else path for path in captures]
    try:
        campaign.map_batch_rollouts(root, duplicate_inputs)
    except campaign.CampaignError as error:
        assert "session identity" in str(error)
    else:
        raise AssertionError("batch mapping accepted a reused session identity")

    state = campaign.load_campaign(root)["cycles"][campaign.cycle_key("volicord", 1)]
    descriptor = campaign.read_json(campaign.evaluator_descriptor_path(root, "volicord", 1))
    wrong_cases = (
        (
            "task",
            descriptor["work_user_task"],
            descriptor["work_user_task"] + " changed",
        ),
        ("revision", state["repository_revision"], "cd" * 20),
        ("workspace", state["repository_path"], str(parent / "wrong-workspace")),
    )
    for label, old, new in wrong_cases:
        invalid = replaced_capture(work, parent / f"wrong-{label}.jsonl", old, new)
        inputs = [invalid if path == work else path for path in captures]
        try:
            campaign.map_batch_rollouts(root, inputs)
        except campaign.CampaignError as error:
            assert "unambiguously" in str(error)
        else:
            raise AssertionError(f"batch mapping accepted wrong {label}")

    original_run_checked = campaign.run_checked
    campaign.run_checked = lambda _argv, cwd=campaign.ROOT: {
        "project_trust": "user_controlled"
    }
    try:
        activation = campaign.activate_all(root)
    finally:
        campaign.run_checked = original_run_checked
    assert activation["cycle_count"] == 12
    assert activation["repository_and_hook_trust"] == "user_controlled_not_automated"

    for kind in campaign.CLASSES:
        for cycle in range(1, len(campaign.BEHAVIOR_CLASSES) + 1):
            runtime = campaign.cycle_root(root, kind, cycle) / "runtime"
            (runtime / "canonical.sqlite3").write_bytes(b"BATCH-PRIVATE-STORE")
            derived = runtime / "derived/analysis/private"
            derived.mkdir(parents=True)
            (derived / "private.json").write_bytes(b"BATCH-PRIVATE-DERIVED")
    summary = campaign.collect_batch(
        root,
        list(reversed(captures)),
        exporter=batch_exporter(bundles),
        documenter=documenter,
    )
    assert summary["status"] == "passed", summary
    assert summary["outcome"] == "evidence_collected"
    assert summary["session_distinctness"] == {
        "status": "passed",
        "expected_count": 24,
        "observed_count": 24,
    }
    assert len(summary["cycles"]) == 12
    for item in summary["cycles"]:
        assert item["supported_evidence_complete"] is True
        assert item["terminal_work_failure_preserved"] is False
        cycle_path = campaign.cycle_root(root, item["repository_class"], item["cycle"])
        descriptor_value = campaign.read_json(
            campaign.evaluator_descriptor_path(root, item["repository_class"], item["cycle"])
        )
        assert set(descriptor_value["evidence"]) == {
            "captures",
            "canonical_bundle",
            "runtime_summary",
            "activation_summary",
            "generated_documents",
            "viewer_snapshot",
        }
        snapshot = campaign.read_json(cycle_path / "viewer-snapshot-summary.json")
        assert snapshot["status"] == "passed"
        assert snapshot["project_id"] == "01" * 16
        assert snapshot["candidate_head"] == harness.git_head(campaign.ROOT)
        raw_source = next(
            source
            for source in captures
            if source.name
            == f"{item['repository_class']}-{item['cycle']}-work-events.jsonl"
        )
        assert (cycle_path / "evidence/work.rollout.jsonl").read_bytes() == raw_source.read_bytes()

    campaign.finalize_manifest(root)
    archive = campaign.build_review_package(root, parent / "batch-review.tar.gz")
    with tarfile.open(archive, "r:gz") as opened:
        names = opened.getnames()
        assert "batch-intake-summary.json" in names
        assert len([name for name in names if name.endswith("/evidence/viewer-snapshot.html")]) == 12
        assert not any(Path(name).name in campaign.RAW_NAMES for name in names)
        assert not any(
            any(part in {"runtime", "install", "bootstrap-runtime", "derived"} for part in Path(name).parts)
            for name in names
        )
        body = b"".join(
            file.read()
            for member in opened.getmembers()
            if member.isfile() and (file := opened.extractfile(member)) is not None
        )
        assert b"BATCH-PRIVATE-STORE" not in body
        assert b"BATCH-PRIVATE-DERIVED" not in body

    tampered = campaign.cycle_root(root, "small-python", 2) / "evidence/viewer-snapshot.html"
    tampered.write_bytes(tampered.read_bytes() + b"tamper")
    try:
        campaign.build_review_package(root, parent / "batch-tampered.tar.gz")
    except campaign.CampaignError as error:
        assert "hash mismatch" in str(error)
    else:
        raise AssertionError("tampered batch Viewer evidence was not detected")

    blocker_root, blocker_captures, blocker_bundles = prepared_batch(
        parent,
        "batch-blocker-campaign",
        binary,
    )
    blocker_work = next(
        path for path in blocker_captures if path.name == "volicord-1-work-events.jsonl"
    )
    blocked = filtered_capture(
        blocker_work,
        parent / "batch-blocked-work.jsonl",
        '"type":"mcp_tool_call_end"',
    )
    blocker_inputs = [blocked if path == blocker_work else path for path in blocker_captures]
    blocker_summary = campaign.collect_batch(
        blocker_root,
        blocker_inputs,
        exporter=batch_exporter(blocker_bundles),
        documenter=documenter,
    )
    blocked_cycle = blocker_summary["cycles"][0]
    assert blocker_summary["outcome"] == "campaign_stop"
    assert blocked_cycle["terminal_work_failure_preserved"] is True
    assert (
        campaign.cycle_root(blocker_root, "volicord", 1)
        / "evidence/resume.rollout.jsonl"
    ).is_file()

    activation_root, activation_captures, activation_bundles = prepared_batch(
        parent,
        "batch-activation-campaign",
        binary,
    )
    activation_work = next(
        path for path in activation_captures if path.name == "volicord-1-work-events.jsonl"
    )
    missing_activation = filtered_capture(
        activation_work,
        parent / "batch-missing-activation.jsonl",
        "Volicord is active because",
    )
    activation_inputs = [
        missing_activation if path == activation_work else path
        for path in activation_captures
    ]
    activation_summary = campaign.collect_batch(
        activation_root,
        activation_inputs,
        exporter=batch_exporter(activation_bundles),
        documenter=documenter,
    )
    assert activation_summary["outcome"] == "operator_environment_invalid"
    activation_blocker = campaign.read_json(
        campaign.cycle_root(activation_root, "volicord", 1) / "blocker-result.json"
    )
    assert activation_blocker["classification"] == "operator_environment_setup_failure"


def assert_successful_campaign(parent: Path, binary: Path) -> None:
    root = parent / "successful-campaign"
    prepare(root, parent / "successful-sources", binary)
    fixtures: dict[tuple[str, int], tuple[dict[str, object], Path, Path, Path]] = {}
    for kind in campaign.CLASSES:
        for cycle in range(1, len(campaign.BEHAVIOR_CLASSES) + 1):
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
                snapshotter=snapshotter,
            )
            assert resume_result["project_id"] == "01" * 16
            assert resume_result["descriptor_evidence_completed"] is True
            assert resume_result["viewer_snapshot_evidence"]["status"] == "passed"
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

    review_path = root / "operator/human-review.json"
    campaign.write_json(review_path, {
        "kind": "phase8_dogfood_human_review",
        "automated_result_sha256": "cd" * 32,
        "state": "passed",
    })
    campaign.register_artifact(root, review_path)
    qualified_path = root / "qualified-result.json"
    campaign.write_json(qualified_path, {
        "kind": "phase8_dogfood_result",
        "human_review": {"state": "passed"},
        "replacement_qualification": {"status": "passed"},
    })
    campaign.register_artifact(root, qualified_path)

    archive = campaign.build_review_package(root, parent / "review.tar.gz")
    with tarfile.open(archive, "r:gz") as opened:
        names = opened.getnames()
        assert len([name for name in names if name.startswith("evaluator/descriptors/")]) == 12
        assert len([name for name in names if name.startswith("behavior-reviews/") and name.endswith(".json")]) == 13
        assert len([name for name in names if "/evidence/generated-documents/" in name]) == 96
        assert len([name for name in names if name.endswith("/evidence/viewer-snapshot.html")]) == 12
        assert len([name for name in names if name.endswith("/viewer-snapshot-summary.json")]) == 12
        assert len([name for name in names if name.endswith("/documents-summary.json")]) == 12
        assert len([name for name in names if name.startswith("operator/document-review/")]) == 12
        assert "operator/human-review.json" in names
        assert "qualified-result.json" in names
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
        assert len([name for name in opened.getnames() if Path(name).name in campaign.RAW_NAMES]) == 24


def assert_failed_document_kind_is_machine_failure(parent: Path, binary: Path) -> None:
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
        snapshotter=snapshotter,
    )
    failed = result["document_evidence"]["documents"]["implementation-plan"]
    assert result["document_evidence"]["status"] == "failed"
    assert failed["status"] == "failed"
    assert all(item["status"] == "failed" for item in failed["formats"].values())


def assert_campaign_level_human_review_operations(parent: Path, binary: Path) -> None:
    root = parent / "human-review-campaign"
    prepare(root, parent / "human-review-sources", binary)
    automated = parent / "automated-result.json"
    automated.write_bytes(b'{"kind":"fixture-automated-result"}\n')
    expected_sha = hashlib.sha256(automated.read_bytes()).hexdigest()
    original_template = harness.human_review_template
    original_combine = harness.combine_human_review

    def fake_template(result: dict[str, object], result_sha: str) -> dict[str, object]:
        assert result == {"kind": "fixture-automated-result"}
        assert result_sha == expected_sha
        return {
            "kind": "phase8_dogfood_human_review",
            "automated_result_sha256": result_sha,
            "status": "not_provided",
        }

    def fake_combine(
        result: dict[str, object],
        review: dict[str, object],
        result_sha: str,
    ) -> dict[str, object]:
        assert result == {"kind": "fixture-automated-result"}
        assert review["status"] == "passed"
        assert result_sha == expected_sha
        return {
            "kind": "phase8_dogfood_result",
            "automated_result_sha256": result_sha,
            "replacement_qualification": {"status": "passed"},
        }

    harness.human_review_template = fake_template
    harness.combine_human_review = fake_combine
    try:
        review_path = campaign.prepare_human_review(root, automated)
        review = campaign.read_json(review_path)
        assert review["automated_result_sha256"] == expected_sha
        review["status"] = "passed"
        campaign.write_json(review_path, review)
        qualified_path = campaign.qualify_human_review(
            root,
            automated,
            review_path,
            root / "qualified-result.json",
        )
        assert campaign.read_json(qualified_path)["automated_result_sha256"] == expected_sha
        assert automated.read_bytes() == b'{"kind":"fixture-automated-result"}\n'
    finally:
        harness.human_review_template = original_template
        harness.combine_human_review = original_combine


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="volicord-dogfood-campaign-") as temporary:
        parent = Path(temporary)
        binary = parent / "candidate/bin/volicord"
        write_fake_binary(binary)
        assert_strict_cli_contract(parent, binary)
        assert_sealing_and_provenance(parent, binary)
        assert_blockers(parent, binary)
        assert_batch_workflow(parent, binary)
        assert_failed_document_kind_is_machine_failure(parent, binary)
        assert_campaign_level_human_review_operations(parent, binary)
        assert_successful_campaign(parent, binary)
    print(json.dumps({
        "status": "passed",
        "checks": [
            "campaign_level_human_review_operations",
            "strict_current_cli_positive_and_obsolete_negative_cases",
            "sealed_evaluator_operator_isolation",
            "typed_behavior_review_provenance_verification",
            "terminal_work_blocker_stops_collection",
            "missing_activation_operator_environment_invalid",
            "unordered_twenty_four_rollout_batch_mapping",
            "missing_duplicate_and_wrong_identity_batch_rejection",
            "batch_terminal_work_failure_preserved_with_later_resume",
            "batch_activation_all_preserves_user_controlled_trust",
            "automatic_project_identity_and_bundle_export",
            "four_kind_markdown_html_document_evidence",
            "static_viewer_snapshot_evidence",
            "failed_document_kind_is_machine_failure",
            "bounded_runtime_summary",
            "deterministic_manifest",
            "bounded_default_review_archive",
            "campaign_level_human_review_packaging",
            "explicit_raw_rollout_archive_option",
            "evidence_hash_tamper_detection",
        ],
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
