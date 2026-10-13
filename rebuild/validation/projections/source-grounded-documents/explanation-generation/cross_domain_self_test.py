"""Independent cross-language/test evidence through scoped reads and reading HTML.

Authored fixture responses and host joins are transport controls, never measured
generation, historical test execution, semantic assessment or holdout evidence.
"""
import base64
import copy
import json
from pathlib import Path
import unittest

import inputs as i
from input_self_test import spec
from grounding import validate_output
from reader_contract import DIRECTED_CONTRACT
import render_self_test as fixture
from source_reading import locate
from source_tools import Surface


# CC0-1.0 authored bytes; coordinates counted independently of the consumer.
C_BEFORE = '/* λ */\r\nint run(int n) {\r\n  if (n < 0) return -1;\r\n  cleanup();\r\n  return n;\r\n}\r\n'
C_AFTER = C_BEFORE.replace('return -1;', 'return -2;')
TS = 'export function call(n: number): number {\n  return nativeRun(n);\n}\n'
TEST = 'test("negative input", () => {\n  expect(call(-1)).toBe(-2);\n});\n'


class CrossDomainTests(unittest.TestCase):
    def setUp(self):
        self.h = fixture.RenderTests(methodName='runTest')
        self.h.setUp()
        self.addCleanup(self.h.doCleanups)
        template = spec(self.h.source)
        template['entries'] = []
        records = [
            ('c-before', 'native/operation.c', C_BEFORE, 'source', 'explicit_before_patch', 'fixture:change:before'),
            ('c-after', 'native/operation.c', C_AFTER, 'source', 'explicit_work_patch', 'fixture:change'),
            ('ts-context', 'client/operation.ts', TS, 'source', 'repository_context', 'fixture:client'),
            ('test-definition', 'client/operation.test.ts', TEST, 'source', 'repository_context', 'fixture:test'),
            ('failed-invocation', 'commands/first.json', i.encoded({'argv': ['test', 'negative'], 'exit_code': 1,
                'termination': 'exited', 'output': 'Assertion failed.'}).decode(),
                'command_observation', 'recorded_execution', 'fixture:first'),
            ('passed-invocation', 'commands/second.json', i.encoded({'argv': ['test', 'negative'], 'exit_code': 0,
                'termination': 'exited', 'output': 'One test passed.'}).decode(),
                'command_observation', 'recorded_execution', 'fixture:second'),
        ]
        for number, (identity, path, body, role, attribution, locator) in enumerate(records):
            asset = self.h.root / identity
            asset.write_bytes(body.encode())
            entry = spec(asset)['entries'][0]
            entry.update(id=identity, path=path, role=role, attribution=attribution, locator=locator,
                         before_state='available' if identity.startswith('c-') else 'unavailable')
            if role == 'command_observation':
                entry['chronology']['observed_at'] = f'2026-10-08T09:0{number}:00+00:00'
            template['entries'].append(entry)
        path = self.h.root / 'cross-domain-spec.json'
        path.write_bytes(i.encoded(template))
        self.manifest = i.freeze(path, self.h.root / 'cross-domain-frozen', self.h.root)
        self.frozen = i.verify(self.manifest)
        self.entries = {e['id']: e for e in self.frozen['entries']}
        self.surface = Surface(self.manifest, 'archive_diagnostic', {'reads': 32, 'read_bytes': 10000},
                               self.h.root / 'cross-domain-reads.jsonl')
        self.response = {'prose': '', 'selections': [], 'gaps': [
            'Static language-boundary source is not a demonstrated runtime trace.',
            'Authored command observations are independent transport fixtures, not actual test runs.'],
            'claims': [], 'primary_sites': []}
        self.h.record.update(input=i.binding(self.manifest), output_contract=DIRECTED_CONTRACT)

    def selection(self, identity, start=0, end=None):
        data = Path(self.entries[identity]['asset']['path']).read_bytes()
        end = len(data) if end is None else end
        row = self.surface.call('read', {'id': identity, 'offset': start, 'limit': end - start})
        self.assertEqual(row['status'], 'returned')
        self.assertEqual(row['result']['text'].encode(), data[start:end])
        self.assertEqual(row['result']['metadata']['sha256'], i.digest(data[start:end]))
        state = 'before' if identity == 'c-before' else 'after' if identity == 'c-after' else 'context'
        self.response['selections'].append({'id': identity, 'start': start, 'end': end,
            'sha256': i.digest(data[start:end]), 'state': state})
        return len(self.response['selections']) - 1

    def claim(self, text, kind, selections):
        start = len(self.response['prose'].encode())
        self.response['prose'] += text + '\n'
        self.response['claims'].append({'start': start, 'end': start + len(text.encode()),
            'kind': kind, 'selections': selections, 'authority': None})
        return len(self.response['claims']) - 1

    def site(self, selections, claims):
        self.response['primary_sites'].append({'selections': selections, 'claims': claims,
            'reason': 'Authored source binding; runtime relationship and relevance remain unassessed.',
            'extent_reason': 'The selected statement is the complete fixture operation being cited.'})

    def validation(self, frozen=None, trace=None):
        return validate_output(self.response, frozen or self.frozen, 'archive_diagnostic',
            trace if trace is not None else [json.loads(line) for line in self.surface.trace.read_bytes().splitlines()],
            contract=DIRECTED_CONTRACT)

    def present(self):
        self.h.response_path.write_bytes(i.encoded(self.response))
        self.h.record.update(retrievals=i.binding(self.surface.trace),
            original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        self.h.host_reads(self.surface.trace)  # Explicitly authored host/ledger join.
        original = self.h.response_path.read_bytes()
        body, witness, parser = self.h.present()
        self.assertEqual(self.h.response_path.read_bytes(), original)
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        downloads = [base64.b64decode(link.split(',', 1)[1]) for link in parser.links if link.startswith('data:')]
        self.assertIn(original, downloads)
        return body, witness, parser

    def boundary(self):
        old = self.selection('c-before', 30, 51)
        new = self.selection('c-after', 30, 51)
        client = self.selection('ts-context', 44, 64)
        first = self.claim('The C branch returned -1 in the before state.', 'source_fact', [old])
        second = self.claim('The C branch returns -2 in the after state.', 'source_fact', [new])
        third = self.claim('The TypeScript source declares a nativeRun expression.', 'source_fact', [client])
        self.site([old, new], [first, second])
        self.site([client], [third])

    def test_exact_c_and_typescript_ranges_do_not_manufacture_parser_or_runtime_proof(self):
        self.boundary()
        validation = self.validation()
        self.assertEqual(validation['reading']['status'], 'valid_binding')
        expected = [('if (n < 0) return -1;', [3, 3], [3, 24]),
                    ('if (n < 0) return -2;', [3, 3], [3, 24]),
                    ('return nativeRun(n);', [2, 3], [2, 23])]
        for selection, result, (text, start, end) in zip(self.response['selections'], validation['selections'], expected):
            location = locate(self.frozen, self.entries[selection['id']], result)
            self.assertEqual((location['text'], location['start'], location['end']), (text, start, end))
            self.assertIsNone(location['callable'])  # The optional syntax producer is Python-only.
        body, witness, parser = self.present()
        self.assertEqual(parser.values['code'][:3], [e[0] for e in expected])
        self.assertIn('Static language-boundary source is not a demonstrated runtime trace.', body)
        self.assertEqual(witness['outputs'][0]['reading']['semantic_correctness'], 'pending_independent_examination')

    def test_existing_cleanup_is_unchanged_context_in_actual_comparison(self):
        old, new = self.selection('c-before', 55, 65), self.selection('c-after', 55, 65)
        claim = self.claim('Both source states contain the cleanup statement.', 'source_fact', [old, new])
        self.site([old, new], [claim])
        body, witness, parser = self.present()
        comparison = witness['outputs'][0]['comparisons'][0]
        self.assertEqual(comparison['status'], 'verified_pair')
        self.assertEqual(comparison['unchanged_selected_excerpts'], 2)
        self.assertEqual(comparison['focused_hunks'], 0)
        self.assertNotIn('diff', parser.values)
        self.assertIn('No selected-span diff is invented.', body)

    def test_definition_failure_pass_and_inferred_effect_keep_distinct_bindings(self):
        definition = self.selection('test-definition', 33, 59)
        failed, passed = self.selection('failed-invocation'), self.selection('passed-invocation')
        claim = self.claim('The source defines a negative-input assertion.', 'source_fact', [definition])
        self.site([definition], [claim])
        self.claim('The earlier authored invocation records failure and exit 1.', 'execution_evidence', [failed])
        self.claim('The later authored invocation records one pass and exit 0.', 'execution_evidence', [passed])
        self.claim('Broader Product effects remain unverified.', 'unverified_expectation', [definition])
        body, witness, parser = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertEqual(parser.values['bound-claim'], ['The source defines a negative-input assertion.'])
        # Non-primary claims remain in unchanged prose and the exact sidecar;
        # the code reading surface does not turn commands into primary code.
        for statement in self.response['prose'].splitlines():
            self.assertIn(statement, parser.values['prose'][0])
        self.assertEqual([c['kind'] for c in self.response['claims']],
                         ['source_fact', 'execution_evidence', 'execution_evidence', 'unverified_expectation'])
        self.assertIn('not actual test runs', body)
        for identity in ('failed-invocation', 'passed-invocation'):
            expected = Path(self.entries[identity]['asset']['path']).read_bytes().decode()
            self.assertIn(expected, parser.values['code'])
        trace = [json.loads(line) for line in self.surface.trace.read_bytes().splitlines()
                 if json.loads(line)['arguments']['id'] != 'passed-invocation']
        self.assertEqual(self.validation(trace=trace)['reading']['status'], 'invalid')

    def test_cross_domain_negative_grounding_and_missing_host_observation(self):
        self.boundary()
        for defect in ('work', 'chronology', 'before', 'range', 'retrieval'):
            with self.subTest(defect=defect):
                frozen, response = copy.deepcopy(self.frozen), copy.deepcopy(self.response)
                trace = None
                if defect == 'work': frozen['entries'][0]['work'] = 'foreign'
                if defect == 'chronology': frozen['entries'][0]['chronology']['observed_at'] = '2026-10-09T00:00:00+00:00'
                if defect == 'before': self.response['selections'][2]['state'] = 'before'
                if defect == 'range': self.response['selections'][0]['end'] = 100000
                if defect == 'retrieval': trace = []
                self.assertEqual(self.validation(frozen, trace)['reading']['status'], 'invalid')
                self.response = response
        self.present()
        self.h.record['calls'] = []
        body, witness, parser = self.h.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'invalid')
        self.assertNotIn('code', parser.values)
        self.assertEqual(parser.values['prose'], [self.response['prose']])


if __name__ == '__main__':
    unittest.main()
