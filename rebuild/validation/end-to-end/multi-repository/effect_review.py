#!/usr/bin/env python3
"""Seal Source-first effect expectations and check an attributed comparison.

This checks review completeness and citation correspondence, NOT entailment.
The reviewer supplies semantic judgments; a supported label cannot compensate
for missing effect-producing implementations. No generation or Product writes.
"""
import argparse
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return json.dumps(value, ensure_ascii=False, indent=2).encode()


def write(path, data):
    with path.open('xb') as stream:
        stream.write(data)


def span(data, start, end):
    require(type(start) is int and type(end) is int and 0 <= start < end <= len(data),
            'invalid half-open UTF-8 range')
    data[:start].decode('utf-8')
    return data[start:end].decode('utf-8')


def freeze(input_path, output):
    output.mkdir(parents=False, exist_ok=False)
    raw = input_path.read_bytes()
    write(output / 'original-input.json', raw)
    basis = json.loads(raw)
    require(basis['classification'] in {'dependent', 'isolated_source_first'}, 'unknown isolation')
    require(basis['scope'] and basis['reviewer'], 'missing scope/reviewer')
    sources = basis['sources']
    require(sources and len(sources) <= 128, 'missing or excessive Source ranges')
    for key, source in sources.items():
        require(key.isascii() and key.replace('-', '').replace('_', '').isalnum(), 'invalid range key')
        original = Path(source['path']).read_bytes()
        require(digest(original) == source['file_sha256'], 'original file changed')
        text = span(original, source['start_byte'], source['end_byte'])
        require(len(text.encode()) <= 16384, 'Source range exceeds copy bound')
        receipt = Path(source['read_receipt']).read_bytes()
        read = json.loads(receipt)
        binding = read['citation_basis']
        detail = read['observation']['detail']
        require(binding['source_id'] == source['source_id']
                and binding['source_revision'] == source['source_revision']
                and detail['sha256'] == source['file_sha256'], 'wrong Source receipt')
        body = detail['body'].encode()
        base = detail.get('region', {}).get('start_byte', 0)
        require(source['start_byte'] >= base
                and span(body, source['start_byte'] - base, source['end_byte'] - base) == text,
                'original differs from permitted Source bytes')
        source.update(range_sha256=digest(text.encode()), receipt_sha256=digest(receipt),
                      citation_basis=binding)
        write(output / (key + '.source'), text.encode())
        write(output / (key + '.receipt.json'), receipt)
    ids = set()
    for expectation in basis['expectations']:
        require(expectation['id'] not in ids, 'duplicate expectation')
        ids.add(expectation['id'])
        require(expectation['condition'] and expectation['effect'] and expectation['non_effects'],
                'effect needs conditions, effects and bounded non-effects')
        require(expectation['evidence_scope'] in {'static', 'reported', 'observed'}, 'unknown evidence scope')
        require(expectation['required_ranges'] and all(k in sources for k in expectation['required_ranges']),
                'effect implementation/callback ranges missing')
    require(ids, 'no expectations')
    write(output / 'basis.json', encoded(basis))
    write(output / 'seal.json', encoded({p.name: digest(p.read_bytes()) for p in output.iterdir()}))


def verified_basis(directory):
    seal = json.loads((directory / 'seal.json').read_bytes())
    for name, checksum in seal.items():
        require(Path(name).name == name and digest((directory / name).read_bytes()) == checksum,
                'frozen input changed')
    basis = json.loads((directory / 'basis.json').read_bytes())
    for key, source in basis['sources'].items():
        require(digest(Path(source['path']).read_bytes()) == source['file_sha256'],
                'Source changed; reassess on a new basis')
        require(digest((directory / (key + '.source')).read_bytes()) == source['range_sha256']
                and digest((directory / (key + '.receipt.json')).read_bytes()) == source['receipt_sha256'],
                'frozen Source changed')
    return basis


INSTRUCTIONS = ('Compare the identified publication with the supplied original Source expectations. '
                'Read conditions, effect-producing callees and callbacks, and bounded non-effects. '
                'For every expectation, select exact UTF-8 claim spans in every applicable unit, '
                'including headings and synopsis. Return supported, contradicted or unestablished '
                'with reasons and evidence scope. If not asserted, explain that omission separately. '
                'Record identity and isolation limits. A binding or caller citation proves no effect. '
                'Do not infer device observation from static code or a reported check.\n')


def prepare(basis_dir, publication, output):
    output.mkdir(parents=False, exist_ok=False)
    basis = verified_basis(basis_dir)
    raw = publication.read_bytes()
    value = json.loads(raw)
    require(value['explanation_state'] == 'current' and value['provenance'], 'publication is not current')
    write(output / 'publication.json', raw)
    write(output / 'basis.json', encoded(basis))
    for key in basis['sources']:
        write(output / (key + '.source'), (basis_dir / (key + '.source')).read_bytes())
    write(output / 'instructions.txt', INSTRUCTIONS.encode())
    write(output / 'inputs.json', encoded({
        'classification': 'dependent_source_comparison',
        'limits': 'No presentation-first input isolation or human acceptance is established.',
        'basis_directory': str(basis_dir.resolve()),
        'files': {p.name: digest(p.read_bytes()) for p in output.iterdir()}}))


