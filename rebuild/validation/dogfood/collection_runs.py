"""Candidate-preserving collection identities and copy-on-write reprocessing.

Only the current format is produced. Source observations are copied, never
repaired; Product reads use the observed executable and private Runtime copies.
"""
from contextlib import contextmanager
from contextvars import ContextVar
from datetime import datetime, timezone
import copy
import json
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import tempfile

import machine_findings as machine
import review_operations as operations

CONTRACT = "immutable-dogfood-collection-1"
RUNTIMES = ContextVar("collection_runtimes", default={})
PROCESS_ROOT = ContextVar("collection_process_root", default=None)
OWNER_FILES = ("rebuild/docs/design/validation-plan.md",
    "rebuild/docs/design/privacy-and-provider-boundary.md",
    "rebuild/docs/design/failure-and-recovery.md")


def api():
    import campaign
    return campaign


def runtime_path(path):
    return RUNTIMES.get().get(str(path), Path(path))


@contextmanager
def runtime_scope(paths, process_root=None):
    token = RUNTIMES.set(paths)
    process_token = PROCESS_ROOT.set(process_root)
    try:
        yield
    finally:
        RUNTIMES.reset(token)
        PROCESS_ROOT.reset(process_token)


def record_product_read(meta, stdout, stderr):
    root = PROCESS_ROOT.get()
    if root is not None:
        directory = root / secrets.token_hex(16)
        directory.mkdir(parents=True)
        (directory / "stdout").write_bytes(stdout)
        (directory / "stderr").write_bytes(stderr)
        api().write_json(directory / "process.json", {**meta, "phase": "post_session_collection"})


def require_publication(root):
    path = root / "collection/publication.json"
    if path.is_file():
        value = api().read_json(path)
        if value.get("collection_state") != "collected":
            raise api().CampaignError("evidence-set prerequisite not published", diagnostic={
                "kind": "dogfood_prerequisite_not_run", "status": "not_run", "invocation_count": 0,
                "prerequisite": {"path": "collection/publication.json", "sha256": api().harness.sha256(path),
                    "collection_run_id": value.get("collection_run_id"), "reason": value.get("blocker")}})


def verify_publication(root):
    """Copied publications verify using bounded retained files, never live state."""
    c = api()
    publication = c.read_json(root / 'collection/publication.json')
    if publication.get('publication_id') != machine.digest({k: v for k, v in publication.items() if k != 'publication_id'}):
        raise c.CampaignError('collection publication hash changed')
    c.verify_inventory(root)
    if publication['collection_state'] == 'collected':
        manifest = c.read_json(root / 'evidence-set.json')
        if manifest.get('schema_version') != 9:
            raise c.CampaignError('unsupported collection evidence-set schema')
        verify(root, manifest)
        c.verify_retained_repository_states(root, manifest)
    elif publication['collection_state'] == 'incomplete':
        run = c.read_json(root / 'collection/run.json')
        original = c.read_json(root / 'collection/inputs/source/campaign.json')
        normalized = c.read_json(root / 'collection/normalization.json')
        # Verify run provenance against retained source metadata; this ephemeral
        # comparison supplies no evidence-set publication or evaluation authority.
        verify(root, {**original, 'raw_inputs': normalized['raw_inputs'],
            'collection_run': {'path': 'collection/run.json', 'run_id': run['run_id'],
                'sha256': c.harness.sha256(root / 'collection/run.json')}}, publication_output=False)
        if (publication.get('evidence_set') is not None or (root / 'evidence-set.json').exists()
            or not isinstance(publication.get('blocker'), dict)
            or publication.get('collection_run_id') != run.get('run_id')
            or normalized.get('collection_run_id') != run['run_id']):
            raise c.CampaignError('incomplete collection cannot supply evidence-set success')
        expected = {name: {'status': 'not_run', 'invocation_count': 0,
            'prerequisite': {'stage': 'collection', 'run_id': run['run_id'], 'reason': publication['blocker']}}
            for name in ('evaluation', 'cli_observations', 'review', 'qualification', 'result_lineage')}
        if publication.get('dependent_stages') != expected:
            raise c.CampaignError('dependent stage executed without evidence-set prerequisite')
    else:
        raise c.CampaignError('unknown collection publication state')
    return {'state': 'verified', 'collection_state': publication['collection_state'],
        'publication_id': publication['publication_id'], 'external_staging_paths_used': False}


