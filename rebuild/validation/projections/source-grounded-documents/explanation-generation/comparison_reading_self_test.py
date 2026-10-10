"""Independently authored compact diff expectations and primary scope controls."""
import base64
import copy
import json
import unittest

import inputs as i
import source_reading_self_test as source_controls
from render_comparison import card


class ComparisonNavigationTests(unittest.TestCase):
    def setUp(self):
        self.fixture = source_controls.ComparisonTests(methodName='runTest')
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.h = self.fixture.h

    def test_long_insertion_has_bounded_source_order_hunks_exact_coordinates_and_lossless_remainder(self):
        old = 'header\nfooter\n'
        added = ''.join('added_' + str(n).zfill(2) + '\n' for n in range(1, 51))
        new = 'header\n' + added + 'footer\n'
        self.fixture.paired(old, new, path='src/authored.ts')
        body, witness, parser = self.h.present()
        comparison = witness['outputs'][0]['comparisons'][0]
        self.assertEqual((comparison['displayed_hunks'], comparison['remaining_hunks'], comparison['navigation_hunks']), (3, 2, 5))
        self.assertIn('3 of 5 line hunks shown in source order', body)
        self.assertIn('Remaining 2 change hunks', body)
        self.assertEqual(len(parser.values['diff']), 5)
        self.assertIn('@@ -1,1 +1,13 @@\n header\n+added_01\n', parser.values['diff'][0])
        self.assertIn('@@ -1,0 +14,12 @@\n+added_13\n', parser.values['diff'][1])
        self.assertIn('@@ -2,1 +50,3 @@\n+added_49\n+added_50\n footer\n', parser.values['diff'][4])
        changes = [line for hunk in parser.values['diff'] for line in hunk.splitlines()
                   if line.startswith('+') and not line.startswith('+++')]
        self.assertEqual(changes, ['+added_' + str(n).zfill(2) for n in range(1, 51)])
        downloads = [base64.b64decode(link.split(',', 1)[1]).decode(errors='replace') for link in parser.links if link.startswith('data:')]
        self.assertTrue(any('@@ -1,2 +1,52 @@' in data and '+added_50\n' in data for data in downloads))
        _, other = card(self.h.attempt, 'Sample 99')
        self.assertEqual(witness['outputs'], other['outputs'])
        self.assertEqual(len(parser.anchors), len(set(parser.anchors)))
        self.assertTrue(all(link[1:] in parser.anchors for link in parser.links if link.startswith('#')))

    def test_empty_before_interval_at_start_has_correct_zero_count_header(self):
        self.fixture.paired('tail\n', 'new\ntail\n', ranges=[(0, 5), (0, 4)])
        _, _, parser = self.h.present()
        self.assertIn('@@ -1,1 +1,2 @@\n+new\n tail\n', parser.values['diff'][0])
        # No context at all gives the standard zero-count insertion position.
        from source_reading import diff_text
        self.assertEqual(diff_text('a', [[('insert', 0, 0, 0, 1)]], [], ['new\n']),
                         '--- a (before)\n+++ a (after)\n@@ -0,0 +1,1 @@\n+new\n')

    def test_long_replacement_slices_cover_every_original_changed_line_once(self):
        old = ''.join('old_' + str(n).zfill(2) + '\n' for n in range(1, 41))
        new = ''.join('new_' + str(n).zfill(2) + '\n' for n in range(1, 41))
        self.fixture.paired(old, new)
        _, witness, parser = self.h.present()
        self.assertEqual(witness['outputs'][0]['comparisons'][0]['navigation_hunks'], 7)
        self.assertIn('@@ -1,6 +1,6 @@', parser.values['diff'][0])
        self.assertIn('@@ -37,4 +37,4 @@', parser.values['diff'][-1])
        removed, added = [], []
        for text in parser.values['diff']:
            changes = [line for line in text.splitlines() if line[:1] in {'-', '+'}
                       and not line.startswith(('---', '+++'))]
            self.assertLessEqual(len(changes), 12)
            removed.extend(line[1:] for line in changes if line.startswith('-'))
            added.extend(line[1:] for line in changes if line.startswith('+'))
        self.assertEqual(removed, old.splitlines())
        self.assertEqual(added, new.splitlines())

    def test_many_short_edits_do_not_accumulate_large_ordinary_context(self):
        before, after = [], []
        for n in range(8):
            before.append('old_' + str(n) + '\n')
            after.append('new_' + str(n) + '\n')
            for line in range(3):
                shared = 'context_' + str(n) + '_' + str(line) + '\n'
                before.append(shared); after.append(shared)
        self.fixture.paired(''.join(before), ''.join(after))
        _, _, parser = self.h.present()
        for text in parser.values['diff']:
            rows = [line for line in text.splitlines() if not line.startswith(('---', '+++', '@@'))]
            self.assertLessEqual(len(rows), 16)

    def test_unchanged_primary_does_not_promote_changed_helper_from_file_grouping(self):
        before = 'def operation():\n    return 1\n\ndef unrelated_helper():\n    return 2\n'
        after = 'def operation():\n    return 1\n\ndef unrelated_helper():\n    return 99\n'
        selected = len('def operation():\n    return 1\n')
        response = self.fixture.paired(before, after, ranges=[(0, selected), (0, selected)])
        response.update(claims=[{'start': 0, 'end': len(response['prose'].encode()),
                                'kind': 'model_interpretation', 'selections': [0, 1]}],
                        primary_sites=[{'selections': [0, 1], 'claims': [0], 'reason': 'Operation only; authored selection.'}])
        # Secondary evidence in the same file cannot add primary focus.
        response['selections'].append(dict(response['selections'][1], start=selected,
                                           end=len(after.encode()), sha256=i.digest(after.encode()[selected:])))
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(output_contract='reader_oriented', original_outputs=[i.binding(self.h.response_path)],
                             generation_output=i.binding(self.h.response_path))
        body, witness, parser = self.h.present()
        primary = witness['outputs'][0]['primary_comparisons'][0]['comparisons'][0]
        self.assertEqual(primary['navigation_hunks'], 0)
        self.assertNotIn('diff', parser.values)
        self.assertIn('No changes within the selected line ranges', body)
        self.assertIn('unselected edits acquire no primary claim relevance', body)
        self.assertEqual(parser.values['prose'], [response['prose']])
        self.assertEqual(witness['outputs'][0]['primary_comparisons'][0]['site'], response['primary_sites'][0])

    def test_primary_site_order_and_original_spans_survive_navigation(self):
        response = self.fixture.paired('old\n', 'new\n')
        original = copy.deepcopy(response['selections'])
        response.update(claims=[{'start': 0, 'end': len(response['prose'].encode()),
                                'kind': 'source_fact', 'selections': [0, 1]}],
                        primary_sites=[{'selections': [1], 'claims': [0], 'reason': 'After first.'},
                                       {'selections': [0], 'claims': [0], 'reason': 'Before second.'}])
        self.h.response_path.write_bytes(i.encoded(response))
        self.h.record.update(output_contract='reader_oriented', original_outputs=[i.binding(self.h.response_path)],
                             generation_output=i.binding(self.h.response_path))
        body, witness, _ = self.h.present()
        self.assertLess(body.index('After first.'), body.index('Before second.'))
        self.assertEqual([s['selection'] for s in witness['outputs'][0]['selections']], original)
        self.assertTrue(all(c['comparisons'][0]['status'] == 'gap' for c in witness['outputs'][0]['primary_comparisons']))
        self.assertIn('one independently verified before and after', body)


if __name__ == '__main__':
    unittest.main()
