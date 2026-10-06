#!/usr/bin/env python3
"""Sanitized reviewer judgments; never semantic grading of naturalistic work."""
import copy
import json
from pathlib import Path
import re
import unittest

import qualitative_review as q
import machine_findings as m
from authority_obligations_self_test import assessment, interaction_assessment, interaction_fixture


def preparation(kind="agent"):
    definition = json.loads(Path(__file__).with_name("evaluation.json").read_text())
    policy = q.rubric(definition)
    sample = {"sample_id": "journey-volicord-work-a", "journey_id": "journey-volicord",
        "repository_class": "volicord", "work": "A", "resume_pair": True,
        "workload_intent": "decision_rich",
        "materiality_obligations": ["explicit_user_owned_decision"],
        "authority_obligations": ["all-material-outcomes"],
        "authority_evidence": {"work_capture": "work_capture", "canonical_bundle": "canonical_bundle"}}
    journey = {"sample_id": "journey-volicord", "journey_id": "journey-volicord",
        "repository_class": "volicord", "represented_work_sample_ids": [sample["sample_id"]]}
    index = {"samples": [sample], "journey_samples": [journey],
        "cli_samples": [{"sample_id": repository_class, "repository_class": repository_class}
                        for repository_class in ("volicord", "small-python", "polyglot-medium")],
        "live_viewer_sample": "journey-volicord", "machine_findings": {}, "evidence": {}}
    for surface in {s for surfaces in q.SURFACES.values() for s in surfaces}:
        for locale in (["en", "ko"] if surface == "live_viewer_observation" else [None]):
            name = surface + ("-" + locale if locale else "")
            journey_surface = surface in {"documents", "viewer_snapshot", "viewer_navigation_machine",
                "live_viewer_observation"}
            sample_id = journey["sample_id"] if journey_surface else sample["sample_id"]
            sample_ids = [sample["sample_id"], journey["sample_id"]] if surface in {
                "work_capture", "canonical_bundle"} else [sample_id]
            index["evidence"][name] = {"sample_id": sample_id, "sample_ids": sample_ids,
                "surface": surface,
                "locale": locale, "sha256": "a" * 64, "path": name,
                "locators": [{"kind": "json_pointer", "value": "/fact"}]}
    for entry in index["evidence"].values():
        if entry["surface"] in {"work_capture", "resume_capture"}:
            entry["projection"] = {"semantic_complete": True, "semantic_omission_count": 0}
    index["evidence"].pop("cli_observation")
    for repository_class in ("volicord", "small-python", "polyglot-medium"):
        index["evidence"][repository_class + "-cli"] = {"sample_id": repository_class,
            "sample_ids": [repository_class],
            "repository_class": repository_class,
            "surface": "cli_observation", "locale": None, "sha256": "a" * 64,
            "path": repository_class + "-cli", "locators": [{"kind": "json_pointer", "value": "/fact"}]}
    for document_kind in sorted(q.DOCUMENT_KINDS):
        index["evidence"]["document-" + document_kind] = {**index["evidence"]["documents"], "document_kind": document_kind}
    return {"binding": {"state": "verified", "source": "immutable_campaign_evidence",
                "candidate_head": "a" * 40, "evidence_set": {"sha256": "b" * 64}, "rubric_sha256": m.digest(policy)},
        "reviewer": q.reviewer(kind, "c" * 32, "independent-review-session" if kind == "agent" else None),
        "evaluated_sessions": ["work-session", "resume-session"], "rubric": policy, "index": index,
        "unavailable_surfaces": []}


def completed(p):
    value = q.template(p, "d" * 64)
    value["observation_scope"]["inspected_evidence"] = sorted(p["index"]["evidence"])
    for observation in value["assessments"]:
        fill(observation, p)
    return value


