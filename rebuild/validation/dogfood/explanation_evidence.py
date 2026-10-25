"""Private append-only Work/Decision explanation lifecycle evidence.

Product CLI constructs plans, records responses and supplies shared-answer reads.
This collector never generates prose or invokes a provider. All steward activity
is post-session material, independent of earlier measured-session observations.
"""
from __future__ import annotations

import copy
import datetime as dt
import hashlib
import json
import evidence_purpose
from pathlib import Path
import secrets
import subprocess
import tempfile

import answer_observations
import codex_events
import document_realization

SCHEMA_VERSION = 2
RESPONSE_BYTE_LIMIT = 49152
MIN_RESPONSE_BYTE_CAPACITY = 16384
PARAGRAPH_CHARACTER_LIMIT = 16384
PLAN_BYTE_LIMIT = 131072
RETAINED_BYTE_LIMIT = 147456
INPUT_BYTE_LIMIT = 65536
WORK_QUESTIONS = {'purpose', 'reported_change', 'expected_effect', 'verification', 'next_step'}
DECISION_QUESTIONS = {'user_rationale', 'recommendation', 'consequences', 'applicability'}


def compact_bytes(value):
    """Product compact JSON byte measurement; formatting is a separate boundary."""
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode('utf-8')


def retention_metadata_bytes(plan):
    evidence = copy.deepcopy(plan['evidence'])
    for item in evidence:
        item['content'] = None
    retained = {'realization': None, 'project_id': plan['project_id'], 'subject': plan['subject'],
        'question': plan['question'], 'evidence': evidence,
        'source_status': [{k: v for k, v in status.items() if k != 'observation'} for status in plan['source_status']],
        'conflicts': plan['conflicts'], 'generated_at_unix_micros': -(1 << 63),
        'generator_identity_status': 'self_reported_not_independently_verified', 'semantic_approval_status': 'not_established'}
    return len(compact_bytes(retained)) - len(b'null')


def api():
    return document_realization.campaign_api()


def timestamp():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def binding(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def entry_path(root, identity):
    if api().re.fullmatch(r'[0-9a-f]{32}', identity or '') is None:
        raise api().CampaignError('invalid explanation evidence identity')
    return root / 'explanations' / identity


def bound(root, path):
    return document_realization.bound_bytes(root, path)


def invoke(binary, runtime, project, args):
    runtime = __import__("collection_runs").runtime_path(runtime)
    argv = [str(binary), '--runtime', str(runtime), '--project', project, '--json', *args]
    started = timestamp()
    import time
    start = time.monotonic_ns()
    try:
        completed = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True, timeout=60, check=False)
        stdout, stderr, code, termination = completed.stdout, completed.stderr, completed.returncode, 'exited' if completed.returncode >= 0 else 'signal'
    except subprocess.TimeoutExpired as error:
        stdout, stderr, code, termination = error.stdout or b'', error.stderr or b'', None, 'timeout'
    duration = (time.monotonic_ns() - start) / 1_000_000
    meta = {'started_at': started, 'ended_at': timestamp(), 'duration_ms': duration,
        'exit_code': code, 'termination': termination,
        'stdout': binding(stdout), 'stderr': binding(stderr),
        'command': args[:], 'candidate_executable_sha256': api().harness.sha256(binary)}
    __import__('collection_runs').record_product_read(meta, stdout, stderr)
    if termination != 'exited' or code != 0:
        # Failed process truth is retained privately, even before publication.
        destination = api().ROOT / 'rebuild/.local/explanation-process-failures' / secrets.token_hex(16)
        destination.mkdir(parents=True, mode=0o700)
        (destination / 'stdout').write_bytes(stdout)
        (destination / 'stderr').write_bytes(stderr)
        api().write_json(destination / 'result.json', meta)
        raise api().CampaignError('Product explanation command failed; private process evidence retained')
    if len(stdout) > 32 << 20 or len(stderr) > 1 << 20:
        raise api().CampaignError('explanation process output exceeds private bound')
    try:
        value = codex_events.strict_json(stdout.decode('utf-8'))
    except (ValueError, UnicodeError) as error:
        raise api().CampaignError('Product explanation command returned unresolvable JSON') from error
    return value, {'process.json': api().json_bytes(meta), 'stdout.json': stdout, 'stderr.txt': stderr}


def list_items(value):
    return value if isinstance(value, list) else []


def subject_answers(readback, subject):
    if subject['kind'] == 'decision':
        candidates = list_items(readback.get('decisions'))
        identity_key = 'identity'
    else:
        candidates = [readback.get('selected_work')] + [item for name in
            ('current_work', 'completed_work', 'remaining_work', 'work_history', 'next_steps')
            for item in list_items(readback.get(name))]
        identity_key = 'work_item_id'
    matches = [value for value in candidates if isinstance(value, dict)
        and value.get(identity_key) == subject['identity']]
    answers = [value.get('answers') for value in matches]
    if not answers:
        return {'state': 'unresolvable', 'answers': None, 'reason': 'subject_outside_returned_read_bound'}
    if any(answer != answers[0] for answer in answers) or not isinstance(answers[0], dict):
        raise api().CampaignError('Product shared-answer copies disagree or are malformed')
    return {'state': answers[0].get('explanation_state'), 'answers': answers[0], 'reason': None}


def read(binary, runtime, project, subject, language):
    return invoke(binary, runtime, project, ['decisions' if subject['kind'] == 'decision' else 'status', '--language', language])


def validate_plan(plan, project, subject, language):
    require = api().CampaignError
    if (not isinstance(plan, dict) or plan.get('project_id') != project or plan.get('subject') != {'kind': subject['kind'], 'identity': list(bytes.fromhex(subject['identity']))}
            or plan.get('requested_language') != language
            or api().re.fullmatch(r'sha256:[0-9a-f]{64}', plan.get('fingerprint', '')) is None
            or not isinstance(plan.get('evidence'), list) or not plan['evidence']):
        raise require('Product explanation plan subject/language/basis mismatch')
    reserve = retention_metadata_bytes(plan)
    expected_budget = {'response_byte_limit': RESPONSE_BYTE_LIMIT, 'retained_byte_limit': RETAINED_BYTE_LIMIT,
        'metadata_byte_reserve': reserve, 'response_byte_capacity': min(RESPONSE_BYTE_LIMIT, RETAINED_BYTE_LIMIT - reserve)}
    if (plan.get('retention_budget') != expected_budget or reserve + MIN_RESPONSE_BYTE_CAPACITY > RETAINED_BYTE_LIMIT
            or len(compact_bytes(plan)) > PLAN_BYTE_LIMIT):
        raise require('Product explanation plan retention/serialization budget mismatch; prepare current evidence before generation')
    evidence = plan['evidence']
    if (len({e.get('key') for e in evidence if isinstance(e, dict)}) != len(evidence)
            or any(not isinstance(e, dict) or not isinstance(e.get('sources'), list)
                or type(e.get('revision')) is not int or e['revision'] < 1
                or e.get('record_kind') not in {'context_item', 'checkpoint', 'decision', 'question'}
                or not isinstance(e.get('field'), str) for e in evidence)):
        raise require('Product explanation evidence keys/revisions are malformed')


