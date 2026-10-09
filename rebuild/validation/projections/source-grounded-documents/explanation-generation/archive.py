"""Read-only historical adapter using existing capture normalization and Git bytes.

This is diagnostic enrichment, not a Product producer or portable schema decoder.
No prototype, review, generated explanation or later upstream checkout is read.
"""
from __future__ import annotations

import argparse
import datetime as dt
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tomllib

from inputs import binding, digest, encoded, freeze, require

DOGFOOD = Path(__file__).resolve().parents[3] / 'dogfood'
sys.path.insert(0, str(DOGFOOD))
import codex_events  # Existing current archive/capture adapter, no legacy bridge.


def decode_patch(value):
    # Captured exec cells can contain a second JSON-string transport layer.
    # Decode that literal layer only; never evaluate captured JS or shell code.
    if value.startswith('*** Begin Patch\\n'):
        value = json.loads('"' + value.replace('"', '\\"') + '"')
    return value


def patch_literals(events):
    for index, event in enumerate(events):
        payload = event.get('payload', {})
        text = payload.get('input', '')
        if 'tools.apply_patch' not in text:
            continue
        # Extract only a literal argument. The surrounding code is never run.
        for match in re.finditer(r'tools\.apply_patch\(("(?:\\.|[^"\\])*")\)', text):
            literal = codex_events.JsLiteralParser(match[1]).parse()
            yield index + 1, payload['call_id'], decode_patch(literal)


def update_files(files, patch, repository):
    """Exact unique-context replay; ambiguous/unsupported hunks fail closed."""
    lines = patch.splitlines()
    require(lines[0] == '*** Begin Patch' and lines[-1] == '*** End Patch', 'unsupported patch')
    changed = []
    index = 1
    while index < len(lines) - 1:
        header = lines[index]
        require(header.startswith('*** Update File: '), 'only exact Update File replay supported')
        name = header.removeprefix('*** Update File: ')
        if Path(name).is_absolute():
            name = str(Path(name).relative_to(repository))
        require(name in files and '..' not in Path(name).parts, 'patch outside baseline inventory')
        body = files[name].decode('utf-8').splitlines(keepends=True)
        require(all(line.endswith('\n') for line in body), 'unsupported unterminated source')
        text = ''.join(body)
        index += 1
        while index < len(lines) - 1 and not lines[index].startswith('*** '):
            require(lines[index].startswith('@@'), 'patch hunk marker missing')
            index += 1
            before, after = [], []
            while index < len(lines) - 1 and not lines[index].startswith(('@@', '*** ')):
                line = lines[index]
                require(line and line[0] in ' +-', 'unsupported patch hunk')
                if line[0] in ' -':
                    before.append(line[1:] + '\n')
                if line[0] in ' +':
                    after.append(line[1:] + '\n')
                index += 1
            old, new = ''.join(before), ''.join(after)
            require(old and text.count(old) == 1, 'ambiguous or absent patch context')
            text = text.replace(old, new, 1)
        files[name] = text.encode('utf-8')
        changed.append(name)
    return changed


def baseline(repository, revision):
    result = subprocess.run(['git', '--no-optional-locks', '-C', str(repository),
                             'archive', revision], capture_output=True, timeout=60, check=True)
    files = {}
    with tarfile.open(fileobj=io.BytesIO(result.stdout)) as archive:
        for member in archive:
            require(not member.name.startswith('/') and '..' not in Path(member.name).parts,
                    'unsafe Git archive path')
            if member.isfile():
                files[member.name] = archive.extractfile(member).read()
    return files


def canonical_records(bundle, project, work, cutoff_micros):
    """Select by explicit identities/relations, never path or commit proximity."""
    tables = {table['name']: [dict(zip(table['columns'], (cell.get('value') for cell in row)))
                             for row in table['rows']] for table in bundle['payload']['tables']}
    selected = {}
    def choose(name, predicate):
        rows = tables.get(name, [])
        selected[name] = [(n, row) for n, row in enumerate(rows)
                          if row.get('project_id', project) == project and predicate(row)
                          and (row.get('recorded_at') is None or row['recorded_at'] <= cutoff_micros)]
        return [row for _, row in selected[name]]
    goals = choose('context_items', lambda row: row['id'] == work)
    checkpoints = choose('checkpoints', lambda row: row['work_item_id'] == work)
    checkpoint_ids = {row['id'] for row in checkpoints}
    sources = set()
    for name in ('checkpoint_source_relations', 'checkpoint_verifications', 'context_item_sources'):
        rows = choose(name, lambda row: row.get('checkpoint_id') in checkpoint_ids
                      or row.get('context_item_id') == work)
        sources.update(row['source_id'] for row in rows if row.get('source_id'))
    applied = choose('checkpoint_decisions', lambda row: row['checkpoint_id'] in checkpoint_ids)
    decision_ids = {row['decision_id'] for row in applied}
    decisions = choose('decisions', lambda row: row.get('work_item_id') == work or row['id'] in decision_ids)
    question_refs = {(row['question_id'], row['question_revision']) for row in decisions}
    questions = choose('question_revisions', lambda row: (row['question_id'], row['revision']) in question_refs)
    for row in decisions + questions:
        if row.get('user_turn_source_id'):
            sources.add(row['user_turn_source_id'])
        for key in ('source_basis', 'recommendation_sources'):
            value = row.get(key)
            if isinstance(value, str):
                try:
                    sources.update(v for v in json.loads(value) if isinstance(v, str))
                except (TypeError, ValueError):
                    pass
    choose('sources', lambda row: row['id'] in sources)
    require(goals, 'exact canonical Work absent at cutoff')
    return selected


