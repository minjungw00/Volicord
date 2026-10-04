"""Local shell support, distinct from actual-host tool-shell and measured use."""
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import sys


def run(root, binary, logs, product):
    import campaign as c
    path=c.ROOT/'rebuild/validation/linux-codex-integration/launch_readiness.py'
    spec=importlib.util.spec_from_file_location('local_readiness_support',path)
    owner=importlib.util.module_from_spec(spec);spec.loader.exec_module(owner)
    root.mkdir()
    prefix=root/'installed path with spaces'/'bin';prefix.mkdir(parents=True)
    for name in c.CANDIDATE_ARTIFACTS: shutil.copy2(binary.with_name(name),prefix/name)
    cli=prefix/'volicord';runtime=root/'runtime with spaces';repository=root/'repository with spaces';repository.mkdir()
    logs.run(['git','-C',repository,'init','--quiet'])
    common=[cli,'--runtime',runtime,'--repository',repository,'--json']
    project=json.loads(logs.run([*common,'init','Local launch boundary support']))['project_id']
    logs.run([*common,'codex','enable'])
    client=product(prefix/'volicord-mcp',runtime,repository,logs)
    try:
        resolved=client.tool('project_resolve',{'repository':str(repository)})
        assert resolved['project_id']==project
    finally: client.close()
    mcp_process=logs.records[-1]['identity']
    shadow=root/'conflicting bin';shadow.mkdir()
    (shadow/'volicord').write_text('#!/bin/sh\nexit 64\n');(shadow/'volicord').chmod(0o755)
    startup=root/'isolated startup';startup.mkdir()
    fixture_path=str(prefix)+':'+os.environ.get('PATH','')
    (startup/'.zprofile').write_text('export PATH='+shlex.quote(str(shadow)+':'+fixture_path)+'\n')
    args=[sys.executable,'-B',path,'--binary',cli,'--runtime',runtime,'--repository',repository,
        '--cli-sha256',owner.digest(cli),'--mcp-sha256',owner.digest(prefix/'volicord-mcp')]
    routes=[]
    zsh=shutil.which('zsh'); assert zsh is not None
    for flags,match in (('-c',True),('-lc',False)):
        # Only fixture PATH/ZDOTDIR are passed explicitly; full environment stays private.
        output=logs.run(['env','PATH='+fixture_path,'ZDOTDIR='+str(startup),zsh,flags,shlex.join(map(str,args))])
        proof=json.loads(output);assert proof['status']=='ready' and proof['bare_matches_candidate']==match
        assert proof['project_id']==project
        routes.append({'shell':'non_login' if match else 'login','bare_matches_candidate':match,
            'status':proof['status'],'cli_sha256':proof['cli_sha256'],'mcp_sha256':proof['mcp_sha256'],
            'runtime_binding':proof['runtime_binding'],'repository_binding':proof['repository_binding'],
            'process':logs.records[-1]['identity'],'output':c.explanation_evidence.binding(output)})
    negatives={}
    for name,wrong_cli,wrong_runtime in [('wrong_executable',shadow/'volicord',runtime),('wrong_runtime',cli,root/'foreign runtime')]:
        try: owner.probe(wrong_cli,wrong_runtime,repository,owner.digest(cli),owner.digest(prefix/'volicord-mcp'))
        except (ValueError,OSError): negatives[name]='rejected'
        else: raise AssertionError('foreign route became ready')
    return {'kind':'launch_boundary_support','execution_channel':'local_subprocess_support',
        'actual_host_proof':'not_supplied_by_local_rehearsal','project_id':project,'mcp_process':mcp_process,
        'routes':routes,'negative_controls':negatives}
