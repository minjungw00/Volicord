#!/usr/bin/env python3
"""Close a tested candidate and its local observations into a new review package."""
import argparse
from html import escape as esc
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess

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
    if args.initial:
        initial=design.load(args.initial/'manifest.json'); earlier=design.load(args.initial_browser/'result.json')
        if earlier['status']!='passed' or earlier['manifest_sha256']!=design.sha((args.initial/'manifest.json').read_bytes()):
            raise ValueError('initial observations do not match initial artifact')
        for name,d in initial['files'].items():design.read(args.initial/name,d['sha256'])
        shutil.copytree(args.initial,output/'initial')
        shutil.copytree(args.initial_browser,output/'initial-observations')
    (output/'validation').mkdir()
    for n,run in enumerate(args.validation):
        result=design.load(run/'result.json')
        if result['exit_code']!=0 or result['termination'] is not None:
            raise ValueError('cannot publish a passing check from failed process evidence')
        dest=output/'validation'/f'check-{n:02}';shutil.copytree(run,dest)
    (output/'support').mkdir(exist_ok=True)
    shutil.copyfile(HERE/'package.py',output/'support/package.py')
    shutil.copyfile(HERE/'review.cjs',output/'support/review.cjs')
    trips=sum(c.get('trips',0) for c in observation['checks'])
    body=f'<p class="eyebrow">Technical observations · 사용자 수락 아님</p><h1>같은 내용, 다른 읽기 구조.</h1><p class="intro">1440 / 768 / 390 / 320 CSS px · {trips} citation round trips · 모든 prose·Goal·fact와 원래 Source 좌표 비교 통과. Browser 상태와 사람의 이해·선호는 별도입니다.</p><p><a href="index.html">← 검토 시작 페이지</a> · <a href="observations/result.json">전체 browser 결과</a> · <a href="candidate-manifest.json">Browser가 검사한 후보 manifest</a></p><div class="review-grid">'
    for width in [1440,768,390,320]:
        body+=f'<section class="review-card"><h2>{width} CSS px</h2><a href="observations/overview-{width}.png">프로젝트 개요</a><a href="observations/discovery-{width}.png">전체 작업</a>'
        for key in ['a','c']:
            body+=f'<a href="observations/{key}-{width}-record.png">{key.upper()} · 원래 작업 기록</a>'
        for key in ['b','click']:
            for name,label in [(f'{key}-{width}-baseline.png','현재 Viewer'),(f'{key}-{width}-reading.png','새 문서'),(f'{key}-{width}-live-point-5.png','선택한 코드'),(f'{key}-{width}-unavailable.png','Source 이용 불가'),(f'{key}-{width}-records.png','원래 지시·전체 근거 박스')]:
                body+=f'<a href="observations/{name}">{key.upper()} · {label}</a>'
        body+=f'<a href="observations/click-{width}-comparisons.png">Click · 조건별 관측</a></section>'
    body+='</div><div class="instructions"><h2>직접 확인한 시각적 차이</h2><p>프로젝트 개요는 목표와 최근 작업을, 작업 페이지는 전체 목록을 보여줍니다. 카드는 작업명·현황·내용 요약으로 구성하며 현재 자료의 세 작업은 두 페이지에 동일하게 나타납니다. 본문은 제목·lead 다음에 설명을 읽고, 그 뒤에 한계와 별도 기록 박스를 배치합니다. Desktop에서는 본문과 오른쪽 근거 패널을 함께 읽습니다. 900 px 이하에서는 전체 화면으로 읽고 돌아옵니다. Code는 desktop 15 px, 768 px에서 13 px, 600 px 이하에서 12 px이며 줄바꿈 없이 영역 안에서 스크롤합니다. Viewer와 디자인 검토는 별도 화면입니다. 선택 전에는 임의 Source를 보여주지 않습니다. Syntax highlighting은 생성 시 적용되며 원문과 좌표를 바꾸지 않습니다. Blue-gray 팔레트의 패널·경계·선택 색상은 참고 prototype과 같습니다.</p><h2>남은 관찰</h2><p>사람의 이해·선호·수락, 실제 screen reader, native zoom은 확인하지 않았습니다. 긴 코드 줄은 가로 스크롤이 필요하고, 좁은 화면의 긴 critical limit는 여전히 공간을 차지합니다. 독립적인 새 자율 생성 품질이나 현재 Source freshness를 검증하지 않았습니다. 두 control에 실제 before-state와 연결된 Decision은 없습니다.</p><h2>프로세스 증거</h2><p>서버 연결 검사는 idle·partial·disconnect·slow dribble 뒤 같은 PID 생존과 다음 GET 200을 확인했습니다. Browser child는 명시적 SIGTERM으로 정리했습니다. 아래 wrapper는 numeric exit, termination, 전체 stdout·stderr를 보존합니다.</p><ul>'
    for n,run in enumerate(args.validation):
        for name in ['result.json','stdout.log','stderr.log']:
            body+=f'<li><a href="validation/check-{n:02}/{name}">{esc(run.name)} · {name}</a></li>'
    body+='</ul></div>'
    (output/'observations.html').write_text(design.frame('관찰과 화면',body,active='index'))
    index=(output/'index.html').read_text()
    index=index.replace('<h2>살펴볼 순서</h2>','<p><a href="observations.html">실제 화면·기술 검사·남은 관찰 보기 →</a></p><h2>살펴볼 순서</h2>')
    (output/'index.html').write_text(index)
    if (browser/'journeys.json').exists():
        measured=design.load(browser/'journeys.json')
        body='<h1>여정 관찰 · 기술 검사</h1><p>사용자 이해·시간·오해·선호는 미관찰입니다. 아래 수치는 Chromium에서 정해진 위치로 이동한 scripted task입니다. Tab은 실제 입력 횟수이며, 문단 위치 선택은 automation의 사전 지식을 사용합니다. 사람의 탐색 시간으로 해석하지 마세요.</p><p><a href="journeys.html">답안 없는 검토 과제</a> · <a href="observations/journeys.json">전체 측정</a> · <a href="candidate-manifest.json">검사한 정확한 후보</a></p>'
        if args.initial:
            body+='<p><a href="initial/journeys.html">수정 전 동일 여정</a> · <a href="initial-observations/journeys.json">수정 전 측정</a></p>'
        body+='<table class="condition-table"><caption>동일 control Work · entry 이후 viewport/scroll · CSS px</caption><thead><tr><th>폭 / 사례 / 구성</th><th>Tab / 문단까지 scroll</th><th>코드 글자 / 영역 폭</th></tr></thead><tbody>'
        for r in measured['rows']:
            code=r.get('code',{});find=r.get('find',{})
            body+=f'<tr><th scope="row">{r["width"]} / {r["work"]} / {r["layout"]}</th><td>{find.get("native_tabs","—")} / {r.get("read_scroll","—")}</td><td>{code.get("font","—")} / {code.get("width","—")}</td></tr>'
        body+='</tbody></table><p>Baseline은 중요한 한계를 먼저 보여줍니다. 새 본문은 설명 흐름을 우선하고 한계 바로가기를 제공합니다. Desktop에서는 본문과 근거를 함께 읽으며 모바일에서는 작은 글자로 가로 스크롤 부담을 줄입니다. 모든 중요한 문단·사실·원래 목표·Source는 보존합니다. Repository Map, Decisions, Analysis와 Documents는 별도 검토의 baseline 목적지에 있으며 새 구성에서 구현됐다고 주장하지 않습니다.</p><p>Source 없는 Goal-only 사례는 설명과 코드 task가 이용 불가로 남습니다. 레이아웃이 결손 정보를 복구하지 않습니다. Native zoom·실제 screen reader·브라우저 외부 다운로드 환경·독립 모델 평가도 미관찰입니다.</p>'
        (output/'journey-observations.html').write_text(design.frame('여정 관찰',body,active='index'))
        p=output/'index.html';p.write_text(p.read_text().replace('<h2>살펴볼 순서</h2>','<p><a class="button" href="journey-observations.html">수정 전·후 여정과 실제 측정 보기 →</a></p><h2>살펴볼 순서</h2>'))
    manifest['browser_basis_manifest']='candidate-manifest.json'
    manifest['browser_result']='observations/result.json'
    manifest['packaging_changes']='copied candidate without content edits; added observations page and entry link'
    manifest['support_commit']=subprocess.check_output(['git','rev-parse','HEAD'],cwd=design.REPO,text=True).strip()
    manifest['initial_comparison']='initial/manifest.json' if args.initial else None
    manifest['files']={str(p.relative_to(output)):dict(sha256=design.sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(output.rglob('*')) if p.is_file() and p!=output/'manifest.json'}
    (output/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2))
    print(json.dumps(dict(status='packaged',output=str(output),files=len(manifest['files']),trips=trips)))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--candidate',type=Path,required=True);p.add_argument('--browser',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--validation',type=Path,nargs='+',required=True);p.add_argument('--initial',type=Path);p.add_argument('--initial-browser',type=Path);args=p.parse_args()
    if bool(args.initial)!=bool(args.initial_browser):p.error('initial artifact and observations must be supplied together')
    package(args)
