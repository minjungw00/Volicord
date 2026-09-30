#!/usr/bin/env python3
"""Disposable end-to-end checks for the private dogfood campaign helper."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import shutil
import tempfile

import campaign
import harness
import repository_state


REVISION = "ab" * 20
STRICT_FAKE = campaign.ROOT / "rebuild/validation/shared/strict_fake_volicord.py"
# Synthetic transport fixtures exercise existing technical branches; campaign preparation
# never assigns these cases to ordinary Works.
FIXTURE_BEHAVIOR_CASES = [
    ("volicord", "A", ("learning_deliberation",)),
    ("volicord", "B", ("hidden_user_owned_decision",)),
    ("volicord", "C", ("delegated_implementation_choice",)),
    ("small-python", "A", ("hidden_user_owned_decision", "exploratory_uncertainty")),
    ("polyglot-medium", "A", ("research_or_no_question", "repository_or_environment_fact")),
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


# Explicit test double for the pre-existing lightweight (non-Git) cloner.
# Real Git attestation is exercised separately by repository_state_self_test.
FIXTURE_REPOSITORY_REVISIONS: dict[Path, str] = {}
FIXTURE_COMMIT_LINES: dict[Path, tuple[str, ...]] = {}
REAL_OBSERVE = repository_state.observe
REAL_REVISION_IS_BOUND = campaign.revision_is_bound
REAL_COMMITTED_WORK_PATHS = campaign.committed_delta_paths
REAL_COMMIT_HISTORY = repository_state.commit_history


def fixture_revision_is_bound(repository, baseline, observed):
    line = FIXTURE_COMMIT_LINES.get(repository.resolve())
    if line is None or (repository / ".git").exists():
        return REAL_REVISION_IS_BOUND(repository, baseline, observed)
    if observed is None:
        return True
    return baseline in line and observed in line and line.index(baseline) <= line.index(observed)


campaign.revision_is_bound = fixture_revision_is_bound


def fixture_committed_delta_paths(repository, base, boundary):
    if repository.resolve() not in FIXTURE_COMMIT_LINES or (repository / ".git").exists():
        return REAL_COMMITTED_WORK_PATHS(repository, base, boundary)
    # Explicit counterpart to the non-Git cloner's synthetic revision line.
    # Coverage/rejection semantics are tested against real Git by RepositoryStateTests.
    return ["src/existing.rs", "tests/existing.rs", "src/resume.rs",
        "backend/src/existing.rs", "frontend/src/existing.ts"] if base != boundary else []


campaign.committed_delta_paths = fixture_committed_delta_paths


def fixture_commit_history(repository, base, end):
    line = FIXTURE_COMMIT_LINES.get(repository.resolve())
    if line is None or (repository / ".git").exists():
        return REAL_COMMIT_HISTORY(repository, base, end)
    if base is None or end is None:
        return {"state": "not_computable", "base": base, "end": end, "commits": []}
    commits = [{"revision": revision, "parents": [line[index - 1]],
                "paths": fixture_committed_delta_paths(repository, line[index - 1], revision)}
               for index, revision in enumerate(line)
               if line.index(base) < index <= line.index(end)]
    return {"state": "computed", "base": base, "end": end, "commits": commits}


repository_state.commit_history = fixture_commit_history



def append_boundary_check(path, revision):
    """Synthetic structured transport, inside the terminal task lifecycle."""
    events = [json.loads(line) for line in path.read_text().splitlines()]
    terminal = events.pop()
    assert terminal["payload"]["type"] == "task_complete"
    turn = terminal["payload"]["turn_id"]
    call_id = f"boundary-{path.stem}"
    arguments = {"cmd": campaign.GIT_STATE_CHECK,
        "workdir": events[0]["payload"]["cwd"]}
    metadata = {"turn_id": turn}
    # Model the actual operator follow-up, including commit housekeeping after
    # the Product checkpoint/verification and before terminal status.
    maintenance_id = f"commit-{path.stem}"
    maintenance = {"cmd": "git add -- src/existing.rs && git commit -m 'fixture Work'",
        "workdir": arguments["workdir"]}
    events.extend([
        {"type": "response_item", "payload": {"type": "custom_tool_call",
            "call_id": maintenance_id, "name": "exec", "status": "completed",
            "input": "const commit=await tools.exec_command(" + json.dumps(maintenance)
                + "); text(JSON.stringify(commit));",
            "internal_chat_message_metadata_passthrough": metadata}},
        {"type": "response_item", "payload": {"type": "custom_tool_call_output",
            "call_id": maintenance_id, "output": [
                {"type": "input_text", "text": "Script completed\nWall time 0.1 seconds\nOutput:\n"},
                {"type": "input_text", "text": json.dumps({"output": "fixture commit\n", "exit_code": 0})}],
            "internal_chat_message_metadata_passthrough": metadata}}])
    events.extend([
        {"type": "response_item", "payload": {"type": "custom_tool_call",
            "call_id": call_id, "name": "exec", "status": "completed",
            "input": "const boundary=await tools.exec_command(" + json.dumps(arguments)
                + "); text(JSON.stringify(boundary));",
            "internal_chat_message_metadata_passthrough": metadata}},
        {"type": "response_item", "payload": {"type": "custom_tool_call_output",
            "call_id": call_id, "output": [
                {"type": "input_text", "text": "Script completed\nWall time 0.1 seconds\nOutput:\n"},
                {"type": "input_text", "text": json.dumps({"output": revision + "\n", "exit_code": 0})}],
            "internal_chat_message_metadata_passthrough": metadata}},
        terminal])
    path.write_text("\n".join(json.dumps(event, separators=(",", ":")) for event in events) + "\n")


def observe_fixture_repository(repository):
    revision = FIXTURE_REPOSITORY_REVISIONS.get(repository.resolve())
    if revision is None or (repository / ".git").exists():
        return REAL_OBSERVE(repository)
    patches = {"staged": b"", "unstaged": b""}
    state = {"kind": "dogfood_journey_repository_state", "schema_version": 1,
             "final_head": FIXTURE_COMMIT_LINES[repository.resolve()][-1],
             "status": [], "index": [], "tracked": [], "untracked": [],
             "diffs": {key: {"bytes": 0, "sha256": repository_state.digest(data)} for key, data in patches.items()},
             "workspace_clean": True,
             "boundary": "HEAD_index_tracked_and_nonignored_untracked; ignored_content_excluded"}
    state["fingerprint"] = repository_state.digest(b"dogfood-journey-repository-state\0" + repository_state.encoded(state))
    return state, patches




repository_state.observe = observe_fixture_repository


def prepare(
    root: Path,
    source_root: Path,
    binary: Path,
) -> None:
    source = repository_input(root.parent / f"{root.name}-repositories.json", source_root)
    original_clean = harness.git_clean
    original_specs = harness.load_repository_specs
    harness.git_clean = lambda _path: True
    harness.load_repository_specs = lambda _path, _head, _definition: (
        campaign.repository_spec_map(json.loads(source.read_text(encoding="utf-8"))),
        fake_identities(),
    )
    task_files = {}
    task_directory = root.parent / f"{root.name}-tasks"
    for kind in campaign.CLASSES:
        for label in campaign.work_labels(kind):
            obligation = next(value[2][0] for value in FIXTURE_BEHAVIOR_CASES
                if value[:2] == (kind, label))
            revision = harness.git_head(campaign.ROOT) if kind == "volicord" else REVISION
            fixture_directory = root.parent / f"{root.name}-task-fixtures" / f"{kind}-{label}"
            fixture_directory.mkdir(parents=True, exist_ok=True)
            fixture = harness.real_session_fixture(kind, label, revision,
                fixture_directory, materiality_obligations=obligation)
            statement = ("I want to learn through one meaningful agent-owned technical fork before implementation."
                if (kind, label) == ("volicord", "A") else None)
            entry = {"workload_intent": campaign.workload_intents.WORKLOAD_INTENTS[campaign.work_key(kind, label)],
                "learning_collaboration_statement": statement}
            for role in campaign.session_roles(kind, label):
                field = "work_user_task" if role == "start" else "fresh_resume_user_task"
                path = task_directory / f"{kind}-{label}-{role}.txt"
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(fixture[field].encode("utf-8"))
                entry[role] = str(path)
            task_files[campaign.work_key(kind, label)] = entry
    task_manifest = root.parent / f"{root.name}-tasks.json"
    task_manifest.write_text(json.dumps({"tasks": task_files}), encoding="utf-8")
    try:
        def fake_clone(_source: Path, destination: Path, _revision: str) -> None:
            destination.mkdir(parents=True)
            FIXTURE_REPOSITORY_REVISIONS[destination.resolve()] = _revision
            FIXTURE_COMMIT_LINES[destination.resolve()] = tuple([
                _revision,
                *(hashlib.sha256(f"{_revision}:{i}".encode()).hexdigest()[:40]
                  for i in range(1, 4 if destination.parent.name == "journey-volicord" else 2)),
            ])

        campaign.prepare_campaign(
            root,
            root.name,
            harness.git_head(campaign.ROOT),
            source,
            task_manifest,
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
        Path(campaign.work_state(campaign_root, kind, cycle)["repository_path"])
        if campaign_root is not None
        else None
    )
    obligations = next(value[2] for value in FIXTURE_BEHAVIOR_CASES
        if value[:2] == (kind, cycle))
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
    if "resume" not in campaign.session_roles(kind, cycle):
        descriptor["fresh_resume_user_task"] = None
    work = fixture_root / f"{kind}-{cycle}-work-events.jsonl"
    resume = fixture_root / f"{kind}-{cycle}-resume-events.jsonl"
    if campaign_root is not None and kind == "volicord" and cycle in {"B", "C"}:
        line = FIXTURE_COMMIT_LINES[repository.resolve()]
        descendant = line[1 if cycle == "B" else 2]
        text = work.read_text(encoding="utf-8")
        work.write_text(text.replace(f'"commit_hash":"{revision}"',
            f'"commit_hash":"{descendant}"'), encoding="utf-8")
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
        if campaign_root is not None:
            line = FIXTURE_COMMIT_LINES[repository.resolve()]
            append_boundary_check(path, line[{"A": 1, "B": 2, "C": 3}[cycle]])
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
        for cycle in campaign.work_labels(kind):
            _descriptor, work, resume, bundle = fixture_for(
                parent / f"{name}-fixtures", kind, cycle, campaign_root=root,
            )
            captures.append(work)
            if "resume" in campaign.session_roles(kind, cycle):
                captures.append(resume)
            state = campaign.work_state(root, kind, cycle)
            bundles[state["work_slot_id"]] = bundle
    original_run_checked = campaign.run_checked
    campaign.run_checked = fake_enable_command
    try:
        campaign.activate_all(root)
    finally:
        campaign.run_checked = original_run_checked
    return root, captures, bundles




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








































def assert_inventory_diagnostic(parent: Path, binary: Path) -> None:
    root = parent / "inventory-diagnostic"
    prepare(root, parent / "diagnostic-sources", binary)
    for version in (7, 3):
        value = campaign.read_json(campaign.campaign_file(root))
        value["schema_version"] = version
        campaign.write_json(campaign.campaign_file(root), value)
        before = {path.relative_to(root).as_posix(): path.read_bytes()
            for path in root.rglob("*") if path.is_file()}
        result = campaign.diagnose_campaign(root, parent / f"diagnostic-{version}")
        assert result["inspection_state"] == "identity_and_inventory_only"
        assert result["qualification_state"] == "not_run"
        assert result["phase_9_ready"] is False
        assert campaign.read_json(Path(result["diagnostic"]))[
            "historical_campaign_schema_version"] == version
        assert before == {path.relative_to(root).as_posix(): path.read_bytes()
            for path in root.rglob("*") if path.is_file()}


def assert_current_campaign_contract(parent: Path, binary: Path) -> None:
    # These ordinary, outcome-open examples prove five intents fit eight frozen turns.
    examples = campaign.read_json(Path(__file__).with_name("fixtures") / "workload-tasks.json")
    manifest = copy.deepcopy(examples)
    for slot, entry in manifest["tasks"].items():
        for role in ("start", "resume"):
            if role in entry:
                path = parent / f"ordinary-{slot}-{role}.txt"
                path.write_text(entry[role], encoding="utf-8")
                entry[role] = str(path)
    path = parent / "ordinary-workloads.json"
    campaign.write_json(path, manifest)
    tasks, metadata = campaign.frozen_tasks(path)
    assert len(tasks) == 8
    assert {slot: entry["workload_intent"] for slot, entry in metadata.items()} == campaign.workload_intents.WORKLOAD_INTENTS
    root, captures, bundles = prepared_batch(parent, "current-campaign", binary)
    state = campaign.load_campaign(root)
    assert {slot: work["workload_intent"] for slot, work in state["works"].items()} == campaign.workload_intents.WORKLOAD_INTENTS
    assert len(state["journeys"]) == 3 and len(state["works"]) == 5
    assert state["schema_version"] == 8
    assert all("review_slot_id" not in work and "provisional_review" not in work
        and "sealed_semantic_sha256" not in work for work in state["works"].values())
    assert all(work["work_slot_id"] == key for key, work in state["works"].items())
    assert sum(len(work["operator_task_artifacts"]) for work in state["works"].values()) == 8
    assert all(work["state"] == "frozen" for work in state["works"].values())
    assert (root / "operator/RUN-SHEET.md").read_text().count("### Session `") == 8
    run_sheet = root / "operator/RUN-SHEET.md"
    guidance = run_sheet.read_text()
    assert "repository-owned Git policy" in guidance
    assert "dirty carryover across distinct Works" in guidance
    assert "harness does not infer a commit obligation" in guidance
    assert campaign.GIT_STATE_CHECK in guidance
    assert "Dogfood requires no Work commit" in guidance
    original_run_sheet = run_sheet.read_bytes()
    run_sheet.write_bytes(original_run_sheet + b"\nEVALUATOR_ONLY\n")
    try:
        try:
            campaign.assert_operator_artifacts_do_not_leak(root)
        except campaign.CampaignError:
            pass
        else:
            raise AssertionError("operator projection accepted private reviewer material")
    finally:
        run_sheet.write_bytes(original_run_sheet)
    volicord = [state["works"][campaign.work_key("volicord", label)] for label in ("A", "B", "C")]
    assert len({work["repository_path"] for work in volicord}) == 1
    assert len({work["runtime_home"] for work in volicord}) == 1
    assert len({work["repository_path"] for work in state["works"].values()}) == 3
    assert len({work["runtime_home"] for work in state["works"].values()}) == 3

    task_manifest = campaign.read_json(parent / "current-campaign-tasks.json")
    incomplete = copy.deepcopy(task_manifest)
    incomplete["tasks"].pop(campaign.work_key("volicord", "C"))
    missing = parent / "missing-work-tasks.json"
    campaign.write_json(missing, incomplete)
    try:
        campaign.frozen_tasks(missing)
    except campaign.CampaignError:
        pass
    else:
        raise AssertionError("missing frozen Work was accepted")
    incomplete = copy.deepcopy(task_manifest)
    incomplete["tasks"][campaign.work_key("volicord", "A")].pop("resume")
    campaign.write_json(missing, incomplete)
    try:
        campaign.frozen_tasks(missing)
    except campaign.CampaignError:
        pass
    else:
        raise AssertionError("missing resume task was accepted")
    for field, invalid in (("workload_intent", "routine_bounded"),
                           ("workload_intent", None),
                           ("learning_collaboration_statement", None),
                           ("learning_collaboration_statement", "not in the user task")):
        incomplete = copy.deepcopy(task_manifest)
        incomplete["tasks"][campaign.work_key("volicord", "A")][field] = invalid
        campaign.write_json(missing, incomplete)
        try:
            campaign.frozen_tasks(missing)
        except campaign.CampaignError:
            pass
        else:
            raise AssertionError("missing intent or explicit learning request was accepted")
    try:
        campaign.require_current_candidate("0" * 40)
    except campaign.CampaignError:
        pass
    else:
        raise AssertionError("wrong Product candidate was accepted")
    original = campaign.campaign_file(root).read_bytes()
    changed = campaign.load_campaign(root)
    changed["works"][campaign.work_key("volicord", "C")]["state"] = "prepared"
    campaign.save_campaign(root, changed)
    try:
        try:
            campaign.activate_all(root)
        except campaign.CampaignError:
            pass
        else:
            raise AssertionError("activation accepted incomplete frozen preparation")
    finally:
        campaign.campaign_file(root).write_bytes(original)

    start = next(path for path in captures
        if "volicord-A-work" in path.name)
    descriptor = campaign.read_json(campaign.frozen_descriptor_path(root, "volicord", "A"))
    mismatched = replaced_capture(start, parent / "wrong-first-task.jsonl",
        descriptor["work_user_task"], descriptor["work_user_task"] + " changed")
    try:
        campaign.map_batch_rollouts(root, [mismatched if path == start else path for path in captures])
    except campaign.CampaignError as error:
        assert "frozen_task_mismatch" in str(error.diagnostic)
    else:
        raise AssertionError("task-byte mismatch mapped as a frozen session")

    first, second = captures[:2]
    first_id = campaign.load_codex_capture(first).session_id
    second_id = campaign.load_codex_capture(second).session_id
    duplicate = replaced_capture(second, parent / "duplicate-session.jsonl", second_id, first_id)
    try:
        campaign.map_batch_rollouts(root, [duplicate if path == second else path for path in captures])
    except campaign.CampaignError as error:
        assert "session identity" in str(error)
    else:
        raise AssertionError("duplicate fresh session identity was accepted")
    assert len(campaign.map_batch_rollouts(root, captures)) == 8

    # The exact same frozen campaign admits CLI and extension captures. Metadata
    # remains observed; no filename or source/originator pair decides admission.
    for index, path in enumerate(captures):
        events = [json.loads(line) for line in path.read_text().splitlines()]
        source, originator = (("cli", "codex_cli_rs") if index % 2 == 0 else
                             ("vscode", "codex_vscode/alternate"))
        events[0]["payload"].update(source=source, originator=originator,
                                    client_version="observed-client", model_provider="openai")
        path.write_text("".join(json.dumps(event) + "\n" for event in events))
    assert len(campaign.map_batch_rollouts(root, captures)) == 8

    original_run_checked = campaign.run_checked
    campaign.run_checked = fake_enable_command
    try:
        assert campaign.activate_all(root)["journey_count"] == 3
    finally:
        campaign.run_checked = original_run_checked

    def reject_identity(source: Path, old: str, new: str, label: str) -> None:
        changed = replaced_capture(source, parent / f"{label}.jsonl", old, new)
        before = campaign.campaign_file(root).read_bytes()
        try:
            campaign.collect_batch(root,
                [changed if path == source else path for path in captures],
                exporter=batch_exporter(bundles), documenter=documenter,
                snapshotter=snapshotter)
        except campaign.CampaignError:
            pass
        else:
            raise AssertionError(f"{label} was accepted")
        assert campaign.campaign_file(root).read_bytes() == before
        assert not (root / "evidence-set.json").exists()

    start_capture = campaign.load_codex_capture(start)
    reject_identity(start, str(start_capture.cwd), str(parent / "wrong-workspace"), "wrong-workspace")
    from codex_events import activation_identity
    reject_identity(start, activation_identity(start_capture.cwd, start_capture.session_id),
                    "missing activation", "missing-activation")
    reject_identity(start, activation_identity(start_capture.cwd, start_capture.session_id),
                    activation_identity(start_capture.cwd, "wrong-session"), "wrong-activation")

    journey = campaign.load_campaign(root)["journeys"][campaign.journey_id("volicord")]
    repository = Path(journey["repository_path"])
    # Live candidate-owned MCP/Runtime binding must still match at collection.
    for old, new, label in ((journey["runtime_home"], str(parent / "wrong-runtime"), "wrong-runtime"),
                            (str(binary.with_name("volicord-mcp")), str(parent / "other-mcp"), "wrong-candidate-mcp")):
        config = repository / ".codex/config.toml"
        original_config = config.read_bytes()
        try:
            config.write_text(config.read_text().replace(old, new))
            try:
                campaign.collect_batch(root, captures, exporter=batch_exporter(bundles),
                                       documenter=documenter, snapshotter=snapshotter)
            except campaign.CampaignError:
                pass
            else:
                raise AssertionError(f"{label} was accepted")
            assert not (root / "evidence-set.json").exists()
        finally:
            config.write_bytes(original_config)

    reject_identity(captures[1],
        hashlib.sha256(b"work:volicord:A").hexdigest()[:32],
        hashlib.sha256(b"work:volicord:B").hexdigest()[:32],
        "wrong-same-work-resume")
    reject_identity(captures[1],
        hashlib.sha256(b"work:volicord:A").hexdigest()[:32], "",
        "missing-same-work-identity-with-valid-git-boundary")
    reject_identity(captures[2],
        hashlib.sha256(b"project:volicord").hexdigest()[:32],
        hashlib.sha256(b"project:small-python").hexdigest()[:32],
        "wrong-volicord-project")
    reject_identity(captures[4],
        hashlib.sha256(b"project:small-python").hexdigest()[:32],
        hashlib.sha256(b"project:volicord").hexdigest()[:32],
        "cross-journey-project-mixing")

    summary = campaign.collect_batch(root, captures,
        exporter=batch_exporter(bundles), documenter=documenter,
        snapshotter=snapshotter)
    assert summary["collection_state"] == "collected"
    assert len(summary["works"]) == 5 and len(summary["journey_final_evidence"]) == 3
    assert len({work["sessions"]["start"]["session_id"] for work in summary["works"]}) == 5
    assert len({work["project_id"] for work in summary["works"]}) == 3
    volicord = [work for work in summary["works"] if work["repository_class"] == "volicord"]
    assert len({work["project_id"] for work in volicord}) == 1
    assert len({work["work_item_id"] for work in volicord}) == 3
    for work in summary["works"]:
        if work["work_label"] == "A":
            assert work["sessions"]["start"]["session_id"] != work["sessions"]["resume"]["session_id"]
    manifest = campaign.load_evidence_set(root)
    assert manifest["schema_version"] == 5 and len(manifest["raw_inputs"]) == 8
    observed_sources = set()
    for work in manifest["work_evidence"]:
        for session in work["sessions"].values():
            retained = campaign.load_codex_capture(root / session["relative_evidence_path"])
            assert session["provenance"] == retained.provenance_evidence()
            observed_sources.add(session["provenance"]["observed_metadata"]["session_meta"]["source"])
    assert observed_sources == {"cli", "vscode"}
    raw = root / manifest["work_evidence"][0]["sessions"]["start"]["relative_evidence_path"]
    original_raw = raw.read_bytes()
    raw.write_bytes(original_raw + b"\n")
    try:
        try:
            campaign.load_evidence_set(root)
        except campaign.CampaignError:
            pass
        else:
            raise AssertionError("raw evidence tampering was accepted")
    finally:
        raw.write_bytes(original_raw)
    campaign.load_evidence_set(root)
    evaluation = campaign.evaluate_campaign(root, parent / "current-evaluation")
    assert evaluation["qualification_state"] == "not_run"
    evaluated = campaign.read_json(root / evaluation["evaluation"])
    assert all(work["observation"]["machine_facts"]["measured_session_provenance"]["status"]
               == "confirmed_pass" for work in evaluated["works"])


def assert_checkpoint_free_completed_resume_collects(parent: Path, binary: Path) -> None:
    root, captures, bundles = prepared_batch(parent, "checkpoint-free-resume", binary)
    start = next(path for path in captures if "volicord-A-work" in path.name)
    resume = next(path for path in captures if "volicord-A-resume" in path.name)
    start.write_text(start.read_text(encoding="utf-8").replace("paused", "completed"),
        encoding="utf-8")
    repository = Path(campaign.work_state(root, "volicord", "A")["repository_path"])
    baseline, committed = FIXTURE_COMMIT_LINES[repository.resolve()][:2]
    lines = []
    for line in resume.read_text(encoding="utf-8").splitlines():
        event = json.loads(line)
        call_id = event.get("payload", {}).get("call_id", "")
        if ("resume-patch-" in call_id or "resume-checkpoint-call" in call_id
                or call_id.startswith("commit-")):
            continue
        lines.append(line.replace("paused", "completed").replace(
            f'"commit_hash":"{baseline}"', f'"commit_hash":"{committed}"'))
    resume.write_text("\n".join(lines) + "\n", encoding="utf-8")
    bundle = bundles[campaign.work_key("volicord", "A")]
    envelope = json.loads(bundle.read_text(encoding="utf-8").replace("paused", "completed"))
    payload = envelope["payload"]
    envelope["checksum"] = hashlib.sha256(json.dumps(payload, ensure_ascii=False,
        separators=(",", ":")).encode()).hexdigest()
    campaign.write_json(bundle, envelope)
    original_run_checked = campaign.run_checked
    campaign.run_checked = fake_enable_command
    try:
        campaign.activate_all(root)
    finally:
        campaign.run_checked = original_run_checked
    no_write = campaign.load_codex_capture(resume)
    assert not no_write.successful_calls("checkpoint_record")
    assert not harness.meaningful_work_path_observations(no_write)
    descriptor = campaign.read_json(campaign.frozen_descriptor_path(root, "volicord", "A"))
    state = {**campaign.work_state(root, "volicord", "A"),
        "project_id": hashlib.sha256(b"project:volicord").hexdigest()[:32],
        "work_item_id": hashlib.sha256(b"work:volicord:A").hexdigest()[:32],
        "start_session_id": campaign.load_codex_capture(start).session_id}
    assert campaign.inspect_resume(no_write, descriptor, state) == state["project_id"]
    summary = campaign.collect_batch(root, captures,
        exporter=batch_exporter(bundles), documenter=documenter,
        snapshotter=snapshotter)
    assert summary["collection_state"] == "collected"
    work = next(item for item in summary["works"]
        if item["repository_class"] == "volicord" and item["work_label"] == "A")
    assert work["work_item_id"] == hashlib.sha256(b"work:volicord:A").hexdigest()[:32]
    lineage = next(entry["repository_revision_lineage"]
        for entry in summary["journey_final_evidence"] if entry["journey_id"] == "journey-volicord")
    observation = next(entry for entry in lineage["session_git_observations"]
                       if entry["work_label"] == "A" and entry["role"] == "start")
    assert observation["next_observation_head"] == committed
    assert observation["path_correlation_to_next_observation"]["correlated_paths"] == ["src/existing.rs", "tests/existing.rs"]
    assert observation["end_status"]["source_sha256"] == campaign.load_codex_capture(start).source_sha256
    assert observation["end_status"]["session_id"] != no_write.session_id


def main() -> int:
    from resume_self_test import check_resume_regressions
    from document_realization_self_test import check_document_realization_regressions
    from long_lived_project_self_test import check_long_lived_project_regressions
    from repository_state_self_test import check_repository_state_regressions
    from evidence_controls_self_test import check_evidence_control_regressions
    check_resume_regressions()
    check_long_lived_project_regressions()
    check_document_realization_regressions()
    check_repository_state_regressions()
    check_evidence_control_regressions()
    original_clean = harness.git_clean
    harness.git_clean = lambda _path: True
    try:
        with tempfile.TemporaryDirectory(prefix="volicord-dogfood-campaign-") as temporary:
            parent = Path(temporary)
            binary = parent / "candidate/bin/volicord"
            write_fake_binary(binary)
            assert_inventory_diagnostic(parent, binary)
            assert_current_campaign_contract(parent, binary)
            assert_checkpoint_free_completed_resume_collects(parent, binary)
    finally:
        harness.git_clean = original_clean
    print(json.dumps({"status": "passed", "checks": [
        "three_journeys_five_works_three_resumes_eight_frozen_tasks",
        "activation_requires_complete_preparation",
        "candidate_task_session_project_work_and_raw_identity",
        "current_prepare_activate_collect_evaluate_without_semantic_profile",
        "document_realization_resume_and_repository_state_support",
    ]}, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
