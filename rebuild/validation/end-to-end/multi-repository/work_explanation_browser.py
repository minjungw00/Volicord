#!/usr/bin/env python3
"""Narrow actual-host Work reading proof. Requires prior public prepare/record.
Never generates text, installs tools, invokes a provider, or claims human review.
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


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--fixture', type=Path, required=True, help='Fresh seed_work_explanation_runtime manifest')
    p.add_argument('--chromium', type=Path, required=True)
    p.add_argument('--playwright-module', type=Path, required=True)
    p.add_argument('--library-path', type=Path)
    args = p.parse_args()
    root = harness.ROOT
    output = Path(tempfile.mkdtemp(prefix='work-explanation-browser-', dir=root / 'rebuild/.local/validation'))
    fixture = json.loads(args.fixture.read_text())
    recorder = harness.Recorder(output)
    env = os.environ.copy()
    if args.library_path:
        env['LD_LIBRARY_PATH'] = str(args.library_path.resolve()) + (':' + env['LD_LIBRARY_PATH'] if env.get('LD_LIBRARY_PATH') else '')
    result = {'kind': 'work_explanation_browser_support', 'status': 'not_run', 'output': str(output),
              'human_acceptance': 'not_established', 'external_provider': 'not_invoked', 'operations': []}
    process = None

    def run(label, argv):
        record = recorder.run(label, [str(a) for a in argv], env, timeout=120, cwd=root)
        result['operations'].append(record)
        if record['outcome'] != 'succeeded':
            raise RuntimeError(f'{label}: {record["outcome"]}; inspect full streams')
        return harness.decoded(record)

    cli = [root / 'rebuild/target/debug/volicord', '--runtime', fixture['runtime'], '--project', fixture['project'], '--json']
    try:
        fonts=run('korean-fonts', ['fc-list', '--format', '%{file}\n', ':lang=ko']).splitlines()
        if not fonts:
            raise RuntimeError('No Korean-capable font is configured; configure fontconfig before browser proof')
        result['korean_fonts']=[{'path':f,'sha256':harness.sha256(Path(f))} for f in sorted(set(fonts))]
        snapshots={}
        for language in ['en','ko']:
            target=output / f'answers-{language}.html'
            run(f'snapshot-{language}',cli+['--locale',language,'viewer','export','--output',target,'--language',language])
            snapshots[language]=str(target)
        before = json.loads(run('privacy-before', cli + ['privacy', 'status']))
        run('canonical-before', cli + ['context', 'export', '--output', output / 'before.json'])
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', 0))
            port = probe.getsockname()[1]
        argv = [str(root / 'rebuild/target/debug/volicord-viewer'), '--runtime', fixture['runtime'], '--project', fixture['project'], '--bind', f'127.0.0.1:{port}']
        with (output / 'viewer.stdout').open('wb') as stdout, (output / 'viewer.stderr').open('wb') as stderr:
            process = subprocess.Popen(argv, cwd=root, env=env, stdout=stdout, stderr=stderr)
        url = f'http://127.0.0.1:{port}/'
        for _ in range(100):
            if process.poll() is not None:
                raise RuntimeError('Viewer exited before serving; inspect viewer.stderr')
            try:
                urllib.request.urlopen(url, timeout=1).close()
                break
            except OSError:
                time.sleep(0.05)
        else:
            raise RuntimeError('Viewer startup timeout')
        cases = json.loads((Path(__file__).parent / 'fixtures/viewer-reading/answer-cases.json').read_text())
        config = {'url': url, 'fixture': fixture, 'output': str(output), 'chromium': str(args.chromium.resolve()),
                  'playwright': str(args.playwright_module.resolve()), 'claim_terms': cases['browser_claim_terms'], 'decision_terms': cases['decision_browser_claim_terms'], 'snapshots': snapshots}
        harness.write_json(output / 'config.json', config)
        run('browser', ['node', Path(__file__).parent / 'viewer_browser_driver.cjs', output / 'config.json', 'work-explanation'])
        after = json.loads(run('privacy-after', cli + ['privacy', 'status']))
        for field in ['request_count', 'managed_derived_count']:
            if before[field] != after[field]:
                raise RuntimeError(f'GET changed {field}')
        run('canonical-after', cli + ['context', 'export', '--output', output / 'after.json'])
        if (output / 'before.json').read_bytes() != (output / 'after.json').read_bytes():
            raise RuntimeError('GET changed canonical export')
        result['status'] = 'passed'
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
            result['viewer_termination'] = {'argv': argv, 'returncode': process.returncode, 'reason': 'explicit test cleanup'}
        harness.write_json(output / 'result.json', result)
        print(json.dumps(result))
    return 0 if result['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
