"""One initial comparison opportunity per Work/approach, with bounded fresh calls."""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import shutil
import sys
import tempfile
import time

from grounding import validate_output
from inputs import append_index, binding, check_binding, digest, encoded, generation_inventory, require, verify
from invocations import HERE, REPOSITORY, capture, environment

APPROACHES = ('current', 'direct', 'note_then_prose')
DESTINATION = 'OpenAI Codex service'


def initial_input(spec, lane, *, compact=False):
    view = generation_inventory(spec, lane)
    if compact:
        # Keep every initial record/diff ID in the same unranked order. Detailed
        # provenance and all other authorized entries remain on inventory, with
        # bodies on read. This changes context size, never evidence membership.
        view['entries'] = [{k: e[k] for k in ('id', 'path', 'role', 'lane', 'representation',
                                             'attribution', 'before_state', 'missing')}
                           | {'chronology': e['chronology']['state']}
                           for e in view['entries']]
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


def retrieval_audit(spec, lane, calls, retrievals):
    """Join actual completed host results to ledger rows and independently read bytes.

    Process success, inventory, an empty read and model prose are not source proof.
    This reports transport observations, never authorship or semantic quality.
    """
    entries = {e['id']: e for e in spec['entries'] if lane == 'archive_diagnostic' or e['lane'] == 'product'}
    outcomes, matched, issues = [], set(), []
    for call in calls:
        if call.get('type') != 'mcp_tool_call' or call.get('server') != 'evidence':
            continue
        outcome = {'host_id': call.get('id'), 'tool': call.get('tool')}
        try:
            if call.get('error'):
                message = call['error'].get('message', '')
                outcome['outcome'] = 'host_denied' if 'requires approval' in message else 'host_call_failed'
                outcomes.append(outcome)
                continue
            require(call.get('status') in {'completed', 'failed'}, 'host call incomplete')
            content = call['result']['content']
            require(len(content) == 1 and content[0]['type'] == 'text', 'unexpected host tool result')
            row = json.loads(content[0]['text'])
            sequence = row['sequence']
            require(type(sequence) is int and 0 <= sequence < len(retrievals), 'ledger sequence absent')
            require(sequence not in matched and retrievals[sequence] == row, 'host/ledger mismatch')
            require(row['name'] == call['tool'] and row['arguments'] == call['arguments'], 'host arguments mismatch')
            # Codex reports isError Reader replies as terminal failed calls with
            # an exact result payload. Account that denial without letting a
            # failed host call attest a successful evidence return.
            require(call.get('status') != 'failed' or row['status'] == 'failed',
                    'host failure cannot attest returned evidence')
            matched.add(sequence)
            outcome.update(sequence=sequence, outcome='reader_failed')
            if row['status'] == 'failed':
                require(row.get('outcome') in {None, 'policy_denied', 'source_unavailable', 'evidence_changed',
                                               'budget_exhausted', 'invalid_utf8'}, 'invalid failed-read outcome')
                outcome['outcome'] = row.get('outcome', 'reader_failed')
            else:
                require(row['status'] == 'returned', 'invalid ledger status')
            if row['status'] == 'returned' and row['name'] == 'read':
                meta, text = row['result']['metadata'], row['result']['text']
                entry = entries[meta['id']]
                require(entry['project'] == spec['scope']['project'] and entry['work'] == spec['scope']['work'], 'foreign source scope')
                check_binding(entry['asset'])
                start, length = meta['offset'], meta['bytes']
                require(type(start) is int and type(length) is int and start >= 0 and length >= 0, 'invalid returned range')
                require(meta['id'] == row['arguments']['id'] and start == row['arguments'].get('offset', 0)
                        and length <= row['arguments'].get('limit', 2048), 'returned/requested range mismatch')
                with Path(entry['asset']['path']).open('rb') as source:
                    source.seek(start)
                    data = source.read(length)
                require(data == text.encode('utf-8') and len(data) == length
                        and digest(data) == meta['sha256'] and row['charged_bytes'] == length, 'returned bytes mismatch')
                require(meta['representation'] == entry['representation'], 'returned representation mismatch')
                require(meta['role'] == 'untrusted_evidence'
                        and meta['complete_asset'] == (start == 0 and length == entry['asset']['bytes']), 'returned metadata mismatch')
                outcome.update(outcome='source_read_verified' if length else 'empty_read',
                               source_id=meta['id'], offset=start, bytes=length, sha256=meta['sha256'])
            elif row['status'] == 'returned':
                outcome['outcome'] = 'inventory_returned'
        except (ValueError, KeyError, TypeError, OSError, AttributeError) as error:
            outcome.update(outcome='unverified', error=str(error))
            issues.append(str(error))
        outcomes.append(outcome)
    unmatched = [r['sequence'] for r in retrievals if r['sequence'] not in matched]
    if unmatched:
        issues.append('ledger rows without matching host results')
    return {'outcomes': outcomes, 'verified_source_reads': sum(o['outcome'] == 'source_read_verified' for o in outcomes),
            'unmatched_sequences': unmatched, 'issues': sorted(set(issues))}


