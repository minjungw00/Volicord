"""Freeze a blocked fresh-generation plan beside unedited original presentations."""
from __future__ import annotations

import argparse
import base64
from collections import Counter
from html.parser import HTMLParser
import json
from pathlib import Path
import shutil
import subprocess

from approaches import DESTINATION, HERE, attempt, events
from inputs import append_index, binding, check_binding, digest, encoded, require, verify
from render_comparison import card, document


class OriginalBytes(HTMLParser):
    """Independent transport oracle: original prose and diagnostic downloads."""
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.prose, self.downloads, self.active = [], [], False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'pre' and attrs.get('class') == 'prose':
            self.active = True; self.prose.append('')
        if tag == 'a' and attrs.get('download', '').endswith('.bin'):
            self.downloads.append((attrs['download'], base64.b64decode(attrs['href'].split(',', 1)[1])))

    def handle_data(self, data):
        if self.active:
            self.prose[-1] += data

    def handle_endtag(self, tag):
        if tag == 'pre':
            self.active = False


def verify_presentation(plan):
    """Real-source byte equality only; original content is never a semantic oracle."""
    check_binding(plan['original_presentation'])
    parser = OriginalBytes(); parser.feed(Path(plan['original_presentation']['path']).read_text())
    expected_prose, transports, selections = [], [], []
    for original in plan['originals']:
        check_binding(original['attempt']); check_binding(original['input'])
        record = json.loads(Path(original['attempt']['path']).read_bytes())
        entries = {e['id']: e for e in verify(record['input']['path'])['entries']}
        ordered = sorted(enumerate(record['original_outputs']),
                         key=lambda item: item[1] != record.get('generation_output'))
        for number, output in ordered:
            check_binding(output)
            raw = Path(output['path']).read_bytes()
            response = json.loads(raw)
            expected_prose.append(response['prose'])
            transports.append(('original-response-' + str(number + 1) + '.bin', raw))
            witness = original['witness']['outputs'][number]
            require([row['selection'] for row in witness['selections']] == response['selections'],
                    'original selections changed')
            for index, row in enumerate(witness['selections'], 1):
                if row['reference_status'].startswith('valid_reference'):
                    selection = row['selection']; entry = entries[selection['id']]
                    check_binding(entry['asset'])
                    selections.append(('selection-' + str(index) + '.bin',
                                       Path(entry['asset']['path']).read_bytes()[selection['start']:selection['end']]))
    require(parser.prose == expected_prose, 'original prose changed or display bound reached')
    require(Counter(parser.downloads) == Counter(transports + selections),
            'original/selected source download bytes differ or display bound reached')
    return {'prose_outputs': len(expected_prose), 'selection_downloads': len(selections),
            'identity': 'passed', 'semantic_correctness': 'pending', 'human_comprehension': 'pending'}


