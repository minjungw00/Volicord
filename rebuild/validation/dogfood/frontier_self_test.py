"""Sanitized prospective authority regressions using maintained session fixtures."""
from copy import deepcopy
from dataclasses import replace
from pathlib import Path
import json
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

    def learning_revision(self, kind="research_evidence"):
        descriptor, capture, bundle = self.fixture("learning_deliberation")
        record = next(c for c in capture.tool_calls if c.operation == "materiality_review"
            and c.arguments.get("action") == "record")
        inspect = next(c for c in capture.tool_calls if c.operation == "materiality_review"
            and c.arguments.get("action") == "inspect")
        judgments = deepcopy(record.arguments["judgments"])
        dimension = next(j for j in judgments if j["learning_value"]["state"] == "deliberation_worthy")
        dimension["learning_value"] = {"state": "routine", "rationale": "Current evidence removes the trade-off."}
        basis = {"dimension_id": dimension["choice_id"], "kind": kind,
            "source_ids": ["0f" * 16], "evidence_basis": ["Both representations share the enforced invariant."],
            "rationale": "The previously credible trade-off is no longer present."}
        if kind == "current_user_withdrawal":
            statement = "I withdraw learning participation for this choice; proceed routinely."
            turn = capture.user_turns[-1]
            source_id = "ab" * 16
            source = {**bundle.rows("sources")[0], "id": source_id, "locator": statement}
            bundle = replace(bundle, tables={**bundle.tables, "sources": (*bundle.rows("sources"), source)})
            capture = replace(capture, user_turns=(*capture.user_turns,
                replace(turn, sequence=inspect.sequence - 30, text=statement, turn_id="withdrawal")))
            basis = {"dimension_id": dimension["choice_id"], "kind": kind,
                "user_turn_source_id": source_id, "verbatim_statement": statement,
                "rationale": "The user explicitly withdrew participation for this choice."}
        revision = replace(record, call_id="learning-value-revision", sequence=inspect.sequence - 20,
            completion_sequence=inspect.sequence - 10,
            arguments={"action": "revise", "project_id": bundle.project_id,
                "review_candidate_id": record.result["review_candidate_id"],
                "rationale": "Reassess the prior learning fork from supported evidence.",
                "learning_participation": deepcopy(record.arguments["learning_participation"]),
                "judgments": judgments, "learning_value_revision_bases": [basis]},
            result={**record.result, "action": "revise", "review_revision": 2})
        output = deepcopy(inspect.result)
        output["review_revision"] = 2
        output["executable_work_scope"]["authority_basis"]["review_revision"] = 2
        calls = [replace(c, result=output) if c is inspect else c for c in capture.tool_calls]
        capture = replace(capture, tool_calls=tuple(sorted((*calls, revision), key=lambda c: c.sequence)))
        return descriptor, capture, bundle, revision

    def test_revision_optional_field_is_closed_and_omission_equals_empty(self):
        descriptor, capture, bundle = self.fixture("explicit_user_owned_decision")
        revision = next(c for c in capture.tool_calls if c.arguments.get("action") == "revise")
        for arguments, expected in ((revision.arguments, True),
            ({**revision.arguments, "learning_value_revision_bases": []}, True),
            ({**revision.arguments, "unsupported": []}, False),
            ({k: v for k, v in revision.arguments.items() if k != "rationale"}, False)):
            with self.subTest(arguments=arguments):
                changed = replace(capture, tool_calls=tuple(replace(c, arguments=arguments)
                    if c is revision else c for c in capture.tool_calls))
                self.assertEqual(self.facts(descriptor, changed, bundle)[0], expected)

    def test_supported_learning_revision_bases(self):
        for kind in ("research_evidence", "prototype_evidence", "current_user_withdrawal"):
            with self.subTest(kind=kind):
                descriptor, capture, bundle, _ = self.learning_revision(kind)
                self.assertTrue(self.facts(descriptor, capture, bundle)[0])
                self.assertEqual(capture.calls("decision_record"), [])

    def test_learning_revision_rejects_missing_malformed_and_inappropriate_bases(self):
        descriptor, capture, bundle, revision = self.learning_revision()
        basis = revision.arguments["learning_value_revision_bases"][0]
        cases = [None, {}, "research", [], [None], [basis, basis],
            [{**basis, "kind": "agent_preference"}], [{**basis, "dimension_id": "missing"}],
            [{**basis, "dimension_id": revision.arguments["judgments"][1]["choice_id"]}],
            [{**basis, "extra": True}], [{k: v for k, v in basis.items() if k != "rationale"}],
            [{**basis, "rationale": " "}], [{**basis, "evidence_basis": []}],
            [{**basis, "evidence_basis": [None]}], [{**basis, "source_ids": []}],
            [{**basis, "source_ids": ["missing"]}], [{**basis, "source_ids": ["03" * 16]}],
            [{**basis, "source_ids": basis["source_ids"] * 2}]]
        for value in cases:
            with self.subTest(value=value):
                arguments = {**revision.arguments, "learning_value_revision_bases": value}
                changed = replace(capture, tool_calls=tuple(replace(c, arguments=arguments)
                    if c is revision else c for c in capture.tool_calls))
                self.assertFalse(self.facts(descriptor, changed, bundle)[0])
        arguments = {k: v for k, v in revision.arguments.items() if k != "learning_value_revision_bases"}
        changed = replace(capture, tool_calls=tuple(replace(c, arguments=arguments)
            if c is revision else c for c in capture.tool_calls))
        self.assertFalse(self.facts(descriptor, changed, bundle)[0])

    def test_learning_revision_rejects_noncurrent_evidence_and_false_withdrawal(self):
        for kind in ("research_evidence", "prototype_evidence", "current_user_withdrawal"):
            descriptor, capture, bundle, revision = self.learning_revision(kind)
            basis = revision.arguments["learning_value_revision_bases"][0]
            source_id = basis.get("user_turn_source_id", "0f" * 16)
            mutations = [{"availability": state} for state in ("stale", "unavailable", "unknown")]
            mutations += [{"project_id": "ff" * 16}, {"source_kind": "unsupported"}]
            if kind == "current_user_withdrawal":
                mutations += [{"actor_kind": "agent"}, {"detail_one": "other-host"},
                    {"detail_two": "other-session"}, {"locator": "I still want to deliberate."}]
            for mutation in mutations:
                with self.subTest(kind=kind, mutation=mutation):
                    changed_bundle = replace(bundle, tables={**bundle.tables, "sources": tuple(
                        {**s, **mutation} if s["id"] == source_id else s for s in bundle.rows("sources"))})
                    self.assertFalse(self.facts(descriptor, capture, changed_bundle)[0])
            if kind == "current_user_withdrawal":
                for turns in (capture.user_turns[:-1], (*capture.user_turns[:-1],
                    replace(capture.user_turns[-1], sequence=revision.completion_sequence + 1))):
                    self.assertFalse(self.facts(descriptor, replace(capture, user_turns=turns), bundle)[0])

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

    def test_delegated_behavior_does_not_hide_independent_user_owned_policy(self):
        descriptor, capture, bundle = self.fixture("explicit_user_owned_decision")
        descriptor["behavior_class"] = "delegated_implementation_choice"
        self.assertTrue(self.observe(descriptor, capture))
        self.assertTrue(self.facts(descriptor, capture, bundle)[0])
        capture = replace(capture, tool_calls=tuple(c for c in capture.tool_calls if c.operation != "decision_record"))
        self.assertFalse(self.observe(descriptor, capture))
        self.assertFalse(self.facts(descriptor, capture, bundle)[0])

    def test_experiment_resolves_uncertainty_into_agent_owned_choice(self):
        descriptor, capture, bundle = self.fixture("exploratory_uncertainty")
        _, agent_capture, _ = self.fixture("learning_routine_control")
        record = next(c for c in capture.tool_calls if c.arguments.get("action") == "record" and c.operation == "materiality_review")
        agent = next(c for c in agent_capture.tool_calls if c.arguments.get("action") == "record" and c.operation == "materiality_review")
        initial = deepcopy(record.arguments)
        initial["judgments"][0]["exploratory_disposition"] = "research_required"
        pending = replace(record, arguments=initial,
            result={**record.result, "workflow": {**record.result["workflow"], "stage": "research"}})
        resolved = replace(record, call_id="evidence-resolved-review", sequence=record.completion_sequence + 10,
            completion_sequence=record.completion_sequence + 20,
            arguments={"action": "revise", "project_id": "01" * 16, "review_candidate_id": "18" * 16,
                "rationale": "The experiment establishes the public outcome and leaves only internal representation.",
                "learning_participation": {"state": "inactive"}, "judgments": deepcopy(agent.arguments["judgments"])},
            result={**record.result, "action": "revise", "review_revision": 2})
        for judgment in resolved.arguments["judgments"]:
            judgment["learning_authority"] = {"state": "inactive"}
        calls = tuple(pending if c is record else c for c in capture.tool_calls)
        capture = replace(capture, tool_calls=tuple(sorted((*calls, resolved), key=lambda c: c.sequence)))
        self.assertTrue(self.observe(descriptor, capture))
        self.assertTrue(self.facts(descriptor, capture, bundle)[0])

    def test_resolved_historical_question_does_not_block_settled_rediscovery(self):
        descriptor, capture, bundle = self.fixture("explicit_user_owned_decision")
        _, settled, _ = self.fixture()
        discovery = settled.successful_calls("engineering_choice_discovery")[0]
        review = next(c for c in settled.tool_calls if c.operation == "materiality_review" and c.arguments.get("action") == "record")
        inspect = next(c for c in capture.tool_calls if c.operation == "materiality_review" and c.arguments.get("action") == "inspect")
        discovery = replace(discovery, call_id="rediscovery", sequence=inspect.sequence - 80, completion_sequence=inspect.sequence - 70,
            result={**discovery.result, "discovery_candidate_id": "ad" * 16})
        review = replace(review, call_id="settled-review", sequence=inspect.sequence - 60, completion_sequence=inspect.sequence - 50,
            arguments={**review.arguments, "engineering_choice_discovery_candidate_id": "ad" * 16},
            result={**review.result, "review_candidate_id": "ae" * 16})
        output = deepcopy(inspect.result)
        output.update(review_candidate_id="ae" * 16, review_revision=1)
        output["executable_work_scope"]["authority_basis"].update(review_candidate_id="ae" * 16,
            engineering_choice_discovery_candidate_id="ad" * 16, review_revision=1)
        for identity in output["workflow"]["satisfied_basis_identities"]:
            if identity["kind"] == "materiality_review_candidate":
                identity["identity"] = "ae" * 16
        binding = replace(inspect, arguments={**inspect.arguments, "review_candidate_id": "ae" * 16}, result=output)
        calls = tuple(binding if c is inspect else c for c in capture.tool_calls)
        capture = replace(capture, tool_calls=tuple(sorted((*calls, discovery, review), key=lambda c: c.sequence)))
        self.assertTrue(self.observe(descriptor, capture))
        self.assertTrue(self.facts(descriptor, capture, bundle)[0])

    def test_source_evidence_settles_initial_user_uncertainty_without_question(self):
        descriptor, capture, bundle = self.fixture("explicit_user_owned_decision")
        _, settled, _ = self.fixture()
        facts = next(c for c in settled.tool_calls if c.operation == "materiality_review" and c.arguments.get("action") == "record")
        calls = []
        for call in capture.tool_calls:
            if call.operation in {"candidate_manage", "inquiry_frontier", "decision_record"}:
                continue
            if call.arguments.get("action") == "revise":
                call = replace(call, arguments={**call.arguments, "judgments": deepcopy(facts.arguments["judgments"])})
            calls.append(call)
        capture = replace(capture, tool_calls=tuple(calls))
        self.assertTrue(self.observe(descriptor, capture))
        result = self.facts(descriptor, capture, bundle)
        self.assertTrue(result[0])
        self.assertEqual(result[3]["user_owned_dimension_ids"], [])

    def test_hidden_investigation_can_supply_ready_to_ask_evidence(self):
        descriptor, _, _ = self.fixture("hidden_user_owned_decision")
        path = self.root / descriptor["evidence"]["captures"]["work"]["file"]
        events = []
        for line in path.read_text().splitlines():
            event = json.loads(line)
            call_id = str(event.get("payload", {}).get("call_id", ""))
            if "candidate-research-call" in call_id or "candidate-ready-call" in call_id:
                continue
            if "candidate-submit-call" in call_id:
                event = json.loads(line.replace("research_required", "ready_to_ask"))
            events.append(event)
        path.write_text("".join(json.dumps(event) + "\n" for event in events))
        descriptor["evidence"]["captures"]["work"]["sha256"] = h.sha256(path)
        result = h.real_session_evidence(descriptor, kind="volicord", cycle=1, repository_revision="0" * 40)
        self.assertEqual(result["checks"]["hidden_material_discovery_order"], "passed")
        self.assertEqual(result["checks"]["appropriate_inquiry_outcome"], "passed")

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
