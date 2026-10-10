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
        response['selections'][0].update(start=56, end=59, sha256=i.digest('한'.encode()[1:] + b'"'))
        response['selections'].append({'id':'source-0001','state':'context','start':47,'end':56,
                                      'sha256':i.digest('return "한'.encode()[:9])})
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        body, _, parser = self.h.present()
        self.assertIn('invalid', body)
        self.assertEqual(len(parser.values['code']), 1)

    def test_large_selected_source_has_a_truthful_display_gap(self):
        _, response = self.fixture('a' * 70000 + '\n')
        body, witness, parser = self.h.present()
        self.assertIn('Source display gap',body)
        self.assertIn('source display byte bound',body)
        self.assertNotIn('code',parser.values)
        self.assertTrue(witness['outputs'][0]['selections'][0]['reference_status'].startswith('valid_reference'))
        self.assertIn(self.h.response_path.read_bytes(),[base64.b64decode(link.split(',',1)[1])
                      for link in parser.links if link.startswith('data:')])

    def test_overbound_transport_is_explicitly_rejected(self):
        self.fixture()
        self.h.response_path.write_bytes(json.dumps({'prose':'x' * (2 * 1024 * 1024), 'selections':[], 'gaps':[]}).encode())
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        with self.assertRaisesRegex(ValueError,'response exceeds local presentation bound'):
            self.h.present()

    def test_repeated_output_identity_has_distinct_exact_local_anchors(self):
        self.fixture()
        self.h.record['original_outputs'] *= 2
        _, _, parser = self.h.present()
        self.assertEqual(len(parser.anchors),len(set(parser.anchors)))
        self.assertTrue(all(link[1:] in parser.anchors for link in parser.links if link.startswith('#')))

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


