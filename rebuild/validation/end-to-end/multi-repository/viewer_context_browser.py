#!/usr/bin/env python3
"""Focused real-browser context lifecycle support; labeled fixture prose only.

Uses the maintained explanation seed, public CLI and existing browser driver.
No provider, external host activation or human comprehension verdict is supplied.
"""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.request

import harness
import viewer_browser as supporting


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--chromium', type=Path, required=True)
    p.add_argument('--playwright-module', type=Path, required=True)
    p.add_argument('--library-path', type=Path)
    p.add_argument('--bin-dir', type=Path)
    p.add_argument('--require-clean', action='store_true')
    args = p.parse_args()
    root = harness.ROOT
    here = Path(__file__).resolve().parent
    output = Path(tempfile.mkdtemp(prefix='viewer-context-browser-', dir=root/'rebuild/.local/validation'))
    recorder = harness.Recorder(output)
    env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1', 'VOLICORD_EXPLANATION_FIXTURE_ROOT': str(output/'seed')}
    if args.library_path:
        env['LD_LIBRARY_PATH'] = str(args.library_path.resolve()) + ':' + env.get('LD_LIBRARY_PATH', '')
    bins = args.bin_dir.resolve() if args.bin_dir else root/'rebuild/target/debug'
    result = {'kind': 'viewer_display_context_browser_support', 'status': 'not_run',
        'candidate_head': supporting.candidate(), 'initial_clean': supporting.clean(), 'operations': [],
        'fixture_generation': 'labeled structural only', 'human_judgment': 'not_established',
        'output': str(output)}
    process = None

    def run(label, argv):
        record = recorder.run(label, [str(a) for a in argv], env, timeout=180, cwd=root)
        result['operations'].append(record)
        if record['outcome'] != 'succeeded':
            raise RuntimeError(f'{label}: {record["outcome"]}; inspect preserved streams')
        return harness.decoded(record)

    def port():
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', 0))
            return probe.getsockname()[1]

    try:
        if args.require_clean and not result['initial_clean']:
            raise RuntimeError('committed candidate required')
        result['executables'] = {name: {'path': str(bins/name), 'sha256': harness.sha256(bins/name)}
            for name in ['volicord', 'volicord-viewer']}
        result['browser_version'] = run('browser-version', [args.chromium, '--version']).strip()
        fonts = run('korean-fonts', ['fc-list', '--format', '%{file}\n', ':lang=ko']).splitlines()
        if not fonts:
            raise RuntimeError('Korean-capable font required')
        result['korean_fonts'] = [{'path': f, 'sha256': harness.sha256(Path(f))} for f in sorted(set(fonts))]
        run('seed', ['cargo', 'test', '--manifest-path', root/'rebuild/Cargo.toml', '-p', 'volicord-viewer',
            '--test', 'work_explanation', 'seed_work_explanation_runtime', '--', '--exact'])
        fixture = json.loads((output/'seed/fixture.json').read_bytes())
        viewer_port, debug_port = port(), port()
        argv = [str(bins/'volicord-viewer'), '--runtime', fixture['runtime'], '--project', fixture['project'],
            '--bind', f'127.0.0.1:{viewer_port}']
        url = f'http://127.0.0.1:{viewer_port}/'
        with (output/'viewer.stdout').open('wb') as stdout, (output/'viewer.stderr').open('wb') as stderr:
            process = subprocess.Popen(argv, env=env, cwd=root, stdout=stdout, stderr=stderr)
        for _ in range(200):
            if process.poll() is not None:
                raise RuntimeError('Viewer exited before serving')
            try:
                urllib.request.urlopen(url, timeout=1).close()
                break
            except OSError:
                time.sleep(0.05)
        else:
            raise RuntimeError('Viewer startup timeout')
        extension = output/'zoom-extension'
        extension.mkdir()
        harness.write_json(extension/'manifest.json', {'manifest_version': 3, 'name': 'Viewer supporting tab zoom',
            'version': '1.0', 'permissions': ['tabs'], 'background': {'service_worker': 'worker.js'}})
        (extension/'worker.js').write_text('chrome.runtime.onInstalled.addListener(() => {});\n')
        config = {'candidate_head': result['candidate_head'], 'viewer_sha256': result['executables']['volicord-viewer']['sha256'],
            'viewer': str(bins/'volicord-viewer'), 'cli': str(bins/'volicord'), 'fixture': fixture, 'url': url,
            'debug_port': debug_port, 'capture_tool': str(here/'capture_viewer_context.py'),
            'chromium': str(args.chromium.resolve()), 'playwright': str(args.playwright_module.resolve()),
            'extension': str(extension), 'output': str(output)}
        harness.write_json(output/'config.json', config)
        run('browser-context-lifecycle', ['node', here/'viewer_browser_driver.cjs', output/'config.json', 'context-lifecycle'])
        # Validate every actual receipt with the same closed consumer contract.
        import sys
        sys.path.insert(0, str(root/'rebuild/validation/dogfood'))
        import resource_observer
        import viewer_observation
        observed = json.loads((output/'context-lifecycle-result.json').read_bytes())
        for value in observed['display_captures']:
            viewer_observation.validate_capture(value, candidate_head=result['candidate_head'],
                viewer_sha256=config['viewer_sha256'], runtime_binding=resource_observer.path_binding(Path(fixture['runtime'])),
                project=fixture['project'])
            data = (output/value['screenshot']['path']).read_bytes()
            if len(data) != value['screenshot']['bytes'] or harness.sha256(output/value['screenshot']['path']) != value['screenshot']['sha256']:
                raise RuntimeError('captured screenshot integrity changed')
        result.update(status='passed', display_captures=len(observed['display_captures']),
            browser_support=observed['checks'], native_zoom=observed['zoom'])
    except Exception as error:
        result.update(status='failed', detail=str(error))
    finally:
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
            result['viewer_termination'] = {'argv': argv, 'exit_code': process.returncode, 'termination': 'explicit cleanup'}
        result['final_candidate_head'] = supporting.candidate()
        result['final_clean'] = supporting.clean()
        if result['candidate_head'] != result['final_candidate_head'] or args.require_clean and not result['final_clean']:
            result.update(status='candidate_changed')
        for value in result.get('executables', {}).values():
            if harness.sha256(Path(value['path'])) != value['sha256']:
                result.update(status='executable_changed')
        harness.write_json(output/'result.json', result)
        print(json.dumps(result))
    return 0 if result['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