def validate_response(plan, response):
    size = len(compact_bytes(response))
    capacity = plan['retention_budget']['response_byte_capacity']
    if size > capacity:
        raise api().CampaignError(f'explanation compact realization JSON is {size} bytes; prepared response capacity {capacity}; preserve grounding and seek Product support for this evidence shape')
    if (not isinstance(response, dict) or set(response) != {'format_kind', 'format_version',
            'plan_fingerprint', 'language', 'generator', 'paragraphs', 'diagrams'}
            or response['format_kind'] != 'volicord_explanation' or response['format_version'] != 8
            or response['plan_fingerprint'] != plan['fingerprint']
            or response['language'] != plan['requested_language']):
        raise api().CampaignError('explanation response format/plan/language mismatch')
    if not isinstance(response["diagrams"], list) or len(response["diagrams"]) > 4:
        raise api().CampaignError("invalid diagram collection")
    generator = response['generator']
    if (not isinstance(generator, dict) or set(generator) != {'host', 'session', 'agent', 'model'}
            or any(not isinstance(generator[k], str) or not generator[k].strip() for k in ('host', 'session'))
            or any(generator[k] is not None and (not isinstance(generator[k], str) or not generator[k].strip())
                for k in ('agent', 'model'))):
        raise api().CampaignError('invalid self-reported explanation generator')
    paragraphs, keys = response['paragraphs'], {e['key'] for e in plan['evidence']}
    allowed = ((WORK_QUESTIONS | {'observation', 'comparison', 'reading_point', 'flow_step'}) if plan['subject']['kind'] == 'work' else DECISION_QUESTIONS) | {'heading', 'synopsis', 'limits', 'section_heading', 'narrative', 'comparison_condition', 'comparison_result'}
    if (not isinstance(paragraphs, list) or not paragraphs
            or any(not isinstance(p, dict) or set(p) != {'question', 'text', 'evidence_keys', 'citations'}
                or p['question'] not in allowed or not isinstance(p['text'], str) or not p['text'].strip()
                or len(p['text']) > PARAGRAPH_CHARACTER_LIMIT
                or not isinstance(p['evidence_keys'], list) or not p['evidence_keys']
                or not all(isinstance(k, str) and k in keys for k in p['evidence_keys']) for p in paragraphs)
            or any(not isinstance(p.get('citations'), list) for p in paragraphs)):
        raise api().CampaignError('explanation required claims/grounding missing or malformed')

    for index, paragraph in enumerate(paragraphs):
        role = paragraph['question']
        if ((role == 'comparison_condition' and (index + 1 == len(paragraphs) or paragraphs[index + 1]['question'] != 'comparison_result'))
                or (role == 'comparison_result' and (index == 0 or paragraphs[index - 1]['question'] != 'comparison_condition'))):
            raise api().CampaignError('comparison requires adjacent condition/result fragments')
        if role in {'heading', 'section_heading'} and len(paragraph['text']) > 240:
            raise api().CampaignError('article heading exceeds Product bound')


def validate_lifecycle(preparation, response, receipt, after):
    """Source-independent linkage; hashes do not certify prose or authorship."""
    plan = preparation['plan']
    validate_plan(plan, preparation['project_id'], preparation['subject'], preparation['language'])
    validate_response(plan, response)
    retained = receipt.get('explanation')
    stripped = copy.deepcopy(plan['evidence'])
    for evidence in stripped:
        evidence['content'] = None
    statuses = [{k: v for k, v in value.items() if k != 'observation'} for value in plan['source_status']]
    if (receipt.get('operation') != 'explanation_record' or not isinstance(retained, dict)
            or retained.get('realization') != response or retained.get('project_id') != plan['project_id']
            or retained.get('subject') != plan['subject'] or retained.get('evidence') != stripped
            or retained.get('source_status') != statuses or retained.get('conflicts') != plan['conflicts']
            or retained.get('question') != plan['question']
            or retained.get('generator_identity_status') != 'self_reported_not_independently_verified'
            or retained.get('semantic_approval_status') != 'not_established'
            or type(retained.get('generated_at_unix_micros')) is not int):
        raise api().CampaignError('explanation record receipt subject/revision/Source/provenance mismatch')
    if len(compact_bytes(retained)) > RETAINED_BYTE_LIMIT:
        raise api().CampaignError('explanation retained envelope exceeds Product bound; no current readback proof')
    if after.get('project_id') != plan['project_id']:
        raise api().CampaignError('explanation readback Project mismatch')
    observation = subject_answers(after, preparation['subject'])
    answers = observation['answers']
    if observation['state'] != 'current' or not isinstance(answers, dict):
        raise api().CampaignError('recorded explanation readback is unavailable or non-current')
    provenance = answers.get('provenance')
    # Ordinary readback shows paragraph-cited references; the immutable receipt
    # still must retain the complete offered preparation basis checked above.
    cited_keys = {key for paragraph in response['paragraphs'] for key in paragraph['evidence_keys']}
    cited = [item for item in stripped if item['key'] in cited_keys]
    uncited_count = len(stripped) - len(cited)
    if (not isinstance(provenance, dict)
            or type(provenance.get('uncited_evidence_count')) is not int
            or provenance['uncited_evidence_count'] != uncited_count
            or any(provenance.get(k) != value for k, value in (
        ('project_id', plan['project_id']), ('subject', plan['subject']), ('question', plan['question']), ('language', response['language']),
        ('fingerprint', plan['fingerprint']), ('evidence', cited), ('source_status', statuses),
        ('conflicts', plan['conflicts']), ('generator', response['generator']),
        ('generator_identity_status', retained['generator_identity_status']),
        ('semantic_approval_status', 'not_established'),
        ('citations', [c for p in response['paragraphs'] for c in p['citations']]),
        ('generated_at_unix_micros', retained['generated_at_unix_micros'])))):
        raise api().CampaignError('explanation readback provenance differs from exact receipt')
    if answers.get('diagrams') != response['diagrams']:
        raise api().CampaignError('explanation readback changed admitted diagram meaning')
    prose = answers.get('prose')
    if not isinstance(prose, list) or len(prose) != len(response['paragraphs']):
        raise api().CampaignError('explanation returned claims omitted')
    for paragraph, returned in zip(response['paragraphs'], prose, strict=True):
        if (not isinstance(returned, dict) or returned.get('text') != paragraph['text']
                or returned.get('evidence_keys') != paragraph['evidence_keys']
                or returned.get('citations') != paragraph['citations']
                or returned.get('role') != 'generated_interpretation'
                or returned.get('question') != ''.join(word.title() for word in paragraph['question'].split('_'))):
            raise api().CampaignError('explanation readback changed recorded claims/role')
    return observation


