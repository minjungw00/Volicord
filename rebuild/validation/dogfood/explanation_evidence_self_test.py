"""Fixture-only lifecycle linkage; actual-host proof remains separate private evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import explanation_evidence as e
import capture_self_test as captures


class MeasuredExecutionTests(unittest.TestCase):
    def test_corpus_shaped_explanation_lifecycle_reaches_final_consumer(self):
        helper = captures.CurrentExecutionTests()
        for kind in ('work', 'decision'):
            preparation, _, record, readback = lifecycle(kind)
            commands = [f'volicord --runtime /private/runtime --repository /phase8/repository --json {kind} explain prepare --{kind} {"08" * 16} --language en',
                f'volicord --runtime /private/runtime --repository /phase8/repository --json {kind} explain record --plan /private/plan.json --response /private/response.json',
                'volicord --json status' if kind == 'work' else 'volicord --json decisions']
            source = '\n'.join('text(await tools.exec_command(' + json.dumps({'cmd': cmd}) + '));' for cmd in commands)
            returned = [{'plan': preparation['plan']}, record, readback]
            capture = helper.command(source, [json.dumps({'output': json.dumps(result), 'exit_code': 0}) for result in returned])
            observed = e.measured_cli_operations(capture)
            self.assertEqual([v['operation'] for v in observed], ['explanation_prepare', 'explanation_record',
                'project_status' if kind == 'work' else 'decisions'])
            self.assertEqual([v['result'] for v in observed], returned)
            self.assertEqual([v['exit_code'] for v in observed], [0, 0, 0])
            self.assertEqual(len({v['call_id'] for v in observed}), 3)
            # A numeric failure cannot acquire a successful Product receipt.
            failed = helper.command(source, [json.dumps({'output': json.dumps(result), 'exit_code': 143}) for result in returned])
            self.assertTrue(all(v['result'] is None and v['exit_code'] == 143 for v in e.measured_cli_operations(failed)))

    def test_corpus_shaped_test_execution_reaches_validation_consumer(self):
        import harness
        capture = captures.CurrentExecutionTests().command(
            'text(await tools.exec_command({cmd:"rebuild/scripts/validate focused reuse -- cargo test --manifest-path rebuild/Cargo.toml -p volicord-operations --test analysis_reuse"}));',
            ['{"output":"tests passed","exit_code":143}'])
        result = harness.meaningful_resume_validation(capture, 0)
        self.assertFalse(result['qualified'])
        self.assertTrue(result['unresolved_terminal_failure'])
        self.assertEqual(result['terminal_execution_identity'], 'custom_call:test:0')


def lifecycle(kind='work', language='en', before_state='unavailable'):
    project, identity, source = '01' * 16, '08' * 16, '03' * 16
    subject = {'kind': kind, 'identity': identity}
    questions = sorted(e.WORK_QUESTIONS if kind == 'work' else e.DECISION_QUESTIONS)
    evidence = [{'key': 'goal' if kind == 'work' else 'choice',
        'record_kind': 'context_item' if kind == 'work' else 'decision', 'identity': identity,
        'revision': 1, 'field': 'statement' if kind == 'work' else 'choice', 'sources': [source],
        'content': 'Independently authored structural fixture; no prose adequacy claim'}]
    plan = {'project_id': project, 'subject': {'kind': kind, 'identity': list(bytes.fromhex(identity))},
        'question': 'work_outcome' if kind == 'work' else 'decision_rationale', 'requested_language': language,
        'evidence': evidence, 'source_status': [{'identity': source, 'availability': 'Available',
            'freshness': 'Current', 'snapshot': 'a' * 40, 'observation': 'fixture'}],
        'conflicts': [], 'instructions': 'fixture only', 'fingerprint': 'sha256:' + 'a' * 64}
    plan['retention_budget'] = {'response_byte_limit': 16384, 'retained_byte_limit': 147456,
        'metadata_byte_reserve': e.retention_metadata_bytes(plan), 'response_byte_capacity': 16384}
    response = {'format_kind': 'volicord_explanation', 'format_version': 1,
        'plan_fingerprint': plan['fingerprint'], 'language': language,
        'generator': {'host': 'authored fixture', 'session': 'fixture-only', 'agent': None, 'model': None},
        'paragraphs': [{'question': q, 'text': '구조 검사 예제입니다.' if language == 'ko' else 'Structural example only.',
            'evidence_keys': [evidence[0]['key']]} for q in questions]}
    stripped = copy.deepcopy(evidence); stripped[0]['content'] = None
    statuses = [{k: v for k, v in value.items() if k != 'observation'} for value in plan['source_status']]
    retained = {'realization': response, 'project_id': project, 'subject': plan['subject'],
        'question': plan['question'], 'evidence': stripped, 'source_status': statuses, 'conflicts': [],
        'generated_at_unix_micros': 100, 'generator_identity_status': 'self_reported_not_independently_verified'}
    answers = {'diagnostic': None, 'explanation_state': 'current', 'facts': ([{'question':'NextStepAvailability', 'text':'No Checkpoint is recorded in this structural fixture.',
        'role':'unavailable', 'evidence_keys':[]}] if kind == 'work' else []),
        'prose': [{'question': ''.join(word.title() for word in p['question'].split('_')),
            'text': p['text'], 'role': 'generated_interpretation', 'evidence_keys': p['evidence_keys']} for p in response['paragraphs']],
        'provenance': {k: v for k, v in retained.items() if k != 'realization'} | {
            'language': language, 'fingerprint': plan['fingerprint'], 'generator': response['generator']}}
    readback = {'project_id': project, 'selected_work': {'work_item_id': identity, 'checkpoint_ids': [], 'answers': answers},
        'decisions': [{'identity': identity, 'answers': answers}]}
    preparation = {'kind': 'dogfood_explanation_preparation', 'schema_version': e.SCHEMA_VERSION,
        'identity': 'bb' * 16, 'evidence_purpose': 'naturalistic', 'project_id': project, 'subject': subject, 'language': language,
        'plan': plan, 'journey_id': 'journey-volicord', 'candidate_head': 'a' * 40,
        'candidate_executable_sha256': 'c' * 64, 'phase': 'post_session_steward',
        'observed_at': '2026-10-03T00:00:00+00:00', 'raw_inputs': [],
        'before_observation': {'state': before_state, 'answers': None},
        'canonical_bundle_sha256': 'd' * 64,
        'generation_authority': 'current_active_host_interaction_required_no_provider_dispatch',
        'generator_identity_limit': 'self_reported_not_independently_verified'}
    return preparation, response, {'operation': 'explanation_record', 'explanation': retained}, readback


def publish_fixture(root, *, kind='work', language='en', before_state='unavailable', mapped=None, candidate=None, subject_id=None, project_id=None, identity=None, paragraph_text=None, revision=1):
    preparation, response, record, after = lifecycle(kind, language, before_state)
    if revision != 1:
        fingerprint = 'sha256:' + str(revision) * 64
        preparation['plan']['fingerprint'] = fingerprint
        response['plan_fingerprint'] = fingerprint
        for value in (preparation['plan'], record['explanation'], after['selected_work']['answers']['provenance']):
            value['evidence'][0]['revision'] = revision
        after['selected_work']['answers']['provenance']['fingerprint'] = fingerprint
    if paragraph_text is not None:
        for paragraph in response['paragraphs']:
            paragraph['text'] = paragraph_text
        for selected in (after['selected_work'], after['decisions'][0]):
            for paragraph in selected['answers']['prose']:
                paragraph['text'] = paragraph_text
    if identity:
        preparation['identity'] = identity
    if project_id:
        preparation, response, record, after = [json.loads(json.dumps(v).replace('01' * 16, project_id))
            for v in (preparation, response, record, after)]
    if subject_id:
        old = preparation['subject']['identity']
        preparation, response, record, after = [json.loads(json.dumps(v).replace(old, subject_id))
            for v in (preparation, response, record, after)]
        for value in (preparation['plan'], record['explanation'], after['selected_work']['answers']['provenance'],
                after['decisions'][0]['answers']['provenance']):
            value['subject']['identity'] = list(bytes.fromhex(subject_id))
    preparation['plan']['retention_budget']['metadata_byte_reserve'] = e.retention_metadata_bytes(preparation['plan'])
    if candidate:
        preparation['candidate_head'] = candidate['candidate_head']
        preparation['candidate_executable_sha256'] = candidate['candidate_artifacts']['volicord']['sha256']
    if mapped:
        preparation['raw_inputs'] = e.document_realization.raw_binding(mapped)
    before = copy.deepcopy(after)
    for answer in (before['selected_work']['answers'], before['decisions'][0]['answers']):
        if before_state != 'current':
            answer.update(explanation_state=before_state, provenance=None, prose=[{'question':'ExplanationAvailability',
                'text':'Fixture explanation is unavailable.', 'role':'unavailable', 'evidence_keys':[]}])
    preparation['before_observation'] = e.subject_answers(before, preparation['subject'])
    directory = e.entry_path(root, preparation['identity'])
    preparation['observation_order'] = e.declare_attempt(root, preparation)
    values = {'before': c.json_bytes(before), 'preparation': c.json_bytes(preparation), 'response': c.json_bytes(response),
        'record': c.json_bytes(record), 'after': c.json_bytes(after)}
    receipt = {'kind': 'dogfood_explanation_receipt', 'schema_version': e.SCHEMA_VERSION,
        'identity': preparation['identity'], 'observation_order': preparation['observation_order'], 'phase': 'post_session_steward',
        'observed_at': '2026-10-03T00:01:00+00:00',
        'preparation': e.binding(values['preparation']), 'response': e.binding(values['response']),
        'record': e.binding(values['record']), 'readback': e.binding(values['after']),
        'evidence_purpose': preparation['evidence_purpose'], 'candidate_head': preparation['candidate_head'], 'candidate_executable_sha256': preparation['candidate_executable_sha256'],
        'generator_identity_status': 'self_reported_not_independently_verified', 'after_state': 'current',
        'host_response_locator': {'kind': 'submitted_response_file', 'sha256': e.binding(values['response'])['sha256'],
            'session': response['generator']['session'], 'raw_capture_sha256': None, 'turn_id': None,
            'limit': 'no independent response authorship or host-turn attestation'}}
    values['receipt'] = c.json_bytes(receipt)
    e.document_realization.publish(root, {directory / (name + '.json'): data for name, data in values.items()})
    return directory


class LifecycleTests(unittest.TestCase):
    def test_compact_response_budget_counts_unicode_escaping_and_generator_metadata(self):
        for kind in ('work', 'decision'):
            for language in ('en', 'ko'):
                p, original, _, _ = lifecycle(kind, language)
                for target in (16383, 16384, 16385):
                    response = copy.deepcopy(original)
                    response['generator']['model'] = '모델 "model"\\' * 100
                    response['paragraphs'][0]['text'] += '한글 "quoted"\\\n'
                    response['paragraphs'][0]['text'] += 'x' * (target - len(e.compact_bytes(response)))
                    self.assertEqual(len(e.compact_bytes(response)), target)
                    self.assertGreater(len(json.dumps(response, ensure_ascii=False, indent=2).encode()), 16384)
                    if target <= 16384:
                        e.validate_response(p['plan'], response)
                    else:
                        with self.assertRaisesRegex(c.CampaignError, 'compact realization JSON is 16385 bytes; limit 16384'):
                            e.validate_response(p['plan'], response)
                bad = copy.deepcopy(p['plan']); bad['retention_budget']['metadata_byte_reserve'] += 1
                with self.assertRaisesRegex(c.CampaignError, 'budget mismatch'):
                    e.validate_plan(bad, p['project_id'], p['subject'], language)

    def test_completed_history_survives_changed_current_basis(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            c.write_json(c.inventory_path(root), c.load_inventory(root))
            old = publish_fixture(root, identity='ff' * 16)
            old_bytes = {p.name: p.read_bytes() for p in old.glob('*.json')}
            new = publish_fixture(root, identity='aa' * 16, revision=2)
            current = json.loads((new / 'preparation.json').read_bytes())
            campaign = {'evidence_purpose': 'naturalistic', 'candidate_head': 'a' * 40,
                'candidate_artifacts': {'volicord': {'sha256': 'c' * 64}},
                'candidate_binary': '/unused/volicord',
                'candidate_artifacts': {'volicord': {'sha256': 'c' * 64}},
                'journeys': {'journey-volicord': {'runtime_home': '/unused/runtime'}}}
            with patch.object(c, 'candidate_artifact_use'), patch.object(e, 'invoke', return_value=({'plan': current['plan']}, {})), \
                    patch.object(e, 'read', return_value=(json.loads((new / 'after.json').read_bytes()), {})):
                e.require_ready(root, campaign, {})
            self.assertEqual({p.name: p.read_bytes() for p in old.glob('*.json')}, old_bytes)

    def test_publication_scope_history_tampering_and_failed_obligations(self):
        import review_explanations as review
        for kind in ('work', 'decision'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary); c.write_json(c.inventory_path(root), c.load_inventory(root))
                old = publish_fixture(root, kind=kind, identity='ff' * 16)
                new = publish_fixture(root, kind=kind, identity='aa' * 16, revision=2)
                ko = publish_fixture(root, kind=kind, language='ko', identity='bb' * 16)
                other = publish_fixture(root, kind=kind, subject_id='09' * 16, identity='cc' * 16)
                repeated = publish_fixture(root, kind=kind, identity='dd' * 16, revision=2)
                relations = e.publication_relations(root)
                self.assertEqual(relations['ff' * 16], {'publication_role': 'historical', 'selected_identity': 'dd' * 16})
                self.assertEqual(relations['aa' * 16]['publication_role'], 'historical')
                self.assertEqual({k for k,v in relations.items() if v['publication_role'] == 'final'}, {'bb' * 16, 'cc' * 16, 'dd' * 16})
                indexed = e.collection_index(root, {})
                review.validate_publication_index(indexed)
                bad = copy.deepcopy(indexed)
                bad['steward_lifecycles'][0]['publication_role'] = 'historical'
                bad['steward_lifecycles'][0]['selected_identity'] = 'ee' * 16
                with self.assertRaises(ValueError):
                    review.validate_publication_index(bad)
                (old / 'response.json').write_bytes(b'{}')
                with self.assertRaises(c.CampaignError):
                    e.publication_relations(root)
        # A declared attempt whose Product preparation failed remains an obligation,
        # even when a later valid observation completes on the same basis.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); c.write_json(c.inventory_path(root), c.load_inventory(root))
            publish_fixture(root, identity='aa' * 16)
            context = lifecycle()[0]; context['identity'] = 'bb' * 16
            e.declare_attempt(root, context)
            publish_fixture(root, identity='cc' * 16)
            with self.assertRaisesRegex(c.CampaignError, 'pending or failed'):
                e.publication_relations(root)

    def test_predecessor_scope_hash_fork_and_schema_cannot_supply_selection(self):
        for mutation in ('scope', 'hash', 'fork', 'schema'):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary); c.write_json(c.inventory_path(root), c.load_inventory(root))
                publish_fixture(root, identity='aa' * 16)
                directory = publish_fixture(root, identity='bb' * 16)
                path = directory / 'attempt.json'; value = json.loads(path.read_bytes())
                if mutation == 'scope':
                    value['language'] = 'ko'
                elif mutation == 'hash':
                    value['previous']['attempt']['sha256'] = '0' * 64
                elif mutation == 'fork':
                    value['previous'] = None
                else:
                    value['schema_version'] = 1
                # Refresh outer inventory: independently specified relationship
                # rejection must survive cooperative filesystem rehashing.
                path.write_bytes(c.json_bytes(value))
                inventory = c.load_inventory(root)
                inventory['artifacts'][c.relative(root, path)] = e.binding(path.read_bytes())
                c.write_json(c.inventory_path(root), inventory)
                with self.assertRaises(c.CampaignError):
                    e.ordered_attempts(root)

    def test_actual_producer_prepare_record_and_collection_index(self):
        import contextlib
        import shutil
        from types import SimpleNamespace
        import harness as h
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture_root = root / 'fixture'; fixture_root.mkdir()
            descriptor = h.real_session_fixture('volicord', 1, '0' * 40, fixture_root)
            source = fixture_root / descriptor['evidence']['captures']['work']['file']
            mapped = {('volicord', 'A', 'start'): SimpleNamespace(source=source, capture=h.load_codex_capture(source))}
            p,response,product_receipt,after = lifecycle()
            candidate = {'evidence_purpose': 'naturalistic', 'candidate_head': p['candidate_head'], 'candidate_binary': '/unused/volicord',
                'document_language': 'en', 'candidate_artifacts': {'volicord': {'sha256': p['candidate_executable_sha256']}},
                'journeys': {'journey-volicord': {'repository_class': 'volicord', 'runtime_home': '/unused/runtime',
                    'repository_path': '/unused/repository'}}}
            c.write_json(c.inventory_path(root), c.load_inventory(root))
            recorded = []
            def invoke(binary, runtime, project, args):
                if 'record' in args:
                    recorded.append(True)
                    return product_receipt, {}
                return {'operation': 'explanation_prepare', 'plan': p['plan']}, {}
            def read(*args):
                value = copy.deepcopy(after)
                if not recorded:
                    value['selected_work']['answers'].update(explanation_state='unavailable', provenance=None, prose=[])
                return value, {}
            def export(binary, runtime, repository, target):
                shutil.copyfile(fixture_root / descriptor['evidence']['canonical_bundle']['file'], target)
            with patch.object(c, 'load_campaign_for_mutation', return_value=candidate), \
                    patch.object(c, 'verify_frozen_campaign'), patch.object(c, 'verify_inventory'), \
                    patch.object(c, 'candidate_artifact_use', side_effect=lambda *args: contextlib.nullcontext()), \
                    patch.object(c, 'default_export', side_effect=export), \
                    patch.object(c, 'map_batch_rollouts', return_value=mapped), \
                    patch.object(e, 'invoke', side_effect=invoke), patch.object(e, 'read', side_effect=read):
                prepared = e.prepare(root, [source], work_ids=[p['subject']['identity']])
                identity = prepared['explanations'][0]['identity']
                with self.assertRaisesRegex(c.CampaignError, 'missing host response'):
                    e.require_ready(root, candidate, mapped)
                input_path = root / 'host-response.json'; input_path.write_bytes(c.json_bytes(response))
                result = e.record(root, identity, input_path)
                self.assertEqual(result['state'], 'recorded')
                e.require_ready(root, candidate, mapped)
                observed = e.collection_index(root, mapped)
                self.assertEqual(observed['steward_lifecycles'][0]['before_state'], 'unavailable')
                self.assertEqual(observed['steward_lifecycles'][0]['after_state'], 'current')
                original = (e.entry_path(root, identity) / 'preparation.json').read_bytes()
                with self.assertRaisesRegex(c.CampaignError, 'immutable'):
                    e.record(root, identity, input_path)
                newer = e.prepare(root, [source], work_ids=[p['subject']['identity']])
                new_id = newer['explanations'][0]['identity']
                newer_value = json.loads(e.bound(root, e.entry_path(root, new_id) / 'preparation.json'))
                self.assertEqual(newer_value['before_observation']['state'], 'current')
                self.assertEqual((e.entry_path(root, identity) / 'preparation.json').read_bytes(), original)

    def test_work_and_decision_same_and_cross_language_linkage(self):
        for kind in ['work', 'decision']:
            for language in ['en', 'ko']:
                for state in ['unavailable', 'stale', 'current']:
                    prepared, response, record, after = lifecycle(kind, language, state)
                    self.assertEqual(e.validate_lifecycle(prepared, response, record, after)['state'], 'current')
                    self.assertEqual(prepared['before_observation']['state'], state)
                    self.assertNotIn('verified', response['generator'])

    def test_wrong_subject_language_plan_revision_source_and_provenance(self):
        for mutation in [lambda p,r,c,a: r.update(language='de'),
                lambda p,r,c,a: r.update(plan_fingerprint='sha256:' + 'b' * 64),
                lambda p,r,c,a: c['explanation']['subject'].update(identity=[0] * 16),
                lambda p,r,c,a: c['explanation']['evidence'][0].update(revision=2),
                lambda p,r,c,a: c['explanation']['evidence'][0].update(sources=['ff' * 16]),
                lambda p,r,c,a: a['selected_work']['answers']['provenance'].update(generator_identity_status='verified'),
                lambda p,r,c,a: a['selected_work']['answers']['prose'].pop(),
                lambda p,r,c,a: r['paragraphs'][0].update(evidence_keys=['unknown'])]:
            p,r,record,a = copy.deepcopy(lifecycle())
            mutation(p,r,record,a)
            with self.assertRaises(c.CampaignError):
                e.validate_lifecycle(p,r,record,a)

    def test_inventory_mutation_missing_response_and_source_independent_verification(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            c.write_json(c.inventory_path(root), {'kind': 'phase8_dogfood_evidence_inventory', 'schema_version': 1, 'artifacts': {}})
            directory = publish_fixture(root, before_state='stale')
            p, receipt = e.verify(root, directory / 'preparation.json')
            self.assertEqual(p['before_observation']['state'], 'stale')
            self.assertEqual(receipt['after_state'], 'current')
            # Copy only package bytes; no Runtime, executable or staging paths needed.
            import shutil
            copied = root / 'copy'; copied.mkdir()
            shutil.copytree(directory, copied / 'explanations' / p['identity'])
            c.write_json(c.inventory_path(copied), c.load_inventory(root))
            e.verify(copied, copied / 'explanations' / p['identity'] / 'preparation.json')
            (directory / 'response.json').write_text('{}')
            with self.assertRaises(c.CampaignError):
                e.verify(root, directory / 'preparation.json')
            (directory / 'response.json').unlink()
            with self.assertRaises(c.CampaignError):
                e.verify(root, directory / 'preparation.json')

    def test_missing_receipt_and_changed_basis_before_collection(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary);c.write_json(c.inventory_path(root), {'kind': 'phase8_dogfood_evidence_inventory', 'schema_version': 1, 'artifacts': {}})
            directory = publish_fixture(root)
            campaign = {'evidence_purpose': 'naturalistic', 'candidate_head': 'a' * 40, 'candidate_binary': '/unused/volicord',
                'candidate_artifacts': {'volicord': {'sha256': 'c' * 64}},
                'journeys': {'journey-volicord': {'runtime_home': '/unused/runtime'}}}
            with patch.object(c, 'candidate_artifact_use'), patch.object(e, 'invoke', return_value=({'plan': {}}, {})):
                with self.assertRaisesRegex(c.CampaignError, 'basis changed'):
                    e.require_ready(root, campaign, {})
            (directory / 'receipt.json').unlink()
            with self.assertRaisesRegex(c.CampaignError, 'missing host response'):
                e.require_ready(root, campaign, {})


if __name__ == '__main__':
    unittest.main()