def evidence_configuration(configuration_path, tool_timeout_seconds=20):
    return ('\n[mcp_servers.evidence]\ncommand = ' + json.dumps(sys.executable)
            + '\nargs = ' + json.dumps(['-B', str(HERE / 'source_tools.py'), str(configuration_path)])
            + '\nstartup_timeout_sec = 20\ntool_timeout_sec = ' + str(tool_timeout_seconds) + '\nrequired = true\n'
            + 'enabled_tools = ["inventory", "read"]\ndefault_tools_approval_mode = "approve"\n')


def execution_policy(conditions):
    policy = conditions.get('execution')
    if policy is None:
        return None  # Historical conditions remain available, byte-identical.
    require(isinstance(policy, dict), 'execution policy must be an object')
    require(isinstance(conditions.get('condition_id'), str) and conditions['condition_id'].strip(),
            'new execution policy requires an explicit condition identity')
    if policy.get('allocation_mode') == 'safety_only':
        require(set(policy) == {'allocation_mode', 'compact_initial_inventory'}
                and type(policy['compact_initial_inventory']) is bool, 'invalid safety-only allocation')
        budgets = conditions['budgets']
        require(set(budgets) == {'total_seconds', 'reads', 'read_bytes', 'output_bytes',
                                'stream_bytes', 'retries', 'cleanup_seconds'}, 'explicit safety ceilings required')
        for key in ('total_seconds', 'cleanup_seconds'):
            require(type(budgets[key]) in {int, float} and math.isfinite(budgets[key])
                    and budgets[key] > 0, 'finite positive safety time required')
        for key in ('reads', 'read_bytes', 'output_bytes', 'stream_bytes'):
            require(type(budgets[key]) is int and budgets[key] > 0, 'positive integer safety ceiling required')
        require(type(budgets['retries']) is int and budgets['retries'] == 0, 'diagnostic retries must be zero')
        return dict(policy, condition_id=conditions['condition_id'])
    require(set(policy) == {'compact_initial_inventory', 'analysis_seconds', 'prose_reserve_seconds',
                            'analysis_output_bytes', 'analysis_finalization_seconds', 'prose_finalization_seconds'},
            'explicit execution allocation required')
    require(type(policy['compact_initial_inventory']) is bool, 'explicit inventory preparation required')
    budgets = conditions['budgets']
    for key in ('analysis_seconds', 'prose_reserve_seconds', 'analysis_finalization_seconds', 'prose_finalization_seconds'):
        require(type(policy[key]) in {int, float} and 0 < policy[key] < budgets['total_seconds'],
                'invalid execution time allocation')
    require(policy['analysis_seconds'] + policy['prose_reserve_seconds'] <= budgets['total_seconds']
            and policy['analysis_finalization_seconds'] < policy['analysis_seconds']
            and policy['prose_finalization_seconds'] < policy['prose_reserve_seconds'], 'infeasible execution allocation')
    require(type(policy['analysis_output_bytes']) is int
            and 0 < policy['analysis_output_bytes'] < budgets['output_bytes'], 'invalid output allocation')
    return dict(policy, condition_id=conditions['condition_id'])