def reader_applicability(plan, response=None, *, source_reads=(), verification_read=None, origin='post_session_replay',
                         host_authority='unknown', now_unix_micros=None):
    """Per-claim assessment inputs, never semantic approval or capture attestation.

    Called by preparation/inspection, after their existing candidate/hash checks.
    Optional reads must be actual public scoped responses, kept privately by the
    collector. This diagnostic never fetches, renews, executes or grants authority.
    """
    import time
    now = time.time_ns() // 1000 if now_unix_micros is None else now_unix_micros
    if origin not in {'historical_record', 'post_session_replay', 'authored_control', 'prospective'}:
        raise api().CampaignError('unknown reader evidence origin')
    if host_authority not in {'unknown', 'unavailable', 'available'}:
        raise api().CampaignError('unknown current host authority')
    def result(state, reason):
        return {'state': state, 'reason': reason}
    def identity(value):
        return bytes(value).hex() if isinstance(value, list) else value
    subject = plan['subject']
    work = identity(subject['identity']) if subject['kind'] == 'work' else None
    offered = {v['key']: v for v in plan['evidence']}
    claims = {'record_reading': result('eligible_for_assessment', 'exact_prepared_record_fields'),
        'article': result('not_observed', 'no_recorded_response_supplied'),
        'code_body_read': result('not_observed', 'no_scoped_file_body_read_supplied'),
        'two_observed_states': result('not_observed', 'missing_before_state'),
        'named_command': result('not_observed', 'no_named_execution_read_supplied'),
        'retest_coverage': result('not_observed', 'no_public_typed_verification_read_supplied'),
        'task_time_capture': result('blocked' if host_authority == 'unavailable' else 'not_established',
            'current_trusted_activation_or_source_authority_unavailable' if host_authority == 'unavailable'
            else 'origin_and_readback_do_not_attest_authenticated_task_time_capture'),
        'human_acceptance': result('pending', 'requires_separate_actual_reader_judgment')}
    for key, field in [('recommendation_reason', 'recommendation'), ('user_reason', 'user_rationale')]:
        item = offered.get(field)
        if subject['kind'] != 'decision':
            claims[key] = result('not_applicable', 'not_a_decision_subject')
        elif item is None:
            claims[key] = result('not_observed', 'field_not_in_supplied_plan')
        else:
            value = item['content'].get('rationale') if field == 'recommendation' and isinstance(item['content'], dict) else item['content']
            claims[key] = result('absent', 'optional_reason_not_recorded') if value is None else result(
                'eligible_for_assessment', 'recorded_reason_independent_of_supporting_source_availability')
    if response is not None:
        # Current format/bindings only. Historical bytes remain immutable and can
        # still supply record/presentation evidence without becoming a replay.
        if response.get('format_kind') != 'volicord_explanation' or response.get('format_version') != 8:
            claims['article'] = result('unsupported', 'non_current_explanation_format')
        else:
            validate_response(plan, response)
            claims['article'] = result('eligible_for_assessment', 'current_bound_response_not_semantic_approval')
    usable, blocked, seen = {}, {}, {}
    for read in source_reads:
        obs, meta = read.get('observation', {}), read.get('metadata', {})
        if (work is None or identity(obs.get('project_id')) != plan['project_id']
                or identity(obs.get('work_item_id')) != work
                or obs.get('format_kind') != 'volicord_source_observation' or obs.get('format_version') != 4
                or read.get('read_only') is not True
                or read.get('observation_fingerprint') != read.get('citation_basis', {}).get('observation_fingerprint')
                or identity(obs.get('source_id')) != read.get('citation_basis', {}).get('source_id')
                or obs.get('source_revision') != read.get('citation_basis', {}).get('source_revision')
                or any(obs.get(k) != read.get('citation_basis', {}).get(k)
                    for k in ('analysis_snapshot_id', 'repository_snapshot_id'))
                or type(obs.get('work_revision')) is not int):
            raise api().CampaignError('reader Source scope/format/binding mismatch')
        goal = offered.get('goal')
        if goal and obs['work_revision'] != goal['revision']:
            raise api().CampaignError('reader Source Work revision mismatch')
        source = identity(obs['source_id'])
        # One assessment has one read basis. Never let input order choose an
        # available copy over an expired/corrected copy of the same Source.
        # Separate observations belong to separate candidate-bound assessments.
        encoded = compact_bytes(read)
        if source in seen and seen[source] != encoded:
            raise api().CampaignError('conflicting reader Source responses; assess each read basis separately')
        seen[source] = encoded
        until = meta.get('retained_until_unix_micros')
        if type(until) is not int or until <= now or meta.get('retained_state') != 'available':
            blocked[source] = 'expired_retention' if type(until) is int and until <= now else meta.get('retained_state', 'source_body_inaccessible')
            continue
        usable[source] = read
    files = [r for r in usable.values() if r['observation']['detail'].get('kind') == 'file'
        and isinstance(r['observation']['detail'].get('body'), str)]
    if files:
        claims['code_body_read'] = result('eligible_for_assessment', 'permitted_observed_file_bytes_not_causal_proof')
    elif blocked:
        claims['code_body_read'] = result('inaccessible', next(iter(blocked.values())))
    comparison_gaps = []
    for read in sorted(files, key=lambda r: identity(r['observation']['source_id'])):
        after = read['observation']; detail = after['detail']; before = usable.get(detail.get('before_source_id'))
        if before is None:
            if detail.get('before_source_id'):
                reason = blocked.get(detail['before_source_id'], 'missing_before_inspection')
                comparison_gaps.append({'source_id': identity(after['source_id']),
                    **result('inaccessible' if detail['before_source_id'] in blocked else 'not_observed', reason)})
            else:
                comparison_gaps.append({'source_id': identity(after['source_id']),
                    **result('not_observed', 'missing_before_state')})
            continue
        prior = before['observation']; old = prior['detail']
        hashes_present = all(isinstance(v, str) and api().re.fullmatch(r'[0-9a-f]{64}', v)
            for v in (old.get('sha256'), detail.get('sha256')))
        expected = 'no_change' if old.get('sha256') == detail.get('sha256') else 'changed'
        if (old.get('kind') == 'file' and old.get('path') == detail.get('path')
                and old.get('representation') == detail.get('representation')
                and all((old.get('region') or {}).get(k) == (detail.get('region') or {}).get(k)
                    for k in ('start_byte', 'end_byte'))
                and prior['observed_at_unix_micros'] < after['observed_at_unix_micros']
                and hashes_present and detail.get('comparison') == expected):
            claims['two_observed_states'] = result('eligible_for_assessment', 'explicit_same_work_path_region_pair_not_actor_attribution')
        else:
            comparison_gaps.append({'source_id': identity(after['source_id']),
                **result('not_observed', 'inconsistent_observed_comparison')})
    if comparison_gaps:
        if claims['two_observed_states']['state'] != 'eligible_for_assessment':
            # An initial unpaired observation cannot conceal an unusable
            # explicitly claimed comparison in this same case.
            gap = next((g for g in comparison_gaps if g['reason'] != 'missing_before_state'), comparison_gaps[0])
            claims['two_observed_states'] = {k: gap[k] for k in ('state', 'reason')}
        claims['two_observed_states']['gaps'] = comparison_gaps
    for read in usable.values():
        detail, command = read['observation']['detail'], read.get('command_source')
        if (detail.get('kind') == 'command' and isinstance(command, dict) and command.get('command_label')
                and detail.get('operation_id') and type(detail.get('started_at_unix_micros')) is int
                and type(detail.get('ended_at_unix_micros')) is int
                and detail['started_at_unix_micros'] <= detail['ended_at_unix_micros']
                and command.get('exit_code') == detail.get('exit_code')
                and command.get('termination') == detail.get('termination')):
            claims['named_command'] = result('eligible_for_assessment', 'observed_named_outcome_not_assertion_coverage_or_retest')
    if verification_read is not None:
        claims['retest_coverage'] = verification_applicability(plan, verification_read)
    return {'project_id': plan['project_id'], 'subject': subject, 'plan_fingerprint': plan['fingerprint'],
        'origin': origin, 'host_authority': host_authority, 'claims': claims,
        'input_bindings': {'plan': binding(compact_bytes(plan)),
            'response': None if response is None else binding(compact_bytes(response)),
            'verification_read': None if verification_read is None else binding(compact_bytes(verification_read)),
            'source_reads': [binding(compact_bytes(v)) for v in source_reads]},
        'qualification': 'not_established', 'source_lifetime_recheck': 'required_at_every_product_read'}


