"""Output-bound presentation controls, separate from semantic/human assessment."""
import copy
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import re
import subprocess
import unittest

import inputs as i
import problem_reading_self_test as problem_controls
from reading_input import verify_reading_input
from render_comparison import card, record_feedback
from render_readings import compare


class Paragraphs(HTMLParser):
    def __init__(self):
        super().__init__(); self.active = False; self.prose = []

    def handle_starttag(self, tag, attrs):
        if tag == 'pre' and dict(attrs).get('class') == 'prose-paragraph':
            self.active = True; self.prose.append('')

    def handle_data(self, data):
        if self.active:
            self.prose[-1] += data

    def handle_endtag(self, tag):
        if tag == 'pre':
            self.active = False


class PresentationTests(unittest.TestCase):
    def setUp(self):
        fixture = problem_controls.ProblemReadingTests('runTest'); fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        self.fixture, self.h = fixture, fixture.h
        # Independent multilingual prose, distinct paragraphs and claim offsets.
        prose = '이 작업은 반환 값을 조사한다. The operation returns a new value.\n\n두 상태를 비교한다. Verification remains historical.'
        fixture.response['prose'] = prose
        fixture.response['claims'][0]['end'] = len(prose.encode())
        fixture.freeze_language('ko'); fixture.sync()

    def assert_original(self, display, prose):
        parser = Paragraphs(); parser.feed(display.read_text())
        self.assertEqual(''.join(parser.prose), prose)

    def sample(self, display, response, name='authored', *, missing_before=False):
        codes = ([{'label': 'src/change.py · before · 2:5–2:13', 'text': 'return 1'},
                  {'label': 'src/change.py · after · 2:5–2:13', 'text': 'return 2'}] if not missing_before else
                 [{'label': 'src/change.py · after · 2:5–2:13', 'text': 'return 2'}])
        return {'name': name, 'display': str(display), 'prose': response['prose'],
                'title': response['prose'].split('. ')[0] + '.',
                'purposes': [s['reason'] for s in response['primary_sites']], 'codes': codes,
                'diffCount': 0 if missing_before else 1, 'missingBefore': missing_before, 'language': 'ko'}

    def browser(self, plan, *, failure=None):
        if not os.environ.get('EXPLANATION_CHROMIUM') or not os.environ.get('EXPLANATION_PLAYWRIGHT'):
            self.skipTest('explicit Chromium and Playwright required for browser observations')
        path = self.h.root / 'browser-plan.json'; path.write_bytes(i.encoded(plan))
        result = subprocess.run(['node', str(Path(__file__).with_name('presentation_browser_self_test.cjs')),
            str(path), os.environ['EXPLANATION_CHROMIUM'], os.environ['EXPLANATION_PLAYWRIGHT']],
            capture_output=True, text=True, timeout=180)
        if failure:
            self.assertNotEqual(result.returncode, 0); self.assertIn(failure, result.stderr)
        else:
            self.assertEqual(result.returncode, 0, result.stderr)
            print(result.stdout)

    def test_original_baseline_and_presentation_feedback(self):
        original = self.h.response_path.read_bytes()
        compare(self.h.attempt, self.h.root / 'compare')
        baseline = self.h.root / 'compare/diagnostic/comparison.html'
        display = self.h.root / 'compare/presentation/comparison.html'
        self.assert_original(display, self.fixture.response['prose'])
        self.assertIn('<h2>Sample 01</h2>', baseline.read_text())
        self.assertNotIn('<h2>Sample 01</h2>', display.read_text())
        self.assertIn(self.fixture.response['primary_sites'][0]['reason'], display.read_text())
        feedback = record_feedback(display, None, self.h.root / 'feedback.json')
        value = json.loads(feedback.read_bytes()); self.assertEqual(value['H1'], 'pending')
        self.assertEqual(value['original_outputs'], self.h.record['original_outputs'])
        # Old view is not eligible to bind to the new view's identity.
        integrity = display.parent / 'integrity.json'; saved = integrity.read_bytes()
        record = json.loads(saved); record['presentation'] = i.binding(baseline)
        integrity.write_bytes(i.encoded(record))
        with self.assertRaisesRegex(ValueError, 'display path changed'):
            record_feedback(display, None, self.h.root / 'older-view.json')
        integrity.write_bytes(saved)
        body = display.read_bytes(); display.write_bytes(body + b'changed')
        with self.assertRaises(ValueError):
            record_feedback(display, None, self.h.root / 'tamper.json')
        record = json.loads(saved); record['presentation'] = i.binding(display); integrity.write_bytes(i.encoded(record))
        with self.assertRaisesRegex(ValueError, 'displayed content changed'):
            record_feedback(display, None, self.h.root / 'rehashed-tamper.json')
        self.assertEqual(self.h.response_path.read_bytes(), original)

    def test_false_source_stale_evidence_and_missing_before(self):
        self.fixture.response['selections'][0]['sha256'] = '0' * 64; self.fixture.sync()
        body, witness = card(self.h.attempt, 'diagnostic', presentation='reader')
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'invalid')
        self.assertNotIn('class="site-purpose"', body)
        self.fixture.response['selections'] = self.fixture.response['selections'][1:]
        self.fixture.response['claims'][0]['selections'] = [0]
        self.fixture.response['primary_sites'][0]['selections'] = [0]; self.fixture.sync()
        compare(self.h.attempt, self.h.root / 'missing-before')
        display = self.h.root / 'missing-before/presentation/comparison.html'
        self.assert_original(display, self.fixture.response['prose'])
        self.assertNotIn('class="diff"', display.read_text())
        self.assertIn('Missing state is not inferred', display.read_text())
        frozen = json.loads(Path(self.h.record['input']['path']).read_bytes())
        Path(frozen['entries'][0]['asset']['path']).write_bytes(b'stale evidence')
        body, witness = card(self.h.attempt, 'stale', presentation='reader')
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'unavailable_input')
        self.assertNotIn('class="primary-navigation"', body)

    def test_actual_browser_normal_false_link_and_hierarchy_controls(self):
        compare(self.h.attempt, self.h.root / 'browser')
        display = self.h.root / 'browser/presentation/comparison.html'
        sample = self.sample(display, self.fixture.response)
        self.browser([sample])
        original = display.read_text()
        false = re.sub(r'(<nav class="primary-navigation".*?<a href=")[^"]+', r'\1#invented-source', original, count=1, flags=re.S)
        self.assertNotEqual(original, false)
        display.write_text(false); self.browser([sample], failure='source target absent')
        display.write_text(original.replace('</style>', '.reader-narrative{margin-top:1000px}</style>'))
        self.browser([sample], failure='prose obscured by hierarchy')
        self.fixture.response['selections'] = self.fixture.response['selections'][1:]
        self.fixture.response['claims'][0]['selections'] = [0]
        self.fixture.response['primary_sites'][0]['selections'] = [0]; self.fixture.sync()
        compare(self.h.attempt, self.h.root / 'browser-missing-before')
        missing = self.h.root / 'browser-missing-before/presentation/comparison.html'
        self.browser([self.sample(missing, self.fixture.response, 'missing-before', missing_before=True)])

    def test_historical_producer_is_not_a_source_integrity_bypass(self):
        original = json.loads(Path(self.h.record['input']['path']).read_bytes())
        head = subprocess.run(['git','rev-parse','HEAD'], capture_output=True, text=True, check=True).stdout.strip()
        for reason, producer in [
            ('hash', {'head': head, 'adapter': dict(i.binding(Path(__file__).with_name('archive.py')), sha256='0'*64)}),
            ('revision', {'head': '0'*40, 'adapter': dict(i.binding(Path(__file__).with_name('archive.py')), sha256='0'*64)}),
            ('path', {'head': head, 'adapter': dict(i.binding(self.h.response_path), sha256='0'*64)})]:
            value = copy.deepcopy(original); value['identities']['experiment_producer'] = producer
            path = self.h.root / (reason + '-input.json'); path.write_bytes(i.encoded(value))
            with self.assertRaises(ValueError):
                verify_reading_input(path)

    @unittest.skipUnless(os.environ.get('EXPLANATION_RETAINED_WORK_B') and os.environ.get('EXPLANATION_RETAINED_CLICK'), 'explicit actual retained outputs required')
    def test_retained_work_b_and_click(self):
        expected = {'work-b': ('5984df1535f91c326b0f7f3285490ab60143e0907d806fff690b28f5eeae35e0', '1d25357823a4407d5600e5414ca7d48ba327d388fa684b1f8429619f8e2f3522', [(51,1274,1284),(52,1274,1286),(53,1341,1378),(54,1378,1411),(55,1423,1440),(58,175,176),(59,175,176),(56,1470,1500),(57,1500,1523)]),
            'click': ('f5d3d649d21ddddf715dc9d45b0172568b2ac9c7d98a7901c879c6fdb0e924e8', '67c42f8fc0ac6f566e9fbe7ec0d4e88a57444fe56542d593c7626b24a2e008dc', [(0,985,998),(1,140,159),(2,170,182),(3,450,453)])}
        plans = []
        root = Path(os.environ.get('EXPLANATION_READER_OUTPUT', self.h.root))
        for name, env in [('work-b','EXPLANATION_RETAINED_WORK_B'),('click','EXPLANATION_RETAINED_CLICK')]:
            path = Path(os.environ[env]); attempt = json.loads(path.read_bytes())
            attempt_hash, output_hash, positions = expected[name]
            self.assertEqual(i.binding(path)['sha256'], attempt_hash)
            self.assertEqual(attempt['original_outputs'][0]['sha256'], output_hash)
            i.check_binding(attempt['original_outputs'][0]); raw = Path(attempt['original_outputs'][0]['path']).read_bytes()
            response = json.loads(raw); spec = verify_reading_input(attempt['input']['path']); entries = {e['id']:e for e in spec['entries']}
            compare(path, root / name)
            display = root / name / 'presentation/comparison.html'
            self.assert_original(display, response['prose'])
            _, witness = card(path, 'Sample 01', presentation='reader')
            self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
            self.assertEqual([s['site'] for s in witness['outputs'][0]['reading']['sites']], response['primary_sites'])
            codes = []
            for index, start, end in positions:
                selection = response['selections'][index]; entry = entries[selection['id']]
                data = Path(entry['asset']['path']).read_bytes(); i.check_binding(entry['asset'])
                self.assertEqual(data[:selection['start']].count(b'\n')+1,start)
                self.assertEqual(data[:selection['end']].count(b'\n')+1,end)
                text = data[selection['start']:selection['end']]
                self.assertEqual(i.digest(text),selection['sha256'])
                codes.append({'label':f"{entry['path']} · {selection['state']} · {start}:1–{end}:1", 'text':text.decode()})
            feedback = record_feedback(display,None,root / name / 'pending-feedback.json')
            self.assertEqual(json.loads(feedback.read_bytes())['H1'],'pending')
            plans.append({'name':name,'display':str(display),'prose':response['prose'],
                'title':response['prose'].split('. ')[0]+'.','language':'ko',
                'purposes':[s['reason'] for s in response['primary_sites']], 'codes':codes,'diffCount':2 if name=='work-b' else 0})
            self.assertEqual(Path(attempt['original_outputs'][0]['path']).read_bytes(),raw)
        self.browser(plans)


if __name__ == '__main__':
    unittest.main()
