// Presentation and exact same-origin retained citation reads only. No writes.
(() => {
  const saved = new Map();
  const diagramReturns = new Map();
  const originTriggers = new Map();
  const narrow = matchMedia('(max-width:900px)');
  let selected = null;
  let focusedHash = null;
  let pendingRead = null;
  const reads = new WeakMap();
  const outlineScroll = new WeakMap();
  const ko = document.documentElement.lang === 'ko';
  const syntaxNames = {javascript:'JavaScript', typescript:'TypeScript', rust:'Rust', python:'Python', shell:'Shell', json:'JSON'};
  const tokenKinds = new Set(['keyword','comment','string','type','function','number','variable']);
  function paintCode(code, source, highlight) {
    // The server's bounded lexer is shared with script-free rendering. Treat
    // ranges as untrusted presentation hints: malformed hints fall back to text.
    const runs = highlight?.tokens;
    const language = Object.hasOwn(syntaxNames, highlight?.language) ? highlight.language : 'plain';
    const boundary = i => i === 0 || i === source.length ||
      !(source.charCodeAt(i) >= 0xdc00 && source.charCodeAt(i) <= 0xdfff && source.charCodeAt(i-1) >= 0xd800 && source.charCodeAt(i-1) <= 0xdbff);
    let end = 0;
    const valid = language !== 'plain' && Array.isArray(runs) && runs.length <= 256 && runs.every(run => {
      if (!Array.isArray(run) || run.length !== 3) return false;
      const [start, stop, kind] = run;
      if (!Number.isInteger(start) || !Number.isInteger(stop) || start < end || stop <= start || stop > source.length ||
          !tokenKinds.has(kind) || !boundary(start) || !boundary(stop)) return false;
      end = stop; return true;
    });
    const fragment = document.createDocumentFragment(); end = 0;
    if (valid) for (const [start, stop, kind] of runs) {
      if (start > end) fragment.append(document.createTextNode(source.slice(end,start)));
      const span = document.createElement('span'); span.className = 'syn-' + kind;
      span.textContent = source.slice(start,stop); fragment.append(span); end = stop;
    }
    fragment.append(document.createTextNode(source.slice(end)));
    code.replaceChildren(fragment); code.dataset.syntax = language;
    return syntaxNames[language] || (ko ? '일반 텍스트' : 'Plain text');
  }
  // A deep Source fragment carries its exact originating diagram claim. The
  // native href remains script-free; only JS adds this bounded return selector.
  function fragment() {
    const [id, origin, extra] = location.hash.slice(1).split('!');
    const target = document.getElementById(id);
    if (extra !== undefined) return {target:null};
    if (origin !== undefined) {
      const claim = document.getElementById(origin);
      if (!target?.matches('.source-point') || !claim?.matches('.diagram-claim') ||
          ![...claim.querySelectorAll('a.evidence-link')].some(a=>a.getAttribute('href')==='#'+id)) return {target:null};
    }
    return {target, origin};
  }
  function withholdDiagrams(realization) {
    // Known dependency failure must not leave a cached SVG or text alternative.
    // Retain only opaque return IDs and a safe notice, never old generated text.
    for (const figure of document.querySelectorAll('.admitted-diagram')) {
      if (figure.dataset.realization !== realization) continue;
      const ids=[...figure.querySelectorAll('[id]')].map(n=>n.id);
      const notice=document.createElement('p'); notice.setAttribute('role','status');
      notice.textContent=ko?'이 도식의 현재 근거를 사용할 수 없습니다. 현재 문서를 다시 여세요.':'This diagram’s current evidence is unavailable. Reopen the current document.';
      const placeholders=ids.map(id=>{const n=document.createElement('span');n.id=id;n.tabIndex=-1;return n;});
      figure.replaceChildren(notice,...placeholders);
      figure.dataset.unavailable='true';
    }
  }
  for (const figure of document.querySelectorAll('.admitted-diagram')) {
    const svg=figure.querySelector('svg');
    if (!svg) continue;
    figure.dataset.baseWidth=svg.getAttribute('width');
    figure.dataset.baseHeight=svg.getAttribute('height');
    figure.dataset.scale='1';figure.querySelector('.diagram-controls').hidden=false;
  }
  async function readCitation(point, offset = 0) {
    const control = point.querySelector('.citation-inspection');
    if (!control) return;
    pendingRead?.abort();
    const controller = new AbortController();
    pendingRead = controller;
    reads.set(control, controller);
    const status = control.querySelector('.citation-status');
    const button = control.querySelector('button');
    button.disabled = true;
    point.querySelectorAll('pre.code-region,.code-caption').forEach(n => n.hidden = true);
    status.textContent = ko ? '현재 허용된 인용 바이트 확인 중…' : 'Checking current permitted cited bytes…';
    try {
      const url = new URL(control.dataset.citationUrl, location.href);
      if (url.origin !== location.origin || url.pathname !== '/citation') throw Error('scope');
      url.searchParams.set('offset', String(offset));
      const response = await fetch(url, {credentials:'same-origin', cache:'no-store', redirect:'error', signal:controller.signal});
      const raw = await response.text();
      if (new TextEncoder().encode(raw).length > 32768) throw Error('bound');
      const result = JSON.parse(raw);
      if (!response.ok) { const error = Error('unavailable'); error.reason = result.reason; throw error; }
      const chunk = result.chunk;
      const textBytes = typeof chunk?.text === 'string' ? new TextEncoder().encode(chunk.text).length : 0;
      const subjectIdentity = Array.isArray(result.subject?.identity)
        ? result.subject.identity.map(b => Number.isInteger(b) && b >= 0 && b <= 255 ? b.toString(16).padStart(2,'0') : '?').join('')
        : result.subject?.identity;
      if (result.realization_fingerprint !== control.dataset.realization ||
          String(result.unit_index) !== control.dataset.unit || String(result.citation_index) !== control.dataset.citation ||
          `${result.subject?.kind}:${subjectIdentity}` !== url.searchParams.get('subject') ||
          result.language !== url.searchParams.get('language') || chunk.citation?.source_id !== point.dataset.sourceId ||
          result.claim !== point.querySelector('.claim-quote').textContent ||
          !textBytes || textBytes > 4096 || chunk.omitted_before_utf8_bytes !== offset ||
          chunk.displayed_start_byte !== chunk.citation.start_byte + offset ||
          chunk.displayed_end_byte !== chunk.displayed_start_byte + textBytes ||
          chunk.displayed_end_byte > chunk.citation.end_byte ||
          !Number.isSafeInteger(chunk.displayed_start_line) || chunk.displayed_start_line < 1 ||
          chunk.displayed_end_line - chunk.displayed_start_line + 1 !== chunk.text.split('\n').length - Number(chunk.text.endsWith('\n'))) throw Error('binding');
      if (controller.signal.aborted || reads.get(control) !== controller || selected !== point || !point.isConnected) return;
      point.querySelectorAll('pre.code-region,.code-caption,.initial-range-omission').forEach(n => n.hidden = true);
      let region = control.querySelector('pre');
      if (!region) {
        region = document.createElement('pre'); region.className = 'code-region';
        region.tabIndex = 0; region.setAttribute('role','region');
        const numbers = document.createElement('span'); numbers.className = 'source-line-numbers'; numbers.setAttribute('aria-hidden','true');
        const code = document.createElement('code'); code.lang = '';
        region.append(numbers, code);
        const caption = document.createElement('p'); caption.className = 'code-caption';
        control.prepend(caption, region);
      }
      region.hidden = false;
      const syntaxName = paintCode(region.querySelector('code'), chunk.text, result.highlight);
      control.querySelector('.code-caption').textContent = `${syntaxName} · ${ko ? '관측된 인용 범위 · 구문 색상은 읽기 보조 표시' : 'Observed cited range · syntax colors are a reading aid'}`;
      control.querySelector('.code-caption').hidden = false;
      const numbers = region.querySelector('.source-line-numbers'); numbers.replaceChildren();
      for (let line = chunk.displayed_start_line; line <= chunk.displayed_end_line; line++) {
        const marker = document.createElement('span'); marker.dataset.line = line; numbers.append(marker);
      }
      region.setAttribute('aria-label', `${chunk.citation.locator}:${chunk.displayed_start_line}–${chunk.displayed_end_line}`);
      status.textContent = `${ko ? '표시 범위' : 'Displayed'}: ${chunk.displayed_start_line}–${chunk.displayed_end_line} · bytes ${chunk.displayed_start_byte}–${chunk.displayed_end_byte} · ${ko ? '생략된 UTF-8 바이트 (앞/뒤)' : 'omitted UTF-8 bytes (before/after)'}: ${chunk.omitted_before_utf8_bytes}/${chunk.omitted_after_utf8_bytes}`;
      button.dataset.nextOffset = chunk.next_offset ?? 0;
      button.textContent = chunk.next_offset === null
        ? (ko ? '인용 범위 처음부터 다시 읽기' : 'Read cited range again from start')
        : (ko ? '다음 인용 바이트 읽기 (최대 4,096)' : 'Read next cited bytes (up to 4,096)');
    } catch (error) {
      if (controller.signal.aborted || reads.get(control) !== controller || selected !== point || !point.isConnected) return;
      // Remove previously displayed bytes from this failed reading point.
      // A deleted shared Source must not leave an earlier claim's cached code.
      document.querySelectorAll('.source-point').forEach(other => {
        if (other.dataset.sourceId === point.dataset.sourceId)
          other.querySelectorAll('pre.code-region,.code-caption').forEach(n => n.remove());
      });
      point.querySelectorAll('.initial-range-omission').forEach(n => n.hidden = true);
      const reasons = {
        expired: ko ? '보존 기간이 만료되었습니다.' : 'Retained evidence expired.',
        deleted: ko ? '설명 또는 필수 Source가 삭제되었습니다.' : 'The publication or a required Source was deleted.',
        changed: ko ? '설명 또는 근거가 변경되었습니다.' : 'The publication or its evidence changed.',
        withheld: ko ? '기록 삭제의 읽기 차단으로 보류되었습니다.' : 'Content is withheld by the forgetting barrier.',
        comparison_unavailable: ko ? '현재 저장소와 비교할 수 없습니다.' : 'A current repository comparison is unavailable.'
      };
      if (Object.hasOwn(reasons,error.reason)) withholdDiagrams(control.dataset.realization);
      status.textContent = reasons[error.reason] || (ko ? '이 정확한 인용은 이용할 수 없거나 변경되었습니다. 설명으로 돌아가 현재 버전을 다시 여세요.'
        : 'This exact citation is unavailable or changed. Return to the explanation and reopen its current version.');
      button.dataset.nextOffset = 0;
      button.textContent = ko ? '허용된 인용 다시 확인' : 'Check permitted citation again';
    } finally {
      if (reads.get(control) === controller) button.disabled = false;
    }
  }
  const fragmentNotice = document.createElement('p');
  fragmentNotice.className = 'fragment-unavailable';
  fragmentNotice.setAttribute('role', 'status');
  fragmentNotice.hidden = true;
  fragmentNotice.textContent = ko ? '이 문단 링크는 현재 문서에서 사용할 수 없습니다. 목차에서 현재 내용을 선택하세요.'
    : 'This section link is unavailable in the current document. Choose a current section from the TOC.';
  document.querySelector('main')?.prepend(fragmentNotice);
  function updateCue() {
    document.querySelectorAll('.document-nav').forEach(toc => {
      const links = [...toc.querySelectorAll('.toc-primary a')];
      let current = null;
      for (const link of links) {
        const node = document.getElementById(link.hash.slice(1));
        if (node && node.getBoundingClientRect().top <= 180) current = link;
      }
      for (const link of links) {
        if (link === current) link.setAttribute('aria-current', 'location');
        else link.removeAttribute('aria-current');
      }
      toc.querySelector('.toc-cue').textContent = current?.textContent || (ko ? '이 문서의 내용' : 'On this page');
    });
  }
  function apply() {
    const {target,origin:diagramOrigin} = fragment();
    fragmentNotice.hidden = !location.hash || !!target;
    if (target) {
      if (target.matches('details')) target.open = true;
      for (let parent = target.parentElement; parent; parent = parent.parentElement) {
        if (parent.matches('details')) parent.open = true;
      }
    }
    const point = target?.closest('.source-point');
    document.body.dataset.sourceOpen=point?'true':'false';
    document.querySelectorAll('.source-point[data-selected],.appendix[data-open],.diagram-claim[data-active]').forEach(n=>{delete n.dataset.selected;delete n.dataset.open;delete n.dataset.active;});
    if (point) {
      point.dataset.selected='true';point.closest('.appendix').dataset.open='true';
      const returnId=diagramOrigin || point.dataset.origin;
      point.querySelector('a.return').setAttribute('href','#'+returnId);
      const claim=document.getElementById(returnId);
      if (claim?.matches('.diagram-claim')) claim.dataset.active='true';
    }
    updateCue();
    const articles = document.querySelectorAll('.doc');
    const header = document.querySelector('.masthead');
    articles.forEach(article => article.inert = !!point && narrow.matches);
    if (header) header.inert = !!point && narrow.matches;
    if (selected && selected !== point) {
      pendingRead?.abort();
      selected.removeAttribute('role');
      selected.removeAttribute('aria-modal');
    }
    if (point) {
      const changed = focusedHash !== location.hash;
      selected = point;
      focusedHash = location.hash;
      if (narrow.matches) {
        point.setAttribute('role', 'dialog');
        point.setAttribute('aria-modal', 'true');
        point.setAttribute('aria-label', point.querySelector('.source-path').textContent);
      } else {
        point.removeAttribute('role');
        point.removeAttribute('aria-modal');
      }
      const origin = document.getElementById(diagramOrigin || point.dataset.origin);
      if (changed) readCitation(point);
      if (changed && !narrow.matches) origin?.scrollIntoView({block:'center'});
      if (!point.contains(document.activeElement)) point.focus({preventScroll:true});
    } else if (selected) {
      pendingRead?.abort();
      selected = null;
      focusedHash = null;
      if (target) {
        const trigger = originTriggers.get(target.id);
        (trigger?.isConnected ? trigger : target).focus({preventScroll:true});
        const offset = saved.get(target.id);
        if (offset !== undefined) window.scrollTo(0, offset);
        else target.scrollIntoView({block:'center'});
      }
    }
    if (!point && target?.matches('.diagram-claim')) {
      target.focus({preventScroll:true});target.scrollIntoView({block:'nearest'});
    }
    if (!point && target?.matches('.diagram-element')) {
      const prior=diagramReturns.get(target.id);
      (prior?.trigger?.isConnected?prior.trigger:target).focus({preventScroll:true});
      if (prior) {
        const viewport=target.closest('.diagram-viewport');
        viewport.scrollLeft=prior.left;viewport.scrollTop=prior.top;
        window.scrollTo(0,prior.y);
      } else target.scrollIntoView({block:'center',inline:'center'});
    }
  }
  document.addEventListener('focusin',event=>{
    const element=event.target.closest?.('a.diagram-element');
    if(element && !diagramReturns.has(element.id)) element.scrollIntoView({block:'nearest',inline:'nearest'});
  });
  document.addEventListener('click', event => {
    const diagramControl=event.target.closest('button[data-diagram-action]');
    if (diagramControl) {
      const figure=diagramControl.closest('.admitted-diagram');const action=diagramControl.dataset.diagramAction;
      const svg=figure.querySelector('svg');
      if (action==='expand') {
        const expanded=figure.classList.toggle('diagram-expanded');
        diagramControl.setAttribute('aria-expanded',String(expanded));
      } else {
        const old=Number(figure.dataset.scale);const next=action==='reset'?1:Math.min(2,Math.max(.75,old+(action==='in'?.25:-.25)));
        figure.dataset.scale=String(next);svg.style.width=(Number(figure.dataset.baseWidth)*next)+'px';svg.style.height=(Number(figure.dataset.baseHeight)*next)+'px';
        figure.querySelector('.diagram-scale').textContent=Math.round(next*100)+'%';
        figure.querySelector('[data-diagram-action=in]').disabled=next===2;
        figure.querySelector('[data-diagram-action=out]').disabled=next===.75;
      }
    }
    const diagramLink=event.target.closest('a.diagram-element,a.diagram-text-link');
    if (diagramLink) {
      const id=diagramLink.matches('.diagram-element')?diagramLink.id:diagramLink.dataset.element;
      const element=document.getElementById(id);const viewport=element?.closest('.diagram-viewport');
      if (viewport) diagramReturns.set(id,{trigger:diagramLink,y:scrollY,left:viewport.scrollLeft,top:viewport.scrollTop});
      requestAnimationFrame(apply);
    }
    const diagramReturn=event.target.closest('a.diagram-return');
    if (diagramReturn) {
      event.preventDefault();
      history.pushState(null,'',diagramReturn.getAttribute('href'));
      apply();requestAnimationFrame(apply);
    }
    const button = event.target.closest('button.citation-read');
    if (button) readCitation(button.closest('.source-point'), Number(button.dataset.nextOffset || 0));
    const outlineTrigger = event.target.closest('.document-nav > summary');
    if (outlineTrigger) {
      const toc = outlineTrigger.parentElement;
      if (!toc.open) outlineScroll.set(toc, scrollY);
      else {
        const offset = outlineScroll.get(toc);
        requestAnimationFrame(() => { if (offset !== undefined) window.scrollTo(0, offset); });
      }
    }
    const outlineLink = event.target.closest('.document-nav a');
    if (outlineLink) {
      const toc = outlineLink.closest('.document-nav');
      toc.open = false;
      const target = document.getElementById(outlineLink.hash.slice(1));
      // Run even for a repeated fragment, which emits no hashchange.
      requestAnimationFrame(() => {
        apply();
        if (target) { target.focus({preventScroll:true}); target.scrollIntoView({block:'start'}); }
      });
    }
    const link = event.target.closest('a.evidence-link');
    if (link && (!selected || link.closest('.diagram-claim'))) {
      if (!selected) saved.set(link.dataset.origin, scrollY);
      originTriggers.set(link.dataset.origin, link);
      document.querySelectorAll('.document-nav[open]').forEach(toc => toc.open = false);
      if (link.closest('.diagram-claim')) {
        event.preventDefault();
        history.pushState(null,'',link.getAttribute('href')+'!'+link.dataset.origin);
        apply();
      }
    }
  });
  document.addEventListener('keydown', event => {
    const point = fragment().target?.closest('.source-point') || selected;
    if (!point) {
      const toc = event.target.closest('.document-nav[open]');
      if (event.key === 'Escape' && toc) {
        event.preventDefault(); toc.open = false; toc.querySelector('summary').focus({preventScroll:true});
        const offset = outlineScroll.get(toc);
        if (offset !== undefined) window.scrollTo(0, offset);
      }
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      point.querySelector('.return').click();
      apply();
    } else if (event.key === 'Tab' && narrow.matches) {
      const controls = [...point.querySelectorAll('a[href],button:not(:disabled),summary,[tabindex="0"]')]
        .filter(n => n.getClientRects().length && !n.closest('details:not([open]) :not(summary)'));
      const first = controls[0], last = controls.at(-1);
      if (event.shiftKey && (document.activeElement === first || document.activeElement === point)) {
        event.preventDefault(); last?.focus();
      } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === point)) {
        event.preventDefault(); first?.focus();
      }
    }
  });
  window.addEventListener('hashchange', apply);
  window.addEventListener('popstate', () => {apply();requestAnimationFrame(()=>{if(fragment().target?.matches('.diagram-element')) apply();});});
  let scheduled = false;
  window.addEventListener('scroll', () => {
    if (!scheduled) { scheduled = true; requestAnimationFrame(() => { scheduled = false; updateCue(); }); }
  }, {passive:true});
  narrow.addEventListener('change', apply);
  apply();
})();