def verification_applicability(plan, read):
    """Assess the existing public comparison, never recompute Product semantics.

    The supplied receipt still needs collector/candidate authentication. Canonical
    comparisons survive body expiry; eligibility grants no original-stream access.
    Keep every unresolved/non-comparable row alongside an eligible comparison.
    """
    work = plan['subject']['identity']
    work = bytes(work).hex() if isinstance(work, list) else work
    report = read.get('verification_report', {})
    if (plan['subject']['kind'] != 'work' or read.get('operation') != 'work_verification_read'
            or read.get('project_id') != plan['project_id'] or read.get('work_item_id') != work
            or read.get('read_only') is not True or not isinstance(report, dict)
            or not isinstance(report.get('checks'), list) or not isinstance(report.get('executions'), list)):
        raise api().CampaignError('reader verification scope/public receipt mismatch')
    rows = []
    originals = {}
    for check in report['checks']:
        original = check.get('original', {})
        if (set(original) != {'checkpoint_id', 'checkpoint_revision', 'verification_index'}
                or not isinstance(original['checkpoint_id'], str)
                or type(original['checkpoint_revision']) is not int or original['checkpoint_revision'] < 1
                or type(original['verification_index']) is not int or original['verification_index'] < 0):
            raise api().CampaignError('reader verification original identity malformed')
        key = compact_bytes(original)
        if key in originals:
            raise api().CampaignError('reader verification duplicate original identity')
        originals[key] = check
    for check in report['checks']:
        comparison = check.get('retest_comparison', {})
        state = comparison.get('state')
        if state not in {'unresolved', 'not_comparable', 'comparable'}:
            raise api().CampaignError('reader verification comparison state malformed')
        if state == 'comparable':
            prior = originals.get(compact_bytes(comparison.get('previous')))
            if prior is None or prior is check or comparison.get('source_id') != prior.get('source_id'):
                raise api().CampaignError('reader verification prior comparison identity mismatch')
            for row in (prior, check):
                scoped = row.get('scoped_run', {})
                run = scoped.get('value', {})
                if (scoped.get('state') != 'observed' or run.get('work_item_id') != work
                        or run.get('work_revision') != next((e['revision'] for e in plan['evidence'] if e['key'] == 'goal'), None)):
                    raise api().CampaignError('reader verification scoped Work/revision mismatch')
        rows.append({'original': check['original'], 'state': state,
            'reason': comparison.get('reason')})
    return {'state': 'eligible_for_assessment' if any(r['state'] == 'comparable' for r in rows) else 'not_observed',
        'reason': 'public_typed_comparison_not_capture_or_semantic_approval' if any(r['state'] == 'comparable' for r in rows)
            else 'no_comparable_relation_in_public_report', 'comparisons': rows}


def preparations(root):
    inventory = api().load_inventory(root)['artifacts']
    return [root / name for name in sorted(inventory)
        if name.startswith('explanations/') and name.endswith('/preparation.json')]


# Scope includes the complete frozen input identity; a different locale/subject
# is never a predecessor or a fallback publication candidate.
SCOPE_FIELDS = ('evidence_purpose', 'candidate_head', 'candidate_executable_sha256',
    'journey_id', 'project_id', 'subject', 'language', 'raw_inputs')


def scope(value):
    return api().json_bytes({key: value[key] for key in SCOPE_FIELDS})


