"""Authored authority regressions through real source readers and HTML consumers.

These evidence expectations establish roles/coordinates, not general prose truth.
"""
import copy
import json
from pathlib import Path
import unittest

import inputs as i
import render_self_test as fixture
from grounding import validate_output
from reader_contract import DIRECTED_CONTRACT
from source_tools import Surface


class WorkReaderTests(unittest.TestCase):
    def setUp(self):
        self.h = fixture.RenderTests(methodName='runTest')
        self.h.setUp(); self.addCleanup(self.h.doCleanups)
        self.h.record['output_contract'] = DIRECTED_CONTRACT
        self.response = self.h.response
        self.response.update(claims=[{'start': 0, 'end': len(self.response['prose'].encode()),
            'kind': 'source_fact', 'selections': [0, 1], 'authority': None}],
            primary_sites=[{'selections': [0, 1], 'claims': [0], 'reason': 'Authored source pair.'}])
        scope = self.h.record['scope']
        self.cp = {'id': 'cp', 'revision': 3, 'project_id': scope['project'],
            'work_item_id': scope['work'], 'recorded_at': 3, 'goal': 'Inspect retry behavior.',
            'next_step': 'Review the retry branch before extending policy.', 'work_state': 'completed',
            'state_change': 'Retry branch changed.', 'known_limits': 'Concurrent requests unverified.',
            'user_review': 'not_requested', 'user_acceptance': 'not_requested'}

    def evidence(self, role, text, *, table=None):
        spec = i.verify(self.h.manifest)
        entry = copy.deepcopy(spec['entries'][0]); identity = 'evidence-' + str(len(spec['entries']))
        path = self.h.root / identity
        path.write_bytes(i.encoded({'table': table, 'row': text}) if table else text.encode())
        entry.update(id=identity, role=role, path='records/' + identity, locator='original/' + identity,
            attribution='original_record', before_state='unavailable', origin=i.binding(path),
            asset=i.binding(path), file_sha256=i.binding(path)['sha256'])
        spec['entries'].append(entry)
        self.h.manifest.write_bytes(i.encoded(spec)); self.h.record['input'] = i.binding(self.h.manifest)
        surface = Surface(self.h.manifest, 'archive_diagnostic', {'reads': 100, 'read_bytes': 100000}, self.h.root / 'trace')
        result = surface.call('read', {'id': identity, 'limit': 10000})
        self.response['selections'].append({'id': identity, 'start': 0, 'end': result['result']['metadata']['bytes'],
            'sha256': result['result']['metadata']['sha256'], 'state': 'context'})
        self.h.record['retrievals'] = i.binding(surface.trace); self.h.host_reads(surface.trace)
        return len(self.response['selections']) - 1

    def claim(self, kind, selection, row=None, field=None):
        # Separate authored statement; no case-specific benchmark expected prose.
        start = len(self.response['prose'].encode()) + 1
        text = 'Independent evidence-bound statement.'
        self.response['prose'] += '\n' + text
        self.response['claims'].append({'start': start, 'end': start + len(text.encode()), 'kind': kind,
            'selections': [selection], 'authority': {'selection': selection,
                'record_id': row['id'] if row else None, 'revision': row['revision'] if row else None, 'field': field}})

    def validation(self):
        return validate_output(self.response, i.verify(self.h.manifest), 'archive_diagnostic',
            [json.loads(line) for line in (self.h.root / 'trace').read_bytes().splitlines()], contract=DIRECTED_CONTRACT)

    def present(self):
        self.h.response_path.write_bytes(i.encoded(self.response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        return self.h.present()

    def test_conditional_task_cannot_be_choice_rationale_or_learning_consent(self):
        index = self.evidence('task', 'Investigate reconnect behavior; if a defect is demonstrated, fix it. Otherwise report uncertainty.')
        self.claim('task_instruction', index)
        self.assertEqual(self.validation()['reading']['status'], 'valid_binding')
        body, witness, parser = self.present()
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
        for kind in ('user_choice', 'user_rationale', 'learning_consent'):
            self.response['claims'][-1]['kind'] = kind
            with self.assertRaises(ValueError): self.validation()
            body, witness, _ = self.present()
            self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'invalid_response')
            self.assertNotIn('class="primary-sites"', body)

    def test_explicit_choice_and_missing_rationale_are_separate(self):
        scope = self.h.record['scope']
        row = {'id': 'decision', 'revision': 2, 'project_id': scope['project'], 'work_item_id': scope['work'],
            'choice_value': 'bounded_retry', 'user_rationale': None, 'recommendation_rationale': 'Limit waiting.'}
        index = self.evidence('canonical_record', row, table='decisions')
        self.claim('user_choice', index, row, 'choice_value')
        self.assertEqual(self.validation()['reading']['status'], 'valid_binding')
        self.claim('user_rationale', index, row, 'user_rationale')
        with self.assertRaisesRegex(ValueError, 'field absent'): self.validation()
        self.response['claims'][-1]['authority']['field'] = 'recommendation_rationale'
        with self.assertRaisesRegex(ValueError, 'wrong canonical authority field'): self.validation()
        self.response['claims'][-1]['kind'] = 'agent_recommendation'
        self.assertEqual(self.validation()['reading']['status'], 'valid_binding')

    def test_actual_user_response_rationale_keeps_original_provenance(self):
        index = self.evidence('user_response', 'Choose bounded retries because indefinite waiting hides service failures.')
        self.claim('user_choice', index); self.claim('user_rationale', index)
        self.assertEqual(self.validation()['reading']['status'], 'valid_binding')
        body, witness, _ = self.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
        self.assertIn('entailment unassessed', body)

    def test_latest_recorded_action_survives_generated_omission_and_later_report(self):
        index = self.evidence('canonical_record', self.cp, table='checkpoints')
        self.evidence('agent_report', 'Only verification was repeated; it passed in this bounded scope.')
        body, witness, parser = self.present()
        self.assertEqual(witness['recorded_work']['checkpoint'], self.cp)
        self.assertEqual(parser.values['direction'], [self.cp['next_step']])
        self.assertIn('Concurrent requests unverified.', body)
        self.assertIn('Retry branch changed.', body)
        self.assertIn('User acceptance: not_requested', body)
        self.claim('recorded_next_action', index, self.cp, 'next_step')
        self.assertEqual(self.validation()['reading']['status'], 'valid_binding')
        self.response['claims'][-1]['authority']['revision'] = 2
        with self.assertRaisesRegex(ValueError, 'revision mismatch'): self.validation()

    def test_changed_blank_superseded_foreign_and_missing_action(self):
        old = dict(self.cp, id='old', recorded_at=1, next_step='Obsolete recorded action.')
        index = self.evidence('canonical_record', old, table='checkpoints')
        self.evidence('canonical_record', self.cp, table='checkpoints')
        self.claim('recorded_next_action', index, old, 'next_step')
        with self.assertRaisesRegex(ValueError, 'latest applicable'): self.validation()
        self.response['claims'].pop()
        body, witness, parser = self.present()
        self.assertEqual(parser.values['direction'], [self.cp['next_step']])
        self.evidence('canonical_record', dict(self.cp, id='foreign', recorded_at=99, work_item_id='other'), table='checkpoints')
        self.assertEqual(self.present()[1]['recorded_work']['checkpoint']['id'], 'cp')
        self.evidence('canonical_record', dict(self.cp, id='blank', recorded_at=4, next_step=''), table='checkpoints')
        body, witness, parser = self.present()
        self.assertEqual(witness['recorded_work']['direction_status'], 'absent')
        self.assertNotIn('direction', parser.values)
        self.evidence('canonical_record', dict(self.cp, id='replaced', recorded_at=5, work_state='superseded'), table='checkpoints')
        body, witness, parser = self.present()
        self.assertEqual(witness['recorded_work']['direction_status'], 'superseded')
        self.assertNotIn('direction', parser.values)
        self.assertIn('historical action is not an applicable next step', body)

    def test_foreign_record_and_after_cutoff_authority_fail(self):
        index = self.evidence('canonical_record', dict(self.cp, work_item_id='foreign'), table='checkpoints')
        self.claim('recorded_next_action', index, self.cp, 'next_step')
        with self.assertRaisesRegex(ValueError, 'foreign canonical'): self.validation()
        self.response['claims'].pop()
        spec = i.verify(self.h.manifest)
        spec['entries'][-1]['chronology']['observed_at'] = '2999-01-01T00:00:00+00:00'
        trace = [json.loads(line) for line in (self.h.root / 'trace').read_bytes().splitlines()]
        result = validate_output(self.response, spec, 'archive_diagnostic', trace, contract=DIRECTED_CONTRACT)
        self.assertEqual(result['reading']['status'], 'invalid')
        self.assertIn('source after cutoff', result['selections'][-1]['issues'])

    def test_absent_withheld_and_unordered_direction_stays_unavailable(self):
        self.assertIsNone(self.present()[1]['recorded_work']['checkpoint'])
        self.evidence('canonical_record', self.cp, table='checkpoints')
        spec = i.verify(self.h.manifest)
        spec['entries'][-1]['chronology'] = {'state': 'ambiguous', 'reason': 'Original order unavailable.'}
        self.h.manifest.write_bytes(i.encoded(spec)); self.h.record['input'] = i.binding(self.h.manifest)
        body, witness, parser = self.present()
        self.assertIsNone(witness['recorded_work']['checkpoint'])
        self.assertNotIn('direction', parser.values)
        self.assertIn('no uniquely ordered same-Work Checkpoint', body)

    def test_wrong_field_partial_row_and_missing_authority_fail(self):
        index = self.evidence('canonical_record', self.cp, table='checkpoints')
        self.claim('recorded_next_action', index, self.cp, 'next_step')
        authority = self.response['claims'][-1]['authority']
        authority['field'] = 'goal'
        with self.assertRaisesRegex(ValueError, 'wrong canonical'): self.validation()
        authority['field'] = 'next_step'
        sel = self.response['selections'][index]; sel['end'] -= 1
        entry = i.verify(self.h.manifest)['entries'][-1]
        sel['sha256'] = i.digest(Path(entry['asset']['path']).read_bytes()[:sel['end']])
        with self.assertRaisesRegex(ValueError, 'complete retrieved row'): self.validation()
        self.response['claims'][-1]['authority'] = None
        with self.assertRaisesRegex(ValueError, 'authority coordinates'): self.validation()


if __name__ == '__main__':
    unittest.main()
