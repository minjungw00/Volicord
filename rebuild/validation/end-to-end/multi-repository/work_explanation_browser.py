#!/usr/bin/env python3
"""Narrow actual-host Work reading proof. Requires prior public prepare/record.
Never generates text, installs tools, invokes a provider, or claims human review.
"""
import argparse
import json
import shutil
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
    p.add_argument('--fixture', type=Path, required=True, help='Fresh seed_work_explanation_runtime manifest')
    p.add_argument('--chromium', type=Path, required=True)
    p.add_argument('--playwright-module', type=Path, required=True)
    p.add_argument('--library-path', type=Path)
    p.add_argument('--bin-dir', type=Path, help='Installed sibling executables from this candidate')
    p.add_argument('--require-clean', action='store_true')
    p.add_argument('--lifecycle-response',type=Path,help='Active-host relay/en response prepared for a fresh import of this canonical export; used again only after punctuation correction')
    args = p.parse_args()
    root = harness.ROOT
    output = Path(tempfile.mkdtemp(prefix='work-explanation-browser-', dir=root / 'rebuild/.local/validation'))
    fixture = json.loads(args.fixture.read_text())
    recorder = harness.Recorder(output)
    env = os.environ.copy()
    if args.library_path:
        env['LD_LIBRARY_PATH'] = str(args.library_path.resolve()) + (':' + env['LD_LIBRARY_PATH'] if env.get('LD_LIBRARY_PATH') else '')
    result = {'kind': 'work_explanation_browser_support', 'status': 'not_run', 'output': str(output),
              'human_acceptance': 'not_established', 'external_provider': 'not_invoked', 'operations': [],
              'candidate_head': supporting.candidate(), 'initial_clean': supporting.clean(),
              'fixture_sha256': harness.sha256(args.fixture), 'fixture_repository_sha256': harness.tree_hash(Path(fixture['repository'])),
              'generator_identity': 'self_reported_not_independently_verified'}
    process = None

    def run(label, argv):
        record = recorder.run(label, [str(a) for a in argv], env, timeout=120, cwd=root)
        result['operations'].append(record)
        if record['outcome'] != 'succeeded':
            raise RuntimeError(f'{label}: {record["outcome"]}; inspect full streams')
        return harness.decoded(record)

    binaries = args.bin_dir.resolve() if args.bin_dir else root / 'rebuild/target/debug'
    cli = [binaries / 'volicord', '--runtime', fixture['runtime'], '--project', fixture['project'], '--json']
    try:
        if args.require_clean and not result['initial_clean']:
            raise RuntimeError('Final content/browser evidence requires a clean committed candidate')
        result['executables']={name:{'path':str(binaries/name),'sha256':harness.sha256(binaries/name)} for name in ['volicord','volicord-viewer']}
        result['inputs']={str(p.relative_to(root)):harness.sha256(p) for p in [Path(__file__).resolve(),Path(__file__).parent/'viewer_browser_driver.cjs',Path(__file__).parent/'fixtures/viewer-reading/answer-cases.json',root/'rebuild/Cargo.lock']}
        result['browser']={'sha256':harness.sha256(args.chromium),'version':run('browser-version',[args.chromium,'--version']).strip(), 'driver_version':json.loads((args.playwright_module/'package.json').read_text())['version']}
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
        argv = [str(binaries / 'volicord-viewer'), '--runtime', fixture['runtime'], '--project', fixture['project'], '--bind', f'127.0.0.1:{port}']
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
        basis={}
        for key in cases['browser_claim_terms']:
            basis[key]={}
            for language in ['en','ko']:
                plan=json.loads(run(f'basis-{key}-{language}',cli+['work','explain','prepare','--work',fixture['goals'][key],'--language',language]))['plan']
                basis[key][language]={'fingerprint':plan['fingerprint'],'evidence':[{k:e[k] for k in ['identity','revision','field']} for e in plan['evidence'] if e['key'] in ['goal','result','verification','next_step']]}
        lifecycle=[]
        if args.lifecycle_response:
            # Immutable original fixture; correction/forget operate on a disposable copy.
            copy=output/'lifecycle-runtime'
            # A raw Runtime copy retains managed bundle destinations; forgetting
            # would then scrub original evidence files. Import a disposable bundle
            # into a fresh Runtime so managed paths belong only to this copy.
            imported=output/'lifecycle-import.json'
            shutil.copy2(output/'before.json',imported)
            run('lifecycle-import',[binaries/'volicord','--runtime',copy,'--json','context','import','--input',imported])
            scoped=[binaries/'volicord','--runtime',copy,'--project',fixture['project'],'--json']
            run('lifecycle-bind',scoped+['--repository',fixture['repository'],'bind'])
            subject=fixture['goals']['relay']
            plan=json.loads(run('lifecycle-prepare',scoped+['work','explain','prepare','--work',subject,'--language','en']))['plan']
            goal=next(e for e in plan['evidence'] if e['key']=='goal')
            original_result=next(e for e in plan['evidence'] if e['key']=='result')
            expected_action=next(case['next_step'] for case in cases['cases'] if case['key']=='relay')
            response=json.loads(args.lifecycle_response.read_text())
            if response['plan_fingerprint']!=plan['fingerprint']:
                raise RuntimeError('Lifecycle response must belong to this fresh imported preparation; import can change repository Source availability, so an original-runtime response cannot be reused')
            result['lifecycle_basis']={'fingerprint':plan['fingerprint'],'source_status':plan['source_status'],'response_sha256':harness.sha256(args.lifecycle_response)}
            def export_phase(phase):
                target=output/f'lifecycle-{phase}.html'
                run('lifecycle-export-'+phase,scoped+['viewer','export','--output',target,'--language','en'])
                lifecycle.append({'phase':phase,'snapshot':str(target),'work':subject,'reported_change':response['paragraphs'][1]['text'],'recorded_action':expected_action if phase!='forgotten-result' else None})
            export_phase('absent-interpretation')
            run('lifecycle-record-imported',scoped+['work','explain','record','--work',subject,'--input',args.lifecycle_response.resolve()])
            export_phase('restart-current')
            partial={**response,'paragraphs':response['paragraphs'][:-1]}
            partial_file=output/'partial-response.json';harness.write_json(partial_file,partial)
            before_failed=json.loads(run('lifecycle-privacy-before',scoped+['privacy','status']))
            failed=recorder.run('lifecycle-partial-record',[str(a) for a in scoped+['work','explain','record','--work',subject,'--input',partial_file]],env,timeout=120,cwd=root)
            result['operations'].append(failed)
            if failed['outcome']!='failed' or failed.get('returncode',failed.get('exit_code'))==0:
                raise RuntimeError('Partial generation was not rejected with a failed process')
            after_failed=json.loads(run('lifecycle-privacy-after',scoped+['privacy','status']))
            if before_failed['managed_derived_count']!=after_failed['managed_derived_count']:
                raise RuntimeError('Partial generation changed retained answer count')
            run('lifecycle-correct',scoped+['advanced','records','correct-context',subject,'--revision',str(goal['revision']),'--source',goal['sources'][0],'--text',goal['content']+'.'])
            export_phase('stale-correction')
            stale=recorder.run('lifecycle-stale-record',[str(a) for a in scoped+['work','explain','record','--work',subject,'--input',args.lifecycle_response.resolve()]],env,timeout=120,cwd=root)
            result['operations'].append(stale)
            if stale['outcome']!='failed':raise RuntimeError('Revision-mismatched response was not rejected')
            fresh=json.loads(run('lifecycle-prepare-corrected',scoped+['work','explain','prepare','--work',subject,'--language','en']))['plan']
            if next(e for e in fresh['evidence'] if e['key']=='result')!=original_result:
                raise RuntimeError('Expression correction changed the reported result basis')
            # Current host explicitly supplied this interpretation for expression-only
            # replay; no feature text is generated by this validation runner.
            refreshed={**response,'plan_fingerprint':fresh['fingerprint']}
            refreshed_file=output/'expression-replay-response.json';harness.write_json(refreshed_file,refreshed)
            run('lifecycle-record-corrected',scoped+['work','explain','record','--work',subject,'--input',refreshed_file])
            export_phase('regenerated-current')
            run('lifecycle-forget-result',scoped+['advanced','records','forget','checkpoint',original_result['identity'],'--source',goal['sources'][0]])
            export_phase('forgotten-result')
            run('lifecycle-delete',scoped+['work','explain','delete','--work',subject])
            result['lifecycle']={'status':'passed','partial_generation':'rejected','revision_mismatch':'rejected','result_basis_preserved':True,'original_runtime':'unchanged','expression_replay':'active-host interpretation unchanged after punctuation-only correction'}
        else:
            result['lifecycle']={'status':'not_run','reason':'No active-host response supplied'}
        config = {'candidate_head':result['candidate_head'],'viewer_sha256':result['executables']['volicord-viewer']['sha256'],'url': url, 'fixture': fixture, 'output': str(output), 'chromium': str(args.chromium.resolve()),
                  'playwright': str(args.playwright_module.resolve()), 'claim_terms': cases['browser_claim_terms'], 'decision_terms': cases['decision_browser_claim_terms'], 'forbidden_patterns':cases['browser_forbidden_patterns'], 'basis':basis, 'snapshots': snapshots, 'lifecycle_snapshots':lifecycle}
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
        result['final_candidate_head']=supporting.candidate()
        result['final_clean']=supporting.clean()
        if result['candidate_head']!=result['final_candidate_head'] or (args.require_clean and not result['final_clean']):
            result.update(status='candidate_changed',detail='Candidate continuity failed')
        for identity in result.get('executables',{}).values():
            if harness.sha256(Path(identity['path']))!=identity['sha256']:
                result.update(status='executable_changed')
        observed=output/'work-explanation-result.json'
        if observed.is_file():
            browser=json.loads(observed.read_text())
            # Separate bounded review summary; never embed answer bodies/raw logs in the gate capsule.
            summary={'candidate_head':result['candidate_head'],'status':result['status'],'checks':[{k:c[k] for k in ['id','status','reason'] if k in c} for c in browser['checks']], 'executables':result.get('executables'),'human_acceptance':'not_established','generator_identity':result['generator_identity']}
            harness.write_json(output/'content-summary.json',summary)
        harness.write_json(output / 'result.json', result)
        print(json.dumps(result))
    return 0 if result['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