def ordered_attempts(root):
    """Verify append-only causal links, without filename/time/ID ordering."""
    inventory = api().load_inventory(root)['artifacts']
    paths = [root / name for name in inventory
        if name.startswith('explanations/') and name.endswith('/attempt.json')]
    values = {}
    for path in paths:
        data = bound(root, path)
        value = json.loads(data)
        if (set(value) != {'kind', 'schema_version', 'identity', 'previous', *SCOPE_FIELDS}
                or value['kind'] != 'dogfood_explanation_attempt' or value['schema_version'] != SCHEMA_VERSION
                or entry_path(root, value['identity']) != path.parent):
            raise api().CampaignError('explanation attempt identity/schema changed')
        values[value['identity']] = (value, binding(data))
    groups = {}
    for value, digest in values.values():
        groups.setdefault(scope(value), []).append((value, digest))
    ordered = []
    for group in groups.values():
        following = {}
        for value, digest in group:
            previous = value['previous']
            if previous is not None:
                if (not isinstance(previous, dict) or set(previous) != {'identity', 'attempt'}
                        or previous['identity'] not in values):
                    raise api().CampaignError('explanation predecessor missing')
                prior, prior_digest = values[previous['identity']]
                if prior_digest != previous['attempt'] or scope(prior) != scope(value):
                    raise api().CampaignError('explanation predecessor binding/scope changed')
            key = previous['identity'] if previous else None
            if key in following:
                raise api().CampaignError('explanation observation relationship forks')
            following[key] = value
        chain, key = [], None
        while key in following:
            value = following[key]
            chain.append(value)
            key = value['identity']
            if len(chain) > len(group):
                raise api().CampaignError('explanation observation relationship cycles')
        if len(chain) != len(group):
            raise api().CampaignError('explanation observation relationship disconnected')
        ordered.append(chain)
    prepared_ids = {path.parent.name for path in preparations(root)}
    if not prepared_ids <= set(values):
        raise api().CampaignError('explanation preparation has no recorded attempt')
    return ordered


def declare_attempt(root, context):
    chains = ordered_attempts(root)
    same = [chain for chain in chains if scope(chain[0]) == scope(context)]
    previous = same[0][-1] if same else None
    attempt = {'kind': 'dogfood_explanation_attempt', 'schema_version': SCHEMA_VERSION,
        'identity': context['identity'], **{k: context[k] for k in SCOPE_FIELDS},
        'previous': {'identity': previous['identity'], 'attempt': binding(bound(root,
            entry_path(root, previous['identity']) / 'attempt.json'))} if previous else None}
    # An obligation exists before Product preparation: failure cannot disappear
    # or select an older successful observation. No abandon/fallback operation.
    document_realization.publish(root, {entry_path(root, context['identity']) / 'attempt.json': api().json_bytes(attempt)})
    return {'attempt': binding(api().json_bytes(attempt)), 'previous': attempt['previous']}


def publication_relations(root):
    relations = {}
    for chain in ordered_attempts(root):
        for number, attempt in enumerate(chain):
            path = entry_path(root, attempt['identity']) / 'preparation.json'
            if not path.exists():
                raise api().CampaignError('explanation attempt missing host response/preparation (pending or failed)')
            verify(root, path)  # Every retained obligation must be completed.
            relations[attempt['identity']] = {
                'publication_role': 'final' if number == len(chain) - 1 else 'historical',
                'selected_identity': chain[-1]['identity']}
    return relations


def prepare(root, raw_paths, *, languages=None, work_ids=None, decision_ids=None):
    c = api()
    campaign = c.load_campaign_for_mutation(root)
    c.verify_frozen_campaign(root, campaign)
    if campaign.get('collection_state') == 'collected' or (root / 'realization-bindings.json').exists():
        raise c.CampaignError('prepare explanations before document realization and final collection')
    mapped = c.map_batch_rollouts(root, raw_paths)
    languages = list(dict.fromkeys(languages or [campaign['document_language']]))
    files, index = {}, []
    for journey_id, journey in sorted(campaign['journeys'].items()):
        kind, runtime = journey['repository_class'], Path(journey['runtime_home'])
        projects = c.observed_project_ids(mapped[(kind, 'A', 'start')].capture)
        if len(projects) != 1:
            raise c.CampaignError('explanation Project cannot be resolved from measured sessions')
        project = projects[0]
        binary = Path(campaign['candidate_binary'])
        with tempfile.TemporaryDirectory() as temporary:
            bundle_path = Path(temporary) / 'context.json'
            with c.candidate_artifact_use(campaign, ('volicord',)):
                c.default_export(binary, runtime, Path(journey['repository_path']), bundle_path)
            canonical = c.harness.load_canonical_bundle(bundle_path)
        if canonical.project_id != project:
            raise c.CampaignError('explanation canonical Project differs from raw sessions')
        subjects = [('work', value['id']) for value in canonical.rows('context_items')
            if value.get('project_id') == project and value.get('role') == 'goal'
            and (work_ids is None or value['id'] in work_ids)]
        subjects += [('decision', value['id']) for value in canonical.rows('decisions')
            if value.get('project_id') == project and (decision_ids is None or value['id'] in decision_ids)]
        if work_ids is not None and decision_ids is None:
            subjects = [s for s in subjects if s[0] == 'work']
        if decision_ids is not None and work_ids is None:
            subjects = [s for s in subjects if s[0] == 'decision']
        for subject_kind, subject_id in subjects:
            for language in languages:
                identity = secrets.token_hex(16)
                directory = entry_path(root, identity)
                subject = {'kind': subject_kind, 'identity': subject_id}
                context = {'identity': identity, 'evidence_purpose': campaign['evidence_purpose'],
                    'candidate_head': campaign['candidate_head'],
                    'candidate_executable_sha256': campaign['candidate_artifacts']['volicord']['sha256'],
                    'journey_id': journey_id, 'project_id': project, 'subject': subject, 'language': language,
                    'raw_inputs': document_realization.raw_binding(mapped)}
                observation_order = declare_attempt(root, context)
                with c.candidate_artifact_use(campaign, ('volicord',)):
                    before, before_process = read(binary, runtime, project, subject, language)
                    result, plan_process = invoke(binary, runtime, project,
                        [subject_kind, 'explain', 'prepare', '--' + subject_kind, subject_id, '--language', language])
                if before.get('project_id') != project:
                    raise c.CampaignError('explanation before-read Project mismatch')
                plan = result.get('plan')
                validate_plan(plan, project, subject, language)
                preparation = {'kind': 'dogfood_explanation_preparation', 'schema_version': SCHEMA_VERSION,
                    'identity': identity, 'evidence_purpose': campaign['evidence_purpose'], 'candidate_head': campaign['candidate_head'],
                    'candidate_executable_sha256': campaign['candidate_artifacts']['volicord']['sha256'],
                    'journey_id': journey_id, 'project_id': project, 'subject': subject, 'language': language,
                    'observation_order': observation_order,
                    'phase': 'post_session_steward', 'observed_at': timestamp(),
                    'raw_inputs': document_realization.raw_binding(mapped),
                    'before_observation': subject_answers(before, subject), 'plan': plan,
                    'canonical_bundle_sha256': canonical.source_sha256,
                    'generation_authority': ('self_authored_support_no_provider_dispatch' if campaign['evidence_purpose'] == evidence_purpose.REHEARSAL else 'current_active_host_interaction_required_no_provider_dispatch'),
                    'generator_identity_limit': 'self_reported_not_independently_verified'}
                files[directory / 'preparation.json'] = c.json_bytes(preparation)
                files[directory / 'before.json'] = c.json_bytes(before)
                for phase, process in (('before-process', before_process), ('prepare-process', plan_process)):
                    files.update({directory / phase / name: value for name, value in process.items()})
                document_realization.publish(root, files)
                files = {}
                index.append({'identity': identity, 'subject': subject, 'language': language,
                    'preparation': c.relative(root, directory / 'preparation.json'),
                    'reader_applicability': reader_applicability(plan)})
    if not index:
        raise c.CampaignError('no requested explanation subjects found in measured Projects')
    selected = {item['subject']['identity'] for item in index}
    if not set((work_ids or []) + (decision_ids or [])) <= selected:
        raise c.CampaignError('requested explanation subject is not in measured Projects')
    if any(c.harness.sha256(value.source) != value.capture.source_sha256 for value in mapped.values()):
        raise c.CampaignError('raw inputs changed during explanation preparation')
    return {'state': 'prepared', 'explanations': index, 'qualification_state': 'not_run'}


