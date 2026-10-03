"""Private review linkage for steward lifecycles, distinct from measured returns."""
from __future__ import annotations

import datetime as dt
import json
import re

import answer_projection as projection
import explanation_evidence as evidence
import review_captures
import evidence_purpose

SCHEMA_VERSION = 1
SURFACE = 'explanation_lifecycle'
CONTEXT_FIELDS = {'evidence_purpose', 'identity', 'journey_id', 'project_id', 'subject', 'language', 'candidate_head',
    'candidate_executable_sha256', 'phase', 'observed_at', 'raw_inputs', 'generator_identity_limit',
    'generation_authority', 'canonical_bundle_sha256'}


def ops():
    import review_operations
    return review_operations


def subject_read(value, subject):
    """Select exact returned row with its original pointer, never a latest bundle."""
    if subject['kind'] == 'decision':
        fields, key = ('decisions',), 'identity'
    else:
        fields, key = ('selected_work', 'current_work', 'completed_work', 'remaining_work', 'work_history', 'next_steps'), 'work_item_id'
    matches = []
    for field in fields:
        rows = [value.get(field)] if field == 'selected_work' else value.get(field, [])
        for number, row in enumerate(rows if isinstance(rows, list) else []):
            if isinstance(row, dict) and row.get(key) == subject['identity']:
                matches.append((('/' + field) if field == 'selected_work' else f'/{field}/{number}', row))
    if not matches:
        return {'project_id': value.get('project_id'), 'selected_work': None}, [], 'project_status'
    row = matches[0][1]
    if subject['kind'] == 'decision':
        result, operation = {'project_id': value.get('project_id'), 'decisions': [row], 'omissions': 0}, 'decisions'
    else:
        result, operation = {'project_id': value.get('project_id'), 'selected_work': row}, 'project_status'
    return result, [path for path, _ in matches], operation


def project(root, preparation_path, *, evidence_set_sha256):
    o = ops()
    preparation, receipt = evidence.verify(root, preparation_path)
    directory = preparation_path.parent
    raw = {name: evidence.bound(root, directory / (name + '.json'))
        for name in ('preparation', 'before', 'response', 'record', 'after', 'receipt')}
    before, before_paths, before_op = subject_read(json.loads(raw['before']), preparation['subject'])
    after, after_paths, after_op = subject_read(json.loads(raw['after']), preparation['subject'])
    inputs = {
        'plan': projection.project({'operation': 'explanation_prepare', 'plan': preparation['plan']}, 'explanation_prepare'),
        'response': projection.select(json.loads(raw['response']), projection.RESPONSE),
        'record': projection.project(json.loads(raw['record']), 'explanation_record'),
        'before': projection.project(before, before_op), 'after': projection.project(after, after_op)}
    stages = {name: review_captures.body_projection(value) for name, value in inputs.items()}
    value = {'kind': 'dogfood_review_explanation_lifecycle', 'schema_version': SCHEMA_VERSION,
        'context': {k: preparation[k] for k in CONTEXT_FIELDS},
        'evidence_set_sha256': evidence_set_sha256,
        'private_artifacts': {name: evidence.binding(data) for name, data in raw.items()},
        'receipt': receipt, 'stages': stages,
        'readback_subject_locators': {'before': before_paths, 'after': after_paths},
        'before_observation': {k: preparation['before_observation'].get(k) for k in ('state', 'reason')},
        'before_state': preparation['before_observation']['state'], 'after_state': receipt['after_state'],
        'limits': ['post-session generation cannot attest measured use or adoption',
            'selected_json stage hashes differ from exact private artifact hashes',
            'self-reported generator/response identity has no independent host-turn attestation']}
    value['semantic_complete'] = complete(value)
    data = o.encoded(value)
    validate(data)
    return data, {'schema_version': SCHEMA_VERSION, 'semantic_complete': value['semantic_complete'],
        'review_bytes': len(data), 'review_sha256': o.digest(data)}


def complete(value):
    if value['before_state'] == 'unresolvable':
        return False
    for name, body in value['stages'].items():
        if body['state'] != 'retained':
            return False
        selected = body['value']
        if projection.omissions(selected) or name != 'response' and not selected['semantic_complete']:
            return False
    return True


def stage_bindings(value):
    return {name: {k: body[k] for k in ('state', 'reason', 'source_body_encoding',
        'source_body_bytes', 'source_body_sha256')} for name, body in value['stages'].items()}


