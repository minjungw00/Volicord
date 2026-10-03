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

SCHEMA_VERSION = 14
# Shared by completion/handoff reporting and replacement qualification.
HIGH_IMPACT_INSUFFICIENCY_GROUPS = ("authority", "context_recovery", "campaign_interaction")
STATES = ["satisfied", "violated", "insufficient_evidence", "not_observed", "not_applicable", "not_reviewed"]
NOT_OBSERVED_OPPORTUNITIES = frozenset({
    "explicit_material_handling_quality", "hidden_material_discovery_quality",
    "learning_fork_value", "learning_alternatives_and_tradeoffs",
    "pre_response_recommendation_anchoring", "post_response_feedback_quality",
    "learning_implementation_fidelity", "routine_detail_omission",
    "proportional_learning_cost",
})
LEARNING_OPPORTUNITIES = frozenset(name for name in NOT_OBSERVED_OPPORTUNITIES
    if name.startswith("learning_") or name in {
        "pre_response_recommendation_anchoring", "post_response_feedback_quality",
        "routine_detail_omission", "proportional_learning_cost"})
RELATIONSHIPS = ["agrees", "clarifies_indeterminate", "probable_false_positive",
                 "probable_false_negative", "cannot_resolve"]
DOCUMENT_KINDS = {"project-architecture-guide", "decision-report", "implementation-plan", "handoff-resume"}
CRITERION_GROUPS = (
    "interaction", "documents", "viewer_snapshot", "viewer_navigation",
    "repository_intelligence", "cli", "live_viewer",
    "context_recovery", "campaign_interaction",
)
SURFACES = {
    "interaction": ["work_capture"], "documents": ["documents", "canonical_bundle"],
    "viewer_snapshot": ["viewer_snapshot"], "repository_intelligence": ["work_capture"],
    "viewer_navigation": ["viewer_navigation_machine"],
    "cli": ["cli_observation"], "live_viewer": ["live_viewer_observation"],
    "context_recovery": ["work_capture", "resume_capture", "canonical_bundle"],
    "authority": ["work_capture"],
    "campaign_interaction": ["task_selection", "interaction_diagnostics", "work_capture", "canonical_bundle"],
}
GROUP_PROMPTS = {
    "campaign_interaction": "Assess whether planned workload intents and actual interactions provide reliable evidence for replacement Question/Learning behavior. Inspect bounded projected conversation turns and explicit omissions, diagnostic facts, source/authority evidence and independent agent semantic review. Correct no-question behavior counts as evidence. Weak selection or sparse evidence means insufficient_evidence; violated requires substantive observed Product behavior failure. No operation-count threshold applies.",
    "interaction": "Judge necessary and omitted Questions against actual material outcomes, user-owned authority and source evidence. Do not require evaluator wording, answers, counts or a manufactured Question. Assess comprehension, repetition and interruption cost; distinguish user judgment from agent recommendation.",
    "documents": "Inspect all four documents: architecture guide, Decision report, implementation plan and handoff/resume. Compare each with current Sources and Decisions; assess practical understanding/handoff value, accurate remaining work and gaps, and actual requested-language prose rather than metadata-only language claims.",
    "viewer_snapshot": "Assess whether Project Understanding explains completed/current/remaining work, next steps, Decision rationale, affected code and component/request/data flow. Distinguish source facts from generated interpretation; inspect evidence-grounded diagram topology and useful readability rather than raw record listings.",
    "viewer_navigation": "Assess candidate-bound Viewer request responsiveness from retained monotonic machine evidence and its declared scope. Snapshot export duration is a bounded request proxy, not browser interaction latency; do not substitute human stopwatch prose or claim unmeasured live navigation timing.",
    "repository_intelligence": "Assess useful navigation and analysis for actual work, honest source snapshot/coverage/freshness/uncertainty, semantic value beyond structure, and language/component boundaries and flows in polyglot work. Unsupported or unavailable capabilities must remain visible.",
    "cli": "Inspect observed help discovery and representative repository-relative tasks without opaque Project IDs. Assess readable outcomes and next actions. Captured invocation is evidence of a surface, not proof that every CLI task was usable.",
    "live_viewer": "Assess actual observed keyboard reachability, visible focus, non-color-only meaning, narrow/zoom presentation and browser input/paint responsiveness in both en and ko. Static markup and snapshot-export timing cannot establish live interaction; use insufficient_evidence if the needed observation is absent.",
    "context_recovery": "Compare work with fresh resume: recover goal, applicable Decisions and rationale, current/completed/remaining state and open questions accurately without repeating answered judgments. A later repair does not make an earlier false completion claim truthful.",
}
WORKLOAD_PROMPTS = {
    "learning_collaborative": "The frozen first turn explicitly requests learning/collaboration. Inspect whether runtime participation recognized the full request and whether fresh resume recovered it. If no Learning interaction occurred, assess source-grounded absence of a meaningful agent-owned learning-worthy fork, insufficient evidence, or failure to honor explicit participation. Do not silently mark Learning criteria not_observed. A satisfied judgment may reflect correctly omitted interaction when sources establish no meaningful fork and participation was honored. Routine details remain non-interrupting; no fixed call count applies.",
    "decision_rich": "If zero Questions occurred, inspect current source/repository authority and actual commitments. Satisfaction requires evidence that no user-owned material Question was required. Do not fail from a zero count or preselect an outcome.",
    "routine_bounded": "Review any unexpected Question for necessity and interruption cost against the clear task and current repository authority.",
    "exploratory_debugging": "Review investigation, uncertainty, research and justified deferment against the evidence, without a desired Question outcome.",
    "cross_stack_integration": "Review source grounding and understanding across relevant language/component boundaries, without a fixed semantic answer.",
}
CRITERION_PROMPTS = {
    "interaction_coverage_adequacy": "satisfied requires executed required intents and enough evidence to assess important interaction behavior, including correct non-question behavior. insufficient_evidence leaves replacement unresolved when evidence is too sparse or unreliable. violated is reserved for substantive Product behavior violations, never weak task selection alone. This required campaign criterion cannot be not_observed or not_applicable.",
    "architecture_components_flow": "Inspect the actual component identities, relationships, direction and request/data flow. Judge topology independently from nearby prose about code behavior.",
    "code_behavior": "Inspect concrete affected code behavior and its code/source basis. Missing or weak architecture topology does not by itself make code behavior absent.",
    "diagram_usefulness": "Inspect the rendered diagram itself, its grounded nodes/edges and whether it materially explains this work. Artifact existence or adjacent prose is not diagram usefulness.",
    "project_purpose_vs_current_work_clarity": "Inspect whether enduring Project purpose and the current Work Item goal are both understandable and visibly distinct.",
    "multiple_work_organization": "Inspect whether separate Work Items retain stable identity, boundaries and per-item state instead of collapsing into one latest-work narrative.",
    "evidence_explanation_comprehensibility": "Inspect whether evidence, freshness, coverage and uncertainty are explained in terms a reader can connect to the visible claim, rather than exposed only as opaque provenance metadata.",
    "ordinary_reading_audit_detail_exposure": "Inspect the primary reading path for hashes, opaque IDs, raw diagnostics and integrity bookkeeping that should be progressively disclosed rather than competing with product meaning.",
    "diagram_structural_readability": "Inspect diagram node labels, edge direction, grouping, crossings and topology at the rendered size. This is separate from whether a diagram exists or is grounded.",
    "information_hierarchy_and_cognitive_burden": "Inspect one bounded reading path: prioritization, grouping, progressive disclosure and the effort required to identify purpose, current work, state and next action. Do not replace these dimensions with a global aesthetic score.",
    "navigation_responsiveness": "Inspect the candidate-bound monotonic Viewer request duration, completion state and measurement scope. Treat missing timing as insufficient evidence and snapshot-export timing as a limited proxy, never as measured browser input latency.",
    "browser_input_and_paint_responsiveness": "Inspect direct live-browser observation of input response and resulting paint in the named locale. Snapshot generation, request completion and server/export duration are not browser input or paint measurements.",
    "usefulness": "Inspect each document's primary user-facing semantic sections for readable project meaning and handoff value. A digest, byte count, bounded-source placeholder, audit appendix or valid artifact hash is not meaningful primary content.",
    "fidelity": "Compare the Decision Report and other affected documents with canonical Decision meaning. Explicitly distinguish user choice, recommended alternative, user rationale, recommendation rationale and alternative-specific consequences.",
}
CRITERION_OBSERVATIONS = {
    "learning_fork_value": ["explicit_participation_scope", "runtime_recognition", "source_grounded_meaningful_fork_or_absence", "routine_noninterruption", "evidence_gap_vs_product_failure"],
    "correct_no_question_behavior": ["repository_and_source_authority", "actual_implementation_commitments", "unresolved_user_owned_outcomes"],
    "unnecessary_interruption": ["question_necessity", "repository_and_source_authority", "proportional_interruption_cost"],
    "interaction_coverage_adequacy": ["planned_workload_intents", "actual_projected_interactions", "machine_diagnostic_facts", "source_and_authority", "independent_agent_semantic_review", "question_and_learning_assessability", "product_failure_vs_evidence_gap"],
    "architecture_components_flow": ["components", "relationships", "flow_direction", "separate_from_code_behavior"],
    "code_behavior": ["affected_code", "concrete_behavior", "source_basis", "separate_from_topology"],
    "diagram_usefulness": ["actual_diagram", "grounded_nodes_edges", "material_explanatory_value"],
    "project_purpose_vs_current_work_clarity": ["project_purpose", "current_work_goal", "visible_conceptual_distinction"],
    "multiple_work_organization": ["stable_work_identities", "work_boundaries", "per_work_state", "current_work_selection"],
    "evidence_explanation_comprehensibility": ["visible_claim", "source_freshness_coverage", "reader_connection", "uncertainty"],
    "ordinary_reading_audit_detail_exposure": ["primary_reading_path", "opaque_identity_and_hash_detail", "raw_diagnostics", "progressive_disclosure"],
    "diagram_structural_readability": ["node_labels", "edge_direction", "grouping_and_topology", "rendered_legibility"],
    "information_hierarchy_and_cognitive_burden": ["purpose_and_current_work_priority", "scan_path", "progressive_disclosure", "bounded_cognitive_burden"],
    "navigation_responsiveness": ["candidate_bound_machine_timing", "request_completion", "measurement_scope", "proxy_limit"],
    "browser_input_and_paint_responsiveness": ["live_browser_input", "resulting_paint", "locale", "observation_limits"],
    "usefulness": ["primary_semantic_content", "readability", "handoff_value", "placeholder_or_audit_only_check"],
    "fidelity": ["user_choice", "recommended_alternative", "user_rationale", "recommendation_rationale", "alternative_specific_consequences"],
}
FIELDS = {"criterion_id", "assessment", "reasoning", "inspected_evidence", "evidence", "uncertainty",
          "criterion_observations",
          "counterevidence", "applicability_reason", "machine_relationships", "authority",
          "human_answer_trace"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def criteria_contract(contract):
    """Load the sole maintained criterion inventory from evaluation.json."""
    criteria = contract.get("common_criteria")
    require(isinstance(criteria, dict) and tuple(criteria) == CRITERION_GROUPS,
        "qualitative criterion groups changed")
    require(all(isinstance(names, list) and names
                and len(names) == len(set(names))
                and all(isinstance(name, str) and name for name in names)
                for names in criteria.values()),
        "qualitative criteria must be distinct nonempty names")
    return copy.deepcopy(criteria)


def rubric(definition):
    contract = definition["qualitative_review_contract"]
    return {"schema_version": SCHEMA_VERSION, "policy_revision": contract["policy_revision"],
        "criteria": criteria_contract(contract), "group_prompts": GROUP_PROMPTS,
        "criterion_prompts": CRITERION_PROMPTS, "workload_prompts": WORKLOAD_PROMPTS,
        "criterion_observations": CRITERION_OBSERVATIONS,
        "required_surfaces": SURFACES,
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
        for group in ("interaction", "context_recovery"):
            if group == "context_recovery" and not sample["resume_pair"]:
                continue
            names = policy["criteria"][group]
            names = list(names)
            if group == "interaction":
                names += sorted(policy["behavior_criteria"])
            for name in names:
                specs.append({"criterion_id": f"{sample_id}/{group}/{name}",
                    "sample_id": sample_id, "group": group, "name": name, "locale": None,
                    "workload_intent": sample.get("workload_intent")})
        for obligation in sample["authority_obligations"]:
            specs.append({"criterion_id": f"{sample_id}/authority/{obligation}",
                "sample_id": sample_id, "group": "authority", "name": obligation, "locale": None})
        specs.append({"criterion_id": f"{sample_id}/authority/coverage", "sample_id": sample_id,
            "group": "authority", "name": "coverage", "locale": None})
    for sample in index["journey_samples"]:
        sample_id = sample["sample_id"]
        for group in ("documents", "viewer_snapshot", "viewer_navigation", "repository_intelligence", "live_viewer"):
            if group == "live_viewer" and sample_id != index["live_viewer_sample"]:
                continue
            for locale in (["en", "ko"] if group == "live_viewer" else [None]):
                for name in policy["criteria"][group]:
                    specs.append({"criterion_id": "/".join(filter(None, [sample_id, group, locale, name])),
                        "sample_id": sample_id, "group": group, "name": name, "locale": locale})
    for sample in index["cli_samples"]:
        for name in policy["criteria"]["cli"]:
            specs.append({"criterion_id": f"{sample['sample_id']}/cli/{name}",
                "sample_id": sample["sample_id"], "group": "cli", "name": name, "locale": None})
    specs.append({"criterion_id": "campaign/campaign_interaction/interaction_coverage_adequacy",
        "sample_id": None, "group": "campaign_interaction", "name": "interaction_coverage_adequacy", "locale": None})
    return specs


def human_only(spec):
    return spec["group"] == "live_viewer" or (
        spec["group"] == "interaction"
        and spec["name"] == "decision_comprehension_when_applicable"
    ) or (spec["group"] == "viewer_snapshot"
        and spec["name"] == "multiple_work_organization"
        and spec["sample_id"] == "journey-volicord")


def completion_obligations(index, policy):
    """Expose mechanical review scope without making any semantic judgment."""
    specs = criterion_specs(index, policy)
    by_group = {}
    for spec in specs:
        by_group.setdefault(spec["group"], []).append(spec["criterion_id"])
    findings = []
    for finding_id, item in sorted(index["machine_findings"].items()):
        finding = item["finding"]
        findings.append({
            "finding_id": finding_id,
            "sample_id": item["sample_id"],
            "status": finding["status"],
            "disposition": finding["disposition"],
            "permitted_review_groups": sorted(machine.review_groups(finding["check"])),
            "qualitative_relationship_required": (
                finding["disposition"] == "qualitative_review_required"
            ),
        })
    authority_by_sample = {}
    for sample in index["samples"]:
        prefix = sample["sample_id"] + "/authority/"
        authority_by_sample[sample["sample_id"]] = {
            "initial_obligation_ids": list(sample["authority_obligations"]),
            "required_criterion_ids": [
                spec["criterion_id"]
                for spec in specs
                if spec["criterion_id"].startswith(prefix)
            ],
            "additional_actual_outcomes": "declare_each_as_additional_outcome_if_found",
        }
    cli_by_class = {
        sample["repository_class"]: [
            spec["criterion_id"]
            for spec in specs
            if spec["group"] == "cli" and spec["sample_id"] == sample["sample_id"]
        ]
        for sample in index["cli_samples"]
    }
    human_ids = sorted(spec["criterion_id"] for spec in specs if human_only(spec))
    return {
        "semantic_judgment_automatic": False,
        "criterion_coverage": {
            "required_count": len(specs),
            "required_ids_by_group": {key: value for key, value in sorted(by_group.items())},
        },
        "machine_findings": findings,
        "authority_outcomes": authority_by_sample,
        "cli_class_coverage": cli_by_class,
        "human_only_criteria": human_ids,
        "targeted_escalation_rules": {
            "machine_relationships": "qualitative_review_required findings need an evidence-backed permitted-group relationship",
            "high_impact_insufficiency_groups": list(HIGH_IMPACT_INSUFFICIENCY_GROUPS),
            "review_conflicts": "human review must name each conflicting review run for the exact criterion",
        },
    }


def completion_progress(preparation, value, specs):
    """Report missing structure and escalation targets; never derive a verdict."""
    assessments = {item["criterion_id"]: item for item in value["assessments"]}
    assessments.update({item["finding"]["criterion_id"]: item["finding"]
                        for item in value["additional_outcomes"]})
    reviewed = {criterion_id for criterion_id, item in assessments.items()
                if item["assessment"] != "not_reviewed"}
    missing = [spec["criterion_id"] for spec in specs if spec["criterion_id"] not in reviewed]
    related = set()
    spec_by_id = {spec["criterion_id"]: spec for spec in specs}
    for criterion_id, item in assessments.items():
        group = spec_by_id.get(criterion_id, {"group": "authority"})["group"]
        for relation in item["machine_relationships"]:
            if (group in machine.review_groups(
                    preparation["index"]["machine_findings"][relation["finding_id"]]["finding"]["check"])
                    and relation["relationship"] in {"clarifies_indeterminate", "probable_false_positive"}):
                related.add(relation["finding_id"])
    required_findings = sorted(
        finding_id for finding_id, item in preparation["index"]["machine_findings"].items()
        if item["finding"]["disposition"] == "qualitative_review_required"
    )
    missing_authority = [criterion_id for criterion_id in missing if "/authority/" in criterion_id]
    missing_cli = {
        sample["repository_class"]: [criterion_id for criterion_id in missing
            if criterion_id.startswith(sample["sample_id"] + "/cli/")]
        for sample in preparation["index"]["cli_samples"]
    }
    human_ids = set(preparation.get("completion_obligations",
        completion_obligations(preparation["index"], preparation["rubric"]))["human_only_criteria"])
    human_remaining = sorted(human_ids if preparation["reviewer"]["kind"] != "human"
                             else human_ids - reviewed)
    high_impact = sorted(criterion_id for criterion_id, item in assessments.items()
        if item["assessment"] == "insufficient_evidence"
        and spec_by_id.get(criterion_id, {"group": "authority"})["group"]
            in HIGH_IMPACT_INSUFFICIENCY_GROUPS)
    return {
        "semantic_correctness_assessed": False,
        "required_criterion_count": len(specs),
        "reviewed_criterion_count": len(specs) - len(missing),
        "missing_criterion_ids": missing,
        "machine_finding_dispositions": [
            {key: item[key] for key in ("finding_id", "status", "disposition")}
            for item in preparation.get("completion_obligations",
                completion_obligations(preparation["index"], preparation["rubric"]))["machine_findings"]
        ],
        "unaddressed_review_required_finding_ids": sorted(set(required_findings) - related),
        "missing_authority_criterion_ids": missing_authority,
        "missing_cli_criterion_ids_by_class": missing_cli,
        "human_only_criterion_ids_requiring_human_review": human_remaining,
        "targeted_escalations": {
            "high_impact_insufficient_criterion_ids": high_impact,
            "declared_conflict_resolution_criterion_ids": sorted(value["resolves_review_runs"]),
        },
    }


def observation(criterion_id):
    return {"criterion_id": criterion_id, "assessment": "not_reviewed", "reasoning": None,
        "inspected_evidence": [], "evidence": [], "uncertainty": None, "criterion_observations": [],
        "counterevidence": None, "applicability_reason": None,
        "machine_relationships": [], "authority": None, "human_answer_trace": None}


def template(preparation, preparation_sha256):
    return {"kind": "dogfood_qualitative_review", "schema_version": SCHEMA_VERSION,
        "preparation_sha256": preparation_sha256, "binding": copy.deepcopy(preparation["binding"]),
        "reviewer": copy.deepcopy(preparation["reviewer"]),
        "observation_scope": {"available_evidence": sorted(preparation["index"]["evidence"]),
            "inspected_evidence": [], "unavailable_surfaces": copy.deepcopy(preparation["unavailable_surfaces"]),
            "limits": ["Review is limited to indexed artifacts; unobserved behavior is not established."]},
        "assessments": [observation(spec["criterion_id"]) for spec in criterion_specs(preparation["index"], preparation["rubric"])],
        "additional_outcomes": [], "resolves_review_runs": {}, "human_controls": {}}


def evidence_applies(entry, sample_id):
    return sample_id is None or sample_id in entry.get("sample_ids", [entry.get("sample_id")])


def validate_references(references, index, inspected, spec, *, allow_empty=False):
    require(isinstance(references, list) and len(references) <= 64 and (allow_empty or references),
            "assessment requires bounded evidence references")
    for ref in references:
        require(isinstance(ref, dict) and set(ref) == {"evidence_id", "locator", "criterion_id", "relevance"}
            and ref["criterion_id"] == spec["criterion_id"] and authority.bounded_text(ref["relevance"]),
            "evidence reference must explain its criterion-specific relevance")
        entry = index["evidence"].get(ref["evidence_id"]) if isinstance(ref["evidence_id"], str) else None
        require(entry is not None and evidence_applies(entry, spec["sample_id"])
            and (entry["surface"] != "cli_observation" or
                 (entry["sample_id"] == spec["sample_id"] and entry.get("repository_class") == spec["sample_id"]))
            and ref["evidence_id"] in inspected, "evidence reference is missing, uninspected or belongs to another sample")
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
    trace = value["human_answer_trace"]
    if preparation["reviewer"]["kind"] == "human":
        require(isinstance(trace, list) and trace and len(trace) <= 128,
            "reviewed human criterion requires its conversational answer trace")
        for turn in trace:
            require(isinstance(turn, dict) and set(turn) == {"prompt", "answer"}
                and authority.bounded_text(turn["prompt"]) and authority.bounded_text(turn["answer"]),
                "human answer trace must preserve bounded prompt/answer text")
    else:
        require(trace is None, "agent review cannot claim a human answer trace")
    require(all(authority.bounded_text(value[f]) for f in ("reasoning", "uncertainty")),
            "reviewed criterion requires bounded reasoning and explicit uncertainty")
    index = preparation["index"]
    criterion_inspected = value["inspected_evidence"]
    require(isinstance(criterion_inspected, list)
        and (criterion_inspected or state == "insufficient_evidence")
        and len(criterion_inspected) == len(set(criterion_inspected))
        and all(isinstance(identity, str) and identity in inspected
                for identity in criterion_inspected),
        "reviewed criterion requires distinct per-criterion inspected evidence")
    for identity in criterion_inspected:
        entry = index["evidence"].get(identity)
        require(entry is not None and evidence_applies(entry, spec["sample_id"]),
            "per-criterion inspected evidence belongs to another sample")
    validate_references(value["evidence"], index, criterion_inspected, spec,
        allow_empty=state in {"insufficient_evidence", "not_observed"})
    counter = value["counterevidence"]
    require(isinstance(counter, dict) and set(counter) == {"state", "reasoning", "evidence"}
        and counter["state"] in {"cited", "none_found", "not_observable"}
        and authority.bounded_text(counter["reasoning"]), "explicit counterevidence or its absence is required")
    validate_references(counter["evidence"], index, criterion_inspected, spec, allow_empty=counter["state"] != "cited")
    require(counter["state"] == "cited" or not counter["evidence"], "absence cannot contain counterevidence")
    require(state != "insufficient_evidence" or not value["evidence"],
        "insufficient evidence records inspection and missing information without fabricated citations")
    require(state != "not_observed" or not value["evidence"], "not_observed cannot fabricate event citations")
    require(state != "not_observed" or (spec["group"] == "interaction"
        and spec["name"] in NOT_OBSERVED_OPPORTUNITIES),
        "not_observed requires an optional naturalistic opportunity")
    if spec["name"] == "interaction_coverage_adequacy":
        require(state not in {"not_observed", "not_applicable"}, "required interaction coverage cannot be unobserved or inapplicable")
    if state == "not_observed" and spec["name"] in LEARNING_OPPORTUNITIES:
        require(spec.get("workload_intent") != "learning_collaborative",
            "explicit Learning intent requires a judgment about recognition, meaningful forks or insufficient evidence")
        learning = preparation["index"]["machine_findings"].get(
            spec["sample_id"] + "/learning_participation")
        if learning is not None:
            require(learning["finding"]["basis"].get("reason")
                == "runtime_learning_participation_not_active",
                "active or uncertain Learning participation cannot be marked not_observed")

    require(state != "not_observed" or spec["group"] not in {"live_viewer", "cli"}, "required direct observations cannot be not_observed")
    require(state != "satisfied" or counter["state"] != "not_observable", "unobservable counterevidence cannot satisfy a criterion")
    observations = value["criterion_observations"]
    required_observations = preparation["rubric"]["criterion_observations"].get(spec["name"], [])
    require(isinstance(observations, list) and len(observations) == len(set(observations))
        and all(isinstance(item, str) for item in observations), "criterion observations must be distinct strings")
    if state in {"satisfied", "violated"}:
        require(observations == required_observations,
            "criterion-specific semantic dimensions were not inspected independently")
    else:
        require(not observations, "incomplete or inapplicable judgment cannot claim completed semantic inspection")
    if state == "not_applicable":
        rule = preparation["rubric"]["not_applicable_rules"].get(spec["name"])
        reason = value["applicability_reason"]
        require(rule is not None and isinstance(reason, dict) and set(reason) == {"code", "reasoning"}
            and reason["code"] == rule and authority.bounded_text(reason["reasoning"]),
            "not_applicable requires a criterion-permitted applicability reason")
        if rule == "single_language_scope":
            sample = next(s for s in [*index["samples"], *index["journey_samples"]]
                if s["sample_id"] == spec["sample_id"])
            require(sample["repository_class"] != "polyglot-medium", "polyglot scope cannot be declared single-language")
    else:
        require(value["applicability_reason"] is None, "applicability reason only belongs to not_applicable")
    if state in {"satisfied", "violated"}:
        surfaces = {index["evidence"][r["evidence_id"]]["surface"] for r in value["evidence"]
            if spec["locale"] is None or index["evidence"][r["evidence_id"]].get("locale") == spec["locale"]}
        required_surfaces = set(SURFACES[spec["group"]])
        if (spec["sample_id"] == "journey-volicord"
                and spec["group"] == "viewer_snapshot"
                and spec["name"] == "multiple_work_organization"
                and preparation["reviewer"]["kind"] == "human"):
            required_surfaces.add("live_viewer_observation")
        require(required_surfaces <= surfaces, "criterion lacks its required observation surface; use insufficient_evidence")
        capture_surfaces = required_surfaces & {"work_capture", "resume_capture"}
        if capture_surfaces:
            # No current alternative surface supplies missing actual conversation.
            # Check every applicable required capture, not only favorable citations.
            applicable = [entry for entry in index["evidence"].values()
                if evidence_applies(entry, spec["sample_id"]) and entry["surface"] in capture_surfaces]
            require(all(entry.get("projection", {}).get("semantic_complete") is True for entry in applicable),
                "required capture is semantically incomplete; use insufficient_evidence")
        if spec["name"] == "interaction_coverage_adequacy":
            require(all(entry.get("projection", {}).get("semantic_complete") is True
                for entry in index["evidence"].values() if entry["surface"] in {"work_capture", "resume_capture"}),
                "interaction coverage requires every complete Work/resume projection; use insufficient_evidence")
            entries = [index["evidence"][identity] for identity in criterion_inspected]
            for sample in index["samples"]:
                required = {"task_selection", "work_capture"} | ({"resume_capture"} if sample["resume_pair"] else set())
                present = {entry["surface"] for entry in entries if evidence_applies(entry, sample["sample_id"])}
                require(required <= present, "interaction coverage requires inspection of every frozen task and Work/resume projection")
        selected_lifecycles = [index['evidence'][r['evidence_id']] for r in value['evidence']
            if index['evidence'][r['evidence_id']]['surface'] == 'explanation_lifecycle']
        require(all(e.get('projection', {}).get('semantic_complete') is True for e in selected_lifecycles),
            'cited explanation lifecycle is semantically incomplete; use insufficient_evidence')
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
                common = ({r["evidence_id"] for r in value["evidence"]}
                    if state in {"satisfied", "violated"} else set(criterion_inspected))
                require(target in common,
                    "authority evidence must also be present in inspected evidence")
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
        require(finding["sample_id"] == spec["sample_id"], "machine finding belongs to another sample")
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
                "indeterminate": "insufficient_evidence", "not_observed": "not_observed", "not_applicable": "not_applicable"}
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
    controls = value["human_controls"]
    require(isinstance(controls, dict) and len(controls) <= 1024,
        "invalid human review controls")
    require(not controls or value["reviewer"]["kind"] == "human",
        "agent review cannot claim conversational human controls")
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
    spec_by_id = {spec["criterion_id"]: spec for spec in specs}
    assessment_by_id = {item["criterion_id"]: item for item in value["assessments"]}
    for criterion_id, control in controls.items():
        require(criterion_id in spec_by_id and isinstance(control, dict)
            and set(control) == {"action", "reference_criterion_id", "reuse_scope", "answer_trace"}
            and control["action"] in {"direct", "skip", "already_covered", "same_as_prior",
                "same_as_other_locale", "cannot_assess", "not_applicable"}
            and isinstance(control["answer_trace"], list) and control["answer_trace"],
            "invalid structured human control")
        reference = control["reference_criterion_id"]
        if control["action"] in {"already_covered", "same_as_prior", "same_as_other_locale"}:
            require(reference in spec_by_id and reference != criterion_id,
                "human reference control requires another prepared criterion")
            current, prior = spec_by_id[criterion_id], spec_by_id[reference]
            require(current["sample_id"] == prior["sample_id"] and current["group"] == prior["group"],
                "human reference control crosses an unrelated sample or group")
            if control["action"] == "same_as_other_locale":
                require(current["name"] == prior["name"] and current["locale"] != prior["locale"],
                    "same-locale reference must bind the matching other-locale criterion")
                require(control["reuse_scope"] == "exact_semantic_judgment",
                    "locale mirror requires explicit exact-semantic reuse")
            else:
                require(control["reuse_scope"] == "observation_evidence_context",
                    "cross-criterion reference may reuse only observation/evidence context")
            require(assessment_by_id[reference]["assessment"] != "not_reviewed",
                "human reference control targets an unresolved criterion")
        else:
            require(reference is None and control["reuse_scope"] is None,
                "non-reference human control cannot name reused meaning")
        assessment = assessment_by_id[criterion_id]
        if control["action"] == "skip":
            require(assessment == observation(criterion_id), "skip must remain not_reviewed")
        elif control["action"] == "cannot_assess":
            require(assessment["assessment"] == "insufficient_evidence",
                "cannot-assess must remain insufficient evidence")
        elif control["action"] == "not_applicable":
            require(assessment["assessment"] == "not_applicable",
                "not-applicable control contradicts assessment")
        elif control["action"] == "direct":
            require(assessment["assessment"] != "not_reviewed", "direct control requires an assessment")
        elif control["action"] == "same_as_other_locale":
            prior_assessment = assessment_by_id[reference]
            semantic_fields = ("assessment", "reasoning", "uncertainty", "criterion_observations",
                "counterevidence", "applicability_reason", "authority")
            require(all(assessment[field] == prior_assessment[field]
                        for field in semantic_fields if field != "counterevidence")
                and assessment["counterevidence"]["state"] == prior_assessment["counterevidence"]["state"]
                and assessment["counterevidence"]["reasoning"] == prior_assessment["counterevidence"]["reasoning"],
                "locale mirror must preserve the exact semantic judgment")
        else:
            prior_inspected = assessment_by_id[reference]["inspected_evidence"]
            require(prior_inspected and set(prior_inspected) <= set(assessment["inspected_evidence"]),
                "cross-criterion reference must preserve the reused observation context")
            require(len(control["answer_trace"]) >= 2
                and assessment["human_answer_trace"] == control["answer_trace"]
                and assessment["reasoning"] == control["answer_trace"][1]["answer"],
                "cross-criterion reuse requires a separately authored current-criterion judgment")
    states = [validate_assessment(a, s, preparation, inspected) for a, s in zip(value["assessments"], specs)]
    additional = value["additional_outcomes"]
    require(isinstance(additional, list) and len(additional) <= 64, "additional material outcomes must be bounded")
    ids = {s["criterion_id"] for s in specs}
    for item in additional:
        require(isinstance(item, dict) and set(item) == {"sample_id", "finding"}, "invalid additional outcome")
        require(item["sample_id"] in {s["sample_id"] for s in preparation["index"]["samples"]},
                "additional outcome belongs to unknown naturalistic sample")
        finding = item["finding"]
        require(isinstance(finding, dict) and isinstance(finding.get("criterion_id"), str)
            and re.fullmatch(re.escape(item["sample_id"]) + r"/authority/additional-[a-z0-9-]{1,64}", finding["criterion_id"])
            and finding["criterion_id"] not in ids, "duplicate or invalid additional outcome identity")
        state = validate_assessment(finding, {"criterion_id": finding["criterion_id"], "sample_id": item["sample_id"],
            "group": "authority", "name": "additional", "locale": None}, preparation, inspected)
        ids.add(finding["criterion_id"])
        states.append(state)
    for cid, runs in resolutions.items():
        require(cid in ids and isinstance(runs, list) and 1 <= len(runs) <= 64
            and len(set(runs)) == len(runs) and all(re.fullmatch(r"[0-9a-f]{32}", str(r))
                and r != value["reviewer"]["run_id"] for r in runs), "invalid resolved criterion/review run identities")
    assessment_state = ("violated" if "violated" in states else "insufficient_evidence" if "insufficient_evidence" in states
        else "not_reviewed" if "not_reviewed" in states
        else "not_observed" if "not_observed" in states else "satisfied")
    return {"state": "valid", "assessment_state": assessment_state,
        "counts": {s: states.count(s) for s in STATES},
        "hard_machine_findings": sorted(k for k, v in preparation["index"]["machine_findings"].items()
            if v["finding"]["disposition"] == "hard_blocking"),
        "completion_preflight": completion_progress(preparation, value, specs),
        "semantic_judgment_verified": False, "qualification_state": "not_run", "phase_9_ready": False}
