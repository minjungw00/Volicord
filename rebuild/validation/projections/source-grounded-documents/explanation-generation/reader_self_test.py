"""Independent authored controls of actual generation-to-reading transport."""
import copy
import argparse
import json
from html.parser import HTMLParser
from pathlib import Path
import unittest
from unittest.mock import patch

import approaches as a
import inputs as i
from grounding import validate_output
from reader_contract import CONTRACT
import render_self_test as baseline
import exploratory_self_test as controls


class ClaimLinks(HTMLParser):
    def __init__(self):
        super().__init__()
        self.active, self.claims = None, {}

    def handle_starttag(self, tag, attributes):
        attributes = dict(attributes)
        if tag == 'p' and attributes.get('class') == 'claim-sources':
            self.active = int(attributes['data-claim-index'])
            self.claims[self.active] = []
        if tag == 'a' and self.active is not None:
            self.claims[self.active].append(int(attributes['data-selection-index']))

    def handle_endtag(self, tag):
        if tag == 'p':
            self.active = None


class ReaderTests(unittest.TestCase):
    def setUp(self):
        self.h = baseline.RenderTests(methodName='runTest')
        self.h.setUp()
        self.addCleanup(self.h.doCleanups)
        self.response = self.h.response
        self.response['claims'] = [{'start': 0, 'end': len(self.response['prose'].encode()),
                                   'kind': 'source_fact', 'selections': [0, 1]}]
        self.response['primary_sites'] = [{'selections': [0, 1], 'claims': [0],
                                          'reason': 'Authored reason <tag>; independent fixture only.'}]
        self.h.record['output_contract'] = CONTRACT

    def present(self):
        self.h.response_path.write_bytes(i.encoded(self.response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)],
                             generation_output=i.binding(self.h.response_path))
        return self.h.present()

    def validation(self, spec=None, trace=None):
        return validate_output(self.response, spec or i.verify(self.h.manifest), 'archive_diagnostic',
                               trace if trace is not None else [json.loads(line) for line in
                                   (self.h.root / 'trace').read_bytes().splitlines()], contract=CONTRACT)

    def test_changed_pair_and_claim_round_trip_are_lossless(self):
        body, witness, parser = self.present()
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        self.assertEqual(parser.values['bound-claim'], [self.response['prose']])
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertEqual(witness['outputs'][0]['comparisons'][0]['status'], 'verified_pair')
        self.assertEqual(witness['outputs'][0]['reading']['sites'][0]['site'], self.response['primary_sites'][0])
        self.assertLess(body.index('class="prose"'), body.index('class="primary-sites"'))
        self.assertLess(body.index('class="primary-sites"'), body.index('class="secondary"'))
        self.assertIn('Authored reason &lt;tag&gt;', body)
        self.assertTrue(all(link[1:] in parser.anchors for link in parser.links if link.startswith('#')))

    def split_pair(self):
        self.response['prose'] = 'Before value.\nAfter value.'
        self.response['claims'] = [{'start': 0, 'end': 13, 'kind': 'source_fact', 'selections': [1]},
                                   {'start': 14, 'end': 26, 'kind': 'source_fact', 'selections': [0]}]
        self.response['primary_sites'] = [{'selections': [0, 1], 'claims': [0, 1],
                                          'reason': 'Independent before and after statements explain the pair.'}]

    def test_separate_before_after_claims_keep_exact_side_links(self):
        self.split_pair()
        body, witness, parser = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertEqual(witness['outputs'][0]['comparisons'][0]['status'], 'verified_pair')
        self.assertEqual(parser.values['prose'], ['Before value.\nAfter value.'])
        self.assertEqual(parser.values['bound-claim'], ['Before value.', 'After value.'])
        links = ClaimLinks(); links.feed(body)
        self.assertEqual(links.claims, {0: [1], 1: [0]})

    def test_pair_cannot_hide_unbound_side_or_attach_unrelated_claim(self):
        self.split_pair()
        self.response['claims'][1]['selections'] = [1]
        with self.assertRaisesRegex(ValueError, 'unbound primary selection'):
            self.validation()
        self.response['selections'].append(copy.deepcopy(self.response['selections'][0]))
        self.response['claims'][1]['selections'] = [2]
        with self.assertRaisesRegex(ValueError, 'primary claim does not bind site selections'):
            self.validation()

    def test_no_change_context_is_readable_without_diff(self):
        # Independent source classification, never inferred from an empty diff.
        frozen = i.verify(self.h.manifest)
        frozen['entries'][0]['attribution'] = 'pinned_baseline'
        path = self.h.root / 'context.json'; path.write_bytes(i.encoded(frozen))
        self.h.record['input'] = i.binding(path)
        self.response['selections'] = [dict(self.h.selections[0], state='context')]
        self.response['claims'][0]['selections'] = [0]
        self.response['primary_sites'][0]['selections'] = [0]
        body, witness, parser = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertNotIn('diff', parser.values)
        self.assertIn('no diff is inferred', body)

    def test_primary_beyond_secondary_bound_preserves_exact_generator_index(self):
        first = copy.deepcopy(self.response['selections'][0])
        self.response['selections'] = [first for _ in range(140)] + [self.response['selections'][1]]
        self.response['claims'][0]['selections'] = [140]
        self.response['primary_sites'][0]['selections'] = [140]
        body, witness, parser = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertIn('selection-141', body.split('class="primary-sites"', 1)[1].split('</nav>', 1)[0])
        self.assertEqual(len(parser.values['selection-request']), 128)
        self.assertEqual(witness['outputs'][0]['selections'][140]['selection'], self.response['selections'][140])
        self.assertIn('13 selections available only', body)

    def test_secondary_downloads_cannot_consume_primary_byte_reserve(self):
        source = self.h.root / 'large-source'; source.write_bytes(b'x' * 65536)
        value = baseline.spec(source)
        path = self.h.root / 'large-spec.json'; path.write_bytes(i.encoded(value))
        manifest = i.freeze(path, self.h.root / 'large-frozen', self.h.root)
        surface = baseline.Surface(manifest, 'archive_diagnostic', {'reads': 2, 'read_bytes': 100000},
                                   self.h.root / 'large-trace')
        row = surface.call('read', {'id': 'source-0001', 'limit': 65536})
        selection = {'id': 'source-0001', 'start': 0, 'end': 65536,
                     'sha256': row['result']['metadata']['sha256'], 'state': 'context'}
        self.response['selections'] = [selection for _ in range(33)]
        self.response['claims'][0]['selections'] = [32]
        self.response['primary_sites'][0]['selections'] = [32]
        self.h.record.update(input=i.binding(manifest), retrievals=i.binding(surface.trace))
        self.h.host_reads(surface.trace)
        body, witness, parser = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertEqual(len(parser.values['code']), 32)
        self.assertIn('selection-33', body.split('class="primary-sites"', 1)[1].split('</nav>', 1)[0])
        self.assertIn('Source display gap · selection 32', body)

    def test_structurally_valid_helper_does_not_certify_relevance(self):
        body, witness, _ = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['relevance'], 'pending_independent_examination')
        self.assertIn('entailment unassessed', body)

    def test_invalid_indices_claim_span_kind_and_site_fields_fail_closed(self):
        original = copy.deepcopy(self.response)
        bad = []
        for field, value in (('start', True), ('end', 99999), ('kind', 'verified_success'),
                             ('selections', [99]), ('selections', [True]), ('selections', [0, 0])):
            response = copy.deepcopy(original); response['claims'][0][field] = value; bad.append(response)
        for field, value in (('claims', []), ('claims', [99]), ('selections', [99]),
                             ('reason', ''), ('selections', [0, 1, 0])):
            response = copy.deepcopy(original); response['primary_sites'][0][field] = value; bad.append(response)
        response = copy.deepcopy(original); response['primary_sites'] *= 7; bad.append(response)
        response = copy.deepcopy(original); response['claims'][0]['selections'] = [1]; bad.append(response)
        response = copy.deepcopy(original); response['primary_sites'][0]['function'] = 'invented'; bad.append(response)
        for response in bad:
            self.response = response
            with self.assertRaises((ValueError, TypeError)):
                self.validation()
            body, _, parser = self.present()
            self.assertNotIn('class="primary-sites"', body)
            self.assertEqual(parser.values['prose'], [response['prose']])

    def test_multibyte_prose_boundary_is_rejected(self):
        self.response['prose'] = '한 result'
        self.response['claims'][0].update(start=1, end=3)
        with self.assertRaises(UnicodeError):
            self.validation()

    def test_ambiguous_wrong_state_hash_scope_missing_and_unretrieved_are_invalid(self):
        frozen = i.verify(self.h.manifest)
        for change in ('chronology', 'work', 'project', 'unavailable', 'excerpt', 'locator', 'state', 'hash', 'read'):
            spec = copy.deepcopy(frozen); original = copy.deepcopy(self.response)
            trace = None
            if change == 'chronology': spec['entries'][0]['chronology']['state'] = 'ambiguous'
            if change in {'work', 'project'}: spec['entries'][0][change] = 'foreign'
            if change == 'unavailable': spec['entries'][0]['asset'] = None
            if change == 'excerpt': spec['entries'][0].update(representation='bounded_excerpt', file_sha256=None)
            if change == 'locator': spec['entries'][0]['locator'] = 'another-change'
            if change == 'state': self.response['selections'][0]['state'] = 'before'
            if change == 'hash': self.response['selections'][0]['sha256'] = '0' * 64
            if change == 'read': trace = []
            self.assertEqual(self.validation(spec, trace)['reading']['status'], 'invalid', change)
            self.response = original

    def test_non_source_cannot_be_primary_but_original_is_retained(self):
        spec = i.verify(self.h.manifest); spec['entries'][0]['role'] = 'agent_report'
        self.assertEqual(self.validation(spec)['reading']['status'], 'invalid')

    def test_missing_before_stays_explicit_gap(self):
        self.response['selections'] = self.response['selections'][:1]
        self.response['claims'][0]['selections'] = [0]
        self.response['primary_sites'][0]['selections'] = [0]
        body, witness, parser = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertIn('Missing state is not inferred', body)
        self.assertNotIn('diff', parser.values)

    def test_failed_host_join_cannot_supply_primary(self):
        self.h.record['calls'] = []
        body, witness, parser = self.present()
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'invalid')
        self.assertNotIn('class="primary-sites"', body)
        self.assertNotIn('code', parser.values)

    def test_stale_source_withholds_primary_and_preserves_original(self):
        self.h.source.write_text('different bytes\n')
        body, _, parser = self.present()
        self.assertNotIn('class="primary-sites"', body)
        self.assertNotIn('code', parser.values)
        self.assertEqual(parser.values['prose'], [self.response['prose']])


