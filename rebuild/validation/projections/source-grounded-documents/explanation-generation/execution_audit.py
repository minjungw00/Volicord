"""Read retained public process/tool observations; never inspect private reasoning."""
from __future__ import annotations

import argparse
import base64
from collections import Counter
from html.parser import HTMLParser
from datetime import datetime
import json
from pathlib import Path
import subprocess
import tomllib

from approaches import context_audit, events, retrieval_audit
from grounding import validate_output
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
            # Code-mode public text can contain multiple complete JSON results
            # in one content block. Locate each original row without treating
            # malformed/truncated lines as returned evidence. Single-line
            # non-JSON text must terminate recursion.
            lines = value.splitlines()
            return [sequence for line in lines for sequence in sequences(line)] if len(lines) > 1 else []
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
            'publicly_located_ledger_sequences': sorted({s for b in evidence for s in b['ledger_sequences']}),
            'timing_limits': 'first/last and evidence batch totals cover only public results exposing ledger sequences; '
                             'filtered/truncated tool output can omit other reads; batch timing includes host orchestration '
                             'and does not give per-read/provider timing; '
                             'wall timestamps must not be subtracted from monotonic durations'}


def assessment_display(record):
    """Only the bound review server's exact display extension qualifies."""
    if not record.get('configuration') or not record.get('presentation'):
        return None
    check_binding(record['configuration'])
    server = tomllib.loads(Path(record['configuration']['path']).read_text()).get('mcp_servers', {}).get('evidence', {})
    tools = server.get('enabled_tools', [])
    if not set(tools) & {'display_inventory', 'display_read'}:
        return None
    require(set(tools) == {'inventory', 'read', 'display_inventory', 'display_read'}
            and len(tools) == 4, 'unsupported assessment display tools')
    adapter = list({encoded(s): s for s in record.get('support', [])
                    if Path(s['path']).name == 'display_transport.py'}.values())
    require(len(adapter) == 1 and server.get('args', [])[:2] == ['-B', adapter[0]['path']],
            'assessment display adapter unbound')
    check_binding(adapter[0])
    check_binding(record['presentation'])
    return record['presentation']


def display_index(data):
    """Original experimental HTML index coordinates; no execution or Source authority."""
    text = data.decode('utf-8')
    offsets, cursor = [0], 0
    for character in text:
        cursor += len(character.encode())
        offsets.append(cursor)
    lines = [0] + [n + 1 for n, c in enumerate(text) if c == '\n']

    class Index(HTMLParser):
        def __init__(self):
            super().__init__(convert_charrefs=False)
            self.stack, self.rows = [], []

        def point(self):
            line, column = self.getpos()
            return lines[line - 1] + column

        def handle_starttag(self, tag, attrs):
            if tag in {'h1', 'h2', 'h3', 'p', 'pre', 'summary', 'blockquote', 'li', 'a'}:
                self.stack.append((tag, self.point(), len(self.get_starttag_text()), dict(attrs)))

        def handle_endtag(self, tag):
            matches = [n for n, row in enumerate(self.stack) if row[0] == tag]
            if matches:
                _, start, size, attrs = self.stack.pop(matches[-1])
                end = self.point()
                self.rows.append({'tag': tag, 'start': offsets[start], 'end': offsets[end + len('</' + tag + '>')],
                    'content_start': offsets[start + size], 'content_end': offsets[end],
                    'id': attrs.get('id'), 'class': attrs.get('class'),
                    'href': attrs.get('href') if not attrs.get('href', '').startswith('data:') else 'embedded-download',
                    'preview': text[start + size:end][:160]})
    parser = Index(); parser.feed(text); parser.close()
    return [dict(row, ordinal=n) for n, row in enumerate(sorted(parser.rows, key=lambda r: r['start']))]


