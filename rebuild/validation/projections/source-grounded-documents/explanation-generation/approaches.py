"""One initial comparison opportunity per Work/approach, with bounded fresh calls."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil
import sys
import tempfile
import time

from grounding import validate_output
from inputs import append_index, binding, encoded, generation_inventory, require, verify
from invocations import HERE, REPOSITORY, capture, environment

APPROACHES = ('current', 'direct', 'note_then_prose')
DESTINATION = 'OpenAI Codex service'


def initial_input(spec, lane):
    view = generation_inventory(spec, lane)
    # Mechanical inventory, no case-specific ranking. Bodies stay behind Reader.
    return {'scope': view['scope'], 'identities': view['identities'],
            'inventory_boundary': view['inventory_boundary'],
            'starting_evidence': [e for e in view['entries'] if e['role'] != 'source'],
            'recorded_diff_inventory': [e for e in view['entries'] if e['role'] == 'source'
                                        and e['attribution'] in {'explicit_work_patch', 'explicit_before_patch'}],
            'source_count': sum(e['role'] == 'source' for e in view['entries']),
            'source_inventory': 'Use the inventory tool to page the complete unranked source inventory.'}


def current_blockers(spec):
    preparations = [e for e in spec['entries'] if e['role'] == 'current_preparation'
                    and e['lane'] == 'product' and e['representation'] == 'full_file'
                    and e['chronology']['state'] == 'known']
    if not preparations:
        return ['cutoff_bound_current_prepare_missing']
    # A frozen prose plan alone cannot establish which executable produced it,
    # or establish that preparation ran on a disposable Runtime. Fail closed.
    return ['current_producer_and_disposable_runtime_execution_receipt_missing']


def context_audit(prompt_result, prompt_bytes, configured_prompt, calls):
    """Only scoped MCP calls qualify; opaque event gaps cannot certify isolation."""
    issues = []
    if prompt_result['outcome'] != 'succeeded' or not prompt_result['streams_complete']:
        issues.append('configured_context_uninspectable')
    try:
        context = json.loads(prompt_bytes)
        require(isinstance(context, list), 'prompt context must be a list')
        text = json.dumps(context, ensure_ascii=False)
        bodies = [part.get('text', '') for item in context for part in item.get('content', [])]
        require(configured_prompt in bodies, 'configured user prompt absent')
        # Fixed prohibited artifact families, not answer strings or named functions.
        for needle in ('prototype-data', 'agent-review-support', 'original-inventory.json',
                       'remediation-plan', 'answer-tour', 'qualitative-verdict'):
            if needle in text:
                issues.append('prohibited_context:' + needle)
    except (ValueError, TypeError):
        issues.append('configured_context_uninspectable')
    for call in calls:
        if call.get('type') not in {'mcp_tool_call'} or call.get('server') != 'evidence':
            issues.append('non_scoped_tool_observed')
    return sorted(set(issues))


def events(path):
    result, invalid = [], False
    for line in Path(path).read_bytes().splitlines():
        try:
            result.append(json.loads(line))
        except (ValueError, UnicodeError):
            invalid = True
    return result, invalid


def attempt(manifest, approach, lane, runtime, output, *, executable=None, auth_path=None):
    require(approach in APPROACHES and lane in {'product', 'archive_diagnostic'}, 'unsupported experiment scope')
    spec = verify(manifest)
    conditions = json.loads((HERE / 'conditions.json').read_bytes())
    output = Path(output).resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    frozen_initial = initial_input(spec, lane)
    (output / 'initial-input.json').write_bytes(encoded(frozen_initial))
    budgets = conditions['budgets']
    record = {'format_version': 1, 'approach': approach, 'lane': lane, 'input': binding(manifest),
              'initial_input': binding(output / 'initial-input.json'), 'scope': spec['scope'],
              'runtime': runtime, 'budgets': budgets, 'conditions': binding(HERE / 'conditions.json'),
              'instructions': binding(HERE / 'instructions.txt'),
              'support': [binding(HERE / p) for p in ('approaches.py', 'source_tools.py', 'grounding.py', 'response-schema.json', 'invocations.py')],
              'status': 'not_run', 'blockers': [], 'calls': [], 'original_outputs': [],
              'generation_output': None, 'tokens': [], 'price': None, 'corrections': [],
              'isolation': 'fresh process/home/cwd; cooperative filesystem access, not enforced isolation',
              'semantic_quality': 'not_assessed', 'clean_comparison': False}
    if approach == 'current':
        record['blockers'].extend(current_blockers(spec))
    require(set(runtime) == {'model', 'reasoning_effort', 'destination', 'authorization'}, 'explicit runtime fields required')
    expected_scope = {'destination': runtime['destination'], 'purpose': 'explanation-generation-experiment',
                      'input_sha256': record['input']['sha256'], 'lane': lane,
                      'conditions_sha256': record['conditions']['sha256'],
                      'instructions_sha256': record['instructions']['sha256'], 'approach': approach}
    authorization = runtime['authorization']
    if (not isinstance(authorization, dict) or authorization.get('scope') != expected_scope
            or not authorization.get('current_request_locator')):
        record['blockers'].append('current_destination_purpose_source_authorization_missing')
    if runtime['destination'] != DESTINATION or not runtime['model'] or not runtime['reasoning_effort']:
        record['blockers'].append('explicit_supported_runtime_missing')
    executable = executable or shutil.which('codex')
    if not executable:
        record['blockers'].append('installed_codex_unavailable')
    if not record['blockers']:
        try:
            _execute(record, spec, manifest, frozen_initial, output, executable, auth_path)
        except (ValueError, KeyError, TypeError, OSError) as error:
            record.update(status='adapter_failed', adapter_error={'kind': type(error).__name__, 'message': str(error)})
    else:
        record['status'] = 'blocked'
    # Original outputs and streams have already been written. Validation adds a
    # separate file, never replaces a response. Zero correction/retry budget.
    path = output / 'attempt.json'
    path.write_bytes(encoded(record))
    append_index(output.parent, {'input': 'verified', 'approach': record['status'], 'feedback': 'not_requested'},
                 [path, output / 'initial-input.json', *[p['path'] for p in record['original_outputs']]])
    return record


def _execute(record, spec, manifest, initial, output, executable, auth_path):
    budgets = record['budgets']
    deadline = time.monotonic() + budgets['total_seconds']
    stages = ('analysis', 'prose') if record['approach'] == 'note_then_prose' else ('prose',)
    output_used, stream_used, note = 0, 0, None
    trace = output / 'retrievals.jsonl'
    trace.touch(mode=0o600)
    record['executable'] = binding(executable)
    with tempfile.TemporaryDirectory(prefix='volicord-explanation-call-') as temporary:
        base = Path(temporary)
        require(not base.is_relative_to(REPOSITORY), 'fresh cwd outside repository required')
        for stage in stages:
            require(binding(executable) == record['executable'], 'executable identity drift')
            require(binding(manifest) == record['input'], 'input identity drift')
            for support in record['support']:
                require(binding(support['path']) == support, 'support identity drift')
            remaining = deadline - time.monotonic()
            if remaining <= 0 or stream_used >= budgets['stream_bytes'] or output_used >= budgets['output_bytes']:
                record['status'] = 'budget_exhausted'
                break
            workspace = base / stage
            workspace.mkdir()
            env = environment(workspace)
            # Authentication remains CLI-owned. No credential bytes are read,
            # hashed, copied, or retained by this experiment support.
            if auth_path is not None:
                (workspace / 'codex/auth.json').symlink_to(Path(auth_path).resolve())
            configuration = {'manifest': str(Path(manifest).resolve()), 'lane': record['lane'],
                             'budgets': budgets, 'trace': str(trace)}
            (workspace / 'reader.json').write_bytes(encoded(configuration))
            config = workspace / 'codex/config.toml'
            # Use explicit top-level -c settings. MCP stdio configuration is the
            # documented current CLI shape, checked with installed mcp list.
            with config.open('a') as stream:
                stream.write('\n[mcp_servers.evidence]\ncommand = ' + json.dumps(sys.executable)
                             + '\nargs = ' + json.dumps(['-B', str(HERE / 'source_tools.py'), str(workspace / 'reader.json')])
                             + '\nstartup_timeout_sec = 20\ntool_timeout_sec = 20\n')
            directive = ('Write a short cited technical analysis, separating observations and interpretations.'
                         if stage == 'analysis' else conditions_directive(record['approach']))
            prompt = ((HERE / 'instructions.txt').read_text() + '\n' + directive
                      + '\nLanguage: ' + json.loads((HERE / 'conditions.json').read_bytes())['language']
                      + '\nUse only the evidence MCP tools. Choose important code and paragraph structure yourself.'
                      + '\nReturn free prose and a separate strict sidecar: selections[{id,start,end,sha256,state}], gaps[str].'
                      + '\nSelections use exact retrieved byte spans and their SHA-256, state before/after/context.'
                      + '\nInitial evidence inventory (bodies require reads):\n' + json.dumps(initial, ensure_ascii=False)
                      + ('\nPrior short technical analysis, revisitable and correctable:\n' + note if note else '')
                      + '\nRemaining response UTF-8 byte budget: ' + str(budgets['output_bytes'] - output_used))
            stage_root = output / stage
            stage_root.mkdir()
            settings = ['-c', 'model=' + json.dumps(record['runtime']['model']), '-c',
                        'model_reasoning_effort=' + json.dumps(record['runtime']['reasoning_effort'])]
            probe = capture([executable, *settings, 'debug', 'prompt-input', prompt], cwd=workspace, env=env,
                            output=stage_root / 'context', timeout=min(15, remaining),
                            stream_bytes=budgets['stream_bytes'] - stream_used)
            stream_used += probe['retained_stream_bytes']
            issues = context_audit(probe, Path(probe['stdout']['path']).read_bytes(), prompt, [])
            record['calls'].append({'stage': stage, 'kind': 'local_context_probe', 'process': probe})
            if issues:
                record.update(status='context_blocked', exposure_issues=issues)
                break
            response_path = stage_root / 'original-response.json'
            remaining = deadline - time.monotonic()
            if remaining <= 0 or stream_used >= budgets['stream_bytes']:
                record['status'] = 'budget_exhausted'
                break
            process = capture([executable, *settings, 'exec', '--skip-git-repo-check', '--ignore-rules',
                               '--json', '--color', 'never', '--output-schema', str(HERE / 'response-schema.json'),
                               '-o', str(response_path), '-'], cwd=workspace, env=env,
                              output=stage_root / 'process', timeout=remaining,
                              stream_bytes=budgets['stream_bytes'] - stream_used,
                              cleanup_seconds=budgets['cleanup_seconds'], stdin=prompt.encode())
            stream_used += process['retained_stream_bytes']
            record['calls'].append({'stage': stage, 'kind': 'model_call', 'process': process})
            observed, invalid = events(process['stdout']['path'])
            tool_calls = [event['item'] for event in observed if event.get('type') == 'item.completed'
                          and event.get('item', {}).get('type', '').endswith(('tool_call', 'command_execution'))]
            record['tokens'].extend(event.get('usage') for event in observed if event.get('type') == 'turn.completed')
            record.setdefault('exposure_issues', []).extend(context_audit(probe, Path(probe['stdout']['path']).read_bytes(), prompt, tool_calls))
            if invalid or not process['streams_complete']:
                record['exposure_issues'].append('observed_events_incomplete')
            # Retain model-visible input messages, tool events and exact metadata;
            # no private reasoning transcript is selected for this experiment.
            observed_context = []
            for rollout in (workspace / 'codex/sessions').rglob('*.jsonl'):
                rows, bad = events(rollout)
                if bad:
                    record['exposure_issues'].append('observed_context_uninspectable')
                observed_context.extend(row for row in rows if row.get('type') in {'session_meta', 'turn_context', 'compacted'}
                    or (row.get('type') == 'response_item' and row.get('payload', {}).get('type') != 'reasoning'))
            (stage_root / 'observed-context.json').write_bytes(encoded(observed_context))
            if not observed_context:
                record['exposure_issues'].append('observed_context_uninspectable')
            if any(row.get('type') == 'compacted' for row in observed_context):
                record['exposure_issues'].append('context_compacted')
            if response_path.exists():
                record['original_outputs'].append(binding(response_path))
                output_used += response_path.stat().st_size
            if process['outcome'] != 'succeeded':
                record['status'] = process['outcome']
                break
            if not response_path.exists():
                record['status'] = 'response_absent'
                break
            if output_used > budgets['output_bytes']:
                record['status'] = 'budget_exhausted'
                break
            try:
                response = json.loads(response_path.read_bytes())
                retrievals = [json.loads(line) for line in trace.read_bytes().splitlines()]
                validation = validate_output(response, spec, record['lane'], retrievals)
                (stage_root / 'grounding.json').write_bytes(encoded(validation))
            except (ValueError, KeyError, TypeError, OSError) as error:
                (stage_root / 'grounding.json').write_bytes(encoded({'error': str(error), 'semantic_quality': 'not_assessed'}))
                record['status'] = 'invalid_response'
                break
            if stage == 'analysis':
                note = response_path.read_text()
            else:
                record['generation_output'] = binding(response_path)
                record['status'] = ('invalid_references' if any(s['reference_status'] == 'invalid' for s in validation['selections'])
                                    else 'captured')
        record['workspace_cleanup'] = 'pending'
    record['workspace_cleanup'] = 'complete'
    record['retrievals'] = binding(trace)
    record['output_bytes'] = output_used
    record['retained_stream_bytes'] = stream_used
    record['clean_comparison'] = False  # Requires independent observed-context review.


def conditions_directive(approach):
    conditions = json.loads((HERE / 'conditions.json').read_bytes())
    if approach == 'note_then_prose':
        return 'Write source-grounded natural prose. Revisit evidence and correct the short technical analysis where needed.'
    return conditions['conditions'][approach]['instructions']


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--approach', choices=APPROACHES, required=True)
    parser.add_argument('--lane', choices=('product', 'archive_diagnostic'), required=True)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--auth-path', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = attempt(args.manifest, args.approach, args.lane, json.loads(args.runtime.read_bytes()), args.output, auth_path=args.auth_path)
    print(json.dumps({'status': result['status'], 'blockers': result['blockers'], 'model_calls': sum(c['kind'] == 'model_call' for c in result['calls']),
                      'output_count': len(result['original_outputs'])}))
