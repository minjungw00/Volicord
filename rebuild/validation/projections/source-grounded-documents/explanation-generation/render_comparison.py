"""Unedited, label-blinded private presentation; never a semantic evaluator."""
from __future__ import annotations

import argparse
import difflib
import html
import json
from pathlib import Path
import secrets

from approaches import retrieval_audit
from grounding import validate_output
from inputs import append_index, binding, check_binding, encoded, require, verify


def escaped(value):
    return html.escape(str(value), quote=True)


def card(attempt_path, label):
    from source_reading import (TRANSPORT_BYTES, download, output_anchor, preview, prose_html, selections_html)
    attempt_path = Path(attempt_path)
    record = json.loads(attempt_path.read_bytes())
    check_binding(record['input'])
    spec = verify(record['input']['path'])
    require(record['scope'] == spec['scope'], 'attempt input scope changed')
    entries = {entry['id']: entry for entry in spec['entries']}
    trace = []
    if record.get('retrievals'):
        check_binding(record['retrievals'])
        trace = [json.loads(line) for line in Path(record['retrievals']['path']).read_bytes().splitlines()]
    failed_tools, host_calls = [], []
    for call in record.get('calls', []):
        if call['kind'] != 'model_call':
            continue
        process = call['process']
        check_binding(process['stdout'])
        for line in Path(process['stdout']['path']).read_bytes().splitlines():
            try:
                event = json.loads(line)
            except (ValueError, UnicodeError):
                continue
            item = event.get('item', {})
            if event.get('type') == 'item.completed' and item.get('type') == 'mcp_tool_call':
                host_calls.append(item)
                if item.get('status') == 'failed':
                    failed_tools.append(item.get('error'))
    audit = retrieval_audit(spec, record['lane'], host_calls, trace)
    verified_sequences = {row['sequence'] for row in audit['outcomes']
                          if row['outcome'] == 'source_read_verified'}
    verified_trace = [row for row in trace if row['sequence'] in verified_sequences]
    model_calls = [call for call in record.get('calls', []) if call['kind'] == 'model_call']
    parts = ['<article><h2>' + escaped(label) + '</h2>',
             '<p class="outcome">Outcome: ' + escaped(record['status']) + '</p>',
             '<p>Semantic quality and inference support: not assessed. Reference checks certify byte identity only.</p>']
    if record.get('blockers'):
        parts.append('<p>Gaps: ' + escaped('; '.join(record['blockers'])) + '</p>')
    final = record.get('generation_output')
    if final:
        require(final in record['original_outputs'], 'final output absent from original outputs')
    else:
        parts.append('<p>No verified final explanation. Captured intermediate or incomplete responses remain below.</p>')
    if not record['original_outputs']:
        parts.append('<p>No generated output. No code selected. Before/after comparison unavailable.</p>')
    witnesses = {'label': label, 'attempt': binding(attempt_path), 'outputs': [], 'semantic_quality': 'not_assessed'}
    views = []
    for number, output in enumerate(record['original_outputs'], 1):
        check_binding(output)
        require(output['bytes'] <= TRANSPORT_BYTES, 'response exceeds local presentation bound')
        raw = Path(output['path']).read_bytes()
        display_original = dict(output, attempt_sha256=witnesses['attempt']['sha256'])
        stage = ('Final response' if output == final else 'Intermediate response' if final
                 else 'Captured response; stage completion unverified')
        view = ['<section id="' + output_anchor(spec['scope'], display_original) + '"><h3>' + stage + ' ' + str(number) + '</h3>']
        witness = {'original': output, 'prose_sha256': None, 'selections': []}
        witnesses['outputs'].append(witness)
        response = None
        try:
            response = json.loads(raw)
            if isinstance(response, dict) and isinstance(response.get('prose'), str):
                response['prose'].encode('utf-8')
            validation = validate_output(response, spec, record['lane'], verified_trace)
        except (ValueError, KeyError, TypeError, OSError) as error:
            view.append('<p>Response/grounding unavailable: ' + escaped(error) + '</p>')
            if isinstance(response, dict) and isinstance(response.get('prose'), str):
                view.append('<pre class="prose">' + preview(response['prose']) + '</pre>')
        else:
            from inputs import digest
            witness['prose_sha256'] = digest(response['prose'].encode())
            source, diagnostic, rows, selections = selections_html(spec, entries, validation, display_original)
            witness['selections'] = selections
            run_verified = (record['status'] == 'captured' and not audit['issues'] and record.get('clean_comparison', False))
            for selection in selections:
                if not run_verified:
                    selection['reference_status'] += '; run not verified'
            view.append(prose_html(response['prose'], rows))
            view.append('<p>Declared gaps: ' + preview('; '.join(response['gaps']) if response['gaps'] else 'none declared; not independently checked') + '</p>')
            view.append('<p>Run verification: ' + ('verified' if run_verified else 'not verified') +
                        '. Valid source bytes remain distinct from whole-run and semantic validity.</p>')
            view.append(source)
            pairings = {}
            for row in rows:
                location, entry = row['location'], row['entry']
                if location and entry['role'] == 'source' and location['state'] in {'before', 'after'}:
                    pairings.setdefault(entry['path'], {'before': [], 'after': []})[location['state']].append(location['text'])
            for path, states in pairings.items():
                view.append('<details><summary>Before/after: ' + escaped(path) + '</summary>')
                if len(states['before']) == 1 and len(states['after']) == 1:
                    diff = ''.join(difflib.unified_diff(states['before'][0].splitlines(True), states['after'][0].splitlines(True),
                                                      fromfile='selected before span', tofile='selected after span'))
                    view.append('<p>Diff of the two selected spans; not a whole-file diff.</p><pre class="diff">' + preview(diff) + '</pre>')
                else:
                    view.append('<p>Comparison gap: one exact before and after span was not selected. Missing state is not inferred.</p>')
                view.append('</details>')
            view.append(diagnostic)
        view.append('<details><summary>Original response transport · lossless diagnostic download</summary><pre class="original">'
                    + preview(raw.decode('utf-8', errors='backslashreplace')) + '</pre>'
                    + download(raw, 'original-response-' + str(number) + '.bin', 'Complete original response bytes') + '</details></section>')
        body = ''.join(view)
        if final and output != final:
            body = '<details class="intermediate"><summary>' + stage + ' ' + str(number) + '</summary>' + body + '</details>'
        views.append((output == final, body))
    parts.extend(body for _, body in sorted(views, key=lambda item: not item[0]))
    run_audit = {'lane': record['lane'], 'isolation': record.get('isolation', 'unknown'),
                 'clean_comparison': record.get('clean_comparison', False),
                 'exposure_issues': record.get('exposure_issues', []), 'retrieval_audit': audit,
                 'inventory_boundary': spec['inventory_boundary'], 'tokens': record.get('tokens'), 'price': record.get('price')}
    parts.append('<details class="audit"><summary>Run, retrieval and inventory audit</summary><p>Returned evidence reads: '
                 + str(sum(r['status'] == 'returned' and r['name'] == 'read' for r in trace))
                 + '</p><p>Model calls: ' + str(len(model_calls)) + '; process exits: '
                 + escaped([c['process'].get('returncode', c['process'].get('exit_code')) for c in model_calls])
                 + '</p><p>Host tool failures: ' + preview(json.dumps(failed_tools, ensure_ascii=False))
                 + '</p><pre>' + preview(json.dumps(run_audit, ensure_ascii=False, indent=2)) + '</pre>'
                 + download(encoded(run_audit), 'run-audit.json', 'Complete derived audit') + '</details></article>')
    body = ''.join(parts)
    require(len(body.encode()) <= 16 * 1024 * 1024, 'derived presentation exceeds local HTML bound')
    return body, witnesses


