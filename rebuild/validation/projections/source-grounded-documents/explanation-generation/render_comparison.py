"""Unedited, label-blinded private presentation; never a semantic evaluator."""
from __future__ import annotations

import argparse
import html
import json
from pathlib import Path
import secrets

from approaches import retrieval_audit
from grounding import validate_output
from inputs import append_index, binding, check_binding, digest, encoded, require, verify


def reading_receipt(record, attempt, output, *, status, issues=(), validation=None):
    """Deterministic current read receipt, separate from the historical execution."""
    here = Path(__file__).parent
    return {'authority': 'current local structural reading validation; not Product completion',
            'status': status, 'issues': list(issues), 'attempt': attempt, 'input': record['input'],
            'output': output, 'contract': record.get('output_contract'),
            'verifier': [binding(here / name) for name in ('render_comparison.py', 'grounding.py',
                         'reader_contract.py', 'source_reading.py', 'inputs.py', 'approaches.py', 'recorded_work.py')],
            'retrievals': record.get('retrievals'),
            'host_observations': [call['process']['stdout'] for call in record.get('calls', [])
                                  if call['kind'] == 'model_call'],
            'selection_count': len(validation['selections']) if validation else None,
            'reading': validation.get('reading') if validation else None}


def escaped(value):
    return html.escape(str(value), quote=True)