def expected_display_result(display, name, arguments):
    check_binding(display)
    data = Path(display['path']).read_bytes()
    require(isinstance(arguments, dict) and set(arguments) <= {'offset', 'limit'}, 'unexpected display arguments')
    offset, limit = arguments.get('offset', 0), arguments.get('limit', 40 if name == 'display_inventory' else 2048)
    require(type(offset) is int and offset >= 0 and type(limit) is int and limit > 0, 'invalid display range')
    if name == 'display_inventory':
        require(limit <= 80, 'invalid display range')
        index = display_index(data)
        result = {'display': display, 'elements': index[offset:offset + limit], 'total': len(index),
                  'next_offset': offset + limit if offset + limit < len(index) else None,
                  'boundary': 'Mechanical HTML element index only; preview is incomplete raw content. All raw display bytes available on display_read; no Source authority.'}
        return result, len(encoded(result))
    require('offset' in arguments, 'unexpected display arguments')
    require(offset <= len(data), 'invalid display range')
    selected = data[offset:offset + limit]
    return {'display': display, 'offset': offset, 'bytes': len(selected), 'sha256': digest(selected),
            'text': selected.decode('utf-8'), 'authority': 'untrusted generated display, not original Source'}, len(selected)


class DisplayOriginals(HTMLParser):
    """Inspect the maintained diagnostic display's original transports and prose.

    This reads historical HTML without re-rendering it with today's producer.
    Hidden citation text retains the original literal byte coordinates.
    """
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.samples, self.current, self.active = [], None, None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'article':
            require(self.current is None, 'nested displayed sample')
            self.current = {'label': '', 'prose': [], 'originals': []}
        if self.current is None:
            return
        if tag == 'h2' and not self.current['label']:
            self.active = 'label'
        if tag == 'pre' and attrs.get('class') == 'prose':
            self.active = 'prose'
            self.current['prose'].append('')
        if tag == 'a' and attrs.get('download', '').startswith('original-response-'):
            require(attrs['href'].startswith('data:application/octet-stream;base64,'),
                    'unexpected original display transport')
            self.current['originals'].append((attrs['download'], base64.b64decode(
                attrs['href'].split(',', 1)[1], validate=True)))

    def handle_data(self, data):
        if self.current and self.active == 'label':
            self.current['label'] += data
        elif self.current and self.active == 'prose':
            self.current['prose'][-1] += data

    def handle_endtag(self, tag):
        if tag in {'h2', 'pre'}:
            self.active = None
        if tag == 'article':
            require(self.current is not None, 'unpaired displayed sample')
            self.samples.append(self.current)
            self.current, self.active = None, None


def attest_display(integrity, attempts):
    from source_reading import DISPLAY_BYTES
    require(not integrity.get('editorial') and integrity.get('presentation_mode', 'diagnostic') == 'diagnostic',
            'full assessment requires original diagnostic display')
    parser = DisplayOriginals()
    parser.feed(Path(integrity['presentation']['path']).read_text())
    require(parser.current is None and len(parser.samples) == len(integrity['samples']),
            'displayed samples incomplete or duplicated')
    for shown, sample, attempt in zip(parser.samples, integrity['samples'], attempts):
        require(shown['label'] == sample['label'], 'displayed sample identity changed')
        originals = attempt['original_outputs']
        require(len({o['path'] for o in originals}) == len(originals)
                and [o['original'] for o in sample['outputs']] == originals,
                'review display differs from attempt originals')
        transports, prose = [], []
        ordered = sorted(enumerate(originals, 1), key=lambda item: item[1] != attempt.get('generation_output'))
        for number, original in ordered:
            check_binding(original)
            raw = Path(original['path']).read_bytes()
            transports.append(('original-response-' + str(number) + '.bin', raw))
            response = json.loads(raw)
            text = response['prose']
            require(len(text.encode()) <= DISPLAY_BYTES, 'full review prose exceeds display bound')
            prose.append(text)
        require(Counter(shown['originals']) == Counter(transports) and shown['prose'] == prose,
                'displayed original bytes or prose changed')


