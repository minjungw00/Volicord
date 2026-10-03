"""Conversational capture for human-owned Dogfood observations and judgments.

The adapter never chooses a verdict or writes semantic prose. It preserves each
answer, derives schema identities and evidence locators, and leaves the generated
draft mutable for human inspection before the existing immutable record step.
"""
from __future__ import annotations

import json
import os
import copy
from pathlib import Path
import secrets
import tempfile

import authority_obligations as authority
import qualitative_review as review
import viewer_observation


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


def _ask_multiline(prompt, input_fn, output_fn, trace):
    """Accept a natural bounded block; interactive input ends with a line containing END."""
    output_fn(prompt + " (multiple paragraphs are allowed; finish with END on its own line)")
    first = input_fn()
    if input_fn is not input:
        answer = first.strip()
    else:
        lines = []
        current = first
        while current != "END":
            lines.append(current)
            current = input_fn()
        answer = "\n".join(lines).strip()
    review.require(authority.bounded_text(answer), "human answer must be non-empty and bounded")
    trace.append({"prompt": prompt, "answer": answer})
    return answer


def _split_observation_and_limits(answer):
    """Split explicit human-authored sections without interpreting their meaning."""
    marker = "\nLIMITS:\n"
    review.require(answer.startswith("OBSERVATION:\n") and marker in answer,
        "grouped observation must contain OBSERVATION: and LIMITS: sections")
    observation, limits = answer[len("OBSERVATION:\n"):].split(marker, 1)
    review.require(authority.bounded_text(observation.strip()) and authority.bounded_text(limits.strip()),
        "grouped observation and limits must both be non-empty and bounded")
    return observation.strip(), limits.strip()


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


def observation_confirmation(locale, contexts):
    identities = ", ".join(c["context"]["render_id"] for c in contexts)
    return f"Did you personally inspect the live {locale} displays identified by render IDs {identities}, including their actual view/subject/basis? Browser captures alone cannot answer yes."


def _live_observation_requests():
    return [
        {
            "surface": "live_viewer_observation",
            "locale": locale,
            "prompt": (
                f"For locale {locale}, describe what you personally observed for keyboard/focus/color/zoom "
                "(including narrow screens, non-color cues and actual browser 200% zoom), and live browser input/paint responsiveness. "
                "Use Overview, Work selection, Decisions, related Code and Source disclosures; record observed gaps and limits using OBSERVATION: and "
                "LIMITS: sections. Type SAME AS ENGLISH for an exact locale reference."
            ),
        }
        for locale in ("en", "ko")
    ]