def record(root, identity, input_path):
    c = api()
    campaign = c.load_campaign_for_mutation(root)
    c.verify_inventory(root)
    if campaign.get('collection_state') == 'collected' or (root / 'realization-bindings.json').exists():
        raise c.CampaignError('record explanations before document realization and final collection')
    directory = entry_path(root, identity)
    preparation_bytes = bound(root, directory / 'preparation.json')
    preparation = json.loads(preparation_bytes)
    evidence_purpose.require_same(campaign, preparation)
    if preparation['candidate_head'] != campaign['candidate_head'] or preparation['candidate_executable_sha256'] != campaign['candidate_artifacts']['volicord']['sha256']:
        raise c.CampaignError('explanation candidate binding changed')
    chains = ordered_attempts(root)
    chain = next((chain for chain in chains if scope(chain[0]) == scope(preparation)), [])
    if not chain or chain[-1]['identity'] != identity:
        raise c.CampaignError('only the latest explanation attempt can be recorded; older obligation remains unresolved')
    if (directory / 'receipt.json').exists():
        raise c.CampaignError('explanation receipt is immutable; prepare a new observation to regenerate')
    if input_path.is_relative_to(root) and c.relative(root, input_path) in c.load_inventory(root)['artifacts']:
        raise c.CampaignError('inventory-bound response is not mutable host input')
    with input_path.open('rb') as stream:
        data = stream.read(INPUT_BYTE_LIMIT + 1)
    if len(data) > INPUT_BYTE_LIMIT:
        raise c.CampaignError(f'explanation input file exceeds {INPUT_BYTE_LIMIT} bytes (read at least {len(data)}); remove formatting whitespace or reduce response, preserve grounding, then retry')
    response = codex_events.strict_json(data.decode())
    validate_response(preparation['plan'], response)
    journey = campaign['journeys'][preparation['journey_id']]
    binary, runtime = Path(campaign['candidate_binary']), Path(journey['runtime_home'])
    subject, language, project = preparation['subject'], preparation['language'], preparation['project_id']
    with c.candidate_artifact_use(campaign, ('volicord',)):
        plan_result, _ = invoke(binary, runtime, project,
            [subject['kind'], 'explain', 'prepare', '--' + subject['kind'], subject['identity'], '--language', language])
        if plan_result.get('plan') != preparation['plan']:
            raise c.CampaignError('explanation evidence basis changed; prepare a new observation')
        # Invoke using the exact frozen response bytes, never a mutable path race.
        with tempfile.TemporaryDirectory() as temporary:
            response_path = Path(temporary) / 'response.json'
            response_path.write_bytes(data)
            result, record_process = invoke(binary, runtime, project, [subject['kind'], 'explain', 'record',
                '--' + subject['kind'], subject['identity'], '--language', language, '--input', str(response_path)])
        after, read_process = read(binary, runtime, project, subject, language)
    observation = validate_lifecycle(preparation, response, result, after)
    receipt = {'kind': 'dogfood_explanation_receipt', 'schema_version': SCHEMA_VERSION,
        'identity': identity, 'observation_order': preparation['observation_order'],
        'phase': 'post_session_steward', 'observed_at': timestamp(),
        'preparation': binding(preparation_bytes), 'response': binding(data),
        'record': binding(c.json_bytes(result)), 'readback': binding(c.json_bytes(after)),
        'evidence_purpose': preparation['evidence_purpose'], 'candidate_head': preparation['candidate_head'], 'candidate_executable_sha256': preparation['candidate_executable_sha256'],
        'generator_identity_status': 'self_reported_not_independently_verified',
        'host_response_locator': {'kind': 'submitted_response_file', 'sha256': binding(data)['sha256'],
            'session': response['generator']['session'], 'raw_capture_sha256': None, 'turn_id': None,
            'limit': 'no independent response authorship or host-turn attestation'},
        'after_state': observation['state']}
    files = {directory / 'response.json': data, directory / 'record.json': c.json_bytes(result),
        directory / 'after.json': c.json_bytes(after), directory / 'receipt.json': c.json_bytes(receipt)}
    for phase, process in (('record-process', record_process), ('readback-process', read_process)):
        files.update({directory / phase / name: value for name, value in process.items()})
    document_realization.publish(root, files)
    return {'state': 'recorded', 'identity': identity, 'receipt': c.relative(root, directory / 'receipt.json'),
        'qualification_state': 'not_run'}


