#!/usr/bin/env python3
"""Disposable editorial design over verified retained readbacks; no generation."""
import argparse
import hashlib
from html import escape as esc
from html.parser import HTMLParser
import json
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
ROLES = {'purpose': '문제와 요청', 'reported_change': '구조와 동작', 'reading_point': '작동 원리',
         'observation': '관측과 범위', 'comparison': '조건별 관측', 'next_step': '기록된 방향', 'limits': '결과를 읽을 때의 한계',
         'heading': '제목 근거', 'synopsis': '요약 근거'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def read(path, expected=None):
    raw = Path(path).read_bytes()
    if expected and sha(raw) != expected.removeprefix('sha256:'):
        raise ValueError(f'changed input: {path}')
    return raw


def load(path):
    return json.loads(Path(path).read_text())


class Text(HTMLParser):
    def __init__(self, html):
        super().__init__(); self.parts = []; self.feed(html)
    def handle_data(self, value):
        self.parts.append(value)


def admit(root):
    """Original file bytes and frozen public readback are independent of rendering."""
    identities = load(root / 'review/identities.json')
    originals = [(Path(i['path']), read(i['path'], i['sha256'])) for i in identities['original_assets']]
    authored = json.loads(read(root / 'review-input-final.json', identities['input_sha256']))
    fixture = load(root / 'maintained-review/fixtures.json')
    if fixture['input_sha256'] != 'sha256:' + sha((root / 'review-input-final.json').read_bytes()):
        raise ValueError('fixture input identity mismatch')
    if len(authored['works']) != len(fixture['cases']):
        raise ValueError('case count mismatch')
    works = []
    for work, control in zip(authored['works'], fixture['cases'], strict=True):
        if work['key'] != control['key'] or work['units'] != control['expected_units'] or work['historical_work'] != control['historical_work']:
            raise ValueError('case/Work identity mismatch')
        name = f"browser-final/{work['key']}-status.stdout"
        raw = read(root / name, identities['files'][name])
        status = json.loads(raw)
        matches = [w for w in status['work_history'] if w['work_item_id'] == control['work']]
        if len(matches) != 1:
            raise ValueError('wrong Work readback')
        selected = matches[0]; answers = selected['answers']
        if answers['explanation_state'] != 'current' or answers['diagnostic'] is not None or answers['omitted_source_excerpt_count']:
            raise ValueError('unavailable/incomplete captured interpretation')
        if answers['provenance']['realization_fingerprint'] != next(c for c in identities['cases'] if c['key'] == work['key'])['realization_fingerprint']:
            raise ValueError('output identity mismatch')
        if [u['text'] for u in work['units']] != [p['text'] for p in answers['prose']]:
            raise ValueError('substantive readback mismatch')
        sources = {}; source_files = []
        for f in work['files']:
            body = read(f['input'], f['sha256']); source_files.append((f, body))
            for region in f['regions']:
                start, end = region['start_line'], region['end_line']
                lines = body.splitlines(keepends=True)
                if not 1 <= start <= end <= len(lines):
                    raise ValueError('invalid source coordinates')
                exact = b''.join(lines[start-1:end]); start_byte = len(b''.join(lines[:start-1]))
                candidates = [s for s in answers['source_excerpts'] if s['citation']['locator'] == f['path'] and s['citation']['start_line'] == start and s['citation']['end_line'] == end]
                if len(candidates) != 1:
                    raise ValueError('Source unavailable or ambiguous')
                s = candidates[0]; c = s['citation']
                if s['comparison'] != 'missing_before_state' or s['before_source_id'] is not None:
                    raise ValueError('trial requires explicit missing-before observation; do not invent a diff')
                if s['omitted_utf8_bytes'] or s['text'].encode() != exact or c['range_sha256'] != 'sha256:' + sha(exact) or c['start_byte'] != start_byte or c['end_byte'] != start_byte + len(exact):
                    raise ValueError('Source bytes/range mismatch')
                if s['work_revision'] != selected['evidence']['goal']['basis']['revision']:
                    raise ValueError('Source Work revision mismatch')
                sources[region['key']] = s
        for unit, paragraph in zip(work['units'], answers['prose'], strict=True):
            # Each navigation action must be offered by this exact paragraph,
            # rather than merely pointing somewhere in the same Work.
            if unit['role'] not in ROLES or ''.join(word.capitalize() for word in unit['role'].split('_')) != paragraph['question']:
                raise ValueError('unit role mismatch')
            for c in paragraph['citations']:
                claim = paragraph['text'].encode()[c['claim_start_byte']:c['claim_end_byte']]
                if not claim or c['claim_sha256'] != 'sha256:' + sha(claim):
                    raise ValueError('claim hash mismatch')
            for ref in unit['refs']:
                if ref not in sources:
                    raise ValueError('unknown Source reference')
                s = sources[ref]['citation']
                if not any(all(c[k] == s[k] for k in ['source_id','source_revision','locator','start_byte','end_byte','range_sha256']) for c in paragraph['citations']):
                    raise ValueError('wrong claim-to-Source association')
        name = f"browser-final/{work['key']}-ko.html"
        baseline = read(root / name, identities['files'][name])
        text = ''.join(Text(baseline.decode()).parts)
        if not all(u['text'] in text for u in work['units']) or work['goal'] not in text:
            raise ValueError('baseline loses substantive content')
        works.append(dict(authored=work, control=control, selected=selected, sources=sources,
                          source_files=source_files, raw=raw, baseline=baseline))
    return identities, originals, works


def frame(title, body, scripted=False):
    css = (HERE / 'design.css').read_text()
    return f'''<!doctype html><html lang="ko"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{esc(title)} · Volicord design review</title><style>{css}</style></head><body><a class="skip" href="#main">본문으로</a><header class="masthead"><div class="shell"><a class="brand" href="works.html">Volicord<span>Work reader</span></a><nav aria-label="주요 탐색"><a href="works.html">작업</a><a href="index.html">비교와 검토 안내</a></nav><span class="trial">읽기 전용 · 디자인 검토</span></div></header><main class="shell" id="main">{body}<footer>보존된 읽기 자료 · 새 생성·실행·수락 아님 · 디자인 trial · <a href="index.html">검토 안내</a></footer></main>{'<script src="design.js" defer></script>' if scripted else ''}</body></html>'''


def links(work, index):
    u = work['authored']['units'][index]
    return '<div class="evidence-actions">' + ''.join(f'<a class="evidence-link" data-origin="claim-{index}" href="#point-{index}-{j}">{ROLES[u["role"]]} · 근거 {j+1} ↗<span>{esc(locator(work["sources"][ref]))}</span></a>' for j, ref in enumerate(u['refs'])) + '</div>'


def locator(s):
    c = s['citation']; return f'{c["locator"]}:{c["start_line"]}–{c["end_line"]}'


def unit(work, i):
    u = work['authored']['units'][i]
    return f'<div class="unit" id="claim-{i}" tabindex="-1"><p class="unit-text" data-unit="{i}">{esc(u["text"])}</p>{links(work,i)}</div>'


def work_page(work, scripted=True, failure=False):
    a = work['authored']; units = a['units']; selected = work['selected']; key = a['key']
    title = next((u['text'] for u in units if u['role'] == 'heading'), '기록된 작업')
    body = '<article class="doc"><p class="breadcrumb"><a href="works.html">← 작업 목록</a> / 보존된 Work</p><header class="doc-head"><p class="eyebrow">Work document · 해석과 관측</p>'
    if not any(u['role'] == 'heading' for u in units):
        body += '<h1>기록된 작업</h1>'
    for i,u in enumerate(units):
        if u['role'] == 'heading':
            body += f'<div id="claim-{i}" tabindex="-1"><h1 data-unit="{i}">{esc(u["text"])}</h1></div>'
        elif u['role'] == 'synopsis':
            body += f'<div id="claim-{i}" tabindex="-1"><p class="lead" data-unit="{i}">{esc(u["text"])}</p></div>'
    body += '<p class="interpretation">Source에서 작성한 검토용 해석 · 의미 정확성·사용자 수락은 별도 검토</p></header>'
    if failure:
        body += '<p class="unavailable" role="status">검토용 Source 이용 불가 상태입니다. 아래 원문은 숨겨집니다. 보존된 설명과 identity는 과거 readback이며 현재 근거 이용 가능성을 뜻하지 않습니다.</p>'
    for i,u in enumerate(units):
        if u['role'] == 'limits':
            body += f'<aside class="guard" aria-label="중요한 한계"><h2>결과를 읽을 때의 한계</h2>{unit(work,i)}</aside>'
    # Every original fact remains exact. The opening facts carry outcome and
    # direction boundaries; remaining technical facts stay ordinary in a disclosure.
    facts = selected['answers']['facts']
    front = {'WorkState','VerificationState','UserAcceptance','ReportedResult','NextStepAvailability'}
    body += '<div class="record-strip" aria-label="기록의 상태">'
    for f in facts:
        if f['question'] in front:
            body += f'<p data-fact="{esc(f["question"])}">{esc(f["text"])}</p>'
    body += '</div>'
    body += '<nav class="audit-links" aria-label="본문과 기록"><a href="#reading">설명 읽기 ↓</a><a href="#original">원래 지시</a><a href="#audit">전체 근거와 상태</a></nav><div id="reading">'
    narrative = [i for i,u in enumerate(units) if u['role'] not in {'heading','synopsis','limits'}]
    group = []
    for i in narrative:
        if not group or units[group[-1][-1]]['role'] != units[i]['role']:
            group.append([i])
        else:
            group[-1].append(i)
    for indices in group:
        role = units[indices[0]]['role']
        body += f'<section class="reading-section"><h2>{ROLES.get(role,"설명")}</h2>'
        if role == 'comparison':
            body += '<p class="source-note">서로 다른 조건의 보존 관측 · 시간 순서나 수정 전후 diff를 뜻하지 않습니다.</p><div class="comparisons">'
            for n,i in enumerate(indices):
                body += f'<div class="comparison-card"><p class="serial">관측 {n+1:02}</p>{unit(work,i)}</div>'
            body += '</div>'
        else:
            body += ''.join(unit(work,i) for i in indices)
        body += '</section>'
    body += '</div><div class="audit"><details id="original"><summary>원래 지시 · 전체 Goal</summary><blockquote class="goal">' + esc(a['goal']) + '</blockquote></details>'
    body += '<details id="audit"><summary>전체 근거 · 상태 · 출력 identity</summary><p>Captured Current는 과거 readback의 상태입니다. 현재 Source freshness나 새 검증을 뜻하지 않습니다.</p>'
    body += f'<p>현재 control Work: {esc(selected["work_item_id"])} · 과거 Work: {esc(a["historical_work"])}</p>'
    for f in facts:
        if f['question'] not in front:
            body += f'<p data-fact="{esc(f["question"])}">{esc(f["text"])}</p>'
    body += '<p>제목·요약의 근거</p>' + ''.join(links(work,i) for i,u in enumerate(units) if u['role'] in {'heading','synopsis'})
    body += '<p>연결된 Decision: ' + (', '.join(map(esc,selected['decision_ids'])) or '없음 · 사용자 rationale을 추측하지 않음') + '</p>'
    body += f'<pre class="full-audit">{esc(json.dumps(selected,ensure_ascii=False,indent=2))}</pre><p><a href="inputs/{key}-status.json">원래 Product readback</a> · <a href="manifest.json">입력·후보 identity</a> · <a href="baseline-{key}.html#decision-reading">현재 Viewer의 Decisions</a> · <a href="baseline-{key}.html#health">현재 Viewer의 진단</a></p></details></div></article>'
    body += '<aside class="appendix" aria-label="선택한 설명의 Source"><h2>설명에 연결된 원문</h2>'
    for i,u in enumerate(units):
        for j, ref in enumerate(u['refs']):
            s = work['sources'][ref]; c=s['citation']; point=f'point-{i}-{j}'
            body += f'<section class="source-point" id="{point}" data-origin="claim-{i}" tabindex="-1"><div class="point-bar"><a class="return" href="#claim-{i}">← 설명으로 돌아가기</a><span class="point-counter">{ROLES[u["role"]]} · 근거 {j+1}/{len(u["refs"])}</span></div><p class="point-title">이 설명을 확인하는 근거</p><blockquote class="claim-quote">{esc(u["text"])}</blockquote><h3 class="source-path">{esc(locator(s))}</h3><p class="source-note">보존된 관측 범위 · 수정 전 원문 없음 ({esc(s["comparison"])}) · 줄별 변경이나 더 좁은 설명 범위를 추정하지 않음</p>'
            if failure:
                body += '<p class="unavailable">Source unavailable · 원문을 읽을 수 없습니다.</p>'
            else:
                body += f'<pre class="code-region" tabindex="0" role="region" aria-label="{esc(locator(s))} 원문"><code>'
                for n,line in enumerate(s['text'].splitlines(keepends=True)):
                    body += f'<span class="line"><span class="number" aria-hidden="true">{c["start_line"]+n}</span><span class="code-text">{esc(line.rstrip(chr(10)))}</span></span>'
                body += '</code></pre>'
            body += '<nav class="point-others" aria-label="같은 설명의 다른 근거">'
            for k,r in enumerate(u['refs']):
                if j != k:
                    body += f'<a href="#point-{i}-{k}">근거 {k+1} · {esc(locator(work["sources"][r]))}</a>'
            body += '</nav><details><summary>Source identity · 관측 범위</summary><pre>' + esc(json.dumps(s,ensure_ascii=False,indent=2)) + f'</pre><a href="inputs/{key}-source-{ref}.txt">보존된 전체 파일 열기</a></details></section>'
    body += '</aside>'
    return frame(title,body,scripted)


def build(args):
    root = args.retained.resolve(); output=args.output.resolve()
    if not output.is_relative_to(REPO/'rebuild/.local'):
        raise ValueError('output must be under ignored rebuild/.local')
    if output.exists():
        raise ValueError('output must be new')
    identities, originals, works = admit(root)
    prototype = args.prototype.resolve(); prototype_raw=read(prototype)
    # A design reference identity is separate from admitted Product/source inputs.
    reference = identities['candidate']
    paths=['rebuild/crates','rebuild/Cargo.toml','rebuild/Cargo.lock']
    if subprocess.check_output(['git','diff',reference,'HEAD','--',*paths],cwd=REPO):
        raise ValueError('baseline Product source changed; recapture current renderer first')
    output.mkdir(parents=True); (output/'inputs').mkdir(); (output/'support').mkdir()
    manifest = dict(entry_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip(),
        baseline_capture_head=reference, baseline_source_equivalent=True, production_migration=False,
        classification='retained Product/Source readbacks of independently authored controls; no new generation or execution',
        design_reference=dict(path=str(prototype),sha256=sha(prototype_raw),bytes=len(prototype_raw),use='visual/interaction only; excluded from content inputs'),
        human_acceptance='pending', inputs={}, cases=[])
    manifest['verified_historical_assets'] = []
    for n,(p,raw) in enumerate(originals):
        admitted = p.name.startswith('source-')
        manifest['verified_historical_assets'].append(dict(original=str(p),sha256=sha(raw),bytes=len(raw),admitted_as_content=False,classification='historical byte identity only; independent full Source files and public answers below supply reader content'))
        # Never deliver the executable reference or admit its prose/feedback as reader data.
        if not admitted:
            continue
        name=f'inputs/original-{n:02}{p.suffix}';(output/name).write_bytes(raw)
        manifest['inputs'][name]=dict(original=str(p),sha256=sha(raw),bytes=len(raw))
    for name in ['review/identities.json','review-input-final.json','maintained-review/fixtures.json']:
        raw=read(root/name); dest='inputs/'+Path(name).name;(output/dest).write_bytes(raw)
        manifest['inputs'][dest]=dict(original=str(root/name),sha256=sha(raw),bytes=len(raw))
    rows=''
    for n,w in enumerate(works):
        a=w['authored']; key=a['key']; (output/f'inputs/{key}-status.json').write_bytes(w['raw'])
        (output/f'baseline-{key}.html').write_bytes(w['baseline'])
        for f,raw in w['source_files']:
            for region in f['regions']:
                (output/f'inputs/{key}-source-{region["key"]}.txt').write_bytes(raw)
        for scripted,suffix in [(True,''),(False,'-offline')]:
            (output/f'{key}{suffix}.html').write_text(work_page(w,scripted))
        (output/f'{key}-unavailable.html').write_text(work_page(w,True,True))
        heading=next((u['text'] for u in a['units'] if u['role']=='heading'),'기록된 작업')
        lead=next((u['text'] for u in a['units'] if u['role']=='synopsis'),None)
        limit=next((u['text'] for u in a['units'] if u['role']=='limits'),None)
        rows+=f'<article class="work-row"><p class="serial">{n+1:02}</p><div><h2><a data-work="{key}" href="{key}.html">{esc(heading)}</a></h2><details class="row-synopsis"><summary>문제와 작동 방식</summary><p class="lead">{esc(lead) if lead else "요약 없음 · 원래 지시는 상세에서 확인"}</p></details><p class="meta">보존된 control · {esc(w["selected"]["state"])}</p></div><div class="limit">{esc(limit) if limit else "기록된 해석 한계 없음 · 전체 근거 확인"}</div><a class="arrow" aria-label="{esc(heading)} 열기" href="{key}.html">↗</a></article>'
        manifest['cases'].append(dict(key=key,work=w['selected']['work_item_id'],historical_work=a['historical_work'],output_fingerprint=w['selected']['answers']['provenance']['realization_fingerprint'],source_ranges=len(w['sources']),units=len(a['units']),facts=len(w['selected']['answers']['facts']),before_state='missing',decisions=w['selected']['decision_ids']))
    (output/'works.html').write_text(frame('작업 읽기',f'<p class="eyebrow">Work index / 보존된 두 작업</p><h1 class="index-title">작업 읽기</h1><p class="intro index-intro">작업을 고르고, 설명에 연결된 원문까지 읽습니다.</p><div class="list-head"><span>WORK</span><span>문제와 작동 방식</span><span>중요한 결과와 한계</span><span></span></div>{rows}'))
    for name in ['build.py','design.css','design.js','serve.py','browser.cjs','self_test.py']:
        shutil.copyfile(HERE/name,output/'support'/name)
    shutil.copyfile(HERE/'design.js',output/'design.js')
    shutil.copyfile(HERE/'serve.py',output/'serve.py')
    cards='<div class="review-grid"><section class="review-card"><p class="eyebrow">01 / current Viewer</p><h2>현재 구조와 같은 내용</h2><p>현재와 동일한 Product 소스에서 캡처한 과거 snapshot입니다. Work·Code·Decisions·진단을 native navigation으로 읽습니다.</p>'
    for w in works:
        key=w['authored']['key']; cards+=f'<a href="baseline-{key}.html#overview">{esc(key.upper())} · Overview / Work / Code</a>'
    cards+='</section><section class="review-card"><p class="eyebrow">02 / target design</p><h2>작업에서 시작하는 읽기</h2><p>간결한 작업 목록, 문서형 설명과 조건별 관측. 선택한 설명 바로 옆에서 근거를 읽고 돌아갑니다.</p><a href="works.html">새 작업 목록 열기 →</a>'
    for w in works:
        key=w['authored']['key']; cards+=f'<a href="{key}-offline.html">{esc(key.upper())} · script-free 문서와 원문</a><a href="{key}-unavailable.html">{esc(key.upper())} · Source 이용 불가 control</a>'
    cards+='</section></div>'
    intro='''<p class="eyebrow">Volicord / disposable design trial</p><h1>작업을 이해하는<br>새로운 읽기 공간.</h1><p class="intro">현재 Viewer와 제안한 디자인을 같은 보존 자료로 비교합니다. 원래 지시, 해석, 결과, 중요한 한계와 전체 근거를 두 디자인 모두에서 확인할 수 있습니다.</p>'''
    instructions='''<div class="instructions"><h2>살펴볼 순서</h2><ol><li>현재 Viewer와 새 목록에서 B를 찾고, 작동 원리와 중요한 실패를 설명할 수 있는지 비교하세요.</li><li>작동 원리의 근거 링크를 열어 정확한 원문을 읽고, 같은 설명으로 돌아오세요.</li><li>Click의 조건별 관측을 비교하세요. 패치 없음, 과거 실행, 미검증 범위와 수정 전 원문 없음을 구별할 수 있는지 보세요.</li></ol><h2>키보드와 마우스</h2><p>Tab / Shift+Tab으로 링크와 코드 영역에 이동하고 Enter로 선택합니다. 코드는 줄바꿈 없이 가로 스크롤합니다. 코드에서 “설명으로 돌아가기” 또는 Escape로 원래 문단과 스크롤 위치로 돌아옵니다. 브라우저 뒤로/앞으로도 사용할 수 있습니다. 좁은 화면에서는 코드가 별도 전체 화면으로 열립니다. Script-free 문서는 native fragment와 반환 링크를 사용합니다.</p><h2>정확한 입력과 실행</h2><p>이 폴더에서 <code>python3 serve.py</code>를 실행하고 출력된 loopback URL을 여세요. 종료는 Ctrl-C입니다. Node, Runtime, provider나 외부 asset은 필요 없습니다. 이 페이지와 script-free 문서는 파일로 직접 열 수도 있습니다.</p><p><a href="manifest.json">후보·입력·전체 파일 SHA-256</a> · <a href="inputs/identities.json">원래 readback identities</a> · <a href="support/build.py">재현 지원 소스</a></p><h2>검토의 범위</h2><p>해석 문구는 이전에 독립 Source에서 작성하고 Product를 통해 기록·읽은 control입니다. 새 자율 생성이 아닙니다. Prototype은 시각·상호작용 참고로만 사용했습니다. Baseline은 historical content capture이며 현재 Runtime freshness나 새 테스트 성공을 뜻하지 않습니다. 전체 campaign ZIP은 재검증하지 않았습니다. 기술 검사는 사람의 이해·선호·수락을 증명하지 않습니다. 새 frontend framework나 production migration은 선택하지 않았습니다. 이 패키지는 private local review용입니다.</p></div>'''
    (output/'index.html').write_text(frame('디자인 비교',intro+cards+instructions))
    (output/'README.txt').write_text('Open index.html directly, or run: python3 serve.py\nStop with Ctrl-C. All review links are package-relative. Private local material; do not publish.\n')
    manifest['files']={str(p.relative_to(output)):dict(sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in sorted(output.rglob('*')) if p.is_file()}
    (output/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2))
    print(json.dumps(dict(output=str(output),cases=manifest['cases'],status='built'),ensure_ascii=False))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--retained',type=Path,required=True);p.add_argument('--prototype',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    build(p.parse_args())
