#!/usr/bin/env python3
"""Bounded two-candidate builder; accepts retained authored controls, never generates prose."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    return json.loads(path.read_text())


def literal(value):
    # JSON string escapes are Rust-compatible for these UTF-8 inputs except controls.
    if any(ord(c) < 32 and c not in '\n\r\t' for c in value):
        raise ValueError('unsupported Rust literal control')
    return json.dumps(value, ensure_ascii=False)


def prepare(args):
    root = args.retained.resolve()
    output = args.output.resolve()
    if 'rebuild/.local/' not in str(output):
        raise ValueError('output must be ignored rebuild/.local state')
    output.mkdir(parents=True, exist_ok=False)
    evidence = {}

    def verified(path, expected=None):
        raw = path.read_bytes()
        actual = sha(raw)
        if expected and actual != expected.removeprefix('sha256:'):
            raise ValueError(f'input changed: {path}')
        evidence[str(path)] = {'sha256': actual, 'bytes': len(raw)}
        return raw

    identities = load(root / 'review/identities.json')
    for item in identities['original_assets']:
        verified(Path(item['path']), item['sha256'])
    authored = json.loads(verified(root / 'review-input-final.json', identities['input_sha256']))
    fixtures = load(root / 'maintained-review/fixtures.json')
    data = []
    for work, fixture in zip(authored['works'], fixtures['cases'], strict=True):
        if work['key'] != fixture['key'] or work['units'] != fixture['expected_units']:
            raise ValueError('authored fixture identity mismatch')
        record = json.loads(verified(root / 'browser-final' / (work['key'] + '-status.stdout'), identities['files']['browser-final/'+work['key']+'-status.stdout']))
        selected = next(w for w in record['work_history'] if w['work_item_id'] == fixture['work'])
        answers = selected['answers']
        if answers['provenance']['realization_fingerprint'] != next(c for c in identities['cases'] if c['key']==work['key'])['realization_fingerprint']:
            raise ValueError('output identity mismatch')
        sources = []
        refs = {}
        for file in work['files']:
            raw = verified(Path(file['input']), file['sha256'])
            for region in file['regions']:
                start, end = region['start_line'], region['end_line']
                exact = b''.join(raw.splitlines(keepends=True)[start-1:end]).decode()
                matches = [s for s in answers['source_excerpts'] if s['citation']['locator']==file['path'] and s['citation']['start_line']==start and s['citation']['end_line']==end]
                if len(matches)!=1 or matches[0]['text']!=exact or matches[0]['omitted_utf8_bytes']!=0 or matches[0]['citation']['range_sha256']!='sha256:'+sha(exact.encode()):
                    raise ValueError('exact admitted Source range mismatch')
                refs[region['key']] = len(sources)
                sources.append(dict(label=f"{file['path']}:{start}–{end}", text=exact, start=start, comparison=matches[0]['comparison']))
        paragraphs = answers['prose']
        if [u['text'] for u in work['units']] != [p['text'] for p in paragraphs]:
            raise ValueError('retained substantive content mismatch')
        for p in paragraphs:
            for citation in p['citations']:
                claim=p['text'].encode()[citation['claim_start_byte']:citation['claim_end_byte']]
                if citation['claim_sha256']!='sha256:'+sha(claim):
                    raise ValueError('claim identity mismatch')
        data.append(dict(key=work['key'], id=fixture['work'], goal=work['goal'],
            status=f"control {selected['state']} · captured {answers['explanation_state']} · historical Work {work['historical_work']} · acceptance pending",
            units=[dict(role=u['role'],text=u['text'],refs=[refs[r] for r in u['refs']]) for u in work['units']],
            sources=sources,audit=json.dumps(selected,ensure_ascii=False,indent=2)))
    (output/'data.json').write_text(json.dumps(data,ensure_ascii=False,separators=(',',':')))
    (output/'lineage.json').write_text(json.dumps(dict(reference_head=identities['candidate'], classification=authored['classification'], inputs=evidence, data_sha256=sha((output/'data.json').read_bytes()), human_acceptance='pending'),indent=2))
    rust = ['const WORKS: &[Work] = &[']
    for w in data:
        fields=','.join(f'{k}:{literal(w[k])}' for k in ['key','goal','status','audit'])
        units=','.join('Unit{role:'+literal(u['role'])+',text:'+literal(u['text'])+',refs:&'+str(u['refs'])+'}' for u in w['units'])
        sources=','.join('Source{label:'+literal(s['label'])+',text:'+literal(s['text'])+',start:'+str(s['start'])+',comparison:'+literal(s['comparison'])+'}' for s in w['sources'])
        rust.append('Work{'+fields+',units:&['+units+'],sources:&['+sources+']},')
    rust.append('];')
    (output/'data.rs').write_text('\n'.join(rust))
    for f in ['reader.rs','reader.css','enhance.js','vue.js']:
        shutil.copyfile(HERE/f,output/f)
    shutil.copyfile(HERE/'enhance.js',output/'app.js')
    subprocess.run(['rustc','--edition=2021','-O',str(output/'reader.rs'),'-o',str(output/'rust-reader')],check=True)
    for key in ['b','click']:
        subprocess.run([str(output/'rust-reader'),str(output/(key+'-offline.html')),key],check=True)
    print(output)


if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--retained',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    prepare(parser.parse_args())
