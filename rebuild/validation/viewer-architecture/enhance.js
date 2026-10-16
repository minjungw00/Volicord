// Only reading interaction state; no network, canonical operation or HTML injection.
let origin = null;
document.querySelectorAll('.source').forEach((node, i) => node.hidden = i > 0);
document.addEventListener('click', event => {
  const link = event.target.closest('a[data-source]');
  if (link) {
    event.preventDefault();
    origin = {id: link.dataset.claim, y: scrollY};
    document.querySelectorAll('.source').forEach(node => node.hidden = node.id !== link.dataset.source);
    document.querySelectorAll('.selected-claim').forEach(node => node.textContent = document.getElementById(origin.id).querySelector('.copy').textContent);
    document.body.classList.add('code-focus');
    const target = document.getElementById(link.dataset.source);
    target.focus({preventScroll: true});
    if (innerWidth <= 800) target.scrollIntoView();
  }
  if (event.target.closest('.return') && origin) {
    event.preventDefault();
    document.body.classList.remove('code-focus');
    document.getElementById(origin.id).focus({preventScroll: true});
    scrollTo(0, origin.y);
  }
});
document.documentElement.dataset.ready = 'true';
