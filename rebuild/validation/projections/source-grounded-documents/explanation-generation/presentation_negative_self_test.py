"""Independent hostile display/editorial controls; no semantic or human verdict.

Reuse only disposable Source/attempt setup. Expected prose, locations, mutations
and rejection reasons are authored here, independently of presentation helpers.
"""
import copy
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import unittest

import inputs as i
from editorial_reading import read
import problem_reading_self_test as fixture_support
from render_comparison import card, record_feedback, verify_presentation
from render_readings import compare


class ReadingText(HTMLParser):
    def __init__(self):
        super().__init__()
        self.active = False
        self.paragraphs = []

    def handle_starttag(self, tag, attrs):
        if tag == 'pre' and dict(attrs).get('class') in {'prose', 'prose-paragraph'}:
            self.active = True
            self.paragraphs.append('')

    def handle_data(self, text):
        if self.active:
            self.paragraphs[-1] += text

    def handle_endtag(self, tag):
        if tag == 'pre':
            self.active = False


class PresentationNegativeTests(unittest.TestCase):
    def setUp(self):
        fixture = fixture_support.ProblemReadingTests('runTest')
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        self.fixture, self.h = fixture, fixture.h
        self.parts = [
            '이전 반환 값은 1이다. The old value is 1.\n\n',
            '두 반환 식을 비교한다. Compare the recorded expressions.\n\n',
            '현재 반환 값은 2이다. The new value is 2.\n\n',
            '이 설명은 새로운 검증이나 사용자 수락을 보고하지 않는다.',
        ]
        self.original_prose = ''.join(self.parts)
        response = fixture.response
        response['prose'] = self.original_prose
        response['claims'] = []
        offset = 0
        for number, part in enumerate(self.parts):
            response['claims'].append({
                'start': offset, 'end': offset + len(part.rstrip().encode()),
                'kind': 'source_fact',
                'selections': [0] if number == 0 else [1] if number == 2 else [0, 1],
                'authority': None,
            })
            offset += len(part.encode())
        response['primary_sites'][0].update(
            claims=[0, 1, 2, 3], reason='Compare only the two recorded return expressions.')
        response['gaps'] = ['No new command execution or user acceptance is supplied by this fixture.']
        fixture.freeze_language('ko')
        fixture.sync()
        self.original_response = self.h.response_path.read_bytes()

    def comparison(self, name, *, editorial=False):
        root = self.h.root / name
        if editorial:
            compare(self.h.attempt, root, editorial_order=[0, 2, 1, 3],
                    editor='Independently authored test editor',
                    reason='Compare the observed values before explaining the paired expressions.')
        else:
            compare(self.h.attempt, root)
        return root

    def assert_prose(self, display, expected):
        parser = ReadingText()
        parser.feed(display.read_text())
        self.assertEqual(''.join(parser.paragraphs), expected)

    def rehash_display(self, display):
        integrity_path = display.parent / 'integrity.json'
        integrity = json.loads(integrity_path.read_bytes())
        integrity['presentation'] = i.binding(display)
        integrity_path.write_bytes(i.encoded(integrity))
        # The attack passes byte-binding checks; reconstruction must reject it.
        i.check_binding(integrity['presentation'])

    def assert_feedback_rejected(self, display, output, reason):
        with self.assertRaisesRegex(ValueError, reason):
            record_feedback(display, 'Authored test observation.', output,
                            reviewer_kind='agent')
        self.assertFalse(output.exists(), 'Rejected display must not create a feedback record')
        self.assertEqual(self.h.response_path.read_bytes(), self.original_response)

    def test_invented_effect_title_fails_after_display_rehash(self):
        root = self.comparison('invented-title')
        display = root / 'presentation/comparison.html'
        verify_presentation(display)
        original = display.read_text()
        changed, count = re.subn(r'(<h2\b[^>]*>).*?(</h2>)',
                                r'\1All callers now receive reliable results\2',
                                original, count=1, flags=re.S)
        self.assertEqual(count, 1)
        self.assertNotEqual(changed, original)
        display.write_text(changed)
        self.assert_prose(display, self.original_prose)
        self.rehash_display(display)
        self.assert_feedback_rejected(display, root / 'rejected.json', 'displayed content changed')

    def test_changed_display_claim_coordinates_fail_after_rehash(self):
        root = self.comparison('display-coordinate')
        display = root / 'presentation/comparison.html'
        original = display.read_text()
        self.assertIn('data-reading-start="0"', original)
        display.write_text(original.replace('data-reading-start="0"',
                                            'data-reading-start="1"', 1))
        self.assert_prose(display, self.original_prose)
        self.rehash_display(display)
        self.assert_feedback_rejected(display, root / 'rejected.json', 'displayed content changed')

    def test_changed_editorial_claim_offsets_fail_with_valid_file_hashes(self):
        root = self.comparison('editorial-coordinate', editorial=True)
        receipt_path = root / 'editorial-reading/editorial.json'
        revised, receipt = read(self.h.attempt, receipt_path)
        expected = self.parts[0] + self.parts[2] + self.parts[1] + self.parts[3]
        self.assertEqual(revised['prose'], expected)
        # Change a valid UTF-8 boundary, rather than relying on decoder failure.
        last = revised['claims'][1]['end']
        self.assertEqual(expected.encode()[last - 1:last], b'.')
        revised['claims'][1]['end'] -= 1
        response_path = Path(receipt['response']['path'])
        response_path.write_bytes(i.encoded(revised))
        receipt['response'] = i.binding(response_path)
        receipt['revised_claims'][1]['revised'][1] -= 1
        receipt_path.write_bytes(i.encoded(receipt))
        i.check_binding(receipt['response'])
        with self.assertRaisesRegex(ValueError, 'unsupported editorial prose, claims or Source substitution'):
            read(self.h.attempt, receipt_path)
        self.assertEqual(self.h.response_path.read_bytes(), self.original_response)

    def test_unsupported_editorial_effect_is_rejected_after_all_file_rehashes(self):
        root = self.comparison('unsupported-editorial', editorial=True)
        receipt_path = root / 'editorial-reading/editorial.json'
        revised, receipt = read(self.h.attempt, receipt_path)
        invented = ' This change prevents every downstream failure.'
        revised['prose'] += invented
        response_path = Path(receipt['response']['path'])
        prose_path = Path(receipt['prose']['path'])
        response_path.write_bytes(i.encoded(revised))
        prose_path.write_bytes(revised['prose'].encode())
        receipt.update(response=i.binding(response_path), prose=i.binding(prose_path))
        receipt_path.write_bytes(i.encoded(receipt))
        i.check_binding(receipt['response'])
        i.check_binding(receipt['prose'])
        with self.assertRaisesRegex(ValueError, 'unsupported editorial prose, claims or Source substitution'):
            read(self.h.attempt, receipt_path)
        self.assertEqual(self.h.response_path.read_bytes(), self.original_response)

    def test_stale_code_locations_fail_even_when_prose_and_hashes_match(self):
        root = self.comparison('stale-location')
        display = root / 'presentation/comparison.html'
        original = display.read_text()
        # Independently known from the authored Source: return 1 is line 2,
        # columns 5 through 13; line 999 has no corresponding evidence.
        self.assertIn('src/change.py · before · 2:5–2:13', original)
        display.write_text(original.replace('2:5–2:13', '999:5–999:13'))
        self.assert_prose(display, self.original_prose)
        self.rehash_display(display)
        self.assert_feedback_rejected(display, root / 'rejected.json', 'displayed content changed')

    def test_changed_source_withholds_navigation_and_purpose(self):
        root = self.comparison('source-changed')
        display = root / 'presentation/comparison.html'
        verify_presentation(display)
        frozen = json.loads(Path(self.h.record['input']['path']).read_bytes())
        asset = Path(frozen['entries'][0]['asset']['path'])
        asset.write_bytes(b'not the observed return expression\n')
        body, witness = card(self.h.attempt, 'Independent source mutation', presentation='reader')
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'unavailable_input')
        self.assertNotIn('class="primary-navigation"', body)
        self.assertNotIn('class="site-purpose"', body)
        self.assert_feedback_rejected(display, root / 'rejected.json', 'output/selection identity changed')

    def test_feedback_cannot_transfer_to_a_different_presentation(self):
        root = self.comparison('feedback-transfer')
        reader = root / 'presentation/comparison.html'
        diagnostic = root / 'diagnostic/comparison.html'
        verify_presentation(reader)
        verify_presentation(diagnostic)
        self.assertNotEqual(reader.read_bytes(), diagnostic.read_bytes())
        self.assert_prose(reader, self.original_prose)
        self.assert_prose(diagnostic, self.original_prose)
        copied = copy.deepcopy(json.loads((reader.parent / 'integrity.json').read_bytes()))
        copied['presentation'] = i.binding(diagnostic)
        (diagnostic.parent / 'integrity.json').write_bytes(i.encoded(copied))
        i.check_binding(copied['presentation'])
        self.assert_feedback_rejected(diagnostic, root / 'rejected.json', 'displayed content changed')


if __name__ == '__main__':
    unittest.main()
