"""Experimental claim/site bindings; verifies provenance, never entailment."""
from inputs import require

CONTRACT = 'reader_oriented'
MAX_SITES = 6
KINDS = {'source_fact', 'reported_behavior', 'user_choice', 'user_rationale',
         'model_interpretation', 'execution_evidence', 'unverified_expectation'}


def indices(value, count, name):
    require(isinstance(value, list) and value and
            all(type(n) is int and 0 <= n < count for n in value) and
            len(set(value)) == len(value), 'invalid ' + name + ' indices')


def validate_reading(response, spec, validation):
    """Bindings are structural. Independent semantic examination remains pending."""
    require(isinstance(response['claims'], list) and response['claims'], 'claims required')
    require(isinstance(response['primary_sites'], list) and
            1 <= len(response['primary_sites']) <= MAX_SITES, 'one to six primary sites required')
    prose = response['prose'].encode('utf-8')
    selections = validation['selections']
    entries = {e['id']: e for e in spec['entries']}
    for claim in response['claims']:
        require(isinstance(claim, dict) and set(claim) == {'start', 'end', 'kind', 'selections'},
                'strict claim fields required')
        start, end = claim['start'], claim['end']
        require(type(start) is int and type(end) is int and 0 <= start < end <= len(prose),
                'invalid prose byte span')
        prose[:start].decode('utf-8'); prose[start:end].decode('utf-8'); prose[end:].decode('utf-8')
        require(claim['kind'] in KINDS, 'unknown claim kind')
        indices(claim['selections'], len(selections), 'claim selection')
    seen = set()
    sites, issues = [], []
    for site in response['primary_sites']:
        require(isinstance(site, dict) and set(site) == {'selections', 'claims', 'reason'},
                'strict primary site fields required')
        indices(site['selections'], len(selections), 'primary selection')
        indices(site['claims'], len(response['claims']), 'primary claim')
        require(len(site['selections']) <= 2, 'at most two states per site')
        require(isinstance(site['reason'], str) and site['reason'].strip(), 'primary reason absent')
        require(not seen.intersection(site['selections']), 'duplicate primary selection')
        seen.update(site['selections'])
        for claim_index in site['claims']:
            require(set(site['selections']) <= set(response['claims'][claim_index]['selections']),
                    'primary claim does not bind site selections')
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


def primary_html(response, validation, rows):
    from source_reading import escaped, preview
    reading = validation['reading']
    if reading['status'] != 'valid_binding':
        return ('<p>Primary reading unavailable: ' + escaped('; '.join(reading['issues'])) +
                '. Original requests remain in the response download.</p>')
    body = '<nav class="primary-sites" aria-label="Generator ordered primary code sites"><ol>'
    by_index = {row['index']: row for row in rows}
    for result in reading['sites']:
        site = result['site']
        selected = [by_index[n] for n in site['selections'] if n in by_index]
        if len(selected) != len(site['selections']) or any(
                row['location'] is None or row['location']['start'] is None for row in selected):
            body += '<li>Primary display gap; original site remains in the response download.</li>'
            continue
        body += '<li>' + ' / '.join('<a href="#' + row['anchor'] + '">' + escaped(row['label']) + '</a>'
                                   for row in selected)
        body += '<p class="primary-reason">' + escaped(site['reason']) + '</p>'
        body += '<details><summary>Generator claim bindings; entailment unassessed</summary>'
        for index in site['claims']:
            claim = response['claims'][index]
            text = response['prose'].encode()[claim['start']:claim['end']].decode()
            body += '<p>' + escaped(claim['kind']) + '</p><pre class="bound-claim">' + preview(text) + '</pre>'
        body += '</details></li>'
    return body + '</ol></nav>'
