// Actual retained Product outputs; reference structure, independent Source/prose oracles.
const fs=require('node:fs'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium}=require(config.playwright);
const sourceBytes=item=>item.expected_sources||[{source_id:item.after,text:'def value():\n    return 5\n'},...(item.before?[{source_id:item.before,text:`def value():\n    return ${item.comparison==='changed'?4:5}\n`}]:[])];
function checkDiagnostic(text,expected){assert.ok(text.includes(expected),'distinct missing/changed dependency diagnostic');}
const ordered=expected=>['Heading','Synopsis'].flatMap(q=>expected.prose.filter(p=>p.question===q)).concat(expected.prose.filter(p=>!['Heading','Synopsis','Limits'].includes(p.question)),expected.prose.filter(p=>p.question==='Limits'));
async function checkDocument(page,item,language,expected){
 const doc=page.locator(`article.doc[data-work-id="${item.work}"]`);
 assert.equal(await doc.count(),1,'exact Work document');
 const prose=doc.locator('[data-content-language][data-question]');
 assert.deepEqual(await prose.allTextContents(),ordered(expected).map(p=>p.text),'complete original interpretation');
 assert.ok(await prose.evaluateAll((nodes,language)=>nodes.every(n=>n.lang===language&&n.dataset.contentLanguage===language),language),'requested prose language');
 assert.equal(await doc.locator('h1').count(),1,'one document title');
 assert.equal(await doc.locator(':scope > .audit > details.record-box').count(),2,'reference original/full-evidence disclosures');
 assert.ok(!(await doc.locator('.record-strip').innerText()).includes('Checkpoint verification'),'opening contains concise lifecycle only');
 for(const removed of ['.card-status-strip','.work-audit','.fact-states','.overview-jumps','.work-state-history'])assert.equal(await doc.locator(removed).count(),0,'obsolete presentation removed');
 for(const source of sourceBytes(item)){
  const points=page.locator(`.source-point[data-source-id="${source.source_id}"]`);
  assert.ok(await points.count()>0,'exact current Source identity');
  for(const point of await points.all()){assert.equal(await point.locator('code').textContent(),source.text,'actual current code bytes');assert.equal(await point.locator('code').evaluate(n=>getComputedStyle(n).whiteSpace),'pre');assert.equal(await point.locator('code').evaluate(n=>getComputedStyle(n).overflowWrap),'normal');}
 }
 const links=doc.locator('a.evidence-link');
 assert.equal(await links.count(),expected.prose.reduce((n,p)=>n+p.citations.length,0),'every authored citation remains reachable');
 for(const link of await links.all()){
  const point=page.locator(`[id="${(await link.getAttribute('href')).slice(1)}"]`);
  assert.equal(await point.count(),1,'unique claim-specific Source target');
  assert.equal((await point.locator('.return').getAttribute('href')).slice(1),await link.getAttribute('data-origin'),'exact paragraph return');
 }
 const ids=await page.locator('[id]').evaluateAll(nodes=>nodes.map(n=>n.id));
 assert.equal(ids.length,new Set(ids).size,'unique fragment identities');
 return doc;
}
async function roundTrip(page,link,width,live){
 await link.press('Enter');
 const id=(await link.getAttribute('href')).slice(1),point=page.locator(`[id="${id}"]`),panel=point.locator('xpath=..');
 const box=await panel.boundingBox();
 assert.ok(box&&Math.abs(box.y-(width>900?76:0))<2&&Math.abs(box.x+box.width-width)<2,'Source at viewport edge');
 assert.ok(width>900?box.x>width*.4:box.x<2,'responsive Source panel');
 const back=point.locator('.return'),origin=(await back.getAttribute('href')).slice(1);
 if(live&&width<=900){await page.waitForFunction(id=>document.getElementById(id)?.getAttribute('aria-modal')==='true',id);assert.equal(await point.getAttribute('aria-modal'),'true');assert.ok(await page.locator('.doc').evaluate(n=>n.inert),'narrow background inert');}
 // Native Tab/Enter, including script-free snapshots, must reach the return.
 for(let i=0;i<5;i++){await page.keyboard.press('Tab');if(await back.evaluate(n=>n===document.activeElement))break;}
 assert.ok(await back.evaluate(n=>n===document.activeElement),'keyboard reaches exact return');
 await page.keyboard.press('Enter');
 if(live)await page.waitForFunction(id=>document.activeElement.id===id,origin);
 assert.equal(await page.evaluate(()=>document.activeElement.id),origin,'return restores exact claim focus');
 if(live){await link.press('Enter');await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>document.activeElement.id),origin,'Escape restores exact claim focus');}
}
(async()=>{
 const browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox']});
 const context=await browser.newContext();const page=await context.newPage();const observations=[];
 try{
  if(config.denied_state){
   let controls=0;
   for(const item of config.cases)for(const language of ['en','ko'])for(const view of ['work','code']){
    await page.goto(`${config.url}?view=${view}${view==='code'?'&scope=work':''}&work=${item.work}&locale=${language}&language=${language}`);
    assert.equal(await page.locator('.source-point').count(),0,'unavailable Source body withheld');
    const main=await page.locator('main').textContent(),original=config.basis[item.project][language].prose.find(p=>p.question==='ReportedChange').text;
    assert.ok(!main.includes(original),'stale interpretation withheld');
    checkDiagnostic(main,config.expected_diagnostic);
    assert.throws(()=>checkDiagnostic(main.replaceAll(config.expected_diagnostic,'opposite dependency failure control'),config.expected_diagnostic),/distinct missing\/changed dependency diagnostic/);
    if(view==='work'){assert.ok(await page.locator('.explanation-diagnostic').isVisible(),'actual failure visible');controls++;}
   }
   console.log(JSON.stringify({status:'passed',control:config.denied_state,diagnostic_controls:controls}));return;
  }
  for(const item of config.cases)for(const language of config.languages||['en','ko'])for(const width of [390,768,1440]){
   const expected=config.basis[item.project][language];await page.setViewportSize({width,height:1000});
   const base=config.offline?pathToFileURL(config.snapshots[item.project][language]).href:config.url;
   if(config.offline)await context.setOffline(true);
   for(const view of ['overview','work']){
    await page.goto(config.offline?base+`#${view==='work'?'works':'overview'}`:`${base}?view=${view}&locale=${language}&language=${language}`);
    await page.evaluate(()=>scrollTo(0,0));
    assert.equal(await page.locator('.masthead nav a').count(),2,'reference masthead has exactly two links');
    assert.equal(await page.locator('.masthead details').count(),0,'no old tool/language menus');
    const root=config.offline?page.locator(view==='work'?'#works':'#overview'):page.locator('main');
    for(const removed of ['.overview-jumps','.category-details','.current-work','.card-status-strip','.summary-records'])assert.equal(await root.locator(removed).count(),0,'obsolete ordinary structure removed');
    assert.ok(await root.locator('.work-card').count()>0,'actual Work cards');
    for(const card of await root.locator('.work-card').all()){
     const box=await card.boundingBox(),gutter=width>900?48:width>600?28:20;
     assert.ok(box&&Math.abs(box.x-gutter)<2&&Math.abs(box.width-(width-2*gutter))<2,'reference full-width cards');
     assert.equal(await card.locator('details').count(),0,'no audit disclosures inside cards');
     assert.ok((await card.locator('.card-action').boundingBox()).height>=44,'reference reading action');
    }
    await page.screenshot({path:`${config.output}/${item.comparison}-${language}-${width}-${view}-${config.offline?'offline':'live'}.png`});
   }
   const url=config.offline?base+`#work-${item.work}`:`${base}?view=work&work=${item.work}&locale=${language}&language=${language}`;
   const response=await page.goto(url);await page.evaluate(()=>scrollTo(0,0));
   if(config.offline){assert.equal(await page.locator('script,form').count(),0);assert.ok(await page.locator('a[href]').evaluateAll(nodes=>nodes.every(n=>n.getAttribute('href').startsWith('#'))),'offline links remain local');}
   else{
    const digest=crypto.createHash('sha256').update(await page.locator('script[data-viewer-reading]').textContent()).digest('base64');
    assert.ok((await response.allHeaders())['content-security-policy'].includes(`script-src 'sha256-${digest}'`),'exact script CSP');
   }
   const doc=await checkDocument(page,item,language,expected),box=await doc.boundingBox();
   assert.ok(Math.abs(box.x-(width>=1100?182.4:width>600?28:20))<2,'reference document offset');
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'ordinary viewport fits');
   await page.screenshot({path:`${config.output}/${item.comparison}-${language}-${width}-reading-${config.offline?'offline':'live'}.png`});
   for(const unit of expected.prose){
    for(let n=0;n<unit.citations.length;n++){
     const citation=unit.citations[n],claim=Buffer.from(unit.text).subarray(citation.claim_start_byte,citation.claim_end_byte).toString('utf8');
     const source=page.locator(`.source-point[data-source-id="${citation.source_id}"]`).filter({has:page.locator('.claim-quote',{hasText:claim})});
     assert.ok(await source.count()>0,'exact bound claim retained with its Source');
    }
   }
   const ordinary=doc.locator('.unit-text').first(),original=await ordinary.textContent();
   await ordinary.evaluate(n=>n.textContent='fabricated prose control');
   await assert.rejects(()=>checkDocument(page,item,language,expected),/complete original interpretation/);
   await ordinary.evaluate((n,text)=>n.textContent=text,original);
   const point=page.locator(`.source-point[data-source-id="${item.after}"]`).first(),code=point.locator('code'),bytes=await code.textContent();
   await code.evaluate(n=>n.textContent='fabricated Source control');
   await assert.rejects(()=>checkDocument(page,item,language,expected),/actual current code bytes/);
   await code.evaluate((n,text)=>n.textContent=text,bytes);
   const identityPoints=page.locator(`.source-point[data-source-id="${item.after}"]`);
   await identityPoints.evaluateAll(nodes=>nodes.forEach(n=>n.dataset.sourceId='foreign-source-control'));
   await assert.rejects(()=>checkDocument(page,item,language,expected),/exact current Source identity/);
   await page.locator('[data-source-id="foreign-source-control"]').evaluateAll((nodes,id)=>nodes.forEach(n=>n.dataset.sourceId=id),item.after);
   const audit=doc.locator('.audit > details').last();if(!await audit.evaluate(n=>n.open))await audit.locator(':scope > summary').click();
   for(const link of await doc.locator('.evidence-link').all())await roundTrip(page,link,width,!config.offline);
   const first=doc.locator('.evidence-link').first();await first.press('Enter');
   await page.screenshot({path:`${config.output}/${item.comparison}-${language}-${width}-source-${config.offline?'offline':'live'}.png`});
   for(let step=0;step<12;step++){await page.keyboard.press('Tab');if(await page.evaluate(()=>document.activeElement.matches('.code-region')))break;}
   assert.ok(await page.evaluate(()=>document.activeElement.matches('.code-region')),'keyboard enters code scroll region');
   const wide=await page.evaluate(()=>document.activeElement.scrollWidth>document.activeElement.clientWidth+1);
   if(wide){await page.keyboard.press('End');await page.keyboard.press('ArrowRight');await page.waitForTimeout(100);assert.ok(await page.evaluate(()=>document.activeElement.scrollLeft>0),'keyboard scrolls long original code');}
   await page.locator('.source-point:target .return').press('Enter');
   observations.push({comparison:item.comparison,language,width,mode:config.offline?'offline':'live',status:'passed'});
  }
 }finally{await browser.close();}
 fs.writeFileSync(`${config.output}/${config.offline?'offline':'live'}-observations.json`,JSON.stringify(observations,null,2));
 console.log(JSON.stringify({status:'passed',observations:observations.length,human_comprehension:'not_assessed'}));
})().catch(error=>{console.error(error.stack);process.exitCode=1;});
