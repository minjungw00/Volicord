#!/usr/bin/env python3
"""Disposable end-to-end checks for the private dogfood campaign helper."""

from __future__ import annotations

import copy
from collections import Counter
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
TEST_ASSIGNMENTS = [
    ("volicord", 1, "explicit_user_owned_decision"),
    ("volicord", 2, "hidden_user_owned_decision"),
    ("small-python", 1, "research_or_no_question"),
    ("small-python", 2, "hidden_user_owned_decision"),
    ("polyglot-medium", 1, "delegated_implementation_choice"),
    ("polyglot-medium", 2, "exploratory_uncertainty"),
]


def write_fake_binary(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(STRICT_FAKE, path)
    path.chmod(0o755)
    mcp = path.with_name("volicord-mcp")
    shutil.copyfile(STRICT_FAKE, mcp)
    mcp.chmod(0o755)
    viewer = path.with_name("volicord-viewer")
    viewer.write_text(
        "#!/bin/sh\n"
        "destination=\n"
        "for argument in \"$@\"; do destination=$argument; done\n"
        "printf '%s\\n' '<!doctype html><html lang=\"en\"><body data-viewer-mode=\"snapshot\">fixture</body></html>' > \"$destination\"\n",
        encoding="utf-8",
    )
    viewer.chmod(0o755)


def write_static_integration(repository: Path, runtime: Path, binary: Path) -> dict[str, object]:
    repository = repository.resolve()
    runtime = runtime.resolve()
    binary = binary.resolve()
    mcp = binary.with_name("volicord-mcp").resolve()
    codex = repository / ".codex"
    codex.mkdir(parents=True, exist_ok=True)
    manifest = {
        "kind": "volicord_codex_repository_integration",
        "schema_version": 1,
        "repository": str(repository),
        "runtime": str(runtime),
        "volicord": str(binary),
        "volicord_mcp": str(mcp),
        "config_created": True,
        "excluded_paths": ["/.codex/config.toml", "/.codex/volicord-integration.json"],
    }
    campaign.write_json(codex / "volicord-integration.json", manifest)
    command = (
        f"{campaign.shell_quote_path(binary)} --runtime {campaign.shell_quote_path(runtime)} "
        f"--repository {campaign.shell_quote_path(repository)} codex hook"
    )
    (codex / "config.toml").write_text(
        "[mcp_servers.volicord]\n"
        f'command = "{mcp}"\n'
        "enabled = true\n"
        "required = true\n"
        f'env = {{ VOLICORD_RUNTIME_DIR = "{runtime}" }}\n\n'
        "[[hooks.SessionStart]]\n"
        'matcher = "^(startup|resume|clear|compact)$"\n\n'
        "[[hooks.SessionStart.hooks]]\n"
        'type = "command"\n'
        f'command = "{command}"\n'
        "timeout = 5\n"
        'statusMessage = "Activating Volicord repository context"\n'
        "additionalContextLimit = 2000\n",
        encoding="utf-8",
    )
    return {
        "operation": "codex_enable",
        "repository": str(repository),
        "config": str(codex / "config.toml"),
        "mcp_server": "volicord",
        "mcp_executable": str(mcp),
        "runtime": str(runtime),
        "session_start_matcher": "^(startup|resume|clear|compact)$",
        "project_trust": "user_controlled",
    }


def fake_enable_command(argv: list[str]) -> dict[str, object]:
    if argv[-2:] and argv[-2:] == ["codex", "disable"]:
        return {}
    runtime = Path(argv[argv.index("--runtime") + 1])
    repository = Path(argv[argv.index("--repository") + 1])
    return write_static_integration(repository, runtime, Path(argv[0]))


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


def prepare(
    root: Path,
    source_root: Path,
    binary: Path,
    *,
    slot_id_factory=campaign.new_review_slot_id,
) -> None:
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
            slot_id_factory=slot_id_factory,
            behavior_assignment_factory=lambda: list(TEST_ASSIGNMENTS),
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
        behavior_class=(
            campaign.cycle_state(campaign_root, kind, cycle)["behavior_class"]
            if campaign_root is not None
            else "explicit_user_owned_decision"
        ),
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
    preparation = campaign.prepare_review(root, kind, cycle, source)
    provisional = copy.deepcopy(
        value["behavior_review"]["independent_review"]["provisional_review"]
    )
    provisional["preparation_sha256"] = preparation["preparation_sha256"]
    provisional["review_slot_id"] = preparation["review_slot_id"]
    provisional_path = root.parent / f"{preparation['review_slot_id']}-provisional.json"
    campaign.write_json(provisional_path, provisional)
    campaign.record_provisional_review(
        root,
        campaign.load_campaign(root)["candidate_head"],
        preparation["review_slot_id"],
        provisional_path,
    )
    campaign.seal_cycle(root, kind, cycle, source)


def set_provisional_classification(
    provisional: dict[str, object], classification: str
) -> None:
    provisional["classification"] = classification
    user_owned = harness.is_user_owned_behavior(classification)
    provisional["materiality_conclusion"] = (
        "user_owned_material_outcome"
        if user_owned
        else "no_user_owned_material_outcome"
    )
    provisional["material_outcome_unavoidable"] = user_owned
    provisional["operator_prompt_does_not_disclose_material_outcome"] = (
        True
        if classification == "hidden_user_owned_decision"
        else False
        if classification == "explicit_user_owned_decision"
        else None
    )


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


def compacted_capture(source: Path, destination: Path, after_phrase: str) -> Path:
    lines = source.read_text(encoding="utf-8").splitlines()
    insertion = next(index for index, line in enumerate(lines) if after_phrase in line) + 1
    lines.insert(
        insertion,
        json.dumps(
            {
                "timestamp": "2026-08-15T00:00:00Z",
                "type": "event_msg",
                "payload": {"type": "context_compacted"},
            },
            separators=(",", ":"),
        ),
    )
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
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
        for cycle in range(1, campaign.CYCLES_PER_REPOSITORY + 1):
            descriptor, work, resume, bundle = fixture_for(
                parent / f"{name}-fixtures",
                kind,
                cycle,
                campaign_root=root,
            )
            install_descriptor(root, kind, cycle, descriptor)
            captures.extend((work, resume))
            state = campaign.cycle_state(root, kind, cycle)
            bundles[state["review_slot_id"]] = bundle
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


def assert_opaque_slot_preparation(parent: Path, binary: Path) -> None:
    generated_assignments = campaign.new_private_behavior_assignments()
    campaign.validate_private_behavior_assignments(generated_assignments)
    assert Counter(
        behavior for _kind, _cycle, behavior in generated_assignments
    ) == campaign.QUALIFICATION_BEHAVIOR_COUNTS
    assert len({
        kind
        for kind, _cycle, behavior in generated_assignments
        if behavior == "hidden_user_owned_decision"
    }) == 2
    campaign_source = Path(campaign.__file__).read_text(encoding="utf-8")
    assert "index(behavior_class)" not in campaign_source
    assert "BEHAVIOR_CLASSES[" not in campaign_source
    duplicate_root = parent / "duplicate-slot-campaign"
    try:
        prepare(
            duplicate_root,
            parent / "duplicate-slot-sources",
            binary,
            slot_id_factory=lambda: "00" * 16,
        )
    except campaign.CampaignError as error:
        assert "duplicate" in str(error)
    else:
        raise AssertionError("duplicate opaque slots mutated a campaign")
    assert not duplicate_root.exists()

    root = parent / "opaque-slot-campaign"
    prepare(root, parent / "opaque-slot-sources", binary)
    state = campaign.load_campaign(root)
    assert len(state["cycles"]) == campaign.QUALIFICATION_CYCLE_COUNT
    slots = [item["review_slot_id"] for item in state["cycles"].values()]
    assert len(slots) == len(set(slots)) == campaign.QUALIFICATION_CYCLE_COUNT
    assert all(campaign.REVIEW_SLOT_ID.fullmatch(slot) for slot in slots)
    for kind in campaign.CLASSES:
        ordered_cycles = [
            item["cycle"]
            for item in sorted(
                (
                    value
                    for value in state["cycles"].values()
                    if value["repository_class"] == kind
                ),
                key=lambda value: value["review_slot_id"],
            )
        ]
        assert ordered_cycles != list(range(1, campaign.CYCLES_PER_REPOSITORY + 1))
        assert sum(
            item["repository_class"] == kind for item in state["cycles"].values()
        ) == campaign.CYCLES_PER_REPOSITORY
    assert Counter(
        item["behavior_class"] for item in state["cycles"].values()
    ) == campaign.QUALIFICATION_BEHAVIOR_COUNTS
    assert len({
        item["repository_class"]
        for item in state["cycles"].values()
        if item["behavior_class"] == "hidden_user_owned_decision"
    }) == 2
    for value in state["cycles"].values():
        slot = value["review_slot_id"]
        obvious = hashlib.sha256(
            f"{value['repository_class']}:{value['cycle']}".encode("utf-8")
        ).hexdigest()[:32]
        assert slot != obvious
        assert Path(value["repository_path"]).resolve() == (
            root / "slots" / slot / "repository"
        ).resolve()
        assert Path(value["runtime_home"]).resolve() == (
            root / "slots" / slot / "runtime"
        ).resolve()
        assert Path(value["reviewer_repository_path"]).resolve() == (
            root / "reviewer/workspaces" / slot / "repository"
        ).resolve()

    index = campaign.read_json(campaign.reviewer_index_path(root))
    assert index["ordering"] == "opaque_review_slot_id"
    assert [entry["review_slot_id"] for entry in index["entries"]] == sorted(slots)
    serialized_index = json.dumps(index, sort_keys=True)
    assert "repository_class" not in serialized_index
    assert "logical_cycle" not in serialized_index
    assert not any(value in serialized_index for value in campaign.BEHAVIOR_CLASSES)

    mapping_path = campaign.slot_mapping_path(root)
    original_mapping = mapping_path.read_bytes()
    mapping = json.loads(original_mapping)
    mapping["entries"][0]["review_slot_id"], mapping["entries"][1]["review_slot_id"] = (
        mapping["entries"][1]["review_slot_id"],
        mapping["entries"][0]["review_slot_id"],
    )
    campaign.write_json(mapping_path, mapping)
    try:
        campaign.verify_inventory(root)
    except campaign.CampaignError as error:
        assert "hash mismatch" in str(error)
    else:
        raise AssertionError("swapped opaque slot mapping passed inventory verification")
    try:
        campaign.load_campaign(root)
    except campaign.CampaignError as error:
        assert "mapping" in str(error)
    else:
        raise AssertionError("swapped opaque slot mapping passed campaign binding")
    mapping_path.write_bytes(original_mapping)
    campaign.verify_inventory(root)
    campaign_path = campaign.campaign_file(root)
    original_campaign = campaign_path.read_bytes()
    changed_campaign = json.loads(original_campaign)
    first_key, second_key = list(changed_campaign["cycles"])[:2]
    changed_campaign["cycles"][first_key]["review_slot_id"], changed_campaign["cycles"][
        second_key
    ]["review_slot_id"] = (
        changed_campaign["cycles"][second_key]["review_slot_id"],
        changed_campaign["cycles"][first_key]["review_slot_id"],
    )
    campaign.write_json(campaign_path, changed_campaign)
    try:
        campaign.load_campaign(root)
    except campaign.CampaignError as error:
        assert "mapping" in str(error)
    else:
        raise AssertionError("campaign-side opaque slot swap passed private mapping binding")
    campaign_path.write_bytes(original_campaign)
    campaign.load_campaign(root)


def assert_blockers(parent: Path, binary: Path) -> None:
    blocker_root = parent / "blocker-campaign"
    prepare(blocker_root, parent / "blocker-sources", binary)
    descriptor, work, resume, _bundle = fixture_for(
        parent, "volicord", 1, campaign_root=blocker_root
    )
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
    activation_descriptor, activation_work, _activation_resume, _activation_bundle = fixture_for(
        parent / "activation-fixture",
        "volicord",
        1,
        campaign_root=activation_root,
    )
    install_descriptor(activation_root, "volicord", 1, activation_descriptor)
    missing = filtered_capture(
        activation_work, parent / "missing-activation.jsonl", "Volicord is active because"
    )
    invalid = campaign.collect_work(activation_root, "volicord", 1, missing)
    assert invalid["outcome"] == "operator_environment_invalid"
    assert invalid["classification"] == "operator_environment_setup_failure"


def assert_blind_recording_non_oracle(parent: Path, binary: Path) -> None:
    results: list[dict[str, object]] = []
    recorded_paths: list[Path] = []
    all_read_paths: list[Path] = []
    original_comparison = harness.classification_comparison_errors
    original_read_json = campaign.read_json

    def forbidden_comparison(*_args: object, **_kwargs: object) -> list[str]:
        raise AssertionError("recording invoked evaluator-relative comparison")

    def tracking_read_json(path: Path):
        all_read_paths.append(Path(path).resolve())
        return original_read_json(path)

    try:
        harness.classification_comparison_errors = forbidden_comparison
        campaign.read_json = tracking_read_json
        for label, classification in (
            ("correct", "explicit_user_owned_decision"),
            ("wrong", "research_or_no_question"),
        ):
            root = parent / f"blind-recording-{label}-campaign"
            prepare(root, parent / f"blind-recording-{label}-sources", binary)
            descriptor, _work, _resume, _bundle = fixture_for(
                parent / f"blind-recording-{label}-fixture",
                "volicord",
                1,
                campaign_root=root,
            )
            descriptor.pop("_evidence_directory", None)
            descriptor.pop("_evidence_file_sha256", None)
            descriptor.pop("evidence", None)
            draft_path = parent / f"blind-recording-{label}-draft.json"
            campaign.write_json(draft_path, descriptor)
            preparation = campaign.prepare_review(root, "volicord", 1, draft_path)
            provisional = copy.deepcopy(
                descriptor["behavior_review"]["independent_review"]["provisional_review"]
            )
            set_provisional_classification(provisional, classification)
            provisional["preparation_sha256"] = preparation["preparation_sha256"]
            provisional["review_slot_id"] = preparation["review_slot_id"]
            source = parent / f"blind-recording-{label}-provisional.json"
            campaign.write_json(source, provisional)
            result = campaign.record_provisional_review(
                root,
                campaign.load_campaign(root)["candidate_head"],
                preparation["review_slot_id"],
                source,
            )
            fixed = campaign.reviewer_provisional_path(root, "volicord", 1)
            assert fixed.read_bytes() == source.read_bytes()
            assert harness.sha256(fixed) == result["provisional_review_sha256"]
            assert campaign.cycle_state(root, "volicord", 1)["state"] == "provisional_recorded"
            results.append(result)
            recorded_paths.append(fixed)
    finally:
        harness.classification_comparison_errors = original_comparison
        campaign.read_json = original_read_json

    assert len(results) == len(recorded_paths) == 2
    assert set(results[0]) == set(results[1])
    for result in results:
        serialized = json.dumps(result, sort_keys=True)
        assert result["state"] == "provisional_recorded"
        assert result["evaluator_material_exposed"] is False
        assert "match" not in serialized.casefold()
        assert "expected" not in serialized.casefold()
        assert not any(value in serialized for value in campaign.BEHAVIOR_CLASSES)
        assert "repository_class" not in serialized
        assert "logical_cycle" not in serialized
    for field in ("kind", "candidate_head", "state", "evaluator_material_exposed"):
        assert results[0][field] == results[1][field]
    assert not any("evaluator/descriptors" in path.as_posix() for path in all_read_paths)


def assert_sealing_and_provenance(parent: Path, binary: Path) -> None:
    root = parent / "sealing-campaign"
    prepare(root, parent / "sealing-sources", binary)
    helper = campaign.ROOT / "rebuild/scripts/dogfood-campaign"
    record_help = subprocess.run(
        [str(helper), "record-provisional-review", "--help"],
        text=True,
        capture_output=True,
        check=False,
    )
    seal_help = subprocess.run(
        [str(helper), "seal-cycle", "--help"],
        text=True,
        capture_output=True,
        check=False,
    )
    assert record_help.returncode == seal_help.returncode == 0
    assert "--review-slot-id" in record_help.stdout
    assert "--provisional-review" in record_help.stdout
    assert "--provisional-review" not in seal_help.stdout
    run_sheet = root / "operator/RUN-SHEET.md"
    initial = run_sheet.read_text(encoding="utf-8")
    assert "descriptor" not in initial.casefold()
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

    descriptor, _work, _resume, _bundle = fixture_for(
        parent, "volicord", 1, campaign_root=root
    )
    descriptor.pop("_evidence_directory", None)
    descriptor.pop("_evidence_file_sha256", None)
    descriptor.pop("evidence", None)
    draft_path = parent / "review-preparation-draft.json"
    campaign.write_json(draft_path, descriptor)
    preparation = campaign.prepare_review(root, "volicord", 1, draft_path)
    preparation_body = campaign.read_json(root / preparation["preparation"])
    serialized_preparation = json.dumps(preparation_body, sort_keys=True)
    review_slot_id = preparation["review_slot_id"]
    assert set(preparation_body) == {
        "kind",
        "review_slot_id",
        "candidate_head",
        "repository_revision",
        "reviewer_repository_path",
        "work_user_task",
        "fresh_resume_user_task",
        "work_scope",
        "owner_document_locations",
    }
    assert preparation_body["review_slot_id"] == review_slot_id
    assert Path(preparation["preparation"]).name == f"{review_slot_id}.json"
    assert Path(preparation_body["reviewer_repository_path"]).resolve() == (
        root / "reviewer/workspaces" / review_slot_id / "repository"
    ).resolve()
    assert "evaluation_basis" not in serialized_preparation
    assert "counterfactual" not in serialized_preparation
    assert "possible_material_concerns" not in serialized_preparation
    assert '"cycle"' not in serialized_preparation
    assert '"repository_class"' not in serialized_preparation
    assert not any(value in serialized_preparation for value in campaign.BEHAVIOR_CLASSES)
    provisional = copy.deepcopy(
        descriptor["behavior_review"]["independent_review"]["provisional_review"]
    )
    provisional["preparation_sha256"] = preparation["preparation_sha256"]
    provisional["review_slot_id"] = review_slot_id
    provisional_path = parent / f"{review_slot_id}-fixed-provisional-review.json"
    campaign.write_json(provisional_path, provisional)
    assert not campaign.reviewer_provisional_path(root, "volicord", 1).exists()
    assert not campaign.evaluator_descriptor_path(root, "volicord", 1).exists()
    try:
        campaign.seal_cycle(root, "volicord", 1, draft_path)
    except campaign.CampaignError as error:
        assert "recorded provisional review" in str(error)
    else:
        raise AssertionError("cycle sealed without a recorded provisional review")

    def recording_snapshot() -> tuple[tuple[str, int, str], ...]:
        return tuple(
            (
                path.relative_to(root).as_posix(),
                path.stat().st_size,
                harness.sha256(path),
            )
            for path in sorted(root.rglob("*"))
            if path.is_file()
        )

    malformed = parent / f"{review_slot_id}-malformed-provisional.json"
    campaign.write_json(malformed, {"kind": "phase8_provisional_behavior_review"})
    wrong_preparation = copy.deepcopy(provisional)
    wrong_preparation["preparation_sha256"] = "00" * 32
    wrong_preparation_path = parent / f"{review_slot_id}-wrong-preparation.json"
    campaign.write_json(wrong_preparation_path, wrong_preparation)
    evaluator_leak = copy.deepcopy(provisional)
    evaluator_leak["evaluation_basis"] = {"behavior_class": "hidden"}
    evaluator_leak_path = parent / f"{review_slot_id}-evaluator-leak.json"
    campaign.write_json(evaluator_leak_path, evaluator_leak)
    self_contradictory = copy.deepcopy(provisional)
    self_contradictory["classification"] = "research_or_no_question"
    self_contradictory_path = parent / f"{review_slot_id}-self-contradictory.json"
    campaign.write_json(self_contradictory_path, self_contradictory)
    candidate_head = campaign.load_campaign(root)["candidate_head"]
    invalid_recordings = (
        ("different campaign candidate", "00" * 20, review_slot_id, provisional_path),
        ("does not identify", candidate_head, "00" * 16, provisional_path),
        ("does not qualify", candidate_head, review_slot_id, malformed),
        ("does not qualify", candidate_head, review_slot_id, wrong_preparation_path),
        ("does not qualify", candidate_head, review_slot_id, evaluator_leak_path),
        ("does not qualify", candidate_head, review_slot_id, self_contradictory_path),
    )
    for expected, attempted_candidate, attempted_slot, attempted_review in invalid_recordings:
        before = recording_snapshot()
        try:
            campaign.record_provisional_review(
                root,
                attempted_candidate,
                attempted_slot,
                attempted_review,
            )
        except campaign.CampaignError as error:
            assert expected in str(error)
        else:
            raise AssertionError("invalid provisional review recording succeeded")
        assert recording_snapshot() == before

    original_atomic_write = campaign.atomic_write_bytes
    injected_failure = {"raised": False}

    def fail_inventory_publish_once(path: Path, content: bytes) -> None:
        if path == campaign.inventory_path(root) and not injected_failure["raised"]:
            injected_failure["raised"] = True
            raise OSError("injected inventory publication failure")
        original_atomic_write(path, content)

    before_publish_failure = recording_snapshot()
    campaign.atomic_write_bytes = fail_inventory_publish_once
    try:
        campaign.record_provisional_review(
            root,
            candidate_head,
            review_slot_id,
            provisional_path,
        )
    except OSError as error:
        assert "injected inventory publication failure" in str(error)
    else:
        raise AssertionError("injected provisional publication failure succeeded")
    finally:
        campaign.atomic_write_bytes = original_atomic_write
    assert recording_snapshot() == before_publish_failure

    read_paths: list[Path] = []
    original_read_json = campaign.read_json

    def tracking_read_json(path: Path):
        read_paths.append(Path(path).resolve())
        return original_read_json(path)

    campaign.read_json = tracking_read_json
    try:
        recorded = campaign.record_provisional_review(
            root,
            candidate_head,
            review_slot_id,
            provisional_path,
        )
    finally:
        campaign.read_json = original_read_json
    serialized_recorded = json.dumps(recorded, sort_keys=True)
    assert recorded["state"] == "provisional_recorded"
    assert recorded["evaluator_material_exposed"] is False
    assert not any(value in serialized_recorded for value in campaign.BEHAVIOR_CLASSES)
    assert "repository_class" not in serialized_recorded
    assert "logical_cycle" not in serialized_recorded
    assert not any("evaluator/descriptors" in path.as_posix() for path in read_paths)

    bypassable = copy.deepcopy(descriptor)
    bypassable_counterfactual = bypassable["behavior_review"]["independent_review"][
        "counterfactual_review"
    ]
    bypassable_counterfactual["no_question_approaches"][0].update({
        "task_satisfaction": "fully_satisfies_without_user_owned_outcome",
        "assessment": (
            "A narrower implementation fully satisfies the frozen request without choosing the claimed public outcome."
        ),
    })
    bypassable_path = parent / "bypassable-user-owned-input.json"
    campaign.write_json(bypassable_path, bypassable)
    try:
        campaign.seal_cycle(root, "volicord", 1, bypassable_path)
    except campaign.CampaignError as error:
        assert "defensible no-question path" in str(error)
    else:
        raise AssertionError("bypassable material user-owned descriptor sealed")
    fixed_provisional = campaign.reviewer_provisional_path(root, "volicord", 1)
    assert fixed_provisional.read_bytes() == provisional_path.read_bytes()
    assert campaign.cycle_state(root, "volicord", 1)["state"] == "provisional_recorded"
    assert not campaign.evaluator_descriptor_path(root, "volicord", 1).exists()
    altered_provisional = copy.deepcopy(provisional)
    altered_provisional["basis"] += " altered"
    altered_provisional_path = parent / f"{review_slot_id}-altered-provisional.json"
    campaign.write_json(altered_provisional_path, altered_provisional)
    fixed_bytes = fixed_provisional.read_bytes()
    try:
        campaign.record_provisional_review(
            root,
            candidate_head,
            review_slot_id,
            altered_provisional_path,
        )
    except campaign.CampaignError as error:
        assert "review-prepared opaque slot" in str(error)
    else:
        raise AssertionError("fixed provisional review was retroactively altered")
    assert fixed_provisional.read_bytes() == fixed_bytes

    tampered_bytes = fixed_bytes.replace(b'"status": "recorded"', b'"status": "recordex"')
    assert tampered_bytes != fixed_bytes
    fixed_provisional.write_bytes(tampered_bytes)
    try:
        campaign.seal_cycle(root, "volicord", 1, bypassable_path)
    except campaign.CampaignError as error:
        assert "evidence hash mismatch" in str(error)
    else:
        raise AssertionError("changed fixed provisional bytes passed sealing")
    fixed_provisional.write_bytes(fixed_bytes)
    campaign.verify_inventory(root)
    inventory_bytes = campaign.inventory_path(root).read_bytes()
    inventory = json.loads(inventory_bytes)
    inventory["artifacts"][campaign.relative(root, fixed_provisional)]["sha256"] = "00" * 32
    campaign.write_json(campaign.inventory_path(root), inventory)
    try:
        campaign.seal_cycle(root, "volicord", 1, bypassable_path)
    except campaign.CampaignError as error:
        assert "evidence hash mismatch" in str(error)
    else:
        raise AssertionError("changed provisional inventory passed sealing")
    campaign.inventory_path(root).write_bytes(inventory_bytes)
    campaign.verify_inventory(root)

    rewritten = copy.deepcopy(descriptor)
    rewritten["behavior_review"]["independent_review"]["provisional_review"][
        "materiality_conclusion"
    ] = "no_user_owned_material_outcome"
    rewritten_path = parent / "rewritten-provisional-conclusion-input.json"
    campaign.write_json(rewritten_path, rewritten)
    try:
        campaign.seal_cycle(root, "volicord", 1, rewritten_path)
    except campaign.CampaignError as error:
        assert "cannot rewrite" in str(error)
    else:
        raise AssertionError("final comparison rewrote a provisional conclusion")
    assert fixed_provisional.read_bytes() == fixed_bytes

    disagreement = copy.deepcopy(descriptor)
    disagreement_agreement = disagreement["behavior_review"]["independent_review"][
        "fact_authority_agreement"
    ]
    disagreement_agreement.update({
        "status": "unresolved_conflict",
        "conflicts": [
            "Evaluator and reviewer disagree whether the active owner delegates the outcome."
        ],
        "resolution_basis": "The cited evidence has not resolved the authority disagreement.",
    })
    disagreement_path = parent / "unresolved-review-disagreement-input.json"
    campaign.write_json(disagreement_path, disagreement)
    try:
        campaign.seal_cycle(root, "volicord", 1, disagreement_path)
    except campaign.CampaignError as error:
        assert "disagreement blocks sealing" in str(error)
    else:
        raise AssertionError("unresolved evaluator/reviewer disagreement sealed")

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

    accepted_path = parent / "unavoidable-user-owned-input.json"
    campaign.write_json(accepted_path, descriptor)
    sealed = campaign.seal_cycle(root, "volicord", 1, accepted_path)
    assert sealed["review_slot_id"] == review_slot_id
    sealed_run_sheet = run_sheet.read_text(encoding="utf-8")
    assert f"Slot `{review_slot_id}`" in sealed_run_sheet
    assert "cycle 1" not in sealed_run_sheet.casefold()
    assert "-cycle-" not in sealed_run_sheet
    assert descriptor["work_user_task"] in sealed_run_sheet
    assert provisional["basis"] not in sealed_run_sheet
    assert provisional["materiality_conclusion"] not in sealed_run_sheet
    assert (
        descriptor["behavior_review"]["independent_review"]["counterfactual_review"][
            "specific_unresolved_outcome"
        ]
        not in sealed_run_sheet
    )

    cli_descriptor, _work, _resume, _bundle = fixture_for(
        parent, "volicord", 2, campaign_root=root
    )
    cli_descriptor.pop("_evidence_directory", None)
    cli_descriptor.pop("_evidence_file_sha256", None)
    cli_descriptor.pop("evidence", None)
    cli_draft = parent / "cli-recording-draft.json"
    campaign.write_json(cli_draft, cli_descriptor)
    cli_preparation = campaign.prepare_review(root, "volicord", 2, cli_draft)
    cli_provisional = copy.deepcopy(
        cli_descriptor["behavior_review"]["independent_review"]["provisional_review"]
    )
    set_provisional_classification(cli_provisional, "research_or_no_question")
    cli_provisional["preparation_sha256"] = cli_preparation["preparation_sha256"]
    cli_provisional["review_slot_id"] = cli_preparation["review_slot_id"]
    cli_provisional_path = parent / "cli-provisional-review.json"
    campaign.write_json(cli_provisional_path, cli_provisional)
    cli_recorded = subprocess.run(
        [
            str(helper),
            "record-provisional-review",
            "--campaign-root",
            str(root),
            "--candidate-head",
            candidate_head,
            "--review-slot-id",
            cli_preparation["review_slot_id"],
            "--provisional-review",
            str(cli_provisional_path),
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    assert cli_recorded.returncode == 0, cli_recorded.stdout + cli_recorded.stderr
    cli_recorded_result = json.loads(cli_recorded.stdout)
    assert cli_recorded_result["state"] == "provisional_recorded"
    assert cli_recorded_result["evaluator_material_exposed"] is False

    fixed_cli_provisional = campaign.reviewer_provisional_path(root, "volicord", 2)
    fixed_cli_bytes = fixed_cli_provisional.read_bytes()
    fixed_cli_sha256 = harness.sha256(fixed_cli_provisional)
    comparison_descriptor = copy.deepcopy(cli_descriptor)
    comparison_descriptor["behavior_review"]["independent_review"][
        "provisional_review"
    ] = copy.deepcopy(cli_provisional)
    comparison = comparison_descriptor["behavior_review"]["independent_review"][
        "classification_comparison"
    ]
    comparison.update({
        "provisional_classification": "research_or_no_question",
        "evaluator_classification": "hidden_user_owned_decision",
        "disagreements": [
            "classification",
            "materiality_conclusion",
            "material_outcome_unavoidable",
            "operator_prompt_disclosure",
        ],
        "resolution_basis": (
            "The cited active-owner evidence establishes the hidden user-owned outcome after reveal."
        ),
        "provenance_reference_indices": [0],
    })
    falsely_agreed = copy.deepcopy(comparison_descriptor)
    falsely_agreed["behavior_review"]["independent_review"][
        "classification_comparison"
    ]["status"] = "agreed"
    falsely_agreed_path = parent / "mismatched-falsely-agreed-input.json"
    campaign.write_json(falsely_agreed_path, falsely_agreed)
    try:
        campaign.seal_cycle(root, "volicord", 2, falsely_agreed_path)
    except campaign.CampaignError as error:
        assert "cannot be marked agreed" in str(error)
    else:
        raise AssertionError("mismatched provisional review masqueraded as agreement")

    unresolved = copy.deepcopy(comparison_descriptor)
    unresolved["behavior_review"]["independent_review"][
        "classification_comparison"
    ]["status"] = "unresolved_conflict"
    unresolved_path = parent / "mismatched-unresolved-input.json"
    campaign.write_json(unresolved_path, unresolved)
    try:
        campaign.seal_cycle(root, "volicord", 2, unresolved_path)
    except campaign.CampaignError as error:
        assert "disagreement blocks sealing" in str(error)
    else:
        raise AssertionError("unresolved classification disagreement sealed")

    resolved = copy.deepcopy(comparison_descriptor)
    resolved["behavior_review"]["independent_review"][
        "classification_comparison"
    ]["status"] = "resolved_from_evidence"
    resolved_path = parent / "mismatched-evidence-resolved-input.json"
    campaign.write_json(resolved_path, resolved)
    resolved_result = campaign.seal_cycle(root, "volicord", 2, resolved_path)
    assert resolved_result["review_slot_id"] == cli_preparation["review_slot_id"]
    assert fixed_cli_provisional.read_bytes() == fixed_cli_bytes
    assert harness.sha256(fixed_cli_provisional) == fixed_cli_sha256

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
    assert not any(value in run_sheet for value in campaign.BEHAVIOR_CLASSES)
    assert not any(
        f"cycle {number}" in run_sheet.casefold()
        for number in range(1, campaign.CYCLES_PER_REPOSITORY + 1)
    )
    assert "-cycle-" not in run_sheet
    for kind in campaign.CLASSES:
        for cycle in range(1, campaign.CYCLES_PER_REPOSITORY + 1):
            descriptor = campaign.read_json(
                campaign.evaluator_descriptor_path(root, kind, cycle)
            )
            state = campaign.cycle_state(root, kind, cycle)
            slot_heading = f"### Slot `{state['review_slot_id']}`"
            start = run_sheet.index(slot_heading)
            later_slots = [
                position
                for marker in ("\n### Slot `", "\n## Repository `")
                if (position := run_sheet.find(marker, start + len(slot_heading))) >= 0
            ]
            end = min(later_slots, default=len(run_sheet))
            slot_entry = run_sheet[start:end]
            assert descriptor["work_user_task"] in slot_entry
            assert descriptor["fresh_resume_user_task"] in slot_entry
            assert state["repository_path"] in run_sheet
            assert state["runtime_home"] in run_sheet

    compacted_work_source = next(
        path for path in captures if path.name == "volicord-1-work-events.jsonl"
    )
    compacted_work = compacted_capture(
        compacted_work_source,
        parent / "volicord-1-work-events.jsonl",
        '"call_id":"volicord-status-call-1"',
    )
    captures = [compacted_work if path == compacted_work_source else path for path in captures]
    mapped = campaign.map_batch_rollouts(root, list(reversed(captures)))
    assert len(mapped) == campaign.BATCH_CAPTURE_COUNT
    assert len({capture.session_id for _path, capture in mapped.values()}) == 12
    compacted_mapped_capture = mapped[("volicord", 1, "work")][1]
    assert compacted_mapped_capture.fresh_user_thread is True
    assert len(compacted_mapped_capture.compacted_sequences) == 1

    directory = parent / "unordered-rollouts"
    directory.mkdir()
    for index, source in enumerate(reversed(captures)):
        shutil.copyfile(source, directory / f"capture-{index:02}.jsonl")
    directory_paths = campaign.batch_rollout_paths(None, directory)
    assert len(campaign.map_batch_rollouts(root, directory_paths)) == 12
    campaign_before_failed_batch = (root / "campaign.json").read_bytes()
    inventory_before_failed_batch = (root / "evidence-inventory.json").read_bytes()
    try:
        campaign.collect_batch(root, captures[:-1])
    except campaign.CampaignError as error:
        assert "missing" in str(error)
    else:
        raise AssertionError("batch collection accepted a missing rollout")
    assert (root / "campaign.json").read_bytes() == campaign_before_failed_batch
    assert (root / "evidence-inventory.json").read_bytes() == inventory_before_failed_batch
    assert not any(
        (campaign.cycle_root(root, kind, cycle) / "evidence/work.rollout.jsonl").exists()
        for kind in campaign.CLASSES
        for cycle in range(1, campaign.CYCLES_PER_REPOSITORY + 1)
    )

    work = compacted_work
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
    multiple_newline_work = replaced_capture(
        work,
        parent / "multiple-terminal-newlines.jsonl",
        descriptor["work_user_task"],
        descriptor["work_user_task"] + "\\n\\n",
    )
    multiple_newline_inputs = [
        multiple_newline_work if path == work else path for path in captures
    ]
    multiple_newline_bytes = multiple_newline_work.read_bytes()
    multiple_newline_mapping = campaign.map_batch_rollouts(root, multiple_newline_inputs)
    mapped_source, mapped_capture = multiple_newline_mapping[("volicord", 1, "work")]
    assert mapped_source == multiple_newline_work
    assert mapped_source.read_bytes() == multiple_newline_bytes
    assert mapped_capture.source_sha256 == hashlib.sha256(multiple_newline_bytes).hexdigest()

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
            assert "zero sealed cycle roles" in str(error)
            assert error.diagnostic is not None
            assert error.diagnostic["candidate_count"] == 0
            expected_reason = {
                "task": "frozen_task_mismatch",
                "revision": "repository_revision_mismatch",
                "workspace": "workspace_mismatch",
            }[label]
            assert expected_reason in error.diagnostic["mismatch_reasons"]
        else:
            raise AssertionError(f"batch mapping accepted wrong {label}")

    provenance_cases = (
        ("source", '"source":"vscode"', '"source":"exec"'),
        ("originator", '"originator":"codex_vscode"', '"originator":"codex_cli_rs"'),
        ("fork", '"thread_source":"user"', '"thread_source":"user","forked_from_id":"parent"'),
        ("thread-source", '"thread_source":"user"', '"thread_source":"subagent"'),
    )
    for label, old, new in provenance_cases:
        invalid = replaced_capture(work, parent / f"wrong-{label}.jsonl", old, new)
        inputs = [invalid if path == work else path for path in captures]
        try:
            campaign.map_batch_rollouts(root, inputs)
        except campaign.CampaignError as error:
            assert error.diagnostic is not None
            assert error.diagnostic["candidate_count"] == 0
            assert any(
                reason in error.diagnostic["mismatch_reasons"]
                for reason in (
                    "provenance_mismatch",
                    "provenance_or_capture_format_mismatch",
                )
            )
        else:
            raise AssertionError(f"batch mapping accepted wrong {label} provenance")

    original_load_descriptor = campaign.load_sealed_descriptor
    def ambiguous_descriptor(root_value, kind, cycle, campaign_value=None):
        path, value = original_load_descriptor(root_value, kind, cycle, campaign_value)
        if kind == "volicord" and cycle == 1:
            value = copy.deepcopy(value)
            value["fresh_resume_user_task"] = value["work_user_task"]
        return path, value
    campaign.load_sealed_descriptor = ambiguous_descriptor
    try:
        campaign.map_batch_rollouts(root, captures)
    except campaign.CampaignError as error:
        assert "multiple sealed cycle roles" in str(error)
        assert error.diagnostic is not None
        assert error.diagnostic["candidate_count"] == 2
        assert {item["role"] for item in error.diagnostic["matching_opaque_roles"]} == {
            "work", "resume"
        }
        assert all(
            set(item) == {"review_slot_id", "role"}
            for item in error.diagnostic["matching_opaque_roles"]
        )
        assert "repository_class" not in json.dumps(error.diagnostic)
        assert "behavior_class" not in json.dumps(error.diagnostic)
    else:
        raise AssertionError("batch mapping accepted a multi-role capture")
    finally:
        campaign.load_sealed_descriptor = original_load_descriptor

    original_run_checked = campaign.run_checked
    campaign.run_checked = lambda argv, cwd=campaign.ROOT: fake_enable_command(argv)
    try:
        activation = campaign.activate_all(root)
    finally:
        campaign.run_checked = original_run_checked
    assert activation["cycle_count"] == campaign.QUALIFICATION_CYCLE_COUNT
    assert activation["repository_and_hook_trust"] == "user_controlled_not_automated"
    assert all(
        item["result"]["static_verification"]["status"] == "passed"
        and item["result"]["static_verification"]["repository_and_hook_trust"]
        == "user_controlled_not_automated"
        and item["result"]["static_verification"]["runtime_session_start_execution"]
        == "not_proven_by_static_verification"
        for item in activation["cycles"]
    )

    first_state = campaign.cycle_state(root, "volicord", 1)
    first_repository = Path(first_state["repository_path"])
    first_runtime = Path(first_state["runtime_home"])
    first_result = write_static_integration(first_repository, first_runtime, binary)
    config_path = first_repository / ".codex/config.toml"
    original_config = config_path.read_bytes()
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            str(first_runtime), str(parent / "wrong-runtime")
        ),
        encoding="utf-8",
    )
    try:
        campaign.verify_static_codex_integration(
            first_repository, first_runtime, binary, first_result
        )
    except campaign.CampaignError as error:
        assert "MCP entry" in str(error)
    else:
        raise AssertionError("static integration inconsistency passed activation verification")
    config_path.write_bytes(original_config)

    campaign_state = campaign.load_campaign(root)
    campaign_state["cycles"][campaign.cycle_key("volicord", 1)]["codex_enabled"] = False
    campaign.save_campaign(root, campaign_state)
    def inconsistent_enable(argv: list[str], cwd=campaign.ROOT):
        result = fake_enable_command(argv)
        if argv[-2:] == ["codex", "enable"]:
            repository = Path(argv[argv.index("--repository") + 1])
            config = repository / ".codex/config.toml"
            config.write_text(
                config.read_text(encoding="utf-8").replace(
                    str(first_runtime), str(parent / "wrong-runtime")
                ),
                encoding="utf-8",
            )
        return result
    campaign.run_checked = inconsistent_enable
    try:
        campaign.activate_cycle(root, "volicord", 1)
    except campaign.CampaignError as error:
        assert "MCP entry" in str(error)
    else:
        raise AssertionError("activate-cycle completed with inconsistent static state")
    finally:
        campaign.run_checked = original_run_checked
    assert campaign.cycle_state(root, "volicord", 1)["codex_enabled"] is False
    write_static_integration(first_repository, first_runtime, binary)

    for kind in campaign.CLASSES:
        for cycle in range(1, campaign.CYCLES_PER_REPOSITORY + 1):
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
        "expected_count": 12,
        "observed_count": 12,
    }
    assert len(summary["cycles"]) == campaign.QUALIFICATION_CYCLE_COUNT
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
        assert "evaluator/slot-mapping.json" in names
        assert "batch-intake-summary.json" in names
        assert len([name for name in names if name.endswith("/evidence/viewer-snapshot.html")]) == 6
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
    assert activation_summary["environment_invalid_diagnostics"] == [{
        "kind": "phase8_dogfood_missing_session_start_activation",
        "classification": "operator_environment_setup_failure",
        "source_file": str(missing_activation.resolve()),
        "source_sha256": hashlib.sha256(missing_activation.read_bytes()).hexdigest(),
        "session_id": "volicord-work-session-1",
        "review_slot_id": campaign.cycle_state(
            activation_root, "volicord", 1
        )["review_slot_id"],
        "role": "work",
        "volicord_mcp_calls_observed": True,
        "runtime_session_start_activation_observed": False,
    }]
    activation_blocker = campaign.read_json(
        campaign.cycle_root(activation_root, "volicord", 1) / "blocker-result.json"
    )
    assert activation_blocker["classification"] == "operator_environment_setup_failure"


