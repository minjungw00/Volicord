#!/usr/bin/env python3
"""Pinned engine/runtime notices; actual installed rendering is a separate check."""
from pathlib import Path
import json
import shutil
import tempfile
import unittest

import build_boundary as boundary


class LayoutDependencies(unittest.TestCase):
    def test_installed_notices_match_verified_bytes_and_uninstall_preserves_other_files(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix = Path(directory)
            files = boundary.install_layout_notices(boundary.ROOT, prefix, {'path': 'controlled-unit-input', 'sha256': 'test-only'})
            dest = prefix / 'share/volicord/licenses/diagram-layout'
            manifest = json.loads((dest / 'manifest.json').read_text())
            self.assertEqual(set(files), set(boundary.LAYOUT_NOTICES) | {'manifest.json'})
            for name, expected in manifest['notices'].items():
                self.assertEqual(boundary.digest(dest / name), expected)
            self.assertIn('elkjs/0.11.1;', manifest['engine'])
            (dest / 'user-note.txt').write_text('unrelated reader-owned file')
            boundary.uninstall_layout_notices(prefix)
            self.assertEqual(list(dest.iterdir()), [dest / 'user-note.txt'])

    def test_tampered_engine_and_unpinned_runtime_fail_before_packaging(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            viewer = root / 'rebuild/crates/volicord-viewer'
            viewer.mkdir(parents=True)
            original = boundary.ROOT / 'rebuild/crates/volicord-viewer'
            shutil.copytree(original / 'vendor', viewer / 'vendor')
            shutil.copytree(original / 'src', viewer / 'src')
            shutil.copyfile(original / 'Cargo.toml', viewer / 'Cargo.toml')
            shutil.copyfile(boundary.ROOT / 'rebuild/Cargo.lock', root / 'rebuild/Cargo.lock')
            boundary.layout_dependency_metadata(root)
            engine = viewer / 'vendor/elkjs/elk.bundled.js'
            data = engine.read_bytes()
            engine.write_bytes(data + b'\n// tampered dependency control')
            with self.assertRaises(ValueError):
                boundary.layout_dependency_metadata(root)
            engine.write_bytes(data)
            cargo = viewer / 'Cargo.toml'
            cargo.write_text(cargo.read_text().replace('rquickjs = "=0.11.0"', 'rquickjs = "0.11"'))
            with self.assertRaises(ValueError):
                boundary.layout_dependency_metadata(root)


if __name__ == '__main__':
    unittest.main()
