"""Review delivery integrity and separation; no Product or human-quality oracle."""
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import package_product_review as package


def case():
    return {'key': 'recorded', 'project_name': 'Example', 'project': '01' * 16,
        'evidence_class': 'record-only', 'origin': 'historical canonical record replay'}


class ReviewPackageTests(unittest.TestCase):
    def test_entry_has_questions_without_case_answers_or_implicit_normal_success(self):
        page = package.reader_entry([case()])
        self.assertIn('record-only', page)
        self.assertIn('pending', page)
        self.assertIn('technical.html', page)
        for leaked_answer in ['compact', '1 passed', '기록되지 않은 이유를', '과거 검증 실패', '의도한 차이']:
            self.assertNotIn(leaked_answer, page)
        self.assertNotIn('<script', page)

    def test_origin_is_required_and_unsafe_case_keys_are_rejected(self):
        for field, value in [('origin', ''), ('evidence_class', 'approved'), ('key', '../private')]:
            c = case(); c[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                package.reader_entry([c])

    def test_only_product_outputs_are_copied_and_reader_keys_never_travel(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); observed = root / 'observed'; observed.mkdir()
            (observed / 'recorded-offline.html').write_text('actual Product bytes')
            (observed / 'source-first-expectations.json').write_text('private evaluator answer')
            (observed / 'runtime').mkdir()
            (observed / 'runtime' / 'private.sqlite3').write_text('not a review export')
            package.copy_product_outputs(observed, root / 'package', [case()])
            self.assertEqual([p.name for p in (root / 'package').iterdir()], ['recorded-offline.html'])
            self.assertEqual((root / 'package' / 'recorded-offline.html').read_bytes(),
                (observed / 'recorded-offline.html').read_bytes())

    def test_moved_package_verifies_and_tampered_added_or_missing_files_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); original = root / 'original'; original.mkdir()
            (original / 'index.html').write_text(package.reader_entry([case()]))
            (original / 'manifest.json').write_text(json.dumps({'candidate': 'a' * 40,
                'human_acceptance': 'pending', 'files': {'index.html': package.digest(original / 'index.html')}}))
            moved = root / 'moved'; shutil.copytree(original, moved)
            self.assertEqual(package.verify_package(moved)['human_acceptance'], 'pending')
            (moved / 'extra.json').write_text('unbound')
            with self.assertRaisesRegex(ValueError, 'membership'):
                package.verify_package(moved)
            (moved / 'extra.json').unlink()
            (moved / 'index.html').write_text('changed')
            with self.assertRaisesRegex(ValueError, 'bytes/path'):
                package.verify_package(moved)
            (moved / 'index.html').unlink()
            with self.assertRaisesRegex(ValueError, 'membership'):
                package.verify_package(moved)


if __name__ == '__main__':
    unittest.main()