def assert_successful_campaign(parent: Path, binary: Path) -> None:
    root = parent / "successful-campaign"
    prepare(root, parent / "successful-sources", binary)
    fixtures: dict[tuple[str, int], tuple[dict[str, object], Path, Path, Path]] = {}
    for kind in campaign.CLASSES:
        for cycle in range(1, campaign.CYCLES_PER_REPOSITORY + 1):
            descriptor, work, resume, bundle = fixture_for(
                parent, kind, cycle, campaign_root=root
            )
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
        assert "evaluator/slot-mapping.json" in names
        assert len([name for name in names if name.startswith("evaluator/descriptors/")]) == 6
        assert len([name for name in names if name.startswith("behavior-reviews/") and name.endswith(".json")]) == 7
        assert len([name for name in names if name.startswith("reviewer/preparations/")]) == 6
        assert len([name for name in names if name.startswith("reviewer/provisional/")]) == 6
        assert len([name for name in names if "/evidence/generated-documents/" in name]) == 48
        assert len([name for name in names if name.endswith("/evidence/viewer-snapshot.html")]) == 6
        assert len([name for name in names if name.endswith("/viewer-snapshot-summary.json")]) == 6
        assert len([name for name in names if name.endswith("/documents-summary.json")]) == 6
        assert len([name for name in names if name.startswith("operator/document-review/")]) == 6
        for prefix in (
            "reviewer/preparations/",
            "reviewer/provisional/",
            "operator/document-review/",
        ):
            for name in (item for item in names if item.startswith(prefix)):
                assert campaign.REVIEW_SLOT_ID.fullmatch(Path(name).stem)
        review_index_file = opened.extractfile("behavior-reviews/index.json")
        assert review_index_file is not None
        review_index = json.loads(review_index_file.read())
        assert len(review_index["reviews"]) == 6
        assert all(
            campaign.REVIEW_SLOT_ID.fullmatch(item["review_slot_id"])
            and item["logical_cycle"] in range(1, campaign.CYCLES_PER_REPOSITORY + 1)
            and item["expected_behavior_class"] in campaign.BEHAVIOR_CLASSES
            for item in review_index["reviews"]
        )
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
        assert len([name for name in opened.getnames() if Path(name).name in campaign.RAW_NAMES]) == 12