def audit_cohort(root, repository, *, receipts=None, plan_path=None):
    root, repository = Path(root).resolve(), Path(repository).resolve()
    plan_path = Path(plan_path or root / 'plan.json')
    plan = json.loads(plan_path.read_bytes())
    reports = []
    if receipts is None:
        receipts = [root / name / filename for name, filename in (
            ('work-b-direct', 'attempt.json'), ('work-b-note_then_prose', 'attempt.json'),
            ('click-direct', 'attempt.json'), ('click-note_then_prose', 'attempt.json'),
            ('work-b-independent-review', 'review-receipt.json'),
            ('click-independent-review', 'review-receipt.json'))]
    require(receipts and len({str(Path(p).resolve()) for p in receipts}) == len(receipts),
            'distinct explicit receipts required')
    for receipt in map(Path, receipts):
        require(receipt.name in {'attempt.json', 'review-receipt.json'}, 'unknown receipt kind')
        name = receipt.parent.name
        review = receipt.name == 'review-receipt.json'
        record = json.loads(receipt.read_bytes())
        identities = []
        for key in ('input', 'initial_input', 'conditions', 'instructions', 'retrievals', 'mcp_protocol',
                    'presentation', 'integrity', 'review_input', 'producer', 'configuration', 'executable'):
            if record.get(key):
                identities.append({'binding': record[key], 'verified_from': historical_binding(
                    record[key], plan['producer_head'], repository)})
        if review and record.get('original_review'):
            # Reviewer output is a runtime artifact: Git history cannot repair it.
            check_binding(record['original_review'])
            identities.append({'binding': record['original_review'], 'verified_from': 'original_path'})
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
        stages, host_calls, scope_calls, pending_tools = [], [], [], set()
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
                scope_calls.extend(e['item'] for e in public
                                   if e.get('item', {}).get('type', '').endswith(
                                       ('tool_call', 'command_execution')))
                for event in public:
                    item = event.get('item', {})
                    if item.get('type', '').endswith(('tool_call', 'command_execution')):
                        identity = (call['stage'], item.get('id'))
                        if event.get('type') in {'item.started', 'item.updated'}:
                            pending_tools.add(identity)
                        elif event.get('type') == 'item.completed':
                            pending_tools.discard(identity)
                stages.append({'stage': call['stage'], **stage_timeline(
                    process, process_root.parent / 'observed-context.json', ledger)})
        from reading_input import verify_reading_input
        spec = verify_reading_input(record['input']['path'])
        display = assessment_display(record) if review else None
        observed_audit = retrieval_audit(spec,
                                        record.get('lane', 'archive_diagnostic'), host_calls, ledger, budgets=record.get('budgets'), display=display)
        historical_audit_matches = observed_audit == record['evidence_reads']
        if review and record['status'] == 'review_captured':
            require(record.get('assessment_scope', 'full_display') == 'full_display',
                    'partial assessment cannot certify full display')
            process = record['process']
            require(record.get('original_review') and record.get('scope') == spec['scope']
                    and process['outcome'] == 'succeeded' and process['returncode'] == 0
                    and process['streams_complete'] and process['cleanup']['complete']
                    and record['context_probe']['streams_complete'] and not record.get('exposure_issues')
                    and record['context_probe']['outcome'] == 'succeeded'
                    and record['context_probe']['returncode'] == 0
                    and record['context_probe']['cleanup']['complete']
                    and observed_audit['verified_source_reads'] > 0 and not observed_audit['issues']
                    and record['workspace_cleanup'] == 'complete', 'incomplete review claimed completion')
            # Recompute observed scope; a stored empty issue list is not proof.
            require(not pending_tools and not context_audit(record['context_probe'],
                    Path(record['context_probe']['stdout']['path']).read_bytes(),
                    Path(record['review_input']['path']).read_text(), scope_calls,
                    allowed_tools={'inventory', 'read', 'display_inventory', 'display_read'} if display else {'inventory', 'read'}),
                    'review context/scope unverified')
            check_binding(record['observed_context'])
            context = json.loads(Path(record['observed_context']['path']).read_bytes())
            sessions = [r['payload']['id'] for r in context if r.get('type') == 'session_meta']
            public, invalid = events(process['stdout']['path'])
            host_sessions = [e['thread_id'] for e in public if e.get('type') == 'thread.started']
            require(sessions and sessions == record['review_sessions']
                    and sessions == host_sessions
                    and not any(r.get('type') == 'compacted' for r in context),
                    'review session separation unverified')
            require(all(record['review_input'][k] == process['stdin'][k] for k in ('sha256', 'bytes')),
                    'review input/process binding changed')
            integrity = json.loads(Path(record['integrity']['path']).read_bytes())
            require(integrity['presentation'] == record['presentation'], 'review display identity changed')
            displayed = {s['label']: s for s in integrity['samples']}
            require(record.get('displayed_outputs'), 'review target outputs absent')
            targets = record['displayed_outputs']
            require(len(displayed) == len(integrity['samples']) == len(targets)
                    and {t['label'] for t in targets} == set(displayed),
                    'review target samples incomplete or duplicated')
            generator_sessions = set()
            displayed_attempts = []
            for target in targets:
                sample = displayed[target['label']]
                check_binding(sample['attempt'])
                attempt = json.loads(Path(sample['attempt']['path']).read_bytes())
                require(attempt['input'] == record['input'] and attempt['scope'] == spec['scope'],
                        'review target Work/input changed')
                displayed_attempts.append(attempt)
                require([o['original'] for o in sample['outputs']] == attempt['original_outputs']
                        and len({o['path'] for o in attempt['original_outputs']}) == len(attempt['original_outputs']),
                        'review display differs from attempt originals')
                target_sessions = []
                for call in attempt.get('calls', []):
                    if call['kind'] != 'model_call':
                        continue
                    check_binding(call['process']['stdout'])
                    generation, malformed = events(call['process']['stdout']['path'])
                    require(not malformed, 'generation session evidence malformed')
                    target_sessions.extend(e['thread_id'] for e in generation
                                           if e.get('type') == 'thread.started')
                require(target_sessions, 'generation session evidence absent')
                generator_sessions.update(target_sessions)
                originals = {o['original']['path']: o for o in sample['outputs']}
                require(target['outputs'], 'review target outputs absent')
                require(len(originals) == len(sample['outputs']) == len(target['outputs'])
                        and {o['path'] for o in target['outputs']} == set(originals),
                        'review target originals incomplete or duplicated')
                for claimed in target['outputs']:
                    original = originals[claimed['path']]
                    check_binding(original['original'])
                    require(original['original'] in attempt['original_outputs']
                            and all(claimed[k] == original['original'][k]
                                    for k in ('path', 'bytes', 'sha256')), 'review target output changed')
                    if 'prose_sha256' in claimed:
                        body = json.loads(Path(original['original']['path']).read_bytes())
                        require(claimed['prose_sha256'] == original['prose_sha256']
                                == digest(body['prose'].encode()), 'review target prose changed')
            attest_display(integrity, displayed_attempts)
            require(generator_sessions == set(record['generator_sessions'])
                    and not set(sessions) & generator_sessions, 'review session separation unverified')
            raw = Path(record['original_review']['path']).read_bytes()
            public, invalid = events(process['stdout']['path'])
            messages = [e['item']['text'] for e in public if e.get('type') == 'item.completed'
                        and e.get('item', {}).get('type') == 'agent_message']
            require(not invalid and messages and messages[-1].strip() == raw.decode('utf-8').strip(),
                    'review output/final host response changed')
            validation = validate_output(json.loads(raw), spec, 'archive_diagnostic', ledger)
            require(not any(s['reference_status'] == 'invalid' for s in validation['selections']),
                    'completed review references invalid')
        reports.append({'name': name, 'receipt': binding(receipt), 'identities': identities,
                        'status': record['status'], 'calls': calls, 'stages': stages,
                        'evidence_reads': observed_audit, 'historical_read_audit_matches': historical_audit_matches, 'ledger_rows': len(ledger),
                        'returned_bytes': sum(r['charged_bytes'] for r in ledger),
                        'tokens': record['tokens'], 'price': record['price'],
                        'original_outputs': record.get('original_outputs', []),
                        'original_review': record.get('original_review'),
                        'review_completion_verified': review and record['status'] == 'review_captured',
                        'workspace_cleanup': record['workspace_cleanup']})
    return {'cohort_plan': binding(plan_path), 'attempts': reports,
            'limits': 'Historical observations only. No semantic assessment or new model execution. '
                      'Unreported token usage, provider latency and preparation before capture remain unknown.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cohort', type=Path, required=True)
    parser.add_argument('--repository', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, action='append',
                        help='Exact original receipt; repeat to audit a separately identified cohort.')
    parser.add_argument('--plan', type=Path, help='Explicit producer plan; never relabel original receipts.')
    args = parser.parse_args()
    result = audit_cohort(args.cohort, args.repository, receipts=args.receipt, plan_path=args.plan)
    with args.output.open('xb') as stream:
        stream.write(encoded(result))
    print(json.dumps({'attempts': len(result['attempts']), 'identities': 'verified', 'provider_dispatch': 'not_run'}))
