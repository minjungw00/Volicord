"""Fixture-only lifecycle linkage; actual-host proof remains separate private evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import explanation_evidence as e


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
    preparation = {'kind': 'dogfood_explanation_preparation', 'schema_version': 1,
        'identity': 'bb' * 16, 'evidence_purpose': 'naturalistic', 'project_id': project, 'subject': subject, 'language': language,
        'plan': plan, 'journey_id': 'journey-volicord', 'candidate_head': 'a' * 40,
        'candidate_executable_sha256': 'c' * 64, 'phase': 'post_session_steward',
        'observed_at': '2026-10-03T00:00:00+00:00', 'raw_inputs': [],
        'before_observation': {'state': before_state, 'answers': None},
        'canonical_bundle_sha256': 'd' * 64,
        'generation_authority': 'current_active_host_interaction_required_no_provider_dispatch',
        'generator_identity_limit': 'self_reported_not_independently_verified'}
    return preparation, response, {'operation': 'explanation_record', 'explanation': retained}, readback


def publish_fixture(root, *, kind='work', language='en', before_state='unavailable', mapped=None, candidate=None, subject_id=None, project_id=None, identity=None, paragraph_text=None):
    preparation, response, record, after = lifecycle(kind, language, before_state)
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
    values = {'before': c.json_bytes(before), 'preparation': c.json_bytes(preparation), 'response': c.json_bytes(response),
        'record': c.json_bytes(record), 'after': c.json_bytes(after)}
    receipt = {'kind': 'dogfood_explanation_receipt', 'schema_version': 1,
        'identity': preparation['identity'], 'phase': 'post_session_steward',
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
                'journeys': {'journey-volicord': {'runtime_home': '/unused/runtime'}}}
            with patch.object(c, 'candidate_artifact_use'), patch.object(e, 'invoke', return_value=({'plan': {}}, {})):
                with self.assertRaisesRegex(c.CampaignError, 'basis changed'):
                    e.require_ready(root, campaign, {})
            (directory / 'receipt.json').unlink()
            with self.assertRaisesRegex(c.CampaignError, 'missing host response'):
                e.require_ready(root, campaign, {})


if __name__ == '__main__':
    unittest.main()
