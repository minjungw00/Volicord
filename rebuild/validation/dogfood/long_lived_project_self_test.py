#!/usr/bin/env python3
"""Deterministic one-Project/multiple-Work Dogfood acceptance fixture."""
from __future__ import annotations

import json
from pathlib import Path
import unittest

import qualitative_review as review


FIXTURE = Path(__file__).with_name("fixtures") / "long-lived-multi-work-project.json"


class LongLivedProjectTests(unittest.TestCase):
    def setUp(self):
        self.value = json.loads(FIXTURE.read_text(encoding="utf-8"))

    def test_one_project_accumulates_distinct_work_over_time(self):
        value = self.value
        self.assertEqual(value["kind"], "dogfood_long_lived_project_fixture")
        self.assertEqual(value["schema_version"], 1)
        self.assertEqual(value["project"]["identity"], "one-stable-project")
        timeline = value["timeline"]
        self.assertEqual([event["sequence"] for event in timeline], list(range(1, len(timeline) + 1)))
        self.assertEqual({event["work_item"] for event in timeline}, {"alpha", "beta", "gamma"})
        alpha = [event for event in timeline if event["work_item"] == "alpha"]
        self.assertEqual([event["event"] for event in alpha],
            ["pause_checkpoint", "completion_checkpoint"])
        self.assertEqual({event["decision"] for event in alpha}, {"alpha-decision"})
        self.assertEqual({event["changed_path"] for event in alpha}, {"src/alpha.rs"})
        beta = next(event for event in timeline if event["work_item"] == "beta")
        self.assertEqual(beta["decision"], "beta-decision")
        self.assertNotEqual(beta["changed_path"], alpha[0]["changed_path"])

    def test_viewer_and_review_contract_distinguish_every_work_item(self):
        groups = self.value["expected_viewer_groups"]
        self.assertEqual(set(groups), {"completed-work", "current-work", "remaining-work"})
        flattened = [work for values in groups.values() for work in values]
        self.assertEqual(sorted(flattened), ["alpha", "beta", "gamma"])
        self.assertEqual(len(flattened), len(set(flattened)))
        criteria = set(self.value["expected_review_criteria"])
        self.assertTrue(criteria <= set(review.CRITERIA["viewer_snapshot"]))
        self.assertIn("multiple_work_organization", review.CRITERION_OBSERVATIONS)
        self.assertIn("stable_work_identities",
            review.CRITERION_OBSERVATIONS["multiple_work_organization"])

    def test_fixture_routes_to_production_restart_portability_cli_and_viewer_tests(self):
        routes = {(route["package"], route["test"]) for route in self.value["production_routes"]}
        self.assertEqual(routes, {
            ("volicord-operations", "multi_work_identity_survives_restart_portability_and_read_consumers"),
            ("volicord-viewer", "project_understanding_renders_three_stable_work_items_as_separate_hierarchy"),
        })

    def test_deterministic_fixture_is_not_naturalistic_qualification_evidence(self):
        self.assertEqual(self.value["evidence_class"], "deterministic_support_only")
        self.assertNotIn("long_lived_project_observation", self.value)


def check_long_lived_project_regressions():
    result = unittest.TextTestRunner(verbosity=1).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(LongLivedProjectTests))
    if not result.wasSuccessful():
        raise AssertionError("long-lived multi-work Project regressions failed")


if __name__ == "__main__":
    check_long_lived_project_regressions()
