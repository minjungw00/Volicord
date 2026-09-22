#!/usr/bin/env python3
"""Disposable end-to-end checks for the private dogfood campaign helper."""

from __future__ import annotations

import copy
from collections import Counter
from dataclasses import replace
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

import campaign
import harness
from codex_events import activation_identity


REVISION = "ab" * 20
STRICT_FAKE = campaign.ROOT / "rebuild/validation/shared/strict_fake_volicord.py"
TEST_ASSIGNMENTS = [
    ("volicord", "A", ("explicit_user_owned_decision", "delegated_implementation_choice")),
    ("volicord", "B", ("hidden_user_owned_decision",)),
    ("volicord", "C", ("learning_deliberation", "learning_routine_control")),
    ("small-python", "A", ("hidden_user_owned_decision", "exploratory_uncertainty")),
    ("polyglot-medium", "A", ("research_or_no_question", "repository_or_environment_fact")),
]


import campaign as campaign_api

def collect_work_fixture(root: campaign_api.Path, kind: str, cycle: int, raw_capture: campaign_api.Path) -> dict[str, campaign_api.Any]:
    campaign = campaign_api.load_campaign_for_mutation(root)
    if campaign_api.document_realization.required(campaign['document_language'], campaign['viewer_locale']):
        raise campaign_api.CampaignError('cross-locale collection requires prepare-document-realizations and collect-batch')
    campaign_api.verify_inventory(root)
    if campaign.get('terminal_outcome') is not None:
        raise campaign_api.CampaignError('campaign already stopped; create a new campaign identity')
    key = campaign_api.work_key(kind, cycle)
    state = campaign['cycles'][key]
    if state['state'] != 'sealed':
        raise campaign_api.CampaignError('work collection requires a valid sealed evaluator descriptor')
    descriptor_path, descriptor = campaign_api.load_sealed_descriptor(root, kind, cycle, campaign)
    destination = campaign_api.work_root(root, kind, cycle) / 'evidence/work.rollout.jsonl'
    campaign_api.copy_exact(raw_capture.resolve(), destination)
    try:
        capture = campaign_api.load_codex_capture(destination)
    except (OSError, campaign_api.EvidenceError) as error:
        raise campaign_api.CampaignError('work rollout is not a supported normalized Codex capture') from error
    project_ids = campaign_api.observed_project_ids(capture)
    result: dict[str, campaign_api.Any]
    try:
        blocker = campaign_api.harness.build_work_observation(campaign['candidate_head'], descriptor, campaign_api.harness.sha256(descriptor_path), capture, target_repository=campaign_api.Path(state['repository_path']))
    except campaign_api.harness.NoWorkObservation:
        if len(project_ids) != 1:
            raise campaign_api.CampaignError('qualifying work capture must expose one Project identity')
        result = {'kind': 'phase8_dogfood_work_intake', 'outcome': 'resume_allowed', 'repository_class': kind, 'cycle': cycle, 'project_id': project_ids[0], 'work_capture_sha256': capture.source_sha256, 'repository_scoped_activation_observed': True}
        state['state'] = 'work_collected'
        state['project_id'] = project_ids[0]
        state['work_session_id'] = capture.session_id
    except campaign_api.harness.WorkCaptureContractError as error:
        result = campaign_api.work_capture_failure_result(kind, cycle, error)
        result['failure_attribution'] = campaign_api.bounded_failure_attribution('work', 'evidence', error.basis, [error.check])
        state['state'] = 'evidence_failed'
        campaign['terminal_outcome'] = 'evidence_failed'
    else:
        result = blocker
        if blocker['outcome'] == 'review_required':
            state['state'] = 'work_collected'
            state['project_id'] = project_ids[0] if len(project_ids) == 1 else None
            state['work_session_id'] = capture.session_id
        else:
            state['state'] = blocker['outcome']
            campaign['terminal_outcome'] = blocker['outcome']
        campaign_api.write_json(campaign_api.work_root(root, kind, cycle) / 'blocker-result.json', blocker)
        campaign_api.register_artifact(root, campaign_api.work_root(root, kind, cycle) / 'blocker-result.json')
    campaign_api.write_json(campaign_api.work_root(root, kind, cycle) / 'work-intake.json', result)
    activation = campaign_api.update_activation_summary(root, kind, cycle, work_session_start_activation_observed=capture.repository_scoped_activation_observed)
    campaign_api.save_campaign(root, campaign)
    for path in (destination, campaign_api.work_root(root, kind, cycle) / 'work-intake.json', activation):
        campaign_api.register_artifact(root, path)
    return result

def collect_resume_fixture(root: campaign_api.Path, kind: str, cycle: int, raw_capture: campaign_api.Path, *, exporter: campaign_api.Callable[[campaign_api.Path, campaign_api.Path, campaign_api.Path, campaign_api.Path], None]=campaign_api.default_export, documenter: campaign_api.Callable[[campaign_api.Path, campaign_api.Path, campaign_api.Path, str, str, campaign_api.Path, str, str], dict[str, campaign_api.Any]]=campaign_api.generate_document, snapshotter: campaign_api.Callable[[campaign_api.Path, campaign_api.Path, str, campaign_api.Path, str, str], dict[str, campaign_api.Any]]=campaign_api.generate_viewer_snapshot) -> dict[str, campaign_api.Any]:
    campaign = campaign_api.load_campaign_for_mutation(root)
    if campaign_api.document_realization.required(campaign['document_language'], campaign['viewer_locale']):
        raise campaign_api.CampaignError('cross-locale collection requires prepare-document-realizations and collect-batch')
    campaign_api.verify_inventory(root)
    if campaign.get('terminal_outcome') is not None:
        raise campaign_api.CampaignError('later collection is blocked; create a new campaign identity')
    state = campaign['cycles'][campaign_api.work_key(kind, cycle)]
    if state['state'] != 'work_collected':
        raise campaign_api.CampaignError('resume collection requires a resume_allowed work intake')
    destination = campaign_api.work_root(root, kind, cycle) / 'evidence/resume.rollout.jsonl'
    campaign_api.copy_exact(raw_capture.resolve(), destination)
    try:
        capture = campaign_api.load_codex_capture(destination)
    except (OSError, campaign_api.EvidenceError) as error:
        raise campaign_api.CampaignError('resume rollout is not a supported normalized Codex capture') from error
    return campaign_api.extract_resume_evidence(root, kind, cycle, capture, destination, exporter=exporter, documenter=documenter, snapshotter=snapshotter)

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
            obligation_assignment_factory=lambda: list(TEST_ASSIGNMENTS),
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
        Path(campaign.work_state(campaign_root, kind, cycle)["repository_path"])
        if campaign_root is not None
        else None
    )
    obligations = (
        campaign.private_materiality_obligations(
            campaign_root,
            campaign.work_state(campaign_root, kind, cycle),
        )
        if campaign_root is not None
        else ("explicit_user_owned_decision",)
    )
    descriptor = harness.real_session_fixture(
        kind,
        cycle,
        revision,
        fixture_root,
        repository_path=repository,
        materiality_obligations=obligations[0],
    )
    descriptor["kind"] = "phase8_work_descriptor"
    descriptor.pop("cycle", None)
    descriptor["journey_id"] = campaign.journey_id(kind)
    descriptor["work_slot_id"] = campaign.work_key(kind, cycle)
    descriptor["work_label"] = cycle
    descriptor["materiality_obligations"] = list(obligations)
    descriptor["evaluation_basis"]["materiality_obligations"] = list(obligations)
    if "delegated_implementation_choice" in obligations:
        descriptor["evaluation_basis"]["delegated_boundaries"] = [
            "The user delegated bounded internal implementation choices."
        ]
    review = descriptor["behavior_review"]
    review["classification"] = list(obligations)
    comparison = review["independent_review"]["classification_comparison"]
    comparison["evaluator_classification"] = list(obligations)
    provisional = review["independent_review"]["provisional_review"]
    comparison["disagreements"] = (
        [] if harness.obligation_set(provisional["classification"]) == set(obligations)
        else ["classification"]
    )
    comparison["status"] = "agreed" if not comparison["disagreements"] else "resolved_from_evidence"
    if "resume" not in campaign.session_roles(kind, cycle):
        descriptor["fresh_resume_user_task"] = None
    work = fixture_root / f"{kind}-{cycle}-work-events.jsonl"
    resume = fixture_root / f"{kind}-{cycle}-resume-events.jsonl"
    bundle = fixture_root / f"{kind}-{cycle}-context.bundle.json"
    project_id = hashlib.sha256(f"project:{kind}".encode()).hexdigest()[:32]
    work_item_id = hashlib.sha256(f"work:{kind}:{cycle}".encode()).hexdigest()[:32]
    for path in (work, resume):
        path.write_text(
            path.read_text(encoding="utf-8")
            .replace("01" * 16, project_id)
            .replace("08" * 16, work_item_id),
            encoding="utf-8",
        )
    envelope = json.loads(
        bundle.read_text(encoding="utf-8")
        .replace("01" * 16, project_id)
        .replace("08" * 16, work_item_id)
    )
    payload = envelope["payload"]
    semantic = {"project_id": payload["project_id"], "tables": payload["tables"]}
    history_basis = hashlib.sha256(
        json.dumps(semantic, ensure_ascii=False, separators=(",", ":")).encode()
    ).hexdigest()
    payload["lineage"] = {
        "common_base_basis": history_basis,
        "history_basis": history_basis,
    }
    envelope["checksum"] = hashlib.sha256(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode()
    ).hexdigest()
    campaign.write_json(bundle, envelope)
    return descriptor, work, resume, bundle


def record_descriptor_review(
    root: Path, kind: str, cycle: int, descriptor: dict[str, object]
) -> Path:
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
    return source


def reveal_and_seal_descriptors(
    root: Path, descriptors: dict[tuple[str, int], Path]
) -> None:
    candidate_head = campaign.load_campaign(root)["candidate_head"]
    revealed = campaign.reveal_qualification_profile(root, candidate_head)
    assert revealed["provisional_count"] == campaign.QUALIFICATION_WORK_COUNT
    assert revealed["profile_validation"] == "passed"
    for (kind, cycle), source in descriptors.items():
        campaign.seal_work(root, kind, cycle, source)


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
    locale: str,
) -> dict[str, object]:
    assert repository.name == "repository"
    assert kind in campaign.DOCUMENT_KINDS
    assert format_name in {name for name, _suffix in campaign.DOCUMENT_FORMATS}
    assert language == "en"
    assert locale == "en"
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
    locale: str,
) -> dict[str, object]:
    if kind == "implementation-plan":
        return {"status": "failed", "basis": "fixture document kind unavailable"}
    return documenter(
        binary,
        runtime,
        repository,
        kind,
        format_name,
        destination,
        language,
        locale,
    )


def assert_default_document_process_evidence(parent: Path, binary: Path) -> None:
    repository = parent / "document-process-repository"
    runtime = parent / "document-process-runtime"
    repository.mkdir()
    runtime.mkdir()
    destination = parent / "document-process/evidence/generated-documents/handoff.md"
    destination.parent.mkdir(parents=True)
    result = campaign.generate_document(
        binary,
        runtime,
        repository,
        "handoff-resume",
        "markdown",
        destination,
        "en",
        "en",
    )
    assert result["status"] == "passed"
    process = result["_process_result"]
    evidence = campaign.bounded_document_process_evidence(parent, process)
    assert evidence["outcome"] == "succeeded"
    assert evidence["exit_code"] == 0
    assert evidence["termination"] is None
    assert evidence["spawn_failed"] is False
    assert evidence["stdout"]["bytes"] > 0
    assert evidence["stderr"]["bytes"] == 0
    assert Path(process["stdout"]).is_file()
    assert Path(process["stderr"]).is_file()
    assert Path(process["stdout"]).with_name("command.json").is_file()
    assert Path(process["stdout"]).with_name("result.json").is_file()

    unavailable_destination = destination.with_name("unavailable.md")
    unavailable = campaign.generate_document(
        binary,
        runtime,
        repository,
        "handoff-resume",
        "markdown",
        unavailable_destination,
        "zz",
        "en",
    )
    assert unavailable["status"] == "failed"
    assert unavailable["product_result"]["outcome"] == "unavailable"
    assert "active-host realization unavailable" in unavailable["basis"]
    assert not unavailable_destination.exists()
    unavailable_process = unavailable["_process_result"]
    assert unavailable_process["exit_code"] == 0
    assert unavailable_process["termination"] is None
    assert Path(unavailable_process["stdout"]).is_file()
    assert Path(unavailable_process["stderr"]).is_file()


def snapshotter(
    _binary: Path,
    _runtime: Path,
    project_id: str,
    destination: Path,
    locale: str,
    language: str,
) -> dict[str, object]:
    assert campaign.PROJECT_ID.fullmatch(project_id)
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


def current_user_message_capture(
    source: Path,
    destination: Path,
    *,
    keep_legacy: bool = False,
    inject_host_user_material: bool = False,
    conflict_first_text: bool = False,
) -> Path:
    events = [
        json.loads(line) for line in source.read_text(encoding="utf-8").splitlines()
    ]
    session_id = events[0]["payload"]["session_id"]
    current_turn: str | None = None
    injected = False
    converted: list[dict[str, object]] = []
    for value in events:
        payload = value.get("payload", {})
        if value.get("type") == "event_msg" and payload.get("type") == "task_started":
            current_turn = payload.get("turn_id")
        if value.get("type") != "event_msg" or payload.get("type") != "user_message":
            converted.append(value)
            continue
        if not isinstance(current_turn, str):
            raise AssertionError("legacy fixture user message has no active turn")
        if inject_host_user_material and not injected:
            converted.append(
                {
                    "timestamp": "2026-08-15T00:00:00Z",
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "# AGENTS.md\nHost-injected instruction material",
                            }
                        ],
                    },
                }
            )
            injected = True
        if keep_legacy:
            converted.append(value)
        text = payload["message"]
        if conflict_first_text and not any(
            event.get("payload", {}).get("type") == "item_completed"
            for event in converted
        ):
            text += " conflicting"
        client_id = payload["client_id"]
        converted.append(
            {
                "timestamp": value.get("timestamp"),
                "type": "event_msg",
                "payload": {
                    "type": "item_completed",
                    "thread_id": session_id,
                    "turn_id": current_turn,
                    "item": {
                        "type": "UserMessage",
                        "id": f"item-{client_id}",
                        "client_id": client_id,
                        "content": [
                            {"type": "text", "text": text, "text_elements": []}
                        ],
                    },
                },
            }
        )
    destination.write_text(
        "".join(json.dumps(value, separators=(",", ":")) + "\n" for value in converted),
        encoding="utf-8",
    )
    return destination


