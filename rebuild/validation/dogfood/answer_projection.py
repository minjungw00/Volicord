"""Typed returned meaning for the private review plane, never generic RPC bodies.

Selection is data minimization. The caller still applies the unchanged sensitive
body policy. Contradictory Product returns remain reviewable as returns; computed
consistency diagnostics are not semantic judgments or authenticated provenance.
"""
from __future__ import annotations

import re

SCHEMA_VERSION = 3
MARKER = '_semantic_omission'
UNKNOWN = '_unretained_field_count'

# None is retained as a genuine Product null. Optional means the Product DTO may
# omit the member (notably QuestionAnswer.recorded_action), rather than null it.
def nullable(schema):
    return ('nullable', schema)


def optional(schema):
    return ('optional', nullable(schema))


S, I, B = str, int, bool
PRINCIPAL = {'kind': S, 'identity': S}
STATUS = {k: optional(t) for k, t in {
    'source_id': S, 'identity': S, 'availability': S, 'freshness': S,
    'snapshot_basis': S, 'snapshot': S, 'immutable': B, 'body': S, 'recorded_at': I,
    'recorded_at_unix_micros': I, 'actor': ('union', S, PRINCIPAL),
    'observer': ('union', S, PRINCIPAL)}.items()}
SUBJECT = {'kind': S, 'identity': [I]}
GENERATOR = {'host': S, 'session': S, 'agent': nullable(S), 'model': nullable(S)}
REFERENCE = {'checkpoint_id': S, 'checkpoint_revision': I, 'verification_index': I}
SCOPED_RUN = {'work_item_id': S, 'work_revision': I,
    'declaration': {'key': S, 'purpose': S, 'assertion': S, 'coverage': nullable(S),
        'environment': nullable(S), 'target_paths': [S]},
    **{k: S for k in ('operation_id', 'invocation_fingerprint', 'analysis_snapshot_id',
        'repository_snapshot_id', 'repository_source_id')},
    'started_at_unix_micros': I, 'ended_at_unix_micros': I,
    'artifacts': [{'path': S, 'sha256': S}], 'target_unchanged_during_run': B,
    'assertion_state': S, 'assertion_evidence': nullable({
        'stream': S, 'start_byte': I, 'end_byte': I, 'sha256': S}),
    'retest': nullable({**REFERENCE, 'source_id': S, 'target_relation': S, 'rationale': S})}
VERIFICATION = {'state': S, 'outcome': optional(S), 'source_id': optional(S),
    'source': optional(S), 'scoped_run': optional(SCOPED_RUN)}
REVIEW = {'state': S, 'source_id': optional(S)}
CONTENT = {k: optional(t) for k, t in {
    'acceptance': ('union', S, REVIEW), 'review': ('union', S, REVIEW),
    'later_changes': [S], 'observed_at': I, 'verification': [VERIFICATION],
    'work_state': S, 'known_limits': [S], 'non_goals': [S], 'choice': S,
    'chosen_alternative_key': S, 'state': S, 'alternative_key': S, 'rationale': S,
    'consequence': S, 'key': S, 'label': S, 'assumptions': [S], 'components': [S],
    'paths': [S], 'source_ids': [S], 'reported_change': S,
    'review_basis': [S], 'revisit_triggers': [S], 'scope': S,
    'work_contexts': [S], 'kind': S, 'goal': S, 'next_step': S,
    'open_questions': [{'identity': S, 'revision': I}], 'forgotten_source_count': I,
    'content_kind': S, 'byte_length': I, 'sha256': S}.items()}
EVIDENCE = {'key': S, 'record_kind': S, 'identity': S, 'revision': I,
    'field': S, 'sources': [S], 'content': nullable(('union', S, CONTENT, [CONTENT]))}
CONFLICT = {'from': ('union', S, SUBJECT, [I]), 'relation': S, 'to': ('union', S, SUBJECT, [I])}
CITATION = {**{k: S for k in ('source_id','observation_fingerprint','analysis_snapshot_id','repository_snapshot_id','repository_source_id','locator','field','range_sha256','claim_sha256','use_kind')},
    **{k: I for k in ('source_revision','start_line','end_line','start_byte','end_byte','claim_start_byte','claim_end_byte')}}
