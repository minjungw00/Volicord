"""Experimental claim/site bindings; verifies provenance, never entailment."""
from inputs import require

CONTRACT = 'reader_oriented'
DIRECTED_CONTRACT = 'work_directed_reader'
CONTRACTS = {CONTRACT, DIRECTED_CONTRACT}
MAX_SITES = 6
KINDS = {'source_fact', 'reported_behavior', 'user_choice', 'user_rationale',
         'model_interpretation', 'execution_evidence', 'unverified_expectation'}
DIRECTED_KINDS = KINDS | {'task_instruction', 'recorded_next_action',
                         'agent_recommendation', 'generated_suggestion'}
AUTHORITY_KINDS = {'task_instruction', 'recorded_next_action', 'user_choice',
                   'user_rationale', 'agent_recommendation'}


def validate_authority(claim, spec, selections, lane):
    """Check declared record coordinates, not the intent of natural language."""
    import json
    from pathlib import Path
    kind, authority = claim['kind'], claim['authority']
    if kind not in AUTHORITY_KINDS:
        require(authority is None, 'non-authority claim needs null authority')
        return
    require(isinstance(authority, dict) and set(authority) ==
            {'selection', 'record_id', 'revision', 'field'}, 'exact authority coordinates required')
    index = authority['selection']
    require(type(index) is int and index in claim['selections'], 'authority selection not bound to claim')
    result = selections[index]
    require(result['reference_status'] == 'valid_reference', 'invalid authority reference')
    entry = next(e for e in spec['entries'] if e['id'] == result['selection']['id'])
    if entry['role'] != 'canonical_record':
        allowed = {'task_instruction': {'task', 'user_response'},
                   'user_choice': {'user_response'}, 'user_rationale': {'user_response'},
                   'agent_recommendation': {'agent_report'}}
        require(entry['role'] in allowed.get(kind, set()), 'wrong authority evidence role')
        require(all(authority[k] is None for k in ('record_id', 'revision', 'field')),
                'original turn has no canonical record coordinates')
        return
    require(entry['representation'] == 'full_file', 'authority needs complete canonical row')
    selection = result['selection']
    require(selection['start'] == 0 and selection['end'] == entry['asset']['bytes'],
            'canonical authority needs the complete retrieved row')
    value = json.loads(Path(entry['asset']['path']).read_bytes())
    row, table = value['row'], value['table']
    require(row.get('project_id') == spec['scope']['project'] and
            row.get('work_item_id') == spec['scope']['work'], 'foreign canonical authority scope')
    require(authority['record_id'] == row.get('id') and type(authority['revision']) is int
            and authority['revision'] == row.get('revision'), 'authority record/revision mismatch')
    allowed = {'user_choice': ('decisions', {'choice_value'}),
               'user_rationale': ('decisions', {'user_rationale'}),
               'agent_recommendation': ('decisions', {'recommendation_rationale'}),
               'recorded_next_action': ('checkpoints', {'next_step'})}
    require(kind in allowed and table == allowed[kind][0] and
            authority['field'] in allowed[kind][1], 'wrong canonical authority field')
    require(isinstance(row.get(authority['field']), str) and row[authority['field']].strip(),
            'claimed authority field absent')
    if kind == 'recorded_next_action':
        from recorded_work import recorded_work
        _, facts = recorded_work(spec, lane, None)
        require(facts['checkpoint'] == row and row.get('work_state') not in {'superseded', 'abandoned'},
                'next action is not the latest applicable same-Work Checkpoint')


def indices(value, count, name):
    require(isinstance(value, list) and value and
            all(type(n) is int and 0 <= n < count for n in value) and
            len(set(value)) == len(value), 'invalid ' + name + ' indices')