def verify_manifest(value, manifest):
    """Cross-check the copied source manifest, without original private files."""
    o = ops(); context = value['context']
    matches = [v for v in manifest['explanation_evidence']['steward_lifecycles']
        if v['identity'] == context['identity']]
    o.review.require(len(matches) == 1, 'copied lifecycle missing from source evidence index')
    indexed = matches[0]
    o.review.require(all(indexed[k] == context[k] for k in ('subject', 'language', 'journey_id', 'project_id', 'phase'))
        and indexed['before_state'] == value['before_state'] and indexed['after_state'] == value['after_state']
        and context['candidate_head'] == manifest['candidate_head']
        and context['candidate_executable_sha256'] == manifest['candidate_artifacts']['volicord']['sha256'],
        'copied lifecycle subject/candidate/index changed')
    o.review.require(indexed['review_stage_bindings'] == stage_bindings(value)
        and indexed['readback_subject_locators'] == value['readback_subject_locators'],
        'copied lifecycle meaning/locators differ from source index')
    directory = o.Path(indexed['preparation']).parent
    for stage, binding in value['private_artifacts'].items():
        name = (directory / (stage + '.json')).as_posix()
        o.review.require(manifest['artifacts'].get(name) == binding, 'copied lifecycle private source binding changed')
    indexed_raw = {v['session_slot_id']: v for v in manifest['raw_inputs']}
    o.review.require(len(context['raw_inputs']) == len(indexed_raw)
        and {v['session_slot_id'] for v in context['raw_inputs']} == set(indexed_raw),
        'copied lifecycle measured-source coverage changed')
    for raw in context['raw_inputs']:
        source = indexed_raw.get(raw['session_slot_id'])
        o.review.require(source is not None and all(raw[k] == source[k] for k in ('session_slot', 'session_id', 'sha256')),
            'copied lifecycle measured-source binding changed')


