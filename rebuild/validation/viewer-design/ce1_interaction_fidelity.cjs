// Real selected-reference interactions, shared by repository and Work documents.
// This complements composition/Source authority checks; no human verdict is inferred.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const crypto=require('node:crypto'),{spawn}=require('node:child_process'),{pathToFileURL}=require('node:url');
const {citationPoint,citationReady}=require('./product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2])),out=path.resolve(config.output);
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
assert.equal(sha(config.reference),'295961bcee3df01f9a7c7bfab126b33ce2ad00cc6b9fcdebc34b088e8d794046');
fs.mkdirSync(out,{recursive:false});
const report={outcome:'failed',viewer:sha(config.viewer),reference:sha(config.reference),driver:sha(__filename),config:sha(process.argv[2]),publications:config.cases.map(c=>({key:c.key,project:c.project,work:c.work||null,language:c.language,locale:c.locale,publication:c.publication,native_sha256:sha(c.offline)})),rows:[],failures:[],unobserved:['human acceptance','independent first reading','assistive technology','physical touch','other browser engines']};
const save=(name,value)=>fs.writeFileSync(path.join(out,name+'.json'),JSON.stringify(value,null,2),{flag:'wx'});
async function settle(page){await page.evaluate(()=>document.fonts.ready);await page.waitForTimeout(100);}
async function styles(locator){return locator.evaluate(n=>{const s=getComputedStyle(n);return Object.fromEntries(['fontSize','fontWeight','lineHeight','paddingTop','paddingRight','paddingBottom','paddingLeft','borderTopWidth','borderRadius','backgroundColor','color'].map(k=>[k,s[k]]));});}
async function inspect(page,key,width,reference=false,native=false){
 const row={key,width,reference,native},doc=page.locator('.doc:visible'),toc=doc.locator('.document-nav');
 assert.equal(await doc.locator(':scope > .work-reading-group').count(),3,'same three document parents');
 assert.equal(await toc.getAttribute('data-overflow'),'true','single compact contents for repositories and Work');
 const headings=await toc.locator('.toc-primary a').evaluateAll(links=>links.map(a=>({id:a.hash.slice(1),title:a.lastElementChild.textContent})).filter(v=>document.getElementById(v.id)?.getClientRects().length));
 assert(headings.length>=2,'actual authored sections');
 row.scroll=[];
 for(const h of headings.slice(0,3)){
  await page.evaluate(id=>{const n=document.getElementById(id),nav=n.closest('.doc').querySelector('.document-nav');scrollTo(0,scrollY+n.getBoundingClientRect().top-nav.getBoundingClientRect().height-20);},h.id);await settle(page);
  const cue=await toc.locator('.toc-cue').textContent();row.scroll.push({heading:h.title,cue});
  if(!native){const current=await toc.locator('[aria-current=location] span:last-child').textContent();assert.equal(cue,current);assert.equal(cue,h.title,'passive section cue follows actual scroll, never a hidden record');}
 }
 // The text selector stays independent of Work-only root ordering.
 const fullEvidence=doc.locator('[data-reading-group=records] > details').filter({hasText:/전체 근거|Full evidence/}).first();
 await fullEvidence.locator(':scope > summary').click();
 const categories=fullEvidence.locator(':scope > div > details');assert(await categories.count()>=5,'all actual supporting categories');
 row.category=await styles(categories.first().locator(':scope > summary'));
 const index=fullEvidence.locator('.evidence-index');
 assert.deepEqual(await styles(index.locator(':scope > summary')),row.category,'evidence index has the same category surface');
 await index.locator(':scope > summary').click();
 const entry=index.locator(':scope > details').first();row.index=await styles(entry.locator(':scope > summary'));
 assert.equal(await entry.evaluate(n=>getComputedStyle(n).borderTopWidth),'1px','index items are subordinate cards');
 assert.equal(await entry.evaluate(n=>getComputedStyle(n).borderRadius),'8px');
 await entry.locator(':scope > summary').click();
 await index.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(out,key+'-'+width+'-records.png')});
 await fullEvidence.locator(':scope > summary').click();
 const figure=doc.locator('figure').first();await figure.evaluate(n=>n.scrollIntoView({block:'center'}));await settle(page);
 if(native&&await figure.locator('.native-diagram-controls:visible').count())assert(await figure.evaluate(n=>n.querySelector('svg').getBoundingClientRect().top>=n.querySelector('.native-diagram-controls').getBoundingClientRect().bottom+8),'native scale controls do not cover the first diagram content');
 if(!native){
  const controls=figure.locator(reference?'.spatial-navigation':'.diagram-controls');
  row.toolbarTop=await controls.evaluate(n=>Math.round(n.getBoundingClientRect().top-n.parentElement.getBoundingClientRect().top));
  const move=controls.locator('details');await move.locator(':scope > summary').click();
  const jump=move.locator(reference?'a[data-world-target]':'a[data-diagram-target]');
  row.jumpCount=await jump.count();assert(row.jumpCount>5,'Move includes the real relation, condition and participant locations');
  const next=jump.nth(1),target=await next.getAttribute(reference?'data-world-target':'data-diagram-target');await next.click();
  assert.equal(await move.getAttribute('open'),null,'jump closes the menu');
  assert.equal(await page.evaluate(()=>document.activeElement.id),target,'jump focuses exact role at readable scale');
  const strip=figure.locator(reference?'.spatial-selection':'.diagram-selection');
  assert((await strip.textContent()).includes(await page.locator('[id="'+target+'"]').getAttribute('aria-label')),'selected relationship remains visible beside its evidence action');
  await page.screenshot({path:path.join(out,key+'-'+width+'-diagram.png')});
  if(!reference){
   await strip.locator('a').click();await settle(page);
   const selected=page.locator('.source-point[data-selected=true]');await citationReady(selected);
   await selected.locator(':scope > .point-bar > a.return').click();await settle(page);
   assert.equal(await page.evaluate(()=>document.activeElement.id),target,'location-menu evidence returns to that exact diagram role');
  }
 }
 const actions=doc.locator('.unit > .evidence-actions > a[data-citation-targets]');
 const multiple=await actions.evaluateAll(ns=>ns.findIndex(n=>JSON.parse(n.dataset.citationTargets).length>1));
 const action=multiple>=0?actions.nth(multiple):doc.locator('.paragraph-evidence-action').first();
 const href=await action.getAttribute('href');await action.click();await settle(page);
 const pane=page.locator('.appendix:visible');let point;
 if(!reference){point=await citationPoint(page,href);if(!native)await citationReady(point);}
 row.pane={width:await pane.evaluate(n=>n.getBoundingClientRect().width)};
 if(!native){
  const select=pane.locator('select:visible');assert.equal(await select.count(),1,'one chooser in selected context');
  row.chooser=await styles(select);row.options=await select.locator('option').count();
  assert(row.options>=1);if(multiple>=0)assert(row.options>1,'whole original range family');
  if(row.options>1){await select.selectOption({index:1});await settle(page);assert.equal(await pane.locator('select:visible').inputValue(),await select.locator('option').nth(1).getAttribute('value'));assert.equal(await page.evaluate(()=>document.activeElement.tagName),'SELECT','chooser retains keyboard context');}
  if(width>760){
   const handle=pane.locator(reference?'.evidence-resize-handle':'.evidence-resizer');const bounds=await handle.boundingBox();assert(bounds.height>=990,'resize hit target spans the pane boundary');
   const before=await pane.evaluate(n=>n.getBoundingClientRect().width);await page.mouse.move(bounds.x+bounds.width/2,500);await page.mouse.down();await page.mouse.move(bounds.x+bounds.width/2-112,500,{steps:8});await page.mouse.up();await settle(page);
   const after=await pane.evaluate(n=>n.getBoundingClientRect().width);assert(Math.abs(after-before-112)<3,'actual drag changes panel width');
   await handle.press('Home');assert.equal(await pane.evaluate(n=>Math.round(n.getBoundingClientRect().width)),width>=1200?360:320);
   await handle.press('End');assert.equal(await pane.evaluate(n=>Math.round(n.getBoundingClientRect().width)),Math.min(960,width-(width>=1200?600:48)));
   await handle.press('Enter');assert.equal(await pane.evaluate(n=>Math.round(n.getBoundingClientRect().width)),width>=1200?464:540);
   await handle.press('ArrowLeft');const preferred=await pane.evaluate(n=>Math.round(n.getBoundingClientRect().width));
   await page.setViewportSize({width:390,height:1000});await settle(page);await page.setViewportSize({width,height:1000});await settle(page);
   assert.equal(await pane.evaluate(n=>Math.round(n.getBoundingClientRect().width)),preferred,'mobile full screen preserves the adjusted desktop/sheet width');await handle.press('Enter');
   row.pane.drag={before,after};
  }
 }
 await page.screenshot({path:path.join(out,key+'-'+width+'-evidence.png')});
 const back=pane.locator('.point-bar:visible a.return');const returnId=(await back.getAttribute('href')).slice(1);await back.click();await settle(page);
 assert.equal(await pane.isVisible(),false,'return closes evidence');
 assert.equal(await page.evaluate(()=>location.hash.slice(1)),returnId,'exact originating claim');
 if(!native&&!reference)assert(await action.evaluate((trigger,id)=>document.activeElement===trigger||document.activeElement.id===id||document.getElementById(id)?.contains(document.activeElement),returnId),'focus restores the actual origin or its evidence trigger');
 return row;
}
async function main(){let browser;const processes=[];try{
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 const expected=new Map();
 for(const c of [{key:'reference',work:config.referenceWork,reference:true},...config.cases]){
  let child,address='',log='';
  if(!c.reference){child=spawn(config.viewer,['--runtime',c.runtime,'--project',c.project,'--bind','127.0.0.1:0']);processes.push(child);address=await new Promise((ok,no)=>{const timer=setTimeout(()=>no(Error(log||'listener timeout')),15000);child.on('error',no);child.stderr.on('data',b=>{log+=b;const m=log.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);ok(m[0]);}});});}
  try{for(const width of config.widths||[1440,1024,768,390,320])for(const native of c.reference?[false]:[false,true]){
   const context=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:!native}),page=await context.newPage();page.setDefaultTimeout(10000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
   await context.route('**/*',r=>r.request().url().startsWith(address||'file:')||r.request().url().startsWith('file:')||r.request().url().startsWith('data:')?r.continue():r.abort());
   const url=c.reference?pathToFileURL(path.resolve(config.reference)).href+'#work-'+c.work:native?pathToFileURL(path.resolve(c.offline)).href+(c.work?'#work-'+c.work:'#repository-explanation'):address+'?view='+(c.work?'work&work='+c.work:'repository')+'&locale='+c.locale+'&language='+c.language;
   try{await page.goto(url);await settle(page);const row=await inspect(page,c.key+(native?'-native':''),width,c.reference,native);assert.deepEqual(errors,[]);if(c.reference)expected.set(width,row);else {assert.deepEqual(row.category,expected.get(width).category,'category design agrees with original');assert.deepEqual(row.index,expected.get(width).index,'index item design agrees with original');if(!native){assert.deepEqual(row.chooser,expected.get(width).chooser,'range chooser design agrees with original');assert.equal(row.toolbarTop,expected.get(width).toolbarTop,'controls sit at the same position inside the drawing frame');}}report.rows.push(row);}
   catch(e){report.failures.push({key:c.key,width,native,error:e.stack});await page.screenshot({path:path.join(out,c.key+'-'+width+'-'+native+'-failure.png')}).catch(()=>{});if(c.reference)throw e;}
   finally{await context.close();}
  }}finally{child?.kill('SIGTERM');if(child)fs.writeFileSync(path.join(out,c.key+'-viewer.stderr'),log,{flag:'wx'});}
 }
 assert.equal(report.failures.length,0,'interaction differences retained in result.json');report.outcome='passed';
 }finally{for(const p of processes)p.kill('SIGTERM');await browser?.close();save('result',report);}console.log(JSON.stringify({outcome:report.outcome,rows:report.rows.length,failures:report.failures.length}));}
main().catch(e=>{console.error(e);process.exitCode=1});
