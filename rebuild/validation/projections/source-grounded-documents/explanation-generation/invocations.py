"""Bounded local capture and fail-closed installed-host experiment admission."""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import tempfile
import time
import tomllib

from inputs import append_index, binding, check_binding, digest, encoded, require, verify

HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parents[4]
PROCESS_OWNER = REPOSITORY / 'rebuild/validation/end-to-end/multi-repository/harness.py'
FEATURES_OFF = ('hooks', 'memories', 'apps', 'plugins', 'remote_plugin', 'multi_agent',
                'shell_snapshot', 'skill_search', 'browser_use', 'browser_use_external',
                'computer_use', 'image_generation', 'external_agent_memory_import')
read_stream = os.read


def cleanup_owner():
    # Import existing Linux group cleanup only; never call V11 main/Recorder/gate.
    spec = importlib.util.spec_from_file_location('explanation_process_owner', PROCESS_OWNER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def environment(root):
    """Fresh child environment; no inherited credentials, host IDs, MCP or startup."""
    root = Path(root)
    for name in ('home', 'codex', 'tmp', 'cache'):
        (root / name).mkdir(mode=0o700)
    config = 'approval_policy = "never"\nsandbox_mode = "read-only"\nweb_search = "disabled"\n'
    config += '[features]\n' + ''.join(f'{name} = false\n' for name in FEATURES_OFF)
    (root / 'codex/config.toml').write_text(config)
    return {'HOME': str(root / 'home'), 'CODEX_HOME': str(root / 'codex'),
            'TMPDIR': str(root / 'tmp'), 'XDG_CACHE_HOME': str(root / 'cache'),
            'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'LC_ALL': 'C.UTF-8',
            'PYTHONDONTWRITEBYTECODE': '1', 'GIT_CONFIG_NOSYSTEM': '1',
            'GIT_CONFIG_GLOBAL': '/dev/null'}


def capture(argv, *, cwd, env, output, timeout, stream_bytes, cleanup_seconds=2, stdin=b'',
            response_file=None, response_bytes=None):
    """Exact bounded streams plus numeric process truth, including leader-exit children."""
    require(timeout > 0 and stream_bytes > 0 and cleanup_seconds > 0, 'positive process budgets required')
    require((response_file is None and response_bytes is None) or
            (response_file is not None and type(response_bytes) is int and response_bytes > 0),
            'positive response file budget required')
    output, cwd = Path(output), Path(cwd).resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    input_path = output / 'stdin.bin'; input_path.write_bytes(stdin)
    start = time.monotonic()
    result = {'argv': [str(a) for a in argv], 'cwd': str(cwd),
              'started_at_unix_ns': time.time_ns(), 'stdin': binding(input_path),
              'budgets': {'seconds': timeout, 'stream_bytes': stream_bytes, 'cleanup_seconds': cleanup_seconds},
              'executable': binding(argv[0]) if Path(argv[0]).is_file() else None,
              'process_owner': binding(PROCESS_OWNER), 'adapter': binding(Path(__file__)),
              'environment_keys': sorted(env), 'stop_cause': None, 'spawn_error': None,
              'stream_error': None,
              'exit_code': None, 'signal_number': None, 'returncode': None,
              'cleanup': None, 'streams_complete': True,
              'response_budget': {'path': str(response_file), 'bytes': response_bytes} if response_file is not None else None}
    (output / 'command.json').write_bytes(encoded(result))
    process = None
    streams = {'stdout': output / 'stdout.bin', 'stderr': output / 'stderr.bin'}
    retained, observed = 0, 0
    owner = cleanup_owner()
    interrupted = None
    with input_path.open('rb') as incoming, streams['stdout'].open('wb') as out, streams['stderr'].open('wb') as err:
        try:
            result['spawn_started_monotonic'] = time.monotonic()
            process = subprocess.Popen(result['argv'], cwd=cwd, env=env, stdin=incoming,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            result['spawned_monotonic'] = time.monotonic()
            result['pid'] = process.pid
            with selectors.DefaultSelector() as selector:
                for pipe, sink in ((process.stdout, out), (process.stderr, err)):
                    os.set_blocking(pipe.fileno(), False)
                    selector.register(pipe, selectors.EVENT_READ, sink)
                # EOF is not process exit. Keep observing the watchdog/response
                # file even if a live leader closed both pipes deliberately.
                while selector.get_map() or process.poll() is None:
                    if (result['cleanup'] is None and response_file is not None and Path(response_file).exists()
                            and Path(response_file).stat().st_size > response_bytes):
                        result['stop_cause'] = 'response_budget'
                        result['cleanup'] = owner.cleanup_process_group(process, cleanup_seconds / 2, cleanup_seconds / 2)
                    if result['cleanup'] is None and (process.poll() is not None or time.monotonic() - start >= timeout):
                        if process.poll() is None:
                            result['stop_cause'] = 'timeout'
                        result['cleanup'] = owner.cleanup_process_group(process, cleanup_seconds / 2, cleanup_seconds / 2)
                    for key, _ in selector.select(0.01):
                        data = read_stream(key.fileobj.fileno(), 65536)
                        if not data:
                            selector.unregister(key.fileobj); key.fileobj.close()
                            continue
                        observed += len(data)
                        kept = data[:max(0, stream_bytes - retained)]
                        key.data.write(kept); retained += len(kept)
                        if len(kept) != len(data):
                            result['streams_complete'] = False
                            if result['stop_cause'] is None:
                                result['stop_cause'] = 'stream_budget'
                            if result['cleanup'] is None:
                                result['cleanup'] = owner.cleanup_process_group(process, cleanup_seconds / 2, cleanup_seconds / 2)
                    if result['cleanup'] is not None and time.monotonic() - start > timeout + cleanup_seconds + 1:
                        result['streams_complete'] = False
                        result['stream_drain_timed_out'] = True
                        if result['stop_cause'] is None:
                            result['stop_cause'] = 'stream_drain_timeout'
                        break
        except OSError as error:
            failure = {'kind': type(error).__name__, 'errno': error.errno}
            if process is None:
                result['spawn_error'] = failure
            else:
                result['stream_error'] = failure
                result['stop_cause'] = 'stream_observation_failed'
                result['streams_complete'] = False
        except BaseException as error:
            interrupted = error
            result['stop_cause'] = 'interruption'
            result['streams_complete'] = False
        finally:
            if process is not None:
                if result['cleanup'] is None:
                    result['cleanup'] = owner.cleanup_process_group(process, cleanup_seconds / 2, cleanup_seconds / 2)
                for pipe in (process.stdout, process.stderr):
                    if pipe is not None:
                        pipe.close()
                result['returncode'] = process.returncode
                if process.returncode is not None:
                    if process.returncode >= 0:
                        result['exit_code'] = process.returncode
                    else:
                        result['signal_number'] = -process.returncode
    result.update(ended_at_unix_ns=time.time_ns(), duration_seconds=time.monotonic() - start,
                  observed_stream_bytes=observed, retained_stream_bytes=retained,
                  stdout=binding(streams['stdout']), stderr=binding(streams['stderr']))
    # A file written just before normal exit must obey the same limit. Preserve
    # its original bytes; exit zero cannot override an exhausted response budget.
    if response_file is not None and Path(response_file).exists():
        result['observed_response_bytes'] = Path(response_file).stat().st_size
        if result['observed_response_bytes'] > response_bytes and result['stop_cause'] is None:
            result['stop_cause'] = 'response_budget'
    result['outcome'] = ('spawn_failed' if result['spawn_error'] else
                         'stopped' if result['stop_cause'] else
                         'failed' if result['exit_code'] != 0 or not result['cleanup']['complete']
                         or not result['streams_complete'] else 'succeeded')
    (output / 'result.json').write_bytes(encoded(result))
    if interrupted is not None:
        raise interrupted
    return result


def runtime_snapshot(path):
    """Read non-secret explicit settings only; no auth or full environment capture."""
    path = Path(path)
    if not path.is_file():
        return {'state': 'unavailable', 'path': str(path)}
    config = tomllib.loads(path.read_text())
    names = ('model', 'model_provider', 'model_reasoning_effort', 'sandbox_mode', 'approval_policy')
    return {'state': 'observed_explicit_settings', 'path': str(path),
            'values': {name: config[name] for name in names if name in config},
            'hooks_configured': bool(config.get('hooks') or config.get('features', {}).get('hooks')),
            'mcp_server_count': len(config.get('mcp_servers', {})), 'effective_defaults': 'unobserved'}


def freeze_run(input_manifest, runtime, approach, lane, output):
    require(set(runtime) <= {'model', 'reasoning_effort', 'destination', 'language', 'authorization'},
            'only non-secret explicit runtime fields allowed')
    spec = verify(input_manifest)
    conditions = json.loads((HERE / 'conditions.json').read_bytes())
    require(approach in conditions['conditions'], 'unsupported approach')
    require(lane in {'product', 'archive_diagnostic'}, 'unsupported input lane')
    blockers = []
    for key in ('model', 'reasoning_effort', 'destination'):
        if not isinstance(runtime.get(key), str) or not runtime[key].strip():
            blockers.append('explicit_' + key + '_missing')
    if runtime.get('language') != conditions['language']:
        blockers.append('language_not_frozen')
    if approach == 'current' and (lane != 'product' or not any(
            e['role'] == 'current_preparation' and e['representation'] == 'full_file'
            for e in spec['entries'])):
        blockers.append('cutoff_bound_current_prepare_missing')
    authorization = runtime.get('authorization')
    expected = {'destination': runtime.get('destination'), 'purpose': 'explanation-generation-experiment',
                'input_sha256': binding(input_manifest)['sha256'],
                'instructions_sha256': binding(HERE / 'instructions.txt')['sha256'],
                'conditions_sha256': binding(HERE / 'conditions.json')['sha256'],
                'lane': lane, 'approach': approach}
    if not isinstance(authorization, dict) or authorization.get('scope') != expected or not authorization.get('current_request_locator'):
        blockers.append('current_destination_purpose_source_authorization_missing')
    # This adapter has not verified real host authentication, scoped tool invocation,
    # or model-side neutral controls. It deliberately offers no live dispatch path.
    blockers.append('live_host_retrieval_and_neutral_model_controls_unverified')
    result = {'format_version': 1, 'status': 'blocked', 'blockers': blockers,
              'input': binding(input_manifest), 'instructions': binding(HERE / 'instructions.txt'),
              'conditions': binding(HERE / 'conditions.json'), 'adapter': binding(Path(__file__)),
              'language': conditions['language'], 'approach': approach, 'lane': lane,
              'runtime': runtime, 'budgets': conditions['budgets'],
              'authentication': 'unobserved', 'model_identity': 'requested_not_attested',
              'authorization_status': 'missing_or_declared_not_verified',
              'settings': 'explicit_requested_only', 'tokens': None, 'price': None,
              'isolation': {'fresh_cwd': 'required_outside_source_and_archive',
                            'retrieval': 'opaque-ID enforced', 'filesystem_reads': 'cooperative_not_hermetic'},
              'dispatch': 'not_run', 'generation_output': None}
    output = Path(output)
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    path = output / 'run-manifest.json'; path.write_bytes(encoded(result))
    append_index(output.parent, {'input': 'frozen', 'approach': 'blocked', 'feedback': 'not_requested'}, [path])
    return result


def diagnose(output):
    """Installed local help/context probes. Never exec a model, resume or authenticate."""
    output = Path(output)
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    executable = shutil.which('codex')
    require(executable is not None, 'installed Codex unavailable')
    current_home = Path(os.environ.get('CODEX_HOME', Path.home() / '.codex'))
    snapshot = runtime_snapshot(current_home / 'config.toml')
    operations = []
    with tempfile.TemporaryDirectory(prefix='volicord-explanation-neutral-') as directory:
        workspace = Path(directory)
        require(not workspace.is_relative_to(REPOSITORY), 'fresh cwd outside source required')
        env = environment(workspace)
        (workspace / 'AGENTS.md').write_text('Neutral context control. Do not execute or read external files.\n')
        (output / 'isolated-config.toml').write_bytes((workspace / 'codex/config.toml').read_bytes())
        for label, args in (('version', ['--version']), ('exec-help', ['exec', '--help']),
                            ('prompt-help', ['debug', 'prompt-input', '--help']),
                            ('features', ['features', 'list']),
                            ('prompt-input', ['debug', 'prompt-input', 'Return the word NEUTRAL. Do not use tools.'])):
            operations.append(capture([executable, *args], cwd=workspace, env=env,
                                      output=output / label, timeout=15, stream_bytes=1 << 20))
        prompt = (output / 'prompt-input/stdout.bin').read_bytes()
        forbidden = [str(REPOSITORY).encode(), str(current_home).encode(), b'prototype-data',
                     b'agent-review-support', b'original-inventory', b'Volicord_']
        prompt_valid = operations[-1]['outcome'] == 'succeeded'
        try:
            json.loads(prompt)
        except (ValueError, UnicodeError):
            prompt_valid = False
        flags = {}
        for line in (output / 'features/stdout.bin').read_text().splitlines():
            fields = line.split()
            if len(fields) >= 3 and fields[-1] in {'true', 'false'}:
                flags[fields[0]] = fields[-1] == 'true'
        disabled_observed = all(flags.get(feature) is False for feature in FEATURES_OFF)
        result = {'kind': 'local_installed_host_diagnostic', 'operations': operations,
                  'source_configuration': snapshot, 'isolated_configuration': binding(output / 'isolated-config.toml'),
                  'prompt_json_observed': prompt_valid,
                  'disabled_features_observed': disabled_observed,
                  'forbidden_configured_context_observed': [needle.decode() for needle in forbidden if needle in prompt],
                  'authentication': 'not_probed', 'provider_dispatch': 'not_run', 'measured_prose': 'not_generated',
                  'filesystem_boundary': 'cooperative; same-user absolute filesystem reads remain possible',
                  'neutral_model_controls': 'not_run_authorization_missing',
                  'workspace_cleanup': 'TemporaryDirectory teardown pending'}
        result['status'] = ('passed_local_context_only' if prompt_valid and disabled_observed
                            and not result['forbidden_configured_context_observed']
                            and all(o['outcome'] == 'succeeded' for o in operations) else 'blocked')
    result['workspace_cleanup'] = 'complete'
    path = output / 'diagnostic.json'; path.write_bytes(encoded(result))
    append_index(output.parent, {'input': 'neutral', 'approach': 'local_diagnostic_only', 'feedback': 'not_requested'}, [path])
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--diagnose', action='store_true', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = diagnose(args.output)
    print(json.dumps({key: result[key] for key in ('prompt_json_observed',
                     'disabled_features_observed', 'forbidden_configured_context_observed',
                     'provider_dispatch', 'neutral_model_controls')}, indent=2))
    raise SystemExit(0 if result['status'] == 'passed_local_context_only' else 77)