def prepare(archive_root, slot, destination, producer_head):
    root, destination = Path(archive_root).resolve(), Path(destination).resolve()
    campaign_root = root / 'campaign'
    campaign_path = campaign_root / 'campaign.json'
    campaign = json.loads(campaign_path.read_bytes())
    work = campaign['works'][slot]
    journey = campaign['journeys'][work['journey_id']]
    repository = Path(journey['repository_path'])
    require(repository.is_relative_to(root), 'repository outside original archive')
    captures = []
    for phase in ('start', 'resume'):
        path = campaign_root / 'slots' / slot / 'evidence' / (phase + '.rollout.jsonl')
        if path.exists():
            raw = path.read_bytes()
            events = codex_events.capture_events(raw)
            captures.append((path, codex_events.parse_codex_capture(raw), events))
    require(captures, 'no original capture')
    expected_sessions = {v for k, v in work.items() if k in {'start_session_id', 'resume_session_id'} and v}
    require({c.session_id for _, c, _ in captures} == expected_sessions, 'wrong historical sessions')
    project, identity = work['project_id'], work['work_item_id']
    cut = max(e['timestamp'] for _, _, events in captures for e in events)
    destination.mkdir(parents=True, mode=0o700, exist_ok=False)
    source_root = destination / 'preparation'
    source_root.mkdir()
    entries, investigation, omissions = [], [], []

    def add(data, *, name, role, producer, locator, observed_at, lane='archive_diagnostic',
            representation='full_file', attribution='explicit_work_record', proof=None,
            before_state='unavailable', extent=None, origin_witness=None):
        key = f'input-{len(entries):06d}'
        path = source_root / key
        path.write_bytes(data)
        entries.append({'id': key, 'path': name, 'role': role, 'project': project, 'work': identity,
                        'lane': lane, 'producer': producer, 'representation': representation,
                        'origin': binding(path), 'locator': locator, 'extent': extent,
                        'file_sha256': digest(data) if role == 'source' and representation != 'bounded_excerpt' else None,
                        'chronology': {'state': 'known', 'observed_at': observed_at},
                        'before_state': before_state, 'attribution': attribution, 'missing': None,
                        'proof': proof, 'origin_witness': origin_witness})

    product_read = {'state': 'available', 'name': 'canonical read / current explanation prepare'}
    archive_read = {'state': 'missing_capability', 'name': 'archive capture; historical body not retained by Product'}
    for phase, task in work['operator_task_artifacts'].items():
        path = campaign_root / task['path']
        require(binding(path)['sha256'] == task['sha256'], 'historical task bytes changed')
        add(path.read_bytes(), name=f'task/{phase}', role='task', producer=archive_read,
            locator=str(path.relative_to(root)), observed_at=captures[0][2][0]['timestamp'],
            origin_witness=binding(path))
    for path, capture, events in captures:
        witness = binding(path)
        for command in capture.commands:
            # Preserve every observed command; never curate only known answer functions.
            investigation.append({'project': project, 'work': identity, 'session': capture.session_id,
                                  'sequence': command.sequence, 'completion_sequence': command.completion_sequence,
                                  'invocation': command.parsed_command, 'exit_code': command.exit_code,
                                  'termination': command.termination, 'output_state': command.output_state,
                                  'output_sha256': digest(command.output.encode()),
                                  'attribution': 'observed_in_work_session_not_automatic_code_attribution'})
            add(encoded(vars(command)), name=f'observation/{capture.session_id}/{command.sequence}',
                role='command_observation', producer=archive_read,
                locator=f'{path.relative_to(root)}#record={command.completion_sequence + 1}',
                observed_at=events[command.completion_sequence]['timestamp'],
                representation='bounded_excerpt', extent={'kind': command.output_state,
                    'coordinates': 'command output; file coordinates unverified'}, origin_witness=witness)
        for call in capture.tool_calls:
            # Preserve bounded original Product observations; do not expose the raw
            # rollout's inherited developer/context messages or later review material.
            add(encoded(vars(call)), name=f'product-observation/{capture.session_id}/{call.sequence}',
                role='candidate_detail' if call.operation == 'candidate_inspect' else 'command_observation',
                producer={'state': 'available', 'name': f'historical MCP {call.operation}; read-time policy still applies'},
                locator=f'{path.relative_to(root)}#record={call.completion_sequence + 1}',
                observed_at=events[call.completion_sequence]['timestamp'], origin_witness=witness,
                attribution='observed_in_work_session_not_record_ownership')
        for number, event in enumerate(events, 1):
            payload = event.get('payload', {})
            if payload.get('type') == 'message' and payload.get('role') == 'assistant':
                add(encoded({'role': 'original_agent_report', 'content': payload['content']}),
                    name=f'report/{capture.session_id}/{number}', role='agent_report', producer=archive_read,
                    locator=f'{path.relative_to(root)}#record={number}', observed_at=event['timestamp'],
                    origin_witness=witness)
            # User runtime replies only, excluding injected AGENTS/environment context.
            if event.get('type') == 'event_msg' and payload.get('type') == 'user_message':
                add(encoded({'role': 'original_user_message', 'message': payload['message']}),
                    name=f'user/{capture.session_id}/{number}', role='user_response', producer=archive_read,
                    locator=f'{path.relative_to(root)}#record={number}', observed_at=event['timestamp'],
                    origin_witness=witness)

    bundle_path = campaign_root / 'slots' / journey['work_slot_ids'][0] / 'context.bundle.json'
    bundle = json.loads(bundle_path.read_bytes())
    cutoff_micros = int(dt.datetime.fromisoformat(cut).timestamp() * 1_000_000)
    selected = canonical_records(bundle, project, identity, cutoff_micros)
    for table, records in selected.items():
        for number, row in records:
            stamp = row.get('recorded_at')
            observed = (dt.datetime.fromtimestamp(stamp / 1_000_000, dt.timezone.utc).isoformat()
                        if stamp is not None else cut)
            add(encoded({'table': table, 'row': row}), name=f'canonical/{table}/{number}',
                role='canonical_record', producer=product_read, lane='product',
                locator=f'{bundle_path.relative_to(root)}#/payload/tables/{table}/rows/{number}',
                observed_at=observed, origin_witness=binding(bundle_path))
    files = baseline(repository, journey['repository_revision'])
    before = dict(files)
    changes, unsupported_paths = {}, set()
    # Replay all earlier same-repository literal patches in time order. No association
    # comes from final diff proximity. Shell writes/add/delete are explicitly unsupported.
    earlier = []
    for previous in campaign['works'].values():
        if previous['journey_id'] != work['journey_id']:
            continue
        for phase in ('start', 'resume'):
            p = campaign_root / 'slots' / previous['work_slot_id'] / 'evidence' / (phase + '.rollout.jsonl')
            if p.exists():
                events = codex_events.capture_events(p.read_bytes())
                for number, call_id, patch in patch_literals(events):
                    if events[number - 1]['timestamp'] <= cut:
                        earlier.append((events[number - 1]['timestamp'], p, number, call_id, patch, previous))
    for time, p, number, call_id, patch, previous in sorted(earlier, key=lambda item: item[0]):
        saved = dict(files)
        try:
            changed = update_files(files, patch, repository)
        except (ValueError, KeyError, UnicodeError) as error:
            files = saved
            for line in patch.splitlines():
                if line.startswith('*** Update File: '):
                    name = line.removeprefix('*** Update File: ')
                    unsupported_paths.add(str(Path(name).relative_to(repository)) if Path(name).is_absolute() else name)
            omissions.append({'kind': 'unsupported_patch_replay', 'locator': f'{p.relative_to(root)}#record={number}',
                              'reason': str(error), 'cutoff': time})
            continue
        for name in changed:
            prior = changes.get(name)
            changes[name] = {'time': time, 'witness': binding(p), 'locator': f'{p.relative_to(root)}#record={number}',
                             'work': previous['work_item_id'],
                             'before': prior['before'] if prior and prior['work'] == previous['work_item_id'] else saved[name],
                             'patches': (prior['patches'] if prior else []) + [
                                 {'origin': binding(p), 'record': number, 'call_id': call_id,
                                  'patch_sha256': digest(patch.encode())}]}

    state_path = campaign_root / 'journeys' / work['journey_id'] / 'evidence/repository-state.json'
    state = json.loads(state_path.read_bytes())
    final_hashes = {row['path']: row.get('sha256') for row in state['tracked']}
    for name, data in sorted(files.items()):
        change = changes.get(name)
        if name in unsupported_paths:
            omissions.append({'kind': 'unavailable_cutoff_file', 'path': name,
                              'reason': 'unsupported patch; baseline/later file not substituted'})
            continue
        if change:
            # The later final inventory corroborates bytes only. It supplies no Work attribution.
            if final_hashes.get(name) != digest(data):
                # Formatting is a diagnostic reconstruction, admitted only when it
                # exactly matches independent historical bytes. Never run captured shell.
                formatted = None
                if name.endswith('.rs') and any('cargo fmt' in c.parsed_command.get('cmd', '')
                        for _, capture, _ in captures for c in capture.commands):
                    try:
                        manifest = tomllib.loads(files.get('rebuild/Cargo.toml', files.get('Cargo.toml', b'')).decode())
                        edition = str(manifest.get('workspace', {}).get('package', {}).get('edition',
                                      manifest.get('package', {}).get('edition', '2015')))
                        result = subprocess.run(['rustfmt', '--edition', edition, '--emit', 'stdout'],
                                                input=data, capture_output=True, timeout=30, check=True)
                        formatted = result.stdout
                    except (OSError, subprocess.SubprocessError):
                        pass
                if formatted is not None and digest(formatted) == final_hashes.get(name):
                    data = formatted
                else:
                    omissions.append({'kind': 'reconstruction_not_final_hash_verified', 'path': name,
                                      'cutoff_bytes_sha256': digest(data), 'later_state_not_substituted': True})
                    continue
            add(data, name=name, role='source', producer=archive_read, locator=change['locator'],
                observed_at=change['time'], representation='verified_reconstruction',
                before_state='available', attribution='explicit_work_patch' if change['work'] == identity else 'earlier_work_context',
                proof={'expected_sha256': final_hashes[name], 'patches': change['patches'],
                       'independent_witnesses': [change['witness'], binding(state_path)]})
            if change['work'] == identity:
                add(change['before'], name=name, role='source', producer=archive_read,
                    locator=change['locator'] + ':before', observed_at=change['time'],
                    attribution='explicit_before_patch', before_state='available')
        else:
            # Include the entire pinned tracked repository, including original tests,
            # callers, helpers and config, irrespective of language or known answer sites.
            add(data, name=name, role='source', producer=archive_read,
                locator=f'git:{journey["repository_revision"]}:{name}',
                observed_at=captures[0][2][0]['timestamp'], attribution='pinned_repository_context',
                before_state='available' if name in before else 'unavailable')

    preparations = []
    for p in (campaign_root / 'explanations').glob('*/preparation.json'):
        prep = json.loads(p.read_bytes())
        if prep['subject'] == {'kind': 'work', 'identity': identity} and prep['project_id'] == project:
            preparations.append({'origin': binding(p), 'language': prep['language'],
                                 'observed_at': prep['observed_at'], 'fingerprint': prep['plan']['fingerprint'],
                                 'state': 'after_cutoff_requires_fresh_cutoff_bound_product_prepare'})
    # Do not inject a later preparation as the unchanged current baseline.
    omissions.append({'kind': 'current_prepare_at_cutoff_missing', 'later_preparations': preparations})
    omissions.append({'kind': 'replay_boundary', 'reason': 'Only literal Update File patches replayed. '
                      'Shell writes, formatting and unavailable original dirty bytes need independent corroboration. '
                      'Pinned context is not proof that every untouched file remained unchanged.'})
    spec = {'format_version': 1, 'scope': {'project': project, 'work': identity, 'cutoff': cut},
            'contract': [binding(Path(__file__).with_name(name)) for name in
                         ('inputs.py', 'conditions.json', 'instructions.txt')],
            'identities': {'product_candidate': {'head': campaign['candidate_head'], 'artifacts': campaign['candidate_artifacts']},
                           'explained_repository': {'revision': journey['repository_revision'], 'cutoff': cut,
                                                    'dirty_state': 'partial_recorded_patch_replay'},
                           'experiment_producer': {'head': producer_head, 'adapter': binding(Path(__file__))}},
            'entries': entries, 'investigation_inventory': investigation,
            'inventory_boundary': {'scope': 'all normalized commands in exact start/resume captures and all pinned tracked files',
                                   'raw_context_excluded': True, 'no_source_tour_selection': True,
                                   'omissions': omissions}, 'archive_witnesses': [binding(campaign_path), binding(state_path), binding(bundle_path)]}
    spec_path = destination / 'input-spec.json'
    spec_path.write_bytes(encoded(spec))
    return freeze(spec_path, destination / 'frozen', destination.parent)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--slot', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--producer-head', required=True)
    args = parser.parse_args()
    print(prepare(args.archive, args.slot, args.output, args.producer_head))