PROVENANCE = {'project_id': S, 'subject': SUBJECT, 'question': S, 'language': S,
    'fingerprint': S, 'generated_at_unix_micros': I, 'generator': GENERATOR,
    'generator_identity_status': S, 'semantic_approval_status': optional(S), 'citations': optional([CITATION]), 'evidence': [EVIDENCE], 'uncited_evidence_count': optional(I),
    'source_status': [STATUS], 'conflicts': [CONFLICT]}
PROVENANCE.update(publication_id=optional(S), realization_fingerprint=optional(S))
ACTION = {'work_item_id': S, 'checkpoint_id': S, 'revision': I, 'field': S,
    'recorded_text': S, 'source_ids': [S], 'source_status': [STATUS]}
CLAIM = {'question': S, 'text': S, 'role': S, 'evidence_keys': [S], 'citations': optional([CITATION]), 'recorded_action': optional(ACTION)}
DIAGRAM_LABEL = {'text': S, 'unit': I, 'start_byte': I, 'end_byte': I}
DIAGRAM_NODE = {'id': S, 'kind': S, 'label': DIAGRAM_LABEL, 'identifier': nullable(DIAGRAM_LABEL)}
DIAGRAM_EDGE = {'id': S, 'kind': S, 'from': S, 'to': S, 'label': DIAGRAM_LABEL}
DIAGRAM_MEANING = {'kind': S, 'nodes': optional([DIAGRAM_NODE]), 'participants': optional([DIAGRAM_NODE]),
    'owner': optional(DIAGRAM_LABEL),
    'relations': optional([{**DIAGRAM_EDGE, 'condition': nullable(DIAGRAM_LABEL)}]),
    'groups': optional([{'order': S, 'condition': nullable(DIAGRAM_LABEL), 'messages': [DIAGRAM_EDGE]}])}
DIAGRAM = {'id': S, 'title': DIAGRAM_LABEL, 'caption_unit': I, 'semantic_fingerprint': S, 'meaning': DIAGRAM_MEANING}
ANSWERS = {'diagrams': optional([DIAGRAM]), 'facts': [CLAIM], 'prose': [CLAIM], 'explanation_state': S,
    'diagnostic': nullable(S), 'provenance': nullable(PROVENANCE)}
ANSWERS.update({k: optional(t) for k, t in {
    'evidence_index': [{'unit_index': I, 'evidence_keys': [S], 'citations': [CITATION]}],
    'cautions': [{'kind': S, 'checkpoint': nullable(REFERENCE), 'source_id': nullable(S), 'historical': B}],
    'sections': [{'layer': S, 'heading_index': nullable(I), 'unit_indices': [I]}],
    'applicability': {'publication': S, 'canonical_basis_current': nullable(B),
        'source_ranges': [{'unit_index': I, 'citation_index': I, 'available': B,
            'expires_at_unix_micros': optional(I), 'limitation': nullable(S)}],
        'explained_snapshots': [{k: S for k in ('analysis_snapshot_id', 'repository_snapshot_id',
            'repository_source_id', 'use_kind')}],
        'generated_at_unix_micros': nullable(I), 'checked_at_unix_micros': nullable(I),
        'repository_comparison': S, 'comparison_scope': nullable(S), 'limitation': nullable(S)},
    'embedded_source_scope': [{'unit_index': I, 'state': S, 'included_citation_indices': [I],
        'omitted_citation_indices': [I], 'unavailable_citation_indices': [I]}],
    'omitted_source_excerpt_count': I,
    'source_excerpts': [{'code_origin': nullable({'path': S, 'revision': S,
        'start_byte': I, 'end_byte': I, 'start_line': I, 'end_line': I,
        'file_sha256': S, 'verified_at_unix_micros': I}),
        'citation': CITATION, 'work_revision': nullable(I), 'observed_at_unix_micros': I,
        'text': S, 'displayed_end_byte': I, 'displayed_end_line': I, 'omitted_utf8_bytes': I,
        'comparison': nullable(S), 'before_source_id': nullable(S),
        'source_representation': nullable(S),
        # Arbitrary Value extensions are not admitted as generic source/process
        # bodies. Nonempty unknown shapes retain explicit omission metadata.
        'observation_region': nullable({}), 'execution': nullable({})}],
}.items()})
ANSWERS['work_name'] = optional(ANSWERS)
ANSWERS['work_summary'] = optional(ANSWERS)
BASIS = {'record_kind': S, 'identity': S, 'revision': I, 'field': S,
    'source_ids': [S], 'source_status': [STATUS], 'available_revisions': [I],
    'analysis_snapshot_ids': [S], 'repository_snapshot_ids': [S]}
