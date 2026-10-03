#!/usr/bin/env python3
"""Real sibling-process positive; simulated failures are negative support only."""
import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch
import resource_observer as observer


def require_rejected(fn):
    try: fn()
    except (ValueError, OSError): return
    raise AssertionError('bad process/telemetry accepted')


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--binary', type=Path, default=Path('rebuild/target/debug/volicord-mcp'))
    args = p.parse_args(); binary = args.binary.resolve()
    artifacts = {'volicord-mcp': {'path':str(binary), 'sha256':observer.digest(binary)}}
    processes = []
    with tempfile.TemporaryDirectory(prefix='mcp-observer-proof-') as temporary:
        root = Path(temporary); runtime = root/'runtime'; runtime.mkdir(); repository = root/'repository'; repository.mkdir()
        sentinel = 'private-conversation-provider-credential-SENTINEL-728193'
        env = os.environ | {'VOLICORD_RUNTIME_DIR':str(runtime), 'PRIVATE_TEST_SENTINEL':sentinel}
        def launch():
            began = time.monotonic_ns()
            child = subprocess.Popen([str(binary)], stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                cwd=repository, env=env, text=True)
            processes.append(child)
            child.stdin.write(json.dumps({'jsonrpc':'2.0','id':1,'method':'initialize','params':{
                'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':sentinel,'version':'test'}}})+'\n')
            child.stdin.flush()
            line = child.stdout.readline(); response = json.loads(line)
            assert response['id'] == 1 and 'result' in response, line
            return child, time.monotonic_ns()-began
        def finish(child):
            child.stdin.close(); assert child.wait(timeout=5) == 0
            assert child.stdout.read() == '', 'registration corrupted MCP stdout'
        first, startup = launch(); second, _ = launch()
        output = root/'observed'
        cmd = [sys.executable, str(Path(observer.__file__).resolve()), 'attach', '--binary', str(binary),
            '--runtime', str(runtime), '--output',str(output),'--duration-seconds','5','--interval-ms','250']
        watched = subprocess.Popen(cmd, stdout=subprocess.PIPE,stderr=subprocess.PIPE, text=True)
        try:
            # Real candidate is a sibling, not a descendant of the observer.
            assert int(Path(f'/proc/{first.pid}/stat').read_text().rsplit(') ',1)[1].split()[1]) == os.getpid()
            assert first.pid != watched.pid
            for _ in range(250):
                if (output/'control.json').exists(): break
                time.sleep(.02)
            time.sleep(.75); finish(first)
            restarted, _ = launch()
            time.sleep(.75); finish(second)
            stdout, stderr = watched.communicate(timeout=15)
            assert watched.returncode == 0, stderr
            result = json.loads((output/'resource.json').read_text()); observer.validate(result, artifacts['volicord-mcp']['sha256'])
            assert result['status'] in {'measured','partial'}, result['measurement']
            assert len(result['instances']) == 3 and all(i['samples'] for i in result['instances']), result
            assert len({(i['identity']['pid'],i['identity']['start_ticks'],i['identity']['instance_id']) for i in result['instances']}) == 3
            assert restarted.poll() is None, 'observer stop signaled external candidate'
            finish(restarted)
            assert not list(repository.iterdir()), 'observer mutated repository'
            registrations = [json.loads(p.read_text()) for p in (runtime/'observations/mcp').glob('*.json')]
            assert len(registrations) == 3 and all(r['state'] == 'stopped' for r in registrations)
            combined = json.dumps(result)+json.dumps(registrations)
            assert sentinel not in combined
            # Explicit stop file exits only the observer; real Product remains usable.
            active, _ = launch(); output2=root/'stop-proof'
            watch2 = subprocess.Popen(cmd[:cmd.index('--output')]+['--output',str(output2),'--duration-seconds','10','--interval-ms','50'],
                stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            for _ in range(100):
                if (output2/'control.json').exists(): break
                time.sleep(.02)
            stopped = subprocess.run([sys.executable,str(Path(observer.__file__).resolve()),'stop','--output',str(output2)],capture_output=True)
            assert stopped.returncode == 0, stopped.stderr
            watch2.communicate(timeout=5); assert watch2.returncode == 0
            assert json.loads((output2/'resource.json').read_text())['termination'] == 'stop_requested'
            assert active.poll() is None
            regpath = next(p for p in (runtime/'observations/mcp').glob('*.json') if json.loads(p.read_text())['pid'] == active.pid)
            reg = json.loads(regpath.read_text())
            assert observer.sample(reg, artifacts['volicord-mcp']['sha256'], {}) > 0
            reused = copy.deepcopy(reg); reused['start_ticks'] += 1
            require_rejected(lambda: observer.sample(reused, artifacts['volicord-mcp']['sha256'], {}))
            wrong = copy.deepcopy(reg); wrong['executable_sha256'] = '0'*64
            require_rejected(lambda: observer.sample(wrong, artifacts['volicord-mcp']['sha256'], {}))
            wrongpath = copy.deepcopy(reg); wrongpath['executable_path'] = '/bin/false'
            require_rejected(lambda: observer.sample(wrongpath, artifacts['volicord-mcp']['sha256'], {}))
            # Closed telemetry rejects deliberate content/environment/RPC fields.
            for field in ['rpc_arguments','conversation_body','source_body','provider_response','credentials','environment']:
                bad=copy.deepcopy(result);bad[field]=sentinel
                require_rejected(lambda: observer.validate(bad))
                bad=copy.deepcopy(result);bad['instances'][0]['identity'][field]=sentinel
                require_rejected(lambda: observer.validate(bad))
            bad=copy.deepcopy(result);bad['measurement']['peak_rss_bytes']=0
            require_rejected(lambda: observer.validate(bad))
            bad=copy.deepcopy(result);bad['instances'][0]['identity']['host_session']=sentinel
            require_rejected(lambda: observer.validate(bad))
            # Deliberate inaccessible/gone/PID-reuse fixtures are not positive measurement.
            for exception in [PermissionError(),FileNotFoundError(),observer.ObservationError('pid_reused')]:
                def failure(*args): raise exception
                simulated=observer.observe(artifacts,[runtime],duration_seconds=.4,interval_ms=50,proc_sample=failure)
                assert simulated['status'] in {'not_observed','environment_blocked'} and simulated['measurement']['peak_rss_bytes'] is None
                assert simulated['measurement']['sample_count']==0
            def failure(*args): raise RuntimeError(sentinel)
            failed=observer.observe(artifacts,[runtime],duration_seconds=.4,interval_ms=50,proc_sample=failure)
            assert failed['status']=='failed' and sentinel not in json.dumps(failed), failed
            assert active.poll() is None
            finish(active)
            short,_=launch(); finish(short)
            unobserved=observer.observe(artifacts,[runtime],duration_seconds=.4,interval_ms=50)
            assert unobserved['status']=='not_observed' and 'unsampled_instance' in unobserved['measurement']['measurement_errors']
            empty=root/'empty';empty.mkdir()
            missing=observer.observe(artifacts,[empty],duration_seconds=.4,interval_ms=50)
            assert missing['status']=='not_observed' and missing['measurement']['measurement_errors']==['missing_registration']
            def slow(*args): time.sleep(.15);return 123
            gaps=observer.observe(artifacts,[runtime],duration_seconds=.4,interval_ms=50,proc_sample=slow)
            # Stopped entries cannot become samples; use a running fixture solely for gap control.
            active,_=launch()
            gaps=observer.observe(artifacts,[runtime],duration_seconds=.4,interval_ms=50,proc_sample=slow)
            assert gaps['status']=='partial' and 'gap' in gaps['measurement']['measurement_errors']
            finish(active)
            # Broken registration publication does not disable canonical/MCP operations.
            blocked=root/'blocked';blocked.mkdir();(blocked/'observations').write_text('block registration')
            child=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                cwd=repository,env=env|{'VOLICORD_RUNTIME_DIR':str(blocked)},text=True);processes.append(child)
            child.stdin.write('{"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}\n');child.stdin.flush()
            assert json.loads(child.stdout.readline())['id']==2
            child.stdin.close();assert child.wait(timeout=5)==0
            assert 'lifecycle observation unavailable' in child.stderr.read()
            print(json.dumps({'status':'passed','positive':'three real sibling candidate processes',
                'measurement_status':result['status'],'measurement_errors':result['measurement']['measurement_errors'],
                'sample_count':result['measurement']['sample_count'],'observed_peak_rss_bytes':result['measurement']['peak_rss_bytes'],
                'candidate_startup_to_initialize_ns':startup,
                'registration_duration_ns':[r['registration_duration_ns'] for r in registrations],
                'observer_cpu_ns':result['observer_cpu_ns'],'observer_peak_rss_bytes':result['observer_peak_rss_bytes'],
                'observer_duration_ns':result['duration_ns'],'teardown':'EOF exits; observer stop leaves host-owned MCP alive',
                'simulations':'negative support only'}))
        finally:
            if watched.poll() is None: watched.kill(); watched.wait(timeout=5)
            for child in processes:
                if child.poll() is None:
                    child.kill();child.wait(timeout=5)
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
