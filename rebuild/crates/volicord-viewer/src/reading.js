// Presentation and exact same-origin retained citation reads only. No writes.
(() => {
  document.body.dataset.readerEnhanced='true';
  const diagramReturns = new Map();
  const sourceReturns = new Map();
  const expandedReturns = new WeakMap();
  let navigationSequence = 0;
  const remember = (map, value) => {
    const token = String(++navigationSequence);
    map.set(token, value);
    if (map.size > 128) map.delete(map.keys().next().value);
    return token;
  };
  const state = () => history.state && typeof history.state === 'object' ? history.state : {};
  function navigate(hash, next = state()) {
    // Native fragment navigation updates :target as well as URL/history.
    // pushState alone leaves CSS targeting a previously selected element.
    if (location.hash !== hash) location.hash = hash;
    history.replaceState(next,'');
    apply(); requestAnimationFrame(apply);
  }
  function closeExpansion(figure) {
    const prior = expandedReturns.get(figure);
    figure.classList.remove('diagram-expanded');
    figure.removeAttribute('role');figure.removeAttribute('aria-modal');synchronizeModal(null);
    const control = figure.querySelector('[data-diagram-action=expand]');
    control.setAttribute('aria-expanded','false');
    control.textContent = ko ? '확대·이동 보기' : 'Zoom and pan view';
    if (prior) {
      const viewport = figure.querySelector('.diagram-viewport');
      viewport.scrollLeft = prior.left; viewport.scrollTop = prior.top;
      control.focus({preventScroll:true}); window.scrollTo(0,prior.y);
    }
  }
  const narrow = matchMedia('(max-width:1199px)');
  let selected = null;
  let selectedReturnToken = null;
  let focusedHash = null;
  let lastAppliedHash = null;
  let pendingRead = null;
  const reads = new WeakMap();
  const codeBuffers = new WeakMap();
  const modalSiblings = new Map();
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
    const valid = language !== 'plain' && Array.isArray(runs) && runs.length <= 1024 && runs.every(run => {
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
    const [id, encodedOrigin, extra] = location.hash.slice(1).split('!');
    let origin=encodedOrigin;
    let target = document.getElementById(id);
    if(target?.matches('.shared-source-target,.source-origin-target')){
      const nativeOrigin=target.dataset.origin;
      const selectedPoint=document.getElementById(target.dataset.selectedPoint);
      if(!selectedPoint?.matches('.source-point')||selectedPoint.dataset.readerPublication!==target.closest('.source-point')?.dataset.readerPublication)return {target:null};
      if(nativeOrigin){if(origin&&origin!==nativeOrigin)return {target:null};origin=nativeOrigin;}
      target=selectedPoint;
    }
    if (extra !== undefined) return {target:null};
    if (origin !== undefined) {
      const claim = document.getElementById(origin);
      if (!target?.matches('.source-point') || !claim?.matches('.diagram-claim') ||
          ![...claim.querySelectorAll('a.evidence-link')].some(a=>a.getAttribute('href')==='#'+id||a.getAttribute('href')+'-shared-code'==='#'+id)) return {target:null};
    }
    return {target, origin};
  }
  function withholdPublication(publication, reason) {
    // A citation failure invalidates all dependent meaning, not just its file.
    // Retain opaque return positions, but no hidden code, prose or alternatives.
    // Return memories contain DOM references; release affected detached nodes
    // as well as the visible regions and the original-code buffers below.
    for(const memory of [diagramReturns,sourceReturns]){
      for(const [token,prior] of memory){
        if([prior.trigger,prior.figure,prior.viewport].some(node=>node?.closest?.('[data-reader-publication]')?.dataset.readerPublication===publication))memory.delete(token);
      }
    }
    for (const region of document.querySelectorAll('[data-reader-publication]')) {
      if (region.dataset.readerPublication !== publication || !region.isConnected) continue;
      region.classList.remove('diagram-expanded');
      region.querySelectorAll('.source-point').forEach(n=>codeBuffers.delete(n));
      if(region.matches('.source-point'))codeBuffers.delete(region);
      const ids=[...region.querySelectorAll('[id]')].map(n=>n.id);
      const back=region.matches('.source-point') ? region.querySelector('a.return')?.cloneNode(true) : null;
      const notice=document.createElement('p'); notice.setAttribute('role','status');
      notice.className='publication-unavailable';
      notice.textContent=reason+' '+(ko?'현재 문서를 다시 여세요.':'Reopen the current document.');
      const placeholders=ids.map(id=>{const n=document.createElement('span');n.id=id;n.tabIndex=-1;return n;});
      region.replaceChildren(...(back?[back]:[]),notice,...placeholders);
      region.dataset.unavailable='true';
    }
  }
  function synchronizeModal(root) {
    for(const [node,prior] of modalSiblings) if(node.isConnected)node.inert=prior;
    modalSiblings.clear();
    if(!root)return;
    for(let child=root;child?.parentElement;child=child.parentElement){
      for(const sibling of child.parentElement.children){
        if(sibling===child||sibling.matches('script,.evidence-backdrop'))continue;
        modalSiblings.set(sibling,sibling.inert);sibling.inert=true;
      }
      if(child.parentElement===document.body)break;
    }
  }
  function sizeFigure(figure) {
    const svg=figure.querySelector('svg.connected-overview');if(!svg)return;
    const available=figure.querySelector('.diagram-viewport').clientWidth;
    const variant=available>=760?'wide':available>=520?'medium':'narrow';
    if(figure.dataset.variant===variant)return;
    figure.dataset.variant=variant;
    for(const name of ['width','height']){
      const size=parseFloat(svg.style.getPropertyValue(`--map-${variant}-${name}`));
      figure.dataset[name==='width'?'baseWidth':'baseHeight']=size;
      svg.style[name]=(size*(Number(figure.dataset.scale)||1))+'px';
    }
  }
  for (const figure of document.querySelectorAll('.admitted-diagram')) {
    if(!figure.querySelector('svg'))continue;
    figure.dataset.scale='1';figure.querySelector('.diagram-controls').hidden=false;
    new ResizeObserver(()=>sizeFigure(figure)).observe(figure.querySelector('.diagram-viewport'));
    sizeFigure(figure);
  }
  function purgeBody(source,reason='expired') {
    if(selected?.dataset.sourceId===source)pendingRead?.abort();
    for(const point of document.querySelectorAll('.source-point')){
      if(point.dataset.sourceId!==source)continue;
      codeBuffers.delete(point);
      point.querySelectorAll('pre.code-region,.code-caption,.initial-range-omission').forEach(n=>n.remove());
      point.dataset.rangeState=reason;
      let notice=point.querySelector('.range-unavailable');
      if(!notice){notice=document.createElement('p');notice.className='source-note range-unavailable';point.querySelector('.source-path')?.after(notice);}
      notice.dataset.rangeState=reason;
      notice.textContent=reason==='expired'?(ko?'보존 기간이 만료되었습니다. 원래 코드를 제공할 수 없습니다.':'Retained evidence expired. The original code is unavailable.'):(ko?'이 원래 근거 범위를 이용할 수 없습니다.':'This original evidence range is unavailable.');
      const status=point.querySelector('.citation-status');if(status)status.textContent=notice.textContent;
    }
  }
  let expiryTimer;
  function expireBodies(){
    clearTimeout(expiryTimer);let earliest=Infinity;
    for(const marker of document.querySelectorAll('.source-deadline[data-expires-at]')){
      const deadline=Number(marker.dataset.expiresAt)/1000;
      if(!Number.isFinite(deadline))continue;
      if(deadline<=Date.now()){purgeBody(marker.closest('.source-point').dataset.sourceId);marker.remove();}
      else earliest=Math.min(earliest,deadline);
    }
    if(Number.isFinite(earliest))expiryTimer=setTimeout(expireBodies,Math.min(2147483647,Math.max(1,earliest-Date.now()+1)));
  }
  function updateAvailability(publication,basis){
    for(const availability of basis?.source_ranges||[]){
      const point=[...document.querySelectorAll('.source-point')].find(point=>{
        const c=point.querySelector('.citation-inspection');return c?.dataset.publicationId===publication&&Number(c.dataset.unit)===availability.unit_index&&Number(c.dataset.citation)===availability.citation_index;
      });
      if(!point)continue;
      if(!availability.available){purgeBody(point.dataset.sourceId,availability.limitation||'unavailable');continue;}
      if(Number.isSafeInteger(availability.expires_at_unix_micros)){
        const marker=point.querySelector('.source-deadline');if(marker)marker.dataset.expiresAt=String(availability.expires_at_unix_micros);
      }
    }
    expireBodies();
  }
  function updateComparison(publication, basis) {
    const labels = {
      verified_at_read: ko ? '이번 비교에서 로컬 저장소가 작성 당시 스냅샷과 일치했습니다. 원격 최신성은 확인하지 않았습니다.' : 'This local comparison matched the original snapshot. Remote freshness was not checked.',
      changed: ko ? '저장소가 변경되었습니다. 설명은 작성 당시 근거로 보존되며 의미상 영향은 알 수 없습니다.' : 'Repository changed. The explanation retains its original basis; semantic impact is unknown.',
      unavailable: ko ? '현재 저장소 비교를 이용할 수 없습니다. 설명은 작성 당시 근거로 보존됩니다.' : 'Current repository comparison is unavailable. The explanation retains its original basis.',
      not_checked: ko ? '현재 저장소 비교가 확립되지 않았습니다.' : 'No current repository comparison is established.'
    };
    if (!basis || !Object.hasOwn(labels,basis.repository_comparison)) return;
    for(const point of document.querySelectorAll('.source-point')){
      if(point.dataset.readerPublication!==publication)continue;
      const note=point.querySelector('[data-evidence-comparison]');if(note)note.textContent=labels[basis.repository_comparison];
    }
    for (const region of document.querySelectorAll('.applicability')) {
      if (region.dataset.readerPublication !== publication) continue;
      region.dataset.comparisonState = basis.repository_comparison;
      region.querySelector('summary').textContent = labels[basis.repository_comparison];
      const time = region.querySelector('[data-comparison-time]');
      if (time && Number.isSafeInteger(basis.checked_at_unix_micros)) time.textContent = new Date(basis.checked_at_unix_micros/1000).toISOString();
    }
  }
  async function readCitation(point, offset = 0) {
    if (point.dataset.unavailable === 'true') return;
    const control = point.querySelector('.citation-inspection');
    if (!control) return;
    pendingRead?.abort();
    if(offset===0)codeBuffers.delete(point);
    const controller = new AbortController();
    pendingRead = controller;
    reads.set(control, controller);
    const status = control.querySelector('.citation-status');
    const button = control.querySelector('button');
    button.disabled = true;
    point.querySelectorAll('.source-code-reference').forEach(n => n.hidden = true);
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
      if (!response.ok) { const error = Error('unavailable'); error.reason = result.reason; error.publicationReadable = result.publication_readable === true; throw error; }
      const chunk = result.chunk;
      const expectedCitation = JSON.parse(control.dataset.citationBinding);
      // Identity at the publication/unit level cannot stand in for the exact
      // Source revision, snapshot, range, claim and use selected on this page.
      const boundCitation = chunk?.citation &&
        Object.keys(chunk.citation).length === Object.keys(expectedCitation).length &&
        Object.entries(expectedCitation).every(([key,value]) => chunk.citation[key] === value);
      const textBytes = typeof chunk?.text === 'string' ? new TextEncoder().encode(chunk.text).length : 0;
      const subjectIdentity = Array.isArray(result.subject?.identity)
        ? result.subject.identity.map(b => Number.isInteger(b) && b >= 0 && b <= 255 ? b.toString(16).padStart(2,'0') : '?').join('')
        : result.subject?.identity;
      if (!boundCitation || result.publication_id !== control.dataset.publicationId ||
          result.realization_fingerprint !== control.dataset.realization ||
          String(result.unit_index) !== control.dataset.unit || String(result.citation_index) !== control.dataset.citation ||
          `${result.subject?.kind}:${subjectIdentity}` !== url.searchParams.get('subject') ||
          result.language !== url.searchParams.get('language') || chunk.citation?.source_id !== point.dataset.sourceId ||
          result.claim !== point.querySelector('.claim-quote').textContent ||
          !textBytes || textBytes > 4096 || chunk.omitted_before_utf8_bytes !== offset ||
          chunk.displayed_start_byte !== chunk.citation.start_byte + offset ||
          chunk.displayed_end_byte !== chunk.displayed_start_byte + textBytes ||
          chunk.displayed_end_byte > chunk.citation.end_byte ||
          chunk.omitted_after_utf8_bytes !== chunk.citation.end_byte - chunk.displayed_end_byte ||
          chunk.next_offset !== (chunk.omitted_after_utf8_bytes === 0 ? null : offset + textBytes) ||
          !Number.isSafeInteger(chunk.displayed_start_line) || chunk.displayed_start_line < 1 ||
          chunk.displayed_end_line - chunk.displayed_start_line + 1 !== chunk.text.split('\n').length - Number(chunk.text.endsWith('\n'))) throw Error('binding');
      if (controller.signal.aborted || reads.get(control) !== controller || selected !== point || !point.isConnected) return;
      updateComparison(control.dataset.publicationId,result.applicability);
      updateAvailability(control.dataset.publicationId,result.applicability);
      const originalDeadline=point.querySelector('.source-deadline');
      if(point.dataset.rangeState==='expired'||(originalDeadline&&Number(originalDeadline.dataset.expiresAt)/1000<=Date.now())){
        const error=Error('body expired during read');error.reason='expired';error.publicationReadable=true;throw error;
      }
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
      let buffer=codeBuffers.get(point);
      if(offset===0)buffer={text:'',tokens:[],startLine:chunk.displayed_start_line,language:result.highlight?.language};
      if(!buffer || new TextEncoder().encode(buffer.text).length!==offset || offset+textBytes>16384)throw Error('contiguous range');
      const previousLength=buffer.text.length;
      buffer.text+=chunk.text;
      for(const token of result.highlight?.tokens||[])buffer.tokens.push([token[0]+previousLength,token[1]+previousLength,token[2]]);
      codeBuffers.set(point,buffer);
      const syntaxName = paintCode(region.querySelector('code'), buffer.text, {language:buffer.language,tokens:buffer.tokens});
      control.querySelector('.code-caption').textContent = `${syntaxName} · ${ko ? '관측된 인용 범위 · 구문 색상은 읽기 보조 표시' : 'Observed cited range · syntax colors are a reading aid'}`;
      control.querySelector('.code-caption').hidden = false;
      const numbers = region.querySelector('.source-line-numbers'); numbers.replaceChildren();
      for (let line = buffer.startLine; line <= chunk.displayed_end_line; line++) {
        const marker = document.createElement('span'); marker.dataset.line = line; numbers.append(marker);
      }
      region.setAttribute('aria-label', `${chunk.citation.locator}:${buffer.startLine}–${chunk.displayed_end_line}`);
      status.textContent = `${ko ? '표시 범위' : 'Displayed'}: ${buffer.startLine}–${chunk.displayed_end_line} · bytes ${chunk.citation.start_byte}–${chunk.displayed_end_byte} · ${ko ? '생략된 UTF-8 바이트 (앞/뒤)' : 'omitted UTF-8 bytes (before/after)'}: 0/${chunk.omitted_after_utf8_bytes}`;
      button.dataset.nextOffset = chunk.next_offset ?? 0;
      button.textContent = chunk.next_offset === null
        ? (ko ? '인용 범위 처음부터 다시 읽기' : 'Read cited range again from start')
        : (ko ? '전체 인용 범위 읽는 중…' : 'Reading the complete cited range…');
      if(chunk.next_offset!==null)await readCitation(point,chunk.next_offset);
    } catch (error) {
      if (controller.signal.aborted || reads.get(control) !== controller || selected !== point || !point.isConnected) return;
      // Remove previously displayed bytes from this failed reading point.
      // A deleted shared Source must not leave an earlier claim's cached code.
      document.querySelectorAll('.source-point').forEach(other => {
        if (other.dataset.sourceId === point.dataset.sourceId) {
          codeBuffers.delete(other);
          other.querySelectorAll('pre.code-region,.code-caption').forEach(n => n.remove());
          if (error.reason) other.dataset.rangeState = error.reason;
        }
      });
      point.querySelectorAll('.initial-range-omission').forEach(n => n.hidden = true);
      const reasons = {
        expired: ko ? '보존 기간이 만료되었습니다.' : 'Retained evidence expired.',
        deleted: ko ? '설명 또는 필수 Source가 삭제되었습니다.' : 'The publication or a required Source was deleted.',
        changed: ko ? '설명 또는 근거가 변경되었습니다.' : 'The publication or its evidence changed.',
        withheld: ko ? '기록 삭제의 읽기 차단으로 보류되었습니다.' : 'Content is withheld by the forgetting barrier.',
        comparison_unavailable: ko ? '현재 저장소와 비교할 수 없습니다.' : 'A current repository comparison is unavailable.',
        unavailable: ko ? '설명 또는 필수 근거를 이용할 수 없습니다.' : 'The publication or required evidence is unavailable.',
        missing: ko ? '설명 또는 필수 근거가 없습니다.' : 'The publication or required evidence is missing.',
        corrupt: ko ? '설명 또는 근거의 무결성을 확인할 수 없습니다.' : 'The publication or evidence failed its integrity check.',
        unsupported: ko ? '설명 또는 근거의 형식이 지원되지 않습니다.' : 'The publication or evidence format is unsupported.'
      };
      if (!error.publicationReadable && Object.hasOwn(reasons,error.reason)) withholdPublication(control.dataset.publicationId,reasons[error.reason]);
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
      const links=[...toc.querySelectorAll('.toc-primary a')];
      const clearance=toc.getBoundingClientRect().bottom+16;
      let current=links[0];
      for(const link of links){const node=document.getElementById(link.hash.slice(1));if(node&&node.getBoundingClientRect().top<=clearance)current=link;}
      const article=toc.closest('.doc');
      if(article&&article.getBoundingClientRect().bottom<=innerHeight+2)current=links.at(-1);
      for(const link of links){if(link===current)link.setAttribute('aria-current','location');else link.removeAttribute('aria-current');}
      const cue=toc.querySelector('.toc-cue');
      if(cue)cue.textContent=current?.querySelector('span:last-child')?.textContent||(ko?'이 문서의 내용':'On this page');
      const count=toc.querySelector('.toc-position');if(count)count.textContent=links.length?`${links.indexOf(current)+1} / ${links.length}`:'';
    });
  }
  function apply() {
    const {target,origin:diagramOrigin} = fragment();
    let restoredSource = false;
    const navigationChanged = lastAppliedHash !== location.hash;
    lastAppliedHash = location.hash;
    fragmentNotice.hidden = !location.hash || !!target;
    if (target) {
      if (target.matches('details')) target.open = true;
      for (let parent = target.parentElement; parent; parent = parent.parentElement) {
        if (parent.matches('details')) parent.open = true;
      }
    }
    if (navigationChanged && target?.matches('h2,h3')) {
      target.focus({preventScroll:true});requestAnimationFrame(()=>target.scrollIntoView({block:'start'}));
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
    if(!point&&target?.matches('.diagram-claim'))target.dataset.active='true';
    updateCue();
    const expanded=document.querySelector('.diagram-expanded');
    synchronizeModal(point&&narrow.matches?point:!point?expanded:null);
    if(expanded){expanded.setAttribute('role','dialog');expanded.setAttribute('aria-modal',String(!point));}
    const backdrop=document.querySelector('.evidence-backdrop');if(backdrop)backdrop.hidden=!(point&&narrow.matches);
    document.querySelectorAll('.diagram-element[data-selected]').forEach(n=>delete n.dataset.selected);
    if(diagramOrigin){const element=document.getElementById(document.getElementById(diagramOrigin)?.dataset.element);if(element)element.dataset.selected='true';}
    if (selected && selected !== point) {
      pendingRead?.abort();
      selected.removeAttribute('role');
      selected.removeAttribute('aria-modal');
    }
    if (point) {
      const changed = focusedHash !== location.hash;
      selected = point;
      selectedReturnToken = state().sourceReturn || null;
      focusedHash = location.hash;
      if (narrow.matches) {
        point.setAttribute('role', 'dialog');
        point.setAttribute('aria-modal', 'true');
        point.setAttribute('aria-label', point.querySelector('.source-path')?.textContent ||
          (ko ? '근거를 사용할 수 없습니다' : 'Evidence unavailable'));
      } else {
        point.removeAttribute('role');
        point.removeAttribute('aria-modal');
      }
      const origin = document.getElementById(diagramOrigin || point.dataset.origin);
      const title=point.querySelector('.evidence-claim');
      if(title){const wording=origin?.querySelector('.diagram-selected-label')?.textContent;title.textContent=wording?`${origin.querySelector('h3').textContent}: ${wording}`:point.querySelector('.claim-quote')?.textContent||'';}
      if (changed) readCitation(point);
      if (changed && !narrow.matches && !state().sourceReturn) origin?.scrollIntoView({block:'center'});
      if (!point.contains(document.activeElement)) (point.querySelector('a.return')||point).focus({preventScroll:true});
    } else if (selected) {
      restoredSource = true;
      pendingRead?.abort();
      selected = null;
      focusedHash = null;
      if (target) {
        const sourceReturn = sourceReturns.get(selectedReturnToken || state().sourceBack || state().sourceReturn);
        selectedReturnToken = null;
        const trigger = sourceReturn?.claim === target.id ? sourceReturn.trigger : null;
        const element=target.matches('.diagram-claim')?document.getElementById(target.dataset.element):null;
        const destination = trigger?.isConnected ? trigger : element||target;
        const offset = sourceReturn?.claim === target.id ? sourceReturn.y : undefined;
        const hash = location.hash;
        const restore = () => {
          if (selected || location.hash !== hash || !destination.isConnected) return;
          const active = document.activeElement;
          // Native fragment navigation may focus the paragraph after this
          // event. Restore the activating Source button once more after that
          // navigation, without taking focus from another user-selected control.
          if (active !== target && active !== destination && active !== document.body) return;
          if(sourceReturn?.figure?.isConnected){sourceReturn.figure.scrollLeft=sourceReturn.figureLeft;sourceReturn.figure.scrollTop=sourceReturn.figureTop;}
          if(sourceReturn?.viewport?.isConnected){sourceReturn.viewport.scrollLeft=sourceReturn.left;sourceReturn.viewport.scrollTop=sourceReturn.top;}
          destination.focus({preventScroll:true});
          if (offset !== undefined) {window.scrollTo(0, offset);if(sourceReturn.triggerTop!==undefined)window.scrollBy(0,destination.getBoundingClientRect().top-sourceReturn.triggerTop);}
          else destination.scrollIntoView({block:'center'});
        };
        destination.focus({preventScroll:true});
        if (offset !== undefined) window.scrollTo(0, offset);
        else destination.scrollIntoView({block:'center'});
        requestAnimationFrame(()=>requestAnimationFrame(restore));
      }
    }
    if (!point && !restoredSource && navigationChanged && target?.matches('.diagram-claim')) {
      target.dataset.active='true';target.focus({preventScroll:true});target.scrollIntoView({block:'nearest'});
    }
    if (!point && target?.matches('.diagram-element')) {
      const candidate=diagramReturns.get(state().diagramReturn);
      const prior=candidate?.element===target.id?candidate:null;
      if (!navigationChanged && document.activeElement !== target && document.activeElement !== prior?.trigger) return;
      (prior?.trigger?.isConnected?prior.trigger:target).focus({preventScroll:true});
      if (prior) {
        const viewport=target.closest('.diagram-viewport');
        const figure=target.closest('.admitted-diagram');
        const claim=document.getElementById(prior.claim);
        if (claim?.matches('.diagram-claim') && claim.closest('.admitted-diagram')===figure) claim.dataset.active='true';
        figure.scrollLeft=prior.figureLeft;figure.scrollTop=prior.figureTop;
        viewport.scrollLeft=prior.left;viewport.scrollTop=prior.top;
        window.scrollTo(0,prior.y);
        const trigger=prior.trigger?.isConnected?prior.trigger:target;
        window.scrollBy(0,trigger.getBoundingClientRect().top-prior.triggerTop);
      } else target.scrollIntoView({block:'center',inline:'center'});
    }
  }
  window.addEventListener('popstate',()=>{
    if (!location.hash) {
      const prior=diagramReturns.get(state().diagramBack);
      if(prior?.trigger?.isConnected) {prior.trigger.focus({preventScroll:true});window.scrollTo(0,prior.y);}
    }
  });
  document.addEventListener('focusin',event=>{
    const element=event.target.closest?.('a.diagram-element');
    if(element && diagramReturns.get(state().diagramReturn)?.element !== element.id) element.scrollIntoView({block:'nearest',inline:'nearest'});
  });
  function openEvidence(link, trigger=link, activation=null) {
    document.querySelectorAll('.document-nav[open]').forEach(toc=>toc.open=false);
    const figure=trigger.closest('.admitted-diagram'),viewport=figure?.querySelector('.diagram-viewport');
    const token=remember(sourceReturns,{claim:link.dataset.origin,trigger,y:activation?.y??scrollY,triggerTop:activation?.triggerTop??trigger.getBoundingClientRect().top,figure,figureLeft:activation?.figureLeft??figure?.scrollLeft,figureTop:activation?.figureTop??figure?.scrollTop,viewport,left:activation?.left??viewport?.scrollLeft,top:activation?.top??viewport?.scrollTop});
    const next={...state(),sourceReturn:token};
    history.replaceState({...state(),sourceBack:token},'');
    const hash=link.getAttribute('href')+(link.closest('.diagram-claim')?'!'+link.dataset.origin:'');
    const repeated=location.hash===hash;navigate(hash,next);
    if(repeated&&selected)readCitation(selected);
  }
  document.addEventListener('click', event => {
    // Modified links belong to native navigation (new tab/window/download).
    // They must not select, close, focus or scroll this reading context.
    const plainLink = event.button === 0 && !event.ctrlKey && !event.metaKey && !event.shiftKey && !event.altKey;
    const diagramControl=event.target.closest('button[data-diagram-action]');
    if (diagramControl) {
      const figure=diagramControl.closest('.admitted-diagram');const action=diagramControl.dataset.diagramAction;
      const svg=figure.querySelector('svg');
      if (action==='expand') {
        if (figure.classList.contains('diagram-expanded')) closeExpansion(figure);
        else {
          const viewport=figure.querySelector('.diagram-viewport');
          expandedReturns.set(figure,{y:scrollY,left:viewport.scrollLeft,top:viewport.scrollTop});
          figure.classList.add('diagram-expanded');
          diagramControl.setAttribute('aria-expanded','true');
          diagramControl.textContent=ko?'개요로 돌아가기 (Esc)':'Return to overview (Esc)';
          figure.setAttribute('role','dialog');figure.setAttribute('aria-modal','true');synchronizeModal(figure);
          sizeFigure(figure);
        }
      } else {
        const old=Number(figure.dataset.scale);const next=action==='reset'?1:Math.min(2,Math.max(.75,old+(action==='in'?.25:-.25)));
        figure.dataset.scale=String(next);svg.style.width=(Number(figure.dataset.baseWidth)*next)+'px';svg.style.height=(Number(figure.dataset.baseHeight)*next)+'px';
        svg.querySelector('.diagram-content')?.setAttribute('transform',`scale(${next})`);
        figure.querySelector('.diagram-scale').textContent=Math.round(next*100)+'%';
        figure.querySelector('[data-diagram-action=in]').disabled=next===2;
        figure.querySelector('[data-diagram-action=out]').disabled=next===.75;
      }
    }
    const diagramLink=event.target.closest('a.diagram-element,a.diagram-text-link');
    if (diagramLink) {
      const id=diagramLink.matches('.diagram-element')?diagramLink.id:diagramLink.dataset.element;
      const element=document.getElementById(id);const viewport=element?.closest('.diagram-viewport');
      if (viewport && plainLink) {
        event.preventDefault();
        const figure=viewport.closest('.admitted-diagram');
        const claim=[...figure.querySelectorAll('.diagram-claim')].find(n=>n.getClientRects().length);
        const token=remember(diagramReturns,{element:id,trigger:diagramLink,y:scrollY,left:viewport.scrollLeft,top:viewport.scrollTop,
          claim:claim?.id || null,figureLeft:figure.scrollLeft,figureTop:figure.scrollTop,
          triggerTop:diagramLink.getBoundingClientRect().top});
        const next={...state(),diagramReturn:token};
        // Both the native Back destination and the new selected claim retain
        // this activation, even if a later activation uses the same element.
        history.replaceState({...state(),diagramBack:token},'');
        navigate(diagramLink.getAttribute('href'),next);
        const chosen=document.getElementById(diagramLink.getAttribute('href').slice(1));
        const evidence=chosen?.querySelector('a.evidence-link');
        if(evidence)openEvidence(evidence,diagramLink.matches('.diagram-route')?element:diagramLink,diagramReturns.get(token));
      }
    }
    const diagramReturn=event.target.closest('a.diagram-return');
    if (diagramReturn && plainLink) {
      event.preventDefault();
      navigate(diagramReturn.getAttribute('href'));
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
    if (outlineLink && plainLink) {
      const toc = outlineLink.closest('.document-nav');
      toc.open = false;
      const target = document.getElementById(outlineLink.hash.slice(1));
      // Run even for a repeated fragment, which emits no hashchange.
      requestAnimationFrame(() => {
        apply();
        if (target) { target.focus({preventScroll:true}); target.scrollIntoView({block:'start'}); }
      });
    }
    const range=event.target.closest('.point-others a');
    if(range&&plainLink){
      event.preventDefault();range.closest('.range-selector')?.removeAttribute('open');
      const claim=fragment().origin;const scope=claim&&document.getElementById(claim);
      const bound=scope&&[...scope.querySelectorAll('a.evidence-link')].find(a=>document.getElementById(a.hash.slice(1))?.dataset.selectedPoint===range.hash.slice(1)||a.getAttribute('href')===range.getAttribute('href'));
      navigate((bound?bound.getAttribute('href'):range.getAttribute('href'))+(bound?'!'+claim:''));
    }
    const codeReturn=event.target.closest('.source-point a.return');
    if (codeReturn && plainLink) {
      event.preventDefault();navigate(codeReturn.getAttribute('href'));
    }
    const link = event.target.closest('a.evidence-link');
    if (link && plainLink) {
      event.preventDefault();openEvidence(link);
    }
  });
  document.addEventListener('keydown', event => {
    const point = fragment().target?.closest('.source-point') || selected;
    if (!point) {
      const figure=event.target.closest('.diagram-expanded');
      if(event.key==='Tab'&&figure){const controls=[...figure.querySelectorAll('a[href],button:not(:disabled),summary,[tabindex="0"]')].filter(n=>n.getClientRects().length);const first=controls[0],last=controls.at(-1);if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus();}}
      if (event.key==='Escape' && figure) {event.preventDefault();closeExpansion(figure);return;}
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
  const evidenceBackdrop=document.createElement('button');evidenceBackdrop.className='evidence-backdrop';evidenceBackdrop.type='button';evidenceBackdrop.tabIndex=-1;evidenceBackdrop.hidden=true;evidenceBackdrop.setAttribute('aria-label',ko?'근거 닫기':'Close evidence');document.body.append(evidenceBackdrop);
  evidenceBackdrop.addEventListener('click',()=>selected?.querySelector('a.return')?.click());
  expireBodies();
  document.addEventListener('visibilitychange',()=>{if(!document.hidden)expireBodies();});
  apply();
})();