DETAIL_INSPECTION = {'tool': S, 'record_kind': S, 'record_id': S, 'revision': I, 'field': S}
READING = {'availability': S, 'basis': BASIS, 'original_text': nullable(S),
    'representation': S, 'display_english': S, 'display_korean': S,
    'original_language_preserved': B, 'omitted_characters': I, 'omitted_utf8_bytes': I, 'gaps': [S],
    'detail_inspection': optional(DETAIL_INSPECTION)}
STATE = {'checkpoint_id': S, 'checkpoint_revision': I, 'work_state': S,
    'observed_at_unix_micros': I, 'work_source_basis': [S], 'verification': [VERIFICATION],
    'user_review': REVIEW, 'user_acceptance': REVIEW, 'later_changed_checkpoint_ids': [S],
    'detail_inspection': optional(DETAIL_INSPECTION)}
WORK_EVIDENCE = {'goal': READING, 'next_step': READING, 'result': nullable(READING),
    'result_observed_at': nullable(I), 'original_changes': [READING], 'states': [STATE],
    'latest_state': nullable(STATE), 'verification': nullable(STATE), 'review': nullable(STATE), 'acceptance': nullable(STATE),
    'analysis_snapshot_ids': [S], 'repository_snapshot_ids': [S], 'source_status': [STATUS], 'code_availability': S}
WORK = {'work_item_id': S, 'checkpoint_ids': [S], 'answers': ANSWERS,
    **{k: optional(t) for k, t in {'state': S, 'source_ids': [S], 'decision_ids': [S],
        'open_question_ids': [S], 'changed_paths': [S], 'changed_components': [S], 'evidence': WORK_EVIDENCE}.items()}}
WORK_EVENT = {'kind': S, 'recorded_at_unix_micros': I, 'record_kind': S,
    'record_identity': S, 'revision': I}
WORK.update(creation_event=optional(WORK_EVENT), latest_work_record_event=optional(WORK_EVENT),
    reading_title=optional(S))


def evidence_detail(schema):
    return {'state': S, 'value': optional(schema)}


STREAM_CAPTURE = {'observed_bytes': I, 'omitted_bytes': nullable(I),
    'representation': S, 'capture_completeness': S}
COMMAND_READING = {**{k: S for k in ('operation_id', 'analysis_snapshot_id',
    'repository_snapshot_id', 'repository_source_id', 'target_state')},
    **{k: I for k in ('work_revision', 'started_at_unix_micros', 'ended_at_unix_micros',
        'retained_until_unix_micros')}, 'signal_number': nullable(I),
    'timeout_detected': B, 'cancellation_requested': B, 'stdout': STREAM_CAPTURE, 'stderr': STREAM_CAPTURE}
WORK['verification_report'] = optional({
    'report_count_unit': S, 'report_count_scope': S, 'reported_test_count_basis': S,
    'report_counts': {k: I for k in ('total', 'passed', 'failed', 'partial', 'not_run')},
    'checks': [{'original': REFERENCE, 'reported_at_unix_micros': I, 'reported_state': S,
        'original_outcome': nullable(S), 'source_id': nullable(S),
        **{k: evidence_detail(S) for k in ('intended_check', 'assertion_result', 'declared_coverage', 'declared_environment')},
        'scoped_run': evidence_detail(SCOPED_RUN), 'applicability': S,
        'later_changed_checkpoint_ids': [S], 'retest_comparison': {'state': S,
            'previous': optional(REFERENCE), **{k: optional(S) for k in (
                'source_id', 'target_relation', 'earlier_assertion', 'later_assertion', 'reason')}}}],
    'executions': [{'source_id': S, 'source_revision': I,
        'source_status': {'availability': S, 'freshness': S, 'snapshot_basis': nullable(S)},
        'command_label': S, 'invocation_fingerprint': S, 'capture_fingerprint': nullable(S),
        'recorded_at_unix_micros': I, 'exit_code': nullable(I), 'termination': S, 'body_state': S,
        'observation': evidence_detail(COMMAND_READING), 'observation_fingerprint': nullable(S),
        'environment': evidence_detail(S), 'assertion_result': evidence_detail(S),
        'checkpoint_reports': [REFERENCE]}]})
