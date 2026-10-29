// Independent authored expectations; admitted prose is a binding/geometry oracle,
// never the expected design. Called on actual live Product output before controls.
const assert=require('node:assert/strict');
const {inspectDiagram}=require('./diagram_geometry.cjs');
async function citationHierarchy(page) {
 const actions=await page.locator('.unit .evidence-choice .evidence-link').evaluateAll(nodes=>nodes.map(n=>({
  visible:n.textContent.trim(),purpose:n.getAttribute('aria-label'),
  repeatedPath:!!n.parentElement.querySelector('.source-label')
 })));
 return actions.flatMap(a=>[
  ...(/^(?:Source|근거) \d+ ↗$/.test(a.visible)?[]:['repeated article claim']),
  ...(/^(?:Inspect|확인:) .+ · .+:\d+–\d+$/.test(a.purpose||'')?[]:['missing accessible claim/range']),
  ...(a.repeatedPath?['repeated article path']:[])
 ]);
}
async function inspect(page,expected) {
 const errors=await citationHierarchy(page);
 const figure=page.locator(`.admitted-diagram[id$="-${expected.diagram}"]`);
 for(const [id,[primary,support]] of Object.entries(expected.nodes)) {
  const label=figure.locator(`a.diagram-element[id$="-${id}"] .diagram-node-label:visible`);
  if(await label.count()!==1){errors.push('node '+id);continue;}
  const spans=label.locator('tspan');
  const identifier=figure.locator(`a.diagram-element[id$="-${id}:identifier"] .diagram-label-support text:visible`);
  if(await label.textContent()!==primary || await identifier.count()!==1 || await identifier.textContent()!==support)errors.push('wording '+id);
  const style=n=>({size:parseFloat(getComputedStyle(n).fontSize),y:n.getBoundingClientRect().y,visible:getComputedStyle(n).visibility});
  const first=await spans.first().evaluate(style),last=await identifier.evaluate(style);
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
 assert.deepEqual((await inspectDiagram(figure.locator(':scope > .diagram-full-view svg'),d,publication.prose)).errors,[]);
 if(!negative)return {hierarchy:true};
 const action=page.locator('.unit .evidence-choice .evidence-link').first();
 assert.ok(await action.count(),'real article citation action required');
 const originalAction=await action.textContent();
 await action.evaluate(n=>n.textContent=n.getAttribute('aria-label'));
 assert.ok((await citationHierarchy(page)).includes('repeated article claim'),'a full claim repeated before the map fails');
 await action.evaluate((n,s)=>n.textContent=s,originalAction);
 const originalPurpose=await action.getAttribute('aria-label');
 await action.evaluate(n=>n.removeAttribute('aria-label'));
 assert.ok((await citationHierarchy(page)).includes('missing accessible claim/range'),'compact wording cannot remove exact accessible purpose');
 await action.evaluate((n,s)=>n.setAttribute('aria-label',s),originalPurpose);
 const name=figure.locator('.diagram-node-label:visible').first(),saved=await name.innerHTML();
 await name.locator('tspan').first().evaluate(n=>n.style.fontSize='12px');
 assert.ok((await inspect(page,expected)).some(e=>e.startsWith('hierarchy')),'miniature primary fails');
 await name.evaluate((n,s)=>n.innerHTML=s,saved);
 const support=figure.locator('.diagram-label-support text:visible').first(),supportStyle=await support.getAttribute('style');
 await support.evaluate(n=>n.style.fontSize='16px');
 assert.ok((await inspect(page,expected)).some(e=>e.startsWith('hierarchy')),'support cannot replace primary');
 await support.evaluate((n,s)=>s===null?n.removeAttribute('style'):n.setAttribute('style',s),supportStyle);
 const id=Object.keys(expected.relations)[0],label=figure.locator(`a.diagram-element[id$="-${id}"] .relation-wording>text:visible`).first(),html=await label.innerHTML();
 await label.evaluate(n=>n.textContent='묶습니다');
 assert.ok((await inspect(page,expected)).includes('relation '+id),'ambiguous generic relation fails');
 await label.evaluate((n,s)=>n.innerHTML=s,html);
 const route=figure.locator(`a.diagram-route[data-element$="-${id}"] .map-wide .relation-line`),path=await route.getAttribute('d');
 await route.evaluate(n=>n.setAttribute('d','M0 0H2'));
 assert.ok((await inspectDiagram(figure.locator(':scope > .diagram-full-view svg'),d,publication.prose)).errors.some(e=>e.kind==='wrong-edge-direction'),'foreign/reversed endpoints fail');
 await route.evaluate((n,s)=>n.setAttribute('d',s),path);
 const heading=page.locator('.article-section>.unit h2').first(),tag=await heading.evaluate(n=>{const p=document.createElement('p');p.innerHTML=n.innerHTML;n.replaceWith(p);return p.outerHTML;});
 assert.ok((await inspect(page,expected)).includes('section meaning'),'heading/type substitution fails');
 // This is the end of disposable controls. Caller reloads the public route.
 return {repeatedClaimRejected:true,missingAccessiblePurposeRejected:true,miniaturePrimaryRejected:true,promotedSupportRejected:true,ambiguousRelationRejected:true,wrongEndpointRejected:true,headingSubstitutionRejected:!!tag};
}
module.exports={inspect,verify,citationHierarchy};

// Necessary source-derived mechanisms are independent of candidate prose and
// seals. This intentionally does not purport to judge natural-language entailment.
function explanatoryObligations(texts, expected) {
 const errors=[];
 for(const words of expected.mechanisms)if(!texts.some(t=>words.every(w=>t.includes(w))))errors.push('missing explanatory mechanism: '+words.join(' / '));
 for(const padding of expected.forbidden)if(texts.some(t=>t.includes(padding)))errors.push('label-dictionary padding: '+padding);
 return errors;
}
function corpusObligations(cases, contract) {
 assert.deepEqual(cases.map(c=>c.key).sort(),Object.keys(contract.cases).sort(),'independent complete corpus membership');
 for(const c of cases){
  const expected=contract.cases[c.key],p=JSON.parse(require('node:fs').readFileSync(c.public));
  assert.equal(c.language,expected.language);
  const membership=p.diagrams.map(d=>[d.meaning.kind,d.id]).sort();
  assert.deepEqual(membership,Object.entries(expected.figures).flatMap(([kind,ids])=>ids.map(id=>[kind,id])).sort(),'independent figure/language membership');
  assert.deepEqual(explanatoryObligations(p.prose.map(p=>p.text),expected),[],'source-derived necessary prose; bindings alone do not establish explanatory content');
 }
}
module.exports.explanatoryObligations=explanatoryObligations;
module.exports.corpusObligations=corpusObligations;