def current_mcp_tool_call_capture(source: Path, destination: Path) -> Path:
    events = [
        json.loads(line) for line in source.read_text(encoding="utf-8").splitlines()
    ]
    session_id = events[0]["payload"]["session_id"]
    current_turn: str | None = None
    converted: list[dict[str, object]] = []
    for value in events:
        payload = value.get("payload", {})
        if value.get("type") == "event_msg" and payload.get("type") == "task_started":
            current_turn = payload.get("turn_id")
        if value.get("type") != "event_msg" or payload.get("type") != "mcp_tool_call_end":
            converted.append(value)
            continue
        if not isinstance(current_turn, str):
            raise AssertionError("legacy fixture MCP completion has no active turn")
        invocation = payload["invocation"]
        raw_result = payload["result"]
        ok = raw_result.get("Ok") if isinstance(raw_result, dict) else None
        if not isinstance(ok, dict):
            raise AssertionError("legacy fixture MCP result is not convertible")
        converted.append(
            {
                "timestamp": value.get("timestamp"),
                "type": "event_msg",
                "payload": {
                    "type": "item_completed",
                    "thread_id": session_id,
                    "turn_id": current_turn,
                    "item": {
                        "type": "McpToolCall",
                        "id": payload["call_id"],
                        "server": invocation["server"],
                        "tool": invocation["tool"],
                        "arguments": invocation["arguments"],
                        "status": "failed" if ok["isError"] else "completed",
                        "result": ok,
                    },
                },
            }
        )
    destination.write_text(
        "".join(json.dumps(value, separators=(",", ":")) + "\n" for value in converted),
        encoding="utf-8",
    )
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
    descriptors: dict[tuple[str, int], Path] = {}
    for kind in campaign.CLASSES:
        for cycle in campaign.work_labels(kind):
            descriptor, work, resume, bundle = fixture_for(
                parent / f"{name}-fixtures",
                kind,
                cycle,
                campaign_root=root,
            )
            descriptors[(kind, cycle)] = record_descriptor_review(
                root, kind, cycle, descriptor
            )
            captures.append(work)
            if "resume" in campaign.session_roles(kind, cycle):
                captures.append(resume)
            state = campaign.work_state(root, kind, cycle)
            bundles[state["review_slot_id"]] = bundle
    reveal_and_seal_descriptors(root, descriptors)
    return root, captures, bundles


def assert_same_path_candidate_replacement_rejected(parent: Path, binary: Path) -> None:
    root, captures, bundles = prepared_batch(
        parent, "same-path-candidate-replacement", binary
    )
    def campaign_snapshot() -> dict[str, bytes]:
        return {
            path.relative_to(root).as_posix(): path.read_bytes()
            for path in root.rglob("*")
            if path.is_file()
        }

    before = campaign_snapshot()
    original_run_checked = campaign.run_checked
    campaign.run_checked = fake_enable_command
    try:
        for artifact_name in ("volicord", "volicord-mcp"):
            path = binary.with_name(artifact_name)
            original = path.read_bytes()
            path.write_bytes(original + b"\n# same-path replacement\n")
            path.chmod(0o755)
            try:
                try:
                    campaign.activate_cycle(root, "volicord", 1)
                except campaign.CampaignError as error:
                    assert f"candidate executable content mismatch: {artifact_name}" in str(error)
                else:
                    raise AssertionError(
                        f"campaign activation accepted a same-path {artifact_name} replacement"
                    )
                assert campaign_snapshot() == before
            finally:
                path.write_bytes(original)
                path.chmod(0o755)

        viewer = binary.with_name("volicord-viewer")
        original = viewer.read_bytes()
        viewer.write_bytes(original + b"\n# same-path replacement\n")
        viewer.chmod(0o755)
        try:
            try:
                campaign.collect_batch(
                    root,
                    captures,
                    exporter=batch_exporter(bundles),
                    documenter=documenter,
                    snapshotter=snapshotter,
                )
            except campaign.CampaignError as error:
                assert "candidate executable content mismatch: volicord-viewer" in str(error)
            else:
                raise AssertionError(
                    "campaign collection accepted a same-path volicord-viewer replacement"
                )
            assert campaign_snapshot() == before
        finally:
            viewer.write_bytes(original)
            viewer.chmod(0o755)
    finally:
        campaign.run_checked = original_run_checked


def batch_exporter(bundles: dict[str, Path]):
    grouped: dict[str, list[dict[str, object]]] = {}
    for source in bundles.values():
        envelope = json.loads(source.read_text(encoding="utf-8"))
        grouped.setdefault(str(envelope["payload"]["project_id"]), []).append(envelope)

    merged_by_slot: dict[str, bytes] = {}
    for slot_id, source in bundles.items():
        source_envelope = json.loads(source.read_text(encoding="utf-8"))
        project_id = str(source_envelope["payload"]["project_id"])
        merged = copy.deepcopy(source_envelope)
        tables: dict[str, dict[str, object]] = {
            str(table["name"]): table for table in merged["payload"]["tables"]
        }
        for envelope in grouped[project_id]:
            for table in envelope["payload"]["tables"]:
                target = tables[str(table["name"])]
                known = {json.dumps(row, sort_keys=True) for row in target["rows"]}
                for row in table["rows"]:
                    encoded = json.dumps(row, sort_keys=True)
                    if encoded not in known:
                        target["rows"].append(row)
                        known.add(encoded)
        semantic = {
            "project_id": merged["payload"]["project_id"],
            "tables": merged["payload"]["tables"],
        }
        history_basis = hashlib.sha256(
            json.dumps(semantic, ensure_ascii=False, separators=(",", ":")).encode()
        ).hexdigest()
        merged["payload"]["lineage"] = {
            "common_base_basis": history_basis,
            "history_basis": history_basis,
        }
        merged["checksum"] = hashlib.sha256(
            json.dumps(merged["payload"], ensure_ascii=False, separators=(",", ":")).encode()
        ).hexdigest()
        merged_by_slot[slot_id] = campaign.json_bytes(merged)

    def export(_binary: Path, _runtime: Path, repository: Path, destination: Path) -> None:
        assert repository.name == "repository"
        destination.write_bytes(merged_by_slot[destination.parent.name])

    return export


ACTIVATION_ORDERING_FIXTURE = (
    Path(__file__).parent / "fixtures/vscode-session-start-ordering.jsonl"
)


def assert_session_start_ordering(parent: Path) -> None:
    """Minimal VS Code ordering and negative fixtures; no private rollout data."""
    original = [json.loads(line) for line in ACTIVATION_ORDERING_FIXTURE.read_text().splitlines()]
    variants = {"vscode": (copy.deepcopy(original), "valid")}

    def variant(label: str, state: str) -> list[dict]:
        events = copy.deepcopy(original)
        variants[label] = events, state
        return events

    events = variant("before-task-started", "valid")
    events.insert(1, events.pop(7))
    events = variant("after-visible-user-before-user-event", "late")
    events.insert(8, events.pop(7))
    events = variant("after-legacy-user-event", "late")
    events.insert(9, events.pop(7))
    events = variant("after-mcp-without-user", "late")
    activation = events.pop(7)
    del events[7:9]
    events.insert(9, activation)
    events = variant("after-repository-command-without-user", "late")
    activation = events.pop(7)
    del events[7:9]
    events[7]["payload"]["input"] = 'const r = await tools.exec_command({"cmd":"rg --files","workdir":"/fixture/repository"}); text(JSON.stringify(r));'
    del events[8]
    events.insert(8, activation)  # A pending command already proves work began.
    events = variant("missing-with-later-mcp", "absent")
    del events[7]
    for label in ("wrong-repository", "wrong-session", "late-wrong-binding"):
        events = variant(label, "binding_mismatch")
        events[7]["payload"]["content"][0]["text"] = activation_identity(
            Path("/another/repository" if label != "wrong-session" else "/fixture/repository"),
            "another-session" if label == "wrong-session" else "fixture-session",
        )
        if label == "late-wrong-binding":
            events.insert(9, events.pop(7))
    for label in ("malformed", "late-malformed", "conflicting"):
        events = variant(label, "malformed")
        item = events[7]["payload"]["content"][0]
        identity = item["text"].splitlines()[0]
        item["text"] = identity[:-1] if label != "conflicting" else identity + "\n" + identity
        if label == "late-malformed":
            events.insert(9, events.pop(7))
    events = variant("conflict-across-messages", "binding_mismatch")
    conflict = copy.deepcopy(variants["wrong-session"][0][7])
    events.insert(10, conflict)
    events = variant("repeated-valid-context", "valid")
    events.insert(10, copy.deepcopy(events[7]))
    events = variant("unknown-setup-ordering", "indeterminate")
    events[4]["payload"]["content"] = [{"type": "input_text", "text": "Uncorrelated host or user text."}]
    events = variant("missing-with-unknown-setup", "indeterminate")
    events[4]["payload"]["content"] = [{"type": "input_text", "text": "Uncorrelated host or user text."}]
    del events[7]
    events = variant("mixed-setup-and-task", "indeterminate")
    events[4]["payload"]["content"].append({"type": "input_text", "text": "Extra uncorrelated task."})
    events = variant("unknown-pre-activation-event", "indeterminate")
    events.insert(7, {"type": "unfamiliar_transport", "payload": {}})
    events = variant("unsupported-pre-activation-tool", "indeterminate")
    events.insert(7, {"type": "response_item", "payload": {
        "type": "custom_tool_call", "name": "unknown_tool", "input": "unknown work",
    }})
    events = variant("incomplete-capture", "indeterminate")
    del events[8:]
    events = variant("unsupported-developer-content", "indeterminate")
    events[7]["payload"]["content"] = {"text": "Unsupported context transport"}
    events = variant("unsupported-developer-after-user", "indeterminate")
    events[7]["payload"]["content"] = {"text": "Unsupported context transport"}
    events.insert(9, events.pop(7))
    events = variant("unreadable-context-before-late-identity", "indeterminate")
    events[2]["payload"]["content"] = {"text": "Unsupported context transport"}
    events.insert(9, events.pop(7))
    events = variant("unreadable-context-after-valid-identity", "indeterminate")
    events.insert(10, {"type": "response_item", "payload": {
        "type": "message", "role": "developer", "content": {"text": "Unsupported context transport"},
    }})
    events = variant("uncorrelated-response-user", "indeterminate")
    del events[9:]
    events.insert(8, events.pop(7))
    for label, (events, expected) in list(variants.items()):
        path = parent / f"ordering-{label}.jsonl"
        path.write_text("".join(json.dumps(event) + "\n" for event in events))
        for transport in ("legacy", "current"):
            source = path if transport == "legacy" else current_user_message_capture(
                path, parent / f"ordering-{label}-current.jsonl"
            )
            capture = harness.load_codex_capture(source)
            assert capture.activation_evidence_state == expected, (label, transport, capture.activation_evidence_state)
            assert capture.repository_scoped_activation_observed == (expected == "valid"), label
            if label in {"vscode", "missing-with-later-mcp", "after-mcp-without-user"}:
                assert capture.tool_calls, label
            failure = harness.activation_failure(capture)
            assert failure == (None if expected == "valid" else harness.ACTIVATION_FAILURES[expected]), label
    # Current UserMessage alone is also a boundary; no response_item copy needed.
    events = copy.deepcopy(original)
    del events[8]
    path = parent / "current-user-only.jsonl"
    path.write_text("".join(json.dumps(event) + "\n" for event in events))
    current = current_user_message_capture(path, parent / "current-user-only-converted.jsonl")
    assert harness.load_codex_capture(current).activation_evidence_state == "valid"
    events = [json.loads(line) for line in current.read_text().splitlines()]
    events.insert(8, events.pop(7))
    current.write_text("".join(json.dumps(event) + "\n" for event in events))
    assert harness.load_codex_capture(current).activation_evidence_state == "late"
    # Completion-only current MCP and FileChange representations prove earlier
    # work even when neither a user message nor a custom invocation was retained.
    path = parent / "ordering-after-mcp-without-user.jsonl"
    current = current_mcp_tool_call_capture(path, parent / "current-mcp-before-activation.jsonl")
    events = [json.loads(line) for line in current.read_text().splitlines()]
    del events[7]
    current.write_text("".join(json.dumps(event) + "\n" for event in events))
    assert harness.load_codex_capture(current).activation_evidence_state == "late"
    events[7]["payload"]["item"] = {
        "type": "FileChange", "id": "fixture-patch", "status": "completed",
        "stdout": "", "stderr": "", "changes": {
            "src/main.py": {"type": "update", "unified_diff": "@@ -1 +1 @@\n-old\n+new\n", "move_path": None},
        },
    }
    current.write_text("".join(json.dumps(event) + "\n" for event in events))
    assert harness.load_codex_capture(current).activation_evidence_state == "late"


