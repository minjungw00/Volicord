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
    if campaign.get("document_realization_route") != route(Path(campaign["candidate_binary"])):
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
    return [{"cycle": list(key), "session_id": value.capture.session_id,
             "sha256": value.capture.source_sha256} for key, value in sorted(mapped.items())]


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


def prepare(root: Path, raw_paths: list[Path]) -> dict[str, Any]:
    c = campaign_api()
    campaign = c.load_campaign_for_mutation(root)
    c.verify_inventory(root)
    if campaign.get("terminal_outcome") is not None or any(s.get("state") != "sealed" for s in campaign["cycles"].values()):
        raise c.CampaignError("realization preparation requires an untouched sealed campaign")
    if not required(campaign["document_language"], campaign["viewer_locale"]):
        return {"state": "fixed_locale", "qualification_state": "not_run"}
    verify_route(campaign)
    binding_path = root / "realization-bindings.json"
    if binding_path.exists():
        raise c.CampaignError("realization plans are already prepared; edit only unfixed drafts")
    mapped = c.map_batch_rollouts(root, raw_paths)
    files, drafts, bindings, index = {}, {}, [], []
    binary = Path(campaign["candidate_binary"])
    for key, state in sorted(campaign["cycles"].items()):
        kind, cycle = next((kind, cycle) for kind in c.CLASSES for cycle in c.cycle_numbers(kind)
                           if c.cycle_key(kind, cycle) == key)
        work_ids = c.observed_project_ids(mapped[(kind, cycle, "work")].capture)
        resume_ids = c.observed_project_ids(mapped[(kind, cycle, "resume")].capture)
        if len(work_ids) != 1 or resume_ids != work_ids:
            raise c.CampaignError("raw work/resume captures do not establish one exact cycle Project")
        for document_kind in c.DOCUMENT_KINDS:
            identity = secrets.token_hex(16)
            preparation = {"kind": "active_host_document_preparation", "schema_version": 1,
                "realization_id": identity, "candidate_head": campaign["candidate_head"],
                "project_id": work_ids[0], "document_kind": document_kind,
                "language": campaign["document_language"], "locale": campaign["viewer_locale"]}
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
            draft = {"preparation_sha256": hashlib.sha256(data).hexdigest(),
                "requested_language": preparation["language"], "all_generated_prose_realized": False,
                "realization": {"plan_fingerprint": plan["plan_fingerprint"], "title": None,
                    "generator": {"generator": None, "agent": None, "model": None},
                    "sections": [{"key": s["key"], "title": None,
                        "claims": [{"identity": claim["identity"], "text": None} for claim in s["claims"]]}
                        for s in plan["sections"]]}}
            drafts[artifact(root, "drafts", identity)] = c.json_bytes(draft)
            bindings.append({"cycle_key": key, "realization_id": identity, "document_kind": document_kind})
            index.append({"realization_id": identity, "preparation": c.relative(root, plan_path),
                          "draft": c.relative(root, artifact(root, "drafts", identity))})
    if any(harness.sha256(value.source) != value.capture.source_sha256 for value in mapped.values()):
        raise c.CampaignError("raw inputs changed during realization preparation")
    files[binding_path] = c.json_bytes({"candidate_head": campaign["candidate_head"],
        "campaign_sha256": harness.sha256(c.campaign_file(root)), "raw_inputs": raw_binding(mapped), "documents": bindings})
    files[root / "realizer/index.json"] = c.json_bytes({"documents": sorted(index, key=lambda x: x["realization_id"])})
    publish(root, files, drafts=drafts)
    return {"state": "realization_required", "index": "realizer/index.json", "qualification_state": "not_run"}


def validate_value(preparation: dict[str, Any], preparation_bytes: bytes, value: Any) -> None:
    c = campaign_api()
    def require(condition, message):
        if not condition:
            raise c.CampaignError(message)
    def text(value):
        return isinstance(value, str) and bool(value.strip()) and len(value.encode("utf-8")) <= 4096
    require(isinstance(value, dict) and set(value) == {"preparation_sha256", "requested_language",
        "all_generated_prose_realized", "realization"}, "invalid realization draft shape")
    require(value["preparation_sha256"] == hashlib.sha256(preparation_bytes).hexdigest(), "wrong preparation hash")
    require(value["requested_language"] == preparation["language"] and value["all_generated_prose_realized"] is True,
            "active host must review all generated prose in the exact requested language")
    plan, realized = preparation["plan"], value["realization"]
    require(isinstance(realized, dict) and set(realized) == {"plan_fingerprint", "title", "generator", "sections"},
            "invalid NarrativeRealization shape")
    require(realized["plan_fingerprint"] == plan["plan_fingerprint"], "wrong plan fingerprint")
    require(text(realized["title"]) and realized["title"] != plan["source_title"], "requested-language title is missing or unrealized")
    generator = realized["generator"]
    require(isinstance(generator, dict) and set(generator) == {"generator", "agent", "model"}
        and all(text(v) for v in generator.values()), "active-host generator identities are required")
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