def binding(path):
    return {"bytes": path.stat().st_size, "sha256": api().harness.sha256(path)}


def producer_paths():
    # Reuse the bounded maintained pipeline dependency inventory.
    import rehearsal_contract
    base = Path(__file__).parent
    paths = [(base / name).resolve() for name in rehearsal_contract.PRODUCER_FILES]
    paths += [Path(__file__).resolve(), *(api().ROOT / name for name in OWNER_FILES)]
    return {path.relative_to(api().ROOT).as_posix(): path for path in sorted(set(paths))}


def verify_collector_revision(value):
    """Exact immutable Git revision, not a claim that current Product ran chats."""
    for name, expected in value["producers"].items():
        result = subprocess.run(["git", "show", value["collector_revision"] + ":" + name],
            cwd=api().ROOT, stdin=subprocess.DEVNULL, capture_output=True, check=False)
        if result.returncode or {"bytes": len(result.stdout), "sha256": operations.digest(result.stdout)} != expected:
            raise api().CampaignError("collector revision/dependency identity mismatch: " + name)


def source_files(root, raw_paths):
    c = api()
    c.verify_inventory(root)
    names = set(c.load_inventory(root)["artifacts"]) | {"campaign.json", "evidence-inventory.json"}
    # Retained observations outside the inventory must not vanish on reprocessing.
    for directory in ("raw-rollouts", "setup-rollouts", "recheck-rollouts", "steward", "operator",
                      "slots", "tasks", "explanations", "resources"):
        names.update(p.relative_to(root).as_posix() for p in (root / directory).rglob("*") if p.is_file())
    files = {"source/" + name: operations.safe_path(root, name) for name in sorted(names)}
    for index, path in enumerate(sorted(Path(p).resolve() for p in raw_paths)):
        files[f"raw/{index}.jsonl"] = path
    return files


def request(root, raw_paths, *, mode, rejected=()):
    c = api()
    campaign = c.load_campaign(root)
    files = source_files(root, raw_paths)
    for index, path in enumerate(rejected):
        files[f"rejected/{index}"] = Path(path).resolve()
    producers = producer_paths()
    for path in [*files.values(), *producers.values()]:
        if path.is_symlink() or not path.is_file():
            raise c.CampaignError("collection input must be a retained regular file: " + str(path))
    value = {"kind": "dogfood_collection_run", "schema_version": 1, "contract": CONTRACT,
        "mode": mode, "source_campaign": {"campaign_id": campaign["campaign_id"],
            "root": str(root.resolve()), "sha256": c.harness.sha256(root / "campaign.json")},
        "candidate_head": campaign["candidate_head"], "candidate_artifacts": campaign["candidate_artifacts"],
        "collector_revision": c.harness.git_head(c.ROOT), "run_nonce": secrets.token_hex(16),
        "started_at": datetime.now(timezone.utc).isoformat(),
        "inputs": {name: binding(path) for name, path in files.items()},
        "rejected_attempts": [f"rejected/{i}" for i in range(len(rejected))],
        "producers": {name: binding(path) for name, path in producers.items()},
        "policy": {"contract": CONTRACT, "sha256": machine.digest({name: binding(c.ROOT / name) for name in OWNER_FILES})}}
    value["run_id"] = machine.digest(value)
    return value, files, producers


def retain(root, value, files, producers):
    c = api()
    for prefix, sources in (("collection/inputs/", files), ("collection/producers/", producers)):
        for name, path in sources.items():
            destination = root / (prefix + name)
            c.copy_exact(path, destination)
            c.register_artifact(root, destination)
    c.write_json(root / "collection/run.json", value)
    c.register_artifact(root, root / "collection/run.json")


def unchanged(value, files, producers):
    raw_paths = [path for name, path in files.items() if name.startswith('raw/')]
    current = source_files(Path(value['source_campaign']['root']), raw_paths)
    current.update({name: path for name, path in files.items() if name.startswith('rejected/')})
    if ({name: binding(path) for name, path in current.items()} != value["inputs"]
        or {name: binding(path) for name, path in producers.items()} != value["producers"]
        or api().harness.git_head(api().ROOT) != value["collector_revision"]):
        raise api().CampaignError("collection source or collector changed during processing")


