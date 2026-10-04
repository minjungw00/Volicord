"""Actual candidate explanation retention through the campaign lifecycle collector.

Only campaign admission/capture mapping are labeled support seams. Raw Product
prepare/record/readback processes and copied projections remain in ignored output.
"""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import time
from types import SimpleNamespace
from unittest.mock import patch
import campaign as c
import explanation_evidence as e
import rehearsal_support
import review_explanations as review


def run(binary, fixture, root):
    root.mkdir(parents=True, exist_ok=False)
    runtime, repository = Path(fixture['runtime']), Path(fixture['repository'])
    project, work, decision = fixture['project'], fixture['goals']['relay'], fixture['decisions']['project']
    raw=root/'support-capture.jsonl'; raw.write_text('Explicit authored capture seam, no measured interaction.\n')
    capture=SimpleNamespace(source_sha256=c.harness.sha256(raw),session_id='retention-support',
        successful_calls=lambda operation:[SimpleNamespace(result={'project_id':project})] if operation=='project_resolve' else [], commands=[])
    mapped={('volicord','A','start'):SimpleNamespace(source=raw,capture=capture)}
    artifacts={name:{'path':str(binary.with_name(name)),'sha256':c.harness.sha256(binary.with_name(name))} for name in c.CANDIDATE_ARTIFACTS}
    campaign={'evidence_purpose':'dogfood_rehearsal','candidate_head':c.harness.git_head(c.ROOT),
        'candidate_binary':str(binary),'candidate_artifacts':artifacts,'document_language':'en',
        'journeys':{'journey-volicord':{'repository_class':'volicord','runtime_home':str(runtime),'repository_path':str(repository)}}}
    c.write_json(c.inventory_path(root),c.load_inventory(root))
    lives=[]
    with patch.object(c,'load_campaign_for_mutation',return_value=campaign), patch.object(c,'verify_frozen_campaign'), patch.object(c,'map_batch_rollouts',return_value=mapped):
        prepared=e.prepare(root,[raw],languages=['en','ko'],work_ids=[work],decision_ids=[decision])
        for item in prepared['explanations']:
            directory=e.entry_path(root,item['identity'])
            preparation=json.loads(e.bound(root,directory/'preparation.json')); plan=preparation['plan']
            response=rehearsal_support.realization(plan)
            keys={'purpose':'goal','reported_change':'result','expected_effect':'result','verification':'verification','next_step':'next_step',
                'user_rationale':'user_rationale','recommendation':'recommendation','consequences':'consequences','applicability':'applicability'}
            for paragraph in response['paragraphs']: paragraph['evidence_keys']=[keys[paragraph['question']]]
            # Compact response matches the independent Session-1 byte reproducer.
            base=len(e.compact_bytes(response)); assert base<=3181
            response['paragraphs'][0]['text']+='x'*(3181-base)
            response_path=root/(item['identity']+'-response.json');response_path.write_bytes(e.compact_bytes(response))
            e.record(root,item['identity'],response_path)
            receipt=json.loads(e.bound(root,directory/'record.json'))
            retained=receipt['explanation']; size=len(e.compact_bytes(retained))
            assert 16384 < size <= e.RETAINED_BYTE_LIMIT
            assert retained['realization']==response and len(retained['source_status'])>=32
            lives.append({'subject_kind':plan['subject']['kind'],'language':plan['requested_language'],
                'source_count':len(retained['source_status']),'response_bytes':3181,'retained_bytes':size,
                'after_state':'current','identity':item['identity'],
                'stages':{name:e.binding(e.bound(root,directory/(name+'.json'))) for name in ('attempt','preparation','response','record','after','receipt')},
                'copied_review':None})
        for life in lives:
            directory=e.entry_path(root,life['identity'])
            projected,_=review.project(root,directory/'preparation.json',evidence_set_sha256='0'*64)
            review.validate(projected)
            (directory/'copied-review.json').write_bytes(projected)
            life['copied_review']=e.binding(projected)
        e.require_ready(root,campaign,mapped)
        index=e.collection_index(root,mapped)
        review.validate_publication_index(index)
        c.write_json(root/'collection-index.json',index)
        # Deliberate failed Product call is isolated from the completed campaign obligations.
        selected=next(item for item in prepared['explanations'] if item['subject']['kind']=='work' and item['language']=='en')
        directory=e.entry_path(root,selected['identity']); preparation=json.loads(e.bound(root,directory/'preparation.json'))
        accepted=json.loads(e.bound(root,directory/'response.json')); recorded=json.loads(e.bound(root,directory/'record.json'))
        oversized=copy.deepcopy(accepted); oversized['paragraphs'][0]['text']='x'*16385
        path=root/'disposable-oversize.json';path.write_bytes(e.compact_bytes(oversized))
        began=time.monotonic_ns()
        result=subprocess.run([str(binary),'--runtime',str(runtime),'--project',project,'--json','work','explain','record',
            '--work',work,'--language','en','--input',str(path)],capture_output=True,timeout=30)
        (root/'oversize.stdout').write_bytes(result.stdout); (root/'oversize.stderr').write_bytes(result.stderr)
        assert result.returncode>0 and b'16384' in result.stderr and (b'bytes' in result.stderr or b'byte' in result.stderr)
        after,process=e.read(binary,runtime,project,preparation['subject'],'en')
        e.validate_lifecycle(preparation,accepted,recorded,after)
        for name,data in process.items(): (root/('oversize-readback-'+name)).write_bytes(data)
        e.require_ready(root,campaign,mapped)
        negative={'exit_code':result.returncode,'termination':'exited','duration_ns':time.monotonic_ns()-began,
            'prior_record_preserved':True,'actionable_byte_error':True,'stdout':e.binding(result.stdout),'stderr':e.binding(result.stderr),
            'readback':e.binding(c.json_bytes(after))}
        c.write_json(root/'oversize-result.json',negative)
    return {'kind':'retention_boundary_support','candidate_cli_sha256':artifacts['volicord']['sha256'],
        'lifecycles':lives,'collection_index':e.binding((root/'collection-index.json').read_bytes()),
        'oversize':negative,'evidence_role':'authored_real_product_not_host_semantic_or_naturalistic'}


def main():
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();result=run(a.binary.resolve(),json.loads(a.fixture.read_bytes()),a.output.resolve())
    c.write_json(a.output/'result.json',result);print(json.dumps(result,sort_keys=True))


if __name__=='__main__':main()
