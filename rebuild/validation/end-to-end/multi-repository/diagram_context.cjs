// Reading-position checks follow ordinary entry and precede selection/Source.
// Expected identities/scope come from admitted meaning, never SVG coordinates.
async function inspectReadingContext(page, figure, diagram, prose, screenshot) {
  const label=l=>l.text;
  const ko=await page.evaluate(()=>document.documentElement.lang==='ko');
  const m=diagram.meaning,nodes=m.nodes||m.participants,items=[];
  const name=id=>{const n=nodes.find(n=>n.id===id),text=label(n.label);return nodes.filter(n=>label(n.label)===text).length>1?`${text} (${label(n.identifier)})`:text;};
  const stages=await figure.locator(':scope > .diagram-full-view svg').getAttribute('data-sequence-layout')==='stages';
  for(const [gi,g] of (m.groups||[]).entries()) for(const r of g.messages) {
    const order=g.order==='sequential'?'strict':g.order==='parallel'?(ko?'par · 상대 순서 없음':'par · no relative order'):(ko?'strict · 순차':'strict · sequential');
    items.push({id:r.id,label:label(r.label),expected:[...(stages?[name(r.from),name(r.to)]:[`${name(r.from)} → ${name(r.to)}`]),order+(g.condition?` · opt · ${ko?'조건:':'condition:'} ${label(g.condition)}`:'')],group:gi+1});
  }
  const owner=m.owner && `${ko?'상태 소유자:':'State owner:'} ${label(m.owner)}`;
  if(owner)for(const n of nodes)items.push({id:n.id,label:label(n.label),expected:[owner]});
  for(const r of m.relations||[])items.push({id:r.id,label:label(r.label),expected:[`${name(r.from)} → ${name(r.to)}`,...(owner?[owner]:[])],guard:r.condition&&{id:r.id+':condition',text:label(r.condition)}});
  const selected=[...new Set([0,Math.floor(items.length/2),items.length-1])].map(i=>items[i]);
  const ns=await figure.getAttribute('id'),observations=[],errors=[];
  for(const direction of ['forward','reverse','native'])for(const item of direction==='reverse'?[...selected].reverse():selected) {
    const element=figure.locator('[id="'+ns+'-'+item.id+'"]');
    const offset=await element.evaluate((n,label)=>{
      const t=[...n.querySelectorAll('text')].find(t=>t.getBoundingClientRect().height>0&&t.textContent===label);
      if(!t)throw Error('complete reading label unavailable');
      const occlusion=Math.max(0,...[...document.querySelectorAll('.masthead,.document-nav')].map(n=>{const r=n.getBoundingClientRect();return r.top<innerHeight/2&&r.bottom>0?r.bottom:0;}));
      const first=Math.min(...[...n.querySelectorAll('text')].map(t=>t.getBoundingClientRect()).filter(r=>r.height>0).map(r=>r.top));
      return first-occlusion-24;
    },item.label);
    if(direction==='native')await page.evaluate(id=>{location.hash=id;},ns+'-'+item.id);
    else {await page.mouse.move(8,page.viewportSize().height-20);await page.mouse.wheel(0,offset);}
    await page.waitForTimeout(80);
    const result=await element.evaluate((n,item)=>{
      const obstacles=[...document.querySelectorAll('.masthead,.document-nav')].map(n=>n.getBoundingClientRect()).filter(r=>r.top<innerHeight/2&&r.bottom>0);
      const clear=t=>{const r=t.getBoundingClientRect();return r.width>0&&r.top>=0&&r.bottom<=innerHeight&&!obstacles.some(o=>o.left<r.right&&o.right>r.left&&o.top<r.bottom&&o.bottom>r.top);};
      const texts=[...n.querySelectorAll('text')].filter(t=>t.getBoundingClientRect().height>0);
      const missing=item.expected.filter(s=>!texts.some(t=>(t.textContent===s||t.textContent.endsWith(' · '+s))&&clear(t)));
      const wording=texts.find(t=>t.textContent===item.label);
      if(!wording||!clear(wording))missing.push(item.label);
      if(item.guard){const a=document.getElementById(n.closest('figure').id+'-'+item.guard.id);if(![...a?.querySelectorAll('text')||[]].some(t=>t.textContent===item.guard.text&&clear(t)))missing.push(item.guard.text);}
      return {element:n.id,pageY:scrollY,focus:document.activeElement.id,texts:texts.map(t=>({text:t.textContent,top:t.getBoundingClientRect().top,bottom:t.getBoundingClientRect().bottom})),missing};
    },item);
    observations.push({direction,...result});
    errors.push(...result.missing.map(text=>({kind:'missing-reading-context',element:item.id,direction,text})));
    if(screenshot)await screenshot(direction,item.id);
  }
  return {classification:'dependent normal wheel-scroll technical observation, not human comprehension',observations,errors};
}
module.exports={inspectReadingContext};

// Exercise a visible routed stroke through the same existing return records.
async function inspectRouteAlias(page, figure, element, claim, action, point) {
  const id=await element.getAttribute('id');
  const route=figure.locator('a.diagram-route[data-element="'+id+'"]');
  if(!await route.count())return {applicable:false};
  if(await route.getAttribute('href')!==await element.getAttribute('href'))throw Error('route alias changed claim');
  const hit=await route.evaluate(n=>{
    const viewport=n.closest('.diagram-viewport').getBoundingClientRect();
    for(const p of n.querySelectorAll('.relation-line'))if(p.getBoundingClientRect().height>0) {
      const length=p.getTotalLength(),matrix=p.getScreenCTM();
      for(let i=1;i<500;i++) {
        const q=p.getPointAtLength(length*i/500),at=new DOMPoint(q.x,q.y).matrixTransform(matrix);
        if(at.x>viewport.left&&at.x<viewport.right&&at.y>Math.max(160,viewport.top)&&at.y<Math.min(innerHeight,viewport.bottom)&&document.elementFromPoint(at.x,at.y)?.closest('a')===n)return {x:at.x,y:at.y};
      }
    }
    throw Error('no visible pointer route');
  });
  const before=await element.evaluate(n=>({top:n.getBoundingClientRect().top,pageY:scrollY,left:n.closest('.diagram-viewport').scrollLeft,panY:n.closest('.diagram-viewport').scrollTop}));
  await page.mouse.click(hit.x,hit.y);await action.click();
  await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-inspection pre.code-region code')?.textContent.length>0,await point.getAttribute('id'));
  await point.locator('a.return:visible').click();await claim.locator('a.diagram-return').click();
  await page.waitForFunction(id=>document.activeElement?.matches('a.diagram-route')&&document.activeElement.dataset.element===id,id);
  const after=await element.evaluate(n=>({top:n.getBoundingClientRect().top,pageY:scrollY,left:n.closest('.diagram-viewport').scrollLeft,panY:n.closest('.diagram-viewport').scrollTop}));
  for(const k of Object.keys(before))if(Math.abs(before[k]-after[k])>5)throw Error('route alias lost reading context: '+k);
  return {applicable:true,hit,before,after};
}
module.exports.inspectRouteAlias=inspectRouteAlias;
