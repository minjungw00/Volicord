#!/usr/bin/env python3
"""Actual MCP, loopback HTTP and offline consumption of an excerpt competition fixture."""
import argparse
import hashlib
from html.parser import HTMLParser
import importlib.util
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import time
import urllib.parse
import urllib.request

class Code(HTMLParser):
    def __init__(self):
        super().__init__();self.depth=0;self.current=[];self.windows=[]
    def handle_starttag(self,tag,attrs):
        if tag=='code':self.depth+=1
    def handle_endtag(self,tag):
        if tag=='code':
            self.depth-=1
            if not self.depth:self.windows.append(''.join(self.current));self.current=[]
    def handle_data(self,data):
        if self.depth:self.current.append(data)

def write(path,value):
    with path.open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)

def state(runtime):
    result={}
    for name in ['canonical.sqlite3','privacy.sqlite3']:
        path=runtime/name
        if not path.exists():continue
        with sqlite3.connect(path.resolve().as_uri()+'?mode=ro',uri=True) as db:
            result[name]=hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    assert len(result)==2,result
    return result

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--fixture',type=Path,required=True)
    p.add_argument('--bin-dir',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--expected',choices=['included','omitted'],default='included')
    args=p.parse_args();args.output.mkdir()
    f=json.loads(args.fixture.read_bytes());out=args.output.resolve();binary=args.bin_dir.resolve()
    write(out/'invocation.json',dict(fixture_sha256=hashlib.sha256(args.fixture.read_bytes()).hexdigest(),
        bin_sha256={name:hashlib.sha256((binary/name).read_bytes()).hexdigest() for name in ['volicord','volicord-mcp','volicord-viewer']},
        expected=args.expected,reader_question=f['reader_question']))
    before=state(Path(f['runtime']))
    # Import only the maintained original-record comparator; never a compact-projection oracle.
    spec=importlib.util.spec_from_file_location('measure',Path('rebuild/validation/end-to-end/multi-repository/complete_read_measure.py'))
    measure=importlib.util.module_from_spec(spec);spec.loader.exec_module(measure)
    case=dict(label='competition',runtime=f['runtime'],project=f['project'],subject_kind='repository',
        subject_id=f['analysis'],language='en',repository=f['repository'],realization=f['original_realization'],
        recording_receipt=f['recording_receipt'],public=f['public'])
    mcp=out/'mcp';mcp.mkdir();server=measure.Server(binary/'volicord-mcp',case,mcp)
    try:measure.journey(server,case,measure.original(case),mcp/'journey')
    finally:server.close()
    stdout=(out/'viewer.stdout').open('xb');stderr=(out/'viewer.stderr').open('xb')
    child=subprocess.Popen([str(binary/'volicord-viewer'),'--runtime',f['runtime'],'--project',f['project'],'--language','en','--bind','127.0.0.1:0'],stdout=stdout,stderr=stderr)
    try:
        deadline=time.monotonic()+20;address=None
        while time.monotonic()<deadline:
            raw=(out/'viewer.stderr').read_text()
            match=re.search(r'http://127\.0\.0\.1:\d+/',raw)
            if match:address=match[0];break
            assert child.poll() is None,raw
            time.sleep(.05)
        assert address,'Viewer startup timeout'
        with urllib.request.urlopen(address+'?view=repository',timeout=60) as response:
            assert response.status==200;live=response.read()
        (out/'live.html').write_bytes(live)
        public=json.loads(Path(f['public']).read_bytes())
        offset=0;parts=[];pages=0
        while True:
            query=urllib.parse.urlencode(dict(subject='repository:'+f['analysis'],language='en',
                realization=public['provenance']['realization_fingerprint'],unit=0,citation=0,offset=offset))
            with urllib.request.urlopen(address+'citation?'+query,timeout=60) as response:
                raw=response.read();assert len(raw)<=32768
            write(out/f'http-citation-{pages}.json',json.loads(raw));selected=json.loads(raw)['chunk'];pages+=1
            assert selected['citation']==f['critical_citation']
            parts.append(selected['text'])
            if selected['next_offset'] is None:break
            offset=selected['next_offset']
        assert ''.join(parts)==f['critical_source']
    finally:
        child.terminate();code=child.wait(timeout=20);stdout.close();stderr.close()
        write(out/'viewer-exit.json',dict(exit_code=code,termination='explicit SIGTERM cleanup'))
    argv=[str(binary/'volicord'),'--runtime',f['runtime'],'--project',f['project'],'--json',
          'viewer','export','--language','en','--output',str(out/'offline.html')]
    exported=subprocess.run(argv,capture_output=True,timeout=120)
    (out/'export.stdout').write_bytes(exported.stdout);(out/'export.stderr').write_bytes(exported.stderr)
    write(out/'export-process.json',dict(argv=argv,exit_code=exported.returncode));assert exported.returncode==0,exported.stderr
    findings={}
    for mode,data in [('live',live),('offline',(out/'offline.html').read_bytes())]:
        parsed=Code();parsed.feed(data.decode());included=f['critical_source'] in parsed.windows
        findings[mode]=dict(critical_complete=included,html_bytes=len(data))
        assert included==(args.expected=='included'),(mode,included,args.expected)
    after=state(Path(f['runtime']));assert before==after,(before,after)
    write(out/'result.json',dict(exit_code=0,expected=args.expected,findings=findings,critical_bytes=len(f['critical_source'].encode()),
        citation_pages=pages,canonical_privacy_unchanged=True,critical_evidence_available_offline=args.expected=='included',reader_question_answered='not_observed',
        semantic_or_human_acceptance='not_established'))
    print(json.dumps(findings),flush=True)

if __name__=='__main__':main()
