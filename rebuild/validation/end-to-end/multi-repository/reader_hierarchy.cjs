// Independent authored expectations; admitted prose is a binding/geometry oracle,
// never the expected design. Called on actual live Product output before controls.
const assert=require('node:assert/strict');
const {inspectDiagram}=require('./diagram_geometry.cjs');
async function inspect(page,expected) {
 const errors=[];
 const figure=page.locator(`.admitted-diagram[id$="-${expected.diagram}"]`);
 for(const [id,[primary,support]] of Object.entries(expected.nodes)) {
  const label=figure.locator(`a.diagram-element[id$="-${id}"] .diagram-node-label:visible`);
  if(await label.count()!==1){errors.push('node '+id);continue;}
  const spans=label.locator('tspan');
  if(await label.textContent()!==primary+'\u2028'+support)errors.push('wording '+id);
  const style=n=>({size:parseFloat(getComputedStyle(n).fontSize),y:n.getBoundingClientRect().y,visible:getComputedStyle(n).visibility});
  const first=await spans.first().evaluate(style),last=await spans.last().evaluate(style);
  if(first.size!==16||last.size!==12||last.y<=first.y||first.visible!=='visible'||last.visible!=='visible')errors.push('hierarchy '+id);
 }
 for(const [id,word] of Object.entries(expected.relations)) {
  const label=figure.locator(`a.diagram-element[id$="-${id}"] .relation-wording>text:visible`).first();
  if(await label.textContent()!==word)errors.push('relation '+id);
 }
 const headings=page.locator('.article-section>.unit h2');
 if(expected.sections && JSON.stringify(await headings.allTextContents())!==JSON.stringify(expected.sections))errors.push('section meaning');
 if(!await headings.count())errors.push('section heading');
 else {
  const h=await headings.first().evaluate(n=>({size:parseFloat(getComputedStyle(n).fontSize),weight:getComputedStyle(n).fontWeight,number:getComputedStyle(n.closest('.unit'),'::before').content,figure:n.closest('.article-section').querySelector('figure')?.getBoundingClientRect().top,top:n.getBoundingClientRect().top}));
  if(h.size<20||Number(h.weight)<600||h.figure<=h.top||h.number==='none')errors.push('section hierarchy');
 }
 return errors;
}
async function verify(page,expected,publication,negative=false) {
 assert.ok(Array.isArray(expected.sections)&&expected.sections.length,'independently declared section names required');
 assert.deepEqual(await inspect(page,expected),[],'authored names/identifiers/relations and section hierarchy');
 const figure=page.locator(`.admitted-diagram[id$="-${expected.diagram}"]`),d=publication.diagrams.find(d=>d.id===expected.diagram);
 assert.deepEqual((await inspectDiagram(figure.locator('svg'),d,publication.prose)).errors,[]);
 if(!negative)return {hierarchy:true};
 const name=figure.locator('.diagram-node-label:visible').first(),saved=await name.innerHTML();
 await name.locator('tspan').first().evaluate(n=>n.style.fontSize='12px');
 assert.ok((await inspect(page,expected)).some(e=>e.startsWith('hierarchy')),'miniature primary fails');
 await name.evaluate((n,s)=>n.innerHTML=s,saved);
 await name.locator('tspan').last().evaluate(n=>n.style.fontSize='16px');
 assert.ok((await inspect(page,expected)).some(e=>e.startsWith('hierarchy')),'support cannot replace primary');
 await name.evaluate((n,s)=>n.innerHTML=s,saved);
 const id=Object.keys(expected.relations)[0],label=figure.locator(`a.diagram-element[id$="-${id}"] .relation-wording>text:visible`).first(),html=await label.innerHTML();
 await label.evaluate(n=>n.textContent='묶습니다');
 assert.ok((await inspect(page,expected)).includes('relation '+id),'ambiguous generic relation fails');
 await label.evaluate((n,s)=>n.innerHTML=s,html);
 const route=figure.locator(`a.diagram-route[data-element$="-${id}"] .map-wide .relation-line`),path=await route.getAttribute('d');
 await route.evaluate(n=>n.setAttribute('d','M0 0H2'));
 assert.ok((await inspectDiagram(figure.locator('svg'),d,publication.prose)).errors.some(e=>e.kind==='wrong-edge-direction'),'foreign/reversed endpoints fail');
 await route.evaluate((n,s)=>n.setAttribute('d',s),path);
 const heading=page.locator('.article-section>.unit h2').first(),tag=await heading.evaluate(n=>{const p=document.createElement('p');p.innerHTML=n.innerHTML;n.replaceWith(p);return p.outerHTML;});
 assert.ok((await inspect(page,expected)).includes('section meaning'),'heading/type substitution fails');
 // This is the end of disposable controls. Caller reloads the public route.
 return {miniaturePrimaryRejected:true,promotedSupportRejected:true,ambiguousRelationRejected:true,wrongEndpointRejected:true,headingSubstitutionRejected:!!tag};
}
module.exports={inspect,verify};
