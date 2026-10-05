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
import rehearsal_contract


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
    return {"evidence_purpose": "naturalistic", "run_id": "a" * 64, "works": works, "journeys": journeys,
        "coverage": copy.deepcopy(policy.TOPOLOGY)}


def rehash_lineage_qualification(root, value):
    """Coherent internal rewrite: semantic controls must get past every hash."""
    import result_lineage
    ops = policy.operations
    value["run_id"] = m.digest({k: v for k, v in value.items() if k != "run_id"})
    policy.validate_result(value)
    data = ops.encoded(value)
    path = root / "qualification/qualification.json"
    path.chmod(0o600)
    path.write_bytes(data)
    index = json.loads((root / "index.json").read_bytes())
    index["qualification"].update(run_id=value["run_id"], sha256=ops.digest(data),
        replacement_qualification=value["replacement_qualification"], phase_9_ready=value["phase_9_ready"])
    index["lineage_id"] = m.digest({k: v for k, v in index.items() if k != "lineage_id"})
    index_data = ops.encoded(index)
    receipt = json.loads((root / "receipt.json").read_bytes())
    receipt.update(lineage_id=index["lineage_id"], index_sha256=ops.digest(index_data))
    receipt["artifacts"] = {name: result_lineage._binding((root / name).read_bytes())
        for name in receipt["artifacts"]}
    for name, content in (("index.json", index_data), ("receipt.json", ops.encoded(receipt))):
        (root / name).chmod(0o600)
        (root / name).write_bytes(content)


def promote_lineage_qualification(value, evaluation):
    """Structurally consistent success claim with no supporting review changes."""
    qualitative = value["qualitative_review"]
    qualitative["resolved_criteria"] = sorted(set(qualitative["resolved_criteria"]
        + qualitative["unresolved_criteria"] + qualitative["violated_criteria"]))
    for field in ("unresolved_criteria", "violated_criteria", "human_escalations"):
        qualitative[field] = []
    qualitative["state"] = "complete"
    value["machine_summary"]["hard_findings"] = []
    value["technical_gate"] = {"state": "passed", "candidate_head": value["candidate_head"], "rehearsal": {"contract": rehearsal_contract.CONTRACT, "status": "passed", "result_sha256": "8" * 64}}
    value.update(evidence_validity="valid", replacement_qualification="qualified",
        replacement_pass_candidate=True)
    value["naturalistic_evidence"] = policy.naturalistic_summary(value, evaluation,
        value["naturalistic_evidence"]["naturalistic_resource"])


class LaunchReadinessTests(unittest.TestCase):
    """Environment scope controls, with authored Product-read expectations."""
    def setUp(self):
        import importlib.util
        import campaign
        self.temp = tempfile.TemporaryDirectory(prefix='volicord-readiness-')
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.binary = root / 'install with spaces/bin/volicord'
        self.binary.parent.mkdir(parents=True)
        for name in ('volicord', 'volicord-mcp'):
            shutil.copy2('/bin/true', self.binary.with_name(name))
        self.repository = root / 'repository'; self.repository.mkdir()
        self.runtime = root / 'runtime'; self.runtime.mkdir()
        (self.runtime / 'canonical.sqlite3').write_bytes(b'support store identity')
        import campaign_self_test as support
        support.write_static_integration(self.repository, self.runtime, self.binary)
        path = campaign.ROOT / 'rebuild/validation/linux-codex-integration/launch_readiness.py'
        spec = importlib.util.spec_from_file_location('scope_readiness_test', path)
        self.owner = importlib.util.module_from_spec(spec); spec.loader.exec_module(self.owner)
        self.context = {'candidate_head': 'a' * 40, 'execution_channel': 'vscode_tool_shell',
            'host_version': 'authored-host-version', 'sandbox_mode': 'workspace-write',
            'sandbox_permissions': 'use_default', 'writable_roots': [str(self.runtime)],
            'workspace_trust': 'user_confirmed', 'hook_trust': 'user_confirmed',
            'permission_basis': 'authored-effective-policy'}
        self.hashes = [self.owner.digest(self.binary), self.owner.digest(self.binary.with_name('volicord-mcp'))]

    def probe(self, **kwargs):
        return self.owner.probe(self.binary, self.runtime, self.repository, *self.hashes,
            context=kwargs.pop('context', self.context), **kwargs)

    def first(self):
        with patch.object(self.owner, 'scoped_status', return_value=({'project_id': '1' * 32}, '2' * 64)) as status:
            value = self.probe()
            status.assert_called_once_with(self.binary, self.runtime, self.repository)
        self.assertEqual(value['execution'], 'checked')
        return value

    def test_unchanged_scope_reuses_numeric_hash_bound_result_without_execution(self):
        value = self.first()
        stdout = Path(self.temp.name) / 'stdout.log'
        stdout.write_text(json.dumps(value))
        result = Path(self.temp.name) / 'result.json'
        result.write_text(json.dumps({'exit_code': 0, 'termination': None,
            'stdout_sha256': self.owner.digest(stdout)}))
        previous = self.owner.retained_observation(stdout, result)
        with patch.object(self.owner.subprocess, 'run', side_effect=AssertionError('redundant subprocess')), \
             patch.object(self.owner.tempfile, 'TemporaryFile', side_effect=AssertionError('redundant permission write')):
            repeated = self.probe(previous=previous)
            self.assertEqual(repeated['execution'], 'reused')
            self.assertEqual(repeated['reused'], ['Runtime_write', 'scoped_status'])
            self.assertEqual(repeated['readback_sha256'], '2' * 64)
            self.assertEqual(self.probe(previous=repeated)['execution'], 'reused')
        for change in ({'exit_code': 1}, {'termination': 'timeout'}, {'stdout_sha256': 'f' * 64}):
            record = {'exit_code': 0, 'termination': None, 'stdout_sha256': self.owner.digest(stdout)}
            record.update(change); result.write_text(json.dumps(record))
            with self.assertRaises(ValueError): self.owner.retained_observation(stdout, result)

    def test_changed_host_candidate_permissions_or_trust_needs_only_scoped_probe(self):
        previous = self.first()
        for key, changed in (('candidate_head', 'b' * 40), ('execution_channel', 'codex_cli_tool_shell'),
                ('host_version', 'changed-version'), ('permission_basis', 'changed-effective-policy'),
                ('sandbox_permissions', 'require_escalated'), ('hook_trust', 'unverified')):
            context = {**self.context, key: changed}
            with self.subTest(key=key), patch.object(self.owner, 'scoped_status',
                    return_value=({'project_id': '1' * 32}, '3' * 64)) as status:
                self.assertEqual(self.probe(context=context, previous=previous)['execution'], 'checked')
                status.assert_called_once()
        for key, changed in (('sandbox_mode', 'read-only'), ('writable_roots', [])):
            with self.subTest(key=key), patch.object(self.owner.subprocess, 'run', side_effect=AssertionError('missing permission')):
                with self.assertRaisesRegex(ValueError, 'Runtime write permission missing'):
                    self.probe(context={**self.context, key: changed}, previous=previous)

    def test_relevant_configuration_and_filesystem_changes_invalidate_only_smoke(self):
        previous = self.first()
        config = self.repository / '.codex/config.toml'
        original = config.read_text()
        config.write_text('model = "unrelated-task-model"\n' + original)
        with patch.object(self.owner, 'scoped_status', side_effect=AssertionError('unrelated change')):
            self.assertEqual(self.probe(previous=previous)['execution'], 'reused')
        config.write_text(original.replace('timeout = 5', 'timeout = 6'))
        with patch.object(self.owner, 'scoped_status', return_value=({'project_id': '1' * 32}, '3' * 64)) as status:
            self.assertEqual(self.probe(previous=previous)['execution'], 'checked'); status.assert_called_once()
        config.write_text(original)
        self.runtime.chmod(0o700)
        with patch.object(self.owner, 'scoped_status', return_value=({'project_id': '1' * 32}, '3' * 64)) as status:
            self.assertEqual(self.probe(previous=previous)['execution'], 'checked'); status.assert_called_once()
        with patch.object(self.owner.tempfile, 'TemporaryFile', side_effect=PermissionError('sandbox denied')):
            with self.assertRaisesRegex(ValueError, 'Runtime write permission missing'): self.probe()

    def test_wrong_executable_runtime_or_submitted_observation_is_rejected(self):
        previous = self.first()
        self.binary.write_bytes(self.binary.read_bytes() + b'changed')
        with patch.object(self.owner.subprocess, 'run', side_effect=AssertionError('wrong bytes')):
            with self.assertRaisesRegex(ValueError, 'mismatch'): self.probe(previous=previous)
        shutil.copy2('/bin/true', self.binary)
        with self.assertRaisesRegex(ValueError, 'mismatch'):
            self.owner.probe(self.binary, Path(self.temp.name) / 'wrong', self.repository, *self.hashes)
        for changed in ({'status': 'failed'}, {'exit_code': 1}, {'project_id': None}, {'readback_sha256': None}):
            with self.subTest(changed=changed), self.assertRaises(ValueError): self.probe(previous={**previous, **changed})

    def test_inspection_stages_actual_hook_before_trust_without_product_or_writes(self):
        with patch.object(self.owner.subprocess, 'run', side_effect=AssertionError('inspection executed Product')), \
             patch.object(self.owner.tempfile, 'TemporaryFile', side_effect=AssertionError('inspection wrote Runtime')):
            value = self.probe(inspect=True)
        self.assertEqual(value['status'], 'unverified')
        self.assertIn(str(self.runtime), value['runtime_permission_config'])
        import shlex
        self.assertEqual(shlex.split(value['review_hook']),
            [str(self.binary), '--runtime', str(self.runtime), '--repository', str(self.repository), 'codex', 'hook'])
        with patch.object(self.owner, 'scoped_status', return_value=({'project_id': '1' * 32}, '2' * 64)):
            local = self.probe(context={**self.context, 'execution_channel': 'local_subprocess'})
        self.assertIn('other_host_tool_shell', local['unverified'])
        with patch.object(self.owner, 'scoped_status', return_value=({'project_id': '1' * 32}, '2' * 64)) as status:
            self.assertEqual(self.probe(previous=local)['execution'], 'checked'); status.assert_called_once()