def document(cards):
    return ('<!doctype html><html lang="en"><meta charset="utf-8">'
                '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; style-src \'unsafe-inline\'">'
                '<title>Explanation comparison</title><style>'
                'body{font:16px system-ui;margin:2rem;max-width:1100px}article{border:1px solid #999;padding:1.5rem;margin:1rem 0}'
                'pre{white-space:pre-wrap;overflow-wrap:anywhere;padding:1rem;background:#f3f3f3}.prose{font:inherit;white-space:pre-wrap}'
                'details{margin:.8rem 0}summary{cursor:pointer}a{overflow-wrap:anywhere}:target{outline:2px solid #467}'
                '.citation span{display:none}.citation:after{content:attr(data-location)}'
                '.selection{border-left:3px solid #aaa;padding-left:1rem}pre.code,pre.diff{max-height:32rem;overflow:auto}</style><h1>Explanation comparison</h1>'
                '<p>Approach labels are withheld. Final prose precedes closed evidence and audit disclosures. Original bytes remain in diagnostic downloads. '
                'Self-identifying wording and stage counts may reveal an approach. No ranking or semantic verdict is provided.</p>'
                + ''.join(cards) + '</html>')


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
    (output / 'comparison.html').write_text(document(cards))
    # Mapping and detailed private paths never appear in the presentation.
    (output / 'approach-mapping.json').write_bytes(encoded({'format_version': 1, 'mapping': mapping}))
    (output / 'integrity.json').write_bytes(encoded({'format_version': 1, 'presentation': binding(output / 'comparison.html'), 'samples': integrity}))
    append_index(output.parent, {'input': 'verified', 'approach': 'presentation_only', 'feedback': 'not_requested'},
                 [output / name for name in ('comparison.html', 'approach-mapping.json', 'integrity.json')])
    return output / 'comparison.html'


def record_feedback(presentation, response, output):
    """Retain one literal response or pending state; never infer a human verdict."""
    presentation, output = Path(presentation).resolve(), Path(output).resolve()
    require(response is None or isinstance(response, str) and response.strip(), 'literal response required')
    integrity_path = presentation.parent / 'integrity.json'
    integrity = json.loads(integrity_path.read_bytes())
    require(integrity['presentation']['path'] == str(presentation), 'feedback display path changed')
    check_binding(integrity['presentation'])
    scopes, originals, cards = [], [], []
    for sample in integrity['samples']:
        check_binding(sample['attempt'])
        body, witness = card(sample['attempt']['path'], sample['label'])
        cards.append(body)
        require(witness == sample, 'feedback output/selection identity changed')
        attempt = json.loads(Path(sample['attempt']['path']).read_bytes())
        scopes.append(attempt['scope'])
        originals.extend(o['original'] for o in sample['outputs'])
    require(scopes and all(scope == scopes[0] for scope in scopes), 'feedback must concern one Work')
    require(presentation.read_bytes() == document(cards).encode(), 'feedback displayed content changed')
    value = {'presentation': integrity['presentation'], 'integrity': binding(integrity_path),
             'scope': scopes[0], 'original_outputs': originals, 'literal_response': response,
             'H1': 'pending' if response is None else 'response_recorded',
             'limits': 'Literal supplied observation only; no inferred verdict, authorship attestation or other Work approval.'}
    with output.open('xb') as stream:
        stream.write(encoded(value))
    append_index(output.parent, {'input': 'verified', 'approach': 'presentation_only',
                                 'feedback': value['H1']}, [output])
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', type=Path, action='append', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(render(args.attempt, args.output))
