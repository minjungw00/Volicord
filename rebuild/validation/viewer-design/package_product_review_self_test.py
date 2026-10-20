"""Review delivery integrity and separation; no Product or human-quality oracle."""
import json
import hashlib
import struct
import zipfile
import zlib
from pathlib import Path
import shutil
import tempfile
import subprocess
import sys
import unittest

import package_product_review as package


def case():
    return {'key': 'recorded', 'project_name': 'Example', 'project': '01' * 16,
        'evidence_class': 'record-only', 'origin': 'historical canonical record replay'}


HTML = b'<!doctype html><meta charset="utf-8"><p>Bounded synthetic export</p>'

def seal(root):
    meta = {'candidate': 'a' * 40, 'outcome': 'passed', 'cases': [case()],
        'commands': [{'label': 'export', 'exit': 0, 'argv': ['private-command-body'], 'error': 'private-diagnostic'}],
        'files': {p.name: {'sha256': hashlib.sha256(p.read_bytes()).hexdigest(), 'bytes': p.stat().st_size}
            for p in root.iterdir() if p.is_file() and p.name != 'result.json'}}
    (root / 'result.json').write_text(json.dumps(meta))
    return meta


def manifest(root, candidate='a' * 40):
    (root / 'manifest.json').write_text(json.dumps({'candidate': candidate, 'files': {
        p.relative_to(root).as_posix(): package.digest(p) for p in root.rglob('*') if p.is_file() and p.name != 'manifest.json'}}))


def png():
    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 1, 1, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(b'\0\0\0\0')) + chunk(b'IEND', b'')


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
            (observed / 'recorded-offline.html').write_text('<!doctype html><p>Actual bounded Product export</p>')
            seal(observed)
            (observed / 'source-first-expectations.json').write_text('private evaluator answer')
            (observed / 'runtime').mkdir()
            (observed / 'runtime' / 'private.sqlite3').write_text('not a review export')
            package.copy_product_outputs(observed, root / 'package', [case()])
            self.assertEqual(sorted(p.name for p in (root / 'package').iterdir()), ['recorded-offline.html', 'result.json'])
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
            with self.assertRaisesRegex(ValueError, 'membership|role'):
                package.verify_package(moved)
            (moved / 'extra.json').unlink()
            (moved / 'index.html').write_text((moved / 'index.html').read_text() + 'changed')
            with self.assertRaisesRegex(ValueError, 'bytes/path'):
                package.verify_package(moved)
            (moved / 'index.html').unlink()
            with self.assertRaisesRegex(ValueError, 'membership|role'):
                package.verify_package(moved)


