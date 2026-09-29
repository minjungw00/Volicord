"""Private, non-qualifying active-host document preparation for Dogfood.

Only Product constructs plans and renders content. This module never realizes
prose, dispatches a semantic provider, or changes canonical Project records.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import sys
from typing import Any

import harness
import identity_provenance
import codex_events


def campaign_api():
    main = sys.modules.get("__main__")
    if getattr(main, "__file__", None) and Path(main.__file__).resolve() == Path(__file__).with_name("campaign.py"):
        return main
    import campaign
    return campaign


def required(language: str, locale: str) -> bool:
    # Same bounded locale availability as Product; regional tags are supported.
    language = language.strip().lower()
    return not (language == locale or language.startswith(locale + "-"))


def preview(binary: Path, runtime: Path, arguments: dict[str, Any]) -> dict[str, Any]:
    """Reuse the maintained stdio MCP client, without running a V11 journey."""
    c = campaign_api()
    env = {**os.environ, "VOLICORD_RUNTIME_DIR": str(runtime)}
    client = harness.load_v11().Mcp(binary.with_name("volicord-mcp"), env)
    try:
        catalog = client.initialize()
        if not any(t.get("name") == "document_preview" for t in catalog):
            raise c.CampaignError("candidate does not expose document_preview")
        result, ok = client.tool("document_preview", arguments)
        if not ok or not isinstance(result, dict):
            raise c.CampaignError("Product rejected document realization or preview")
        if result.get("canonical_mutation") is not False:
            raise c.CampaignError("document preview did not preserve its read-only contract")
        return result
    finally:
        cleanup = client.close()
        if cleanup.get("exit_code") != 0:
            raise c.CampaignError("document preview MCP process did not exit successfully")


def route(binary: Path) -> dict[str, str]:
    c = campaign_api()
    mcp = binary.with_name("volicord-mcp")
    if not mcp.is_file() or not os.access(mcp, os.X_OK):
        raise c.CampaignError("cross-locale documents require the candidate volicord-mcp executable before sessions")
    return {"boundary": "active_host_document_preview", "mcp_sha256": harness.sha256(mcp)}


def verify_route(campaign: dict[str, Any]) -> None:
    c = campaign_api()
    c.verify_candidate_artifacts(campaign, ("volicord-mcp",))
    bound = campaign["candidate_artifacts"]["volicord-mcp"]
    if (campaign.get("document_realization_route") != {
            "boundary": "active_host_document_preview", "mcp_sha256": bound["sha256"]}):
        raise campaign_api().CampaignError("cross-locale active-host realization route is missing or changed")


def artifact(root: Path, plane: str, identity: str) -> Path:
    if re.fullmatch(r"[0-9a-f]{32}", identity or "") is None:
        raise campaign_api().CampaignError("invalid realization identity")
    return root / "realizer" / plane / f"{identity}.json"


def bound_bytes(root: Path, path: Path) -> bytes:
    c = campaign_api()
    name = c.relative(root, path)
    if not path.is_file():
        raise c.CampaignError("required realization artifact is not prepared or fixed")
    data = path.read_bytes()
    entry = c.load_inventory(root)["artifacts"].get(name)
    if entry != {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}:
        raise c.CampaignError("realization artifact inventory binding changed")
    return data


def publish(root: Path, files: dict[Path, bytes], *, drafts: dict[Path, bytes] | None = None) -> None:
    """Freeze new private artifacts and inventory with rollback on publication error."""
    c = campaign_api()
    inventory_path = c.inventory_path(root)
    original = inventory_path.read_bytes()
    inventory = json.loads(original)
    all_files = {**files, **(drafts or {})}
    if any(p.exists() or c.relative(root, p) in inventory["artifacts"] for p in all_files):
        raise c.CampaignError("realization preparation/record is immutable once published")
    created = []
    try:
        for path, data in all_files.items():
            created.append(path)
            c.atomic_write_bytes(path, data)
            if path in files:
                inventory["artifacts"][c.relative(root, path)] = {
                    "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
        c.atomic_write_bytes(inventory_path, c.json_bytes(inventory))
    except BaseException:
        for path in created:
            path.unlink(missing_ok=True)
        c.atomic_write_bytes(inventory_path, original)
        raise


def raw_binding(mapped) -> list[dict[str, Any]]:
    return [
        {
            "session_slot": list(key),
            "session_slot_id": campaign_api().session_slot_id(*key),
            "session_id": value.capture.session_id,
            "sha256": value.capture.source_sha256,
        }
        for key, value in sorted(mapped.items())
    ]


def request(preparation: dict[str, Any], format_name: str) -> dict[str, Any]:
    return {"project_id": preparation["project_id"], "kind": preparation["document_kind"],
            "locale": preparation["locale"], "language": preparation["language"], "format": format_name}


def current_plan(binary: Path, runtime: Path, preparation: dict[str, Any], format_name: str) -> dict[str, Any]:
    c = campaign_api()
    result = preview(binary, runtime, request(preparation, format_name))
    plan = result.get("plan")
    if (result.get("outcome") != "realization_required" or not isinstance(plan, dict)
        or result.get("kind") != preparation["document_kind"]
        or result.get("format") != format_name
        or result.get("requested_language") != preparation["language"]
        or plan.get("document_kind") != preparation["document_kind"]
        or plan.get("requested_language") != preparation["language"]
        or not re.fullmatch(r"sha256:[0-9a-f]{64}", plan.get("plan_fingerprint", ""))):
        raise c.CampaignError("requested-language document plan is unavailable or invalid; active-host preparation required")
    return plan


def provenance_template(binding: dict[str, Any]) -> dict[str, Any]:
    return {"preparation_binding": binding,
        "runtime_observation": {"state": "not_provided", "source": None,
            "rollout_sha256": None},
        **{field: {"state": "unknown", "value": None}
           for field in ("host", "agent", "model", "session", "runtime")}}


def observed_runtime_provenance(binding: dict[str, Any], rollout: Path) -> dict[str, Any]:
    """Extract host-recorded identity without claiming author attestation."""
    c = campaign_api()
    try:
        capture = codex_events.load_codex_capture(rollout)
        events = [json.loads(line) for line in rollout.read_text(encoding="utf-8").splitlines()]
    except (codex_events.EvidenceError, OSError, UnicodeError, json.JSONDecodeError) as error:
        raise c.CampaignError("runtime identity rollout is unavailable or invalid") from error
    models = []
    for event in events:
        payload = event.get("payload") if isinstance(event, dict) else None
        if event.get("type") == "turn_context" and isinstance(payload, dict) and "model" in payload:
            model = payload["model"]
            if not isinstance(model, str) or not model.strip() or model != model.strip() or len(model.encode()) > 256:
                raise c.CampaignError("runtime model identity is malformed or unbounded")
            models.append(model)
    if not models or len(set(models)) != 1:
        raise c.CampaignError("runtime rollout does not provide one exact consistent model identity")
    observation = {"state": "observed", "source": "codex_vscode_rollout",
        "rollout_sha256": capture.source_sha256}
    return {"preparation_binding": binding, "runtime_observation": observation,
        "host": {"state": "runtime_observed", "value": capture.source},
        "agent": {"state": "runtime_observed", "value": capture.originator},
        "model": {"state": "runtime_observed", "value": models[0]},
        "session": {"state": "runtime_observed", "value": capture.session_id},
        "runtime": {"state": "runtime_observed", "value": f"codex-cli/{capture.cli_version}"}}


def validate_provenance(preparation: dict[str, Any], provenance: Any, *,
                        runtime_rollout: Path | None = None, allow_bound_runtime=False) -> None:
    """The preparation proves a local route, never authorship or exact model identity."""
    c = campaign_api()
    binding = preparation.get("provenance_binding")
    if (preparation.get("schema_version") != 4 or not isinstance(binding, dict)
        or set(binding) != {"state", "source", "candidate_head", "mcp_sha256"}
        or binding.get("state") != "verified"
        or binding.get("source") != "candidate_local_document_preview"
        or binding.get("candidate_head") != preparation.get("candidate_head")
        or not re.fullmatch(r"[0-9a-f]{40}", str(binding.get("candidate_head", "")))
        or not re.fullmatch(r"[0-9a-f]{64}", str(binding.get("mcp_sha256", "")))
        or not isinstance(provenance, dict)
        or set(provenance) != {"preparation_binding", "runtime_observation", "host", "agent", "model",
                              "session", "runtime"}
        or provenance.get("preparation_binding") != binding):
        raise c.CampaignError("realization provenance does not match verified preparation binding")
    observation = provenance["runtime_observation"]
    empty_observation = {"state": "not_provided", "source": None, "rollout_sha256": None}
    if observation == empty_observation:
        allow_runtime = False
        if runtime_rollout is not None:
            raise c.CampaignError("runtime rollout was supplied but stronger identity was not bound into the draft")
    elif (isinstance(observation, dict)
          and set(observation) == {"state", "source", "rollout_sha256"}
          and observation.get("state") == "observed"
          and observation.get("source") == "codex_vscode_rollout"
          and re.fullmatch(r"[0-9a-f]{64}", str(observation.get("rollout_sha256", "")))):
        allow_runtime = True
    else:
        raise c.CampaignError("invalid realization runtime identity observation")
    for field in ("host", "agent", "model", "session", "runtime"):
        try:
            identity_provenance.validate_claim(provenance[field], allow_runtime_observed=allow_runtime)
        except ValueError as error:
            raise c.CampaignError(str(error)) from error
    runtime_claims = all(provenance[field]["state"] == "runtime_observed"
                         for field in ("host", "agent", "model", "session", "runtime"))
    if allow_runtime != runtime_claims:
        raise c.CampaignError("runtime observation and identity claim provenance disagree")
    if allow_runtime:
        if runtime_rollout is not None:
            if provenance != observed_runtime_provenance(binding, runtime_rollout):
                raise c.CampaignError("runtime-observed identity differs from the bound Codex rollout")
        elif not allow_bound_runtime:
            raise c.CampaignError("runtime-observed identity requires its exact rollout during preflight/record")


def bind_runtime_provenance(root: Path, identity: str, draft_path: Path,
                            rollout: Path) -> dict[str, Any]:
    """Replace weaker mutable draft claims with exact host-recorded observations."""
    c = campaign_api()
    preparation_bytes = bound_bytes(root, artifact(root, "plans", identity))
    preparation = json.loads(preparation_bytes)
    if c.relative(root, draft_path) in c.load_inventory(root)["artifacts"]:
        raise c.CampaignError("inventory-bound artifact cannot be a mutable realization draft")
    original = draft_path.read_bytes()
    if len(original) > 2 * 1024 * 1024:
        raise c.CampaignError("realization draft exceeds the private artifact bound")
    draft = json.loads(original)
    validate_value(preparation, preparation_bytes, draft, allow_bound_runtime=True)
    observed = observed_runtime_provenance(preparation["provenance_binding"], rollout)
    for field in ("host", "agent", "model", "session", "runtime"):
        claim = draft["provenance"][field]
        if claim["state"] == "self_reported" and claim["value"] != observed[field]["value"]:
            raise c.CampaignError(f"self-reported {field} conflicts with stronger runtime observation")
    draft["provenance"] = observed
    data = c.json_bytes(draft)
    validate_value(preparation, preparation_bytes, draft, runtime_rollout=rollout)
    c.atomic_write_bytes(draft_path, data)
    return {"state": "runtime_provenance_bound", "realization_id": identity,
        "draft_sha256": hashlib.sha256(data).hexdigest(), "provenance": observed,
        "mutation": "mutable_draft_only", "qualification_state": "not_run"}


def product_generator(provenance: dict[str, Any]) -> dict[str, str]:
    """Adapt provenance to existing Product metadata without implying verification."""
    return {target: ("unknown (unverified)" if provenance[source]["state"] == "unknown"
            else "self_reported (unverified): " + provenance[source]["value"]
            if provenance[source]["state"] == "self_reported"
            else "runtime_observed (host-recorded, authorship unverified): " + provenance[source]["value"])
        for target, source in (("generator", "host"), ("agent", "agent"), ("model", "model"))}


def stderr_progress(value: dict[str, Any]) -> None:
    """Emit bounded machine-readable progress without exposing private paths."""
    print(json.dumps({"kind": "document_realization_preparation_progress", **value},
                     sort_keys=True), file=sys.stderr, flush=True)


def inspect_state(root: Path) -> dict[str, Any]:
    """Inspect immutable preparation/recording state without candidate mutation."""
    c = campaign_api()
    campaign = c.load_campaign(root)
    c.verify_inventory(root)
    total = len(campaign["journeys"]) * len(c.DOCUMENT_KINDS)
    common = {"kind": "document_realization_operation_state", "mutation": "none",
        "candidate_head": campaign["candidate_head"], "expected_realizations": total,
        "qualification_state": "not_run"}
    if not required(campaign["document_language"], campaign["viewer_locale"]):
        return {**common, "state": "fixed_locale_not_required", "preparation_published": False,
            "recorded_realizations": 0, "remaining_realizations": 0,
            "next_action": "continue_with_fixed_locale_collection"}
    binding_path, index_path = root / "realization-bindings.json", root / "realizer/index.json"
    prepared_paths = [binding_path.exists(), index_path.exists()]
    realizer_files = list((root / "realizer").rglob("*.json")) if (root / "realizer").is_dir() else []
    if not any(prepared_paths) and not realizer_files:
        return {**common, "state": "not_prepared", "preparation_published": False,
            "recorded_realizations": 0, "remaining_realizations": total,
            "next_action": "run_prepare_document_realizations"}
    if not all(prepared_paths):
        return {**common, "state": "repair_required", "preparation_published": False,
            "recorded_realizations": 0, "remaining_realizations": total,
            "next_action": "preserve_interrupted_publication_for_inspection",
            "diagnostic": "preparation publication is incomplete"}
    bindings = json.loads(bound_bytes(root, binding_path))
    index = json.loads(bound_bytes(root, index_path))
    documents = index.get("documents") if isinstance(index, dict) else None
    bound_documents = bindings.get("documents") if isinstance(bindings, dict) else None
    if (not isinstance(documents, list) or not isinstance(bound_documents, list)
        or len(documents) != total or len(bound_documents) != total):
        raise c.CampaignError("published realization preparation has an invalid document inventory")
    index_ids = [item.get("realization_id") for item in documents if isinstance(item, dict)]
    binding_ids = [item.get("realization_id") for item in bound_documents if isinstance(item, dict)]
    if (len(index_ids) != total or len(set(index_ids)) != total or set(index_ids) != set(binding_ids)):
        raise c.CampaignError("published realization preparation identities are incomplete or inconsistent")
    recorded = []
    remaining = []
    for item in documents:
        identity = item["realization_id"]
        preparation_bytes = bound_bytes(root, artifact(root, "plans", identity))
        destination = artifact(root, "recorded", identity)
        if destination.exists():
            draft = json.loads(bound_bytes(root, destination))
            validate_value(json.loads(preparation_bytes), preparation_bytes, draft,
                           allow_bound_runtime=True)
            recorded.append(identity)
        else:
            remaining.append(identity)
    state = ("realizations_fully_recorded" if not remaining else
             "preparation_published" if not recorded else "realizations_partially_recorded")
    next_action = ("continue_with_collect_batch" if not remaining else
                   "edit_validate_and_record_existing_drafts")
    return {**common, "state": state, "preparation_published": True,
        "recorded_realizations": len(recorded), "remaining_realizations": len(remaining),
        "remaining_realization_ids": remaining, "next_action": next_action}


def prepare(root: Path, raw_paths: list[Path], *, progress=None) -> dict[str, Any]:
    c = campaign_api()
    campaign = c.load_campaign_for_mutation(root)
    c.verify_inventory(root)
    if campaign.get("terminal_outcome") is not None or any(s.get("state") != "frozen" for s in campaign["works"].values()):
        raise c.CampaignError("realization preparation requires an untouched frozen campaign")
    if not required(campaign["document_language"], campaign["viewer_locale"]):
        return {"state": "fixed_locale", "qualification_state": "not_run"}
    verify_route(campaign)
    binding_path = root / "realization-bindings.json"
    if binding_path.exists():
        raise c.CampaignError("realization plans are already prepared; edit only unfixed drafts")
    if progress:
        progress({"phase": "mapping_inputs", "completed": 0, "total": 1})
    mapped = c.map_batch_rollouts(root, raw_paths)
    if progress:
        progress({"phase": "mapping_inputs", "completed": 1, "total": 1})
    files, drafts, bindings, index = {}, {}, [], []
    binary = Path(campaign["candidate_binary"])
    expected = len(campaign["journeys"]) * len(c.DOCUMENT_KINDS)
    prepared = 0
    for key, state in sorted(campaign["journeys"].items()):
        kind = state["repository_class"]
        work_ids = c.observed_project_ids(mapped[(kind, "A", "start")].capture)
        resume_ids = c.observed_project_ids(mapped[(kind, "A", "resume")].capture)
        if len(work_ids) != 1 or resume_ids != work_ids:
            raise c.CampaignError("raw start/resume captures do not establish one exact journey Project")
        for document_kind in c.DOCUMENT_KINDS:
            identity = secrets.token_hex(16)
            preparation = {"kind": "active_host_document_preparation", "schema_version": 4,
                "realization_id": identity, "candidate_head": campaign["candidate_head"],
                "project_id": work_ids[0], "document_kind": document_kind,
                "language": campaign["document_language"], "locale": campaign["viewer_locale"]}
            preparation["provenance_binding"] = {"state": "verified",
                "source": "candidate_local_document_preview", "candidate_head": campaign["candidate_head"],
                "mcp_sha256": campaign["document_realization_route"]["mcp_sha256"]}
            plan = current_plan(binary, Path(state["runtime_home"]), preparation, "markdown")
            # Product fingerprint excludes output format and generation time.
            if current_plan(binary, Path(state["runtime_home"]), preparation, "html") != plan:
                raise c.CampaignError("Product returned format-dependent realization plans")
            preparation["plan"] = plan
            data = c.json_bytes(preparation)
            if len(data) > 2 * 1024 * 1024:
                raise c.CampaignError("Product plan exceeds the private preparation artifact bound")
            plan_path = artifact(root, "plans", identity)
            files[plan_path] = data
            draft = {"schema_version": 4, "preparation_sha256": hashlib.sha256(data).hexdigest(),
                "provenance": provenance_template(preparation["provenance_binding"]),
                "requested_language": preparation["language"], "all_generated_prose_realized": False,
                "realization": {"plan_fingerprint": plan["plan_fingerprint"], "title": None,
                    "sections": [{"key": s["key"], "title": None,
                        "claims": [{"identity": claim["identity"], "text": None} for claim in s["claims"]]}
                        for s in plan["sections"]]}}
            drafts[artifact(root, "drafts", identity)] = c.json_bytes(draft)
            bindings.append({"journey_id": key, "realization_id": identity, "document_kind": document_kind})
            index.append({"realization_id": identity, "preparation": c.relative(root, plan_path),
                          "draft": c.relative(root, artifact(root, "drafts", identity))})
            prepared += 1
            if progress:
                progress({"phase": "preparing_plans", "completed": prepared, "total": expected})
    if any(harness.sha256(value.source) != value.capture.source_sha256 for value in mapped.values()):
        raise c.CampaignError("raw inputs changed during realization preparation")
    verify_route(campaign)
    files[binding_path] = c.json_bytes({"candidate_head": campaign["candidate_head"],
        "campaign_sha256": harness.sha256(c.campaign_file(root)), "raw_inputs": raw_binding(mapped), "documents": bindings})
    files[root / "realizer/index.json"] = c.json_bytes({"documents": sorted(index, key=lambda x: x["realization_id"])})
    if progress:
        progress({"phase": "publishing", "completed": 0, "total": 1})
    publish(root, files, drafts=drafts)
    if progress:
        progress({"phase": "published", "completed": 1, "total": 1})
    return {"state": "realization_required", "index": "realizer/index.json", "qualification_state": "not_run"}


def validate_value(preparation: dict[str, Any], preparation_bytes: bytes, value: Any, *,
                   runtime_rollout: Path | None = None, allow_bound_runtime=False) -> None:
    c = campaign_api()
    def require(condition, message):
        if not condition:
            raise c.CampaignError(message)
    def text(value):
        return isinstance(value, str) and bool(value.strip()) and len(value.encode("utf-8")) <= 4096
    require(isinstance(value, dict) and set(value) == {"schema_version", "provenance", "preparation_sha256", "requested_language",
        "all_generated_prose_realized", "realization"}, "invalid realization draft shape")
    require(value["schema_version"] == 4, "unsupported realization draft version")
    validate_provenance(preparation, value["provenance"], runtime_rollout=runtime_rollout,
                        allow_bound_runtime=allow_bound_runtime)
    require(value["preparation_sha256"] == hashlib.sha256(preparation_bytes).hexdigest(), "wrong preparation hash")
    require(value["requested_language"] == preparation["language"] and value["all_generated_prose_realized"] is True,
            "active host must review all generated prose in the exact requested language")
    plan, realized = preparation["plan"], value["realization"]
    require(isinstance(realized, dict) and set(realized) == {"plan_fingerprint", "title", "sections"},
            "invalid NarrativeRealization shape")
    require(realized["plan_fingerprint"] == plan["plan_fingerprint"], "wrong plan fingerprint")
    require(text(realized["title"]) and realized["title"] != plan["source_title"], "requested-language title is missing or unrealized")
    sections = realized["sections"]
    require(isinstance(sections, list) and len(sections) == len(plan["sections"]), "wrong section topology")
    for source, section in zip(plan["sections"], sections):
        require(isinstance(section, dict) and set(section) == {"key", "title", "claims"}
            and section["key"] == source["key"] and text(section["title"]), "invalid section identity or text")
        claims = section["claims"]
        require(isinstance(claims, list) and len(claims) == len(source["claims"]), "wrong claim topology")
        for original, claim in zip(source["claims"], claims):
            require(isinstance(claim, dict) and set(claim) == {"identity", "text"}
                and claim["identity"] == original["identity"] and text(claim["text"]), "invalid claim identity or text")
            require(all(term in claim["text"] for term in original["protected_terms"]), "protected code/path term changed")


def validate(root: Path, identity: str, draft_path: Path,
             runtime_rollout: Path | None = None) -> dict[str, Any]:
    """Read only realizer-visible preparation, input and inventory membership."""
    c = campaign_api()
    if c.relative(root, draft_path) in c.load_inventory(root)["artifacts"]:
        raise c.CampaignError("inventory-bound artifact cannot be a mutable realization draft")
    data = bound_bytes(root, artifact(root, "plans", identity))
    if draft_path.stat().st_size > 2 * 1024 * 1024:
        raise c.CampaignError("realization draft exceeds the private artifact bound")
    draft = c.read_json(draft_path)
    validate_value(json.loads(data), data, draft, runtime_rollout=runtime_rollout)
    return {"state": "valid", "realization_id": identity, "qualification_state": "not_run"}


def record(root: Path, identity: str, draft_path: Path,
           runtime_rollout: Path | None = None) -> dict[str, Any]:
    c = campaign_api()
    campaign = c.load_campaign_for_mutation(root)
    c.verify_inventory(root)
    if campaign.get("terminal_outcome") is not None or any(s.get("state") != "frozen" for s in campaign["works"].values()):
        raise c.CampaignError("realization recording requires an untouched frozen campaign")
    verify_route(campaign)
    validate(root, identity, draft_path, runtime_rollout)
    data = draft_path.read_bytes()
    preparation_bytes = bound_bytes(root, artifact(root, "plans", identity))
    preparation = json.loads(preparation_bytes)
    validate_value(preparation, preparation_bytes, json.loads(data), runtime_rollout=runtime_rollout)
    bindings = json.loads(bound_bytes(root, root / "realization-bindings.json"))
    matches = [b for b in bindings["documents"] if b["realization_id"] == identity]
    if len(matches) != 1 or preparation["candidate_head"] != campaign["candidate_head"]:
        raise c.CampaignError("realization preparation candidate/identity mismatch")
    if preparation["provenance_binding"]["mcp_sha256"] != campaign["document_realization_route"]["mcp_sha256"]:
        raise c.CampaignError("verified preparation route differs from candidate route")
    state = campaign["journeys"][matches[0]["journey_id"]]
    for format_name, _ in c.DOCUMENT_FORMATS:
        consume(Path(campaign["candidate_binary"]), Path(state["runtime_home"]), preparation,
                json.loads(data), format_name, runtime_rollout=runtime_rollout)
    verify_route(campaign)
    destination = artifact(root, "recorded", identity)
    publish(root, {destination: data})
    return {"state": "fixed", "realization_id": identity,
            "sha256": hashlib.sha256(data).hexdigest(), "provenance": json.loads(data)["provenance"],
            "qualification_state": "not_run"}


def consume(binary: Path, runtime: Path, preparation: dict[str, Any], draft: dict[str, Any],
            format_name: str, *, runtime_rollout: Path | None = None,
            allow_bound_runtime=False) -> str:
    c = campaign_api()
    if current_plan(binary, runtime, preparation, format_name) != preparation["plan"]:
        raise c.CampaignError("current Product plan changed after preparation")
    validate_provenance(preparation, draft.get("provenance"), runtime_rollout=runtime_rollout,
                        allow_bound_runtime=allow_bound_runtime)
    generator = product_generator(draft["provenance"])
    result = preview(binary, runtime, {**request(preparation, format_name),
        "realization": {**draft["realization"],
            "requested_language": draft["requested_language"],
            "all_generated_prose_realized": draft["all_generated_prose_realized"],
            "generator": generator}})
    if (result.get("outcome") != "realized" or result.get("kind") != preparation["document_kind"]
        or result.get("format") != format_name or result.get("requested_language") != preparation["language"]
        or result.get("generator") != generator
        or not isinstance(result.get("content"), str) or not result["content"].strip()):
        raise c.CampaignError("Product did not return the exact requested-language realization")
    return result["content"]


def fixed(root: Path, campaign: dict[str, Any], key: str, document_kind: str):
    c = campaign_api()
    bindings = json.loads(bound_bytes(root, root / "realization-bindings.json"))
    matches = [b for b in bindings["documents"] if b["journey_id"] == key and b["document_kind"] == document_kind]
    if len(matches) != 1:
        raise c.CampaignError("required document realization has not been prepared")
    identity = matches[0]["realization_id"]
    preparation_bytes = bound_bytes(root, artifact(root, "plans", identity))
    preparation = json.loads(preparation_bytes)
    draft = json.loads(bound_bytes(root, artifact(root, "recorded", identity)))
    validate_value(preparation, preparation_bytes, draft, allow_bound_runtime=True)
    if (preparation["candidate_head"] != campaign["candidate_head"]
        or preparation["provenance_binding"]["mcp_sha256"] != campaign["document_realization_route"]["mcp_sha256"]
        or preparation["document_kind"] != document_kind
        or preparation["language"] != campaign["document_language"] or preparation["locale"] != campaign["viewer_locale"]):
        raise c.CampaignError("fixed document realization binding mismatch")
    return preparation, draft


def require_batch_ready(root: Path, campaign: dict[str, Any], mapped) -> None:
    c = campaign_api()
    if not required(campaign["document_language"], campaign["viewer_locale"]):
        return
    verify_route(campaign)
    if not (root / "realization-bindings.json").is_file():
        raise c.CampaignError("prepare and fix all cross-locale realizations before collect-batch")
    bindings = json.loads(bound_bytes(root, root / "realization-bindings.json"))
    if (bindings["candidate_head"] != campaign["candidate_head"]
        or bindings["campaign_sha256"] != harness.sha256(c.campaign_file(root))
        or bindings["raw_inputs"] != raw_binding(mapped)):
        raise c.CampaignError("realization preparation does not bind the current campaign and eight raw inputs")
    for key, state in campaign["journeys"].items():
        for kind in c.DOCUMENT_KINDS:
            preparation, _ = fixed(root, campaign, key, kind)
            binding = (state["repository_class"], "A", "start")
            if binding is None or c.observed_project_ids(mapped[binding].capture) != [preparation["project_id"]]:
                raise c.CampaignError("realization Project does not match exact mapped input")


def generate(root: Path, kind: str, work_label: str, project_id: str, document_kind: str,
             format_name: str, destination: Path) -> dict[str, Any]:
    c = campaign_api()
    try:
        campaign = c.load_campaign(root)
        verify_route(campaign)
        key = c.journey_id(kind)
        preparation, draft = fixed(root, campaign, key, document_kind)
        if preparation["project_id"] != project_id:
            raise c.CampaignError("fixed realization Project differs from journey evidence")
        content = consume(Path(campaign["candidate_binary"]), Path(campaign["journeys"][key]["runtime_home"]),
                          preparation, draft, format_name, allow_bound_runtime=True)
        verify_route(campaign)
    except c.CampaignError as error:
        raise c.IntegrityError("realization_binding", error) from error
    c.atomic_write_bytes(destination, content.encode("utf-8"))
    return {"status": "passed", "provenance": draft["provenance"],
        "product_result": {"project_id": project_id, "outcome": "realized"}}
