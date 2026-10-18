#!/usr/bin/env python3
"""Package unchanged actual Product exports and local visual comparisons for review."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import shutil
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--reference-manifest', type=Path, required=True)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--early', action='store_true', help='Package exact early output with explicit dirty-source status; no clean-candidate claim')
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    observed = Path(config['output'])
    result = json.loads((observed / 'result.json').read_text())
    assert result['outcome'] == 'passed'
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    assert result['candidate'] == head, 'review belongs to another candidate'
    if not args.early:
        assert result['diff_sha256'] == hashlib.sha256(b'').hexdigest(), 'reviewed tracked files were dirty'
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    shutil.copytree(observed, output / 'product')
    (output / 'comparison').mkdir()
    comparisons = []
    for source in sorted(args.baseline.glob('*.png')):
        target = output / 'comparison' / source.name
        shutil.copyfile(source, target)
        comparisons.append({'file': str(target.relative_to(output)), 'sha256': digest(target)})
    reference = json.loads(args.reference_manifest.read_text())
    for name, metadata in reference['files'].items():
        relative = Path(name)
        assert not relative.is_absolute() and '..' not in relative.parts
        original = args.reference_manifest.parent / relative
        assert original.is_file() and digest(original) == metadata['sha256'], f'changed visual reference: {name}'
        assert original.stat().st_size == metadata['bytes'], f'changed reference size: {name}'
    shutil.copyfile(args.reference_manifest, output / 'selected-reference-manifest.json')
    escape = html.escape
    entries = []
    figures = []
    for case in config['cases']:
        key = case['key']
        prefix = f'product/{key}-offline.html'
        links = [('개요', 'overview'), ('저장소 설명', 'repository-explanation'), ('작업 목록', 'works'), ('결정', 'decision-reading'), ('분석', 'analysis-tool'), ('문서', 'documents-tool'), ('기억', 'memory-tool'), ('근거', 'evidence-tool')]
        if case.get('work'):
            links.append(('선택한 원래 Work', 'work-' + case['work']))
        if case.get('decision'):
            links.append(('실제 Decision', 'decision-' + case['decision']))
        entries.append('<article><h2>' + escape(case['project_name']) + ' · ' + escape(key) + '</h2><p>Project: <code>' + escape(case['project']) + '</code></p>' + ''.join(f'<a href="{prefix}#{escape(fragment)}">{escape(label)}</a> ' for label, fragment in links) + '</article>')
        for width in [1440, 1024, 768, 390, 320]:
            for page in ['overview', 'repository', 'works', 'work', 'decision', 'source']:
                current = f'product/{key}-{page}-{width}.png'
                if not (output / current).exists():
                    continue
                reference = 'overview' if page == 'overview' else 'works' if page == 'works' else 'b'
                ref = f'comparison/reference-{reference}-{width}.png'
                baseline_key = 'work' if case.get('work') else 'workless'
                baseline_page = f"work-work-{case['work']}" if page == 'work' else page
                baseline = f'comparison/{baseline_key}-{baseline_page}-{width}.png'
                images = [(name, file) for name, file in [('선택된 과거 시각 기준', ref), ('수정 전 실제 Product', baseline), ('현재 실제 Product', current)] if (output / file).exists()]
                figures.append(f'<section><h3>{escape(key)} · {page} · {width}px</h3><div class="screens">' + ''.join(f'<figure><figcaption>{name}</figcaption><a href="{file}"><img src="{file}" alt="{escape(key)} {page} {width}px {name}" loading="lazy"></a></figure>' for name, file in images) + '</div></section>')
    document = '''<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Volicord Product 검토</title><style>
body{font:16px/1.8 system-ui,sans-serif;margin:32px;color:#1c2b40;background:#f3f5f9}main{max-width:1500px;margin:auto}a{color:#2357a0;overflow-wrap:anywhere}article,section{padding:24px;margin:24px 0;background:white;border-radius:8px}article a{display:inline-block;padding:8px}code{overflow-wrap:anywhere}.screens{display:flex;gap:20px;align-items:start}.screens figure{flex:1;min-width:0;margin:0}img{width:100%;height:auto}figcaption{font-size:.85rem} @media(max-width:700px){body{margin:16px}.screens{display:block}figure{margin-bottom:20px!important}}
</style><main><h1>실제 Product 읽기·탐색 검토</h1><p>기술 검사 완료 · 현재 전달물의 사람 디자인·독해 수락은 대기 중입니다. 아래 문서는 실제 Product가 만든 변경 없는 script-free export입니다. 네트워크·실행 중인 서버 없이 열 수 있습니다.</p>
<p>과거 화면은 시각·상호작용 비교용입니다. 과거의 문구, Work 이름과 검증 상태를 현재 프로젝트의 사실로 재사용하지 않았습니다. 보존된 실제 A/B/C Work는 원래 identity·시간·상태의 역사 기록으로 읽습니다.</p>
<p>이전 전달 UI는 후속 사용자 피드백에서 거절되었습니다. 원래 manifest와 평가는 보존합니다. 이 새 산출물은 실제 검토 전까지 수용 미확인입니다.</p><h2>직접 해 볼 검토</h2><ol><li>Work 없는 프로젝트 개요에서 저장소 설명을 찾고, 역할·대표 작동 방식·인용 영역과 제외 범위를 자신의 말로 설명합니다.</li><li>normal과 unavailable에서 같은 실제 Work를 열고, 새 이름·문제/접근/결과 요약·본문의 차이와 완료·과거 검증 실패·검토·수락을 구분합니다. 원래 Goal과 결과 인용, 전체 근거를 확인합니다.</li><li>실제 Decision을 찾아 선언된 범위와 이유를 읽습니다. 기록되지 않은 이유를 에이전트 권고로 오인하지 않는지 확인합니다.</li><li>도구에서 분석·문서·기억·근거로 이동하고 개요로 돌아갑니다. 오프라인 파일에는 수정·내보내기 폼이 없습니다.</li><li>저장소 본문의 Source를 열고 같은 문단으로 돌아갑니다. 데스크톱 병렬 읽기, 390/320px 코드 가독성, 긴 줄의 가로 스크롤을 확인합니다. live 키보드·Escape·history 결과는 Product 검사 기록에 별도로 남아 있습니다.</li></ol>
<p>피드백에는 <a href="manifest.json">전달물 manifest</a>, 페이지·너비·수행 경로·이해하기 어려웠던 문장·선호를 함께 기록합니다. 자동 검사는 독해나 선호의 증거가 아닙니다. 실제 screen reader와 native zoom 관찰도 별도로 필요합니다.</p>'''
    document += '<p>검토 HEAD: <code>' + escape(head) + '</code> · <a href="product/result.json">전체 브라우저 관측과 실행 identity</a></p>'
    document += ''.join(entries) + '<h2>의도한 차이</h2><p>현재 Product는 실제 기록된 Purpose만 표시합니다. 선택된 과거 디자인의 charter 인용이나 수작업 카드 문구를 가져오지 않습니다. 실제 host가 새로 생성한 heading·synopsis는 같은 Work 카드와 상세에서 읽습니다. 부재 control은 생산 미완료를 표시하고 원문은 상세에 보존합니다. 생성과 마지막 작업 기록의 날짜는 서로 다른 원래 Goal/Checkpoint 사건에 묶입니다. 단일 우측 메뉴에서 저장소 설명·작업·결정 검사·기존 도구로 이동합니다. 개요에는 실제 Project 이름과 canonical Purpose 또는 부재, 최근 작업을 표시합니다. 중요한 실패·범위 한계는 본문 가까이에, 전체 진단은 상세에 둡니다.</p>' + ''.join(figures) + '</main></html>'
    (output / 'index.html').write_text(document)
    manifest = {'candidate': head, 'classification': 'actual Product review; historical screenshots are visual references only', 'human_acceptance': 'unassessed', 'prior_delivered_ui_feedback': 'rejected by subsequent user feedback; original historical manifest remains unchanged', 'source_status': 'early dirty-source output' if args.early else 'clean committed candidate', 'source_diff_sha256': result['diff_sha256'], 'source_review_result_sha256': digest(observed / 'result.json'), 'selected_reference_manifest_sha256': digest(args.reference_manifest), 'canonical_bundle_sha256': digest(args.bundle), 'cases': config['cases'], 'files': {str(p.relative_to(output)): digest(p) for p in sorted(output.rglob('*')) if p.is_file()}}
    (output / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2))
    print(f'Actual Product review package: {output / "index.html"}')


if __name__ == '__main__':
    main()
