"""Authored review receipts through the retained-evidence audit; no provider."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import approaches as a
import execution_audit as audit
import inputs as i
from input_self_test import spec
from source_tools import Surface


class ReviewAuditTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        source = self.root / 'source'; source.write_text('authored evidence\n')
        path = self.root / 'spec.json'; path.write_bytes(i.encoded(spec(source)))
        self.manifest = i.freeze(path, self.root / 'frozen', self.root)
        (self.root / 'plan.json').write_bytes(i.encoded({'producer_head': 'authored-fixture'}))
        empty = self.root / 'empty.jsonl'; empty.write_bytes(b'')
        for name in ('work-b-direct', 'work-b-note_then_prose', 'click-direct', 'click-note_then_prose'):
            directory = self.root / name; directory.mkdir()
            record = {'input': i.binding(self.manifest), 'retrievals': i.binding(empty),
                      'calls': [], 'evidence_reads': a.retrieval_audit(spec(source), 'archive_diagnostic', [], []),
                      'status': 'blocked', 'tokens': [], 'price': None, 'workspace_cleanup': 'complete'}
            (directory / 'attempt.json').write_bytes(i.encoded(record))
        surface = Surface(self.manifest, 'archive_diagnostic', {'reads': 4, 'read_bytes': 1024}, self.root / 'ledger')
        row = surface.call('read', {'id': 'source-0001', 'limit': 64})
        self.host = {'id': 'authored-call', 'type': 'mcp_tool_call', 'server': 'evidence', 'tool': 'read',
                     'arguments': row['arguments'], 'status': 'completed',
                     'result': {'content': [{'type': 'text', 'text': json.dumps(row)}]}}
        self.response = {'prose': 'Authored review fixture, not a semantic judgment.', 'gaps': [],
                         'selections': [{'id': 'source-0001', 'start': 0, 'end': row['charged_bytes'],
                                         'state': 'context', 'sha256': row['result']['metadata']['sha256']}]}
        self.original = self.root / 'original-review.json'; self.original.write_bytes(i.encoded(self.response))
        stage = self.root / 'stage'; stage.mkdir()
        process_root = stage / 'process'; process_root.mkdir()
        for name, data in [('stdout', b''.join((json.dumps(e) + '\n').encode() for e in [
                {'type': 'item.completed', 'item': self.host},
                {'type': 'item.completed', 'item': {'type': 'agent_message', 'text': self.original.read_text()}}])),
                           ('stderr', b''), ('stdin', b'authored reviewer input')]:
            (process_root / (name + '.bin')).write_bytes(data)
        self.process = {'outcome': 'succeeded', 'returncode': 0, 'streams_complete': True,
                        'cleanup': {'complete': True}, 'started_at_unix_ns': 0,
                        'ended_at_unix_ns': 1_000_000_000, 'duration_seconds': 1,
                        **{k: i.binding(process_root / (k + '.bin')) for k in ('stdin', 'stdout', 'stderr')}}
        (process_root / 'result.json').write_bytes(i.encoded(self.process))
        self.context = stage / 'observed-context.json'
        self.context.write_bytes(i.encoded([{'type': 'session_meta', 'payload': {'id': 'authored-review-session'}}]))
        generated = self.root / 'generated.json'; generated.write_bytes(i.encoded({'prose': 'Authored explanation'}))
        attempt_path = self.root / 'work-b-direct/attempt.json'
        attempt = json.loads(attempt_path.read_bytes())
        attempt.update(scope=surface.spec['scope'], original_outputs=[i.binding(generated)])
        attempt_path.write_bytes(i.encoded(attempt))
        presentation = self.root / 'comparison.html'; presentation.write_text('<p>Authored display fixture</p>')
        integrity = self.root / 'integrity.json'
        integrity.write_bytes(i.encoded({'presentation': i.binding(presentation), 'samples': [
            {'label': 'Sample 01', 'attempt': i.binding(attempt_path), 'outputs': [
                {'original': i.binding(generated), 'prose_sha256': i.digest(b'Authored explanation')}]}]}))
        self.record = {'input': i.binding(self.manifest), 'scope': surface.spec['scope'],
                       'presentation': i.binding(presentation), 'integrity': i.binding(integrity),
                       'displayed_outputs': [{'label': 'Sample 01', 'outputs': [i.binding(generated)]}],
                       'retrievals': i.binding(surface.trace), 'observed_context': i.binding(self.context),
                       'review_input': self.process['stdin'], 'original_review': i.binding(self.original),
                       'context_probe': self.process, 'process': self.process,
                       'evidence_reads': a.retrieval_audit(surface.spec, 'archive_diagnostic', [self.host], [row]),
                       'status': 'review_captured', 'exposure_issues': [], 'distinct_generation_session': True,
                       'generator_sessions': ['authored-generation-session'], 'review_sessions': ['authored-review-session'],
                       'tokens': [], 'price': None, 'workspace_cleanup': 'complete'}

    def run_audit(self, record=None):
        for name in ('work-b-independent-review', 'click-independent-review'):
            directory = self.root / name; directory.mkdir(exist_ok=True)
            (directory / 'review-receipt.json').write_bytes(i.encoded(record or self.record))
        return audit.audit_cohort(self.root, self.root)

    def test_genuine_authored_review_remains_auditable(self):
        reports = self.run_audit()['attempts']
        self.assertEqual([r['status'] for r in reports[-2:]], ['review_captured'] * 2)

    def test_changed_original_review_cannot_keep_completed_identity(self):
        self.original.write_text('tampered reviewer output')
        with self.assertRaises(ValueError):
            self.run_audit()

    def test_incomplete_review_cannot_claim_completion(self):
        for mutation in ('absent', 'foreign_work', 'exposure', 'same_session', 'invalid_selection', 'unjoined_final',
                         'wrong_target', 'target_missing'):
            with self.subTest(mutation=mutation):
                record = copy.deepcopy(self.record)
                if mutation == 'absent': record['original_review'] = None
                if mutation == 'foreign_work': record['scope']['work'] = 'foreign'
                if mutation == 'exposure': record['exposure_issues'] = ['observed_events_incomplete']
                if mutation == 'same_session': record['generator_sessions'] = record['review_sessions']
                if mutation == 'wrong_target': record['displayed_outputs'][0]['outputs'][0]['bytes'] += 1
                if mutation == 'target_missing': record['displayed_outputs'] = []
                if mutation in {'invalid_selection', 'unjoined_final'}:
                    response = copy.deepcopy(self.response)
                    if mutation == 'invalid_selection': response['selections'][0]['sha256'] = '0' * 64
                    else: response['prose'] = 'Different review, with otherwise valid references.'
                    path = self.root / (mutation + '.json'); path.write_bytes(i.encoded(response))
                    record['original_review'] = i.binding(path)
                with self.assertRaises(ValueError):
                    self.run_audit(record)

    def test_partial_review_is_preserved_without_a_completed_judgment(self):
        record = copy.deepcopy(self.record)
        record.update(status='review_incomplete_or_unverified', original_review=None,
                      exposure_issues=['observed_events_incomplete'])
        reports = self.run_audit(record)['attempts']
        self.assertEqual(reports[-1]['status'], 'review_incomplete_or_unverified')


if __name__ == '__main__':
    unittest.main()