def use_vscode_activation_ordering(path: Path) -> None:
    events = [json.loads(line) for line in path.read_text().splitlines()]
    template = ACTIVATION_ORDERING_FIXTURE.read_text().replace(
        "/fixture/repository", events[0]["payload"]["cwd"]
    )
    setup = [json.loads(line) for line in template.splitlines()][2:7]
    setup[-1]["payload"]["turn_id"] = events[2]["payload"]["turn_id"]
    task = copy.deepcopy(events[3])
    visible = {"type": "response_item", "payload": {
        "type": "message", "role": "user", "content": [
            {"type": "input_text", "text": task["payload"]["message"]},
        ],
    }}
    events = [events[0], events[2], *setup, events[1], visible, *events[3:]]
    path.write_text("".join(json.dumps(event) + "\n" for event in events))


def assert_production_session_start(parent: Path, binary: Path) -> None:
    """Sanitized eight-session production hook -> collect-batch parser regression."""
    root, captures, bundles = prepared_batch(parent, "production-activation", binary)
    mapped = campaign.map_batch_rollouts(root, captures)
    assert len(mapped) == 8
    assert all(item.capture.repository_scoped_activation_observed for item in mapped.values())
    assert all(item.capture.tool_calls for item in mapped.values())
    work = next(path for path in captures if path.name == "volicord-1-work-events.jsonl")
    original = [json.loads(line) for line in work.read_text().splitlines()]
    context = original[1]["payload"]["content"][0]["text"]
    identity = context.splitlines()[0]
    assert context != identity  # Actual production guidance, not just a marker fixture.
    controls = {
        "missing": (None, "absent"),
        "unrelated": ("Volicord project_resolve workflow.required_next_action", "absent"),
        "truncated": (identity[:-1], "malformed"),
        "unsupported": (identity + ":unknown", "malformed"),
        "wrong-binding": (identity[:-1] + ("0" if identity[-1] != "0" else "1"), "binding_mismatch"),
        "prose-change": (identity + "\nReworded human guidance.", "valid"),
        "late": (context, "late"),
        "wrong-repository": (context, "binding_mismatch"),
        "wrong-session": (context, "binding_mismatch"),
    }
    for label, (replacement, expected) in controls.items():
        events = copy.deepcopy(original)
        if replacement is None:
            del events[1]
        else:
            events[1]["payload"]["content"][0]["text"] = replacement
        if label == "late":
            events.insert(4, events.pop(1))
        elif label == "wrong-repository":
            events[0]["payload"]["cwd"] += "/another"
        elif label == "wrong-session":
            events[0]["payload"]["id"] += "-another"
            events[0]["payload"]["session_id"] += "-another"
        path = parent / f"production-activation-{label}.jsonl"
        path.write_text("\n".join(json.dumps(event) for event in events) + "\n")
        capture = harness.load_codex_capture(path)
        assert capture.activation_evidence_state == expected, label
        assert capture.repository_scoped_activation_observed == (expected == "valid"), label
        assert capture.tool_calls  # MCP cannot rescue any failed activation control.
    for path in captures:
        use_vscode_activation_ordering(path)
    mapped = campaign.map_batch_rollouts(root, captures)
    assert len(mapped) == 8
    assert all(item.capture.activation_evidence_state == "valid" for item in mapped.values())
    summary = campaign.collect_batch(root, captures, exporter=batch_exporter(bundles))
    assert summary["outcome"] == "evidence_collected"
    assert not summary["failure_attribution"]


