// Presentation only: no requests, persistence, generation or Runtime mutations.
(() => {
  const saved = new Map();
  const narrow = matchMedia('(max-width:900px)');
  let selected = null;
  let focusedHash = null;
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
      if (changed && !narrow.matches) origin?.scrollIntoView({block:'center'});
      if (!point.contains(document.activeElement)) point.focus({preventScroll:true});
    } else if (selected) {
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
    const link = event.target.closest('a.evidence-link');
    if (link && !selected) saved.set(link.dataset.origin, scrollY);
  });
  document.addEventListener('keydown', event => {
    const point = document.getElementById(location.hash.slice(1))?.closest('.source-point');
    if (!point) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      point.querySelector('.return').click();
      apply();
    } else if (event.key === 'Tab' && narrow.matches) {
      const controls = [...point.querySelectorAll('a[href],summary,[tabindex="0"]')]
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
