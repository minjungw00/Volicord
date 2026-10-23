"""Independent empty-font controls for actual local browser consumers.

The supplied Playwright module is loaded, but Chromium must never launch. These
controls establish prerequisite rejection, not screenshots or glyph quality.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
REBUILD = HERE.parents[3]


class KoreanFontPrerequisiteTests(unittest.TestCase):
    def setUp(self):
        if not shutil.which('node') or not shutil.which('fc-list'):
            self.skipTest('Node and fontconfig fc-list are required')
        module = os.environ.get('EXPLANATION_PLAYWRIGHT')
        if not module or not Path(module).exists():
            self.skipTest('Explicit existing EXPLANATION_PLAYWRIGHT module is required')
        self.playwright = str(Path(module).resolve())
        private = REBUILD / '.local' / 'font-prerequisite-controls'
        private.mkdir(parents=True, exist_ok=True)
        temp = tempfile.TemporaryDirectory(prefix='empty-fonts-', dir=private)
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        cache = self.root / 'empty-cache'
        cache.mkdir()
        self.fontconfig = self.root / 'fonts.conf'
        # No <dir> or <include> exists: system fonts and fallback configurations
        # cannot turn this negative fixture into an installed Korean-font case.
        self.fontconfig.write_text(
            '<?xml version="1.0"?>\n<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">\n'
            '<fontconfig><cachedir>' + str(cache) + '</cachedir></fontconfig>\n')
        self.env = dict(os.environ, FONTCONFIG_FILE=str(self.fontconfig),
                        FONTCONFIG_PATH=str(self.root))
        fonts = subprocess.run(
            ['fc-list', '--format', '%{file}\n', ':lang=ko'], env=self.env,
            capture_output=True, text=True, timeout=15)
        self.assertEqual(fonts.returncode, 0, fonts.stderr)
        self.assertEqual(fonts.stdout.strip(), '', 'Negative fixture must have zero Korean fonts')
        self.chromium = str(self.root / 'nonexistent-chromium')
        self.assertFalse(Path(self.chromium).exists())
        self.display = self.root / 'authored.html'
        self.display.write_text('<!doctype html><html lang="ko"><p>반환 값을 조사한다.</p></html>')

    def assert_prerequisite_rejection(self, command):
        result = subprocess.run(command, env=self.env, capture_output=True,
                                text=True, timeout=20)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Korean font prerequisite unavailable', result.stderr)
        self.assertNotIn('browserType.launch', result.stderr,
                         'Font rejection must precede even the nonexistent Chromium launch')
        self.assertNotIn('Executable doesn\'t exist', result.stderr)
        self.assertNotIn('Cannot find module', result.stderr)
        self.assertEqual(result.stdout, '', 'Rejected prerequisites cannot emit completed observations')
        self.assertFalse(list(self.root.rglob('*.png')))
        self.assertFalse(list(self.root.rglob('observations.json')))

    def test_grounded_diagrams_require_korean_fonts_before_browser_launch(self):
        executable = self.root / 'dummy-executable'
        executable.write_text('immutable prerequisite control, never executed')
        config = self.root / 'diagram-config.json'
        config.write_text(json.dumps({
            'playwright': self.playwright, 'viewer': str(executable),
            'chromium': str(executable), 'output': str(self.root / 'diagram-output'),
            'cases': [{'language': 'ko'}], 'fontconfig': str(self.fontconfig),
        }))
        self.assert_prerequisite_rejection([
            'node', str(REBUILD / 'validation/end-to-end/multi-repository/grounded_diagrams_browser.cjs'),
            str(config),
        ])

    def test_experimental_korean_reader_rejects_empty_font_inventory_before_launch(self):
        plan = self.root / 'plan.json'
        plan.write_text(json.dumps([{
            'name': 'independently-authored-empty-font-reader',
            'display': str(self.display), 'language': 'ko',
            'prose': '반환 값을 조사한다.', 'title': '반환 값 조사',
            'purposes': [], 'codes': [], 'diffCount': 0,
        }], ensure_ascii=False))
        self.assert_prerequisite_rejection([
            'node', str(HERE / 'presentation_browser_self_test.cjs'),
            str(plan), self.chromium, self.playwright,
        ])

    def product_config(self, body_language, locale):
        readback = self.root / 'readback.json'
        # This authored readback is sufficient only for the font prerequisite.
        # It is deliberately no completed Product explanation or semantic claim.
        readback.write_text(json.dumps({'selected_work': {
            'answers': {'explanation_state': 'current'},
        }}))
        output = self.root / 'output'
        output.mkdir()
        config = self.root / 'config.json'
        config.write_text(json.dumps({
            'playwright': self.playwright, 'chromium': self.chromium,
            'readback': str(readback), 'output': str(output),
            'body_language': body_language,
            'displays': [{'path': str(self.display), 'locale': locale}],
        }))
        return config

    def test_product_korean_body_requires_fonts_with_english_interface(self):
        config = self.product_config('ko', 'en')
        self.assert_prerequisite_rejection([
            'node', str(REBUILD / 'validation/end-to-end/multi-repository/recorded_explanation_browser.cjs'),
            str(config),
        ])

    def test_product_korean_interface_requires_fonts_with_english_body(self):
        config = self.product_config('en', 'ko')
        self.assert_prerequisite_rejection([
            'node', str(REBUILD / 'validation/end-to-end/multi-repository/recorded_explanation_browser.cjs'),
            str(config),
        ])


if __name__ == '__main__':
    unittest.main()
