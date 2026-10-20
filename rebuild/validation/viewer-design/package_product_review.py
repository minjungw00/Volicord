#!/usr/bin/env python3
"""Package unchanged Product exports with answer-free tasks and explicit origins."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import os
import subprocess
import review_boundary as boundary


# No case outcome, rationale, failure, code location or expected conclusion.
# Source-first expectations belong to a separate private evaluator input.
READER_TASKS = (
    '한 Work를 골라 무엇을 하려던 작업인지, 어떤 접근과 결과가 기록되어 있는지 자신의 말로 설명해 주세요.',
    '작업의 이유나 원인, 확인한 범위와 아직 모르는 점을 구분해 주세요. 그렇게 읽은 문장을 표시해 주세요.',
    '검증 화면에서 어떤 실행과 결과가 연결되는지 설명해 주세요. 서로 다른 기록을 비교할 수 있는지 판단한 근거를 찾아 주세요.',
    'Work에 관련 Decision이 있으면 제안과 실제 선택, 각각의 이유를 설명해 주세요. 관련 기록이 없으면 없는 상태를 유지해 주세요.',
    '주장 하나의 근거를 열고 해당 범위를 확인한 다음 읽던 문장으로 돌아와 주세요. 어려웠던 동작이나 문장을 적어 주세요.',
    'Work가 있는 프로젝트와 Work가 없는 프로젝트의 저장소 설명을 각각 읽고, 무엇을 이해할 수 있고 어디까지 확인할 수 있는지 설명해 주세요.',
    '데스크톱과 좁은 화면에서 문서 탐색, 코드 읽기와 복귀를 해 보고 이해하거나 조작하기 어려웠던 점을 적어 주세요.',
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return boundary.hash_file(path)


def case_scope(case):
    classification, origin = case.get('evidence_class'), case.get('origin')
    if classification not in {'normal', 'record-only', 'degraded', 'synthetic-control'} or not isinstance(origin, str) or not origin.strip():
        raise ValueError('case requires explicit evidence class and origin; no implied normal-content success')
    key = case['key']
    if not isinstance(key, str) or not key or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-_' for c in key):
        raise ValueError('unsafe review case key')
    return classification, origin


def reader_entry(cases):
    escape = html.escape
    entries = []
    for case in cases:
        classification, origin = case_scope(case)
        prefix = f"product/{case['key']}-offline.html"
        links = [('개요', 'overview'), ('저장소 설명', 'repository-explanation'),
            ('작업 목록', 'works')]
        if case.get('work'):
            links.append(('선택한 Work', 'work-' + case['work']))
        entries.append('<article><h2>' + escape(case['project_name']) + ' · ' + escape(case['key'])
            + '</h2><p>' + escape(classification) + ' · ' + escape(origin) + '</p>'
            + ''.join(f'<a href="{prefix}#{escape(fragment)}">{escape(label)}</a> ' for label, fragment in links) + '</article>')
    return '''<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>Volicord 읽기 검토</title><style>body{font:16px/1.8 system-ui,sans-serif;margin:24px;color:#1c2b40;background:#f3f5f9}main{max-width:1000px;margin:auto}article{padding:24px;margin:24px 0;background:white}a{display:inline-block;padding:8px;color:#2357a0;overflow-wrap:anywhere}</style>
<main><h1>실제 Product 읽기·탐색 검토</h1><p>아래 문서는 현재 후보가 내보낸 변경 없는 script-free 파일입니다. 인터넷이나 서버 없이 열 수 있습니다. 각 사례의 origin과 자료 한계를 함께 읽어 주세요.</p>
<p>이전 전달 UI에 대한 거절은 보존됩니다. 이 전달물의 사람 독해·디자인 수용·보조기술·native zoom은 실제 관찰 전까지 pending입니다. 에이전트와 브라우저 검사로 승인하지 않습니다.</p>
<p>normal은 현재 읽기 가능한 콘텐츠라는 표시입니다. 새 작업의 자동 수집·새 생성·의미 정확성·사용자 수락을 뜻하지 않습니다. record-only와 degraded도 각 origin의 한계를 유지합니다.</p>
<p>먼저 문서만 읽고 자신의 답을 남겨 주세요. 기술 결과는 그 다음에 확인합니다. source-first 기대와 평가 정답은 이 패키지에 포함하지 않습니다.</p><ol>''' \
        + ''.join('<li>' + escape(task) + '</li>' for task in READER_TASKS) + '</ol>' \
        + ''.join(entries) \
        + '<p><a href="technical.html">독해 후 기술 결과 확인</a> · <a href="manifest.json">정확한 전달물 identity</a></p></main></html>'


def verify_package(root, expected_candidate=None):
    return boundary.verify_package(root, expected_candidate)


def copy_product_outputs(observed, output, cases):
    """Deliver static Product exports/screenshots and sanitized numeric results only."""
    meta = json.loads(boundary.read_member(observed, 'result.json'))
    bindings = meta.get('files')
    if not isinstance(bindings, dict):
        raise ValueError('review driver must bind output files before packaging')
    allowed = set()
    pages = ('overview', 'repository', 'works', 'work', 'decision', 'source',
        'status', 'documents', 'memory', 'evidence', 'offline-overview',
        'offline-work', 'offline-repository', 'offline-source')
    for case in cases:
        case_scope(case)
        key = case['key']
        allowed.add(key + '-offline.html')
        for width in (1440, 1024, 768, 390, 320):
            allowed.update(f'{key}-{page}-{width}.png' for page in pages)
    boundary.copy_bound(observed, output, allowed, bindings)
    boundary.write_new(output, 'result.json', json.dumps(boundary.sanitized_result(meta), ensure_ascii=False, indent=2).encode())


def copy_supplement(source, destination):
    meta = json.loads(boundary.read_member(source, 'result.json'))
    bindings = meta.get('files')
    if not isinstance(bindings, dict):
        raise ValueError('supplement must bind reviewed output files')
    allowed = {'review.html', 'product-snapshot.html'}
    for name in bindings:
        boundary.relative(name)
        if len(Path(name).parts) == 1 and name.endswith('.png'):
            allowed.add(name)
    boundary.copy_bound(source, destination, allowed, bindings)
    boundary.write_new(destination, 'result.json', json.dumps(boundary.sanitized_result(meta), ensure_ascii=False, indent=2).encode())


def public_case(case):
    return {key: value for key, value in case.items() if key in {
        'key', 'project_name', 'project', 'work', 'decision', 'evidence_class', 'origin', 'language', 'locale'}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--reference-manifest', type=Path)
    parser.add_argument('--bundle', type=Path)
    parser.add_argument('--inspection-review', type=Path)
    parser.add_argument('--citation-review', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--archive', type=Path, help='Optional new sanitized ZIP; verified without extraction')
    parser.add_argument('--early', action='store_true', help='Explicit dirty-source preview; no clean-candidate claim')
    args = parser.parse_args()
    config = json.loads(boundary.read_member(args.config.absolute().parent, args.config.name))
    observed = Path(config['output'])
    result = json.loads(boundary.read_member(observed, 'result.json'))
    require(result['outcome'] == 'passed', 'review precondition failed')
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    require(result['candidate'] == head, 'review belongs to another candidate')
    require(result['config_sha256'] == digest(args.config), 'review config changed')
    require(result['executable_sha256'] == digest(Path(config['viewer'])), 'reviewed viewer changed')
    require(result['cli_sha256'] == digest(Path(config['cli'])), 'reviewed CLI changed')
    require(result['driver_sha256'] == digest(Path(__file__).with_name('product_review.cjs')), 'review driver changed')
    require(result['controls_sha256'] == digest(Path(__file__).with_name('product_review_controls.cjs')), 'review controls changed')
    require(len(result['cases']) == len(config['cases']), 'review case coverage changed')
    for case in config['cases']:
        case_scope(case)
        matches = [r for r in result['cases'] if r['key'] == case['key']]
        require(len(matches) == 1 and all(matches[0].get(k) == case.get(k)
            for k in ('project', 'work', 'decision', 'evidence_class', 'origin')), 'case identity/origin changed')
    if not args.early:
        require(result['diff_sha256'] == hashlib.sha256(b'').hexdigest(), 'reviewed tracked source was dirty')
        require(not subprocess.check_output(['git', 'status', '--porcelain', '--', 'rebuild']), 'current source is dirty')
    if not config['cases'] or len({c['key'] for c in config['cases']}) != len(config['cases']):
        raise ValueError('review requires unique explicit cases')
    output = args.output.absolute()
    os_fd = boundary.directory(output.parent)
    os.close(os_fd)
    for source in [observed, args.baseline, args.inspection_review, args.citation_review]:
        if source is None: continue
        fd = boundary.directory(source); os.close(fd)
        root = source.absolute()
        if output == root or output.is_relative_to(root) or root.is_relative_to(output):
            raise ValueError('review output must be separate from retained evidence')
    boundary.new_directory(output)
    copy_product_outputs(observed, output / 'product', config['cases'])
    for case in config['cases']:
        require((output / 'product' / (case['key'] + '-offline.html')).is_file(), 'missing Product export')
    # Historical visual references stay separate from the first reading.
    if args.baseline:
        names = [p.name for p in args.baseline.iterdir() if p.suffix == '.png']
        bindings = {name: {'sha256': hashlib.sha256(boundary.read_member(args.baseline, name)).hexdigest(), 'bytes': len(boundary.read_member(args.baseline, name))} for name in names}
        boundary.copy_bound(args.baseline, output / 'comparison', names, bindings)
    if args.reference_manifest:
        reference = json.loads(boundary.read_member(args.reference_manifest.absolute().parent, args.reference_manifest.name))
        selected = {}
        for name, metadata in reference['files'].items():
            original = boundary.read_member(args.reference_manifest.absolute().parent, name)
            if hashlib.sha256(original).hexdigest() != metadata['sha256'] or len(original) != metadata['bytes']:
                raise ValueError('changed reference')
            if name.endswith('.png'): selected[name] = metadata
        boundary.write_new(output, 'selected-reference-manifest.json', json.dumps({'files': selected}, ensure_ascii=False, indent=2).encode())
    boundary.write_new(output, 'index.html', reader_entry(config['cases']).encode())
    normal_basis = config.get('normal_source_rich_article', 'not_observed; no genuine positive supplied')
    technical = '<!doctype html><meta charset="utf-8"><h1>Technical observations</h1><p>Candidate: ' + head + '</p><a href="product/result.json">Browser results and measurements</a><p>This packager attests technical file bindings only; independent semantic review requires separate evidence. Human acceptance: pending. Normal source-rich prospective article: ' + html.escape(normal_basis) + '. No Final/V11 or Phase 8 qualification.</p><p>Offline files retain only the evidence admitted at publication. Original Source retention is unchanged; later live reads must recheck it. These files grant no live query, write, host or provider authority.</p>'
    supplements = {}
    for name, source in [('inspection', args.inspection_review), ('citation', args.citation_review)]:
        if source is None:
            continue
        meta = json.loads(boundary.read_member(source, 'result.json'))
        require(meta.get('outcome', meta.get('status')) == 'passed' and meta['candidate'] == head, 'supplement candidate/result mismatch')
        require(meta.get('executable_sha256', meta.get('viewer_sha256')) == result['executable_sha256'], 'supplement viewer changed')
        driver = Path(__file__).with_name('inspection_browser.cjs') if name == 'inspection' else Path(__file__).parents[1] / 'end-to-end/multi-repository/citation_reading_browser.cjs'
        require(meta['driver_sha256'] == digest(driver), 'supplement driver changed')
        if not args.early:
            require(meta.get('worktree_diff_sha256', meta.get('diff_sha256')) == hashlib.sha256(b'').hexdigest(), 'supplement source was dirty')
        destination = output / name
        copy_supplement(source, destination)
        supplements[name] = {'result_sha256': digest(source / 'result.json'), 'classification': 'synthetic technical/browser controls only'}
        technical += f'<p><a href="{name}/result.json">{name}: synthetic technical controls</a></p>'
        if (destination / 'review.html').exists():
            technical += f'<a href="{name}/review.html">{name} inspection screenshots and offline subset</a>'
    for p in sorted((output / 'product').glob('*.png')):
        name = p.relative_to(output).as_posix()
        technical += f'<p><a href="{html.escape(name)}">{html.escape(p.name)}</a></p>'
    boundary.write_new(output, 'technical.html', technical.encode())
    manifest = {'candidate': head, 'classification': 'current Product consumer review; per-case origin limits apply',
        'human_acceptance': 'pending', 'independent_review': 'not_attested_by_packager', 'source_first_answer_keys': 'excluded',
        'normal_source_rich_article': normal_basis,
        'prior_delivered_ui_feedback': 'rejected; original evidence unchanged',
        'source_status': 'early dirty-source output' if args.early else 'clean committed candidate',
        'source_diff_sha256': result['diff_sha256'], 'source_review_result_sha256': digest(observed / 'result.json'),
        'selected_reference_manifest_sha256': digest(args.reference_manifest) if args.reference_manifest else None,
        'canonical_bundle_sha256': digest(args.bundle) if args.bundle else None,
        'cases': [public_case(c) for c in config['cases']], 'supplements': supplements,
        'files': {str(p.relative_to(output)): digest(p) for p in sorted(output.rglob('*')) if p.is_file()}}
    boundary.write_new(output, 'manifest.json', json.dumps(manifest, ensure_ascii=False, indent=2).encode())
    verify_package(output, head)
    if args.archive:
        boundary.write_archive(output, args.archive, head)
    print(f'Actual Product review package: {output / "index.html"}')


if __name__ == '__main__':
    main()
