// Actual Product HTTP/native/export journeys. No generation, capture or semantic verdict.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync}=require('node:child_process');
const {pathToFileURL}=require('node:url'),crypto=require('node:crypto');
const {citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('./product_review_controls.cjs');
const {inspectDiagram}=require('../end-to-end/multi-repository/diagram_geometry.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
const sha=x=>crypto.createHash('sha256').update(x).digest('hex');
fs.mkdirSync(config.output,{recursive:false});
const out=config.output,rows=[],processes=[],errors=[];
const write=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2));
async function main(){
 const child=spawn(config.viewer,['--runtime',config.runtime,'--project',config.project,'--bind','127.0.0.1:0']);
 let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);
 let browser;
 try{
  const address=await new Promise((resolve,reject)=>{
   const timer=setTimeout(()=>reject(Error('Viewer startup timeout')),15000);
   child.stderr.on('data',b=>{stderr+=b;const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});
   child.once('error',reject);child.once('exit',code=>{clearTimeout(timer);reject(Error('Viewer exited '+code+stderr));});
  });
  const {chromium}=require(config.playwright);browser=await chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
  const realization=JSON.parse(fs.readFileSync(config.realization));
  const diagram=realization.diagrams.find(d=>d.id===config.diagram);assert(diagram);
  for(const width of config.widths||[1440,1024,768,390,320])for(const mode of config.modes||['live','live-no-js','offline']){
   const scripted=mode==='live',context=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:scripted});
   const page=await context.newPage();page.on('pageerror',e=>errors.push(String(e)));
   await page.route('**/*',route=>{const u=new URL(route.request().url());if(['file:','data:'].includes(u.protocol)||u.origin===new URL(address).origin)route.continue();else route.abort();});
   const url=mode==='offline'?pathToFileURL(config.offline).href+'#work-'+config.work:`${address}?view=work&work=${config.work}&locale=ko&language=ko`;
   const start=performance.now();
   const catalogue=mode==='offline'?pathToFileURL(config.offline).href+'#works':`${address}?view=work&locale=ko&language=ko`;
   await page.goto(catalogue);
   const summary=JSON.parse(fs.readFileSync(config.summary)).paragraphs[0].text;
   const row=mode==='offline'?page.locator(`#works .work-card[data-work-id="${config.work}"]`):page.locator(`.work-row[data-work-id="${config.work}"]`);
   assert((await row.textContent()).includes(summary),'independent summary precedes article');
   if(mode==='offline')await row.locator('a').first().click();else await row.click();
   await page.locator('.doc-head h1:visible').waitFor();await page.waitForTimeout(120);
   assert.equal(await page.locator('.doc-head .lead:visible').first().textContent(),realization.paragraphs.find(p=>p.question==='synopsis').text);

   const doc=page.locator(`[id="work-${config.work}"] .doc`),figure=doc.locator('.admitted-diagram').filter({has:page.locator(`[data-origin-namespace$="diagram-${config.diagram}"]`)});
   assert.equal(await figure.count(),1);assert.equal(await figure.locator('svg').count(),1,'one production scene');
   assert.equal(await figure.locator('.diagram-timeline,.diagram-alternative,.diagram-relations').count(),0,'superseded views absent');
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true,'ordinary page width');
   for(const [i,u]of realization.paragraphs.entries()){
    if(u.question==='heading'||u.question==='synopsis')continue;
    const text=await doc.locator(`[id$="-p${i+1}"] > .unit-text`).first().textContent();assert.equal(text,u.text,'admitted original wording '+i);
   }
   assert.equal(await doc.locator('.verification-reading .verification-execution').count(),0,'report is distinct from execution originals');
   const records=doc.locator('.audit');assert.equal(await records.locator('.verification-execution[open]').count(),0,'command originals start closed');
   await figure.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(out,`${mode}-${width}-initial.png`)});
   const svg=figure.locator('svg');
   if(scripted){
    await figure.locator('[data-diagram-action=expand]').click();assert.equal(await figure.getAttribute('aria-modal'),'true');
    await page.keyboard.press('Escape');assert.equal(await figure.locator('[data-diagram-action=expand]').getAttribute('aria-expanded'),'false');
    await figure.locator('[data-diagram-action=reset]').click();await figure.locator('[data-diagram-action=in]').click();
    const v=figure.locator('.diagram-viewport');await v.focus();await v.press('ArrowDown');await v.press('ArrowRight');
    await figure.locator('[data-diagram-action=right]').click();
   }
   const geometry=await inspectDiagram(svg,diagram,realization.paragraphs);write(`${mode}-${width}-geometry.json`,geometry);
   assert.deepEqual(geometry.errors,[],'source-derived visible geometry');
   const ns=await figure.getAttribute('id'),element=figure.locator(`[id="${ns}-${config.element}"]`);
   assert.equal(await element.count(),1);
   if(scripted&&width===1024)await figure.locator('[data-diagram-action=expand]').click();
   await element.focus();await page.waitForTimeout(60);
   const before=await element.evaluate(n=>({id:n.id,y:scrollY,left:n.closest('.diagram-viewport').scrollLeft,top:n.closest('.diagram-viewport').scrollTop,scale:n.closest('figure').dataset.scale}));
   await element.press('Enter');await page.waitForTimeout(120);
   const claim=page.locator(`[id="${ns}-${config.element}-claim"]`);
   const action=claim.locator('a.evidence-link').first();
   if(!scripted){await action.click();await page.waitForTimeout(60);}
   const href=await action.getAttribute('href'),point=await citationPoint(page,href);
   let code;
   if(scripted){await point.locator('.citation-inspection code').waitFor({state:'visible'});await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-read')?.disabled===false,await point.getAttribute('id'));code=await point.locator('.citation-inspection code').textContent();}
   else {code=await (await openEmbeddedCode(point)).textContent();await returnFromEmbeddedCode(point);}
   const label=diagram.meaning.groups?.flatMap(g=>g.messages).find(m=>m.id===config.element)?.label||diagram.meaning.relations?.find(m=>m.id===config.element)?.label;
   assert(label);const c=realization.paragraphs[label.unit].citations.find(c=>c.claim_start_byte<=label.start_byte&&c.claim_end_byte>=label.end_byte);
   const source=fs.readFileSync(path.join(config.repository,c.locator)).subarray(c.start_byte,c.end_byte);
   assert.equal('sha256:'+sha(source),c.range_sha256,'original repository range');assert.equal(code,source.toString('utf8'),'complete permitted bytes');
   if(scripted&&width>760){
    const handle=point.locator('xpath=..').locator(':scope > .evidence-resizer');await handle.focus();await handle.press('ArrowLeft');await handle.press('Home');
   }
   if(scripted){
    const chosen=await point.getAttribute('id');
    await page.setViewportSize({width:width===390?1440:390,height:1000});await page.waitForTimeout(60);
    assert.equal(await page.locator('.source-point[data-selected]').getAttribute('id'),chosen,'resize preserves publication/claim/range');
    await page.setViewportSize({width,height:1000});await page.waitForTimeout(60);
   }
   await page.screenshot({path:path.join(out,`${mode}-${width}-code.png`)});
   if(scripted)await page.keyboard.press('Escape');else await point.locator('a.return:visible').click();
   await page.waitForTimeout(180);
   const after=await element.evaluate(n=>({id:document.activeElement.id,left:n.closest('.diagram-viewport').scrollLeft,top:n.closest('.diagram-viewport').scrollTop,scale:n.closest('figure').dataset.scale}));
   if(scripted){assert.equal(after.id,before.id,'exact focus');assert.equal(after.scale,before.scale);assert(Math.abs(after.left-before.left)<3&&Math.abs(after.top-before.top)<3,'same camera return');}
   await page.screenshot({path:path.join(out,`${mode}-${width}-return.png`)});
   if(scripted&&width===1024){assert.equal(await figure.locator('[data-diagram-action=expand]').getAttribute('aria-expanded'),'true','code return restores expanded view');await page.keyboard.press('Escape');}
   if(scripted&&[1440,320].includes(width)){
    await element.press('Enter');await point.locator('.citation-inspection code').waitFor({state:'visible'});
    await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-read')?.disabled===false,await point.getAttribute('id'));
    const publication=await point.getAttribute('data-reader-publication');
    await page.route('**/citation?*',route=>route.fulfill({status:410,contentType:'application/json',body:JSON.stringify({reason:'deleted',publication_readable:false})}));
    await point.locator('.citation-read').click();await page.waitForFunction(id=>document.getElementById(id)?.dataset.unavailable==='true',await point.getAttribute('id'));
    assert.equal(await page.locator(`[data-reader-publication="${publication}"] code`).count(),0,'denial purges every dependent code copy');
    assert.equal(await page.locator(`[data-reader-publication="${publication}"] svg`).count(),0,'denial purges scene/labels/alternatives');
    await page.setViewportSize({width:1024,height:900});await page.goBack();await page.waitForTimeout(80);
    assert.equal(await page.locator(`[data-reader-publication="${publication}"] svg`).count(),0,'resize/history cannot revive a denied scene');
   }
   rows.push({width,mode,code_bytes:Buffer.byteLength(code),load_ms:performance.now()-start,before,after,geometry_errors:geometry.errors.length});write('results.json',{rows,errors,processes});await context.close();
  }
  assert.deepEqual(errors,[],'no browser exceptions');
 }finally{
  await browser?.close();if(child.exitCode===null){await new Promise(resolve=>{child.once('exit',(code,signal)=>{processes.push({exit:code,signal,termination:'explicit cleanup'});resolve();});child.kill('SIGTERM');});}
  fs.writeFileSync(path.join(out,'viewer.stdout'),stdout);fs.writeFileSync(path.join(out,'viewer.stderr'),stderr);
  write('results.json',{rows,errors,processes,viewer_sha256:sha(fs.readFileSync(config.viewer)),assessment:'dependent source-first technical control; human/assistive acceptance unobserved'});
 }
}
main().catch(e=>{console.error(e.stack);process.exitCode=1;});
