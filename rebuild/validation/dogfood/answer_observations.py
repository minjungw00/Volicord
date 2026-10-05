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
from recorded_action_evidence import field_omission, recorded_action_errors, transport_omission
from answer_projection import PROVENANCE, ANSWERS, CLAIM, EVIDENCE


def omitted_goal(result, work_id):
    """An exact canonical section-bound report, never generic omission authority."""
    reports = result.get('omissions') if isinstance(result, dict) else None
    if not isinstance(reports, list) or work_id is None:
        return None
    return next((r for r in reports if isinstance(r, dict) and r == {
        'identity': work_id, 'kind': 'context_goal', 'reason': 'bound',
        'expandable_basis': 'expand context_goal by identity'}), None)


def recall_identity_errors(result, project=None, work_id=None, *, work_state='required', checkpoint_state='required'):
    """Present contradictions are independent of lifecycle/temporal limitations.

    Defaults retain the strict campaign resume identity consumer. The observer
    supplies independently resolved required/absent/unknown states for each read.
    """
    identity = lambda value: isinstance(value, str) and re.fullmatch(r'[0-9a-f]{32}', value) is not None
    if not isinstance(result, dict):
        return ['structured Recall unavailable']
    errors = []
    if not identity(result.get('project_id')) or project is not None and result.get('project_id') != project:
        errors.append('Project identity')
    selected, checkpoint, goals = result.get('selected_work'), result.get('checkpoint'), result.get('goal_basis')
    selected_id = selected.get('work_item_id') if isinstance(selected, dict) else None
    if selected is None:
        if work_state == 'required' or 'selected_work' not in result:
            errors.append('selected Work identity')
    elif (work_state == 'absent' or not identity(selected_id)
            or work_id is not None and selected_id != work_id):
        errors.append('selected Work identity')
    if checkpoint is None:
        if checkpoint_state == 'required' or 'checkpoint' not in result:
            errors.append('Checkpoint identity/revision')
    elif (checkpoint_state == 'absent' or not isinstance(checkpoint, dict)
            or not identity(checkpoint.get('identity')) or not identity(checkpoint.get('work_item_id'))
            or checkpoint.get('work_item_id') != selected_id
            or type(checkpoint.get('revision')) is not int or checkpoint['revision'] < 1):
        errors.append('Checkpoint identity/revision')
    if not isinstance(goals, list):
        errors.append('Goal basis identity')
    else:
        for goal in goals:
            if (not isinstance(goal, dict) or goal.get('role') != 'goal' or not identity(goal.get('identity'))):
                errors.append('Goal basis identity')
            elif 'source_ids' in goal and (not isinstance(goal['source_ids'], list)
                    or not all(identity(value) for value in goal['source_ids'])):
                errors.append('Goal basis Sources malformed')
        matching = [g for g in goals if isinstance(g, dict) and g.get('identity') == (work_id or selected_id)]
        if ((work_state == 'absent' and goals) or (selected is None and goals)
                or (work_state == 'required' or selected is not None) and len(matching) != 1
                and not (not matching and omitted_goal(result, work_id or selected_id))):
            errors.append('Goal basis identity')
    if result.get('read_only') is not True:
        errors.append('read-only Recall assertion')
    return errors


def bounded_field(value, field, contract):
    return field_omission(value, field, fields=contract,
        optional={'recorded_action'} if contract is CLAIM else ())


def suffix_omission(values):
    return (isinstance(values, list) and bool(values) and transport_omission(values[-1])
        and 'omitted_count' in values[-1]['transport_omission'])


