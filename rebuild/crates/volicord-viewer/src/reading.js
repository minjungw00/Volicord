// Presentation and exact same-origin retained citation reads only. No writes.
(() => {
  const saved = new Map();
  const narrow = matchMedia('(max-width:900px)');
  let selected = null;
  let focusedHash = null;
  let pendingRead = null;
  const ko = document.documentElement.lang === 'ko';
  async function readCitation(point, offset = 0) {
    const control = point.querySelector('.citation-inspection');
    if (!control) return;
    pendingRead?.abort();
    const controller = new AbortController();
    pendingRead = controller;
    const status = control.querySelector('.citation-status');
    const button = control.querySelector('button');
    button.disabled = true;
    status.textContent = ko ? '현재 허용된 인용 바이트 확인 중…' : 'Checking current permitted cited bytes…';
    try {
      const url = new URL(control.dataset.citationUrl, location.href);
      if (url.origin !== location.origin || url.pathname !== '/citation') throw Error('scope');
      url.searchParams.set('offset', String(offset));
      const response = await fetch(url, {credentials:'same-origin', cache:'no-store', redirect:'error', signal:controller.signal});
      if (!response.ok) throw Error('unavailable');
      const raw = await response.text();
      if (new TextEncoder().encode(raw).length > 32768) throw Error('bound');
      const result = JSON.parse(raw), chunk = result.chunk;
      if (result.realization_fingerprint !== control.dataset.realization ||
          String(result.unit_index) !== control.dataset.unit || String(result.citation_index) !== control.dataset.citation ||
          result.claim !== point.querySelector('.claim-quote').textContent ||
          new TextEncoder().encode(chunk.text).length > 4096) throw Error('binding');
      if (controller.signal.aborted) return;
      point.querySelectorAll('pre.code-region,.initial-range-omission').forEach(n => n.hidden = true);
      let region = control.querySelector('pre');
      if (!region) {
        region = document.createElement('pre'); region.className = 'code-region';
        region.tabIndex = 0; region.setAttribute('role','region');
        const numbers = document.createElement('span'); numbers.className = 'source-line-numbers'; numbers.setAttribute('aria-hidden','true');
        region.append(numbers, document.createElement('code')); control.prepend(region);
      }
      region.hidden = false;
      region.querySelector('code').textContent = chunk.text;
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
      if (controller.signal.aborted) return;
      // Remove previously displayed bytes from this failed reading point.
      point.querySelectorAll('pre.code-region').forEach(n => n.remove());
      point.querySelectorAll('.initial-range-omission').forEach(n => n.hidden = true);
      status.textContent = ko ? '이 정확한 인용은 이용할 수 없거나 변경되었습니다. 설명으로 돌아가 현재 버전을 다시 여세요.'
        : 'This exact citation is unavailable or changed. Return to the explanation and reopen its current version.';
      button.dataset.nextOffset = 0;
      button.textContent = ko ? '허용된 인용 다시 확인' : 'Check permitted citation again';
    } finally {
      if (!controller.signal.aborted) button.disabled = false;
    }
  }
  function apply() {
    const target = document.getElementById(location.hash.slice(1));
    if (target) {
      if (target.matches('details')) target.open = true;
      for (let parent = target.parentElement; parent; parent = parent.parentElement) {
        if (parent.matches('details')) parent.open = true;
      }
    }
    const point = target?.closest('.source-point');
    const article = document.querySelector('.doc');
    const header = document.querySelector('.masthead');
    if (article) article.inert = !!point && narrow.matches;
    if (header) header.inert = !!point && narrow.matches;
    if (selected && selected !== point) {
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
      const origin = document.getElementById(point.dataset.origin);
      if (changed) readCitation(point);
      if (changed && !narrow.matches) origin?.scrollIntoView({block:'center'});
      if (!point.contains(document.activeElement)) point.focus({preventScroll:true});
    } else if (selected) {
      pendingRead?.abort();
      selected = null;
      focusedHash = null;
      if (target) {
        target.focus({preventScroll:true});
        const offset = saved.get(target.id);
        if (offset !== undefined) window.scrollTo(0, offset);
        else target.scrollIntoView({block:'center'});
      }
    }
  }
  document.addEventListener('click', event => {
    const button = event.target.closest('button.citation-read');
    if (button) readCitation(button.closest('.source-point'), Number(button.dataset.nextOffset || 0));
    const link = event.target.closest('a.evidence-link');
    if (link && !selected) saved.set(link.dataset.origin, scrollY);
  });
  document.addEventListener('keydown', event => {
    const point = document.getElementById(location.hash.slice(1))?.closest('.source-point');
    if (!point) {
      const menu = document.querySelector('.tools-menu[open]');
      if (menu && event.key === 'Escape') {
        event.preventDefault(); menu.open = false; menu.querySelector('summary').focus();
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
  narrow.addEventListener('change', apply);
  apply();
})();
