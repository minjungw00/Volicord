"""Actual-process controls with authored values; never invoke external generation."""
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import tempfile
import unittest
from unittest.mock import patch

import inputs as i
import invocations as v
from input_self_test import spec

CONTROL_ARTIFACT_ROOT = None


class InvocationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='volicord-explanation-control-')
        self.addCleanup(self.temp.cleanup)
        temporary = Path(self.temp.name)
        self.root = temporary
        if CONTROL_ARTIFACT_ROOT is not None:
            self.root = CONTROL_ARTIFACT_ROOT / self._testMethodName
            self.root.mkdir(parents=True, mode=0o700)
        self.workspace = temporary / 'fresh'; self.workspace.mkdir()
        self.env = v.environment(self.workspace)
        self.count = 0

    def run_process(self, command, stdin=b'', timeout=1, stream_bytes=16384):
        self.count += 1
        return v.capture([sys.executable, '-B', '-c', command], cwd=self.workspace, env=self.env,
                         output=self.root / f'process-{self.count}', timeout=timeout,
                         stream_bytes=stream_bytes, cleanup_seconds=0.4, stdin=stdin)

    def test_exact_streams_input_numeric_failure_and_signal(self):
        result = self.run_process('import sys; data=sys.stdin.buffer.read(); sys.stdout.buffer.write(data); '
                                  'sys.stderr.buffer.write(b"err\\x00\\n"); raise SystemExit(23)', b'out\x00\n')
        self.assertEqual(result['exit_code'], 23)
        self.assertEqual(result['outcome'], 'failed')
        self.assertEqual(Path(result['stdout']['path']).read_bytes(), b'out\x00\n')
        self.assertEqual(Path(result['stderr']['path']).read_bytes(), b'err\x00\n')
        self.assertEqual(Path(result['stdin']['path']).read_bytes(), b'out\x00\n')
        self.assertIsNotNone(result['executable']['sha256'])
        self.assertTrue(result['cleanup']['complete'])
        result = self.run_process('import os,signal; os.kill(os.getpid(),signal.SIGTERM)')
        self.assertIsNone(result['exit_code'])
        self.assertEqual(result['signal_number'], 15)
        self.assertEqual(result['returncode'], -15)

    def test_timeout_descendants_and_leader_exit_cleanup(self):
        for suffix in ('time.sleep(10)', 'raise SystemExit(0)'):
            result = self.run_process('import subprocess,sys,time; '
                'subprocess.Popen([sys.executable,"-c","import time; time.sleep(10)"]); '
                + suffix, timeout=0.15)
            self.assertTrue(result['cleanup']['complete'])
            self.assertIn(15, result['cleanup']['signals_sent'])
            self.assertLess(result['duration_seconds'], 3)
            if suffix.startswith('time.'):
                self.assertEqual(result['stop_cause'], 'timeout')
            else:
                self.assertEqual(result['exit_code'], 0)

    def test_bounded_output_and_spawn_failure(self):
        result = self.run_process('import os; os.write(1,b"x"*100000); import time; time.sleep(10)', stream_bytes=128)
        self.assertEqual(result['stop_cause'], 'stream_budget')
        self.assertFalse(result['streams_complete'])
        self.assertEqual(result['retained_stream_bytes'], 128)
        self.assertTrue(result['cleanup']['complete'])
        result = v.capture([str(self.root / 'missing')], cwd=self.workspace, env=self.env,
                           output=self.root / 'missing-result', timeout=1, stream_bytes=128)
        self.assertEqual(result['outcome'], 'spawn_failed')
        self.assertEqual(result['spawn_error']['errno'], 2)
        self.assertIsNone(result['exit_code'])

    def test_fresh_process_allowed_forbidden_instruction_and_environment_controls(self):
        source = self.workspace / 'source.txt'
        source.write_bytes((v.HERE / 'fixtures/source.txt').read_bytes())
        reader_spec = spec(source)
        reader_spec['entries'][0]['asset'] = i.binding(source)
        (self.workspace / 'reader-input.json').write_bytes(i.encoded(reader_spec))
        for name in ('inputs.py', 'neutral_process.py'):
            shutil.copyfile(v.HERE / name, self.workspace / name)
        outside = self.root / 'private-answer.txt'; outside.write_text('authored forbidden canary\n')
        with patch.dict(os.environ, {'EXPLANATION_INHERITED_CANARY': 'must-not-arrive'}):
            result = v.capture([sys.executable, '-B', str(self.workspace / 'neutral_process.py')],
                               cwd=self.workspace, env=self.env, output=self.root / 'neutral',
                               timeout=2, stream_bytes=16384,
                               stdin=i.encoded({'outside_file': str(outside)}))
        self.assertEqual(result['exit_code'], 0)
        response = json.loads(Path(result['stdout']['path']).read_bytes())
        self.assertEqual(response['kind'], 'deterministic_process_control_not_model')
        self.assertEqual(response['allowed']['bytes'], 89)
        self.assertEqual(response['forbidden_read_count'], 4)
        self.assertEqual(response['reads'], 1)
        self.assertTrue(response['instruction_like_text_retained_as_data'])
        self.assertFalse(response['inherited_canary_present'])
        self.assertFalse(response['hook_marker_present'])
        self.assertFalse(response['inherited_config_present'])
        self.assertTrue(response['direct_filesystem_read_possible'])  # Honest non-hermetic boundary.
        self.assertEqual(Path(result['stderr']['path']).read_bytes(), b'neutral-stderr\n')

    def test_unauthorized_and_unsupported_dispatch_make_no_process(self):
        source = self.root / 'source.txt'; source.write_text('authored\n')
        spec_path = self.root / 'spec.json'; spec_path.write_bytes(i.encoded(spec(source)))
        manifest = i.freeze(spec_path, self.root / 'inputs', self.root)
        runtime = {'language': 'ko', 'model': 'explicit-model', 'reasoning_effort': 'high',
                   'destination': 'explicit-destination', 'authorization': None}
        with patch.object(v.subprocess, 'Popen', side_effect=AssertionError('unauthorized dispatch')):
            result = v.freeze_run(manifest, runtime, 'direct', 'archive_diagnostic', self.root / 'run')
        self.assertEqual(result['dispatch'], 'not_run')
        self.assertIsNone(result['generation_output'])
        self.assertIn('current_destination_purpose_source_authorization_missing', result['blockers'])
        self.assertIn('live_host_retrieval_and_neutral_model_controls_unverified', result['blockers'])
        self.assertIsNone(result['tokens']); self.assertIsNone(result['price'])
        with patch.object(v.subprocess, 'Popen', side_effect=AssertionError('unsupported dispatch')):
            result = v.freeze_run(manifest, runtime, 'current', 'product', self.root / 'current')
        self.assertIn('cutoff_bound_current_prepare_missing', result['blockers'])

    def test_snapshot_records_only_explicit_nonsecret_configuration(self):
        path = self.root / 'config.toml'
        path.write_text('model="configured"\nmodel_reasoning_effort="high"\n'
                        '[model_providers.example]\nenv_key="PRIVATE_CREDENTIAL_VARIABLE"\n')
        result = v.runtime_snapshot(path)
        self.assertEqual(result['values'], {'model': 'configured', 'model_reasoning_effort': 'high'})
        self.assertEqual(result['effective_defaults'], 'unobserved')
        self.assertNotIn('PRIVATE_CREDENTIAL_VARIABLE', json.dumps(result))


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument('--retain', type=Path)
    args, remaining = parser.parse_known_args()
    if args.retain:
        CONTROL_ARTIFACT_ROOT = args.retain.resolve()
        CONTROL_ARTIFACT_ROOT.mkdir(parents=True, mode=0o700, exist_ok=False)
    unittest.main(argv=[sys.argv[0], *remaining])
