// Browser-side inspection shared by native HTML and SVG readers. SVG anchors
// expose href as SVGAnimatedString and do not have HTMLAnchorElement.hash.
function fragmentErrors() {
  const counts=new Map();
  for(const n of document.querySelectorAll('[id]')) counts.set(n.id,(counts.get(n.id)||0)+1);
  const errors=[...counts].filter(([,n])=>n!==1).map(([id,n])=>`${id}: ${n} targets`);
  for(const a of document.querySelectorAll('a[href^="#"]')) {
    const href=a.getAttribute('href');
    if(counts.get(href.slice(1))!==1) errors.push(`bad link ${href}`);
  }
  for(const n of document.querySelectorAll('[data-origin]'))
    if(counts.get(n.dataset.origin)!==1) errors.push(`bad return ${n.dataset.origin}`);
  return errors;
}
module.exports={fragmentErrors};
