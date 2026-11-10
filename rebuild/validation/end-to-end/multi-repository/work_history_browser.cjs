// Technical reading controls over installed live/native/offline consumers.
// No authoring, hidden capture, semantic verdict or simulated human observation.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync,execFileSync}=require('node:child_process');
const {pathToFileURL}=require('node:url'),crypto=require('node:crypto');
const {fragmentErrors}=require('./reader_fragments.cjs');
const C=JSON.parse(fs.readFileSync(process.argv[2]));
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
fs.mkdirSync(C.output,{recursive:false});
const result={status:'not_run',candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
 source_diff_sha256:sha(execFileSync('git',['diff','HEAD','--','rebuild'])),
 worktree_clean:execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim()==='',
 driver_sha256:sha(fs.readFileSync(__filename)),viewer_sha256:sha(fs.readFileSync(C.viewer)),
 cli_sha256:sha(fs.readFileSync(C.cli)),browser_sha256:sha(fs.readFileSync(C.chromium)),
 human_observation:'not_performed',actual_browser_zoom:'unavailable_in_headless_shell',checks:[],processes:[]};
if(C.require_clean)assert.equal(result.worktree_clean,true,'exact clean candidate required');
function command(label,binary,args){
 const p=spawnSync(binary,args,{encoding:'utf8',timeout:120000});
 fs.writeFileSync(path.join(C.output,label+'.stdout'),p.stdout||'');
 fs.writeFileSync(path.join(C.output,label+'.stderr'),p.stderr||'');
 result.processes.push({label,binary,args,exit_code:p.status,signal:p.signal,error:p.error?.message});
 assert.equal(p.status,0,label+': '+p.stderr);return p.stdout;
}
function state(c,label){return command(label,'python3',['-c',`import sqlite3,pathlib,hashlib,json,sys
r={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 with sqlite3.connect(p.resolve().as_uri()+'?mode=ro',uri=True) as db:r[p.name]=hashlib.sha256(('\\n'.join(db.iterdump())).encode()).hexdigest()
print(json.dumps(r,sort_keys=True))`,c.runtime]);}
async function serve(c){
 const child=spawn(C.viewer,['--runtime',c.runtime,'--project',c.project,'--bind','127.0.0.1:0']);
 let stderr='',stdout='';child.stdout.on('data',b=>stdout+=b);
 const address=await new Promise((resolve,reject)=>{
  const timer=setTimeout(()=>{child.kill();reject(Error('Viewer startup timeout'));},15000);
  child.stderr.on('data',b=>{stderr+=b;const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});
  child.once('error',reject);child.once('exit',code=>{clearTimeout(timer);reject(Error('Viewer exited '+code+stderr));});
 });
 return {address,async stop(){await new Promise(resolve=>{child.once('exit',(code,signal)=>{
  fs.writeFileSync(path.join(C.output,c.key+'-viewer.stderr'),stderr);fs.writeFileSync(path.join(C.output,c.key+'-viewer.stdout'),stdout);
  result.processes.push({label:c.key+'-viewer',exit_code:code,signal,termination:'explicit cleanup'});resolve();});child.kill('SIGTERM');});}};
}
const frames=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
async function inspect(page,c,url,width,mode,language){
 await page.goto('about:blank');await page.setViewportSize({width,height:1000});const started=performance.now();const response=await page.goto(url);await page.evaluate(()=>document.fonts.ready);
 const doc=page.locator(`[id="work-${c.work}"] > article.doc`);await doc.waitFor();
 assert.deepEqual(await page.evaluate(fragmentErrors),[],'unique exact fragments throughout actual public document');
 assert.equal(await doc.locator('.document-nav').count(),1,'one page-level Work TOC');
 assert.equal(await doc.locator('article.doc').count(),0,'no appended/nested Decision page');
 if(c.decisions!==undefined)assert.equal(await doc.locator('.decision-reading').count(),c.decisions);
 const groups=await doc.locator(':scope > .work-reading-group').evaluateAll(ns=>ns.map(n=>n.dataset.readingGroup));
 assert.deepEqual(groups,await doc.locator('.decision-reading').count()?['work','decisions','checks','records']:['work','checks','records'],'Work explanation, related Decisions, checks and originals are sibling groups');
 for(const group of ['checks','records']){
  const section=doc.locator(`:scope > .work-reading-group[data-reading-group="${group}"]`);
  assert.equal(await section.locator(':scope > h2.reading-group-heading').count(),1,'explicit Work-level section heading');
  assert.ok((await section.innerText()).trim().length>(await section.locator(':scope > h2').innerText()).trim().length,'section has content or an honest empty-state message');
 }
 const decisionFolds=doc.locator('.decision-reading > details.related-decision');
 assert.equal(await decisionFolds.count(),await doc.locator('.decision-reading').count(),'one shared disclosure per Decision');
 assert.equal(await decisionFolds.evaluateAll(ns=>ns.every(n=>!n.open)),true,'each Decision starts closed');
 const style=n=>{const s=getComputedStyle(n);return Object.fromEntries(['borderTopWidth','borderTopStyle','borderRadius','backgroundColor','paddingTop','paddingRight','fontSize','minHeight','textDecorationLine'].map(k=>[k,s[k]]));};
 const indents=await doc.locator('.document-nav nav ol a').evaluateAll(ns=>ns.map(n=>getComputedStyle(n).paddingLeft));
 assert.ok(new Set(indents).size<=1,'all outline child entries share the same indentation');
 for(const card of await doc.locator(':scope > [data-reading-group="records"] > details').all()){
  assert.equal(await card.evaluate(n=>getComputedStyle(n).borderTopStyle),'solid','original-record sections share bordered cards');
  assert.ok(await card.evaluate(n=>parseFloat(getComputedStyle(n).borderTopWidth))>=1);
 }
 for(const metadata of await doc.locator('.reading-metadata').all()){
  assert.equal(await metadata.locator('.comparison-icon').count(),1,'shared comparison status icon');
  assert.ok((await metadata.locator('[data-comparison-label]').textContent()).trim().length>0,'state has explicit non-color wording');
  assert.ok(!(await metadata.locator('.reading-dates').textContent()).includes('Unix'),'calendar format matches Work dates');
  for(const time of await metadata.locator('.reading-dates time').all())assert.match(await time.textContent(),/^\d{4}\.\d{2}\.\d{2} \d{2}:\d{2} UTC$/);
 }
 const evidenceStyle=doc.locator('a.evidence-link:not(.admitted-diagram a)').first();
 if(await evidenceStyle.count())for(const action of await doc.locator('.notice-actions > a,.notice-report-picker > summary').all())
  assert.deepEqual(await action.evaluate(style),await evidenceStyle.evaluate(style),'notice actions share actual evidence-control styling');
 if(await decisionFolds.count())await doc.locator('.related-decisions').screenshot({path:path.join(C.output,`${c.key}-${language}-${mode}-${width}-decisions-closed.png`)});
 for(const action of await doc.locator('.failure-check-links a').all())assert.equal(await action.evaluate(n=>!!n.closest('.reading-warning')),true,'notice owns every failure action');
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+2),false,'body remains bounded');
 // Open history alone: every long original is still individually folded.
 const history=doc.locator('.record-box').filter({has:page.locator('.recorded-timeline')});
 await history.locator(':scope > summary').click();
 const timeline=history.locator('.recorded-timeline');
 const originals=timeline.locator('.timeline-original');
 assert.equal(await originals.evaluateAll(ns=>ns.every(n=>!n.open)),true,'individual originals closed');
 for(const quote of await originals.locator('blockquote').all())assert.equal(await quote.isVisible(),false);
 if(await originals.count()){
  const original=originals.first(),quote=original.locator('blockquote'),text=await quote.textContent();
  await original.locator(':scope > summary').click();assert.equal(await quote.isVisible(),true);assert.equal(await quote.textContent(),text,'complete original text unchanged');
  const back=original.locator('a[href^="#"]').last(),target=await back.getAttribute('href');
  await back.press('Enter');if(mode==='live')await frames(page);
  assert.equal(await page.locator(`[id="${target.slice(1)}"]`).isVisible(),true);
  if(mode==='live')assert.equal(await page.evaluate(()=>document.activeElement.id),target.slice(1),'event focus return');
 }
 const failure=doc.locator('.reading-warning[data-warning="failure"]').first();
 if(await failure.count()){
  const picker=failure.locator('.notice-report-picker'),trigger=picker.locator(':scope > summary');
  assert.equal(await picker.evaluate(n=>n.open),false,'failure reports behind one closed picker');
  assert.equal(await trigger.count(),1,'one report action regardless of report count');
  const actionBox=await failure.locator('.notice-actions').boundingBox(),triggerBox=await trigger.boundingBox();
  assert.ok(Math.abs((triggerBox.x+triggerBox.width)-(actionBox.x+actionBox.width))<2,'report action right aligned');
  await trigger.press('Enter');assert.equal(await picker.evaluate(n=>n.open),true,'keyboard exposes report choices below action');
  if(c.distinct_failed_report_options){
   const labels=await picker.locator('.notice-report-options > a').allTextContents();
   assert.equal(new Set(labels).size,labels.length,'distinct original reports have distinguishable selector labels');
  }
  await failure.screenshot({path:path.join(C.output,`${c.key}-${language}-${mode}-${width}-failure-report-options.png`)});
 }
 const failed=failure.locator('.notice-report-options a[href^="#"]').first();
 if(await failed.count()){
  const target=await failed.getAttribute('href');await failed.press('Enter');if(mode==='live')await frames(page);
  assert.equal(await page.locator(`[id="${target.slice(1)}"]`).isVisible(),true,'exact failed report opens folded ancestors');
 }
 const decision=doc.locator('.decision-reading').first();
 if(await decision.count()){
  assert.equal(await decision.locator('.recorded-choice').count(),1,'canonical choice survives prose availability');
  const fold=decision.locator(':scope > details.related-decision'),summary=fold.locator(':scope > summary');
  await summary.press('Enter');assert.equal(await fold.evaluate(n=>n.open),true,'keyboard opens this Decision');
  if(await decision.getAttribute('data-decision-scope')==='work-item')assert.equal(await decision.locator('.related-decision-body > .scope-summary').count(),0,'Work association is not repeated as primary prose');
  await decision.screenshot({path:path.join(C.output,`${c.key}-${language}-${mode}-${width}-decision-explanation.png`)});
  const originals=decision.locator('details.decision-originals');
  assert.equal(await originals.evaluate(n=>n.open),false,'original reports stay folded beneath authored explanation');
  assert.equal(await decision.locator('.recorded-choice').isVisible(),false,'opening explanation does not expand raw choice reports');
  await originals.locator(':scope > summary').press('Enter');
  assert.equal(await decision.locator('.recorded-choice').isVisible(),true,'complete canonical choice is deliberately inspectable');
  await originals.locator(':scope > summary').press('Enter');
  await summary.press('Enter');assert.equal(await fold.evaluate(n=>n.open),false,'keyboard folds this Decision');
  const toc=doc.locator('.document-nav'),link=toc.locator('a').filter({hasText:await decision.locator('.decision-title').textContent()}).first();
  assert.equal(await link.count(),1,'related Decision participates in the Work outline');
  const parent=toc.locator('[data-toc-group="decisions"] > .toc-caption > a');
  assert.ok(await link.evaluate(n=>parseFloat(getComputedStyle(n).paddingLeft))>await parent.evaluate(n=>parseFloat(getComputedStyle(n).paddingLeft)),'Decision is visibly a child in the one Work outline');
  assert.equal(await fold.evaluate(n=>getComputedStyle(n).borderTopStyle),'solid','individual Decision uses the bordered child disclosure');
  assert.ok(await fold.evaluate(n=>parseFloat(getComputedStyle(n).borderTopWidth))>=1);
  if(!await toc.evaluate(n=>n.open))await toc.locator(':scope > summary').click();
  const target=await link.getAttribute('href');await link.press('Enter');if(mode==='live')await frames(page);
  assert.equal(await page.locator(`[id="${target.slice(1)}"]`).isVisible(),true);
  assert.equal(await fold.evaluate(n=>n.open),true,'outline opens the exact folded Decision, including native/offline');
  assert.equal(await decisionFolds.evaluateAll(ns=>ns.slice(1).every(n=>!n.open)),true,'other Decisions stay independently folded');
  if(mode==='live')assert.equal(await page.evaluate(()=>document.activeElement.id),target.slice(1),'Decision navigation focus');
 }
 const evidence=doc.locator('.unit > .evidence-actions > a.paragraph-evidence-action').first();
 if(await evidence.count()){
  const origin=await evidence.evaluate(n=>n.closest('.unit').id),href=await evidence.getAttribute('href');
  await evidence.evaluate(n=>{for(let p=n.parentElement;p;p=p.parentElement)if(p.matches('details'))p.open=true;});
  await evidence.press('Enter');if(mode==='live')await frames(page);
  const point=page.locator(`[id="${href.slice(1)}"]`);assert.equal(await point.isVisible(),true,'exact Source target reachable');
  const back=point.locator('a.return').first();assert.equal(await back.getAttribute('href'),'#'+origin);
  await back.press('Enter');if(mode==='live')await frames(page);
  if(mode==='live')assert.equal(await evidence.evaluate(n=>document.activeElement===n),true,'Source focus returns to the exact activating evidence action');
 }
 if(c.adverse_layout_control){
  const notice=doc.locator('.reading-warning').first();
  if(await notice.count()){
   await notice.evaluate(n=>{const p=document.createElement('p');p.textContent='command/path/identifier'.repeat(120);n.append(p);});
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+2),false,'unbroken adverse content stays inside body');
   assert.equal(await notice.evaluate(n=>n.scrollWidth<=n.clientWidth+2),true,'notice wraps complete unbroken content');
  }
 }
 result.checks.push({case:c.key,language,width,mode,request_ms:performance.now()-started,
  http_status:response?.status(),timeline_events:await timeline.locator('.work-timeline>li').count(),originals:await originals.count()});
 await page.screenshot({path:path.join(C.output,`${c.key}-${language}-${mode}-${width}.png`)});
}
(async()=>{
 const browser=await require(C.playwright).chromium.launch({executablePath:C.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:C.fontconfig}});
 try{for(const c of C.cases){
  const before=state(c,c.key+'-before'),server=await serve(c);
  try{for(const language of ['en','ko']){
   const offline=path.join(C.output,c.key+'-'+language+'.html');
   command(c.key+'-export-'+language,C.cli,['--runtime',c.runtime,'--project',c.project,'--locale',language,'viewer','export','--output',offline,'--language',language]);
   for(const mode of ['live','native','offline']){
    const context=await browser.newContext({javaScriptEnabled:mode==='live'}),page=await context.newPage();
    if(mode==='offline'){await context.setOffline(true);await context.route('**/*',r=>/^(file|data):/.test(r.request().url())?r.continue():r.abort());}
    const url=mode==='offline'?pathToFileURL(offline).href+'#work-'+c.work:`${server.address}?view=work&work=${c.work}&locale=${language}&language=${language}`;
    for(const width of [390,768,1440])await inspect(page,c,url,width,mode,language);
    await context.close();
   }
  }}finally{await server.stop();}
  assert.equal(state(c,c.key+'-after'),before,'reading does not mutate canonical/managed databases');
 }result.status='passed';}finally{await browser.close();}
})().catch(e=>{result.status='failed';result.error=e.stack;console.error(e.stack);process.exitCode=1;}).finally(()=>{
 fs.writeFileSync(path.join(C.output,'result.json'),JSON.stringify(result,null,2),{flag:'wx'});
});