class ReaderExecutionTests(unittest.TestCase):
    def setUp(self):
        self.h = controls.ExploratoryTests(methodName='runTest')
        self.h._testMethodName = self._testMethodName
        self.h.setUp()
        self.addCleanup(self.h.doCleanups)
        self.condition = a.HERE / 'conditions-reader.json'
        self.runtime = {'model': 'authored-local-process', 'reasoning_effort': 'high',
                        'destination': a.DESTINATION, 'authorization': None}

    def test_condition_and_schema_are_generic_and_single_call(self):
        condition = json.loads(self.condition.read_bytes())
        historical = json.loads((a.HERE / 'conditions-exploratory.json').read_bytes())
        self.assertEqual(condition['budgets'], historical['budgets'])
        self.assertEqual(condition['execution'], historical['execution'])
        self.assertEqual(set(condition['conditions']), {'direct'})
        schema = json.loads((a.HERE / 'reader-response-schema.json').read_bytes())
        self.assertEqual(set(schema['required']), {'prose', 'selections', 'gaps', 'claims', 'primary_sites'})
        self.assertEqual(schema['properties']['primary_sites']['maxItems'], 6)
        with self.assertRaisesRegex(ValueError, 'one safety-only generation call'):
            a.attempt(self.h.manifest, 'note_then_prose', 'archive_diagnostic', self.runtime,
                      self.h.root / 'unsupported', conditions_path=self.condition)

    def test_generic_preparation_keeps_original_prose_out_of_fresh_inventory(self):
        from reader_experiment import prepare
        h = baseline.RenderTests(methodName='runTest'); h.setUp()
        self.addCleanup(h.doCleanups)
        h.record.update(conditions=i.binding(a.HERE / 'conditions-exploratory.json'),
                        instructions=i.binding(a.HERE / 'instructions.txt'))
        h.record['initial_input'] = i.binding(h.response_path)  # Deliberately different initial identity.
        h.attempt.write_bytes(i.encoded(h.record))
        # Synthetic baseline host fixture has only stdout; no claimed process receipt.
        h.record['calls'] = []
        h.attempt.write_bytes(i.encoded(h.record))
        with patch.object(a, 'capture', side_effect=AssertionError('plan must never dispatch')):
            plan = prepare([h.attempt], h.root / 'plan', model='explicit-model', effort='high',
                           executable=self.h.executable('ordinary', 0))
        self.assertEqual(plan['model_calls'], 0)
        self.assertEqual(plan['fresh'][0]['state'], 'blocked')
        initial = Path(plan['fresh'][0]['initial_input']['path']).read_text()
        self.assertNotIn(h.response['prose'], initial)
        self.assertNotIn('primary_sites', initial)
        self.assertEqual(plan['originals'][0]['original_outputs'], h.record['original_outputs'])
        self.assertFalse(plan['fresh'][0]['starting_inventory_matches_original'])

    def test_authorization_for_original_condition_cannot_dispatch_reader(self):
        self.runtime['authorization'] = {'current_request_locator': 'authored-only', 'scope': {
            'destination': a.DESTINATION, 'purpose': 'explanation-generation-experiment',
            'input_sha256': i.binding(self.h.manifest)['sha256'], 'lane': 'archive_diagnostic',
            'conditions_sha256': i.binding(a.HERE / 'conditions-exploratory.json')['sha256'],
            'instructions_sha256': i.binding(a.HERE / 'instructions.txt')['sha256'], 'approach': 'direct'}}
        with patch.object(a, 'capture', side_effect=AssertionError('unauthorized dispatch')):
            record = a.attempt(self.h.manifest, 'direct', 'archive_diagnostic', self.runtime,
                               self.h.root / 'blocked', conditions_path=self.condition)
        self.assertEqual(record['status'], 'blocked')
        self.assertEqual(record['calls'], [])
        self.assertEqual(record['output_contract'], CONTRACT)
        self.assertEqual(record['response_schema']['sha256'], record['frozen_response_schema']['sha256'])

    def test_actual_subprocess_output_to_reading_and_all_costs(self):
        executable = self.h.executable('ordinary', 0)
        code = executable.read_text()
        # Authored process output is independent of renderer/validator output.
        code = code.replace("'gaps':['Historical before bytes unavailable.']}",
            "'gaps':['Historical before bytes unavailable.'], 'claims':[{'start':0,'end':27,"
            "'kind':'source_fact','selections':[0]}], 'primary_sites':[{'selections':[0],"
            "'claims':[0],'reason':'Authored code-site reason; no model authorship.'}]}")
        executable.write_text(code)
        self.runtime['authorization'] = {'current_request_locator': 'authored-local-controls-only', 'scope': {
            'destination': a.DESTINATION, 'purpose': 'explanation-generation-experiment',
            'input_sha256': i.binding(self.h.manifest)['sha256'], 'lane': 'archive_diagnostic',
            'conditions_sha256': i.binding(self.condition)['sha256'],
            'instructions_sha256': i.binding(a.HERE / 'reader-instructions.txt')['sha256'], 'approach': 'direct'}}
        output = self.h.root / 'reader-attempt'
        record = a.attempt(self.h.manifest, 'direct', 'archive_diagnostic', self.runtime, output,
                           executable=executable, conditions_path=self.condition)
        self.assertEqual(record['status'], 'captured')
        self.assertEqual(record['resource_accounting']['model_calls'], 1)
        self.assertEqual(record['resource_accounting']['evidence_calls'], 2)
        self.assertEqual(record['evidence_reads']['issues'], [])
        self.assertEqual(record['tokens'], [{'input_tokens': 1, 'output_tokens': 1}])
        self.assertEqual(record['output_bytes'], record['generation_output']['bytes'])
        from render_comparison import card
        raw = Path(record['generation_output']['path']).read_bytes()
        body, witness = card(output / 'attempt.json', 'Authored first attempt')
        parser = baseline.Texts(); parser.feed(body)
        self.assertEqual(parser.values['prose'], [json.loads(raw)['prose']])
        self.assertEqual(witness['outputs'][0]['reading']['status'], 'valid_binding')
        self.assertEqual(Path(record['generation_output']['path']).read_bytes(), raw)
        command = json.loads((output / 'prose/process/command.json').read_bytes())
        self.assertIn(record['frozen_response_schema']['path'], command['argv'])
        self.assertEqual(record['corrections'], [])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--retain', type=Path)
    args, remaining = parser.parse_known_args()
    if args.retain:
        args.retain.mkdir(parents=True, exist_ok=False)
        controls.ARTIFACT_ROOT = args.retain.resolve()
    unittest.main(argv=[__file__, *remaining])