def card(attempt_path, label):
    from source_reading import (TRANSPORT_BYTES, comparison_html, download, output_anchor, preview,
                                prose_html, public_receipt, selections_html)
    from recorded_work import direction_html, recorded_work
    attempt_path = Path(attempt_path)
    record = json.loads(attempt_path.read_bytes())
    check_binding(record['input'])
    require(record['lane'] in {'product', 'archive_diagnostic'}, 'unknown input lane')
    spec = json.loads(Path(record['input']['path']).read_bytes())
    require(record['scope'] == spec['scope'], 'attempt input scope changed')
    input_gap = None
    try:
        spec = verify(record['input']['path'])
    except (ValueError, KeyError, TypeError, OSError) as error:
        # The manifest/attempt binding is intact, but its dependencies are not.
        # Keep original output diagnostics without granting any source validity.
        input_gap = str(error)
    entries = {entry['id']: entry for entry in spec['entries']
               if (record['lane'] == 'archive_diagnostic' or entry['lane'] == 'product')
               and entry['project'] == spec['scope']['project'] and entry['work'] == spec['scope']['work']}
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
    audit = (retrieval_audit(spec, record['lane'], host_calls, trace) if input_gap is None else
             {'outcomes': [], 'issues': ['frozen input unavailable: ' + input_gap]})
    verified_sequences = {row['sequence'] for row in audit['outcomes']
                          if row['outcome'] == 'source_read_verified'}
    verified_trace = [row for row in trace if row['sequence'] in verified_sequences]
    model_calls = [call for call in record.get('calls', []) if call['kind'] == 'model_call']
    problem, work_facts = recorded_work(spec, record['lane'], input_gap)
    parts = ['<article><h2>' + escaped(label) + '</h2>']
    directed = record.get('output_contract') == 'work_directed_reader'
    work_details = '<details class="work-problem-basis"><summary>Recorded Work problem · original wording</summary>' + problem + '</details>'
    invocation_notice = ''
    if record['status'] != 'captured':
        invocation_notice = ('<p class="gap">The original invocation did not complete successfully. '
                             'Any currently valid reading below is a diagnostic preview only; '
                             'the historical execution receipt is unchanged.</p>')
    if record.get('blockers'):
        parts.append('<p>Gaps: ' + escaped('; '.join(record['blockers'])) + '</p>')
    if input_gap is not None:
        parts.append('<p>Input verification gap: ' + escaped(input_gap)
                     + '. All source anchors withheld for this frozen input; original outputs remain diagnostic.</p>')
    final = record.get('generation_output')
    if final:
        require(final in record['original_outputs'], 'final output absent from original outputs')
    else:
        invocation_notice += ('<p>No final response identity was retained by the original invocation. '
                              'Current reading validation is reported separately for each captured response.</p>')
    if not record['original_outputs']:
        parts.append('<p>No generated output. No code selected. Before/after comparison unavailable.</p>')
    witnesses = {'label': label, 'attempt': binding(attempt_path), 'outputs': [], 'semantic_quality': 'not_assessed',
                 'recorded_work': work_facts,
                 'original_invocation': {'authority': 'immutable original attempt receipt',
                    'attempt': binding(attempt_path),
                    'status': record['status'], 'input': record['input'], 'generation_output': final,
                    'original_outputs': record['original_outputs'], 'verifier': record.get('support', []),
                    'frozen_verifier': record.get('frozen_support', []),
                    'instructions': record.get('instructions'), 'conditions': record.get('conditions')}}
    views = []
    for number, output in enumerate(record['original_outputs'], 1):
        try:
            check_binding(output)
        except FileNotFoundError:
            receipt = reading_receipt(record, witnesses['attempt'], output, status='unavailable_response',
                                      issues=['Captured response file missing; no substitute response'])
            witnesses['outputs'].append({'original': output, 'prose_sha256': None, 'selections': [],
                                        'derived_reading': receipt})
            parts.append('<p class="gap">Response availability: missing. Derived reading unavailable_response; no substitute prose or code.</p>')
            continue
        require(output['bytes'] <= TRANSPORT_BYTES, 'response exceeds local presentation bound')
        raw = Path(output['path']).read_bytes()
        display_original = dict(output, attempt_sha256=witnesses['attempt']['sha256'], output_sequence=number)
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
            require(input_gap is None, 'frozen input unavailable: ' + str(input_gap))
            validation = validate_output(response, spec, record['lane'], verified_trace,
                                         contract=record.get('output_contract'))
        except (ValueError, KeyError, TypeError, OSError) as error:
            status = 'unavailable_input' if input_gap else 'invalid_response'
            witness['derived_reading'] = reading_receipt(record, witnesses['attempt'], output,
                                                        status=status, issues=[str(error)])
            view.append('<p class="gap">Response availability: retained. Current derived reading: '
                        + status + '. Response/grounding unavailable: ' + escaped(error) + '</p>')
            view.append('<p>Unverified captured prose · diagnostic only.</p>')
            if isinstance(response, dict) and isinstance(response.get('prose'), str):
                view.append('<pre class="prose">' + preview(response['prose']) + '</pre>')
            view.append(invocation_notice)
        else:
            witness['prose_sha256'] = digest(response['prose'].encode())
            valid = (all(s['reference_status'] == 'valid_reference' for s in validation['selections'])
                     and validation.get('reading', {}).get('status', 'valid_binding') == 'valid_binding')
            status = 'valid_binding' if valid else 'invalid'
            issues = [issue for s in validation['selections'] for issue in s['issues']]
            issues += validation.get('reading', {}).get('issues', [])
            receipt = reading_receipt(record, witnesses['attempt'], output, status=status,
                                      issues=issues, validation=validation)
            # Exact Source dependencies are independent of original run success.
            used = dict.fromkeys(s['selection']['id'] for s in validation['selections']
                                 if isinstance(s['selection'], dict) and isinstance(s['selection'].get('id'), str)
                                 and s['selection']['id'] in entries)
            receipt['source_dependencies'] = [{key: entries[identity][key]
                for key in ('id', 'asset', 'locator', 'chronology')} for identity in used]
            witness['derived_reading'] = receipt
            primary_indices = ({n for site in response.get('primary_sites', []) for n in site['selections']}
                               if validation.get('reading', {}).get('status') == 'valid_binding' else set())
            source, diagnostic, rows, selections = selections_html(spec, entries, validation, display_original,
                                                                   primary_indices=primary_indices)
            witness['selections'] = selections
            run_verified = (record['status'] == 'captured' and not audit['issues'] and record.get('clean_comparison', False))
            for selection in selections:
                if not run_verified:
                    selection['reference_status'] += '; run not verified'
            direction_target = output_anchor(spec['scope'], display_original) + '-direction'
            view.append('<p class="reading-status">Original invocation: ' + escaped(record['status'])
                        + ' · current byte binding: ' + status + ' · semantic assessment: not assessed. '
                        '<a href="#' + direction_target + '">Recorded next action and verification scope</a></p>')
            if record.get('output_contract'):
                from reader_contract import primary_navigation_html
                view.append(primary_navigation_html(response, validation, rows))
            view.append(prose_html(response['prose'], rows))
            view.append(invocation_notice)
            view.append('<div id="' + direction_target + '">' + direction_html(work_facts) + '</div>')
            view.append(work_details)
            view.append('<p class="derived-reading">Response availability: retained. Current derived reading: '
                        + status + ' · ' + ('diagnostic preview' if valid else 'unverified captured prose')
                        + '. Current local verifier checks exact response/Source bindings; '
                        'semantic quality and inference support are not assessed. '
                        'Reference checks certify byte identity only.</p>')
            if issues:
                view.append('<p class="gap">Current validation gaps: ' + escaped('; '.join(issues)) + '</p>')
            if record.get('output_contract'):
                from reader_contract import authority_html, primary_html
                witness['reading'] = validation['reading']
                if directed:
                    authority, witness['claim_authority'] = authority_html(response, validation, rows)
                    view.append('<details class="authority-details"><summary>Task and action claim meanings · original model classifications</summary>'
                                + authority + '</details>')
                primary_comparisons = []
                witness['primary_comparisons'] = []
                if validation['reading']['status'] == 'valid_binding':
                    for site_number, site in enumerate(response['primary_sites'], 1):
                        site_rows = [row for index in site['selections'] for row in rows if row['index'] == index]
                        site_html, site_witness = comparison_html(spec, site_rows, namespace='primary-' + str(site_number))
                        primary_comparisons.append(site_html)
                        witness['primary_comparisons'].append({'site': site, 'comparisons': site_witness})
                view.append(primary_html(response, validation, rows, comparisons=primary_comparisons))
            view.append('<p>Declared gaps: ' + preview('; '.join(response['gaps']) if response['gaps'] else 'none declared; not independently checked') + '</p>')
            view.append('<details><summary>Original run verification</summary><p>Run verification: '
                        + ('verified' if run_verified else 'not verified') +
                        '. Valid source bytes remain distinct from whole-run and semantic validity.</p></details>')
            if record.get('output_contract'):
                source = '<details class="secondary"><summary>Complete secondary source selections</summary>' + source + '</details>'
            view.append(source)
            comparison, witness['comparisons'] = comparison_html(spec, rows,
                compact=not bool(record.get('output_contract')), namespace='diagnostic')
            if record.get('output_contract'):
                comparison = '<details class="all-comparisons"><summary>All selected file comparisons · diagnostic grouping</summary>' + comparison + '</details>'
            view.append(comparison)
            view.append(diagnostic)
        data = json.dumps(public_receipt(witness['derived_reading']), ensure_ascii=False,
                          sort_keys=True, separators=(',', ':')).encode()
        summary = {key: value for key, value in public_receipt(witness['derived_reading']).items()
                   if key not in {'verifier', 'host_observations'}}
        view.append('<details class="reading-receipt"><summary>Current derived validation authority and exact identities</summary><pre>'
                    + preview(json.dumps(summary, ensure_ascii=False)) + '</pre>'
                    + download(data, 'derived-reading.json', 'Complete current reading receipt') + '</details>')
        view.append('<details><summary>Original response transport · lossless diagnostic download</summary><pre class="original">'
                    + preview(raw.decode('utf-8', errors='backslashreplace')) + '</pre>'
                    + download(raw, 'original-response-' + str(number) + '.bin', 'Complete original response bytes') + '</details></section>')
        body = ''.join(view)
        if final and output != final:
            body = '<details class="intermediate"><summary>' + stage + ' ' + str(number) + '</summary>' + body + '</details>'
        views.append((output == final, body))
    parts.extend(body for _, body in sorted(views, key=lambda item: not item[0]))
    if not views:
        parts.append(invocation_notice)
    if not any(output.get('prose_sha256') for output in witnesses['outputs']):
        parts.append(direction_html(work_facts))
        parts.append(work_details)
    original = encoded(public_receipt(witnesses['original_invocation']))
    parts.append('<details class="invocation"><summary>Original invocation outcome and retained response identities</summary>'
                 + '<p>Original invocation outcome: ' + escaped(record['status']) + '</p><pre>'
                 + preview(original.decode()) + '</pre>'
                 + download(original, 'original-invocation.json', 'Complete original invocation identities') + '</details>')
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
                'body{font:16px system-ui;margin:2rem;max-width:1100px;overflow-wrap:anywhere}article{border:1px solid #999;padding:1.5rem;margin:1rem 0}'
                'pre{white-space:pre-wrap;overflow-wrap:anywhere;padding:1rem;background:#f3f3f3}.prose{font:inherit;white-space:pre-wrap}'
                'details{margin:.8rem 0}summary{cursor:pointer}a{overflow-wrap:anywhere}:target{outline:2px solid #467}'
                '.citation span{display:none}.citation:after{content:attr(data-location)}'
                '.selection{border-left:3px solid #aaa;padding-left:1rem}pre.code,pre.diff{max-height:32rem;overflow:auto}'
                '.primary-navigation ol{padding-left:1.5rem}.primary-navigation li{margin:.2rem 0}'
                '@media(max-width:600px){body{margin:.75rem}article{padding:.75rem}pre{padding:.5rem}.selection{padding-left:.5rem}'
                '.primary-navigation .path-directory{display:none}}</style><h1>Explanation comparison</h1>'
                '<p>Original generated explanation · diagnostic preview, not Product completion. This presentation supplies no semantic or human verdict.</p>'
                '<details class="presentation-basis"><summary>Presentation scope and original evidence</summary>'
                '<p>Approach labels are withheld. Primary navigation preserves generator order; no ranking or semantic verdict is provided. '
                'Original bytes remain in diagnostic downloads. Self-identifying wording and stage counts may reveal an approach.</p></details>'
                + ''.join(cards) + '</html>')


