// Actual Product journeys and review packaging. No case prose or expected claims
// are authored here; all pages are served/exported by the selected executable.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync,execFileSync}=require('node:child_process');
const {pathToFileURL}=require('node:url'),crypto=require('node:crypto');

function reviewedFiles(directory){const files={};for(const name of fs.readdirSync(directory).sort()){const file=path.join(directory,name);if(!fs.lstatSync(file).isFile()||!['.png','.html'].includes(path.extname(name)))continue;const bytes=fs.readFileSync(file);files[name]={sha256:require('node:crypto').createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length};}return files;}
const {createOutput,publication,documentMetrics}=require('./product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const {chromium}=require(config.playwright);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const output=path.resolve(config.output);createOutput(output);
const result={candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
 diff_sha256:hash(execFileSync('git',['diff','HEAD','--','rebuild'])),
 executable_sha256:hash(fs.readFileSync(config.viewer)),cli_sha256:hash(fs.readFileSync(config.cli)),
 controls_sha256:hash(fs.readFileSync(path.join(__dirname,'product_review_controls.cjs'))),
 config_sha256:hash(fs.readFileSync(process.argv[2])),driver_sha256:hash(fs.readFileSync(__filename)),commands:[],cases:[],
 human_acceptance:'pending',screen_reader:'not_observed',native_zoom:'not_observed'};
function command(label,argv){const started=performance.now();const p=spawnSync(argv[0],argv.slice(1),{encoding:'utf8',timeout:120000,maxBuffer:16*1024*1024,env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
 fs.writeFileSync(path.join(output,label+'.stdout'),p.stdout||'');fs.writeFileSync(path.join(output,label+'.stderr'),p.stderr||'');
 const r={label,argv,exit:p.status,signal:p.signal,error:p.error?.message,duration_ms:performance.now()-started,stdout_bytes:Buffer.byteLength(p.stdout||''),stderr_bytes:Buffer.byteLength(p.stderr||'')};result.commands.push(r);assert.equal(p.status,0,JSON.stringify(r));return p.stdout;}
function state(runtime){return execFileSync('python3',['-c',`import sqlite3,json,hashlib,sys,pathlib
r={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 c=sqlite3.connect('file:'+str(p)+'?mode=ro',uri=True)
 for (t,) in c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"):
  rows=c.execute('SELECT * FROM "'+t.replace('"','""')+'"').fetchall()
  r[p.name+':'+t]=hashlib.sha256(json.dumps(sorted(map(repr,rows))).encode()).hexdigest()
print(json.dumps(r,sort_keys=True))`,runtime],{encoding:'utf8'});}
async function serve(c){const p=spawn(config.viewer,['--runtime',c.runtime,'--project',c.project,'--locale',c.locale||'ko','--language',c.language||'ko','--bind','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
 let stdout='',stderr='';p.stdout.on('data',b=>stdout+=b);p.stderr.on('data',b=>stderr+=b);
 const exited=new Promise(resolve=>p.on('exit',(code,signal)=>resolve({code,signal})));
 const base=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error(stderr||'listener timeout')),20000);p.stderr.on('data',()=>{const m=stderr.match(/http:\/\/127.0.0.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0])}});p.on('exit',()=>{clearTimeout(timer);reject(Error(stderr))})});
 return {base,async stop(){p.kill('SIGTERM');const termination=await exited;fs.writeFileSync(path.join(output,c.key+'-viewer.stdout'),stdout);fs.writeFileSync(path.join(output,c.key+'-viewer.stderr'),stderr);return termination;}};}
