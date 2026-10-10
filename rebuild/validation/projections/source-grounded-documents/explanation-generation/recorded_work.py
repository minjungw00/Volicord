"""Read frozen canonical row evidence; no Product decoder or new authority."""
from pathlib import Path
import json
import datetime as dt

from inputs import check_binding, encoded, require
from source_reading import download, escaped, preview, public_receipt


def recorded_work(spec, lane, input_gap):
    facts = {'status': 'unavailable', 'problem': None, 'checkpoint': None,
             'verification': [], 'basis': [], 'limitations': [], 'history': [],
             'direction_status': 'unavailable'}
    if input_gap:
        facts['limitations'].append('Frozen input unavailable: ' + input_gap)
        return '', facts
    tables, tasks = {}, []
    for entry in spec['entries']:
        if lane == 'product' and entry['lane'] != 'product':
            continue
        if entry['project'] != spec['scope']['project'] or entry['work'] != spec['scope']['work']:
            continue
        if not entry['asset']:
            if entry['role'] == 'canonical_record':
                facts['limitations'].append('Canonical evidence unavailable; latest Work direction cannot be established')
            continue
        if entry['role'] not in {'task', 'canonical_record'} or entry['representation'] != 'full_file':
            if entry['role'] == 'canonical_record':
                facts['limitations'].append('Incomplete canonical row cannot establish latest Work direction')
            continue
        check_binding(entry['asset'])
        raw = Path(entry['asset']['path']).read_bytes()
        basis = {'id': entry['id'], 'locator': entry['locator'], 'asset': entry['asset'],
                 'producer': entry['producer'], 'chronology': entry['chronology'],
                 'project': entry['project'], 'work': entry['work']}
        if entry['chronology']['state'] == 'known':
            observed = dt.datetime.fromisoformat(entry['chronology']['observed_at'])
            if observed.tzinfo is None or observed > dt.datetime.fromisoformat(spec['scope']['cutoff']):
                facts['limitations'].append('Evidence after explanation cutoff')
                continue
        if entry['role'] == 'task':
            if entry['chronology']['state'] == 'known':
                tasks.append((raw.decode('utf-8'), basis))
            continue
        try:
            value = json.loads(raw)
            row = value['row']
            require(isinstance(row, dict), 'canonical row unavailable')
            if (value['table'] == 'checkpoints' and row.get('project_id') == spec['scope']['project']
                    and row.get('work_item_id') == spec['scope']['work']
                    and entry['chronology']['state'] != 'known'):
                facts['limitations'].append('Same-Work Checkpoint chronology ambiguous')
            if (row.get('project_id') == spec['scope']['project']
                    and (entry['chronology']['state'] == 'known'
                         or value['table'] in {'checkpoint_source_relations', 'checkpoint_verifications'})):
                tables.setdefault(value['table'], []).append((row, basis))
        except (ValueError, KeyError, TypeError) as error:
            # Unknown row evidence cannot establish complete/latest history.
            facts['limitations'].append('Canonical row unavailable: ' + str(error))
    checkpoints = [(row, basis) for row, basis in tables.get('checkpoints', [])
                   if row.get('work_item_id') == spec['scope']['work']]
    facts['history'] = [{'checkpoint': row, 'basis': basis} for row, basis in checkpoints]
    if checkpoints and not all(type(row.get('recorded_at')) is int and type(row.get('revision')) is int
                               for row, _ in checkpoints):
        facts['limitations'].append('Checkpoint ordering unavailable')
    if any(type(row.get('recorded_at')) is int and row['recorded_at'] >
           int(dt.datetime.fromisoformat(spec['scope']['cutoff']).timestamp() * 1000000)
           for row, _ in checkpoints):
        facts['limitations'].append('Checkpoint recorded after explanation cutoff')
    if checkpoints and not facts['limitations']:
        latest_time = max(row['recorded_at'] for row, _ in checkpoints)
        latest = [(row, basis) for row, basis in checkpoints if row['recorded_at'] == latest_time]
        if len(latest) != 1:
            facts['limitations'].append('Latest Checkpoint chronology ambiguous')
        else:
            row, basis = latest[0]
            facts.update(status='recorded', checkpoint=row, problem=row.get('goal'))
            facts['direction_status'] = ('superseded' if row.get('work_state') in {'superseded', 'abandoned'}
                                         else 'recorded' if row.get('next_step') else 'absent')
            facts['basis'].append(basis)
            sources = {r['id']: (r, b) for r, b in tables.get('sources', [])}

            def member_basis(member):
                if member['chronology']['state'] == 'known':
                    return True
                # Untimestamped child rows have no independent observation order.
                # Only exact membership in this cutoff-bound parent bundle is usable.
                return ('#/payload/' in basis['locator'] and '#/payload/' in member['locator']
                        and member['locator'].split('#/payload/')[0] == basis['locator'].split('#/payload/')[0]
                        and member['producer'] == basis['producer'])

            for relation, relation_basis in tables.get('checkpoint_source_relations', []):
                if relation.get('checkpoint_id') == row['id'] and member_basis(relation_basis):
                    facts['basis'].append(relation_basis)
                    source = sources.get(relation.get('source_id'))
                    if source:
                        facts['basis'].append(source[1])
                        if source[0].get('availability') != 'available':
                            facts['limitations'].append('Recorded supporting Source availability: '
                                                        + str(source[0].get('availability', 'unknown')))
                    else:
                        facts['limitations'].append('Checkpoint supporting Source unavailable')
            for observation, observation_basis in tables.get('checkpoint_verifications', []):
                if observation.get('checkpoint_id') != row['id'] or not member_basis(observation_basis):
                    continue
                source = sources.get(observation.get('source_id'))
                facts['verification'].append({'observation': observation, 'basis': observation_basis,
                                               'command_source': source[0] if source else None,
                                               'source_basis': source[1] if source else None})
    if not facts['problem'] and len(tasks) == 1:
        facts['problem'] = tasks[0][0]
        facts['basis'].append(tasks[0][1])
    body = ''
    if facts['problem']:
        body += '<h3>Recorded Work problem</h3><pre class="work-problem">' + preview(facts['problem']) + '</pre>'
    else:
        body += '<p>Recorded Work problem unavailable in this frozen input.</p>'
    return body, facts


