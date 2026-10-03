"""Observation-time shared answers from MCP and supported JSON CLI reads.

Later bundles corroborate immutable records; they do not attest historical Source
freshness or generated-text truth. Contradictions and unresolved basis are separate.
"""
from pathlib import Path
import sys

import codex_events as c

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'shared'))
from recorded_action_evidence import recorded_action_errors


def returned_recalls(capture):
    if capture is None:
        return []
    values = [{'transport': 'mcp', 'call_id': call.call_id, 'turn_id': call.turn_id,
        'sequence': call.sequence, 'completion_sequence': call.completion_sequence,
        'result': call.result} for call in capture.successful_calls('recall')]
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
            'completion_sequence': command.completion_sequence, 'result': result})
    return sorted(values, key=lambda value: value['sequence'])


def observe(work, resume, bundle, work_id):
    observations = []
    for capture in (work, resume):
        for returned in returned_recalls(capture):
            errors, limits = [], []
            result = returned['result']
            # Only previously observed authoring is a temporal witness. No latest
            # bundle selection and no use of returned identity to choose an oracle.
            temporal_goals = [call for call in capture.successful_calls('context_record')
                if call.arguments.get('role') == 'goal' and call.completion_sequence < returned['sequence']]
            expected_work = (work_id if capture is resume else
                temporal_goals[-1].result.get('context_item_id') if temporal_goals else None)
            prior = [call for source in (work, resume) if source
                for call in source.successful_calls('checkpoint_record')
                if (source is work and capture is resume or source is capture
                    and call.completion_sequence < returned['sequence'])
                and call.arguments.get('goal_context_id') == expected_work]
            latest = prior[-1] if prior else None
            goal_calls = [call for call in work.successful_calls('context_record')
                if call.result.get('context_item_id') == work_id] if work else []
            goal = goal_calls[0] if goal_calls else None
            checkpoint = bundle.one('checkpoints', project_id=bundle.project_id,
                id=latest.result.get('checkpoint_id')) if bundle and latest else None
            if not isinstance(result, dict):
                limits.append('returned_json_unresolvable')
            else:
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
                    expected = {'project_id': bundle.project_id, 'goal_id': work_id,
                        'checkpoint_id': checkpoint['id'], 'checkpoint_revision': checkpoint['revision'],
                        'checkpoint_sources': sources, 'next_step': latest.arguments.get('next_step')}
                    if checkpoint.get('work_item_id') != work_id or checkpoint.get('next_step') != expected['next_step']:
                        errors.append('authoring and immutable Checkpoint disagree')
                    errors.extend(recorded_action_errors(expected, result))
                    cp = result.get('checkpoint')
                    if not isinstance(cp, dict) or any(cp.get(key) != value for key, value in (
                            ('identity', checkpoint['id']), ('revision', checkpoint['revision']), ('work_item_id', work_id))):
                        errors.append('observation-time Checkpoint identity/revision')
                    answers = selected.get('answers') if isinstance(selected, dict) else None
                    if isinstance(answers, dict):
                        state, provenance = answers.get('explanation_state'), answers.get('provenance')
                        prose = answers.get('prose', [])
                        if not isinstance(prose, list):
                            errors.append('shared prose section malformed')
                            prose = []
                        if state not in {'current', 'unavailable', 'stale', 'corrupt', 'unsupported'}:
                            errors.append('explanation availability state')
                        if state == 'current':
                            if not isinstance(provenance, dict) or provenance.get('project_id') != bundle.project_id or provenance.get('subject') != {'kind': 'work', 'identity': work_id}:
                                errors.append('generated Work/Project scope')
                            else:
                                for key, identity, revision, field in (
                                        ('next_step', checkpoint['id'], checkpoint['revision'], 'next_step'),
                                        ('goal', work_id, goal.result.get('revision') if goal else None, 'statement')):
                                    evidence = provenance.get('evidence', [])
                                    matches = [e for e in evidence if isinstance(e, dict) and e.get('key') == key] if isinstance(evidence, list) else []
                                    if revision is None:
                                        limits.append('historical Goal revision unavailable')
                                    elif len(matches) != 1 or any(matches[0].get(k) != v for k, v in (('identity', identity), ('revision', revision), ('field', field))):
                                        errors.append(f'generated {key} revision basis')
                                    elif matches[0].get('sources') != (sources if key == 'next_step' else
                                            [goal.result.get('source_id')] if goal else []):
                                        errors.append(f'generated {key} Source basis')
                                keys = {e.get('key') for e in provenance.get('evidence', []) if isinstance(e, dict)}
                                for paragraph in prose if isinstance(prose, list) else []:
                                    if not isinstance(paragraph, dict) or paragraph.get('role') != 'generated_interpretation' or not isinstance(paragraph.get('text'), str) or not paragraph['text'].strip() or not isinstance(paragraph.get('evidence_keys'), list) or not set(paragraph['evidence_keys']) <= keys:
                                        errors.append('generated paragraph role/grounding')
                            limits.append('generated prose adequacy requires qualitative review')
                        elif provenance is not None or any(isinstance(p, dict) and p.get('question') == 'NextStep' for p in prose if isinstance(prose, list)):
                            errors.append('unusable generated direction revived')
                else:
                    limits.append('observation-time Checkpoint basis unavailable')
            status = ('confirmed_violation' if errors else 'indeterminate'
                if any('unavailable' in limit or 'unresolvable' in limit for limit in limits) else 'confirmed_pass')
            observations.append({k: v for k, v in returned.items() if k != 'result'} | {
                'raw_capture_sha256': capture.source_sha256, 'status': status,
                'errors': sorted(set(errors)), 'limits': sorted(set(limits)),
                'basis_kind': 'prior_authoring_and_immutable_checkpoint',
                'source_status_basis': 'returned_observation_only_not_later_bundle'})
    status = ('confirmed_violation' if any(o['status'] == 'confirmed_violation' for o in observations)
        else 'indeterminate' if any(o['status'] == 'indeterminate' for o in observations)
        else 'confirmed_pass' if observations else 'not_observed')
    return {'status': status, 'basis': {'observations': observations,
        'identity_basis': 'Project-scoped canonical Work and prior record receipts',
        'generated_semantics': 'qualitative_review_required'}}
