"""Qualification authority negative controls; no naturalistic passage claims."""
import copy
import json
from pathlib import Path
from types import SimpleNamespace
import shutil
import tempfile
import unittest
from unittest.mock import patch

import machine_findings as m
import harness
import qualitative_review as review
import qualitative_review_self_test as fixtures
import qualification_policy as policy


def evaluation():
    works = [
        {"repository_class": repository_class, "work": work,
         "work_slot_id": work_slot_id, "resume_pair": resume_pair, "findings": [],
         }
        for repository_class, work, work_slot_id, resume_pair in sorted(policy.EXPECTED_WORKS)
    ]
    journeys = []
    for repository_class, journey_id, work_slot_ids in sorted(policy.EXPECTED_JOURNEYS):
        findings = [m.finding(rule, "confirmed_pass", {"journey_id": journey_id})
                    for rule in sorted(policy.STRUCTURAL_RULES)]
        journeys.append({"repository_class": repository_class, "journey_id": journey_id,
            "work_slot_ids": list(work_slot_ids), "findings": findings})
    return {"run_id": "a" * 64, "works": works, "journeys": journeys,
        "coverage": copy.deepcopy(policy.TOPOLOGY)}


class DefinitionDependencyTests(unittest.TestCase):
    def test_definition_is_independent_of_v11_leaf_names_and_count(self):
        for leaves in (('new_v11_only_leaf',), ('renamed_technical_leaf', 'another_new_leaf')):
            with self.subTest(leaves=leaves), \
                 patch.object(harness, 'load_v11', return_value=SimpleNamespace(REQUIRED_STEPS=leaves)) as loader:
                definition = harness.load_definition()
            loader.assert_not_called()
            self.assertEqual(definition['technical_evidence_dependency'],
                harness.TECHNICAL_EVIDENCE_DEPENDENCY)
            self.assertNotIn('required_product_steps', definition)

    def test_technical_dependency_and_journey_topology_are_required(self):
        original = harness.load_definition()
        mutations = []
        for dependency in (None, 'unverified_gate_capsule', 'verified_prior_contract'):
            changed = copy.deepcopy(original)
            if dependency is None:
                changed.pop('technical_evidence_dependency')
            else:
                changed['technical_evidence_dependency'] = dependency
            mutations.append(changed)
        for field, value in (('journey_count', 4), ('work_count', 6),
                             ('resume_pair_count', 2), ('session_count', 9)):
            changed = copy.deepcopy(original)
            changed['campaign_topology'][field] = value
            mutations.append(changed)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'evaluation.json'
            with patch.object(harness, 'DEFINITION', path):
                for changed in mutations:
                    with self.subTest(changed=changed.get('technical_evidence_dependency'),
                                      topology=changed['campaign_topology']):
                        path.write_text(json.dumps(changed), encoding='utf-8')
                        with self.assertRaises(ValueError):
                            harness.load_definition()


