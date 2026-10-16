// Bounded progressive enhancement. All content and native links exist without it.
(() => {
  const points = [...document.querySelectorAll('.source-point')];
  if (!points.length) return;
  document.body.classList.add('enhanced');
  const saved = new Map();
  const appendix = document.querySelector('.appendix');
  const background = [...document.querySelectorAll('.doc,.masthead,footer,.skip')];
  const narrow = matchMedia('(max-width: 900px)');
  let selected = null;
  function apply() {
    const id = location.hash.slice(1);
    const point = points.find(p => p.id === id);
    // Native fragments do not open a closed details element consistently.
    const target = document.getElementById(id);
    if (target) {
      if (target.matches('details')) target.open = true;
      for (let parent = target.parentElement; parent; parent = parent.parentElement) {
        if (parent.matches('details')) parent.open = true;
      }
    }
    if (point) {
      const changed = selected !== point;
      selected = point;
      document.body.classList.add('code-open');
      points.forEach(p => p.classList.toggle('active', p === point));
      background.forEach(n => n.inert = narrow.matches);
      if (narrow.matches) {
        appendix.setAttribute('role', 'dialog');
        appendix.setAttribute('aria-modal', 'true');
      } else {
        appendix.removeAttribute('role');
        appendix.removeAttribute('aria-modal');
        document.getElementById(point.dataset.origin)?.scrollIntoView({block:'center'});
      }
      if (changed) appendix.scrollTop = 0;
      point.focus({preventScroll:true});
    } else {
      document.body.classList.remove('code-open');
      background.forEach(n => n.inert = false);
      appendix.removeAttribute('role');
      appendix.removeAttribute('aria-modal');
      points.forEach(p => p.classList.remove('active'));
      if (selected) {
        const origin = document.getElementById(id);
        const y = saved.get(id);
        if (origin) origin.focus({preventScroll:true});
        if (y !== undefined) {
          window.scrollTo(0, y);
          requestAnimationFrame(() => {
            if (location.hash === '#' + id && !document.body.classList.contains('code-open')) window.scrollTo(0, y);
          });
        }
        else if (origin) origin.scrollIntoView({block:'center'});
        selected = null;
      }
    }
  }
  document.addEventListener('click', event => {
    const link = event.target.closest('a.evidence-link');
    if (!link) return;
    const origin = link.dataset.origin;
    if (!selected) saved.set(origin, scrollY);
    else if (origin !== selected.dataset.origin) saved.delete(origin);
    history.replaceState(null, '', '#' + origin);
  });
  document.addEventListener('keydown', event => {
    if (event.key === 'Tab' && selected && narrow.matches) {
      const controls = [...selected.querySelectorAll('a[href],summary,[tabindex="0"]')]
        .filter(n => n.getClientRects().length);
      const first = controls[0], last = controls.at(-1);
      if (event.shiftKey && (document.activeElement === first || document.activeElement === selected)) {
        event.preventDefault(); last?.focus();
      } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === selected)) {
        event.preventDefault(); first?.focus();
      }
    }
    if (event.key === 'Escape' && selected) {
      event.preventDefault();
      location.hash = selected.dataset.origin;
    }
  });
  window.addEventListener('hashchange', apply);
  narrow.addEventListener('change', apply);
  apply();
})();