def verify(root, manifest, *, publication_output=True):
    """Verify retained provenance without the source campaign or staging Runtime."""
    c = api()
    value = c.read_json(root / "collection/run.json")
    if (value.get("kind") != "dogfood_collection_run" or value.get("schema_version") != 1
        or value.get("contract") != CONTRACT or value.get("mode") not in {"live", "reprocess"}
        or value.get("run_id") != machine.digest({k: v for k, v in value.items() if k != "run_id"})
        or manifest.get("collection_run") != {"path": "collection/run.json", "run_id": value["run_id"],
            "sha256": c.harness.sha256(root / "collection/run.json")}
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("collector_revision"))) is None
        or re.fullmatch(r"[0-9a-f]{32}", str(value.get("run_nonce"))) is None):
        raise c.CampaignError("invalid collection run identity")
    for key, prefix in (("inputs", "collection/inputs/"), ("producers", "collection/producers/")):
        for name, expected in value[key].items():
            path = operations.safe_path(root, prefix + name)
            if path.is_symlink() or binding(path) != expected:
                raise c.CampaignError("collection retained input/producer changed")
    if set(value["producers"]) != set(producer_paths()):
        raise c.CampaignError("collection producer dependency inventory changed")
    if value["mode"] == "reprocess":
        verify_collector_revision(value)
    original = c.read_json(root / "collection/inputs/source/campaign.json")
    if (value["source_campaign"] != {"campaign_id": original["campaign_id"], "root": original["campaign_root"],
            "sha256": value["inputs"]["source/campaign.json"]["sha256"]}
        or value["candidate_head"] != original["candidate_head"]
        or value["candidate_head"] != manifest["candidate_head"]
        or value["candidate_artifacts"] != original["candidate_artifacts"]
        or value["candidate_artifacts"] != manifest["candidate_artifacts"]
        or original["campaign_id"] != manifest["campaign_id"]
        or original["naturalistic_memory_evidence"] != manifest["naturalistic_memory_evidence"]):
        raise c.CampaignError("collection source/Product candidate or resource binding changed")
    original_inventory = c.read_json(root / 'collection/inputs/source/evidence-inventory.json')
    if any(value['inputs'].get('source/' + name) != expected
            for name, expected in original_inventory['artifacts'].items()):
        raise c.CampaignError('collection omitted an inventoried source artifact')
    if value['mode'] == 'live' and value['candidate_head'] != value['collector_revision']:
        raise c.CampaignError('live collection must be candidate-owned')
    rejected = sorted(name for name in value["inputs"] if name.startswith("rejected/"))
    if sorted(value["rejected_attempts"]) != rejected:
        raise c.CampaignError("historical rejected attempt omitted")
    owners = {name: value["producers"][name] for name in OWNER_FILES}
    if value["policy"] != {"contract": CONTRACT, "sha256": machine.digest(owners)}:
        raise c.CampaignError("collection policy identity changed")
    retained_raw = sorted(v["sha256"] for n, v in value["inputs"].items() if n.startswith("raw/"))
    if retained_raw != sorted(item["sha256"] for item in manifest["raw_inputs"]):
        raise c.CampaignError("collection raw input binding changed")
    publication_path = root / "collection/publication.json"
    if publication_output and publication_path.is_file():
        publication = c.read_json(publication_path)
        if (publication.get("publication_id") != machine.digest({k: v for k, v in publication.items() if k != "publication_id"})
            or publication.get("collection_run_id") != value["run_id"]
            or publication.get("candidate_head") != value["candidate_head"]
            or publication.get("collector_revision") != value["collector_revision"]
            or publication.get("blocker") is not None
            or publication.get("collection_state") != "collected"
            or publication.get("evidence_set") != {"path": "evidence-set.json", "sha256": c.harness.sha256(root / "evidence-set.json")}):
            raise c.CampaignError("collection publication output identity changed")
    return value


def tree_identity(root):
    """Runtime copies require every retained authoritative/derived byte, no symlinks."""
    if not root.is_dir():
        raise api().CampaignError("original candidate Runtime unavailable: " + str(root))
    paths = sorted(root.rglob("*"))
    if any(p.is_symlink() or not (p.is_file() or p.is_dir()) for p in paths):
        raise api().CampaignError("unsupported original Runtime entry: " + str(root))
    return {p.relative_to(root).as_posix(): binding(p) for p in paths if p.is_file()}


