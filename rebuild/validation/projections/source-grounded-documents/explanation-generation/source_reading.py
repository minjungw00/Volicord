"""Derived local locators and reading HTML, never semantic source selection."""
from __future__ import annotations

import ast
import base64
import html
import json
import re
from pathlib import Path, PurePosixPath

from inputs import check_binding, digest, encoded, require

DISPLAY_BYTES = 16384
DISPLAY_SELECTIONS = 128
TRANSPORT_BYTES = 2 * 1024 * 1024


def escaped(value):
    return html.escape(str(value), quote=True)


def preview(text):
    data = text.encode('utf-8', errors='backslashreplace')
    text = data.decode('utf-8')
    if len(data) <= DISPLAY_BYTES:
        return escaped(text)
    prefix = data[:DISPLAY_BYTES].decode('utf-8', errors='ignore')
    return escaped(prefix) + '\n[Display bound; exact complete bytes in diagnostic download]'


def download(data, name, label):
    return ('<a download="' + escaped(name) + '" href="data:application/octet-stream;base64,'
            + base64.b64encode(data).decode('ascii') + '">' + escaped(label) + '</a>')


def anchor(scope, original, number):
    # Independent of randomized sample labels and other displayed Works.
    return output_anchor(scope, original) + '-selection-' + str(number)


def output_anchor(scope, original):
    return ('work-' + digest(encoded(scope))[:20] + '-attempt-' + original.get('attempt_sha256', 'unbound')
            + '-output-' + original['sha256'])


def repository_path(value):
    require(isinstance(value, str) and value and '\\' not in value and '\x00' not in value,
            'repository-relative path unavailable')
    path = PurePosixPath(value)
    require(not path.is_absolute() and all(p not in {'', '.', '..'} for p in value.split('/'))
            and ':' not in value, 'repository-relative path unavailable')
    return value


def position(data, offset):
    """One-based lines/scalar columns; half-open end, LF or CRLF, no tab expansion."""
    prefix = data[:offset].decode('utf-8')  # Also verifies the requested boundary.
    line = prefix.count('\n') + 1
    tail = prefix.rsplit('\n', 1)[-1]
    return [line, len(tail) + 1]


def callable_at(path, data, start, end):
    """Python AST only; an enclosing declaration is syntax, not claim relevance."""
    if not path.endswith('.py') or len(data) > TRANSPORT_BYTES:
        return None
    try:
        tree = ast.parse(data.decode('utf-8'))
        if re.search(rb'\r(?!\n)', data):
            return None  # Python normalizes lone CR; our locator convention uses LF.
        lines = data.split(b'\n')
        bases, offset = [], 0
        for line in lines:
            bases.append(offset); offset += len(line) + 1
        found = []

        def visit(node, parents):
            named = isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))
            names = parents + [node.name] if named else parents
            if named and node.end_lineno is not None:
                left = bases[node.lineno - 1] + node.col_offset
                right = bases[node.end_lineno - 1] + node.end_col_offset
                if left <= start < end <= right:
                    found.append((right - left, {'name': '.'.join(names), 'start': left, 'end': right,
                                                'basis': 'Python AST of verified whole-file bytes'}))
            for child in ast.iter_child_nodes(node):
                visit(child, names)

        visit(tree, [])
        return min(found, key=lambda item: item[0])[1] if found else None
    except (SyntaxError, UnicodeError, RecursionError, ValueError):
        return None


def locate(spec, entry, result):
    selection = result['selection']
    require(result['reference_status'] == 'valid_reference', 'invalid provenance; no verified locator')
    require(entry['project'] == spec['scope']['project'] and entry['work'] == spec['scope']['work'],
            'foreign Work/Project')
    require(entry['chronology']['state'] == 'known', 'ambiguous chronology')
    actual = ('before' if entry['attribution'] == 'explicit_before_patch' else
              'after' if entry['attribution'] == 'explicit_work_patch' else 'context')
    require(selection['state'] == actual, 'wrong-state span')
    require(entry['asset'] is not None, 'unavailable source')
    check_binding(entry['asset'])
    data = Path(entry['asset']['path']).read_bytes()
    start, end = selection['start'], selection['end']
    require(type(start) is int and type(end) is int and 0 <= start < end <= len(data), 'invalid byte span')
    require(digest(data[start:end]) == selection['sha256'], 'span hash mismatch')
    data[start:end].decode('utf-8')
    result = {'path': entry['path'], 'state': actual, 'text': data[start:end].decode('utf-8'),
              'bytes': data[start:end], 'start': None, 'end': None, 'callable': None,
              'coordinate_basis': 'asset-relative bytes; absolute repository coordinates unavailable'}
    if entry['role'] == 'source':
        result['path'] = repository_path(entry['path'])
        if entry['representation'] in {'full_file', 'verified_reconstruction'}:
            require(digest(data) == entry['file_sha256'], 'full-file hash mismatch')
            if entry['representation'] == 'verified_reconstruction':
                proof = entry['proof']
                require(proof['expected_sha256'] == digest(data) and proof['independent_witnesses'],
                        'unverified reconstruction')
                for witness in proof['independent_witnesses']:
                    check_binding(witness)
            data.decode('utf-8')
            result.update(start=position(data, start), end=position(data, end),
                          coordinate_basis='whole-file; one-based Unicode scalar columns; half-open end',
                          callable=callable_at(entry['path'], data, start, end))
    return result