def attempt(manifest, approach, lane, runtime, output, *, executable=None, auth_path=None, conditions_path=None):
    preparation_started = time.monotonic()
    require(approach in APPROACHES and lane in {'product', 'archive_diagnostic'}, 'unsupported experiment scope')
    spec = verify(manifest)
    conditions_path = Path(conditions_path or HERE / 'conditions.json')
    condition_bytes = conditions_path.read_bytes()
    condition_binding = binding(conditions_path)
    require(digest(condition_bytes) == condition_binding['sha256'], 'condition identity drift')
    conditions = json.loads(condition_bytes)
    policy = execution_policy(conditions)
    instruction_bytes = (HERE / 'instructions.txt').read_bytes()
    instruction_binding = binding(HERE / 'instructions.txt')
    require(digest(instruction_bytes) == instruction_binding['sha256'], 'instruction identity drift')
    output = Path(output).resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    (output / 'frozen-conditions.json').write_bytes(condition_bytes)
    (output / 'frozen-instructions.txt').write_bytes(instruction_bytes)
    frozen_initial = initial_input(spec, lane, compact=bool(policy and policy['compact_initial_inventory']))
    (output / 'initial-input.json').write_bytes(encoded(frozen_initial))
    budgets = conditions['budgets']
    record = {'format_version': 1, 'approach': approach, 'lane': lane, 'input': binding(manifest),
              'initial_input': binding(output / 'initial-input.json'), 'scope': spec['scope'],
              'runtime': runtime, 'budgets': budgets, 'conditions': condition_binding,
              'frozen_conditions': binding(output / 'frozen-conditions.json'), 'language': conditions['language'],
              'execution': policy,
              'instructions': instruction_binding,
              'frozen_instructions': binding(output / 'frozen-instructions.txt'),
              'support': [binding(HERE / p) for p in ('approaches.py', 'source_tools.py', 'inputs.py', 'grounding.py', 'response-schema.json', 'invocations.py')],
              'status': 'not_run', 'blockers': [], 'calls': [], 'original_outputs': [],
              'generation_output': None, 'tokens': [], 'price': None, 'corrections': [],
              'isolation': 'fresh process/home/cwd; cooperative filesystem access, not enforced isolation',
              'semantic_quality': 'not_assessed', 'clean_comparison': False}
    record['preparation_budget_boundary'] = 'local manifest verification/freezing precedes execution; stage setup and probes share total_seconds'
    producer_sources = output / 'producer-sources'
    producer_sources.mkdir()
    record['frozen_support'] = []
    for origin in record['support']:
        data = Path(origin['path']).read_bytes()
        require(digest(data) == origin['sha256'] and len(data) == origin['bytes'], 'support identity drift')
        snapshot = producer_sources / Path(origin['path']).name
        snapshot.write_bytes(data)
        record['frozen_support'].append({'origin': origin, 'snapshot': binding(snapshot)})
    record['preparation_seconds'] = time.monotonic() - preparation_started
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
    started = time.monotonic()
    deadline = started + budgets['total_seconds']
    policy = record.get('execution')
    safety_only = bool(policy and policy.get('allocation_mode') == 'safety_only')
    allocated = policy if not safety_only else None
    conditions = json.loads(Path(record['frozen_conditions']['path']).read_bytes())
    record['execution_started_monotonic'] = started
    record['execution_deadline_monotonic'] = deadline
    stages = ('analysis', 'prose') if record['approach'] == 'note_then_prose' else ('prose',)
    output_used, stream_used, note = 0, 0, None
    trace = output / 'retrievals.jsonl'
    trace.touch(mode=0o600)
    protocol_trace = output / 'mcp-protocol.jsonl'
    protocol_trace.touch(mode=0o600)
    all_tool_calls = []
    record['executable'] = binding(executable)
    with tempfile.TemporaryDirectory(prefix='volicord-explanation-call-') as temporary:
        base = Path(temporary)
        require(not base.is_relative_to(REPOSITORY), 'fresh cwd outside repository required')
        for stage in stages:
            require(binding(executable) == record['executable'], 'executable identity drift')
            require(binding(manifest) == record['input'], 'input identity drift')
            for support in record['support']:
                require(binding(support['path']) == support, 'support identity drift')
            check_binding(record['frozen_conditions'])
            check_binding(record['frozen_instructions'])
            stage_deadline = (min(started + allocated['analysis_seconds'], deadline - allocated['prose_reserve_seconds'])
                              if allocated and stage == 'analysis' else deadline)
            remaining = stage_deadline - time.monotonic()
            if allocated and stage == 'prose' and note is not None and remaining < allocated['prose_reserve_seconds']:
                # Cleanup/grounding between calls also consumes the shared time.
                # Report a lost reserve rather than silently start a starved call.
                record['status'] = 'finalization_reserve_unavailable'
                record['prose_seconds_remaining'] = max(0, remaining)
                break
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
                             'budgets': budgets, 'trace': str(trace), 'protocol_trace': str(protocol_trace)}
            allocation = budgets['output_bytes'] - output_used
            if allocated:
                allocation = min(allocation, policy['analysis_output_bytes']) if stage == 'analysis' else allocation
                finalization = policy['analysis_finalization_seconds'] if stage == 'analysis' else policy['prose_finalization_seconds']
                configuration.update(stage=stage, read_deadline_monotonic=stage_deadline - finalization,
                                     execution_deadline_monotonic=deadline)
                record.setdefault('stage_allocations', []).append({'stage': stage,
                    'deadline_monotonic': stage_deadline, 'read_deadline_monotonic': stage_deadline - finalization,
                    'output_bytes': allocation, 'remaining_seconds_before_preparation': remaining})
            elif safety_only:
                configuration.update(stage=stage, safety_only=True,
                                     read_deadline_monotonic=deadline, execution_deadline_monotonic=deadline)
                record.setdefault('stage_allocations', []).append({'stage': stage,
                    'deadline_monotonic': deadline, 'read_deadline_monotonic': deadline,
                    'output_bytes': allocation, 'remaining_seconds_before_preparation': remaining})
            (workspace / 'reader.json').write_bytes(encoded(configuration))
            config = workspace / 'codex/config.toml'
            # Use explicit top-level -c settings. MCP stdio configuration is the
            # documented current CLI shape, checked with installed mcp list.
            with config.open('a') as stream:
                stream.write(evidence_configuration(workspace / 'reader.json',
                    max(1, math.ceil(remaining)) if safety_only else 20))
            stage_root = output / stage
            stage_root.mkdir()
            shutil.copyfile(config, stage_root / 'effective-config.toml')
            record.setdefault('configured_context', []).append(binding(stage_root / 'effective-config.toml'))
            directive = ('Write a short cited technical analysis, separating observations and interpretations.'
                         if stage == 'analysis' else conditions_directive(record['approach'], conditions))
            prompt = (Path(record['frozen_instructions']['path']).read_text() + '\n' + directive
                      + '\nLanguage: ' + conditions['language']
                      + '\nUse only the evidence MCP tools. Choose important code and paragraph structure yourself.'
                      + '\nReturn free prose and a separate strict sidecar: selections[{id,start,end,sha256,state}], gaps[str].'
                      + '\nSelections use exact retrieved byte spans and their SHA-256, state before/after/context.'
                      + '\nInitial evidence inventory (bodies require reads):\n' + json.dumps(initial, ensure_ascii=False)
                      + ('\nPrior short technical analysis, revisitable and correctable:\n' + note if note else '')
                      + '\nRemaining response UTF-8 byte budget: ' + str(allocation))
            if allocated:
                prompt += ('\nExecution allocation: ' + json.dumps({'stage': stage,
                    'stage_seconds_remaining': max(0, stage_deadline - time.monotonic()),
                    'evidence_seconds_remaining': max(0, configuration['read_deadline_monotonic'] - time.monotonic()),
                    'total_reads': budgets['reads'], 'total_returned_bytes': budgets['read_bytes'],
                    'stage_output_bytes': allocation, 'retries': budgets['retries']})
                    + '\nTool results report remaining time/reads/bytes. Finish the complete response within this allocation.'
                    + '\nReserve finalization time; after the read deadline use inspected evidence and explicitly report unresolved material gaps.'
                    + '\nDo not treat absent evidence or unfinished analysis as a complete explanation.')
            elif safety_only:
                prompt += ('\nSafety-only allocation: investigation, note and prose share one outer watchdog'
                    + ' and aggregate ceilings. Choose when to finish analysis and begin prose.'
                    + ' There is no reserved stage time, response allowance or early read cutoff.'
                    + '\nAll tool calls (including denials/inventory), returned evidence bytes, intermediate'
                    + ' responses and process streams count across both stages; no retries.'
                    + '\nRemaining safety seconds: ' + str(max(0, deadline - time.monotonic()))
                    + '\nAggregate ceilings: ' + json.dumps(budgets)
                    + '\nRevisit the same authorized evidence in the final stage. Report unresolved gaps.')
            settings = ['-c', 'model=' + json.dumps(record['runtime']['model']), '-c',
                        'model_reasoning_effort=' + json.dumps(record['runtime']['reasoning_effort'])]
            remaining = stage_deadline - time.monotonic()
            if remaining <= 0:
                record['status'] = 'budget_exhausted'
                break
            probe = capture([executable, *settings, 'debug', 'prompt-input', prompt], cwd=workspace, env=env,
                            output=stage_root / 'context', timeout=remaining if safety_only else min(15, remaining),
                            stream_bytes=budgets['stream_bytes'] - stream_used,
                            **({'cleanup_seconds': budgets['cleanup_seconds']} if safety_only else {}))
            stream_used += probe['retained_stream_bytes']
            issues = context_audit(probe, Path(probe['stdout']['path']).read_bytes(), prompt, [])
            record['calls'].append({'stage': stage, 'kind': 'local_context_probe', 'process': probe})
            if issues:
                record.update(status='context_blocked', exposure_issues=issues)
                break
            response_path = stage_root / 'original-response.json'
            remaining = stage_deadline - time.monotonic()
            if remaining <= 0 or stream_used >= budgets['stream_bytes']:
                record['status'] = 'budget_exhausted'
                break
            process = capture([executable, *settings, 'exec', '--skip-git-repo-check', '--ignore-rules',
                               '--json', '--color', 'never', '--output-schema', str(HERE / 'response-schema.json'),
                               '-o', str(response_path), '-'], cwd=workspace, env=env,
                              output=stage_root / 'process', timeout=remaining,
                              stream_bytes=budgets['stream_bytes'] - stream_used,
                              cleanup_seconds=budgets['cleanup_seconds'], stdin=prompt.encode(),
                              **({'response_file': response_path, 'response_bytes': allocation} if policy else {}))
            stream_used += process['retained_stream_bytes']
            record['calls'].append({'stage': stage, 'kind': 'model_call', 'process': process})
            observed, invalid = events(process['stdout']['path'])
            tool_calls = [event['item'] for event in observed if event.get('type') == 'item.completed'
                          and event.get('item', {}).get('type', '').endswith(('tool_call', 'command_execution'))]
            (stage_root / 'host-tool-calls.json').write_bytes(encoded(tool_calls))
            all_tool_calls.extend(tool_calls)
            retrievals = [json.loads(line) for line in trace.read_bytes().splitlines()]
            record['evidence_reads'] = retrieval_audit(spec, record['lane'], all_tool_calls, retrievals)
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
            record.setdefault('runtime_observations', []).extend(
                {'stage': stage, 'kind': row['type'], 'identity_status': 'runtime_observed_not_attested',
                 'values': {key: row.get('payload', {}).get(key) for key in
                            ('id', 'source', 'originator', 'cli_version', 'model_provider', 'model', 'effort', 'cwd')}}
                for row in observed_context if row.get('type') in {'session_meta', 'turn_context'})
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
            if output_used > budgets['output_bytes'] or response_path.stat().st_size > allocation:
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
                                    else 'evidence_unverified' if record['evidence_reads']['issues']
                                    else 'captured' if record['evidence_reads']['verified_source_reads']
                                    else 'captured_without_evidence_reads')
        record['workspace_cleanup'] = 'pending'
    record['workspace_cleanup'] = 'complete'
    record['retrievals'] = binding(trace)
    record['mcp_protocol'] = binding(protocol_trace)
    protocol, invalid_protocol = events(protocol_trace)
    record['observed_tool_sets'] = [row['response']['result']['tools'] for row in protocol
                                   if isinstance(row.get('request'), dict) and row['request'].get('method') == 'tools/list'
                                   and row.get('response', {}).get('result', {}).get('tools') is not None]
    record['host_tool_set_completeness'] = 'unknown; MCP tools/list observed, built-in tool definitions not exported'
    record['mcp_protocol_incomplete'] = invalid_protocol
    record['output_bytes'] = output_used
    record['retained_stream_bytes'] = stream_used
    record['execution_elapsed_seconds'] = time.monotonic() - started
    if safety_only:
        safety_outcome(record, retrievals if 'retrievals' in locals() else [], deadline)
    record['clean_comparison'] = False  # Requires independent observed-context review.


