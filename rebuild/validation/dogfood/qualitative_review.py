"""One evidence-bound qualitative rubric for agent and human judgment.

Validation establishes structure, identity and evidence membership, never the
semantic truth of prose. No function here grants replacement approval.
"""
from __future__ import annotations

import copy
import re
from typing import Any

import authority_obligations as authority
import identity_provenance
import machine_findings as machine

SCHEMA_VERSION = 2
STATES = ["satisfied", "violated", "insufficient_evidence", "not_applicable", "not_reviewed"]
RELATIONSHIPS = ["agrees", "clarifies_indeterminate", "probable_false_positive",
                 "probable_false_negative", "cannot_resolve"]
DOCUMENT_KINDS = {"project-architecture-guide", "decision-report", "implementation-plan", "handoff-resume"}
# Explicit migration of every former human-review field. Behavior-specific
# criteria/prompts remain owned by evaluation.json; there is only one rubric.
CRITERIA = {
    "interaction": ["question_necessity_and_relevance", "user_ownership", "source_grounding",
        "decision_comprehension_when_applicable", "repeat_behavior", "correct_no_question_behavior"],
    "documents": ["fidelity", "usefulness", "source_grounding", "remaining_work_accuracy",
        "requested_language_body_content"],
    "viewer_snapshot": ["completed_current_remaining_work", "next_step", "decision_rationale",
        "architecture_components_flow", "code_behavior", "fact_versus_interpretation", "diagram_usefulness"],
    "repository_intelligence": ["structural_navigation_usefulness", "semantic_value_over_structural_only",
        "capability_honesty", "polyglot_comprehension_when_applicable"],
    "cli": ["discover_with_cli_help", "status_without_project_id", "analyze_without_project_id",
        "recall_without_project_id", "documents_without_project_id", "export_without_project_id",
        "doctor_without_project_id"],
    "live_viewer": ["keyboard_reachability", "visible_focus", "not_color_only", "narrow_and_zoomed_presentation"],
    "context_recovery": ["goal_decision_rationale_state_and_open_questions"],
}
SURFACES = {
    "interaction": ["work_capture"], "documents": ["documents", "canonical_bundle"],
    "viewer_snapshot": ["viewer_snapshot"], "repository_intelligence": ["work_capture"],
    "cli": ["cli_observation"], "live_viewer": ["live_viewer_observation"],
    "context_recovery": ["work_capture", "resume_capture", "canonical_bundle"],
    "authority": ["work_capture"],
}
GROUP_PROMPTS = {
    "interaction": "Judge necessary and omitted Questions against actual material outcomes, user-owned authority and source evidence. Do not require evaluator wording, answers, counts or a manufactured Question. Assess comprehension, repetition and interruption cost; distinguish user judgment from agent recommendation.",
    "documents": "Inspect all four documents: architecture guide, Decision report, implementation plan and handoff/resume. Compare each with current Sources and Decisions; assess practical understanding/handoff value, accurate remaining work and gaps, and actual requested-language prose rather than metadata-only language claims.",
    "viewer_snapshot": "Assess whether Project Understanding explains completed/current/remaining work, next steps, Decision rationale, affected code and component/request/data flow. Distinguish source facts from generated interpretation; inspect evidence-grounded diagram topology and useful readability rather than raw record listings.",
    "repository_intelligence": "Assess useful navigation and analysis for actual work, honest source snapshot/coverage/freshness/uncertainty, semantic value beyond structure, and language/component boundaries and flows in polyglot work. Unsupported or unavailable capabilities must remain visible.",
    "cli": "Inspect observed help discovery and representative repository-relative tasks without opaque Project IDs. Assess readable outcomes and next actions. Captured invocation is evidence of a surface, not proof that every CLI task was usable.",
    "live_viewer": "Assess actual observed keyboard reachability, visible focus, non-color-only meaning and narrow/zoom presentation in both en and ko. Static markup cannot establish live interaction; use insufficient_evidence if the needed observation is absent.",
    "context_recovery": "Compare work with fresh resume: recover goal, applicable Decisions and rationale, current/completed/remaining state and open questions accurately without repeating answered judgments. A later repair does not make an earlier false completion claim truthful.",
}
FIELDS = {"criterion_id", "assessment", "reasoning", "evidence", "uncertainty",
          "counterevidence", "applicability_reason", "machine_relationships", "authority"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def rubric(definition):
    contract = definition["qualitative_review_contract"]
    return {"schema_version": SCHEMA_VERSION, "policy_revision": contract["policy_revision"],
        "criteria": CRITERIA, "group_prompts": GROUP_PROMPTS, "required_surfaces": SURFACES,
        "behavior_criteria": contract["interaction_behavior_criterion_contracts"],
        "authority_obligation_contract": authority.assessment_contract(),
        "assessment_states": STATES, "machine_relationships": RELATIONSHIPS,
        "not_applicable_rules": {
            "decision_comprehension_when_applicable": "no_user_decision_in_scope",
            "polyglot_comprehension_when_applicable": "single_language_scope"},
        "missing_surface_state": "insufficient_evidence",
        "counterevidence": "Cite counterevidence, or explicitly state none found after inspection.",
        "semantic_judgment_owner": "identified_reviewer", "qualification_authority": False}


def reviewer(kind, run_id, session_id=None):
    require(kind in {"agent", "human"}, "reviewer kind must be agent or human")
    return {"kind": kind, "run_id": run_id, "identity_state": "unverified",
        "identity": {field: ( {"state": "self_reported", "value": session_id}
            if field == "session" and session_id else identity_provenance.unknown())
            for field in ("host", "agent", "model", "session", "person")},
        "relationship": {"evaluated_session": False, "statistical_independence_claimed": False,
            "basis": "Distinct review run; shared model, context, training or preparation may correlate judgments."}}


def validate_reviewer(value, evaluated_sessions):
    require(isinstance(value, dict) and set(value) == {"kind", "run_id", "identity_state", "identity", "relationship"},
            "invalid reviewer identity")
    require(value["kind"] in {"agent", "human"} and re.fullmatch(r"[0-9a-f]{32}", str(value["run_id"])),
            "reviewer kind/run identity is required")
    require(value["identity_state"] == "unverified", "no reviewer identity attestation is available")
    identity = value["identity"]
    require(isinstance(identity, dict) and set(identity) == {"host", "agent", "model", "session", "person"},
            "reviewer identity components are incomplete")
    for claim in identity.values():
        identity_provenance.validate_claim(claim)
    if value["kind"] == "human":
        require(all(identity[f] == identity_provenance.unknown() for f in ("agent", "model", "session")),
                "agent/session/model authorship cannot be represented as human review")
    else:
        require(identity["person"] == identity_provenance.unknown(), "agent review cannot claim human authorship")
        require(identity["session"]["state"] == "self_reported", "agent review requires explicit session correlation")
        require(identity["session"]["value"] == identity["session"]["value"].strip()
            and len(identity["session"]["value"].encode()) <= 512, "agent session identity must be exact and bounded")
        require(identity["session"]["value"] not in evaluated_sessions, "agent reviewer is an evaluated work/resume session")
    relation = value["relationship"]
    require(isinstance(relation, dict) and set(relation) == {"evaluated_session", "statistical_independence_claimed", "basis"}
        and relation["evaluated_session"] is False and relation["statistical_independence_claimed"] is False
        and authority.bounded_text(relation["basis"]), "review independence must be bounded and explicit")


def criterion_specs(index, policy):
    specs = []
    for sample in index["samples"]:
        sample_id = sample["sample_id"]
        for group in CRITERIA:
            names = policy["criteria"][group]
            if group == "live_viewer" and sample_id != index["live_viewer_sample"]:
                continue
            names = list(names)
            if group == "interaction":
                names += [name for name, rule in sorted(policy["behavior_criteria"].items())
                          if sample["behavior_class"] in rule["applies_to"]]
            for locale in (["en", "ko"] if group == "live_viewer" else [None]):
                for name in names:
                    specs.append({"criterion_id": "/".join(filter(None, [sample_id, group, locale, name])),
                        "sample_id": sample_id, "group": group, "name": name, "locale": locale})
        for obligation in sample["authority_obligations"]:
            specs.append({"criterion_id": f"{sample_id}/authority/{obligation}",
                "sample_id": sample_id, "group": "authority", "name": obligation, "locale": None})
        specs.append({"criterion_id": f"{sample_id}/authority/coverage", "sample_id": sample_id,
            "group": "authority", "name": "coverage", "locale": None})
    return specs


def observation(criterion_id):
    return {"criterion_id": criterion_id, "assessment": "not_reviewed", "reasoning": None,
        "evidence": [], "uncertainty": None, "counterevidence": None, "applicability_reason": None,
        "machine_relationships": [], "authority": None}


def template(preparation, preparation_sha256):
    return {"kind": "dogfood_qualitative_review", "schema_version": SCHEMA_VERSION,
        "preparation_sha256": preparation_sha256, "binding": copy.deepcopy(preparation["binding"]),
        "reviewer": copy.deepcopy(preparation["reviewer"]),
        "observation_scope": {"available_evidence": sorted(preparation["index"]["evidence"]),
            "inspected_evidence": [], "unavailable_surfaces": copy.deepcopy(preparation["unavailable_surfaces"]),
            "limits": ["Review is limited to indexed artifacts; unobserved behavior is not established."]},
        "assessments": [observation(spec["criterion_id"]) for spec in criterion_specs(preparation["index"], preparation["rubric"])],
        "additional_outcomes": [], "resolves_review_runs": {}}


def validate_references(references, index, inspected, sample_id, *, allow_empty=False):
    require(isinstance(references, list) and len(references) <= 64 and (allow_empty or references),
            "assessment requires bounded evidence references")
    for ref in references:
        require(isinstance(ref, dict) and set(ref) == {"evidence_id", "locator"}, "invalid evidence reference")
        entry = index["evidence"].get(ref["evidence_id"]) if isinstance(ref["evidence_id"], str) else None
        require(entry is not None and entry["sample_id"] in {None, sample_id}
            and ref["evidence_id"] in inspected, "evidence reference is missing, uninspected or belongs to another cycle")
        locator = ref["locator"]
        require(isinstance(locator, dict) and set(locator) == {"kind", "value"}
            and (locator in entry["locators"] or (
                locator["kind"] == "line" and type(locator["value"]) is int
                and 1 <= locator["value"] <= entry.get("line_count", 0))),
            "evidence locator does not resolve in the bound review index")


def validate_assessment(value, spec, preparation, inspected):
    require(isinstance(value, dict) and set(value) == FIELDS
        and value["criterion_id"] == spec["criterion_id"], "criterion identity/shape changed")
    state = value["assessment"]
    require(state in STATES, "invalid criterion assessment")
    if state == "not_reviewed":
        require(value == observation(spec["criterion_id"]), "not_reviewed cannot conceal findings")
        return state
    require(all(authority.bounded_text(value[f]) for f in ("reasoning", "uncertainty")),
            "reviewed criterion requires bounded reasoning and explicit uncertainty")
    index = preparation["index"]
    validate_references(value["evidence"], index, inspected, spec["sample_id"])
    counter = value["counterevidence"]
    require(isinstance(counter, dict) and set(counter) == {"state", "reasoning", "evidence"}
        and counter["state"] in {"cited", "none_found", "not_observable"}
        and authority.bounded_text(counter["reasoning"]), "explicit counterevidence or its absence is required")
    validate_references(counter["evidence"], index, inspected, spec["sample_id"], allow_empty=counter["state"] != "cited")
    require(counter["state"] == "cited" or not counter["evidence"], "absence cannot contain counterevidence")
    require(state != "satisfied" or counter["state"] != "not_observable", "unobservable counterevidence cannot satisfy a criterion")
    if state == "not_applicable":
        rule = preparation["rubric"]["not_applicable_rules"].get(spec["name"])
        reason = value["applicability_reason"]
        require(rule is not None and isinstance(reason, dict) and set(reason) == {"code", "reasoning"}
            and reason["code"] == rule and authority.bounded_text(reason["reasoning"]),
            "not_applicable requires a criterion-permitted applicability reason")
        if rule == "single_language_scope":
            sample = next(s for s in index["samples"] if s["sample_id"] == spec["sample_id"])
            require(sample["repository_class"] != "polyglot-medium", "polyglot scope cannot be declared single-language")
    else:
        require(value["applicability_reason"] is None, "applicability reason only belongs to not_applicable")
    if state in {"satisfied", "violated"}:
        surfaces = {index["evidence"][r["evidence_id"]]["surface"] for r in value["evidence"]
            if spec["locale"] is None or index["evidence"][r["evidence_id"]].get("locale") == spec["locale"]}
        require(set(SURFACES[spec["group"]]) <= surfaces, "criterion lacks its required observation surface; use insufficient_evidence")
        if state == "satisfied" and spec["group"] == "documents":
            document_kinds = {index["evidence"][r["evidence_id"]].get("document_kind") for r in value["evidence"]}
            require(DOCUMENT_KINDS <= document_kinds, "document satisfaction must inspect all four required documents")
    detail = value["authority"]
    if spec["group"] == "authority" and spec["name"] != "coverage":
        if state in {"satisfied", "violated"}:
            require(detail is not None, "material authority requires outcome/commitment/resolution/chronology assessment")
        if detail is not None:
            sample = next(s for s in index["samples"] if s["sample_id"] == spec["sample_id"])
            # The established authority engine retains its stricter actual-work,
            # canonical evidence, unrelated-authority and prospective protections.
            local_index = {alias: index["evidence"][target] for alias, target in sample["authority_evidence"].items()}
            result = authority.assess(detail, local_index)
            for reference in detail["evidence"]:
                target = sample["authority_evidence"][reference["evidence_id"]]
                require(target in {r["evidence_id"] for r in value["evidence"]}, "authority evidence must also have resolvable common references")
            expected = {"passed": "satisfied", "failed": "violated", "insufficient_evidence": "insufficient_evidence"}[result]
            require(state == expected, "authority disposition contradicts criterion assessment")
    else:
        require(detail is None, "authority details belong only to material outcome criteria")
    relations = value["machine_relationships"]
    require(isinstance(relations, list) and len(relations) <= 64, "machine relationships must be bounded")
    seen = set()
    for relation in relations:
        require(isinstance(relation, dict) and set(relation) == {"finding_id", "relationship", "reasoning"}
            and relation["relationship"] in RELATIONSHIPS and authority.bounded_text(relation["reasoning"]),
            "invalid machine finding relationship")
        finding_id = relation["finding_id"]
        require(isinstance(finding_id, str) and finding_id not in seen and finding_id in index["machine_findings"],
                "missing or duplicate machine finding")
        seen.add(finding_id)
        finding = index["machine_findings"][finding_id]
        require(finding["sample_id"] == spec["sample_id"], "machine finding belongs to another cycle")
        machine.validate_finding(finding["finding"])
        f = finding["finding"]
        # Disagreement is retained as disagreement; it never changes disposition.
        if relation["relationship"] == "clarifies_indeterminate":
            require(f["disposition"] == "qualitative_review_required" and state in {"satisfied", "violated"},
                    "hard machine violations cannot be clarified or overridden")
        if relation["relationship"] == "probable_false_positive":
            require(f["status"] in {"confirmed_violation", "indeterminate", "not_observed"}, "not a possible false positive")
        if relation["relationship"] == "probable_false_negative":
            require(f["status"] == "confirmed_pass" and state == "violated", "false negative requires an observed violation")
        if relation["relationship"] == "agrees":
            expected = {"confirmed_pass": "satisfied", "confirmed_violation": "violated",
                "indeterminate": "insufficient_evidence", "not_observed": "insufficient_evidence", "not_applicable": "not_applicable"}
            require(state == expected[f["status"]], "machine/reviewer disagreement cannot be labeled agreement")
    return state


def validate_value(preparation, preparation_sha256, value):
    try:
        return _validate_value(preparation, preparation_sha256, value)
    except (KeyError, TypeError, AttributeError, IndexError) as error:
        raise ValueError("malformed qualitative review field or nested assessment") from error


def _validate_value(preparation, preparation_sha256, value):
    expected = template(preparation, preparation_sha256)
    require(isinstance(value, dict) and set(value) == set(expected), "invalid qualitative review shape")
    for field in ("kind", "schema_version", "preparation_sha256", "binding", "reviewer"):
        require(value[field] == expected[field], f"immutable review {field} changed")
    validate_reviewer(value["reviewer"], preparation["evaluated_sessions"])
    resolutions = value["resolves_review_runs"]
    require(isinstance(resolutions, dict) and len(resolutions) <= 512, "invalid review conflict resolutions")
    require(not resolutions or value["reviewer"]["kind"] == "human", "only human review may resolve escalated review conflicts")
    criterion_ids = {s["criterion_id"] for s in criterion_specs(preparation["index"], preparation["rubric"])}
    for cid, runs in resolutions.items():
        require(cid in criterion_ids and isinstance(runs, list) and 1 <= len(runs) <= 64
            and len(set(runs)) == len(runs) and all(re.fullmatch(r"[0-9a-f]{32}", str(r))
                and r != value["reviewer"]["run_id"] for r in runs), "invalid resolved criterion/review run identities")
    scope = value["observation_scope"]
    require(isinstance(scope, dict) and set(scope) == set(expected["observation_scope"]), "invalid observation scope")
    for field in ("available_evidence", "unavailable_surfaces"):
        require(scope[field] == expected["observation_scope"][field], "review observation availability changed")
    inspected = scope["inspected_evidence"]
    require(isinstance(inspected, list) and all(isinstance(i, str) for i in inspected)
        and len(inspected) == len(set(inspected)) and set(inspected) <= set(scope["available_evidence"]), "unknown/duplicate inspected evidence")
    require(isinstance(scope["limits"], list) and 1 <= len(scope["limits"]) <= 64
        and all(authority.bounded_text(limit) for limit in scope["limits"]), "explicit bounded observation limits are required")
    specs = criterion_specs(preparation["index"], preparation["rubric"])
    require(specs and len({s["criterion_id"] for s in specs}) == len(specs), "review requires distinct, nonempty criterion identities")
    require(isinstance(value["assessments"], list) and len(value["assessments"]) == len(specs), "required criteria cannot be omitted")
    states = [validate_assessment(a, s, preparation, inspected) for a, s in zip(value["assessments"], specs)]
    additional = value["additional_outcomes"]
    require(isinstance(additional, list) and len(additional) <= 64, "additional material outcomes must be bounded")
    ids = {s["criterion_id"] for s in specs}
    for item in additional:
        require(isinstance(item, dict) and set(item) == {"sample_id", "finding"}, "invalid additional outcome")
        require(item["sample_id"] in {s["sample_id"] for s in specs}, "additional outcome belongs to unknown sample")
        finding = item["finding"]
        require(isinstance(finding, dict) and isinstance(finding.get("criterion_id"), str)
            and re.fullmatch(re.escape(item["sample_id"]) + r"/authority/additional-[a-z0-9-]{1,64}", finding["criterion_id"])
            and finding["criterion_id"] not in ids, "duplicate or invalid additional outcome identity")
        ids.add(finding["criterion_id"])
        states.append(validate_assessment(finding, {"criterion_id": finding["criterion_id"], "sample_id": item["sample_id"],
            "group": "authority", "name": "additional", "locale": None}, preparation, inspected))
    assessment_state = ("violated" if "violated" in states else "insufficient_evidence" if "insufficient_evidence" in states
        else "not_reviewed" if "not_reviewed" in states else "satisfied")
    return {"state": "valid", "assessment_state": assessment_state,
        "counts": {s: states.count(s) for s in STATES},
        "hard_machine_findings": sorted(k for k, v in preparation["index"]["machine_findings"].items()
            if v["finding"]["disposition"] == "hard_blocking"),
        "semantic_judgment_verified": False, "qualification_state": "not_run", "phase_9_ready": False}
