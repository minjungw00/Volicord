"""Returned meaning, privacy omissions and copied-package semantic controls."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import answer_projection as a
from answer_observations_self_test import AnswerTests
import campaign as c
import codex_events
import explanation_evidence as e
from explanation_evidence_self_test import lifecycle, publish_fixture
import review_captures
import review_explanations as r
import review_operations as ops


def rehash_package(root, entry_id, data):
    """Fixture tampering refreshes all wrapper hashes, not semantic diagnostics."""
    import machine_findings
    preparation = json.loads((root / 'preparation.json').read_bytes())
    entry = preparation['index']['evidence'][entry_id]
    artifact = root / entry['path']; artifact.chmod(0o600); artifact.write_bytes(data)
    entry.update(bytes=len(data), sha256=ops.digest(data))
    entry['locators'], entry['line_count'] = ops.locators(data)
    if entry['surface'] in review_captures.CAPTURE_SURFACES:
        entry['projection'] = review_captures.metadata(data)
    elif entry['surface'] == r.SURFACE:
        v = json.loads(data)
        entry['projection'] = {'schema_version': r.SCHEMA_VERSION, 'semantic_complete': v['semantic_complete'],
            'review_bytes': len(data), 'review_sha256': ops.digest(data)}
    preparation['package_id'] = machine_findings.digest({'binding': preparation['binding'],
        'index': preparation['index'], 'unavailable_surfaces': preparation['unavailable_surfaces']})
    encoded = ops.encoded(preparation)
    (root / 'preparation.json').chmod(0o600); (root / 'preparation.json').write_bytes(encoded)
    package = json.loads((root / 'package.json').read_bytes())
    package.update(package_id=preparation['package_id'], preparation_sha256=ops.digest(encoded))
    for name in package['artifacts']:
        package['artifacts'][name] = e.binding((root / name).read_bytes())
    (root / 'package.json').chmod(0o600); (root / 'package.json').write_bytes(ops.encoded(package))


class MeaningTests(unittest.TestCase):
    def test_retention_status_and_checkpoint_course_survive_copied_lifecycle(self):
        preparation, response, record, after = lifecycle()
        course = {'goal': 'Preserve the bounded repository shape', 'kind': 'Completed',
            'next_step': 'Inspect the result', 'observed_at': 100,
            'open_questions': [{'identity': '09' * 16, 'revision': 2}],
            'forgotten_source_count': 1}
        for status in (preparation['plan']['source_status'][0],
                       record['explanation']['source_status'][0]):
            status['body'] = 'policy_withheld'
        preparation['plan']['source_status'][0]['observation'] = 'Excluded Source observation prose'
        preparation['plan']['retention_budget']['metadata_byte_reserve'] = e.retention_metadata_bytes(preparation['plan'])
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); c.write_json(c.inventory_path(root), c.load_inventory(root))
            with patch('explanation_evidence_self_test.lifecycle',
                       return_value=(preparation, response, record, after)):
                directory = publish_fixture(root, extra_evidence=[{
                    'key': 'course:09', 'record_kind': 'checkpoint', 'identity': '09' * 16,
                    'revision': 2, 'field': 'goal,kind,next_step,open_questions',
                    'sources': ['03' * 16], 'content': course}])
            data, metadata = r.project(root, directory / 'preparation.json', evidence_set_sha256='b' * 64)
            self.assertTrue(metadata['semantic_complete'])
            value = r.validate(data)
            plan = value['stages']['plan']['value']['value']['plan']
            self.assertEqual(plan['evidence'][1]['content'], course)
            self.assertEqual(plan['source_status'][0]['body'], 'policy_withheld')
            self.assertNotIn(b'Excluded Source observation prose', data)
            self.assertEqual(value['private_artifacts']['record'],
                e.binding(c.json_bytes(value['stages']['record']['value']['value'])))
        self.assertTrue(r.validate(data)['semantic_complete'])

    def test_canonical_detail_coordinates_preserve_types_and_unknown_limits(self):
        coordinates = {'tool': 'canonical_inspect', 'record_kind': 'checkpoint',
            'record_id': '09' * 16, 'revision': 2, 'field': 'verification'}
        reading_coordinates = {**coordinates, 'field': 'next_step'}
        state = {'checkpoint_id': '09' * 16, 'checkpoint_revision': 2,
            'work_state': 'completed', 'observed_at_unix_micros': 100,
            'work_source_basis': ['03' * 16], 'verification': [],
            'user_review': {'state': 'not_requested'}, 'user_acceptance': {'state': 'not_requested'},
            'later_changed_checkpoint_ids': [], 'detail_inspection': coordinates}
        reading = {'availability': 'available', 'original_text': 'Inspect the result',
            'representation': 'original_quotation', 'display_english': 'Inspect the result',
            'display_korean': 'Inspect the result', 'original_language_preserved': True,
            'omitted_characters': 0, 'omitted_utf8_bytes': 0, 'gaps': [],
            'detail_inspection': reading_coordinates,
            'basis': {'record_kind': 'checkpoint', 'identity': '09' * 16, 'revision': 2,
                'field': 'next_step', 'source_ids': ['03' * 16], 'source_status': [],
                'available_revisions': [1, 2], 'analysis_snapshot_ids': [], 'repository_snapshot_ids': []}}
        self.answer['selected_work']['evidence'] = {'goal': reading, 'next_step': reading,
            'result': None, 'result_observed_at': None, 'original_changes': [], 'states': [state],
            'latest_state': state, 'verification': state, 'review': state, 'acceptance': state,
            'analysis_snapshot_ids': [], 'repository_snapshot_ids': [], 'source_status': [],
            'code_availability': 'unavailable'}
        data, metadata = self.capture()
        self.assertTrue(metadata['semantic_complete'])
        record = next(r for r in review_captures.validate(data)['records'] if r.get('operation') == 'recall')
        selected = record['body']['value']['returned_meaning']
        self.assertTrue(selected['semantic_complete'])
        self.assertEqual(selected['value']['selected_work']['evidence']['goal']['detail_inspection'], reading_coordinates)
        pointers, _ = ops.locators(data)
        self.assertTrue(any(p['value'].endswith('/states/0/detail_inspection/record_id') for p in pointers))
        for field, value in (('revision', 'wrong-type'), ('unregistered_field', 'unknown')):
            malformed = copy.deepcopy(self.answer)
            malformed['selected_work']['evidence']['states'][0]['detail_inspection'][field] = value
            self.assertFalse(a.project(malformed, 'recall')['semantic_complete'])

    def test_complete_recorded_detail_preserves_schema_gap_without_filling_missing_semantics(self):
        self.answer['selected_work']['authored_additional_fact'] = 'Actual bounded retained fact.'
        data, metadata = self.capture()
        value = review_captures.validate(data)
        returned = next(v for v in value['records'] if v.get('operation') == 'recall')
        self.assertFalse(returned['body']['value']['returned_meaning']['semantic_complete'])
        self.assertEqual(returned['detail']['value']['result']['selected_work']['authored_additional_fact'],
            'Actual bounded retained fact.')
        self.assertTrue(metadata['semantic_complete'])
        # A real omitted required field is still missing, even with a recorded detail body.
        del self.answer['selected_work']['answers']
        _, metadata = self.capture()
        self.assertFalse(metadata['semantic_complete'])

    def test_multiple_nested_omissions_survive_json_key_order(self):
        # Independently authored incomplete DTO, not a copied rollout.
        result = {"project_id": "01" * 16, "selected_work": {"work_item_id": "02" * 16}}
        projected = a.project(result, "recall")
        self.assertGreater(len(projected["omissions"]), 1)
        retained = json.loads(json.dumps(projected, sort_keys=True))
        a.validate(retained)
        retained["omissions"] = list(reversed(retained["omissions"]))
        with self.assertRaises(ValueError):
            a.validate(retained)

    def setUp(self):
        self.fixture = AnswerTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.answer = copy.deepcopy(self.fixture.answer)

    def capture(self):
        self.fixture.evaluate(self.answer)
        raw = self.fixture.path.read_bytes()
        parsed = codex_events.parse_codex_capture(raw)
        return review_captures.project(raw, origin={'kind': 'evidence_set_member', 'path': 'resume.jsonl',
            'raw_bytes': len(raw), 'raw_sha256': ops.digest(raw)}, role='resume', session_id=parsed.session_id,
            candidate_head='a' * 40, evidence_set_sha256='b' * 64)

    def test_actual_returned_action_claims_basis_and_deep_locators(self):
        data, meta = self.capture()
        self.assertTrue(meta['semantic_complete'])
        value = review_captures.validate(data)
        record = next(v for v in value['records'] if v.get('operation') == 'recall')
        returned = record['body']['value']['returned_meaning']
        self.assertEqual(returned['value']['selected_work']['answers'], self.answer['selected_work']['answers'])
        self.assertEqual(returned['value']['next_step'], self.answer['next_step'])
        self.assertEqual(returned['consistency_errors'], [])
        pointers, _ = ops.locators(data)
        self.assertTrue(any(v['value'].endswith('/selected_work/answers/facts/0/recorded_action/recorded_text') for v in pointers))
        self.assertEqual(record['transport'], 'mcp')
        self.assertEqual(record['requested_language'], 'en')

    def test_correction_coordinates_and_revision_receipts_survive_review(self):
        self.fixture.insert_correction()
        data, _ = self.capture()
        value = review_captures.validate(data)
        correction = next(r for r in value['records'] if r.get('operation') == 'canonical_mutate')
        body = correction['body']['value']
        self.assertEqual(body['request']['record_id'], self.answer['selected_work']['work_item_id'])
        self.assertEqual(body['request']['expected_revision'], 1)
        self.assertEqual(body['result']['revision'], 2)
        self.assertEqual(body['result']['identity'], body['request']['record_id'])
        self.assertEqual(body['result']['record_kind'], 'context_item')
        self.assertEqual(body['result']['user_response_source_id'], 'fa' * 16)
        self.assertNotIn('user_turn', body['request'])
        self.assertNotIn('corrected_text', body['request'])
        self.assertLessEqual(correction['sequence'], correction['completion_sequence'])
        parsed = codex_events.load_codex_capture(self.fixture.path)
        self.assertEqual(correction['sequence'], parsed.calls('canonical_mutate')[0].sequence)
        recall = next(r for r in value['records'] if r.get('operation') == 'recall')
        self.assertEqual(recall['sequence'], parsed.calls('recall')[0].sequence)
        pointers, _ = ops.locators(data)
        self.assertTrue(any(p['value'].endswith('/body/value/result/revision') for p in pointers))

    def test_stale_and_absent_keep_the_recorded_action_through_actual_consumer(self):
        for state in ('unavailable', 'stale'):
            self.answer['selected_work']['answers']['explanation_state'] = state
            result, fact = self.fixture.evaluate(self.answer)
            self.assertEqual(fact['status'], 'confirmed_pass')
            self.assertEqual(result['checks']['recall_matches_checkpoint_decision_and_context'], 'passed')
            projected = a.project(self.answer, 'recall')
            self.assertTrue(projected['semantic_complete'])
            self.assertEqual(projected['value']['next_step'], self.answer['next_step'])

    def test_generated_paraphrase_is_retained_without_semantic_certification(self):
        preparation, _, _, after = lifecycle()
        generated = after['selected_work']['answers']
        generated['prose'][0]['text'] = 'A different independently authored interpretation, not a prescribed sentence.'
        self.answer['selected_work']['answers']['prose'] = generated['prose']
        self.answer['selected_work']['answers']['provenance'] = generated['provenance']
        self.answer['selected_work']['answers']['explanation_state'] = 'current'
        projected = a.project(self.answer, 'recall')
        self.assertTrue(projected['semantic_complete'])
        self.assertIn('different independently authored', projected['value']['selected_work']['answers']['prose'][0]['text'])
        a.validate(projected)

    def test_typed_omission_has_scope_and_cannot_excuse_contradiction(self):
        del self.answer['selected_work']['answers']['facts'][0]['text']
        self.answer['next_step'] = 'Wrong returned direction'
        self.answer['transport_omission'] = {'reason': 'serialized_byte_budget', 'omitted_field_count': 1,
            'basis': 'inspect the authoritative record at this identity'}
        projected = a.project(self.answer, 'recall')
        self.assertFalse(projected['semantic_complete'])
        self.assertIn({'pointer': '/selected_work/answers/facts/0/text', 'reason': 'transport_omission'}, projected['omissions'])
        self.assertIn('returned task direction/Checkpoint mismatch', projected['consistency_errors'])
        self.assertEqual(projected['value']['next_step'], 'Wrong returned direction')
        a.validate(projected)
        missing = a.project(None, 'recall')
        self.assertFalse(missing['semantic_complete'])
        self.assertEqual(missing['omissions'], [{'pointer': '', 'reason': 'unresolvable_observation'}])

    def test_genuine_absence_is_distinct_from_missing_transport(self):
        value = {'project_id': self.answer['project_id'], 'selected_work': None, 'next_step': None,
            'checkpoint': None, 'goal_basis': [], 'decisions': []}
        absent = a.project(value, 'recall')
        self.assertTrue(absent['semantic_complete'])
        del value['selected_work']
        missing = a.project(value, 'recall')
        self.assertFalse(missing['semantic_complete'])
        self.assertEqual(missing['omissions'][0]['pointer'], '/selected_work')
        from recorded_action_evidence import recorded_action_errors
        recall = copy.deepcopy(self.answer)
        work = recall['selected_work']
        expected = {'project_id': recall['project_id'], 'goal_id': work['work_item_id'],
            'checkpoint_id': work['checkpoint_ids'][0], 'checkpoint_revision': 1, 'next_step': None}
        recall['next_step'] = None
        work['answers']['facts'] = [{'question': 'NextStepAvailability', 'text': 'No next action is recorded in the latest Checkpoint.',
            'role': 'unavailable', 'evidence_keys': []}]
        self.assertEqual(recorded_action_errors(expected, recall), [])
        recall['next_step'] = 'Repair the explanation'
        self.assertIn('top-level recorded next action', recorded_action_errors(expected, recall))

    def test_unsafe_claim_and_unknown_nested_body_are_explicit_omissions(self):
        self.answer['selected_work']['answers']['facts'][0]['text'] = 'access_token=synthetic-sensitive-value-817263'
        data, meta = self.capture()
        self.assertNotIn(b'synthetic-sensitive-value-817263', data)
        self.assertFalse(meta['semantic_complete'])
        self.assertTrue(any(v.get('body', {}).get('reason') == 'sensitive_payload' for v in json.loads(data)['records']))
        self.answer = copy.deepcopy(self.fixture.answer)
        self.answer['selected_work']['answers']['private_prompt'] = 'Never copy this private value'
        selected = a.project(self.answer, 'recall')
        self.assertNotIn('Never copy', json.dumps(selected))
        self.assertFalse(selected['semantic_complete'])
        self.assertTrue(any(v['reason'] == 'unsupported_typed_fields' for v in selected['omissions']))

    def test_fresh_body_hash_does_not_excuse_inconsistent_retained_claim(self):
        data, _ = self.capture(); value = json.loads(data)
        record = next(v for v in value['records'] if v.get('operation') == 'recall')
        record['body']['value']['returned_meaning']['value']['next_step'] = 'Corrupted task'
        record['body'] = review_captures.body_projection(record['body']['value'])
        with self.assertRaisesRegex(ValueError, 'omissions/consistency changed'):
            review_captures.validate(ops.encoded(value))

    def test_supported_cli_plan_record_and_readback_have_exact_raw_coordinates(self):
        p, response, record, after = lifecycle(kind='decision', language='ko')
        for args, payload, operation in (
            ('decision explain prepare --decision ' + p['subject']['identity'], {'operation': 'explanation_prepare', 'plan': p['plan']}, 'explanation_prepare'),
            ('decision explain record --decision ' + p['subject']['identity'] + ' --input /private/response.json', record, 'explanation_record'),
            ('decisions', {'operation': 'decisions', 'project_id': p['project_id'], 'decisions': after['decisions'], 'omissions': 0}, 'decisions')):
            fixture = AnswerTests(); fixture.setUp(); self.addCleanup(fixture.doCleanups)
            fixture.evaluate(fixture.answer, payload)
            for event in fixture.events:
                body = event['payload']
                if body.get('type') == 'custom_tool_call' and body.get('call_id') == 'cli-recall':
                    body['input'] = body['input'].replace('volicord recall --json', 'volicord --json ' + args + ' --language ko')
            fixture.path.write_text(''.join(json.dumps(v) + '\n' for v in fixture.events))
            raw = fixture.path.read_bytes(); parsed = codex_events.parse_codex_capture(raw)
            data, _ = review_captures.project(raw, origin={'kind': 'evidence_set_member', 'path': 'cli.jsonl',
                'raw_bytes': len(raw), 'raw_sha256': ops.digest(raw)}, role='resume', session_id=parsed.session_id,
                candidate_head='a' * 40, evidence_set_sha256='b' * 64)
            returned = next(v for v in json.loads(data)['records'] if v.get('transport') == 'cli')
            self.assertEqual(returned['operation'], operation)
            self.assertEqual(returned['requested_language'], 'ko')
            self.assertIsInstance(returned['sequence'], int)
            self.assertIsInstance(returned['completion_sequence'], int)
            self.assertTrue(returned['body']['value']['returned_meaning']['semantic_complete'])
            self.assertEqual(b'/private/response.json' in data, '--input /private/response.json' in args)
            # Exact invocation retains the recorded path; it grants no file-content access.

    def test_lifecycle_work_decision_languages_copy_and_corruption_controls(self):
        for kind in ('work', 'decision'):
            for language in ('en', 'ko', 'fr'):
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    c.write_json(c.inventory_path(root), c.load_inventory(root))
                    directory = publish_fixture(root, kind=kind, language=language, before_state='stale')
                    data, meta = r.project(root, directory / 'preparation.json', evidence_set_sha256='b' * 64)
                    self.assertTrue(meta['semantic_complete'])
                    value = r.validate(data)
                    self.assertEqual(value['before_state'], 'stale')
                    self.assertEqual(value['after_state'], 'current')
                    self.assertEqual(value['context']['phase'], 'post_session_steward')
                    for mutation in ('claims', 'subject', 'language', 'source', 'provenance', 'before', 'time'):
                        bad = copy.deepcopy(value)
                        if mutation == 'claims':
                            returned = bad['stages']['after']['value']['value']
                            subject = returned['selected_work'] if kind == 'work' else returned['decisions'][0]
                            subject['answers']['prose'] = []
                        elif mutation == 'subject':
                            bad['context']['subject']['identity'] = 'ff' * 16
                        elif mutation == 'language':
                            bad['context']['language'] = 'en' if language == 'ko' else 'ko'
                        elif mutation == 'source':
                            bad['stages']['plan']['value']['value']['plan']['evidence'][0]['revision'] = 2
                        elif mutation == 'provenance':
                            bad['receipt']['generator_identity_status'] = 'verified'
                        elif mutation == 'before':
                            bad['before_state'] = 'current'
                        else:
                            bad['context']['observed_at'] = '2027-01-01T00:00:00+00:00'
                        # Fresh selected-body and receipt hashes; linkage/meaning
                        # must still reject corruption without original sources.
                        for name in bad['stages']:
                            bad['stages'][name] = review_captures.body_projection(bad['stages'][name]['value'])
                        bad['private_artifacts']['receipt'] = e.binding(c.json_bytes(bad['receipt']))
                        with self.assertRaises((ValueError, TypeError, KeyError)):
                            r.validate(ops.encoded(bad))
                    copied = ops.encoded(value)
                self.assertEqual(r.validate(copied)['before_state'], 'stale')

    def test_historical_change_and_source_basis_survive_copied_lifecycle(self):
        evidence = [
            {'key': 'original-change', 'record_kind': 'checkpoint', 'identity': '09' * 16,
                'revision': 2, 'field': 'state_change', 'sources': ['03' * 16],
                'content': {'reported_change': 'Original change precedes a later verification.', 'observed_at': 100}},
            {'key': 'changed-basis', 'record_kind': 'checkpoint', 'identity': '09' * 16,
                'revision': 2, 'field': 'changed_paths,changed_source_basis', 'sources': ['03' * 16],
                'content': {'paths': ['src/relay.ts'], 'source_ids': ['03' * 16]}}]
        for language in ('en', 'ko'):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary); c.write_json(c.inventory_path(root), c.load_inventory(root))
                directory = publish_fixture(root, language=language, before_state='stale', extra_evidence=evidence)
                data, metadata = r.project(root, directory / 'preparation.json', evidence_set_sha256='b' * 64)
                self.assertTrue(metadata['semantic_complete'])
                value = r.validate(data)
                plan = value['stages']['plan']['value']['value']['plan']
                self.assertEqual(plan['evidence'][1:], evidence)
                answers = value['stages']['after']['value']['value']['selected_work']['answers']
                self.assertEqual(answers['provenance']['uncited_evidence_count'], 2)
                self.assertEqual(len(answers['provenance']['evidence']), 1)
                self.assertEqual(len(value['stages']['record']['value']['value']['explanation']['evidence']), 3)
                # Unsupported future fields must remain an explicit omission.
                bad = copy.deepcopy(plan); bad['evidence'][1]['content']['future_unregistered_field'] = 'unknown'
                self.assertFalse(a.project({'operation': 'explanation_prepare', 'plan': bad}, 'explanation_prepare')['semantic_complete'])
            self.assertTrue(r.validate(data)['semantic_complete'])

    def test_private_lifecycle_prose_is_omitted_before_package_selection(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); c.write_json(c.inventory_path(root), c.load_inventory(root))
            directory = publish_fixture(root, paragraph_text='access_token=synthetic-sensitive-value-513729')
            # Private exact Product receipt/response remains bound and valid.
            e.verify(root, directory / 'preparation.json')
            data, meta = r.project(root, directory / 'preparation.json', evidence_set_sha256='b' * 64)
            self.assertFalse(meta['semantic_complete'])
            self.assertNotIn(b'synthetic-sensitive-value-513729', data)
            value = r.validate(data)
            self.assertEqual(value['stages']['response']['reason'], 'sensitive_payload')
            self.assertEqual(value['receipt']['generator_identity_status'], 'self_reported_not_independently_verified')


if __name__ == '__main__':
    unittest.main()
