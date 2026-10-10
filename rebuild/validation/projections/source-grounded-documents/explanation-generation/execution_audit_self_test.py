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


class PublicTimelineTests(unittest.TestCase):
    def test_real_multiline_public_results_locate_reads_without_promoting_a_final(self):
        from invocations import capture, environment
        import sys
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ledger = [
                {'sequence': 0, 'name': 'read', 'arguments': {'id': 'alpha'},
                 'status': 'returned', 'charged_bytes': 3},
                {'sequence': 1, 'name': 'read', 'arguments': {'id': 'beta'},
                 'status': 'returned', 'charged_bytes': 4},
            ]
            # A real producer emits two separate JSON lines, as code-mode text
            # does. Expected timestamps and sequences are independently authored.
            process = capture([sys.executable, '-B', '-c',
                'import json,sys; [print(json.dumps(row)) for row in json.load(sys.stdin)]'],
                cwd=root, env=environment(root), output=root / 'process',
                timeout=2, stream_bytes=8192, stdin=i.encoded(ledger))
            self.assertEqual(process['exit_code'], 0)
            self.assertTrue(process['streams_complete'])
            context = root / 'context.json'
            context.write_bytes(i.encoded([
                {'type': 'response_item', 'timestamp': '2026-01-01T00:00:02Z',
                 'payload': {'type': 'custom_tool_call', 'call_id': 'batch', 'name': 'exec'}},
                {'type': 'response_item', 'timestamp': '2026-01-01T00:00:03Z',
                 'payload': {'type': 'custom_tool_call_output', 'call_id': 'batch', 'output': [
                     {'type': 'input_text', 'text': Path(process['stdout']['path']).read_text()}]}},
                {'type': 'response_item', 'payload': {'type': 'reasoning',
                 'text': json.dumps(dict(ledger[0], sequence=99))}},
            ]))
            result = audit.stage_timeline(process, context, ledger)
            self.assertEqual(result['public_tool_batches'][0]['ledger_sequences'], [0, 1])
            self.assertEqual(result['evidence_batch_wall_seconds'], 1)
            self.assertEqual(result['first_nonempty_read_return_at'], '2026-01-01T00:00:03Z')
            self.assertEqual(result['last_nonempty_read_return_at'], '2026-01-01T00:00:03Z')
            self.assertEqual(result['unpaired_public_tool_calls'], [])
            self.assertEqual(result['publicly_located_ledger_sequences'], [0, 1])
            self.assertNotIn('99', json.dumps(result['public_tool_batches']))
            self.assertFalse((root / 'original-response.json').exists())

    def test_filtered_truncated_and_intermediate_text_cannot_certify_other_reads(self):
        rows = [dict(sequence=n, name='read', arguments={'id': str(n)}, status='returned') for n in (0, 1)]
        text = '\n'.join([json.dumps(rows[0]), 'Warning: truncated output',
                          json.dumps({'metadata': {'id': '1'}, 'text': 'source excerpt'}),
                          json.dumps({'prose': 'Public intermediate candidate'}),
                          json.dumps(rows[1])[:-3]])
        self.assertEqual(audit.sequences(text), [0])


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
                {'type': 'thread.started', 'thread_id': 'authored-review-session'},
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
        review_input = self.root / 'review-input.txt'; review_input.write_text('authored reviewer input')
        probe_root = self.root / 'probe'; probe_root.mkdir()
        self.probe = copy.deepcopy(self.process)
        for name, data in [('stdout', i.encoded([{'content': [{'text': review_input.read_text()}]}])),
                           ('stderr', b''), ('stdin', b'')]:
            path = probe_root / (name + '.bin'); path.write_bytes(data)
            self.probe[name] = i.binding(path)
        (probe_root / 'result.json').write_bytes(i.encoded(self.probe))
        generated = self.root / 'generated.json'; generated.write_bytes(i.encoded({'prose': 'Authored explanation'}))
        attempt_path = self.root / 'work-b-direct/attempt.json'
        attempt = json.loads(attempt_path.read_bytes())
        generator_root = self.root / 'generator/process'; generator_root.mkdir(parents=True)
        generator_stream = generator_root / 'stdout.bin'
        generator_stream.write_bytes((json.dumps({'type': 'thread.started',
                                                'thread_id': 'authored-generation-session'}) + '\n').encode())
        generator_process = copy.deepcopy(self.process)
        generator_process['stdout'] = i.binding(generator_stream)
        (generator_root / 'result.json').write_bytes(i.encoded(generator_process))
        (generator_root.parent / 'observed-context.json').write_bytes(i.encoded([
            {'type': 'session_meta', 'payload': {'id': 'authored-generation-session'}}]))
        attempt.update(scope=surface.spec['scope'], original_outputs=[i.binding(generated)],
                       calls=[{'kind': 'model_call', 'stage': 'prose',
                               'process': generator_process}])
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
                       'review_input': i.binding(review_input), 'original_review': i.binding(self.original),
                       'context_probe': self.probe, 'process': self.process,
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
                         'wrong_target', 'target_missing', 'changed_input'):
            with self.subTest(mutation=mutation):
                record = copy.deepcopy(self.record)
                if mutation == 'absent': record['original_review'] = None
                if mutation == 'foreign_work': record['scope']['work'] = 'foreign'
                if mutation == 'exposure': record['exposure_issues'] = ['observed_events_incomplete']
                if mutation == 'same_session': record['generator_sessions'] = record['review_sessions']
                if mutation == 'wrong_target': record['displayed_outputs'][0]['outputs'][0]['bytes'] += 1
                if mutation == 'target_missing': record['displayed_outputs'] = []
                if mutation == 'changed_input':
                    path = self.root / 'changed-input.txt'; path.write_text('different reviewer input')
                    record['review_input'] = i.binding(path)
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

    def test_non_scoped_host_observation_cannot_keep_completed_identity(self):
        # Refresh the exact stream/result hashes: identity alone cannot attest scope.
        stream = Path(self.process['stdout']['path'])
        event = {'type': 'item.completed', 'item': {'type': 'command_execution',
                 'command': 'read outside the permitted evidence surface', 'exit_code': 0}}
        stream.write_bytes(stream.read_bytes() + (json.dumps(event) + '\n').encode())
        self.process['stdout'] = i.binding(stream)
        (stream.parent / 'result.json').write_bytes(i.encoded(self.process))
        with self.assertRaisesRegex(ValueError, 'review context/scope unverified'):
            self.run_audit()

    def test_unfinished_tool_observation_cannot_keep_completed_identity(self):
        stream = Path(self.process['stdout']['path'])
        event = {'type': 'item.started', 'item': {'id': 'unfinished', 'type': 'mcp_tool_call',
                 'server': 'evidence', 'tool': 'read', 'status': 'in_progress'}}
        stream.write_bytes(stream.read_bytes() + (json.dumps(event) + '\n').encode())
        self.process['stdout'] = i.binding(stream)
        (stream.parent / 'result.json').write_bytes(i.encoded(self.process))
        with self.assertRaisesRegex(ValueError, 'review context/scope unverified'):
            self.run_audit()

    def test_every_tool_phase_is_scoped_even_with_terminal_identity_substitution(self):
        stream = Path(self.process['stdout']['path'])
        original = stream.read_bytes()
        for phase in ('item.started', 'item.updated', 'item.completed'):
            with self.subTest(phase=phase):
                # The ID deliberately matches the legitimate completed MCP read.
                # Refresh all receipts, so rejection must be based on scope.
                event = {'type': phase, 'item': {'id': self.host['id'],
                         'type': 'command_execution', 'command': 'outside scoped Reader'}}
                stream.write_bytes((json.dumps(event) + '\n').encode() + original)
                self.process['stdout'] = i.binding(stream)
                (stream.parent / 'result.json').write_bytes(i.encoded(self.process))
                with self.assertRaisesRegex(ValueError, 'review context/scope unverified'):
                    self.run_audit()
        stream.write_bytes(original)
        self.process['stdout'] = i.binding(stream)
        (stream.parent / 'result.json').write_bytes(i.encoded(self.process))
        self.assertTrue(self.run_audit()['attempts'][-1]['review_completion_verified'])

    def test_updated_scoped_call_without_completion_is_partial(self):
        stream = Path(self.process['stdout']['path'])
        event = {'type': 'item.updated', 'item': {'id': 'unfinished',
                 'type': 'mcp_tool_call', 'server': 'evidence', 'tool': 'read',
                 'status': 'in_progress'}}
        stream.write_bytes(stream.read_bytes() + (json.dumps(event) + '\n').encode())
        self.process['stdout'] = i.binding(stream)
        (stream.parent / 'result.json').write_bytes(i.encoded(self.process))
        with self.assertRaisesRegex(ValueError, 'review context/scope unverified'):
            self.run_audit()

    def test_generator_and_reviewer_sessions_require_actual_host_evidence(self):
        for mutation in ('missing', 'invented', 'same_session_hidden', 'foreign_review_context'):
            with self.subTest(mutation=mutation):
                record = copy.deepcopy(self.record)
                if mutation == 'missing': record['generator_sessions'] = []
                if mutation == 'invented': record['generator_sessions'] = ['invented-session']
                if mutation == 'same_session_hidden': record['review_sessions'] = ['authored-generation-session']
                if mutation in {'same_session_hidden', 'foreign_review_context'}:
                    context = self.root / (mutation + '-context.json')
                    context.write_bytes(i.encoded([{'type': 'session_meta', 'payload': {
                        'id': record['review_sessions'][0] if mutation == 'same_session_hidden'
                              else 'invented-review-session'}}]))
                    record['observed_context'] = i.binding(context)
                    if mutation == 'foreign_review_context': record['review_sessions'] = ['invented-review-session']
                with self.assertRaisesRegex(ValueError, 'review session separation unverified'):
                    self.run_audit(record)
        generator = self.root / 'generator/process/stdout.bin'
        generator.write_bytes((json.dumps({'type': 'thread.started', 'thread_id': 'authored-review-session'}) + '\n').encode())
        attempt_path = self.root / 'work-b-direct/attempt.json'
        attempt = json.loads(attempt_path.read_bytes())
        attempt['calls'][0]['process']['stdout'] = i.binding(generator)
        (generator.parent / 'result.json').write_bytes(i.encoded(attempt['calls'][0]['process']))
        attempt_path.write_bytes(i.encoded(attempt))
        integrity = json.loads(Path(self.record['integrity']['path']).read_bytes())
        integrity['samples'][0]['attempt'] = i.binding(attempt_path)
        Path(self.record['integrity']['path']).write_bytes(i.encoded(integrity))
        self.record['integrity'] = i.binding(self.record['integrity']['path'])
        self.record['generator_sessions'] = ['authored-review-session']
        with self.assertRaisesRegex(ValueError, 'review session separation unverified'):
            self.run_audit()

    def test_explicit_receipts_use_actual_identity_without_directory_name_inference(self):
        self.run_audit()
        directory = self.root / 'neutral-label'; directory.mkdir()
        receipt = directory / 'review-receipt.json'
        receipt.write_bytes(i.encoded(self.record))
        result = audit.audit_cohort(self.root, self.root, receipts=[receipt])
        self.assertTrue(result['attempts'][0]['review_completion_verified'])
        self.assertEqual(result['attempts'][0]['receipt'], i.binding(receipt))
        receipt.unlink()
        with self.assertRaises(FileNotFoundError):
            audit.audit_cohort(self.root, self.root, receipts=[receipt])

    def test_failed_partial_or_foreign_probe_cannot_keep_completed_identity(self):
        for mutation in ('failed', 'incomplete_streams', 'incomplete_cleanup', 'foreign_prompt', 'prohibited_context'):
            with self.subTest(mutation=mutation):
                record = copy.deepcopy(self.record)
                probe = record['context_probe']
                if mutation == 'failed': probe.update(outcome='failed', returncode=1)
                if mutation == 'incomplete_streams': probe['streams_complete'] = False
                if mutation == 'incomplete_cleanup': probe['cleanup']['complete'] = False
                if mutation in {'foreign_prompt', 'prohibited_context'}:
                    content = [{'text': 'another prompt'}] if mutation == 'foreign_prompt' else [
                        {'text': 'authored reviewer input'}, {'text': 'prototype-data'}]
                    path = self.root / (mutation + '.json')
                    path.write_bytes(i.encoded([{'content': content}]))
                    probe['stdout'] = i.binding(path)
                    # Each process keeps its own matching result receipt.
                    for key in ('stdin', 'stderr'):
                        target = self.root / (mutation + '-' + key)
                        target.write_bytes(Path(probe[key]['path']).read_bytes())
                        probe[key] = i.binding(target)
                    probe_root = self.root / mutation; probe_root.mkdir()
                    destination = probe_root / 'stdout.bin'; destination.write_bytes(path.read_bytes())
                    probe['stdout'] = i.binding(destination)
                    (probe_root / 'result.json').write_bytes(i.encoded(probe))
                else:
                    (Path(probe['stdout']['path']).parent / 'result.json').write_bytes(i.encoded(probe))
                with self.assertRaises(ValueError): self.run_audit(record)
        (Path(self.probe['stdout']['path']).parent / 'result.json').write_bytes(i.encoded(self.probe))
        self.assertTrue(self.run_audit()['attempts'][-1]['review_completion_verified'])

    def test_incomplete_host_process_cannot_keep_completed_identity(self):
        for mutation in ('failed', 'nonzero', 'incomplete_streams', 'incomplete_cleanup'):
            with self.subTest(mutation=mutation):
                record = copy.deepcopy(self.record)
                process = record['process']
                if mutation == 'failed': process['outcome'] = 'failed'
                if mutation == 'nonzero': process['returncode'] = 1
                if mutation == 'incomplete_streams': process['streams_complete'] = False
                if mutation == 'incomplete_cleanup': process['cleanup']['complete'] = False
                receipt = Path(process['stdout']['path']).parent / 'result.json'
                receipt.write_bytes(i.encoded(process))
                with self.assertRaisesRegex(ValueError, 'incomplete review claimed completion'):
                    self.run_audit(record)
        receipt.write_bytes(i.encoded(self.process))
        self.assertTrue(self.run_audit()['attempts'][-1]['review_completion_verified'])

    def test_missing_review_receipt_never_returns_completed_assessment(self):
        self.run_audit()
        (self.root / 'click-independent-review/review-receipt.json').unlink()
        with self.assertRaises(FileNotFoundError): audit.audit_cohort(self.root, self.root)

    def test_forged_process_receipt_cannot_certify_complete_capture(self):
        receipt = Path(self.process['stdout']['path']).parent / 'result.json'
        value = copy.deepcopy(self.process); value['duration_seconds'] = 99
        receipt.write_bytes(i.encoded(value))
        with self.assertRaisesRegex(ValueError, 'process receipt mismatch'):
            self.run_audit()

    def test_absent_generation_host_identity_cannot_certify_separation(self):
        attempt_path = self.root / 'work-b-direct/attempt.json'
        attempt = json.loads(attempt_path.read_bytes())
        attempt['calls'] = []
        attempt_path.write_bytes(i.encoded(attempt))
        integrity_path = Path(self.record['integrity']['path'])
        integrity = json.loads(integrity_path.read_bytes())
        integrity['samples'][0]['attempt'] = i.binding(attempt_path)
        integrity_path.write_bytes(i.encoded(integrity))
        self.record['integrity'] = i.binding(integrity_path)
        with self.assertRaisesRegex(ValueError, 'generation session evidence absent'):
            self.run_audit()


if __name__ == '__main__':
    unittest.main()
