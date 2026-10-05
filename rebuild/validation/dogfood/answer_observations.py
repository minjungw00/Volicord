"""Observation-time shared answers from MCP and supported JSON CLI reads.

Later bundles corroborate immutable records; they do not attest historical Source
freshness or generated-text truth. Contradictions and unresolved basis are separate.
"""
from pathlib import Path
import datetime as dt
import re
import sys

import codex_events as c

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'shared'))
from recorded_action_evidence import recorded_action_errors, transport_omission


def recall_identity_errors(result, project=None, work_id=None):
    """Validate each structured read independently; repetition supplies no oracle."""
    identity = lambda value: isinstance(value, str) and re.fullmatch(r'[0-9a-f]{32}', value) is not None
    if not isinstance(result, dict):
        return ['structured Recall unavailable']
    errors = []
    if not identity(result.get('project_id')) or project is not None and result.get('project_id') != project:
        errors.append('Project identity')
    selected, checkpoint, goals = result.get('selected_work'), result.get('checkpoint'), result.get('goal_basis')
    recalled = checkpoint.get('work_item_id') if isinstance(checkpoint, dict) else None
    if (not identity(recalled) or work_id is not None and recalled != work_id
            or not isinstance(selected, dict) or selected.get('work_item_id') != recalled):
        errors.append('selected Work identity')
    if (not isinstance(checkpoint, dict) or not identity(checkpoint.get('identity'))
            or type(checkpoint.get('revision')) is not int or checkpoint['revision'] < 1):
        errors.append('Checkpoint identity/revision')
    matching = [g for g in goals if isinstance(g, dict) and g.get('role') == 'goal'
        and g.get('identity') == recalled] if isinstance(goals, list) else []
    if len(matching) != 1:
        errors.append('Goal basis identity')
    elif 'source_ids' in matching[0] and (not isinstance(matching[0]['source_ids'], list)
            or not all(identity(value) for value in matching[0]['source_ids'])):
        errors.append('Goal basis Sources malformed')
    if result.get('read_only') is not True:
        errors.append('read-only Recall assertion')
    return errors


def bounded_field(value, field):
    return transport_omission(value.get(field)) or (field not in value
        and transport_omission({'transport_omission': value.get('transport_omission')}))


def returned_recalls(capture):
    if capture is None:
        return []
    values = [{'transport': 'mcp', 'call_id': call.call_id, 'turn_id': call.turn_id,
        'sequence': call.sequence, 'completion_sequence': call.completion_sequence,
        'invocation_sequence': capture.observed_metadata.get('mcp_invocations', {}).get(call.call_id),
        'requested_language': call.arguments.get('requested_language', 'en'), 'result': call.result} for call in capture.successful_calls('recall')]
    for command in capture.commands:
        argvs = c.command_argvs(command.parsed_command)
        if len(argvs) != 1:
            continue
        argv = argvs[0]
        if Path(argv[0]).name != 'volicord' or 'recall' not in argv[1:] or '--json' not in argv[1:]:
            continue
        result = None
        if command.exit_code == 0 and command.evidence_state == 'completed':
            try:
                result = c.strict_json(command.output)
            except (ValueError, UnicodeError):
                pass
        values.append({'transport': 'cli', 'call_id': command.execution_identity,
            'turn_id': command.turn_id, 'sequence': command.sequence,
            'completion_sequence': command.completion_sequence, 'invocation_sequence': command.sequence, 'requested_language': argv[argv.index('--language') + 1] if '--language' in argv and argv.index('--language') + 1 < len(argv) else 'en', 'result': result})
    return sorted(values, key=lambda value: value['sequence'])