def fill(value, p, state="satisfied"):
    scope = None if value["criterion_id"].startswith("campaign/") else value["criterion_id"].split("/", 1)[0]
    criterion = value["criterion_id"].rsplit("/", 1)[-1]
    value.update(assessment=state, reasoning=f"Synthetic reviewer independently inspected {criterion} in the cited artifact.",
        uncertainty="Fixture-only judgment; no actual Product qualification.",
        criterion_observations=(list(p["rubric"]["criterion_observations"].get(criterion, []))
                                if state in {"satisfied", "violated"} else []),
        counterevidence={"state": "none_found", "reasoning": "No contrary evidence in the inspected synthetic case.", "evidence": []},
        human_answer_trace=([{"prompt": "What is your bounded judgment?",
            "answer": f"The human reviewer selected {state} for {criterion}."}]
            if p["reviewer"]["kind"] == "human" else None),
        evidence=[{"evidence_id": name, "locator": entry["locators"][0],
                   "criterion_id": value["criterion_id"],
                   "relevance": f"This cited location was inspected specifically for {criterion}."}
            for name, entry in sorted(p["index"]["evidence"].items())
            if q.evidence_applies(entry, scope) and not (scope is None and entry["surface"] == "cli_observation")])
    value["inspected_evidence"] = sorted({reference["evidence_id"] for reference in value["evidence"]})
    if state == "insufficient_evidence":
        value["evidence"] = []
    if "/authority/" in value["criterion_id"] and not value["criterion_id"].endswith("/coverage"):
        value["authority"] = assessment()
    return value


def compatibility_review_result():
    p = preparation()
    value = completed(p)
    finding = next(a for a in value["assessments"] if a["criterion_id"] == "journey-volicord-work-a/interaction/source_grounding")
    finding.update(assessment="violated", reasoning="Work claimed completion after focused tests, but fresh resume proved an existing supported input was rejected and repaired it. Later repair does not establish the original completion claim.")
    result = q.validate_value(p, "d" * 64, value)
    if result["assessment_state"] != "violated" or result["phase_9_ready"]:
        raise AssertionError("later compatibility repair erased original reviewed failure")
    return "failed"