def returned_recalls(capture):
    if capture is None:
        return []
    values = [{'transport': 'mcp', 'call_id': call.call_id, 'turn_id': call.turn_id,
        'sequence': call.sequence, 'completion_sequence': call.completion_sequence,
        'invocation_sequence': capture.observed_metadata.get('mcp_invocations', {}).get(call.call_id),
        'requested_language': call.arguments.get('requested_language', 'en'),
        'requested_project': call.arguments.get('project_id'), 'result': call.result} for call in capture.successful_calls('recall')]
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
            'completion_sequence': command.completion_sequence, 'invocation_sequence': command.sequence,
            'requested_project': argv[argv.index('--project') + 1] if '--project' in argv and argv.index('--project') + 1 < len(argv) else None, 'requested_language': argv[argv.index('--language') + 1] if '--language' in argv and argv.index('--language') + 1 < len(argv) else 'en', 'result': result})
    return sorted(values, key=lambda value: value['sequence'])


def resume_relationship(work, resume, bundle, work_id, returned, *, handoff_only=True):
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
        and call.arguments.get('goal_context_id') == work_id
        and (not handoff_only or call.arguments.get('kind') == 'handoff')
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
    return {'kind': ('completed_handoff_then_fresh_project_resolution' if handoff_only
        else 'completed_work_then_fresh_project_resolution'),
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


def lifecycle_at(work, resume, capture, returned, bundle, work_id, relationship):
    """Resolve LatestWork from ordered observation-time receipts, checkpoint first.

    Receipt order establishes publication order, never final-export chronology.
    Goal fallback needs a witnessed empty Project and no possible Checkpoint.
    Resume identity is an independent constraint, not a selector substitute.
    Returns the serializable basis and the independently selected receipt.
    """
    project = bundle.project_id if bundle else returned.get('requested_project')
    boundary = returned['invocation_sequence']
    existence = (resume_relationship(work, resume, bundle, work_id, returned, handoff_only=False)
        if capture is resume else None)
    scopes = [work, capture] if capture is resume and existence else [capture]
    empty, latest_goal, latest, expected = False, None, None, None
    checkpoint_unknown, goal_unknown, checkpoint_exists = False, False, False
    witnesses, errors, limits = [], [], []
    seen_checkpoints = {}
    identity = lambda value: isinstance(value, str) and re.fullmatch(r'[0-9a-f]{32}', value) is not None
    for source in scopes:
        last_checkpoint_completion, last_goal_completion = -1, -1
        for call in sorted(source.tool_calls, key=lambda call: call.completion_sequence):
            outcome = ('indeterminate' if call.error in {'malformed_mcp_completion',
                'mcp_wrapper_completion_mismatch', 'ambiguous_mcp_wrapper_correlation'} else call.outcome)
            if (call.operation not in {'project_initialize', 'context_record', 'checkpoint_record', 'canonical_mutate'}
                    or call.arguments.get('project_id', call.result.get('project_id')) not in (None, project)
                    or outcome == 'failed'):
                continue
            invocation = source.observed_metadata.get('mcp_invocations', {}).get(call.call_id)
            if source is capture and invocation is not None and invocation > returned['completion_sequence']:
                continue  # Later state cannot decide an earlier Recall.
            is_goal = call.operation == 'context_record' and call.arguments.get('role') == 'goal'
            is_checkpoint = call.operation == 'checkpoint_record'
            # In-place Goal correction preserves record creation order. Its CAS/
            # supporting Sources are checked separately by goal_basis_at.
            other_mutation = call.operation == 'canonical_mutate' and not (
                call.arguments.get('action') == 'correct_context')
            if not (is_goal or is_checkpoint or other_mutation or call.operation == 'project_initialize'):
                continue
            witness = {'operation': call.operation, 'call_id': call.call_id,
                'sequence': call.sequence, 'invocation_sequence': invocation,
                'completion_sequence': call.completion_sequence,
                'raw_capture_sha256': source.source_sha256, 'session_id': source.session_id}
            witnesses.append(witness)
            ordered = (invocation is not None and invocation < call.completion_sequence
                and (source is not capture or boundary is not None and call.completion_sequence < boundary))
            if not ordered or outcome != 'succeeded' or other_mutation:
                checkpoint_unknown |= is_checkpoint or other_mutation
                goal_unknown |= is_goal or other_mutation
                if call.operation == 'project_initialize':
                    empty = False
                continue
            if call.operation == 'project_initialize':
                if call.result.get('project_id') == project and not latest_goal and not latest:
                    empty = True
            elif is_goal:
                receipt = call.result
                if receipt.get('project_id') != project or receipt.get('role') != 'goal':
                    errors.append('Goal creation receipt Project/role disagreement')
                    goal_unknown = True
                elif not identity(receipt.get('context_item_id')):
                    goal_unknown = True
                elif type(receipt.get('revision')) is not int or receipt['revision'] < 1:
                    goal_unknown = True
                elif (call.arguments.get('work_transition') != 'continue'
                        and receipt.get('canonical_mutation') is not False):
                    goal_unknown |= invocation <= last_goal_completion or receipt['revision'] != 1
                    latest_goal = receipt['context_item_id']
                elif latest_goal is None:
                    goal_unknown = True  # Continuation cannot establish an empty-state creation.
                last_goal_completion = max(last_goal_completion, call.completion_sequence)
            else:
                receipt = call.result
                associated = call.arguments.get('goal_context_id')
                if receipt.get('goal_context_id') != associated:
                    errors.append('Checkpoint receipt Work disagreement')
                    checkpoint_unknown = True
                if (not identity(receipt.get('checkpoint_id')) or not identity(associated)
                        or type(receipt.get('revision')) is not int or receipt['revision'] < 1):
                    checkpoint_unknown = True
                key = receipt.get('checkpoint_id') if isinstance(receipt.get('checkpoint_id'), str) else None
                publication = (associated, receipt.get('revision'), call.arguments.get('next_step'))
                if key in seen_checkpoints:
                    if seen_checkpoints[key] != publication:
                        errors.append('Checkpoint receipt identity/revision disagreement')
                        checkpoint_unknown = True
                    continue  # Same immutable receipt cannot become a new publication.
                seen_checkpoints[key] = publication
                # A replayed publication without its original receipt has unknown order.
                checkpoint_exists |= (identity(receipt.get('checkpoint_id'))
                    and identity(associated) and type(receipt.get('revision')) is int and receipt['revision'] > 0)
                if receipt.get('replayed') is True:
                    checkpoint_unknown = True
                checkpoint_unknown |= invocation <= last_checkpoint_completion
                last_checkpoint_completion = max(last_checkpoint_completion, call.completion_sequence)
                latest = call
    if boundary is None:
        checkpoint_unknown = goal_unknown = True
    if latest and not checkpoint_unknown:
        expected = latest.arguments['goal_context_id']
        selector_basis = 'latest_checkpoint'
    elif empty and not checkpoint_unknown and not goal_unknown:
        expected = latest_goal
        selector_basis = 'latest_goal' if latest_goal else 'empty_project'
    else:
        selector_basis = 'unknown'
        limits.append('observation-time LatestWork selector basis unavailable')
    if selector_basis == 'unknown':
        latest = None
    work_state = 'required' if expected is not None else 'absent' if selector_basis == 'empty_project' else 'unknown'
    checkpoint_state = ('required' if latest or checkpoint_exists or existence else 'absent' if selector_basis in {'empty_project', 'latest_goal'} else 'unknown')
    return {'work': work_state, 'checkpoint': checkpoint_state, 'expected_work': expected,
        'selector_basis': selector_basis, 'errors': errors, 'limits': limits,
        'resume_expected_work': work_id if capture is resume else None,
        'resume_checkpoint_required': bool(existence),
        'call_role': 'resume' if capture is resume else 'start',
        'witnesses': witnesses, 'existence_relationship': existence,
        'revision_relationship': relationship}, latest


def observe(work, resume, bundle, work_id):
    observations = []
    for capture in (work, resume):
        for returned in returned_recalls(capture):
            errors, limits, omissions = [], [], []
            def omitted(value, field, contract, scope):
                report = bounded_field(value, field, contract)
                if report:
                    evidence = {"scope": scope, "field": field, **report}
                    if evidence not in omissions:
                        omissions.append(evidence)
                return report
            result = returned['result']
            project = bundle.project_id if bundle else returned.get('requested_project')
            # Only previously observed authoring is a temporal witness. No latest
            # bundle selection and no use of returned identity to choose an oracle.
            relationship = resume_relationship(work, resume, bundle, work_id, returned) if capture is resume else None
            lifecycle, latest = lifecycle_at(work, resume, capture, returned, bundle, work_id, relationship)
            expected_work = lifecycle['expected_work']
            limits.extend(lifecycle['limits'])
            errors.extend(lifecycle['errors'])
            if latest is None and work and lifecycle['checkpoint'] != 'absent':
                # Exact asserted records can expose contradictions without proving
                # that they were latest. This never supplies selector expectation.
                known = [call for call in work.successful_calls('checkpoint_record')
                    if call.arguments.get('project_id') == (bundle.project_id if bundle else None)
                    and isinstance(result, dict) and isinstance(result.get('checkpoint'), dict)
                    and result['checkpoint'].get('identity') == call.result.get('checkpoint_id')]
                latest = known[-1] if known else None
                limits.append('observation-time Checkpoint ordering unavailable')
            goal_work = expected_work or lifecycle['resume_expected_work']
            goal = goal_basis_at(work, resume, capture, returned, project, goal_work, relationship)
            checkpoint = bundle.one('checkpoints', project_id=bundle.project_id,
                id=latest.result.get('checkpoint_id')) if bundle and latest else None
            sources = None
            checkpoint_id = latest.result.get('checkpoint_id') if latest else None
            checkpoint_revision = latest.result.get('revision') if latest else None
            if not isinstance(result, dict):
                limits.append('returned_json_unresolvable')
            else:
                errors.extend(recall_identity_errors(result, project, expected_work,
                    work_state=lifecycle['work'], checkpoint_state=lifecycle['checkpoint']))
                if lifecycle['resume_expected_work'] is not None:
                    resume_errors = recall_identity_errors(result, project, lifecycle['resume_expected_work'],
                        work_state='required', checkpoint_state='required' if lifecycle['resume_checkpoint_required'] else 'unknown')
                    errors.extend(resume_errors)
                    if 'selected Work identity' in resume_errors:
                        errors.append('resume Work identity')
                errors.extend(goal['errors'])
                if lifecycle['work'] != 'absent':
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
                    and g.get('identity') == goal_work] if isinstance(goals, list) else []
                if len(matching) == 1 and goal['sources'] is not None and 'source_ids' in matching[0] and matching[0]['source_ids'] != goal['sources']:
                    errors.append('observation-time Goal supporting Sources')
                if not matching and omitted_goal(result, goal_work):
                    # The global Goal list is bounded independently of selected
                    # Work. Its exact omission cannot erase visible contradictions
                    # or replace missing observation-time authoring evidence.
                    selected_goal = result.get('selected_work')
                    evidence = selected_goal.get('evidence') if isinstance(selected_goal, dict) else None
                    reading = evidence.get('goal') if isinstance(evidence, dict) else None
                    basis = reading.get('basis') if isinstance(reading, dict) else None
                    if not isinstance(basis, dict):
                        limits.append('selected Goal basis unavailable after exact Goal-list bound omission')
                    else:
                        for field, expected in (('record_kind', 'context_item'), ('identity', goal_work),
                                ('field', 'statement'), ('revision', goal['revision']), ('source_ids', goal['sources'])):
                            if expected is None or field not in basis:
                                limits.append('selected Goal basis unavailable after exact Goal-list bound omission')
                            elif basis[field] != expected or field == 'revision' and type(basis[field]) is not int:
                                errors.append('selected Goal observation-time basis')
                        limits.append('supported omission: exact Goal identity omitted from bounded global list')
                if bundle and result.get('project_id') != bundle.project_id:
                    errors.append('Project identity')
                selected = result.get('selected_work')
                if lifecycle['work'] == 'unknown':
                    limits.append('observation-time Work basis unavailable')
                elif lifecycle['work'] == 'absent':
                    limits.append('supported absence: Project has no Work')
                elif not isinstance(selected, dict) or selected.get('work_item_id') != expected_work:
                    errors.append('selected Work identity')
                if latest and checkpoint and checkpoint.get('revision') == latest.result.get('revision'):
                    sources = [row['source_id'] for row in sorted(bundle.rows('checkpoint_source_relations'),
                        key=lambda row: row.get('position', -1)) if row.get('project_id') == bundle.project_id
                        and row.get('checkpoint_id') == checkpoint['id'] and row.get('relation_kind') == 'supported_by']
                    expected = {'project_id': bundle.project_id, 'goal_id': checkpoint.get('work_item_id'),
                        'checkpoint_id': checkpoint['id'], 'checkpoint_revision': checkpoint['revision'],
                        'checkpoint_sources': sources, 'next_step': latest.arguments.get('next_step')}
                    if (checkpoint.get('work_item_id') != latest.arguments.get('goal_context_id')
                            or checkpoint.get('next_step') != expected['next_step']):
                        errors.append('authoring and immutable Checkpoint disagree')
                    errors.extend(recorded_action_errors(expected, result))
                    cp = result.get('checkpoint')
                    if not isinstance(cp, dict) or any(cp.get(key) != value for key, value in (
                            ('identity', checkpoint['id']), ('revision', checkpoint['revision']), ('work_item_id', checkpoint.get('work_item_id')))):
                        errors.append('observation-time Checkpoint identity/revision')
                else:
                    if lifecycle['checkpoint'] == 'absent':
                        limits.append('supported absence: no meaningful Checkpoint')
                        if expected_work is not None:
                            errors.extend(recorded_action_errors({'project_id': bundle.project_id,
                                'goal_id': expected_work, 'checkpoint_id': None, 'next_step': None}, result))
                        elif result.get('next_step') is not None:
                            errors.append('top-level recorded next action')
                    else:
                        limits.append('observation-time Checkpoint basis unavailable')
                    if latest:
                        expected = {'project_id': bundle.project_id, 'goal_id': latest.arguments.get('goal_context_id'),
                            'checkpoint_id': latest.result.get('checkpoint_id'), 'checkpoint_revision': latest.result.get('revision'),
                            'next_step': latest.arguments.get('next_step')}
                        errors.extend(recorded_action_errors(expected, result))
                        cp = result.get('checkpoint')
                        if not isinstance(cp, dict) or any(cp.get(key) != value for key, value in (
                                ('identity', latest.result.get('checkpoint_id')), ('revision', latest.result.get('revision')),
                                ('work_item_id', latest.arguments.get('goal_context_id')))):
                            errors.append('observation-time Checkpoint identity/revision')
                answers = selected.get('answers') if isinstance(selected, dict) else None
                if isinstance(answers, dict):
                    state, provenance = answers.get('explanation_state'), answers.get('provenance')
                    prose = answers.get('prose')
                    if not isinstance(prose, list):
                        if omitted(answers, 'prose', ANSWERS, '/selected_work/answers'):
                            limits.append('generated prose unavailable through scoped transport omission')
                        else:
                            errors.append('shared prose section malformed')
                        prose = []
                    if state not in {'current', 'unavailable', 'stale', 'corrupt', 'unsupported'}:
                        errors.append('explanation availability state')
                    if state == 'current':
                        scope = '/selected_work/answers/provenance'
                        generated_work = expected_work or selected.get('work_item_id')
                        provenance_omitted = omitted(answers, 'provenance', ANSWERS, '/selected_work/answers')
                        if provenance_omitted:
                            limits.append('generated provenance unavailable through scoped transport omission')
                            provenance = {}
                        elif not isinstance(provenance, dict):
                            errors.append('generated Work/Project scope')
                            provenance = {}
                        project_value = provenance.get('project_id')
                        if provenance_omitted or omitted(provenance, 'project_id', PROVENANCE, scope):
                            limits.append('generated Project scope unavailable through scoped transport omission')
                        elif (not isinstance(project_value, str) or re.fullmatch(r'[0-9a-f]{32}', project_value) is None
                                or project is not None and project_value != project):
                            errors.append('generated Work/Project scope')
                        subject = provenance.get('subject')
                        identity = subject.get('identity') if isinstance(subject, dict) else None
                        if provenance_omitted or omitted(provenance, 'subject', PROVENANCE, scope):
                            limits.append('generated subject scope unavailable through scoped transport omission')
                        elif (not isinstance(subject, dict) or subject.get('kind') != 'work'
                                or not isinstance(identity, list) or len(identity) != 16
                                or not all(type(value) is int and 0 <= value <= 255 for value in identity)
                                or generated_work is not None and (not isinstance(generated_work, str)
                                    or re.fullmatch(r'[0-9a-f]{32}', generated_work) is None
                                    or identity != list(bytes.fromhex(generated_work)))):
                            errors.append('generated Work/Project scope')
                        for name, expected_value in (('language', returned['requested_language']),
                                ('generator_identity_status', 'self_reported_not_independently_verified')):
                            if provenance.get(name) != expected_value:
                                if provenance_omitted or omitted(provenance, name, PROVENANCE, scope):
                                    limits.append(f'generated {name} unavailable through scoped transport omission')
                                else:
                                    errors.append('generated language/provenance assertion')
                        evidence = provenance.get('evidence')
                        bounded_evidence = provenance_omitted or omitted(provenance, 'evidence', PROVENANCE, scope)
                        if suffix_omission(evidence):
                            bounded_evidence = True
                            limits.append('generated evidence suffix unavailable through scoped transport omission')
                            omissions.append({'scope': scope + '/evidence', 'placement': 'stable_suffix',
                                'marker': evidence[-1]['transport_omission']})
                        for index, item in enumerate(evidence if isinstance(evidence, list) else []):
                            if transport_omission(item) and 'exact_json_bytes' in item['transport_omission']:
                                bounded_evidence = True
                                limits.append('generated evidence item unavailable through scoped transport omission')
                                omissions.append({'scope': scope + '/evidence/' + str(index), 'placement': 'whole_item',
                                    'marker': item['transport_omission']})
                            if omitted(item, 'key', EVIDENCE, scope + '/evidence/' + str(index)):
                                bounded_evidence = True
                        errors.extend(goal['errors'])
                        limits.extend(goal['limits'])
                        for key, identity, revision, field in (
                                ('next_step', checkpoint_id, checkpoint_revision, 'next_step'),
                                ('goal', generated_work, goal['revision'], 'statement')):
                            matches = [e for e in evidence if isinstance(e, dict) and e.get('key') == key] if isinstance(evidence, list) else []
                            if not matches and bounded_evidence:
                                limits.append(f'generated {key} basis unavailable through scoped transport omission')
                            elif len(matches) != 1:
                                errors.append(f'generated {key} revision basis')
                            else:
                                item = matches[0]
                                item_scope = scope + '/evidence/' + str(evidence.index(item))
                                expected_sources = sources if key == 'next_step' else goal['sources']
                                for name, expected_value in (('identity', identity), ('revision', revision),
                                        ('field', field), ('sources', expected_sources)):
                                    if omitted(item, name, EVIDENCE, item_scope):
                                        limits.append(f'generated {key} {name} unavailable through scoped transport omission')
                                        continue
                                    value = item.get(name)
                                    if name == 'sources' and suffix_omission(value):
                                        prefix = value[:-1]
                                        if (not all(isinstance(v, str) and re.fullmatch(r'[0-9a-f]{32}', v) for v in prefix)
                                                or expected_value is not None and (prefix != expected_value[:len(prefix)]
                                                    or value[-1]['transport_omission']['omitted_count'] != len(expected_value) - len(prefix))):
                                            errors.append(f'generated {key} Source basis')
                                        limits.append(f'generated {key} sources unavailable through scoped transport omission')
                                        omissions.append({'scope': item_scope + '/sources', 'placement': 'stable_suffix',
                                            'marker': value[-1]['transport_omission']})
                                        continue
                                    valid = (isinstance(value, str) and re.fullmatch(r'[0-9a-f]{32}', value) is not None
                                        if name == 'identity' else type(value) is int and value > 0
                                        if name == 'revision' else isinstance(value, list) and all(
                                            isinstance(v, str) and re.fullmatch(r'[0-9a-f]{32}', v) for v in value)
                                        if name == 'sources' else isinstance(value, str))
                                    if not valid or expected_value is not None and value != expected_value:
                                        errors.append(f'generated {key} Source basis' if name == 'sources'
                                            else f'generated {key} revision basis')
                        evidence = provenance.get('evidence')
                        if not isinstance(evidence, list) and not bounded_evidence:
                            errors.append('generated evidence keys malformed')
                        for index, item in enumerate(evidence if isinstance(evidence, list) else []):
                            if transport_omission(item) and (index == len(evidence) - 1 and suffix_omission(evidence)
                                    or 'exact_json_bytes' in item['transport_omission']):
                                continue
                            if (not isinstance(item, dict) or not isinstance(item.get('key'), str)) and not omitted(
                                    item, 'key', EVIDENCE, scope + '/evidence/' + str(index)):
                                errors.append('generated evidence keys malformed')
                        keys = {e['key'] for e in evidence if isinstance(e, dict) and isinstance(e.get('key'), str)} if isinstance(evidence, list) else set()
                        for paragraph_index, paragraph in enumerate(prose):
                            if transport_omission(paragraph):
                                limits.append('generated paragraph unavailable through scoped transport omission')
                                continue
                            if not isinstance(paragraph, dict) or paragraph.get('role') != 'generated_interpretation':
                                errors.append('generated paragraph role/grounding')
                                continue
                            if omitted(paragraph, 'text', CLAIM, '/selected_work/answers/prose/' + str(paragraph_index)):
                                limits.append('generated paragraph text unavailable through scoped transport omission')
                            elif not isinstance(paragraph.get('text'), str) or not paragraph['text'].strip():
                                errors.append('generated paragraph role/grounding')
                            cited = paragraph.get('evidence_keys')
                            if omitted(paragraph, 'evidence_keys', CLAIM, '/selected_work/answers/prose/' + str(paragraph_index)):
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
            scope_conflict = any(error in {'Project identity', 'selected Work identity',
                'generated Work/Project scope'} for error in errors)
            for report in omissions:
                report['scope_status'] = 'contradicted' if scope_conflict else 'bounded'
            status = ('confirmed_violation' if errors else 'indeterminate'
                if any('unavailable' in limit or 'unresolvable' in limit for limit in limits) else 'confirmed_pass')
            observations.append({k: v for k, v in returned.items() if k != 'result'} | {
                'raw_capture_sha256': capture.source_sha256, 'status': status,
                'errors': sorted(set(errors)), 'limits': sorted(set(limits)),
                'basis_kind': 'observation_time_authoring_and_immutable_checkpoint',
                'goal_basis': goal, 'lifecycle_basis': lifecycle, 'transport_omissions': omissions,
                'source_status_basis': 'returned_observation_only_not_later_bundle'})
    status = ('confirmed_violation' if any(o['status'] == 'confirmed_violation' for o in observations)
        else 'indeterminate' if any(o['status'] == 'indeterminate' for o in observations)
        else 'confirmed_pass' if observations else 'not_observed')
    return {'status': status, 'basis': {'observations': observations,
        'identity_basis': 'Project-scoped canonical Work and prior record receipts',
        'generated_semantics': 'qualitative_review_required'}}