def reprocess(source, raw_paths, output, *, source_sha256, rejected=(),
              exporter=None, documenter=None, snapshotter=None):
    """One immutable source; failures publish normalization plus one causal blocker."""
    c = api()
    source, output = source.resolve(), output.resolve()
    if output.is_relative_to(source) or source.is_relative_to(output) or output.exists():
        raise c.CampaignError("collection publication must be new and disjoint from source")
    c.require_current_candidate(c.harness.git_head(c.ROOT))
    campaign = c.load_campaign(source)
    if c.harness.sha256(source / "campaign.json") != source_sha256:
        raise c.CampaignError("source campaign hash mismatch")
    c.verify_frozen_campaign(source, campaign, verify_executables=False)
    if campaign["collection_state"] != "pending" or campaign["terminal_outcome"] is not None:
        raise c.CampaignError("reprocessing requires an immutable pending source, not a stopped Product campaign")
    value, files, producers = request(source, raw_paths, mode="reprocess", rejected=rejected)
    verify_collector_revision(value)
    mapped = c.map_batch_rollouts(source, raw_paths)
    for rollout in mapped.values():
        failure = c.harness.activation_failure(rollout.capture)
        if failure is not None:
            raise c.CampaignError("required retained session activation is invalid")
    output.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".collection-", dir=output.parent))
    runtime_copies, runtime_before, repositories = {}, {}, {}
    try:
        for name in c.load_inventory(source)["artifacts"]:
            c.copy_exact(source / name, stage / name)
        c.write_json(stage / "evidence-inventory.json", c.load_inventory(source))
        staged = copy.deepcopy(campaign)
        staged["campaign_root"] = str(stage)
        c.save_campaign(stage, staged)
        retain(stage, value, files, producers)
        for (kind, work, role), rollout in mapped.items():
            destination = c.work_root(stage, kind, work) / "evidence" / f"{role}.rollout.jsonl"
            c.copy_exact(rollout.source, destination)
            c.register_artifact(stage, destination)
        c.resolve_batch_identities(stage, mapped)
        normalization = {"kind": "dogfood_retained_observations", "schema_version": 1,
            "collection_run_id": value["run_id"], "candidate_head": campaign["candidate_head"],
            "raw_inputs": c.document_realization.raw_binding(mapped),
            "works": c.load_campaign(stage)["works"],
            "execution_coverage": {c.session_slot_id(*slot): rollout.capture.execution_evidence()
                for slot, rollout in mapped.items()}}
        c.write_json(stage / "collection/normalization.json", normalization)
        c.register_artifact(stage, stage / "collection/normalization.json")
        blocker = None
        try:
            c.verify_candidate_artifacts(campaign)
            for identity, journey in campaign["journeys"].items():
                runtime, repository = Path(journey["runtime_home"]), Path(journey["repository_path"])
                c.verify_static_codex_integration(repository, runtime, Path(campaign["candidate_binary"]))
                repositories[identity] = c.repository_state.observe(repository)[0]
                before = tree_identity(runtime)
                runtime_before[str(runtime)] = before
                destination = stage / ".capabilities" / identity
                shutil.copytree(runtime, destination)
                if tree_identity(destination) != before or tree_identity(runtime) != before:
                    raise c.CampaignError("original Runtime changed during immutable copy: " + str(runtime))
                runtime_copies[str(runtime)] = destination
            c.write_json(stage / "collection/capabilities.json", {
                "phase": "post_session_collection", "candidate_artifacts": campaign["candidate_artifacts"],
                "runtime_inputs": runtime_before, "repository_inputs": repositories,
                "runtime_policy": "verified_private_copy_preserving_original_repository_binding",
                "expected_projects": {key: journey["project_id"] for key, journey in c.load_campaign(stage)["journeys"].items()}})
            c.register_artifact(stage, stage / "collection/capabilities.json")
            with runtime_scope(runtime_copies, stage / "collection/post-session-processes"):
                c.explanation_evidence.require_ready(stage, campaign, mapped)
                # Frozen document binding uses the original campaign bytes.
                c.document_realization.require_batch_ready(source, campaign, mapped)
                for path in (stage / "collection/post-session-processes").rglob("*"):
                    if path.is_file():
                        c.register_artifact(stage, path)
                with c.candidate_artifact_use(campaign):
                    summary = c.normalize_batch(stage, mapped, exporter=exporter or observed_export,
                        documenter=documenter or c.generate_document, snapshotter=snapshotter or observed_snapshot)
            c.verify_final_repository_states_for_publication(stage)
        except (ValueError, OSError) as error:
            blocker = {"kind": type(error).__name__, "detail": str(error),
                "diagnostic": getattr(error, "diagnostic", None),
                "required_observation": "original-candidate post-session materialization; retained chats are not rerun"}
            summary = {"collection_state": "incomplete", "outcome": "evidence_failed"}
        # Source drift is an integrity rejection, never a publishable incomplete result.
        unchanged(value, files, producers)
        for path, before in runtime_before.items():
            if tree_identity(Path(path)) != before:
                raise c.CampaignError("source Runtime mutated during reprocessing: " + path)
        for identity, state in repositories.items():
            c.repository_state.verify(Path(campaign["journeys"][identity]["repository_path"]), state)
        c.require_current_candidate(value["collector_revision"])
        shutil.rmtree(stage / ".capabilities", ignore_errors=True)
        for path in (stage / "collection/post-session-processes").rglob("*"):
            if path.is_file() and c.relative(stage, path) not in c.load_inventory(stage)["artifacts"]:
                c.register_artifact(stage, path)
        publication = {"kind": "dogfood_collection_publication", "schema_version": 1,
            "collection_run_id": value["run_id"], "candidate_head": campaign["candidate_head"],
            "collector_revision": value["collector_revision"], "blocker": blocker,
            "collection_state": summary["collection_state"], "completed_at": datetime.now(timezone.utc).isoformat(),
            "evidence_set": c.load_campaign(stage).get("evidence_set") if blocker is None else None}
        if blocker is not None:
            # An incomplete publication has observations, no evidence-dependent claims.
            (stage / "evidence-set.json").unlink(missing_ok=True)
            final = c.load_campaign(stage)
            final.update(campaign_root=str(output), collection_state="incomplete", evidence_set=None,
                evaluation_state="not_run", qualification_state="not_run")
            c.save_campaign(stage, final)
            publication["dependent_stages"] = {name: {"status": "not_run", "invocation_count": 0,
                "prerequisite": {"stage": "collection", "run_id": value["run_id"], "reason": blocker}}
                for name in ("evaluation", "cli_observations", "review", "qualification", "result_lineage")}
        else:
            c.load_evidence_set(stage)
            final = c.load_campaign(stage)
            final["campaign_root"] = str(output)
            c.save_campaign(stage, final)
        publication["publication_id"] = machine.digest(publication)
        c.write_json(stage / "collection/publication.json", publication)
        # Include only indexed observation outputs, provenance and closed metadata.
        names = set(c.load_inventory(stage)["artifacts"]) | {"campaign.json", "evidence-inventory.json", "collection/publication.json"}
        names = {n for n in names if (stage / n).is_file()}
        operations.publish_directory(output, {name: (stage / name).read_bytes() for name in names})
        return {**publication, "publication_root": str(output)}
    finally:
        shutil.rmtree(stage)


