#!/usr/bin/env python3
"""Capture the currently displayed local Viewer tab using the existing browser driver.
No navigation, generation, mutation, browser replacement or human verdict occurs.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
from urllib.parse import urlparse

HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]/'dogfood'))
import viewer_observation as observation
import resource_observer


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--campaign-root',type=Path)
    p.add_argument('--binary',type=Path)
    p.add_argument('--runtime',type=Path)
    p.add_argument('--project')
    p.add_argument('--cdp-url',required=True)
    p.add_argument('--page-url',required=True)
    p.add_argument('--playwright-module',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    observation.local_url(args.page_url)
    endpoint=urlparse(args.cdp_url)
    observation.require(endpoint.scheme in {'http','ws'} and endpoint.hostname in {'127.0.0.1','localhost','::1'}
        and endpoint.port and endpoint.username is None and endpoint.password is None,'local browser debugger required')
    if args.campaign_root:
        import campaign
        manifest=campaign.load_evidence_set(args.campaign_root.resolve())
        campaign.require_current_candidate(manifest['candidate_head'])
        campaign.verify_candidate_artifacts(campaign.load_campaign(args.campaign_root.resolve()))
        candidate=manifest['candidate_head']; binary=Path(manifest['candidate_artifacts']['volicord-viewer']['path'])
    else:
        observation.require(args.binary and args.runtime and args.project,'explicit candidate binary/Runtime/Project required')
        binary=args.binary.resolve()
        candidate=subprocess.run(['git','rev-parse','HEAD'],cwd=HERE.parents[3],capture_output=True,text=True,check=True).stdout.strip()
        manifest=None
    expected_hash=resource_observer.digest(binary)
    output=args.output.resolve();output.mkdir(parents=True,mode=0o700,exist_ok=False)
    config={'output':str(output),'playwright':str(args.playwright_module.resolve()),'url':args.page_url,
        'cdp_url':args.cdp_url,'candidate_head':candidate,'viewer_sha256':expected_hash}
    (output/'config.json').write_text(json.dumps(config))
    with (output/'stdout.log').open('wb') as stdout,(output/'stderr.log').open('wb') as stderr:
        try:
            completed=subprocess.run(['node',str(HERE/'viewer_browser_driver.cjs'),str(output/'config.json'),'observe'],
                stdout=stdout,stderr=stderr,timeout=30,check=False)
        except subprocess.TimeoutExpired:
            (output/'process.json').write_text(json.dumps({'exit_code':None,'termination':'timeout'}))
            raise ValueError('browser capture timed out; complete streams preserved')
    process={'exit_code':completed.returncode,'termination':'exited' if completed.returncode>=0 else 'signal'}
    (output/'process.json').write_text(json.dumps(process))
    observation.require(completed.returncode==0,'actual browser capture failed; inspect preserved streams')
    result=json.loads((output/'observe-result.json').read_bytes())
    observation.require(result['status']=='passed' and len(result['display_captures'])==1,'missing actual displayed capture')
    value=result['display_captures'][0]
    if manifest is not None:
        subjects = observation.load_subjects(args.campaign_root.resolve(), manifest)
        observation.for_manifest(manifest,value,value['context']['locale'],subjects)
        campaign.require_current_candidate(candidate)
        campaign.verify_candidate_artifacts(campaign.load_campaign(args.campaign_root.resolve()))
    else:
        observation.validate_capture(value,candidate_head=candidate,viewer_sha256=expected_hash,
            runtime_binding=resource_observer.path_binding(args.runtime),project=args.project)
    observation.require(resource_observer.digest(binary)==expected_hash,'candidate executable changed during browser capture')
    (output/'display-context.json').write_text(json.dumps(value,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'status':'captured','evidence_class':'browser_display_capture','human_judgment':'not_established',
        'locale':value['context']['locale'],'view':value['context']['view'],'render_id':value['context']['render_id']}))
    return 0

if __name__=='__main__':
    raise SystemExit(main())
