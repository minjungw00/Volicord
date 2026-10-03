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

SCHEMA_VERSION = 1
WORK_QUESTIONS = {'purpose', 'reported_change', 'expected_effect', 'verification', 'next_step'}
DECISION_QUESTIONS = {'user_rationale', 'recommendation', 'consequences', 'applicability'}


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
    evidence = plan['evidence']
    if (len({e.get('key') for e in evidence if isinstance(e, dict)}) != len(evidence)
            or any(not isinstance(e, dict) or not isinstance(e.get('sources'), list)
                or type(e.get('revision')) is not int or e['revision'] < 1
                or e.get('record_kind') not in {'context_item', 'checkpoint', 'decision'}
                or not isinstance(e.get('field'), str) for e in evidence)):
        raise require('Product explanation evidence keys/revisions are malformed')


def validate_response(plan, response):
    if (not isinstance(response, dict) or set(response) != {'format_kind', 'format_version',
            'plan_fingerprint', 'language', 'generator', 'paragraphs'}
            or response['format_kind'] != 'volicord_explanation' or response['format_version'] != 1
            or response['plan_fingerprint'] != plan['fingerprint']
            or response['language'] != plan['requested_language']):
        raise api().CampaignError('explanation response format/plan/language mismatch')
    generator = response['generator']
    if (not isinstance(generator, dict) or set(generator) != {'host', 'session', 'agent', 'model'}
            or any(not isinstance(generator[k], str) or not generator[k].strip() for k in ('host', 'session'))
            or any(generator[k] is not None and (not isinstance(generator[k], str) or not generator[k].strip())
                for k in ('agent', 'model'))):
        raise api().CampaignError('invalid self-reported explanation generator')
    paragraphs, keys = response['paragraphs'], {e['key'] for e in plan['evidence']}
    required = WORK_QUESTIONS if plan['subject']['kind'] == 'work' else DECISION_QUESTIONS
    if (not isinstance(paragraphs, list) or not required <= {p.get('question') for p in paragraphs if isinstance(p, dict)}
            or any(not isinstance(p, dict) or set(p) != {'question', 'text', 'evidence_keys'}
                or p['question'] not in required | {'limits'} or not isinstance(p['text'], str) or not p['text'].strip()
                or not isinstance(p['evidence_keys'], list) or not p['evidence_keys']
                or not all(isinstance(k, str) and k in keys for k in p['evidence_keys']) for p in paragraphs)
            or len({p['question'] for p in paragraphs}) != len(paragraphs)):
        raise api().CampaignError('explanation required claims/grounding missing or malformed')


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
            or type(retained.get('generated_at_unix_micros')) is not int):
        raise api().CampaignError('explanation record receipt subject/revision/Source/provenance mismatch')
    if after.get('project_id') != plan['project_id']:
        raise api().CampaignError('explanation readback Project mismatch')
    observation = subject_answers(after, preparation['subject'])
    answers = observation['answers']
    if observation['state'] != 'current' or not isinstance(answers, dict):
        raise api().CampaignError('recorded explanation readback is unavailable or non-current')
    provenance = answers.get('provenance')
    if not isinstance(provenance, dict) or any(provenance.get(k) != value for k, value in (
        ('project_id', plan['project_id']), ('subject', plan['subject']), ('question', plan['question']), ('language', response['language']),
        ('fingerprint', plan['fingerprint']), ('evidence', stripped), ('source_status', statuses),
        ('conflicts', plan['conflicts']), ('generator', response['generator']),
        ('generator_identity_status', retained['generator_identity_status']),
        ('generated_at_unix_micros', retained['generated_at_unix_micros']))):
        raise api().CampaignError('explanation readback provenance differs from exact receipt')
    prose = answers.get('prose')
    if not isinstance(prose, list) or len(prose) != len(response['paragraphs']):
        raise api().CampaignError('explanation returned claims omitted')
    for paragraph, returned in zip(response['paragraphs'], prose, strict=True):
        if (not isinstance(returned, dict) or returned.get('text') != paragraph['text']
                or returned.get('evidence_keys') != paragraph['evidence_keys']
                or returned.get('role') != 'generated_interpretation'
                or returned.get('question') != ''.join(word.title() for word in paragraph['question'].split('_'))):
            raise api().CampaignError('explanation readback changed recorded claims/role')
    return observation


