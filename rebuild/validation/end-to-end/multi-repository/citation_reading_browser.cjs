// Exact candidate support: original fixture bytes are the oracle, never the DOM.
// Local technical observations only; no semantic/human/design approval.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync,execFileSync}=require('node:child_process');
const {pathToFileURL}=require('node:url'),crypto=require('node:crypto');
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const {chromium}=require(config.playwright);
if(config.fontconfig)assert.ok(fs.existsSync(config.fontconfig),'fontconfig file must exist');
const fixture=JSON.parse(fs.readFileSync(path.join(config.fixture,'fixture.json'),'utf8'));
fs.mkdirSync(config.output,{recursive:true});
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function state(){
 return execFileSync('python3',['-c',`import sqlite3,json,hashlib,sys,pathlib
result={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 c=sqlite3.connect('file:'+str(p)+'?mode=ro',uri=True)
 for (t,) in c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"):
  rows=c.execute('SELECT * FROM "'+t.replace('"','""')+'"').fetchall()
  encoded=json.dumps(sorted([repr(r) for r in rows]),ensure_ascii=False).encode()
  result[p.name+':'+t]={'count':len(rows),'sha256':hashlib.sha256(encoded).hexdigest()}
print(json.dumps(result,sort_keys=True))`,fixture.runtime],{encoding:'utf8'});
}
const commands=[];
function command(label,args){
 const start=performance.now();
 const child=spawnSync(config.cli,args,{encoding:'utf8',timeout:30000});
 fs.writeFileSync(path.join(config.output,label+'.stdout'),child.stdout||'');
 fs.writeFileSync(path.join(config.output,label+'.stderr'),child.stderr||'');
 const receipt={argv:[config.cli,...args],exit:child.status,signal:child.signal,error:child.error?.message,ms:performance.now()-start};
 commands.push(receipt);
 if(child.status!==0 || child.error)throw Error(JSON.stringify(receipt));
 return receipt;
}
(async()=>{
 const fontCheck=spawnSync('fc-list',[':lang=ko'],{encoding:'utf8',env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
 fs.writeFileSync(path.join(config.output,'korean-fonts.stdout'),fontCheck.stdout||'');fs.writeFileSync(path.join(config.output,'korean-fonts.stderr'),fontCheck.stderr||'');
 assert.equal(fontCheck.status,0);assert.ok(fontCheck.stdout.trim(),'Korean-capable fonts required');
 const result={initial_unique_text_bytes:fixture.answers.source_excerpts.reduce((n,e)=>n+Buffer.byteLength(e.text),0),
  omitted_unique_ranges:fixture.answers.omitted_source_excerpt_count,
  admitted_cited_source_bytes:fixture.originals.reduce((n,s)=>n+Buffer.byteLength(s),0),
  offline_html_bytes:fs.statSync(path.join(config.fixture,'offline.html')).size,
  candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
  worktree_diff_sha256:hash(execFileSync('git',['diff','HEAD','--','rebuild'])),
  executable_sha256:hash(fs.readFileSync(config.viewer)),fixture_sha256:hash(fs.readFileSync(path.join(config.fixture,'fixture.json'))),
  driver_sha256:hash(fs.readFileSync(__filename)),
  semantic_approval:'not_established',human_comprehension:'pending',design_acceptance:'pending',commands,reads:[],screens:[]};
 const child=spawn(config.viewer,['--runtime',fixture.runtime,'--project',fixture.project,'--bind','127.0.0.1:0','--view','repository'],{stdio:['ignore','pipe','pipe']});
 let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);
 let browser,base,stopped=false;
 const exit=new Promise(resolve=>child.on('exit',(code,signal)=>resolve({code,signal})));
 try {
  base=await new Promise((resolve,reject)=>{
   const timer=setTimeout(()=>reject(Error('listener startup timeout')),10000);
   child.stderr.on('data',()=>{const match=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});
   child.on('exit',()=>{clearTimeout(timer);reject(Error(stderr));});
  });
  browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
  result.browser_version=browser.version();
  const before=state();result.state_before_sha256=hash(before);
  const context=await browser.newContext();
  const measured=[];
  await context.route('**/*',route=>new URL(route.request().url()).origin===new URL(base).origin?route.continue():route.abort());
  const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
  page.on('response',r=>{if(new URL(r.url()).pathname==='/citation'){measured.push((async()=>{
   try { const bytes=await r.body();result.reads.push({status:r.status(),bytes:bytes.length,url:r.url(),duration_ms:r.request().timing().responseEnd}); }
   catch(e){result.reads.push({status:r.status(),body_measurement:'unavailable after navigation/abort',url:r.url(),error:e.message});}
  })());}});
  let exactPoint;
  for(const locale of ['en','ko']) for(const width of [1440,1024,768,390,320]){
   await page.setViewportSize({width,height:900});await page.goto(base+`?view=repository&locale=${locale}&language=en`);
   const navigation=page.locator('.doc .document-nav');
   assert.equal(await navigation.count(),1,'one document section navigator');
   assert.equal(await page.locator('.audit-links,.opening-limits,.limit-shortcut').count(),0,'obsolete duplicate controls removed');
   const destinations=await navigation.locator('a').evaluateAll(links=>links.map(a=>a.hash.slice(1)));
   for(const id of destinations)assert.equal(await page.locator(`[id="${id}"]`).count(),1,'each navigation destination actually exists');
   for(const id of destinations){
    const anchor=navigation.locator(`a[href="#${id}"]`);await anchor.focus();await page.keyboard.press('Enter');
    await page.waitForURL(url=>url.hash==='#'+id);
    const target=page.locator(`[id="${id}"]`);await target.waitFor({state:'visible'});
    const geometry=await page.evaluate(id=>{
     const nav=document.querySelector('.document-nav').getBoundingClientRect();
     const target=document.getElementById(id).getBoundingClientRect();
     return {navTop:nav.top,navBottom:nav.bottom,targetTop:target.top};
    },id);
    assert.ok(geometry.navTop>=-1,'persistent navigation remains on screen');
    assert.ok(geometry.targetTop>=geometry.navBottom-2,'fragment is not covered by navigation');
   }
   await page.evaluate(()=>window.scrollBy(0,400));
   const sticky=await navigation.boundingBox();assert.ok(sticky.y>=-1 && sticky.y<100,'bounded navigation persists after offscreen scrolling');
   await page.goto(base+`?view=repository&locale=${locale}&language=en`);
   const html=await page.content();result.initial_html_bytes=Buffer.byteLength(html);fs.writeFileSync(path.join(config.output,`live-${locale}-${width}.html`),html);
   const points=page.locator('.source-point');assert.equal(await points.count(),12);
   const link=page.locator('.doc a.evidence-link').nth(8);exactPoint=(await link.getAttribute('href')).slice(1);
   await link.focus();await page.keyboard.press('Enter');
   const point=page.locator(`[id="${exactPoint}"]`);
   await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   assert.equal(await point.locator('.citation-inspection code').textContent(),fixture.originals[8]);
   assert.equal(await point.locator('.claim-quote').textContent(),fixture.answers.prose[1].text);
   assert.equal(await page.evaluate(()=>document.activeElement.id),exactPoint);
   assert.equal(await page.locator('.doc').evaluate(n=>n.inert),width<900);
   assert.equal(await page.locator('.masthead').evaluate(n=>n.inert),width<900);
   assert.equal(await point.locator('.citation-inspection pre').evaluate(n=>getComputedStyle(n).fontSize),width<=600?'12px':width<=900?'13px':'15px');
   assert.equal(await point.getAttribute('role'),width<900?'dialog':null);
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
   const box=await point.boundingBox();if(width>900)assert.ok(box.x>500,'desktop right evidence pane');
   const screenshot=path.join(config.output,`${locale}-${width}.png`);await page.screenshot({path:screenshot});result.screens.push({path:screenshot,sha256:hash(fs.readFileSync(screenshot))});
   for(const n of [9,10]) {
    await point.locator('.return').click();await page.locator('.doc a.evidence-link').nth(n).click();
    const other=page.locator('.source-point:target');await other.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
    const text=await other.locator('.citation-inspection code').textContent();
    assert.equal(text,Buffer.from(fixture.originals[n]).subarray(0,Buffer.byteLength(text)).toString('utf8'));
    assert.ok(Buffer.byteLength(text)<=4096);await other.locator('.return').click();await link.click();
    await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   }
   await page.reload();await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   assert.equal(await point.locator('.citation-inspection code').textContent(),fixture.originals[8]);
   await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>document.activeElement.id),await point.getAttribute('data-origin'));
   await page.goBack();await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   await point.locator('.return').click();
   // A second claim sharing the same Source must keep its own quote/return target.
   await page.locator('.doc a.evidence-link').nth(11).click();
   const shared=page.locator('.source-point:target');await shared.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   assert.equal(await shared.locator('.claim-quote').textContent(),fixture.answers.prose[2].text);
   assert.equal(await shared.locator('.citation-inspection code').textContent(),fixture.originals[8]);
  }
  // A permitted response delayed in transport cannot paint a closed or switched claim.
  await page.setViewportSize({width:390,height:900});
  await page.goto(base+'?view=repository');
  let release, intercepted;
  const arrived=new Promise(resolve=>intercepted=resolve);
  const held=new Promise(resolve=>release=resolve);
  const delayed=async route=>{
   const response=await route.fetch();intercepted();await held;
   try{await route.fulfill({response});}catch(error){/* aborted selected read */}
  };
  await context.route('**/citation?*',delayed);
  await page.locator('.doc a.evidence-link').nth(8).click();await arrived;
  await page.keyboard.press('Escape');
  const originFocus=await page.evaluate(()=>document.activeElement.id);
  release();await context.unroute('**/citation?*',delayed);
  await page.locator('.doc a.evidence-link').nth(11).click();
  let active=page.locator('.source-point:target');
  await active.locator('.citation-status').filter({hasText:/Displayed/}).waitFor();
  assert.equal(await active.locator('.claim-quote').textContent(),fixture.answers.prose[2].text);
  assert.equal(await active.locator('.citation-read').isEnabled(),true,'aborted request does not strand its button');
  const modalControls=active.locator('a.return');await modalControls.focus();
  await page.keyboard.press('Shift+Tab');assert.ok(await active.evaluate(n=>n.contains(document.activeElement)),'mobile focus remains inside the Source dialog');
  await page.keyboard.press('Escape');assert.ok(originFocus);
  // Pagination replaces a bounded chunk rather than appending retained file bytes.
  await page.goto(base+'?view=repository');await page.locator('.doc a.evidence-link').first().click();
  let selected=page.locator('.source-point:target');await selected.locator('.citation-status').filter({hasText:/Displayed/}).waitFor();
  assert.equal(await selected.locator('.citation-inspection code').textContent(),fixture.originals[0].slice(0,4096));
  await selected.locator('.citation-read').click();await selected.locator('.citation-status').filter({hasText:/bytes 4096/}).waitFor();
  assert.equal(await selected.locator('.citation-inspection code').textContent(),fixture.originals[0].slice(4096));
  assert.deepEqual(errors,[]);
  assert.equal(state(),before,'GETs preserve all canonical, retention, provider and Derived rows');result.get_state_unchanged=true;
  // Keep the exact old page open while actual Product deletion removes its support.
  await page.goto(base+'?view=repository#'+exactPoint);selected=page.locator('.source-point:target');
  await selected.locator('.citation-status').filter({hasText:/Displayed/}).waitFor();
  const c=fixture.answers.prose[1].citations[0];
  result.deletion=command('delete',['--runtime',fixture.runtime,'--project',fixture.project,'repository','source','delete','--analysis',fixture.analysis,'--source',c.source_id,'--json']);
  const deleted=state();await selected.locator('.citation-read').click();
  await selected.locator('.citation-status').filter({hasText:/unavailable or changed/}).waitFor();
  assert.equal(await selected.locator('pre.code-region').count(),0,'unavailable read removes cached code from this point');
  await page.screenshot({path:path.join(config.output,'unavailable.png')});
  const denied=await context.request.get(new URL(await selected.locator('.citation-inspection').getAttribute('data-citation-url'),base).href);
  assert.equal(denied.status(),409);const deniedBytes=await denied.body();assert.ok(!deniedBytes.toString().includes(fixture.originals[8].split('\n')[0].trim()));result.unavailable_response_bytes=deniedBytes.length;
  await selected.locator('.return').click();assert.equal(state(),deleted);
  await Promise.all(measured);
  assert.ok(result.reads.every(r=>r.bytes===undefined || r.bytes<=32768));
  assert.ok(result.reads.some(r=>r.status===409),'real unavailable response recorded');
  await context.close();child.kill('SIGTERM');result.listener=await exit;stopped=true;
  fs.renameSync(fixture.runtime,fixture.runtime+'.unavailable');
  try {
   const offline=await browser.newContext({javaScriptEnabled:false});await offline.route('**/*',r=>new URL(r.request().url()).protocol==='file:'?r.continue():r.abort());const p=await offline.newPage();
   for(const width of [1440,1024,768,390,320]){
    await p.setViewportSize({width,height:900});await p.goto(pathToFileURL(path.join(config.fixture,'offline.html')).href+'#repository-explanation');
    assert.equal(await p.locator('script,.citation-inspection,[data-citation-url]').count(),0);
    const nav=p.locator('#repository-explanation .document-nav');assert.equal(await nav.count(),1);
    for(const href of await nav.locator('a').evaluateAll(a=>a.map(n=>n.getAttribute('href')))){
     await nav.locator(`a[href="${href}"]`).focus();await p.keyboard.press('Enter');
     assert.equal(new URL(p.url()).hash,href);assert.ok(await p.locator(`[id="${href.slice(1)}"]`).isVisible());
    }
    await p.goto(pathToFileURL(path.join(config.fixture,'offline.html')).href+'#repository-explanation');
    const link=p.locator('#repository-explanation .doc a.evidence-link').nth(8);await link.focus();await p.keyboard.press('Enter');
    const point=p.locator('.source-point:target');assert.equal(await point.locator('code').textContent(),fixture.originals[8],'complete representative mechanism embedded offline');
    assert.ok((await point.textContent()).includes('Offline: only the displayed bytes are included'));
    await p.screenshot({path:path.join(config.output,`offline-${width}.png`)});await point.locator('.return').click();
   }
   await offline.close();
  } finally {fs.renameSync(fixture.runtime+'.unavailable',fixture.runtime);}
  result.outcome='passed';
 } catch(e){result.outcome='failed';result.error=e.stack;process.exitCode=1;}
 finally {
  if(browser)await browser.close();if(!stopped){child.kill('SIGTERM');result.listener=await exit;}
  fs.writeFileSync(path.join(config.output,'viewer.stdout'),stdout);fs.writeFileSync(path.join(config.output,'viewer.stderr'),stderr);
  fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify(result,null,2));
  console.log(JSON.stringify({outcome:result.outcome,error:result.error,reads:result.reads.length,output:config.output}));
 }
})();