def resume_relationship(work, resume, bundle, work_id, returned):
    """A clock alone is insufficient: require completed handoff and resolution."""
    if (not work or not resume or not bundle or work.session_id == resume.session_id
            or work.cwd != resume.cwd or returned['invocation_sequence'] is None):
        return None
    before = work.observed_metadata.get('capture_bounds')
    after = resume.observed_metadata.get('capture_bounds')
    if not before or not after or dt.datetime.fromisoformat(before['last']) >= dt.datetime.fromisoformat(after['first']):
        return None
    if (not work.fresh_user_thread or not resume.fresh_user_thread
            or work.turn_lifecycle.state != 'completed' or not work.turn_lifecycle.turns
            or any(call.completion_sequence > work.turn_lifecycle.turns[-1].end_sequence for call in work.tool_calls)):
        return None
    handoffs = [call for call in work.successful_calls('checkpoint_record')
        if call.arguments.get('project_id') == bundle.project_id
        and call.arguments.get('goal_context_id') == work_id and call.arguments.get('kind') == 'handoff'
        and call.result.get('goal_context_id') == work_id]
    resolutions = [call for call in resume.successful_calls('project_resolve')
        if call.completion_sequence < returned['invocation_sequence']
        and call.arguments.get('repository') == str(resume.cwd)
        and call.result.get('status') == 'found' and call.result.get('project_id') == bundle.project_id
        and isinstance(call.result.get('binding'), dict)
        and call.result['binding'].get('canonical_repository_path') == str(resume.cwd)
        and call.result['binding'].get('availability') == 'available']
    if not handoffs or not resolutions:
        return None
    handoff = max(handoffs, key=lambda call: call.sequence)
    checkpoint = bundle.one('checkpoints', project_id=bundle.project_id, id=handoff.result.get('checkpoint_id'))
    if not checkpoint or checkpoint.get('work_item_id') != work_id or checkpoint.get('revision') != handoff.result.get('revision'):
        return None
    return {'kind': 'completed_handoff_then_fresh_project_resolution',
        'work_capture_sha256': work.source_sha256, 'resume_capture_sha256': resume.source_sha256,
        'work_session': work.session_id, 'resume_session': resume.session_id,
        'handoff_call_id': handoff.call_id, 'handoff_completion_sequence': handoff.completion_sequence,
        'resolve_call_id': resolutions[0].call_id, 'resolve_completion_sequence': resolutions[0].completion_sequence,
        'work_bounds': before, 'resume_bounds': after}


def goal_events(capture):
    events = []
    if not capture:
        return events
    for call in capture.tool_calls:
        if call.operation == 'context_record' and call.arguments.get('role') == 'goal':
            kind = 'context_record'
        elif call.operation == 'canonical_mutate' and call.arguments.get('action') == 'correct_context':
            kind = 'correct_context'
        else:
            continue
        events.append({'kind': kind, 'arguments': call.arguments, 'result': call.result,
            'outcome': ('indeterminate' if call.error in {'malformed_mcp_completion', 'mcp_wrapper_completion_mismatch', 'ambiguous_mcp_wrapper_correlation'} else call.outcome), 'sequence': call.sequence, 'completion_sequence': call.completion_sequence,
            'call_id': call.call_id, 'transport': 'mcp',
            'invocation_sequence': capture.observed_metadata.get('mcp_invocations', {}).get(call.call_id)})
    for command in capture.commands:
        argvs = c.command_argvs(command.parsed_command)
        if len(argvs) != 1 or Path(argvs[0][0]).name != 'volicord':
            continue
        argv = argvs[0]
        if 'advanced' not in argv or argv[argv.index('advanced'):argv.index('advanced') + 3] != ('advanced', 'records', 'correct-context'):
            continue
        index = argv.index('advanced') + 3
        option = lambda name: argv[argv.index(name) + 1] if name in argv and argv.index(name) + 1 < len(argv) else None
        expected = option('--revision')
        args = {'project_id': option('--project'), 'record_id': argv[index] if index < len(argv) else None,
            'expected_revision': int(expected) if expected and expected.isdecimal() else None,
            'corrected_text': option('--text'), 'user_authorization_source_id': option('--source')}
        result = None
        if command.exit_code == 0 and command.evidence_state == 'completed':
            try:
                result = c.strict_json(command.output)
            except (ValueError, UnicodeError):
                pass
        outcome = ('succeeded' if isinstance(result, dict) else 'failed'
            if command.evidence_state == 'completed' and type(command.exit_code) is int and command.exit_code != 0 else 'indeterminate')
        events.append({'kind': 'correct_context', 'arguments': args, 'result': result, 'outcome': outcome,
            'sequence': command.sequence, 'completion_sequence': command.completion_sequence,
            'call_id': command.execution_identity, 'transport': 'cli', 'invocation_sequence': command.sequence})
    return sorted(events, key=lambda event: event['sequence'])