class BoundaryAttackTests(unittest.TestCase):
    def observed(self, root):
        source = root / 'observed'; source.mkdir()
        (source / 'recorded-offline.html').write_bytes(HTML)
        return source

    def test_raw_runtime_source_secret_and_log_neighbors_stay_private(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); source = self.observed(root)
            fixtures = {'private.sqlite3': b'SQLite format 3\0private', 'recorded-viewer.stderr': b'AWS_SECRET_ACCESS_KEY=synthetic-credential',
                'raw-source.rs': b'private source body', '.env': b'password=synthetic-credential', 'recorded-public-status.json': b'{"content":"private Derived body"}',
                'recorded-repository-1440.html': b'<script>live authority</script>'}
            for name, data in fixtures.items(): (source / name).write_bytes(data)
            seal(source); original = {p.name: p.read_bytes() for p in source.iterdir()}
            package.copy_product_outputs(source, root / 'delivery', [case()])
            self.assertEqual(set(p.name for p in (root / 'delivery').iterdir()), {'recorded-offline.html', 'result.json'})
            public = (root / 'delivery' / 'result.json').read_text()
            for marker in ['private-command-body', 'private-diagnostic', 'argv', 'error', 'content']:
                self.assertNotIn(marker, public)
            self.assertEqual(original, {p.name: p.read_bytes() for p in source.iterdir()})

    def test_disguised_database_credentials_and_raw_json_are_rejected(self):
        for data in [b'SQLite format 3\0private', b'<!doctype html><p>AWS_SECRET_ACCESS_KEY=synthetic-credential</p>', b'<!doctype html><p>-----BEGIN PRIVATE KEY-----</p>', b'{"body":"raw Source"}']:
            with self.subTest(data=data[:20]), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); source = self.observed(root); (source / 'recorded-offline.html').write_bytes(data); seal(source)
                with self.assertRaises(ValueError): package.copy_product_outputs(source, root / 'delivery', [case()])
                self.assertEqual((source / 'recorded-offline.html').read_bytes(), data)

    def test_sealed_output_cannot_change_after_review(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); source = self.observed(root); seal(source)
            (source / 'recorded-offline.html').write_bytes(HTML + b'<p>changed</p>')
            with self.assertRaisesRegex(ValueError, 'since browser review'):
                package.copy_product_outputs(source, root / 'delivery', [case()])

    def test_symlink_members_parent_directories_and_roots_are_refused(self):
        boundary = package.boundary
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); source = self.observed(root); seal(source)
            outside = root / 'outside'; outside.mkdir(); (outside / 'private.html').write_bytes(HTML)
            (source / 'link').symlink_to(outside, target_is_directory=True)
            (root / 'linked-root').symlink_to(source, target_is_directory=True)
            (source / 'recorded-offline.html').unlink(); (source / 'recorded-offline.html').symlink_to(outside / 'private.html')
            for base, name in [(source, 'recorded-offline.html'), (source, 'link/private.html'), (root / 'linked-root', 'result.json')]:
                with self.subTest(name=name), self.assertRaises(ValueError): boundary.read_member(base, name)
            with self.assertRaises(ValueError): package.copy_product_outputs(source, root / 'delivery', [case()])

    def test_traversal_absolute_and_noncanonical_member_names_are_refused(self):
        for name in ['../private.sqlite3', '/outside/private.html', 'a/../../private.html', 'a\\private.html', 'a//private.html', './private.html', 'file:private.html', '%2e%2e/private.html', 'a?path=private']:
            with self.subTest(name=name), self.assertRaises(ValueError): package.boundary.relative(name)

    def test_actual_png_is_accepted_and_disguised_png_and_trailing_payload_are_refused(self):
        package.boundary.validate('recorded-overview-1440.png', png())
        for data in [b'SQLite format 3\0private', png() + b'private Source trailer']:
            with self.assertRaises(ValueError): package.boundary.validate('recorded-overview-1440.png', data)

    def test_active_or_external_html_has_no_delivery_authority(self):
        for tail in ['<script>fetch("private")</script>', '<form action="/memory/forget"></form>', '<img src="https://foreign.invalid/source">', '<div onclick="capture()">x</div>', '<style>body{background:url(file:/private)}</style>']:
            with self.subTest(tail=tail), self.assertRaises(ValueError): package.boundary.validate('index.html', b'<!doctype html>' + tail.encode())

    def test_supplements_exclude_raw_logs_and_unsealed_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); source = root / 'source'; source.mkdir()
            (source / 'product-snapshot.html').write_bytes(HTML)
            (source / 'review.html').write_bytes(b'<!doctype html><a href="product-snapshot.html">Snapshot</a>')
            (source / 'screen.png').write_bytes(png()); (source / 'raw.stderr').write_text('private log')
            seal(source); package.copy_supplement(source, root / 'supplement')
            self.assertEqual(set(p.name for p in (root / 'supplement').iterdir()), {'result.json', 'product-snapshot.html', 'review.html', 'screen.png'})

    def test_member_and_total_bounds_are_enforced_without_copying(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); source = self.observed(root); seal(source)
            with self.assertRaises(ValueError): package.boundary.read_member(source, 'recorded-offline.html', 4)
            with self.assertRaises(ValueError): package.boundary.validate('index.html', b' ' * (package.boundary.MEMBER_BYTES + 1))

    def test_candidate_manifest_archive_and_original_preservation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); delivery = root / 'delivery'; delivery.mkdir(); (delivery / 'index.html').write_bytes(HTML); manifest(delivery)
            archive = root / 'review.zip'; original = (delivery / 'index.html').read_bytes()
            package.boundary.write_archive(delivery, archive, 'a' * 40)
            self.assertEqual(package.boundary.verify_archive(archive, 'a' * 40)['candidate'], 'a' * 40)
            with self.assertRaises(ValueError): package.boundary.verify_archive(archive, 'b' * 40)
            with zipfile.ZipFile(archive) as z:
                self.assertEqual(set(z.namelist()), {'index.html', 'manifest.json'})
            prefixed = root / 'prefixed.zip'; prefixed.write_bytes(b'private raw Source' + archive.read_bytes())
            with self.assertRaises(ValueError): package.boundary.verify_archive(prefixed)
            self.assertEqual((delivery / 'index.html').read_bytes(), original)
            with self.assertRaises(FileExistsError): package.boundary.write_archive(delivery, archive, 'a' * 40)

    def test_archive_traversal_symlinks_duplicates_and_raw_members_are_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for i, name in enumerate(['../index.html', '/index.html', 'runtime.sqlite3', 'private.rs']):
                path = root / f'bad-{i}.zip'
                with zipfile.ZipFile(path, 'w') as z: z.writestr(name, HTML)
                with self.assertRaises(ValueError): package.boundary.verify_archive(path)
            path = root / 'symlink.zip'
            with zipfile.ZipFile(path, 'w') as z:
                info = zipfile.ZipInfo('index.html'); info.create_system = 3; info.external_attr = 0o120777 << 16; z.writestr(info, 'outside')
            with self.assertRaises(ValueError): package.boundary.verify_archive(path)
            path = root / 'duplicate.zip'
            import warnings
            with warnings.catch_warnings():
                warnings.simplefilter('ignore', UserWarning)
                with zipfile.ZipFile(path, 'w') as z: z.writestr('index.html', HTML); z.writestr('index.html', HTML)
            with self.assertRaises(ValueError): package.boundary.verify_archive(path)

    def test_recomputed_manifest_cannot_admit_arbitrary_raw_source_json_or_wrong_candidate(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root / 'index.html').write_bytes(HTML); (root / 'product').mkdir(); (root / 'product/private-source.json').write_text('{"text":"private source"}'); manifest(root)
            with self.assertRaises(ValueError): package.verify_package(root)
            (root / 'product/private-source.json').unlink(); manifest(root, 'b' * 40)
            with self.assertRaises(ValueError): package.verify_package(root, 'a' * 40)


