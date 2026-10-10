"""Evidence-only selections through original bytes, HTML and feedback consumers.

Authored fixtures supply the oracle. Optional retained Click input is read offline;
no model invocation, original response repair or chronology inference is performed.
"""
import base64
import copy
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import inputs as i
from input_self_test import spec
from grounding import validate_output
from reader_contract import DIRECTED_CONTRACT
from render_comparison import card, render, record_feedback
import render_self_test as baseline
from source_tools import Surface

HERE = Path(__file__).parent
CASES = json.loads((HERE / 'selection-fixtures/cases.json').read_bytes())


class SelectionReadingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        source = self.root / 'source'; source.write_text(CASES['source'])
        value = spec(source)
        value['entries'][0].update(id='primary-source', path='src/main.py')
        template = value['entries'][0]
        for identity, role, text in (
                ('secondary-source', 'source', CASES['secondary']),
                ('unicode-source', 'source', CASES['unicode']),
                ('decision', 'canonical_record', json.dumps({'table': 'decisions', 'row': CASES['decision']})),
                ('uncertain-checkpoint', 'canonical_record', json.dumps({'table': 'checkpoints', 'row': CASES['checkpoint']}))):
            path = self.root / identity; path.write_text(text)
            entry = copy.deepcopy(template)
            entry.update(id=identity, role=role, path='src/' + identity + '.py', locator='authored/' + identity,
                         origin=i.binding(path), file_sha256=i.binding(path)['sha256'])
            if identity == 'uncertain-checkpoint':
                entry['chronology'] = CASES['chronology']['ambiguous']
            value['entries'].append(entry)
        unavailable = copy.deepcopy(template)
        unavailable.update(id='unavailable-source', representation='unavailable', origin=None,
                           file_sha256=None, missing='Original body not retained.')
        value['entries'].append(unavailable)
        preparation = self.root / 'spec.json'; preparation.write_bytes(i.encoded(value))
        self.manifest = i.freeze(preparation, self.root / 'frozen', self.root)
        self.spec = i.verify(self.manifest)
        self.trace = self.root / 'retrievals.jsonl'
        surface = Surface(self.manifest, 'archive_diagnostic', {'reads': 20, 'read_bytes': 10000}, self.trace)
        self.selections = {}
        for entry in self.spec['entries']:
            if entry['asset'] is None:
                continue
            returned = surface.call('read', {'id': entry['id'], 'limit': 2048})
            data = Path(entry['asset']['path']).read_bytes()
            # Independent byte oracle, rather than trusting a reader/validator digest.
            self.assertEqual(returned['result']['text'].encode(), data)
            self.selections[entry['id']] = {'id': entry['id'], 'start': 0, 'end': len(data),
                'sha256': hashlib.sha256(data).hexdigest(), 'state': 'context'}
        prose = 'Authored mechanism. Authored user choice.'
        self.response = {'prose': prose, 'selections': [self.selections[k] for k in
            ('primary-source', 'secondary-source', 'decision')], 'gaps': [CASES['gap']],
            'claims': [{'start': 0, 'end': 19, 'kind': 'source_fact', 'selections': [0], 'authority': None},
                       {'start': 20, 'end': len(prose.encode()), 'kind': 'user_choice', 'selections': [2],
                        'authority': {'selection': 2, 'record_id': 'decision', 'revision': 3,
                                      'field': 'decisions.choice_value'}}],
            'primary_sites': [{'selections': [0], 'claims': [0], 'reason': 'Authored return expression.',
                               'extent_reason': 'Declaration and return are the complete authored mechanism.'}]}
        self.output = self.root / 'original.json'
        self.attempt = self.root / 'attempt.json'
        self.record = {'input': i.binding(self.manifest), 'scope': self.spec['scope'],
            'status': 'captured', 'lane': 'archive_diagnostic', 'approach': 'direct',
            'output_contract': DIRECTED_CONTRACT, 'blockers': [], 'clean_comparison': False}
        self.save_trace()

    def save_trace(self, omit=None):
        rows = [json.loads(line) for line in self.trace.read_bytes().splitlines()]
        host = self.root / 'host.jsonl'
        host.write_bytes(b''.join(i.encoded({'type': 'item.completed', 'item': {
            'id': str(row['sequence']), 'type': 'mcp_tool_call', 'server': 'evidence',
            'tool': row['name'], 'arguments': row['arguments'], 'status': 'completed',
            'result': {'content': [{'type': 'text', 'text': json.dumps(row)}]}}}).replace(b'\n', b'') + b'\n'
            for row in rows if row['arguments'].get('id') != omit))
        self.record.update(retrievals=i.binding(self.trace), calls=[{'kind': 'model_call',
                           'process': {'stdout': i.binding(host), 'exit_code': 0}}])

    def present(self, response=None):
        self.output.write_bytes(i.encoded(response or self.response))
        self.record.update(original_outputs=[i.binding(self.output)], generation_output=i.binding(self.output))
        self.attempt.write_bytes(i.encoded(self.record))
        body, witness = card(self.attempt, 'Sample 01')
        parser = baseline.Texts(); parser.feed(body)
        self.assertEqual(parser.values['prose'], [(response or self.response)['prose']])
        self.assertIn(self.output.read_bytes(), [base64.b64decode(link.split(',', 1)[1])
                      for link in parser.links if link.startswith('data:')])
        return body, witness['outputs'][0], parser

    def assert_feedback(self, status):
        display = render([self.attempt], self.root / 'display')
        feedback = record_feedback(display, 'Authored observation, not human comprehension evidence.',
                                   self.root / 'feedback.json', reviewer_kind='agent')
        value = json.loads(feedback.read_bytes())
        self.assertEqual(value['original_outputs'], self.record['original_outputs'])
        self.assertEqual(value['H1'], 'pending')
        integrity = json.loads((display.parent / 'integrity.json').read_bytes())
        self.assertEqual(integrity['samples'][0]['outputs'][0]['derived_reading']['status'], status)
        self.assertEqual(value['integrity'], i.binding(display.parent / 'integrity.json'))
        # New feedback cannot overwrite the previous observation.
        retained = feedback.read_bytes()
        with self.assertRaises(FileExistsError):
            record_feedback(display, None, feedback)
        self.assertEqual(feedback.read_bytes(), retained)

    def test_verified_primary_claim_authority_and_unbound_secondary_reach_feedback(self):
        body, output, parser = self.present()
        self.assertEqual(output['derived_reading']['status'], 'valid_binding')
        self.assertEqual(output['claim_authority']['status'], 'valid_binding')
        self.assertIn('class="primary-sites"', body)
        self.assertIn(CASES['source'], parser.values['code'])
        self.assertIn(CASES['secondary'], parser.values['code'])
        self.assertEqual([json.loads(s) for s in parser.values['selection-request']], self.response['selections'])
        self.assert_feedback('valid_binding')

    def test_one_invalid_secondary_fails_whole_reading_without_discarding_valid_evidence(self):
        self.response['selections'].append(self.selections['uncertain-checkpoint'])
        original = copy.deepcopy(self.response)
        body, output, parser = self.present()
        self.assertEqual(output['derived_reading']['status'], 'invalid')
        self.assertEqual(output['reading']['issues'], ['invalid secondary/claim reference'])
        self.assertEqual(output['claim_authority']['status'], 'withheld_invalid_binding')
        self.assertNotIn('class="primary-sites"', body)
        self.assertIn('ambiguous chronology', body)
        self.assertEqual([json.loads(s) for s in parser.values['selection-request']], original['selections'])
        self.assertEqual(len(output['selections']), 4)
        self.assertIn(CASES['source'], parser.values['code'])
        self.assertIn(CASES['secondary'], parser.values['code'])
        self.assertNotIn(json.dumps({'table': 'checkpoints', 'row': CASES['checkpoint']}), parser.values['code'])
        self.assertEqual(output['selections'][-1]['reference_status'].split(';')[0], 'invalid')
        self.assert_feedback('invalid')
        self.assertEqual(json.loads(self.output.read_bytes()), original)

    def test_gaps_only_uncertainty_keeps_full_inventory_and_append_only_retrieval(self):
        before = self.trace.read_bytes()
        surface = Surface(self.manifest, 'archive_diagnostic', {'reads': 20, 'read_bytes': 10000}, self.trace)
        surface.call('inventory', {})
        self.assertTrue(self.trace.read_bytes().startswith(before))
        self.save_trace()
        body, output, _ = self.present()
        self.assertEqual(output['derived_reading']['status'], 'valid_binding')
        self.assertIn(CASES['gap'], body)
        self.assertEqual(i.verify(self.manifest)['entries'], self.spec['entries'])
        self.assertEqual(self.spec['entries'][4]['chronology'], CASES['chronology']['ambiguous'])
        self.assert_feedback('valid_binding')

    def test_secondary_negative_matrix_through_html(self):
        for case, reason in CASES['rejections'].items():
            with self.subTest(case=case):
                value = copy.deepcopy(self.response)
                frozen = copy.deepcopy(self.spec)
                selection = value['selections'][1]
                entry = frozen['entries'][1]
                if case in {'ambiguous', 'future'}:
                    entry['chronology'] = CASES['chronology'][case]
                elif case.startswith('foreign_'):
                    entry[case.removeprefix('foreign_')] = 'other'
                elif case == 'wrong_state': selection['state'] = 'before'
                elif case == 'wrong_hash': selection['sha256'] = '0' * 64
                elif case == 'unavailable': selection['id'] = 'unavailable-source'
                trace = [json.loads(line) for line in self.trace.read_bytes().splitlines()]
                if case == 'absent_retrieval':
                    trace = [row for row in trace if row['arguments']['id'] != selection['id']]
                validation = validate_output(value, frozen, 'archive_diagnostic', trace, contract=DIRECTED_CONTRACT)
                self.assertEqual(validation['selections'][1]['reference_status'], 'invalid')
                self.assertIn(reason, validation['selections'][1]['issues'])
                self.assertEqual(validation['reading']['status'], 'invalid')
                # Attempt/manifest identity remains exact even for malicious metadata.
                self.manifest.write_bytes(i.encoded(frozen)); self.record['input'] = i.binding(self.manifest)
                if case == 'absent_retrieval': self.save_trace(omit=selection['id'])
                else: self.save_trace()
                body, output, parser = self.present(value)
                expected = 'unavailable_input' if case in {'future', 'foreign_work', 'foreign_project'} else 'invalid'
                self.assertEqual(output['derived_reading']['status'], expected)
                self.assertNotIn('class="primary-sites"', body)
                if expected == 'invalid':
                    self.assertEqual([json.loads(s) for s in parser.values['selection-request']], value['selections'])
                    self.assertIn(reason, body)
                else:
                    self.assertNotIn('code', parser.values)
                self.assertNotIn('Current derived reading: valid_binding', body)
        self.manifest.write_bytes(i.encoded(self.spec))

    def test_unknown_identity_is_distinct_from_unavailable_and_ambiguous(self):
        self.response['selections'].append(dict(self.selections['secondary-source'], id='missing-identity'))
        body, output, parser = self.present()
        self.assertEqual(output['derived_reading']['status'], 'invalid')
        self.assertIn("'missing-identity'", output['derived_reading']['issues'])
        self.assertIn('No substitute code', body)
        self.assertEqual(len(parser.values['selection-request']), 4)

    def test_matching_hash_cannot_certify_malformed_utf8_range(self):
        data = CASES['unicode'].encode()
        self.response['selections'].append({'id': 'unicode-source', 'start': 0, 'end': 1,
            'sha256': hashlib.sha256(data[:1]).hexdigest(), 'state': 'context'})
        body, output, parser = self.present()
        self.assertEqual(output['derived_reading']['status'], 'invalid')
        self.assertTrue(any('utf-8' in issue for issue in output['derived_reading']['issues']))
        self.assertNotIn(CASES['unicode'], parser.values['code'])
        self.assertEqual(len(parser.values['selection-request']), 4)

    def test_stale_revision_cannot_support_choice_and_preserves_original_feedback_identity(self):
        self.response['claims'][1]['authority']['revision'] = 2
        body, output, _ = self.present()
        self.assertEqual(output['derived_reading']['status'], 'invalid_response')
        self.assertIn('authority record/revision mismatch', body)
        self.assertNotIn('data-authority-selection', body)
        self.assert_feedback('invalid_response')

    def test_forged_known_timing_cannot_replace_frozen_ambiguous_identity(self):
        self.present()
        forged = copy.deepcopy(self.spec)
        forged['entries'][4]['chronology'] = CASES['chronology']['known']
        self.manifest.write_bytes(i.encoded(forged))
        with self.assertRaisesRegex(ValueError, 'changed bytes or stale preparation'):
            card(self.attempt, 'Sample 01')
        self.assertEqual(json.loads(self.output.read_bytes()), self.response)

    def test_archive_observations_do_not_acquire_product_read_authority(self):
        surface = Surface(self.manifest, 'product', {'reads': 20, 'read_bytes': 10000}, self.root / 'product-trace')
        denied = surface.call('read', {'id': 'uncertain-checkpoint', 'limit': 2048})
        self.assertEqual(denied['status'], 'failed')
        self.assertEqual(denied['outcome'], 'policy_denied')
        self.record['lane'] = 'product'
        body, output, parser = self.present()
        self.assertEqual(output['derived_reading']['status'], 'invalid_response')
        self.assertNotIn('code', parser.values)
        self.assertNotIn('direction', parser.values)
        self.assertNotIn('class="primary-sites"', body)

    def test_ambiguous_and_future_evidence_cannot_support_claim_primary_or_authority(self):
        for chronology in ('ambiguous', 'future'):
            for use in ('claim', 'primary', 'user_choice', 'recorded_next_action'):
                with self.subTest(chronology=chronology, use=use):
                    value = copy.deepcopy(self.response); frozen = copy.deepcopy(self.spec)
                    index = 4 if use == 'recorded_next_action' else 0 if use in {'claim', 'primary'} else 3
                    entry = frozen['entries'][index]; entry['chronology'] = CASES['chronology'][chronology]
                    if use == 'recorded_next_action':
                        value['selections'][2] = self.selections['uncertain-checkpoint']
                        claim = value['claims'][1]; claim['kind'] = use
                        claim['authority'].update(record_id='checkpoint', revision=2, field='checkpoints.next_step')
                    trace = [json.loads(line) for line in self.trace.read_bytes().splitlines()]
                    if use in {'user_choice', 'recorded_next_action'}:
                        with self.assertRaisesRegex(ValueError, 'invalid authority reference'):
                            validate_output(value, frozen, 'archive_diagnostic', trace, contract=DIRECTED_CONTRACT)
                    else:
                        validation = validate_output(value, frozen, 'archive_diagnostic', trace, contract=DIRECTED_CONTRACT)
                        self.assertEqual(validation['reading']['status'], 'invalid')
                        self.assertEqual(validation['reading']['sites'][0]['status'], 'invalid')
                    self.manifest.write_bytes(i.encoded(frozen)); self.record['input'] = i.binding(self.manifest)
                    body, output, _ = self.present(value)
                    self.assertNotEqual(output['derived_reading']['status'], 'valid_binding')
                    self.assertNotIn('class="primary-sites"', body)
                    self.assertNotIn('data-authority-selection', body)
        self.manifest.write_bytes(i.encoded(self.spec))


class RetainedSelectionReadingTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('EXPLANATION_ACTUAL_UNCERTAIN_ATTEMPT'), 'explicit retained Click attempt required')
    def test_original_click_failure_frozen_validation_current_html_and_feedback(self):
        path = Path(os.environ['EXPLANATION_ACTUAL_UNCERTAIN_ATTEMPT']).resolve()
        self.assertEqual(i.binding(path)['sha256'], '2f56afc681451392a66e2669a8aacc811420a16fa0db0f4c85b929fe1d453c55')
        record = json.loads(path.read_bytes())
        self.assertEqual(record['status'], 'invalid_response')
        self.assertIsNone(record['generation_output'])
        self.assertEqual(record['input']['sha256'], '946977bacc2029f597a1e095f57cf060771718bf8763821d1ccc8541badb1f33')
        self.assertEqual(record['original_outputs'][0]['sha256'], '3a7f86d34812eb34e1d06c75d7a28bb2dc972d19c4443ee1b58a41b6f866bd8c')
        roots = [path.parent.parent, Path(record['input']['path']).parent]
        retained = {p: hashlib.sha256(p.read_bytes()).hexdigest() for root in roots for p in root.rglob('*') if p.is_file()}
        for key in ('input', 'retrievals', 'frozen_conditions', 'frozen_instructions', 'frozen_response_schema'):
            i.check_binding(record[key])
        for source in record['frozen_support']:
            i.check_binding(source['snapshot'])
            self.assertEqual(source['snapshot']['sha256'], source['origin']['sha256'])
            repository = HERE.resolve().parents[4]
            relative = Path(source['origin']['path']).relative_to(repository).as_posix()
            git = subprocess.run(['git', 'show', 'b27e1316:' + relative], cwd=repository,
                                 capture_output=True, timeout=10)
            self.assertEqual(git.returncode, 0, git.stderr.decode())
            self.assertEqual(hashlib.sha256(git.stdout).hexdigest(), source['snapshot']['sha256'])
        for output in record['original_outputs']: i.check_binding(output)
        spec_value = i.verify(record['input']['path'])
        response = json.loads(Path(record['original_outputs'][0]['path']).read_bytes())
        trace = [json.loads(line) for line in Path(record['retrievals']['path']).read_bytes().splitlines()]
        entries = {e['id']: e for e in spec_value['entries']}
        uncertain = []
        for index, selection in enumerate(response['selections']):
            entry = entries[selection['id']]
            whole = Path(entry['asset']['path']).read_bytes()
            self.assertEqual(hashlib.sha256(whole).hexdigest(), entry['asset']['sha256'])
            self.assertEqual((entry['project'], entry['work']), (record['scope']['project'], record['scope']['work']))
            data = whole[selection['start']:selection['end']]
            self.assertEqual(hashlib.sha256(data).hexdigest(), selection['sha256'])
            data.decode('utf-8')
            if entry['chronology']['state'] == 'ambiguous': uncertain.append(index)
            else:
                observed = dt.datetime.fromisoformat(entry['chronology']['observed_at'])
                self.assertIsNotNone(observed.tzinfo)
                self.assertLessEqual(observed, dt.datetime.fromisoformat(record['scope']['cutoff']))
            # Independently verify returned source bytes and selected coverage.
            # A byte mask supplies a separate oracle from the validator's interval cursor.
            coverage = bytearray(len(data))
            for row in trace:
                if row['name'] != 'read' or row['status'] != 'returned': continue
                meta = row['result']['metadata']
                if meta['id'] != selection['id']: continue
                left, right = meta['offset'], meta['offset'] + meta['bytes']
                returned = row['result']['text'].encode('utf-8')
                self.assertEqual(returned, whole[left:right])
                self.assertEqual(hashlib.sha256(returned).hexdigest(), meta['sha256'])
                low, high = max(left, selection['start']), min(right, selection['end'])
                if low < high: coverage[low-selection['start']:high-selection['start']] = b'\x01' * (high-low)
            self.assertTrue(all(coverage))
        self.assertEqual(uncertain, list(range(51, 63)))
        self.assertEqual([response['selections'][n]['id'] for n in uncertain],
                         ['input-' + str(n).zfill(6) for n in range(129, 141)])
        self.assertTrue(all(entries[response['selections'][n]['id']]['role'] == 'canonical_record' for n in uncertain))
        used = {n for claim in response['claims'] for n in claim['selections']}
        used |= {n for site in response['primary_sites'] for n in site['selections']}
        self.assertFalse(used.intersection(uncertain))
        self.assertIn('Keep relevant\nuncertain evidence in gaps', Path(record['frozen_instructions']['path']).read_text())
        calls = [c for c in record['calls'] if c['kind'] == 'model_call']
        self.assertEqual(len(calls), 1); self.assertEqual(calls[0]['process']['exit_code'], 0)
        i.check_binding(calls[0]['process']['stdout'])
        # Run the exact historical validator, not a new generation or repaired response.
        script = '''import json,sys
from pathlib import Path
from grounding import validate_output
import reader_contract
r=json.loads(Path(sys.argv[1]).read_bytes())
s=json.loads(Path(r['input']['path']).read_bytes())
o=json.loads(Path(r['original_outputs'][0]['path']).read_bytes())
t=[json.loads(l) for l in Path(r['retrievals']['path']).read_bytes().splitlines()]
# Observe the selection results before the historical authority error. Delegate
# unchanged arguments to the exact frozen reader, without changing any verdict.
original=reader_contract.validate_reading
def observe(response,spec,validation,**kwargs):
 print(json.dumps([{'id':v['selection']['id'],'issues':v['issues']} for v in validation['selections'] if v['reference_status']=='invalid']))
 return original(response,spec,validation,**kwargs)
reader_contract.validate_reading=observe
try: validate_output(o,s,r['lane'],t,contract=r['output_contract'])
except ValueError as error: print(str(error)); sys.exit(7)
sys.exit(0)
'''
        result = subprocess.run([sys.executable, '-B', '-c', script, str(path)],
            cwd=path.parent / 'producer-sources', capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 7, result.stdout + result.stderr)
        lines = result.stdout.splitlines()
        self.assertEqual(json.loads(lines[0]), [{'id': response['selections'][n]['id'],
                                               'issues': ['ambiguous chronology']} for n in uncertain])
        self.assertEqual(lines[1], 'wrong canonical authority field')
        self.assertEqual(json.loads((path.parent / 'prose/grounding.json').read_bytes())['error'], lines[1])
        from approaches import retrieval_audit
        host = [json.loads(line) for line in Path(calls[0]['process']['stdout']['path']).read_bytes().splitlines()]
        audit = retrieval_audit(spec_value, record['lane'], [e['item'] for e in host
            if e.get('type') == 'item.completed' and e.get('item', {}).get('type') == 'mcp_tool_call'], trace)
        self.assertEqual(audit['issues'], [])
        verified = {row['sequence'] for row in audit['outcomes'] if row['outcome'] == 'source_read_verified'}
        validation = validate_output(response, spec_value, record['lane'],
            [row for row in trace if row['sequence'] in verified], contract=DIRECTED_CONTRACT)
        self.assertEqual([n for n, row in enumerate(validation['selections']) if row['reference_status'] == 'invalid'], uncertain)
        self.assertTrue(all(validation['selections'][n]['issues'] == ['ambiguous chronology'] for n in uncertain))
        body, witness = card(path, 'Sample 01'); output = witness['outputs'][0]
        self.assertEqual(witness['original_invocation']['status'], 'invalid_response')
        self.assertEqual(output['derived_reading']['status'], 'invalid')
        self.assertEqual(output['derived_reading']['reading']['semantic_correctness'], 'pending_independent_examination')
        parser = baseline.Texts(); parser.feed(body)
        self.assertEqual(parser.values['prose'], [response['prose']])
        # HTML groups repeated file selections; original indexed order is retained
        # by the witness, while every exact request remains in the disclosures.
        self.assertEqual([s['selection'] for s in output['selections']], response['selections'])
        self.assertCountEqual([json.loads(s) for s in parser.values['selection-request']], response['selections'])
        self.assertNotIn('class="primary-sites"', body)
        self.assertEqual(body.count('Reference: invalid; ambiguous chronology'), 12)
        self.assertIn('unverified captured prose', body)
        self.assertTrue(all(output['selections'][n]['reference_status'].split(';')[0] == 'invalid' for n in uncertain))
        for n in uncertain:
            selection = response['selections'][n]
            entry = entries[selection['id']]
            text = Path(entry['asset']['path']).read_bytes()[selection['start']:selection['end']].decode()
            self.assertNotIn(text, parser.values['code'])
        with tempfile.TemporaryDirectory() as directory:
            display = render([path], Path(directory) / 'display')
            feedback = record_feedback(display, None, Path(directory) / 'feedback.json')
            self.assertEqual(json.loads(feedback.read_bytes())['original_outputs'], record['original_outputs'])
        self.assertEqual({p: hashlib.sha256(p.read_bytes()).hexdigest() for p in retained}, retained)


if __name__ == '__main__':
    unittest.main()