CONTEXT = {'identity': S, 'role': S, 'statement': S, 'source_ids': [S], 'revision': optional(I)}
CHECKPOINT = {k: optional(t) for k, t in {
    'identity': S, 'work_item_id': S, 'revision': I, 'kind': S, 'goal': S,
    'work_state': S, 'state_change': S, 'source_basis': [S], 'changed_source_basis': [S],
    'changed_paths': [S], 'known_limits': [S], 'non_goals': [S], 'open_questions': [('union', S, {'identity': S, 'revision': I})],
    'verification': [VERIFICATION], 'user_review': REVIEW, 'user_acceptance': REVIEW,
    'next_step': S, 'handoff_to': S,
    'recorded_at_unix_micros': I,
    'applied_decisions': [('union', S, {'decision_id': S, 'revision': I})]}.items()}
ALTERNATIVE = {k: optional(S) for k in ('alternative_key', 'expected_consequence', 'key', 'consequence', 'label')}
SCOPE = {'kind': S, 'context_item_id': optional(S), 'work_item_id': optional(S)}
DECISION_EVIDENCE = {k: optional(t) for k, t in {
    'available_revisions': [I], 'user_rationale': S, 'recommendation_rationale': S,
    'user_source_basis': [S], 'recommendation_source_basis': [S], 'assumptions': [S],
    'known_limits': [S], 'question_uncertainty': [S], 'review_basis': [S], 'revisit_triggers': [S]}.items()}
DECISION_EVIDENCE.update({k: optional(t) for k, t in {
    'question_reference': {'identity': S, 'revision': I},
    'question_context': {'identity': S, 'revision': I, 'prompt': S, 'source_ids': [S]},
    'question_source_status': [{'identity': S, 'availability': nullable(S), 'freshness': S}],
}.items()})
DECISION = {k: optional(t) for k, t in {'identity': S, 'revision': I, 'state': S,
    'choice': ('union', S, {'kind': S, 'alternative_key': optional(S)}), 'chosen_alternative_key': S,
    'recommended_alternative_key': S, 'displayed_alternatives': [ALTERNATIVE], 'work_scope': SCOPE,
    'source_basis': [S], 'source_ids': [S], 'rationale': S, 'user_rationale': S,
    'recommendation_rationale': S, 'assumptions': [S], 'known_limits': [S],
    'revisit_triggers': [S], 'answers': ANSWERS, 'evidence': DECISION_EVIDENCE}.items()}
DECISION['identity'] = S
DECISION['answers'] = ANSWERS
PLAN = {'project_id': S, 'subject': SUBJECT, 'question': S, 'requested_language': S,
    'fingerprint': S, 'evidence': [EVIDENCE], 'source_status': [STATUS], 'conflicts': [CONFLICT],
    'question_grounding': optional([{'question': S, 'required_any_evidence_keys': [S]}]),
    'retention_budget': {k: I for k in ('response_byte_limit', 'retained_byte_limit', 'metadata_byte_reserve', 'response_byte_capacity')}}
RESPONSE_CITATION = ('tuple', I, I, I, I, I, S, I, I, S)
RESPONSE_CONTEXT = {k: S for k in ('analysis_snapshot_id', 'repository_snapshot_id', 'repository_source_id')}
RESPONSE_SOURCE = {'context': I, 'source_revision': I,
    **{k: S for k in ('source_id', 'observation_fingerprint', 'locator', 'field', 'use_kind')}}
