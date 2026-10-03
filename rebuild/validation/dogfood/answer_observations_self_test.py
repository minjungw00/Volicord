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

    def evaluate(self, answer=None, cli=None):
        if answer is not None:
            self.output['output'][1]['text'] = json.dumps(answer)
            for event in self.events:
                payload = event['payload']
                if payload.get('type') == 'mcp_tool_call_end' and payload.get('invocation', {}).get('tool') == 'recall':
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
            self.events[index:index] = additions
        self.path.write_text(''.join(json.dumps(event) + '\n' for event in self.events))
        self.reference['sha256'] = h.sha256(self.path)
        result = h.real_session_evidence(self.descriptor, kind='volicord', cycle=1, repository_revision='0' * 40)
        return result, result['machine_facts']['shared_answer_integrity']

    def test_valid_unavailable_explanation_keeps_recorded_action(self):
        result, fact = self.evaluate()
        self.assertEqual(fact['status'], 'confirmed_pass', fact)
        self.assertEqual(result['checks']['recall_matches_checkpoint_decision_and_context'], 'passed')

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


if __name__ == '__main__':
    unittest.main()
