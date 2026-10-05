#!/usr/bin/env python3
"""Independent Product-contract transition controls for the Naturalistic observer.

Transport captures are authored support, not measured Product/human observations.
The expected table comes from selection.rs and projections-and-documents.md.
"""
import copy
from dataclasses import replace
import json
from pathlib import Path
import sys
import tempfile
import unittest

import answer_observations as a
import harness as h
import machine_findings as m

A, B, C = '08' * 16, 'aa' * 16, 'cc' * 16
CPA, CPB = '09' * 16, 'bb' * 16


class LatestWorkTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = self.root = Path(temporary.name)
        descriptor = h.real_session_fixture('volicord', 1, '0' * 40, root)
        self.descriptor = descriptor
        evidence = descriptor['evidence']
        self.work = h.load_codex_capture(root / evidence['captures']['work']['file'])
        self.resume = h.load_codex_capture(root / evidence['captures']['resume']['file'])
        self.bundle = h.load_canonical_bundle(root / evidence['canonical_bundle']['file'])
        self.answer = self.resume.successful_calls('recall')[0].result
        self.templates = {op: self.work.successful_calls(op)[0]
            for op in ('project_initialize', 'context_record', 'checkpoint_record')}

    def scenario(self, events, *, returned_work=A, checkpoint=CPA, missing_request=None,
                 overlap=None, resume=False, revision=1, wrong_project=False, future=None, wrong_goal=False, malformed=None, continued=None, wrong_sources=False):
        """Explicit receipts/answers; no oracle output feeds expected values."""
        if resume:
            events = [('init', None), ('goal', A), ('cp', A)] + events
        calls, invocations = [], {}
        for index, (operation, identity) in enumerate(events, 1):
            op = {'init': 'project_initialize', 'goal': 'context_record', 'cp': 'checkpoint_record'}[operation]
            template = self.templates[op]
            args, receipt = copy.deepcopy(template.arguments), copy.deepcopy(template.result)
            if operation == 'goal':
                args['statement'] = 'Independent Goal ' + identity
                args['work_transition'] = 'start_new'
                receipt.update(context_item_id=identity, canonical_mutation=True)
            elif operation == 'cp':
                args['goal_context_id'] = identity
                receipt.update(goal_context_id=identity, checkpoint_id=CPA if identity == A else CPB)
            call = replace(template, call_id='transition-' + str(index), sequence=index * 4 + 1,
                completion_sequence=index * 4 + 1, arguments=args, result=receipt)
            calls.append(call)
            invocations[call.call_id] = index * 4
        if malformed is not None:
            calls[malformed] = replace(calls[malformed], outcome='failed', error='malformed_mcp_completion')
        if continued is not None:
            calls[continued] = replace(calls[continued],
                arguments=dict(calls[continued].arguments, work_transition='continue'),
                result=dict(calls[continued].result, canonical_mutation=False))
        if missing_request is not None:
            invocations.pop(calls[missing_request].call_id)
        if overlap is not None:
            invocations[calls[overlap].call_id] = calls[overlap - 2].completion_sequence
        answer = copy.deepcopy(self.answer)
        answer['goal_basis'][0]['identity'] = returned_work
        answer['selected_work']['work_item_id'] = returned_work
        answer['selected_work']['checkpoint_ids'] = [checkpoint] if checkpoint else []
        if checkpoint:
            answer['checkpoint'].update(identity=checkpoint, work_item_id=returned_work, revision=revision)
            action = answer['selected_work']['answers']['facts'][0]['recorded_action']
            action.update(work_item_id=returned_work, checkpoint_id=checkpoint, revision=revision)
            answer['selected_work']['answers']['facts'][0]['evidence_keys'] = [f'checkpoint:{checkpoint}@{revision}:next_step']
        else:
            answer.update(checkpoint=None, next_step=None)
            answer['selected_work']['answers']['facts'] = [{'question': 'NextStepAvailability',
                'role': 'unavailable', 'text': 'No next action is recorded: this Work has no Checkpoint.', 'evidence_keys': []}]
        if wrong_goal:
            answer['goal_basis'][0]['identity'] = C
        if wrong_sources:
            answer['goal_basis'][0]['source_ids'] = ['ff' * 16]
        self.returned_answer = answer
        if wrong_project:
            answer['project_id'] = 'ff' * 16
        recall = replace(self.resume.successful_calls('recall')[0], call_id='transition-recall',
            sequence=len(calls) * 4 + 9, completion_sequence=len(calls) * 4 + 9, result=answer)
        invocations[recall.call_id] = recall.sequence - 1
        if future is not None:
            invocations[calls[future].call_id] = recall.completion_sequence + 1
            calls[future] = replace(calls[future], sequence=recall.completion_sequence + 2,
                completion_sequence=recall.completion_sequence + 2)
        tables = copy.deepcopy(self.bundle.tables)
        cp = dict(self.bundle.one('checkpoints', project_id=self.bundle.project_id, id=CPA), id=CPB, work_item_id=B)
        tables['checkpoints'] += (cp,)
        tables['checkpoint_source_relations'] += tuple(dict(row, checkpoint_id=CPB)
            for row in tables['checkpoint_source_relations'] if row.get('checkpoint_id') == CPA)
        bundle = replace(self.bundle, tables=tables)
        capture = replace(self.work, tool_calls=tuple(calls + [recall]), commands=(),
            observed_metadata={**self.work.observed_metadata, 'mcp_invocations': invocations})
        if resume:
            # Retain a genuinely completed handoff/fresh-resolution relationship;
            # independently bound resume Work A must survive selector inference B.
            reads = [replace(call, result=answer) if call.operation == 'recall' else call
                for call in self.resume.tool_calls]
            capture = replace(self.resume, tool_calls=tuple(reads), commands=())
            selection = a.observe(replace(self.work, tool_calls=tuple(calls), commands=(),
                observed_metadata={**self.work.observed_metadata, 'mcp_invocations': invocations}), capture, bundle, A)
        else:
            selection = a.observe(capture, None, bundle, A)
        finding = next(f for f in m.from_observation({'checks': {}, 'machine_facts': {
            'shared_answer_integrity': selection}}) if f['check'] == 'shared_answer_integrity')
        return selection['basis']['observations'][0], finding

    def check(self, events, expected, *, returned_work=None, checkpoint=None, status='confirmed_pass', **kwargs):
        observation, finding = self.scenario(events, returned_work=returned_work or expected or A,
            checkpoint=checkpoint, **kwargs)
        self.assertEqual(observation['status'], status, observation)
        self.assertEqual(observation['lifecycle_basis']['expected_work'], expected, observation)
        self.assertEqual(finding['disposition'], {'confirmed_pass': 'advisory',
            'confirmed_violation': 'hard_blocking', 'indeterminate': 'qualitative_review_required'}[status])
        return observation

    def test_checkpoint_then_new_goal_preserves_checkpoint_work(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B)], A, checkpoint=CPA)

    def test_new_work_checkpoint_advances_selection(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B), ('cp', B)], B, checkpoint=CPB)

    def test_goal_fallback_requires_project_checkpoint_absence(self):
        self.check([('init', None), ('goal', A)], A)
        self.check([('init', None), ('goal', A), ('goal', B)], B)

    def test_ordered_checkpoints_and_unrelated_later_goal(self):
        for newest, cp in ((A, CPA), (B, CPB)):
            self.check([('init', None), ('goal', A), ('goal', B), ('cp', B if newest == A else A),
                ('cp', newest), ('goal', C)], newest, checkpoint=cp)

    def test_missing_or_overlapping_checkpoint_chronology_is_indeterminate(self):
        events = [('init', None), ('goal', A), ('cp', A), ('goal', B), ('cp', B)]
        for kwargs in ({'missing_request': 4}, {'overlap': 4}):
            self.check(events, None, returned_work=B, checkpoint=CPB, status='indeterminate', **kwargs)

    def test_known_checkpoint_existence_survives_unknown_latest_order(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B), ('cp', B)],
            None, returned_work=B, checkpoint=None, missing_request=4, status='confirmed_violation')

    def test_malformed_checkpoint_completion_is_uncertainty_not_failed_write(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B)], None,
            returned_work=B, checkpoint=None, malformed=2, status='indeterminate')

    def test_repeated_checkpoint_receipt_cannot_supersede_newer_publication(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B), ('cp', B), ('cp', A)],
            B, checkpoint=CPB)

    def test_continuation_is_not_newest_goal_creation(self):
        self.check([('init', None), ('goal', A), ('goal', B), ('goal', A)], B, continued=3)
        self.check([('init', None), ('goal', A)], None, continued=1, status='indeterminate')

    def test_fresh_goal_without_empty_project_witness_cannot_prove_no_prior_checkpoint(self):
        self.check([('goal', B)], None, returned_work=B, status='indeterminate')

    def test_wrong_work_and_checkpoint_absence_are_hard(self):
        events = [('init', None), ('goal', A), ('cp', A), ('goal', B)]
        for cp in (None, CPB):
            self.check(events, A, returned_work=B, checkpoint=cp, status='confirmed_violation')
        self.check(events, A, checkpoint=None, status='confirmed_violation')

    def test_correct_work_wrong_checkpoint_revision_is_hard(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B)], A,
            checkpoint=CPA, revision=2, status='confirmed_violation')

    def test_checkpoint_removal_and_work_swap_change_actual_basis(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B)], A, checkpoint=CPA)
        self.check([('init', None), ('goal', A), ('goal', B)], B)
        self.check([('init', None), ('goal', A), ('goal', B)], B,
            returned_work=A, checkpoint=CPA, status='confirmed_violation')
        self.check([('init', None), ('goal', A), ('cp', B), ('goal', B)], B, checkpoint=CPB)

    def test_unknown_selection_does_not_hide_project_contradiction(self):
        self.check([('goal', B)], None, returned_work=B, wrong_project=True, status='confirmed_violation')

    def test_later_checkpoint_cannot_change_earlier_recall(self):
        self.check([('init', None), ('goal', A), ('cp', A), ('goal', B), ('cp', B)],
            A, checkpoint=CPA, future=4)

    def test_unknown_selection_keeps_goal_and_revision_contradictions(self):
        self.check([('goal', B)], None, returned_work=B, wrong_goal=True, status='confirmed_violation')
        self.check([('goal', A), ('cp', A)], None, checkpoint=CPA, revision=2,
            missing_request=1, status='confirmed_violation')
        observation, _ = self.scenario([('goal', B), ('cp', B)], returned_work=A,
            checkpoint=CPA, resume=True, malformed=4, wrong_sources=True)
        self.assertEqual(observation['lifecycle_basis']['selector_basis'], 'unknown')
        self.assertIn('observation-time Goal supporting Sources', observation['errors'])
        self.assertEqual(observation['status'], 'confirmed_violation')

    def test_raw_transition_reaches_campaign_and_machine_consumers(self):
        # Add the new Goal and read to original authored raw fixture bytes. The
        # real parser, harness observation and machine finding consumer execute.
        reference = self.descriptor['evidence']['captures']['work']
        path = self.root / reference['file']
        original = [json.loads(line) for line in path.read_text().splitlines()]
        for returned_work, checkpoint, expected_status in ((A, CPA, 'confirmed_pass'),
                (B, None, 'confirmed_violation')):
            self.scenario([('init', None), ('goal', A), ('cp', A), ('goal', B)],
                returned_work=returned_work, checkpoint=checkpoint)
            answer = copy.deepcopy(self.returned_answer)
            events = copy.deepcopy(original)
            turn = [e['payload']['turn_id'] for e in events if e['payload'].get('type') == 'task_started'][-1]
            args = copy.deepcopy(self.templates['context_record'].arguments)
            args['work_transition'] = 'start_new'
            receipt = dict(self.templates['context_record'].result, context_item_id=B, canonical_mutation=True)
            additions = []
            for operation, call_id, arguments, result in (
                    ('context_record', 'transition-goal', args, receipt),
                    ('recall', 'transition-recall', {'project_id': self.bundle.project_id}, answer)):
                additions += [
                    {'timestamp': events[-1]['timestamp'], 'type': 'response_item', 'payload': {
                        'type': 'custom_tool_call', 'name': 'exec', 'call_id': call_id + '-wrapper',
                        'status': 'completed', 'input': f'const r=await tools.mcp__volicord__{operation}(' + json.dumps(arguments) + '); text(JSON.stringify(r));',
                        'internal_chat_message_metadata_passthrough': {'turn_id': turn}}},
                    {'timestamp': events[-1]['timestamp'], 'type': 'event_msg', 'payload': {
                        'type': 'mcp_tool_call_end', 'call_id': call_id, 'turn_id': turn,
                        'invocation': {'server': 'volicord', 'tool': operation, 'arguments': arguments},
                        'result': {'Ok': {'content': [{'type': 'text', 'text': json.dumps(result)}],
                            'structuredContent': result, 'isError': False}}}}]
            events[-1:-1] = additions
            path.write_text(''.join(json.dumps(event) + '\n' for event in events))
            reference['sha256'] = h.sha256(path)
            result = h.real_session_evidence(self.descriptor, kind='volicord', cycle=1, repository_revision='0' * 40)
            observation = next(o for o in result['machine_facts']['shared_answer_integrity']['basis']['observations']
                if o['call_id'] == 'transition-recall')
            self.assertEqual(observation['status'], expected_status, observation)
            self.assertEqual(observation['lifecycle_basis']['expected_work'], A)
            finding = next(f for f in m.from_observation(result) if f['check'] == 'shared_answer_integrity')
            self.assertEqual(finding['disposition'], 'advisory' if expected_status == 'confirmed_pass' else 'hard_blocking')

    def test_resume_identity_conflict_is_not_masked_by_selector(self):
        observation, finding = self.scenario([('goal', B), ('cp', B)], returned_work=B, checkpoint=CPB, resume=True)
        self.assertEqual(observation['lifecycle_basis']['expected_work'], B)
        self.assertEqual(observation['lifecycle_basis']['selector_basis'], 'latest_checkpoint')
        self.assertEqual(observation['status'], 'confirmed_violation', observation)
        self.assertIn('resume Work identity', observation['errors'])
        self.assertEqual(finding['disposition'], 'hard_blocking')


def probe():
    test = LatestWorkTests()
    test.setUp()
    try:
        events = [('init', None), ('goal', A), ('cp', A), ('goal', B)]
        results = {}
        for name, work, cp in (('contract_correct_a', A, CPA), ('contract_incorrect_b', B, None)):
            observation, finding = test.scenario(events, returned_work=work, checkpoint=cp)
            results[name] = {'observation': observation, 'finding': finding}
        print(json.dumps({'support_contract': 'Product LatestWork checkpoint-first; source-contract authored controls',
            'evaluator_head': h.git_head(h.ROOT), 'results': results}, indent=2, sort_keys=True))
    finally:
        test.doCleanups()


if __name__ == '__main__':
    if sys.argv[1:] == ['--probe']:
        probe()
    else:
        unittest.main()
