// Actual Product journeys and review packaging. No case prose or expected claims
// are authored here; all pages are served/exported by the selected executable.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync,execFileSync}=require('node:child_process');
const {pathToFileURL}=require('node:url'),crypto=require('node:crypto');
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const {chromium}=require(config.playwright);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const output=path.resolve(config.output);fs.mkdirSync(output,{recursive:true});
const result={candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
 diff_sha256:hash(execFileSync('git',['diff','HEAD','--','rebuild'])),
 executable_sha256:hash(fs.readFileSync(config.viewer)),cli_sha256:hash(fs.readFileSync(config.cli)),
 config_sha256:hash(fs.readFileSync(process.argv[2])),commands:[],cases:[],
 human_acceptance:'pending',screen_reader:'not_observed',native_zoom:'not_observed'};
function command(label,argv){const p=spawnSync(argv[0],argv.slice(1),{encoding:'utf8',timeout:120000});
 fs.writeFileSync(path.join(output,label+'.stdout'),p.stdout||'');fs.writeFileSync(path.join(output,label+'.stderr'),p.stderr||'');
 const r={label,argv,exit:p.status,signal:p.signal,error:p.error?.message};result.commands.push(r);assert.equal(p.status,0,JSON.stringify(r));return p.stdout;}
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
async function sourceJourney(page,c,width,offline=false){
 const article=page.locator('#repository-explanation .doc');
 const links=article.locator('a.evidence-link:visible');if(!await links.count())return {state:'unavailable'};
 // A reading point selected by the real document, rather than any historical case.
 const link=links.last(),target=(await link.getAttribute('href')).slice(1);
 const origin=await link.getAttribute('data-origin');await link.focus();await link.press('Enter');
 const point=page.locator(`[id="${target}"]`);await point.waitFor({state:'visible'});
 if(!offline){assert.equal(await page.evaluate(()=>document.activeElement.id),target);
  assert.equal(await point.getAttribute('role'),width<=900?'dialog':null);
  assert.equal(await article.evaluate(n=>n.inert),width<=900);
  const status=point.locator('.citation-status');if(await status.count())await status.filter({hasText:/Displayed|표시 범위/}).waitFor();
  const region=point.locator('.citation-inspection code');
  if(c.source_root && await region.count()){
   const control=point.locator('.citation-inspection'),url=new URL(await control.getAttribute('data-citation-url'),page.url());
   const response=await page.request.get(url.href);assert.equal(response.status(),200);const chunk=(await response.json()).chunk;
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
 assert.equal(new URL(page.url()).hash,'#'+returnId);return {state:'read',target,returnId};
}
(async()=>{let browser;try{
 browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
 for(const c of config.cases){const r={key:c.key,project:c.project,work:c.work,decision:c.decision,widths:[]};result.cases.push(r);
  const before=state(c.runtime);r.state_before_sha256=hash(before);
  const args=[config.cli,'--runtime',c.runtime,'--project',c.project,'--json'];
  const status=JSON.parse(command(c.key+'-status',[...args,'status','--language',c.language||'ko']));
  fs.writeFileSync(path.join(output,c.key+'-public-status.json'),JSON.stringify(status,null,2));
  const offlinePath=path.join(output,c.key+'-offline.html');
  command(c.key+'-export',[...args,'viewer','export','--output',offlinePath,'--language',c.language||'ko','--locale',c.locale||'ko']);
  const server=await serve(c);const context=await browser.newContext();const errors=[],external=[];
  context.on('request',q=>{if(new URL(q.url()).origin!==new URL(server.base).origin)external.push(q.url())});
  const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
  try {for(const width of [1440,1024,768,390,320]){
   const row={width};r.widths.push(row);await page.setViewportSize({width,height:1000});
   await page.goto(server.base+'?view=overview');assert.equal(await page.locator('#overview h1').textContent(),c.project_name);
   assert.equal(await page.locator('#overview .project-purpose[data-statement-role=verified-canonical]').count(),c.purpose_count||0);
   const tools=page.locator('.tools-menu');await tools.locator('summary').focus();await tools.locator('summary').press('Enter');
   assert.ok(await tools.evaluate(n=>n.open));await page.keyboard.press('Escape');assert.ok(!await tools.evaluate(n=>n.open));
   assert.equal(await page.evaluate(()=>document.activeElement.matches('.tools-menu>summary')),true);
   await capture(page,c,'overview',width);
   await page.locator('.repository-entry a').click();assert.equal(new URL(page.url()).searchParams.get('view'),'repository');
   fs.writeFileSync(path.join(output,`${c.key}-repository-${width}.html`),await page.content());await capture(page,c,'repository',width);
   row.source=await sourceJourney(page,c,width);
   await page.locator('.masthead .nav-link').nth(1).click();assert.equal(new URL(page.url()).searchParams.get('view'),'work');await capture(page,c,'works',width);
   if(c.work){const card=page.locator(`.work-card[data-work-id="${c.work}"]`);assert.equal(await card.locator('a').count(),1);
    await card.locator('.card-action').click();assert.equal(new URL(page.url()).searchParams.get('work'),c.work);
    assert.equal(await page.locator('.doc').getAttribute('data-work-id'),c.work);await capture(page,c,'work',width);
    await page.reload();assert.equal(await page.locator('.doc').getAttribute('data-work-id'),c.work);
    await page.locator('.breadcrumb a').click();assert.equal(new URL(page.url()).searchParams.get('view'),'work');
   }else assert.ok((await page.locator('#works').textContent()).match(/No Work|기록된 작업이 없/));
   await page.locator('.project-navigation>nav a').nth(1).click();assert.equal(new URL(page.url()).searchParams.get('view'),'decisions');
   if(c.decision){await page.locator(`.decision-summary[data-decision-id="${c.decision}"] a`).click();assert.equal(new URL(page.url()).searchParams.get('decision'),c.decision);assert.equal(await page.locator('.doc').count(),1);await capture(page,c,'decision',width);}
   else assert.ok((await page.locator('#decision-reading').textContent()).match(/No Decisions|기록된 결정이 없/));
   for(const tool of ['status','documents','memory','evidence']){
    const menu=page.locator('.tools-menu');if(!await menu.evaluate(n=>n.open))await menu.locator('summary').click();
    await menu.locator(`a[href*="tool=${tool}"]`).click();assert.equal(new URL(page.url()).searchParams.get('tool'),tool);
    assert.equal(await page.locator('.page-head h1').count(),1);await capture(page,c,tool,width);
    if(tool==='memory')assert.ok(await page.locator('form').evaluateAll(forms=>forms.every(f=>f.querySelector('input[name=request_authenticity]')?.value.length>0)),'existing mutation forms retain authenticity without granting new authority');
   }
   await page.locator('.brand').click();assert.equal(new URL(page.url()).searchParams.get('view'),'overview');
  }
  for(const selector of ['work=00000000000000000000000000000000','work=bad']){
   const response=await context.request.get(server.base+'?view=work&'+selector);assert.equal(response.status(),selector.endsWith('bad')?400:404);
  }
  const host=await context.request.get(server.base,{headers:{Host:'foreign.invalid'}});assert.equal(host.status(),421);
  const post=await context.request.post(server.base+'memory/context/correct',{headers:{Origin:'http://foreign.invalid'},data:'x=1'});assert.equal(post.status(),403);
  assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
  }finally {await context.close();r.listener=await server.stop();}
  const offline=await browser.newContext({javaScriptEnabled:false});await offline.setOffline(true);const p=await offline.newPage();
  for(const width of [1440,1024,768,390,320]){await p.setViewportSize({width,height:1000});await p.goto(pathToFileURL(offlinePath).href);
   assert.equal(await p.locator('script,form,[data-citation-url]').count(),0);
   await capture(p,c,'offline-overview',width);await p.locator('.repository-entry a').click();await capture(p,c,'offline-repository',width);await sourceJourney(p,c,width,true);
   for(const id of ['analysis-tool','documents-tool','memory-tool','evidence-tool']){await p.goto(pathToFileURL(offlinePath).href+'#'+id);assert.ok(await p.locator('#'+id).isVisible());await fits(p);}
  }
  await offline.close();assert.equal(state(c.runtime),before,'GET/export changed canonical, retention, provider or Candidate rows');r.read_state_unchanged=true;
 }
 result.outcome='passed';
 }catch(e){result.outcome='failed';result.error=e.stack;process.exitCode=1;}finally{if(browser)await browser.close();fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({outcome:result.outcome,error:result.error,output}));}
})();
