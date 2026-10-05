"""Actual Naturalistic consumer controls for identity and shared answer substitution."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import harness as h
import machine_findings as m


class AnswerTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.descriptor = h.real_session_fixture('volicord', 1, '0' * 40, self.root)
        self.reference = self.descriptor['evidence']['captures']['resume']
        self.path = self.root / self.reference['file']
        self.events = [json.loads(line) for line in self.path.read_text().splitlines()]
        self.output = next(event['payload'] for event in self.events if event['payload'].get('type') == 'custom_tool_call_output'
            and 'recall-call' in event['payload'].get('call_id', ''))
        self.answer = json.loads(self.output['output'][1]['text'])

    def evaluate(self, answer=None, cli=None, cli_before=False):
        if answer is not None:
            self.output['output'][1]['text'] = json.dumps(answer)
            for event in self.events:
                payload = event['payload']
                if payload.get('type') == 'mcp_tool_call_end' and payload.get('call_id') == 'exec-' + self.output['call_id']:
                    payload['result']['Ok']['structuredContent'] = copy.deepcopy(answer)
        if cli is not None:
            turn = next(event['payload']['turn_id'] for event in self.events
                if event['payload'].get('type') == 'task_started')
            metadata = {'turn_id': turn}
            additions = [
                {'type': 'response_item', 'payload': {'type': 'custom_tool_call', 'call_id': 'cli-recall',
                    'name': 'exec', 'status': 'completed', 'id': 'ctc-cli-recall', 'input': 'const r=await tools.exec_command({"cmd":"volicord recall --json","workdir":"/phase8/repository"}); text(JSON.stringify(r));',
                    'internal_chat_message_metadata_passthrough': metadata}},
                {'type': 'response_item', 'payload': {'type': 'custom_tool_call_output', 'call_id': 'cli-recall',
                    'output': [{'type': 'input_text', 'text': 'Script completed\nWall time 0.1 seconds\nOutput:\n' + json.dumps({'output': json.dumps(cli), 'exit_code': 0})}],
                    'internal_chat_message_metadata_passthrough': metadata}}]
            # Place beside the measured Recall, before subsequent canonical writes.
            index = self.events.index(next(e for e in self.events if e['payload'] is self.output)) + 1
            if cli_before:
                index = next(i for i,e in enumerate(self.events) if e['payload'].get('type') == 'custom_tool_call'
                    and '__canonical_mutate(' in e['payload'].get('input', ''))
            for event in additions:
                event['timestamp'] = self.events[index]['timestamp']
            self.events[index:index] = additions
        self.path.write_text(''.join(json.dumps(event) + '\n' for event in self.events))
        self.reference['sha256'] = h.sha256(self.path)
        result = h.real_session_evidence(self.descriptor, kind='volicord', cycle=1, repository_revision='0' * 40)
        return result, result['machine_facts']['shared_answer_integrity']

    def test_valid_unavailable_explanation_keeps_recorded_action(self):
        result, fact = self.evaluate()
        self.assertEqual(fact['status'], 'confirmed_pass', fact)
        self.assertEqual(result['checks']['recall_matches_checkpoint_decision_and_context'], 'passed')

    def test_unavailable_explanation_still_checks_goal_source_provenance(self):
        for sources in (['ff' * 16], ['malformed'], None):
            wrong = copy.deepcopy(self.answer)
            wrong['goal_basis'][0]['source_ids'] = sources
            _, fact = self.evaluate(wrong)
            self.assertEqual(fact['status'], 'confirmed_violation', fact)

    def test_cli_mcp_and_both_wrong_even_with_correct_legacy_fields(self):
        for transport in ('cli', 'mcp', 'both'):
            for mutation in ('identity', 'direction', 'basis'):
                with self.subTest(transport=transport, mutation=mutation):
                    self.setUp()
                    wrong = copy.deepcopy(self.answer)
                    if mutation == 'identity':
                        wrong['selected_work']['work_item_id'] = 'ff' * 16
                    elif mutation == 'direction':
                        wrong['next_step'] = 'Repair explanations instead of continuing the task'
                    else:
                        wrong['selected_work']['answers']['facts'][0]['recorded_action']['revision'] = 2
                    result, fact = self.evaluate(wrong if transport in {'mcp', 'both'} else self.answer,
                        wrong if transport in {'cli', 'both'} else None)
                    self.assertEqual(fact['status'], 'confirmed_violation', fact)
                    self.assertEqual(m.disposition('shared_answer_integrity', fact['status']), 'hard_blocking')
                    self.assertEqual(result['checks']['recall_matches_checkpoint_decision_and_context'], 'failed')

    def test_missing_historical_basis_is_indeterminate_but_wrong_identity_still_fails(self):
        bundle_ref = self.descriptor['evidence']['canonical_bundle']
        path = self.root / bundle_ref['file']
        bundle = json.loads(path.read_text())
        for table in bundle['payload']['tables']:
            if table['name'] == 'checkpoints':
                table['rows'] = []
        self.write_bundle(path, bundle)
        bundle_ref['sha256'] = h.sha256(path)
        _, fact = self.evaluate()
        self.assertEqual(fact['status'], 'indeterminate', fact)
        wrong = copy.deepcopy(self.answer)
        wrong['selected_work']['work_item_id'] = 'ff' * 16
        _, fact = self.evaluate(wrong)
        self.assertEqual(fact['status'], 'confirmed_violation')

    def write_bundle(self, path, bundle):
        payload = bundle['payload']
        state = {'project_id': payload['project_id'], 'tables': payload['tables']}
        payload['lineage']['history_basis'] = h.hashlib.sha256(h.json.dumps(state, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        bundle['checksum'] = h.hashlib.sha256(h.json.dumps(payload, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        path.write_text(json.dumps(bundle))

    def test_equal_title_works_keep_canonical_identity(self):
        reference = self.descriptor['evidence']['canonical_bundle']
        path = self.root / reference['file']
        bundle = json.loads(path.read_text())
        for table in bundle['payload']['tables']:
            if table['name'] == 'context_items':
                row = copy.deepcopy(table['rows'][0])
                row[0]['value'] = 'ff' * 16
                row[-1]['value'] = 0
                table['rows'].append(row)
            if table['name'] == 'context_item_sources':
                row = copy.deepcopy(table['rows'][0])
                row[1]['value'] = 'ff' * 16
                table['rows'].append(row)
        self.write_bundle(path, bundle)
        reference['sha256'] = h.sha256(path)
        result, fact = self.evaluate()
        self.assertEqual(fact['status'], 'confirmed_pass')
        self.assertEqual(result['checks']['recall_matches_checkpoint_decision_and_context'], 'passed', result['task_goal_basis'])

    def test_generic_omission_cannot_excuse_wrong_action(self):
        wrong = copy.deepcopy(self.answer)
        wrong['next_step'] = 'Wrong task'
        wrong['transport_omission'] = {'reason': 'serialized_byte_budget', 'exact_json_bytes': 90000}
        _, fact = self.evaluate(wrong)
        self.assertEqual(fact['status'], 'confirmed_violation')

    def generated_answer(self):
        correct = copy.deepcopy(self.answer)
        work = correct['selected_work']; action = work['answers']['facts'][0]['recorded_action']
        work['answers'].update(explanation_state='current', provenance={
            'project_id': correct['project_id'], 'subject': {'kind': 'work', 'identity': list(bytes.fromhex(work['work_item_id']))},
            'language': 'en', 'generator_identity_status': 'self_reported_not_independently_verified',
            'evidence': [{'key': 'next_step', 'identity': action['checkpoint_id'], 'revision': 1,
                'field': 'next_step', 'sources': action['source_ids']},
                {'key': 'goal', 'identity': work['work_item_id'], 'revision': 1, 'field': 'statement', 'sources': action['source_ids']}]},
            prose=[{'question': 'NextStep', 'role': 'generated_interpretation',
                'text': 'Continue the task using the recorded action.', 'evidence_keys': ['next_step']}])
        return correct

    def insert_correction(self, *, after=False, identity=None, expected=1, revision=2, failed=False, call_id='correction'):
        template = next(e for e in self.events if e['payload'].get('type') == 'mcp_tool_call_end'
            and e['payload'].get('invocation', {}).get('tool') == 'recall')
        event = copy.deepcopy(template)
        work = self.answer['selected_work']['work_item_id']
        args = {'project_id': self.answer['project_id'], 'action': 'correct_context', 'record_id': identity or work,
            'expected_revision': expected, 'corrected_text': self.answer['goal_basis'][0]['statement'] + '.',
            'user_turn': 'Correct punctuation only.'}
        receipt = {'action': 'correct_context', 'record_kind': 'context_item', 'identity': identity or work,
            'revision': revision, 'user_response_source_id': 'fa' * 16}
        event['payload'].update(call_id=call_id, invocation={'server': 'volicord', 'tool': 'canonical_mutate', 'arguments': args},
            result={'Err': 'revision conflict'} if failed else {'Ok': {'content': [{'type': 'text', 'text': json.dumps(receipt)}],
                'structuredContent': receipt, 'isError': False}})
        # The receipt and invocation are actual parser inputs, not resolver mocks.
        if after:
            index = len(self.events) - 1
        else:
            wrapper_id = template['payload']['call_id'].removeprefix('exec-')
            index = next(i for i,e in enumerate(self.events) if e['payload'].get('type') == 'custom_tool_call'
                and e['payload'].get('call_id') == wrapper_id)
        event['timestamp'] = self.events[index]['timestamp']
        turn = next(e['payload']['turn_id'] for e in self.events if e['payload'].get('type') == 'task_started')
        wrapper = {'timestamp': event['timestamp'], 'type': 'response_item', 'payload': {
            'type': 'custom_tool_call', 'name': 'exec', 'call_id': call_id + '-wrapper', 'status': 'completed',
            'input': 'const r=await tools.mcp__volicord__canonical_mutate(' + json.dumps(args) + '); text(JSON.stringify(r));',
            'internal_chat_message_metadata_passthrough': {'turn_id': turn}}}
        self.events[index:index] = [wrapper, event]
        return event

    def test_corrected_goal_revision_is_resolved_before_recall(self):
        self.insert_correction()
        correct = self.generated_answer()
        correct['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 2
        _, fact = self.evaluate(correct)
        self.assertEqual(h.load_codex_capture(self.path).calls('canonical_mutate')[0].outcome, 'succeeded')
        self.assertEqual(fact['status'], 'confirmed_pass', fact)

    def test_old_goal_revision_cannot_be_claimed_current_after_correction(self):
        self.insert_correction()
        _, fact = self.evaluate(self.generated_answer())
        self.assertEqual(fact['status'], 'confirmed_violation', fact)
        self.assertEqual(m.disposition('shared_answer_integrity', fact['status']), 'hard_blocking')

    def test_conflicting_creation_receipts_do_not_choose_a_revision_or_source(self):
        for conflict in ('revision', 'source'):
            with self.subTest(conflict=conflict):
                self.setUp()
                event = self.insert_correction()
                args = {'project_id': self.answer['project_id'], 'role': 'goal', 'work_transition': 'start'}
                receipt = {'project_id': self.answer['project_id'], 'role': 'goal',
                    'context_item_id': self.answer['selected_work']['work_item_id'],
                    'revision': 2 if conflict == 'revision' else 1,
                    'source_id': ('fa' if conflict == 'source' else '03') * 16}
                event['payload']['invocation'].update(tool='context_record', arguments=args)
                event['payload']['result']['Ok'].update(structuredContent=receipt,
                    content=[{'type': 'text', 'text': json.dumps(receipt)}])
                wrapper = next(e['payload'] for e in self.events if e['payload'].get('call_id') == 'correction-wrapper')
                wrapper['input'] = 'const r=await tools.mcp__volicord__context_record(' + json.dumps(args) + '); text(JSON.stringify(r));'
                answer = self.generated_answer()
                answer['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 2
                _, fact = self.evaluate(answer)
                self.assertEqual(fact['status'], 'indeterminate', fact)
                self.assertIsNone(fact['basis']['observations'][0]['goal_basis']['revision'])
                answer['next_step'] = 'Wrong recorded action'
                _, fact = self.evaluate(answer)
                self.assertEqual(fact['status'], 'confirmed_violation', fact)

    def test_pre_and_post_correction_recalls_keep_their_own_basis(self):
        self.insert_correction()
        new = self.generated_answer()
        new['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 2
        _, fact = self.evaluate(new, cli=self.generated_answer(), cli_before=True)
        self.assertEqual(fact['status'], 'confirmed_pass', fact)
        observations = fact['basis']['observations']
        self.assertEqual([o['goal_basis']['revision'] for o in observations], [1, 2])
        self.assertEqual([o['transport'] for o in observations], ['cli', 'mcp'])

    def test_two_mcp_reads_keep_unavailable_then_current_observations(self):
        wrapper_id = self.output['call_id']
        original = [e for e in self.events if e['payload'].get('call_id') in
            {wrapper_id, 'exec-' + wrapper_id}]
        copies = copy.deepcopy(original)
        for event in copies:
            event['payload']['call_id'] = ('exec-first-' + wrapper_id
                if event['payload']['call_id'].startswith('exec-') else 'first-' + wrapper_id)
            event['timestamp'] = original[0]['timestamp']
        # A distinct wrapper/result pair, before the supported later read.
        index = self.events.index(original[0])
        records = []
        turn = next(e['payload']['turn_id'] for e in self.events if e['payload'].get('type') == 'task_started')
        for kind, identity in [('work', self.answer['selected_work']['work_item_id']), ('decision', '07' * 16)]:
            call_id = kind + '-explanation-record'
            receipt = {'operation': 'explanation_record', 'explanation': {
                'project_id': self.answer['project_id'], 'subject': {'kind': kind, 'identity': list(bytes.fromhex(identity))}}}
            common = {'timestamp': original[0]['timestamp'], 'type': 'response_item'}
            metadata = {'turn_id': turn}
            records += [common | {'payload': {'type': 'custom_tool_call', 'name': 'exec', 'status': 'completed',
                'call_id': call_id, 'input': 'text(await tools.exec_command(' + json.dumps({
                    'cmd': 'volicord --json ' + kind + ' explain record --' + kind + ' ' + identity}) + '));',
                'internal_chat_message_metadata_passthrough': metadata}},
                common | {'payload': {'type': 'custom_tool_call_output', 'call_id': call_id,
                    'output': [{'type': 'input_text', 'text': 'Script completed\nWall time 0.1 seconds\nOutput:\n'},
                        {'type': 'input_text', 'text': json.dumps({'exit_code': 0, 'output': json.dumps(receipt)})}],
                    'internal_chat_message_metadata_passthrough': metadata}}]
        self.events[index:index] = copies + records
        _, fact = self.evaluate(self.generated_answer())
        self.assertEqual(fact['status'], 'confirmed_pass', fact)
        reads = [o for o in fact['basis']['observations'] if o['transport'] == 'mcp']
        self.assertEqual(len(reads), 2)
        self.assertLess(reads[0]['sequence'], reads[1]['sequence'])
        cap = h.load_codex_capture(self.path)
        self.assertEqual([c.result['selected_work']['answers']['explanation_state']
            for c in cap.successful_calls('recall')], ['unavailable', 'current'])
        import explanation_evidence
        recorded = explanation_evidence.measured_cli_operations(cap)
        self.assertEqual([v['operation'] for v in recorded], ['explanation_record', 'explanation_record'])
        self.assertTrue(all(v['result'] is not None and v['exit_code'] == 0
            and reads[0]['sequence'] < v['sequence'] < reads[1]['sequence'] for v in recorded))

    def test_later_unrelated_and_failed_corrections_do_not_advance_basis(self):
        for kwargs in ({'after': True}, {'identity': 'fb' * 16}, {'failed': True}):
            with self.subTest(kwargs=kwargs):
                self.setUp(); self.insert_correction(**kwargs)
                _, fact = self.evaluate(self.generated_answer())
                self.assertEqual(fact['status'], 'confirmed_pass', fact)
                self.assertEqual(fact['basis']['observations'][0]['goal_basis']['revision'], 1)

    def test_replayed_receipts_do_not_increment_or_rewind_revision(self):
        self.insert_correction(call_id='correction-first')
        self.insert_correction(call_id='correction-replay')
        self.insert_correction(expected=2, revision=3, call_id='correction-third')
        self.insert_correction(call_id='old-replay-after-third')
        answer = self.generated_answer(); answer['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 3
        _, fact = self.evaluate(answer)
        self.assertEqual(fact['status'], 'confirmed_pass', fact)
        basis = fact['basis']['observations'][0]['goal_basis']
        self.assertEqual(basis['revision'], 3)
        self.assertEqual(basis['sources'], ['03' * 16])
        self.assertEqual(sum(w.get('transition') == 'replayed_receipt_no_advance' for w in basis['witnesses']), 2)
        self.assertEqual({w['authorization_source_id'] for w in basis['witnesses'] if 'authorization_source_id' in w}, {'fa' * 16})
        answer['selected_work']['answers']['provenance']['evidence'][1]['sources'] = ['fa' * 16]
        _, fact = self.evaluate(answer)
        self.assertEqual(fact['status'], 'confirmed_violation', fact)

    def test_missing_temporal_basis_is_scoped_and_never_excuses_proven_errors(self):
        for gap in ('clock', 'overlap', 'handoff', 'predecessor', 'conflicting_predecessor', 'receipt'):
            with self.subTest(gap=gap):
                self.setUp()
                if gap in {'clock', 'overlap'}:
                    for event in self.events:
                        event.pop('timestamp', None) if gap == 'clock' else event.update(timestamp='2026-08-15T00:00:00Z')
                elif gap == 'handoff':
                    reference = self.descriptor['evidence']['captures']['work']; path = self.root / reference['file']
                    events = [json.loads(line) for line in path.read_text().splitlines()]
                    # Missing completion means session relationship is unproven;
                    # earlier valid receipts are retained unchanged.
                    events.pop()
                    path.write_text(''.join(json.dumps(e) + '\n' for e in events)); reference['sha256'] = h.sha256(path)
                elif gap == 'predecessor':
                    self.insert_correction(expected=2, revision=3)
                elif gap == 'conflicting_predecessor':
                    self.insert_correction(call_id='first-correction')
                    self.insert_correction(expected=2, revision=3, call_id='third-revision')
                    event = self.insert_correction(call_id='conflicting-correction')
                    event['payload']['invocation']['arguments']['corrected_text'] = 'Different correction.'
                    wrapper = next(e['payload'] for e in self.events if e['payload'].get('call_id') == 'conflicting-correction-wrapper')
                    wrapper['input'] = 'const r=await tools.mcp__volicord__canonical_mutate(' + json.dumps(event['payload']['invocation']['arguments']) + '); for(const c of (r.content||[])) if(c.type==="text") text(c.text);'
                else:
                    event = self.insert_correction(); receipt = event['payload']['result']['Ok']['structuredContent']
                    receipt.pop('revision'); event['payload']['result']['Ok']['content'][0]['text'] = json.dumps(receipt)
                answer = self.generated_answer()
                if gap == 'predecessor':
                    answer['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 3
                _, fact = self.evaluate(answer)
                self.assertEqual(fact['status'], 'indeterminate', fact)
                self.assertEqual(m.disposition('shared_answer_integrity', fact['status']), 'qualitative_review_required')
                for error in ('Project', 'Work', 'action', 'grounding'):
                    wrong = copy.deepcopy(answer)
                    if error == 'Project': wrong['project_id'] = 'ff' * 16
                    elif error == 'Work': wrong['selected_work']['work_item_id'] = 'ff' * 16
                    elif error == 'action': wrong['next_step'] = 'Another task'
                    else: wrong['selected_work']['answers']['provenance']['evidence'][1]['revision'] = True
                    _, bad = self.evaluate(wrong)
                    self.assertEqual(bad['status'], 'confirmed_violation', (gap, error, bad))

    def test_missing_checkpoint_row_does_not_hide_generated_grounding_or_action_errors(self):
        reference = self.descriptor['evidence']['canonical_bundle']; path = self.root / reference['file']
        bundle = json.loads(path.read_text())
        next(table for table in bundle['payload']['tables'] if table['name'] == 'checkpoints')['rows'] = []
        self.write_bundle(path, bundle); reference['sha256'] = h.sha256(path)
        answer = self.generated_answer()
        _, fact = self.evaluate(answer)
        self.assertEqual(fact['status'], 'indeterminate', fact)
        answer['selected_work']['answers']['provenance']['evidence'][1]['field'] = 'another_field'
        _, fact = self.evaluate(answer)
        self.assertEqual(fact['status'], 'confirmed_violation', fact)

    def test_unavailable_bundle_keeps_generated_temporal_limits_and_typed_errors(self):
        self.descriptor['evidence']['canonical_bundle']['sha256'] = 'f' * 64
        answer = self.generated_answer()
        _, fact = self.evaluate(answer)
        self.assertEqual(fact['status'], 'indeterminate', fact)
        answer['selected_work']['answers']['provenance']['evidence'][1]['revision'] = True
        _, fact = self.evaluate(answer)
        self.assertEqual(fact['status'], 'confirmed_violation', fact)

    def test_supported_cli_correction_receipt_is_project_bound(self):
        for has_project in (True, False):
            with self.subTest(has_project=has_project):
                self.setUp()
                work = self.answer['selected_work']['work_item_id']
                project_flag = '--project ' + self.answer['project_id'] + ' ' if has_project else ''
                cmd = ('volicord ' + project_flag + '--json advanced records correct-context ' + work
                    + ' --revision 1 --source ' + 'fa' * 16 + ' --text "Punctuation correction."')
                receipt = {'operation': 'correct_context', 'record_kind': 'context_item', 'identity': work, 'revision': 2, 'replayed': False}
                index = next(i for i,e in enumerate(self.events) if e['payload'].get('type') == 'custom_tool_call'
                    and 'recall-call' in e['payload'].get('call_id', ''))
                stamp = self.events[index]['timestamp']; turn = next(e['payload']['turn_id'] for e in self.events if e['payload'].get('type') == 'task_started')
                self.events[index:index] = [
                    {'timestamp': stamp, 'type': 'response_item', 'payload': {'type': 'custom_tool_call', 'call_id': 'cli-correction',
                        'name': 'exec', 'status': 'completed', 'input': 'const r=await tools.exec_command(' + json.dumps({'cmd': cmd, 'workdir': '/phase8/repository'}) + '); text(JSON.stringify(r));',
                        'internal_chat_message_metadata_passthrough': {'turn_id': turn}}},
                    {'timestamp': stamp, 'type': 'response_item', 'payload': {'type': 'custom_tool_call_output', 'call_id': 'cli-correction',
                        'output': [{'type': 'input_text', 'text': 'Script completed\nWall time 0.1 seconds\nOutput:\n' + json.dumps({'output': json.dumps(receipt), 'exit_code': 0})}],
                        'internal_chat_message_metadata_passthrough': {'turn_id': turn}}}]
                answer = self.generated_answer(); answer['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 2
                _, fact = self.evaluate(answer)
                self.assertEqual(fact['status'], 'confirmed_pass' if has_project else 'indeterminate', fact)
                if has_project:
                    answer['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 1
                    _, wrong = self.evaluate(answer)
                    self.assertEqual(wrong['status'], 'confirmed_violation', wrong)

    def test_completion_order_does_not_guess_an_overlapping_or_unrecorded_invocation(self):
        for mode in ('overlap', 'missing_recall_start', 'missing_recall_start_without_correction', 'late_completion_only'):
            with self.subTest(mode=mode):
                self.setUp()
                event = (self.insert_correction(after=mode in {'overlap', 'late_completion_only'})
                    if mode != 'missing_recall_start_without_correction' else None)
                if mode.startswith('missing_recall_start'):
                    # A standalone completion remains readable, but its request
                    # boundary is absent; no revision is inferred from the answer.
                    wrapper_id = next(e['payload']['call_id'] for e in self.events if e['payload'].get('type') == 'custom_tool_call'
                        and 'recall-call' in e['payload'].get('call_id', ''))
                    self.events = [e for e in self.events if e['payload'].get('call_id') != wrapper_id]
                elif mode == 'late_completion_only':
                    self.events = [e for e in self.events if e['payload'].get('call_id') != 'correction-wrapper']
                else:
                    self.evaluate(self.generated_answer(), cli=self.generated_answer())
                    wrapper = next(e for e in self.events if e['payload'].get('call_id') == 'correction-wrapper')
                    self.events.remove(wrapper); self.events.remove(event)
                    index = next(i for i,e in enumerate(self.events) if e['payload'].get('type') == 'custom_tool_call'
                        and e['payload'].get('call_id') == 'cli-recall')
                    stamp = self.events[index]['timestamp']; wrapper['timestamp'] = stamp; event['timestamp'] = stamp
                    # The CLI request begins while correction is in flight. Its
                    # intact response cannot tell when the canonical read occurred.
                    self.events.insert(index, wrapper)
                    self.events.insert(index + 2, event)
                _, fact = self.evaluate(self.generated_answer())
                self.assertEqual(fact['status'], 'indeterminate', fact)
                wrong = self.generated_answer(); wrong['selected_work']['answers']['provenance']['evidence'][1]['field'] = 'wrong'
                _, fact = self.evaluate(wrong)
                self.assertEqual(fact['status'], 'confirmed_violation', fact)

    def test_start_correction_folds_into_a_proven_fresh_resume(self):
        event = self.insert_correction()
        wrapper = next(e for e in self.events if e['payload'].get('call_id') == 'correction-wrapper')
        self.events.remove(event); self.events.remove(wrapper)
        reference = self.descriptor['evidence']['captures']['work']; path = self.root / reference['file']
        events = [json.loads(line) for line in path.read_text().splitlines()]
        stamp = events[-1]['timestamp']; turn = events[-1]['payload']['turn_id']
        for value in (wrapper, event): value['timestamp'] = stamp
        wrapper['payload']['internal_chat_message_metadata_passthrough']['turn_id'] = turn
        events[-1:-1] = [wrapper, event]
        path.write_text(''.join(json.dumps(e) + '\n' for e in events)); reference['sha256'] = h.sha256(path)
        answer = self.generated_answer(); answer['selected_work']['answers']['provenance']['evidence'][1]['revision'] = 2
        _, fact = self.evaluate(answer)
        self.assertEqual(fact['status'], 'confirmed_pass', fact)
        basis = fact['basis']['observations'][0]['goal_basis']
        self.assertEqual(basis['revision'], 2)
        self.assertEqual(len({w['raw_capture_sha256'] for w in basis['witnesses']}), 1)

    def test_generated_grounding_types_language_and_provenance_are_factual(self):
        correct = self.generated_answer()
        _, fact = self.evaluate(correct)
        self.assertEqual(fact['status'], 'confirmed_pass', fact)
        self.assertIn('generated prose adequacy requires qualitative review', fact['basis']['observations'][-1]['limits'])
        for mutation in ('claim_key', 'record_key', 'revision', 'language', 'provenance'):
            wrong = copy.deepcopy(correct); answers = wrong['selected_work']['answers']
            if mutation == 'claim_key':
                answers['prose'][0]['evidence_keys'] = [{'unexpected': 'typed value'}]
            elif mutation == 'record_key':
                answers['provenance']['evidence'][0]['key'] = ['unexpected']
            elif mutation == 'revision':
                answers['provenance']['evidence'][0]['revision'] = True
            elif mutation == 'language':
                answers['provenance']['language'] = 'ko'
            else:
                answers['provenance']['generator_identity_status'] = 'verified'
            _, fact = self.evaluate(wrong)
            self.assertEqual(fact['status'], 'confirmed_violation', (mutation, fact))
            self.assertEqual(m.disposition('shared_answer_integrity', fact['status']), 'hard_blocking')

    def test_scoped_generated_omissions_are_indeterminate_without_excusing_wrong_values(self):
        marker = {'transport_omission': {'reason': 'serialized_byte_budget', 'omitted_count': 1,
            'basis': 'same parent identity, field and stable input order; inspect the authoritative record'}}
        whole_field = {'transport_omission': {'reason': 'serialized_byte_budget', 'exact_json_bytes': 400,
            'basis': 'inspect the complete field on the authoritative parent record'}}
        for field in ('evidence', 'evidence_keys', 'text', 'language', 'generator_identity_status', 'prose'):
            value = self.generated_answer(); answers = value['selected_work']['answers']
            if field == 'evidence':
                answers['provenance']['evidence'] = [answers['provenance']['evidence'][0], marker]
            elif field == 'evidence_keys':
                answers['prose'][0]['evidence_keys'] = ['next_step', marker]
            elif field == 'text':
                answers['prose'][0]['text'] = whole_field
            elif field == 'prose':
                answers['prose'] = whole_field
            else:
                answers['provenance'][field] = whole_field
            _, fact = self.evaluate(value)
            self.assertEqual(fact['status'], 'indeterminate', (field, fact))
            value['next_step'] = 'Wrong direction despite a legitimate unrelated omission'
            _, fact = self.evaluate(value)
            self.assertEqual(fact['status'], 'confirmed_violation', (field, fact))
        wrong = self.generated_answer()
        wrong['selected_work']['answers']['prose'][0].update(text=whole_field, evidence_keys=['undeclared'])
        _, fact = self.evaluate(wrong)
        self.assertEqual(fact['status'], 'confirmed_violation', fact)
        wrong = self.generated_answer()
        wrong['selected_work']['answers']['provenance']['language'] = 'ko'
        wrong['selected_work']['answers']['provenance']['transport_omission'] = marker['transport_omission']
        _, fact = self.evaluate(wrong)
        self.assertEqual(fact['status'], 'confirmed_violation', fact)


if __name__ == '__main__':
    unittest.main()
