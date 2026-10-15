"""Seed explicitly authored review controls through the maintained MCP path.

Usage: seed_reader_document_review.py INPUT_JSON NEW_IGNORED_DIRECTORY BIN_DIR
The external input owns wording, evidence keys, original file hashes and ranges.
This is test-only orchestration, never generation, historical import or acceptance.
"""
import hashlib, json, os, re, subprocess, sys
from pathlib import Path

def digest(b):
    return 'sha256:' + hashlib.sha256(b).hexdigest()

def main():
    source = Path(sys.argv[1]).resolve()
    destination = Path(sys.argv[2]).resolve()
    binaries = Path(sys.argv[3]).resolve()
    data = json.loads(source.read_text())
    repository_root = Path(__file__).resolve().parents[4]
    destination.relative_to(repository_root / 'rebuild/.local')
    destination.mkdir(parents=True, exist_ok=False)
    cases = []
    for definition in data['works']:
        if not re.fullmatch('[a-z][a-z0-9_-]{0,48}', definition['key']):
            raise ValueError('invalid control key')
        root = destination / definition['key']
        repo = root / 'repository'
        repo.mkdir(parents=True)
        runtime = root / 'runtime'
        for file in definition['files']:
            locator = Path(file['path'])
            if locator.is_absolute() or '..' in locator.parts:
                raise ValueError('control paths must be repository relative')
            raw = Path(file['input']).read_bytes()
            assert digest(raw) == file['sha256']
            target = repo / file['path']
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
        env = dict(os.environ, VOLICORD_RUNTIME_DIR=str(runtime))
        log = (root / 'mcp.jsonl').open('w')
        errors = (root / 'mcp.stderr').open('w')
        process = subprocess.Popen([str(binaries / 'volicord-mcp')], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors, text=True, env=env)
        sequence = 0

        def call(tool, args):
            nonlocal sequence
            sequence += 1
            request = {'jsonrpc': '2.0', 'id': sequence, 'method': 'tools/call', 'params': {'name': tool, 'arguments': args}}
            log.write(json.dumps({'request': request}, ensure_ascii=False) + '\n')
            log.flush()
            process.stdin.write(json.dumps(request, ensure_ascii=False) + '\n')
            process.stdin.flush()
            line = process.stdout.readline()
            result = json.loads(line)
            log.write(json.dumps({'response': result}, ensure_ascii=False) + '\n')
            log.flush()
            if result.get('result', {}).get('isError') is not False:
                raise RuntimeError(str(result))
            return result['result']['structuredContent']
        try:
            project = call('project_initialize', {'repository': str(repo), 'display_name': 'Reader review · ' + definition['key'] + ' · historical authored control'})['project_id']
            work = call('context_record', {'project_id': project, 'role': 'goal', 'user_turn': definition['goal'], 'statement': definition['goal'], 'work_transition': 'start_new'})['context_item_id']
            purpose = data['project_purpose']
            call('context_record', {'project_id': project, 'role': 'project_purpose', 'user_turn': purpose, 'statement': purpose})
            analysis = call('repository_analyze', {'project_id': project})['analysis_snapshot_id']
            observed = {}
            for file in definition['files']:
                body = (repo / file['path']).read_bytes()
                lines = body.splitlines(keepends=True)
                for region in file['regions']:
                    if not 1 <= region['start_line'] <= region['end_line'] <= len(lines):
                        raise ValueError('invalid original line range')
                    start = sum(map(len, lines[:region['start_line'] - 1]))
                    end = sum(map(len, lines[:region['end_line']]))
                    result = call('work_source', {'action': 'observe_file', 'project_id': project, 'work_item_id': work, 'analysis_snapshot_id': analysis, 'path': file['path'], 'byte_range': {'start_byte': start, 'end_byte': end}, 'retain_seconds': 604800})
                    observed[region['key']] = {'source': result['source_id'], 'path': file['path'], 'start': start, 'end': end, 'start_line': region['start_line'], 'end_line': region['end_line'], 'body': body[start:end]}
            generation = call('work_explanation', {'action': 'generate', 'project_id': project, 'work_item_id': work, 'language': data['language']})
            plan = generation['plan']
            reads = {key: call('work_explanation', {'action': 'read_source', 'project_id': project, 'work_item_id': work, 'generation_id': generation['generation_id'], 'source_id': value['source']}) for key, value in observed.items()}
            units = []
            for unit in definition['units']:
                citations = []
                for ref in unit.get('refs', []):
                    o = observed[ref]
                    basis = reads[ref]['citation_basis']
                    c = {key: basis[key] for key in ['source_id', 'source_revision', 'observation_fingerprint', 'analysis_snapshot_id', 'repository_snapshot_id', 'repository_source_id']}
                    c.update(locator=o['path'], field='body', start_line=o['start_line'], end_line=o['end_line'], start_byte=o['start'], end_byte=o['end'], range_sha256=digest(o['body']), claim_start_byte=0, claim_end_byte=len(unit['text'].encode()), claim_sha256=digest(unit['text'].encode()), use_kind='current_code')
                    citations.append(c)
                units.append(dict(question=unit['role'], text=unit['text'], evidence_keys=unit['evidence_keys'], citations=citations))
            realization = {'format_kind': 'volicord_explanation', 'format_version': 3, 'plan_fingerprint': plan['fingerprint'], 'language': data['language'], 'generator': {'host': 'authored-reader-review-control', 'session': 'historical-input-not-new-execution', 'agent': None, 'model': None}, 'paragraphs': units}
            receipt = call('work_explanation', {'action': 'record', 'project_id': project, 'work_item_id': work, 'generation_id': generation['generation_id'], 'realization': realization})
            (root / 'realization.json').write_text(json.dumps(realization, ensure_ascii=False, indent=2))
            (root / 'receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2))
            case = {'key': definition['key'], 'runtime': str(runtime), 'repository': str(repo), 'project': project, 'work': work, 'input_classification': data['classification'], 'historical_work': definition['historical_work'], 'language': data['language'], 'expected_units': definition['units'], 'expected_sources': {k: {key: v.decode() if isinstance(v, bytes) else v for key, v in o.items()} for k, o in observed.items()}}
            cases.append(case)
        finally:
            process.stdin.close()
            code = process.wait(timeout=10)
            log.close()
            errors.close()
            (root / 'mcp.exit.json').write_text(json.dumps({'exit_code': code}))
            assert code == 0
    (destination / 'fixtures.json').write_text(json.dumps({'input': str(source), 'input_sha256': digest(source.read_bytes()), 'cases': cases, 'human_review': 'pending'}, ensure_ascii=False, indent=2))
    print(destination / 'fixtures.json')
if __name__ == '__main__':
    main()
