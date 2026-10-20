// Exact current Product pages and whole offline documents. No generation or human verdict.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {spawn, spawnSync, execFileSync} = require('node:child_process');
const {pathToFileURL} = require('node:url'), crypto = require('node:crypto');
const c = JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium} = require(c.playwright);
fs.mkdirSync(c.output, {recursive:false});
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const result = {candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
  diff_sha256:hash(execFileSync('git',['diff','HEAD','--','rebuild'])), viewer_sha256:hash(fs.readFileSync(c.viewer)),driver_sha256:hash(fs.readFileSync(__filename)),
  semantic_approval:'not_established',human_acceptance:'unassessed',screen_reader:'not_observed',native_zoom:'not_observed',checks:[],processes:[]};
function command(label, bin, args) {
  const p=spawnSync(bin,args,{encoding:'utf8',timeout:60000});
  fs.writeFileSync(path.join(c.output,label+'.stdout'),p.stdout||''); fs.writeFileSync(path.join(c.output,label+'.stderr'),p.stderr||'');
  result.processes.push({label,argv:[bin,...args],exit:p.status,signal:p.signal});
  assert.equal(p.status,0,p.stderr); return p.stdout;
}
function state(x,label) {return command(label,'python3',['-c',`import sqlite3,pathlib,hashlib,json,sys
r={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 with sqlite3.connect(p.resolve().as_uri()+'?mode=ro',uri=True) as db:r[p.name]=hashlib.sha256(('\\n'.join(db.iterdump())).encode()).hexdigest()
print(json.dumps(r,sort_keys=True))`,x.runtime]);}
async function serve(x) {
 const child=spawn(c.viewer,['--runtime',x.runtime,'--project',x.project,'--bind','127.0.0.1:0']);let err='',out='';child.stdout.on('data',b=>out+=b);
 const base=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('listener timeout')),15000);child.stderr.on('data',b=>{err+=b;const m=err.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});child.on('error',reject);child.on('exit',()=>{clearTimeout(timer);reject(Error(err));});});
 return {base,async stop(){await new Promise(resolve=>{child.once('exit',(code,signal)=>{fs.writeFileSync(path.join(c.output,x.key+'-viewer.stderr'),err);fs.writeFileSync(path.join(c.output,x.key+'-viewer.stdout'),out);result.processes.push({label:x.key+'-viewer',exit:code,signal,termination:'explicit cleanup'});resolve();});child.kill();});}};
}
async function fragments(page) {
 const errors=await page.evaluate(()=>{const counts=new Map();for(const n of document.querySelectorAll('[id]'))counts.set(n.id,(counts.get(n.id)||0)+1);
 const errors=[...counts].filter(([,v])=>v!==1).map(([id,n])=>`${id}: ${n} targets`);
 for(const a of document.querySelectorAll('a[href^="#"]'))if(counts.get(a.hash.slice(1))!==1)errors.push(`bad link ${a.getAttribute('href')}`);
 for(const n of document.querySelectorAll('[data-origin]'))if(counts.get(n.dataset.origin)!==1)errors.push(`bad return ${n.dataset.origin}`);
 return errors;});assert.deepEqual(errors,[],'every internal link and return has exactly one target');
}
(async()=>{const browser=await chromium.launch({executablePath:c.chromium,headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});
 result.browser_version=browser.version();
 try {for(const x of c.cases){const before=state(x,x.key+'-before');const server=await serve(x);
 try {
 const typed=x.work?JSON.parse(command(x.key+'-verification',c.cli,['--runtime',x.runtime,'--project',x.project,'--json','work','verification','--work',x.work])).verification_report:null;
 const snapshot=path.join(c.output,x.key+'.html');command(x.key+'-snapshot',c.viewer,['--runtime',x.runtime,'--project',x.project,'--locale',x.locale,'--language',x.language,'--snapshot',snapshot]);
 for(const width of (c.fragments_only?[1440,390]:[1440,1024,768,390,320]))for(const mode of ['live','offline','live-no-js']){
 const js=mode==='live';const ctx=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:js});
 await ctx.route('**/*',r=>{const u=new URL(r.request().url());return u.protocol==='file:'||u.origin===new URL(server.base).origin?r.continue():r.abort();});
 try{const page=await ctx.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const url=mode==='offline'?pathToFileURL(snapshot).href+(x.work?'#work-'+x.work:'#repository-explanation'):server.base+`?view=${x.work?'work&work='+x.work:'repository'}&locale=${x.locale}&language=${x.language}`;
 await page.goto(url);await fragments(page);
 if(!c.fragments_only){
 const doc=page.locator('.doc:visible').first(),toc=doc.locator('.document-nav');assert.equal(await toc.count(),1);
 assert.equal(await toc.evaluate(n=>n.tagName),'DETAILS');assert.equal(await toc.getAttribute('open'),null);
 assert.equal(await toc.locator('a:visible').count(),0);
 const authored=await doc.locator('.article-section h2[data-question=SectionHeading]').allTextContents();
 assert.deepEqual(await toc.locator('.toc-primary a').allTextContents(),authored);
 const summary=toc.locator(':scope > summary');await summary.focus();await page.keyboard.press('Space');assert.equal(await toc.evaluate(n=>n.open),true);
 await page.keyboard.press('Tab');assert.ok(await toc.evaluate(n=>n.contains(document.activeElement)));
 await page.keyboard.press('Shift+Tab');assert.equal(await summary.evaluate(n=>n===document.activeElement),true);
 if(js){await page.keyboard.press('Escape');assert.equal(await toc.evaluate(n=>n.open),false);await summary.click();}
 const links=await toc.locator('a').evaluateAll(ns=>ns.map(a=>a.getAttribute('href')));
 for(const fragment of links){if(!await toc.evaluate(n=>n.open))await summary.click();
 const a=toc.locator(`a[href="${fragment}"]`);if(await a.locator('xpath=ancestor::details[contains(@class,"toc-more")]').count() && !await toc.locator('.toc-more').evaluate(n=>n.open))await toc.locator('.toc-more > summary').click();await a.focus();await page.keyboard.press('Enter');
 const target=page.locator(`[id="${fragment.slice(1)}"]`);await target.waitFor({state:'visible'});
 // Without JS the native disclosure remains open in flow; it never overlays text.
 const overlap=await target.evaluate(n=>{const t=n.getBoundingClientRect(),nav=n.closest('.doc').querySelector('.document-nav').getBoundingClientRect();return nav.top<t.bottom&&nav.bottom>t.top&&getComputedStyle(n.closest('.doc').querySelector('.document-nav')).position==='sticky';});
 assert.equal(overlap,false,'expanded TOC cannot cover selected reading text');
 if(js){assert.equal(await toc.evaluate(n=>n.open),false);
  await page.waitForFunction(id=>document.activeElement?.id===id,fragment.slice(1));
  assert.equal(await target.evaluate(n=>document.activeElement===n),true,'selected section restores keyboard reading focus');
  const current=toc.locator('.toc-primary [aria-current="location"]');
  assert.ok(await current.count()<=1,'at most one current section');
  if(await current.count())assert.equal(await toc.locator('.toc-cue').textContent(),await current.textContent());
 }else assert.equal(await target.evaluate(n=>document.activeElement===n),true,'native script-free fragment focus');
 }
 await page.goto(url);
 // A same-file offline fragment change retains disclosures deliberately
 // opened above. Check initial defaults on a fresh document, not that history.
 await page.reload();
 const supplemental=doc.locator('details.verification-report,details.evidence-index,.audit > details');
 assert.ok(await supplemental.count()>0);assert.equal(await supplemental.evaluateAll(ns=>ns.every(n=>!n.open)),true);
 assert.equal(await doc.locator('.verification-execution > header h3:visible').count(),0,'command labels are secondary');
 if(typed){
  const report=doc.locator('.verification-report');await report.locator(':scope > summary').click();
  assert.deepEqual(await report.locator('.verification-execution > header h3').allTextContents(),typed.executions.slice(0,32).map(e=>e.command_label));
  for(const execution of typed.executions.slice(0,32)){
   const row=report.locator(`[data-execution-source="${execution.source_id}"]`);assert.equal(await row.count(),1);
   const numeric=await row.locator(':scope > dl').first().locator('dd').nth(1).textContent();
   if(execution.exit_code!==null)assert.equal(numeric,String(execution.exit_code));
  }
  await page.goto(url);
 }
 if(x.expected_failure!==undefined){
  assert.equal(await doc.locator('[data-warning="failure"]:visible').count()>0,x.expected_failure);
  if(x.expected_failure){const a=doc.locator('.failure-check-links a').first(),id=(await a.getAttribute('href')).slice(1);
   await a.focus();await page.keyboard.press('Enter');const check=page.locator(`[id="${id}"]`);await check.waitFor({state:'visible'});
   assert.equal(await check.evaluate(n=>n.classList.contains('verification-check')),true);
   assert.ok((await check.textContent()).match(/failed|실패/));await page.goto(url);
  }
 }
 const evidence=doc.locator('.unit a.evidence-link:visible,.doc-head a.evidence-link:visible').first();
 if(await evidence.count()){
 assert.equal(await doc.locator('.evidence-index').evaluate(n=>n.open),false);
 const origin=await evidence.getAttribute('data-origin'),fragment=await evidence.getAttribute('href');await evidence.focus();await page.keyboard.press('Enter');
 const point=page.locator(`[id="${fragment.slice(1)}"]`);await point.waitFor({state:'visible'});
 if(js){await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();await page.keyboard.press('Escape');}
 else {await point.locator('.return').focus();await page.keyboard.press('Enter');}
 await page.waitForURL(u=>u.hash==='#'+origin);await page.goBack();assert.equal(new URL(page.url()).hash,fragment);await page.reload();await point.waitFor({state:'visible'});await page.goForward();
 }
 for(const article of await page.locator('.doc').all()) {
  if(!await article.isVisible())continue;
  const details=article.locator('.verification-report,.evidence-index');assert.equal(await details.evaluateAll(ns=>ns.every(n=>n.tagName==='DETAILS')),true);
 }
 await page.goto(url);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no horizontal page overflow');
 await page.screenshot({path:path.join(c.output,`${x.key}-${width}-${mode}.png`)});
 await page.emulateMedia({media:'print'});assert.equal(await toc.isVisible(),false);await page.emulateMedia({media:'screen'});
 if(js){await page.goto(url+'#reader-retired-fragment');await page.locator('.fragment-unavailable').waitFor({state:'visible'});}
 }
 if(mode==='offline'){assert.equal(await page.locator('script,form').count(),0);if(!c.fragments_only){await page.goto(pathToFileURL(snapshot).href+'#reader-retired-fragment');await page.locator('.snapshot-link-help').waitFor({state:'visible'});assert.equal(await page.locator('.doc:visible').count(),0);}}
 assert.deepEqual(errors,[]);result.checks.push({case:x.key,width,mode,status:'passed'});
 }finally{await ctx.close();}}
 }finally{await server.stop();}
 assert.equal(state(x,x.key+'-after'),before,'reading/export must not mutate logical stores');
 }
 if(c.outline_controls)for(const count of [0,1,12])for(const width of [1440,1024,768,390,320]) {
  const page=await browser.newPage({viewport:{width,height:1000},javaScriptEnabled:false});
  try {await page.goto(pathToFileURL(path.join(c.outline_controls,`outline-${count}.html`)).href);const toc=page.locator('.document-nav');
   await toc.locator(':scope > summary').focus();await page.keyboard.press('Enter');assert.equal(await toc.evaluate(n=>n.open),true);
   assert.equal(await toc.locator('.toc-primary a:visible').count(),count);
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
   for(const i of count?[0,count-1]:[]){const a=toc.locator('.toc-primary a').nth(i);await a.focus();await page.keyboard.press('Enter');
    const id=(await a.getAttribute('href')).slice(1);assert.equal(await page.locator(`[id="${id}"]`).count(),1);
    assert.equal(await page.evaluate(id=>document.activeElement.id===id,id),true);
   }
   result.checks.push({case:'authored-outline-control',count,width,mode:'script-free',status:'passed'});
  }finally{await page.close();}
 }
 result.status='passed';}finally{await browser.close();}
})().catch(e=>{result.status='failed';result.error=e.stack;process.exitCode=1;}).finally(()=>{fs.writeFileSync(path.join(c.output,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({status:result.status,checks:result.checks.length,error:result.error}));});
