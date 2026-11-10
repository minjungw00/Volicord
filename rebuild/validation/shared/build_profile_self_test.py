#!/usr/bin/env python3
"""Actual Cargo/profile/CLI controls, not Product qualification or a latency test."""
from pathlib import Path
import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import build_boundary as boundary


class ProfileControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        output = os.environ.get('VOLICORD_PROFILE_CONTROL_OUTPUT')
        cls.temporary = None if output else tempfile.TemporaryDirectory()
        cls.root = Path(output or cls.temporary.name).resolve()
        if output:
            cls.root.mkdir(parents=True, exist_ok=False)
        cls.builds = []
        for label, release, number, opt in [('dev-a', False, 7, None),
                ('release-a', True, 7, None), ('dev-b', False, 8, None),
                ('dev-optimized', False, 7, 1)]:
            root = cls.root / label
            crate = root / 'rebuild/crates/probe'
            (crate / 'src').mkdir(parents=True)
            (root / 'rebuild/Cargo.toml').write_text('[workspace]\nmembers=["crates/probe"]\nresolver="2"\n')
            if opt is not None:
                with (root / 'rebuild/Cargo.toml').open('a') as stream:
                    stream.write(f'\n[profile.dev]\nopt-level={opt}\n')
            (crate / 'Cargo.toml').write_text('[package]\nname="probe"\nversion="0.1.0"\nedition="2021"\n')
            (crate / 'src/main.rs').write_text(f'fn main() {{ println!("{{}}", {number}); }}\n')
            with patch.dict(os.environ, {'CARGO_TARGET_DIR': str(root / 'target')}), contextlib.redirect_stderr(io.StringIO()):
                artifacts = boundary.build_binaries(root, ('probe',), release=release)
            attempt, = (root / 'rebuild/.local/builds').iterdir()
            cls.builds.append(attempt)
            run = subprocess.run([artifacts['probe']['path']], capture_output=True, text=True, check=True)
            assert run.stdout.strip() == str(number)
        cls.dev, cls.release, cls.other_dev, cls.optimized_dev = cls.builds

    @classmethod
    def tearDownClass(cls):
        if cls.temporary:
            cls.temporary.cleanup()

    def cli(self, left, right):
        return subprocess.run([sys.executable, '-B', str(Path(boundary.__file__).resolve()),
            'compare-profiles', 'probe', str(left), str(right)], capture_output=True, text=True)

    def copied(self, label, original):
        target = self.root / label
        shutil.copytree(original, target)
        return target

    def test_different_binaries_with_matching_profiles_are_only_profile_comparable(self):
        result = self.cli(self.dev, self.other_dev)
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(value['comparison_scope'], 'compiler_and_cargo_profile_only')
        self.assertEqual(value['state'], 'matching')
        self.assertNotEqual(value['left']['executable_sha256'], value['right']['executable_sha256'])
        self.assertEqual(value['left']['cargo_profile']['opt_level'], '0')

    def test_real_release_and_debug_comparison_is_refused(self):
        result = self.cli(self.dev, self.release)
        self.assertEqual(result.returncode, 2)
        self.assertIn('requested_profile mismatch', result.stderr)
        self.assertFalse(result.stdout)
        self.assertEqual(boundary.artifact_profile(self.release, 'probe')['cargo_profile']['opt_level'], '3')

    def test_same_requested_profile_with_different_codegen_is_refused(self):
        result = self.cli(self.dev, self.optimized_dev)
        self.assertEqual(result.returncode, 2)
        self.assertIn('cargo_profile mismatch', result.stderr)

    def test_requested_profile_cannot_be_relabelled_after_compilation(self):
        target = self.copied('changed-request', self.dev)
        path = target / 'command.json'
        value = json.loads(path.read_text())
        value['requested_profile'] = 'release'
        path.write_text(json.dumps(value))
        result = self.cli(target, self.release)
        self.assertEqual(result.returncode, 2)
        self.assertIn('disagrees with command', result.stderr)

    def test_missing_or_relabelled_profile_cannot_supply_a_build_identity(self):
        for label, change in [
            ('missing-profile', lambda value: value.pop('profiles')),
            ('relabelled-profile', lambda value: value['profiles']['probe'].update(opt_level='3')),
        ]:
            target = self.copied(label, self.dev)
            path = target / 'result.json'
            value = json.loads(path.read_text())
            change(value)
            path.write_text(json.dumps(value))
            result = self.cli(target, self.other_dev)
            self.assertEqual(result.returncode, 2)
            self.assertIn('unknown or inconsistent', result.stderr)

    def test_mutated_executable_and_compiler_identity_are_not_matching_evidence(self):
        target = self.copied('changed-binary', self.dev)
        value = json.loads((target / 'result.json').read_text())
        executable = target / 'mutated-probe'
        original = Path(value['artifacts']['probe']['path'])
        executable.write_bytes(original.read_bytes() + b'changed')
        value['artifacts']['probe']['path'] = str(executable)
        (target / 'result.json').write_text(json.dumps(value))
        result = self.cli(target, self.other_dev)
        self.assertEqual(result.returncode, 2)
        self.assertIn('executable changed', result.stderr)
        target = self.copied('changed-compiler', self.dev)
        path = target / 'command.json'
        value = json.loads(path.read_text())
        value['rustc']['toolchain_binaries']['rustc']['sha256'] = '0' * 64
        path.write_text(json.dumps(value))
        result = self.cli(target, self.other_dev)
        self.assertEqual(result.returncode, 2)
        self.assertIn('compiler_sha256 mismatch', result.stderr)


if __name__ == '__main__':
    unittest.main()