class ContractTests(unittest.TestCase):
    def test_high_impact_handoff_uses_groups_and_includes_additional_authority(self):
        p = preparation()
        value = completed(p)
        specs = q.criterion_specs(p["index"], p["rubric"])
        expected = set()
        for spec, item in zip(specs, value["assessments"]):
            if spec["group"] in {"authority", "context_recovery", "campaign_interaction"}:
                fill(item, p, "insufficient_evidence")
                item["authority"] = None
                expected.add(spec["criterion_id"])
            elif spec["group"] == "cli":
                fill(item, p, "insufficient_evidence")
        additional_id = "journey-volicord-work-a/authority/additional-durability"
        extra = fill(q.observation(additional_id), p, "insufficient_evidence")
        extra["authority"] = None
        value["additional_outcomes"] = [{"sample_id": "journey-volicord-work-a", "finding": extra}]
        expected.add(additional_id)
        progress = q.validate_value(p, "d" * 64, value)["completion_preflight"]
        self.assertIn("campaign/campaign_interaction/interaction_coverage_adequacy", expected)
        self.assertEqual(progress["targeted_escalations"]["high_impact_insufficient_criterion_ids"],
            sorted(expected))

        # Presentation IDs cannot add or remove structured high-impact authority.
        for number, (spec, item) in enumerate(zip(specs, value["assessments"])):
            old_id = spec["criterion_id"]
            new_id = (f"opaque-{number}" if spec["group"] in q.HIGH_IMPACT_INSUFFICIENCY_GROUPS
                else "misleading/authority/" + old_id)
            spec["criterion_id"] = item["criterion_id"] = new_id
            if old_id in expected:
                expected.remove(old_id)
                expected.add(new_id)
        self.assertEqual(q.completion_progress(p, value, specs)["targeted_escalations"]
            ["high_impact_insufficient_criterion_ids"], sorted(expected))

    def test_required_interaction_coverage_cannot_be_unobserved(self):
        p = preparation()
        value = completed(p)
        criterion = next(a for a in value["assessments"] if a["criterion_id"].endswith("/interaction_coverage_adequacy"))
        fill(criterion, p, "insufficient_evidence")
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "insufficient_evidence")
        for state in ("not_observed", "not_applicable"):
            fill(criterion, p, state)
            criterion["evidence"] = []
            with self.assertRaises(ValueError):
                q.validate_value(p, "d" * 64, value)

    def test_explicit_learning_without_runtime_evidence_requires_judgment(self):
        for runtime in (None, "inactive", "active"):
            p = preparation()
            p["index"]["samples"][0]["workload_intent"] = "learning_collaborative"
            if runtime is not None:
                p["index"]["machine_findings"]["journey-volicord-work-a/learning_participation"] = {
                    "sample_id": "journey-volicord-work-a", "finding": m.finding("learning_participation",
                        "not_observed" if runtime == "inactive" else "indeterminate",
                        {"reason": "runtime_learning_participation_not_active" if runtime == "inactive" else "runtime_learning_active_requires_post_hoc_review"})}
            value = completed(p)
            for name in q.LEARNING_OPPORTUNITIES:
                criterion = next(a for a in value["assessments"] if a["criterion_id"].endswith("/" + name))
                fill(criterion, p, "not_observed")
                criterion["evidence"] = []
                with self.assertRaisesRegex(ValueError, "explicit Learning intent"):
                    q.validate_value(p, "d" * 64, value)
                fill(criterion, p, "insufficient_evidence")
                q.validate_value(p, "d" * 64, value)
                # A source-grounded no-meaningful-fork judgment remains reviewer-owned.
                fill(criterion, p, "satisfied")
            q.validate_value(p, "d" * 64, value)

    def test_decision_rich_zero_questions_and_routine_questions_are_reviewable(self):
        for intent in ("decision_rich", "routine_bounded"):
            p = preparation()
            p["index"]["samples"][0]["workload_intent"] = intent
            value = completed(p)
            q.validate_value(p, "d" * 64, value)
            self.assertIn("source", p["rubric"]["workload_prompts"][intent].lower()
                if intent == "decision_rich" else p["rubric"]["group_prompts"]["interaction"].lower())

    def test_evaluation_contract_is_the_only_criterion_inventory(self):
        definition = json.loads(Path(__file__).with_name("evaluation.json").read_text())
        contract = definition["qualitative_review_contract"]
        self.assertNotIn("live_viewer_criteria", contract)
        self.assertEqual(q.rubric(definition)["criteria"], contract["common_criteria"])
        self.assertIn("browser_input_and_paint_responsiveness",
            contract["common_criteria"]["live_viewer"])

    def test_viewer_usability_dimensions_and_machine_timing_remain_independent(self):
        p = preparation()
        expected = {
            "project_purpose_vs_current_work_clarity",
            "multiple_work_organization",
            "evidence_explanation_comprehensibility",
            "ordinary_reading_audit_detail_exposure",
            "diagram_structural_readability",
            "information_hierarchy_and_cognitive_burden",
        }
        self.assertTrue(expected <= set(p["rubric"]["criteria"]["viewer_snapshot"]))
        self.assertEqual(p["rubric"]["criteria"]["viewer_navigation"],
            ["navigation_responsiveness"])
        self.assertEqual(p["rubric"]["required_surfaces"]["viewer_navigation"],
            ["viewer_navigation_machine"])
        self.assertIn("browser input latency",
            p["rubric"]["criterion_prompts"]["navigation_responsiveness"])
        self.assertIn("browser_input_and_paint_responsiveness",
            p["rubric"]["criteria"]["live_viewer"])
        self.assertNotIn("long_lived_project", p["rubric"]["criteria"])
        self.assertFalse(q.human_only({"sample_id": "journey-volicord",
            "group": "viewer_snapshot", "name": "multiple_work_organization"}))
        self.assertNotEqual(
            p["rubric"]["criterion_observations"]["diagram_usefulness"],
            p["rubric"]["criterion_observations"]["diagram_structural_readability"])
        self.assertEqual(len(p["rubric"]["criterion_observations"][
            "information_hierarchy_and_cognitive_burden"]), 4)

    def test_live_complaint_and_historical_fidelity_have_separate_authority(self):
        p = preparation("human")
        value = q.template(p, "d" * 64)
        specs = q.criterion_specs(p["index"], p["rubric"])
        for name in ("multiple_work_comprehension", "displayed_decision_comprehension"):
            n = next(i for i, spec in enumerate(specs)
                if spec["name"] == name and spec["locale"] == "en")
            finding = fill(value["assessments"][n], p, "violated")
            finding["evidence"] = [r for r in finding["evidence"]
                if r["evidence_id"] == "live_viewer_observation-en"]
            finding["inspected_evidence"] = ["live_viewer_observation-en"]
            finding["reasoning"] = "I cannot distinguish the displayed items; user rationale was not recorded."
            finding["human_answer_trace"] = [{"prompt": "What did you experience?", "answer": finding["reasoning"]}]
        value["observation_scope"]["inspected_evidence"] = ["live_viewer_observation-en"]
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "violated")
        for name in ("multiple_work_organization", "decision_comprehension_when_applicable"):
            n = next(i for i, spec in enumerate(specs) if spec["name"] == name)
            finding = fill(value["assessments"][n], p)
            finding["evidence"] = [r for r in finding["evidence"]
                if r["evidence_id"] == "live_viewer_observation-en"]
            finding["inspected_evidence"] = ["live_viewer_observation-en"]
            with self.assertRaises(ValueError):
                q.validate_value(p, "d" * 64, value)
            value["assessments"][n] = q.observation(specs[n]["criterion_id"])
        # A formal live answer never writes or resolves a historical assertion.
        self.assertTrue(all(a["assessment"] == "not_reviewed" for a, spec in zip(value["assessments"], specs)
            if spec["name"] in {"multiple_work_organization", "decision_comprehension_when_applicable"}))

    def test_historical_decision_fidelity_requires_complete_original_conversation(self):
        p = preparation()
        value = completed(p)
        p["index"]["evidence"]["work_capture"]["projection"]["semantic_complete"] = False
        with self.assertRaisesRegex(ValueError, "semantically incomplete"):
            q.validate_value(p, "d" * 64, value)

    def test_campaign_derived_criteria_remain_independent(self):
        fixture = json.loads((Path(__file__).with_name("fixtures") /
                              "qualitative-review-regressions.json").read_text())
        p = preparation()
        for case in fixture["cases"]:
            value = completed(p)
            for name, state in case["assessments"].items():
                finding = next(a for a in value["assessments"] if a["criterion_id"].endswith("/" + name))
                fill(finding, p, state)
                finding["reasoning"] = case["evidence_summary"]
            result = q.validate_value(p, "d" * 64, value)
            self.assertEqual(result["assessment_state"],
                "violated" if "violated" in case["assessments"].values() else
                "insufficient_evidence" if "insufficient_evidence" in case["assessments"].values() else "satisfied", case["id"])
        mixed = completed(p)
        architecture = next(a for a in mixed["assessments"] if a["criterion_id"].endswith("/architecture_components_flow"))
        code = next(a for a in mixed["assessments"] if a["criterion_id"].endswith("/code_behavior"))
        fill(architecture, p, "violated")
        fill(code, p, "satisfied")
        self.assertNotEqual(architecture["reasoning"], code["reasoning"])
        self.assertEqual(q.validate_value(p, "d" * 64, mixed)["assessment_state"], "violated")

    def test_satisfaction_requires_criterion_specific_citations_and_dimensions(self):
        p = preparation()
        value = completed(p)
        usefulness = next(a for a in value["assessments"] if a["criterion_id"].endswith("/documents/usefulness"))
        usefulness["criterion_observations"] = []
        with self.assertRaisesRegex(ValueError, "semantic dimensions"):
            q.validate_value(p, "d" * 64, value)
        value = completed(p)
        usefulness = next(a for a in value["assessments"] if a["criterion_id"].endswith("/documents/usefulness"))
        usefulness["evidence"][0].pop("relevance")
        with self.assertRaisesRegex(ValueError, "criterion-specific relevance"):
            q.validate_value(p, "d" * 64, value)

    def test_later_repair_does_not_erase_work_judgment(self):
        self.assertEqual(compatibility_review_result(), "failed")

    def test_semantic_omissions_require_insufficient_interaction_evidence(self):
        for surface in ("work_capture", "resume_capture"):
            for state in ("satisfied", "violated"):
                with self.subTest(surface=surface, state=state):
                    p = preparation()
                    p["index"]["evidence"][surface]["projection"].update(
                        semantic_complete=False, semantic_omission_count=1)
                    value = q.template(p, "d" * 64)
                    value["observation_scope"]["inspected_evidence"] = sorted(p["index"]["evidence"])
                    target = next(a for a in value["assessments"] if a["criterion_id"].endswith("/interaction_coverage_adequacy"))
                    fill(target, p, state)
                    with self.assertRaisesRegex(ValueError, "complete Work/resume|semantically incomplete"):
                        q.validate_value(p, "d" * 64, value)
                    fill(target, p, "insufficient_evidence")
                    target["reasoning"] = "Inspected the required capture omission metadata; actual interaction was omitted for privacy."
                    self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "insufficient_evidence")

    def test_all_decisive_direct_interaction_groups_require_complete_captures(self):
        p = preparation()
        p["index"]["evidence"]["work_capture"]["projection"]["semantic_complete"] = False
        for spec in q.criterion_specs(p["index"], p["rubric"]):
            if "work_capture" not in q.SURFACES[spec["group"]]:
                continue
            for state in ("satisfied", "violated"):
                value = q.observation(spec["criterion_id"])
                fill(value, p, state)
                with self.subTest(criterion=spec["criterion_id"], state=state):
                    with self.assertRaisesRegex(ValueError, "semantically incomplete"):
                        q.validate_assessment(value, spec, p, list(p["index"]["evidence"]))

    def test_shared_rubric_and_immutable_kind(self):
        agent, human = preparation(), preparation("human")
        self.assertEqual(q.criterion_specs(agent["index"], agent["rubric"]), q.criterion_specs(human["index"], human["rubric"]))
        for p in (agent, human):
            self.assertEqual(q.validate_value(p, "d" * 64, completed(p))["assessment_state"], "satisfied")
        value = completed(agent)
        value["reviewer"]["kind"] = "human"
        with self.assertRaisesRegex(ValueError, "reviewer"):
            q.validate_value(agent, "d" * 64, value)
        agent["reviewer"]["kind"] = "human"
        with self.assertRaisesRegex(ValueError, "human"):
            q.validate_reviewer(agent["reviewer"], agent["evaluated_sessions"])

    def test_identity_is_never_attested_by_arbitrary_model(self):
        p = preparation()
        for field in ("host", "agent", "model", "session", "person"):
            v = copy.deepcopy(p["reviewer"])
            v["identity"][field] = {"state": "verified", "value": "arbitrary-exact-identity"}
            with self.assertRaisesRegex(ValueError, "verified"):
                q.validate_reviewer(v, p["evaluated_sessions"])
        p["reviewer"]["identity"]["session"]["value"] = "work-session"
        with self.assertRaisesRegex(ValueError, "evaluated"):
            q.validate_reviewer(p["reviewer"], p["evaluated_sessions"])

    def test_incomplete_and_insufficient_do_not_satisfy(self):
        p = preparation()
        value = q.template(p, "d" * 64)
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "not_reviewed")
        value = completed(p)
        value["assessments"][0]["assessment"] = "insufficient_evidence"
        value["assessments"][0]["evidence"] = []
        result = q.validate_value(p, "d" * 64, value)
        self.assertEqual(result["assessment_state"], "insufficient_evidence")
        self.assertFalse(result["phase_9_ready"])
        self.assertFalse(result["semantic_judgment_verified"])
        value["assessments"].pop()
        with self.assertRaisesRegex(ValueError, "omitted"):
            q.validate_value(p, "d" * 64, value)
        value = completed(p)
        authority_index = next(i for i, a in enumerate(value["assessments"]) if a["authority"] is not None)
        value["assessments"][authority_index] = copy.deepcopy(value["assessments"][0])
        with self.assertRaisesRegex(ValueError, "criterion identity"):
            q.validate_value(p, "d" * 64, value)

    def test_references_counterevidence_and_observation_limits(self):
        p = preparation()
        for field, replacement in [("evidence_id", "missing"), ("locator", {"kind": "json_pointer", "value": "/missing"})]:
            value = completed(p)
            value["assessments"][0]["evidence"][0][field] = replacement
            with self.assertRaises(ValueError):
                q.validate_value(p, "d" * 64, value)
        value = completed(p)
        value["assessments"][0]["counterevidence"] = None
        with self.assertRaisesRegex(ValueError, "counterevidence"):
            q.validate_value(p, "d" * 64, value)
        value = completed(p)
        value["observation_scope"]["limits"] = []
        with self.assertRaisesRegex(ValueError, "limits"):
            q.validate_value(p, "d" * 64, value)
        value = completed(p)
        value["assessments"][0]["evidence"] = [r for r in value["assessments"][0]["evidence"] if r["evidence_id"] != "work_capture"]
        with self.assertRaisesRegex(ValueError, "surface"):
            q.validate_value(p, "d" * 64, value)
        value = completed(p)
        finding = next(a for a in value["assessments"] if "/documents/" in a["criterion_id"])
        finding["evidence"] = [r for r in finding["evidence"] if r["evidence_id"] != "document-handoff-resume"]
        with self.assertRaisesRegex(ValueError, "all four"):
            q.validate_value(p, "d" * 64, value)

    def test_applicability_is_bounded_by_criterion(self):
        p = preparation()
        value = completed(p)
        v = value["assessments"][0]
        v.update(assessment="not_applicable", applicability_reason={"code": "not_observed", "reasoning": "No surface."})
        with self.assertRaisesRegex(ValueError, "applicability"):
            q.validate_value(p, "d" * 64, value)
        value = completed(p)
        v = next(a for a in value["assessments"] if a["criterion_id"].endswith("polyglot_comprehension_when_applicable"))
        v.update(assessment="not_applicable", applicability_reason={"code": "single_language_scope", "reasoning": "Inspected one language in this fixture."})
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "satisfied")
        p["index"]["journey_samples"][0]["repository_class"] = "polyglot-medium"
        with self.assertRaisesRegex(ValueError, "polyglot"):
            q.validate_value(p, "d" * 64, value)

    def test_machine_disagreement_and_hard_block_remain(self):
        p = preparation()
        finding = m.finding("raw_hash", "confirmed_violation", {"reason": "fixture"})
        p["index"]["machine_findings"]["f1"] = {"sample_id": "journey-volicord-work-a", "finding": finding}
        value = completed(p)
        relation = {"finding_id": "f1", "relationship": "probable_false_positive", "reasoning": "Reviewer disputes the machine basis; policy remains blocking."}
        value["assessments"][0]["machine_relationships"] = [relation]
        result = q.validate_value(p, "d" * 64, value)
        self.assertEqual(result["hard_machine_findings"], ["f1"])
        self.assertEqual(result["completion_preflight"]["machine_finding_dispositions"], [{
            "finding_id": "f1", "status": "confirmed_violation", "disposition": "hard_blocking"}])
        self.assertEqual(result["qualification_state"], "not_run")
        for name in ("agrees", "clarifies_indeterminate"):
            relation["relationship"] = name
            with self.assertRaises(ValueError):
                q.validate_value(p, "d" * 64, value)
        p["index"]["machine_findings"]["f1"]["finding"] = m.finding("source_grounded_checkpoint", "indeterminate", {"reason": "fixture"})
        reviewed = q.validate_value(p, "d" * 64, value)
        self.assertEqual(reviewed["hard_machine_findings"], [])
        self.assertEqual(reviewed["completion_preflight"]["unaddressed_review_required_finding_ids"], [])
        value["assessments"][0]["machine_relationships"] = []
        self.assertEqual(q.validate_value(p, "d" * 64, value)["completion_preflight"]
            ["unaddressed_review_required_finding_ids"], ["f1"])
        value["override_hard_findings"] = True
        with self.assertRaises(ValueError):
            q.validate_value(p, "d" * 64, value)

    def test_authority_protections_and_additional_outcomes(self):
        p = preparation()
        for case in interaction_fixture()["cases"]:
            value = completed(p)
            extra = fill(q.observation("journey-volicord-work-a/authority/additional-durability"), p,
                "satisfied" if case["expected"] == "passed" else "violated")
            extra["authority"] = interaction_assessment(case)
            value["additional_outcomes"] = [{"sample_id": "journey-volicord-work-a", "finding": extra}]
            result = q.validate_value(p, "d" * 64, value)
            self.assertEqual(result["assessment_state"], extra["assessment"], case["id"])
            if case["expected"] == "failed":
                extra["assessment"] = "satisfied"
                with self.assertRaisesRegex(ValueError, "authority disposition"):
                    q.validate_value(p, "d" * 64, value)

    def test_human_resolution_accepts_only_a_valid_bound_additional_outcome(self):
        p = preparation("human")
        value = completed(p)
        criterion_id = "journey-volicord-work-a/authority/additional-durability"
        extra = fill(q.observation(criterion_id), p)
        extra["authority"] = assessment()
        value["additional_outcomes"] = [{"sample_id": "journey-volicord-work-a", "finding": extra}]
        value["resolves_review_runs"] = {criterion_id: ["a" * 32]}
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "satisfied")

        for runs in (["a" * 32, "a" * 32], [value["reviewer"]["run_id"]], ["malformed"]):
            invalid = copy.deepcopy(value)
            invalid["resolves_review_runs"][criterion_id] = runs
            with self.assertRaisesRegex(ValueError, "invalid resolved"):
                q.validate_value(p, "d" * 64, invalid)
        invalid = copy.deepcopy(value)
        invalid["additional_outcomes"][0]["sample_id"] = "small-python-1"
        with self.assertRaisesRegex(ValueError, "additional outcome"):
            q.validate_value(p, "d" * 64, invalid)
        unknown = copy.deepcopy(value)
        unknown["resolves_review_runs"] = {"journey-volicord-work-a/authority/additional-unknown": ["a" * 32]}
        with self.assertRaisesRegex(ValueError, "invalid resolved"):
            q.validate_value(p, "d" * 64, unknown)

    def test_partial_authority_observation_is_insufficient(self):
        p = preparation()
        value = completed(p)
        finding = next(a for a in value["assessments"] if a["authority"] is not None)
        references = copy.deepcopy(finding["evidence"])
        finding["authority"]["authority_relation_to_outcome"] = "uncertain"
        finding["assessment"] = "insufficient_evidence"
        finding["evidence"] = []
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "insufficient_evidence")
        finding["assessment"] = "satisfied"
        finding["evidence"] = references
        with self.assertRaisesRegex(ValueError, "authority disposition"):
            q.validate_value(p, "d" * 64, value)
        finding["authority"]["chronology"] = "late"
        finding["assessment"] = "violated"
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "violated")

    def test_behavior_opportunities_are_available_without_profile_assignment(self):
        p = preparation()
        p["index"]["samples"][0].pop("materiality_obligations", None)
        names = {s["name"] for s in q.criterion_specs(p["index"], p["rubric"])}
        self.assertTrue(set(p["rubric"]["behavior_criteria"]) <= names)

    def test_not_observed_learning_requires_inactive_runtime_evidence(self):
        p = preparation()
        value = completed(p)
        finding = next(a for a in value["assessments"]
            if a["criterion_id"].endswith("/learning_fork_value"))
        finding["assessment"] = "not_observed"
        finding["evidence"] = []
        finding["criterion_observations"] = []
        sample = finding["criterion_id"].split("/")[0]
        identity = sample + "/learning_participation"
        import machine_findings as machine
        p["index"]["machine_findings"][identity] = {
            "sample_id": sample, "finding": machine.finding(
                "learning_participation", "indeterminate",
                {"reason": "runtime_learning_active_requires_post_hoc_review"})}
        with self.assertRaisesRegex(ValueError, "active or uncertain Learning"):
            q.validate_value(p, "d" * 64, value)
        p["index"]["machine_findings"][identity] = {
            "sample_id": sample, "finding": machine.finding(
                "learning_participation", "not_observed",
                {"reason": "runtime_learning_participation_not_active"})}
        self.assertEqual(q.validate_value(p, "d" * 64, value)["assessment_state"], "not_observed")

    def test_cli_criteria_are_repository_class_scoped_once(self):
        p = preparation()
        specs = q.criterion_specs(p["index"], p["rubric"])
        cli = [spec for spec in specs if spec["group"] == "cli"]
        self.assertEqual(len(cli), 21)
        self.assertEqual({spec["sample_id"] for spec in cli},
            {"volicord", "small-python", "polyglot-medium"})
        self.assertFalse(any(re.match(r".+-cycle-[0-9]+/cli/|.+-[0-9]+/cli/", spec["criterion_id"])
                             for spec in cli))


def run_contract_tests():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(ContractTests))
    if not result.wasSuccessful():
        raise AssertionError("qualitative review contract self-test failed")


if __name__ == "__main__":
    unittest.main()