def prepare(baselines, output, *, model, effort, executable=None, conditions_path=None,
            allow_reader_baselines=False):
    """Local only. This API cannot authorize or dispatch a model call."""
    require(baselines and model and effort, 'original attempts and explicit runtime required')
    output = Path(output).resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    grouped, originals, cards, audits = {}, [], [], []
    for number, path in enumerate(baselines, 1):
        record = json.loads(Path(path).read_bytes())
        require(allow_reader_baselines or not record.get('output_contract'),
                'baseline must be the original output condition')
        spec = verify(record['input']['path'])
        check_binding(record['input'])
        require(record['scope'] == spec['scope'] and record['original_outputs'], 'baseline scope/output missing')
        for original in record['original_outputs']:
            check_binding(original)
        key = record['input']['sha256']
        grouped.setdefault(key, record['input'])
        body, witness = card(path, 'Original output · presentation only · sample ' + str(number))
        cards.append(body)
        originals.append({'attempt': binding(path), 'input': record['input'],
                          'conditions': record['conditions'], 'instructions': record['instructions'],
                          'original_outputs': record['original_outputs'], 'witness': witness})
        processes = []
        for call in record.get('calls', []):
            process = call['process']
            for stream in ('stdin', 'stdout', 'stderr'):
                check_binding(process[stream])
            receipt = Path(process['stdout']['path']).parent / 'result.json'
            require(json.loads(receipt.read_bytes()) == process, 'original process receipt changed')
            processes.append({'kind': call['kind'], 'stage': call['stage'], 'receipt': binding(receipt),
                              **{k: process.get(k) for k in ('exit_code', 'returncode', 'streams_complete',
                                  'duration_seconds', 'stop_cause', 'cleanup')}})
            if call['kind'] == 'model_call':
                public, invalid = events(process['stdout']['path'])
                messages = [event['item']['text'] for event in public if event.get('type') == 'item.completed'
                            and event.get('item', {}).get('type') == 'agent_message']
                responses = [o for o in record['original_outputs'] if Path(o['path']).parent.name == call['stage']]
                processes[-1]['original_response_host_binding'] = (
                    not invalid and len(responses) == 1 and bool(messages) and
                    messages[-1].strip() == Path(responses[0]['path']).read_text().strip())
        audits.append({'attempt': binding(path), 'status': record['status'], 'processes': processes,
                       'resources': record.get('resource_accounting'), 'tokens': record.get('tokens'),
                       'elapsed_seconds': record.get('execution_elapsed_seconds'), 'price': record.get('price'),
                       'safety': record.get('diagnostic_outcome'), 'outputs': witness['outputs']})
    executable = executable or shutil.which('codex')
    require(executable, 'installed executable unavailable')
    conditions = Path(conditions_path or HERE / 'conditions-reader.json')
    runtime = {'model': model, 'reasoning_effort': effort, 'destination': DESTINATION, 'authorization': None}
    runtime_path = output / 'blocked-runtime.json'; runtime_path.write_bytes(encoded(runtime))
    generations = []
    for number, manifest in enumerate(grouped.values(), 1):
        spec = verify(manifest['path'])
        directory = output / ('fresh-' + str(number))
        record = attempt(manifest['path'], 'direct', 'archive_diagnostic', runtime, directory,
                         executable=executable, conditions_path=conditions)
        require(record['status'] == 'blocked' and not record['calls'], 'preparation cannot dispatch')
        authorization_scope = {'destination': DESTINATION, 'purpose': 'explanation-generation-experiment',
            'input_sha256': manifest['sha256'], 'lane': 'archive_diagnostic', 'approach': 'direct',
            'conditions_sha256': record['conditions']['sha256'],
            'instructions_sha256': record['instructions']['sha256']}
        if record['output_contract'] == 'work_directed_reader':
            authorization_scope.update(response_schema_sha256=record['response_schema']['sha256'],
                executable_sha256=record['executable']['sha256'], support_sha256=digest(encoded(record['support'])),
                model=model, reasoning_effort=effort)
        generations.append({'scope': spec['scope'], 'input': manifest, 'attempt': binding(directory / 'attempt.json'),
            'initial_input': record['initial_input'], 'response_schema': record['response_schema'],
            'executable': record['executable'], 'authorization_scope': authorization_scope,
            'original_input_roles': sorted({e['role'] for e in spec['entries']}),
            'source_count': sum(e['role'] == 'source' for e in spec['entries']),
            'starting_inventory_matches_original': all(
                json.loads(Path(original['attempt']['path']).read_bytes())['initial_input']['sha256']
                == record['initial_input']['sha256'] for original in originals if original['input'] == manifest),
            'current_product_baseline': 'unavailable',
            'future_command': ['python3', '-B', str(HERE / 'approaches.py'), '--manifest', manifest['path'],
                '--approach', 'direct', '--lane', 'archive_diagnostic', '--runtime',
                str(output / ('authorized-runtime-' + str(number) + '.json')),
                '--conditions', str(conditions), '--output', str(output / ('measured-' + str(number)))],
            'state': record['status'], 'blockers': record['blockers']})
    presentation = output / 'original-presentation.html'
    presentation.write_text(document(cards))
    plan = {'format_version': 1, 'classification': 'archive-grounded feasibility; not Product output',
            'producer': binding(Path(__file__)),
            'consumers': [binding(HERE / name) for name in ('reader_contract.py', 'source_reading.py',
                                                           'render_comparison.py', 'grounding.py')],
            'producer_head': subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=HERE, check=True,
                                             capture_output=True, text=True).stdout.strip(),
            'comparison': 'existing output/presentation only versus fresh reader-oriented generation; '
                          'instructions, source prioritization and output schema change together; '
                          'not a controlled single-factor model experiment',
            'originals': originals, 'original_presentation': binding(presentation), 'fresh': generations,
            'conditions': binding(conditions), 'instructions': record['instructions'],
            'runtime': runtime, 'budgets': json.loads(conditions.read_bytes())['budgets'],
            'model_calls': 0, 'generation_results': 'missing_current_authorization', 'automatic_retries': 0,
            'context': 'independent fresh process/home/cwd; opaque-ID MCP; independent filesystem access '
                       'remains cooperative and audited; no original prose is supplied to fresh input',
            'retention': 'ignored local originals and audit retained; provider retention/deletion unknown',
            'quality': 'independent semantic correctness, relevance and user comprehension pending',
            'attribution': 'presentation fidelity can be checked locally; narrative/selection improvement '
                           'and causal attribution remain unmeasured without fresh responses'}
    plan['presentation_verification'] = verify_presentation(plan)
    (output / 'baseline-audit.json').write_bytes(encoded(audits))
    (output / 'plan.json').write_bytes(encoded(plan))
    # Recheck originals after rendering/freezing; immutable comparisons are never edited.
    for original in originals:
        check_binding(original['attempt']); check_binding(original['input'])
        for response in original['original_outputs']:
            check_binding(response)
    append_index(output, {'input': 'verified', 'approach': 'blocked_reader_plan', 'feedback': 'not_requested'},
                 [output / 'plan.json', output / 'baseline-audit.json', presentation, runtime_path])
    return plan


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-attempt', type=Path, action='append', required=True)
    parser.add_argument('--model', required=True)
    parser.add_argument('--reasoning-effort', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--conditions', type=Path)
    parser.add_argument('--allow-reader-baselines', action='store_true')
    args = parser.parse_args()
    plan = prepare(args.baseline_attempt, args.output, model=args.model, effort=args.reasoning_effort,
                   conditions_path=args.conditions, allow_reader_baselines=args.allow_reader_baselines)
    print(json.dumps({'state': 'blocked', 'model_calls': 0, 'originals': len(plan['originals']),
                      'fresh_scopes': len(plan['fresh']), 'plan': str(args.output / 'plan.json')}))
