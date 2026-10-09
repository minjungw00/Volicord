"""Unedited, label-blinded private presentation; never a semantic evaluator."""
from __future__ import annotations

import argparse
import difflib
import html
import json
from pathlib import Path
import secrets

from grounding import validate_output
from inputs import append_index, binding, check_binding, encoded, require, verify


def escaped(value):
    return html.escape(str(value), quote=True)


def card(attempt_path, label):
    attempt_path = Path(attempt_path)
    record = json.loads(attempt_path.read_bytes())
    check_binding(record['input'])
    spec = verify(record['input']['path'])
    entries = {entry['id']: entry for entry in spec['entries']}
    trace = []
    if record.get('retrievals'):
        check_binding(record['retrievals'])
        trace = [json.loads(line) for line in Path(record['retrievals']['path']).read_bytes().splitlines()]
    parts = ['<article><h2>' + escaped(label) + '</h2>',
             '<p class="outcome">Outcome: ' + escaped(record['status']) + '</p>',
             '<p>Lane: ' + escaped(record['lane']) + '</p>',
             '<p>Semantic quality and inference support: not assessed. Reference checks certify byte identity only.</p>']
    if record.get('blockers'):
        parts.append('<p>Gaps: ' + escaped('; '.join(record['blockers'])) + '</p>')
    if record.get('exposure_issues'):
        parts.append('<p>Exposure gaps: ' + escaped('; '.join(record['exposure_issues'])) + '</p>')
    parts.append('<p>Isolation: ' + escaped(record.get('isolation', 'unknown')) + '</p>')
    parts.append('<p>Clean comparison: ' + escaped(record.get('clean_comparison', False)) + '</p>')
    witnesses = {'label': label, 'attempt': binding(attempt_path), 'outputs': [], 'semantic_quality': 'not_assessed'}
    if not record['original_outputs']:
        parts.append('<p>No generated output. No code selected. Before/after comparison unavailable.</p>')
    for number, output in enumerate(record['original_outputs'], 1):
        check_binding(output)
        raw = Path(output['path']).read_bytes()
        parts.append('<section><h3>Captured output ' + str(number) + '</h3>')
        witness = {'original': output, 'prose_sha256': None, 'selections': []}
        witnesses['outputs'].append(witness)
        response = None
        try:
            response = json.loads(raw)
            validation = validate_output(response, spec, record['lane'], trace)
        except (ValueError, KeyError, TypeError, OSError) as error:
            parts.append('<p>Response/grounding unavailable: ' + escaped(error) + '</p>')
            if isinstance(response, dict) and isinstance(response.get('prose'), str):
                parts.append('<pre class="prose">' + escaped(response['prose']) + '</pre>')
            parts.append('<pre class="original">' + escaped(raw.decode('utf-8', errors='backslashreplace')) + '</pre></section>')
            continue
        from inputs import digest
        witness['prose_sha256'] = digest(response['prose'].encode())
        parts.append('<pre class="prose">' + escaped(response['prose']) + '</pre>')
        parts.append('<p>Declared gaps: ' + escaped('; '.join(response['gaps']) if response['gaps'] else 'none declared; not independently checked') + '</p>')
        if not response['selections']:
            parts.append('<p>No code selected. Before/after comparison unavailable.</p>')
        pairings = {}
        for index, result in enumerate(validation['selections'], 1):
            selection = result['selection']
            status = result['reference_status']
            # Preserve code selection order, even where it is invalid. No repair,
            # offset adjustment, preferred entity or automatically chosen counterpart.
            anchor = label + '-output-' + str(number) + '-selection-' + str(index)
            parts.append('<div class="selection" id="' + escaped(anchor) + '"><h4>Selection ' + str(index) + '</h4>')
            parts.append('<pre class="selection-request">' + escaped(json.dumps(selection, ensure_ascii=False, sort_keys=True)) + '</pre>')
            byte_valid = status == 'valid_reference'
            if record['status'] == 'budget_exhausted' or not record.get('clean_comparison', False):
                status += '; run not verified'
            parts.append('<p>Reference: ' + escaped(status) + '</p>')
            if result['issues']:
                parts.append('<p>Gaps: ' + escaped('; '.join(result['issues'])) + '</p>')
            witness['selections'].append({'anchor': anchor, 'selection': selection, 'reference_status': status})
            if byte_valid:
                entry = entries[selection['id']]
                check_binding(entry['asset'])
                text = Path(entry['asset']['path']).read_bytes()[selection['start']:selection['end']].decode('utf-8')
                parts.append('<p>Path: ' + escaped(entry['path']) + '; selected state: ' + escaped(selection['state']) + '</p>')
                parts.append('<pre class="code">' + escaped(text) + '</pre>')
                if selection['state'] in {'before', 'after'} and entry['role'] == 'source':
                    pairings.setdefault(entry['path'], {'before': [], 'after': []})[selection['state']].append(text)
                elif selection['state'] == 'context':
                    parts.append('<p>Repository context; this span does not establish a Work change or after-state.</p>')
            else:
                parts.append('<p>Selected bytes withheld from verified presentation. No substitute code.</p>')
            parts.append('</div>')
        for path, states in pairings.items():
            parts.append('<h4>Before/after: ' + escaped(path) + '</h4>')
            if len(states['before']) == 1 and len(states['after']) == 1:
                diff = ''.join(difflib.unified_diff(states['before'][0].splitlines(True), states['after'][0].splitlines(True),
                                                  fromfile='selected before span', tofile='selected after span'))
                parts.append('<p>Diff of the two selected spans; not a whole-file diff.</p><pre class="diff">' + escaped(diff) + '</pre>')
            else:
                parts.append('<p>Comparison gap: one exact before and after span was not selected. Missing state is not inferred.</p>')
        parts.append('<details><summary>Original response transport</summary><pre class="original">' + escaped(raw.decode('utf-8')) + '</pre></details></section>')
    parts.append('</article>')
    return ''.join(parts), witnesses