async function fits(page){assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),JSON.stringify(await page.evaluate(()=>({url:location.href,width:innerWidth,scroll:document.documentElement.scrollWidth,overflow:[...document.querySelectorAll('main *')].filter(n=>n.getBoundingClientRect().right>innerWidth+1).slice(0,8).map(n=>({tag:n.tagName,cls:n.className,width:n.getBoundingClientRect().width}))}))));}
async function capture(page,c,name,width){await fits(page);await page.screenshot({path:path.join(output,`${c.key}-${name}-${width}.png`)});}
async function sourceJourney(page,c,width,offline=false,selector='#repository-explanation .doc'){
 let verificationRead=null;
 const article=page.locator(selector);
 const links=article.locator('a.evidence-link:visible');if(!await links.count())return {state:'unavailable'};
 // A reading point selected by the real document, rather than any historical case.
 const link=links.last(),target=(await link.getAttribute('href')).slice(1);
 const origin=await link.getAttribute('data-origin');await link.focus();await link.press('Enter');
 const point=page.locator(`[id="${target}"]`);await point.waitFor({state:'visible'});
 if(!offline){await page.waitForFunction(({id,narrow})=>{const n=document.getElementById(id);return n&&document.activeElement===n&&n.getAttribute('role')===(narrow?'dialog':null);},{id:target,narrow:width<=900});
  assert.equal(await page.evaluate(()=>document.activeElement.id),target);
  assert.equal(await point.getAttribute('role'),width<=900?'dialog':null);
  assert.equal(await article.evaluate(n=>n.inert),width<=900);
  assert.equal(await point.evaluate(n=>Boolean(n.closest('[inert]'))),false,'Source controls must remain outside the inert article');
  await page.keyboard.press('Tab');
  assert.equal(await point.evaluate(n=>n.contains(document.activeElement)),true,'Source keyboard controls remain interactive');
  const status=point.locator('.citation-status');if(await status.count())await status.filter({hasText:/Displayed|표시 범위/}).waitFor();
  const region=point.locator('.citation-inspection code');
  if(c.source_root && await region.count()){
   const control=point.locator('.citation-inspection'),url=new URL(await control.getAttribute('data-citation-url'),page.url());
   const started=performance.now(),response=await page.request.get(url.href);assert.equal(response.status(),200);const bytes=await response.body();verificationRead={status:response.status(),bytes:bytes.length,duration_ms:performance.now()-started};const chunk=JSON.parse(bytes).chunk;
   const source=fs.readFileSync(path.join(c.source_root,chunk.citation.locator));
   assert.equal(await region.textContent(),source.subarray(chunk.displayed_start_byte,chunk.displayed_end_byte).toString('utf8'));
  }
 }
 assert.equal(await point.locator('.code-region').last().evaluate(n=>getComputedStyle(n).fontSize),width<=600?'12px':width<=900?'13px':'15px');
 await capture(page,c,offline?'offline-source':'source',width);
 const returnId=(await point.locator('.return').getAttribute('href')).slice(1);assert.equal(returnId,origin);
 if(!offline){await page.reload();await point.waitFor({state:'visible'});await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>document.activeElement.id),returnId);
  await page.goBack();await point.waitFor({state:'visible'});await page.goForward();assert.equal(new URL(page.url()).hash,'#'+returnId);
 }else await point.locator('.return').click();
 assert.equal(new URL(page.url()).hash,'#'+returnId);return {state:'read',target,returnId,verification_read:verificationRead};
}
(async()=>{let browser;try{
 if(config.fontconfig)assert.ok(fs.existsSync(config.fontconfig),'fontconfig file must exist');
 assert.ok(command('korean-fonts',['fc-list',':lang=ko']).trim(),'Korean-capable fonts are required for readable review screenshots');
 browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
 result.browser_version=browser.version();
 for(const c of config.cases){const r={key:c.key,project:c.project,work:c.work,decision:c.decision,evidence_class:c.evidence_class||'unclassified',origin:c.origin||'not supplied; no authorship claim',widths:[]};result.cases.push(r);
  const before=state(c.runtime);r.state_before_sha256=hash(before);
  let originalEvents;
  if(c.source_bundle && c.work){
   const bundle=JSON.parse(fs.readFileSync(c.source_bundle,'utf8'));
   const rows=name=>{const table=bundle.payload.tables.find(t=>t.name===name);return table.rows.map(row=>Object.fromEntries(table.columns.map((key,n)=>[key,row[n].value])));};
   const goal=rows('context_item_revisions').find(v=>v.context_item_id===c.work&&v.revision===1);
   const records=rows('checkpoints').filter(v=>v.work_item_id===c.work).sort((a,b)=>a.recorded_at-b.recorded_at||a.id.localeCompare(b.id));
   originalEvents={creation:goal,latest:records.at(-1)};r.event_oracle='original portable canonical record/revision timestamps';
  }
  const args=[config.cli,'--runtime',c.runtime,'--project',c.project,'--json'];
  const status=JSON.parse(command(c.key+'-status',[...args,'status','--language',c.language||'ko']));
  r.publication=publication(c,status);
  fs.writeFileSync(path.join(output,c.key+'-public-status.json'),JSON.stringify(status,null,2));
  const offlinePath=path.join(output,c.key+'-offline.html');
  command(c.key+'-export',[...args,'viewer','export','--output',offlinePath,'--language',c.language||'ko','--locale',c.locale||'ko']);
  r.http_reads=[];r.source_reads=[];const sourceReads=[];
  const server=await serve(c);const context=await browser.newContext();const errors=[],external=[];
  context.on('request',q=>{if(new URL(q.url()).origin!==new URL(server.base).origin)external.push(q.url())});
  context.on('response',response=>{if(new URL(response.url()).pathname==='/citation')sourceReads.push((async()=>{try {const body=await response.body();r.source_reads.push({status:response.status(),bytes:body.length});}catch {r.source_reads.push({status:response.status(),state:'body_unavailable'});}})());});
  const page=await context.newPage();const cpu=await context.newCDPSession(page);await cpu.send('Performance.enable');
  async function taskCpu(){return (await cpu.send('Performance.getMetrics')).metrics.find(m=>m.name==='TaskDuration').value*1000;}
  page.on('pageerror',e=>errors.push(e.message));
  try {
   for(const view of ['overview','work','repository'])for(let attempt=0;attempt<3;attempt++){
    const started=performance.now(),response=await context.request.get(server.base+'?view='+view);
    const body=await response.body();assert.equal(response.status(),200);
    r.http_reads.push({view,attempt,status:response.status(),duration_ms:performance.now()-started,bytes:body.length});
   }
   for(const width of [1440,1024,768,390,320]){
   const row={width};r.widths.push(row);await page.setViewportSize({width,height:1000});
   await page.goto(server.base+'?view=overview');assert.equal(await page.locator('#overview h1').textContent(),c.project_name);
   assert.equal(await page.locator('#overview .project-purpose[data-statement-role=verified-canonical]').count(),c.purpose_count||0);
   assert.equal(await page.locator('.project-navigation,.masthead .surface-status,.masthead .trial').count(),0);
   assert.equal(await page.locator('.masthead nav').count(),1);
   assert.equal(await page.locator('#overview .repository-entry,#overview .related-decisions').count(),0);
   assert.ok(!(await page.locator('#overview').textContent()).includes('RECENT WORK'));
   const geometry=await page.evaluate(()=>{const brand=document.querySelector('.brand').getBoundingClientRect(),nav=document.querySelector('.masthead nav').getBoundingClientRect(),shell=document.querySelector('.masthead>.shell').getBoundingClientRect();return {brand:brand.x,nav:nav.x,right:nav.right,shellRight:shell.right};});
   assert.ok(Math.abs(geometry.right-(width-(width>1440?(width-1440)/2:0)-(width<=600?20:width<=900?28:48)))<2,'right aligned navigation');
   if(width>=768)assert.ok(geometry.nav>geometry.brand+100);
   row.navigation_geometry=geometry;
   if(c.normal_work){const card=page.locator(`.work-card[data-work-id="${c.normal_work}"]`);
    assert.equal(await card.locator('h3').getAttribute('data-title-basis'),'generated-interpretation');
    assert.equal(await card.locator('blockquote').count(),0);assert.equal(await card.locator('[data-question="Synopsis"]').count(),1);
    assert.equal(await card.locator('time[data-event-kind="goal_created"]').count(),1);
    const canonicalWork=status.work_history.find(w=>w.work_item_id===c.normal_work);
    const hasCheckpoint=originalEvents?Boolean(originalEvents.latest):canonicalWork.checkpoint_ids.length>0;
    assert.equal(await card.locator('time[data-event-kind="checkpoint_recorded"]').count(),hasCheckpoint?1:0,'no Checkpoint means no fabricated latest-work instant');
    assert.ok((await card.locator('h3').textContent()).length<160);
    if(originalEvents){
     const creation=card.locator('time[data-event-kind="goal_created"]'),latest=card.locator('time[data-event-kind="checkpoint_recorded"]');
     assert.equal(await creation.getAttribute('data-recorded-at'),String(originalEvents.creation.recorded_at));
     assert.equal(await creation.getAttribute('data-revision'),'1');
     assert.equal(await creation.getAttribute('data-record-id'),c.work);
     row.creation_event={record:c.work,revision:1,micros:originalEvents.creation.recorded_at};
     if(originalEvents.latest){
      assert.equal(await latest.getAttribute('data-recorded-at'),String(originalEvents.latest.recorded_at));
      assert.equal(await latest.getAttribute('data-record-id'),originalEvents.latest.id);
      row.latest_work_event={record:originalEvents.latest.id,revision:originalEvents.latest.revision,micros:originalEvents.latest.recorded_at};
     }else row.latest_work_event='absent: no original Checkpoint';
    }
    row.reading_title=await card.locator('h3').textContent();row.synopsis=await card.locator('[data-question="Synopsis"]').textContent();
    row.normal_content='current public name and synopsis; author identity and semantic quality are not established by this browser';
   }
   if(c.unavailable_work)assert.equal(await page.locator(`.work-card[data-work-id="${c.unavailable_work}"] .content-unavailable`).count(),1);
   assert.equal(await page.locator('.tools-menu,.inspection-query').count(),0);
   assert.equal(await page.locator('.masthead nav a').count(),3);
   await capture(page,c,'overview',width);
   const clickStarted=performance.now(),cpuBefore=await taskCpu();
   await page.locator('.masthead a[href*="view=repository"]').click();
   await page.waitForURL(u=>u.searchParams.get('view')==='repository');
   await page.waitForLoadState('domcontentloaded');
   await page.locator('#repository-explanation').waitFor();
   row.repository_cost={navigation_ms:performance.now()-clickStarted,browser_task_ms:(await taskCpu())-cpuBefore,...await page.evaluate(documentMetrics)};assert.equal(new URL(page.url()).searchParams.get('view'),'repository');
   fs.writeFileSync(path.join(output,`${c.key}-repository-${width}.html`),await page.content());await capture(page,c,'repository',width);
   row.source=await sourceJourney(page,c,width);
   await page.locator('.masthead a[href*="view=work"]').click();assert.equal(new URL(page.url()).searchParams.get('view'),'work');await capture(page,c,'works',width);
   if(c.work){const card=page.locator(`.work-card[data-work-id="${c.work}"]`);assert.equal(await card.locator('a').count(),1);
    await card.locator('.card-action').click();assert.equal(new URL(page.url()).searchParams.get('work'),c.work);
    assert.equal(await page.locator('.doc[data-work-id]').getAttribute('data-work-id'),c.work);
    if(c.normal_work){
     // Name, summary and article are independently admitted publications. A
     // card cannot supply the expected title/lead for a different realization.
     const answers=status.work_history.find(w=>w.work_item_id===c.work).answers;
     if(answers.explanation_state==='current'){
      assert.equal(await page.locator('.doc[data-work-id] h1').textContent(),answers.prose.find(p=>p.question==='Heading').text);
      assert.equal(await page.locator('.doc[data-work-id] .lead[data-question="Synopsis"]').textContent(),answers.prose.find(p=>p.question==='Synopsis').text);
     }else assert.ok(answers.prose.every(p=>p.role!=='generated_interpretation'),'unavailable article supplies only lifecycle notices');
    }
    const sectionNav=page.locator('.doc[data-work-id] .document-nav');assert.equal(await sectionNav.count(),1);
    assert.equal(await page.locator('.audit-links,.opening-limits,.limit-shortcut').count(),0);
    const workReport=status.work_history.find(w=>w.work_item_id===c.work).verification_report;
    assert.equal(await page.locator('.verification-report .verification-execution').count(),Math.min(32,workReport.executions.length));
    assert.equal(await page.locator('.verification-report .verification-check').count(),Math.min(64,workReport.checks.length));
    for(const run of workReport.executions.slice(0,32)){
     const execution=page.locator(`[data-execution-source="${run.source_id}"]`);
     assert.equal(await execution.locator('h3').textContent(),run.command_label);
     assert.ok(!(await execution.textContent()).includes('Some('),'numeric facts do not use Rust debug option syntax');
    }
    for(const id of await sectionNav.locator('a').evaluateAll(a=>a.map(n=>n.hash.slice(1))))assert.equal(await page.locator(`[id="${id}"]`).count(),1);
    await capture(page,c,'work',width);
    row.work_source=await sourceJourney(page,c,width,false,'.doc[data-work-id]');
    await page.reload();assert.equal(await page.locator('.doc[data-work-id]').getAttribute('data-work-id'),c.work);
    await page.locator('.doc[data-work-id] .breadcrumb a').click();assert.equal(new URL(page.url()).searchParams.get('view'),'work');
   }else assert.ok((await page.locator('#works').textContent()).match(/No Work|기록된 작업이 없/));
   if(c.work && c.decision){
    await page.goto(server.base+`?view=work&work=${c.work}`);
    assert.equal(await page.locator(`.decision-reading[data-decision-id="${c.decision}"]`).count(),1);
    await capture(page,c,'contextual-decision',width);
   }
   await page.locator('.brand').click();assert.equal(new URL(page.url()).searchParams.get('view'),'overview');
  }
  for(const selector of ['work=00000000000000000000000000000000','work=bad']){
   const response=await context.request.get(server.base+'?view=work&'+selector);assert.equal(response.status(),selector.endsWith('bad')?400:404);
  }
  for(const query of ['?view=tools','?view=decisions','?view=code&scope=repository'])assert.equal((await context.request.get(server.base+query)).status(),404);
  for(const route of ['inspection','inspection.json'])assert.equal((await context.request.get(server.base+route)).status(),404);
  for(const route of ['memory/context/correct','memory/decision/correct','memory/decision/supersede','memory/forget','documents/export'])assert.equal((await context.request.post(server.base+route,{data:'x=1'})).status(),404);
  const host=await context.request.get(server.base,{headers:{Host:'foreign.invalid'}});assert.equal(host.status(),421);
  const post=await context.request.post(server.base+'memory/context/correct',{headers:{Origin:'http://foreign.invalid'},data:'x=1'});assert.equal(post.status(),404);
  assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
  }finally {await Promise.all(sourceReads);await context.close();r.listener=await server.stop();}
  const offline=await browser.newContext({javaScriptEnabled:false});await offline.setOffline(true);const p=await offline.newPage();
  for(const width of [1440,1024,768,390,320]){await p.setViewportSize({width,height:1000});await p.goto(pathToFileURL(offlinePath).href);
   assert.equal(await p.locator('script,form,[data-citation-url]').count(),0);
   assert.equal(await p.locator('#overview h1').textContent(),c.project_name);
   assert.equal(await p.locator('.masthead nav').count(),1);
   assert.equal(await p.locator('#overview .repository-entry,#overview .related-decisions').count(),0);
   await capture(p,c,'offline-overview',width);
   if(c.work){await p.locator('.masthead a[href="#works"]').click();await p.locator(`#works .work-card[data-work-id="${c.work}"] .card-action`).click();
    assert.equal(await p.locator(`article.doc[data-work-id="${c.work}"]:visible`).count(),1);
    await capture(p,c,'offline-work',width);}
   await p.locator('.masthead a[href="#repository-explanation"]').click();await capture(p,c,'offline-repository',width);await sourceJourney(p,c,width,true);
   for(const id of ['analysis-tool','documents-tool','memory-tool','evidence-tool','code','decision-reading','repository-code'])assert.equal(await p.locator('#'+id).count(),0);
   assert.equal(await p.locator('.masthead nav a').count(),3);
  }
  await offline.close();assert.equal(state(c.runtime),before,'GET/export changed canonical, retention, provider or Candidate rows');r.read_state_unchanged=true;
 }
 result.outcome='passed';
 }catch(e){result.outcome='failed';result.error=e.stack;process.exitCode=1;}finally{if(browser)await browser.close();result.files=reviewedFiles(output);fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({outcome:result.outcome,error:result.error,output}));}
})();
