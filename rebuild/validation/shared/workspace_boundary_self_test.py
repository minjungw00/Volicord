#!/usr/bin/env python3
"""Actual diagnostic entry-point collision and failed-attempt/retry controls."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ENTRY = Path(__file__).with_name('workspace_boundary.py')


def inventory(path):
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in path.iterdir()}


class WorkspaceBoundaryTests(unittest.TestCase):
    def test_success_failure_collisions_and_fresh_retry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            success, failure, retry = (root / name for name in ('success', 'failure', 'retry'))

            def run(path, *extra):
                return subprocess.run([sys.executable, str(ENTRY), '--output', str(path), *extra],
                                      capture_output=True)

            self.assertEqual(run(success).returncode, 0)
            self.assertEqual(run(failure, '--cargo', '/bin/false').returncode, 1)
            self.assertEqual(json.loads((failure / 'boundary-result.json').read_text())['exit_code'], 1)
            for path in (success, failure):
                before = inventory(path)
                self.assertNotEqual(run(path).returncode, 0)
                self.assertEqual(before, inventory(path))
            self.assertEqual(run(retry).returncode, 0)
            self.assertEqual(json.loads((retry / 'boundary-result.json').read_text())['exit_code'], 0)
            # File and dangling symlink destinations are existing ownership too.
            existing = root / 'existing'
            existing.write_bytes(b'original diagnostic')
            self.assertNotEqual(run(existing).returncode, 0)
            self.assertEqual(existing.read_bytes(), b'original diagnostic')
            link = root / 'link'
            link.symlink_to(root / 'missing')
            self.assertNotEqual(run(link).returncode, 0)
            self.assertTrue(link.is_symlink())


if __name__ == '__main__':
    unittest.main()