def assert_activation_failure_attribution(parent: Path, binary: Path) -> None:
    for label, domain, outcome in (
        ("absent", "environment", "operator_environment_invalid"),
        ("late", "environment", "operator_environment_invalid"),
        ("malformed", "evidence", "evidence_failed"),
        ("binding_mismatch", "evidence", "evidence_failed"),
        ("indeterminate", "evidence", "evidence_failed"),
        ("validator_mismatch", "validation_internal", "evidence_failed"),
    ):
        root, captures, bundles = prepared_batch(parent, f"attribution-{label}", binary)
        originals = {path: path.read_bytes() for path in captures}
        if label != "validator_mismatch":
            for path in captures:
                events = [json.loads(line) for line in path.read_text().splitlines()]
                if label == "absent":
                    del events[1]
                elif label == "late":
                    events.insert(4, events.pop(1))
                elif label == "indeterminate":
                    events.insert(1, {"type": "response_item", "payload": {
                        "type": "message", "role": "user", "content": [
                            {"type": "input_text", "text": "Uncorrelated setup or task."},
                        ],
                    }})
                else:
                    item = events[1]["payload"]["content"][0]
                    identity, prose = item["text"].split("\n", 1)
                    if label == "malformed":
                        identity = identity[:-1]
                    else:
                        identity = identity[:-1] + ("0" if identity[-1] != "0" else "1")
                    item["text"] = identity + "\n" + prose
                path.write_text("\n".join(json.dumps(event) for event in events) + "\n")

        original_loader = campaign.load_codex_capture

        def inconsistent_validator(path: Path):
            capture = original_loader(path)
            assert capture.activation_evidence_state == "valid"
            assert capture.repository_scoped_activation_observed
            # Inject the old false-negative shape after recognition of real hook evidence.
            return replace(capture, repository_scoped_activation_observed=False)

        if label == "validator_mismatch":
            campaign.load_codex_capture = inconsistent_validator
        before = {name: (root / name).read_bytes() for name in ("campaign.json", "evidence-inventory.json")}
        try:
            campaign.collect_batch(root, captures, exporter=batch_exporter(bundles))
        except campaign.CampaignError as error:
            assert error.diagnostic["failure_attribution"]["domain"] == domain
        else:
            raise AssertionError("invalid activation was published")
        finally:
            campaign.load_codex_capture = original_loader
        assert not (root / "evidence-set.json").exists()
        assert all((root / name).read_bytes() == data for name, data in before.items())


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
    generated_assignments = campaign.new_private_work_obligation_assignments()
    campaign.validate_private_work_obligation_assignments(generated_assignments)
    coverage = Counter(
        obligation
        for _kind, _work, obligations in generated_assignments
        for obligation in obligations
    )
    assert all(
        coverage[obligation] >= minimum
        for obligation, minimum in harness.PRIVATE_OBLIGATION_MINIMUMS.items()
    )
    assert len({
        kind
        for kind, _work, obligations in generated_assignments
        if "hidden_user_owned_decision" in obligations
    }) >= 2
    campaign_source = Path(campaign.__file__).read_text(encoding="utf-8")
    assert "index(materiality_obligations)" not in campaign_source
    assert "MATERIALITY_OBLIGATIONS[" not in campaign_source
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
    assert len(state["works"]) == campaign.QUALIFICATION_WORK_COUNT
    serialized_state = json.dumps(state, sort_keys=True)
    assert not any(value in serialized_state for value in campaign.MATERIALITY_OBLIGATIONS)
    assert "commitment_nonce" not in serialized_state
    assert state["provisional_count"] == 0
    assert state["qualification_profile_state"] == "hidden"
    memory = state["naturalistic_memory_evidence"]
    assert memory["status"] == "unsupported_current_architecture"
    assert memory["candidate_mcp_sha256"] == state["candidate_artifacts"]["volicord-mcp"]["sha256"]
    assert memory["process_ownership"] == "vscode_codex_host_external_to_campaign_harness"
    assert memory["measurement"]["peak_rss_bytes"] is None
    assert memory["measurement"]["sample_count"] == 0
    assert memory["technical_gate_rss_evidence"] == "retained_separately_not_relabelled_naturalistic"
    assert all(value is False for value in memory["privacy"].values())
    obligations = state["live_evidence_obligations"]
    assert obligations["long_lived_project"] == {
        "status": "required_live_observation",
        "scope": "one_candidate_bound_project_multiple_distinct_work_items_across_fresh_sessions",
        "deterministic_fixture": "supporting_regression_only_not_qualification_evidence",
        "required_surface": "long_lived_project_observation",
    }
    assert obligations["viewer_performance"]["browser_input_and_paint"] \
        == "unmeasured_until_direct_live_observation"
    assert obligations["viewer_performance"]["proxy_may_be_relabelled_browser_latency"] is False
    assert obligations["naturalistic_resource"] == memory
    slots = [item["review_slot_id"] for item in state["works"].values()]
    assert len(slots) == len(set(slots)) == campaign.QUALIFICATION_WORK_COUNT
    assert all(campaign.REVIEW_SLOT_ID.fullmatch(slot) for slot in slots)
    for kind in campaign.CLASSES:
        ordered_works = [
            item["work_label"]
            for item in sorted(
                (
                    value
                    for value in state["works"].values()
                    if value["repository_class"] == kind
                ),
                key=lambda value: value["review_slot_id"],
            )
        ]
        if len(campaign.work_labels(kind)) > 1:
            assert ordered_works != list(campaign.work_labels(kind))
        assert sum(
            item["repository_class"] == kind for item in state["works"].values()
        ) == len(campaign.WORK_SLOTS_BY_REPOSITORY[kind])
    assert all("materiality_obligations" not in item for item in state["works"].values())
    private_mapping = campaign.read_json(campaign.slot_mapping_path(root))
    private_coverage = Counter(
        obligation
        for item in private_mapping["entries"]
        for obligation in item["materiality_obligations"]
    )
    assert all(
        private_coverage[obligation] >= minimum
        for obligation, minimum in harness.PRIVATE_OBLIGATION_MINIMUMS.items()
    )
    assert len({
        item["repository_class"]
        for item in private_mapping["entries"]
        if "hidden_user_owned_decision" in item["materiality_obligations"]
    }) >= 2
    for value in state["works"].values():
        slot = value["review_slot_id"]
        obvious = hashlib.sha256(
            f"{value['repository_class']}:{value['work_label']}".encode("utf-8")
        ).hexdigest()[:32]
        assert slot != obvious
        journey = state["journeys"][value["journey_id"]]
        assert value["repository_path"] == journey["repository_path"]
        assert value["runtime_home"] == journey["runtime_home"]
        assert Path(value["reviewer_repository_path"]).resolve() == (
            root / "reviewer/workspaces" / slot / "repository"
        ).resolve()

    index = campaign.read_json(campaign.reviewer_index_path(root))
    assert index["ordering"] == "opaque_review_slot_id"
    assert index["provisional_review_contract"] == (
        campaign.reviewer_provisional_contract_reference(root)
    )
    assert index["preflight_operation"] == "validate-provisional-review"
    assert index["preflight_mutates_campaign"] is False
    assert [entry["review_slot_id"] for entry in index["entries"]] == sorted(slots)
    assert all("provisional_review_template" not in entry for entry in index["entries"])
    assert all(
        entry["provisional_review_draft"] == f"drafts/{entry['review_slot_id']}.json"
        for entry in index["entries"]
    )
    assert not (root / "reviewer/templates").exists()
    serialized_index = json.dumps(index, sort_keys=True)
    assert "repository_class" not in serialized_index
    assert "logical_cycle" not in serialized_index
    assert not any(value in serialized_index for value in campaign.MATERIALITY_OBLIGATIONS)
    reviewer_safe_paths = [
        root / "campaign.json",
        root / "preparation.json",
        root / "reviewer/index.json",
        root / "operator/RUN-SHEET.md",
    ]
    for reviewer_safe_path in reviewer_safe_paths:
        reviewer_safe_text = reviewer_safe_path.read_text(encoding="utf-8")
        assert "behavior_histogram" not in reviewer_safe_text
        assert "hidden_repository_classes" not in reviewer_safe_text
        assert not any(
            behavior in reviewer_safe_text for behavior in campaign.MATERIALITY_OBLIGATIONS
        )

    # The remaining historical assertions below exercised the retired
    # one-behavior-per-cycle profile.  Current profile and immutable-intake
    # tamper coverage is exercised by assert_journey_campaign_contract.
    return

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
    first_key, second_key = list(changed_campaign["works"])[:2]
    changed_campaign["works"][first_key]["review_slot_id"], changed_campaign["works"][
        second_key
    ]["review_slot_id"] = (
        changed_campaign["works"][second_key]["review_slot_id"],
        changed_campaign["works"][first_key]["review_slot_id"],
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

    invalid_mapping = json.loads(original_mapping)
    for entry in invalid_mapping["entries"]:
        entry["expected_behavior_class"] = "explicit_user_owned_decision"
    campaign.write_json(mapping_path, invalid_mapping)
    invalid_campaign = json.loads(original_campaign)
    invalid_campaign["opaque_slot_mapping_sha256"] = harness.sha256(mapping_path)
    invalid_profile = campaign.qualification_profile_value(
        invalid_campaign["campaign_id"],
        invalid_campaign["candidate_head"],
        invalid_mapping,
    )
    campaign.write_json(campaign.qualification_profile_path(root), invalid_profile)
    invalid_campaign["qualification_profile_sha256"] = harness.sha256(
        campaign.qualification_profile_path(root)
    )
    campaign.write_json(campaign_path, invalid_campaign)
    try:
        campaign.load_campaign(root)
    except campaign.CampaignError as error:
        assert "behavior assignment violates qualification constraints" in str(error)
    else:
        raise AssertionError("malformed post-reveal private qualification profile passed")
    mapping_path.write_bytes(original_mapping)
    campaign_path.write_bytes(original_campaign)
    campaign.write_json(
        campaign.qualification_profile_path(root),
        campaign.qualification_profile_value(
            state["campaign_id"], state["candidate_head"], json.loads(original_mapping)
        ),
    )
    campaign.load_campaign(root)


def assert_blockers(parent: Path, binary: Path) -> None:
    blocker_root, blocker_captures, _blocker_bundles = prepared_batch(
        parent, "blocker-campaign", binary
    )
    work = next(path for path in blocker_captures if path.name == "volicord-1-work-events.jsonl")
    resume = next(path for path in blocker_captures if path.name == "volicord-1-resume-events.jsonl")
    broken = filtered_capture(work, parent / "missing-completions.jsonl", '"type":"mcp_tool_call_end"')
    result = collect_work_fixture(blocker_root, "volicord", 1, broken)
    assert result["outcome"] == "review_required"
    assert result["failure_attribution"] == {
        "domain": "behavior_contract",
        "basis": "maintained_work_behavior_contract_failed",
        "failed_checks": result["failed_checks"],
    }
    assert campaign.load_campaign(blocker_root)["terminal_outcome"] is None
    assert campaign.load_campaign(blocker_root)["works"]["volicord-cycle-1"]["state"] == "work_collected"

    activation_root, activation_captures, _activation_bundles = prepared_batch(
        parent, "activation-campaign", binary
    )
    activation_work = next(
        path for path in activation_captures if path.name == "volicord-1-work-events.jsonl"
    )
    missing = filtered_capture(
        activation_work,
        parent / "missing-activation.jsonl",
        harness.ACTIVATION_PREFIX,
    )
    invalid = collect_work_fixture(activation_root, "volicord", 1, missing)
    assert invalid["outcome"] == "operator_environment_invalid"
    assert invalid["classification"] == "operator_environment_setup_failure"
    assert invalid["failure_attribution"] == {
        "domain": "environment",
        "basis": "repository_session_activation_missing",
        "failed_checks": [harness.SETUP_ACTIVATION_CHECK],
    }

    evidence_root, evidence_captures, _evidence_bundles = prepared_batch(
        parent, "evidence-transport-campaign", binary
    )
    evidence_work = next(
        path
        for path in evidence_captures
        if path.name == "volicord-1-work-events.jsonl"
    )
    current_evidence_work = current_mcp_tool_call_capture(
        evidence_work,
        parent / "evidence-current-work.jsonl",
    )
    events = [
        json.loads(line)
        for line in current_evidence_work.read_text(encoding="utf-8").splitlines()
    ]
    project_item = next(
        value["payload"]["item"]
        for value in events
        if value.get("payload", {}).get("type") == "item_completed"
        and value.get("payload", {}).get("item", {}).get("type") == "McpToolCall"
        and value["payload"]["item"].get("tool") == "project_initialize"
    )
    project_item["result"].pop("structuredContent")
    malformed = parent / "evidence-malformed-project-work.jsonl"
    malformed.write_text(
        "".join(
            json.dumps(value, separators=(",", ":")) + "\n" for value in events
        ),
        encoding="utf-8",
    )
    evidence_result = collect_work_fixture(
        evidence_root, "volicord", 1, malformed
    )
    assert evidence_result["outcome"] == "review_required"
    assert evidence_result["classification"] == "evidence_transport_failure"
    assert evidence_result["evidence_transport"]["state"] == "indeterminate"
    assert evidence_result["failure_attribution"] == {
        "domain": "evidence",
        "basis": "required_evidence_transport_indeterminate",
        "failed_checks": evidence_result["failed_checks"],
    }


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
        for classification in campaign.MATERIALITY_OBLIGATIONS:
            label = classification.replace("_", "-")
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
            assert campaign.work_state(root, "volicord", 1)["state"] == "provisional_recorded"
            results.append(result)
            recorded_paths.append(fixed)
    finally:
        harness.classification_comparison_errors = original_comparison
        campaign.read_json = original_read_json

    assert len(results) == len(recorded_paths) == len(campaign.MATERIALITY_OBLIGATIONS)
    assert all(set(result) == set(results[0]) for result in results[1:])
    for result in results:
        serialized = json.dumps(result, sort_keys=True)
        assert result["state"] == "provisional_recorded"
        assert result["evaluator_material_exposed"] is False
        assert "match" not in serialized.casefold()
        assert "expected" not in serialized.casefold()
        assert not any(value in serialized for value in campaign.MATERIALITY_OBLIGATIONS)
        assert "repository_class" not in serialized
        assert "logical_cycle" not in serialized
    for field in ("kind", "candidate_head", "state", "evaluator_material_exposed"):
        assert all(result[field] == results[0][field] for result in results[1:])
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
    validate_help = subprocess.run(
        [str(helper), "validate-provisional-review", "--help"],
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
    reveal_help = subprocess.run(
        [str(helper), "reveal-qualification-profile", "--help"],
        text=True,
        capture_output=True,
        check=False,
    )
    assert (
        record_help.returncode
        == validate_help.returncode
        == seal_help.returncode
        == reveal_help.returncode
        == 0
    )
    assert "--review-slot-id" in record_help.stdout
    assert "--provisional-review" in record_help.stdout
    assert "--review-slot-id" in validate_help.stdout
    assert "--provisional-review" in validate_help.stdout
    assert "--provisional-review" not in seal_help.stdout
    assert "--candidate-head" in reveal_help.stdout
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
        collect_work_fixture(root, "volicord", 1, parent / "absent-rollout.jsonl")
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
    serialized_preparation_result = json.dumps(preparation, sort_keys=True)
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
        "provisional_review_contract",
        "preflight",
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
    assert not any(value in serialized_preparation for value in campaign.MATERIALITY_OBLIGATIONS)
    assert "behavior_histogram" not in serialized_preparation_result
    assert "hidden_repository_classes" not in serialized_preparation_result
    assert not any(
        value in serialized_preparation_result for value in campaign.MATERIALITY_OBLIGATIONS
    )
    contract_path = campaign.reviewer_provisional_contract_path(root)
    contract = campaign.read_json(contract_path)
    assert contract == harness.provisional_review_contract()
    contract_keys = campaign.json_keys(contract)
    assert not contract_keys.intersection(
        {
            "materiality_obligations",
            "repository_class",
            "logical_cycle",
            "evaluation_basis",
            "evaluator_classification",
            "expected_question",
            "expected_decision",
            "counterfactual_review",
            "hidden_behavior_review_conclusion",
            "slot_to_behavior_mapping",
            "qualification_profile",
            "qualification_profile_truth",
        }
    )
    assert preparation_body["provisional_review_contract"] == {
        "path": campaign.relative(root, contract_path),
        "sha256": harness.sha256(contract_path),
    }
    assert contract["artifact_ownership"] == {
        "preparation": "read_only_inventory_bound_campaign_evidence",
        "draft_path": "reviewer/drafts/<review_slot_id>.json",
        "draft": "reviewer_owned_mutable_work_product_before_recording",
        "draft_mutable_before_recording": True,
        "draft_inventory_bound_before_recording": False,
        "recorded_path": "reviewer/provisional/<review_slot_id>.json",
        "recorded_provisional": "immutable_inventory_bound_campaign_evidence",
        "recorded_bytes": "exact_accepted_input_bytes",
    }
    assert contract["preflight"]["inventory_bound_campaign_artifact_allowed_as_input"] is False
    assert preparation_body["preflight"] == {
        "operation": "validate-provisional-review",
        "mutation": "none",
    }
    assert "provisional_review_template" not in preparation
    assert preparation["provisional_review_draft_ownership"] == (
        "reviewer_owned_mutable_before_recording"
    )
    assert preparation["provisional_review_draft_inventory_bound"] is False
    provisional_path = root / preparation["provisional_review_draft"]
    provisional_draft = campaign.read_json(provisional_path)
    assert provisional_draft["_draft_state"] == (
        "INCOMPLETE_REMOVE_THIS_FIELD_BEFORE_PREFLIGHT"
    )
    assert provisional_draft["status"] == "recorded"
    assert provisional_draft["classification"] is None
    assert provisional_draft["materiality_conclusion"] is None
    assert provisional_draft["basis"] == ""
    assert provisional_draft["provenance_reference_indices"] == []
    draft_inventory_name = campaign.relative(root, provisional_path)
    assert draft_inventory_name not in campaign.load_inventory(root)["artifacts"]
    campaign.verify_inventory(root)
    provisional = copy.deepcopy(
        descriptor["behavior_review"]["independent_review"]["provisional_review"]
    )
    provisional["preparation_sha256"] = preparation["preparation_sha256"]
    provisional["review_slot_id"] = review_slot_id
    campaign.write_json(provisional_path, provisional)
    campaign.verify_inventory(root)
    assert draft_inventory_name not in campaign.load_inventory(root)["artifacts"]

    preparation_path = root / preparation["preparation"]
    original_preparation = preparation_path.read_bytes()
    campaign_before_preparation_corruption = campaign.campaign_file(root).read_bytes()
    preparation_path.write_bytes(original_preparation + b" ")
    try:
        campaign.record_provisional_review(
            root, campaign.load_campaign(root)["candidate_head"], review_slot_id, provisional_path
        )
    except campaign.CampaignError as error:
        assert "evidence hash mismatch" in str(error)
    else:
        raise AssertionError("corrupt inventory-bound preparation allowed campaign mutation")
    assert campaign.campaign_file(root).read_bytes() == campaign_before_preparation_corruption
    assert not campaign.reviewer_provisional_path(root, "volicord", 1).exists()
    preparation_path.write_bytes(original_preparation)
    campaign.verify_inventory(root)

    def reviewer_plane_snapshot() -> tuple[tuple[str, int, str], ...]:
        return tuple(
            (
                path.relative_to(root).as_posix(),
                path.stat().st_size,
                harness.sha256(path),
            )
            for path in sorted(root.rglob("*"))
            if path.is_file()
        )

    candidate_head = campaign.load_campaign(root)["candidate_head"]
    valid_preflight_paths: list[Path] = []
    preflight_read_paths: list[Path] = []
    original_read_json = campaign.read_json

    def track_preflight_read(path: Path):
        preflight_read_paths.append(Path(path).resolve())
        return original_read_json(path)

    campaign.read_json = track_preflight_read
    try:
        for classification in campaign.MATERIALITY_OBLIGATIONS:
            valid = copy.deepcopy(provisional)
            set_provisional_classification(valid, classification)
            valid_path = parent / f"{review_slot_id}-{classification}-preflight.json"
            campaign.write_json(valid_path, valid)
            before = reviewer_plane_snapshot()
            result = campaign.validate_provisional_review(
                root, candidate_head, review_slot_id, valid_path
            )
            assert reviewer_plane_snapshot() == before
            assert result["status"] == "passed"
            assert result["campaign_mutated"] is False
            assert result["validation_semantics"] == "shared_with_record-provisional-review"
            serialized_result = json.dumps(result, sort_keys=True)
            assert not any(value in serialized_result for value in campaign.MATERIALITY_OBLIGATIONS)
            assert "repository_class" not in serialized_result
            assert "logical_cycle" not in serialized_result
            assert "evaluation_basis" not in serialized_result
            assert "expected" not in serialized_result.casefold()
            valid_preflight_paths.append(valid_path)
    finally:
        campaign.read_json = original_read_json
    assert len(valid_preflight_paths) == len(campaign.MATERIALITY_OBLIGATIONS)
    assert set(preflight_read_paths) == {
        contract_path.resolve(),
        campaign.inventory_path(root).resolve(),
        (root / preparation["preparation"]).resolve(),
    }
    before_cli_preflight = reviewer_plane_snapshot()
    cli_preflight = subprocess.run(
        [
            str(helper),
            "validate-provisional-review",
            "--campaign-root",
            str(root),
            "--candidate-head",
            candidate_head,
            "--review-slot-id",
            review_slot_id,
            "--provisional-review",
            str(valid_preflight_paths[0]),
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    assert cli_preflight.returncode == 0, cli_preflight.stdout + cli_preflight.stderr
    assert json.loads(cli_preflight.stdout)["campaign_mutated"] is False
    assert reviewer_plane_snapshot() == before_cli_preflight

    before_inventory_bound_misuse = reviewer_plane_snapshot()
    try:
        campaign.validate_provisional_review(
            root,
            candidate_head,
            review_slot_id,
            root / preparation["preparation"],
        )
    except campaign.CampaignError as error:
        assert "inventory-bound campaign artifact" in str(error)
        assert "mutable reviewer draft" in str(error)
    else:
        raise AssertionError("inventory-bound campaign evidence passed reviewer preflight")
    assert reviewer_plane_snapshot() == before_inventory_bound_misuse

    invalid_preflights: list[dict[str, object]] = []
    for status in ("pending", "complete"):
        invalid = copy.deepcopy(provisional)
        invalid["status"] = status
        invalid_preflights.append(invalid)
    invented = copy.deepcopy(provisional)
    invented["classification"] = "implementation_policy_choice"
    invalid_preflights.append(invented)
    free_form_materiality = copy.deepcopy(provisional)
    free_form_materiality["materiality_conclusion"] = "This seems material to the user."
    invalid_preflights.append(free_form_materiality)
    wrong_preparation_hash = copy.deepcopy(provisional)
    wrong_preparation_hash["preparation_sha256"] = "00" * 32
    invalid_preflights.append(wrong_preparation_hash)
    wrong_slot = copy.deepcopy(provisional)
    wrong_slot["review_slot_id"] = "00" * 16
    invalid_preflights.append(wrong_slot)
    for indices in ([], [1], [-1], [0, 0], [True]):
        invalid = copy.deepcopy(provisional)
        invalid["provenance_reference_indices"] = indices
        invalid_preflights.append(invalid)
    for classification in (
        "explicit_user_owned_decision",
        "hidden_user_owned_decision",
    ):
        for field, value in (
            ("materiality_conclusion", "no_user_owned_material_outcome"),
            ("material_outcome_unavoidable", False),
            (
                "operator_prompt_does_not_disclose_material_outcome",
                classification != "hidden_user_owned_decision",
            ),
        ):
            invalid = copy.deepcopy(provisional)
            set_provisional_classification(invalid, classification)
            invalid[field] = value
            invalid_preflights.append(invalid)
    for field, value in (
        ("materiality_conclusion", "user_owned_material_outcome"),
        ("material_outcome_unavoidable", True),
        ("operator_prompt_does_not_disclose_material_outcome", False),
    ):
        invalid = copy.deepcopy(provisional)
        set_provisional_classification(invalid, "research_or_no_question")
        invalid[field] = value
        invalid_preflights.append(invalid)
    for number, invalid in enumerate(invalid_preflights):
        invalid_path = parent / f"{review_slot_id}-invalid-preflight-{number}.json"
        campaign.write_json(invalid_path, invalid)
        before = reviewer_plane_snapshot()
        try:
            campaign.validate_provisional_review(
                root, candidate_head, review_slot_id, invalid_path
            )
        except campaign.CampaignError as error:
            assert "does not qualify" in str(error)
        else:
            raise AssertionError("invalid provisional review passed reviewer preflight")
        assert reviewer_plane_snapshot() == before
    invalid_cli_path = parent / f"{review_slot_id}-invalid-preflight-0.json"
    before_invalid_cli = reviewer_plane_snapshot()
    invalid_cli = subprocess.run(
        [
            str(helper),
            "validate-provisional-review",
            "--campaign-root",
            str(root),
            "--candidate-head",
            candidate_head,
            "--review-slot-id",
            review_slot_id,
            "--provisional-review",
            str(invalid_cli_path),
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    assert invalid_cli.returncode == 1
    assert json.loads(invalid_cli.stdout)["status"] == "failed"
    assert reviewer_plane_snapshot() == before_invalid_cli

    original_contract = contract_path.read_bytes()
    stale_contract = copy.deepcopy(contract)
    stale_contract["fixed_values"]["status"] = "complete"
    campaign.write_json(contract_path, stale_contract)
    before_stale_preflight = reviewer_plane_snapshot()
    try:
        campaign.validate_provisional_review(
            root, candidate_head, review_slot_id, provisional_path
        )
    except campaign.CampaignError as error:
        assert "stale or contradictory" in str(error)
    else:
        raise AssertionError("stale reviewer contract passed preflight")
    assert reviewer_plane_snapshot() == before_stale_preflight
    contract_path.write_bytes(original_contract)
    campaign.verify_inventory(root)
    assert not campaign.reviewer_provisional_path(root, "volicord", 1).exists()
    assert not campaign.evaluator_descriptor_path(root, "volicord", 1).exists()
    try:
        campaign.seal_work(root, "volicord", 1, draft_path)
    except campaign.CampaignError as error:
        assert "all eight provisional reviews" in str(error)
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
    evaluator_leak["evaluation_basis"] = {"materiality_obligations": "hidden"}
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
        before_campaign = campaign.load_campaign(root, validate_private=False)
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
        after_campaign = campaign.load_campaign(root, validate_private=False)
        assert after_campaign["provisional_count"] == before_campaign["provisional_count"] == 0
        assert after_campaign["qualification_profile_state"] == before_campaign[
            "qualification_profile_state"
        ] == "hidden"

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

    shared_validation_calls = {"count": 0}
    original_shared_validation = campaign.load_and_validate_reviewer_provisional_review

    def tracking_shared_validation(*args, **kwargs):
        shared_validation_calls["count"] += 1
        return original_shared_validation(*args, **kwargs)

    campaign.read_json = tracking_read_json
    campaign.load_and_validate_reviewer_provisional_review = tracking_shared_validation
    try:
        recorded = campaign.record_provisional_review(
            root,
            candidate_head,
            review_slot_id,
            provisional_path,
        )
    finally:
        campaign.read_json = original_read_json
        campaign.load_and_validate_reviewer_provisional_review = original_shared_validation
    assert shared_validation_calls["count"] == 1
    serialized_recorded = json.dumps(recorded, sort_keys=True)
    assert recorded["state"] == "provisional_recorded"
    assert recorded["evaluator_material_exposed"] is False
    assert recorded["qualification_profile_exposed"] is False
    assert not any(value in serialized_recorded for value in campaign.MATERIALITY_OBLIGATIONS)
    assert "repository_class" not in serialized_recorded
    assert "logical_cycle" not in serialized_recorded
    assert not any("evaluator/descriptors" in path.as_posix() for path in read_paths)
    assert not any("evaluator/slot-mapping" in path.as_posix() for path in read_paths)
    assert not any("evaluator/qualification-profile" in path.as_posix() for path in read_paths)

    try:
        campaign.reveal_qualification_profile(root, candidate_head)
    except campaign.CampaignError as error:
        assert "all eight provisional reviews" in str(error)
    else:
        raise AssertionError("partial provisional completion revealed the private profile")
    try:
        campaign.seal_work(root, "volicord", 1, draft_path)
    except campaign.CampaignError as error:
        assert "all eight provisional reviews" in str(error)
    else:
        raise AssertionError("partial provisional completion revealed evaluator material")

    remaining_descriptors: dict[tuple[str, int], dict[str, object]] = {}
    remaining_paths: dict[tuple[str, int], Path] = {}
    cli_preparation: dict[str, object] | None = None
    cli_provisional: dict[str, object] | None = None
    for remaining_kind in campaign.CLASSES:
        for remaining_cycle in campaign.work_labels(remaining_kind):
            if (remaining_kind, remaining_cycle) == ("volicord", 1):
                continue
            remaining_descriptor, _work, _resume, _bundle = fixture_for(
                parent / "all-provisionals",
                remaining_kind,
                remaining_cycle,
                campaign_root=root,
            )
            for private_field in ("_evidence_directory", "_evidence_file_sha256", "evidence"):
                remaining_descriptor.pop(private_field, None)
            remaining_path = parent / f"{remaining_kind}-{remaining_cycle}-all-provisional.json"
            campaign.write_json(remaining_path, remaining_descriptor)
            remaining_preparation = campaign.prepare_review(
                root, remaining_kind, remaining_cycle, remaining_path
            )
            remaining_provisional = copy.deepcopy(
                remaining_descriptor["behavior_review"]["independent_review"]["provisional_review"]
            )
            if (remaining_kind, remaining_cycle) == ("volicord", 2):
                set_provisional_classification(
                    remaining_provisional, "research_or_no_question"
                )
                cli_preparation = remaining_preparation
                cli_provisional = remaining_provisional
            remaining_provisional["preparation_sha256"] = remaining_preparation[
                "preparation_sha256"
            ]
            remaining_provisional["review_slot_id"] = remaining_preparation[
                "review_slot_id"
            ]
            remaining_provisional_path = parent / (
                f"{remaining_preparation['review_slot_id']}-all-provisional.json"
            )
            campaign.write_json(remaining_provisional_path, remaining_provisional)
            campaign.record_provisional_review(
                root,
                candidate_head,
                remaining_preparation["review_slot_id"],
                remaining_provisional_path,
            )
            remaining_descriptors[(remaining_kind, remaining_cycle)] = remaining_descriptor
            remaining_paths[(remaining_kind, remaining_cycle)] = remaining_path
    assert cli_preparation is not None and cli_provisional is not None
    assert campaign.load_campaign(root, validate_private=False)["provisional_count"] == campaign.QUALIFICATION_WORK_COUNT
    fixed_provisional_snapshots = {
        path: (path.read_bytes(), harness.sha256(path))
        for path in sorted((root / "reviewer/provisional").glob("*.json"))
    }
    assert len(fixed_provisional_snapshots) == campaign.QUALIFICATION_WORK_COUNT
    private_profile_path = campaign.qualification_profile_path(root)
    original_profile_bytes = private_profile_path.read_bytes()
    original_campaign_bytes = campaign.campaign_file(root).read_bytes()
    original_inventory_bytes = campaign.inventory_path(root).read_bytes()
    malformed_profile = json.loads(original_profile_bytes)
    malformed_profile["behavior_histogram"] = {"explicit_user_owned_decision": 6}
    campaign.write_json(private_profile_path, malformed_profile)
    malformed_campaign = json.loads(original_campaign_bytes)
    malformed_campaign["qualification_profile_sha256"] = harness.sha256(
        private_profile_path
    )
    campaign.write_json(campaign.campaign_file(root), malformed_campaign)
    malformed_inventory = json.loads(original_inventory_bytes)
    malformed_inventory["artifacts"][campaign.relative(root, private_profile_path)] = {
        "bytes": private_profile_path.stat().st_size,
        "sha256": harness.sha256(private_profile_path),
    }
    campaign.write_json(campaign.inventory_path(root), malformed_inventory)
    try:
        campaign.reveal_qualification_profile(root, candidate_head)
    except campaign.CampaignError as error:
        assert "profile is malformed or incomplete" in str(error)
    else:
        raise AssertionError("malformed private profile passed post-provisional reveal")
    private_profile_path.write_bytes(original_profile_bytes)
    campaign.campaign_file(root).write_bytes(original_campaign_bytes)
    campaign.inventory_path(root).write_bytes(original_inventory_bytes)
    try:
        campaign.seal_work(root, "volicord", 1, draft_path)
    except campaign.CampaignError as error:
        assert "qualification-profile reveal" in str(error)
    else:
        raise AssertionError("all provisionals bypassed private profile validation")
    reveal = subprocess.run(
        [
            str(helper),
            "reveal-qualification-profile",
            "--campaign-root",
            str(root),
            "--candidate-head",
            candidate_head,
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    assert reveal.returncode == 0, reveal.stdout + reveal.stderr
    reveal_result = json.loads(reveal.stdout)
    assert reveal_result["provisional_count"] == campaign.QUALIFICATION_WORK_COUNT
    assert reveal_result["profile_validation"] == "passed"

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
        campaign.seal_work(root, "volicord", 1, bypassable_path)
    except campaign.CampaignError as error:
        assert "defensible no-question path" in str(error)
    else:
        raise AssertionError("bypassable material user-owned descriptor sealed")
    fixed_provisional = campaign.reviewer_provisional_path(root, "volicord", 1)
    assert fixed_provisional.read_bytes() == provisional_path.read_bytes()
    fixed_after_record = fixed_provisional.read_bytes()
    fixed_hash_after_record = harness.sha256(fixed_provisional)
    provisional["basis"] += " reviewer continues editing the old draft"
    campaign.write_json(provisional_path, provisional)
    campaign.verify_inventory(root)
    assert fixed_provisional.read_bytes() == fixed_after_record
    assert harness.sha256(fixed_provisional) == fixed_hash_after_record
    assert draft_inventory_name not in campaign.load_inventory(root)["artifacts"]
    assert campaign.work_state(root, "volicord", 1)["state"] == "provisional_recorded"
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
        campaign.seal_work(root, "volicord", 1, bypassable_path)
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
        campaign.seal_work(root, "volicord", 1, bypassable_path)
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
        campaign.seal_work(root, "volicord", 1, rewritten_path)
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
        campaign.seal_work(root, "volicord", 1, disagreement_path)
    except campaign.CampaignError as error:
        assert "disagreement blocks sealing" in str(error)
    else:
        raise AssertionError("unresolved evaluator/reviewer disagreement sealed")

    leaked = copy.deepcopy(descriptor)
    leaked["behavior_review"]["independent_review"]["basis"] = leaked["work_user_task"]
    leaked_path = parent / "leaked-evaluator-input.json"
    campaign.write_json(leaked_path, leaked)
    try:
        campaign.seal_work(root, "volicord", 1, leaked_path)
    except campaign.CampaignError as error:
        assert "evaluator-only material" in str(error)
    else:
        raise AssertionError("evaluator material entered the operator run sheet")

    accepted_path = parent / "unavoidable-user-owned-input.json"
    campaign.write_json(accepted_path, descriptor)
    sealed = campaign.seal_work(root, "volicord", 1, accepted_path)
    assert sealed["review_slot_id"] == review_slot_id
    sealed_run_sheet = run_sheet.read_text(encoding="utf-8")
    assert f"Slot `{review_slot_id}`" in sealed_run_sheet
    assert "cycle 1" not in sealed_run_sheet.casefold()
    assert "-cycle-" not in sealed_run_sheet
    assert descriptor["work_user_task"] not in sealed_run_sheet
    assert descriptor["fresh_resume_user_task"] not in sealed_run_sheet
    assert "Copy the exact UTF-8 bytes" in sealed_run_sheet
    task_artifacts = sealed["operator_task_artifacts"]
    assert set(task_artifacts) == {"work", "resume"}
    for role, field in (
        ("work", "work_user_task"),
        ("resume", "fresh_resume_user_task"),
    ):
        record = task_artifacts[role]
        task_path = root / record["path"]
        expected = descriptor[field].encode("utf-8")
        assert task_path.read_bytes() == expected
        assert record["bytes"] == len(expected)
        assert record["sha256"] == hashlib.sha256(expected).hexdigest()
        assert record["sealed_semantic_sha256"] == sealed["sealed_semantic_sha256"]
        assert str(task_path) in sealed_run_sheet
        assert record["sha256"] in sealed_run_sheet
        assert campaign.load_inventory(root)["artifacts"][record["path"]] == {
            "bytes": len(expected),
            "sha256": record["sha256"],
        }
    work_task_path = root / task_artifacts["work"]["path"]
    work_task_bytes = work_task_path.read_bytes()
    work_task_path.write_bytes(work_task_bytes + b" changed")
    try:
        campaign.load_sealed_descriptor(root, "volicord", 1)
    except campaign.CampaignError as error:
        assert "operator task artifact binding changed" in str(error)
    else:
        raise AssertionError("changed raw operator task artifact retained its sealed binding")
    work_task_path.write_bytes(work_task_bytes)
    campaign.load_sealed_descriptor(root, "volicord", 1)
    assert provisional["basis"] not in sealed_run_sheet
    assert provisional["materiality_conclusion"] not in sealed_run_sheet
    assert (
        descriptor["behavior_review"]["independent_review"]["counterfactual_review"][
            "specific_unresolved_outcome"
        ]
        not in sealed_run_sheet
    )

    cli_descriptor = remaining_descriptors[("volicord", 2)]

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
        campaign.seal_work(root, "volicord", 2, falsely_agreed_path)
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
        campaign.seal_work(root, "volicord", 2, unresolved_path)
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
    resolved_result = campaign.seal_work(root, "volicord", 2, resolved_path)
    assert resolved_result["review_slot_id"] == cli_preparation["review_slot_id"]
    assert fixed_cli_provisional.read_bytes() == fixed_cli_bytes
    assert harness.sha256(fixed_cli_provisional) == fixed_cli_sha256
    for path, (original_bytes, original_sha256) in fixed_provisional_snapshots.items():
        assert path.read_bytes() == original_bytes
        assert harness.sha256(path) == original_sha256

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
    assert not harness.work_descriptor_errors(
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
        errors = harness.work_descriptor_errors(
            value,
            candidate_revision=candidate_revision,
            target_repository=target,
            verify_provenance=True,
        )
        assert any(expected in error for error in errors), (expected, errors)


def assert_batch_failure_atomicity(parent: Path, binary: Path) -> None:
    def snapshot(root: Path) -> dict[str, bytes]:
        names = [*campaign.load_inventory(root)["artifacts"], "campaign.json", "evidence-inventory.json"]
        return {name: (root / name).read_bytes() for name in names}

    # Semantic/lifecycle uncertainty is preserved as raw evidence, not rejected.
    root, captures, bundles = prepared_batch(parent, "batch-incomplete", binary)
    work = next(path for path in captures if path.name == "volicord-1-work-events.jsonl")
    events = [json.loads(line) for line in work.read_text().splitlines()]
    terminal = max(n for n, event in enumerate(events)
        if event.get("payload", {}).get("type") in {"task_complete", "task_completed"})
    events.pop(terminal)
    work.write_text("".join(json.dumps(event) + "\n" for event in events))
    original_builder = harness.build_work_observation
    original_inspector = campaign.inspect_resume
    def forbidden(*args, **kwargs):
        raise AssertionError("collection invoked semantic evaluation")
    harness.build_work_observation = campaign.inspect_resume = forbidden
    try:
        summary = campaign.collect_batch(root, captures, exporter=batch_exporter(bundles), documenter=documenter)
    finally:
        harness.build_work_observation = original_builder
        campaign.inspect_resume = original_inspector
    assert summary["collection_state"] == "collected"
    assert summary["qualification_state"] == "not_run"
    assert campaign.load_campaign(root)["terminal_outcome"] is None
    manifest = campaign.load_evidence_set(root)
    assert len(manifest["raw_inputs"]) == 8
    assert summary["works"][0]["work"]["turn_lifecycle"]["state"] == "terminal_incomplete"
    original = snapshot(root)
    try:
        campaign.collect_batch(root, captures, exporter=batch_exporter(bundles))
    except campaign.CampaignError:
        pass
    else:
        raise AssertionError("immutable collection allowed replacement")
    assert snapshot(root) == original
    # A raw input changing after mapping remains a hard failure.
    root, captures, bundles = prepared_batch(parent, "batch-hash-race", binary)
    original_copy = campaign.copy_exact
    def changed_copy(source, destination):
        original_copy(source, destination)
        if destination.name == "work.rollout.jsonl":
            destination.write_bytes(destination.read_bytes() + b"\n")
    campaign.copy_exact = changed_copy
    try:
        campaign.collect_batch(root, captures, exporter=batch_exporter(bundles))
    except campaign.CampaignError:
        pass
    else:
        raise AssertionError("changed raw hash was published")
    finally:
        campaign.copy_exact = original_copy
    assert not (root / "evidence-set.json").exists()

    root, captures, bundles = prepared_batch(parent, "batch-publication-rollback", binary)
    original = snapshot(root)
    atomic_write = campaign.atomic_write_bytes
    writes = 0

    def failing_publication(path, content):
        nonlocal writes
        writes += 1
        assert (root / "batch-publication.json").exists()
        try:
            campaign.load_campaign(root)
        except campaign.CampaignError as error:
            assert error.diagnostic["outcome"] == "repair_required"
        else:
            raise AssertionError("in-flight publication was readable")
        if writes == 3:
            raise OSError("injected controlled-publication failure")
        atomic_write(path, content)

    campaign.atomic_write_bytes = failing_publication
    try:
        try:
            campaign.collect_batch(root, captures, exporter=batch_exporter(bundles), documenter=documenter)
        except OSError:
            pass
        else:
            raise AssertionError("publication fault was ignored")
    finally:
        campaign.atomic_write_bytes = atomic_write
    assert snapshot(root) == original
    assert not (root / "batch-publication.json").exists()
    assert not (root / "batch-intake-summary.json").exists()
    assert not list(root.glob(".batch-intake-*"))

    writes = 0

    def failed_rollback(path, content):
        nonlocal writes
        writes += 1
        if writes >= 3:
            raise OSError("injected publication and rollback failure")
        atomic_write(path, content)

    campaign.atomic_write_bytes = failed_rollback
    try:
        try:
            campaign.collect_batch(root, captures, exporter=batch_exporter(bundles), documenter=documenter)
        except campaign.CampaignError as error:
            assert error.diagnostic["outcome"] == "repair_required"
        else:
            raise AssertionError("failed rollback did not retain a publication barrier")
    finally:
        campaign.atomic_write_bytes = atomic_write
    journal = campaign.read_json(root / "batch-publication.json")
    assert (root / journal["stage_directory"] / "publication-backup").is_dir()
    try:
        campaign.load_campaign(root)
    except campaign.CampaignError as error:
        assert error.diagnostic["basis"] == "batch_publication_incomplete"
    else:
        raise AssertionError("failed rollback was readable as a campaign")


def assert_batch_workflow(parent: Path, binary: Path) -> None:
    root, captures, bundles = prepared_batch(parent, "batch-campaign", binary)
    run_sheet = (root / "operator/RUN-SHEET.md").read_text(encoding="utf-8")
    assert "collect-batch" in run_sheet
    assert "collect-work" not in run_sheet
    assert "collect-resume" not in run_sheet
    assert not any(value in run_sheet for value in campaign.MATERIALITY_OBLIGATIONS)
    assert not any(
        f"cycle {number}" in run_sheet.casefold()
        for number in range(1, max(campaign.CYCLE_COUNT_BY_REPOSITORY.values()) + 1)
    )
    assert "-cycle-" not in run_sheet
    for kind in campaign.CLASSES:
        for cycle in campaign.work_labels(kind):
            descriptor = campaign.read_json(
                campaign.evaluator_descriptor_path(root, kind, cycle)
            )
            state = campaign.work_state(root, kind, cycle)
            slot_heading = f"### Slot `{state['review_slot_id']}`"
            start = run_sheet.index(slot_heading)
            later_slots = [
                position
                for marker in ("\n### Slot `", "\n## Repository `")
                if (position := run_sheet.find(marker, start + len(slot_heading))) >= 0
            ]
            end = min(later_slots, default=len(run_sheet))
            slot_entry = run_sheet[start:end]
            assert descriptor["work_user_task"] not in slot_entry
            assert descriptor["fresh_resume_user_task"] not in slot_entry
            artifacts = state["operator_task_artifacts"]
            assert set(artifacts) == {"work", "resume"}
            for role, field in (
                ("work", "work_user_task"),
                ("resume", "fresh_resume_user_task"),
            ):
                artifact = artifacts[role]
                task_path = root / artifact["path"]
                expected = descriptor[field].encode("utf-8")
                assert task_path.read_bytes() == expected
                assert str(task_path) in slot_entry
                assert artifact["sha256"] == hashlib.sha256(expected).hexdigest()
                assert artifact["sha256"] in slot_entry
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
    assert len({item.capture.session_id for item in mapped.values()}) == campaign.BATCH_CAPTURE_COUNT
    compacted_mapped_capture = mapped[("volicord", 1, "work")].capture
    assert compacted_mapped_capture.fresh_user_thread is True
    assert len(compacted_mapped_capture.compacted_sequences) == 1
    mapped_state = campaign.work_state(root, "volicord", 1)
    raw_work_task = (
        root / mapped_state["operator_task_artifacts"]["work"]["path"]
    ).read_text(encoding="utf-8")
    assert harness.codex_user_turn_transport_identity_matches(
        compacted_mapped_capture.user_turns[0].text,
        raw_work_task,
    )

    directory = parent / "unordered-rollouts"
    directory.mkdir()
    for index, source in enumerate(reversed(captures)):
        shutil.copyfile(source, directory / f"capture-{index:02}.jsonl")
    directory_paths = campaign.batch_rollout_paths(None, directory)
    assert len(campaign.map_batch_rollouts(root, directory_paths)) == campaign.BATCH_CAPTURE_COUNT
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
        (campaign.work_root(root, kind, cycle) / "evidence/work.rollout.jsonl").exists()
        for kind in campaign.CLASSES
        for cycle in campaign.work_labels(kind)
    )

    work = compacted_work
    resume = next(path for path in captures if path.name == "volicord-1-resume-events.jsonl")
    current_user_work = current_user_message_capture(
        work,
        parent / "current-user-message-work.jsonl",
        inject_host_user_material=True,
    )
    current_work = current_mcp_tool_call_capture(
        current_user_work,
        parent / "current-user-message-mcp-work.jsonl",
    )
    current_inputs = [current_work if path == work else path for path in captures]
    current_mapping = campaign.map_batch_rollouts(root, current_inputs)
    current_mapped_capture = current_mapping[("volicord", 1, "work")].capture
    if (
        len(current_mapped_capture.user_turns) != 2
        or current_mapped_capture.user_turns[0].text != raw_work_task
        or "Host-injected instruction material"
        in [turn.text for turn in current_mapped_capture.user_turns]
        or campaign.observed_project_ids(current_mapped_capture) != ["01" * 16]
        or not current_mapped_capture.successful_calls("repository_analyze")
        or not current_mapped_capture.successful_calls("checkpoint_record")
    ):
        raise AssertionError(
            "current UserMessage/McpToolCall mapping lost task, Project, workflow, or host boundary"
        )
    current_descriptor_path, current_descriptor = campaign.load_sealed_descriptor(
        root, "volicord", 1
    )
    try:
        harness.build_work_observation(
            campaign.load_campaign(root)["candidate_head"],
            current_descriptor,
            harness.sha256(current_descriptor_path),
            current_mapped_capture,
            target_repository=Path(mapped_state["repository_path"]),
        )
    except harness.NoWorkObservation:
        pass
    else:
        raise AssertionError("valid current-format work mapping became a work blocker")

    duplicate_work = current_user_message_capture(
        work,
        parent / "duplicate-legacy-current-work.jsonl",
        keep_legacy=True,
    )
    duplicate_representation_inputs = [
        duplicate_work if path == work else path for path in captures
    ]
    duplicate_representation_mapping = campaign.map_batch_rollouts(
        root, duplicate_representation_inputs
    )
    if (
        len(
            duplicate_representation_mapping[("volicord", 1, "work")].capture.user_turns
        )
        != 2
    ):
        raise AssertionError("duplicate legacy/current transport evidence was double counted")

    conflicting_work = current_user_message_capture(
        work,
        parent / "conflicting-legacy-current-work.jsonl",
        keep_legacy=True,
        conflict_first_text=True,
    )
    conflicting_inputs = [conflicting_work if path == work else path for path in captures]
    try:
        campaign.map_batch_rollouts(root, conflicting_inputs)
    except campaign.CampaignError as error:
        assert error.diagnostic is not None
        assert error.diagnostic["mismatch_reasons"] == [
            "provenance_or_capture_format_mismatch"
        ]
    else:
        raise AssertionError("conflicting legacy/current transport evidence mapped")

    current_wrong_task = replaced_capture(
        current_work,
        parent / "current-wrong-task.jsonl",
        raw_work_task,
        raw_work_task + " changed",
    )
    try:
        campaign.map_batch_rollouts(
            root,
            [current_wrong_task if path == work else path for path in captures],
        )
    except campaign.CampaignError as error:
        assert error.diagnostic is not None
        assert "frozen_task_mismatch" in error.diagnostic["mismatch_reasons"]
    else:
        raise AssertionError("current UserMessage wrong task mapped")

    for label, old, new in (
        ("source", '"source":"vscode"', '"source":"exec"'),
        (
            "originator",
            '"originator":"codex_vscode"',
            '"originator":"codex_cli_rs"',
        ),
    ):
        invalid_current = replaced_capture(
            current_work,
            parent / f"current-wrong-{label}.jsonl",
            old,
            new,
        )
        try:
            campaign.map_batch_rollouts(
                root,
                [invalid_current if path == work else path for path in captures],
            )
        except campaign.CampaignError as error:
            assert error.diagnostic is not None
            assert any(
                reason in error.diagnostic["mismatch_reasons"]
                for reason in (
                    "provenance_mismatch",
                    "provenance_or_capture_format_mismatch",
                )
            )
        else:
            raise AssertionError(f"current UserMessage wrong {label} provenance mapped")

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

    state = campaign.load_campaign(root)["works"][campaign.work_key("volicord", 1)]
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
    mapped_rollout = multiple_newline_mapping[("volicord", 1, "work")]
    mapped_source = mapped_rollout.source
    mapped_capture = mapped_rollout.capture
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

    assert harness.compare_frozen_task_transport(
        "transport_marker", "transport\\_marker"
    ).equivalent
    escaped_task = replaced_capture(
        work,
        parent / "reversible-markdown-escape-work.jsonl",
        descriptor["work_user_task"],
        descriptor["work_user_task"] + " transport\\\\_marker",
    )
    escaped_resume = replaced_capture(
        resume,
        parent / "reversible-markdown-escape-resume.jsonl",
        descriptor["fresh_resume_user_task"],
        descriptor["fresh_resume_user_task"] + " transport\\\\_marker",
    )
    escaped_task_bytes = escaped_task.read_bytes()
    escaped_resume_bytes = escaped_resume.read_bytes()
    original_load_descriptor = campaign.load_sealed_descriptor

    def descriptor_with_literal_underscore(
        loaded_root: Path,
        kind: str,
        cycle: int,
        loaded_campaign=None,
    ):
        path, loaded = original_load_descriptor(
            loaded_root, kind, cycle, loaded_campaign
        )
        if (kind, cycle) == ("volicord", 1):
            loaded = copy.deepcopy(loaded)
            loaded["work_user_task"] += " transport_marker"
            loaded["fresh_resume_user_task"] += " transport_marker"
        return path, loaded

    campaign.load_sealed_descriptor = descriptor_with_literal_underscore
    try:
        escaped_inputs = [
            escaped_task
            if path == work
            else escaped_resume
            if path == resume
            else path
            for path in captures
        ]
        escaped_mapping = campaign.map_batch_rollouts(root, escaped_inputs)
        work_transport = escaped_mapping[("volicord", 1, "work")].task_transport
        resume_transport = escaped_mapping[("volicord", 1, "resume")].task_transport
        assert work_transport.equivalent and resume_transport.equivalent
        assert work_transport.transport_equivalence_used
        assert resume_transport.transport_equivalence_used
        assert len(work_transport.ignored_escape_normalized_raw_utf8_offsets) == 1
        assert len(resume_transport.ignored_escape_normalized_raw_utf8_offsets) == 1
        assert escaped_task.read_bytes() == escaped_task_bytes
        assert escaped_resume.read_bytes() == escaped_resume_bytes
        assert escaped_mapping[("volicord", 1, "work")].capture.source_sha256 == hashlib.sha256(
            escaped_task_bytes
        ).hexdigest()
        assert escaped_mapping[("volicord", 1, "resume")].capture.source_sha256 == hashlib.sha256(
            escaped_resume_bytes
        ).hexdigest()
    finally:
        campaign.load_sealed_descriptor = original_load_descriptor

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
        assert "materiality_obligations" not in json.dumps(error.diagnostic)
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
    assert activation["cycle_count"] == campaign.QUALIFICATION_WORK_COUNT
    assert activation["repository_and_hook_trust"] == "user_controlled_not_automated"
    assert all(
        item["result"]["static_verification"]["status"] == "passed"
        and item["result"]["static_verification"]["repository_and_hook_trust"]
        == "user_controlled_not_automated"
        and item["result"]["static_verification"]["runtime_session_start_execution"]
        == "not_proven_by_static_verification"
        for item in activation["works"]
    )

    first_state = campaign.work_state(root, "volicord", 1)
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
    campaign_state["works"][campaign.work_key("volicord", 1)]["codex_enabled"] = False
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
    assert campaign.work_state(root, "volicord", 1)["codex_enabled"] is False
    write_static_integration(first_repository, first_runtime, binary)

    for kind in campaign.CLASSES:
        for cycle in campaign.work_labels(kind):
            runtime = campaign.work_root(root, kind, cycle) / "runtime"
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
    assert summary["intake_state"] == "accepted", summary
    assert summary["qualification_state"] == "not_run", summary
    assert summary["outcome"] == "evidence_collected"
    assert summary["failed_checks"] == []
    assert summary["failure_attribution"] == []
    assert summary["session_distinctness"] == {
        "status": "passed",
        "expected_count": campaign.BATCH_CAPTURE_COUNT,
        "observed_count": campaign.BATCH_CAPTURE_COUNT,
    }
    assert len(summary["works"]) == campaign.QUALIFICATION_WORK_COUNT
    for item in summary["works"]:
        assert item["intake_state"] == "accepted"
        assert item["qualification_state"] == "not_run"
        assert item["failed_checks"] == []
        assert item["failure_attribution"] == []
        assert "status" not in item
        assert item["supported_evidence_complete"] is True
        cycle_path = campaign.work_root(root, item["repository_class"], item["cycle"])
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
        assert snapshot["schema_version"] == 3
        assert snapshot["navigation_responsiveness"]["duration_ms"] >= 0
        assert snapshot["navigation_responsiveness"]["request_completed"] is True
        assert "not browser" in snapshot["navigation_responsiveness"]["scope"]
        assert snapshot["navigation_responsiveness"]["measured"] == [
            "snapshot_export_request_completion", "snapshot_export_request_duration"]
        assert snapshot["navigation_responsiveness"]["unmeasured"] == [
            "browser_input_latency", "browser_paint_latency"]
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
    from review_operations_self_test import assert_review_workflow
    assert_review_workflow(root, parent)


def assert_successful_campaign(parent: Path, binary: Path) -> None:
    root = parent / "successful-campaign"
    prepare(root, parent / "successful-sources", binary)
    fixtures: dict[tuple[str, int], tuple[dict[str, object], Path, Path, Path]] = {}
    descriptor_paths: dict[tuple[str, int], Path] = {}
    for kind in campaign.CLASSES:
        for cycle in campaign.work_labels(kind):
            descriptor, work, resume, bundle = fixture_for(
                parent, kind, cycle, campaign_root=root
            )
            fixtures[(kind, cycle)] = (descriptor, work, resume, bundle)
            descriptor_paths[(kind, cycle)] = record_descriptor_review(
                root, kind, cycle, descriptor
            )
    reveal_and_seal_descriptors(root, descriptor_paths)
    for kind in campaign.CLASSES:
        for cycle in campaign.work_labels(kind):
            descriptor, work, resume, bundle = fixtures[(kind, cycle)]
            runtime = campaign.work_root(root, kind, cycle) / "runtime"
            (runtime / "canonical.sqlite3").write_bytes(b"PRIVATE-STORE-CONTENT")
            derived = runtime / "derived/analysis/project"
            derived.mkdir(parents=True)
            (derived / "private-analysis.json").write_bytes(b"PRIVATE-DERIVED-CONTENT")
            work_result = collect_work_fixture(root, kind, cycle, work)
            assert work_result["outcome"] == "resume_allowed"
            resume_result = collect_resume_fixture(
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
            assert harness.generated_document_summary_valid(
                document_evidence,
                root,
                project_id="01" * 16,
                candidate_revision=campaign.read_json(root / "campaign.json")[
                    "candidate_head"
                ],
                kind=kind,
                cycle=cycle,
            )
            assert set(document_evidence["documents"]) == set(campaign.DOCUMENT_KINDS)
            for document_kind in campaign.DOCUMENT_KINDS:
                formats = document_evidence["documents"][document_kind]["formats"]
                assert set(formats) == {name for name, _suffix in campaign.DOCUMENT_FORMATS}
                for evidence in formats.values():
                    path = root / evidence["relative_evidence_path"]
                    assert evidence["status"] == "passed"
                    assert evidence["bytes"] == path.stat().st_size
                    assert evidence["sha256"] == harness.sha256(path)
            summary_bytes = (campaign.work_root(root, kind, cycle) / "runtime-summary.json").read_bytes()
            assert b"canonical.sqlite3" in summary_bytes
            assert b"PRIVATE-STORE-CONTENT" not in summary_bytes
            assert b"PRIVATE-DERIVED-CONTENT" not in summary_bytes

    control = campaign.read_json(
        campaign.work_root(root, "volicord", 1) / "documents-summary.json"
    )
    candidate = campaign.read_json(root / "campaign.json")["candidate_head"]

    def accepted(value: dict[str, object]) -> bool:
        return harness.generated_document_summary_valid(
            value,
            root,
            project_id="01" * 16,
            candidate_revision=candidate,
            kind="volicord",
            cycle=1,
        )

    assert accepted(control)
    wrong_project = copy.deepcopy(control)
    wrong_project["project_id"] = "02" * 16
    assert not accepted(wrong_project)
    wrong_candidate = copy.deepcopy(control)
    wrong_candidate["candidate_head"] = "cd" * 20
    assert not accepted(wrong_candidate)
    wrong_cycle = copy.deepcopy(control)
    wrong_cycle["cycle"] = 2
    assert not accepted(wrong_cycle)
    wrong_format = copy.deepcopy(control)
    formats = wrong_format["documents"]["handoff-resume"]["formats"]
    formats["pdf"] = formats.pop("html")
    assert not accepted(wrong_format)
    missing = copy.deepcopy(control)
    missing_format = missing["documents"]["handoff-resume"]["formats"]["markdown"]
    missing_format["relative_evidence_path"] = "missing-document.md"
    assert not accepted(missing)
    corrupt = copy.deepcopy(control)
    corrupt["documents"]["handoff-resume"]["formats"]["markdown"]["sha256"] = (
        "00" * 32
    )
    assert not accepted(corrupt)

    manifest = campaign.finalize_manifest(root)
    first = manifest.read_bytes()
    campaign.finalize_manifest(root)
    assert manifest.read_bytes() == first


def assert_resume_baseline_identity_and_ordering(parent: Path) -> None:
    descriptor, _work, resume, _bundle = fixture_for(parent, "volicord", 1)
    revision = harness.git_head(campaign.ROOT)
    assert revision is not None
    state = {
        "repository_revision": revision,
        "repository_path": "/phase8/repository",
        "project_id": "01" * 16,
        "work_session_id": "different-work-session",
    }
    capture = harness.load_codex_capture(resume)
    assert len(capture.successful_calls("repository_analyze")) == 2
    assert campaign.inspect_resume(capture, descriptor, state) == "01" * 16

    # Exercise the campaign's shared canonical observer through resume intake.
    def noncanonical_arguments(arguments: dict[str, object]) -> None:
        arguments["components"] = ["worker", "parser"]
        assessments = arguments["coupled_artifact_review"]["assessments"]
        assessments.reverse()
        for assessment in assessments:
            disposition = assessment["disposition"]
            if disposition["state"] == "included":
                paths = disposition["repository_paths"]
                disposition["repository_paths"] = list(reversed(paths)) + paths[:1]

    canonical_events = [json.loads(line) for line in resume.read_text().splitlines()]
    for value in canonical_events:
        payload = value.get("payload", {})
        if "resume-materiality-scope-binding" not in str(payload.get("call_id", "")):
            continue
        if payload.get("type") == "custom_tool_call_output":
            result = json.loads(payload["output"][1]["text"])
            result["review_revision"] = 7
            result["executable_work_scope"]["components"] = ["parser", "worker"]
            payload["output"][1]["text"] = json.dumps(result)
        elif payload.get("type") == "mcp_tool_call_end":
            noncanonical_arguments(payload["invocation"]["arguments"])
            result = payload["result"]["Ok"]["structuredContent"]
            result["review_revision"] = 7
            result["executable_work_scope"]["components"] = ["parser", "worker"]
        elif payload.get("type") == "custom_tool_call":
            wrapper = harness.parse_mcp_wrapper(payload["input"])
            assert wrapper is not None
            arguments = copy.deepcopy(wrapper.arguments)
            noncanonical_arguments(arguments)
            payload["input"] = payload["input"].replace(
                json.dumps(wrapper.arguments, separators=(",", ":")),
                json.dumps(arguments, separators=(",", ":")),
            )
    canonical_resume = parent / "canonical-result-resume.jsonl"
    canonical_resume.write_text(
        "".join(json.dumps(value) + "\n" for value in canonical_events), encoding="utf-8"
    )
    assert campaign.inspect_resume(
        harness.load_codex_capture(canonical_resume), descriptor, state
    ) == "01" * 16

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

    def read_only_resume(
        name: str,
        *,
        completed: bool,
        include_verification: bool,
    ) -> Path:
        destination = parent / name
        events = [
            json.loads(line)
            for line in resume.read_text(encoding="utf-8").splitlines()
        ]
        excluded_operations = {
            "repository_analyze",
            "engineering_choice_discovery",
            "materiality_review",
            "checkpoint_record",
        }
        filtered = []
        for value in events:
            payload = value.get("payload", {})
            if payload.get("type") == "patch_apply_end":
                continue
            if (
                payload.get("type") == "mcp_tool_call_end"
                and payload.get("invocation", {}).get("tool")
                in excluded_operations
            ):
                continue
            if (
                not include_verification
                and "resume-verification-call" in str(payload.get("call_id", ""))
            ):
                continue
            if completed and payload.get("type") == "custom_tool_call_output" and (
                "recall-call" in str(payload.get("call_id", ""))
            ):
                output = payload.get("output")
                structured = json.loads(output[1]["text"])
                structured["checkpoint"]["work_state"] = "completed"
                output[1]["text"] = json.dumps(structured, separators=(",", ":"))
            if completed and payload.get("type") == "mcp_tool_call_end" and (
                payload.get("invocation", {}).get("tool") == "recall"
            ):
                payload["result"]["Ok"]["structuredContent"]["checkpoint"][
                    "work_state"
                ] = "completed"
            filtered.append(value)
        destination.write_text(
            "".join(
                json.dumps(value, separators=(",", ":")) + "\n"
                for value in filtered
            ),
            encoding="utf-8",
        )
        return destination

    completed_read_only = read_only_resume(
        "completed-read-only-resume.jsonl",
        completed=True,
        include_verification=True,
    )
    completed_capture = harness.load_codex_capture(completed_read_only)
    assert not completed_capture.successful_calls("checkpoint_record")
    assert campaign.inspect_resume(completed_capture, descriptor, state) == "01" * 16

    for name, completed, include_verification in (
        ("completed-read-only-without-verification.jsonl", True, False),
        ("unfinished-read-only-resume.jsonl", False, True),
    ):
        invalid = harness.load_codex_capture(
            read_only_resume(
                name,
                completed=completed,
                include_verification=include_verification,
            )
        )
        try:
            campaign.inspect_resume(invalid, descriptor, state)
        except campaign.CampaignError:
            pass
        else:
            raise AssertionError(f"invalid read-only resume qualified: {name}")


def assert_failed_document_kind_is_machine_failure(parent: Path, binary: Path) -> None:
    root, captures, bundles = prepared_batch(
        parent, "failed-document-campaign", binary
    )
    work = next(path for path in captures if path.name == "volicord-1-work-events.jsonl")
    resume = next(path for path in captures if path.name == "volicord-1-resume-events.jsonl")
    slot = campaign.work_state(root, "volicord", 1)["review_slot_id"]
    bundle = bundles[slot]
    assert collect_work_fixture(root, "volicord", 1, work)["outcome"] == "resume_allowed"
    result = collect_resume_fixture(
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


def assert_superseded_candidate_mutation_guard(parent: Path, binary: Path) -> None:
    root = parent / "superseded-campaign"
    prepare(root, parent / "superseded-sources", binary)
    campaign_state = campaign.load_campaign(root)
    current_candidate = campaign_state["candidate_head"]
    state = campaign_state["works"]["volicord-cycle-1"]
    review_slot_id = state["review_slot_id"]
    missing = parent / "guard-input-does-not-exist.json"
    archive = parent / "guard-review.tar.gz"
    qualified = root / "guard-qualified.json"

    def snapshot() -> dict[str, str]:
        return {
            path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(root.rglob("*"))
            if path.is_file()
        }

    operations = {
        "prepare-document-realizations": lambda: campaign.document_realization.prepare(root, []),
        "record-document-realization": lambda: campaign.document_realization.record(root, "ab" * 16, missing),
        "prepare-review": lambda: campaign.prepare_review(
            root, "volicord", 1, missing
        ),
        "record-provisional-review": lambda: campaign.record_provisional_review(
            root, current_candidate, review_slot_id, missing
        ),
        "reveal-qualification-profile": lambda: campaign.reveal_qualification_profile(
            root, current_candidate
        ),
        "seal-cycle": lambda: campaign.seal_work(root, "volicord", 1, missing),
        "activate-cycle": lambda: campaign.activate_cycle(root, "volicord", 1),
        "activate-all": lambda: campaign.activate_all(root),
        "collect-work": lambda: collect_work_fixture(root, "volicord", 1, missing),
        "collect-resume": lambda: collect_resume_fixture(root, "volicord", 1, missing),
        "collect-batch": lambda: campaign.collect_batch(root, []),
        "finalize-manifest": lambda: campaign.finalize_manifest(root),

    }
    original_head = harness.git_head
    original_clean = harness.git_clean
    before = snapshot()
    harness.git_head = lambda _path: "cd" * 20
    harness.git_clean = lambda _path: True
    try:
        if campaign.load_campaign(root)["candidate_head"] != current_candidate:
            raise AssertionError("read-only superseded campaign inspection changed identity")
        for operation, invoke in operations.items():
            try:
                invoke()
            except campaign.CampaignError as error:
                if "current clean qualifying HEAD" not in str(error):
                    raise AssertionError(
                        f"{operation} failed after rather than at the shared candidate guard"
                    ) from error
            else:
                raise AssertionError(f"{operation} mutated a superseded campaign")
            if snapshot() != before or archive.exists() or qualified.exists():
                raise AssertionError(
                    f"{operation} changed superseded campaign or output state"
                )
    finally:
        harness.git_head = original_head
        harness.git_clean = original_clean

    harness.git_clean = lambda _path: False
    try:
        try:
            campaign.collect_batch(root, [])
        except campaign.CampaignError as error:
            if "current clean qualifying HEAD" not in str(error):
                raise
        else:
            raise AssertionError("collect-batch accepted a dirty qualifying worktree")
        if snapshot() != before:
            raise AssertionError("dirty-worktree rejection changed campaign state")
    finally:
        harness.git_clean = original_clean


def assert_journey_campaign_contract(parent: Path, binary: Path) -> None:
    unsealed = parent / "activation-before-freeze"
    prepare(unsealed, parent / "activation-before-freeze-sources", binary)
    before = campaign.campaign_file(unsealed).read_bytes()
    try:
        campaign.activate_all(unsealed)
    except campaign.CampaignError as error:
        assert "all five Work descriptors" in str(error)
    else:
        raise AssertionError("activation accepted an incompletely frozen sequence")
    assert campaign.campaign_file(unsealed).read_bytes() == before

    dependent, _start, _resume, _bundle = fixture_for(
        parent / "dependent-later-work-fixture",
        "volicord",
        "B",
        campaign_root=unsealed,
    )
    dependent["work_user_task"] = (
        "Continue the previous Work A implementation and preserve the exact approach chosen earlier."
    )
    dependent_path = parent / "dependent-later-work.json"
    campaign.write_json(dependent_path, dependent)
    try:
        campaign.prepare_review(unsealed, "volicord", "B", dependent_path)
    except campaign.CampaignError as error:
        assert "specific earlier Work outcome" in str(error)
    else:
        raise AssertionError("later Volicord Work accepted an exact prior-outcome dependency")

    root, captures, bundles = prepared_batch(parent, "journey-contract", binary)
    state = campaign.load_campaign(root)
    assert len(state["journeys"]) == 3
    assert len(state["works"]) == 5
    assert sum(len(item["session_slot_ids"]) for item in state["works"].values()) == 8
    assert sum(
        "resume" in campaign.session_roles(kind, label)
        for kind in campaign.CLASSES
        for label in campaign.work_labels(kind)
    ) == 3
    volicord = [
        state["works"][campaign.work_key("volicord", label)]
        for label in campaign.work_labels("volicord")
    ]
    assert len({item["repository_path"] for item in volicord}) == 1
    assert len({item["runtime_home"] for item in volicord}) == 1
    assert len({item["review_slot_id"] for item in state["works"].values()}) == 5
    profile = campaign.read_json(campaign.qualification_profile_path(root))
    assert all(
        profile["obligation_coverage"][obligation] >= minimum
        for obligation, minimum in campaign.PRIVATE_OBLIGATION_MINIMUMS.items()
    )
    mapping = campaign.read_json(campaign.slot_mapping_path(root))
    assert any(
        {"learning_deliberation", "learning_routine_control"}
        <= set(entry["materiality_obligations"])
        for entry in mapping["entries"]
    )
    assert len({
        entry["repository_class"] for entry in mapping["entries"]
        if "hidden_user_owned_decision" in entry["materiality_obligations"]
    }) >= 2
    run_sheet = (root / "operator/RUN-SHEET.md").read_text(encoding="utf-8")
    assert run_sheet.count("### Session `") == 8
    assert not any(item in run_sheet for item in campaign.MATERIALITY_OBLIGATIONS)

    rollback_root, rollback_captures, _rollback_bundles = prepared_batch(
        parent, "journey-collection-rollback", binary
    )
    rollback_before = {
        path.relative_to(rollback_root).as_posix(): path.read_bytes()
        for path in rollback_root.rglob("*") if path.is_file()
    }
    def failing_exporter(_binary, _runtime, _repository, _destination):
        raise campaign.CampaignError("injected journey-final export failure")
    try:
        campaign.collect_batch(
            rollback_root,
            rollback_captures,
            exporter=failing_exporter,
            documenter=documenter,
            snapshotter=snapshotter,
        )
    except campaign.CampaignError as error:
        assert "injected journey-final export failure" in str(error)
    else:
        raise AssertionError("failed journey-final extraction was published")
    assert rollback_before == {
        path.relative_to(rollback_root).as_posix(): path.read_bytes()
        for path in rollback_root.rglob("*") if path.is_file()
    }
    assert not (rollback_root / "batch-publication.json").exists()

    mapped = campaign.map_batch_rollouts(root, list(reversed(captures)))
    assert set(mapped) == set(harness.current_session_slots())
    assert len({item.capture.session_id for item in mapped.values()}) == 8
    wrong_revision = replaced_capture(
        captures[0],
        parent / "wrong-journey-revision.jsonl",
        harness.git_head(campaign.ROOT),
        "0" * 40,
    )
    invalid_revision_batch = [wrong_revision, *captures[1:]]
    try:
        campaign.map_batch_rollouts(root, invalid_revision_batch)
    except campaign.CampaignError as error:
        assert "zero sealed work roles" in str(error)
        assert "repository_revision_mismatch" in error.diagnostic["mismatch_reasons"]
    else:
        raise AssertionError("batch mapping accepted an unrelated repository revision")
    try:
        campaign.map_batch_rollouts(root, captures[:-1])
    except campaign.CampaignError as error:
        assert "missing" in str(error)
    else:
        raise AssertionError("batch mapping accepted a missing session slot")

    original_run_checked = campaign.run_checked
    campaign.run_checked = fake_enable_command
    try:
        activation = campaign.activate_all(root)
    finally:
        campaign.run_checked = original_run_checked
    assert activation["journey_count"] == 3

    summary = campaign.collect_batch(
        root,
        captures,
        exporter=batch_exporter(bundles),
        documenter=documenter,
        snapshotter=snapshotter,
    )
    assert summary["session_distinctness"] == {
        "status": "passed", "expected_count": 8, "observed_count": 8
    }
    assert len(summary["works"]) == 5
    assert len(summary["journeys"]) == 3
    assert len(summary["journey_final_evidence"]) == 3
    by_journey: dict[str, list[dict[str, object]]] = {}
    for work in summary["works"]:
        by_journey.setdefault(str(work["journey_id"]), []).append(work)
    assert len({item["project_id"] for item in summary["journeys"].values()}) == 3
    assert len({item["project_id"] for item in by_journey["journey-volicord"]}) == 1
    assert len({item["work_item_id"] for item in by_journey["journey-volicord"]}) == 3
    assert all(item["canonical_evidence"]["checkpoint_ids"] for item in summary["works"])
    volicord_final = next(
        item for item in summary["journey_final_evidence"]
        if item["journey_id"] == "journey-volicord"
    )
    assert len(volicord_final["represented_work_slot_ids"]) == 3
    assert len(volicord_final["ordered_work_item_ids"]) == 3
    assert volicord_final["repository_revision_lineage"]["workspace_clean"] is True
    assert len(volicord_final["repository_revision_lineage"]["ordered_session_revisions"]) == 4
    manifest = campaign.load_evidence_set(root)
    assert manifest["schema_version"] == 4
    assert len(manifest["raw_inputs"]) == 8
    assert len(manifest["work_evidence"]) == 5
    assert len(manifest["journey_final_evidence"]) == 3
    assert {item["session_slot_id"] for item in manifest["raw_inputs"]} == {
        campaign.session_slot_id(*slot) for slot in harness.current_session_slots()
    }
    first_raw = root / manifest["work_evidence"][0]["sessions"]["start"]["relative_evidence_path"]
    original_raw = first_raw.read_bytes()
    first_raw.write_bytes(original_raw + b"\n")
    try:
        campaign.load_evidence_set(root)
    except campaign.CampaignError as error:
        assert any(
            phrase in str(error)
            for phrase in ("artifact changed", "hash binding", "hash mismatch")
        )
    else:
        raise AssertionError("immutable raw evidence accepted post-publication mutation")
    finally:
        first_raw.write_bytes(original_raw)


def assert_historical_campaign_is_inspection_only(parent: Path, binary: Path) -> None:
    root = parent / "historical-inspection"
    prepare(root, parent / "historical-inspection-sources", binary)
    value = campaign.read_json(campaign.campaign_file(root))
    value["schema_version"] = 3
    campaign.write_json(campaign.campaign_file(root), value)
    before = {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in root.rglob("*") if path.is_file()
    }
    output = parent / "historical-inspection-result"
    result = campaign.diagnose_campaign(root, output)
    assert result["inspection_state"] == "identity_and_inventory_only"
    diagnostic = campaign.read_json(output / "diagnostic.json")
    assert diagnostic["historical_campaign_schema_version"] == 3
    assert diagnostic["replacement_pass_candidate"] is False
    assert before == {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in root.rglob("*") if path.is_file()
    }


def main() -> int:
    from resume_self_test import check_resume_regressions
    from document_realization_self_test import check_document_realization_regressions
    from long_lived_project_self_test import check_long_lived_project_regressions
    check_resume_regressions()
    check_long_lived_project_regressions()
    check_document_realization_regressions()
    original_clean = harness.git_clean
    harness.git_clean = lambda _path: True
    try:
        with tempfile.TemporaryDirectory(prefix="volicord-dogfood-campaign-") as temporary:
            parent = Path(temporary)
            binary = parent / "candidate/bin/volicord"
            write_fake_binary(binary)
            assert_session_start_ordering(parent)
            assert_strict_cli_contract(parent, binary)
            assert_default_document_process_evidence(parent, binary)
            assert_opaque_slot_preparation(parent, binary)
            assert_journey_campaign_contract(parent, binary)
            assert_historical_campaign_is_inspection_only(parent, binary)
    finally:
        harness.git_clean = original_clean
    print(json.dumps({
        "status": "passed",
        "checks": [
            "three_journeys_five_works_three_resume_pairs_eight_sessions",
            "independent_overlapping_materiality_obligation_coverage",
            "five_opaque_reviews_and_profile_blindness",
            "complete_task_freeze_before_journey_activation",
            "later_volicord_work_rejects_exact_prior_outcome_dependency",
            "shared_volicord_workspace_runtime_and_isolated_other_journeys",
            "eight_entry_leak_free_operator_run_sheet",
            "unordered_eight_capture_mapping_and_global_session_distinctness",
            "same_work_resume_project_and_work_identity",
            "same_project_distinct_volicord_work_identities",
            "cross_journey_project_isolation",
            "immutable_session_slot_manifest_and_raw_hash_binding",
            "collection_failure_rollback_and_post_publication_raw_immutability",
            "journey_final_document_realization_inventory",
            "historical_cycle_schema_identity_inventory_inspection_only",
            "resume_frontier_and_long_lived_project_regressions",
        ],
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