def assert_resume_baseline_identity_and_ordering(parent: Path) -> None:
    descriptor, _work, resume, _bundle = fixture_for(parent, "volicord", 1)
    revision = harness.git_head(campaign.ROOT)
    assert revision is not None
    state = {
        "repository_revision": revision,
        "project_id": "01" * 16,
        "work_session_id": "different-work-session",
    }
    capture = harness.load_codex_capture(resume)
    assert len(capture.successful_calls("repository_analyze")) == 2
    assert campaign.inspect_resume(capture, descriptor, state) == "01" * 16

    def rewritten(name: str, marker: str, old: str, new: str) -> Path:
        destination = parent / name
        lines = []
        for line in resume.read_text(encoding="utf-8").splitlines():
            lines.append(line.replace(old, new) if marker in line else line)
        destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return destination

    cases = [
        (
            "post-write-selected.jsonl",
            "resume-checkpoint-call",
            "12" * 32,
            "15" * 32,
        ),
        (
            "wrong-project-baseline.jsonl",
            "resume-baseline-call",
            "01" * 16,
            "ff" * 16,
        ),
        (
            "unknown-baseline.jsonl",
            "resume-checkpoint-call",
            "12" * 32,
            "ff" * 32,
        ),
    ]
    for name, marker, old, new in cases:
        invalid = harness.load_codex_capture(rewritten(name, marker, old, new))
        try:
            campaign.inspect_resume(invalid, descriptor, state)
        except campaign.CampaignError:
            pass
        else:
            raise AssertionError(f"invalid resume baseline qualified: {name}")

    values = resume.read_text(encoding="utf-8").splitlines()
    baseline = [line for line in values if "resume-baseline-call" in line]
    remaining = [line for line in values if "resume-baseline-call" not in line]
    recall_index = next(
        index for index, line in enumerate(remaining) if "recall-call" in line
    )
    pre_recall = parent / "pre-recall-baseline.jsonl"
    pre_recall.write_text(
        "\n".join(remaining[:recall_index] + baseline + remaining[recall_index:])
        + "\n",
        encoding="utf-8",
    )
    try:
        campaign.inspect_resume(
            harness.load_codex_capture(pre_recall), descriptor, state
        )
    except campaign.CampaignError:
        pass
    else:
        raise AssertionError("pre-Recall baseline qualified")


