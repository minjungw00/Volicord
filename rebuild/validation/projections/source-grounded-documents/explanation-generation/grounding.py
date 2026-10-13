"""Strict transport grounding, deliberately no prose entailment judgment."""
from inputs import check_binding, digest, require
from pathlib import Path
import datetime as dt


def source_state(entry):
    """Frozen attribution, never a model-selected chronological interpretation."""
    return ('before' if entry['attribution'] == 'explicit_before_patch' else
            'after' if entry['attribution'] == 'explicit_work_patch' else 'context')


def diagnose_selection(selection, spec, entries, retrievals):
    issues, checks, values = [], {}, {}

    def check(name, operation, prerequisites=()):
        blocked = [p for p in prerequisites if checks[p]['status'] != 'passed']
        if blocked:
            checks[name] = {'status': 'not_run', 'prerequisites': blocked}
            return
        try:
            values[name] = operation()
            checks[name] = {'status': 'passed'}
        except (ValueError, KeyError, TypeError, OSError, AttributeError) as error:
            checks[name] = {'status': 'failed', 'reason': str(error)}
            issues.append(str(error))

    check('structure', lambda: require(isinstance(selection, dict) and
          set(selection) == {'id', 'start', 'end', 'sha256', 'state'}, 'strict selection fields required'))
    check('identity', lambda: entries[selection['id']], ('structure',))

    def entry():
        return values['identity']

    check('scope', lambda: require(entry()['project'] == spec['scope']['project'] and
          entry()['work'] == spec['scope']['work'], 'foreign Work'), ('identity',))

    def state():
        require(selection['state'] in {'before', 'after', 'context'}, 'unknown source state')
        actual = source_state(entry())
        require(selection['state'] == actual, 'wrong-state span')
    check('state', state, ('identity',))

    def chronology():
        require(entry()['chronology']['state'] == 'known', 'ambiguous chronology')
        observed = dt.datetime.fromisoformat(entry()['chronology']['observed_at'])
        require(observed.tzinfo is not None, 'source observation timezone absent')
        return observed
    check('chronology', chronology, ('identity',))
    check('cutoff', lambda: require(values['chronology'] <= dt.datetime.fromisoformat(spec['scope']['cutoff']),
          'source after cutoff'), ('chronology',))
    check('availability', lambda: require(entry()['asset'] is not None, 'unavailable source'), ('identity',))

    def span():
        start, end = selection['start'], selection['end']
        require(type(start) is int and type(end) is int and 0 <= start < end, 'invalid byte span')
        return start, end
    check('range_syntax', span, ('structure',))
    check('range', lambda: require(selection['end'] <= entry()['asset']['bytes'], 'invalid byte span'),
          ('range_syntax', 'availability'))

    def asset():
        check_binding(entry()['asset'])
        data = Path(entry()['asset']['path']).read_bytes()
        require(len(data) == entry()['asset']['bytes'] and digest(data) == entry()['asset']['sha256'],
                'changed bytes or stale preparation')
        return data
    # Foreign scope and unavailable/withheld assets cannot be opened for diagnostics.
    check('asset_integrity', asset, ('scope', 'availability'))

    def retrieved():
        spans = sorted((r['result']['metadata']['offset'], r['result']['metadata']['offset'] + r['result']['metadata']['bytes'])
                       for r in retrievals if r['status'] == 'returned' and r['name'] == 'read'
                       and r['result']['metadata']['id'] == selection['id'])
        cursor = selection['start']
        for left, right in spans:
            if left <= cursor:
                cursor = max(cursor, right)
        require(cursor >= selection['end'], 'span not retrieved')
    check('retrieval', retrieved, ('range',))

    def selected():
        return values['asset_integrity'][selection['start']:selection['end']]
    check('hash', lambda: require(digest(selected()) == selection['sha256'], 'span hash mismatch'),
          ('range', 'asset_integrity'))
    check('utf8', lambda: selected().decode('utf-8'), ('range', 'asset_integrity'))
    valid = all(c['status'] == 'passed' for c in checks.values())
    if valid and entry()['representation'] == 'bounded_excerpt':
        issues.append('bounded excerpt; does not certify a complete file')
    return {'selection': selection, 'reference_status': 'valid_reference' if valid else 'invalid',
            'issues': list(dict.fromkeys(issues)), 'checks': checks}


def validate_output(response, spec, lane, retrievals, *, contract=None):
    from reader_contract import CONTRACTS
    fields = {'prose', 'selections', 'gaps'}
    if contract in CONTRACTS:
        fields |= {'claims', 'primary_sites'}
    else:
        require(contract is None, 'unknown output contract')
    require(isinstance(response, dict) and set(response) == fields, 'strict explanation sidecar required')
    require(isinstance(response['prose'], str) and response['prose'].strip(), 'prose absent')
    require(isinstance(response['selections'], list) and isinstance(response['gaps'], list)
            and all(isinstance(g, str) for g in response['gaps']), 'sidecar lists required')
    entries = {e['id']: e for e in spec['entries'] if lane == 'archive_diagnostic' or e['lane'] == 'product'}
    results = [diagnose_selection(selection, spec, entries, retrievals) for selection in response['selections']]
    validation = {'selections': results, 'semantic_quality': 'not_assessed', 'unsupported_inferences': 'not_verified',
                  'gaps': response['gaps'], 'missing_selection': not bool(response['selections'])}
    if contract in CONTRACTS:
        from reader_contract import validate_reading
        validation['reading'] = validate_reading(response, spec, validation, contract=contract, lane=lane)
    return validation