def direction_html(facts):
    row = facts['checkpoint']
    if row is None:
        body = '<p>Recorded direction, completion, verification, user review and acceptance unavailable: no uniquely ordered same-Work Checkpoint in the permitted input.</p>'
        for limit in facts['limitations']:
            body += '<p class="gap">' + escaped(limit) + '</p>'
        data = encoded(public_receipt(facts))
        return body + '<details><summary>Unavailable direction and retained historical basis</summary>' + download(
            data, 'recorded-work.json', 'Complete recorded Work basis') + '</details>'
    body = '<section class="recorded-direction"><h3>Recorded direction</h3>'
    body += ('<p>Latest Work is superseded or abandoned; its historical action is not an applicable next step.</p>'
             if facts['direction_status'] == 'superseded' else
             '<pre class="direction">' + preview(row['next_step']) + '</pre>' if row.get('next_step') else
             '<p>No next meaningful action recorded in the latest Checkpoint.</p>')
    body += '<p>Quoted from the frozen same-Work Checkpoint; no new recommendation or user decision.</p>'
    body += '<p>Recorded Work state: ' + escaped(row.get('work_state', 'unavailable')) + '. '
    body += 'User review: ' + escaped(row.get('user_review', 'unavailable')) + '. '
    body += 'User acceptance: ' + escaped(row.get('user_acceptance', 'unavailable')) + '.</p>'
    observations = facts['verification']
    body += '<p>Recorded verification observations: ' + str(len(observations)) + '. '
    body += 'Coverage is limited to each recorded outcome; this reading runs no commands and grants no overall acceptance.</p>'
    if any(f['basis']['chronology']['state'] != 'known' for f in observations):
        body += '<p>Verification rows are members of the cutoff-bound Checkpoint bundle; independent execution chronology is unavailable.</p>'
    if observations:
        body += '<details><summary>Recorded command and verification outcomes (including failures)</summary>'
        for fact in observations:
            observation, source = fact['observation'], fact['command_source']
            body += '<p>' + escaped(observation.get('verification_state', 'unavailable')) + ': '
            body += escaped(source.get('locator') if source else 'Command Source unavailable') + '</p>'
            if source:
                body += '<p>Recorded command exit: ' + escaped(source.get('exit_code'))
                body += '; termination: ' + escaped(source.get('termination')) + '.</p>'
            body += '<pre>' + preview(observation.get('outcome', 'Outcome unavailable')) + '</pre>'
        body += '</details>'
        failed = sum(f['observation'].get('verification_state') == 'failed' for f in observations)
        if failed:
            body += '<p class="gap">Recorded failed verification observations: ' + str(failed) + '; later passes do not erase their scopes.</p>'
    for limit in facts['limitations']:
        body += '<p class="gap">' + escaped(limit) + '</p>'
    body += '<details class="recorded-course"><summary>Original same-Work course; later silence does not resolve earlier limits</summary>'
    for member in facts['history']:
        checkpoint = member['checkpoint']
        body += '<p>Checkpoint ' + escaped(checkpoint['id']) + '; revision ' + escaped(checkpoint['revision']) + '</p>'
        for field in ('state_change', 'next_step', 'known_limits'):
            if checkpoint.get(field):
                body += '<p>' + escaped(field) + '</p><pre>' + preview(checkpoint[field]) + '</pre>'
    body += '</details>'
    # Complete row bytes (including opaque encoded fields) remain recoverable.
    body += '<details><summary>Exact recorded direction and state basis</summary>'
    data = encoded(public_receipt(facts))
    body += '<pre>' + preview(data.decode()) + '</pre>'
    body += download(data, 'recorded-work.json', 'Complete recorded Work basis')
    return body + '</details></section>'