def assess(directory, response):
    # Save first response before interpreting it, including malformed/failed attempts.
    write(directory / 'first-response.json', response.read_bytes())
    failures = []
    try:
        inputs = json.loads((directory / 'inputs.json').read_bytes())
        for name, checksum in inputs['files'].items():
            require(Path(name).name == name and digest((directory / name).read_bytes()) == checksum,
                    'assessment input changed')
        basis = verified_basis(Path(inputs['basis_directory']))
        require(encoded(basis) == (directory / 'basis.json').read_bytes(), 'assessment basis changed')
        publication = json.loads((directory / 'publication.json').read_bytes())
        review = json.loads((directory / 'first-response.json').read_bytes())
        require(review['reviewer'] and review['isolation'] == 'dependent', 'unsupported isolation claim')
        require(review['publication_sha256'] == digest((directory / 'publication.json').read_bytes()),
                'wrong publication/language/revision')
        expectations = {e['id']: e for e in basis['expectations']}
        require(set(review['expectations']) == set(expectations), 'unreviewed or foreign expectation')
        for key, expectation in expectations.items():
            findings = review['expectations'][key]
            require(findings, 'missing expectation comparison')
            for finding in findings:
                require(finding['reason'], 'missing comparison reason')
                if finding['verdict'] == 'not_asserted':
                    failures.append(key + ': required explanation absent')
                    continue
                index = finding['unit_index']
                require(type(index) is int and 0 <= index < len(publication['prose']), 'invalid unit index')
                unit = publication['prose'][index]
                start, end = finding['start_byte'], finding['end_byte']
                claim = span(unit['text'].encode(), start, end)
                require(claim == finding['quote'], 'comparison quote changed')
                if finding['verdict'] != 'supported':
                    require(finding['verdict'] in {'contradicted', 'unestablished'}, 'unknown verdict')
                    failures.append(key + ': ' + finding['verdict'] + ': ' + finding['reason'])
                    continue
                require(finding['evidence_scope'] == expectation['evidence_scope'], 'evidence scope inflated')
                for required in expectation['required_ranges']:
                    source = basis['sources'][required]
                    # Claim-overlapping exact same-Source citations must include the
                    # implementation range. Other claims in the same file cannot pay for it.
                    matches = [c for c in unit['citations']
                               if c['source_id'] == source['source_id']
                               and c['source_revision'] == source['source_revision']
                               and c['analysis_snapshot_id'] == source['citation_basis']['analysis_snapshot_id']
                               and c['repository_snapshot_id'] == source['citation_basis']['repository_snapshot_id']
                               and c['observation_fingerprint'] == source['citation_basis']['observation_fingerprint']
                               and c['repository_source_id'] == source['citation_basis']['repository_source_id']
                               and c['start_byte'] <= source['start_byte']
                               and c['end_byte'] >= source['end_byte']
                               and c['claim_start_byte'] <= start and c['claim_end_byte'] >= end]
                    if not matches:
                        failures.append(key + ': missing effect-producing range ' + required)
                    for citation in matches:
                        code = span(Path(source['path']).read_bytes(), citation['start_byte'], citation['end_byte'])
                        bound_claim = span(unit['text'].encode(), citation['claim_start_byte'], citation['claim_end_byte'])
                        require(citation['range_sha256'] == 'sha256:' + digest(code.encode())
                                and citation['claim_sha256'] == 'sha256:' + digest(bound_claim.encode()),
                                'citation code/claim bytes changed')
        result = {'status': 'failed' if failures else 'passed', 'failures': failures,
                  'classification': 'dependent_attributed_comparison',
                  'limit': 'Reviewer judgments are attributed, not independently proven entailment or human acceptance.',
                  'first_response_sha256': digest((directory / 'first-response.json').read_bytes())}
    except (ValueError, KeyError, TypeError, IndexError, OSError) as error:
        result = {'status': 'failed', 'failures': [str(error)]}
    write(directory / 'result.json', encoded(result))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    frozen = sub.add_parser('freeze')
    frozen.add_argument('--input', required=True, type=Path)
    frozen.add_argument('--output', required=True, type=Path)
    prepared = sub.add_parser('prepare')
    prepared.add_argument('--basis', required=True, type=Path)
    prepared.add_argument('--publication', required=True, type=Path,
                          help='Exact public QuestionAnswers JSON, not a draft or HTML rewrite')
    prepared.add_argument('--output', required=True, type=Path)
    assessed = sub.add_parser('assess')
    assessed.add_argument('--attempt', required=True, type=Path)
    assessed.add_argument('--response', required=True, type=Path)
    args = parser.parse_args()
    if args.command == 'freeze':
        freeze(args.input, args.output)
    elif args.command == 'prepare':
        prepare(args.basis, args.publication, args.output)
    else:
        result = assess(args.attempt, args.response)
        print(json.dumps(result, ensure_ascii=False))
        return int(result['status'] != 'passed')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
