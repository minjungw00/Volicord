"""Static reading layer over the diagnostic consumer; no source selection authority."""
import json
import re
from pathlib import Path

from inputs import binding, digest, require
from source_reading import DISPLAY_BYTES, escaped


def paragraphs(prose):
    """Keep separators in their original paragraph; coordinates are UTF-8 bytes."""
    result, start = [], 0
    for match in re.finditer(r'.+?(?:\n\s*\n|\Z)', prose, re.S):
        text = match[0]
        end = start + len(text.encode())
        result.append((start, end, text))
        start = end
    require(''.join(p[2] for p in result) == prose, 'paragraph partition changed prose')
    return result


def heading(response, start, end, number, ko):
    kinds = {c['kind'] for c in response.get('claims', []) if start <= c['start'] < end}
    if number == 0:
        return '문제와 보고된 결과' if ko else 'Problem and reported result'
    if 'recorded_next_action' in kinds:
        return '기록된 방향과 수락 상태' if ko else 'Recorded direction and acceptance'
    if 'user_choice' in kinds:
        return '요청과 기록된 선택' if ko else 'Request and recorded choice'
    if 'task_instruction' in kinds:
        return '요청과 조사 범위' if ko else 'Request and investigation scope'
    if 'execution_evidence' in kinds:
        if 'source_fact' in kinds:
            return '관측된 동작과 코드' if ko else 'Observed behavior and code'
        return '당시 검증과 중요한 한계' if ko else 'Recorded checks and important limits'
    # Reuse a site's own purpose only when its bound claim is wholly in this
    # paragraph. This supplies no hand-picked function, line or semantic answer.
    for site in response.get('primary_sites', []):
        if any(start <= response['claims'][n]['start'] < response['claims'][n]['end'] <= end
               for n in site['claims']):
            return site['reason']
    return '동작과 구현' if ko else 'Behavior and implementation'


def reading_title(response, ko):
    """An extractive subject label, never a summary of the opening sentence."""
    first = paragraphs(response['prose'])[0]
    for match in re.finditer(r'`([^`\n]+)`', first[2]):
        subject = match[1]
        # A visual title bound, not an explanation quality or generation limit.
        if len(subject) <= 48:
            start = len(first[2][:match.start(1)].encode())
            return (subject + (' 작업 설명' if ko else ' · Work explanation'),
                    {'kind': 'quoted_code_subject', 'start': start,
                     'end': start + len(subject.encode()), 'sha256': digest(subject.encode())})
    return ('작업 설명' if ko else 'Work explanation', {'kind': 'derived_work_label'})


