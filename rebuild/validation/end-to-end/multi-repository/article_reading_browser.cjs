// Read-only exact Product article review. No generation or semantic/human verdict.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {spawn, spawnSync, execFileSync} = require('node:child_process');
const {pathToFileURL} = require('node:url'), crypto = require('node:crypto');
const config = JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium} = require(config.playwright);
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
fs.mkdirSync(config.output, {recursive:false});
const result = {candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
  diff_sha256:digest(execFileSync('git',['diff','HEAD','--','rebuild'])),
  viewer_sha256:digest(fs.readFileSync(config.viewer)),driver_sha256:digest(fs.readFileSync(__filename)),
  semantic_approval:'not_established',human_acceptance:'pending',checks:[],processes:[]};
function command(label, binary, args) {
  const p = spawnSync(binary,args,{encoding:'utf8',timeout:60000});
  fs.writeFileSync(path.join(config.output,label+'.stdout'),p.stdout||'');
  fs.writeFileSync(path.join(config.output,label+'.stderr'),p.stderr||'');
  result.processes.push({label,binary,args,exit_code:p.status,signal:p.signal,error:p.error?.message});
  assert.equal(p.status,0,label+': '+p.stderr); return p.stdout;
}
function state(c,label) {
  return command(label,'python3',['-c',`import sqlite3,pathlib,hashlib,json,sys
r={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 with sqlite3.connect(p.resolve().as_uri()+'?mode=ro',uri=True) as c:r[p.name]=hashlib.sha256(('\\n'.join(c.iterdump())).encode()).hexdigest()
print(json.dumps(r,sort_keys=True))`,c.runtime]);
}
async function serve(c) {
  const child=spawn(config.viewer,['--runtime',c.runtime,'--project',c.project,'--bind','127.0.0.1:0']);
  let stderr='',stdout='';child.stdout.on('data',b=>stdout+=b);
  const address=await new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(Error('Viewer startup timeout')),15000);
    child.stderr.on('data',b=>{stderr+=b;const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});
    child.once('error',reject);child.once('exit',code=>{clearTimeout(timer);reject(Error('Viewer exited '+code+stderr));});
  });
  return {address,async stop(){await new Promise(resolve=>{child.once('exit',(code,signal)=>{
    fs.writeFileSync(path.join(config.output,c.key+'-viewer.stderr'),stderr);
    fs.writeFileSync(path.join(config.output,c.key+'-viewer.stdout'),stdout);
    result.processes.push({label:c.key+'-viewer',exit_code:code,signal,termination:'explicit cleanup'});resolve();});child.kill('SIGTERM');});}};
}
async function inspect(page,c,qa,url,width,mode,language) {
  const response=await page.goto(url);if(mode==='live')assert.equal(response.status(),200);
  const doc=page.locator(`[id="work-${c.work}"] .doc`).first();await doc.waitFor();
  const nav=doc.locator('.document-nav');assert.equal(await nav.count(),1);
  const primary=qa.sections.filter(s=>s.layer==='article');
  for(const s of primary.filter(s=>s.heading_index!==null)) {
    const text=qa.prose[s.heading_index].text;
    const link=nav.locator('a').filter({hasText:text});assert.equal(await link.count(),1);
    const fragment=await link.getAttribute('href');
    await link.focus();await page.keyboard.press('Enter');await page.waitForURL(u=>u.hash===fragment);
    const target=page.locator(`[id="${fragment.slice(1)}"]`);
    assert.equal(await target.locator('h2').textContent(),text);
    assert.equal(await target.evaluate(n=>document.activeElement===n),true,'authored section has native fragment focus');
    const geometry=await target.evaluate(n=>({top:n.getBoundingClientRect().top,bottom:document.querySelector('.doc .document-nav').getBoundingClientRect().bottom}));
    assert.ok(geometry.top>=geometry.bottom-2,'sticky controls do not cover heading');
    await page.reload();assert.equal(await target.locator('h2').textContent(),text,'direct fragment survives reload');
  }
  for(const role of ['Verification','Limits']) {
    const nodes=doc.locator(`[data-question="${role}"]`);
    assert.deepEqual(await nodes.allTextContents(),qa.prose.filter(u=>u.question===role).map(u=>u.text));
    if(await nodes.count())assert.equal(await nodes.first().evaluate((n,role)=>!!n.closest(role==='Limits'?'.limits-section':'.verification-report'),role),true);
  }
  assert.equal(await doc.locator('.doc-head p.lead').count(),1,'one article lead');
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no ordinary overflow');
  await page.goto(url);await page.screenshot({path:path.join(config.output,`${c.key}-${language}-${width}-${mode}.png`)});
  const evidence=doc.locator('.article-section a.evidence-link').first();
  if(await evidence.count()) {
    const origin=await evidence.getAttribute('data-origin'),fragment=await evidence.getAttribute('href');
    await evidence.focus();await page.keyboard.press('Enter');
    const point=page.locator(`[id="${fragment.slice(1)}"]`);await point.waitFor();
    if(mode==='live')await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
    const id=await point.getAttribute('data-source-id');
    const original=JSON.parse(fs.readFileSync(c.source_reads[id]));
    const detail=original.observation.detail;
    const body=Buffer.from(detail.body);
    const code=point.locator(mode==='live'?'.citation-inspection code':'pre.code-region code').first();
    const shown=Buffer.from(await code.textContent());assert.ok(shown.length>0);
    // The selected first material citation starts at the observation's first byte.
    assert.ok(body.subarray(0,shown.length).equals(shown),'exact permitted Source prefix');
    await page.screenshot({path:path.join(config.output,`${c.key}-${language}-${width}-${mode}-code.png`)});
    if(mode==='live') {
      assert.equal(await doc.evaluate(n=>n.inert),width<=900);
      assert.equal(await point.getAttribute('aria-modal'),width<=900?'true':null);
      await page.keyboard.press('Escape');
    } else {await point.locator('.return').focus();await page.keyboard.press('Enter');}
    await page.waitForURL(u=>u.hash==='#'+origin);
    assert.equal(await page.evaluate(()=>document.activeElement.id),origin,'exact claim return');
    await page.goBack();assert.equal(new URL(page.url()).hash,fragment);
    await page.goForward();assert.equal(new URL(page.url()).hash,'#'+origin);
  }
  if(mode==='offline')assert.equal(await page.locator('script,form').count(),0);
  const ids=await page.locator('[id]').evaluateAll(ns=>ns.map(n=>n.id));assert.equal(ids.length,new Set(ids).size);
  result.checks.push({case:c.key,language,width,mode,authored_sections:primary.filter(s=>s.heading_index!==null).length,status:'passed'});
}
(async()=>{
  const browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
  result.browser_version=browser.version();
  try {
    for(const c of config.cases) {
      const before=state(c,c.key+'-state-before');const live=await serve(c);
      try {for(const language of ['en','ko']) {
        const raw=command(c.key+'-status-'+language,config.cli,['--runtime',c.runtime,'--project',c.project,'--json','status','--language',language]);
        const status=JSON.parse(raw);const qa=status.work_history.find(w=>w.work_item_id===c.work).answers;
        assert.equal(qa.explanation_state,'current');
        const snapshot=path.join(config.output,c.key+'-'+language+'.html');
        command(c.key+'-export-'+language,config.viewer,['--runtime',c.runtime,'--project',c.project,'--language',language,'--locale',language,'--snapshot',snapshot]);
        for(const width of [1440,1024,768,390,320])for(const mode of ['live','offline']) {
          const ctx=await browser.newContext({viewport:{width,height:900},javaScriptEnabled:mode==='live'});
          try {const page=await ctx.newPage();await inspect(page,c,qa,mode==='live'?`${live.address}?view=work&work=${c.work}&locale=${language}&language=${language}`:pathToFileURL(snapshot).href+'#work-'+c.work,width,mode,language);}
          finally {await ctx.close();}
        }
      }} finally {await live.stop();}
      assert.equal(state(c,c.key+'-state-after'),before,'GET/navigation/Source/export leave logical stores unchanged');
    }
    result.status='passed';
  } finally {await browser.close();}
})().catch(error=>{result.status='failed';result.error=error.stack;process.exitCode=1;}).finally(()=>{
  fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify(result,null,2));
  console.log(JSON.stringify({status:result.status,checks:result.checks.length,error:result.error}));
});
