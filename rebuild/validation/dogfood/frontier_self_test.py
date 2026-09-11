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

    def equivalent_hidden_rebase(self):
        _, capture, _ = self.fixture("hidden_user_owned_decision")
        baseline = capture.successful_calls("repository_analyze")[0]
        discovery = capture.successful_calls("engineering_choice_discovery")[0]
        old = replace(discovery, sequence=1700, completion_sequence=1710, call_id="origin-discovery",
            result={**discovery.result, "discovery_candidate_id": "ad" * 16})
        new_id = "cd" * 32
        def rebase(value):
            if isinstance(value, dict):
                return {k: rebase(v) for k, v in value.items()}
            if isinstance(value, list):
                return [rebase(v) for v in value]
            return new_id if value == baseline.result["analysis_snapshot_id"] else value
        new_baseline = replace(baseline, sequence=1750, completion_sequence=1760, call_id="rebase-analysis",
            result={**baseline.result, "analysis_snapshot_id": new_id})
        calls = tuple(replace(c, arguments=rebase(c.arguments), result=rebase(c.result))
                      if c.sequence >= discovery.sequence else c for c in capture.tool_calls)
        capture = replace(capture, tool_calls=tuple(sorted((*calls, old, new_baseline), key=lambda c: c.sequence)))
        return capture, new_baseline, old

    def test_hidden_origin_survives_only_equivalent_rebase(self):
        capture, baseline, origin = self.equivalent_hidden_rebase()
        frontier = min(c.sequence for c in h.meaningful_work_path_observations(capture))
        evidence = h.hidden_investigation_evidence(capture, baseline, frontier)
        self.assertEqual(evidence["state"], "complete")
        self.assertEqual(evidence["sequences"], [1500])
        current = capture.successful_calls("engineering_choice_discovery")[-1]
        for change in ("new_choice", "new_dimension", "new_alternative", "consequence", "source", "snapshot", "ambiguous", "mutation"):
            args = deepcopy(current.arguments)
            changed_baseline = baseline
            extra = ()
            if change == "new_choice":
                args["choices"][0]["choice_id"] = "new-material-choice"
            elif change == "new_dimension":
                args["choices"][0]["effect_categories"].append("security")
            elif change == "new_alternative":
                args["choices"][0]["alternatives"][0]["alternative_id"] = "new-outcome"
            elif change == "consequence":
                args["choices"][0]["alternatives"][0]["technical_consequences"] = ["A materially different result."]
            elif change in {"source", "snapshot"}:
                key = "repository_source_id" if change == "source" else "repository_snapshot_id"
                changed_baseline = replace(baseline, result={**baseline.result, key: "ef" * (16 if change == "source" else 32)})
            elif change == "ambiguous":
                extra = (replace(origin, call_id="ambiguous-origin", sequence=1701),)
            changed = replace(capture, tool_calls=tuple(replace(c, arguments=args) if c is current
                else changed_baseline if c is baseline else c for c in capture.tool_calls) + extra,
                path_observations=(*capture.path_observations,
                    replace(capture.path_observations[0], sequence=1730)) if change == "mutation" else capture.path_observations)
            self.assertFalse(h.equivalent_discovery_basis(changed, origin,
                next(c for c in changed.tool_calls if c.call_id == current.call_id)) if change != "ambiguous"
                else h.originating_discovery_boundary(changed, current, baseline), change)
            # Same prose cannot rescue missing current investigation on a changed basis.
            if change != "new_choice":
                self.assertEqual(h.hidden_investigation_evidence(changed, changed_baseline, frontier)["state"], "missing", change)
        changed_baseline = replace(baseline, result={**baseline.result, "repository_snapshot_id": "ef" * 32})
        renewed = replace(capture.commands[1], sequence=1770, completion_sequence=1780)
        changed = replace(capture, commands=(*capture.commands, renewed),
            tool_calls=tuple(changed_baseline if c is baseline else c for c in capture.tool_calls))
        self.assertEqual(h.hidden_investigation_evidence(changed, changed_baseline, frontier)["state"], "complete")

    def test_equivalent_rebase_cannot_launder_uninvestigated_origin(self):
        capture, baseline, origin = self.equivalent_hidden_rebase()
        too_early = replace(origin, sequence=1400, completion_sequence=1410)
        # Even a later inspection cannot retroactively repair first discovery.
        later = replace(capture.commands[1], sequence=1770, completion_sequence=1780)
        capture = replace(capture, commands=(*capture.commands, later),
            tool_calls=tuple(sorted((too_early if c is origin else c for c in capture.tool_calls), key=lambda c: c.sequence)))
        frontier = min(c.sequence for c in h.meaningful_work_path_observations(capture))
        self.assertEqual(h.hidden_investigation_evidence(capture, baseline, frontier)["state"], "missing")

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
                    replace(c, exit_code=None, termination=None, evidence_state="indeterminate", output="") if state == "indeterminate" else
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
                        "behavior_contract" if state == "missing" else "evidence")
                    self.assertEqual(result["product_failed_checks"],
                        list(h.HIDDEN_INVESTIGATION_CHECKS) if state == "missing" else [])
                    original_loader = h.load_codex_capture
                    work_file = descriptor["evidence"]["captures"]["work"]["file"]
                    with patch.object(h, "load_codex_capture", side_effect=lambda path:
                            changed if path.name == Path(work_file).name else original_loader(path)):
                        full = h.real_session_evidence(descriptor, kind="volicord", cycle=1,
                            repository_revision=head)
                    self.assertEqual(full["checks"]["appropriate_inquiry_outcome"], "passed")
                    self.assertEqual(full["checks"]["hidden_material_discovery_order"],
                        "failed" if state == "missing" else "partial")

    def test_bounded_repository_observation_without_numeric_exit(self):
        _, capture, _ = self.fixture("hidden_user_owned_decision")
        baseline = capture.successful_calls("repository_analyze")[0]
        discovery = capture.successful_calls("engineering_choice_discovery")[0]
        frontier = min(c.sequence for c in h.meaningful_work_path_observations(capture))
        command = replace(capture.commands[0], sequence=baseline.completion_sequence + 1,
            completion_sequence=discovery.sequence - 1,
            parsed_command={"cmd": "rg -n 'Tags|generate_tags' src/main.rs src/tags.rs | head -120",
                "workdir": str(capture.cwd)}, exit_code=None, termination=None, evidence_state="indeterminate",
            output="src/main.rs:12:struct Tags { output_json: bool }\nsrc/tags.rs:32:fn generate_tags(source: &str) -> Tags {", output_was_empty=False)
        for output, exit_code, evidence_state, expected in (
            (command.output, None, "indeterminate", "complete"),
            ("", 0, "completed", "complete"),
            ("", None, "indeterminate", "indeterminate"),
            ("The repository was investigated successfully.", None, "indeterminate", "indeterminate"),
        ):
            changed = replace(capture, commands=(replace(command, output=output, exit_code=exit_code,
                evidence_state=evidence_state, termination="exited" if exit_code is not None else None),))
            self.assertEqual(h.hidden_investigation_evidence(changed, baseline, frontier)["state"], expected)
        self.assertEqual(h.hidden_investigation_evidence(replace(capture, commands=()), baseline, frontier)["state"], "missing")

    def test_structured_repository_understanding_remains_investigation(self):
        _, capture, _ = self.fixture("hidden_user_owned_decision")
        baseline = capture.successful_calls("repository_analyze")[0]
        discovery = capture.successful_calls("engineering_choice_discovery")[0]
        call = replace(baseline, operation="repository_understanding", call_id="understanding",
            sequence=baseline.completion_sequence + 1, completion_sequence=discovery.sequence - 1,
            result={"health": "available", "overview": {}, "repository_map": {}, "read_only": True})
        capture = replace(capture, commands=(), tool_calls=(*capture.tool_calls, call))
        result = h.repository_investigation_evidence(capture,
            after_sequence=baseline.completion_sequence, before_sequence=discovery.sequence)
        self.assertEqual(result["state"], "complete")
        self.assertEqual(result["sequences"], [call.sequence])

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

    def test_literal_interpreter_scratch_has_only_exploration_authority(self):
        _, capture, _ = self.no_write_exploration()
        baseline = capture.successful_calls("repository_analyze")[0]
        experiment = next(c for c in capture.commands if c.execution_identity == "scratch-experiment")
        for cmd in ("python /tmp/x.py", "python3 /tmp/x.py", "/usr/bin/python3 /tmp/x.py",
                    "env NAME=VALUE PYTHONPATH=src python3 /tmp/x.py",
                    "PYTHONPATH=src python3 /tmp/x.py", "python3 /tmp/probe_self_test.py"):
            command = replace(experiment, parsed_command={"cmd": cmd, "workdir": str(capture.cwd)})
            changed = replace(capture, commands=tuple(command if c is experiment else c for c in capture.commands))
            self.assertEqual(h.dogfood_command_role(command.parsed_command, capture.cwd), "exploration")
            self.assertTrue(h.exploratory_no_write_evidence(changed, baseline)["qualified"], cmd)
            self.assertFalse(h.command_is_repository_inspection(command.parsed_command))
            self.assertFalse(h.meaningful_resume_validation(replace(changed, commands=(command,)), 0)["qualified"])
            for fields in ({"exit_code": 1}, {"exit_code": True}, {"exit_code": None},
                           {"termination": None}, {"evidence_state": "indeterminate"}):
                broken = replace(changed, commands=tuple(replace(c, **fields) if c is command else c
                                                        for c in changed.commands))
                self.assertFalse(h.exploratory_no_write_evidence(broken, baseline)["qualified"])
        for cmd in ("python3 scripts/x.py", "python3 /tmp/../etc/x.py", "python3 /var/x.py",
                    "python3 /tmp/x.py && true", "python3 /tmp/x.py;", "python3 /tmp/*.py",
                    "python3 -c 'print(1)'", "python3 -m arbitrary", "env -S 'python3 /tmp/x.py'",
                    "env NAME=$(echo value) python3 /tmp/x.py", "/custom/python3 /tmp/x.py",
                    "unknown /tmp/x.py", "python3 /tmp/x.py > /tmp/result"):
            self.assertFalse(h.interpreter_scratch_experiment({"cmd": cmd}, capture.cwd), cmd)
        self.assertFalse(h.interpreter_scratch_experiment({"cmd": "python3 /tmp/repo/x.py"}, Path("/tmp/repo")))

    def test_rematerialized_exploration_retains_executed_research(self):
        _, capture, _ = self.no_write_exploration()
        baseline = capture.successful_calls("repository_analyze")[0]
        discovery = capture.successful_calls("engineering_choice_discovery")[0]
        record = next(c for c in capture.successful_calls("materiality_review") if c.arguments.get("action") == "record")
        revision = next(c for c in capture.successful_calls("materiality_review") if c.arguments.get("action") == "revise")
        inspect = next(c for c in capture.successful_calls("materiality_review") if c.arguments.get("action") == "inspect")
        checkpoint = h.terminal_checkpoint_call(capture)
        experiment = next(c for c in capture.commands if c.execution_identity == "scratch-experiment")
        experiment = replace(experiment, parsed_command={"cmd": "python3 /tmp/probe.py"})
        def bind(value):
            if isinstance(value, dict):
                return {k: bind(v) for k, v in value.items()}
            if isinstance(value, list):
                return [bind(v) for v in value]
            if value == record.result["review_candidate_id"]:
                return "ef" * 16
            if value == discovery.result["discovery_candidate_id"]:
                return "ed" * 16
            return value
        choices = deepcopy(discovery.arguments["choices"])
        for choice in choices:
            choice["evidence_state"] = "sufficient"
        rebound_discovery = replace(discovery, call_id="resolved-discovery", sequence=checkpoint.sequence - 80,
            completion_sequence=checkpoint.sequence - 75,
            arguments={**discovery.arguments, "choices": choices}, result=bind({**discovery.result, "choices": choices}))
        rebound_record = replace(record, call_id="resolved-record", sequence=checkpoint.sequence - 60,
            completion_sequence=checkpoint.sequence - 55,
            arguments=bind({**record.arguments, "judgments": revision.arguments["judgments"]}),
            result=bind({**revision.result, "action": "record", "review_revision": 2}))
        rebound_inspect = replace(inspect, call_id="resolved-inspect", sequence=checkpoint.sequence - 40,
            completion_sequence=checkpoint.sequence - 35, arguments=bind(inspect.arguments), result=bind(inspect.result))
        claim = {"state": "passed", "command_invocation": "python3 /tmp/probe.py", "exit_code": 0, "termination": "exited"}
        checkpoint = replace(checkpoint, arguments={**checkpoint.arguments, "verification": [claim]})
        capture = replace(capture, commands=tuple(experiment if c.execution_identity == "scratch-experiment" else c for c in capture.commands),
            tool_calls=tuple(sorted((*[checkpoint if c.call_id == checkpoint.call_id else c for c in capture.tool_calls],
                rebound_discovery, rebound_record, rebound_inspect), key=lambda c: c.sequence)))
        repository_read = replace(experiment, sequence=baseline.completion_sequence + 10,
            completion_sequence=baseline.completion_sequence + 20, execution_identity="repository-read",
            parsed_command={"cmd": "cat src/lib.rs"})
        capture = replace(capture, commands=(repository_read, *capture.commands))
        evidence = h.exploratory_no_write_evidence(capture, baseline)
        self.assertTrue(evidence["qualified"], evidence)
        self.assertEqual(evidence["origin_discovery_sequence"], discovery.sequence)
        for missing in ("experiment", "resolution", "binding", "origin", "new_dimension", "new_source"):
            changed_discovery = rebound_discovery
            if missing in {"new_dimension", "new_source"}:
                changed_choices = deepcopy(choices)
                if missing == "new_dimension":
                    changed_choices[0]["effect_categories"].append("security")
                else:
                    changed_choices[0]["source_ids"] = ["ac" * 16]
                changed_discovery = replace(rebound_discovery, arguments={**rebound_discovery.arguments, "choices": changed_choices})
            changed = replace(capture,
                commands=tuple(c for c in capture.commands if missing != "experiment" or c is not experiment),
                tool_calls=tuple(changed_discovery if c is rebound_discovery else c for c in capture.tool_calls if not (
                    missing == "resolution" and c is revision or missing == "binding" and c is rebound_inspect
                    or missing == "origin" and c is discovery)))
            self.assertFalse(h.exploratory_no_write_evidence(changed, baseline)["qualified"], missing)

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

    def test_internal_host_session_preserves_withdrawal_and_learning_audit(self):
        descriptor, capture, bundle, _ = self.learning_revision("current_user_withdrawal")
        tables = deepcopy(bundle.tables)
        for source in tables["sources"]:
            if source["source_kind"] == "current_host_user_turn":
                source["detail_two"] = "internal-host-session"
        bundle = replace(bundle, tables=tables)
        self.assertTrue(self.facts(descriptor, capture, bundle)[0])
        begin = next(c for c in capture.calls("learning_deliberation") if c.arguments["action"] == "begin")
        arguments = (capture, bundle, "learning_deliberation",
            begin.arguments["review_candidate_id"], begin.arguments["dimension_id"],
            min(c.sequence for c in h.meaningful_work_path_observations(capture)),
            {"learning_participation": {"state": "active"}})
        self.assertTrue(h.learning_deliberation_facts(*arguments)[2])
        source = next(s for s in bundle.rows("sources") if s["source_kind"] == "current_host_user_turn")
        tables["decisions"] = (*tables["decisions"], {"id": "manufactured", "project_id": bundle.project_id,
            "user_turn_source_id": source["id"]})
        self.assertFalse(h.learning_deliberation_facts(*arguments)[2])

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
                    {"detail_two": None}, {"locator": "I still want to deliberate."}]
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

    def applicable_successor(self):
        descriptor, capture, bundle, baseline = self.refreshed_rediscovery()
        origin_revision = next(c for c in capture.calls("materiality_review") if c.arguments.get("action") == "revise")
        current = next(c for c in capture.tool_calls if c.call_id == "settled-review")
        arguments = {**current.arguments, "judgments": self.settle_from_decision(origin_revision.arguments)["judgments"]}
        origin_choices = capture.successful_calls("engineering_choice_discovery")[0].arguments["choices"]
        calls = []
        for c in capture.tool_calls:
            if c is current:
                c = replace(c, arguments=arguments)
            if c.call_id == "rediscovery":
                choices = deepcopy(c.arguments["choices"])
                for choice in choices:
                    choice["relationship"] = next(x["relationship"] for x in origin_choices if x["choice_id"] == choice["choice_id"])
                c = replace(c, arguments={**c.arguments, "choices": choices}, result={**c.result, "choices": choices})
            calls.append(c)
        capture = replace(capture, tool_calls=tuple(calls))
        return descriptor, capture, bundle, baseline

    def test_exact_successor_authority_traces_origin(self):
        descriptor, capture, bundle, baseline = self.applicable_successor()
        self.assertTrue(self.observe(descriptor, capture, baseline))
        self.assertTrue(self.facts(descriptor, capture, bundle, baseline)[0])
        # Current applicability must be checked at B's executable scope, even
        # though no executable binding was needed on the historical Review A.
        for table, rows in (
            ("review_due", ({"project_id": bundle.project_id, "decision_id": "07" * 16},)),
            ("decisions", tuple({**r, "applicability_paths": (bytes.fromhex("00000000000000010000000000000009") + b"unrelated").hex()} for r in bundle.rows("decisions"))),
        ):
            changed = replace(bundle, tables={**bundle.tables, table: rows})
            self.assertFalse(self.facts(descriptor, capture, changed, baseline)[0], table)

    def test_hidden_successor_investigation_belongs_to_origin(self):
        descriptor, capture, bundle, baseline = self.applicable_successor()
        original_baseline = capture.successful_calls("repository_analyze")[0]
        origin = capture.successful_calls("engineering_choice_discovery")[0]
        investigation = replace(capture.commands[0], sequence=original_baseline.completion_sequence + 1,
            completion_sequence=origin.sequence - 1, parsed_command={"cmd": "cat src/lib.rs"},
            exit_code=0, evidence_state="completed", termination="exited")
        capture = replace(capture, commands=(investigation,))
        first_write = min(x.sequence for x in h.meaningful_work_path_observations(capture))
        self.assertEqual(h.hidden_investigation_evidence(capture, baseline, first_write)["state"], "complete")
        # A new outcome cannot borrow the old Question's investigation.
        current = next(c for c in capture.tool_calls if c.call_id == "settled-review")
        arguments = deepcopy(current.arguments)
        arguments["judgments"][0]["decision_ids"] = ["ff" * 16]
        changed = replace(capture, tool_calls=tuple(replace(c, arguments=arguments) if c is current else c for c in capture.tool_calls))
        self.assertNotEqual(h.hidden_investigation_evidence(changed, baseline, first_write)["state"], "complete")
        self.assertFalse(self.observe(descriptor, changed, baseline))

    def test_successor_resolution_uses_exact_historical_question(self):
        descriptor, capture, bundle, baseline = self.applicable_successor()
        origin = next(c for c in capture.calls("materiality_review") if c.arguments.get("action") == "record")
        resolution = next(c for c in capture.calls("materiality_review") if c.arguments.get("action") == "revise")
        current = next(c for c in capture.tool_calls if c.call_id == "settled-review")
        revised = replace(resolution, call_id="successor-resolution", sequence=current.completion_sequence + 1,
            completion_sequence=current.completion_sequence + 2,
            arguments={**resolution.arguments, "review_candidate_id": current.result["review_candidate_id"]},
            result={**resolution.result, "review_candidate_id": current.result["review_candidate_id"],
                "baseline_analysis_snapshot_id": baseline.result["analysis_snapshot_id"]})
        pending = replace(current, arguments={**current.arguments, "judgments": origin.arguments["judgments"]},
            result={**current.result, "workflow": origin.result["workflow"]})
        calls = []
        for call in capture.tool_calls:
            if call is current:
                call = pending
            if call.operation == "materiality_review" and call.arguments.get("action") == "inspect":
                output = deepcopy(call.result)
                output["review_revision"] = 2
                output["executable_work_scope"]["authority_basis"]["review_revision"] = 2
                call = replace(call, result=output)
            calls.append(call)
        capture = replace(capture, tool_calls=tuple(sorted((*calls, revised), key=lambda c: c.sequence)))
        self.assertTrue(self.observe(descriptor, capture, baseline))
        facts = self.facts(descriptor, capture, bundle, baseline)
        self.assertTrue(facts[0])
        lifecycle = h.material_question_lifecycle_facts(capture, bundle, descriptor["behavior_class"], {},
            baseline, facts[1], facts[3], h.decision_facts(capture, bundle)[-1])
        self.assertTrue(lifecycle[0])

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

    def test_unpromoted_historical_question_is_not_resolved_by_refresh(self):
        descriptor, capture, bundle, baseline = self.refreshed_rediscovery()
        changed = replace(capture, tool_calls=tuple(c for c in capture.tool_calls
            if c.operation not in {"inquiry_frontier", "decision_record"}
            and not (c.operation == "candidate_manage" and c.arguments.get("action") != "submit_question_from_materiality")))
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

    def test_answered_question_repeat_is_distinct_from_lifecycle(self):
        descriptor, capture, bundle = self.fixture("explicit_user_owned_decision")
        frontier = capture.calls("inquiry_frontier")[0]
        decision = capture.calls("decision_record")[0]
        user = capture.turn_for_call(decision)
        repeat = replace(frontier, call_id="repeat-question", sequence=user.sequence + 1,
            completion_sequence=user.sequence + 2, turn_id=user.turn_id,
            result=deepcopy(frontier.result))
        repeat.result["questions"][0]["presentation_receipt_id"] = "fresh-receipt"
        decision = replace(decision, arguments={**decision.arguments, "presentation_receipt_id": "fresh-receipt"})
        capture = replace(capture, tool_calls=tuple(sorted((*(decision if c.operation == "decision_record" else c
            for c in capture.tool_calls), repeat), key=lambda c: c.sequence)))
        self.assertTrue(self.observe(descriptor, capture))
        self.assertTrue(h.decision_facts(capture, bundle)[0])
        self.assertEqual(len(h.unnecessary_question_repetitions(capture)), 1)
        with patch.object(h, "cycle_descriptor_errors", return_value=[]):
            blocked = h.build_work_blocker_result("0" * 40, descriptor, "0" * 64, capture)
        self.assertEqual(blocked["failed_checks"], ["unnecessary_question_repetition"])
        self.assertEqual(blocked["failure_attribution"]["domain"], "behavior_contract")
        for text in ("Please explain the alternatives first.", "Either option might work."):
            changed = replace(capture, user_turns=tuple(replace(t, text=text) if t is user else t for t in capture.user_turns))
            self.assertEqual(h.unnecessary_question_repetitions(changed), [])
        changed_result = deepcopy(repeat.result)
        changed_result["questions"][0]["revision"] += 1
        changed = replace(capture, tool_calls=tuple(replace(c, result=changed_result) if c is repeat else c for c in capture.tool_calls))
        self.assertEqual(h.unnecessary_question_repetitions(changed), [])
        failed = replace(decision, call_id="stale-receipt-attempt", sequence=user.sequence,
            completion_sequence=repeat.sequence - 1, outcome="failed", result={"error": "stale_presentation"})
        self.assertEqual(h.unnecessary_question_repetitions(replace(capture,
            tool_calls=tuple(sorted((*capture.tool_calls, failed), key=lambda c: c.sequence)))), [])

    def test_exact_option_answer_repeat_before_later_clarification(self):
        descriptor, capture, bundle = self.fixture("explicit_user_owned_decision")
        frontier = capture.calls("inquiry_frontier")[0]
        decision = capture.calls("decision_record")[0]
        user = capture.turn_for_call(decision)
        result = deepcopy(frontier.result)
        result["questions"][0]["alternatives"] = [
            {"key": "concise", "label": "Concise output", "consequence": "Details remain in diagnostics."},
            {"key": "verbose", "label": "Verbose output", "consequence": "Details appear by default."}]
        frontier = replace(frontier, result=result)
        answer = replace(user, sequence=frontier.completion_sequence + 1,
            turn_id="initial-option-answer", text="Concise output  \n")
        repeat = replace(frontier, call_id="repeat-before-clarification", sequence=answer.sequence + 1,
            completion_sequence=answer.sequence + 2, turn_id=answer.turn_id,
            result=deepcopy(result))
        repeat.result["questions"][0]["presentation_receipt_id"] = "fresh-receipt"
        decision = replace(decision, arguments={**decision.arguments, "presentation_receipt_id": "fresh-receipt"})
        capture = replace(capture, user_turns=tuple(sorted((*capture.user_turns, answer), key=lambda t: t.sequence)),
            tool_calls=tuple(sorted((*(frontier if c.operation == "inquiry_frontier" else
                decision if c.operation == "decision_record" else c for c in capture.tool_calls), repeat), key=lambda c: c.sequence)))
        self.assertEqual(len(h.unnecessary_question_repetitions(capture)), 1)
        self.assertTrue(h.decision_facts(capture, bundle)[0])
        for text, expected in (("concise", 1), ("Explain Concise output", 0),
                ("Concise output or Verbose output", 0), ("Verbose output", 0), ("concise?", 0)):
            changed = replace(capture, user_turns=tuple(replace(t, text=text) if t is answer else t
                for t in capture.user_turns))
            self.assertEqual(len(h.unnecessary_question_repetitions(changed)), expected, text)
        ambiguous = deepcopy(result)
        ambiguous["questions"][0]["alternatives"][1]["label"] = "Concise output"
        ambiguous_repeat = deepcopy(ambiguous)
        ambiguous_repeat["questions"][0]["presentation_receipt_id"] = "fresh-receipt"
        changed = replace(capture, tool_calls=tuple(replace(c, result=ambiguous)
            if c is frontier else replace(c, result=ambiguous_repeat) if c is repeat else c
            for c in capture.tool_calls))
        self.assertEqual(h.unnecessary_question_repetitions(changed), [])

    def test_missing_presentation_preserves_independent_authority_evidence(self):
        descriptor, capture, bundle = self.fixture("explicit_user_owned_decision")
        capture = replace(capture, tool_calls=tuple(c for c in capture.tool_calls if c.operation != "inquiry_frontier"))
        facts = self.facts(descriptor, capture, bundle)
        self.assertTrue(facts[0])
        decisions = h.decision_facts(capture, bundle)
        self.assertTrue(decisions[0])
        lifecycle = h.material_question_lifecycle_facts(capture, bundle, descriptor["behavior_class"], {},
            capture.successful_calls("repository_analyze")[0], facts[1], facts[3], decisions[-1])
        self.assertFalse(lifecycle[0])
        self.assertFalse(self.observe(descriptor, capture))

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