class ComparisonTests(unittest.TestCase):
    def setUp(self):
        self.h = baseline.RenderTests(methodName='runTest')
        self.h.setUp()
        self.addCleanup(self.h.doCleanups)

    def paired(self, before_text, after_text, *, ranges=None, before_locator=None, excerpt=False, path='src/change.py'):
        after = self.h.root / 'paired-after'; after.write_bytes(after_text.encode())
        before = self.h.root / 'paired-before'; before.write_bytes(before_text.encode())
        value = spec(after)
        entry = value['entries'][0]
        entry.update(path=path, attribution='explicit_work_patch', before_state='available', locator='fixture:change-1')
        old = copy.deepcopy(entry)
        old.update(id='source-0002', attribution='explicit_before_patch', origin=i.binding(before),
                   file_sha256=i.binding(before)['sha256'], locator=before_locator or 'fixture:change-1:before')
        value['entries'].append(old)
        if excerpt:
            for e in value['entries']:
                e.update(representation='bounded_excerpt', file_sha256=None, extent={'coordinates':'unknown'})
        spec_path = self.h.root / 'paired-spec.json'; spec_path.write_bytes(i.encoded(value))
        manifest = i.freeze(spec_path, self.h.root / 'paired-frozen', self.h.root)
        surface = Surface(manifest, 'archive_diagnostic', {'reads':10,'read_bytes':2000000}, self.h.root / 'paired-trace')
        selections = []
        for number, (identity, state, data) in enumerate([('source-0002','before',before.read_bytes()),
                                                          ('source-0001','after',after.read_bytes())]):
            surface.call('read', {'id':identity,'limit':len(data)})
            left, right = ranges[number] if ranges else (0,len(data))
            selections.append({'id':identity,'state':state,'start':left,'end':right,'sha256':i.digest(data[left:right])})
        response = {'prose':'Independent change fixture.','selections':selections,'gaps':[]}
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(input=i.binding(manifest), retrievals=i.binding(surface.trace),
                             original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        self.h.host_reads(surface.trace)
        return response

    def test_same_path_different_changes_never_become_a_code_pair(self):
        self.paired('old bytes\n','new bytes\n', before_locator='fixture:unrelated-change:before')
        body, _, parser = self.h.present()
        self.assertEqual(parser.values['code'], ['old bytes\n','new bytes\n'])
        self.assertNotIn('diff', parser.values)
        self.assertIn('Comparison gap', body)
        self.assertIn('change identity', body)

    def test_incomplete_excerpts_preserve_bytes_without_inventing_comparison(self):
        self.paired('def unrelated():\n    return 1\n','def actual():\n    return 2\n', excerpt=True)
        body, _, parser = self.h.present()
        self.assertEqual(len(parser.values['code']), 2)
        self.assertNotIn('diff', parser.values)
        self.assertIn('whole-file states unavailable', body)

    def test_unedited_unchanged_selection_does_not_display_other_file_changes(self):
        case = CASES['change']; text = case['unchanged_text']
        self.paired(case['before'], case['after'], ranges=[(0,30),(15,45)])
        body, _, parser = self.h.present()
        self.assertEqual(parser.values['code'], [text,text])
        self.assertNotIn('diff', parser.values)
        self.assertIn('Unchanged selected context', body)

    def test_true_pair_uses_whole_file_coordinates_and_original_selection_focus(self):
        case = CASES['change']
        self.paired(case['before'],case['after'],ranges=[(31,59),(46,74)])
        body, _, parser = self.h.present()
        diff = parser.values['diff'][0]
        self.assertIn('-' + case['removed_line'],diff)
        self.assertIn('+' + case['added_line'],diff)
        self.assertNotIn('+# added header',diff)
        self.assertIn('@@ -3,3 +4,3 @@',diff)
        self.assertIn('whole-file states',body)
        self.assertIn('change identity',body)

    def test_addition_deletion_and_exact_block_relocation_are_distinct(self):
        before = 'one\ntwo\nthree\nfour\nfive\nsix\n'
        after = 'three\nfour\nfive\nsix\none\ntwo\n'
        self.paired(before, after, path='src/example.go')
        body, witness, parser = self.h.present()
        self.assertIn('deletions, additions',body)
        self.assertIn('movement of identical bytes',body)
        self.assertIn('-one\n-two\n',parser.values['diff'][0])
        self.assertIn('+one\n+two\n',parser.values['diff'][0])
        self.assertEqual(witness['outputs'][0]['comparisons'][0]['exact_block_relocations'],1)

    def test_crlf_no_final_newline_and_document_comparisons_stay_exact(self):
        self.paired('old\r\nlast','new\r\nlast',path='docs/change.md')
        body, _, parser = self.h.present()
        self.assertIn('docs/change.md',body)
        self.assertIn('-old\r\n+new\r\n',parser.values['diff'][0])
        self.assertIn('\\ No newline at end of file\n',parser.values['diff'][0])
        self.assertEqual(parser.values['code'],['old\r\nlast','new\r\nlast'])

    def test_equal_whole_file_states_show_no_change_without_empty_diff(self):
        self.paired('same\n','same\n')
        body, witness, parser = self.h.present()
        self.assertNotIn('diff',parser.values)
        self.assertIn('Unchanged selected context',body)
        basis = witness['outputs'][0]['comparisons'][0]
        self.assertEqual(basis['status'],'verified_pair')
        self.assertEqual(basis['displayed_hunks'],0)

    def test_multiple_selections_in_one_verified_state_do_not_block_comparison(self):
        response = self.paired('old\n','new\n')
        response['selections'].append(copy.deepcopy(response['selections'][0]))
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        body, _, parser = self.h.present()
        self.assertEqual(len(parser.values['code']),3)
        self.assertEqual(len(parser.values['diff']),1)
        self.assertIn('-old\n+new\n',parser.values['diff'][0])

    def test_changed_frozen_asset_is_rejected_by_locator(self):
        response = self.paired('old\n','new\n')
        frozen = json.loads(Path(self.h.record['input']['path']).read_bytes())
        entry = frozen['entries'][0]
        Path(entry['asset']['path']).write_bytes(b'drift\n')
        with self.assertRaisesRegex(ValueError,'changed bytes'):
            locate(frozen,entry,{'selection':response['selections'][1],'reference_status':'valid_reference'})
        body, _, parser = self.h.present()
        self.assertIn('Input verification gap',body)
        self.assertIn('changed bytes',body)
        self.assertNotIn('code',parser.values)
        self.assertNotIn('diff',parser.values)
        self.assertEqual(parser.values['prose'],[response['prose']])

    def test_ambiguous_before_never_gains_valid_provenance_from_matching_bytes(self):
        response = self.paired('old\n','new\n')
        path = Path(self.h.record['input']['path'])
        frozen = json.loads(path.read_bytes())
        frozen['entries'][1]['chronology']={'state':'ambiguous','reason':'authored missing observation sequence'}
        path.write_bytes(i.encoded(frozen))
        self.h.record['input']=i.binding(path)
        body, witness, parser = self.h.present()
        self.assertEqual(parser.values['code'],['new\n'])
        self.assertNotIn('diff',parser.values)
        self.assertIn('ambiguous chronology',body)
        self.assertTrue(witness['outputs'][0]['selections'][0]['reference_status'].startswith('invalid'))

    def test_missing_source_asset_never_uses_origin_bytes_as_a_substitute(self):
        response = self.paired('old\n','new\n')
        path = Path(self.h.record['input']['path'])
        frozen = json.loads(path.read_bytes())
        frozen['entries'][1]['asset']=None
        path.write_bytes(i.encoded(frozen))
        self.h.record['input']=i.binding(path)
        body, _, parser = self.h.present()
        self.assertEqual(parser.values['code'],['new\n'])
        self.assertNotIn('diff',parser.values)
        self.assertIn('unavailable source',body)
        self.assertEqual(parser.values['prose'],[response['prose']])

    def test_wrong_lane_does_not_disclose_archive_selection_metadata(self):
        self.paired('old\n','new\n')
        self.h.record['lane']='product'
        body, _, parser = self.h.present()
        self.assertNotIn('code',parser.values)
        self.assertNotIn('diff',parser.values)
        self.assertNotIn('fixture:change-1',body)


if __name__ == '__main__':
    unittest.main()
