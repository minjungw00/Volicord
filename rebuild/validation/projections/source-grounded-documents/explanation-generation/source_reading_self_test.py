"""Consumer expectations authored independently of locator/render output."""
import base64
import copy
from html.parser import HTMLParser
import json
from pathlib import Path
import unittest

import inputs as i
from input_self_test import spec
import render_self_test as baseline
from source_reading import locate, position
from source_tools import Surface

CASES = json.loads((Path(__file__).parent / 'reading-fixtures/cases.json').read_bytes())


class Structure(HTMLParser):
    def __init__(self):
        super().__init__()
        self.tags, self.details, self.labels = [], [], []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        self.tags.append(tag)
        if tag == 'details':
            self.details.append(attrs)
        if tag == 'a' and 'aria-label' in attrs:
            self.labels.append(attrs['aria-label'])


class ReadingTests(unittest.TestCase):
    def setUp(self):
        self.h = baseline.RenderTests(methodName='runTest')
        self.h.setUp()
        self.addCleanup(self.h.doCleanups)

    def fixture(self, text=None, representation='full_file', extent=None):
        case = CASES['unicode_crlf']
        source = self.h.root / 'independent-source'
        source.write_bytes((text if text is not None else case['text']).encode())
        value = spec(source)
        value['entries'][0].update(path=case['path'], representation=representation, extent=extent)
        if representation == 'bounded_excerpt':
            value['entries'][0]['file_sha256'] = None
        path = self.h.root / 'independent-spec.json'; path.write_bytes(i.encoded(value))
        manifest = i.freeze(path, self.h.root / 'independent-frozen', self.h.root)
        surface = Surface(manifest, 'archive_diagnostic', {'reads': 4, 'read_bytes': 1000000}, self.h.root / 'independent-trace')
        surface.call('read', {'id': 'source-0001', 'limit': source.stat().st_size})
        spans = case['spans'] if text is None else [{'start': 0, 'end': source.stat().st_size}]
        selections = [{'id': 'source-0001', 'state': 'context', 'start': s['start'], 'end': s['end'],
                       'sha256': i.digest(source.read_bytes()[s['start']:s['end']])} for s in spans]
        response = {'prose': 'An authored statement.〔source-0001 [47,59)〕\r\n\r\nSecond paragraph.',
                    'selections': selections, 'gaps': []}
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(input=i.binding(manifest), retrievals=i.binding(surface.trace),
                             original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        self.h.host_reads(surface.trace)
        return i.verify(manifest), response

    def test_independent_utf8_crlf_coordinates_exact_bytes_and_names(self):
        frozen, response = self.fixture()
        entry = frozen['entries'][0]
        for expected, selection in zip(CASES['unicode_crlf']['spans'], response['selections']):
            location = locate(frozen, entry, {'selection': selection, 'reference_status': 'valid_reference'})
            self.assertEqual(location['text'], expected['text'])
            self.assertEqual(location['start'], expected['location_start'])
            self.assertEqual(location['end'], expected['location_end'])
            self.assertEqual(location['callable']['name'], expected['callable'])
            self.assertEqual([location['callable']['start'], location['callable']['end']], expected['callable_extent'])
        body, witness, parser = self.h.present()
        self.assertEqual(parser.values['prose'], [response['prose']])
        self.assertEqual(parser.values['code'], [s['text'] for s in CASES['unicode_crlf']['spans']])
        self.assertIn('src/example.py · context · 4:9–4:19 · Box.same', body)
        struct = Structure(); struct.feed(body)
        self.assertTrue(struct.labels)
        self.assertNotIn('script', struct.tags)
        self.assertNotIn('form', struct.tags)
        self.assertTrue(all('open' not in attrs for attrs in struct.details))
        self.assertEqual(struct.tags.count('details'), struct.tags.count('summary'))
        targets = set(parser.anchors)
        self.assertTrue(all(link[1:] in targets for link in parser.links if link.startswith('#')))
        originals = [link for link in parser.links if link.startswith('data:')]
        self.assertIn(self.h.response_path.read_bytes(), [base64.b64decode(link.split(',', 1)[1]) for link in originals])

    def test_excerpt_claimed_first_line_never_becomes_absolute_or_callable(self):
        frozen, response = self.fixture('    return "한"\r\n', 'bounded_excerpt', {'first_line': 400, 'last_line': 400})
        location = locate(frozen, frozen['entries'][0], {'selection': response['selections'][0], 'reference_status': 'valid_reference'})
        self.assertIsNone(location['start']); self.assertIsNone(location['callable'])
        body, _, parser = self.h.present()
        self.assertIn('absolute lines unavailable', body)
        self.assertNotIn('400:', body)
        self.assertEqual(parser.values['code'], ['    return "한"\r\n'])

    def test_work_scoped_stable_anchors_and_repeated_selection(self):
        _, response = self.fixture()
        response['selections'].append(copy.deepcopy(response['selections'][0]))
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        _, witness, parser = self.h.present()
        from render_comparison import card
        _, other = card(self.h.attempt, 'Sample 99')
        self.assertEqual(witness['outputs'], other['outputs'])
        self.assertEqual(len(parser.anchors), len(set(parser.anchors)))
        self.assertEqual(len(parser.values['selection-request']), 3)
        self.assertTrue(all('work-' in target for target in parser.anchors))

    def test_split_unicode_boundary_wrong_state_missing_source_and_foreign_scope(self):
        frozen, response = self.fixture()
        entry = frozen['entries'][0]; selection = response['selections'][0]
        invalids = [(dict(entry, work='foreign'), selection, 'foreign Work'),
                    (dict(entry, project='foreign'), selection, 'foreign Work'),
                    (dict(entry, asset=None), selection, 'unavailable source'),
                    (entry, dict(selection, state='before'), 'wrong-state'),
                    (entry, dict(selection, sha256='0'*64), 'span hash mismatch')]
        for bad_entry, bad_selection, message in invalids:
            with self.assertRaisesRegex(ValueError, message):
                locate(frozen, bad_entry, {'selection': bad_selection, 'reference_status': 'valid_reference'})
        self.assertEqual(position(CASES['unicode_crlf']['text'].encode(), 60), [4, 20])
        response['selections'][0].update(start=57, end=59, sha256=i.digest('한'.encode()[1:] + b'"'))
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        body, _, parser = self.h.present()
        self.assertIn('invalid', body)
        self.assertEqual(len(parser.values['code']), 1)

    def test_bounded_display_preserves_complete_transport_and_exact_omission(self):
        _, response = self.fixture()
        response['prose'] = 'Authored large fixture. ' * 2000
        response['selections'] = [response['selections'][0]] * 160
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        body, witness, parser = self.h.present()
        self.assertLess(len(body.encode()), 400000)
        self.assertIn('32 selections available only', body)
        self.assertEqual(len(witness['outputs'][0]['selections']), 160)
        self.assertEqual(len(parser.values['selection-request']), 128)
        self.assertIn(self.h.response_path.read_bytes(), [base64.b64decode(link.split(',', 1)[1])
                      for link in parser.links if link.startswith('data:')])

    def test_malformed_selection_and_surrogate_prose_remain_diagnostic(self):
        self.fixture()
        for response in [{'prose': 'Original.', 'selections': [None, {'id': [], 'start': 0, 'end': 1}], 'gaps': []},
                         {'prose': '\ud800', 'selections': [], 'gaps': []}, []]:
            self.h.response_path.write_bytes(json.dumps(response).encode())
            self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
            body, _, parser = self.h.present()
            self.assertNotIn('code', parser.values)
            self.assertIn('diagnostic', body)


if __name__ == '__main__':
    unittest.main()