RESPONSE = {'diagrams': [DIAGRAM], 'format_kind': S, 'format_version': I, 'plan_fingerprint': S,
    'language': S, 'generator': GENERATOR,
    'paragraphs': [{'question': S, 'text': S, 'evidence_keys': [S], 'citations': [RESPONSE_CITATION]}],
    'citation_contexts': optional([RESPONSE_CONTEXT]), 'citation_sources': optional([RESPONSE_SOURCE])}
RECORD = {k: t for k, t in PROVENANCE.items() if k not in ('language', 'fingerprint', 'generator', 'uncited_evidence_count', 'citations')}
RECORD['realization'] = RESPONSE
SCHEMAS = {
    'recall': {'project_id': S, 'selected_work': nullable(WORK), 'next_step': nullable(S), 'checkpoint': nullable(CHECKPOINT),
        'goal_basis': [CONTEXT], 'decisions': [DECISION],
        'omissions': optional([{'identity': S, 'kind': S, 'reason': S, 'expandable_basis': S}]),
        'omitted_count': optional(I)},
    'project_status': {'project_id': S, 'selected_work': nullable(WORK),
        **{k: optional([WORK]) for k in ('current_work', 'completed_work', 'remaining_work', 'work_history', 'next_steps')},
        'selected_work_decisions': optional([DECISION])},
    'decisions': {'project_id': S, 'decisions': [DECISION], 'omissions': I},
    'explanation_prepare': {'operation': S, 'plan': PLAN},
    'explanation_record': {'operation': S, 'publication_id': optional(S), 'explanation': RECORD},
}
for schema in SCHEMAS.values():
    if 'operation' not in schema:
        schema['operation'] = optional(S)

# Deliberately excluded Source observation prose and generator instructions are
# never treated as trusted review instructions. Other nested unknowns are visible
# semantic omissions rather than an apparently complete identifier-only record.
TRANSPORT = {'reason': S, 'basis': S, **{k: optional(t) for k,t in {
    'omitted_count': I, 'omitted_field_count': I, 'exact_json_bytes': I, 'first_omitted_identity': S}.items()}}
OMISSIONS = {'unresolvable_observation', 'transport_omission', 'unsupported_typed_value', 'unsupported_typed_fields'}


def pointer(path, key):
    return path + '/' + str(key).replace('~', '~0').replace('/', '~1')


def select(value, schema, path='', *, validating=False, root=False):
    if validating and isinstance(value, dict) and set(value) == {MARKER}:
        if value[MARKER] not in OMISSIONS:
            raise ValueError('invalid typed semantic omission')
        return value
    if value is None:
        if isinstance(schema, tuple) and schema[0] in {'nullable', 'optional'}:
            return None
        return {MARKER: 'unresolvable_observation' if root else 'unsupported_typed_value'}
    if isinstance(value, dict) and set(value) == {'transport_omission'}:
        return {'transport_omission': select(value['transport_omission'], TRANSPORT, path, validating=validating)}
    if isinstance(schema, tuple):
        if schema[0] in {'optional', 'nullable'}:
            return select(value, schema[1], path, validating=validating)
        if schema[0] == 'tuple':
            if not isinstance(value, list) or len(value) != len(schema) - 1:
                return {MARKER: 'unsupported_typed_value'}
            return [select(v, child, pointer(path, n), validating=validating)
                for n, (v, child) in enumerate(zip(value, schema[1:], strict=True))]
        for branch in schema[1:]:
            result = select(value, branch, path, validating=validating)
            if not isinstance(result, dict) or MARKER not in result:
                return result
        return {MARKER: 'unsupported_typed_value'}
    if isinstance(schema, type):
        return value if type(value) is schema else {MARKER: 'unsupported_typed_value'}
    if isinstance(schema, list):
        if not isinstance(value, list) or len(value) > 4096:
            return {MARKER: 'unsupported_typed_value'}
        return [select(v, schema[0], pointer(path, n), validating=validating) for n, v in enumerate(value)]
    if not isinstance(value, dict):
        return {MARKER: 'unsupported_typed_value'}
    result = {}
    for key, child in schema.items():
        if key in value:
            result[key] = select(value[key], child, pointer(path, key), validating=validating)
        elif not (isinstance(child, tuple) and child[0] == 'optional'):
            result[key] = {MARKER: 'transport_omission'}
    if 'transport_omission' in value:
        result['transport_omission'] = select(value['transport_omission'], TRANSPORT, path, validating=validating)
    excluded = {'instructions'} if schema is PLAN else {'observation'} if schema is STATUS else set()
    unknown = set(value) - set(schema) - excluded - {'transport_omission'}
    if validating:
        unknown.discard(UNKNOWN)
        if unknown:
            raise ValueError('unsafe or unsupported retained typed fields')
        if UNKNOWN in value:
            if type(value[UNKNOWN]) is not int or value[UNKNOWN] < 1:
                raise ValueError('invalid omitted typed field count')
            result[UNKNOWN] = value[UNKNOWN]
    elif unknown and not root:
        result[UNKNOWN] = len(unknown)
    # Topic coverage belongs to semantic review, not transport reconstruction.
    # Preserve the actual claim list, including empty lists and absent topics.
    # Only missing DTO members or explicit Product markers establish omissions.
    return result


