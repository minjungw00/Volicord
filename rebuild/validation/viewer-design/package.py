#!/usr/bin/env python3
"""Close a tested candidate and its local observations into a new review package."""
import argparse
from html import escape as esc
import importlib.util
import json
from pathlib import Path
import shutil

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('design',HERE/'build.py');design=importlib.util.module_from_spec(spec);spec.loader.exec_module(design)


def package(args):
    candidate=args.candidate.resolve(); output=args.output.resolve(); browser=args.browser.resolve()
    if not output.is_relative_to(design.REPO/'rebuild/.local') or output.exists():
        raise ValueError('select a new ignored review destination')
    manifest=design.load(candidate/'manifest.json'); observation=design.load(browser/'result.json')
    for name,d in manifest['files'].items():design.read(candidate/name,d['sha256'])
    if observation['status']!='passed' or observation['manifest_sha256']!=design.sha((candidate/'manifest.json').read_bytes()):
        raise ValueError('browser evidence does not match candidate')
    shutil.copytree(candidate,output)
    shutil.copyfile(candidate/'manifest.json',output/'candidate-manifest.json')
    shutil.copytree(browser,output/'observations')
    (output/'validation').mkdir()
    for n,run in enumerate(args.validation):
        result=design.load(run/'result.json')
        if result['exit_code']!=0 or result['termination'] is not None:
            raise ValueError('cannot publish a passing check from failed process evidence')
        dest=output/'validation'/f'check-{n:02}';shutil.copytree(run,dest)
    (output/'support').mkdir(exist_ok=True)
    shutil.copyfile(HERE/'package.py',output/'support/package.py')
    trips=sum(c.get('trips',0) for c in observation['checks'])
    body=f'<p class="eyebrow">Technical observations · 사용자 수락 아님</p><h1>같은 내용, 다른 읽기 구조.</h1><p class="intro">1440 / 768 / 390 / 320 CSS px · {trips} citation round trips · 모든 prose·Goal·fact와 원래 Source 좌표 비교 통과. Browser 상태와 사람의 이해·선호는 별도입니다.</p><p><a href="index.html">← 검토 시작 페이지</a> · <a href="observations/result.json">전체 browser 결과</a> · <a href="candidate-manifest.json">Browser가 검사한 후보 manifest</a></p><div class="review-grid">'
    for width in [1440,768,390,320]:
        body+=f'<section class="review-card"><h2>{width} CSS px</h2><a href="observations/discovery-{width}.png">작업 목록</a>'
        for key in ['b','click']:
            for name,label in [(f'{key}-{width}-baseline.png','현재 Viewer'),(f'{key}-{width}-reading.png','새 문서'),(f'{key}-{width}-live-point-5.png','선택한 코드'),(f'{key}-{width}-unavailable.png','Source 이용 불가')]:
                body+=f'<a href="observations/{name}">{key.upper()} · {label}</a>'
        body+=f'<a href="observations/click-{width}-comparisons.png">Click · 조건별 관측</a></section>'
    body+='</div><div class="instructions"><h2>직접 확인한 시각적 차이</h2><p>목록은 title·한계 중심의 짧은 행이며 synopsis는 펼쳐 읽습니다. 본문은 제목·lead·중요한 한계 다음에 설명을 배치합니다. Desktop은 선택한 정확한 설명과 원문을 동시에 읽습니다. 768 px 이하에서는 선택한 원문을 별도 전체 화면에서 읽고 돌아옵니다. Code는 15 px, 줄바꿈 없이 영역 안에서 스크롤합니다. 선택 전에는 임의 Source를 보여주지 않습니다.</p><h2>남은 관찰</h2><p>사람의 이해·선호·수락, 실제 screen reader, native zoom은 확인하지 않았습니다. 긴 코드 줄은 가로 스크롤이 필요하고, 좁은 화면의 긴 critical limit는 여전히 공간을 차지합니다. 독립적인 새 자율 생성 품질이나 현재 Source freshness를 검증하지 않았습니다. 두 control에 실제 before-state와 연결된 Decision은 없습니다.</p><h2>프로세스 증거</h2><p>서버 연결 검사는 idle·partial·disconnect·slow dribble 뒤 같은 PID 생존과 다음 GET 200을 확인했습니다. Browser child는 명시적 SIGTERM으로 정리했습니다. 아래 wrapper는 numeric exit, termination, 전체 stdout·stderr를 보존합니다.</p><ul>'
    for n,run in enumerate(args.validation):
        for name in ['result.json','stdout.log','stderr.log']:
            body+=f'<li><a href="validation/check-{n:02}/{name}">{esc(run.name)} · {name}</a></li>'
    body+='</ul></div>'
    (output/'observations.html').write_text(design.frame('관찰과 화면',body))
    index=(output/'index.html').read_text()
    index=index.replace('<h2>살펴볼 순서</h2>','<p><a href="observations.html">실제 화면·기술 검사·남은 관찰 보기 →</a></p><h2>살펴볼 순서</h2>')
    (output/'index.html').write_text(index)
    manifest['browser_basis_manifest']='candidate-manifest.json'
    manifest['browser_result']='observations/result.json'
    manifest['packaging_changes']='copied candidate without content edits; added observations page and entry link'
    manifest['files']={str(p.relative_to(output)):dict(sha256=design.sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(output.rglob('*')) if p.is_file() and p.name!='manifest.json'}
    (output/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2))
    print(json.dumps(dict(status='packaged',output=str(output),files=len(manifest['files']),trips=trips)))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--candidate',type=Path,required=True);p.add_argument('--browser',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--validation',type=Path,nargs='+',required=True);package(p.parse_args())