def verify(root, preparation_path, *, allow_unrecorded=False):
    preparation_bytes = bound(root, preparation_path)
    preparation = json.loads(preparation_bytes)
    directory = preparation_path.parent
    if (preparation.get('kind') != 'dogfood_explanation_preparation' or preparation.get('schema_version') != SCHEMA_VERSION
            or preparation.get('identity') != directory.name or preparation.get('phase') != 'post_session_steward'
            or preparation.get('generator_identity_limit') != 'self_reported_not_independently_verified'):
        raise api().CampaignError('explanation preparation identity/phase/provenance changed')
    attempt_bytes = bound(root, directory / 'attempt.json')
    attempt = json.loads(attempt_bytes)
    if (attempt.get('identity') != preparation['identity'] or scope(attempt) != scope(preparation)
            or preparation.get('observation_order') != {'attempt': binding(attempt_bytes), 'previous': attempt.get('previous')}):
        raise api().CampaignError('explanation preparation attempt/order changed')
    validate_plan(preparation['plan'], preparation['project_id'], preparation['subject'], preparation['language'])
    before = json.loads(bound(root, directory / 'before.json'))
    if before.get('project_id') != preparation['project_id'] or subject_answers(before, preparation['subject']) != preparation['before_observation']:
        raise api().CampaignError('explanation earlier observation changed')
    if not (directory / 'receipt.json').exists():
        if allow_unrecorded:
            return preparation, None
        raise api().CampaignError('prepared explanation is missing host response/record receipt/readback')
    receipt = json.loads(bound(root, directory / 'receipt.json'))
    evidence_purpose.require_same(preparation, receipt)
    values = {name: bound(root, directory / (name + '.json')) for name in ('response', 'record', 'after')}
    if (receipt.get('preparation') != binding(preparation_bytes) or receipt.get('response') != binding(values['response'])
            or receipt.get('record') != binding(values['record']) or receipt.get('readback') != binding(values['after'])
            or receipt.get('kind') != 'dogfood_explanation_receipt' or receipt.get('schema_version') != SCHEMA_VERSION
            or receipt.get('observation_order') != preparation['observation_order']
            or receipt.get('identity') != preparation['identity'] or receipt.get('phase') != 'post_session_steward'
            or receipt.get('candidate_head') != preparation['candidate_head']
            or receipt.get('candidate_executable_sha256') != preparation['candidate_executable_sha256']
            or receipt.get('generator_identity_status') != 'self_reported_not_independently_verified'):
        raise api().CampaignError('explanation receipt linkage changed')
    started = dt.datetime.fromisoformat(preparation['observed_at'])
    ended = dt.datetime.fromisoformat(receipt['observed_at'])
    if started.tzinfo is None or ended.tzinfo is None or started > ended:
        raise api().CampaignError('explanation observation order changed')
    response = json.loads(values['response'])
    expected_locator = {'kind': 'submitted_response_file', 'sha256': binding(values['response'])['sha256'],
        'session': response['generator']['session'], 'raw_capture_sha256': None, 'turn_id': None,
        'limit': 'no independent response authorship or host-turn attestation'}
    if receipt.get('host_response_locator') != expected_locator or receipt.get('after_state') != 'current':
        raise api().CampaignError('explanation response locator/provenance assertion changed')
    validate_lifecycle(preparation, response, json.loads(values['record']), json.loads(values['after']))
    return preparation, receipt


def require_ready(root, campaign, mapped):
    relations = publication_relations(root)
    for path in preparations(root):
        preparation, receipt = verify(root, path)
        evidence_purpose.require_same(campaign, preparation)
        if (preparation['raw_inputs'] != document_realization.raw_binding(mapped) or preparation['candidate_head'] != campaign['candidate_head']
                or preparation['candidate_executable_sha256'] != campaign['candidate_artifacts']['volicord']['sha256']
                or preparation['journey_id'] not in campaign['journeys']):
            raise api().CampaignError('explanation preparation does not bind current raw inputs/candidate')
        if relations[preparation['identity']]['publication_role'] != 'final':
            continue
        # Only the selected publication must still match the current Product basis.
        journey = campaign['journeys'][preparation['journey_id']]
        binary = Path(campaign['candidate_binary'])
        with api().candidate_artifact_use(campaign, ('volicord',)):
            result, _ = invoke(binary, Path(journey['runtime_home']), preparation['project_id'],
                [preparation['subject']['kind'], 'explain', 'prepare', '--' + preparation['subject']['kind'],
                    preparation['subject']['identity'], '--language', preparation['language']])
            if result.get('plan') != preparation['plan']:
                raise api().CampaignError('recorded explanation basis changed before collection')
            current, _ = read(binary, Path(journey['runtime_home']), preparation['project_id'],
                preparation['subject'], preparation['language'])
        validate_lifecycle(preparation, json.loads(bound(root, path.parent / 'response.json')),
            json.loads(bound(root, path.parent / 'record.json')), current)
    return relations


def inspect(root):
    values = []
    for chain in ordered_attempts(root):
        for attempt in chain:
            path = entry_path(root, attempt['identity']) / 'preparation.json'
            preparation, receipt = verify(root, path, allow_unrecorded=True) if path.exists() else (None, None)
            values.append({'identity': attempt['identity'], 'subject': attempt['subject'], 'language': attempt['language'],
                'state': 'recorded' if receipt else 'prepared_unrecorded' if preparation else 'pending_or_failed_preparation',
                'selected_identity': chain[-1]['identity'], 'previous': attempt['previous'],
                'phase': 'post_session_steward',
                'reader_applicability': None if preparation is None else reader_applicability(preparation['plan'],
                    json.loads(bound(root, path.parent / 'response.json')) if receipt else None)})
    return {'state': 'inspected', 'mutation': 'none', 'explanations': values}


def measured_cli_operations(capture):
    """Supported Product CLI JSON, with exact raw execution/turn coordinates."""
    values = []
    for command in capture.commands:
        argvs = codex_events.command_argvs(command.parsed_command)
        candidates = [argv for argv in argvs if Path(argv[0]).name == 'volicord'
            and not set(argv[1:]) & {'--help', '-h', '--version', '-V'}]
        if not candidates:
            if codex_events.command_role(command.parsed_command) == 'unknown':
                values.append({'transport': 'cli', 'call_id': command.execution_identity,
                    'raw_call_id': command.raw_call_id, 'turn_id': command.turn_id,
                    'sequence': command.sequence, 'completion_sequence': command.completion_sequence,
                    'group_index': command.group_index, 'operation': 'execution_observation',
                    'requested_language': 'en', 'result': None, 'exit_code': command.exit_code,
                    'termination': command.termination, 'signal_number': command.signal_number,
                    'evidence_state': command.evidence_state,
                    'output_state': command.output_state,
                    'execution_scope': 'whole_shell_invocation',
                    'limit': 'inner_command_meaning_unresolvable'})
            continue
        # Multiple shell statements cannot share one successful Product receipt.
        # Retain a safely identified operation, with no inferred inner outcome.
        for argv in candidates:
            values.extend(_measured_cli_operation(command, argv, len(argvs) == 1))
    for wrapper in capture.execution_wrappers:
        if wrapper.state == 'normalized':
            continue
        values.append({'transport': 'cli', 'call_id': 'execution_wrapper:' + (wrapper.call_id or str(wrapper.sequence)),
            'raw_call_id': wrapper.call_id, 'turn_id': wrapper.turn_id,
            'sequence': wrapper.sequence, 'completion_sequence': wrapper.completion_sequence,
            'operation': 'execution_observation', 'requested_language': 'en', 'result': None,
            'evidence_state': wrapper.state, 'exit_code': None, 'termination': None,
            'execution_scope': 'unresolved_wrapper', 'coverage': dict(vars(wrapper)),
            'limit': 'execution_wrapper_not_normalized'})
    # This projection is compared to the immutable JSON collection index later.
    return codex_events.strict_json(codex_events.canonical_json(sorted(values,
        key=lambda value: (value['sequence'], value.get('group_index', -1)))).decode('utf-8'))


