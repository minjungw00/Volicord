"""Actual Git-object witnesses, verified after the originating Git is unavailable."""
import base64
import copy
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import collector_provenance as proof
import review_operations as operations


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.repository = self.parent / 'repository'
        self.repository.mkdir()
        self.bodies = {'rebuild/validation/reader.py': b'# independent producer\n',
            'rebuild/docs/contract.txt': '독립 원본\n'.encode()}
        for name, body in self.bodies.items():
            p = self.repository / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(body)
        (self.repository / 'unrelated.txt').write_text('unrelated body must not be retained')
        self.git('init', '-q')
        self.git('add', '.')
        self.git('-c', 'user.name=Authored fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-qm', 'independent witness fixture')
        self.value = {'run_id': 'a' * 64,
            'collector_revision': self.git('rev-parse', 'HEAD').decode().strip(),
            'producers': {n: {'bytes': len(b), 'sha256': operations.digest(b)} for n, b in self.bodies.items()}}
        self.root = self.parent / 'retained'
        for name, body in self.bodies.items():
            p = self.root / 'collection/producers' / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(body)
        self.witness = proof.create(self.root, self.value, self.repository)

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.repository, stderr=subprocess.DEVNULL)

    def test_copied_bytes_verify_without_git_source_or_subprocess(self):
        copied = self.parent / 'copied'
        shutil.copytree(self.root, copied)
        shutil.rmtree(self.repository)
        shutil.rmtree(self.root)
        self.assertNotIn(b'unrelated body must not be retained', operations.encoded(self.witness))
        with patch.object(subprocess, 'run', side_effect=AssertionError('copied read used Git')), \
             patch.object(subprocess, 'Popen', side_effect=AssertionError('copied read spawned Git')):
            result = proof.verify(copied, self.value, json.loads(operations.encoded(self.witness)))
        self.assertEqual(result['producer_files'], 2)
        self.assertFalse(result['original_git_required'])

    def test_tampered_missing_foreign_and_unrelated_witnesses_fail(self):
        root_tree = base64.b64decode(self.witness['commit']).split(b'\n', 1)[0][5:].decode()
        cases = []
        for field, value in [('collection_run_id', 'b' * 64), ('collector_revision', 'b' * 40),
                ('schema_version', True), ('commit', 'not base64!')]:
            changed = copy.deepcopy(self.witness)
            changed[field] = value
            cases.append((field, changed))
        changed = copy.deepcopy(self.witness)
        changed['trees'].pop(root_tree)
        cases.append(('missing tree', changed))
        changed = copy.deepcopy(self.witness)
        changed['trees'][root_tree] = base64.b64encode(b'changed').decode()
        cases.append(('wrong tree hash', changed))
        changed = copy.deepcopy(self.witness)
        changed['trees'][proof.oid('tree', b'')] = ''
        cases.append(('unrelated tree', changed))
        for label, witness in cases:
            with self.subTest(case=label), self.assertRaises(ValueError):
                proof.verify(self.root, self.value, witness)
        p = self.root / 'collection/producers/rebuild/validation/reader.py'
        p.write_bytes(b'foreign producer')
        with self.assertRaisesRegex(ValueError, 'producer bytes'):
            proof.verify(self.root, self.value, self.witness)

    def test_rehashed_retained_producer_cannot_replace_git_blob(self):
        body = b'# an attacker rehashed this producer\n'
        value = copy.deepcopy(self.value)
        value['producers']['rebuild/validation/reader.py'] = {
            'bytes': len(body), 'sha256': operations.digest(body)}
        p = self.root / 'collection/producers/rebuild/validation/reader.py'
        p.write_bytes(body)
        with self.assertRaisesRegex(ValueError, 'producer blob'):
            proof.verify(self.root, value, self.witness)

    def test_git_symlink_is_not_a_retained_executable_producer(self):
        name = 'rebuild/validation/reader.py'
        p = self.repository / name
        p.unlink()
        p.symlink_to('../../unrelated.txt')
        self.git('add', name)
        self.git('-c', 'user.name=Authored fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-qm', 'symlink producer refusal')
        value = copy.deepcopy(self.value)
        value['collector_revision'] = self.git('rev-parse', 'HEAD').decode().strip()
        body = b'../../unrelated.txt'
        (self.root / 'collection/producers' / name).write_bytes(body)
        value['producers'][name] = {'bytes': len(body), 'sha256': operations.digest(body)}
        with self.assertRaisesRegex(ValueError, 'producer blob or mode'):
            proof.create(self.root, value, self.repository)

    def test_oversized_commit_is_rejected_by_a_capped_read(self):
        message = self.parent / 'large-message.txt'
        message.write_text('x' * 8192)
        self.git('-c', 'user.name=Authored fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '--allow-empty', '-qF', str(message))
        value = copy.deepcopy(self.value)
        value['collector_revision'] = self.git('rev-parse', 'HEAD').decode().strip()
        original = subprocess.Popen
        sizes, children = [], []
        class CappedReader:
            def __init__(self, stream):
                self.stream = stream
            def read(self, size):
                sizes.append(size)
                if not 0 <= size <= 1025:
                    raise AssertionError('Git output was not bounded before reading')
                return self.stream.read(size)
            def close(self):
                self.stream.close()
        def observe(*args, **kwargs):
            process = original(*args, **kwargs)
            process.stdout = CappedReader(process.stdout)
            children.append(process)
            return process
        with patch.object(operations, 'MAX_FILE_BYTES', 1024), \
             patch.object(subprocess, 'Popen', side_effect=observe):
            with self.assertRaisesRegex(ValueError, 'remaining metadata bound'):
                proof.create(self.root, value, self.repository)
        self.assertTrue(sizes)
        self.assertTrue(all(child.poll() is not None for child in children))


def run_contract_tests():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(ProvenanceTests))
    if not result.wasSuccessful():
        raise AssertionError('collector provenance controls failed')


if __name__ == '__main__':
    unittest.main()
