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
        return '문제와 효과' if ko else 'Problem and effect'
    if 'recorded_next_action' in kinds:
        return '기록된 방향과 수락 상태' if ko else 'Recorded direction and acceptance'
    if kinds & {'user_choice', 'task_instruction'}:
        return '요청과 선택의 근거' if ko else 'Request and choice'
    if 'execution_evidence' in kinds:
        return '당시 검증과 중요한 한계' if ko else 'Recorded checks and important limits'
    return '동작과 구현' if ko else 'Behavior and implementation'


def present_card(body, witness):
    records = []
    for output in witness['outputs']:
        if not output.get('prose_sha256'):
            continue
        response = json.loads(Path(output['original']['path']).read_bytes())
        records.append((output, response))
    final = witness['original_invocation'].get('generation_output')
    records.sort(key=lambda item: item[0]['original'] != final)
    first = next((r for o, r in records if o['original'] == final), records[0][1] if records else None)
    language = witness['output_language']['tag']
    ko = language.startswith('ko')
    title = ('설명 원문을 사용할 수 없습니다' if ko else 'Original explanation unavailable')
    if first:
        # An explicitly attributed quotation, not a newly authored summary.
        title = re.split(r'(?<=[.!?。])\s', first['prose'], maxsplit=1)[0]
    header = ('<article class="reader-work"><header><p class="eyebrow">'
                  + ('작업 설명 · 원문 그대로' if ko else 'Work explanation · unchanged original')
                  + '</p><h2 lang="' + escaped(language) + '">' + escaped(title)
                  + '</h2><p class="title-basis">'
                  + ('제목은 원문 첫 문장의 인용입니다.' if ko else 'Title quotes the original first sentence.')
                  + '</p></header>')
    body = re.sub(r'<article><h2>.*?</h2>', lambda _: header, body, count=1, flags=re.S)
    body = re.sub(r'<h3>((?:Final|Intermediate|Captured) response.*?)</h3>',
                  r'<details class="response-stage"><summary>Original response stage</summary><p>\1</p></details>', body)
    for output, response in records:
        # Match only the verified prose surface built by the existing consumer.
        response_start = body.index('-output-' + output['original']['sha256'] + '-response-')
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
            for number, ((start, end, _), content) in enumerate(zip(spans, html_paragraphs)):
                rendered += '<h4 lang="' + escaped(language) + '">' + heading(response, start, end, number, ko) + '</h4>'
                rendered += '<pre class="prose-paragraph" lang="' + escaped(language) + '" data-original-start="'
                rendered += str(start) + '" data-original-end="' + str(end) + '">' + content + '</pre>'
            rendered += '</section>'
        else:
            rendered = '<section class="reader-narrative">' + match[0] + '</section>'
        body = body[:match.start()] + rendered + body[match.end():]
        # Purpose comes from the unchanged generator-selected site's reason.
        # Exact verified path/state/coordinates remain in the existing links.
        navigation = re.compile(r'<nav class="primary-navigation".*?</nav>', re.S).search(body, response_start)
        if navigation and output.get('reading', {}).get('status') == 'valid_binding':
            sites = response['primary_sites']
            counter = iter(sites)
            def purpose(match):
                site = next(counter)
                return '<li><p class="site-purpose" lang="' + escaped(language) + '">' + escaped(site['reason']) + '</p>' + match[1] + '</li>'
            nav = re.sub(r'<li>(.*?)</li>', purpose, navigation[0], flags=re.S)
            nav = '<section class="reader-code-index"><h3>' + ('코드에서 확인할 내용' if ko else 'What to inspect in code') + '</h3>' + nav + '</section>'
            body = body[:navigation.start()] + body[navigation.end():]
            # Preserve response-local placement, immediately after its prose.
            position = body.index(rendered) + len(rendered)
            body = body[:position] + nav + body[position:]
    return body, {'mode': 'presentation_only', 'wording': 'unchanged', 'paragraph_order': 'unchanged',
                  'title_basis': 'original first sentence quotation', 'heading_basis': 'original claim classifications; not semantic approval',
                  'verifier': binding(Path(__file__)), 'semantic_assessment': 'not_assessed', 'human_acceptance': 'pending',
                  'prose_hashes': [digest(r['prose'].encode()) for _, r in records]}


