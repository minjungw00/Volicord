#!/usr/bin/env python3
"""Sanitized reviewer judgments; never semantic grading of naturalistic work."""
import copy
import json
from pathlib import Path
import unittest

import qualitative_review as q
import machine_findings as m
from authority_obligations_self_test import assessment, interaction_assessment, interaction_fixture


def preparation(kind="agent"):
    definition = json.loads(Path(__file__).with_name("evaluation.json").read_text())
    policy = q.rubric(definition)
    sample = {"sample_id": "volicord-1", "repository_class": "volicord", "cycle": 1,
        "behavior_class": "explicit_user_owned_decision", "authority_obligations": ["all-material-outcomes"],
        "authority_evidence": {"work_capture": "work_capture", "canonical_bundle": "canonical_bundle"}}
    index = {"samples": [sample], "live_viewer_sample": "volicord-1", "machine_findings": {}, "evidence": {}}
    for surface in {s for surfaces in q.SURFACES.values() for s in surfaces}:
        for locale in (["en", "ko"] if surface == "live_viewer_observation" else [None]):
            name = surface + ("-" + locale if locale else "")
            index["evidence"][name] = {"sample_id": sample["sample_id"], "surface": surface,
                "locale": locale, "sha256": "a" * 64, "path": name,
                "locators": [{"kind": "json_pointer", "value": "/fact"}]}
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
    value.update(assessment=state, reasoning="Synthetic reviewer inspected the criterion in the cited artifact.",
        uncertainty="Fixture-only judgment; no actual Product qualification.",
        counterevidence={"state": "none_found", "reasoning": "No contrary evidence in the inspected synthetic case.", "evidence": []},
        evidence=[{"evidence_id": name, "locator": entry["locators"][0]}
            for name, entry in sorted(p["index"]["evidence"].items())])
    if "/authority/" in value["criterion_id"] and not value["criterion_id"].endswith("/coverage"):
        value["authority"] = assessment()
    return value


def compatibility_review_result():
    p = preparation()
    value = completed(p)
    finding = next(a for a in value["assessments"] if a["criterion_id"] == "volicord-1/interaction/source_grounding")
    finding.update(assessment="violated", reasoning="Work claimed completion after focused tests, but fresh resume proved an existing supported input was rejected and repaired it. Later repair does not establish the original completion claim.")
    result = q.validate_value(p, "d" * 64, value)
    if result["assessment_state"] != "violated" or result["phase_9_ready"]:
        raise AssertionError("later compatibility repair erased original reviewed failure")
    return "failed"


class ContractTests(unittest.TestCase):
    def test_later_repair_does_not_erase_work_judgment(self):
        self.assertEqual(compatibility_review_result(), "failed")

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
        result = q.validate_value(p, "d" * 64, value)
        self.assertEqual(result["assessment_state"], "insufficient_evidence")
        self.assertFalse(result["phase_9_ready"])
        self.assertFalse(result["semantic_judgment_verified"])
        value["assessments"].pop()
        with self.assertRaisesRegex(ValueError, "omitted"):
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
        p["index"]["samples"][0]["repository_class"] = "polyglot-medium"
        with self.assertRaisesRegex(ValueError, "polyglot"):
            q.validate_value(p, "d" * 64, value)

    def test_machine_disagreement_and_hard_block_remain(self):
        p = preparation()
        finding = m.finding("source_grounded_checkpoint", "confirmed_violation", {"reason": "fixture"})
        p["index"]["machine_findings"]["f1"] = {"sample_id": "volicord-1", "finding": finding}
        value = completed(p)
        relation = {"finding_id": "f1", "relationship": "probable_false_positive", "reasoning": "Reviewer disputes the machine basis; policy remains blocking."}
        value["assessments"][0]["machine_relationships"] = [relation]
        result = q.validate_value(p, "d" * 64, value)
        self.assertEqual(result["hard_machine_findings"], ["f1"])
        self.assertEqual(result["qualification_state"], "not_run")
        for name in ("agrees", "clarifies_indeterminate"):
            relation["relationship"] = name
            with self.assertRaises(ValueError):
                q.validate_value(p, "d" * 64, value)
        p["index"]["machine_findings"]["f1"]["finding"] = m.finding("source_grounded_checkpoint", "indeterminate", {"reason": "fixture"})
        self.assertEqual(q.validate_value(p, "d" * 64, value)["hard_machine_findings"], [])
        value["override_hard_findings"] = True
        with self.assertRaises(ValueError):
            q.validate_value(p, "d" * 64, value)

    def test_authority_protections_and_additional_outcomes(self):
        p = preparation()
        for case in interaction_fixture()["cases"]:
            value = completed(p)
            extra = fill(q.observation("volicord-1/authority/additional-durability"), p,
                "satisfied" if case["expected"] == "passed" else "violated")
            extra["authority"] = interaction_assessment(case)
            value["additional_outcomes"] = [{"sample_id": "volicord-1", "finding": extra}]
            result = q.validate_value(p, "d" * 64, value)
            self.assertEqual(result["assessment_state"], extra["assessment"], case["id"])
            if case["expected"] == "failed":
                extra["assessment"] = "satisfied"
                with self.assertRaisesRegex(ValueError, "authority disposition"):
                    q.validate_value(p, "d" * 64, value)

    def test_all_behavior_rubric_criteria_preserved(self):
        p = preparation()
        seen = set()
        for behavior in {b for rule in p["rubric"]["behavior_criteria"].values() for b in rule["applies_to"]}:
            p["index"]["samples"][0]["behavior_class"] = behavior
            names = {s["name"] for s in q.criterion_specs(p["index"], p["rubric"])}
            for name, rule in p["rubric"]["behavior_criteria"].items():
                self.assertEqual(name in names, behavior in rule["applies_to"])
            seen |= names
        self.assertTrue(set(p["rubric"]["behavior_criteria"]) <= seen)


def run_contract_tests():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(ContractTests))
    if not result.wasSuccessful():
        raise AssertionError("qualitative review contract self-test failed")


if __name__ == "__main__":
    unittest.main()
