// Technical entry observation, never a substitute for the frozen comprehension
// questions or an isolated/human reading. Call only after the initial screenshot.
async function observeOverview(figure, diagram) {
  const nodes=(diagram.meaning.nodes||diagram.meaning.participants).map(n=>n.id);
  return figure.evaluate((figure,{nodes,sequence})=>{
    const errors=[];
    const initialInteractions=window.__overviewEntryInteractions||[];
    if(initialInteractions.length)errors.push('orientation was assisted before its initial observation');
    const visible=n=>{
      if(!n)return false;
      for(let p=n;p&&p!==figure.parentElement;p=p.parentElement){
        const s=getComputedStyle(p);
        if(s.display==='none'||s.visibility==='hidden'||Number(s.opacity)<.5)return false;
        if(p.tagName==='DETAILS'&&!p.open&&!p.querySelector(':scope > summary')?.contains(n))return false;
      }
      const b=n.getBoundingClientRect();return b.width>0&&b.height>0;
    };
    const svg=figure.querySelector('svg');const viewport=svg?.closest('.diagram-viewport');
    if(viewport&&(viewport.scrollLeft!==0||viewport.scrollTop!==0))errors.push('orientation was panned before its initial observation');
    if(!visible(svg))errors.push('connected map hidden or absent');
    if(viewport&&viewport.scrollWidth>viewport.clientWidth+1)errors.push('ordinary orientation requires horizontal discovery');
    if(viewport&&viewport.scrollHeight>viewport.clientHeight+1)errors.push('overview requires internal vertical discovery');
    for(const id of nodes){
      const anchor=document.getElementById(figure.id+'-'+id);
      if(!anchor||![...anchor.querySelectorAll('text')].some(visible))errors.push('node not in visible map: '+id);
    }
    const glyphs=[...svg?.querySelectorAll('text tspan')||[]].filter(visible);
    // getScreenCTM accounts for CSS/SVG transforms, including miniature maps.
    for(const g of glyphs){const m=g.getScreenCTM();const size=parseFloat(getComputedStyle(g).fontSize)*Math.hypot(m.a,m.b);if(size<(g.closest('.diagram-node-kind,.diagram-label-support')?11.9:g.closest('.relation-wording,.diagram-condition')?12.9:15.9))errors.push('essential text scaled below native reading size');}
    if(!glyphs.length)errors.push('no rendered labels');
    const controls=figure.querySelector('.diagram-controls');
    const claims=[...figure.querySelectorAll('.diagram-claim')].filter(visible);
    if(claims.length)errors.push('detail selected before orientation');
    const b=svg?.getBoundingClientRect();
    if(controls&&visible(controls)){const c=controls.getBoundingClientRect();if(c.top<b.top&&c.bottom>b.top+1)errors.push('figure controls obscure the map');}
    return {errors,initialInteractions,nodes:nodes.length,renderedGlyphRuns:glyphs.length,canvas:b?{width:b.width,height:b.height}:null,scroll:{pageY:scrollY,left:viewport?.scrollLeft,top:viewport?.scrollTop},classification:'technical supporting observation; comprehension assessment remains separate'};
  },{nodes,sequence:diagram.meaning.kind==='sequence'});
}
module.exports={observeOverview};

function entryObserver() {
    // Reinstalling observation must never erase an earlier disclosure/selection.
    if (window.__overviewEntryObserverInstalled) return;
    window.__overviewEntryObserverInstalled=true;
    window.__overviewEntryInteractions=[];
    document.addEventListener('click',event=>{
      if(event.target.closest('.admitted-diagram a,.admitted-diagram button,.admitted-diagram summary'))window.__overviewEntryInteractions.push('figure selection/control');
    },{capture:true});
    new MutationObserver(records=>{
      for(const record of records){const n=record.target;
        if((n.matches('details')&&record.attributeName==='open'&&n.closest('.admitted-diagram'))||
           (n.matches('.admitted-diagram')&&n.classList.contains('diagram-expanded')&&record.attributeName==='class'))window.__overviewEntryInteractions.push('figure disclosure/geometry changed');
      }
    }).observe(document,{subtree:true,attributes:true,attributeFilter:['open','class']});
}
async function installEntryObserver(page) {
  // Register before navigation so ordinary page scripts cannot perform the
  // missing orientation step before the test starts watching. evaluate also
  // supports the current document and native no-script diagnostic inspection.
  await page.addInitScript(entryObserver);
  await page.evaluate(entryObserver);
}
module.exports.installEntryObserver=installEntryObserver;