def preparations(root):
    inventory = api().load_inventory(root)['artifacts']
    return [root / name for name in sorted(inventory)
        if name.startswith('explanations/') and name.endswith('/preparation.json')]


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
                index.append({'identity': identity, 'subject': subject, 'language': language,
                    'preparation': c.relative(root, directory / 'preparation.json')})
    if not index:
        raise c.CampaignError('no requested explanation subjects found in measured Projects')
    selected = {item['subject']['identity'] for item in index}
    if not set((work_ids or []) + (decision_ids or [])) <= selected:
        raise c.CampaignError('requested explanation subject is not in measured Projects')
    if any(c.harness.sha256(value.source) != value.capture.source_sha256 for value in mapped.values()):
        raise c.CampaignError('raw inputs changed during explanation preparation')
    document_realization.publish(root, files)
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
    if (directory / 'receipt.json').exists():
        raise c.CampaignError('explanation receipt is immutable; prepare a new observation to regenerate')
    if input_path.is_relative_to(root) and c.relative(root, input_path) in c.load_inventory(root)['artifacts']:
        raise c.CampaignError('inventory-bound response is not mutable host input')
    data = input_path.read_bytes()
    if len(data) > 16384:
        raise c.CampaignError('explanation host response exceeds Product bound')
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
        'identity': identity, 'phase': 'post_session_steward', 'observed_at': timestamp(),
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
    for path in preparations(root):
        preparation, receipt = verify(root, path)
        evidence_purpose.require_same(campaign, preparation)
        if preparation['raw_inputs'] != document_realization.raw_binding(mapped) or preparation['candidate_head'] != campaign['candidate_head']:
            raise api().CampaignError('explanation preparation does not bind current raw inputs/candidate')
        # Freshness must survive subsequent steward operations before collection.
        journey = campaign['journeys'][preparation['journey_id']]
        binary = Path(campaign['candidate_binary'])
        with api().candidate_artifact_use(campaign, ('volicord',)):
            result, _ = invoke(binary, Path(journey['runtime_home']), preparation['project_id'],
                [preparation['subject']['kind'], 'explain', 'prepare', '--' + preparation['subject']['kind'],
                    preparation['subject']['identity'], '--language', preparation['language']])
        if result.get('plan') != preparation['plan']:
            raise api().CampaignError('recorded explanation basis changed before collection')


def measured_cli_operations(capture):
    """Supported Product CLI JSON, with exact raw execution/turn coordinates."""
    values = []
    for command in capture.commands:
        argvs = codex_events.command_argvs(command.parsed_command)
        if len(argvs) != 1 or Path(argvs[0][0]).name != 'volicord' or '--json' not in argvs[0]:
            continue
        argv = argvs[0]
        expected = next(('explanation_' + argv[n + 2] for n, arg in enumerate(argv[:-2])
            if arg in {'work', 'decision'} and argv[n + 1] == 'explain' and argv[n + 2] in {'prepare', 'record'}), None)
        if expected is None:
            expected = 'project_status' if 'status' in argv else 'decisions' if 'decisions' in argv else None
        if expected is None:
            continue
        result = None
        if command.exit_code == 0 and command.evidence_state == 'completed':
            try:
                result = codex_events.strict_json(command.output)
            except (ValueError, UnicodeError):
                pass
        values.append({'transport': 'cli', 'call_id': command.execution_identity, 'turn_id': command.turn_id,
            'sequence': command.sequence, 'completion_sequence': command.completion_sequence,
            'operation': expected,
            'requested_language': argv[argv.index('--language') + 1] if '--language' in argv and argv.index('--language') + 1 < len(argv) else 'en',
            'result': result})
    return values


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
                'raw_capture_sha256': capture.source_sha256, **{k: v for k, v in returned.items() if k != 'result'},
                'returned_payload_sha256': binding(api().json_bytes(result))['sha256'],
                'review_meaning_sha256': binding(api().json_bytes(answer_projection.project(result, returned['operation'])))['sha256']}
            if not isinstance(result, dict):
                measured.append(common | {'state': 'unresolvable', 'limit': 'returned JSON unavailable'})
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
    steward = []
    for path in preparations(root):
        preparation, receipt = verify(root, path)
        import review_explanations
        projected, _ = review_explanations.project(root, path, evidence_set_sha256='0' * 64)
        review_value = json.loads(projected)
        steward.append({'identity': preparation['identity'], 'phase': 'post_session_steward',
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
