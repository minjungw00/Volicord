"""Sanitized prospective authority regressions using maintained session fixtures."""
from copy import deepcopy
from dataclasses import replace
from pathlib import Path
import json
import tempfile
import unittest
from unittest.mock import patch

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

    def facts(self, descriptor, capture, bundle, baseline=None):
        baseline = baseline or capture.successful_calls("repository_analyze")[0]
        first_write = min(x.sequence for x in h.meaningful_work_path_observations(capture))
        return h.materiality_review_facts(capture, bundle, descriptor["behavior_class"],
            "08" * 16, descriptor["work_user_task"], descriptor["work_user_task"], baseline,
            first_write, "03" * 16, h.decision_facts(capture, bundle)[-1])

    def observe(self, descriptor, capture, baseline=None):
        return h.work_blocker_behavior_observations(capture, descriptor["behavior_class"],
            baseline or capture.successful_calls("repository_analyze")[0],
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

    def test_hidden_investigation_uncertainty_does_not_erase_lifecycle(self):
        head = h.git_head(h.ROOT)
        descriptor = h.real_session_fixture("volicord", 1, head, self.root,
            behavior_class="hidden_user_owned_decision")
        capture = h.load_codex_capture(self.root / descriptor["evidence"]["captures"]["work"]["file"])
        baseline = capture.successful_calls("repository_analyze")[0]
        discovery = capture.successful_calls("engineering_choice_discovery")[0]
        prior = [c for c in capture.commands if baseline.completion_sequence < c.sequence < discovery.sequence
                 and h.command_is_repository_inspection(c.parsed_command)]
        self.assertTrue(prior)
        for state in ("complete", "indeterminate", "missing", "failed", "late"):
            with self.subTest(state=state):
                commands = tuple(c if c not in prior or state == "complete" else
                    replace(c, exit_code=None, termination=None, evidence_state="indeterminate") if state == "indeterminate" else
                    replace(c, exit_code=1) if state == "failed" else
                    replace(c, completion_sequence=discovery.completion_sequence + 1) if state == "late" else None
                    for c in capture.commands)
                changed = replace(capture, commands=tuple(c for c in commands if c is not None))
                if state == "complete":
                    with self.assertRaises(h.NoWorkBlocker):
                        h.build_work_blocker_result(head, descriptor, "0" * 64, changed)
                else:
                    result = h.build_work_blocker_result(head, descriptor, "0" * 64, changed)
                    self.assertEqual(result["failed_checks"], list(h.HIDDEN_INVESTIGATION_CHECKS))
                    self.assertEqual(result["failure_attribution"]["domain"],
                        "evidence" if state == "indeterminate" else "behavior_contract")
                    self.assertEqual(result["product_failed_checks"],
                        [] if state == "indeterminate" else list(h.HIDDEN_INVESTIGATION_CHECKS))
                    original_loader = h.load_codex_capture
                    work_file = descriptor["evidence"]["captures"]["work"]["file"]
                    with patch.object(h, "load_codex_capture", side_effect=lambda path:
                            changed if path.name == Path(work_file).name else original_loader(path)):
                        full = h.real_session_evidence(descriptor, kind="volicord", cycle=1,
                            repository_revision=head)
                    self.assertEqual(full["checks"]["appropriate_inquiry_outcome"], "passed")
                    self.assertEqual(full["checks"]["hidden_material_discovery_order"],
                        "partial" if state == "indeterminate" else "failed")

    def no_write_exploration(self, prototype=True):
        descriptor, capture, bundle = self.fixture("exploratory_uncertainty")
        discovery = capture.successful_calls("engineering_choice_discovery")[0]
        record = next(c for c in capture.successful_calls("materiality_review") if c.arguments["action"] == "record")
        inspect = next(c for c in capture.successful_calls("materiality_review") if c.arguments["action"] == "inspect")
        choice_id = record.arguments["judgments"][0]["choice_id"]
        requirement = "prototype_required" if prototype else "research_required"
        discovered = deepcopy(discovery.arguments)
        next(c for c in discovered["choices"] if c["choice_id"] == choice_id)["evidence_state"] = requirement
        discovery = replace(discovery, arguments=discovered,
            result={**discovery.result, "choices": discovered["choices"]})
        pending = deepcopy(record.arguments)
        pending["judgments"][0]["exploratory_disposition"] = requirement
        resolved = deepcopy(record.arguments["judgments"])
        resolved[0]["evidence_completion_basis"] = ["The bounded scratch experiment resolves the original evidence gap."]
        revision = replace(record, call_id="exploratory-reassessment", sequence=inspect.sequence - 30,
            completion_sequence=inspect.sequence - 20,
            arguments={"action": "revise", "project_id": bundle.project_id,
                "review_candidate_id": record.result["review_candidate_id"], "rationale": "Reassess the experiment.",
                "learning_participation": {"state": "inactive"}, "judgments": resolved},
            result={**record.result, "action": "revise", "review_revision": 2})
        record = replace(record, arguments=pending,
            result={**record.result, "workflow": {**record.result["workflow"],
                "stage": "research_or_prototype", "disposition": "research_required"}})
        experiment = replace(capture.commands[0], sequence=record.completion_sequence + 10,
            completion_sequence=record.completion_sequence + 20, exit_code=0, termination="exited",
            evidence_state="completed", execution_identity="scratch-experiment",
            parsed_command={"cmd": "python3 probe.py" if prototype else "cat src/lib.rs",
                "workdir": "/tmp/sanitized-prototype" if prototype else str(capture.cwd)})
        calls = []
        for c in capture.tool_calls:
            if c.call_id == discovery.call_id:
                c = discovery
            elif c.call_id == record.call_id:
                c = record
            elif c is inspect:
                c = replace(c, result={**c.result, "review_revision": 2})
            elif c.operation == "checkpoint_record":
                c = replace(c, result={**c.result, "changed_paths": [],
                    "baseline_repository_snapshot_id": "aa" * 32, "current_repository_snapshot_id": "bb" * 32})
            calls.append(c)
        capture = replace(capture, path_observations=(), commands=(*capture.commands, experiment),
            tool_calls=tuple(sorted((*calls, revision), key=lambda c: c.sequence)))
        bundle = replace(bundle, tables={**bundle.tables,
            "checkpoints": tuple({**r, "changed_paths": "00" * 8} for r in bundle.rows("checkpoints")),
            "checkpoint_source_relations": tuple(r for r in bundle.rows("checkpoint_source_relations")
                if r.get("relation_kind") != "changed_basis")})
        return descriptor, capture, bundle

    def test_no_write_exploration_requires_affirmative_evidence(self):
        for prototype in (False, True):
            descriptor, capture, bundle = self.no_write_exploration(prototype)
            baseline = capture.successful_calls("repository_analyze")[0]
            self.assertTrue(h.exploratory_no_write_evidence(capture, baseline)["qualified"])
            self.assertEqual(h.work_blocker_behavior_observations(capture, descriptor["behavior_class"], baseline, None), (True, False, False))
            facts = h.materiality_review_facts(capture, bundle, descriptor["behavior_class"],
                "08" * 16, descriptor["work_user_task"], descriptor["work_user_task"], baseline,
                None, "03" * 16, h.decision_facts(capture, bundle)[-1])
            self.assertTrue(facts[0], facts[3])
            checkpoint = h.checkpoint_facts(capture, bundle, [], "08" * 16, "03" * 16,
                baseline.result["analysis_snapshot_id"], descriptor["work_user_task"])
            self.assertTrue(checkpoint[0], checkpoint)
            original_loader = h.load_codex_capture
            work_file = descriptor["evidence"]["captures"]["work"]["file"]
            with patch.object(h, "load_codex_capture", side_effect=lambda path:
                    capture if path.name == Path(work_file).name else original_loader(path)), \
                    patch.object(h, "load_canonical_bundle", return_value=bundle):
                full = h.real_session_evidence(descriptor, kind="volicord", cycle=1, repository_revision="0" * 40)
            for check in ("grounded_pre_work_repository_baseline", "engineering_choice_discovery",
                          "pre_write_materiality_work_authority", "appropriate_inquiry_outcome",
                          "meaningful_ordinary_changes", "source_grounded_checkpoint"):
                self.assertEqual(full["checks"][check], "passed", (check, full["checks"]))
            for missing in ("experiment", "reassessment", "binding", "checkpoint", "routing", "baseline"):
                with self.subTest(prototype=prototype, missing=missing):
                    changed = replace(capture,
                        commands=tuple(c for c in capture.commands if missing != "experiment" or c.execution_identity != "scratch-experiment"),
                        tool_calls=tuple(c for c in capture.tool_calls if not (
                            missing == "reassessment" and c.arguments.get("action") == "revise"
                            or missing == "binding" and c.arguments.get("action") == "inspect"
                            or missing == "checkpoint" and c.operation == "checkpoint_record"
                            or missing == "routing" and c.arguments.get("action") == "record" and c.operation == "materiality_review")))
                    self.assertFalse(h.work_blocker_behavior_observations(changed, descriptor["behavior_class"],
                        None if missing == "baseline" else baseline, None)[0])

    def test_absent_write_is_not_an_exploratory_pass(self):
        descriptor, capture, _ = self.fixture("research_or_no_question")
        capture = replace(capture, path_observations=())
        self.assertFalse(h.work_blocker_behavior_observations(capture, "exploratory_uncertainty",
            capture.successful_calls("repository_analyze")[0], None)[0])

    def test_no_write_unknown_experiment_remains_evidence_indeterminate(self):
        descriptor, capture, bundle = self.no_write_exploration()
        capture = replace(capture, commands=tuple(replace(c, exit_code=None,
            termination=None, evidence_state="indeterminate") if c.execution_identity == "scratch-experiment" else c
            for c in capture.commands))
        baseline = capture.successful_calls("repository_analyze")[0]
        evidence = h.exploratory_no_write_evidence(capture, baseline)
        self.assertFalse(evidence["qualified"])
        self.assertEqual(evidence["state"], "indeterminate")
        self.assertFalse(h.work_blocker_behavior_observations(capture, "exploratory_uncertainty", baseline, None)[0])
        attribution = h.work_evidence_transport_attribution(capture, ["behavior_class_evidence"],
            exploration_issues=evidence["issues"])
        self.assertEqual(attribution["affected_checks"], ["behavior_class_evidence"])
        original_loader = h.load_codex_capture
        work_file = descriptor["evidence"]["captures"]["work"]["file"]
        with patch.object(h, "load_codex_capture", side_effect=lambda path:
                capture if path.name == Path(work_file).name else original_loader(path)), \
                patch.object(h, "load_canonical_bundle", return_value=bundle):
            full = h.real_session_evidence(descriptor, kind="volicord", cycle=1, repository_revision="0" * 40)
        self.assertEqual(full["checks"]["appropriate_inquiry_outcome"], "partial")
        self.assertEqual(full["checks"]["pre_write_materiality_work_authority"], "partial")

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

    @staticmethod
    def settle_from_decision(arguments):
        arguments = deepcopy(arguments)
        for judgment in arguments["judgments"]:
            if judgment["disposition"] == "unresolved_user_owned_outcome":
                judgment["disposition"] = "settled_authority"
                judgment["decision_ids"] = [judgment.pop("resolution_decision_id")]
                judgment["additional_source_ids"] = ["02" * 16]
                for account in judgment["alternative_accounting"]:
                    account["source_ids"] = ["02" * 16]
                judgment.update(authority_source_evidence=[{"source_id": "02" * 16,
                    "role": {"kind": "applicable_decision", "decision_id": judgment["decision_ids"][0]},
                    "rationale": "The exact current-host response selects the observable outcome."}],
                    authority_coverage="The exact disclosed material outcome",
                    unique_outcome_rationale="The user's explicit answer settles the credible alternatives.")
        return arguments

    def settled_same_review(self, behavior="explicit_user_owned_decision"):
        descriptor, capture, bundle = self.fixture(behavior)
        capture = replace(capture, tool_calls=tuple(replace(c, arguments=self.settle_from_decision(c.arguments))
            if c.operation == "materiality_review" and c.arguments.get("action") == "revise"
            else c for c in capture.tool_calls))
        return descriptor, capture, bundle

    def settled_rediscovery(self):
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
        return descriptor, capture, bundle

    def test_resolved_historical_question_does_not_block_settled_rediscovery(self):
        descriptor, capture, bundle = self.settled_rediscovery()
        self.assertTrue(self.observe(descriptor, capture))
        self.assertTrue(self.facts(descriptor, capture, bundle)[0])

    def refreshed_rediscovery(self):
        descriptor, capture, bundle = self.settled_rediscovery()
        baseline = capture.successful_calls("repository_analyze")[0]
        discovery = next(c for c in capture.tool_calls if c.call_id == "rediscovery")
        refreshed = replace(baseline, call_id="refreshed-baseline",
            sequence=discovery.sequence - 20, completion_sequence=discovery.sequence - 10,
            result={**baseline.result, "analysis_snapshot_id": "bf" * 32})
        calls = []
        for call in capture.tool_calls:
            if call.call_id in {"rediscovery", "settled-review"} or (
                call.operation == "materiality_review" and call.arguments.get("action") == "inspect"
            ):
                call = replace(call,
                    arguments=json.loads(json.dumps(call.arguments).replace(baseline.result["analysis_snapshot_id"], "bf" * 32)),
                    result=json.loads(json.dumps(call.result).replace(baseline.result["analysis_snapshot_id"], "bf" * 32)))
            calls.append(call)
        return descriptor, replace(capture, tool_calls=tuple(sorted((*calls, refreshed), key=lambda c: c.sequence))), bundle, refreshed

    def test_historical_resolution_survives_baseline_refresh(self):
        descriptor, capture, bundle, baseline = self.refreshed_rediscovery()
        self.assertTrue(self.observe(descriptor, capture, baseline))
        self.assertTrue(self.facts(descriptor, capture, bundle, baseline)[0])

    def test_refreshed_history_requires_own_baseline_identity_and_chronology(self):
        descriptor, capture, bundle, baseline = self.refreshed_rediscovery()
        old = capture.successful_calls("repository_analyze")[0]
        origin = next(c for c in capture.calls("materiality_review") if c.arguments.get("action") == "record")
        for replacement in (None,
            replace(old, completion_sequence=origin.sequence + 1),
            replace(old, arguments={**old.arguments, "project_id": "ff" * 16}),
            replace(old, result={**old.result, "project_id": "ff" * 16}),
            replace(old, result={**old.result, "analysis_snapshot_id": "ff" * 32})):
            changed = replace(capture, tool_calls=tuple(replacement if c is old else c
                for c in capture.tool_calls if c is not old or replacement is not None))
            self.assertFalse(self.observe(descriptor, changed, baseline))
            self.assertFalse(self.facts(descriptor, changed, bundle, baseline)[0])

    def test_refresh_does_not_resolve_history_or_supply_current_authority(self):
        descriptor, capture, bundle, baseline = self.refreshed_rediscovery()
        for operation, action in (("decision_record", None), ("materiality_review", "revise"),
                                  ("materiality_review", "inspect")):
            changed = replace(capture, tool_calls=tuple(c for c in capture.tool_calls
                if not (c.operation == operation and (action is None or c.arguments.get("action") == action))))
            self.assertFalse(self.observe(descriptor, changed, baseline))
            self.assertFalse(self.facts(descriptor, changed, bundle, baseline)[0])
        origin = next(c for c in capture.calls("materiality_review") if c.arguments.get("action") == "record")
        for field in ("goal_context_id", "review_candidate_id"):
            changed = replace(capture, tool_calls=tuple(replace(c, result={**c.result, field: "ff" * 16})
                if c is origin else c for c in capture.tool_calls))
            self.assertFalse(self.observe(descriptor, changed, baseline))
            self.assertFalse(self.facts(descriptor, changed, bundle, baseline)[0])

    def test_same_review_decision_settlement_passes_both_evaluators(self):
        for behavior in ("explicit_user_owned_decision", "hidden_user_owned_decision"):
            with self.subTest(behavior=behavior):
                descriptor, capture, bundle = self.settled_same_review(behavior)
                self.assertEqual(sum(c.arguments.get("action") == "record"
                    for c in capture.calls("materiality_review")), 1)
                self.assertEqual(len(capture.calls("decision_record")), 1)
                self.assertTrue(self.observe(descriptor, capture))
                self.assertTrue(self.facts(descriptor, capture, bundle)[0])

    def test_same_review_settlement_in_full_session_evaluation(self):
        descriptor, _, _ = self.fixture("explicit_user_owned_decision")
        path = self.root / descriptor["evidence"]["captures"]["work"]["file"]
        events = [json.loads(line) for line in path.read_text().splitlines()]
        for event in events:
            payload = event.get("payload", {})
            if (payload.get("type") == "custom_tool_call"
                and "materiality-revision-call" in payload.get("call_id", "")):
                # Keep the real fixture transport envelope; change only the Product judgment variant.
                wrapper = h.parse_mcp_wrapper(payload["input"])
                self.assertIsNotNone(wrapper)
                arguments = self.settle_from_decision(wrapper.arguments)
                payload["input"] = payload["input"].replace(
                    json.dumps(wrapper.arguments, separators=(",", ":")),
                    json.dumps(arguments, separators=(",", ":")), 1)
                for completion in events:
                    output = completion.get("payload", {})
                    if (output.get("type") == "mcp_tool_call_end"
                        and output.get("call_id") == f"exec-{payload['call_id']}"):
                        output["invocation"]["arguments"] = arguments
        path.write_text("".join(json.dumps(event) + "\n" for event in events))
        descriptor["evidence"]["captures"]["work"]["sha256"] = h.sha256(path)
        result = h.real_session_evidence(descriptor, kind="volicord", cycle=1, repository_revision="0" * 40)
        for check in ("pre_write_materiality_work_authority", "appropriate_inquiry_outcome",
            "recorded_user_owned_authority"):
            self.assertEqual(result["checks"][check], "passed", result)

    def test_same_review_and_history_reject_false_identity_and_late_resolution(self):
        for factory in (self.settled_same_review, self.settled_rediscovery):
            descriptor, capture, bundle = factory()
            revision = next(c for c in capture.tool_calls if c.arguments.get("action") == "revise")
            decision = capture.calls("decision_record")[0]
            submit = next(c for c in capture.tool_calls if c.arguments.get("action") == "submit_question_from_materiality")
            promote = next(c for c in capture.tool_calls if c.arguments.get("action") == "promote_question")
            frontier = capture.calls("inquiry_frontier")[0]
            first_write = min(x.sequence for x in h.meaningful_work_path_observations(capture))
            cases = [
                ("missing Decision", decision, None),
                ("wrong Question", decision, replace(decision, arguments={**decision.arguments, "question_id": "ff" * 16})),
                ("wrong revision", decision, replace(decision, arguments={**decision.arguments, "question_revision": 2})),
                ("wrong receipt", decision, replace(decision, arguments={**decision.arguments, "presentation_receipt_id": "ff" * 16})),
                ("wrong Review", submit, replace(submit, arguments={**submit.arguments, "review_candidate_id": "ff" * 16})),
                ("wrong dimension", submit, replace(submit, arguments={**submit.arguments, "dimension_id": "unrelated"})),
                ("wrong returned dimension", submit, replace(submit, result={**submit.result, "dimension_id": "unrelated"})),
                ("wrong Project", decision, replace(decision, arguments={**decision.arguments, "project_id": "ff" * 16})),
                ("late Decision completion", decision, replace(decision, completion_sequence=revision.sequence + 1)),
                ("post-write Decision", decision, replace(decision, sequence=first_write + 1, completion_sequence=first_write + 2)),
                ("late revision", revision, replace(revision, sequence=first_write + 1, completion_sequence=first_write + 2)),
                ("overlapping promotion", promote, replace(promote, completion_sequence=frontier.sequence + 1)),
                ("overlapping submission", submit, replace(submit, completion_sequence=promote.sequence + 1)),
            ]
            for label, original, replacement in cases:
                with self.subTest(lifecycle=factory.__name__, case=label):
                    calls = tuple(replacement if c is original else c for c in capture.tool_calls
                        if c is not original or replacement is not None)
                    changed = replace(capture, tool_calls=calls)
                    self.assertFalse(self.observe(descriptor, changed))
                    self.assertFalse(self.facts(descriptor, changed, bundle)[0])

    def test_same_review_and_history_require_canonical_scope_and_provenance(self):
        for factory in (self.settled_same_review, self.settled_rediscovery):
            descriptor, capture, bundle = factory()
            for table, mutation in (
                ("decisions", {"question_revision": 2}),
                ("decisions", {"question_id": "ff" * 16}),
                ("decisions", {"id": "ff" * 16}),
                ("decisions", {"user_authority": "agent"}),
                ("questions", {"revision": 2}),
                ("question_revisions", {"material_scope": "0000000000000000"}),
                ("question_revisions", {"materiality": "not_material"}),
                ("question_response_sources", {"source_id": "ff" * 16}),
                ("question_decision_history_witnesses", {"root_decision_id": "ff" * 16}),
            ):
                with self.subTest(lifecycle=factory.__name__, table=table, mutation=mutation):
                    changed = replace(bundle, tables={**bundle.tables, table: tuple(
                        {**row, **mutation} for row in bundle.rows(table))})
                    self.assertFalse(self.facts(descriptor, capture, changed)[0])

    def test_later_revision_cannot_hide_pre_decision_settlement(self):
        descriptor, capture, bundle = self.settled_same_review()
        revision = next(c for c in capture.tool_calls if c.arguments.get("action") == "revise")
        decision = capture.calls("decision_record")[0]
        early = replace(revision, call_id="premature-settlement", sequence=decision.sequence - 20,
            completion_sequence=decision.sequence - 10)
        calls = tuple(replace(c, result={**c.result, "review_revision": 3}) if c is revision else c
            for c in capture.tool_calls)
        capture = replace(capture, tool_calls=tuple(sorted((*calls, early), key=lambda c: c.sequence)))
        self.assertFalse(self.observe(descriptor, capture))
        self.assertFalse(self.facts(descriptor, capture, bundle)[0])

    def test_settlement_rejects_stale_response_and_inapplicable_decision(self):
        descriptor, capture, bundle = self.settled_same_review()
        def strings(values):
            return (len(values).to_bytes(8, "big") + b"".join(
                len(value.encode()).to_bytes(8, "big") + value.encode() for value in values)).hex()
        for field in ("paths", "components", "work_contexts"):
            with self.subTest(field=field):
                changed = replace(bundle, tables={**bundle.tables, "decisions": tuple(
                    {**row, f"applicability_{field}": strings(["unrelated"])} for row in bundle.rows("decisions"))})
                self.assertFalse(self.facts(descriptor, capture, changed)[0])
        binding = next(c for c in capture.calls("materiality_review") if c.arguments.get("action") == "inspect")
        scoped = replace(bundle, tables={**bundle.tables, "decisions": tuple({**row,
            **{f"applicability_{field}": strings(binding.arguments.get(field, []))
                for field in ("paths", "components", "work_contexts")}} for row in bundle.rows("decisions"))})
        self.assertTrue(self.facts(descriptor, capture, scoped)[0])
        for state in ("stale", "unknown", "unavailable"):
            changed = replace(bundle, tables={**bundle.tables, "sources": tuple(
                {**row, "availability": state} if row["id"] == "02" * 16 else row for row in bundle.rows("sources"))})
            self.assertFalse(self.facts(descriptor, capture, changed)[0])

    def test_same_review_requires_exact_pre_write_scope_binding(self):
        descriptor, capture, bundle = self.settled_same_review()
        binding = next(c for c in capture.calls("materiality_review") if c.arguments.get("action") == "inspect")
        first_write = min(x.sequence for x in h.meaningful_work_path_observations(capture))
        for replacement in (None,
            replace(binding, completion_sequence=first_write + 1),
            replace(binding, arguments={**binding.arguments, "review_candidate_id": "ff" * 16})):
            changed = replace(capture, tool_calls=tuple(replacement if c is binding else c
                for c in capture.tool_calls if c is not binding or replacement is not None))
            self.assertFalse(self.observe(descriptor, changed))
            self.assertFalse(self.facts(descriptor, changed, bundle)[0])

    def test_settlement_does_not_hide_independent_authority_behind_behavior_labels(self):
        for behavior in ("delegated_implementation_choice", "research_or_no_question", "exploratory_uncertainty"):
            descriptor, capture, bundle = self.settled_same_review()
            descriptor["behavior_class"] = behavior
            self.assertTrue(self.observe(descriptor, capture))
            self.assertTrue(self.facts(descriptor, capture, bundle)[0])
            changed = replace(capture, tool_calls=tuple(c for c in capture.tool_calls if c.operation != "decision_record"))
            self.assertFalse(self.observe(descriptor, changed))
            self.assertFalse(self.facts(descriptor, changed, bundle)[0])
        descriptor, capture, bundle = self.settled_same_review()
        calls = deepcopy(capture.tool_calls)
        for call in calls:
            if call.arguments.get("action") == "revise":
                for judgment in call.arguments["judgments"]:
                    judgment["learning_authority"]["independent_user_authority"] = False
        self.assertFalse(self.facts(descriptor, replace(capture, tool_calls=calls), bundle)[0])

    def test_settlement_requires_current_uncontested_decision(self):
        descriptor, capture, bundle = self.settled_same_review()
        for table, row in (
            ("canonical_relations", {"from_kind": "decision", "from_id": "ff" * 16,
                "to_kind": "decision", "to_id": "07" * 16, "relation_kind": "supersedes"}),
            ("canonical_relations", {"from_kind": "decision", "from_id": "07" * 16,
                "to_kind": "decision", "to_id": "ff" * 16, "relation_kind": "contradicts"}),
            ("review_due", {"decision_id": "07" * 16}),
        ):
            changed = replace(bundle, tables={**bundle.tables, table: (
                *bundle.rows(table), {**row, "project_id": bundle.project_id})})
            self.assertFalse(self.facts(descriptor, capture, changed)[0])

    def test_later_review_origin_cannot_be_treated_as_historical(self):
        descriptor, capture, bundle = self.settled_rediscovery()
        current = next(c for c in capture.tool_calls if c.call_id == "settled-review")
        origin = next(c for c in capture.tool_calls if c.operation == "materiality_review"
            and c.arguments.get("action") == "record" and c is not current)
        capture = replace(capture, tool_calls=tuple(replace(c, completion_sequence=current.sequence + 1)
            if c is origin else c for c in capture.tool_calls))
        self.assertFalse(self.observe(descriptor, capture))
        self.assertFalse(self.facts(descriptor, capture, bundle)[0])

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
