#!/usr/bin/env python3
"""Bounded Linux process/transfer observations; no gate or provider invocation."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import select
import shutil
import subprocess
import tarfile
import time
import urllib.request


def fingerprint(raw):
    return dict(bytes=len(raw), gzip_bytes=len(gzip.compress(raw, mtime=0)), sha256=hashlib.sha256(raw).hexdigest())


def main(args):
    out=args.output.resolve()
    if 'rebuild/.local/' not in str(out):
        raise ValueError('ignored output required')
    out.mkdir(parents=True,exist_ok=False)
    build=args.build.resolve()
    args.runtime=args.runtime.resolve()
    args.product_bin=args.product_bin.resolve()
    result=dict(clock_ticks_per_second=os.sysconf('SC_CLK_TCK'),samples=[],assets={},packages={},product={})
    env={**os.environ,'PATH':'/usr/bin:/bin'}
    assert shutil.which('node',path=env['PATH']) is None
    assert shutil.which('npm',path=env['PATH']) is None
    for candidate in ['rust','vue']:
        binary=build/(candidate+'-reader')
        result['assets'][binary.name]=fingerprint(binary.read_bytes())
        for repeat in range(7):
            before=resource.getrusage(resource.RUSAGE_CHILDREN)
            started=time.perf_counter()
            proc=subprocess.Popen([str(binary)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            try:
                if not select.select([proc.stdout],[],[],10)[0]:
                    raise TimeoutError('listener startup')
                url=proc.stdout.readline().strip()
                startup=(time.perf_counter()-started)*1000
                for key in ['b','click']:
                    target=url+'?work='+key
                    with urllib.request.urlopen(target,timeout=10) as response:
                        raw=response.read()
                    result['assets'][candidate+'-'+key+'-html']=fingerprint(raw)
                    (out/(candidate+'-'+key+'.html')).write_bytes(raw)
                for endpoint in ['app.js']+(['data.json'] if candidate=='vue' else []):
                    with urllib.request.urlopen(url+endpoint,timeout=10) as response:
                        raw=response.read()
                    result['assets'][candidate+'-'+endpoint]=fingerprint(raw)
                rss=int(next(l for l in Path(f'/proc/{proc.pid}/status').read_text().splitlines() if l.startswith('VmRSS:')).split()[1])
                with urllib.request.urlopen(urllib.request.Request(url,headers={'Host':'foreign.invalid'}),timeout=10):
                    raise AssertionError('wrong Host accepted')
            except urllib.error.HTTPError as error:
                if error.code != 403:
                    raise
            finally:
                proc.terminate();stdout,stderr=proc.communicate(timeout=10)
                after=resource.getrusage(resource.RUSAGE_CHILDREN)
            result['samples'].append(dict(candidate=candidate,repeat=repeat,startup_ms=startup,rss_kib=rss,
                lifecycle_user_ms=(after.ru_utime-before.ru_utime)*1000,lifecycle_system_ms=(after.ru_stime-before.ru_stime)*1000,
                returncode=proc.returncode,termination='explicit SIGTERM cleanup',stdout=stdout,stderr=stderr))
        # Exact executable-only local package; this contains private evaluation material.
        buffer=io.BytesIO()
        with tarfile.open(fileobj=buffer,mode='w') as archive:
            files=[(binary.name,binary.read_bytes(),0o755),('LICENSE.click',args.click_license.read_bytes(),0o644)]
            if candidate=='vue':
                files.append(('LICENSE.vue',(build/'node_modules/vue/LICENSE').read_bytes(),0o644))
            for name,raw,mode in files:
                info=tarfile.TarInfo(name);info.size=len(raw);info.mode=mode;info.mtime=0;archive.addfile(info,io.BytesIO(raw))
        raw=buffer.getvalue();result['packages'][candidate]=fingerprint(raw)
        (out/(candidate+'-linux-x86_64.tar.gz')).write_bytes(gzip.compress(raw,mtime=0))
    for name in ['volicord','volicord-viewer','volicord-mcp']:
        result['product'][name]=fingerprint((args.product_bin/name).read_bytes())
    if args.retained:
        fixtures=json.loads((args.retained/'maintained-review/fixtures.json').read_text())['cases']
        for fixture in fixtures:
            proc=subprocess.Popen([str(args.product_bin/'volicord-viewer'),'--runtime',fixture['runtime'],'--project',fixture['project'],'--bind','127.0.0.1:0','--locale','ko','--language','ko'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=env)
            try:
                import re
                url=None
                for _ in range(20):
                    if not select.select([proc.stderr],[],[],2)[0]:
                        raise TimeoutError('production listener')
                    match=re.search(r'http://127\.0\.0\.1:\d+/',proc.stderr.readline())
                    if match:
                        url=match[0]
                        break
                if url is None:
                    raise RuntimeError('production URL unavailable')
                with urllib.request.urlopen(url+'?view=work&work='+fixture['work'],timeout=10) as response:
                    raw=response.read()
                result['assets']['baseline-'+fixture['key']+'-html']=fingerprint(raw)
                (out/('baseline-'+fixture['key']+'.html')).write_bytes(raw)
            finally:
                proc.terminate();proc.communicate(timeout=10)
    strace=str(args.strace.resolve()) if args.strace else shutil.which('strace')
    (out/'result.json').write_text(json.dumps(result,indent=2))
    if strace:
        for name in ['volicord','volicord-mcp']:
            command=[str(args.product_bin/name)]
            if name=='volicord':
                command+=['--runtime',str(args.runtime),'--json','doctor','check']
            raw='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"local-reader-measurement","version":"1"}}}\n' if name.endswith('mcp') else ''
            trace=out/(name+'.strace')
            run=subprocess.run([strace,'-f','-e','trace=process,network','-o',str(trace)]+command,input=raw,text=True,capture_output=True,env={**env,'VOLICORD_RUNTIME_DIR':str(args.runtime)},timeout=20)
            (out/(name+'.stdout')).write_text(run.stdout);(out/(name+'.stderr')).write_text(run.stderr)
            if run.returncode:
                raise RuntimeError(run.stderr)
            text=trace.read_text()
            result['product'][name]['trace']=dict(exit_code=run.returncode,execve_calls=text.count('execve('),socket_calls=text.count('socket('),connect_calls=text.count('connect('),listen_calls=text.count('listen('),trace=fingerprint(trace.read_bytes()))
            assert text.count('execve(')==1 and text.count('socket(')==0
    else:
        result['trace_unavailable']='strace not installed; no process/network assertion supplied'
    (out/'result.json').write_text(json.dumps(result,indent=2))
    print(out/'result.json')


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    for name in ['build','product-bin','runtime','output','click-license']:
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--strace',type=Path)
    parser.add_argument('--retained',type=Path)
    main(parser.parse_args())
