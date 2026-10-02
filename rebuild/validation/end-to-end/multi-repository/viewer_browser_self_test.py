#!/usr/bin/env python3
"""Coupled fail-closed boundary/tooling controls; never browser or human evidence."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import viewer_browser as browser


class SnapshotBoundaryTests(unittest.TestCase):
    def test_closed_product_shape_and_internal_svg_paint_are_allowed(self):
        page = '<nav><a href="#work">Work</a></nav><section id="work"><details><summary>Evidence</summary><p>Original &lt;script&gt; text</p></details></section><svg><marker id="arrow"/><path marker-end="url(#arrow)"/></svg>'
        self.assertEqual(browser.snapshot_boundary(page), [])

    def test_each_active_or_live_surface_fails_independently(self):
        for html, expected in [
            ('<a href="http://127.0.0.1:3219/">Live</a>', 'snapshot_live_link'),
            ('<a href="javascript:alert(1)">Run</a>', 'snapshot_live_link'),
            ('<a href="#missing">Work</a>', 'missing_fragment'),
            ('<svg><path marker-end="url(#missing)"/></svg>', 'missing_fragment'),
            ('<p id="same"></p><p id="same"></p>', 'duplicate_fragment'),
            ('<script></script>', 'snapshot_active_content'),
            ('<form></form>', 'snapshot_active_content'),
            ('<p onclick="alert(1)">Text</p>', 'snapshot_active_attribute'),
            ('<img src="https://example.invalid/asset">', 'snapshot_active_attribute'),
            ('<meta http-equiv="refresh" content="0;url=/">', 'snapshot_refresh'),
            ('<style>body{background:url(https://example.invalid/x)}</style>', 'snapshot_token_or_asset'),
            ('<input name="request_authenticity" value="test">', 'snapshot_token_or_asset'),
        ]:
            with self.subTest(expected=expected):
                self.assertIn(expected, browser.snapshot_boundary(html))


class ReadSampleTests(unittest.TestCase):
    def setUp(self):
        self.budgets = json.loads((browser.HERE / "viewer-read-budgets.json").read_text())
        self.samples = [dict(workload=route, sample=sample, adapter="fresh-first-then-warm", html_sha256="a"*64, canonical_basis_sha256="b"*64,
                             **counts, **{stage+"_us":ceiling for stage,ceiling in self.budgets["ceilings_us"][route].items()})
                        for route,counts in self.budgets["required_counts"].items() for sample in range(9)]

    def test_all_stages_and_counts_at_ceiling_are_accepted(self):
        browser.validate_read_samples(self.samples, self.budgets, True)

    def test_last_snapshot_stage_exceedance_is_not_hidden_by_total(self):
        self.samples[-1]["documents_us"] += 1
        with self.assertRaisesRegex(RuntimeError, "stage ceiling"):
            browser.validate_read_samples(self.samples, self.budgets, True)
        browser.validate_read_samples(self.samples, self.budgets, False)

    def test_missing_duplicate_counts_and_output_identities_are_rejected(self):
        for mutation in [lambda s:s.pop(), lambda s:s.__setitem__(-1,s[0]),
                         lambda s:s[-1].__setitem__("documents",3), lambda s:s[0].__setitem__("graph_decodes",1),
                         lambda s:s[0].__setitem__("health_graph_decodes",1),lambda s:s[0].__setitem__("html_sha256",None),
                         lambda s:s[0].__setitem__("canonical_basis_sha256","short"),lambda s:s[0].__setitem__("projection_us",None)]:
            rows=copy.deepcopy(self.samples)
            mutation(rows)
            with self.assertRaises(RuntimeError):
                browser.validate_read_samples(rows,self.budgets,False)


class MissingExecutionTests(unittest.TestCase):
    def invoke(self, argv):
        with tempfile.TemporaryDirectory() as root:
            with patch.object(browser, 'ROOT', Path(root)), patch.object(browser, 'candidate', return_value='synthetic-candidate'), patch.object(browser, 'clean', return_value=True), patch.object(sys, 'argv', ['viewer_browser.py', *argv]):
                self.assertEqual(browser.main(), 1)
            results = list(Path(root).glob('rebuild/.local/validation/viewer-browser-*/result.json'))
            self.assertEqual(len(results), 1)
            result = json.loads(results[0].read_text())
            self.assertEqual(result['operations'], [])
            self.assertEqual(result['human_qualification'], 'not_established')
            return result

    def test_browser_absence_is_not_driver_absence_or_a_pass(self):
        result = self.invoke(['--chromium', '/missing-chromium', '--playwright-module', '/missing-driver'])
        self.assertEqual(result['status'], 'browser_unavailable')

    def test_driver_absence_has_distinct_outcome(self):
        result = self.invoke(['--chromium', sys.executable, '--playwright-module', '/missing-driver'])
        self.assertEqual(result['status'], 'driver_unavailable')

    def test_unverified_glyph_coverage_is_blocked_before_browser_execution(self):
        with tempfile.TemporaryDirectory() as package:
            Path(package, 'package.json').write_text('{"version":"synthetic"}')
            available = browser.shutil.which
            with patch.object(browser.shutil, 'which', side_effect=lambda name: None if name == 'fc-list' else available(name)):
                result = self.invoke(['--chromium', sys.executable, '--playwright-module', package])
            self.assertEqual(result['status'], 'font_prerequisite_blocked')

    def test_dirty_final_candidate_is_blocked_before_any_execution(self):
        with patch.object(browser, 'clean', return_value=False):
            # Explicitly override invoke's clean patch for this isolated admission control.
            with tempfile.TemporaryDirectory() as root, patch.object(browser, 'ROOT', Path(root)), patch.object(browser, 'candidate', return_value='synthetic-candidate'), patch.object(sys, 'argv', ['viewer_browser.py', '--chromium', '/missing', '--playwright-module', '/missing', '--require-clean']):
                self.assertEqual(browser.main(), 1)
                result = json.loads(next(Path(root).glob('rebuild/.local/validation/viewer-browser-*/result.json')).read_text())
                self.assertEqual(result['status'], 'candidate_blocked')
                self.assertFalse(result['initial_clean'])
                self.assertEqual(result['operations'], [])


if __name__ == '__main__':
    unittest.main()
