"""Repository inventory must prune excluded trees without narrowing admitted files."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import harness


class RepositoryInventoryTests(unittest.TestCase):
    def fixture(self, root):
        names = ['README.md', '.visible/config.json', 'src/a.rs', 'src/a.py', 'src/a.js',
            'src/a.ts', 'src/a.java', 'src/a.c', 'src/a.cpp', 'src/name with space.txt']
        for name in names:
            p = root / name; p.parent.mkdir(parents=True, exist_ok=True); p.write_text('fixture')
        for name in sorted(harness.IGNORED_PARTS):
            p = root / name / 'nested/hidden.py'; p.parent.mkdir(parents=True); p.write_text('excluded')
            p = root / 'src/nested' / name / 'hidden.rs'; p.parent.mkdir(parents=True, exist_ok=True); p.write_text('excluded')
        # The old policy also excludes files with an ignored basename, not only directories.
        (root / 'src/build').write_text('excluded file')
        (root / 'link.py').symlink_to(root / 'src/a.py')
        (root / 'dangling.rs').symlink_to(root / 'missing')
        (root / 'directory-link').symlink_to(root / 'src', target_is_directory=True)
        return set(names) | {'link.py'}

    def test_complete_file_membership_preserves_symlinks_and_all_language_roles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); expected = self.fixture(root)
            self.assertEqual({p.relative_to(root).as_posix() for p in harness.repository_files(root)}, expected)

    def test_ignored_trees_are_never_entered_or_statted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); expected = self.fixture(root); visited = []
            scandir, stat = os.scandir, Path.stat
            def allowed(path):
                relative = Path(path).relative_to(root)
                self.assertFalse(set(relative.parts) & harness.IGNORED_PARTS,
                    'attempted access inside an excluded subtree: ' + str(relative))
            def scan(path):
                allowed(path); visited.append(Path(path).relative_to(root).as_posix()); return scandir(path)
            def metadata(path, *args, **kwargs):
                allowed(path); return stat(path, *args, **kwargs)
            with patch('os.scandir', side_effect=scan), patch.object(Path, 'stat', metadata):
                self.assertEqual({p.relative_to(root).as_posix() for p in harness.repository_files(root)}, expected)
            self.assertIn('src/nested', visited)

    def test_root_name_is_not_an_exclusion_and_identity_keeps_polyglot_counts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'target'; root.mkdir(); expected = self.fixture(root)
            with patch.object(harness, 'git_head', return_value='a' * 40), \
                    patch.object(harness, 'git_clean', return_value=True), \
                    patch.object(harness, 'command_output', return_value='fixture://source'):
                identity = harness.repository_identity('volicord',
                    {'path': str(root), 'origin': 'fixture://source', 'revision': 'a' * 40},
                    {'repository_classes': {'volicord': {}}})
            self.assertEqual(identity['status'], 'passed')
            self.assertEqual(identity['file_count'], len(expected))
            self.assertEqual(identity['documentation_file_count'], 1)
            self.assertEqual(identity['official_structural_languages'],
                ['C', 'C++', 'Java', 'JavaScript', 'Python', 'Rust', 'TypeScript'])


def run_contract_tests():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(RepositoryInventoryTests))
    if not result.wasSuccessful():
        raise AssertionError('repository inventory pruning controls failed')
    return {'tests_run': result.testsRun, 'status': 'passed'}


if __name__ == '__main__':
    run_contract_tests()