class PolicyTests(unittest.TestCase):
    def setUp(self):
        self.prep = fixtures.preparation()
        self.specs = review.criterion_specs(self.prep["index"], self.prep["rubric"])
        self.agent = fixtures.completed(self.prep)
        self.human_prep = fixtures.preparation("human")
        self.human_prep["reviewer"]["run_id"] = "b" * 32
        self.human = fixtures.completed(self.human_prep)
        self.evaluation = evaluation()
        self.technical = {"state": "passed", "candidate_head": "a" * 40}

    def result(self, reviews=None, technical=None):
        return policy.combine(self.evaluation, self.specs, reviews if reviews is not None else [self.agent, self.human], technical or self.technical)

    def test_declared_qualification_contract_matches_executable_policy(self):
        definition = json.loads(Path(__file__).with_name("evaluation.json").read_text())
        self.assertEqual(definition["qualification_policy"], policy.contract())

    def test_agent_covers_semantics_human_only_targeted_observations(self):
        value = self.result([self.agent])
        self.assertTrue(value["qualitative_review"]["resolved_criteria"])
        self.assertTrue(all('/live_viewer/' in c or c == 'journey-volicord/viewer_snapshot/multiple_work_organization'
            or c.endswith('/decision_comprehension_when_applicable')
            for c in value["qualitative_review"]["human_escalations"]))
        self.assertEqual(value["naturalistic_evidence"]["multi_work_structural_continuity"]["state"],
            "passed")
        self.assertEqual(value["naturalistic_evidence"]["multi_work_viewer_comprehension"]["state"],
            "unresolved")
        self.assertEqual(value["replacement_qualification"], "unresolved")
        self.assertFalse(value["phase_9_ready"])
        required = set(value["qualitative_review"]["human_escalations"])
        for i, a in enumerate(self.human["assessments"]):
            if a["criterion_id"] not in required:
                self.human["assessments"][i] = review.observation(a["criterion_id"])
        review.validate_value(self.human_prep, "d" * 64, self.human)
        self.assertEqual(self.result()["replacement_qualification"], "qualified")
        self.assertFalse(self.result()["phase_9_ready"])

    def test_semantic_disagreement_does_not_invalidate_campaign_integrity(self):
        criterion = next(a for a in self.agent["assessments"]
            if a["criterion_id"].endswith("/hidden_material_discovery_quality"))
        criterion["assessment"] = "violated"
        result = self.result()
        self.assertEqual(result["replacement_qualification"], "unresolved")
        self.assertNotIn("campaign_control_coverage", result)

    def test_not_observed_opportunity_is_distinct_and_nonblocking(self):
        criterion = next(a for a in self.agent["assessments"]
            if a["criterion_id"].endswith("/hidden_material_discovery_quality"))
        criterion["assessment"] = "not_observed"
        criterion["evidence"] = []
        criterion["criterion_observations"] = []
        human_criterion = next(a for a in self.human["assessments"]
            if a["criterion_id"] == criterion["criterion_id"])
        self.human["assessments"][self.human["assessments"].index(human_criterion)] = review.observation(criterion["criterion_id"])
        result = self.result()
        self.assertEqual(result["replacement_qualification"], "qualified")
        self.assertIn(criterion["criterion_id"], result["qualitative_review"]["not_observed_criteria"])
        self.assertNotIn(criterion["criterion_id"], result["qualitative_review"]["resolved_criteria"])
        self.assertNotIn(criterion["criterion_id"], result["qualitative_review"]["violated_criteria"])

    def test_hard_integrity_neither_agent_nor_human_can_override(self):
        self.evaluation["works"][0]["findings"] = [m.finding("raw_hash", "confirmed_violation", {"mismatch": True})]
        for reviews in ([self.agent], [self.human], [self.agent, self.human]):
            self.assertEqual(self.result(reviews)["replacement_qualification"], "blocked")
        self.assertEqual(policy.combine(self.evaluation, self.specs, [], self.technical, evidence_validity="invalid")["replacement_qualification"], "blocked")

    def test_semantic_machine_indeterminacy_is_review_support(self):
        finding = m.finding("appropriate_inquiry_outcome", "indeterminate", {"observed": "ambiguous"})
        self.evaluation["works"][0]["findings"] = [finding]
        value = self.result()
        self.assertEqual(value["replacement_qualification"], "qualified")
        self.assertIn(self.evaluation["works"][0]["work_slot_id"] + "/appropriate_inquiry_outcome",
            value["machine_summary"]["review_support_findings"])


    def test_structural_continuity_and_viewer_comprehension_are_independent(self):
        value = self.result()
        naturalistic = value["naturalistic_evidence"]
        self.assertEqual(naturalistic["multi_work_structural_continuity"]["state"], "passed")
        self.assertEqual(naturalistic["multi_work_viewer_comprehension"]["state"], "satisfied")
        journey = next(item for item in self.evaluation["journeys"]
                       if item["journey_id"] == "journey-volicord")
        finding = next(item for item in journey["findings"]
                       if item["check"] == "journey_work_history")
        journey["findings"][journey["findings"].index(finding)] = m.finding(
            "journey_work_history", "confirmed_violation", {"missing_checkpoint": True})
        value = self.result()
        self.assertEqual(value["naturalistic_evidence"]["multi_work_structural_continuity"]["state"],
            "violated")
        self.assertEqual(value["naturalistic_evidence"]["multi_work_viewer_comprehension"]["state"],
            "satisfied")
        self.assertEqual(value["replacement_qualification"], "blocked")

    def test_exact_journey_work_resume_and_session_topology_is_mandatory(self):
        mutations = []
        duplicate = copy.deepcopy(self.evaluation)
        duplicate["works"][-1] = copy.deepcopy(duplicate["works"][0])
        mutations.append(duplicate)
        missing_resume = copy.deepcopy(self.evaluation)
        next(item for item in missing_resume["works"]
             if item["work_slot_id"] == "journey-small-python-work-a")["resume_pair"] = False
        mutations.append(missing_resume)
        wrong_sessions = copy.deepcopy(self.evaluation)
        wrong_sessions["coverage"]["fresh_sessions"] = 9
        mutations.append(wrong_sessions)
        for changed in mutations:
            with self.assertRaises(ValueError):
                policy.combine(changed, self.specs, [self.agent, self.human], self.technical)

    def test_insufficient_and_missing_technical_gate_cannot_pass(self):
        for a in self.agent["assessments"]:
            a["assessment"] = "insufficient_evidence"
            a["evidence"] = []
        self.assertEqual(self.result([self.agent])["replacement_qualification"], "unresolved")
        self.assertEqual(self.result([self.human], {"state": "not_provided"})["replacement_qualification"], "unresolved")
        self.assertEqual(self.result([self.human], {"state": "failed"})["replacement_qualification"], "blocked")

    def test_completed_qualitative_reviews_cannot_manufacture_technical_success(self):
        for technical, expected in (({'state': 'not_provided'}, 'unresolved'),
                                    ({'state': 'failed'}, 'blocked')):
            with self.subTest(technical=technical):
                result = self.result([self.agent, self.human], technical)
                self.assertEqual(result['qualitative_review']['state'], 'complete')
                self.assertEqual(result['replacement_qualification'], expected)
                self.assertFalse(result['replacement_pass_candidate'])

    def test_one_missing_repository_class_cli_observation_is_a_bounded_gap(self):
        missing = copy.deepcopy(self.agent)
        for assessment in missing["assessments"]:
            if assessment["criterion_id"].startswith("polyglot-medium/cli/"):
                assessment["assessment"] = "insufficient_evidence"
                assessment["evidence"] = []
        human = copy.deepcopy(self.human)
        for index, assessment in enumerate(human["assessments"]):
            if "/cli/" in assessment["criterion_id"]:
                human["assessments"][index] = review.observation(assessment["criterion_id"])
        result = self.result([missing, human])
        gaps = [cid for cid in result["qualitative_review"]["unresolved_criteria"]
                if cid.startswith("polyglot-medium/cli/")]
        self.assertEqual(len(gaps), 7)
        self.assertEqual(result["qualitative_review"]["unresolved_criteria"], gaps)
        self.assertEqual(result["replacement_qualification"], "unresolved")

    def test_conflict_requires_explicit_targeted_human_resolution(self):
        cid = self.agent["assessments"][0]["criterion_id"]
        negative = copy.deepcopy(self.agent)
        negative["reviewer"]["run_id"] = "c" * 32
        negative["assessments"][0]["assessment"] = "violated"
        result = self.result([self.agent, negative, self.human])
        self.assertIn(cid, result["qualitative_review"]["human_escalations"])
        self.human["resolves_review_runs"] = {cid: [self.agent["reviewer"]["run_id"], negative["reviewer"]["run_id"]]}
        self.assertEqual(self.result([self.agent, negative, self.human])["replacement_qualification"], "qualified")
        self.agent["resolves_review_runs"] = self.human["resolves_review_runs"]
        with self.assertRaisesRegex(ValueError, "only human"):
            review.validate_value(self.prep, "d" * 64, self.agent)

    def test_additional_outcome_insufficiency_requires_targeted_human_resolution(self):
        cid = "journey-volicord-work-a/authority/additional-durability"
        agent_extra = fixtures.fill(review.observation(cid), self.prep, "insufficient_evidence")
        agent_extra["authority"] = fixtures.assessment()
        agent_extra["authority"]["authority_relation_to_outcome"] = "uncertain"
        self.agent["additional_outcomes"] = [{"sample_id": "journey-volicord-work-a", "finding": agent_extra}]
        review.validate_value(self.prep, "d" * 64, self.agent)

        human_extra = fixtures.fill(review.observation(cid), self.human_prep)
        human_extra["authority"] = fixtures.assessment()
        self.human["additional_outcomes"] = [{"sample_id": "journey-volicord-work-a", "finding": human_extra}]
        self.assertEqual(self.result()["replacement_qualification"], "unresolved")
        self.human["resolves_review_runs"] = {cid: [self.agent["reviewer"]["run_id"]]}
        review.validate_value(self.human_prep, "d" * 64, self.human)
        result = self.result()
        self.assertEqual(result["replacement_qualification"], "qualified")
        self.assertFalse(result["phase_9_ready"])

    def test_explicit_operator_action_is_append_only_and_binds_complete_state(self):
        import review_operations as ops
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'qualification.json'
            value = {"kind": "phase8_dogfood_result", "schema_version": 3, "policy": policy.identity(),
                "candidate_head": "a" * 40, "evaluator_revision": "b" * 40, "run_nonce": "c" * 32,
                "evidence_set": {"path": "evidence-set.json", "sha256": "d" * 64},
                "evaluation_run": {"run_id": "e" * 64, "sha256": "f" * 64}, "qualitative_review_runs": [],
                **self.result()}
            value["run_id"] = m.digest(value)
            policy.validate_result(value)
            original = ops.encoded(value)
            path.write_bytes(original)
            (root / 'inputs.json').write_bytes(ops.encoded({"campaign_root": str(root / 'source-campaign')}))
            with patch.object(policy, 'verify_qualification', return_value=value) as verify:
                approval = policy.approve(path, root / 'approval', operator='synthetic operator', statement='approve-phase-9')
                verify.assert_called_once_with(path)
                self.assertTrue(approval['phase_9_ready'])
                self.assertEqual(policy.verify_approval(root / 'approval/approval.json', path), approval)
                self.assertEqual(approval['qualification_sha256'], ops.digest(original))
                self.assertEqual((root / 'approval/qualification.json').read_bytes(), original)
                with self.assertRaises(ValueError):
                    policy.approve(path, root / 'approval', operator='synthetic operator', statement='approve-phase-9')
            self.assertEqual(path.read_bytes(), original)
            value['phase_9_ready'] = True
            value['run_id'] = m.digest({k: v for k,v in value.items() if k != 'run_id'})
            with self.assertRaises(ValueError):
                policy.validate_result(value)

    def test_approval_refuses_missing_required_review(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'qualification.json'
            path.write_text(json.dumps({"replacement_qualification": "unresolved"}))
            with patch.object(policy, 'verify_qualification', return_value={"replacement_qualification": "unresolved"}):
                with self.assertRaisesRegex(ValueError, 'cannot replace'):
                    policy.approve(path, Path(directory) / 'approval', operator='operator', statement='approve-phase-9')
            self.assertFalse((Path(directory) / 'approval').exists())


class GateTechnicalBoundaryTests(unittest.TestCase):
    """Actual gate entrypoint/archive/consumer flow with synthetic expensive owners."""

    @classmethod
    def setUpClass(cls):
        import contextlib
        import io
        import runpy

        root = Path(__file__).resolve().parents[3]
        entrypoint = root / 'rebuild/validation/end-to-end/multi-repository/gate_entrypoint_self_test.py'
        cls.temp = tempfile.TemporaryDirectory(prefix='volicord-technical-boundary-')
        cls.addClassCleanup(cls.temp.cleanup)
        parent = Path(cls.temp.name)
        entrypoint_fixture = runpy.run_path(str(entrypoint))
        candidate, _, _, _ = entrypoint_fixture['make_candidate'](parent)
        # Use the maintained V11 result validator instead of the preflight-only
        # fixture's deliberately non-passing shell harness. No real V11 runs.
        shutil.copy2(entrypoint.with_name('harness.py'),
            candidate / 'rebuild/validation/end-to-end/multi-repository/harness.py')
        for helper in ('restart_recall.py', 'materiality_scenarios.py',
                       'multi_work.py', 'result_contract.py', 'multi_work_self_test.py',
                       'analysis_metadata.py'):
            shutil.copy2(entrypoint.with_name(helper),
                candidate / 'rebuild/validation/end-to-end/multi-repository' / helper)
        assert entrypoint_fixture['git'](candidate, 'add', '.').returncode == 0
        assert entrypoint_fixture['git'](candidate, '-c', 'user.name=Validation Fixture',
            '-c', 'user.email=validation@example.invalid', 'commit', '-qm',
            'maintained V11 validator fixture').returncode == 0
        runner = runpy.run_path(str(candidate / 'rebuild/scripts/validate'))
        namespace = runner['run_gate'].__globals__
        gate = runner['load_gate_module']()
        archive_builder = runner['load_evidence_archive_module']()
        collect = archive_builder.create_review_archive

        def archive(**kwargs):
            try:
                return collect(**kwargs)
            except (OSError, RuntimeError, ValueError) as error:
                raise AssertionError('synthetic archive fixture invalid') from error
        owners_fixture = runpy.run_path(str(entrypoint.with_name('gate_self_test.py')))
        fixtures = owners_fixture['admission_overrides'].__globals__
        cls.candidate = entrypoint_fixture['git'](
            candidate, 'rev-parse', 'HEAD').stdout.strip()
        owners = owners_fixture['Owners'](parent / 'owners')
        admitted = gate.evaluate_admission(
            authorization_assertion=gate.AUTHORIZATION_ASSERTION,
            provider_authorization_assertion=gate.PROVIDER_AUTHORIZATION_ASSERTION,
            provider_model='synthetic-model', external_network='available',
            artifact_root=parent / 'admission', command_runner=owners_fixture['unused_command_runner'],
            runner_path=candidate / 'rebuild/scripts/validate',
            overrides={**owners_fixture['admission_overrides'](),
                'candidate_identity_and_clean_worktree': gate.repository_check()[0]},
            environment_evidence=owners_fixture['synthetic_environment_evidence'](),
            dependency_evidence=gate.dependency_snapshot(cls.candidate))
        assert admitted['eligible'], admitted
        blocked_admission = gate.evaluate_admission(
            authorization_assertion=gate.AUTHORIZATION_ASSERTION,
            provider_authorization_assertion=gate.PROVIDER_AUTHORIZATION_ASSERTION,
            provider_model='synthetic-model', external_network='unavailable',
            artifact_root=parent / 'blocked-admission',
            command_runner=owners_fixture['unused_command_runner'],
            runner_path=candidate / 'rebuild/scripts/validate',
            overrides={**owners_fixture['admission_overrides'](),
                'candidate_identity_and_clean_worktree': gate.repository_check()[0]},
            environment_evidence=owners_fixture['synthetic_environment_evidence'](),
            dependency_evidence=gate.dependency_snapshot(cls.candidate))
        assert not blocked_admission['eligible']
        original_orchestrate = gate.orchestrate

        def orchestrate(**kwargs):
            return original_orchestrate(**kwargs,
                contract_execution_owner=lambda summary: gate.check(
                    'contract_coverage_execution', 'passed', 'synthetic execution coverage',
                    execution_owner='exact_candidate_final_workspace_tests', mapped_test_count=12))

        def aggregate(label, commands):
            assert label == 'final' and commands == namespace['FINAL_COMMANDS']
            summary, path = owners.final()
            summary['working_directory'] = str(candidate)
            summary['failure_count'] = sum(command['exit_code'] != 0
                for command in summary['commands'])
            for command in summary['commands']:
                command['working_directory'] = str(candidate)
            runner['write_json'](path, summary)
            return summary, path.parent

        def json_command(command_runner, directory, argv):
            # Replace only costly owners; run_gate still owns their sequencing,
            # Final binding, publication checks, collection and independent verification.
            if '--live' in argv:
                value, execution, path = owners.provider(cls.candidate)
                target = Path(argv[argv.index('--evidence-output') + 1])
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, target)
                return value, execution
            if argv[1] == 'preflight':
                return owners.preflight(cls.candidate, Path(argv[-1]))
            if argv[1] == 'run':
                return owners.v11(cls.candidate, owners.final_path,
                    Path(argv[argv.index('--output-dir') + 1]))
            assert argv[1] == 'credential-audit', argv
            return owners.audit(Path(argv[-1]))

        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.dict(fixtures, HEAD=cls.candidate, FINAL_COMMANDS=namespace['FINAL_COMMANDS']), \
                patch.dict(namespace, load_gate_module=lambda: gate,
                    load_evidence_archive_module=lambda: archive_builder, run_aggregate=aggregate), \
                patch.object(archive_builder, 'create_review_archive', side_effect=archive), \
                patch.object(gate, 'evaluate_admission', return_value=admitted), \
                patch.object(gate, 'orchestrate', side_effect=orchestrate), \
                patch.object(gate, 'run_json_command', side_effect=json_command), \
                contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            arguments = [
                '--external-network', 'available', '--authorize-external-transmission',
                gate.AUTHORIZATION_ASSERTION, '--authorize-provider-source-transmission',
                gate.PROVIDER_AUTHORIZATION_ASSERTION, '--provider-model', 'synthetic-model']
            code = runner['run_gate'](arguments)
            assert owners.counts == {'final': 1, 'provider': 1, 'preflight': 1, 'v11': 1, 'audit': 1}
            owners.final_passes = False
            failed_stdout, failed_stderr = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(failed_stdout), contextlib.redirect_stderr(failed_stderr):
                failed_code = runner['run_gate'](arguments)
            assert failed_code == 1
            blocked_stdout, blocked_stderr = io.StringIO(), io.StringIO()
            counts = owners.counts.copy()
            with patch.object(gate, 'evaluate_admission', return_value=blocked_admission), \
                    contextlib.redirect_stdout(blocked_stdout), contextlib.redirect_stderr(blocked_stderr):
                blocked_code = runner['run_gate'](arguments)
            assert blocked_code == 1 and owners.counts == counts
        assert code == 0, (stdout.getvalue(), stderr.getvalue())
        cls.capsule = json.loads(stdout.getvalue())
        cls.path = Path(next(line.removeprefix('evidence capsule: ')
            for line in stderr.getvalue().splitlines() if line.startswith('evidence capsule: ')))
        identity = json.loads((cls.path.parent / 'evidence-archive.json').read_bytes())
        cls.archive = Path(identity['path'])
        cls.gate = gate
        cls.owners = owners
        cls.failed_path = Path(next(line.removeprefix('evidence capsule: ')
            for line in failed_stderr.getvalue().splitlines() if line.startswith('evidence capsule: ')))
        cls.failed_capsule = json.loads(failed_stdout.getvalue())
        cls.failed_archive = Path(json.loads(
            (cls.failed_path.parent / 'evidence-archive.json').read_bytes())['path'])

        cls.blocked_path = Path(next(line.removeprefix('evidence capsule: ')
            for line in blocked_stderr.getvalue().splitlines() if line.startswith('evidence capsule: ')))
        cls.blocked_archive = Path(json.loads(
            (cls.blocked_path.parent / 'evidence-archive.json').read_bytes())['path'])

    def test_missing_technical_evidence_is_not_verified(self):
        self.assertEqual(policy.verify_technical(self.candidate, None, None)['state'],
            'not_provided')
        for capsule, archive in ((self.path, None), (None, self.archive)):
            with self.assertRaisesRegex(ValueError, 'requires capsule and archive'):
                policy.verify_technical(self.candidate, capsule, archive)

    def test_blocked_producer_evidence_is_non_passing(self):
        self.assertEqual(policy.verify_technical(self.candidate, self.blocked_path,
            self.blocked_archive)['state'], 'failed')

    def test_failed_producer_evidence_cannot_be_promoted(self):
        result = policy.verify_technical(self.candidate, self.failed_path, self.failed_archive)
        self.assertEqual(result['state'], 'failed')
        self.assertFalse(self.failed_capsule['phase_8_ready'])
        self.assertFalse(self.failed_capsule['evidence_archive']['prerequisites_passed'])
        self.assertNotIn('archive_publication', [entry['boundary']
            for entry in self.failed_capsule.get('candidate_continuity_checks', [])])
        changed = copy.deepcopy(self.failed_capsule)
        changed['phase_8_ready'] = True
        changed['evidence_archive']['prerequisites_passed'] = True
        changed.setdefault('candidate_continuity_checks', []).append({'boundary': 'archive_publication',
            **self.gate.candidate_continuity_check(self.candidate, self.candidate, 0, [])})
        path = self.path.parent / 'forged-readiness.json'
        path.write_bytes(policy.operations.encoded(changed))
        with self.assertRaises(ValueError):
            policy.verify_technical(self.candidate, path, self.failed_archive)

    def test_successful_producer_capsule_is_consumed(self):
        before = self.owners.counts.copy()
        result = policy.verify_technical(self.candidate, self.path, self.archive)
        self.assertEqual(self.owners.counts, before, 'technical verification reran a gate owner')
        self.assertEqual(result['state'], 'passed')
        self.assertEqual(result['candidate_head'], self.candidate)
        self.assertEqual(self.capsule['candidate_continuity_checks'][-1]['boundary'], 'archive_publication')

    def test_final_capsule_mutations_are_rejected(self):
        def publication(value):
            return value['candidate_continuity_checks'][-1]

        mutations = {
            'missing_publication': lambda v: v['candidate_continuity_checks'].pop(),
            'altered_prefix': lambda v: v['candidate_continuity_checks'][0].update(summary='changed'),
            'extra_publication': lambda v: v['candidate_continuity_checks'].append(copy.deepcopy(publication(v))),
            'wrong_boundary': lambda v: publication(v).update(boundary='other_publication'),
            'wrong_check': lambda v: publication(v).update(name='arbitrary_success'),
            'failed_publication': lambda v: publication(v).update(status='failed'),
            'blocked_publication': lambda v: publication(v).update(status='environment_blocked'),
            'wrong_expected_candidate': lambda v: publication(v)['details'].update(expected_candidate_head='f' * 40),
            'changed_head': lambda v: publication(v)['details'].update(observed_candidate_head='f' * 40),
            'head_changed_but_passed': lambda v: publication(v)['details'].update(head_unchanged=False),
            'dirty_count': lambda v: publication(v)['details'].update(dirty_entry_count=1),
            'dirty_entries': lambda v: publication(v)['details'].update(dirty_entries=['?? dirty.txt']),
            'numeric_boolean': lambda v: publication(v)['details'].update(head_unchanged=1),
            'boolean_count': lambda v: publication(v)['details'].update(dirty_entry_count=False),
            'extra_publication_detail': lambda v: publication(v)['details'].update(unexpected=True),
            'wrong_contract_kind': lambda v: v.update(kind='obsolete_gate_contract'),
            'old_v11_schema': lambda v: v['official_v11'].update(schema_version=1),
            'old_v11_workload': lambda v: v['official_v11'].update(
                workload_identity='historical-54-workload'),
            'missing_multi_work': lambda v: v['official_v11'].update(
                multi_work_continuity=None),
            'wrong_multi_work_scope': lambda v: v['official_v11'][
                'multi_work_continuity']['authority'].update(decision_work_id='wrong'),
            'wrong_target_counts': lambda v: v['official_v11'].update(
                required_by_target={'volicord': 18, 'small-python': 18, 'polyglot-medium': 18}),
            'archive_hash': lambda v: v['evidence_archive'].update(sha256='f' * 64),
            'archive_size': lambda v: v['evidence_archive'].update(size_bytes=1),
            'archive_members': lambda v: v['evidence_archive'].update(member_count=1),
            'archive_candidate': lambda v: v['evidence_archive'].update(candidate_head='f' * 40),
            'archive_filename': lambda v: v['evidence_archive'].update(filename='other.tar.gz'),
            'archive_prerequisites': lambda v: v['evidence_archive'].update(prerequisites_passed=False),
            'archive_status': lambda v: v['evidence_archive'].update(status='pending'),
            'archive_verification': lambda v: v['evidence_archive'].update(verification_status='failed'),
            'final_artifact': lambda v: v.update(final_summary_sha256='f' * 64),
            'unrelated_field': lambda v: v['execution_environment']['platform'].update(release='changed'),
            'extra_field': lambda v: v.update(unexpected=True),
            'readiness_false': lambda v: v.update(phase_8_ready=False),
        }
        # Swap earlier entries only, leaving the required publication last.
        mutations['reordered_prefix'] = lambda v: v['candidate_continuity_checks'].__setitem__(
            slice(0, 2), list(reversed(v['candidate_continuity_checks'][:2])))
        for label, mutate in mutations.items():
            with self.subTest(label=label):
                changed = copy.deepcopy(self.capsule)
                mutate(changed)
                path = self.path.parent / 'mutated-capsule.json'
                path.write_bytes(policy.operations.encoded(changed))
                with self.assertRaises(ValueError):
                    policy.verify_technical(self.candidate, path, self.archive)
        with self.assertRaises(ValueError):
            policy.verify_technical('f' * 40, self.path, self.archive)
        tampered = self.archive.with_name('tampered.tar.gz')
        tampered.write_bytes(self.archive.read_bytes()[:100])
        with self.assertRaises((ValueError, OSError, EOFError)):
            policy.verify_technical(self.candidate, self.path, tampered)


class FileBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from review_operations_self_test import WorkflowTests
        WorkflowTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_real_evidence_reviews_result_revalidation_and_mismatch(self):
        import campaign
        import review_operations as ops
        from review_operations_self_test import insufficient_draft, snapshot
        before = snapshot(self.root)
        target = self.parent / 'incomplete-review'
        with patch.object(campaign.harness, 'load_v11', side_effect=AssertionError('V11 rerun')), \
             patch.object(policy, 'verify_technical', side_effect=AssertionError('technical verification during review')):
            ops.prepare(self.root, target, reviewer_kind='agent', session_id='separate-review', evaluation_path=self.evaluation)
            insufficient_draft(target)
            ops.record(target, target / 'draft.json')
        output = self.parent / 'qualification'
        with patch.object(campaign.harness, 'real_session_evidence', side_effect=AssertionError('naturalistic rerun')), \
             patch.object(campaign.harness, 'load_v11', side_effect=AssertionError('V11 rerun')), \
             patch.object(policy, 'verify_technical', wraps=policy.verify_technical) as technical_verifier:
            value = policy.qualify(self.root, self.evaluation, output,
                candidate=campaign.load_campaign(self.root)['candidate_head'], review_roots=[target])
            self.assertNotEqual(value['replacement_qualification'], 'qualified')
            self.assertEqual(value['technical_gate']['state'], 'not_provided')
            technical_verifier.assert_called_once_with(campaign.load_campaign(self.root)['candidate_head'], None, None)
            self.assertEqual(policy.verify_qualification(output / 'qualification.json'), value)
            with self.assertRaisesRegex(ValueError, 'cannot replace'):
                policy.approve(output / 'qualification.json', self.parent / 'approval', operator='operator', statement='approve-phase-9')
            with self.assertRaisesRegex(ValueError, 'candidate mismatch'):
                policy.qualify(self.root, self.evaluation, self.parent / 'wrong-candidate', candidate='0' * 40)
        import result_lineage
        published = result_lineage.publish(self.root, self.evaluation, [target],
            output / 'qualification.json')
        lineage_root = Path(published['lineage_root'])
        self.assertEqual(lineage_root, self.parent / 'results' / value['run_id'])
        copied = self.parent / 'copied-result-lineage'
        shutil.copytree(lineage_root, copied)
        self.assertNotIn(str(self.parent), (copied / 'index.json').read_text())
        with patch.object(campaign, 'load_evidence_set', side_effect=AssertionError('original campaign access')):
            verified = result_lineage.verify(copied)
        self.assertEqual(verified['qualification_run_id'], value['run_id'])
        self.assertFalse(verified['external_staging_paths_used'])
        lineage_qualification = json.loads(
            (copied / 'qualification/qualification.json').read_bytes())
        lineage_evidence = json.loads((copied / 'source/evidence-set.json').read_bytes())
        self.assertEqual(
            lineage_qualification['naturalistic_evidence']['naturalistic_resource'],
            lineage_evidence['naturalistic_memory_evidence'])
        evaluation_copy = copied / 'evaluation/evaluation.json'
        evaluation_copy.chmod(0o600)
        evaluation_copy.write_bytes(evaluation_copy.read_bytes() + b' ')
        with self.assertRaisesRegex(ValueError, 'artifact changed'):
            result_lineage.verify(copied)
        self.assertEqual(snapshot(self.root), before)
        changed = copy.deepcopy(value)
        changed['qualitative_review']['unresolved_criteria'] = []
        changed['run_id'] = m.digest({k: v for k, v in changed.items() if k != 'run_id'})
        (output / 'qualification.json').chmod(0o600)
        (output / 'qualification.json').write_bytes(ops.encoded(changed))
        with self.assertRaises(ValueError):
            policy.verify_qualification(output / 'qualification.json')

    def test_agent_cannot_supply_human_observation(self):
        import review_operations as ops
        with self.assertRaisesRegex(ValueError, 'agent preparation'):
            ops.prepare(self.root, self.parent / 'false-human', reviewer_kind='agent', session_id='review',
                human_observations=self.parent / 'missing.json')


def run_contract_tests():
    result = unittest.TextTestRunner().run(unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(cls) for cls in (DefinitionDependencyTests, PolicyTests, GateTechnicalBoundaryTests, FileBoundaryTests)))
    if not result.wasSuccessful():
        raise AssertionError('qualification policy regressions failed')


if __name__ == '__main__':
    unittest.main()