def omissions(value, path=''):
    result = []
    if isinstance(value, dict):
        if MARKER in value:
            return [{'pointer': path, 'reason': value[MARKER]}]
        if 'transport_omission' in value:
            result.append({'pointer': path, 'reason': 'transport_omission'})
        if UNKNOWN in value:
            result.append({'pointer': path, 'reason': 'unsupported_typed_fields', 'count': value[UNKNOWN]})
        for key in sorted(value):
            result.extend(omissions(value[key], pointer(path, key)))
    elif isinstance(value, list):
        for n, child in enumerate(value):
            result.extend(omissions(child, pointer(path, n)))
    return result


def consistency(value, operation):
    """Receipt/identity consistency only; factual machine oracle stays separate."""
    errors = []
    missing = object()

    def unknown(item):
        return item is missing or (isinstance(item, dict) and (MARKER in item
            or ('transport_omission' in item and set(item) <= {'transport_omission', UNKNOWN})))

    def array_length(items):
        report = items[-1].get('transport_omission') if items and isinstance(items[-1], dict) else None
        if isinstance(report, dict) and 'omitted_count' in report:
            count = report['omitted_count']
            return len(items) - 1 + count if type(count) is int and count >= 0 else missing
        return len(items)

    def contradicts(actual, expected):
        # An omission is still incomplete evidence. It supplies no value to
        # compare, and cannot excuse a different visible member of the object.
        if unknown(actual) or unknown(expected):
            return False
        if type(actual) is not type(expected):
            return True
        if isinstance(expected, dict):
            return any(contradicts(actual.get(key, missing), item)
                       for key, item in expected.items())
        if isinstance(expected, list):
            return (any(contradicts(left, right) for left, right in zip(actual, expected))
                or contradicts(array_length(actual), array_length(expected)))
        return actual != expected

    if isinstance(value, dict) and isinstance(value.get('operation'), str) and value['operation'] != operation:
        errors.append('returned operation differs from request')
    def answers(work, kind, project):
        if not isinstance(work, dict) or not isinstance(work.get('answers'), dict):
            return
        a = work['answers']; identity = work.get('work_item_id' if kind == 'work' else 'identity')
        p = a.get('provenance')
        if isinstance(p, dict) and MARKER not in p:
            expected = (list(bytes.fromhex(identity))
                if isinstance(identity, str) and re.fullmatch(r'[0-9a-f]{32}', identity)
                else missing if unknown(identity) else None)
            if (contradicts(p.get('project_id', missing), project)
                    or contradicts(p.get('subject', missing), {'kind': kind, 'identity': expected})):
                errors.append('generated subject/Project mismatch')
            if contradicts(p.get('generator_identity_status', missing), 'self_reported_not_independently_verified'):
                errors.append('generator provenance assertion changed')
            evidence = p.get('evidence'); keys = {e.get('key') for e in evidence if isinstance(e, dict) and isinstance(e.get('key'), str)} if isinstance(evidence, list) else set()
            complete_keys = (isinstance(evidence, list) and all(
                isinstance(item, dict) and isinstance(item.get('key'), str) for item in evidence))
            for claim in a.get('prose', []) if isinstance(a.get('prose'), list) else []:
                if not isinstance(claim, dict) or MARKER in claim:
                    continue
                cited = claim.get('evidence_keys')
                if (contradicts(claim.get('role', missing), 'generated_interpretation')
                        or complete_keys and isinstance(cited, list)
                        and any(k not in keys for k in cited if isinstance(k, str))):
                    errors.append('generated role/grounding mismatch')
        if (contradicts(a.get('explanation_state', missing), 'current')
                and p is not None and not omissions(p)):
            errors.append('non-current explanation carries generated provenance')
        for fact in a.get('facts', []) if isinstance(a.get('facts'), list) else []:
            action = fact.get('recorded_action') if isinstance(fact, dict) else None
            if not isinstance(action, dict) or MARKER in action:
                continue
            if (isinstance(action.get('work_item_id'), str) and contradicts(action['work_item_id'], identity)
                    or isinstance(action.get('checkpoint_id'), str) and isinstance(work.get('checkpoint_ids'), list)
                    and not omissions(work['checkpoint_ids'])
                    and action['checkpoint_id'] not in work['checkpoint_ids']):
                errors.append('recorded action Work/Checkpoint mismatch')
            if any(type(action.get(k)) is not t for k,t in (('checkpoint_id', str), ('revision', int), ('recorded_text', str))):
                continue
            expected_key = f"checkpoint:{action['checkpoint_id']}@{action['revision']}:next_step"
            if (contradicts(fact.get('question', missing), 'RecordedNextStep')
                    or contradicts(fact.get('role', missing), 'deterministic_facts')
                    or contradicts(action.get('field', missing), 'next_step')
                    or contradicts(fact.get('evidence_keys', missing), [expected_key])):
                errors.append('recorded action role/evidence key mismatch')
            if not unknown(fact.get('text', missing)) and fact.get('text') not in [prefix + action['recorded_text'] for prefix in (
                    'Recorded next action quotation (original language): ', '기록된 다음 행동 인용 (원문 언어): ')]:
                errors.append('recorded action quotation mismatch')
            if operation == 'recall':
                cp = value.get('checkpoint')
                if contradicts(value.get('next_step', missing), action['recorded_text']) or isinstance(cp, dict) and any(contradicts(cp.get(k, missing), v) for k, v in (
                    ('identity', action['checkpoint_id']), ('revision', action['revision']), ('work_item_id', identity), ('next_step', action['recorded_text']))):
                    errors.append('returned task direction/Checkpoint mismatch')
    project = value.get('project_id', missing) if isinstance(value, dict) else missing
    if isinstance(value, dict):
        answers(value.get('selected_work'), 'work', project)
        for field in ('decisions', 'selected_work_decisions'):
            for d in value.get(field, []) if isinstance(value.get(field), list) else []:
                answers(d, 'decision', project)
        for field in ('current_work', 'completed_work', 'remaining_work', 'work_history', 'next_steps'):
            for w in value.get(field, []) if isinstance(value.get(field), list) else []:
                answers(w, 'work', project)
    return sorted(set(errors))


def project(value, operation):
    schema = SCHEMAS.get(operation)
    if schema is None:
        raise ValueError('unsupported typed answer operation')
    selected = select(value, schema, root=True)
    missing = omissions(selected)
    return {'schema_version': SCHEMA_VERSION, 'operation': operation, 'value': selected,
        'omissions': missing, 'semantic_complete': not missing,
        'consistency_errors': consistency(selected, operation)}


def validate(value):
    if not isinstance(value, dict) or set(value) != {'schema_version', 'operation', 'value',
            'omissions', 'semantic_complete', 'consistency_errors'} or value['schema_version'] != SCHEMA_VERSION:
        raise ValueError('unsupported returned answer projection')
    schema = SCHEMAS.get(value['operation'])
    if schema is None or value['value'] != select(value['value'], schema, validating=True, root=True):
        raise ValueError('returned answer fields changed')
    missing = omissions(value['value'])
    if (value['omissions'] != missing or type(value['semantic_complete']) is not bool
            or value['semantic_complete'] != (not missing)
            or value['consistency_errors'] != consistency(value['value'], value['operation'])):
        raise ValueError('returned answer omissions/consistency changed')
    return value
