"""Strict transport grounding, deliberately no prose entailment judgment."""
from inputs import check_binding, digest, require
from pathlib import Path


def validate_output(response, spec, lane, retrievals):
    require(set(response) == {'prose', 'selections', 'gaps'}, 'strict explanation sidecar required')
    require(isinstance(response['prose'], str) and response['prose'].strip(), 'prose absent')
    require(isinstance(response['selections'], list) and isinstance(response['gaps'], list)
            and all(isinstance(g, str) for g in response['gaps']), 'sidecar lists required')
    entries = {e['id']: e for e in spec['entries'] if lane == 'archive_diagnostic' or e['lane'] == 'product'}
    results = []
    for selection in response['selections']:
        issues = []
        try:
            require(set(selection) == {'id', 'start', 'end', 'sha256', 'state'}, 'strict selection fields required')
            entry = entries[selection['id']]
            require(entry['project'] == spec['scope']['project'] and entry['work'] == spec['scope']['work'], 'foreign Work')
            require(selection['state'] in {'before', 'after', 'context'}, 'unknown source state')
            actual = ('before' if entry['attribution'] == 'explicit_before_patch' else
                      'after' if entry['attribution'] == 'explicit_work_patch' else 'context')
            require(selection['state'] == actual, 'wrong-state span')
            require(entry['chronology']['state'] == 'known', 'ambiguous chronology')
            require(entry['asset'] is not None, 'unavailable source')
            check_binding(entry['asset'])
            start, end = selection['start'], selection['end']
            require(type(start) is int and type(end) is int and 0 <= start < end <= entry['asset']['bytes'], 'invalid byte span')
            spans = sorted((r['result']['metadata']['offset'], r['result']['metadata']['offset'] + r['result']['metadata']['bytes'])
                           for r in retrievals if r['status'] == 'returned' and r['name'] == 'read'
                           and r['result']['metadata']['id'] == selection['id'])
            cursor = start
            for left, right in spans:
                if left <= cursor:
                    cursor = max(cursor, right)
            require(cursor >= end, 'span not retrieved')
            data = Path(entry['asset']['path']).read_bytes()[start:end]
            require(digest(data) == selection['sha256'], 'span hash mismatch')
            data.decode('utf-8')
            if entry['representation'] == 'bounded_excerpt':
                issues.append('bounded excerpt; does not certify a complete file')
        except (ValueError, KeyError, TypeError, OSError) as error:
            issues.append(str(error))
        results.append({'selection': selection, 'reference_status': 'invalid' if issues and issues != ['bounded excerpt; does not certify a complete file'] else 'valid_reference',
                        'issues': issues})
    return {'selections': results, 'semantic_quality': 'not_assessed', 'unsupported_inferences': 'not_verified',
            'gaps': response['gaps'], 'missing_selection': not bool(response['selections'])}
