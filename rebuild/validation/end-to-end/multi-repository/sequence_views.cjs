// Public sequence scene/timeline -> exact claim/code -> originating view.
// Run only after preserving ordinary article and connected-scene entry.
const assert=require('node:assert/strict');
const {inspectDiagram}=require('./diagram_geometry.cjs');
const {citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('../../viewer-design/product_review_controls.cjs');
async function inspect(page,figure,diagram,answers,{scripted,write,screenshot,settled}) {
 const details=figure.locator(':scope > .diagram-timeline');
 assert.equal(await details.count(),1,'complete native timeline disclosure');
 const m=diagram.meaning,units=new Set([
  ...m.participants.flatMap(n=>[n.label.unit,...(n.identifier?[n.identifier.unit]:[])]),
  ...m.groups.flatMap(g=>[...(g.condition?[g.condition.unit]:[]),...g.messages.map(m=>m.label.unit)])
 ]);units.delete(diagram.caption_unit);
 const claimChoices=figure.locator('.diagram-caption-claims ul a');
 assert.equal(await claimChoices.count(),units.size,'scene evidence exposes the actual underlying claim set');
 for(const unit of units){
  const link=figure.locator(`.diagram-caption-claims ul a[href$="-p${unit+1}"]`);
  assert.equal(await link.count(),1);assert((await link.textContent()).endsWith(answers.prose[unit].text),'chooser retains the complete original claim rather than guessing an aggregate citation');
 }
 const evidence=figure.locator('.diagram-caption-claims');
 await evidence.locator(':scope > summary').click();await settled();
 assert.equal(await evidence.getAttribute('open'),'');
 assert(await evidence.locator('a').first().isVisible(),'actual scene evidence is disclosed natively');
 assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'complete claim choices wrap at the actual reading width');
 await screenshot('scene-claims');
 await evidence.locator(':scope > summary').click();await settled();
 const svg=details.locator('svg'),ns=(await figure.getAttribute('id'))+':timeline';
 if(scripted){
  const expand=figure.locator('[data-diagram-action=expand]');await expand.click();
  await figure.locator('[data-diagram-action=in]').click();
  await details.locator(':scope > summary').click();await settled();
  assert.equal(await svg.locator('.diagram-content').getAttribute('transform'),'scale(1.25)','a timeline disclosed after zoom inherits the actual content scale');
  await figure.locator('[data-diagram-action=reset]').click();await page.keyboard.press('Escape');await settled();
  assert.equal(await expand.getAttribute('aria-expanded'),'false');
  await details.locator(':scope > summary').click();await settled();
 }
 await details.locator(':scope > summary').click();await settled();
 assert.equal(await svg.getAttribute('data-origin-namespace'),ns);
 const geometry=await inspectDiagram(svg,diagram,answers.prose);write('timeline-geometry',geometry);
 assert.deepEqual(geometry.errors,[],'same admitted complete timeline');
 const messages=diagram.meaning.groups.flatMap(g=>g.messages),message=messages.at(-1);
 const element=svg.locator(`[id="${ns}-${message.id}"]`),claimId=(await element.getAttribute('href')).slice(1);
 const claim=page.locator(`[id="${claimId}"]`),unit=answers.prose[message.label.unit];
 const citationIndex=unit.citations.findIndex(q=>q.claim_start_byte<=message.label.start_byte&&q.claim_end_byte>=message.label.end_byte);
 assert(citationIndex>=0);const citation=unit.citations[citationIndex];
 const action=claim.locator(`a.evidence-link[href$="-c${citationIndex+1}"]`);
 const href=await action.getAttribute('href'),point=await citationPoint(page,href);
 await element.focus();await settled();
 const before=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left,pan:n.closest('.diagram-viewport').scrollLeft}));
 await screenshot('timeline-before');await page.keyboard.press('Enter');await settled();
 if(!scripted){await action.click();await settled();}
 let bytes;
 if(scripted){
  await page.waitForFunction(({id,size})=>document.getElementById(id)?.querySelector('.citation-inspection code')?.textContent.length>0,{id:await point.getAttribute('id'),size:citation.end_byte-citation.start_byte});
  bytes=await point.locator('.citation-inspection code').textContent();
 } else {bytes=await (await openEmbeddedCode(point)).textContent();await returnFromEmbeddedCode(point);}
 assert.equal('sha256:'+require('node:crypto').createHash('sha256').update(bytes).digest('hex'),citation.range_sha256,'complete original code, exact underlying citation');
 if(scripted){
  const width=page.viewportSize().width,pointId=await point.getAttribute('id');
  for(const next of [width<1200?1440:390,width]){
   await page.setViewportSize({width:next,height:900});await settled();
   assert.equal(await page.locator('.source-point[data-selected]').getAttribute('id'),pointId,'resize preserves exact timeline evidence');
  }
 }
 await screenshot('timeline-code');
 const deep=page.url();
 if(scripted)await page.keyboard.press('Escape');else await point.locator('a.return:visible').click();
 await settled();assert.equal(await details.getAttribute('open'),'','originating timeline stays open');
 if(scripted){assert(await element.evaluate(n=>document.activeElement===n),'exact timeline focus return');}
 else {assert.equal(new URL(page.url()).hash,'#'+ns+'-'+message.id);}
 const after=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left,pan:n.closest('.diagram-viewport').scrollLeft}));
 if(scripted)for(const key of ['top','left','pan'])assert(Math.abs(before[key]-after[key])<3,'timeline visible context return: '+key);
 await screenshot('timeline-returned');
 // Fresh native/deep evidence entry has no in-memory return token. The view
 // identity must come from the publication-bound origin, never equal wording.
 await page.goto(deep);await settled();
 if(scripted){
  await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-inspection code')?.textContent.length>0,await point.getAttribute('id'));
  await page.keyboard.press('Escape');await settled();
  assert(await element.evaluate(n=>n===document.activeElement),'deep evidence returns to its timeline origin');
  assert.equal(await details.getAttribute('open'),'');
 } else {
  await point.locator('a.return:visible').click();await settled();
  assert.equal(new URL(page.url()).hash,'#'+ns+'-'+message.id);assert.equal(await details.getAttribute('open'),'');
 }
 const duplicateIds=await page.evaluate(()=>{const ids=[...document.querySelectorAll('[id]')].map(n=>n.id);return ids.filter((id,i)=>ids.indexOf(id)!==i);});
 assert.deepEqual(duplicateIds,[],'overview/detail/native evidence never duplicate targets');

 return {origin:ns+'-'+message.id,citation:citationIndex,source:citation.source_id,codeBytes:Buffer.byteLength(bytes),before,after};
}
module.exports={inspect};