def present_card(body, witness):
    records = []
    for output in witness['outputs']:
        if not output.get('prose_sha256'):
            continue
        response_basis = output.get('editorial', {}).get('response', output['original'])
        response = json.loads(Path(response_basis['path']).read_bytes())
        records.append((output, response))
    final = witness['original_invocation'].get('generation_output')
    records.sort(key=lambda item: item[0]['original'] != final)
    first = next((r for o, r in records if o['original'] == final), records[0][1] if records else None)
    language = witness['output_language']['tag']
    ko = language.startswith('ko')
    interface_language = 'ko' if ko else 'en'
    editorial = witness.get('editorial') is not None
    title = ('설명 원문을 사용할 수 없습니다' if ko else 'Original explanation unavailable')
    title_basis = {'kind': 'unavailable'}
    if first:
        title, title_basis = reading_title(first, ko)
    header = ('<article class="reader-work" data-reading-kind="' + ('manual_editorial' if editorial else 'presentation_only') + '"><header><p class="eyebrow" lang="' + interface_language + '">'
                  + (('수동 편집 읽기 · 원문 문단 재배열' if ko else 'Manual editorial reading · original paragraphs reordered') if editorial else
                     ('작업 설명 · 원문 그대로' if ko else 'Work explanation · unchanged original'))
                  + '</p><h2 lang="' + escaped(language) + '">' + escaped(title)
                  + '</h2><details class="title-basis"><summary lang="' + interface_language + '">'
                  + ('제목의 근거' if ko else 'Title basis')
                  + '</summary><p>' + (('원문 코드 표현을 인용한 주제 표지이며 새 요약이 아닙니다.' if ko else
                    'A subject label quoting an original code expression; no new summary.') if title_basis['kind'] == 'quoted_code_subject' else
                    ('작업을 식별하는 표시 표지입니다.' if ko else 'A display label identifying the Work.'))
                  + '</p><pre>' + escaped(json.dumps(title_basis, ensure_ascii=False)) + '</pre></details></header>')
    body = re.sub(r'<article><h2>.*?</h2>', lambda _: header, body, count=1, flags=re.S)
    body = re.sub(r'<h3>((?:Final|Intermediate|Captured) response.*?)</h3>',
                  r'<details class="response-stage"><summary>Original response stage</summary><p>\1</p></details>', body)
    for output, response in records:
        # Match only the verified prose surface built by the existing consumer.
        sequence = next(n for n, candidate in enumerate(witness['outputs'], 1) if candidate is output)
        marker = '-output-' + output['original']['sha256'] + '-response-' + str(sequence)
        if output.get('editorial'):
            marker += '-reading-' + output['editorial']['prose']['sha256']
        response_start = body.index(marker + '"')
        pattern = re.compile(r'<pre class="prose" lang="[^"]*">(.*?)</pre>', re.S)
        match = pattern.search(body, response_start)
        require(match is not None, 'original prose surface missing')
        if len(response['prose'].encode()) <= DISPLAY_BYTES:
            chunks = re.split(r'(\n\s*\n)', match[1])
            html_paragraphs = [chunks[n] + (chunks[n + 1] if n + 1 < len(chunks) else '')
                               for n in range(0, len(chunks), 2) if chunks[n]]
            spans = paragraphs(response['prose'])
            require(len(html_paragraphs) == len(spans), 'HTML paragraph partition differs')
            rendered = '<section class="reader-narrative" aria-label="Original explanation">'
            previous = None
            for number, ((start, end, _), content) in enumerate(zip(spans, html_paragraphs)):
                grounded = (output.get('derived_reading', {}).get('status') == 'valid_binding'
                            and output.get('reading', {}).get('status') == 'valid_binding')
                label = heading(response if grounded else {'claims': []}, start, end, number, ko)
                if label != previous:
                    if previous is not None:
                        rendered += '</section>'
                    rendered += '<section class="reader-section"><h4 lang="' + interface_language + '">' + escaped(label) + '</h4>'
                    previous = label
                rendered += '<pre class="prose-paragraph" lang="' + escaped(language) + '" data-reading-start="'
                rendered += str(start) + '" data-reading-end="' + str(end) + '">' + content + '</pre>'
            rendered += '</section></section>'
        else:
            rendered = '<section class="reader-narrative">' + match[0] + '</section>'
        body = body[:match.start()] + rendered + body[match.end():]
        # Stage/transport classification belongs behind the ordinary narrative.
        # Keep all stage wording inspectable, without making it the reading lead.
        stage = re.compile(r'<details class="response-stage">.*?</details>', re.S).search(
            body, response_start, body.index(rendered, response_start))
        if stage:
            body = body[:stage.start()] + body[stage.end():]
            position = body.index(rendered, response_start) + len(rendered)
            body = body[:position] + stage[0] + body[position:]
        # Purpose comes from the unchanged generator-selected site's reason.
        # Exact verified path/state/coordinates remain in the existing links.
        navigation = re.compile(r'<nav class="primary-navigation".*?</nav>', re.S).search(
            body, response_start, body.index(rendered, response_start))
        if navigation and output.get('reading', {}).get('status') == 'valid_binding':
            sites = response['primary_sites']
            counter = iter(sites)
            def purpose(match):
                site = next(counter)
                return '<li><p class="site-purpose" lang="' + escaped(language) + '">' + escaped(site['reason']) + '</p>' + match[1] + '</li>'
            nav = re.sub(r'<li>(.*?)</li>', purpose, navigation[0], flags=re.S)
            nav = '<section class="reader-code-index"><h3 lang="' + interface_language + '">' + ('코드에서 확인할 내용' if ko else 'What to inspect in code') + '</h3>' + nav + '</section>'
            body = body[:navigation.start()] + body[navigation.end():]
            # Preserve response-local placement, immediately after its prose.
            position = body.index(rendered, response_start) + len(rendered)
            body = body[:position] + nav + body[position:]
    return body, {'mode': 'manual_editorial' if editorial else 'presentation_only', 'wording': 'unchanged within original paragraphs',
                  'paragraph_order': 'manually reordered' if editorial else 'unchanged',
                  'title_basis': title_basis, 'heading_basis': 'original claim classifications and bound site purposes; not semantic approval',
                  'verifier': binding(Path(__file__)), 'semantic_assessment': 'not_assessed', 'human_acceptance': 'pending',
                  'prose_hashes': [digest(r['prose'].encode()) for _, r in records]}