class DefinitionDependencyTests(unittest.TestCase):
    def test_maintained_support_registers_every_readiness_scope_control(self):
        expected = {
            'test_unchanged_scope_reuses_numeric_hash_bound_result_without_execution',
            'test_changed_host_candidate_permissions_or_trust_needs_only_scoped_probe',
            'test_relevant_configuration_and_filesystem_changes_invalidate_only_smoke',
            'test_wrong_executable_runtime_or_submitted_observation_is_rejected',
            'test_inspection_stages_actual_hook_before_trust_without_product_or_writes',
        }
        # Inspect the maintained full registration independently of a focused
        # unittest invocation's global -k filter.
        with patch.object(unittest, 'defaultTestLoader', unittest.TestLoader()), \
             patch.object(unittest.TextTestRunner, 'run',
                return_value=SimpleNamespace(wasSuccessful=lambda: True)) as runner:
            run_contract_tests()
        names = {test.id().rsplit('.', 1)[-1] for group in runner.call_args.args[0] for test in group}
        self.assertTrue(expected <= names, 'maintained qualification support omitted readiness controls')

    def test_responsibility_matrix_and_scope_guidance_have_one_owner(self):
        import campaign
        root = campaign.ROOT / 'rebuild'
        owner = (root / 'docs/design/validation-plan.md').read_text()
        matrix = owner.split('### 3.2 Validation responsibility and evidence reuse\n', 1)[1].split('\n## ', 1)[0]
        categories = {line.split('|')[1].strip() for line in matrix.splitlines()
            if line.startswith('| ') and not line.startswith(('| Category', '| ---'))}
        self.assertEqual(categories, {'Focused tests', 'Ordered Final', 'Formal rehearsal',
            'Provider / V11 technical checks', 'Fixed browser / cost checks',
            'Current-installation smoke', 'Target / task qualification',
            'Optional Naturalistic telemetry', 'Raw integrity', 'Agent review',
            'Human observation', 'Final qualification'})
        for path in ('validation/README.md', 'docs/design/qualitative-review.md',
            'validation/dogfood/campaign-readiness.md', 'validation/dogfood/resource-observation.md'):
            text = (root / path).read_text()
            with self.subTest(path=path):
                self.assertIn('validation-plan.md#32-validation-responsibility-and-evidence-reuse', text)
                self.assertNotIn('| Category | Claim and identity/environment scope |', text)
                self.assertNotIn('resource schema 2', text)
        parser = campaign.parser()
        args = parser.parse_args(['prepare', '--campaign-root', '/unused', '--campaign-id', 'support',
            '--candidate-head', 'a' * 40, '--repositories', '/unused.json', '--tasks', '/unused-tasks.json'])
        self.assertFalse(args.observe_resources)
        self.assertIsNone(args.gate_capsule)
        self.assertIsNone(args.gate_archive)

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
        self.technical = {"state": "passed", "candidate_head": "a" * 40, "rehearsal": {"contract": rehearsal_contract.CONTRACT, "status": "passed", "result_sha256": "8" * 64}}

    def result(self, reviews=None, technical=None):
        return policy.combine(self.evaluation, self.specs, reviews if reviews is not None else [self.agent, self.human], technical or self.technical)

    def test_qualification_rehash_cannot_normalize_missing_instance_sample(self):
        import qualification_policy as policy
        import machine_findings as m
        from resource_coverage_self_test import observation, omit
        value = {'kind': 'phase8_dogfood_result', 'schema_version': 3,
            'policy': policy.identity(), 'candidate_head': 'a' * 40,
            'evaluator_revision': 'b' * 40, 'run_nonce': 'c' * 32, **self.result()}
        resource = observation(2, purpose='naturalistic')
        value['naturalistic_evidence']['naturalistic_resource'] = resource
        value['run_id'] = m.digest(value)
        policy.validate_result(value)
        omit(resource)
        value['run_id'] = m.digest({k: v for k, v in value.items() if k != 'run_id'})
        with self.assertRaisesRegex(ValueError, 'running instance lacks tick sample'):
            policy.validate_result(value)


    def test_sparse_interaction_coverage_is_unresolved_not_product_failure(self):
        cid = "campaign/campaign_interaction/interaction_coverage_adequacy"
        for value in (self.agent, self.human):
            a = next(a for a in value["assessments"] if a["criterion_id"] == cid)
            a["assessment"] = "insufficient_evidence"
        result = self.result()
        self.assertEqual(result["replacement_qualification"], "unresolved")
        self.assertIn(cid, result["qualitative_review"]["unresolved_criteria"])
        self.assertEqual(result["qualitative_review"]["violated_criteria"], [])
        self.assertFalse(result["replacement_pass_candidate"])

    def test_interaction_coverage_requires_independent_agent_review_and_blocks_violation(self):
        cid = "campaign/campaign_interaction/interaction_coverage_adequacy"
        a = next(a for a in self.agent["assessments"] if a["criterion_id"] == cid)
        a["assessment"] = "not_reviewed"
        self.assertEqual(self.result()["replacement_qualification"], "unresolved")
        for value in (self.agent, self.human):
            next(a for a in value["assessments"] if a["criterion_id"] == cid)["assessment"] = "violated"
        self.assertEqual(self.result()["replacement_qualification"], "blocked")

    def test_coverage_gap_requires_targeted_human_resolution(self):
        cid = policy.COVERAGE_CRITERION
        criterion = next(a for a in self.agent["assessments"] if a["criterion_id"] == cid)
        fixtures.fill(criterion, self.prep, "insufficient_evidence")
        handoff = review.validate_value(self.prep, "d" * 64, self.agent)["completion_preflight"]
        self.assertIn(cid, handoff["targeted_escalations"]["high_impact_insufficient_criterion_ids"])
        other_cid = next(s["criterion_id"] for s in self.specs if s["group"] == "context_recovery")
        for resolutions in ({}, {cid: ["f" * 32]},
                {other_cid: [self.agent["reviewer"]["run_id"]]}):
            self.human["resolves_review_runs"] = resolutions
            result = self.result()
            self.assertEqual(result["replacement_qualification"], "unresolved")
            self.assertIn(cid, result["qualitative_review"]["human_escalations"])
            self.assertIn(cid, result["qualitative_review"]["unresolved_criteria"])
        self.human["resolves_review_runs"] = {}
        self.human["resolves_review_runs"][cid] = [self.agent["reviewer"]["run_id"]]
        human_result = review.validate_value(self.human_prep, "d" * 64, self.human)
        self.assertIn(cid, human_result["completion_preflight"]["targeted_escalations"]
            ["declared_conflict_resolution_criterion_ids"])
        self.assertEqual(self.result()["replacement_qualification"], "qualified")
        # Even an explicit resolution cannot supply the independent agent judgment.
        self.agent["assessments"][self.agent["assessments"].index(criterion)] = review.observation(cid)
        review.validate_value(self.prep, "d" * 64, self.agent)
        result = self.result()
        self.assertEqual(result["replacement_qualification"], "unresolved")
        self.assertIn(cid, result["qualitative_review"]["unresolved_criteria"])

    def test_required_interaction_criterion_cannot_be_omitted(self):
        with self.assertRaisesRegex(ValueError, "requires interaction coverage"):
            policy.combine(self.evaluation, [s for s in self.specs if s["name"] != "interaction_coverage_adequacy"],
                [self.agent, self.human], self.technical)

    def test_learning_intent_is_not_an_optional_not_observed_shortcut(self):
        spec = {"group": "interaction", "name": "learning_fork_value", "workload_intent": "learning_collaborative"}
        self.assertFalse(policy.nonblocking_not_observed(spec))
        spec["workload_intent"] = "routine_bounded"
        self.assertTrue(policy.nonblocking_not_observed(spec))

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

    def test_summary_uses_only_exact_volicord_multi_work_criterion(self):
        cid = policy.MULTI_WORK_CRITERION
        other = "journey-small-python/viewer_snapshot/multiple_work_organization"
        spec = copy.deepcopy(next(s for s in self.specs if s["criterion_id"] == cid))
        spec.update(criterion_id=other, sample_id="journey-small-python")
        self.specs.append(spec)
        assessment = copy.deepcopy(next(a for a in self.agent["assessments"] if a["criterion_id"] == cid))
        assessment.update(criterion_id=other, assessment="satisfied")
        self.agent["assessments"].append(assessment)
        for value in (self.agent, self.human):
            next(a for a in value["assessments"] if a["criterion_id"] == cid)["assessment"] = "not_reviewed"
        result = self.result()
        self.assertIn("journey-small-python/viewer_snapshot/multiple_work_organization",
            result["qualitative_review"]["resolved_criteria"])
        self.assertEqual(result["naturalistic_evidence"]["multi_work_viewer_comprehension"]["state"],
            "unresolved")
        self.assert_valid_summary(result, "multi_work_viewer_comprehension")

    def test_browser_summary_requires_every_exact_locale_and_preserves_violation(self):
        ids = policy.browser_criteria()
        self.assertEqual(set(ids), {s["criterion_id"] for s in self.specs
            if s["group"] == "live_viewer" and s["name"] == "browser_input_and_paint_responsiveness"})
        for cid in ids:
            next(a for a in self.agent["assessments"] if a["criterion_id"] == cid)["assessment"] = "not_reviewed"
        for states, expected in ((("satisfied", "not_reviewed"), "unresolved"),
                (("satisfied", "violated"), "violated"),
                (("not_reviewed", "violated"), "violated"),
                (("satisfied", "satisfied"), "satisfied")):
            with self.subTest(states=states):
                for cid, state in zip(ids, states):
                    next(a for a in self.human["assessments"] if a["criterion_id"] == cid)["assessment"] = state
                result = self.result()
                self.assertEqual(result["naturalistic_evidence"]["live_browser_input_and_paint"]["state"], expected)
                self.assert_valid_summary(result, "live_browser_input_and_paint")
        # Neither another repository nor a similar locale can supply a required ID.
        for impostor in (ids[1].replace("journey-volicord", "journey-small-python"),
                ids[1].replace("/ko/", "/ko-extra/")):
            result = self.result()
            result["qualitative_review"]["resolved_criteria"].remove(ids[1])
            result["qualitative_review"]["resolved_criteria"].append(impostor)
            self.assertEqual(policy._criterion_state(result, ids), "unresolved")

    def assert_valid_summary(self, result, summary):
        value = {"kind": "phase8_dogfood_result", "schema_version": 3,
            "policy": policy.identity(), "candidate_head": "a" * 40,
            "evaluator_revision": "b" * 40, "run_nonce": "c" * 32, **result}
        value["run_id"] = m.digest(value)
        policy.validate_result(value)
        value["naturalistic_evidence"][summary]["state"] = "satisfied" if (
            value["naturalistic_evidence"][summary]["state"] != "satisfied") else "unresolved"
        value["run_id"] = m.digest({k: v for k, v in value.items() if k != "run_id"})
        with self.assertRaisesRegex(ValueError, "naturalistic evidence"):
            policy.validate_result(value)

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
        import campaign
        import campaign_self_test as campaign_fixtures
        cls.binary = parent / 'bin/volicord'
        campaign_fixtures.write_fake_binary(cls.binary)
        cls.artifacts = campaign.bind_candidate_artifacts(cls.binary)
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
            def rehearsal(head, final):
                owners.counts['rehearsal'] += 1
                fake = runpy.run_path(str(root / 'rebuild/validation/dogfood/rehearsal_test_support.py'))
                value, execution, path = fake['fake_owner'](kwargs['gate_directory'] / 'dogfood-rehearsal', head,
                    gate.sha256(final), kwargs['gate_directory'].name)
                value.update(gate.REHEARSAL['expected_from_dependencies'](admitted['dependency_snapshot']['dogfood_rehearsal']))
                # Bind the synthetic expensive owner's archive to the maintained
                # support executables used by the actual preparation consumer.
                hashes = {name: binding['sha256'] for name, binding in cls.artifacts.items()}
                value['executables'] = hashes
                value['pipeline']['executables'] = hashes
                boundaries = value['pipeline']['boundary_evidence']
                boundaries['retention']['result']['candidate_cli_sha256'] = hashes['volicord']
                for route in boundaries['launch']['routes']:
                    route.update(cli_sha256=hashes['volicord'], mcp_sha256=hashes['volicord-mcp'])
                boundaries['resource']['result']['candidate_mcp_sha256'] = hashes['volicord-mcp']
                value['result_id'] = gate.REHEARSAL['digest']({k: v for k, v in value.items() if k != 'result_id'})
                path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
                execution['working_directory'] = str(candidate)
                execution['argv'][0] = str(candidate / 'rebuild/validation/dogfood/rehearsal.py')
                (kwargs['gate_directory'] / 'dogfood-rehearsal-invocation/result.json').write_text(json.dumps(execution))
                return value, execution, path

            kwargs["rehearsal_owner"] = rehearsal
            return original_orchestrate(**kwargs,
                contract_execution_owner=lambda summary: gate.check(
                    'contract_coverage_execution', 'passed', 'synthetic execution coverage',
                    execution_owner='exact_candidate_final_workspace_tests', mapped_test_count=12))

        def aggregate(label, commands):
            assert label == 'final' and commands == namespace['FINAL_COMMANDS']
            summary, path = owners.final()
            summary['working_directory'] = str(candidate)
            summary['started_at'] = '2026-08-21T00:00:00.000000+00:00'
            summary['ended_at'] = '2026-08-21T00:00:01.000000+00:00'
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
            assert owners.counts == {'rehearsal': 1, 'final': 1, 'provider': 1, 'preflight': 1, 'v11': 1, 'audit': 1}
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

    def test_preparation_and_qualification_reuse_without_technical_execution(self):
        import campaign as c
        import campaign_self_test as support
        import resource_observer
        import result_lineage
        import subprocess
        import review_operations as ops
        from review_operations_self_test import insufficient_draft
        import contextlib
        import io
        import sys
        before = self.owners.counts.copy()
        def entry(*arguments):
            stdout = io.StringIO()
            with patch.object(sys, 'argv', ['dogfood-campaign', *map(str, arguments)]), contextlib.redirect_stdout(stdout):
                self.assertEqual(c.main(), 0)
            return json.loads(stdout.getvalue())
        # Only support Product collection invokes its fake executables. The reuse
        # boundary itself may inspect Git identity but must execute no technical owner.
        original_run = subprocess.run
        original_popen = subprocess.Popen
        def read_only_spawn(argv, *args, **kwargs):
            if Path(argv[0]).name != 'git':
                raise AssertionError(f'process spawned during evidence reuse: {argv[0]}')
            return original_popen(argv, *args, **kwargs)
        def read_only_git(argv, *args, **kwargs):
            if Path(argv[0]).name != 'git':
                raise AssertionError(f'technical execution during evidence reuse: {argv[0]}')
            return original_run(argv, *args, **kwargs)
        with patch.object(resource_observer, 'observe', side_effect=AssertionError('observer started')), \
             patch.object(c, 'install_candidate', side_effect=AssertionError('candidate rebuilt')), \
             patch.object(subprocess, 'run', side_effect=read_only_git), \
             patch.object(subprocess, 'Popen', side_effect=read_only_spawn):
            # Fixture cloning and identity qualification are authored support;
            # prepare_campaign and verify_technical are the real consumers.
            with patch.object(harness, 'git_head', return_value=self.candidate):
                support.prepare(Path(self.temp.name) / 'reuse-campaign',
                    Path(self.temp.name) / 'reuse-sources', self.binary,
                    capsule_path=self.path, archive_path=self.archive)
        root = Path(self.temp.name) / 'reuse-campaign'
        prep = c.read_json(root / 'preparation.json')
        self.assertEqual(prep['technical_gate']['state'], 'passed')
        self.assertEqual(prep['technical_gate']['execution'], 'reused')
        self.assertEqual(prep['resource_observation'], 'not_selected')
        # Repeat unchanged preparation for a different campaign label. This
        # supplies no new candidate qualification or host-readiness claim.
        with patch.object(harness, 'git_head', return_value=self.candidate), \
             patch.object(c, 'install_candidate', side_effect=AssertionError('repeat rebuilt candidate')), \
             patch.object(subprocess, 'run', side_effect=read_only_git), \
             patch.object(subprocess, 'Popen', side_effect=read_only_spawn):
            support.prepare(Path(self.temp.name) / 'reuse-repeat-campaign',
                Path(self.temp.name) / 'reuse-repeat-sources', self.binary,
                capsule_path=self.path, archive_path=self.archive)
        self.assertEqual(self.owners.counts, before)
        # Build the explicitly authored raw support inputs before exercising
        # collection; fixture construction is not campaign execution.
        # Build/collect the eight raw support slots through maintained consumers.
        captures, bundles = [], {}
        with patch.object(harness, 'git_head', return_value=self.candidate), \
             patch.object(harness, 'git_clean', return_value=True), \
             patch.object(resource_observer, 'observe', side_effect=AssertionError('collection started observer')):
            for kind in c.CLASSES:
                for label in c.work_labels(kind):
                    _, work, resume, bundle = support.fixture_for(
                        Path(self.temp.name) / 'reuse-fixtures', kind, label, campaign_root=root)
                    captures.append(work)
                    if 'resume' in c.session_roles(kind, label): captures.append(resume)
                    bundles[c.work_key(kind, label)] = bundle
            with patch.object(c, 'run_checked', side_effect=support.fake_enable_command):
                activation = entry('activate-all', '--campaign-root', root)
            self.assertTrue(all(j['integration_execution'] == 'generated' for j in activation['journeys']))
            configs = {path: path.read_bytes() for path in root.glob('journeys/*/repository/.codex/*')}
            with patch.object(c, 'run_checked', side_effect=AssertionError('repeat changed reviewed integration')):
                activation = entry('activate-all', '--campaign-root', root)
            self.assertTrue(all(j['integration_execution'] == 'reused' for j in activation['journeys']))
            self.assertEqual(configs, {path: path.read_bytes() for path in configs})
            # A reviewed integration mismatch remains a blocker, with no hidden repair.
            path = next(path for path in configs if path.name == 'volicord-integration.json')
            original = path.read_bytes()
            changed = json.loads(original); changed['runtime'] = '/wrong-runtime'
            path.write_text(json.dumps(changed))
            with patch.object(c, 'run_checked', side_effect=AssertionError('silent repair')), self.assertRaises(ValueError):
                entry('activate-all', '--campaign-root', root)
            path.write_bytes(original)
            with patch.object(subprocess, 'run', side_effect=read_only_git), \
                 patch.object(subprocess, 'Popen', side_effect=read_only_spawn), \
                 patch.object(resource_observer, 'start_identity', side_effect=AssertionError('telemetry PID lookup')), \
                 patch.object(resource_observer, 'sample', side_effect=AssertionError('telemetry sampling')):
                with patch.dict(c.collect_batch.__kwdefaults__, {'exporter': support.batch_exporter(bundles),
                        'documenter': support.documenter, 'snapshotter': support.snapshotter}):
                    entry('collect-batch', '--campaign-root', root,
                        *[argument for capture in captures for argument in ('--raw-rollout', capture)])
                evaluation = entry('evaluate', '--campaign-root', root)
        evaluation_path = root / evaluation['evaluation']
        target = Path(self.temp.name) / 'reuse-review'
        with patch.object(resource_observer, 'observe', side_effect=AssertionError('review started observer')):
            with patch.object(subprocess, 'run', side_effect=read_only_git), \
                 patch.object(subprocess, 'Popen', side_effect=read_only_spawn):
                entry('prepare-qualitative-review', '--campaign-root', root, '--output', target,
                    '--reviewer-kind', 'agent', '--review-session-id', 'independent-support-review',
                    '--machine-evaluation', evaluation_path)
                insufficient_draft(target)
                entry('record-qualitative-review', '--review-root', target, '--draft', target / 'draft.json')
        output = Path(self.temp.name) / 'reuse-qualification'
        with patch.object(subprocess, 'run', side_effect=read_only_git), \
             patch.object(subprocess, 'Popen', side_effect=read_only_spawn), \
             patch.object(resource_observer, 'observe', side_effect=AssertionError('qualification started observer')):
            value = entry('qualify', '--campaign-root', root, '--machine-evaluation', evaluation_path,
                '--output', output, '--candidate-head', self.candidate, '--review-root', target,
                '--gate-capsule', self.path, '--gate-archive', self.archive)
        self.assertEqual(value['technical_gate']['state'], 'passed')
        self.assertEqual(value['naturalistic_evidence']['naturalistic_resource']['status'], 'not_observed')
        self.assertEqual(value['naturalistic_evidence']['naturalistic_resource']['measurement']['peak_rss_bytes'], None)
        self.assertFalse(value['replacement_pass_candidate'])
        self.assertIn(policy.COVERAGE_CRITERION, value['qualitative_review']['unresolved_criteria'])
        self.assertTrue(value['qualitative_review']['human_escalations'])
        with patch.object(subprocess, 'run', side_effect=read_only_git), \
             patch.object(subprocess, 'Popen', side_effect=read_only_spawn):
            published = entry('publish-result-lineage', '--campaign-root', root,
                '--machine-evaluation', evaluation_path, '--review-root', target,
                '--qualification', output / 'qualification.json')
        copied = Path(self.temp.name) / 'reuse-copied'
        shutil.copytree(published['lineage_root'], copied)
        with patch.object(c, 'load_evidence_set', side_effect=AssertionError('original campaign used')):
            with patch.object(subprocess, 'run', side_effect=read_only_git), \
                 patch.object(subprocess, 'Popen', side_effect=read_only_spawn):
                entry('verify-result-lineage', '--lineage-root', copied)
        copied_value = json.loads((copied / 'qualification/qualification.json').read_bytes())
        self.assertEqual(copied_value['naturalistic_evidence']['naturalistic_resource']['status'], 'not_observed')
        self.assertEqual(self.owners.counts, before)

    def test_candidate_executable_binding_remains_required(self):
        self.assertEqual(policy.verify_technical(self.candidate, self.path, self.archive,
            candidate_artifacts=self.artifacts)['state'], 'passed')
        for name in self.artifacts:
            changed = copy.deepcopy(self.artifacts)
            changed[name]['sha256'] = 'f' * 64
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'candidate executable mismatch'):
                policy.verify_technical(self.candidate, self.path, self.archive, candidate_artifacts=changed)

    def test_preparation_rejects_inapplicable_supplied_technical_evidence(self):
        import campaign as c
        import campaign_self_test as support
        tampered = self.path.parent / 'preparation-tampered.tar.gz'
        tampered.write_bytes(self.archive.read_bytes()[:100])
        incompatible = self.path.parent / 'preparation-incompatible.json'
        changed = copy.deepcopy(self.capsule)
        changed['dogfood_rehearsal']['contract'] = 'incompatible-producing-contract'
        incompatible.write_bytes(policy.operations.encoded(changed))
        cases = [('missing-pair', self.path, None, self.candidate),
            ('failed', self.failed_path, self.failed_archive, self.candidate),
            ('tampered', self.path, tampered, self.candidate),
            ('incompatible', incompatible, self.archive, self.candidate),
            ('wrong-candidate', self.path, self.archive, 'f' * 40)]
        counts = self.owners.counts.copy()
        with patch.object(c, 'install_candidate', side_effect=AssertionError('technical prerequisite rebuilt candidate')):
            for label, capsule, archive, candidate in cases:
                root = Path(self.temp.name) / f'prepare-{label}'
                with self.subTest(label=label), patch.object(harness, 'git_head', return_value=candidate):
                    with self.assertRaises((ValueError, OSError, EOFError)):
                        support.prepare(root, Path(self.temp.name) / f'sources-{label}', self.binary,
                            capsule_path=capsule, archive_path=archive)
                    self.assertFalse((root / 'preparation.json').exists())
        self.assertEqual(self.owners.counts, counts)

    def test_opt_in_records_selection_without_starting_observation(self):
        import campaign as c
        import campaign_self_test as support
        import resource_observer as observer
        root = Path(self.temp.name) / 'selected-not-started'
        with patch.object(observer, 'observe', side_effect=AssertionError('selection started observation')):
            support.prepare(root, Path(self.temp.name) / 'selected-sources', self.binary, observe_resources=True)
        preparation = c.read_json(root / 'preparation.json')
        self.assertEqual(preparation['resource_observation'], 'selected')
        self.assertEqual(preparation['technical_gate']['state'], 'not_provided')
        resource = preparation['naturalistic_memory_evidence']
        self.assertEqual(resource, observer.initial(preparation['candidate_artifacts']))
        self.assertEqual(resource['observer_lifecycle'], 'not_started')
        self.assertIsNone(resource['measurement']['peak_rss_bytes'])
        self.assertIn('explicitly selected', (root / 'operator/RUN-SHEET.md').read_text())

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
            'missing_rehearsal': lambda v: v.pop('dogfood_rehearsal'),
            'unstarted_rehearsal': lambda v: v['dogfood_rehearsal'].update(status='not_run'),
            'rehearsal_execution': lambda v: v['dogfood_rehearsal'].update(execution=None),
            'rehearsal_inner': lambda v: v['dogfood_rehearsal']['result']['pipeline'].update(expected_inner_verdict='qualified'),
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

    def test_host_metadata_user_prose_and_history_replay_from_copied_inputs(self):
        import campaign as c
        import campaign_self_test as support
        import result_lineage
        import review_operations as ops
        import review_captures
        from review_operations_self_test import ProjectionTests, rollout_bytes, insufficient_draft
        from review_meaning_self_test import rehash_package
        binary = Path(c.load_campaign(self.root)['candidate_binary'])
        with patch.object(harness, 'git_clean', return_value=True):
            root, raw, bundles = support.prepared_batch(self.parent, 'typed-host-lineage-campaign', binary)
            for path in raw:
                events = __import__('codex_events').capture_events(path.read_bytes())
                terminal = max(i for i, e in enumerate(events) if e['payload'].get('type') in {'task_complete', 'task_completed'})
                turn = events[terminal]['payload']['turn_id']
                page = ProjectionTests().page_event()
                page['payload']['internal_chat_message_metadata_passthrough']['turn_id'] = turn
                # No source session is reused or measured: these are authored support bytes.
                user_text = 'Explain this literal tag: ' + page['payload']['content'][0]['text']
                metadata = {'turn_id': turn}
                events[terminal:terminal] = [page, {'type': 'event_msg', 'payload': {'type': 'user_message',
                    'message': user_text, 'client_id': 'fixture-literal-user'}},
                    {'type': 'response_item', 'payload': {'type': 'custom_tool_call', 'name': 'exec',
                        'status': 'completed', 'call_id': 'unsupported-fixture-shell',
                        'input': 'text(await tools.exec_command({cmd:load("private-command")}));',
                        'internal_chat_message_metadata_passthrough': metadata}},
                    {'type': 'response_item', 'payload': {'type': 'custom_tool_call_output',
                        'call_id': 'unsupported-fixture-shell',
                        'output': [{'type': 'input_text', 'text': 'Script completed\nOutput:\n'},
                            {'type': 'input_text', 'text': '{"output":"bounded fixture","exit_code":0}'}],
                        'internal_chat_message_metadata_passthrough': metadata}}]
                path.write_bytes(rollout_bytes(events))
            c.collect_batch(root, raw, exporter=support.batch_exporter(bundles),
                documenter=support.documenter, snapshotter=support.snapshotter)
        old = c.evaluate_campaign(root, self.parent / 'typed-host-history')
        previous = Path(old['evaluation']); old_bytes = previous.read_bytes()
        new = c.evaluate_campaign(root, self.parent / 'typed-host-current', previous=previous)
        evaluation = Path(new['evaluation'])
        # Real execution coverage uses immutable Python tuples; publication stores
        # JSON arrays. Copied replay must compare the same serialized meaning.
        manifest = c.load_evidence_set(root)
        replayed = c.evaluate_works(root, manifest)
        published_works = json.loads(evaluation.read_bytes())['works']
        limits = replayed[0]['observation']['machine_facts']['required_validation_execution']['basis']['execution_coverage_limits']
        self.assertTrue(limits)
        self.assertIsInstance(limits[0]['reasons'], tuple)
        self.assertNotEqual(replayed, published_works)
        self.assertEqual(ops.encoded(replayed), ops.encoded(published_works))
        target = self.parent / 'typed-host-review'
        ops.prepare(root, target, reviewer_kind='agent', session_id='authored-separate-host-review',
            evaluation_path=evaluation, include_raw=True)
        prepared, _, _ = ops.load_package(target)
        entries = [(k, v) for k, v in prepared['index']['evidence'].items()
            if v['surface'] in review_captures.CAPTURE_SURFACES]
        self.assertEqual(len(entries), 8)
        for _, entry in entries:
            capture = review_captures.validate((target / entry['path']).read_bytes())
            self.assertEqual(len(capture['host_metadata']), 1)
            self.assertEqual(capture['execution_coverage']['unsupported_wrapper_count'], 1)
            self.assertIn(user_text, [r['body']['value'] for r in capture['records']
                if r['semantic_role'] == 'user_turn'])
        # Source-independent review package verifies while original paths are unavailable.
        copied_review = self.parent / 'typed-host-review-copy'; shutil.copytree(target, copied_review)
        hidden = root.with_name(root.name + '-unavailable'); root.rename(hidden)
        try:
            ops.load_package(copied_review)
        finally:
            hidden.rename(root)
        insufficient_draft(target); ops.record(target, target / 'draft.json')
        qualification = self.parent / 'typed-host-qualification'
        policy.qualify(root, evaluation, qualification, candidate=c.load_campaign(root)['candidate_head'],
            review_roots=[target])
        lineage = self.parent / 'typed-host-lineage'
        result_lineage.publish(root, evaluation, [target], qualification / 'qualification.json',
            output=lineage, previous_evaluation=previous)
        copied = self.parent / 'typed-host-lineage-copy'; shutil.copytree(lineage, copied)
        hidden_parent = self.parent.with_name(self.parent.name + '-unavailable')
        with tempfile.TemporaryDirectory() as detached_parent:
            detached = Path(detached_parent) / 'lineage'; shutil.copytree(copied, detached)
            self.parent.rename(hidden_parent)
            try:
                self.assertFalse(result_lineage.verify(detached)['external_staging_paths_used'])
                changed = copy.deepcopy(replayed)
                changed[0]['observation']['machine_facts']['required_validation_execution']['basis']['execution_coverage_limits'][0]['reasons'] += ('invented_coverage_reason',)
                with patch.object(c, 'evaluate_works', return_value=changed), self.assertRaisesRegex(
                        ValueError, 'recomputed immutable observations'):
                    result_lineage.verify(detached)
            finally:
                hidden_parent.rename(self.parent)
        self.assertEqual(previous.read_bytes(), old_bytes)
        with self.assertRaises(ValueError): c.evaluate_campaign(root, previous.parent)
        self.assertEqual(previous.read_bytes(), old_bytes)
        # Fully refresh all outer wrappers: immutable-source replay must still reject.
        for control in ('drop_host', 'user_as_host', 'false_answer_pass', 'evaluator', 'history', 'source'):
            tampered = self.parent / ('typed-host-tamper-' + control); shutil.copytree(copied, tampered)
            index = json.loads((tampered / 'index.json').read_bytes())
            if control in {'drop_host', 'user_as_host'}:
                reference = index['qualitative_reviews'][0]
                review_root = tampered / reference['root']
                prep = json.loads((review_root / 'preparation.json').read_bytes())
                eid = next(k for k, v in prep['index']['evidence'].items() if v['surface'] == 'work_capture')
                path = review_root / prep['index']['evidence'][eid]['path']; capture = json.loads(path.read_bytes())
                if control == 'drop_host':
                    sequence = capture['host_metadata'][0]['sequence']; capture['host_metadata'] = []
                    capture['excluded_records'] = [({'sequence': sequence, 'reason': 'non_semantic_by_design'}
                        if r['sequence'] == sequence else r) for r in capture['excluded_records']]
                else:
                    user = next(r for r in capture['records'] if r['semantic_role'] == 'user_turn'
                        and r['body']['value'].startswith('Explain this literal tag:'))
                    capture['records'].remove(user)
                    host = {**capture['host_metadata'][0], 'sequence': user['sequence'], 'message_id': 'false-host'}
                    capture['host_metadata'].append(host)
                    capture['host_metadata'].sort(key=lambda r: r['sequence'])
                    capture['excluded_records'].append({**host, 'reason': 'host_page_no_selection'})
                    capture['excluded_records'].sort(key=lambda r: r['sequence'])
                    capture.update(review_captures.counts(capture))
                rehash_package(review_root, eid, ops.encoded(capture))
                package = json.loads((review_root / 'package.json').read_bytes())
                prep = json.loads((review_root / 'preparation.json').read_bytes())
                value = json.loads((review_root / 'recorded/review.json').read_bytes())
                value['preparation_sha256'] = package['preparation_sha256']; data = ops.encoded(value)
                receipt = json.loads((review_root / 'recorded/receipt.json').read_bytes())
                receipt.update(preparation_sha256=package['preparation_sha256'], review_sha256=ops.digest(data),
                    result=review.validate_value(prep, package['preparation_sha256'], value))
                for name, body in (('recorded/review.json', data), ('recorded/receipt.json', ops.encoded(receipt))):
                    p = review_root / name; p.chmod(0o600); p.write_bytes(body)
                reference.update(preparation_sha256=package['preparation_sha256'],
                    review_sha256=ops.digest(data), package_id=package['package_id'])
                qpath = tampered / 'qualification/qualification.json'; qvalue = json.loads(qpath.read_bytes())
                qvalue['qualitative_review_runs'][0].update(preparation_sha256=package['preparation_sha256'],
                    review_sha256=ops.digest(data))
                qvalue['run_id'] = m.digest({k: v for k, v in qvalue.items() if k != 'run_id'})
                qpath.chmod(0o600); qpath.write_bytes(ops.encoded(qvalue))
                index['qualification'].update(run_id=qvalue['run_id'], sha256=ops.digest(qpath.read_bytes()))
                expected_error = 'conversation projection differs'
            else:
                path = tampered / 'evaluation/evaluation.json'
                value = json.loads(path.read_bytes())
                if control == 'false_answer_pass':
                    work = value['works'][0]
                    work['observation']['machine_facts']['shared_answer_integrity']['status'] = 'confirmed_pass'
                    expected_error = 'findings do not preserve'
                elif control == 'evaluator':
                    value['evaluator_revision'] = '0' * 40
                    expected_error = 'evaluation binding changed'
                elif control == 'history':
                    value['previous_evaluation']['sha256'] = '0' * 64
                    expected_error = 'historical evaluation reference changed'
                else:
                    origin = entries[0][1]['origin']['path']
                    path = tampered / 'source' / origin; path.chmod(0o600); path.write_bytes(path.read_bytes() + b'\n')
                    expected_error = 'source|raw|artifact|binding'
                if control != 'source':
                    value['run_id'] = m.digest({k: v for k, v in value.items() if k != 'run_id'})
                    path.chmod(0o600); path.write_bytes(ops.encoded(value))
                    receipt_path = path.with_name('receipt.json'); receipt_path.chmod(0o600)
                    receipt_path.write_bytes(ops.encoded({'kind': 'dogfood_evaluation_receipt',
                        'run_id': value['run_id'], 'evaluation_sha256': ops.digest(path.read_bytes())}))
            index['lineage_id'] = m.digest({k: v for k, v in index.items() if k != 'lineage_id'})
            path = tampered / 'index.json'; path.chmod(0o600); path.write_bytes(ops.encoded(index))
            receipt = json.loads((tampered / 'receipt.json').read_bytes())
            receipt.update(lineage_id=index['lineage_id'], index_sha256=ops.digest(path.read_bytes()))
            receipt['artifacts'] = {name: result_lineage._binding((tampered / name).read_bytes()) for name in receipt['artifacts']}
            path = tampered / 'receipt.json'; path.chmod(0o600); path.write_bytes(ops.encoded(receipt))
            with self.subTest(control=control), self.assertRaisesRegex(ValueError, expected_error):
                result_lineage.verify(tampered)

    def test_partial_resource_attachment_is_preserved_and_malformed_input_rejected(self):
        import campaign as c
        import campaign_self_test as support
        import resource_observer as observer
        import result_lineage
        from resource_coverage_self_test import observation, omit
        binary = Path(c.load_campaign(self.root)['candidate_binary'])
        with patch.object(harness, 'git_clean', return_value=True):
            root, raw, bundles = support.prepared_batch(self.parent, 'partial-resource-campaign', binary,
                observe_resources=True)
            campaign = c.load_campaign(root)
            resource = observation(2, candidate=campaign['candidate_artifacts']['volicord-mcp']['sha256'],
                purpose='naturalistic')
            journey = campaign['journeys']['journey-volicord']
            runtime = observer.path_binding(Path(journey['runtime_home']))
            resource['runtimes'][0]['runtime_binding'] = runtime
            for instance in resource['instances']:
                instance['identity'].update(runtime_binding=runtime,
                    cwd_binding=observer.path_binding(Path(journey['repository_path'])))
            for tick in resource['ticks']: tick['runtimes'][0]['runtime_binding'] = runtime
            invalid = omit(copy.deepcopy(resource))
            source = self.parent / 'invalid-resource.json'
            c.write_json(source, invalid)
            with self.assertRaisesRegex(ValueError, 'running instance lacks tick sample'):
                c.record_resources(root, source)
            self.assertFalse((root / 'resources/observation.json').exists())
            # A scoped failed tick preserves other genuine samples as partial.
            partial = omit(copy.deepcopy(resource))
            partial['ticks'][3]['runtimes'][0]['registered'][0]['state'] = 'inaccessible'
            partial['instances'][0]['errors'] = ['inaccessible']
            partial['measurement']['measurement_errors'] = ['inaccessible']
            partial['status'] = 'partial'
            for label, mutate in (
                ('candidate', lambda v: v.update(candidate_mcp_sha256='f' * 64)),
                ('privacy', lambda v: v['privacy'].update(credentials_retained=True)),
                ('binding', lambda v: v['instances'][0]['identity'].update(cwd_binding='f' * 64))):
                invalid = copy.deepcopy(partial); mutate(invalid); c.write_json(source, invalid)
                with self.subTest(label=label), self.assertRaises(ValueError): c.record_resources(root, source)
            c.write_json(source, partial)
            c.record_resources(root, source)
            self.assertEqual(c.load_campaign(root)['naturalistic_memory_evidence'], partial)
            c.write_json(source, observer.initial(campaign['candidate_artifacts']))
            with self.assertRaisesRegex(ValueError, 'immutable'): c.record_resources(root, source)
            c.collect_batch(root, raw, exporter=support.batch_exporter(bundles),
                documenter=support.documenter, snapshotter=support.snapshotter)
            evaluation = c.evaluate_campaign(root)
        output = self.parent / 'partial-resource-qualified'
        value = policy.qualify(root, root / evaluation['evaluation'], output, candidate=campaign['candidate_head'])
        self.assertEqual(value['naturalistic_evidence']['naturalistic_resource'], partial)
        published = result_lineage.publish(root, root / evaluation['evaluation'], [], output / 'qualification.json')
        copied = self.parent / 'partial-resource-lineage'
        shutil.copytree(published['lineage_root'], copied)
        result_lineage.verify(copied)
        retained = json.loads((copied / 'qualification/qualification.json').read_bytes())
        self.assertEqual(retained['naturalistic_evidence']['naturalistic_resource'], partial)

    def test_copied_lineage_rejects_rehashed_missing_process_sample(self):
        import campaign
        import result_lineage
        import review_operations as ops
        from resource_coverage_self_test import observation, omit
        output = self.parent / 'resource-qualification'
        value = policy.qualify(self.root, self.evaluation, output,
            candidate=campaign.load_campaign(self.root)['candidate_head'])
        copied = self.parent / 'resource-copied-lineage'
        result_lineage.publish(self.root, self.evaluation, [], output / 'qualification.json', copied)
        self.assertFalse(result_lineage.verify(copied)['external_staging_paths_used'])
        # Refresh all qualification/index/receipt hashes. The resource primitive
        # contradiction must fail before any later equality with campaign inputs.
        changed = copy.deepcopy(value)
        changed['naturalistic_evidence']['naturalistic_resource'] = omit(observation(2, purpose='naturalistic'))
        changed['run_id'] = m.digest({k: v for k, v in changed.items() if k != 'run_id'})
        path = copied / 'qualification/qualification.json'
        path.chmod(0o600); path.write_bytes(ops.encoded(changed))
        index = json.loads((copied / 'index.json').read_bytes())
        index['qualification'].update(run_id=changed['run_id'], sha256=ops.digest(path.read_bytes()))
        index['lineage_id'] = m.digest({k: v for k, v in index.items() if k != 'lineage_id'})
        data = ops.encoded(index)
        receipt = json.loads((copied / 'receipt.json').read_bytes())
        receipt.update(lineage_id=index['lineage_id'], index_sha256=ops.digest(data))
        receipt['artifacts'] = {name: result_lineage._binding((copied / name).read_bytes())
            for name in receipt['artifacts']}
        for name, body in (('index.json', data), ('receipt.json', ops.encoded(receipt))):
            path = copied / name; path.chmod(0o600); path.write_bytes(body)
        with patch.object(campaign, 'load_evidence_set', side_effect=AssertionError('original campaign access')):
            with self.assertRaisesRegex(ValueError, 'running instance lacks tick sample'):
                result_lineage.verify(copied)

    def test_copied_human_display_lineage_uses_only_packaged_candidate_context(self):
        import campaign
        import human_review
        import result_lineage
        import review_operations as ops
        import viewer_observation
        from review_operations_self_test import insufficient_draft
        from viewer_observation_self_test import context_directory
        manifest = campaign.load_evidence_set(self.root)
        prefix = self.parent / "human-display-lineage"
        prefix.mkdir()
        contexts = [context_directory(prefix, locale, manifest, locale) for locale in ("en", "ko")]
        observations = prefix / "observations"
        # Synthetic declarations are structural controls, never human evidence.
        human_review.capture_viewer_observations(self.root, observations, context_paths=contexts,
            input_fn=iter(["1", "OBSERVATION:\nSynthetic declaration fixture.\nLIMITS:\nNo real human verdict.",
                "1", "SAME AS ENGLISH"]).__next__, output_fn=lambda _: None)
        target = prefix / "review"
        ops.prepare(self.root, target, reviewer_kind="human", human_observations=observations,
            evaluation_path=self.evaluation)
        insufficient_draft(target)
        ops.record(target, target / "draft.json")
        qualified = prefix / "qualification"
        value = policy.qualify(self.root, self.evaluation, qualified,
            candidate=manifest["candidate_head"], review_roots=[target])
        self.assertNotEqual(value["replacement_qualification"], "qualified")
        published = result_lineage.publish(self.root, self.evaluation, [target],
            qualified / "qualification.json", output=prefix / "lineage")
        copied = prefix / "copied"
        shutil.copytree(published["lineage_root"], copied)
        with patch.object(campaign, "load_evidence_set", side_effect=AssertionError("original Runtime/campaign access")), \
             patch.object(viewer_observation, "for_manifest", wraps=viewer_observation.for_manifest) as validate:
            verified = result_lineage.verify(copied)
        self.assertFalse(verified["external_staging_paths_used"])
        self.assertEqual(validate.call_count, 2)
        for call in validate.call_args_list:
            self.assertEqual(call.args[0]["candidate_head"], manifest["candidate_head"])
            self.assertEqual(call.args[1]["context"]["runtime_binding"],
                json.loads((contexts[0] / "display-context.json").read_bytes())["context"]["runtime_binding"])

    def test_real_evidence_reviews_result_revalidation_and_mismatch(self):
        import campaign
        import review_operations as ops
        from review_operations_self_test import insufficient_draft, snapshot
        before = snapshot(self.root)
        target = self.parent / 'incomplete-review'
        with patch.object(campaign.harness, 'load_v11', side_effect=AssertionError('V11 rerun')), \
             patch.object(policy, 'verify_technical', side_effect=AssertionError('technical verification during review')):
            ops.prepare(self.root, target, reviewer_kind='agent', session_id='separate-review', evaluation_path=self.evaluation, include_raw=True)
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
            technical_verifier.assert_called_once_with(campaign.load_campaign(self.root)['candidate_head'], None, None,
                candidate_artifacts=campaign.load_campaign(self.root)['candidate_artifacts'])
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
        # Physically hide the whole original campaign/staging tree, not just
        # its campaign API, while independently verifying a detached copy.
        with tempfile.TemporaryDirectory() as directory:
            detached = Path(directory) / 'lineage'
            shutil.copytree(lineage_root, detached)
            hidden = self.parent.with_name(self.parent.name + '-unavailable')
            self.parent.rename(hidden)
            try:
                self.assertEqual(result_lineage.verify(detached)['qualification_run_id'], value['run_id'])
            finally:
                hidden.rename(self.parent)
        lineage_qualification = json.loads(
            (copied / 'qualification/qualification.json').read_bytes())
        lineage_evidence = json.loads((copied / 'source/evidence-set.json').read_bytes())
        self.assertEqual(
            lineage_qualification['naturalistic_evidence']['naturalistic_resource'],
            lineage_evidence['naturalistic_memory_evidence'])
        # Refresh every copied wrapper, review receipt and qualification reference.
        # Rejection must come from inner returned-meaning/lifecycle consistency.
        from review_meaning_self_test import rehash_package
        import review_captures
        for control in ('resume_capture', 'explanation_lifecycle', 'explanation_plan_content', 'explanation_locator'):
            surface = 'resume_capture' if control == 'resume_capture' else 'explanation_lifecycle'
            tampered = self.parent / ('returned-meaning-tamper-' + control)
            shutil.copytree(lineage_root, tampered)
            index = json.loads((tampered / 'index.json').read_bytes())
            review_entry = index['qualitative_reviews'][0]
            review_root = tampered / review_entry['root']
            prepared = json.loads((review_root / 'preparation.json').read_bytes())
            eid = next(k for k, v in prepared['index']['evidence'].items() if v['surface'] == surface)
            entry = prepared['index']['evidence'][eid]
            content = json.loads((review_root / entry['path']).read_bytes())
            if surface == 'resume_capture':
                returned = next(v for v in content['records'] if v.get('operation') == 'recall')
                returned['body']['value']['returned_meaning']['value']['next_step'] = 'Corrupted captured action'
                returned['body'] = review_captures.body_projection(returned['body']['value'])
                expected_error = 'omissions/consistency changed'
            elif control == 'explanation_lifecycle':
                body = content['stages']['record']
                body['value']['value']['explanation']['realization']['paragraphs'][0]['text'] = 'Corrupted recorded claim'
                content['stages']['record'] = review_captures.body_projection(body['value'])
                expected_error = 'record receipt subject/revision/Source/provenance mismatch'
            elif control == 'explanation_plan_content':
                body = content['stages']['plan']
                body['value']['value']['plan']['evidence'][0]['content'] = 'Corrupted canonical evidence content'
                content['stages']['plan'] = review_captures.body_projection(body['value'])
                expected_error = 'copied lifecycle meaning/locators differ from source index'
            else:
                content['readback_subject_locators']['after'] = ['/work_history/999']
                expected_error = 'copied lifecycle meaning/locators differ from source index'
            rehash_package(review_root, eid, ops.encoded(content))
            package = json.loads((review_root / 'package.json').read_bytes())
            prepared = json.loads((review_root / 'preparation.json').read_bytes())
            review_value = json.loads((review_root / 'recorded/review.json').read_bytes())
            review_value['preparation_sha256'] = package['preparation_sha256']
            review_data = ops.encoded(review_value)
            receipt_value = json.loads((review_root / 'recorded/receipt.json').read_bytes())
            receipt_value.update(preparation_sha256=package['preparation_sha256'], review_sha256=ops.digest(review_data),
                result=review.validate_value(prepared, package['preparation_sha256'], review_value))
            for name, data in (('recorded/review.json', review_data), ('recorded/receipt.json', ops.encoded(receipt_value))):
                path = review_root / name; path.chmod(0o600); path.write_bytes(data)
            review_entry.update(preparation_sha256=package['preparation_sha256'],
                review_sha256=ops.digest(review_data), package_id=package['package_id'])
            qualification = json.loads((tampered / 'qualification/qualification.json').read_bytes())
            qualification['qualitative_review_runs'][0].update(preparation_sha256=package['preparation_sha256'],
                review_sha256=ops.digest(review_data))
            qualification['run_id'] = m.digest({k: v for k, v in qualification.items() if k != 'run_id'})
            qualification_data = ops.encoded(qualification)
            path = tampered / 'qualification/qualification.json'; path.chmod(0o600); path.write_bytes(qualification_data)
            index['qualification'].update(run_id=qualification['run_id'], sha256=ops.digest(qualification_data))
            index['lineage_id'] = m.digest({k: v for k, v in index.items() if k != 'lineage_id'})
            path = tampered / 'index.json'; path.chmod(0o600); path.write_bytes(ops.encoded(index))
            receipt = json.loads((tampered / 'receipt.json').read_bytes())
            receipt.update(lineage_id=index['lineage_id'], index_sha256=ops.digest(ops.encoded(index)))
            receipt['artifacts'] = {name: result_lineage._binding((tampered / name).read_bytes()) for name in receipt['artifacts']}
            path = tampered / 'receipt.json'; path.chmod(0o600); path.write_bytes(ops.encoded(receipt))
            with self.assertRaisesRegex(ValueError, expected_error):
                result_lineage.verify(tampered)
        # Change only a derived criterion, then also try a complete success claim.
        # Neither control changes the immutable recorded insufficient review.
        for promoted in (False, True):
            with self.subTest(promoted=promoted):
                tampered = self.parent / f'semantic-tamper-{promoted}'
                shutil.copytree(lineage_root, tampered)
                changed = copy.deepcopy(lineage_qualification)
                if promoted:
                    promote_lineage_qualification(changed, json.loads(self.evaluation.read_bytes()))
                else:
                    cid = policy.MULTI_WORK_CRITERION
                    changed['qualitative_review']['unresolved_criteria'].remove(cid)
                    changed['qualitative_review']['human_escalations'].remove(cid)
                    changed['qualitative_review']['resolved_criteria'].append(cid)
                    changed['qualitative_review']['resolved_criteria'].sort()
                    changed['naturalistic_evidence']['multi_work_viewer_comprehension']['state'] = 'satisfied'
                rehash_lineage_qualification(tampered, changed)
                with self.assertRaisesRegex(ValueError, 'contradicts evaluation and recorded reviews'):
                    result_lineage.verify(tampered)
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

    def test_no_review_lineage_replays_all_required_gaps_without_source_access(self):
        import campaign
        import result_lineage
        import review_operations as ops
        output = self.parent / 'no-review-qualification'
        value = policy.qualify(self.root, self.evaluation, output,
            candidate=campaign.load_campaign(self.root)['candidate_head'])
        self.assertEqual(value['qualitative_review_runs'], [])
        self.assertTrue(value['qualitative_review']['unresolved_criteria'])
        self.assertFalse(value['replacement_pass_candidate'])
        import resource_observer
        manifest = campaign.load_evidence_set(self.root)
        self.assertNotIn('resources/observation.json', manifest['artifacts'])
        initial = resource_observer.initial(manifest['candidate_artifacts'])
        self.assertEqual(manifest['naturalistic_memory_evidence'], initial)
        self.assertEqual(value['naturalistic_evidence']['naturalistic_resource'], initial)
        published = result_lineage.publish(self.root, self.evaluation, [], output / 'qualification.json')
        copied = self.parent / 'copied-no-review-lineage'
        shutil.copytree(published['lineage_root'], copied)
        with patch.object(campaign, 'load_evidence_set', side_effect=AssertionError('original campaign access')), \
                patch.object(policy, 'qualify', side_effect=AssertionError('original qualification input access')), \
                patch.object(ops, 'select_evidence', side_effect=AssertionError('campaign selection access')):
            verified = result_lineage.verify(copied)
            self.assertEqual(verified['review_run_ids'], [])
            self.assertFalse(verified['external_staging_paths_used'])
            copied_value = json.loads((copied / 'qualification/qualification.json').read_bytes())
            self.assertEqual(copied_value['naturalistic_evidence']['naturalistic_resource'], initial)
            changed = copy.deepcopy(value)
            promote_lineage_qualification(changed, json.loads(self.evaluation.read_bytes()))
            rehash_lineage_qualification(copied, changed)
            with self.assertRaisesRegex(ValueError, 'contradicts evaluation and recorded reviews'):
                result_lineage.verify(copied)

    def test_agent_cannot_supply_human_observation(self):
        import review_operations as ops
        with self.assertRaisesRegex(ValueError, 'agent preparation'):
            ops.prepare(self.root, self.parent / 'false-human', reviewer_kind='agent', session_id='review',
                human_observations=self.parent / 'missing.json')


def run_contract_tests():
    result = unittest.TextTestRunner().run(unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(cls) for cls in (LaunchReadinessTests, DefinitionDependencyTests, PolicyTests, GateTechnicalBoundaryTests, FileBoundaryTests)))
    if not result.wasSuccessful():
        raise AssertionError('qualification policy regressions failed')


if __name__ == '__main__':
    unittest.main()