def validate_reading(response, spec, validation, *, contract=CONTRACT, lane='archive_diagnostic'):
    """Bindings are structural. Independent semantic examination remains pending."""
    require(isinstance(response['claims'], list) and response['claims'], 'claims required')
    directed = contract == DIRECTED_CONTRACT
    require(isinstance(response['primary_sites'], list) and
            (directed or 1 <= len(response['primary_sites']) <= MAX_SITES),
            'historical reader requires one to six primary sites')
    prose = response['prose'].encode('utf-8')
    selections = validation['selections']
    entries = {e['id']: e for e in spec['entries']}
    for claim in response['claims']:
        fields = {'start', 'end', 'kind', 'selections'} | ({'authority'} if directed else set())
        require(isinstance(claim, dict) and set(claim) == fields,
                'strict claim fields required')
        start, end = claim['start'], claim['end']
        require(type(start) is int and type(end) is int and 0 <= start < end <= len(prose),
                'invalid prose byte span')
        prose[:start].decode('utf-8'); prose[start:end].decode('utf-8'); prose[end:].decode('utf-8')
        require(claim['kind'] in (DIRECTED_KINDS if directed else KINDS), 'unknown claim kind')
        indices(claim['selections'], len(selections), 'claim selection')
        if directed:
            validate_authority(claim, spec, selections, lane)
    seen = set()
    sites, issues = [], []
    for site in response['primary_sites']:
        fields = {'selections', 'claims', 'reason'} | ({'extent_reason'} if directed else set())
        require(isinstance(site, dict) and set(site) == fields,
                'strict primary site fields required')
        indices(site['selections'], len(selections), 'primary selection')
        indices(site['claims'], len(response['claims']), 'primary claim')
        require(len(site['selections']) <= 2, 'at most two states per site')
        require(isinstance(site['reason'], str) and site['reason'].strip(), 'primary reason absent')
        if directed:
            require(isinstance(site['extent_reason'], str) and site['extent_reason'].strip(),
                    'primary extent justification absent')
        require(not seen.intersection(site['selections']), 'duplicate primary selection')
        seen.update(site['selections'])
        covered = set()
        for claim_index in site['claims']:
            linked = set(site['selections']) & set(response['claims'][claim_index]['selections'])
            require(linked,
                    'primary claim does not bind site selections')
            covered.update(linked)
        # Separate before/after statements can each cite their own side. Every
        # named claim still needs a site binding, and no selected side is orphaned.
        require(covered == set(site['selections']), 'unbound primary selection')
        errors, selected = [], []
        for index in site['selections']:
            result = selections[index]
            if result['reference_status'] != 'valid_reference':
                errors.append('invalid primary reference: ' + '; '.join(result['issues']))
                continue
            entry = entries[result['selection']['id']]
            if entry['role'] != 'source' or entry['representation'] not in {'full_file', 'verified_reconstruction'}:
                errors.append('primary needs whole-file source coordinates')
            else:
                from source_reading import locate
                try:
                    locate(spec, entry, result)
                except (ValueError, KeyError, TypeError, OSError) as error:
                    errors.append('primary locator unavailable: ' + str(error))
            selected.append((result['selection'], entry))
        if len(selected) == 2:
            ordered = {selection['state']: entry for selection, entry in selected}
            if set(ordered) != {'before', 'after'}:
                errors.append('two selections require before/after states')
            else:
                before, after = ordered['before'], ordered['after']
                if not (before['path'] == after['path'] and before['locator'] == after['locator'] + ':before'
                        and before['producer'] == after['producer']
                        and before['chronology']['observed_at'] == after['chronology']['observed_at']
                        and before['before_state'] == after['before_state'] == 'available'):
                    errors.append('primary change correspondence unverified')
        sites.append({'site': site, 'status': 'invalid' if errors else 'valid_binding', 'issues': errors})
        issues.extend(errors)
    if any(s['reference_status'] != 'valid_reference' for s in selections):
        issues.append('invalid secondary/claim reference')
    return {'status': 'invalid' if issues else 'valid_binding', 'sites': sites,
            'issues': sorted(set(issues)), 'semantic_correctness': 'pending_independent_examination',
            'relevance': 'pending_independent_examination', 'user_comprehension': 'pending'}


def primary_html(response, validation, rows, *, comparisons=()):
    from source_reading import escaped, preview
    reading = validation['reading']
    if reading['status'] != 'valid_binding':
        return ('<p>Primary reading unavailable: ' + escaped('; '.join(reading['issues'])) +
                '. Original requests remain in the response download.</p>')
    body = '<nav class="primary-sites" aria-label="Generator ordered primary code sites"><ol>'
    by_index = {row['index']: row for row in rows}
    for site_number, result in enumerate(reading['sites']):
        site = result['site']
        selected = [by_index[n] for n in site['selections'] if n in by_index]
        if len(selected) != len(site['selections']) or any(
                row['location'] is None or row['location']['start'] is None for row in selected):
            body += '<li>Primary display gap; original site remains in the response download.</li>'
            continue
        body += '<li>' + ' / '.join('<a href="#' + row['anchor'] + '">' + escaped(row['label']) + '</a>'
                                   for row in selected)
        body += '<p class="primary-reason">' + escaped(site['reason']) + '</p>'
        if 'extent_reason' in site:
            body += '<p class="primary-extent">' + escaped(site['extent_reason']) + '</p>'
            for row in selected:
                location = row['location']
                body += '<p>Exact selected region: line:column ' + ':'.join(map(str, location['start']))
                body += '–' + ':'.join(map(str, location['end'])) + '; bytes ['
                body += str(row['selection']['start']) + ',' + str(row['selection']['end']) + ').</p>'
        if site_number < len(comparisons):
            body += comparisons[site_number]
        body += '<details><summary>Generator claim bindings; entailment unassessed</summary>'
        for index in site['claims']:
            claim = response['claims'][index]
            text = response['prose'].encode()[claim['start']:claim['end']].decode()
            body += '<p>' + escaped(claim['kind']) + '</p><pre class="bound-claim">' + preview(text) + '</pre>'
            body += '<p class="claim-sources" data-claim-index="' + str(index) + '">'
            links = []
            for source_index in claim['selections']:
                row = by_index.get(source_index)
                if row is None:
                    links.append('Source selection ' + str(source_index) + ' outside display; exact binding in original response')
                else:
                    links.append('<a data-selection-index="' + str(source_index) + '" href="#'
                                 + row['anchor'] + '">' + escaped(row['label']) + '</a>')
            body += ' · '.join(links) + '</p>'
        body += '</details></li>'
    return body + '</ol></nav>'
