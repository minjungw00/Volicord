"""Original invocation versus current reading controls, including frozen real input."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest

import inputs as i
import render_self_test as baseline
from render_comparison import card, render, record_feedback


class DerivedReadingTests(unittest.TestCase):
    def setUp(self):
        self.h = baseline.RenderTests(methodName='runTest')
        self.h.setUp()
        self.addCleanup(self.h.doCleanups)

    def reader_response(self):
        response = copy.deepcopy(self.h.response)
        size = len(response['prose'].encode())
        response.update(claims=[{'start': 0, 'end': size, 'kind': 'model_interpretation', 'selections': [0, 1]}],
                        primary_sites=[{'selections': [0, 1], 'claims': [0], 'reason': 'Authored paired site.'}])
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(status='invalid_response', output_contract='reader_oriented', generation_output=None,
                             original_outputs=[i.binding(self.h.response_path)])
        return response

    def test_failed_original_has_valid_current_diagnostic_preview_with_independent_authorities(self):
        response = self.reader_response()
        before = self.h.response_path.read_bytes()
        body, witness, parser = self.h.present()
        self.assertEqual(witness['original_invocation']['status'], 'invalid_response')
        self.assertIsNone(witness['original_invocation']['generation_output'])
        receipt = witness['outputs'][0]['derived_reading']
        self.assertEqual(receipt['status'], 'valid_binding')
        self.assertEqual(receipt['input'], self.h.record['input'])
        self.assertEqual(receipt['output'], self.h.record['original_outputs'][0])
        self.assertEqual(len(receipt['source_dependencies']), 2)
        for verifier in receipt['verifier']:
            i.check_binding(verifier)
        self.assertIn('Original invocation outcome: invalid_response', body)
        self.assertIn('Current derived reading: valid_binding · diagnostic preview', body)
        self.assertIn('not Product completion', receipt['authority'])
        self.assertNotIn('No verified final explanation', body)
        self.assertEqual(parser.values['prose'], [response['prose']])
        self.assertEqual(self.h.response_path.read_bytes(), before)

    def test_failure_states_are_distinct_and_never_grant_valid_preview(self):
        self.reader_response()
        self.h.record['original_outputs'] = []
        body, witness, _ = self.h.present()
        self.assertEqual(witness['outputs'], [])
        self.assertIn('No generated output', body)
        self.h.record['original_outputs'] = [i.binding(self.h.response_path)]
        self.h.response_path.unlink()
        body, witness, _ = self.h.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'unavailable_response')
        self.assertIn('Response availability: missing', body)
        self.h.response_path.write_bytes(b'{bad json')
        self.h.record['original_outputs'] = [i.binding(self.h.response_path)]
        body, witness, _ = self.h.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'invalid_response')
        self.assertNotIn('Current derived reading: valid_binding', body)

    def test_current_invalid_primary_and_stale_source_are_separate_states(self):
        response = self.reader_response()
        response['selections'][0]['state'] = 'before'
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record['original_outputs'] = [i.binding(self.h.response_path)]
        body, witness, _ = self.h.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'invalid')
        self.assertIn('wrong-state span', body)
        frozen = json.loads(Path(self.h.record['input']['path']).read_bytes())
        Path(frozen['entries'][0]['asset']['path']).write_bytes(b'tampered')
        body, witness, parser = self.h.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'unavailable_input')
        self.assertNotIn('code', parser.values)
        self.assertNotIn('diff', parser.values)
        self.assertNotIn('Current derived reading: valid_binding', body)

    def test_changed_response_or_input_receipt_is_rejected(self):
        self.reader_response()
        self.h.response_path.write_bytes(b'tampered')
        with self.assertRaisesRegex(ValueError, 'changed bytes'):
            self.h.present()
        self.h.record['input']['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'changed bytes'):
            self.h.present()

    def canonical_rows(self, rows):
        frozen_path = Path(self.h.record['input']['path'])
        frozen = json.loads(frozen_path.read_bytes())
        template = frozen['entries'][0]
        for index, (table, row) in enumerate(rows):
            path = self.h.root / ('canonical-' + str(index))
            path.write_bytes(i.encoded({'table': table, 'row': row}))
            entry = copy.deepcopy(template)
            entry.update(id='canonical-' + str(index), role='canonical_record', path='canonical/' + table,
                         locator='independent-row/' + str(index), asset=i.binding(path), origin=i.binding(path),
                         file_sha256=i.binding(path)['sha256'], representation='full_file')
            frozen['entries'].append(entry)
        frozen_path.write_bytes(i.encoded(frozen))
        self.h.record['input'] = i.binding(frozen_path)

    def test_recorded_direction_and_independent_states_use_latest_exact_work(self):
        project, work = self.h.record['scope']['project'], self.h.record['scope']['work']
        checkpoint = {'id': 'cp', 'project_id': project, 'work_item_id': work, 'revision': 2,
                      'recorded_at': 2, 'goal': 'An authored user problem.', 'next_step': 'Review the actual diff.',
                      'work_state': 'completed', 'user_review': 'not_requested', 'user_acceptance': 'rejected'}
        source = {'id': 'command', 'project_id': project, 'locator': 'Authored failed command',
                  'exit_code': 101, 'termination': 'exited', 'availability': 'available'}
        self.canonical_rows([('checkpoints', dict(checkpoint, id='old', recorded_at=1, next_step='Obsolete action')),
                             ('checkpoints', checkpoint),
                             ('checkpoints', dict(checkpoint, id='foreign', recorded_at=5, work_item_id='another-work')),
                             ('checkpoint_verifications', {'project_id': project, 'checkpoint_id': 'cp',
                                  'source_id': 'command', 'verification_state': 'failed', 'outcome': 'Authored failure.'}),
                             ('sources', source)])
        body, witness, parser = self.h.present()
        self.assertEqual(witness['recorded_work']['checkpoint']['id'], 'cp')
        self.assertIn('Review the actual diff.', body)
        self.assertEqual(parser.values['direction'], ['Review the actual diff.'])
        self.assertIn('Obsolete action', body)  # Historical disclosure, never current direction.
        self.assertIn('User review: not_requested. User acceptance: rejected', body)
        self.assertIn('Recorded command exit: 101', body)
        self.assertIn('Recorded failed verification observations: 1', body)
        self.assertLess(body.index('class="prose"'), body.index('An authored user problem.'))
        self.assertIn('class="work-problem-basis"', body)
        self.assertNotIn(str(self.h.root), body)

    def test_blank_latest_direction_and_tied_chronology_do_not_reuse_old_action(self):
        scope = self.h.record['scope']
        cp = {'id': 'cp', 'project_id': scope['project'], 'work_item_id': scope['work'],
              'revision': 1, 'recorded_at': 2, 'next_step': ''}
        self.canonical_rows([('checkpoints', dict(cp, id='old', recorded_at=1, next_step='Obsolete direction')),
                             ('checkpoints', cp)])
        body, witness, parser = self.h.present()
        self.assertIn('No next meaningful action recorded', body)
        self.assertNotIn('direction', parser.values)
        self.assertIn('Obsolete direction', body)  # Original course remains inspectable.
        self.canonical_rows([('checkpoints', dict(cp, id='tie', next_step='Ambiguous direction'))])
        body, witness, _ = self.h.present()
        self.assertIsNone(witness['recorded_work']['checkpoint'])
        self.assertNotIn('Ambiguous direction', body)

    def test_new_feedback_binds_new_display_and_unchanged_output_old_feedback_stays_old(self):
        self.reader_response()
        self.h.attempt.write_bytes(i.encoded(self.h.record))
        old = render([self.h.attempt], self.h.root / 'old-display')
        old_feedback = record_feedback(old, None, self.h.root / 'old-feedback.json')
        retained = old_feedback.read_bytes()
        # A genuine presentation difference with the same response: label blinding order.
        new = render([self.h.attempt], self.h.root / 'new-display')
        from render_comparison import document
        body, witness = card(self.h.attempt, 'Sample 02')
        new.write_text(document([body]))
        integrity_path = new.parent / 'integrity.json'
        integrity = json.loads(integrity_path.read_bytes())
        integrity.update(presentation=i.binding(new), samples=[witness])
        integrity_path.write_bytes(i.encoded(integrity))
        feedback = record_feedback(new, 'Authored agent observation.', self.h.root / 'new-feedback.json', reviewer_kind='agent')
        value = json.loads(feedback.read_bytes())
        self.assertEqual(value['original_outputs'], self.h.record['original_outputs'])
        self.assertNotEqual(value['presentation']['sha256'], json.loads(retained)['presentation']['sha256'])
        self.assertEqual(old_feedback.read_bytes(), retained)
        self.assertEqual(value['H1'], 'pending')

    @unittest.skipUnless(os.environ.get('EXPLANATION_ACTUAL_ATTEMPT'), 'explicit frozen real attempt path required')
    def test_actual_original_invalid_derived_valid_render_and_feedback(self):
        path = Path(os.environ['EXPLANATION_ACTUAL_ATTEMPT'])
        original = i.binding(path)
        self.assertEqual(original['sha256'], 'f212b4d4bcbac9c4ff79a04ec29dfa0e7f5b67f8e76f6b82b4dd6d074007108a')
        record = json.loads(path.read_bytes())
        self.assertEqual(record['original_outputs'][0]['sha256'], 'd2b1d512288c4eaad46f99b24fae47aabef0e5ce1419bf376c7a8511932f7aa4')
        body, witness = card(path, 'Sample 01')
        self.assertEqual(witness['original_invocation']['status'], 'invalid_response')
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
        self.assertEqual(witness['recorded_work']['checkpoint']['id'], '16e50558f673108832a1b0df679a70d4')
        self.assertIn('Review the three-file CLI guidance diff.', body)
        self.assertIn('User review: not_requested. User acceptance: not_requested', body)
        self.assertEqual(len(witness['recorded_work']['verification']), 10)
        self.assertEqual(sum(f['observation']['verification_state'] == 'failed'
                             for f in witness['recorded_work']['verification']), 3)
        primary = witness['outputs'][0]['primary_comparisons'][0]['comparisons'][0]
        self.assertEqual((primary['displayed_hunks'], primary['remaining_hunks']), (3, 15))
        # Independently inspected original source lines: old 1275 is the if;
        # new 1275-1277 dispatch repository analysis before the previous branch.
        parser = baseline.Texts(); parser.feed(body)
        first = parser.values['diff'][0]
        self.assertIn('@@ -1273,5 +1273,7 @@', first)
        self.assertIn('-                if [\n', first)
        self.assertIn('+                if *key == "repository_analysis" {\n', first)
        self.assertIn('+                    render_analysis_status(field, mode.locale, stdout)?;\n', first)
        self.assertIn('@@ -1323,2 +1325,14 @@', parser.values['diff'][1])
        self.assertIn('+fn render_analysis_status(\n', parser.values['diff'][1])
        with tempfile.TemporaryDirectory() as directory:
            display = render([path], Path(directory) / 'display')
            feedback = record_feedback(display, None, Path(directory) / 'feedback.json')
            self.assertEqual(json.loads(feedback.read_bytes())['original_outputs'], record['original_outputs'])
        self.assertEqual(i.binding(path), original)
        for output in record['original_outputs']:
            i.check_binding(output)

    @unittest.skipUnless(os.environ.get('EXPLANATION_ACTUAL_NO_CHANGE_ATTEMPT'), 'explicit frozen no-change attempt path required')
    def test_actual_no_change_investigation_preserves_prose_and_sites_without_diff(self):
        path = Path(os.environ['EXPLANATION_ACTUAL_NO_CHANGE_ATTEMPT'])
        record = json.loads(path.read_bytes())
        original = i.binding(path)
        self.assertEqual(record['original_outputs'][0]['sha256'], '62e756ee205ad37b292f8e3c1fb92ce916b0c0f507849eb43eab629274598eaa')
        response = json.loads(Path(record['original_outputs'][0]['path']).read_bytes())
        body, witness = card(path, 'Sample 01')
        output = witness['outputs'][0]
        self.assertEqual(output['derived_reading']['status'], 'valid_binding')
        self.assertEqual(output['comparisons'], [])
        self.assertTrue(all(s['state'] == 'context' for s in response['selections']))
        self.assertEqual([s['site'] for s in output['reading']['sites']], response['primary_sites'])
        parser = baseline.Texts(); parser.feed(body)
        self.assertEqual(parser.values['prose'], [response['prose']])
        self.assertNotIn('diff', parser.values)
        self.assertNotIn('full-diff', parser.values)
        self.assertEqual(i.binding(path), original)


if __name__ == '__main__':
    unittest.main()
