// Browser controls compare frozen public answers and original bytes, never DOM-derived truth.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process');
const {pathToFileURL}=require('node:url'),crypto=require('node:crypto');
const c=JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium}=require(c.playwright),build=path.resolve(c.build),out=path.resolve(c.output);
fs.mkdirSync(out,{recursive:false});
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const manifest=JSON.parse(fs.readFileSync(path.join(build,'manifest.json')));
const authored=JSON.parse(fs.readFileSync(path.join(build,'inputs/review-input-final.json')));
const catalog=JSON.parse(fs.readFileSync(path.join(build,'inputs/catalog.json')));
const catalogBundle=JSON.parse(fs.readFileSync(path.join(build,'inputs/catalog-bundle.json')));
const records=name=>catalogBundle.payload.tables.find(t=>t.name===name);
const rows=name=>records(name).rows.map(row=>Object.fromEntries(records(name).columns.map((k,i)=>[k,row[i].value])));
const fixture=JSON.parse(fs.readFileSync(path.join(build,'inputs/fixtures.json')));
const result={status:'running',head:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),driver_sha256:hash(fs.readFileSync(__filename)),manifest_sha256:hash(fs.readFileSync(path.join(build,'manifest.json'))),checks:[],requests:[],processes:[],human_acceptance:'pending'};
function identity(){for(const [f,d] of Object.entries(manifest.files))assert.equal(hash(fs.readFileSync(path.join(build,f))),d.sha256,'package identity '+f);}
async function start(){
 const proc=spawn('python3',['-B',path.join(build,'serve.py')]);let stdout='',stderr='';
 const url=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('startup deadline')),10000);proc.stdout.on('data',b=>{stdout+=b;const m=stdout.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});proc.stderr.on('data',b=>stderr+=b);proc.once('error',reject);proc.once('exit',code=>{clearTimeout(timer);reject(Error('startup exit '+code+stderr));});});
 return {url,proc,async stop(){await new Promise(resolve=>{proc.once('exit',(code,signal)=>{result.processes.push({pid:proc.pid,code,signal,termination:'explicit SIGTERM cleanup',stdout,stderr});resolve();});proc.kill('SIGTERM');});}};
}
async function paint(page){await page.evaluate(()=>document.fonts.ready);}
function expected(work){const control=fixture.cases.find(f=>f.key===work.key);const status=JSON.parse(fs.readFileSync(path.join(build,`inputs/${work.key}-status.json`)));return status.work_history.find(w=>w.work_item_id===control.work);}
async function content(page,work){
 const truth=expected(work);
 assert.equal(await page.locator('.record-strip').getAttribute('data-control-state'),truth.state,'exact control state separated from historical catalog');
 for(const [i,u] of work.units.entries())assert.equal(await page.locator(`[data-unit="${i}"]`).textContent(),u.text,'exact prose '+i);
 assert.equal(await page.locator('.goal').textContent(),work.goal,'full original Goal');
 for(const f of truth.answers.facts)assert.equal(await page.locator(`[data-fact="${f.question}"]`).textContent(),f.text,'fact '+f.question);
 assert.equal(await page.locator('.full-audit').textContent(),JSON.stringify(truth,null,2),'complete original audit');
 assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no ordinary page overflow');
 const ids=await page.locator('[id]').evaluateAll(ns=>ns.map(n=>n.id));assert.equal(ids.length,new Set(ids).size,'unique anchors');
 return truth;
}
async function trip(page,work,truth,width,offline=false){
 let trips=0;
 // Heading and synopsis citations are in a closed audit; exercise them too.
 await page.locator('#audit').evaluate(n=>n.open=true);
 for(const [i,u] of work.units.entries())for(const [j,ref] of u.refs.entries()){
   const link=page.locator(`a.evidence-link[data-origin="claim-${i}"][href="#point-${i}-${j}"]`).first();
   const file=work.files.find(f=>f.regions.some(r=>r.key===ref));const region=file.regions.find(r=>r.key===ref);
   const source=truth.answers.source_excerpts.find(s=>s.citation.locator===file.path&&s.citation.start_line===region.start_line&&s.citation.end_line===region.end_line);
   assert(source,'public Source association');
   // Full source file is independently retained; check exact absolute byte and line coordinates.
   const raw=fs.readFileSync(path.join(build,`inputs/${work.key}-source-${ref}.txt`));
   assert.equal(hash(raw),file.sha256.replace('sha256:',''));
   assert.equal(raw.subarray(source.citation.start_byte,source.citation.end_byte).toString(),source.text,'original Source byte coordinates');
   assert.equal(source.text,raw.toString().split(/(?<=\n)/).slice(region.start_line-1,region.end_line).join(''),'original Source line coordinates');
   await link.focus();const original=await page.evaluate(()=>scrollY);await page.keyboard.press('Enter');
   const point=page.locator(`#point-${i}-${j}`);await point.waitFor({state:'visible'});
   assert.equal(await page.evaluate(()=>document.activeElement.id),`point-${i}-${j}`);
   assert.equal(await point.locator('.claim-quote').textContent(),u.text,'correct origin, even shared Source');
   const lines=await point.locator('.code-text').allTextContents();assert.deepEqual(lines,source.text.replace(/\n$/,'').split('\n'));
   assert.deepEqual(await point.locator('.number').allTextContents(),lines.map((_,n)=>String(region.start_line+n)));
   const regionBox=point.locator('.code-region');await regionBox.focus();await page.keyboard.press('ArrowRight');
   if(await regionBox.evaluate(n=>n.scrollWidth>n.clientWidth+2))await page.waitForFunction(n=>n.scrollLeft>0,await regionBox.elementHandle());
   if(!offline){
     const box=await regionBox.boundingBox();assert(box.y>=0&&box.y<700,'code starts in viewport');
     assert.equal(await page.locator('.doc').isVisible(),width>900,'desktop document and evidence stay visible together');
     assert.equal(await page.locator('.appendix').getAttribute('aria-modal'),width<=900?'true':null);
     if(width>900)assert.equal(await page.locator('#claim-'+i).evaluate(n=>{const r=n.getBoundingClientRect();return r.top>=76&&r.top<innerHeight}),true,'original paragraph visible beside evidence');
   }
   if(u.role==='reading_point'&&j===0&&trips<20)await page.screenshot({path:path.join(out,`${work.key}-${width}-${offline?'offline':'live'}-point-${i}.png`)});
   if(!offline&&trips%2===0){await page.keyboard.press('Escape');}
   else{await point.locator('.return').focus();await page.keyboard.press('Enter');}
   await page.waitForFunction(id=>document.activeElement.id===id && !document.body.classList.contains('code-open'),`claim-${i}`);
   await paint(page);
   if(!offline)assert(Math.abs((await page.evaluate(()=>scrollY))-original)<2,`exact scroll return ${i}/${j}: expected ${original}, got ${await page.evaluate(()=>scrollY)}`);
   if(!offline&&trips===0){
     await page.goBack();await point.waitFor({state:'visible'});
     await page.goForward();await page.waitForFunction(id=>document.activeElement.id===id,`claim-${i}`);
   }
   trips++;
 }
 return trips;
}
(async()=>{let server,browser;
 try{
  identity();server=await start();
  browser=await chromium.launch({executablePath:c.chromium,headless:true,args:['--no-sandbox','--disable-background-networking'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});
  result.chromium=browser.version();
  const entry=await browser.newPage();await entry.goto(server.url);
  assert.equal(await entry.locator('html').getAttribute('data-surface'),'viewer','preview starts at Viewer');
  assert.equal(await entry.locator('h1').textContent(),'Volicord');
  assert.deepEqual(await entry.locator('.masthead nav a').allTextContents(),['개요','작업']);
  assert.equal(await entry.locator('footer').count(),0,'no review footer');await entry.close();
  result.checks.push({viewer_review_separation:'closed viewer/ subtree; default preview opens Viewer; review entry separate'});
  for(const width of [1440,768,390,320]){
   const context=await browser.newContext({viewport:{width,height:900}});const page=await context.newPage();
   page.on('request',r=>result.requests.push(r.url()));page.on('pageerror',e=>{throw e;});
   await page.goto(server.url+'index.html');await paint(page);assert.equal(await page.locator('h1').textContent(),'작업을 이해하는새로운 읽기 공간.');
   await page.screenshot({path:path.join(out,`entry-${width}.png`)});
   await page.goto(server.url+'viewer/works.html');await paint(page);
   assert.equal(await page.locator('[data-work]').count(),catalog.works.length);
   assert.equal(await page.locator('.index-intro').textContent(),'작업의 내용과 진행 현황을 확인합니다.');
   for(const file of ['overview.html','works.html']){
    await page.goto(server.url+'viewer/'+file);await paint(page);
    if(file==='overview.html')assert.equal(await page.locator('.project-purpose').textContent(),catalog.purpose.text);
    assert.equal(await page.locator('.work-card').count(),3);
    const ordered=[...catalog.works].sort((a,b)=>rows('checkpoints').find(r=>r.id===b.checkpoint).recorded_at-rows('checkpoints').find(r=>r.id===a.checkpoint).recorded_at).map(w=>w.key);
    assert.deepEqual(await page.locator('.work-card').evaluateAll(ns=>ns.map(n=>n.dataset.catalogWork)),ordered);
    for(const work of catalog.works){
     const card=page.locator(`[data-catalog-work=${work.key}]`);
     assert.equal(await card.locator('h3').textContent(),work.title);
     assert.equal(await card.locator('a,button,[tabindex]').count(),1,'only reading button is interactive');
     assert.equal(await card.locator('a').getAttribute('data-work'),work.key);
     const goal=rows('context_items').find(r=>r.id===work.work);
     const cpTime=rows('checkpoints').find(r=>r.id===work.checkpoint);
     assert.deepEqual(await card.locator('time').evaluateAll(ns=>ns.map(n=>n.dataset.recordedAt)),[String(goal.recorded_at),String(cpTime.recorded_at)],'exact original creation/completion records');
     const format=n=>new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Seoul',year:'numeric',month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit',hourCycle:'h23'}).format(new Date(n/1000)).replaceAll('-','.');
     assert.deepEqual(await card.locator('time').allTextContents(),[format(goal.recorded_at),format(cpTime.recorded_at)],'KST time independently formatted');assert.equal(await card.locator('.work-summary').textContent(),work.summary);
     const cp=rows('checkpoints').find(r=>r.id===work.checkpoint&&r.work_item_id===work.work);assert(cp,'independent original Work checkpoint');
     assert.equal(await card.locator('.work-status').getAttribute('data-recorded-state'),cp.work_state);
     assert.equal(await card.locator('.work-status').textContent(),'완료');assert.equal(await card.locator('.limit,details').count(),0,'cards contain title/state/summary, no hidden summary or limits');
    }
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
    const actualPalette=await page.evaluate(()=>['--paper','--surface','--soft','--ink','--muted','--line','--accent','--wash'].map(k=>getComputedStyle(document.documentElement).getPropertyValue(k).trim()));
    assert.deepEqual(actualPalette,['#f3f5f9','#ffffff','#f3f6fa','#1c2b40','#53637a','#dce3ec','#2357a0','#eaf1fc']);
    await page.screenshot({path:path.join(out,`${file==='overview.html'?'overview':'discovery'}-${width}.png`)});
   }
   for(const item of catalog.works.filter(w=>!authored.works.some(a=>a.key===w.key))){
    await page.goto(server.url+'viewer/works.html');await page.locator(`[data-work=${item.key}]`).focus();await page.keyboard.press('Enter');await page.waitForURL('**/'+item.key+'.html');
    assert.equal(await page.locator('h1').textContent(),item.title);
    assert.equal(await page.locator('.goal').textContent(),rows('context_items').find(r=>r.id===item.work).statement);
    await page.locator('#original summary').click();assert(await page.locator('.goal').isVisible());await page.screenshot({path:path.join(out,`${item.key}-${width}-record.png`)});
   }
   await page.goto(server.url+'viewer/works.html');
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
   await page.screenshot({path:path.join(out,`discovery-${width}.png`)});
   for(const work of authored.works){
    if(catalog.works.some(i=>i.key===work.key)){await page.goto(server.url+'viewer/works.html');await page.locator(`[data-work=${work.key}]`).focus();await page.keyboard.press('Enter');}
    else await page.goto(server.url+'viewer/'+work.key+'.html');
    await page.waitForURL('**/'+work.key+'.html');await paint(page);
    const truth=await content(page,work);
    assert(await page.locator('#reading').evaluate(n=>!!(n.compareDocumentPosition(document.querySelector('.guard'))&Node.DOCUMENT_POSITION_FOLLOWING)),'limits follow the explanation');
    assert.equal(await page.locator('.record-box').count(),2);
    assert(!(await page.locator('.breadcrumb').textContent()).includes('/ 보존된 Work'));
    const divider=await page.locator('.limits-section').evaluate(n=>({border:getComputedStyle(n).borderTopWidth,padding:parseFloat(getComputedStyle(n).paddingTop),margin:parseFloat(getComputedStyle(n).marginTop)}));
    assert.equal(divider.border,'1px');assert(divider.padding>=28&&divider.margin>=28,'limits separated like narrative sections');
    assert(await page.locator('.code-region .syntax-keyword').count()>0,'build-time syntax highlighting');
    await page.locator('#original summary').click();assert(await page.locator('.goal').isVisible());await page.locator('#original summary').click();
    await page.screenshot({path:path.join(out,`${work.key}-${width}-records.png`)});await page.evaluate(()=>scrollTo(0,0));
    assert.equal(await page.locator('.source-point.active').count(),0,'no arbitrary initial source selection');
    await page.screenshot({path:path.join(out,`${work.key}-${width}-reading.png`)});
    if(await page.locator('.comparisons').count()){await page.locator('.comparisons').scrollIntoViewIfNeeded();await page.screenshot({path:path.join(out,`${work.key}-${width}-comparisons.png`)});}
    const trips=await trip(page,work,truth,width);
    await page.goto(server.url+'viewer/'+work.key+'-unavailable.html');await paint(page);
    await content(page,work);await page.locator('.reading-section .evidence-link').first().click();
    await page.locator('.source-point.active .unavailable').waitFor({state:'visible'});assert.equal(await page.locator('.source-point.active .code-region').count(),0);
    await page.screenshot({path:path.join(out,`${work.key}-${width}-unavailable.png`)});
    // Accurate unchanged baseline, whole snapshot: Work/Code/Decision/diagnostic routes remain inspectable.
    await page.goto(server.url+'baseline-'+work.key+'.html#overview');await paint(page);
    const control=fixture.cases.find(f=>f.key===work.key);
    await page.locator(`#overview [data-work-id="${control.work}"] h4 a`).first().click();
    const article=page.locator(`#work-${control.work}`).first();
    for(const u of work.units)assert((await article.textContent()).includes(u.text),'baseline substantive parity');
    await page.screenshot({path:path.join(out,`${work.key}-${width}-baseline.png`)});
    assert(await page.locator('#decision-reading').count());assert(await page.locator('#health').count());assert(await page.locator('#code').count());
    result.checks.push({width,work:work.key,catalog:'A/B/C; exact title/summary and canonical status; overview goal; palette; keyboard entry',syntax:'lexical spans preserve exact bytes',prose:'exact',facts:'exact',sources:'original bytes/coordinates',trips,keyboard_return:'passed',baseline:'same content; unchanged Product source'});
   }
   await context.close();
  }
  for(const work of authored.works){
   const context=await browser.newContext({javaScriptEnabled:false,viewport:{width:320,height:900}});await context.setOffline(true);const page=await context.newPage();
   await page.goto(pathToFileURL(path.join(build,'viewer',work.key+'-offline.html')).href);await paint(page);
   const truth=await content(page,work);assert.equal(await page.locator('script,form').count(),0);
   // One native exact origin round-trip per script-free document (all bytes checked above).
   const i=work.units.findIndex(u=>u.role==='reading_point');const link=page.locator(`[data-origin="claim-${i}"]`).first();await link.focus();await page.keyboard.press('Enter');
   const point=page.locator(`#point-${i}-0`);assert.equal(await point.locator('.claim-quote').textContent(),work.units[i].text);
   await point.locator('.return').focus();await page.keyboard.press('Enter');assert.equal(await page.evaluate(()=>document.activeElement.id),'claim-'+i);
   await page.screenshot({path:path.join(out,`${work.key}-320-script-free.png`)});
   result.checks.push({work:work.key,script_free:'native citation and return; offline; exact content'});await context.close();
  }
  // Authored stress controls are generated separately by self_test.py, not admitted Product facts.
  if(c.stress){
   for(const width of [1440,768,390,320]){
    const page=await browser.newPage({viewport:{width,height:900}});const traffic=[];page.on('request',r=>traffic.push(r.url()));
    await page.goto(pathToFileURL(c.stress).href);await paint(page);
    assert.equal(await page.locator('img,script').count(),0);assert.equal(await page.evaluate(()=>window.__attack),undefined);
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'hostile mixed-language text does not overflow');
    await page.locator('#original').evaluate(n=>n.open=true);assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
    await page.screenshot({path:path.join(out,`stress-${width}.png`)});await page.close();
   }
   result.checks.push({stress:'long mixed language; hostile Goal, heading, prose, Source and audit; all widths; no injection'});
  }
  if(manifest.journeys){
   result.journey_driver_sha256=hash(fs.readFileSync(path.join(__dirname,'journeys.cjs')));
   assert.equal(result.journey_driver_sha256,manifest.files['support/journeys.cjs'].sha256,'exact journey support');
   result.checks.push(await require('./journeys.cjs')({browser,url:server.url,build,out,refined:!!c.refined,previousBuild:c.previous_build}));
  }
  assert(result.requests.every(url=>new URL(url).origin===new URL(server.url).origin),'no remote request');
  // Deliberate missing route cannot silently select a different Work.
  const bad=await browser.newPage();assert.equal((await bad.goto(server.url+'unknown-work.html')).status(),404);await bad.close();
  result.checks.push({wrong_work:'404',remote_requests:0});result.status='passed';
 }catch(error){result.status='failed';result.error=error.stack;process.exitCode=1;}
 finally{if(browser)await browser.close();if(server)await server.stop();fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({status:result.status,checks:result.checks.length,trips:result.checks.reduce((n,c)=>n+(c.trips||0),0),error:result.error||null}));}
})();
