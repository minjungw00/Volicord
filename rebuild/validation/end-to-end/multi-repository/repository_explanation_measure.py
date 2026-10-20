#!/usr/bin/env python3
"""Measure an explicitly authored Repository draft through current Product APIs.

Focused support, not a generator or semantic evaluator. Freeze source-first
expectations separately before authoring. Each attempt requires a new directory;
failed compose/record bytes and full process streams remain inspectable.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()


def measure(args):
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    draft_bytes = args.input.read_bytes()
    # No generated input is rewritten, rebound, shortened or repaired here.
    (output / 'original-draft.json').write_bytes(draft_bytes)
    metadata = {
        'kind': 'explicit_repository_product_measurement',
        'candidate': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
        'worktree_status': subprocess.check_output(['git', 'status', '--porcelain'], text=True),
        'tracked_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'project': args.project, 'analysis': args.analysis,
        'runtime': str(args.runtime.resolve()),
        'input_sha256': hashlib.sha256(draft_bytes).hexdigest(),
        'input_bytes': len(draft_bytes), 'operations': [],
        'external_model_calls': 0, 'semantic_assessment': 'separate_source_first_review_required',
        'human_acceptance': 'not_observed', 'status': 'incomplete',
    }
    prefix = [str(binary), '--runtime', str(args.runtime.resolve()),
              '--project', args.project, '--json']

    def run(label, command):
        resources = output / (label + '.resources')
        argv = ['/usr/bin/time', '-v', '-o', str(resources)] + prefix + command
        start = time.monotonic()
        try:
            process = subprocess.run(argv, capture_output=True, timeout=180)
        except subprocess.TimeoutExpired as error:
            (output / (label + '.stdout')).write_bytes(error.stdout or b'')
            (output / (label + '.stderr')).write_bytes(error.stderr or b'')
            metadata['operations'].append({
                'label': label, 'argv': argv, 'exit_code': None, 'signal': None,
                'termination': 'timeout_without_child_completion_receipt',
                'duration_ms': (time.monotonic() - start) * 1000,
            })
            raise
        (output / (label + '.stdout')).write_bytes(process.stdout)
        (output / (label + '.stderr')).write_bytes(process.stderr)
        metadata['operations'].append({
            'label': label, 'argv': argv, 'exit_code': process.returncode,
            'signal': -process.returncode if process.returncode < 0 else None,
            'duration_ms': (time.monotonic() - start) * 1000,
            'stdout_bytes': len(process.stdout), 'stderr_bytes': len(process.stderr),
            'stdout_sha256': hashlib.sha256(process.stdout).hexdigest(),
            'stderr_sha256': hashlib.sha256(process.stderr).hexdigest(),
        })
        if process.returncode:
            raise RuntimeError(f'{label} failed with exit {process.returncode}; see preserved streams')
        return json.loads(process.stdout)

    try:
        plan = run('prepare', ['repository', 'explain', 'prepare', '--analysis', args.analysis])['plan']
        metadata['preparation_bytes'] = len(compact(plan))
        metadata['retention_budget'] = plan['retention_budget']
        realization = run('compose', ['repository', 'explain', 'compose', '--analysis', args.analysis,
                                      '--input', str(output / 'original-draft.json')])['realization']
        path = output / 'realization.json'
        path.write_bytes(compact(realization))
        units = realization['paragraphs']
        citations = [c for unit in units for c in unit['citations']]
        metadata.update({
            'realization_bytes': len(compact(realization)), 'unit_count': len(units),
            'authored_text_bytes': sum(len(unit['text'].encode()) for unit in units),
            'citation_count': len(citations),
            'source_count': len({c['source_id'] for c in citations}),
            # Bindings, distinct ranges and authored claim spans are different counts.
            'distinct_source_ranges': len({(c['source_id'], c['field'], c['start_byte'], c['end_byte'])
                                           for c in citations}),
            'distinct_cited_claim_spans': sum(len({(c['claim_start_byte'], c['claim_end_byte'])
                                                   for c in unit['citations']}) for unit in units),
            'editorial_citation_count': sum(len(unit['citations']) for unit in units
                                           if unit['question'] in ('heading', 'section_heading')),
        })
        if args.record:
            record = run('record', ['repository', 'explain', 'record', '--analysis', args.analysis,
                                    '--input', str(path)])
            metadata['record_response_bytes'] = len(compact(record))
            metadata['retained_bytes'] = len(compact(record['explanation']))
            answers = run('read', ['repository', 'explain', 'read'])['answers']
            metadata['reading_state'] = answers['explanation_state']
            metadata['provenance'] = answers['provenance']
            # Production readback must preserve exact original units, not just counts.
            assert [(p['text'], p['citations']) for p in answers['prose']] == [
                (p['text'], p['citations']) for p in units]
        metadata['status'] = 'passed'
    except Exception as error:
        metadata['status'] = 'failed'
        metadata['failure'] = str(error)
        raise
    finally:
        (output / 'result.json').write_text(json.dumps(metadata, ensure_ascii=False, indent=2))
    print(json.dumps(metadata, ensure_ascii=False))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--project', required=True)
    parser.add_argument('--analysis', required=True)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--record', action='store_true', help='Explicitly retain this admitted draft')
    measure(parser.parse_args())


if __name__ == '__main__':
    main()