def reader_document(cards):
    return ('<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">'
            '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; style-src \'unsafe-inline\'">'
            '<title>Work explanation · original reading</title><style>' + STYLE + '</style>'
            '<a class="skip-link" href="#reading">Skip to explanation</a><main id="reading">' + ''.join(cards)
            + '<p class="scope-note">Experimental presentation only. Structural binding, semantic assessment and human acceptance remain separate.</p></main></html>')


STYLE = '''
:root{color-scheme:light;--ink:#182c38;--accent:#15615d;--line:#d7e2e3}
*{box-sizing:border-box}body{margin:0;color:var(--ink);background:#f4f7f6;font:17px/1.75 system-ui,"Noto Sans CJK KR",sans-serif;overflow-wrap:anywhere}
main{max-width:1080px;margin:auto;padding:40px 28px}article{background:#fff;border:1px solid var(--line);border-radius:18px;padding:40px;margin-bottom:32px}
header{max-width:760px;border-bottom:2px solid var(--accent);padding-bottom:20px;margin-bottom:24px}h2{font-size:clamp(1.4rem,3vw,2rem);line-height:1.5;letter-spacing:-.025em;margin:.4rem 0}
.eyebrow{font-size:.8rem;color:var(--accent);font-weight:700;margin:0}.title-basis,.reading-status,.reading-shortcuts,.scope-note{font-size:.8rem;color:#4d626e}
h3{font-size:1.25rem;margin:2rem 0 1rem}h4{font-size:1rem;color:var(--accent);margin:1.75rem 0 .65rem}
.reader-narrative{max-width:72ch}.prose-paragraph,.prose{font:inherit;white-space:pre-wrap;background:none;padding:0;margin:0;overflow-wrap:anywhere}
.reader-code-index{border-block:1px solid var(--line);margin:2rem 0;padding:0 0 1rem}.primary-navigation ol{padding-left:1.4rem}.primary-navigation li{padding:.6rem .2rem}
.site-purpose{font-weight:600;margin:0 0 .25rem}.primary-navigation a{font-size:.8rem}.primary-sites>ol{padding-left:1.4rem}.primary-sites>ol>li{margin-bottom:3rem}
a{color:#0b5c78;text-underline-offset:3px}a:focus-visible,summary:focus-visible{outline:3px solid #c56a17;outline-offset:4px}:target{outline:2px solid var(--accent);scroll-margin-top:16px}
details{margin:1rem 0;border:1px solid var(--line);border-radius:7px;padding:.7rem 1rem;font-size:.85rem}summary{cursor:pointer;font-weight:600}
pre{white-space:pre-wrap;overflow-wrap:anywhere;font:14px/1.65 ui-monospace,monospace;background:#f0f5f5;padding:1rem;border-radius:8px}
pre.code,pre.diff,pre.full-diff{max-height:32rem;overflow:auto;tab-size:4}.primary-source{margin:1.5rem 0}.primary-reason{font-weight:650}.primary-extent{font-size:.9rem}
.gap,.language-gap{border-left:3px solid #a96817;padding:.6rem 1rem;background:#fff8ec}.recorded-direction{border-top:1px solid var(--line);margin-top:2rem}
.citation span{display:none}.citation:after{content:attr(data-location)}.skip-link{position:absolute;left:-9999px}.skip-link:focus{left:1rem;top:1rem;background:white;padding:1rem;z-index:1}
@media(max-width:600px){main{padding:12px 8px}article{padding:18px 14px;border-radius:10px}body{font-size:16px}header{padding-bottom:10px;margin-bottom:12px}h2{font-size:1.25rem}h4{margin-top:1.25rem}pre{padding:.65rem}.reader-code-index{margin:1.5rem 0}.primary-navigation .path-directory{display:none}details{padding:.5rem}.title-basis{margin:.3rem 0}}
'''
