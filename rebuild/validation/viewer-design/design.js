// Bounded progressive enhancement. All content and native links exist without it.
(() => {
  const points = [...document.querySelectorAll('.source-point')];
  if (!points.length) return;
  document.body.classList.add('enhanced');
  const saved = new Map();
  let selected = null;
  function apply() {
    const id = location.hash.slice(1);
    const point = points.find(p => p.id === id);
    if (point) {
      selected = point;
      document.body.classList.add('code-open');
      points.forEach(p => p.classList.toggle('active', p === point));
      point.closest('.appendix').scrollTop = 0;
      point.focus({preventScroll:true});
    } else {
      document.body.classList.remove('code-open');
      points.forEach(p => p.classList.remove('active'));
      if (selected) {
        const origin = document.getElementById(id);
        const y = saved.get(id);
        if (origin) origin.focus({preventScroll:true});
        if (y !== undefined) {
          window.scrollTo(0, y);
          requestAnimationFrame(() => window.scrollTo(0, y));
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
    saved.set(origin, scrollY);
    history.replaceState(null, '', '#' + origin);
  });
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && selected) {
      event.preventDefault();
      location.hash = selected.dataset.origin;
    }
  });
  window.addEventListener('hashchange', apply);
  apply();
})();