def safety_outcome(record, retrievals, deadline):
    """Ceiling contact censors a diagnostic even if the host later exits zero."""
    budgets = record['budgets']
    ceilings = set()
    causes = {'timeout': 'watchdog', 'stream_budget': 'stream', 'response_budget': 'response'}
    for call in record['calls']:
        process = call['process']
        if process['stop_cause'] in causes:
            ceilings.add(causes[process['stop_cause']])
    reads = len(retrievals)
    returned = sum(row['charged_bytes'] for row in retrievals)
    ceilings.update(row['safety_ceiling'] for row in retrievals if row.get('safety_ceiling'))
    for actual, limit, name in ((reads, budgets['reads'], 'read'), (returned, budgets['read_bytes'], 'byte'),
                                 (record['output_bytes'], budgets['output_bytes'], 'response'),
                                 (record['retained_stream_bytes'], budgets['stream_bytes'], 'stream')):
        if actual >= limit:
            ceilings.add(name)
    if time.monotonic() >= deadline:
        ceilings.add('watchdog')
    record['resource_accounting'] = {'evidence_calls': reads, 'returned_evidence_bytes': returned,
        'response_bytes': record['output_bytes'], 'retained_stream_bytes': record['retained_stream_bytes'],
        'model_calls': sum(call['kind'] == 'model_call' for call in record['calls'])}
    record['diagnostic_outcome'] = {'censored': bool(ceilings), 'ceilings': sorted(ceilings),
                                   'state': 'censored' if ceilings else
                                   'ordinary_completion' if record['status'] in {'captured', 'captured_without_evidence_reads'}
                                   else 'model_failure' if any(c['kind'] == 'model_call' and c['process']['outcome'] != 'succeeded'
                                                              for c in record['calls']) else 'not_completed',
                                   'generation_success': 'not_assessed'}
    if ceilings:
        record['status_before_safety_classification'] = record['status']
        record['status'] = 'safety_aborted'
        record['generation_output'] = None


def conditions_directive(approach, conditions=None):
    conditions = conditions or json.loads((HERE / 'conditions.json').read_bytes())
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
    parser.add_argument('--conditions', type=Path, default=HERE / 'conditions.json')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = attempt(args.manifest, args.approach, args.lane, json.loads(args.runtime.read_bytes()), args.output,
                     auth_path=args.auth_path, conditions_path=args.conditions)
    print(json.dumps({'status': result['status'], 'blockers': result['blockers'], 'model_calls': sum(c['kind'] == 'model_call' for c in result['calls']),
                      'output_count': len(result['original_outputs'])}))