def reader_document(cards):
    title = ('Work explanation · manual editorial reading' if any('data-reading-kind="manual_editorial"' in c for c in cards)
             else 'Work explanation · original reading')
    return ('<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">'
            '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; style-src \'unsafe-inline\'">'
            '<title>' + title + '</title><style>' + STYLE + '</style>'
            '<a class="skip-link" href="#reading">Skip to explanation</a><main id="reading">' + ''.join(cards)
            + '<p class="scope-note">Experimental presentation only. Structural binding, semantic assessment and human acceptance remain separate.</p></main></html>')


STYLE = '''
:root{color-scheme:light;--ink:#182c38;--accent:#15615d;--line:#d7e2e3}
*{box-sizing:border-box}body{margin:0;color:var(--ink);background:#f4f7f6;font:17px/1.75 system-ui,"Noto Sans CJK KR",sans-serif;overflow-wrap:anywhere}
main{max-width:1080px;margin:auto;padding:40px 28px}article{background:#fff;border:1px solid var(--line);border-radius:18px;padding:40px;margin-bottom:32px}
header{max-width:760px;border-bottom:2px solid var(--accent);padding-bottom:20px;margin-bottom:24px}h2{font-size:clamp(1.4rem,3vw,2rem);line-height:1.5;letter-spacing:-.025em;margin:.4rem 0}
.eyebrow{font-size:.8rem;color:var(--accent);font-weight:700;margin:0}.title-basis,.reading-status,.reading-shortcuts,.scope-note{font-size:.8rem;color:#4d626e}
h3{font-size:1.25rem;margin:2rem 0 1rem}h4{font-size:1rem;color:var(--accent);margin:1.75rem 0 .65rem}
.reader-section h4{max-width:65ch;line-height:1.5}.reader-section+.reader-section{margin-top:1.5rem}
.prose-paragraph+.prose-paragraph{margin-top:.8rem}.reader-work header .title-basis{margin:.4rem 0;padding:.3rem .6rem;border:0}
.reader-narrative{max-width:72ch}.prose-paragraph,.prose{font:inherit;white-space:pre-wrap;background:none;padding:0;margin:0;overflow-wrap:anywhere}
.reader-code-index{border-block:1px solid var(--line);margin:2rem 0;padding:0 0 1rem}.primary-navigation ol{padding-left:1.4rem}.primary-navigation li{padding:.6rem .2rem}
.site-purpose{font-weight:600;margin:0 0 .25rem}.primary-navigation a{font-size:.8rem}.primary-sites>ol{padding-left:1.4rem}.primary-sites>ol>li{margin-bottom:3rem}
a{color:#0b5c78;text-underline-offset:3px}a:focus-visible,summary:focus-visible{outline:3px solid #c56a17;outline-offset:4px}:target{outline:2px solid var(--accent);scroll-margin-top:16px}
details{margin:1rem 0;border:1px solid var(--line);border-radius:7px;padding:.7rem 1rem;font-size:.85rem}summary{cursor:pointer;font-weight:600}
pre{white-space:pre-wrap;overflow-wrap:anywhere;font:14px/1.65 ui-monospace,monospace;background:#f0f5f5;padding:1rem;border-radius:8px}
pre.code,pre.diff,pre.full-diff{max-height:32rem;overflow:auto;tab-size:4;white-space:pre;overflow-wrap:normal}.primary-source{margin:1.5rem 0}.primary-reason{font-weight:650}.primary-extent{font-size:.9rem}
.gap,.language-gap{border-left:3px solid #a96817;padding:.6rem 1rem;background:#fff8ec}.recorded-direction{border-top:1px solid var(--line);margin-top:2rem}
.citation span{display:none}.citation:after{content:attr(data-location)}.skip-link{position:absolute;left:-9999px}.skip-link:focus{left:1rem;top:1rem;background:white;padding:1rem;z-index:1}
@media(max-width:600px){main{padding:12px 8px}article{padding:18px 14px;border-radius:10px}body{font-size:16px}header{padding-bottom:10px;margin-bottom:12px}h2{font-size:1.25rem}h4{margin-top:1.25rem}pre{padding:.65rem}.reader-code-index{margin:1.5rem 0}.primary-navigation .path-directory{display:none}details{padding:.5rem}.title-basis{margin:.3rem 0}}
'''