def observed_export(binary, runtime, repository, destination):
    c = api()
    recorder = c.harness.load_v11().Recorder(destination.parent / "context-export-process")
    process = recorder.run("post-session-export", [str(binary), "--runtime", str(runtime),
        "--repository", str(repository), "context", "export", "--output", str(destination)],
        __import__("os").environ.copy(), cwd=repository)
    for path in (destination.parent / "context-export-process").rglob("*"):
        if path.is_file():
            c.register_artifact(destination.parents[2], path)
    if process.get("exit_code") != 0 or process.get("termination") or not destination.is_file():
        raise c.CampaignError("original-candidate post-session export failed")


def observed_snapshot(binary, runtime, project, destination, locale, language):
    c = api()
    recorder = c.harness.load_v11().Recorder(destination.parent / "snapshot-process")
    process = recorder.run("post-session-viewer", [str(binary.with_name("volicord-viewer")),
        "--runtime", str(runtime), "--project", project, "--locale", locale,
        "--language", language, "--snapshot", str(destination)], __import__("os").environ.copy(), cwd=c.ROOT)
    for path in (destination.parent / "snapshot-process").rglob("*"):
        if path.is_file():
            c.register_artifact(destination.parents[3], path)
    return {"status": "passed" if process.get("exit_code") == 0 and not process.get("termination")
        and destination.is_file() else "failed"}