def location_label(location, selection):
    label = location['path'] + ' · ' + location['state']
    if location['start'] is not None:
        label += ' · ' + ':'.join(map(str, location['start'])) + '–' + ':'.join(map(str, location['end']))
    else:
        label += f" · asset bytes [{selection['start']},{selection['end']}); absolute lines unavailable"
    if location['callable']:
        label += ' · ' + location['callable']['name']
    return label


def prose_html(prose, rows):
    # Only exact literal sidecar citations establish a navigational association.
    # The underlying text stays unedited; CSS supplies the human locator label.
    lookup = {}
    for row in rows:
        selection = row['selection']
        if isinstance(selection, dict) and set(selection) >= {'id', 'start', 'end'}:
            key = (selection['id'], selection['start'], selection['end'])
            if all(isinstance(v, (str, int)) for v in key):
                lookup.setdefault(key, row)
    text = prose.encode()[:DISPLAY_BYTES].decode('utf-8', errors='ignore')
    parts, cursor = [], 0
    pattern = re.compile(r'([a-z0-9-]+)\s*\[(\d+),(\d+)\)')
    for match in pattern.finditer(text):
        row = lookup.get((match[1], int(match[2]), int(match[3])))
        if row is None:
            continue
        parts.append(escaped(text[cursor:match.start()]))
        parts.append('<a class="citation" href="#' + row['anchor'] + '" aria-label="'
                     + escaped(row['label']) + '" data-location="' + escaped(row['label']) + '">'
                     + '<span>' + escaped(match[0]) + '</span></a>')
        cursor = match.end()
    parts.append(escaped(text[cursor:]))
    if text != prose:
        parts.append('\n[Display bound; complete prose in original response download]')
    return '<pre class="prose">' + ''.join(parts) + '</pre>'


def selections_html(spec, entries, validation, original):
    rows, groups, diagnostics, witnesses = [], {}, [], []
    source_bytes = 0
    for number, result in enumerate(validation['selections'], 1):
        selection = result['selection']
        target = anchor(spec['scope'], original, number)
        witness = {'anchor': target, 'selection': selection, 'reference_status': result['reference_status']}
        witnesses.append(witness)
        if number > DISPLAY_SELECTIONS:
            continue
        location, error = None, None
        try:
            entry = entries[selection['id']]
            require(type(selection['start']) is int and type(selection['end']) is int,
                    'invalid byte span')
            length = selection['end'] - selection['start']
            require(length <= 65536 and source_bytes + max(0, length) <= TRANSPORT_BYTES,
                    'source display byte bound; original selection preserved in diagnostic download')
            location = locate(spec, entry, result)
            source_bytes += length
        except (ValueError, KeyError, TypeError, OSError) as failure:
            error = str(failure)
        label = location_label(location, selection) if location else 'Invalid reference · selection ' + str(number)
        rows.append({'anchor': target, 'selection': selection, 'label': label, 'location': location,
                     'entry': entry if location else None})
        body = '<details class="selection" id="' + target + '"><summary>' + escaped(label) + '</summary>'
        if location:
            body += '<p>' + escaped(location['coordinate_basis']) + '</p><pre class="code">' + preview(location['text']) + '</pre>'
            body += download(location['bytes'], 'selection-' + str(number) + '.bin', 'Exact selected bytes')
            if location['callable']:
                body += '<p>Enclosing declaration: ' + escaped(location['callable']) + '; relevance not assessed.</p>'
        else:
            body += '<p>Selected bytes withheld from verified presentation. No substitute code. ' + escaped(error) + '</p>'
        body += '<p><a href="#' + output_anchor(spec['scope'], original) + '">Back to generated explanation</a></p>'
        body += '<details><summary>Original selection and provenance</summary><pre class="selection-request">'
        body += preview(json.dumps(selection, ensure_ascii=False, sort_keys=True)) + '</pre>'
        body += '<p>Reference: ' + escaped(result['reference_status']) + '; ' + escaped('; '.join(result['issues'])) + '</p>'
        if location:
            body += '<pre>' + preview(json.dumps({k: entry.get(k) for k in
                                ('id', 'locator', 'file_sha256', 'representation', 'extent', 'attribution', 'chronology')}, ensure_ascii=False)) + '</pre>'
        body += '</details></details>'
        if location and entry['role'] == 'source':
            key = (location['path'], location['state'], entry['asset']['sha256'])
            groups.setdefault(key, []).append((rows[-1], body))
        else:
            diagnostics.append(body)
    source = '<p>Source navigation preserves generator selections. Grouping does not establish importance or prose support.</p>'
    source += '<nav aria-label="Verified source locations">' + ' · '.join(
        '<a href="#' + group[0][0]['anchor'] + '">' + escaped(key[0] + ' · ' + key[1]) + '</a>'
        for key, group in groups.items()) + '</nav>'
    for key, group in groups.items():
        source += '<details><summary>' + escaped(key[0] + ' · ' + key[1]) + ' (' + str(len(group)) + ' selections)</summary>'
        source += ''.join(body for _, body in group) + '</details>'
    if not groups:
        source += '<p>No verified repository source selected. No code change is inferred.</p>'
    if len(witnesses) > DISPLAY_SELECTIONS:
        source += '<p>Display bound: ' + str(len(witnesses) - DISPLAY_SELECTIONS) + ' selections available only in the complete original response download.</p>'
    diagnostic = '<details class="diagnostic"><summary>Diagnostic evidence selections (' + str(len(diagnostics)) + ' displayed)</summary>'
    diagnostic += ''.join(diagnostics) + '</details>'
    return source, diagnostic, rows, witnesses
