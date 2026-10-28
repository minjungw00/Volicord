// Owns original-code DOM/buffers, bounded requests, expiry and cancellation.
function createCitationReader({lifetime, isSelected, selectedSource, withholdPublication, updateComparison}) {
  let pendingRead = null;
  const reads = new WeakMap(), codeBuffers = new WeakMap();
  // Weak keys preserve native visibility without retaining detached denied code.
  const nativeVisibility = new WeakMap([...document.querySelectorAll('pre.code-region,.code-caption,.initial-range-omission,.source-code-reference')].map(node=>[node,node.hidden]));
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
  function purgeBody(source,reason='expired') {
    if(selectedSource()===source)pendingRead?.abort();
    for(const point of document.querySelectorAll('.source-point')){
      if(point.dataset.sourceId!==source)continue;
      codeBuffers.delete(point);
      point.querySelectorAll('pre.code-region,.code-caption,.initial-range-omission,.source-code-reference').forEach(n=>n.remove());
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
  async function readCitation(point, offset = 0) {
    if (lifetime.disposed || !isSelected(point) || point.dataset.unavailable === 'true') return;
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
      if (controller.signal.aborted || reads.get(control) !== controller || !isSelected(point) || !point.isConnected) return;
      updateComparison(control.dataset.publicationId,result.applicability);
      updateAvailability(control.dataset.publicationId,result.applicability);
      const originalDeadline=point.querySelector('.source-deadline');
      if(point.dataset.rangeState==='expired'||(originalDeadline&&Number(originalDeadline.dataset.expiresAt)/1000<=Date.now())){
        const error=Error('body expired during read');error.reason='expired';error.publicationReadable=true;throw error;
      }
      point.querySelectorAll('pre.code-region,.code-caption,.initial-range-omission').forEach(n => n.hidden = true);
      let region = control.querySelector('pre');
      if (!region) {
        region = document.createElement('pre'); region.className = 'code-region'; region.dataset.fetchedCode='true';
        region.tabIndex = 0; region.setAttribute('role','region');
        const numbers = document.createElement('span'); numbers.className = 'source-line-numbers'; numbers.setAttribute('aria-hidden','true');
        const code = document.createElement('code'); code.lang = '';
        region.append(numbers, code);
        const caption = document.createElement('p'); caption.className = 'code-caption'; caption.dataset.fetchedCode='true';
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
      if (controller.signal.aborted || reads.get(control) !== controller || !isSelected(point) || !point.isConnected) return;
      // Remove previously displayed bytes from this failed reading point.
      // A deleted shared Source must not leave an earlier claim's cached code.
      document.querySelectorAll('.source-point').forEach(other => {
        if (other.dataset.sourceId === point.dataset.sourceId) {
          codeBuffers.delete(other);
          other.querySelectorAll('pre.code-region,.code-caption,.source-code-reference').forEach(n => n.remove());
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

  function forgetPublication(publication) {
    pendingRead?.abort();
    for (const point of document.querySelectorAll('.source-point')) {
      if (point.dataset.readerPublication === publication) codeBuffers.delete(point);
    }
  }
  const cancel = () => { pendingRead?.abort(); pendingRead=null; };
  lifetime.cleanup(() => {
    cancel(); clearTimeout(expiryTimer);
    document.querySelectorAll('[data-fetched-code]').forEach(node=>node.remove());
    for(const node of document.querySelectorAll('pre.code-region,.code-caption,.initial-range-omission,.source-code-reference')){
      if(nativeVisibility.has(node))node.hidden=nativeVisibility.get(node);
    }
  });
  lifetime.on(document,'visibilitychange',()=>{if(!document.hidden)expireBodies();});
  expireBodies();
  return {read:readCitation, cancel, forgetPublication, expire:expireBodies};
}
