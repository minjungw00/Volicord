"""Independent campaign-to-reader controls; no generated or editorial answers."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import archive
import inputs as i
from source_tools import Surface


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.campaign = self.root / 'campaign'
        self.repository = self.campaign / 'journeys/example/repository'
        self.repository.mkdir(parents=True)
        self.git('init', '-q')
        (self.repository / 'sample.py').write_bytes(b'value = 1\n')
        (self.repository / 'caller.py').write_bytes(b'import sample\n')
        self.git('add', '.')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                 'commit', '-qm', 'Independent input')
        revision = self.git('rev-parse', 'HEAD').strip()
        self.inventory = {}
        self.capture_path = 'slots/work-a/evidence/start.rollout.jsonl'
        patch = ('*** Begin Patch\n*** Update File: sample.py\n@@\n-value = 1\n'
                 '+value = 2\n*** End Patch')
        events = [
            ('09:00:00', 'session_meta', {'id': 'session-a', 'session_id': 'session-a',
                'cwd': str(self.repository), 'source': 'cli', 'originator': 'fixture',
                'cli_version': 'fixture', 'thread_source': 'user', 'git': {'commit_hash': revision}}),
            ('09:00:01', 'event_msg', {'type': 'task_started', 'turn_id': 'turn-a'}),
            ('09:00:02', 'event_msg', {'type': 'user_message', 'message': 'Inspect the value change.'}),
            ('09:30:00', 'response_item', {'type': 'custom_tool_call', 'name': 'functions.exec',
                'call_id': 'patch-a', 'input': 'await tools.apply_patch(' + json.dumps(patch) + ');'}),
            ('10:00:00', 'event_msg', {'type': 'task_completed', 'turn_id': 'turn-a'}),
        ]
        self.write(self.capture_path, b''.join(json.dumps({'timestamp': '2026-10-08T' + stamp + 'Z',
            'type': kind, 'payload': payload}).encode() + b'\n' for stamp, kind, payload in events))
        self.write('operator/start.txt', b'Inspect the value change.\n')

        def table(name, columns, rows):
            return {'name': name, 'columns': columns,
                    'rows': [[{'value': cell} for cell in row] for row in rows]}

        self.bundle = {'payload': {'tables': [
            table('context_items', ['id', 'project_id', 'recorded_at'], [['a', 'p', 1], ['b', 'p', 1]]),
            table('checkpoints', ['id', 'project_id', 'work_item_id', 'recorded_at'],
                  [['checkpoint-a', 'p', 'a', 2], ['foreign', 'p', 'b', 2],
                   ['future', 'p', 'a', 9999999999999999]]),
        ]}}
        self.bundle_path = 'slots/work-a/context.bundle.json'
        self.write(self.bundle_path, i.encoded(self.bundle))
        self.state_path = 'journeys/example/evidence/repository-state.json'
        self.write(self.state_path, i.encoded({'tracked': [
            {'path': 'sample.py', 'sha256': i.digest(b'value = 2\n')},
            {'path': 'caller.py', 'sha256': i.digest(b'import sample\n')}]}))
        self.manifest = {'candidate_head': '1' * 40, 'candidate_artifacts': {},
            'works': {'work-a': {'journey_id': 'example', 'work_slot_id': 'work-a',
                'project_id': 'p', 'work_item_id': 'a', 'start_session_id': 'session-a',
                'resume_session_id': None, 'bundle_sha256': i.digest(i.encoded(self.bundle)),
                'operator_task_artifacts': {'start': {'path': 'operator/start.txt',
                    'sha256': i.digest(b'Inspect the value change.\n')}}}},
            'journeys': {'example': {'repository_path': str(self.repository),
                'repository_revision': revision, 'work_slot_ids': ['work-a']}}}
        self.publish()

    def git(self, *args):
        return subprocess.run(['git', '-C', str(self.repository), *args], capture_output=True,
                              text=True, check=True).stdout

    def write(self, name, data):
        path = self.campaign / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        self.inventory[name] = {'bytes': len(data), 'sha256': i.digest(data)}

    def publish(self):
        (self.campaign / 'campaign.json').write_bytes(i.encoded(self.manifest))
        (self.campaign / 'evidence-inventory.json').write_bytes(i.encoded({'artifacts': self.inventory}))

    def prepare(self, slot='work-a'):
        return archive.prepare(self.root, slot, self.root / 'prepared', '2' * 40)

    def test_original_campaign_slot_reaches_reader_with_independent_states(self):
        path = self.prepare()
        spec = i.verify(path)
        self.assertEqual(spec['scope'], {'project': 'p', 'work': 'a', 'cutoff': '2026-10-08T10:00:00Z'})
        canonical = [json.loads(Path(e['asset']['path']).read_bytes()) for e in spec['entries']
                     if e['role'] == 'canonical_record']
        self.assertEqual([r['row']['id'] for r in canonical], ['a', 'checkpoint-a'])
        files = [e for e in spec['entries'] if e['role'] == 'source']
        self.assertEqual({e['path'] for e in files}, {'sample.py', 'caller.py'})
        selected = [e for e in files if e['path'] == 'sample.py']
        self.assertEqual(len(selected), 2)
        surface = Surface(path, 'archive_diagnostic', {'reads': 8, 'read_bytes': 8192}, self.root / 'reads.jsonl')
        returned = [surface.call('read', {'id': e['id'], 'limit': 100}) for e in selected]
        self.assertEqual([r['result']['text'] for r in returned], ['value = 2\n', 'value = 1\n'])
        self.assertEqual([e['attribution'] for e in selected], ['explicit_work_patch', 'explicit_before_patch'])
        self.assertEqual(selected[1]['locator'], selected[0]['locator'] + ':before')
        self.assertEqual(surface.call('read', {'id': 'evaluator-only'})['status'], 'failed')
        product = Surface(path, 'product', {'reads': 8, 'read_bytes': 8192}, self.root / 'product-reads.jsonl')
        self.assertEqual(product.call('read', {'id': selected[0]['id']})['status'], 'failed')
        self.assertIn('current_prepare_at_cutoff_missing', [o['kind'] for o in spec['inventory_boundary']['omissions']])

    def test_changed_original_capture_is_rejected_before_preparation(self):
        path = self.campaign / self.capture_path
        path.write_bytes(path.read_bytes().replace(b'Inspect the value change.', b'Invent an outcome instead.'))
        with self.assertRaisesRegex(i.InputError, 'historical artifact'):
            self.prepare()
        self.assertFalse((self.root / 'prepared').exists())

    def test_changed_bundle_and_source_witness_are_rejected(self):
        for name in (self.bundle_path, self.state_path):
            with self.subTest(name=name):
                path = self.campaign / name
                original = path.read_bytes()
                path.write_bytes(original + b' ')
                with self.assertRaisesRegex(i.InputError, 'historical artifact'):
                    self.prepare()
                path.write_bytes(original)
                self.assertFalse((self.root / 'prepared').exists())

    def test_absent_inventory_and_unlisted_capture_are_rejected(self):
        path = self.campaign / 'evidence-inventory.json'
        path.unlink()
        with self.assertRaisesRegex(i.InputError, 'historical inventory'):
            self.prepare()
        del self.inventory[self.capture_path]
        self.publish()
        with self.assertRaisesRegex(i.InputError, 'historical artifact'):
            self.prepare()

    def test_foreign_work_is_rejected(self):
        self.manifest['works']['work-a']['work_item_id'] = 'absent'
        self.publish()
        with self.assertRaisesRegex(i.InputError, 'Work absent'):
            self.prepare()

    def test_foreign_session_repository_and_revision_are_rejected(self):
        path = self.campaign / self.capture_path
        original = path.read_bytes()
        events = [json.loads(line) for line in original.splitlines()]
        for field, value in (('cwd', '/other/repository'), ('git', {'commit_hash': 'f' * 40})):
            with self.subTest(field=field):
                changed = copy.deepcopy(events)
                changed[0]['payload'][field] = value
                self.write(self.capture_path, b''.join(json.dumps(e).encode() + b'\n' for e in changed))
                self.publish()
                with self.assertRaisesRegex(i.InputError, 'repository/session binding'):
                    self.prepare()
                self.assertFalse((self.root / 'prepared').exists())
        self.write(self.capture_path, original)
        self.manifest['works']['work-a']['start_session_id'] = 'other-session'
        self.publish()
        with self.assertRaisesRegex(i.InputError, 'wrong historical sessions'):
            self.prepare()

    def test_missing_source_correspondence_stays_unavailable(self):
        self.write(self.state_path, i.encoded({'tracked': [
            {'path': 'sample.py', 'sha256': i.digest(b'value = 99\n')},
            {'path': 'caller.py', 'sha256': i.digest(b'import sample\n')}]}))
        self.publish()
        path = self.prepare()
        spec = i.verify(path)
        selected = [e for e in spec['entries'] if e['path'] == 'sample.py']
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0]['representation'], 'unavailable')
        self.assertIsNone(selected[0]['asset'])
        surface = Surface(path, 'archive_diagnostic', {'reads': 2, 'read_bytes': 1000}, self.root / 'reads.jsonl')
        self.assertEqual(surface.call('read', {'id': selected[0]['id'], 'limit': 100})['outcome'], 'source_unavailable')

    def test_changed_cutoff_and_frozen_source_remain_rejected(self):
        path = self.prepare()
        spec = i.verify(path)
        invalid = copy.deepcopy(spec)
        invalid['scope']['cutoff'] = '2026-10-08T08:00:00Z'
        with self.assertRaisesRegex(i.InputError, 'after cutoff'):
            i.validate(invalid)
        source = next(e for e in spec['entries'] if e['role'] == 'source')
        Path(source['asset']['path']).write_bytes(b'changed\n')
        with self.assertRaisesRegex(i.InputError, 'stale preparation'):
            i.verify(path)


if __name__ == '__main__':
    unittest.main()
