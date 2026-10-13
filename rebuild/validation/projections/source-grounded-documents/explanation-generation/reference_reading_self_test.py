"""Real stdio Reader to scoped handles, original sidecars, reading and feedback."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

import inputs as i
from grounding import validate_output
from input_self_test import spec
from invocations import capture, environment
from render_comparison import render, record_feedback
from verified_references import VerifiedReferences

HERE = Path(__file__).parent


class ReferenceReadingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.body = b'def value():\n    return 7\n'
        source = self.root / 'source'; source.write_bytes(self.body)
        value = spec(source)
        template = value['entries'][0]
        for identity, changes in [
            ('product-source', {'lane': 'product', 'attribution': 'explicit_work_patch'}),
            ('uncertain-source', {'chronology': {'state': 'ambiguous', 'reason': 'Unobserved time.'}}),
            ('missing-before', {'representation': 'unavailable', 'origin': None, 'file_sha256': None,
                                'missing': 'withheld or not retained', 'attribution': 'explicit_before_patch'}),
            ('task', {'role': 'task'})]:
            entry = copy.deepcopy(template); entry.update(id=identity, **changes)
            value['entries'].append(entry)
        path = self.root / 'spec.json'; path.write_bytes(i.encoded(value))
        self.manifest = i.freeze(path, self.root / 'frozen', self.root)
        configuration = self.root / 'reader.json'
        trace = self.root / 'retrievals.jsonl'
        configuration.write_bytes(i.encoded({'manifest': str(self.manifest), 'lane': 'archive_diagnostic',
            'budgets': {'reads': 20, 'read_bytes': 8192}, 'trace': str(trace)}))
        requests = [{'jsonrpc': '2.0', 'id': n, 'method': 'tools/call',
                     'params': {'name': 'read', 'arguments': {'id': identity, 'limit': 64}}}
                    for n, identity in enumerate(['source-0001', 'product-source', 'uncertain-source',
                                                  'missing-before', 'task', 'foreign-id'])]
        workspace = self.root / 'stdio-cwd'; workspace.mkdir()
        process = capture([sys.executable, '-B', str(HERE / 'source_tools.py'), str(configuration)],
            cwd=workspace, env=environment(workspace), output=self.root / 'stdio-process', timeout=5,
            stream_bytes=65536, stdin=b'\n'.join(json.dumps(r).encode() for r in requests) + b'\n')
        self.assertEqual(process['exit_code'], 0)
        self.assertTrue(process['streams_complete'])
        replies = [json.loads(line)['result'] for line in Path(process['stdout']['path']).read_bytes().splitlines()]
        self.rows = [json.loads(reply['content'][0]['text']) for reply in replies]
        self.assertEqual(self.rows[0]['result']['text'].encode(), self.body)
        self.assertEqual(self.rows[0]['charged_bytes'], 26)
        self.assertEqual([r['outcome'] for r in self.rows],
            ['source_returned', 'source_returned', 'source_returned', 'source_unavailable', 'source_returned', 'policy_denied'])
        # Authored Host envelopes carry the actual stdio replies, not invented
        # Reader results. This is local transport evidence, never a model run.
        host = self.root / 'host.jsonl'
        self.calls = [{'id': str(n), 'type': 'mcp_tool_call', 'server': 'evidence', 'tool': 'read',
                       'arguments': requests[n]['params']['arguments'],
                       'status': 'failed' if reply['isError'] else 'completed', 'result': reply}
                      for n, reply in enumerate(replies)]
        host.write_bytes(b''.join((json.dumps({'type': 'item.completed', 'item': call}) + '\n').encode()
                                  for call in self.calls))
        self.record = {'input': i.binding(self.manifest), 'scope': value['scope'], 'lane': 'archive_diagnostic',
            'retrievals': i.binding(trace), 'calls': [{'kind': 'model_call', 'process': {'stdout': i.binding(host), 'exit_code': 0}}],
            'budgets': {'reads': 20, 'read_bytes': 8192}, 'output_contract': 'work_directed_reader'}
        self.observation = self.root / 'observation.json'; self.observation.write_bytes(i.encoded(self.record))
        self.references = VerifiedReferences(self.observation)

    def sidecar(self, prose='The function returns seven.'):
        handle = self.references.issue('source-0001', 17, 25)
        return self.references.sidecar(prose, [handle], [],
            claims=[{'start': 0, 'end': len(prose.encode()), 'kind': 'source_fact', 'selections': [0], 'authority': None}],
            primary_sites=[{'selections': [0], 'claims': [0], 'reason': 'Caller-selected return expression.',
                            'extent_reason': 'Caller-selected eight exact bytes.'}])

    def test_actual_reader_to_sidecar_reduces_metadata_error_and_reaches_consumers(self):
        response, validation = self.sidecar()
        self.assertEqual(response['selections'], [{'id': 'source-0001', 'start': 17, 'end': 25,
            'sha256': hashlib.sha256(b'return 7').hexdigest(), 'state': 'context'}])
        self.assertEqual(validation['reading']['status'], 'valid_binding')
        # The demonstrated original failure surface: interpreting historical
        # chronology as before-state, or copying a whole-read hash for a subrange.
        manual = copy.deepcopy(response)
        manual['selections'][0].update(state='before', sha256=self.rows[0]['result']['metadata']['sha256'])
        failed = validate_output(manual, i.verify(self.manifest), 'archive_diagnostic', self.rows,
                                 contract='work_directed_reader')
        self.assertEqual(failed['selections'][0]['issues'], ['wrong-state span', 'span hash mismatch'])
        self.assertEqual(failed['reading']['status'], 'invalid')
        output = self.root / 'new-response.json'; output.write_bytes(i.encoded(response))
        record = dict(self.record, original_outputs=[i.binding(output)], generation_output=i.binding(output),
                      status='captured', approach='direct', blockers=[], clean_comparison=False)
        attempt = self.root / 'new-attempt.json'; attempt.write_bytes(i.encoded(record))
        display = render([attempt], self.root / 'display')
        integrity = json.loads((display.parent / 'integrity.json').read_bytes())
        self.assertEqual(integrity['samples'][0]['outputs'][0]['derived_reading']['status'], 'valid_binding')
        self.assertIn('return 7', display.read_text())
        feedback = record_feedback(display, 'Authored transport observation.', self.root / 'feedback.json', reviewer_kind='agent')
        self.assertEqual(json.loads(feedback.read_bytes())['original_outputs'], [i.binding(output)])
        self.assertEqual(json.loads(feedback.read_bytes())['H1'], 'pending')
        self.assertEqual(self.references.resolve(self.references.issue('source-0001', 17, 25)), response['selections'][0])

    def test_model_selected_identity_and_range_are_preserved_even_with_identical_bytes(self):
        first = self.references.issue('source-0001', 17, 25)
        second = self.references.issue('product-source', 17, 25)
        self.assertNotEqual(first, second)
        self.assertEqual(self.references.resolve(first)['state'], 'context')
        selected = self.references.resolve(second)
        self.assertEqual(selected['id'], 'product-source')
        self.assertEqual(selected['state'], 'after')
        self.assertEqual((selected['start'], selected['end']), (17, 25))

    def test_ambiguous_unavailable_denied_and_unread_ranges_cannot_issue_handles(self):
        for identity, start, end, error in [
            ('uncertain-source', 17, 25, 'ambiguous chronology'),
            ('missing-before', 0, 1, 'unavailable source'),
            ('foreign-id', 0, 1, 'outside permitted inventory'),
            ('source-0001', True, 25, 'invalid byte span'),
            ('source-0001', 17, 27, 'invalid byte span')]:
            with self.subTest(identity=identity), self.assertRaisesRegex(ValueError, error):
                self.references.issue(identity, start, end)
        # Bytes present locally but absent from actual returned reads are not usable.
        record = copy.deepcopy(self.record)
        # Preserve real Reader outcomes, while excluding that complete observation.
        record['calls'] = []
        path = self.root / 'unobserved.json'; path.write_bytes(i.encoded(record))
        with self.assertRaisesRegex(ValueError, 'unverified reference reads'):
            VerifiedReferences(path)

    def test_partial_reads_lane_policy_and_deleted_source_never_gain_missing_bytes(self):
        from source_tools import Surface
        trace = self.root / 'partial-ledger'
        surface = Surface(self.manifest, 'product', {'reads': 4, 'read_bytes': 8192}, trace)
        row = surface.call('read', {'id': 'product-source', 'offset': 17, 'limit': 4})
        self.assertEqual(row['result']['text'], 'retu')
        host = self.root / 'partial-host'
        host.write_text(json.dumps({'type': 'item.completed', 'item': {
            'id': 'partial', 'type': 'mcp_tool_call', 'server': 'evidence', 'tool': 'read',
            'arguments': row['arguments'], 'status': 'completed',
            'result': {'content': [{'type': 'text', 'text': json.dumps(row)}]}}}) + '\n')
        record = copy.deepcopy(self.record)
        record.update(lane='product', retrievals=i.binding(trace),
                      calls=[{'kind': 'model_call', 'process': {'stdout': i.binding(host)}}])
        observation = self.root / 'partial-observation'; observation.write_bytes(i.encoded(record))
        references = VerifiedReferences(observation)
        handle = references.issue('product-source', 17, 21)
        self.assertEqual(references.resolve(handle)['sha256'], hashlib.sha256(b'retu').hexdigest())
        with self.assertRaisesRegex(ValueError, 'span not retrieved'):
            references.issue('product-source', 17, 25)
        with self.assertRaisesRegex(ValueError, 'outside permitted inventory'):
            references.issue('source-0001', 17, 21)
        asset = Path(i.verify(self.manifest)['entries'][1]['asset']['path'])
        asset.unlink()
        with self.assertRaises((ValueError, OSError)):
            references.resolve(handle)

    def test_changed_project_work_cutoff_revision_and_snapshot_bindings_reject_handles(self):
        for field in ('project', 'work', 'cutoff', 'revision', 'snapshot'):
            with self.subTest(field=field):
                handle = self.references.issue('source-0001', 17, 25)
                original = self.observation.read_bytes()
                record = copy.deepcopy(self.record)
                if field in {'project', 'work', 'cutoff'}:
                    record['scope'][field] = 'changed'
                elif field == 'revision':
                    record['input']['sha256'] = '0' * 64
                else:
                    record['retrievals']['sha256'] = '0' * 64
                self.observation.write_bytes(i.encoded(record))
                with self.assertRaisesRegex(ValueError, 'changed bytes'):
                    self.references.resolve(handle)
                self.observation.write_bytes(original)

    def test_unknown_foreign_and_stale_handles_reject_without_source_substitution(self):
        handle = self.references.issue('source-0001', 17, 25)
        other = VerifiedReferences(self.observation)
        with self.assertRaisesRegex(ValueError, 'unknown or foreign'):
            other.resolve(handle)
        with self.assertRaisesRegex(ValueError, 'unknown or foreign'):
            self.references.resolve('reference-' + '0' * 64)
        asset = Path(i.verify(self.manifest)['entries'][0]['asset']['path'])
        asset.write_bytes(b'def value():\n    return 8\n')
        with self.assertRaisesRegex(ValueError, 'changed bytes'):
            self.references.resolve(handle)

    def test_refreshed_forged_host_and_ledger_cannot_supply_metadata_authority(self):
        record = copy.deepcopy(self.record)
        rows = copy.deepcopy(self.rows)
        rows[0]['result']['metadata']['role'] = 'trusted_source'
        trace = Path(record['retrievals']['path'])
        trace.write_bytes(b''.join((json.dumps(row) + '\n').encode() for row in rows))
        record['retrievals'] = i.binding(trace)
        calls = copy.deepcopy(self.calls); calls[0]['result']['content'][0]['text'] = json.dumps(rows[0])
        host = Path(record['calls'][0]['process']['stdout']['path'])
        host.write_bytes(b''.join((json.dumps({'type': 'item.completed', 'item': c}) + '\n').encode() for c in calls))
        record['calls'][0]['process']['stdout'] = i.binding(host)
        path = self.root / 'forged-observation.json'; path.write_bytes(i.encoded(record))
        with self.assertRaisesRegex(ValueError, 'unverified reference reads'):
            VerifiedReferences(path)

    def test_handle_supplies_no_semantic_truth_or_unsupported_user_authority(self):
        false_prose = 'The function returns secret credentials and the user approved publication.'
        response, validation = self.sidecar(false_prose)
        self.assertEqual(response['prose'], false_prose)
        self.assertEqual(validation['semantic_quality'], 'not_assessed')
        self.assertEqual(validation['unsupported_inferences'], 'not_verified')
        handle = self.references.issue('task', 0, 26)
        with self.assertRaisesRegex(ValueError, 'wrong authority evidence role'):
            self.references.sidecar('The user chose publication.', [handle], [],
                claims=[{'start': 0, 'end': 27, 'kind': 'user_choice', 'selections': [0],
                         'authority': {'selection': 0, 'record_id': None, 'revision': None, 'field': None}}],
                primary_sites=[])


if __name__ == '__main__':
    unittest.main()
