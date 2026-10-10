"""Derived local locators and reading HTML, never semantic source selection."""
from __future__ import annotations

import ast
import base64
import difflib
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


def public_receipt(value):
    """Retain byte identities without disclosing private filesystem bindings."""
    if isinstance(value, dict):
        return {key: public_receipt(item) for key, item in value.items()
                if not (key == 'path' and {'bytes', 'sha256'} <= value.keys())}
    if isinstance(value, list):
        return [public_receipt(item) for item in value]
    return value


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
            + '-output-' + original['sha256'] + '-response-' + str(original.get('output_sequence', 1)))


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


def selections_html(spec, entries, validation, original, *, primary_indices=()):
    rows, groups, diagnostics, witnesses = [], {}, [], []
    source_bytes = 0
    displayed = set(primary_indices)
    require(len(displayed) <= DISPLAY_SELECTIONS, 'primary display bound')
    for index in range(len(validation['selections'])):
        if len(displayed) >= DISPLAY_SELECTIONS:
            break
        displayed.add(index)
    primary_reserve = {}
    for index in primary_indices:
        selection = validation['selections'][index]['selection']
        length = (selection['end'] - selection['start'] if isinstance(selection, dict)
                  and type(selection.get('start')) is int and type(selection.get('end')) is int else 0)
        primary_reserve[index] = min(65536, max(0, length))
    for number, result in enumerate(validation['selections'], 1):
        selection = result['selection']
        target = anchor(spec['scope'], original, number)
        witness = {'anchor': target, 'selection': selection, 'reference_status': result['reference_status']}
        witnesses.append(witness)
        if number - 1 not in displayed:
            continue
        primary_reserve.pop(number - 1, None)
        location, entry, error = None, None, None
        try:
            entry = entries[selection['id']]
            require(type(selection['start']) is int and type(selection['end']) is int,
                    'invalid byte span')
            length = selection['end'] - selection['start']
            require(length <= 65536 and source_bytes + max(0, length) + sum(primary_reserve.values()) <= TRANSPORT_BYTES,
                    'source display byte bound; original selection preserved in diagnostic download')
            location = locate(spec, entry, result)
            source_bytes += length
        except (ValueError, KeyError, TypeError, OSError) as failure:
            error = str(failure)
        label = (location_label(location, selection) if location else
                 ('Source display gap' if result['reference_status'] == 'valid_reference' else 'Invalid reference')
                 + ' · selection ' + str(number))
        rows.append({'index': number - 1, 'anchor': target, 'selection': selection, 'label': label, 'location': location,
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
        if entry is not None:
            body += '<pre>' + preview(json.dumps({k: entry.get(k) for k in
                                ('id', 'path', 'role', 'locator', 'file_sha256', 'representation', 'extent',
                                 'attribution', 'chronology', 'before_state', 'missing')}, ensure_ascii=False)) + '</pre>'
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


def file_lines(data):
    """LF-based lines retaining original CRLF and a missing final newline."""
    parts = data.decode('utf-8').split('\n')
    return [line + '\n' for line in parts[:-1]] + ([parts[-1]] if parts[-1] else [])


def line_interval(location):
    start, end = location['start'], location['end']
    return start[0] - 1, end[0] - 1 + (end[1] != 1)


def intersects(left, right, span):
    # An insertion just before/after a selection is not a change within it.
    return span[0] < left < span[1] if left == right else left < span[1] and span[0] < right


def comparison_html(spec, rows):
    """Compare classified whole-file states; selected excerpts supply focus only."""
    paths, views, witnesses = {}, [], []
    for row in rows:
        entry = row['entry']
        if entry and entry['role'] == 'source' and row['location']['state'] in {'before', 'after'}:
            paths.setdefault(entry['path'], []).append(row)
    if not paths:
        return ('<p>No verified Work change selected. Context and no-change investigation conclusions '
                'remain in the original prose; no diff is inferred.</p>'), witnesses
    for path, selected in paths.items():
        view = '<details class="comparison"><summary>Source before/after: ' + escaped(path) + '</summary>'
        witness = {'path': path, 'status': 'gap', 'reason': None}
        witnesses.append(witness)
        try:
            states = {state: {row['entry']['id']: row['entry'] for row in selected
                              if row['location']['state'] == state} for state in ('before', 'after')}
            require(len(states['before']) == 1 and len(states['after']) == 1,
                    'one independently verified before and after state required; Missing state is not inferred')
            before, after = next(iter(states['before'].values())), next(iter(states['after'].values()))
            for entry in (before, after):
                require(entry['project'] == spec['scope']['project'] and entry['work'] == spec['scope']['work'],
                        'foreign Work/Project')
                require(entry['chronology']['state'] == 'known', 'ambiguous chronology')
                require(entry['representation'] in {'full_file', 'verified_reconstruction'},
                        'whole-file states unavailable; excerpt extent cannot establish correspondence')
                require(entry['before_state'] == 'available', 'before-state availability unverified')
                check_binding(entry['asset'])
            require(before['locator'] == after['locator'] + ':before' and before['producer'] == after['producer'],
                    'change identity differs; path equality does not establish a pair')
            # Archive classification ties the :before locator to the same observed
            # patch record; no pathname/offset proximity supplies this association.
            require(before['chronology']['observed_at'] == after['chronology']['observed_at'],
                    'change observation identity differs')
            old, new = Path(before['asset']['path']).read_bytes(), Path(after['asset']['path']).read_bytes()
            require(digest(old) == before['file_sha256'] and digest(new) == after['file_sha256'],
                    'whole-file hash changed')
            require(len(old) + len(new) <= 2 * TRANSPORT_BYTES, 'comparison byte bound')
            old_lines, new_lines = file_lines(old), file_lines(new)
            require(len(old_lines) + len(new_lines) <= 20000, 'comparison line bound')
            matcher = difflib.SequenceMatcher(None, old_lines, new_lines)
            opcodes = matcher.get_opcodes()
            spans = {state: [line_interval(row['location']) for row in selected if row['location']['state'] == state]
                     for state in ('before', 'after')}

            def focused(opcode):
                kind, i1, i2, j1, j2 = opcode
                return kind != 'equal' and (any(intersects(i1, i2, span) for span in spans['before']) or
                                            any(intersects(j1, j2, span) for span in spans['after']))

            # Keep unselected edits out even when they are close enough to share
            # an ordinary unified-diff hunk. Merge only adjacent selected edits.
            runs = []
            for index, op in enumerate(opcodes):
                if not focused(op):
                    continue
                if (runs and index == runs[-1][-1] + 2 and opcodes[index - 1][0] == 'equal'
                        and opcodes[index - 1][2] - opcodes[index - 1][1] <= 4):
                    runs[-1].append(index)
                else:
                    runs.append([index])
            retained = []
            for run in runs:
                first, last = run[0], run[-1]
                group = list(opcodes[first:last + 1])
                if first and opcodes[first - 1][0] == 'equal':
                    _, i1, i2, j1, j2 = opcodes[first - 1]
                    count = min(2, i2 - i1)
                    group.insert(0, ('equal', i2 - count, i2, j2 - count, j2))
                if last + 1 < len(opcodes) and opcodes[last + 1][0] == 'equal':
                    _, i1, i2, j1, j2 = opcodes[last + 1]
                    count = min(2, i2 - i1)
                    group.append(('equal', i1, i1 + count, j1, j1 + count))
                retained.append(group)
            kinds = sorted({op[0] for group in retained for op in group if op[0] != 'equal'})
            moves = sum(1 for op in opcodes if op[0] == 'delete' and focused(op)
                        and any(other[0] == 'insert' and focused(other)
                                and old_lines[op[1]:op[2]] == new_lines[other[3]:other[4]] for other in opcodes))
            unchanged = 0
            for row in selected:
                left, right = line_interval(row['location'])
                for kind, i1, i2, j1, j2 in opcodes:
                    first, last = (i1, i2) if row['location']['state'] == 'before' else (j1, j2)
                    if kind == 'equal' and first <= left <= right <= last:
                        unchanged += 1
                        break
            witness.update(status='verified_pair', reason=None, change_locator=after['locator'],
                           before=before['asset'], after=after['asset'],
                           kinds=kinds, exact_block_relocations=moves, unchanged_selected_excerpts=unchanged,
                           displayed_hunks=len(retained),
                           omitted_change_blocks=sum(op[0] != 'equal' and not focused(op) for op in opcodes))
            view += '<p>Verified whole-file states from one Work and change identity. Original selections supply '
            view += 'line focus; independently observed surrounding bytes supply context, not new generator selections.</p>'
            if unchanged:
                view += '<p>Unchanged selected context: ' + str(unchanged) + ' excerpt(s) in equal line blocks. '
                view += 'This does not establish an unchanged file or Work.</p>'
            if not retained:
                view += '<p>No changes within the selected line ranges. No selected-span diff is invented.</p>'
            else:
                labels = {'insert': 'additions', 'delete': 'deletions', 'replace': 'replacements'}
                view += '<p>Change display: ' + escaped(', '.join(labels[kind] for kind in kinds)) + '.</p>'
                if moves:
                    view += '<p>Exact block removal/reinsertion: ' + str(moves) + '; movement of identical bytes, '
                    view += 'not verified symbol continuity or semantic identity.</p>'
                diff = '--- ' + path + ' (before)\n+++ ' + path + ' (after)\n'

                def prefixed(prefix, lines):
                    return ''.join(prefix + line + ('' if line.endswith('\n') else '\n\\ No newline at end of file\n')
                                   for line in lines)

                for group in retained:
                    i1, j1, i2, j2 = group[0][1], group[0][3], group[-1][2], group[-1][4]
                    diff += f'@@ -{i1 + 1},{i2 - i1} +{j1 + 1},{j2 - j1} @@\n'
                    for kind, left, right, new_left, new_right in group:
                        if kind == 'equal':
                            diff += prefixed(' ', old_lines[left:right])
                        if kind in {'delete', 'replace'}:
                            diff += prefixed('-', old_lines[left:right])
                        if kind in {'insert', 'replace'}:
                            diff += prefixed('+', new_lines[new_left:new_right])
                view += '<pre class="diff">' + preview(diff) + '</pre>'
                view += download(diff.encode(), 'source-comparison.diff', 'Complete focused source diff')
            if witness['omitted_change_blocks']:
                view += '<p>Unselected change blocks omitted from focus: ' + str(witness['omitted_change_blocks']) + '.</p>'
        except (ValueError, KeyError, TypeError, OSError) as error:
            witness['reason'] = str(error)
            view += '<p>Comparison gap: ' + escaped(error) + '. Verified original excerpts remain available above.</p>'
        view += '<details><summary>Comparison basis and omissions</summary><pre>'
        # Bindings retain the exact assets without exposing private absolute paths.
        public = {key: ({k: v for k, v in value.items() if k != 'path'} if key in {'before', 'after'} else value)
                  for key, value in witness.items()}
        view += preview(json.dumps(public, ensure_ascii=False, indent=2)) + '</pre></details></details>'
        views.append(view)
    return ''.join(views), witnesses