def goal_basis_at(work, resume, capture, returned, project, work_id, relationship):
    """Fold witnessed CAS transitions; answer and latest Goal row are not inputs."""
    revision, sources = None, None
    sources_conflicted = False
    errors, limits, witnesses = [], [], []
    seen = set()
    scopes = [work, capture] if capture is resume and relationship else [capture]
    unordered = [work] if capture is resume and work and not relationship else []
    relevant = lambda e: (e['arguments'].get('project_id') in (None, project) and
        (e['kind'] == 'correct_context' and e['arguments'].get('record_id') == work_id
        or e['kind'] == 'context_record' and (e['result'] or {}).get('context_item_id') == work_id))
    uncertain = False
    for source in unordered:
        if any(relevant(e) and e['outcome'] != 'failed' for e in goal_events(source)):
            limits.append('observation-time Goal basis unavailable: start/resume relationship unproven')
            uncertain = True
    for source in scopes:
        last_completion = -1
        for event in goal_events(source):
            if not relevant(event) or event['outcome'] == 'failed':
                continue
            if source is capture:
                if event['invocation_sequence'] is not None and event['invocation_sequence'] > returned['completion_sequence']:
                    continue  # A later correction cannot invalidate this Recall.
                if returned['invocation_sequence'] is None or event['completion_sequence'] >= returned['invocation_sequence']:
                    limits.append('observation-time Goal basis unavailable: mutation overlaps Recall')
                    uncertain = True
                    continue
            args, receipt = event['arguments'], event['result']
            witness = {k: event[k] for k in ('transport', 'call_id', 'sequence', 'completion_sequence', 'invocation_sequence', 'kind')}
            witness.update(raw_capture_sha256=source.source_sha256, session_id=source.session_id)
            witnesses.append(witness)
            if event['invocation_sequence'] is not None and event['invocation_sequence'] <= last_completion:
                limits.append('observation-time Goal basis unavailable: overlapping mutation receipts')
                uncertain = True
            last_completion = max(last_completion, event['completion_sequence'])
            if event['outcome'] != 'succeeded' or not isinstance(receipt, dict) or args.get('project_id') is None:
                limits.append('observation-time Goal basis unavailable: incomplete mutation receipt/Project')
                uncertain = True
                continue
            actual = receipt.get('revision')
            if event['kind'] == 'context_record':
                if receipt.get('project_id') != project or receipt.get('role') != 'goal':
                    errors.append('Goal creation receipt Project/role disagreement')
                    continue
                source_id = receipt.get('source_id')
                if not isinstance(source_id, str) or re.fullmatch(r'[0-9a-f]{32}', source_id) is None or type(actual) is not int or actual < 1:
                    limits.append('observation-time Goal basis unavailable: creation revision/Source missing')
                    continue
                continued = args.get('work_transition') == 'continue' or receipt.get('canonical_mutation') is False
                key = ('context_record', actual, source_id, continued)
                if sources is not None and sources != [source_id]:
                    limits.append('observation-time Goal basis unavailable: supporting Source receipts conflict')
                    sources = None
                    sources_conflicted = True
                    uncertain = True
                if key not in seen:
                    if continued and revision is not None and actual < revision:
                        limits.append('observation-time Goal basis unavailable: conflicting continuation revision')
                        uncertain = True
                    elif not continued and (actual != 1 or revision is not None):
                        limits.append('observation-time Goal basis unavailable: conflicting creation receipts')
                        uncertain = True
                    else:
                        revision, sources = actual, [source_id]
                    seen.add(key)
            else:
                if (receipt.get('identity') != work_id or receipt.get('record_kind') != 'context_item'
                        or receipt.get('action', receipt.get('operation')) != 'correct_context'):
                    errors.append('Goal correction receipt identity/kind disagreement')
                    continue
                expected = args.get('expected_revision')
                authorization = receipt.get('user_response_source_id', args.get('user_authorization_source_id'))
                if (type(expected) is not int or expected < 1 or type(actual) is not int or actual != expected + 1
                        or not isinstance(args.get('corrected_text'), str) or not args['corrected_text'].strip()
                        or not isinstance(authorization, str) or re.fullmatch(r'[0-9a-f]{32}', authorization) is None):
                    limits.append('observation-time Goal basis unavailable: correction CAS revision/authorization missing or conflicting')
                    uncertain = True
                    continue
                key = ('correct_context', expected, actual, args.get('corrected_text'), authorization)
                witness.update(expected_revision=expected, actual_revision=actual,
                    authorization_source_id=authorization, supporting_sources_changed=False)
                if key in seen:
                    witness['transition'] = 'replayed_receipt_no_advance'
                    continue
                if receipt.get('replayed') is True:
                    limits.append('observation-time Goal basis unavailable: replay without original receipt')
                    uncertain = True
                    continue
                if revision != expected:
                    limits.append('observation-time Goal basis unavailable: missing/conflicting predecessor revision')
                    uncertain = True
                revision = actual  # Returned CAS revision, never a guessed increment.
                seen.add(key)
                witness['transition'] = 'successful_correction'
    if sources is None:
        limits.append('observation-time Goal supporting Sources unavailable')
    if revision is None:
        limits.append('observation-time Goal revision unavailable')
    return {'revision': None if uncertain else revision, 'sources': None if sources_conflicted else sources,
        'errors': errors, 'limits': limits, 'witnesses': witnesses, 'relationship': relationship,
        'recall_window': {k: returned[k] for k in ('invocation_sequence', 'completion_sequence')}}