def validate(data):
    """Copied packages need neither Runtime Home nor the submitted response path."""
    o = ops(); o.require_review_artifact_safe(data)
    value = json.loads(data)
    o.review.require(isinstance(value, dict) and set(value) == {'kind', 'schema_version', 'context',
        'evidence_set_sha256', 'private_artifacts', 'receipt', 'stages', 'readback_subject_locators',
        'before_state', 'after_state', 'before_observation', 'limits', 'semantic_complete'}
        and value['kind'] == 'dogfood_review_explanation_lifecycle' and value['schema_version'] == SCHEMA_VERSION,
        'unsupported review explanation lifecycle')
    context, receipt = value['context'], value['receipt']
    evidence_purpose.require_same(context, receipt)
    o.review.require(isinstance(receipt, dict) and set(receipt) == {'kind', 'schema_version', 'identity',
        'evidence_purpose', 'phase', 'observed_at', 'preparation', 'response', 'record', 'readback', 'candidate_head',
        'candidate_executable_sha256', 'generator_identity_status', 'host_response_locator', 'after_state'},
        'unsupported retained explanation receipt')
    o.review.require(set(context) == CONTEXT_FIELDS and context['phase'] == 'post_session_steward'
        and context['generation_authority'] == ('self_authored_support_no_provider_dispatch' if context['evidence_purpose'] == evidence_purpose.REHEARSAL else 'current_active_host_interaction_required_no_provider_dispatch')
        and re.fullmatch(r'[0-9a-f]{64}', context['canonical_bundle_sha256'])
        and context['generator_identity_limit'] == 'self_reported_not_independently_verified'
        and re.fullmatch(r'[0-9a-f]{40}', context['candidate_head'])
        and re.fullmatch(r'[0-9a-f]{64}', context['candidate_executable_sha256'])
        and re.fullmatch(r'[0-9a-f]{64}', value['evidence_set_sha256'])
        and re.fullmatch(r'[0-9a-f]{32}', context['project_id'])
        and set(context['subject']) == {'kind', 'identity'}
        and context['subject']['kind'] in {'work', 'decision'}
        and re.fullmatch(r'[0-9a-f]{32}', context['subject']['identity'])
        and isinstance(context['language'], str) and context['language'].strip()
        and len(context['language'].encode('utf-8')) <= 128
        and not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in context['language']),
        'review explanation context/authority changed')
    started, ended = dt.datetime.fromisoformat(context['observed_at']), dt.datetime.fromisoformat(receipt['observed_at'])
    o.review.require(started.tzinfo is not None and ended.tzinfo is not None and started <= ended,
        'review explanation observation order changed')
    bindings = value['private_artifacts']
    o.review.require(set(bindings) == {'preparation', 'before', 'response', 'record', 'after', 'receipt'}
        and all(set(b) == {'bytes', 'sha256'} and type(b['bytes']) is int and b['bytes'] > 0
            and re.fullmatch(r'[0-9a-f]{64}', b['sha256']) for b in bindings.values())
        and receipt['preparation'] == bindings['preparation'] and receipt['response'] == bindings['response']
        and receipt['record'] == bindings['record'] and receipt['readback'] == bindings['after']
        and receipt['kind'] == 'dogfood_explanation_receipt' and receipt['schema_version'] == evidence.SCHEMA_VERSION
        and receipt['identity'] == context['identity'] and receipt['phase'] == context['phase']
        and receipt['candidate_head'] == context['candidate_head']
        and receipt['candidate_executable_sha256'] == context['candidate_executable_sha256']
        and receipt['generator_identity_status'] == context['generator_identity_limit']
        and receipt['after_state'] == value['after_state'] == 'current', 'review explanation receipt linkage changed')
    o.review.require(bindings['receipt'] == evidence.binding(evidence.api().json_bytes(receipt)), 'review lifecycle receipt binding changed')
    stages = value['stages']
    o.review.require(set(stages) == {'plan', 'response', 'record', 'before', 'after'}, 'review explanation stage omitted')
    for name, body in stages.items():
        o.review.require(isinstance(body, dict) and set(body) == {'state', 'reason', 'source_body_encoding',
            'source_body_bytes', 'source_body_sha256', 'value'}
            and body['source_body_encoding'] == 'selected_json'
            and type(body['source_body_bytes']) is int and body['source_body_bytes'] >= 0
            and re.fullmatch(r'[0-9a-f]{64}', body['source_body_sha256']), 'invalid review explanation stage body')
        if body['state'] == 'retained':
            o.review.require(body == review_captures.body_projection(body['value']), 'review explanation body hash changed')
            if name == 'response':
                o.review.require(body['value'] == projection.select(body['value'], projection.RESPONSE, validating=True),
                    'unsupported retained explanation response')
            else:
                projection.validate(body['value'])
        else:
            o.review.require(body['state'] == 'omitted' and body['reason'] in {'sensitive_payload', 'body_limit'}
                and body['value'] is None, 'invalid review explanation omission')
    o.review.require(type(value['semantic_complete']) is bool and value['semantic_complete'] == complete(value),
        'review explanation completeness changed')
    o.review.require(set(value['before_observation']) == {'state', 'reason'}
        and value['before_observation']['state'] == value['before_state']
        and value['before_state'] in {'current', 'unavailable', 'stale', 'corrupt', 'unsupported', 'unresolvable'},
        'invalid earlier explanation observation')
    locators = value['readback_subject_locators']
    pattern = (r'/decisions/[0-9]+' if context['subject']['kind'] == 'decision' else
        r'/(?:selected_work|(?:current_work|completed_work|remaining_work|work_history|next_steps)/[0-9]+)')
    o.review.require(isinstance(locators, dict) and set(locators) == {'before', 'after'}
        and all(isinstance(paths, list) and len(paths) <= 4096 and len(set(paths)) == len(paths)
            and all(isinstance(path, str) and re.fullmatch(pattern, path) for path in paths) for paths in locators.values())
        and bool(locators['after']), 'invalid explanation subject row locators')
    # Partial/redacted lifecycles remain explicitly insufficient, never fake proof.
    if value['semantic_complete']:
        plan = stages['plan']['value']['value']['plan']
        response = stages['response']['value']
        record = stages['record']['value']['value']
        before = stages['before']['value']['value']; after = stages['after']['value']['value']
        evidence.validate_lifecycle(dict(context, plan=plan), response, record, after)
        o.review.require(bindings['record'] == evidence.binding(evidence.api().json_bytes(record)),
            'retained record meaning differs from exact private receipt artifact')
        observation = evidence.subject_answers(before, context['subject'])
        o.review.require({k: observation.get(k) for k in ('state', 'reason')} == value['before_observation'],
            'earlier explanation observation changed')
        o.review.require(receipt['host_response_locator'] == {'kind': 'submitted_response_file',
            'sha256': bindings['response']['sha256'], 'session': response['generator']['session'],
            'raw_capture_sha256': None, 'turn_id': None,
            'limit': 'no independent response authorship or host-turn attestation'},
            'review explanation provenance assertion changed')
    return value