def capture_viewer_observations(campaign_root, output, *, input_fn=input, output_fn=print,
                                run_id=None, context_paths=()):
    """Capture required direct live Viewer observations."""
    ops, campaign = _ops(), _campaign()
    root, output = campaign_root.resolve(), output.absolute()
    manifest = campaign.load_evidence_set(root)
    import evidence_purpose
    evidence_purpose.require_measured(manifest)
    evidence_hash = ops.digest(ops.bounded_read(root / "evidence-set.json"))
    contexts = viewer_observation.load_contexts(context_paths, manifest)
    observer = review.reviewer("human", run_id or secrets.token_hex(16))
    observations, answer_trace = [], []
    for request in _live_observation_requests():
        surface, locale = request["surface"], request["locale"]
        trace = []
        confirmation = observation_confirmation(locale, contexts[locale])
        review.require(_yes_no(confirmation, input_fn, output_fn, trace),
            "direct human observation is unavailable for this displayed context")
        answer = _ask_multiline(request["prompt"], input_fn, output_fn, trace)
        if surface == "live_viewer_observation" and locale == "ko" \
                and answer.casefold() == "same as english":
            observations.append({"sample_id": "journey-volicord", "surface": surface,
                "locale": locale, "contexts": contexts[locale], "personally_observed": True,
                "control": {"action": "same_as_locale", "reference_locale": "en"},
                "response": None})
        else:
            observation, limits = _split_observation_and_limits(answer)
            observations.append({"sample_id": "journey-volicord", "surface": surface,
                "locale": locale, "contexts": contexts[locale], "personally_observed": True,
                "control": {"action": "direct", "reference_locale": None},
                "response": {"observation": observation, "limits": limits}})
        answer_trace.append({"surface": surface, "locale": locale, "turns": trace})
    # Recheck the original browser receipt/screenshot after the human interaction.
    review.require(viewer_observation.load_contexts(context_paths, manifest) == contexts,
        "display evidence changed during human capture")
    value = {"kind": "dogfood_human_observations",
        "schema_version": 4,
        "candidate_head": manifest["candidate_head"], "evidence_set_sha256": evidence_hash,
        "observer": observer, "observations": observations}
    data = ops.encoded(value)
    ops.require_review_artifact_safe(data, "human observations contain sensitive payload")
    receipt = {"kind": "dogfood_human_observation_receipt", "schema_version": 4,
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
    """Load a conversational observation directory or direct current-schema JSON."""
    ops = _ops()
    if path.is_file():
        return path
    data = ops.bounded_read(path / "observations.json")
    receipt = json.loads(ops.bounded_read(path / "receipt.json"))
    value = json.loads(data)
    requests = {(item["surface"], item["locale"]): item for item in _live_observation_requests()}
    expected_trace = []
    for item in value.get("observations", []):
        surface, locale = item.get("surface"), item.get("locale")
        request = requests.get((surface, locale), {"prompt": ""})
        expected_trace.append({"surface": surface, "locale": locale, "turns": [
            {"prompt": observation_confirmation(locale, item.get("contexts", [])) + " (1=yes, 2=no)", "answer": "1"},
            {"prompt": request["prompt"],
             "answer": ("SAME AS ENGLISH" if item.get("control", {}).get("action") == "same_as_locale"
                else "OBSERVATION:\n" + item.get("response", {}).get("observation", "")
                + "\nLIMITS:\n" + item.get("response", {}).get("limits", ""))},
        ]})
    expected = {"kind": "dogfood_human_observation_receipt", "schema_version": 4,
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
        if not review.evidence_applies(entry, spec["sample_id"]) or (spec["sample_id"] is None and entry["surface"] == "cli_observation"):
            continue
        rank = 0 if entry["surface"] in required else 1
        entries.append((rank, identity, entry))
    return [(identity, entry) for _rank, identity, entry in sorted(entries)]


def _locator_for_phrase(root, entry, phrase):
    ops = _ops()
    if not phrase:
        return entry["locators"][0] if entry["locators"] else {"kind": "line", "value": 1}
    data = ops.bounded_read(ops.safe_path(root, entry["path"]))
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


def _control_reference(answer, position, specs, draft):
    normalized = " ".join(answer.casefold().split())
    action = None
    if normalized in {"same as previous", "same-as-previous"}:
        action = "same_as_prior"
    elif normalized in {"already covered", "already-covered"}:
        action = "already_covered"
    elif normalized in {"same as english", "same-as-english"}:
        action = "same_as_other_locale"
    if action is None:
        return None
    current = specs[position]
    candidates = []
    for prior_position in range(position):
        prior = specs[prior_position]
        finding = draft["assessments"][prior_position]
        if finding["assessment"] == "not_reviewed" or prior["sample_id"] != current["sample_id"] \
                or prior["group"] != current["group"] or current["group"] == "authority":
            continue
        if action == "same_as_other_locale":
            if prior["name"] == current["name"] and prior["locale"] == "en" and current["locale"] == "ko":
                candidates.append(prior_position)
        elif finding["inspected_evidence"]:
            candidates.append(prior_position)
    review.require(candidates, "the requested control has no compatible prior reviewed criterion")
    return action, candidates[-1]


def _human_control(action, reference_criterion_id, reuse_scope, trace):
    return {"action": action, "reference_criterion_id": reference_criterion_id,
        "reuse_scope": reuse_scope, "answer_trace": trace}


def _reused_references(prior, spec, input_fn, output_fn, trace):
    references = []
    for reference in prior["evidence"]:
        relevance = _ask(
            f"Explain why reused evidence {reference['evidence_id']} is relevant to "
            f"the distinct criterion {spec['criterion_id']}.",
            input_fn, output_fn, trace)
        references.append({**copy.deepcopy(reference), "criterion_id": spec["criterion_id"],
            "relevance": relevance})
    return references


def _locale_identity(preparation, identity, locale):
    entry = preparation["index"]["evidence"][identity]
    if entry.get("locale") is None:
        return identity
    matches = [candidate for candidate, other in preparation["index"]["evidence"].items()
        if other["sample_id"] == entry["sample_id"] and other["surface"] == entry["surface"]
        and other.get("locale") == locale]
    review.require(len(matches) == 1,
        "exact locale reuse requires one matching locale observation")
    return matches[0]


def _locale_mirror(preparation, prior, spec, trace):
    mirrored = copy.deepcopy(prior)
    prior_id = mirrored["criterion_id"]
    mirrored["criterion_id"] = spec["criterion_id"]
    mirrored["inspected_evidence"] = sorted({_locale_identity(
        preparation, identity, spec["locale"]) for identity in mirrored["inspected_evidence"]})
    for reference in [*mirrored["evidence"], *mirrored["counterevidence"]["evidence"]]:
        identity = _locale_identity(preparation, reference["evidence_id"], spec["locale"])
        entry = preparation["index"]["evidence"][identity]
        reference.update(evidence_id=identity, criterion_id=spec["criterion_id"],
            locator=entry["locators"][0],
            relevance=f"Exact-semantic locale reference to {prior_id} for {spec['locale']}.")
    mirrored["machine_relationships"] = []
    mirrored["human_answer_trace"] = trace
    return mirrored


def _store_draft(root, draft_path, draft):
    ops = _ops()
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
    return data


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
    if spec.get("workload_intent") in preparation["rubric"]["workload_prompts"]:
        output_fn(preparation["rubric"]["workload_prompts"][spec["workload_intent"]])
    if spec["name"] in preparation["rubric"]["criterion_prompts"]:
        output_fn(preparation["rubric"]["criterion_prompts"][spec["name"]])
    observation = _ask_multiline(
        "Describe your observation and reasoning. You may instead enter SKIP, ALREADY COVERED, "
        "SAME AS PREVIOUS, SAME AS ENGLISH, NOT SURE, CANNOT ASSESS, or NOT APPLICABLE.",
        input_fn, output_fn, trace)
    normalized = " ".join(observation.casefold().split())
    control_reference = _control_reference(observation, position, specs, draft)
    if normalized == "skip":
        draft["human_controls"][spec["criterion_id"]] = _human_control(
            "skip", None, None, trace)
        result = review.validate_value(preparation, sha, draft)
        data = _store_draft(root, draft_path, draft)
        return {"state": "draft_updated", "criterion_number": position + 1,
            "criterion_id": spec["criterion_id"], "assessment": "not_reviewed",
            "control": "skip", "draft_sha256": ops.digest(data),
            "review_run_id": preparation["reviewer"]["run_id"],
            "candidate_head": preparation["binding"]["candidate_head"],
            "evidence_set_sha256": preparation["binding"]["evidence_set"]["sha256"],
            "review_result": result}
    if control_reference is not None:
        action, prior_position = control_reference
        prior = copy.deepcopy(draft["assessments"][prior_position])
        prior_id = prior["criterion_id"]
        if action == "same_as_other_locale":
            mirrored = _locale_mirror(preparation, prior, spec, trace)
            draft["assessments"][position] = mirrored
            draft["human_controls"][spec["criterion_id"]] = _human_control(
                action, prior_id, "exact_semantic_judgment", trace)
            inspected = set(draft["observation_scope"]["inspected_evidence"])
            inspected.update(mirrored["inspected_evidence"])
            draft["observation_scope"]["inspected_evidence"] = sorted(inspected)
            result = review.validate_value(preparation, sha, draft)
            data = _store_draft(root, draft_path, draft)
            return {"state": "draft_updated", "criterion_number": position + 1,
                "criterion_id": spec["criterion_id"], "assessment": mirrored["assessment"],
                "control": action, "reference_criterion_id": prior_id,
                "reuse_scope": "exact_semantic_judgment",
                "draft_sha256": ops.digest(data), "review_run_id": preparation["reviewer"]["run_id"],
                "candidate_head": preparation["binding"]["candidate_head"],
                "evidence_set_sha256": preparation["binding"]["evidence_set"]["sha256"],
                "review_result": result}
        observation = _ask_multiline(
            "The prior observation context will be retained. Describe the distinct current criterion's "
            "observation and reasoning; its verdict and semantic meaning are not inherited.",
            input_fn, output_fn, trace)
        reuse_action, reuse_prior_id, reused_context = action, prior_id, prior
    else:
        reuse_action, reuse_prior_id, reused_context = None, None, None
    if normalized in {"not sure", "cannot assess", "cannot-assess"}:
        reasoning = _ask_multiline("Explain what you inspected and what evidence is missing.",
            input_fn, output_fn, trace)
        finding = {**review.observation(spec["criterion_id"]),
            "assessment": "insufficient_evidence", "reasoning": reasoning,
            "uncertainty": reasoning,
            "counterevidence": {"state": "not_observable", "reasoning": reasoning, "evidence": []},
            "human_answer_trace": trace}
        draft["assessments"][position] = finding
        draft["human_controls"][spec["criterion_id"]] = _human_control(
            "cannot_assess", None, None, trace)
        result = review.validate_value(preparation, sha, draft)
        data = _store_draft(root, draft_path, draft)
        return {"state": "draft_updated", "criterion_number": position + 1,
            "criterion_id": spec["criterion_id"], "assessment": "insufficient_evidence",
            "control": "cannot_assess", "draft_sha256": ops.digest(data),
            "review_run_id": preparation["reviewer"]["run_id"],
            "candidate_head": preparation["binding"]["candidate_head"],
            "evidence_set_sha256": preparation["binding"]["evidence_set"]["sha256"],
            "review_result": result}
    forced_not_applicable = normalized in {"not applicable", "not-applicable"}
    state = ("not_applicable" if forced_not_applicable else
        _choice("What is your judgment?", review.STATES[:-1], input_fn, output_fn, trace))
    reasoning = (observation if not forced_not_applicable else
        _ask_multiline("Explain why the maintained applicability rule applies.", input_fn, output_fn, trace))
    references = ([] if state == "insufficient_evidence" else
        _reused_references(reused_context, spec, input_fn, output_fn, trace)
        if reused_context is not None and reused_context["evidence"] else
        _references(root, preparation, spec, input_fn, output_fn, trace, purpose="the judgment"))
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
        applicability = {"code": rule, "reasoning": reasoning}
    observations = []
    if state in {"satisfied", "violated"}:
        dimensions = preparation["rubric"]["criterion_observations"].get(spec["name"], [])
        if dimensions:
            answer = _ask("Confirm the independently inspected dimensions by entering their comma-separated names: "
                + ", ".join(dimensions), input_fn, output_fn, trace)
            supplied = [item.strip() for item in answer.split(",")]
            review.require(supplied == dimensions,
                "satisfied/violated requires every unresolved criterion-specific dimension")
            observations.extend(dimensions)
    detail = None
    if spec["group"] == "authority" and spec["name"] != "coverage" \
            and state in {"satisfied", "violated"}:
        detail = _authority_detail(preparation, spec, references, input_fn, output_fn, trace)
    finding = {"criterion_id": spec["criterion_id"], "assessment": state,
        "reasoning": reasoning,
        "inspected_evidence": sorted({
            *([] if reused_context is None else reused_context["inspected_evidence"]),
            *(reference["evidence_id"] for reference in [*references, *counter_refs]),
        }),
        "evidence": references, "uncertainty": uncertainty,
        "criterion_observations": observations,
        "counterevidence": {"state": counter_state, "reasoning": counter_reasoning,
            "evidence": counter_refs}, "applicability_reason": applicability,
        "machine_relationships": [], "authority": detail, "human_answer_trace": trace}
    draft["assessments"][position] = finding
    control_action = (reuse_action or ("not_applicable" if forced_not_applicable else "direct"))
    draft["human_controls"][spec["criterion_id"]] = _human_control(
        control_action, reuse_prior_id,
        "observation_evidence_context" if reuse_action else None, trace)
    inspected = set(draft["observation_scope"]["inspected_evidence"])
    inspected.update(finding["inspected_evidence"])
    draft["observation_scope"]["inspected_evidence"] = sorted(inspected)
    for run_id, other_state in _resolution_runs(resolve_review_roots, spec["criterion_id"]):
        if _yes_no(f"Does this judgment explicitly resolve recorded review {run_id} ({other_state}) for this criterion?",
                   input_fn, output_fn, trace):
            draft["resolves_review_runs"].setdefault(spec["criterion_id"], []).append(run_id)
    finding["human_answer_trace"] = trace
    result = review.validate_value(preparation, sha, draft)
    data = _store_draft(root, draft_path, draft)
    return {"state": "draft_updated", "criterion_number": position + 1,
        "criterion_id": spec["criterion_id"], "assessment": state,
        **({"control": reuse_action, "reference_criterion_id": reuse_prior_id,
            "reuse_scope": "observation_evidence_context"} if reuse_action else {}),
        "draft_sha256": ops.digest(data), "review_run_id": preparation["reviewer"]["run_id"],
        "candidate_head": preparation["binding"]["candidate_head"],
        "evidence_set_sha256": preparation["binding"]["evidence_set"]["sha256"],
        "review_result": result}