class BuilderIdentityTests(unittest.TestCase):
    def test_actual_builder_archive_is_sanitized_bound_and_preserves_originals(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); observed = root / 'observed'; observed.mkdir()
            (observed / 'recorded-offline.html').write_bytes(HTML)
            (observed / 'recorded-viewer.stderr').write_text('AWS_SECRET_ACCESS_KEY=synthetic-private-key')
            viewer = root / 'viewer'; viewer.write_text('synthetic executable identity only')
            cli = root / 'cli'; cli.write_text('synthetic CLI identity only')
            c = case(); c['runtime'] = '/private/retained/runtime'
            config = root / 'config.json'; config.write_text(json.dumps({'output': str(observed), 'viewer': str(viewer), 'cli': str(cli), 'cases': [c]}))
            head = subprocess.check_output(['git','rev-parse','HEAD'], text=True).strip()
            meta = seal(observed); meta.update(candidate=head, config_sha256=package.digest(config), executable_sha256=package.digest(viewer), cli_sha256=package.digest(cli), driver_sha256=package.digest(Path(package.__file__).with_name('product_review.cjs')), diff_sha256='synthetic-dirty-test', cases=[c])
            (observed / 'result.json').write_text(json.dumps(meta))
            originals = {p.name: p.read_bytes() for p in observed.iterdir()}
            delivery = root / 'delivery'; archive = root / 'review.zip'
            argv = [sys.executable, '-B', package.__file__, '--config', str(config), '--output', str(delivery), '--archive', str(archive), '--early']
            completed = subprocess.run(argv, text=True, capture_output=True)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            manifest = package.boundary.verify_archive(archive, head)
            self.assertNotIn('runtime', manifest['cases'][0])
            with zipfile.ZipFile(archive) as z:
                combined = b''.join(z.read(n) for n in z.namelist())
                for private in [b'synthetic-private-key', b'private-command-body', b'/private/retained/runtime']:
                    self.assertNotIn(private, combined)
                self.assertTrue(all(Path(n).suffix in {'.html', '.json', '.png'} for n in z.namelist()))
            self.assertEqual(originals, {p.name: p.read_bytes() for p in observed.iterdir()})
            meta['candidate'] = 'b' * 40; (observed / 'result.json').write_text(json.dumps(meta))
            failed = subprocess.run([sys.executable, '-O', '-B', package.__file__, '--config', str(config), '--output', str(root / 'rejected'), '--early'], text=True, capture_output=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertFalse((root / 'rejected').exists())

    def test_output_cannot_overlap_the_private_input_tree(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root/'index.html').write_bytes(HTML); manifest(root)
            with self.assertRaises(ValueError): package.boundary.write_archive(root, root/'review.zip', 'a' * 40)


if __name__ == '__main__':
    unittest.main()
