"""Manual extractive paragraph reordering, never autonomous rewriting or approval."""
import copy
import json
from pathlib import Path

from inputs import binding, check_binding, digest, encoded, require
from reader_presentation import paragraphs
from source_reading import DISPLAY_BYTES


def transform(response, order):
    spans = paragraphs(response['prose'])
    require(len(spans) >= 3, 'no paragraph sequence to edit')
    require(isinstance(order, list) and all(type(n) is int for n in order)
            and sorted(order) == list(range(len(spans))), 'every original paragraph must occur exactly once')
    require(order != list(range(len(spans))), 'editorial order is unchanged')
    require(order[0] == 0 and order[-1] == len(spans)-1, 'problem and terminal direction must remain in place')
    require(len(response['prose'].encode()) <= DISPLAY_BYTES, 'editorial original exceeds complete reading bound')
    new = copy.deepcopy(response)
    new['prose'] = ''.join(spans[n][2] for n in order)
    require(new['prose'] != response['prose'], 'editorial reorder has no visible text change')
    starts, cursor = {}, 0
    for n in order:
        starts[n] = cursor
        cursor += len(spans[n][2].encode())
    mappings = []
    for index, (claim, revised) in enumerate(zip(response['claims'], new['claims'])):
        containing = [n for n, (start, end, _) in enumerate(spans)
                      if start <= claim['start'] < claim['end'] <= end]
        require(len(containing) == 1, 'claim crosses paragraph boundary; manual rewriting unsupported')
        n = containing[0]; start = spans[n][0]
        revised['start'] = starts[n] + claim['start'] - start
        revised['end'] = starts[n] + claim['end'] - start
        old_bytes = response['prose'].encode()[claim['start']:claim['end']]
        new_bytes = new['prose'].encode()[revised['start']:revised['end']]
        require(old_bytes == new_bytes, 'edited claim bytes differ')
        mappings.append({'claim': index, 'original': [claim['start'], claim['end']],
                         'revised': [revised['start'], revised['end']], 'text_sha256': digest(new_bytes),
                         'selections': revised['selections'], 'authority': revised.get('authority')})
    return new, mappings


def original(attempt):
    from render_comparison import card
    record = json.loads(Path(attempt).read_bytes())
    require(record.get('generation_output') is not None, 'editorial needs an original final output')
    require(record.get('output_contract') in {'reader_oriented', 'work_directed_reader'}, 'editorial needs original claim bindings')
    _, witness = card(attempt, 'Editorial original verification')
    output = record['generation_output']
    current = next(o for o in witness['outputs'] if o['original'] == output)
    require(current['derived_reading']['status'] == 'valid_binding', 'editorial original binding invalid or unavailable')
    require(current.get('reading', {}).get('status') == 'valid_binding', 'editorial original reading invalid')
    return record, json.loads(Path(output['path']).read_bytes()), current


def basis_for(attempt, display, reason):
    from render_comparison import verify_presentation
    require(isinstance(reason, str) and reason.strip(), 'editorial need must be recorded')
    display = Path(display).resolve()
    integrity_path = display.parent / 'integrity.json'
    integrity = json.loads(integrity_path.read_bytes())
    require(integrity.get('presentation_mode') == 'reader' and not integrity.get('editorial'),
            'transformation basis must be presentation-only')
    check_binding(integrity['presentation'])
    require(integrity['presentation']['path'] == str(display), 'transformation basis display path changed')
    require(len(integrity['samples']) == 1 and integrity['samples'][0]['attempt'] == binding(attempt),
            'transformation basis concerns another original Work/output')
    record = json.loads(Path(attempt).read_bytes())
    require([o['original'] for o in integrity['samples'][0]['outputs']] == record['original_outputs'],
            'transformation basis original outputs differ')
    verify_presentation(display)
    return {'reason': reason, 'presentation': binding(display), 'integrity': binding(integrity_path),
            'assessment': 'agent editorial hypothesis; human comprehension unassessed'}


def create(attempt, order, editor, display, reason, output):
    require(isinstance(editor, str) and editor.strip(), 'manual editor identity required')
    record, response, _ = original(attempt)
    basis = basis_for(attempt, display, reason)
    revised, claims = transform(response, order)
    output = Path(output).resolve(); output.mkdir(mode=0o700, parents=True, exist_ok=False)
    prose = output / 'prose.txt'; prose.write_bytes(revised['prose'].encode())
    response_path = output / 'response.json'; response_path.write_bytes(encoded(revised))
    value = {'format_version': 1, 'kind': 'manual_paragraph_reordering',
             'editor': {'name': editor, 'authority': 'manual editorial selection; self-reported identity'},
             'attempt': binding(attempt), 'original_response': record['generation_output'],
             'input': record['input'], 'scope': record['scope'], 'transformation_basis': basis,
             'paragraph_order': order, 'response': binding(response_path), 'prose': binding(prose),
             'revised_claims': claims, 'verifier': binding(Path(__file__)),
             'verification': {'transform': 'lossless extractive reorder with recomputed UTF-8 claim offsets',
                              'semantic_assessment': 'not_assessed', 'H1': 'pending'}}
    path = output / 'editorial.json'; path.write_bytes(encoded(value))
    read(attempt, path)
    return path


def read(attempt, path):
    value = json.loads(Path(path).read_bytes())
    require(value['format_version'] == 1 and value['kind'] == 'manual_paragraph_reordering', 'unknown editorial transformation')
    require(value['attempt'] == binding(attempt), 'editorial attempt changed')
    require(value['editor']['name'].strip() and value['editor']['authority'] == 'manual editorial selection; self-reported identity', 'editor authority changed')
    check_binding(value['verifier']); require(value['verifier'] == binding(Path(__file__)), 'editor verifier changed')
    record, response, current = original(attempt)
    require(value['original_response'] == record['generation_output'] and value['input'] == record['input']
            and value['scope'] == record['scope'], 'editorial original/input/scope changed')
    basis = value['transformation_basis']
    check_binding(basis['presentation']); check_binding(basis['integrity'])
    require(basis == basis_for(attempt, basis['presentation']['path'], basis['reason']), 'editorial transformation basis changed')
    require(value['verification'] == {'transform': 'lossless extractive reorder with recomputed UTF-8 claim offsets',
            'semantic_assessment': 'not_assessed', 'H1': 'pending'}, 'editorial verification authority changed')
    expected, claims = transform(response, value['paragraph_order'])
    check_binding(value['response']); check_binding(value['prose'])
    require(Path(value['response']['path']).read_bytes() == encoded(expected), 'unsupported editorial prose, claims or Source substitution')
    require(Path(value['prose']['path']).read_bytes() == expected['prose'].encode(), 'editorial prose bytes differ')
    require(value['revised_claims'] == claims, 'editorial revised claim coordinates changed')
    return expected, value