def _measured_cli_operation(command, argv, unique):
    expected = next(('explanation_' + argv[n + 2] for n, arg in enumerate(argv[:-2])
        if arg in {'work', 'decision'} and argv[n + 1] == 'explain' and argv[n + 2] in {'prepare', 'record'}), None)
    if expected is None:
        expected = 'project_status' if 'status' in argv else 'decisions' if 'decisions' in argv else None
    if expected is None:
        return []
    result = None
    if unique and '--json' in argv and command.exit_code == 0 and command.evidence_state == 'completed' and command.termination == 'exited':
        try:
            result = codex_events.strict_json(command.output)
        except (ValueError, UnicodeError):
            pass
    return [{'transport': 'cli', 'call_id': command.execution_identity, 'turn_id': command.turn_id,
        'sequence': command.sequence, 'completion_sequence': command.completion_sequence,
        'raw_call_id': command.raw_call_id, 'group_index': command.group_index,
        'continuation_coordinates': command.continuation_coordinates,
        'exit_code': command.exit_code, 'termination': command.termination,
        'signal_number': command.signal_number, 'evidence_state': command.evidence_state,
        'output_state': command.output_state,
        'execution_scope': 'whole_shell_invocation',
        'limit': ('compound_inner_outcome_unresolved' if not unique else 'json_output_not_requested'
            if '--json' not in argv else 'returned_json_unresolvable' if result is None else None),
        'operation': expected,
        'requested_language': argv[argv.index('--language') + 1] if '--language' in argv and argv.index('--language') + 1 < len(argv) else 'en',
        'result': result}]


def collection_index(root, mapped):
    import answer_projection
    measured = []
    for slot, value in sorted(mapped.items()):
        capture = value.capture
        returned_values = [dict(value, operation='recall') for value in answer_observations.returned_recalls(capture)]
        returned_values += measured_cli_operations(capture)
        for returned in sorted(returned_values, key=lambda value: value['sequence']):
            result = returned['result']
            common = {'phase': 'measured_session', 'session_slot_id': api().session_slot_id(*slot),
                # Request Project is an observation-time integrity input, not a
                # returned-meaning/lifecycle index field. Keep the existing
                # immutable index responsibility independent of that observer.
                'raw_capture_sha256': capture.source_sha256, **{k: v for k, v in returned.items()
                    if k not in {'result', 'requested_project'}},
                'returned_payload_sha256': binding(api().json_bytes(result))['sha256'],
                'review_meaning_sha256': binding(api().json_bytes(answer_projection.project(result, returned['operation'])))['sha256']
                    if returned['operation'] in answer_projection.SCHEMAS else None}
            if not isinstance(result, dict):
                measured.append(common | {'state': 'unresolvable', 'limit': returned.get('limit') or 'returned JSON unavailable'})
                continue
            if returned['operation'] in {'explanation_prepare', 'explanation_record'}:
                payload = result.get('plan') if returned['operation'] == 'explanation_prepare' else result.get('explanation')
                measured.append(common | {'state': 'returned' if isinstance(payload, dict) else 'unresolvable',
                    'project_id': payload.get('project_id') if isinstance(payload, dict) else None,
                    'subject': payload.get('subject') if isinstance(payload, dict) else None,
                    'payload_sha256': binding(api().json_bytes(payload))['sha256'],
                    'response_input_state': 'not_independently_captured',
                    'limit': 'returned record realization is Product receipt content; input file/authorship unverified'})
                continue
            selected = result.get('selected_work')
            subjects = [('work', selected)] if isinstance(selected, dict) else []
            subjects += [('decision', decision) for decision in list_items(result.get('decisions'))
                if isinstance(decision, dict)]
            if not subjects:
                measured.append(common | {'state': 'returned_no_subject_answer', 'project_id': result.get('project_id')})
            for kind, selected in subjects:
                answers = selected.get('answers')
                measured.append(common | {'project_id': result.get('project_id'),
                    'subject': {'kind': kind, 'identity': selected.get('work_item_id' if kind == 'work' else 'identity')},
                    'language': returned.get('requested_language'),
                    'explanation_state': answers.get('explanation_state') if isinstance(answers, dict) else 'unresolvable',
                    'returned_answer_sha256': binding(api().json_bytes(answers))['sha256'] if isinstance(answers, dict) else None})
    relations = publication_relations(root)
    steward = []
    for path in preparations(root):
        preparation, receipt = verify(root, path)
        import review_explanations
        projected, _ = review_explanations.project(root, path, evidence_set_sha256='0' * 64)
        review_value = json.loads(projected)
        steward.append({'identity': preparation['identity'], **relations[preparation['identity']],
            'observation_order': preparation['observation_order'], 'phase': 'post_session_steward',
            'subject': preparation['subject'], 'language': preparation['language'],
            'journey_id': preparation['journey_id'], 'project_id': preparation['project_id'],
            'before_state': preparation['before_observation']['state'], 'after_state': receipt['after_state'],
            'review_stage_bindings': review_explanations.stage_bindings(review_value),
            'readback_subject_locators': review_value['readback_subject_locators'],
            'preparation': api().relative(root, path), 'receipt': api().relative(root, path.parent / 'receipt.json')})
    return {'kind': 'dogfood_explanation_evidence', 'schema_version': SCHEMA_VERSION,
        'measured_observations': measured, 'steward_lifecycles': steward,
        'limits': ['steward generation does not establish earlier use, adoption or user experience',
            'generated prose and language adequacy require qualitative review; generator identity is unverified']}
