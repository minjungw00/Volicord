"""Typed returned meaning for the private review plane, never generic RPC bodies.

Selection is data minimization. The caller still applies the unchanged sensitive
body policy. Contradictory Product returns remain reviewable as returns; computed
consistency diagnostics are not semantic judgments or authenticated provenance.
"""
from __future__ import annotations

import re

SCHEMA_VERSION = 1
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
    'snapshot_basis': S, 'snapshot': S, 'immutable': B, 'recorded_at': I,
    'recorded_at_unix_micros': I, 'actor': ('union', S, PRINCIPAL),
    'observer': ('union', S, PRINCIPAL)}.items()}
SUBJECT = {'kind': S, 'identity': [I]}
GENERATOR = {'host': S, 'session': S, 'agent': nullable(S), 'model': nullable(S)}
VERIFICATION = {'state': S, 'outcome': optional(S), 'source_id': optional(S), 'source': optional(S)}
REVIEW = {'state': S, 'source_id': optional(S)}
CONTENT = {k: optional(t) for k, t in {
    'acceptance': ('union', S, REVIEW), 'review': ('union', S, REVIEW),
    'later_changes': [S], 'observed_at': I, 'verification': [VERIFICATION],
    'work_state': S, 'known_limits': [S], 'non_goals': [S], 'choice': S,
    'chosen_alternative_key': S, 'state': S, 'alternative_key': S, 'rationale': S,
    'consequence': S, 'key': S, 'label': S, 'assumptions': [S], 'components': [S],
    'paths': [S], 'review_basis': [S], 'revisit_triggers': [S], 'scope': S,
    'work_contexts': [S]}.items()}
EVIDENCE = {'key': S, 'record_kind': S, 'identity': S, 'revision': I,
    'field': S, 'sources': [S], 'content': nullable(('union', S, CONTENT, [CONTENT]))}
CONFLICT = {'from': ('union', S, SUBJECT, [I]), 'relation': S, 'to': ('union', S, SUBJECT, [I])}
PROVENANCE = {'project_id': S, 'subject': SUBJECT, 'question': S, 'language': S,
    'fingerprint': S, 'generated_at_unix_micros': I, 'generator': GENERATOR,
    'generator_identity_status': S, 'evidence': [EVIDENCE],
    'source_status': [STATUS], 'conflicts': [CONFLICT]}
ACTION = {'work_item_id': S, 'checkpoint_id': S, 'revision': I, 'field': S,
    'recorded_text': S, 'source_ids': [S], 'source_status': [STATUS]}
CLAIM = {'question': S, 'text': S, 'role': S, 'evidence_keys': [S], 'recorded_action': optional(ACTION)}
ANSWERS = {'facts': [CLAIM], 'prose': [CLAIM], 'explanation_state': S,
    'diagnostic': nullable(S), 'provenance': nullable(PROVENANCE)}
BASIS = {'record_kind': S, 'identity': S, 'revision': I, 'field': S,
    'source_ids': [S], 'source_status': [STATUS], 'available_revisions': [I],
    'analysis_snapshot_ids': [S], 'repository_snapshot_ids': [S]}
READING = {'availability': S, 'basis': BASIS, 'original_text': nullable(S),
    'representation': S, 'display_english': S, 'display_korean': S,
    'original_language_preserved': B, 'omitted_characters': I, 'omitted_utf8_bytes': I, 'gaps': [S]}
STATE = {'checkpoint_id': S, 'checkpoint_revision': I, 'work_state': S,
    'observed_at_unix_micros': I, 'work_source_basis': [S], 'verification': [VERIFICATION],
    'user_review': REVIEW, 'user_acceptance': REVIEW, 'later_changed_checkpoint_ids': [S]}
WORK_EVIDENCE = {'goal': READING, 'next_step': READING, 'result': nullable(READING),
    'result_observed_at': nullable(I), 'original_changes': [READING], 'states': [STATE],
    'latest_state': nullable(STATE), 'verification': nullable(STATE), 'review': nullable(STATE), 'acceptance': nullable(STATE),
    'analysis_snapshot_ids': [S], 'repository_snapshot_ids': [S], 'source_status': [STATUS], 'code_availability': S}
WORK = {'work_item_id': S, 'checkpoint_ids': [S], 'answers': ANSWERS,
    **{k: optional(t) for k, t in {'state': S, 'source_ids': [S], 'decision_ids': [S],
        'open_question_ids': [S], 'changed_paths': [S], 'changed_components': [S], 'evidence': WORK_EVIDENCE}.items()}}
CONTEXT = {'identity': S, 'role': S, 'statement': S, 'source_ids': [S]}
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
DECISION = {k: optional(t) for k, t in {'identity': S, 'revision': I, 'state': S,
    'choice': ('union', S, {'kind': S, 'alternative_key': optional(S)}), 'chosen_alternative_key': S,
    'recommended_alternative_key': S, 'displayed_alternatives': [ALTERNATIVE], 'work_scope': SCOPE,
    'source_basis': [S], 'source_ids': [S], 'rationale': S, 'user_rationale': S,
    'recommendation_rationale': S, 'assumptions': [S], 'known_limits': [S],
    'revisit_triggers': [S], 'answers': ANSWERS, 'evidence': DECISION_EVIDENCE}.items()}