def render(attempts, output):
    require(bool(attempts), 'attempts required')
    require(len({str(Path(p).resolve()) for p in attempts}) == len(attempts), 'duplicate attempt')
    output = Path(output).resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    ordering = list(attempts)
    secrets.SystemRandom().shuffle(ordering)
    cards, mapping, integrity = [], [], []
    for number, path in enumerate(ordering, 1):
        label = f'Sample {number:02d}'
        body, witness = card(path, label)
        cards.append(body); integrity.append(witness)
        record = json.loads(Path(path).read_bytes())
        mapping.append({'label': label, 'attempt': binding(path), 'approach': record['approach'], 'scope': record['scope']})
    document = ('<!doctype html><html lang="en"><meta charset="utf-8">'
                '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; style-src \'unsafe-inline\'">'
                '<title>Explanation comparison</title><style>'
                'body{font:16px system-ui;margin:2rem;max-width:1100px}article{border:1px solid #999;padding:1.5rem;margin:1rem 0}'
                'pre{white-space:pre-wrap;overflow-wrap:anywhere;padding:1rem;background:#f3f3f3}.prose{font:inherit;white-space:pre-wrap}'
                '.selection{border-left:3px solid #aaa;padding-left:1rem}</style><h1>Explanation comparison</h1>'
                '<p>Approach labels are withheld. Every captured output is displayed without editing. '
                'Self-identifying wording and stage counts may reveal an approach. No ranking or semantic verdict is provided.</p>'
                + ''.join(cards) + '</html>')
    (output / 'comparison.html').write_text(document)
    # Mapping and detailed private paths never appear in the presentation.
    (output / 'approach-mapping.json').write_bytes(encoded({'format_version': 1, 'mapping': mapping}))
    (output / 'integrity.json').write_bytes(encoded({'format_version': 1, 'presentation': binding(output / 'comparison.html'), 'samples': integrity}))
    append_index(output.parent, {'input': 'verified', 'approach': 'presentation_only', 'feedback': 'not_requested'},
                 [output / name for name in ('comparison.html', 'approach-mapping.json', 'integrity.json')])
    return output / 'comparison.html'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', type=Path, action='append', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(render(args.attempt, args.output))