def validate(root: Path, identity: str, draft_path: Path) -> dict[str, Any]:
    """Read only realizer-visible preparation, input and inventory membership."""
    c = campaign_api()
    if c.relative(root, draft_path) in c.load_inventory(root)["artifacts"]:
        raise c.CampaignError("inventory-bound artifact cannot be a mutable realization draft")
    data = bound_bytes(root, artifact(root, "plans", identity))
    if draft_path.stat().st_size > 2 * 1024 * 1024:
        raise c.CampaignError("realization draft exceeds the private artifact bound")
    draft = c.read_json(draft_path)
    validate_value(json.loads(data), data, draft)
    return {"state": "valid", "realization_id": identity, "qualification_state": "not_run"}


def record(root: Path, identity: str, draft_path: Path) -> dict[str, Any]:
    c = campaign_api()
    campaign = c.load_campaign_for_mutation(root)
    c.verify_inventory(root)
    if campaign.get("terminal_outcome") is not None or any(s.get("state") != "sealed" for s in campaign["cycles"].values()):
        raise c.CampaignError("realization recording requires an untouched sealed campaign")
    verify_route(campaign)
    validate(root, identity, draft_path)
    data = draft_path.read_bytes()
    preparation_bytes = bound_bytes(root, artifact(root, "plans", identity))
    preparation = json.loads(preparation_bytes)
    validate_value(preparation, preparation_bytes, json.loads(data))
    bindings = json.loads(bound_bytes(root, root / "realization-bindings.json"))
    matches = [b for b in bindings["documents"] if b["realization_id"] == identity]
    if len(matches) != 1 or preparation["candidate_head"] != campaign["candidate_head"]:
        raise c.CampaignError("realization preparation candidate/identity mismatch")
    state = campaign["cycles"][matches[0]["cycle_key"]]
    for format_name, _ in c.DOCUMENT_FORMATS:
        consume(Path(campaign["candidate_binary"]), Path(state["runtime_home"]), preparation,
                json.loads(data), format_name)
    destination = artifact(root, "recorded", identity)
    publish(root, {destination: data})
    return {"state": "fixed", "realization_id": identity,
            "sha256": hashlib.sha256(data).hexdigest(), "qualification_state": "not_run"}


def consume(binary: Path, runtime: Path, preparation: dict[str, Any], draft: dict[str, Any], format_name: str) -> str:
    c = campaign_api()
    if current_plan(binary, runtime, preparation, format_name) != preparation["plan"]:
        raise c.CampaignError("current Product plan changed after preparation")
    result = preview(binary, runtime, {**request(preparation, format_name), "realization": draft["realization"]})
    if (result.get("outcome") != "realized" or result.get("kind") != preparation["document_kind"]
        or result.get("format") != format_name or result.get("requested_language") != preparation["language"]
        or result.get("generator") != draft["realization"]["generator"]
        or not isinstance(result.get("content"), str) or not result["content"].strip()):
        raise c.CampaignError("Product did not return the exact requested-language realization")
    return result["content"]


def fixed(root: Path, campaign: dict[str, Any], key: str, document_kind: str):
    c = campaign_api()
    bindings = json.loads(bound_bytes(root, root / "realization-bindings.json"))
    matches = [b for b in bindings["documents"] if b["cycle_key"] == key and b["document_kind"] == document_kind]
    if len(matches) != 1:
        raise c.CampaignError("required document realization has not been prepared")
    identity = matches[0]["realization_id"]
    preparation_bytes = bound_bytes(root, artifact(root, "plans", identity))
    preparation = json.loads(preparation_bytes)
    draft = json.loads(bound_bytes(root, artifact(root, "recorded", identity)))
    validate_value(preparation, preparation_bytes, draft)
    if (preparation["candidate_head"] != campaign["candidate_head"]
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
        raise c.CampaignError("realization preparation does not bind the current campaign and sixteen raw inputs")
    for key in campaign["cycles"]:
        for kind in c.DOCUMENT_KINDS:
            preparation, _ = fixed(root, campaign, key, kind)
            binding = next((slot for slot in mapped if slot[2] == "work" and c.cycle_key(*slot[:2]) == key), None)
            if binding is None or c.observed_project_ids(mapped[binding].capture) != [preparation["project_id"]]:
                raise c.CampaignError("realization Project does not match exact mapped input")


def generate(root: Path, kind: str, cycle: int, project_id: str, document_kind: str,
             format_name: str, destination: Path) -> dict[str, Any]:
    c = campaign_api()
    campaign = c.load_campaign(root)
    verify_route(campaign)
    key = c.cycle_key(kind, cycle)
    preparation, draft = fixed(root, campaign, key, document_kind)
    if preparation["project_id"] != project_id:
        raise c.CampaignError("fixed realization Project differs from cycle evidence")
    content = consume(Path(campaign["candidate_binary"]), Path(campaign["cycles"][key]["runtime_home"]),
                      preparation, draft, format_name)
    c.atomic_write_bytes(destination, content.encode("utf-8"))
    return {"status": "passed", "product_result": {"project_id": project_id, "outcome": "realized"}}
