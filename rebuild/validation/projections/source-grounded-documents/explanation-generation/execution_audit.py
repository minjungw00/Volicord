"""Read retained public process/tool observations; never inspect private reasoning."""
from __future__ import annotations

import argparse
from datetime import datetime
import json
from pathlib import Path
import subprocess

from approaches import events, retrieval_audit
from inputs import binding, check_binding, digest, encoded, require, verify


def historical_binding(value, revision, repository):
    """A changed maintained producer is verified from its original Git object.

    Never rewrite the receipt or make its absolute path point at replacement code.
    Runtime artifacts still require their original bytes at their original path.
    """
    try:
        check_binding(value)
        return 'original_path'
    except ValueError:
        relative = Path(value['path']).relative_to(repository)
        require(str(relative).startswith('rebuild/validation/'), 'runtime artifact drift')
        result = subprocess.run(['git', '-C', str(repository), 'show', revision + ':' + str(relative)],
                                capture_output=True, check=True)
        require(len(result.stdout) == value['bytes'] and digest(result.stdout) == value['sha256'],
                'original producer object mismatch')
        return 'original_git_object'


def seconds(timestamp):
    return datetime.fromisoformat(timestamp.replace('Z', '+00:00')).timestamp()


def sequences(value):
    """Extract ledger sequence identities from public tool results, not prose."""
    if isinstance(value, str):
        try:
            return sequences(json.loads(value))
        except ValueError:
            return []
    if isinstance(value, list):
        return [sequence for item in value for sequence in sequences(item)]
    if isinstance(value, dict):
        if set(('sequence', 'name', 'arguments', 'status')) <= value.keys():
            return [value['sequence']]
        return [sequence for item in value.values() for sequence in sequences(item)]
    return []


def stage_timeline(process, context_path, ledger):
    context = json.loads(Path(context_path).read_bytes())
    pending, batches = {}, []
    # Whitelist public custom tool envelopes. Do not select, interpret or retain
    # reasoning entries even if the input file also contains them.
    for row in context:
        if row.get('type') != 'response_item':
            continue
        payload = row.get('payload', {})
        if payload.get('type') == 'custom_tool_call':
            pending[payload['call_id']] = row
        elif payload.get('type') == 'custom_tool_call_output':
            begin = pending.pop(payload['call_id'], None)
            if begin:
                joined = sequences(payload.get('output'))
                batches.append({'call_id': payload['call_id'], 'tool': begin['payload'].get('name'),
                                'started_at': begin['timestamp'], 'ended_at': row['timestamp'],
                                'wall_seconds': seconds(row['timestamp']) - seconds(begin['timestamp']),
                                'ledger_sequences': joined})
    evidence = [b for b in batches if b['ledger_sequences']]
    reads = [b for b in evidence if any(ledger[s]['name'] == 'read' and ledger[s]['status'] == 'returned'
                                      and ledger[s]['charged_bytes'] for s in b['ledger_sequences'])]
    wall = (process['ended_at_unix_ns'] - process['started_at_unix_ns']) / 1e9
    return {'process': process, 'context': binding(context_path), 'public_tool_batches': batches,
            'unpaired_public_tool_calls': sorted(pending),
            'first_nonempty_read_return_at': reads[0]['ended_at'] if reads else None,
            'last_nonempty_read_return_at': reads[-1]['ended_at'] if reads else None,
            'wall_seconds': wall, 'monotonic_seconds': process['duration_seconds'],
            'clock_difference_seconds': wall - process['duration_seconds'],
            'evidence_batch_wall_seconds': sum(b['wall_seconds'] for b in evidence),
            'timing_limits': 'batch timing includes host orchestration; no per-read/provider timing; '
                             'wall timestamps must not be subtracted from monotonic durations'}


def audit_cohort(root, repository):
    root, repository = Path(root).resolve(), Path(repository).resolve()
    plan = json.loads((root / 'plan.json').read_bytes())
    reports = []
    for name in ('work-b-direct', 'work-b-note_then_prose', 'click-direct', 'click-note_then_prose',
                 'work-b-independent-review', 'click-independent-review'):
        directory = root / name
        review = 'independent-review' in name
        receipt = directory / ('review-receipt.json' if review else 'attempt.json')
        record = json.loads(receipt.read_bytes())
        identities = []
        for key in ('input', 'initial_input', 'conditions', 'instructions', 'retrievals', 'mcp_protocol',
                    'presentation', 'integrity', 'review_input', 'producer', 'configuration', 'executable'):
            if record.get(key):
                identities.append({'binding': record[key], 'verified_from': historical_binding(
                    record[key], plan['producer_head'], repository)})
        for value in record.get('support', []) + record.get('original_outputs', []):
            identities.append({'binding': value, 'verified_from': historical_binding(
                value, plan['producer_head'], repository)})
        ledger, invalid = events(record['retrievals']['path'])
        require(not invalid, 'malformed historical ledger')
        if review:
            calls = [{'stage': 'review', 'kind': 'local_context_probe', 'process': record['context_probe']},
                     {'stage': 'review', 'kind': 'model_call', 'process': record['process']}]
        else:
            calls = record['calls']
        stages, host_calls = [], []
        for call in calls:
            process = call['process']
            for key in ('stdout', 'stderr', 'stdin'):
                check_binding(process[key])
            for key in ('adapter', 'process_owner', 'executable'):
                if process.get(key):
                    identities.append({'binding': process[key], 'verified_from': historical_binding(
                        process[key], plan['producer_head'], repository)})
            process_root = Path(process['stdout']['path']).parent
            check_binding(binding(process_root / 'result.json'))
            require(json.loads((process_root / 'result.json').read_bytes()) == process, 'process receipt mismatch')
            if call['kind'] == 'model_call':
                public, invalid = events(process['stdout']['path'])
                require(not invalid, 'malformed historical host events')
                host_calls.extend(e['item'] for e in public if e.get('type') == 'item.completed'
                                  and e.get('item', {}).get('type') == 'mcp_tool_call')
                stages.append({'stage': call['stage'], **stage_timeline(
                    process, process_root.parent / 'observed-context.json', ledger)})
        observed_audit = retrieval_audit(verify(record['input']['path']),
                                        record.get('lane', 'archive_diagnostic'), host_calls, ledger)
        require(observed_audit == record['evidence_reads'], 'historical read audit mismatch')
        reports.append({'name': name, 'receipt': binding(receipt), 'identities': identities,
                        'status': record['status'], 'calls': calls, 'stages': stages,
                        'evidence_reads': observed_audit, 'ledger_rows': len(ledger),
                        'returned_bytes': sum(r['charged_bytes'] for r in ledger),
                        'tokens': record['tokens'], 'price': record['price'],
                        'original_outputs': record.get('original_outputs', []),
                        'original_review': record.get('original_review'),
                        'workspace_cleanup': record['workspace_cleanup']})
    return {'cohort_plan': binding(root / 'plan.json'), 'attempts': reports,
            'limits': 'Historical observations only. No semantic assessment or new model execution. '
                      'Unreported token usage, provider latency and preparation before capture remain unknown.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cohort', type=Path, required=True)
    parser.add_argument('--repository', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit_cohort(args.cohort, args.repository)
    with args.output.open('xb') as stream:
        stream.write(encoded(result))
    print(json.dumps({'attempts': len(result['attempts']), 'identities': 'verified', 'provider_dispatch': 'not_run'}))
