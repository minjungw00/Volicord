// Single admitted temporal world -> exact claim/code -> originating camera/view.
// Frozen comparison scenes are historical evidence, never extra Product trees.
const assert=require('node:assert/strict');
const {inspectDiagram}=require('./diagram_geometry.cjs');
const {citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('../../viewer-design/product_review_controls.cjs');
async function inspect(page,figure,diagram,answers,{scripted,write,screenshot,settled}) {
 assert.equal(await figure.locator('svg').count(),1,'one complete temporal scene');
 assert.equal(await figure.locator('.diagram-timeline,.diagram-relations,.diagram-alternative').count(),0);
 const svg=figure.locator('svg'),ns=await figure.getAttribute('id');
 assert.equal(await svg.getAttribute('data-origin-namespace'),ns);
 for(const [i,g]of diagram.meaning.groups.entries()){
  for(const label of [...g.messages.map(m=>m.label),...(g.condition?[g.condition]:[])]){
   assert(await figure.locator(`.diagram-claim[data-unit="${label.unit}"]`).count()>0,'each original underlying claim stays selectable');
  }
 }
 if(scripted){await figure.locator('[data-diagram-action=reset]').click();await figure.locator('[data-diagram-action=in]').click();}
 const geometry=await inspectDiagram(svg,diagram,answers.prose);write('timeline-geometry',geometry);assert.deepEqual(geometry.errors,[]);
 const message=diagram.meaning.groups.flatMap(g=>g.messages).at(-1),element=svg.locator(`[id="${ns}-${message.id}"]`);
 const claimId=(await element.getAttribute('href')).slice(1),claim=page.locator(`[id="${claimId}"]`),unit=answers.prose[message.label.unit];
 const citationIndex=unit.citations.findIndex(q=>q.claim_start_byte<=message.label.start_byte&&q.claim_end_byte>=message.label.end_byte);assert(citationIndex>=0);
 const citation=unit.citations[citationIndex],action=claim.locator(`a.evidence-link[href$="-c${citationIndex+1}"]`),point=await citationPoint(page,await action.getAttribute('href'));
 await element.focus();await settled();
 const before=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left,pan:n.closest('.diagram-viewport').scrollLeft}));
 await screenshot('timeline-before');await element.press('Enter');await settled();if(!scripted){await action.click();await settled();}
 let bytes;
 if(scripted){await point.locator('.citation-inspection code').waitFor({state:'visible'});await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-read')?.disabled===false,await point.getAttribute('id'));bytes=await point.locator('.citation-inspection code').textContent();}
 else {bytes=await(await openEmbeddedCode(point)).textContent();await returnFromEmbeddedCode(point);}
 assert.equal('sha256:'+require('node:crypto').createHash('sha256').update(bytes).digest('hex'),citation.range_sha256,'complete exact original code');
 await screenshot('timeline-code');const deep=page.url();
 if(scripted)await page.keyboard.press('Escape');else await point.locator('a.return:visible').click();await settled();
 if(scripted)assert(await element.evaluate(n=>document.activeElement===n),'exact temporal focus return');else assert.equal(new URL(page.url()).hash,'#'+ns+'-'+message.id);
 const after=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left,pan:n.closest('.diagram-viewport').scrollLeft}));
 if(scripted)for(const key of ['top','left','pan'])assert(Math.abs(before[key]-after[key])<3,'visible temporal context return: '+key);
 await screenshot('timeline-returned');await page.goto(deep);await settled();
 if(scripted){await point.locator('.citation-inspection code').waitFor({state:'visible'});await page.keyboard.press('Escape');await settled();assert(await element.evaluate(n=>n===document.activeElement),'fresh deep evidence returns to exact origin');}
 else {await point.locator('a.return:visible').click();await settled();assert.equal(new URL(page.url()).hash,'#'+ns+'-'+message.id);}
 const duplicates=await page.evaluate(()=>{const ids=[...document.querySelectorAll('[id]')].map(n=>n.id);return ids.filter((id,i)=>ids.indexOf(id)!==i);});assert.deepEqual(duplicates,[]);
 return {origin:ns+'-'+message.id,citation:citationIndex,source:citation.source_id,codeBytes:Buffer.byteLength(bytes),before,after};
}
module.exports={inspect};
