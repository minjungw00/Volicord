"""Conversational capture for human-owned Dogfood observations and judgments.

The adapter never chooses a verdict or writes semantic prose. It preserves each
answer, derives schema identities and evidence locators, and leaves the generated
draft mutable for human inspection before the existing immutable record step.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import secrets
import tempfile

import authority_obligations as authority
import qualitative_review as review


def _ops():
    import review_operations
    return review_operations


def _campaign():
    import campaign
    return campaign


def _ask(prompt, input_fn, output_fn, trace):
    output_fn(prompt)
    answer = input_fn().strip()
    review.require(authority.bounded_text(answer), "human answer must be non-empty and bounded")
    trace.append({"prompt": prompt, "answer": answer})
    return answer


def _choice(prompt, choices, input_fn, output_fn, trace):
    labels = ", ".join(f"{number + 1}={label}" for number, label in enumerate(choices))
    answer = _ask(f"{prompt} ({labels})", input_fn, output_fn, trace)
    try:
        number = int(answer)
    except ValueError as error:
        raise ValueError("answer must select one displayed number") from error
    review.require(1 <= number <= len(choices), "answer selected an unavailable choice")
    return choices[number - 1]


def _yes_no(prompt, input_fn, output_fn, trace):
    return _choice(prompt, ["yes", "no"], input_fn, output_fn, trace) == "yes"


def _write_create_only(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    review.require(not path.exists() and not path.is_symlink(), "human observation destination already exists")
    descriptor, temporary = tempfile.mkstemp(prefix=".human-review-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(temporary, path)
        path.chmod(0o400)
    finally:
        Path(temporary).unlink(missing_ok=True)


def capture_viewer_observations(campaign_root, output, *, input_fn=input, output_fn=print,
                                run_id=None):
    """Capture the two current live Viewer observations without schema authoring."""
    ops, campaign = _ops(), _campaign()
    root, output = campaign_root.resolve(), output.absolute()
    manifest = campaign.load_evidence_set(root)
    evidence_hash = ops.digest(ops.bounded_read(root / "evidence-set.json"))
    observer = review.reviewer("human", run_id or secrets.token_hex(16))
    observations, answer_trace = [], []
    for locale in ("en", "ko"):
        trace = []
        observation = _ask(
            f"Describe only what you personally observed in the live Viewer for locale {locale}.",
            input_fn, output_fn, trace)
        limits = _ask(
            f"State the limits of that {locale} observation (what you did not inspect or could not establish).",
            input_fn, output_fn, trace)
        observations.append({"sample_id": "volicord-1", "locale": locale,
            "observation": observation, "limits": limits})
        answer_trace.append({"locale": locale, "turns": trace})
    value = {"kind": "dogfood_human_observations",
        "candidate_head": manifest["candidate_head"], "evidence_set_sha256": evidence_hash,
        "observer": observer, "observations": observations}
    data = ops.encoded(value)
    ops.require_review_artifact_safe(data, "human observations contain sensitive payload")
    receipt = {"kind": "dogfood_human_observation_receipt", "schema_version": 1,
        "candidate_head": manifest["candidate_head"], "evidence_set_sha256": evidence_hash,
        "observer_run_id": observer["run_id"], "observations_sha256": ops.digest(data),
        "answer_trace": answer_trace}
    receipt_data = ops.encoded(receipt)
    ops.require_review_artifact_safe(receipt_data, "human observation receipt contains sensitive payload")
    ops.publish_directory(output, {
        "observations.json": data,
        "receipt.json": receipt_data,
    })
    return {"state": "captured", "observation_root": str(output),
        "observer_run_id": observer["run_id"], "candidate_head": manifest["candidate_head"],
        "evidence_set_sha256": evidence_hash, "observations_sha256": ops.digest(data)}


def load_viewer_observations(path):
    """Load either the conversational receipt directory or the older JSON input."""
    ops = _ops()
    if path.is_file():
        return path
    data = ops.bounded_read(path / "observations.json")
    receipt = json.loads(ops.bounded_read(path / "receipt.json"))
    value = json.loads(data)
    expected_trace = []
    for item in value.get("observations", []):
        locale = item.get("locale")
        expected_trace.append({"locale": locale, "turns": [
            {"prompt": f"Describe only what you personally observed in the live Viewer for locale {locale}.",
             "answer": item.get("observation")},
            {"prompt": f"State the limits of that {locale} observation (what you did not inspect or could not establish).",
             "answer": item.get("limits")},
        ]})
    expected = {"kind": "dogfood_human_observation_receipt", "schema_version": 1,
        "candidate_head": value.get("candidate_head"),
        "evidence_set_sha256": value.get("evidence_set_sha256"),
        "observer_run_id": value.get("observer", {}).get("run_id"),
        "observations_sha256": ops.digest(data), "answer_trace": expected_trace}
    review.require(receipt == expected,
        "human observation receipt or answer trace mismatch")
    return path / "observations.json"


def _eligible_evidence(preparation, spec):
    required = preparation["rubric"]["required_surfaces"].get(spec["group"], [])
    entries = []
    for identity, entry in preparation["index"]["evidence"].items():
        if entry["sample_id"] not in {None, spec["sample_id"]}:
            continue
        rank = 0 if entry["surface"] in required else 1
        entries.append((rank, identity, entry))
    return [(identity, entry) for _rank, identity, entry in sorted(entries)]


def _locator_for_phrase(root, entry, phrase):
    ops = _ops()
    if not phrase:
        return entry["locators"][0] if entry["locators"] else {"kind": "line", "value": 1}
    data = ops.bounded_read(ops.safe_path(root, entry["path"]),
        ops.MAX_RAW_BYTES if entry["path"].startswith("private-rollouts/") else ops.MAX_FILE_BYTES)
    text = data.decode("utf-8", errors="replace")
    matches = [number for number, line in enumerate(text.splitlines(), 1) if phrase in line]
    review.require(len(matches) == 1,
        "the quoted evidence phrase must occur on exactly one line; provide a more specific phrase")
    return {"kind": "line", "value": matches[0]}


def _references(root, preparation, spec, input_fn, output_fn, trace, *, purpose):
    eligible = _eligible_evidence(preparation, spec)
    review.require(eligible, "no evidence is available for this criterion")
    output_fn(f"Available evidence for {spec['criterion_id']}:")
    for number, (identity, entry) in enumerate(eligible, 1):
        output_fn(f"  {number}. {entry['surface']} — {entry['path']}")
    answer = _ask(
        f"Select the evidence item numbers you actually inspected for {purpose}, separated by commas.",
        input_fn, output_fn, trace)
    try:
        numbers = [int(part.strip()) for part in answer.split(",")]
    except ValueError as error:
        raise ValueError("evidence selection must use displayed numbers") from error
    review.require(numbers and len(numbers) == len(set(numbers))
        and all(1 <= number <= len(eligible) for number in numbers),
        "evidence selection is empty, duplicated or unavailable")
    references = []
    for number in numbers:
        identity, entry = eligible[number - 1]
        output_fn(f"Binding evidence {number}: {entry['path']}")
        phrase_answer = _ask(
            "Paste a short exact phrase from the relevant line, or type START to cite the first inspectable location.",
            input_fn, output_fn, trace)
        phrase = "" if phrase_answer == "START" else phrase_answer
        locator = _locator_for_phrase(root, entry, phrase)
        relevance = _ask(
            f"Explain in your own words why this location is relevant to {spec['criterion_id']}.",
            input_fn, output_fn, trace)
        references.append({"evidence_id": identity, "locator": locator,
            "criterion_id": spec["criterion_id"], "relevance": relevance})
    return references


def _authority_detail(preparation, spec, references, input_fn, output_fn, trace):
    path = _choice("How was this material outcome resolved?",
        list(authority.RESOLUTION_AUTHORITIES), input_fn, output_fn, trace)
    detail = {
        "material_outcome": _ask("Describe the independently material outcome.", input_fn, output_fn, trace),
        "observable_implementation_commitment": _ask(
            "Describe the observable production commitment, or the absence of one.", input_fn, output_fn, trace),
        "commitment_state": _choice("What is the commitment state?",
            authority.assessment_contract()["commitment_states"], input_fn, output_fn, trace),
        "resolution_path": path,
        "authority_kind": authority.RESOLUTION_AUTHORITIES[path],
        "authority_basis": _ask("Explain the exact authority basis in your own words.", input_fn, output_fn, trace),
        "authority_relation_to_outcome": _choice("How does that authority relate to this outcome?",
            authority.assessment_contract()["authority_relations"], input_fn, output_fn, trace),
        "chronology": _choice("When did the authority exist relative to the commitment?",
            authority.assessment_contract()["chronology"], input_fn, output_fn, trace),
        "evidence": [],
    }
    sample = next(sample for sample in preparation["index"]["samples"]
                  if sample["sample_id"] == spec["sample_id"])
    targets = {reference["evidence_id"]: reference for reference in references}
    for alias, identity in sample["authority_evidence"].items():
        if identity in targets:
            locator = targets[identity]["locator"]
            detail["evidence"].append({"evidence_id": alias,
                "locator": f"{locator['kind']}:{locator['value']}"})
    return detail


def _resolution_runs(review_roots, criterion_id):
    ops = _ops()
    runs = []
    for root in review_roots:
        preparation, sha, _package = ops.load_package(root)
        recorded = ops.recorded_files(root, preparation, sha)
        review.require(recorded, "a resolved review root must contain an immutable recorded review")
        value = json.loads(recorded["recorded/review.json"])
        finding = next((item for item in value["assessments"]
                        if item["criterion_id"] == criterion_id), None)
        if finding is not None and finding["assessment"] != "not_reviewed":
            runs.append((preparation["reviewer"]["run_id"], finding["assessment"]))
    return runs


def converse_one(review_root, *, criterion_number=None, resolve_review_roots=(),
                 input_fn=input, output_fn=print):
    """Capture exactly one human criterion and update only the mutable draft."""
    ops = _ops()
    root = review_root.resolve()
    preparation, sha, package = ops.load_package(root)
    review.require(preparation["reviewer"]["kind"] == "human",
        "conversational human review requires a human review preparation")
    draft_path = root / "draft.json"
    draft = json.loads(ops.draft_bytes(root, draft_path, package))
    specs = review.criterion_specs(preparation["index"], preparation["rubric"])
    if criterion_number is None:
        positions = [number for number, item in enumerate(draft["assessments"])
                     if item["assessment"] == "not_reviewed"]
        review.require(positions, "all prepared criteria already have a judgment")
        position = positions[0]
    else:
        review.require(1 <= criterion_number <= len(specs), "criterion number is unavailable")
        position = criterion_number - 1
        review.require(draft["assessments"][position]["assessment"] == "not_reviewed",
            "criterion already has a judgment; prepare a new run to correct recorded meaning")
    spec = specs[position]
    trace = []
    output_fn(f"Criterion {position + 1}/{len(specs)}: {spec['criterion_id']}")
    output_fn(preparation["rubric"]["group_prompts"].get(spec["group"], "Inspect this bounded criterion."))
    if spec["name"] in preparation["rubric"]["criterion_prompts"]:
        output_fn(preparation["rubric"]["criterion_prompts"][spec["name"]])
    state = _choice("What is your judgment?", review.STATES[:-1], input_fn, output_fn, trace)
    reasoning = _ask("State the reasoning for that judgment in your own words.", input_fn, output_fn, trace)
    references = _references(root, preparation, spec, input_fn, output_fn, trace, purpose="the judgment")
    uncertainty = _ask("State the remaining uncertainty or explicitly say that none remains.", input_fn, output_fn, trace)
    counter_state = _choice("What counterevidence did you find?",
        ["cited", "none_found", "not_observable"], input_fn, output_fn, trace)
    counter_reasoning = _ask("Explain that counterevidence status in your own words.", input_fn, output_fn, trace)
    counter_refs = (_references(root, preparation, spec, input_fn, output_fn, trace,
                    purpose="counterevidence") if counter_state == "cited" else [])
    applicability = None
    if state == "not_applicable":
        rule = preparation["rubric"]["not_applicable_rules"].get(spec["name"])
        review.require(rule is not None, "this criterion cannot be marked not applicable")
        applicability = {"code": rule, "reasoning": _ask(
            "Explain why the maintained applicability rule applies.", input_fn, output_fn, trace)}
    observations = []
    if state in {"satisfied", "violated"}:
        for dimension in preparation["rubric"]["criterion_observations"].get(spec["name"], []):
            review.require(_yes_no(f"Did you independently inspect '{dimension}'?",
                input_fn, output_fn, trace),
                "satisfied/violated requires every criterion-specific dimension; choose a nonterminal evidence state instead")
            observations.append(dimension)
    detail = None
    if spec["group"] == "authority" and spec["name"] != "coverage" \
            and state in {"satisfied", "violated"}:
        detail = _authority_detail(preparation, spec, references, input_fn, output_fn, trace)
    finding = {"criterion_id": spec["criterion_id"], "assessment": state,
        "reasoning": reasoning,
        "inspected_evidence": sorted({reference["evidence_id"] for reference in [*references, *counter_refs]}),
        "evidence": references, "uncertainty": uncertainty,
        "criterion_observations": observations,
        "counterevidence": {"state": counter_state, "reasoning": counter_reasoning,
            "evidence": counter_refs}, "applicability_reason": applicability,
        "machine_relationships": [], "authority": detail, "human_answer_trace": trace}
    draft["assessments"][position] = finding
    inspected = set(draft["observation_scope"]["inspected_evidence"])
    inspected.update(reference["evidence_id"] for reference in [*references, *counter_refs])
    draft["observation_scope"]["inspected_evidence"] = sorted(inspected)
    for run_id, other_state in _resolution_runs(resolve_review_roots, spec["criterion_id"]):
        if _yes_no(f"Does this judgment explicitly resolve recorded review {run_id} ({other_state}) for this criterion?",
                   input_fn, output_fn, trace):
            draft["resolves_review_runs"].setdefault(spec["criterion_id"], []).append(run_id)
    finding["human_answer_trace"] = trace
    result = review.validate_value(preparation, sha, draft)
    data = ops.encoded(draft)
    ops.require_review_artifact_safe(data, "conversational review draft contains sensitive payload")
    descriptor, temporary = tempfile.mkstemp(prefix=".draft-", dir=root)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, draft_path)
        draft_path.chmod(0o600)
    finally:
        Path(temporary).unlink(missing_ok=True)
    return {"state": "draft_updated", "criterion_number": position + 1,
        "criterion_id": spec["criterion_id"], "assessment": state,
        "draft_sha256": ops.digest(data), "review_run_id": preparation["reviewer"]["run_id"],
        "candidate_head": preparation["binding"]["candidate_head"],
        "evidence_set_sha256": preparation["binding"]["evidence_set"]["sha256"],
        "review_result": result}
