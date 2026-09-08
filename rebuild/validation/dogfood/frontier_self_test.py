"""Sanitized prospective authority regressions using maintained session fixtures."""
from copy import deepcopy
from dataclasses import replace
from pathlib import Path
import tempfile
import unittest

import harness as h


class FrontierTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def fixture(self, behavior="research_or_no_question"):
        descriptor = h.real_session_fixture("volicord", 1, "0" * 40, self.root, behavior_class=behavior)
        capture = h.load_codex_capture(self.root / descriptor["evidence"]["captures"]["work"]["file"])
        # Leave insertion room while retaining every observed partial order.
        fields = {}
        for field in ("tool_calls", "commands", "path_observations", "user_turns"):
            fields[field] = tuple(replace(item, sequence=item.sequence * 100,
                **({"completion_sequence": item.completion_sequence * 100} if hasattr(item, "completion_sequence") else {}))
                for item in getattr(capture, field))
        capture = replace(capture, **fields)
        bundle = h.load_canonical_bundle(self.root / descriptor["evidence"]["canonical_bundle"]["file"])
        return descriptor, capture, bundle

    def facts(self, descriptor, capture, bundle):
        baseline = capture.successful_calls("repository_analyze")[0]
        first_write = min(x.sequence for x in h.meaningful_work_path_observations(capture))
        return h.materiality_review_facts(capture, bundle, descriptor["behavior_class"],
            "08" * 16, descriptor["work_user_task"], descriptor["work_user_task"], baseline,
            first_write, "03" * 16, h.decision_facts(capture, bundle)[-1])

    def observe(self, descriptor, capture):
        return h.work_blocker_behavior_observations(capture, descriptor["behavior_class"],
            capture.successful_calls("repository_analyze")[0],
            min(x.sequence for x in h.meaningful_work_path_observations(capture)))[0]

    def prepend_history(self, capture):
        discovery = capture.successful_calls("engineering_choice_discovery")[0]
        review = next(c for c in capture.successful_calls("materiality_review") if c.arguments["action"] == "record")
        old_discovery = replace(discovery, sequence=discovery.sequence - 40,
            completion_sequence=discovery.sequence - 30, call_id="old-discovery",
            result={**discovery.result, "discovery_candidate_id": "ad" * 16})
        old_review = replace(review, sequence=discovery.sequence - 20,
            completion_sequence=discovery.sequence - 10, call_id="old-review",
            arguments={**review.arguments, "engineering_choice_discovery_candidate_id": "ad" * 16},
            result={**review.result, "review_candidate_id": "ae" * 16})
        return replace(capture, tool_calls=tuple(sorted((*capture.tool_calls, old_discovery, old_review), key=lambda c: c.sequence)))

    def test_rediscovery_selects_current_review_in_both_evaluators(self):
        for behavior in ("research_or_no_question", "hidden_user_owned_decision"):
            descriptor, capture, bundle = self.fixture(behavior)
            capture = self.prepend_history(capture)
            self.assertTrue(self.observe(descriptor, capture))
            facts = self.facts(descriptor, capture, bundle)
            self.assertTrue(facts[0])
            self.assertEqual(facts[1], "18" * 16)

    def test_new_discovery_without_review_cannot_fall_back(self):
        descriptor, capture, bundle = self.fixture()
        capture = self.prepend_history(capture)
        capture = replace(capture, tool_calls=tuple(c for c in capture.tool_calls
            if c.result.get("review_candidate_id") != "18" * 16))
        self.assertFalse(self.observe(descriptor, capture))
        self.assertFalse(self.facts(descriptor, capture, bundle)[0])

    def test_late_revision_cannot_authorize_earlier_write(self):
        descriptor, capture, bundle = self.fixture("hidden_user_owned_decision")
        last_write = max(x.sequence for x in capture.path_observations)
        capture = replace(capture, tool_calls=tuple(replace(c, sequence=last_write + 1,
            completion_sequence=last_write + 2) if c.arguments.get("action") == "revise" else c
            for c in capture.tool_calls))
        self.assertFalse(self.observe(descriptor, capture))
        self.assertFalse(self.facts(descriptor, capture, bundle)[0])

    def test_multiple_revisions_and_later_history_are_prospective(self):
        descriptor, capture, bundle = self.fixture("hidden_user_owned_decision")
        revision = next(c for c in capture.tool_calls if c.arguments.get("action") == "revise")
        intermediate = replace(revision, sequence=revision.sequence - 20,
            completion_sequence=revision.sequence - 10, call_id="intermediate-revision")
        current = replace(revision, result={**revision.result, "review_revision": 3})
        late = replace(current, sequence=100000, completion_sequence=100001, call_id="later-history",
            arguments={**current.arguments, "judgments": []})
        calls = tuple(current if c is revision else c for c in capture.tool_calls)
        capture = replace(capture, tool_calls=tuple(sorted((*calls, intermediate, late), key=lambda c: c.sequence)))
        self.assertTrue(self.observe(descriptor, capture))
        self.assertTrue(self.facts(descriptor, capture, bundle)[0])

    def test_obsolete_review_revised_later_cannot_regain_control(self):
        descriptor, capture, bundle = self.fixture()
        capture = self.prepend_history(capture)
        old_review = next(c for c in capture.tool_calls if c.call_id == "old-review")
        current = next(c for c in capture.tool_calls if c.result.get("review_candidate_id") == "18" * 16)
        obsolete = replace(old_review, call_id="obsolete-revision", sequence=current.completion_sequence + 1,
            completion_sequence=current.completion_sequence + 2,
            arguments={**old_review.arguments, "action": "revise", "review_candidate_id": "ae" * 16},
            result={**old_review.result, "review_revision": 2})
        capture = replace(capture, tool_calls=tuple(sorted((*capture.tool_calls, obsolete), key=lambda c: c.sequence)))
        self.assertTrue(self.observe(descriptor, capture))
        self.assertEqual(self.facts(descriptor, capture, bundle)[1], "18" * 16)

    def test_non_user_authority_is_not_a_behavior_label(self):
        for behavior in ("delegated_implementation_choice", "exploratory_uncertainty"):
            descriptor, capture, bundle = self.fixture()
            descriptor["behavior_class"] = behavior
            self.assertTrue(self.observe(descriptor, capture))
            self.assertTrue(self.facts(descriptor, capture, bundle)[0])

    def test_learning_only_cannot_authorize_canonical_decision(self):
        descriptor, capture, bundle = self.fixture("hidden_user_owned_decision")
        calls = deepcopy(capture.tool_calls)
        for call in calls:
            for judgment in call.arguments.get("judgments", []):
                if judgment["disposition"] == "unresolved_user_owned_outcome":
                    judgment["learning_authority"] = {"state": "assessed", "independent_user_authority": False,
                        "rationale": "Learning participation only", "source_ids": ["03" * 16]}
        capture = replace(capture, tool_calls=calls)
        self.assertFalse(self.observe(descriptor, capture))
        self.assertFalse(self.facts(descriptor, capture, bundle)[0])

    def test_temporal_result_must_match_primary_binding(self):
        _, capture, _ = self.fixture()
        discovery = deepcopy(capture.successful_calls("engineering_choice_discovery")[0].arguments)
        review = next(c for c in capture.successful_calls("materiality_review") if c.arguments["action"] == "record")
        outcome = {"outcome_id": "expiry", "affected_choice_ids": [],
            "credible_outcomes": [{"result_id": "preserved"}, {"result_id": "renewed"}],
            "conclusion": {"state": "no_independent_fork", "basis": "settled_by_current_sources", "result_id": "preserved"}}
        discovery["interaction_review"] = [{"axis": "temporal_and_lifetime", "outcomes": [outcome]}]
        commitment = {"outcome_binding": {"state": "reviewed_interaction", "outcome_id": "expiry", "result_id": "preserved"},
            "temporal_effect": {"state": "reviewed_temporal_outcome", "outcome_id": "expiry", "result_id": "preserved"}}
        self.assertTrue(h.planned_commitments_match_graph([commitment], discovery, review.arguments["judgments"]))
        commitment["temporal_effect"]["result_id"] = "renewed"
        self.assertFalse(h.planned_commitments_match_graph([commitment], discovery, review.arguments["judgments"]))
        commitment["temporal_effect"] = {"state": "no_temporal_change", "rationale": "Omitted"}
        self.assertFalse(h.planned_commitments_match_graph([commitment], discovery, review.arguments["judgments"]))


def check_frontier_regressions():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(FrontierTests))
    if not result.wasSuccessful():
        raise AssertionError("prospective authority regressions failed")


if __name__ == "__main__":
    check_frontier_regressions()