DECISION['identity'] = S
DECISION['answers'] = ANSWERS
PLAN = {'project_id': S, 'subject': SUBJECT, 'question': S, 'requested_language': S,
    'fingerprint': S, 'evidence': [EVIDENCE], 'source_status': [STATUS], 'conflicts': [CONFLICT]}
RESPONSE = {'format_kind': S, 'format_version': I, 'plan_fingerprint': S,
    'language': S, 'generator': GENERATOR,
    'paragraphs': [{'question': S, 'text': S, 'evidence_keys': [S]}]}
RECORD = {k: t for k, t in PROVENANCE.items() if k not in ('language', 'fingerprint', 'generator')}
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
    'explanation_record': {'operation': S, 'explanation': RECORD},
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
    if schema is WORK or schema is DECISION:
        answers = result.get('answers')
        if isinstance(answers, dict):
            def require_questions(field, required, any_question=False):
                claims = answers.get(field)
                if not isinstance(claims, list):
                    return
                present = {c.get('question') for c in claims if isinstance(c, dict) and isinstance(c.get('question'), str)}
                missing = not (required & present) if any_question else not required <= present
                if missing and not any(isinstance(c, dict) and MARKER in c for c in claims):
                    claims.append({MARKER: 'transport_omission'})
            if schema is WORK:
                require_questions('facts', {'RecordedNextStep', 'NextStepAvailability'}, True)
            if answers.get('explanation_state') == 'current':
                required = {'Purpose', 'ReportedChange', 'ExpectedEffect', 'Verification', 'NextStep'} if schema is WORK else {'UserRationale', 'Recommendation', 'Consequences', 'Applicability'}
                require_questions('prose', required)
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
        for key, child in value.items():
            result.extend(omissions(child, pointer(path, key)))
    elif isinstance(value, list):
        for n, child in enumerate(value):
            result.extend(omissions(child, pointer(path, n)))
    return result


def consistency(value, operation):
    """Receipt/identity consistency only; factual machine oracle stays separate."""
    errors = []
    if isinstance(value, dict) and isinstance(value.get('operation'), str) and value['operation'] != operation:
        errors.append('returned operation differs from request')
    def answers(work, kind, project):
        if not isinstance(work, dict) or not isinstance(work.get('answers'), dict):
            return
        a = work['answers']; identity = work.get('work_item_id' if kind == 'work' else 'identity')
        p = a.get('provenance')
        if isinstance(p, dict) and MARKER not in p:
            expected = list(bytes.fromhex(identity)) if isinstance(identity, str) and re.fullmatch(r'[0-9a-f]{32}', identity) else None
            if p.get('project_id') != project or p.get('subject') != {'kind': kind, 'identity': expected}:
                errors.append('generated subject/Project mismatch')
            if p.get('generator_identity_status') != 'self_reported_not_independently_verified':
                errors.append('generator provenance assertion changed')
            evidence = p.get('evidence'); keys = {e.get('key') for e in evidence if isinstance(e, dict) and isinstance(e.get('key'), str)} if isinstance(evidence, list) else set()
            for claim in a.get('prose', []) if isinstance(a.get('prose'), list) else []:
                if not isinstance(claim, dict) or MARKER in claim:
                    continue
                cited = claim.get('evidence_keys')
                if claim.get('role') != 'generated_interpretation' or not isinstance(cited, list) or any(k not in keys for k in cited if isinstance(k, str)):
                    errors.append('generated role/grounding mismatch')
        if a.get('explanation_state') != 'current' and p is not None and not omissions(p):
            errors.append('non-current explanation carries generated provenance')
        for fact in a.get('facts', []) if isinstance(a.get('facts'), list) else []:
            action = fact.get('recorded_action') if isinstance(fact, dict) else None
            if not isinstance(action, dict) or MARKER in action:
                continue
            if (isinstance(action.get('work_item_id'), str) and action['work_item_id'] != identity
                    or isinstance(action.get('checkpoint_id'), str) and isinstance(work.get('checkpoint_ids'), list)
                    and action['checkpoint_id'] not in work['checkpoint_ids']):
                errors.append('recorded action Work/Checkpoint mismatch')
            if any(type(action.get(k)) is not t for k,t in (('checkpoint_id', str), ('revision', int), ('recorded_text', str))):
                continue
            expected_key = f"checkpoint:{action['checkpoint_id']}@{action['revision']}:next_step"
            if fact.get('question') != 'RecordedNextStep' or fact.get('role') != 'deterministic_facts' or action.get('field') != 'next_step' or fact.get('evidence_keys') != [expected_key]:
                errors.append('recorded action role/evidence key mismatch')
            if fact.get('text') not in [prefix + action['recorded_text'] for prefix in (
                    'Recorded next action quotation (original language): ', '기록된 다음 행동 인용 (원문 언어): ')]:
                errors.append('recorded action quotation mismatch')
            if operation == 'recall':
                cp = value.get('checkpoint')
                if value.get('next_step') != action['recorded_text'] or isinstance(cp, dict) and any(cp.get(k) != v for k, v in (
                    ('identity', action['checkpoint_id']), ('revision', action['revision']), ('work_item_id', identity), ('next_step', action['recorded_text']))):
                    errors.append('returned task direction/Checkpoint mismatch')
    project = value.get('project_id') if isinstance(value, dict) else None
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
