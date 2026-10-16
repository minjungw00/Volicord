// Local presentation only. No requests, storage, generation or runtime mutations.
(() => {
  const saved = new Map();
  let selected = null;
  let focusedHash = null;
  const narrow = matchMedia('(max-width:900px)');
  function apply() {
    const target = document.getElementById(location.hash.slice(1));
    if (target) {
      if (target.matches('details')) target.open = true;
      for (let parent = target.parentElement; parent; parent = parent.parentElement) {
        if (parent.matches('details')) parent.open = true;
      }
      if (target.matches('.original-record')) {
        const goal = target.querySelector('details.work-goal');
        if (goal) goal.open = true;
      }
    }
    const panel = target && (target.matches('.source-excerpt') ? target :
      target.matches('.source-reading-point') ? target.closest('.source-excerpt') : null);
    if (panel) {
      const changed = focusedHash !== location.hash;
      focusedHash = location.hash;
      selected = panel;
      const point = target.matches('.source-reading-point') ? target : panel;
      const link = point.querySelector('.return-to-claim,.source-panel-return');
      const origin = link && document.getElementById(link.hash.slice(1));
      if (changed && !narrow.matches && origin) origin.scrollIntoView({block:'center'});
      if (!panel.contains(document.activeElement)) point.focus({preventScroll:true});
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
    const link = event.target.closest('a.source-citation');
    if (!link) return;
    const point = document.getElementById(link.hash.slice(1));
    const back = point && point.querySelector('.return-to-claim');
    if (back && !selected) saved.set(back.hash.slice(1), scrollY);
  });
  document.addEventListener('keydown', event => {
    // Native fragment focus can arrive before the queued hashchange event.
    const active = document.getElementById(location.hash.slice(1));
    selected = active && active.closest('.source-excerpt');
    if (!selected) return;
    const point = selected.querySelector('.source-reading-point:target') || selected;
    const back = point.querySelector('.return-to-claim,.source-panel-return');
    if (event.key === 'Escape' && back) {
      event.preventDefault(); back.click();
      document.getElementById(back.hash.slice(1))?.focus({preventScroll:true});
    } else if (event.key === 'Tab' && narrow.matches) {
      const controls = [...selected.querySelectorAll('a[href],summary,[tabindex="0"]')]
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