def assert_failed_document_kind_is_machine_failure(parent: Path, binary: Path) -> None:
    root = parent / "failed-document-campaign"
    prepare(root, parent / "failed-document-sources", binary)
    descriptor, work, resume, bundle = fixture_for(
        parent, "volicord", 1, campaign_root=root
    )
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
        assert_opaque_slot_preparation(parent, binary)
        assert_blind_recording_non_oracle(parent, binary)
        assert_sealing_and_provenance(parent, binary)
        assert_blockers(parent, binary)
        assert_batch_workflow(parent, binary)
        assert_failed_document_kind_is_machine_failure(parent, binary)
        assert_campaign_level_human_review_operations(parent, binary)
        assert_resume_baseline_identity_and_ordering(parent)
        assert_successful_campaign(parent, binary)
    print(json.dumps({
        "status": "passed",
        "checks": [
            "campaign_level_human_review_operations",
            "strict_current_cli_positive_and_obsolete_negative_cases",
            "opaque_slot_generation_and_private_mapping",
            "duplicate_slot_rejected_before_campaign_mutation",
            "reviewer_filename_workspace_and_order_opacity",
            "standalone_provisional_recording_exits_successfully",
            "correct_and_wrong_same-class_provisionals_record_without_oracle",
            "recording_does_not_invoke_evaluator_relative_comparison",
            "recording_does_not_read_or_expose_evaluator_material",
            "recording_success_shapes_expose_no_match_or_evaluator_class",
            "invalid_provisional_recording_is_mutation_free",
            "self_contradictory_provisional_recording_is_mutation_free",
            "provisional_publication_failure_rolls_back",
            "seal_cycle_has_no_provisional_payload_path",
            "provisional_review_fixed_before_evaluator_comparison",
            "fixed_provisional_review_cannot_be_retroactively_altered",
            "operator_slot_and_workspace_opacity",
            "opaque_mapping_swap_tamper_detection",
            "sealed_evaluator_operator_isolation",
            "bypassable_user_owned_descriptor_rejected_before_sealing",
            "unavoidable_user_owned_descriptor_sealed",
            "mismatched_provisional_cannot_masquerade_as_agreement",
            "unresolved_classification_materiality_disagreement_blocks_sealing",
            "evidence_resolved_classification_materiality_disagreement_seals",
            "resolved_comparison_preserves_provisional_bytes_and_hash",
            "unresolved_fact_authority_disagreement_blocks_sealing",
            "typed_behavior_review_provenance_verification",
            "terminal_work_blocker_stops_collection",
            "missing_activation_operator_environment_invalid",
            "unordered_twelve_rollout_batch_mapping",
            "compacted_fresh_thread_batch_mapping",
            "global_mapping_failure_precedes_campaign_mutation",
            "missing_duplicate_and_wrong_identity_batch_rejection",
            "batch_terminal_work_failure_preserved_with_later_resume",
            "batch_activation_all_preserves_user_controlled_trust",
            "automatic_project_identity_and_bundle_export",
            "resume_baseline_identity_and_ordering",
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
