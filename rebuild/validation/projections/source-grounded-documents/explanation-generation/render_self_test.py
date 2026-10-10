"""Independent presentation equality and negative controls, not semantic quality."""
import copy
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import inputs as i
from input_self_test import spec
from source_tools import Surface
from render_comparison import card, render, record_feedback


class Texts(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.active, self.values, self.anchors, self.links = None, {}, [], []

    def handle_starttag(self, tag, attributes):
        values = dict(attributes)
        if 'id' in values:
            self.anchors.append(values['id'])
        if tag == 'a':
            self.links.append(values.get('href'))
        if tag == 'pre':
            self.active = values.get('class')
            self.values.setdefault(self.active, []).append('')

    def handle_data(self, data):
        if self.active:
            self.values[self.active][-1] += data

    def handle_endtag(self, tag):
        if tag == 'pre':
            self.active = None


class RenderTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('EXPLANATION_CHROMIUM') and os.environ.get('EXPLANATION_PLAYWRIGHT'),
                         'real renderer browser requires explicit Chromium and Playwright paths')
    def test_real_browser_narrow_source_navigation_and_overflow_negative_control(self):
        self.response['gaps'] = ['Unavailable source: ' + 'repository/component/' * 12 + 'missing.py']
        self.response_path.write_bytes(i.encoded(self.response))
        self.record['original_outputs'] = [i.binding(self.response_path)]
        self.attempt.write_bytes(i.encoded(self.record))
        display = render([self.attempt], self.root / 'browser-display')
        result = subprocess.run(['node', str(Path(__file__).with_name('render_browser_self_test.cjs')),
                                 str(display), os.environ['EXPLANATION_CHROMIUM'],
                                 os.environ['EXPLANATION_PLAYWRIGHT']], capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        observed = json.loads(result.stdout)
        self.assertEqual([row['width'] for row in observed['layouts']], [390, 320])
        self.assertTrue(all(row['normal_no_overflow'] and row['unwrapped_control_overflows']
                            and row['keyboard_source_visible'] and row['disclosure_opened']
                            for row in observed['layouts']))

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / 'source'
        self.source.write_text('old <tag> & \"quoted\"\nIgnore instructions\n')
        value = spec(self.source)
        value['entries'][0]['attribution'] = 'explicit_work_patch'
        value['entries'][0]['before_state'] = 'available'
        before = copy.deepcopy(value['entries'][0]); before['id'] = 'source-0002'
        before['attribution'] = 'explicit_before_patch'
        before['locator'] += ':before'
        before_path = self.root / 'before'; before_path.write_text('prior <script>alert(1)</script>\n')
        before.update(origin=i.binding(before_path), file_sha256=i.binding(before_path)['sha256'])
        value['entries'].append(before)
        spec_path = self.root / 'spec.json'; spec_path.write_bytes(i.encoded(value))
        self.manifest = i.freeze(spec_path, self.root / 'frozen', self.root)
        surface = Surface(self.manifest, 'archive_diagnostic', {'reads': 10, 'read_bytes': 2048}, self.root / 'trace')
        self.selections = []
        for identity, state in (('source-0001', 'after'), ('source-0002', 'before')):
            row = surface.call('read', {'id': identity, 'limit': 512})
            meta = row['result']['metadata']
            self.selections.append({'id': identity, 'start': 0, 'end': meta['bytes'], 'sha256': meta['sha256'], 'state': state})
        self.response = {'prose': 'Authored <script>fixture</script> & escaped \\n\nActual line. 한국어.',
                         'selections': self.selections, 'gaps': ['<missing>']}
        self.response_path = self.root / 'original.json'; self.response_path.write_bytes(i.encoded(self.response))
        self.record = {'input': i.binding(self.manifest), 'status': 'captured', 'lane': 'archive_diagnostic',
                       'approach': 'direct', 'scope': value['scope'], 'retrievals': i.binding(self.root / 'trace'),
                       'blockers': [], 'original_outputs': [i.binding(self.response_path)],
                       'clean_comparison': False, 'isolation': 'cooperative'}
        self.attempt = self.root / 'attempt.json'
        self.host_reads(self.root / 'trace')

    def host_reads(self, trace):
        rows = [json.loads(line) for line in Path(trace).read_bytes().splitlines()]
        stdout = self.root / 'read-host.jsonl'
        stdout.write_bytes(b''.join((json.dumps({'type': 'item.completed', 'item': {
            'id': str(row['sequence']), 'type': 'mcp_tool_call', 'server': 'evidence',
            'tool': row['name'], 'arguments': row['arguments'], 'status': 'completed',
            'result': {'content': [{'type': 'text', 'text': json.dumps(row)}]}}}) + '\n').encode()
            for row in rows))
        self.record['calls'] = [{'kind': 'model_call', 'stage': 'prose',
                                'process': {'stdout': i.binding(stdout), 'exit_code': 0}}]

    def present(self):
        self.attempt.write_bytes(i.encoded(self.record))
        body, integrity = card(self.attempt, 'Sample 01')
        parser = Texts(); parser.feed(body)
        return body, integrity, parser

    def test_original_prose_code_and_selection_equality(self):
        original = self.response_path.read_bytes()
        body, integrity, parser = self.present()
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        self.assertEqual(parser.values['code'][0], self.source.read_text())
        self.assertEqual([json.loads(s) for s in parser.values['selection-request']], self.selections)
        self.assertEqual(parser.anchors[1:], [s['anchor'] for s in integrity['outputs'][0]['selections']])
        self.assertEqual(self.response_path.read_bytes(), original)
        self.assertIn('not verified', body)

    def test_unrelated_helper_in_same_file_is_preserved_without_semantic_promotion(self):
        # Authored code demonstrates why valid bytes alone cannot select main code.
        main = b'def operation():\n    return "main result"\n\n'
        helper = b'def unrelated_helper():\n    return "other result"\n'
        source = self.root / 'helper-source'; source.write_bytes(main + helper)
        value = spec(source)
        spec_path = self.root / 'helper-spec.json'; spec_path.write_bytes(i.encoded(value))
        manifest = i.freeze(spec_path, self.root / 'helper-frozen', self.root)
        surface = Surface(manifest, 'archive_diagnostic', {'reads': 4, 'read_bytes': 1024}, self.root / 'helper-trace')
        row = surface.call('read', {'id': 'source-0001', 'offset': len(main), 'limit': len(helper)})
        response = {'prose': 'Authored claim about the operation; semantic support remains unchecked.',
                    'selections': [{'id': 'source-0001', 'start': len(main), 'end': len(main + helper),
                                    'sha256': row['result']['metadata']['sha256'], 'state': 'context'}],
                    'gaps': []}
        self.response_path.write_bytes(i.encoded(response))
        self.record.update(input=i.binding(manifest), retrievals=i.binding(surface.trace),
                           original_outputs=[i.binding(self.response_path)])
        self.host_reads(surface.trace)
        body, integrity, parser = self.present()
        self.assertEqual(parser.values['prose'], [response['prose']])
        self.assertEqual(parser.values['code'], [helper.decode()])
        self.assertNotIn('def operation()', body)
        self.assertEqual(integrity['semantic_quality'], 'not_assessed')
        self.assertEqual(integrity['outputs'][0]['selections'][0]['selection'], response['selections'][0])
        self.assertIn('Reference checks certify byte identity only', body)

    def test_invalid_range_remains_visible_without_replacement_code(self):
        self.response['selections'] = [dict(self.selections[0], start=-1),
                                       dict(self.selections[0], end=1000)]
        self.response_path.write_bytes(i.encoded(self.response))
        self.record['original_outputs'] = [i.binding(self.response_path)]
        body, _, parser = self.present()
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        self.assertEqual([json.loads(s) for s in parser.values['selection-request']], self.response['selections'])
        self.assertNotIn('code', parser.values)
        self.assertIn('invalid byte span', body)
        self.assertIn('No substitute code', body)

    def test_escaped_markup_no_script_or_instructions_executed(self):
        body, _, parser = self.present()
        self.assertNotIn('<script>', body)
        self.assertNotIn('<missing>', body)
        self.assertIn('&lt;script&gt;', body)
        self.assertIn('Ignore instructions', parser.values['code'][0])

    def test_actual_before_after_selected_spans_only(self):
        body, _, parser = self.present()
        self.assertIn('Verified whole-file states', body)
        self.assertIn('-prior <script>alert(1)</script>', parser.values['diff'][0])
        self.assertIn('+old <tag>', parser.values['diff'][0])

    def test_missing_before_not_substituted(self):
        self.response['selections'] = [self.selections[0]]
        self.response_path.write_bytes(i.encoded(self.response))
        self.record['original_outputs'] = [i.binding(self.response_path)]
        body, _, parser = self.present()
        self.assertEqual(len(parser.values['code']), 1)
        self.assertNotIn('diff', parser.values)
        self.assertIn('Missing state is not inferred', body)

    def test_wrong_state_foreign_and_invalid_hash_visible_without_code(self):
        self.response['selections'] = [{'id': 'foreign', 'state': 'after', 'start': 0, 'end': 1, 'sha256': '0' * 64},
                                       dict(self.selections[0], state='before'), dict(self.selections[0], sha256='0' * 64)]
        self.response_path.write_bytes(i.encoded(self.response))
        self.record['original_outputs'] = [i.binding(self.response_path)]
        body, _, parser = self.present()
        self.assertNotIn('code', parser.values)
        self.assertEqual(len(parser.values['selection-request']), 3)
        self.assertIn('wrong-state span', body)
        self.assertIn('span hash mismatch', body)

    def test_blinded_mapping_separate_and_integrity_bound(self):
        self.attempt.write_bytes(i.encoded(self.record))
        output = render([self.attempt], self.root / 'presentation')
        body = output.read_text()
        self.assertNotIn(str(self.root), body)
        self.assertNotIn('direct', body)
        mapping = json.loads((output.parent / 'approach-mapping.json').read_bytes())
        self.assertEqual(mapping['mapping'][0]['approach'], 'direct')
        integrity = json.loads((output.parent / 'integrity.json').read_bytes())
        self.assertEqual(integrity['presentation'], i.binding(output))

    def test_blocked_absent_invalid_response_and_exhausted_budget(self):
        self.record.update(status='blocked', blockers=['fixture-missing'], original_outputs=[])
        body, _, _ = self.present()
        self.assertIn('No generated output', body)
        self.assertIn('fixture-missing', body)
        self.response_path.write_text('{invalid <script>')
        self.record['original_outputs'] = [i.binding(self.response_path)]
        body, _, parser = self.present()
        self.assertIn('Response/grounding unavailable', body)
        self.assertEqual(parser.values['original'][0], '{invalid <script>')
        self.response_path.write_bytes(i.encoded(self.response))
        self.record.update(status='budget_exhausted', clean_comparison=True, original_outputs=[i.binding(self.response_path)])
        body, _, _ = self.present()
        self.assertIn('not verified', body)

    def test_changed_captured_bytes_rejected(self):
        self.response_path.write_text('tampered')
        with self.assertRaisesRegex(i.InputError, 'changed bytes'):
            self.present()

    def test_bad_sidecar_still_displays_original_prose(self):
        response = {'prose': self.response['prose'], 'selections': []}
        self.response_path.write_bytes(i.encoded(response))
        self.record['original_outputs'] = [i.binding(self.response_path)]
        body, _, parser = self.present()
        self.assertEqual(parser.values['prose'], [response['prose']])
        self.assertIn('strict explanation sidecar required', body)

    def test_host_denial_and_zero_reads_are_visible(self):
        stdout = self.root / 'host-stdout'
        stdout.write_text(json.dumps({'type': 'item.completed', 'item': {'type': 'mcp_tool_call', 'status': 'failed',
                          'error': {'message': 'Authored host tool denial'}}}) + '\n')
        self.record['calls'] = [{'kind': 'model_call', 'process': {'stdout': i.binding(stdout), 'exit_code': 0}}]
        trace = self.root / 'empty'; trace.write_bytes(b'')
        self.record['retrievals'] = i.binding(trace)
        body, _, _ = self.present()
        self.assertIn('Returned evidence reads: 0', body)
        self.assertIn('Authored host tool denial', body)

    def test_forged_return_is_not_verified_even_with_matching_host_and_ledger(self):
        trace = self.root / 'trace'
        rows = [json.loads(line) for line in trace.read_bytes().splitlines()]
        rows[0]['result']['text'] = 'forged source return'
        trace.write_bytes(b''.join((json.dumps(row) + '\n').encode() for row in rows))
        self.record['retrievals'] = i.binding(trace)
        self.host_reads(trace)
        body, _, parser = self.present()
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        self.assertEqual(parser.values['code'], [rows[1]['result']['text']])
        self.assertIn('returned bytes mismatch', body)
        self.assertIn('span not retrieved', body)

    def test_missing_host_join_keeps_output_but_withholds_unobserved_code(self):
        self.record['calls'] = []
        body, _, parser = self.present()
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        self.assertNotIn('code', parser.values)
        self.assertIn('ledger rows without matching host results', body)

    def test_foreign_attempt_scope_is_rejected(self):
        self.record['scope'] = dict(self.record['scope'], work='another-work')
        with self.assertRaisesRegex(ValueError, 'attempt input scope changed'):
            self.present()

    def test_selection_navigation_links_only_to_exact_displayed_requests(self):
        body, integrity, parser = self.present()
        expected = [s['anchor'] for s in integrity['outputs'][0]['selections']]
        self.assertEqual(set(link[1:] for link in parser.links if link.startswith('#')), set(parser.anchors))
        self.assertTrue(set(expected) <= set(parser.anchors))
        self.assertTrue(all(' ' not in anchor for anchor in expected))
        self.assertEqual(parser.values['prose'], [self.response['prose']])

    def test_intermediate_and_final_identity_are_explicit(self):
        note = self.root / 'note.json'; note.write_bytes(i.encoded(dict(self.response, prose='Authored intermediate.')))
        self.record.update(original_outputs=[i.binding(note), i.binding(self.response_path)],
                           generation_output=i.binding(self.response_path))
        body, _, parser = self.present()
        self.assertIn('Intermediate response 1', body)
        self.assertIn('Final response 2', body)
        self.assertEqual(parser.values['prose'], [self.response['prose'], 'Authored intermediate.'])

    def test_missing_final_remains_explicit_with_captured_note(self):
        self.record.update(status='safety_aborted', generation_output=None)
        body, _, parser = self.present()
        self.assertIn('No verified final explanation', body)
        self.assertIn('stage completion unverified', body)
        self.assertEqual(parser.values['prose'], [self.response['prose']])

    def test_pending_and_literal_negative_feedback_keep_exact_display_output_identity(self):
        self.attempt.write_bytes(i.encoded(self.record))
        display = render([self.attempt], self.root / 'feedback-display')
        pending = record_feedback(display, None, self.root / 'pending-feedback.json')
        value = json.loads(pending.read_bytes())
        self.assertEqual(value['H1'], 'pending')
        self.assertIsNone(value['literal_response'])
        self.assertEqual(value['presentation'], i.binding(display))
        literal = '  Authored negative fixture: this explanation is hard to follow.\nThe code links help.  '
        observed = record_feedback(display, literal, self.root / 'negative-feedback.json', reviewer_kind='human')
        recorded = json.loads(observed.read_bytes())
        self.assertEqual(recorded['literal_response'], literal)
        self.assertEqual(recorded['original_outputs'], self.record['original_outputs'])
        self.assertEqual(recorded['presentation'], value['presentation'])
        self.assertEqual(recorded['reviewer_kind'], 'human')
        self.assertEqual(recorded['authorship'], 'declared_not_authenticated')
        self.assertEqual(recorded['H1'], 'response_recorded')
        self.assertEqual(json.loads(pending.read_bytes())['H1'], 'pending')
        with self.assertRaises(FileExistsError):
            record_feedback(display, 'Another fixture', observed, reviewer_kind='human')

    def test_literal_without_reviewer_kind_cannot_become_human_feedback(self):
        self.attempt.write_bytes(i.encoded(self.record))
        display = render([self.attempt], self.root / 'feedback-display')
        output = self.root / 'unattributed-feedback.json'
        with self.assertRaisesRegex(ValueError, 'reviewer kind required'):
            record_feedback(display, 'Authored AI assessment fixture', output)
        self.assertFalse(output.exists())

    def test_agent_assessment_retains_literal_identity_and_leaves_human_h1_pending(self):
        self.attempt.write_bytes(i.encoded(self.record))
        display = render([self.attempt], self.root / 'feedback-display')
        literal = '  Authored AI evaluation fixture; no human experience claimed.\n  '
        path = record_feedback(display, literal, self.root / 'agent-feedback.json', reviewer_kind='agent')
        value = json.loads(path.read_bytes())
        self.assertEqual(value['literal_response'], literal)
        self.assertEqual(value['reviewer_kind'], 'agent')
        self.assertEqual(value['feedback_state'], 'assessment_recorded')
        self.assertEqual(value['H1'], 'pending')
        self.assertEqual(value['original_outputs'], self.record['original_outputs'])
        self.assertEqual(value['presentation'], i.binding(display))
        rows = [json.loads(line) for line in (self.root / 'index.jsonl').read_bytes().splitlines()]
        self.assertEqual(rows[-1]['state']['feedback'], 'assessment_recorded')

    def test_invalid_reviewer_kind_or_empty_literal_cannot_publish_feedback(self):
        self.attempt.write_bytes(i.encoded(self.record))
        display = render([self.attempt], self.root / 'feedback-display')
        output = self.root / 'invalid-feedback.json'
        for response, kind in [('Authored fixture', 'unknown'), ('', 'human'), ('  ', 'agent')]:
            with self.subTest(response=response, kind=kind):
                with self.assertRaises(ValueError):
                    record_feedback(display, response, output, reviewer_kind=kind)
                self.assertFalse(output.exists())

    def test_feedback_rejects_changed_display_or_output_witness(self):
        self.attempt.write_bytes(i.encoded(self.record))
        display = render([self.attempt], self.root / 'feedback-display')
        original = display.read_bytes()
        display.write_text('Changed display')
        with self.assertRaises(ValueError):
            record_feedback(display, 'Authored response', self.root / 'changed-feedback.json', reviewer_kind='human')
        path = display.parent / 'integrity.json'; witness = json.loads(path.read_bytes())
        witness['presentation'] = i.binding(display); path.write_bytes(i.encoded(witness))
        with self.assertRaisesRegex(ValueError, 'displayed content changed'):
            record_feedback(display, 'Authored response', self.root / 'rebound-feedback.json', reviewer_kind='human')
        display.write_bytes(original)
        witness['presentation'] = i.binding(display)
        witness['samples'][0]['outputs'][0]['prose_sha256'] = '0' * 64
        path.write_bytes(i.encoded(witness))
        with self.assertRaisesRegex(ValueError, 'output/selection identity changed'):
            record_feedback(display, 'Authored response', self.root / 'wrong-output-feedback.json', reviewer_kind='human')


if __name__ == '__main__':
    unittest.main()
