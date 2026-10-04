"""Real sibling candidate lifecycle controls; raw artifacts remain local support."""
import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import time
import resource_observer as o


def coverage_primitives(value):
    """Bounded rehearsal receipt of current fields; private identities stay local."""
    return {'ticks': copy.deepcopy(value['ticks']), 'duration_ns': value['duration_ns'],
        'interval_ns': value['interval_ns'], 'status': value['status'],
        'measurement': {k: value['measurement'][k] for k in ('sample_count', 'peak_rss_bytes')},
        'instances': [{'identity': {k: i['identity'][k] for k in ('instance_id', 'runtime_binding')},
            'samples': copy.deepcopy(i['samples'])} for i in value['instances']]}


def run(binary, root):
    root.mkdir(parents=True, exist_ok=False)
    repository = root / 'repository'; repository.mkdir()
    runtimes = [root / 'runtime-a', root / 'runtime-b']
    for r in runtimes: r.mkdir()
    artifacts = {'volicord-mcp': {'path': str(binary), 'sha256': o.digest(binary)}}
    children, process_records, results = [], [], {}

    def launch(runtime):
        child = subprocess.Popen([str(binary)], cwd=repository,
            env=os.environ | {'VOLICORD_RUNTIME_DIR': str(runtime)},
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        children.append(child)
        child.started = time.monotonic_ns()
        child.stdin.write(b'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n'); child.stdin.flush()
        child.first = child.stdout.readline()
        assert json.loads(child.first)['id'] == 1
        return child

    def finish(child, abrupt=False):
        if abrupt: child.kill()
        else: child.stdin.close()
        child.wait(timeout=5)
        stdout, stderr = child.first + child.stdout.read(), child.stderr.read()
        name = 'mcp-' + str(children.index(child))
        (root/(name+'.stdout')).write_bytes(stdout); (root/(name+'.stderr')).write_bytes(stderr)
        record = {'label':name,'exit_code':child.returncode,'termination':'sigkill' if abrupt else 'stdin_eof',
            'duration_ns':time.monotonic_ns()-child.started, 'stdout_sha256':o.digest(root/(name+'.stdout')),
            'stderr_sha256':o.digest(root/(name+'.stderr'))}
        process_records.append(record)
        assert child.returncode == (-9 if abrupt else 0)

    def observe(name, selected, **kwargs):
        result = o.observe(artifacts, selected, duration_seconds=kwargs.pop('duration_seconds', .35),
            interval_ms=kwargs.pop('interval_ms', 50), purpose='dogfood_rehearsal', **kwargs)
        o.validate(result, artifacts['volicord-mcp']['sha256'])
        path = root/(name+'.json'); path.write_text(json.dumps(result, sort_keys=True)+'\n')
        results[name] = {'status':result['status'], 'sample_count':result['measurement']['sample_count'],
            'measurement_errors':result['measurement']['measurement_errors'], 'termination':result['termination'],
            'lifecycles':[i['lifecycle'] for i in result['instances']], 'artifact_sha256':o.digest(path)}
        return result

    try:
        active = launch(runtimes[0])
        one = observe('active_runtime', [runtimes[0]])
        assert one['status'] == 'measured'
        concurrent = launch(runtimes[0])
        both = observe('concurrent_runtime', [runtimes[0]])
        assert both['status'] == 'measured' and len(both['instances']) == 2
        bad = copy.deepcopy(both)
        tick_index = 1
        omitted = bad['instances'][0]['identity']['instance_id']
        bad['instances'][0]['samples'].pop(tick_index)
        bad['ticks'][tick_index]['runtimes'][0]['sampled'].remove(omitted)
        numbers = [s['rss_bytes'] for i in bad['instances'] for s in i['samples']]
        bad['measurement'].update(sample_count=len(numbers), peak_rss_bytes=max(numbers))
        mutation_path = root / 'partial_process_sampling.json'
        mutation_path.write_text(json.dumps(bad, sort_keys=True) + '\n')
        try: o.validate(json.loads(mutation_path.read_bytes()))
        except o.ObservationError as error:
            assert str(error) == 'running instance lacks tick sample', 'wrong integrity rejection'
        else: raise AssertionError('partial per-instance sampling accepted')
        # Restoration is the untouched retained observation, not regenerated data.
        restored = json.loads((root / 'concurrent_runtime.json').read_bytes())
        assert o.validate(restored)['status'] == 'measured' and restored == both
        results['partial_process_sampling'] = {'status': 'rejected', 'tick_index': tick_index,
            'omitted_instance_id': omitted, 'basis_sha256': o.digest(root / 'concurrent_runtime.json'),
            'mutated_sha256': o.digest(mutation_path), 'restored_sha256': o.digest(root / 'concurrent_runtime.json'),
            'original': coverage_primitives(both), 'mutated': coverage_primitives(bad),
            'rejection': 'running instance lacks tick sample'}
        finish(concurrent)
        # Use fresh waiting Homes; a stopped-only, never-sampled attachment is a
        # separate unsampled limitation rather than part of this active proof.
        runtimes[0] = root / 'single-active'; runtimes[0].mkdir()
        finish(active); active = launch(runtimes[0])
        waiting = {o.path_binding(runtimes[1]):'waiting'}
        plus = observe('active_plus_waiting', runtimes, expectations=waiting)
        assert plus['status'] == 'measured'
        unknown = observe('active_plus_unknown', runtimes)
        assert unknown['status'] == 'partial' and 'unknown_coverage' in unknown['measurement']['measurement_errors']
        missing = observe('missing_expected', runtimes, expectations={o.path_binding(runtimes[1]):'active'})
        assert missing['status'] == 'partial' and 'missing_registration' in missing['measurement']['measurement_errors']
        detached = observe('stop_during_active', [runtimes[0]], expectations={o.path_binding(runtimes[0]):'active'}, stop=lambda: True)
        assert detached['status'] == 'partial' and 'active_at_detach' in detached['measurement']['measurement_errors']
        assert detached['termination'] == 'stop_requested' and active.poll() is None
        interrupt_tick = 0
        def interrupt_event():
            nonlocal interrupt_tick
            interrupt_tick += 1
            if interrupt_tick == 2: raise KeyboardInterrupt()
            return {}
        interrupted = observe('observer_interruption', [runtimes[0]], expectation_reader=interrupt_event)
        assert interrupted['status']=='partial' and interrupted['termination']=='interrupted'
        assert interrupted['measurement']['sample_count'] > 0 and 'observer_interrupted' in interrupted['measurement']['measurement_errors']
        finish(active)
        # Future Homes stay waiting across three sequential real activations.
        sequential = [root/'sequential-a', root/'sequential-b', root/'sequential-c']
        for r in sequential: r.mkdir()
        phase, current = 0, None
        def events():
            nonlocal phase, current
            phase += 1
            if phase == 2: current = launch(sequential[0])
            if phase == 5: finish(current); current = launch(sequential[1])
            if phase == 8: finish(current); current = launch(sequential[2])
            if phase == 11: finish(current)
            return {}
        seq = observe('sequential_eof', sequential, duration_seconds=3, interval_ms=250,
            expectations={o.path_binding(r):'waiting' for r in sequential}, expectation_reader=events)
        assert phase >= 11 and seq['status'] == 'measured' and len(seq['instances']) == 3
        assert all(i['samples'] and i['identity']['state']=='stopped' and i['lifecycle']=='stopped' for i in seq['instances'])
        # Real sampled instance killed after one tick; useful samples cannot erase disappearance.
        abrupt_runtime = root/'abrupt'; abrupt_runtime.mkdir(); killed = launch(abrupt_runtime)
        tick = 0
        def kill_event():
            nonlocal tick
            tick += 1
            if tick == 2: finish(killed, abrupt=True)
            return {}
        gone = observe('abrupt_exit', [abrupt_runtime], expectation_reader=kill_event)
        assert gone['status']=='partial' and 'process_gone' in gone['measurement']['measurement_errors']
        assert gone['instances'][0]['identity']['state']=='running' and gone['instances'][0]['lifecycle']=='gone'
        pre = root/'pre-attachment'; pre.mkdir(); short=launch(pre); finish(short)
        unsampled=observe('pre_attachment_exit', [pre])
        assert unsampled['status']=='not_observed' and 'unsampled_instance' in unsampled['measurement']['measurement_errors']
        empty = root/'empty'; empty.mkdir()
        zero=observe('zero_expected_samples', [empty], expectations={o.path_binding(empty):'active'})
        assert zero['status']=='not_observed' and zero['measurement']['sample_count']==0
        assert 'missing_registration' in zero['measurement']['measurement_errors']
        # Independent raw verifier rejects completion forgery, timing removal and foreign identity.
        for mutation in ('verdict','expectation','ticks','identity','sample'):
            bad=copy.deepcopy(missing)
            if mutation=='verdict': bad['status']='measured'; bad['measurement']['measurement_errors']=[]
            elif mutation=='expectation': bad['ticks'][0]['runtimes'].pop()
            elif mutation=='ticks': bad['ticks'].pop(0)
            elif mutation=='identity': bad['instances'][0]['identity']['runtime_binding']='0'*64
            elif mutation=='sample': bad['instances'][0]['samples'][0]['elapsed_ns']=bad['duration_ns']
            try: o.validate(bad)
            except (ValueError, TypeError, KeyError): pass
            else: raise AssertionError('forged coverage accepted: '+mutation)
        results['forged_completion']={'status':'rejected','mutations':5,'basis_sha256':o.digest(root/'missing_expected.json')}
        return {'kind':'resource_boundary_support','candidate_mcp_sha256':artifacts['volicord-mcp']['sha256'],
            'results':results,'processes':process_records,'evidence_role':'real_sibling_support_not_host_or_naturalistic'}
    finally:
        for child in children:
            if child.poll() is None: child.kill(); child.wait(timeout=5)
        (root/'processes.json').write_text(json.dumps(process_records,sort_keys=True)+'\n')


def main():
    p=argparse.ArgumentParser(); p.add_argument('--binary',required=True,type=Path); p.add_argument('--output',required=True,type=Path)
    a=p.parse_args(); result=run(a.binary.resolve(),a.output.resolve())
    (a.output/'result.json').write_text(json.dumps(result,sort_keys=True)+'\n')
    print(json.dumps(result,sort_keys=True))


if __name__=='__main__': main()