def observe(work, resume, bundle, work_id):
    observations = []
    for capture in (work, resume):
        for returned in returned_recalls(capture):
            errors, limits = [], []
            result = returned['result']
            # Only previously observed authoring is a temporal witness. No latest
            # bundle selection and no use of returned identity to choose an oracle.
            temporal_goals = [call for call in capture.successful_calls('context_record')
                if call.arguments.get('role') == 'goal' and call.completion_sequence < (returned['invocation_sequence'] if returned['invocation_sequence'] is not None else returned['sequence'])]
            expected_work = (work_id if capture is resume else
                temporal_goals[-1].result.get('context_item_id') if temporal_goals else None)
            relationship = resume_relationship(work, resume, bundle, work_id, returned) if capture is resume else None
            prior = [call for source in (work, resume) if source
                for call in source.successful_calls('checkpoint_record')
                if (source is work and capture is resume and relationship or source is capture
                    and call.completion_sequence < (returned['invocation_sequence'] if returned['invocation_sequence'] is not None else returned['sequence']))
                and call.arguments.get('project_id') == (bundle.project_id if bundle else None)
                and call.arguments.get('goal_context_id') == expected_work]
            latest = prior[-1] if prior else None
            if latest is None and work:
                # Corroborate the exact asserted immutable Checkpoint, without
                # asserting it was latest or deriving any Goal revision from it.
                known = [call for call in work.successful_calls('checkpoint_record')
                    if call.arguments.get('project_id') == (bundle.project_id if bundle else None)
                    and call.arguments.get('goal_context_id') == expected_work
                    and isinstance(result, dict) and isinstance(result.get('checkpoint'), dict)
                    and result['checkpoint'].get('identity') == call.result.get('checkpoint_id')]
                latest = known[-1] if known else None
                limits.append('observation-time Checkpoint ordering unavailable')
            goal = goal_basis_at(work, resume, capture, returned, bundle.project_id if bundle else None, expected_work, relationship)
            checkpoint = bundle.one('checkpoints', project_id=bundle.project_id,
                id=latest.result.get('checkpoint_id')) if bundle and latest else None
            sources = None
            checkpoint_id = latest.result.get('checkpoint_id') if latest else None
            checkpoint_revision = latest.result.get('revision') if latest else None
            if not isinstance(result, dict):
                limits.append('returned_json_unresolvable')
            else:
                errors.extend(recall_identity_errors(result, bundle.project_id if bundle else None, expected_work))
                errors.extend(goal['errors'])
                limits.extend(goal['limits'])
                asserted = result.get('checkpoint')
                immutable = bundle.one('checkpoints', project_id=bundle.project_id,
                    id=asserted.get('identity')) if bundle and isinstance(asserted, dict) else None
                if immutable and (asserted.get('revision') != immutable.get('revision')
                        or asserted.get('work_item_id') != immutable.get('work_item_id')):
                    errors.append('asserted immutable Checkpoint identity/revision')
                if immutable:
                    asserted_sources = [row['source_id'] for row in sorted(bundle.rows('checkpoint_source_relations'),
                        key=lambda row: row.get('position', -1)) if row.get('project_id') == bundle.project_id
                        and row.get('checkpoint_id') == immutable['id'] and row.get('relation_kind') == 'supported_by']
                    errors.extend(recorded_action_errors({'project_id': bundle.project_id,
                        'goal_id': immutable.get('work_item_id'), 'checkpoint_id': immutable['id'],
                        'checkpoint_revision': immutable['revision'], 'checkpoint_sources': asserted_sources,
                        'next_step': immutable.get('next_step')}, result))
                goals = result.get('goal_basis')
                matching = [g for g in goals if isinstance(g, dict) and g.get('role') == 'goal'
                    and g.get('identity') == expected_work] if isinstance(goals, list) else []
                if len(matching) == 1 and goal['sources'] is not None and 'source_ids' in matching[0] and matching[0]['source_ids'] != goal['sources']:
                    errors.append('observation-time Goal supporting Sources')
                if bundle and result.get('project_id') != bundle.project_id:
                    errors.append('Project identity')
                selected = result.get('selected_work')
                if expected_work is None:
                    limits.append('observation-time Work basis unavailable')
                elif not isinstance(selected, dict) or selected.get('work_item_id') != expected_work:
                    errors.append('selected Work identity')
                if latest and checkpoint and checkpoint.get('revision') == latest.result.get('revision'):
                    sources = [row['source_id'] for row in sorted(bundle.rows('checkpoint_source_relations'),
                        key=lambda row: row.get('position', -1)) if row.get('project_id') == bundle.project_id
                        and row.get('checkpoint_id') == checkpoint['id'] and row.get('relation_kind') == 'supported_by']
                    expected = {'project_id': bundle.project_id, 'goal_id': expected_work,
                        'checkpoint_id': checkpoint['id'], 'checkpoint_revision': checkpoint['revision'],
                        'checkpoint_sources': sources, 'next_step': latest.arguments.get('next_step')}
                    if checkpoint.get('work_item_id') != expected_work or checkpoint.get('next_step') != expected['next_step']:
                        errors.append('authoring and immutable Checkpoint disagree')
                    errors.extend(recorded_action_errors(expected, result))
                    cp = result.get('checkpoint')
                    if not isinstance(cp, dict) or any(cp.get(key) != value for key, value in (
                            ('identity', checkpoint['id']), ('revision', checkpoint['revision']), ('work_item_id', expected_work))):
                        errors.append('observation-time Checkpoint identity/revision')
                else:
                    limits.append('observation-time Checkpoint basis unavailable')
                    if latest:
                        expected = {'project_id': bundle.project_id, 'goal_id': expected_work,
                            'checkpoint_id': latest.result.get('checkpoint_id'), 'checkpoint_revision': latest.result.get('revision'),
                            'next_step': latest.arguments.get('next_step')}
                        errors.extend(recorded_action_errors(expected, result))
                answers = selected.get('answers') if isinstance(selected, dict) else None
                if isinstance(answers, dict):
                    state, provenance = answers.get('explanation_state'), answers.get('provenance')
                    prose = answers.get('prose', [])
                    if not isinstance(prose, list):
                        if bounded_field(answers, 'prose'):
                            limits.append('generated prose unavailable through scoped transport omission')
                        else:
                            errors.append('shared prose section malformed')
                        prose = []
                    if state not in {'current', 'unavailable', 'stale', 'corrupt', 'unsupported'}:
                        errors.append('explanation availability state')
                    if state == 'current':
                        subject = provenance.get('subject') if isinstance(provenance, dict) else None
                        identity = subject.get('identity') if isinstance(subject, dict) else None
                        expected_identity = expected_work or work_id
                        if (not isinstance(provenance, dict)
                                or not isinstance(provenance.get('project_id'), str)
                                or re.fullmatch(r'[0-9a-f]{32}', provenance['project_id']) is None
                                or bundle and provenance['project_id'] != bundle.project_id
                                or not isinstance(subject, dict) or subject.get('kind') != 'work'
                                or not isinstance(identity, list) or len(identity) != 16
                                or not all(type(value) is int and 0 <= value <= 255 for value in identity)
                                or expected_identity is not None and identity != list(bytes.fromhex(expected_identity))):
                            errors.append('generated Work/Project scope')
                        else:
                            for name, expected_value in (('language', returned['requested_language']),
                                    ('generator_identity_status', 'self_reported_not_independently_verified')):
                                if provenance.get(name) != expected_value:
                                    if bounded_field(provenance, name):
                                        limits.append(f'generated {name} unavailable through scoped transport omission')
                                    else:
                                        errors.append('generated language/provenance assertion')
                            evidence = provenance.get('evidence')
                            bounded_evidence = transport_omission(evidence) or (isinstance(evidence, list)
                                and any(transport_omission(e) for e in evidence)) or bounded_field(provenance, 'evidence')
                            errors.extend(goal['errors'])
                            limits.extend(goal['limits'])
                            for key, identity, revision, field in (
                                    ('next_step', checkpoint_id, checkpoint_revision, 'next_step'),
                                    ('goal', expected_work or work_id, goal['revision'], 'statement')):
                                matches = [e for e in evidence if isinstance(e, dict) and e.get('key') == key] if isinstance(evidence, list) else []
                                if not matches and bounded_evidence:
                                    limits.append(f'generated {key} basis unavailable through scoped transport omission')
                                elif (len(matches) != 1 or type(matches[0].get('revision')) is not int or matches[0].get('revision', 0) < 1
                                        or not isinstance(matches[0].get('sources'), list)
                                        or not all(isinstance(v, str) and re.fullmatch(r'[0-9a-f]{32}', v) for v in matches[0]['sources']) or matches[0].get('field') != field or not isinstance(matches[0].get('identity'), str)
                                        or re.fullmatch(r'[0-9a-f]{32}', matches[0]['identity']) is None
                                        or identity is not None and matches[0].get('identity') != identity
                                        or revision is not None and matches[0].get('revision') != revision):
                                    errors.append(f'generated {key} revision basis')
                                elif (sources if key == 'next_step' else goal['sources']) is not None and matches[0].get('sources') != (sources if key == 'next_step' else goal['sources']):
                                    errors.append(f'generated {key} Source basis')
                            evidence = provenance.get('evidence')
                            if (not isinstance(evidence, list) and not bounded_evidence or isinstance(evidence, list)
                                    and any((not isinstance(e, dict) or not isinstance(e.get('key'), str))
                                        and not transport_omission(e) for e in evidence)):
                                errors.append('generated evidence keys malformed')
                            keys = {e['key'] for e in evidence if isinstance(e, dict) and isinstance(e.get('key'), str)} if isinstance(evidence, list) else set()
                            for paragraph in prose if isinstance(prose, list) else []:
                                if transport_omission(paragraph):
                                    limits.append('generated paragraph unavailable through scoped transport omission')
                                    continue
                                if not isinstance(paragraph, dict) or paragraph.get('role') != 'generated_interpretation':
                                    errors.append('generated paragraph role/grounding')
                                    continue
                                if bounded_field(paragraph, 'text'):
                                    limits.append('generated paragraph text unavailable through scoped transport omission')
                                elif not isinstance(paragraph.get('text'), str) or not paragraph['text'].strip():
                                    errors.append('generated paragraph role/grounding')
                                cited = paragraph.get('evidence_keys')
                                if bounded_field(paragraph, 'evidence_keys'):
                                    limits.append('generated paragraph grounding unavailable through scoped transport omission')
                                elif not isinstance(cited, list):
                                    errors.append('generated paragraph role/grounding')
                                else:
                                    for cited_key in cited:
                                        if transport_omission(cited_key) or isinstance(cited_key, str) and cited_key not in keys and bounded_evidence:
                                            limits.append('generated paragraph grounding unavailable through scoped transport omission')
                                        elif not isinstance(cited_key, str) or cited_key not in keys:
                                            errors.append('generated paragraph role/grounding')
                        limits.append('generated prose adequacy requires qualitative review')
                    elif provenance is not None or any(isinstance(p, dict) and p.get('question') == 'NextStep' for p in prose if isinstance(prose, list)):
                        errors.append('unusable generated direction revived')
            status = ('confirmed_violation' if errors else 'indeterminate'
                if any('unavailable' in limit or 'unresolvable' in limit for limit in limits) else 'confirmed_pass')
            observations.append({k: v for k, v in returned.items() if k != 'result'} | {
                'raw_capture_sha256': capture.source_sha256, 'status': status,
                'errors': sorted(set(errors)), 'limits': sorted(set(limits)),
                'basis_kind': 'observation_time_authoring_and_immutable_checkpoint',
                'goal_basis': goal,
                'source_status_basis': 'returned_observation_only_not_later_bundle'})
    status = ('confirmed_violation' if any(o['status'] == 'confirmed_violation' for o in observations)
        else 'indeterminate' if any(o['status'] == 'indeterminate' for o in observations)
        else 'confirmed_pass' if observations else 'not_observed')
    return {'status': status, 'basis': {'observations': observations,
        'identity_basis': 'Project-scoped canonical Work and prior record receipts',
        'generated_semantics': 'qualitative_review_required'}}
