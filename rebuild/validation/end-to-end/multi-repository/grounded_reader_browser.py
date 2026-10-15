#!/usr/bin/env python3
"""Read actual host-recorded, Source-cited Work outputs; no generation or feedback."""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.request
import harness
import viewer_browser as supporting


def retained_state(runtime):
    """Read-only content identities, including retention; never emit body bytes."""
    runtime = Path(runtime)
    state = {}
    for database in sorted(runtime.glob('*.sqlite3')):
        with sqlite3.connect(database.resolve().as_uri()+'?mode=ro', uri=True) as connection:
            dump = '\n'.join(connection.iterdump()).encode()
        state[database.name] = hashlib.sha256(dump).hexdigest()
    for artifact in sorted((runtime/'derived').rglob('*')):
        if artifact.is_file():
            state[str(artifact.relative_to(runtime))] = harness.sha256(artifact)
    if 'canonical.sqlite3' not in state or 'privacy.sqlite3' not in state:
        raise RuntimeError('canonical and managed storage inspection required')
    return state


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--bin-dir', type=Path, required=True)
    parser.add_argument('--chromium', type=Path, required=True)
    parser.add_argument('--playwright-module', type=Path, required=True)
    parser.add_argument('--library-path', type=Path)
    parser.add_argument('--require-clean', action='store_true')
    args = parser.parse_args()
    output = Path(tempfile.mkdtemp(prefix='grounded-reader-browser-', dir=harness.ROOT/'rebuild/.local/validation'))
    recorder = harness.Recorder(output)
    environment = os.environ.copy()
    if args.library_path:
        environment['LD_LIBRARY_PATH'] = str(args.library_path.resolve()) + ':' + environment.get('LD_LIBRARY_PATH', '')
    fixture = json.loads(args.fixture.read_text())
    result = {'status': 'not_run', 'candidate_head': supporting.candidate(), 'human_comprehension': 'not_assessed',
              'semantic_assessment': 'not_supplied', 'fixture_sha256': harness.sha256(args.fixture),
              'executables': {n: harness.sha256(args.bin_dir/n) for n in ['volicord','volicord-viewer','volicord-mcp']},
              'browser_sha256': harness.sha256(args.chromium), 'commands': []}

    def run(label, argv):
        record = recorder.run(label, list(map(str, argv)), environment, timeout=120, cwd=harness.ROOT)
        result['commands'].append(record)
        if record['exit_code'] != 0:
            raise RuntimeError(f'{label} failed; inspect complete preserved streams')
        return harness.decoded(record)

    try:
        if args.require_clean and not supporting.clean():
            raise RuntimeError('clean committed candidate required')
        if not run('korean-fonts', ['fc-list', '--format', '%{file}\n', ':lang=ko']).strip():
            raise RuntimeError('Korean font prerequisite unavailable')
        basis, snapshots, before = {}, {}, {}
        for index, item in enumerate(fixture['cases']):
            stored_before = retained_state(item['runtime'])
            cli = [args.bin_dir/'volicord','--runtime',item['runtime'],'--project',item['project'],'--json']
            before[item['project']] = json.loads(run(f'privacy-before-{index}',cli+['privacy','status']))
            basis[item['project']], snapshots[item['project']] = {}, {}
            for language in ['en','ko']:
                status = json.loads(run(f'status-{index}-{language}',cli+['status','--language',language]))
                work = next(w for w in status['work_history'] if w['work_item_id']==item['work'])
                answers = work['answers']
                if answers['explanation_state']!='current' or not answers['source_excerpts']:
                    raise RuntimeError('requires a current recorded Source-cited host output')
                basis[item['project']][language] = answers
                snapshot=output/f'{index}-{language}.html'
                run(f'snapshot-{index}-{language}',cli+['--locale',language,'viewer','export','--output',snapshot,'--language',language])
                snapshots[item['project']][language]=str(snapshot)
            with socket.socket() as probe:
                probe.bind(('127.0.0.1',0)); port=probe.getsockname()[1]
            with (output/f'viewer-{index}.stdout').open('wb') as stdout, (output/f'viewer-{index}.stderr').open('wb') as stderr:
                process=subprocess.Popen([str(a) for a in cli[:-1]+['viewer','open','--bind',f'127.0.0.1:{port}']],
                                         env=environment,stdout=stdout,stderr=stderr,cwd=harness.ROOT)
            case_output=output/f'case-{index}'; case_output.mkdir()
            config={'cases':[item],'basis':basis,'snapshots':snapshots,'chromium':str(args.chromium.resolve()),
                    'playwright':str(args.playwright_module.resolve()),'output':str(case_output),
                    'url':f'http://127.0.0.1:{port}/','offline':False}
            try:
                for _ in range(100):
                    if process.poll() is not None: raise RuntimeError('Viewer exited before serving')
                    try:
                        urllib.request.urlopen(config['url'],timeout=1).close(); break
                    except OSError: time.sleep(.05)
                else: raise RuntimeError('Viewer startup timeout')
                config_path=output/f'config-{index}.json'; config_path.write_text(json.dumps(config))
                # Exercise changed read paths serially and concurrently through the
                # actual listener. A transport failure retains its exception; it is
                # not reclassified as a confirmed Product synchronization defect.
                routes = [f'?view={view}&locale={language}&language={language}'
                          + (f'&work={item["work"]}' if view != 'overview' else '')
                          + ('&scope=work' if view == 'code' else '')
                          for language in ['en', 'ko'] for view in ['overview', 'work', 'code']]
                def fetch(route):
                    try:
                        with urllib.request.urlopen(config['url']+route, timeout=30) as response:
                            body = response.read()
                            return {'route': route, 'http_status': response.status,
                                    'sha256': hashlib.sha256(body).hexdigest(), 'bytes': len(body)}
                    except Exception as error:
                        return {'route': route, 'error': repr(error)}
                serial = [fetch(route) for route in routes]
                with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
                    concurrent_reads = list(pool.map(fetch, routes * 3))
                reads = {'serial': serial, 'concurrent': concurrent_reads, 'process_exit_after_reads': process.poll()}
                (output/f'http-rendering-{index}.json').write_text(json.dumps(reads, indent=2)+'\n')
                result.setdefault('http_rendering', []).append(reads)
                if process.poll() is not None or any(r.get('http_status') != 200 for r in serial+concurrent_reads):
                    raise RuntimeError('serial/concurrent Viewer read failure; inspect preserved HTTP and process evidence')
                run(f'live-browser-{index}',['node',Path(__file__).with_suffix('.cjs'),config_path])
                source=Path(item['repository'])/item.get('path','main.py'); original=source.read_bytes()
                try:
                    source.write_bytes(original + b'\n// changed outside retained observations\n'
                                       if item.get('expected_sources') else b'def value():\n    return 6\n')
                    config.update(denied_state='changed_source',expected_diagnostic='Source bytes changed')
                    config_path.write_text(json.dumps(config))
                    run(f'changed-source-browser-{index}',['node',Path(__file__).with_suffix('.cjs'),config_path])
                    source.unlink()
                    config.update(denied_state='missing_source',expected_diagnostic=(
                        'current excerpt unavailable' if item.get('expected_sources')
                        else 'current Source unavailable'))
                    config_path.write_text(json.dumps(config))
                    run(f'missing-source-browser-{index}',['node',Path(__file__).with_suffix('.cjs'),config_path])
                finally:
                    source.write_bytes(original)
                    config.pop('denied_state',None); config.pop('expected_diagnostic',None)
            finally:
                process.terminate(); process.wait(timeout=10)
                result.setdefault('viewer_exits',[]).append(process.returncode)
            if json.loads(run(f'privacy-after-{index}',cli+['privacy','status']))!=before[item['project']]:
                raise RuntimeError('GET/export changed managed state or retention')
            if retained_state(item['runtime']) != stored_before:
                raise RuntimeError('read/render/export changed canonical, managed, provider or derived contents')
            result.setdefault('retained_state_checks', []).append({
                'project_id':item['project'],'unchanged':True,'before':stored_before,
                'after':retained_state(item['runtime'])})
            # The live server is gone and retained Runtime is physically unavailable.
            runtime=Path(item['runtime']); hidden=runtime.with_name(runtime.name+'-offline-unavailable')
            runtime.rename(hidden)
            try:
                config['offline']=True; config_path.write_text(json.dumps(config))
                run(f'offline-browser-{index}',['node',Path(__file__).with_suffix('.cjs'),config_path])
            finally: hidden.rename(runtime)
        if supporting.candidate()!=result['candidate_head'] or (args.require_clean and not supporting.clean()):
            raise RuntimeError('candidate changed during browser run')
        result['status']='passed'
    except Exception as error:
        result['status']='failed'; result['error']=str(error)
    (output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'status':result['status'],'artifacts':str(output),'candidate_head':result['candidate_head']}))
    return 0 if result['status']=='passed' else 1


if __name__=='__main__':
    raise SystemExit(main())
