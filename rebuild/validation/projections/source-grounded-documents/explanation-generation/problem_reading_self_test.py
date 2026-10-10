"""Problem-first reading through actual output/grounding/browser/feedback consumers.

Inline fixtures are independently authored controls, not model or prototype prose.
Retained outputs require explicit paths and are never regenerated or rewritten.
"""
import base64
import copy
import datetime as dt
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import subprocess
import unittest

import inputs as i
import source_reading_self_test as controls
import work_reader_self_test as work_controls
from render_comparison import card, render, record_feedback


class Targets(HTMLParser):
    def __init__(self):
        super().__init__()
        self.stack, self.targets = [], {}

    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if 'id' in attrs:
            self.targets[attrs['id']] = (tag, sum(parent == 'details' for parent in self.stack))
        if tag not in {'meta', 'br', 'hr', 'input'}:
            self.stack.append(tag)

    def handle_endtag(self, tag):
        if tag in self.stack:
            self.stack = self.stack[:len(self.stack) - 1 - self.stack[::-1].index(tag)]


class ProblemReadingTests(unittest.TestCase):
    def setUp(self):
        self.fixture = controls.ComparisonTests(methodName='runTest')
        self.fixture.setUp(); self.addCleanup(self.fixture.doCleanups)
        self.h = self.fixture.h
        self.response = self.fixture.paired(
            'def operation():\n    return 1\n\ndef unrelated():\n    return 4\n',
            'def operation():\n    return 2\n\ndef unrelated():\n    return 9\n', ranges=[(21, 29), (21, 29)])
        self.response['prose'] = 'The operation returned an older value. The replacement returns a newer value. Inspect the paired statements before continuing.'
        self.response.update(claims=[{'start': 0, 'end': len(self.response['prose'].encode()),
                                     'kind': 'source_fact', 'selections': [0, 1], 'authority': None}],
            primary_sites=[{'selections': [0, 1], 'claims': [0], 'reason': 'Inspect the authored return statements.',
                            'extent_reason': 'The expression is enough; the unrelated helper is not a primary answer.'}])
        self.h.record['output_contract'] = 'work_directed_reader'
        self.sync()

    def sync(self):
        self.h.response_path.write_bytes(i.encoded(self.response))
        self.h.record.update(original_outputs=[i.binding(self.h.response_path)], generation_output=i.binding(self.h.response_path))
        self.h.attempt.write_bytes(i.encoded(self.h.record))

    def test_primary_targets_are_outside_disclosures_and_keep_exact_bytes(self):
        body, witness, parser = self.h.present()
        targets = Targets(); targets.feed(body)
        for selected in witness['outputs'][0]['selections']:
            self.assertEqual(targets.targets[selected['anchor']], ('section', 0))
        self.assertEqual(parser.values['prose'], [self.response['prose']])
        self.assertEqual(parser.values['code'], ['return 1', 'return 2'])
        self.assertEqual(parser.values['diff'][0],
            '--- src/change.py (before)\n+++ src/change.py (after)\n@@ -1,4 +1,4 @@\n def operation():\n-    return 1\n+    return 2\n \n def unrelated():\n')
        self.assertNotIn('return 9', parser.values['diff'][0])
        self.assertIn('+    return 9', parser.values['full-diff'][0])
        self.assertIn('src/change.py · before · 2:5–2:13', body)
        self.assertLess(body.index('class="primary-navigation"'), body.index('class="prose"'))
        self.assertLess(body.index('class="prose"'), body.index('class="work-problem-basis"'))
        downloads = [base64.b64decode(link.split(',', 1)[1]) for link in parser.links if link.startswith('data:')]
        self.assertIn(self.h.response_path.read_bytes(), downloads)
        self.assertIn(b'return 1', downloads)
        self.assertIn(b'return 2', downloads)
        self.assertEqual(len(parser.anchors), len(set(parser.anchors)))

    def test_invalid_evidence_never_gets_a_primary_navigation_target(self):
        original = copy.deepcopy(self.response)
        for field, value in [('id', 'foreign'), ('state', 'context'), ('sha256', '0' * 64), ('start', -1)]:
            with self.subTest(field=field):
                self.response = copy.deepcopy(original)
                self.response['selections'][0][field] = value
                self.sync()
                body, witness, _ = self.h.present()
                self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'invalid')
                self.assertNotIn('class="primary-navigation"', body)
                self.assertNotIn('primary-source', body)
        self.response = original; self.sync()
        frozen = json.loads(Path(self.h.record['input']['path']).read_bytes())
        Path(frozen['entries'][0]['asset']['path']).write_bytes(b'changed hash')
        body, witness, parser = self.h.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'unavailable_input')
        self.assertNotIn('code', parser.values)
        self.assertNotIn('class="primary-navigation"', body)

    def test_missing_before_is_context_with_explicit_gap_not_a_diff(self):
        self.response['selections'] = self.response['selections'][1:]
        self.response['claims'][0]['selections'] = [0]
        self.response['primary_sites'][0]['selections'] = [0]
        self.sync()
        body, witness, parser = self.h.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
        self.assertEqual(parser.values['code'], ['return 2'])
        self.assertNotIn('diff', parser.values)
        self.assertIn('Missing state is not inferred', body)
        self.assertIn('primary-source', body)

    def test_foreign_ambiguous_unpaired_and_absent_evidence_withhold_primary_targets(self):
        path = Path(self.h.record['input']['path'])
        original = path.read_bytes()
        for case in ['foreign-work', 'ambiguous-chronology', 'unmatched-pair']:
            with self.subTest(case=case):
                spec = json.loads(original)
                if case == 'foreign-work':
                    spec['entries'][0]['work'] = 'another-work'
                elif case == 'ambiguous-chronology':
                    spec['entries'][0]['chronology'] = {'state': 'ambiguous', 'reason': 'Authored absent ordering.'}
                else:
                    spec['entries'][1]['locator'] = 'different-change:before'
                path.write_bytes(i.encoded(spec)); self.h.record['input'] = i.binding(path)
                body, witness, parser = self.h.present()
                self.assertNotEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
                self.assertNotIn('class="primary-navigation"', body)
                self.assertNotIn('primary-source', body)
                self.assertNotIn('diff', parser.values)
        path.write_bytes(original); self.h.record['input'] = i.binding(path)
        spec = json.loads(original)
        Path(spec['entries'][0]['asset']['path']).unlink()
        body, witness, parser = self.h.present()
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'unavailable_input')
        self.assertNotIn('code', parser.values)
        self.assertNotIn('class="primary-navigation"', body)
        self.h.response_path.unlink()
        body, witness = card(self.h.attempt, 'Missing response')
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'unavailable_response')
        self.assertNotIn('primary-source', body)

    def browser(self, plan):
        if not (os.environ.get('EXPLANATION_CHROMIUM') and os.environ.get('EXPLANATION_PLAYWRIGHT')):
            self.skipTest('explicit real Chromium and Playwright paths required')
        path = self.h.root / 'browser-plan.json'; path.write_bytes(i.encoded(plan))
        result = subprocess.run(['node', str(Path(__file__).with_name('problem_browser_self_test.cjs')), str(path),
            os.environ['EXPLANATION_CHROMIUM'], os.environ['EXPLANATION_PLAYWRIGHT']], capture_output=True, text=True, timeout=60)
        print(result.stdout, end='')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return json.loads(result.stdout)

    def test_authored_browser_positive_missing_before_and_invalid_source(self):
        plan = []
        for name in ['paired', 'missing-before', 'invalid-source']:
            if name == 'missing-before':
                self.response['selections'] = self.response['selections'][1:]
                self.response['claims'][0]['selections'] = [0]
                self.response['primary_sites'][0]['selections'] = [0]
            elif name == 'invalid-source':
                self.response['selections'][0]['sha256'] = '0' * 64
            self.sync()
            display = render([self.h.attempt], self.h.root / name)
            codes = ([{'index': 0, 'label': 'src/change.py · before · 2:5–2:13', 'text': 'return 1'},
                      {'index': 1, 'label': 'src/change.py · after · 2:5–2:13', 'text': 'return 2'}] if name == 'paired' else
                     [{'index': 0, 'label': 'src/change.py · after · 2:5–2:13', 'text': 'return 2'}] if name == 'missing-before' else [])
            plan.append({'name': name, 'display': str(display), 'codes': codes, 'noDiff': name != 'paired',
                         'diffs': ['--- src/change.py (before)\n+++ src/change.py (after)\n@@ -1,4 +1,4 @@\n def operation():\n-    return 1\n+    return 2\n \n def unrelated():\n'] if name == 'paired' else []})
            baseline_root = os.environ.get('EXPLANATION_READING_BASELINES')
            if name == 'paired' and baseline_root:
                plan[-1]['baseline'] = str((Path(baseline_root) / 'authored-baseline/comparison.html').resolve())
            feedback = record_feedback(display, None, self.h.root / (name + '-feedback.json'))
            self.assertEqual(json.loads(feedback.read_bytes())['H1'], 'pending')
        self.browser(plan)

    def test_checkpoint_warning_scope_preserves_failed_observation_and_action(self):
        control = work_controls.WorkReaderTests(methodName='runTest')
        control.setUp(); self.addCleanup(control.doCleanups)
        control.evidence('canonical_record', control.cp, table='checkpoints')
        control.evidence('canonical_record', {'id': 'relation', 'project_id': control.cp['project_id'],
            'checkpoint_id': 'cp', 'source_id': 'absent-source'}, table='checkpoint_source_relations')
        control.evidence('canonical_record', {'id': 'verification', 'project_id': control.cp['project_id'],
            'checkpoint_id': 'cp', 'source_id': 'absent-source', 'verification_state': 'failed',
            'outcome': 'Independent command failure.'}, table='checkpoint_verifications')
        body, witness, parser = control.present()
        self.assertEqual(parser.values['direction'], [control.cp['next_step']])
        self.assertIn('latest Checkpoint relations', body)
        self.assertIn('Command Source unavailable', body)
        self.assertIn('Recorded failed verification observations: 1', body)
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
        self.assertIn('primary-source', body)
        self.assertIn('User review: not_requested. User acceptance: not_requested', body)

    @unittest.skipUnless(os.environ.get('EXPLANATION_RETAINED_WORK_B') and os.environ.get('EXPLANATION_RETAINED_CLICK'),
                         'explicit retained resource-permissive attempts required')
    def test_retained_outputs_exact_source_browser_and_feedback(self):
        plan = []
        expected = {
            'work-b': ('5984df1535f91c326b0f7f3285490ab60143e0907d806fff690b28f5eeae35e0',
                       '1d25357823a4407d5600e5414ca7d48ba327d388fa684b1f8429619f8e2f3522',
                       [(51, 1274, 1284), (52, 1274, 1286), (53, 1341, 1378), (54, 1378, 1411),
                        (55, 1423, 1440), (58, 175, 176), (59, 175, 176), (56, 1470, 1500), (57, 1500, 1523)]),
            'click': ('f5d3d649d21ddddf715dc9d45b0172568b2ac9c7d98a7901c879c6fdb0e924e8',
                      '67c42f8fc0ac6f566e9fbe7ec0d4e88a57444fe56542d593c7626b24a2e008dc',
                      [(0, 985, 998), (1, 140, 159), (2, 170, 182), (3, 450, 453)])}
        for name, env in [('work-b', 'EXPLANATION_RETAINED_WORK_B'), ('click', 'EXPLANATION_RETAINED_CLICK')]:
            path = Path(os.environ[env]); before = path.read_bytes()
            attempt_hash, output_hash, positions = expected[name]
            self.assertEqual(i.binding(path)['sha256'], attempt_hash)
            record = json.loads(before)
            self.assertEqual(record['status'], 'captured')
            self.assertEqual(record['original_outputs'][0]['sha256'], output_hash)
            original = record['original_outputs'][0]; i.check_binding(original)
            raw = Path(original['path']).read_bytes(); response = json.loads(raw)
            spec = i.verify(record['input']['path']); entries = {e['id']: e for e in spec['entries']}
            display = render([path], self.h.root / name)
            body, witness = card(path, 'Sample 01')
            self.assertEqual(witness['outputs'][0]['derived_reading']['status'], 'valid_binding')
            self.assertEqual([s['site'] for s in witness['outputs'][0]['reading']['sites']], response['primary_sites'])
            self.assertEqual(len(response['primary_sites']), 7 if name == 'work-b' else 4)
            codes = []
            for index, first, end in positions:
                selection = response['selections'][index]; entry = entries[selection['id']]
                data = Path(entry['asset']['path']).read_bytes(); i.check_binding(entry['asset'])
                self.assertEqual(i.digest(data), entry['file_sha256'])
                self.assertEqual(data[:selection['start']].count(b'\n') + 1, first)
                self.assertEqual(data[:selection['end']].count(b'\n') + 1, end)
                self.assertTrue(selection['start'] == 0 or data[:selection['start']].endswith(b'\n'))
                self.assertTrue(data[:selection['end']].endswith(b'\n'))
                selected = data[selection['start']:selection['end']]
                self.assertEqual(i.digest(selected), selection['sha256'])
                self.assertEqual(entry['project'], spec['scope']['project'])
                self.assertEqual(entry['work'], spec['scope']['work'])
                self.assertEqual(entry['chronology']['state'], 'known')
                observed = dt.datetime.fromisoformat(entry['chronology']['observed_at'])
                self.assertIsNotNone(observed.tzinfo)
                self.assertLessEqual(observed, dt.datetime.fromisoformat(spec['scope']['cutoff']))
                expected_anchor = ('work-' + i.digest(i.encoded(spec['scope']))[:20] + '-attempt-' + attempt_hash
                                   + '-output-' + output_hash + '-response-1-selection-' + str(index + 1))
                self.assertEqual(witness['outputs'][0]['selections'][index]['anchor'], expected_anchor)
                codes.append({'index': index, 'label': f"{entry['path']} · {selection['state']} · {first}:1–{end}:1",
                              'text': selected.decode()})
            import render_self_test as baseline
            parser = baseline.Texts(); parser.feed(body)
            self.assertEqual(parser.values['prose'], [response['prose']])
            self.assertEqual([r['selection'] for r in witness['outputs'][0]['selections']], response['selections'])
            diffs = parser.values.get('diff', [])
            if name == 'work-b':
                # Independently inspected old/new statements and surrounding
                # lines. This oracle does not consume renderer diff witnesses.
                old = Path(entries['input-000922']['asset']['path']).read_bytes().decode().splitlines(keepends=True)
                new = Path(entries['input-000921']['asset']['path']).read_bytes().decode().splitlines(keepends=True)
                self.assertEqual(entries['input-000922']['file_sha256'], '67942762e6cfa6191e569a1b519e23788e62b0bd0ff0e65708605eca3689e419')
                self.assertEqual(entries['input-000921']['file_sha256'], '78fdce6dc547df74c3493c5da6fb3c9e96ca970f8d394dd09adf40c0e41ccc9b')
                header = '--- rebuild/crates/volicord-operations/src/cli.rs (before)\n+++ rebuild/crates/volicord-operations/src/cli.rs (after)\n'
                expected_diffs = [header + '@@ -1273,5 +1273,7 @@\n'
                    + ''.join(' ' + line for line in old[1272:1274]) + '-' + old[1274]
                    + ''.join('+' + line for line in new[1274:1277])
                    + ''.join(' ' + line for line in old[1275:1277]),
                    header + '@@ -173,5 +173,5 @@\n'
                    + ''.join(' ' + line for line in old[172:174]) + '-' + old[174] + '+' + new[174]
                    + ''.join(' ' + line for line in old[175:177])]
                self.assertEqual(diffs, expected_diffs)
                self.assertEqual(len(diffs), 2)
                self.assertIn('@@ -1273,5 +1273,7 @@', diffs[0])
                self.assertIn('-                if [\n', diffs[0])
                self.assertIn('+                if *key == "repository_analysis" {\n', diffs[0])
                self.assertIn('+                    render_analysis_status(field, mode.locale, stdout)?;\n', diffs[0])
                self.assertIn('@@ -173,5 +173,5 @@', diffs[1])
                self.assertEqual(sum(f['observation']['verification_state'] == 'failed' for f in witness['recorded_work']['verification']), 3)
            else:
                self.assertEqual(diffs, [])
                self.assertTrue(all(s['state'] == 'context' for s in response['selections']))
            feedback = record_feedback(display, None, self.h.root / (name + '-feedback.json'))
            self.assertEqual(json.loads(feedback.read_bytes())['original_outputs'], record['original_outputs'])
            self.assertEqual(json.loads(feedback.read_bytes())['H1'], 'pending')
            baseline_root = os.environ.get('EXPLANATION_READING_BASELINES')
            sample = {'name': name, 'display': str(display), 'codes': codes, 'diffs': diffs, 'noDiff': name == 'click'}
            if baseline_root:
                old = Path(baseline_root) / (name + '-baseline/comparison.html')
                sample['baseline'] = str(old.resolve())
                self.assertNotEqual(i.binding(old)['sha256'], i.binding(display)['sha256'])
                with self.assertRaises(ValueError):
                    record_feedback(old, None, self.h.root / (name + '-migrated-feedback.json'))
            plan.append(sample)
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(Path(original['path']).read_bytes(), raw)
        self.browser(plan)


if __name__ == '__main__':
    unittest.main()