def render(attempts, output):
    require(bool(attempts), 'attempts required')
    require(len({str(Path(p).resolve()) for p in attempts}) == len(attempts), 'duplicate attempt')
    require(len({binding(p)['sha256'] for p in attempts}) == len(attempts), 'duplicate attempt content')
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


def record_feedback(presentation, response, output, *, reviewer_kind=None):
    """Bind a declared human response or agent assessment; never infer authorship."""
    presentation, output = Path(presentation).resolve(), Path(output).resolve()
    require(response is None or isinstance(response, str) and response.strip(), 'literal response required')
    require(reviewer_kind is None or reviewer_kind in {'human', 'agent'}, 'invalid reviewer kind')
    require(response is None or reviewer_kind is not None, 'reviewer kind required for literal response')
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
             'reviewer_kind': reviewer_kind, 'authorship': 'declared_not_authenticated',
             'feedback_state': ('pending' if response is None else
                                'assessment_recorded' if reviewer_kind == 'agent' else 'response_recorded'),
             'H1': 'response_recorded' if response is not None and reviewer_kind == 'human' else 'pending',
             'limits': 'Literal supplied observation only; declared reviewer kind is not authorship attestation. '
                       'Agent assessment cannot complete human H1; no inferred verdict or other Work approval.'}
    with output.open('xb') as stream:
        stream.write(encoded(value))
    append_index(output.parent, {'input': 'verified', 'approach': 'presentation_only',
                                 'feedback': value['feedback_state']}, [output])
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', type=Path, action='append', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(render(args.attempt, args.output))
