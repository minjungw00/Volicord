// Real Source transport, one shared code window, native fallback and typed denial.
// Fixture bytes are independent input; this supplies no semantic/human approval.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync,execFileSync}=require('node:child_process'),{pathToFileURL}=require('node:url');
const crypto=require('node:crypto');
const {createOutput,embeddedCode,openEmbeddedCode,citationInspection,citationReady}=require('../../viewer-design/product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));createOutput(config.output);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex'),write=(n,v)=>fs.writeFileSync(path.join(config.output,n),JSON.stringify(v,null,2),{flag:'wx'});
let fixture,tasks;
const pause=ms=>new Promise(r=>setTimeout(r,ms));
function state(){return execFileSync('python3',['-c',`import sqlite3,json,hashlib,sys,pathlib
result={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 c=sqlite3.connect('file:'+str(p)+'?mode=ro',uri=True)
 for (t,) in c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"):
  rows=c.execute('SELECT * FROM "'+t.replace('"','""')+'"').fetchall()
  result[p.name+':'+t]=hashlib.sha256(json.dumps(sorted([repr(r) for r in rows]),ensure_ascii=False).encode()).hexdigest()
print(json.dumps(result,sort_keys=True))`,fixture.runtime],{encoding:'utf8'});}
async function pointFor(page,index){
 const t=tasks[index],point=page.locator(`.source-point[data-unit-index="${t.unit}"][data-citation-index="${t.index}"]`);assert.equal(await point.count(),1);
 assert.deepEqual(JSON.parse(await point.getAttribute('data-citation-binding')),t.citation,'independent exact original citation');return point;
}
async function code(point){return point.page().locator('[id="'+await point.getAttribute('data-cited-code')+'"] code');}
async function status(point,pattern){
 const node=(await citationInspection(point)).locator('.citation-status');
 await point.page().waitForFunction(({node,source,flags})=>new RegExp(source,flags).test(node.textContent),{node:await node.elementHandle(),source:pattern.source,flags:pattern.flags});
}
async function activate(page,index,scripted=true,wait=true){
 const point=await pointFor(page,index),id=await point.getAttribute('id'),origin=await point.getAttribute('data-origin');
 const action=page.locator('.doc [id="'+origin+'"] > .evidence-actions > .paragraph-evidence-action');assert.equal(await action.count(),1,'one complete-family action');
 const family=JSON.parse(await action.getAttribute('data-citation-targets'));assert(family.includes('#'+id));
 await action.focus();await action.press('Enter');
 if(scripted){
  const active=page.locator('.source-point[data-selected=true]');
  if(await active.getAttribute('id')!==id)await active.locator('.evidence-choice-picker select').selectOption('#'+id);
  if(wait)await citationReady(point);
 }else{
  if(new URL(page.url()).hash!=='#'+id){const active=page.locator('.source-point:target');if(!await active.locator(':scope > .range-selector').evaluate(n=>n.open))await active.locator(':scope > .range-selector > summary').click();await active.locator('.point-others a[href="#'+id+'"]').click();}
  if(wait)await openEmbeddedCode(point);
 }
 return point;
}
async function close(point){await point.locator('a.return:visible').click();}
async function exact(point,expected){
 const c=await code(point);assert.equal(await c.textContent(),expected,'complete original UTF-8 range');
 const copied=await c.evaluate(n=>{const r=document.createRange();r.selectNodeContents(n);const s=getSelection();s.removeAllRanges();s.addRange(r);const selected=s.toString(),range=r.toString();s.removeAllRanges();const plain=n.cloneNode(false);plain.textContent=n.textContent;n.replaceWith(plain);const p=document.createRange();p.selectNodeContents(plain);s.addRange(p);const baseline=s.toString();s.removeAllRanges();plain.replaceWith(n);return{selected,range,baseline,tags:[...n.querySelectorAll('*')].map(t=>t.tagName)};});
 assert.equal(copied.range,expected);assert.equal(copied.selected,copied.baseline,'native copying equals plain text');assert(copied.tags.every(t=>t==='SPAN'));
}
async function initial(page){
 let included=0;for(let i=0;i<tasks.length;i++){
  const p=await pointFor(page,i),c=await embeddedCode(p),t=tasks[i].citation;
  const e=fixture.answers.source_excerpts.find(e=>e.citation.source_id===t.source_id&&e.citation.range_sha256===t.range_sha256&&e.citation.start_byte===t.start_byte&&e.citation.end_byte===t.end_byte);
  assert.equal(await c.count(),e?1:0,'initial omission agrees with the independent public reading scope');
  if(e){assert.equal(await c.textContent(),e.text);included++;}
 }
 return included;
}
async function contrast(page,point){return (await code(point)).evaluate(code=>{
 const region=code.closest('pre'),rgb=s=>s.match(/[\d.]+/g).slice(0,3).map(Number),lum=c=>c.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;}).reduce((n,v,i)=>n+v*[.2126,.7152,.0722][i],0),bg=lum(rgb(getComputedStyle(region).backgroundColor));
 return ['keyword','comment','string','type','function','number','variable'].map(kind=>{const span=document.createElement('span');span.className='syn-'+kind;span.textContent='sample';code.append(span);const fg=lum(rgb(getComputedStyle(span).color));span.remove();return{kind,ratio:(Math.max(bg,fg)+.05)/(Math.min(bg,fg)+.05)};});
});}
async function main(){
 const result={outcome:'failed',attempt_output:path.resolve(config.output),conditions:[],reads:[],screens:[],rejected_citation_bindings:[],semantic_approval:'not_established',human_acceptance:'unobserved'};
 let child,browser,logs='',stdout='',activePage,exit;const responses=[];
 const fail=error=>{if(!result.error){result.outcome='failed';result.error=error.stack||String(error);}else(result.teardown_errors??=[]).push(error.stack||String(error));};
 try{
 result.config_sha256=hash(fs.readFileSync(process.argv[2]));result.controls_sha256=hash(fs.readFileSync(require.resolve('../../viewer-design/product_review_controls.cjs')));result.driver_sha256=hash(fs.readFileSync(__filename));result.candidate=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();result.worktree_diff_sha256=hash(execFileSync('git',['diff','HEAD','--','rebuild']));result.executable_sha256=hash(fs.readFileSync(config.viewer));result.cli_sha256=hash(fs.readFileSync(config.cli));result.fixture_sha256=hash(fs.readFileSync(path.join(config.fixture,'fixture.json')));
 fixture=JSON.parse(fs.readFileSync(path.join(config.fixture,'fixture.json')));tasks=(fixture.answers.prose||[]).flatMap((u,unit)=>u.citations.map((citation,index)=>({unit,index,citation})));
 if(config.fontconfig)assert(fs.existsSync(config.fontconfig),'fontconfig file must exist');
 const fonts=spawnSync('fc-list',[':lang=ko'],{encoding:'utf8',env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});result.font_check={argv:['fc-list',':lang=ko'],exit:fonts.status,signal:fonts.signal,error:fonts.error?.message};fs.writeFileSync(path.join(config.output,'korean-fonts.stdout'),fonts.stdout||'');fs.writeFileSync(path.join(config.output,'korean-fonts.stderr'),fonts.stderr||'');if(fonts.error)throw fonts.error;assert.equal(fonts.status,0);assert(fonts.stdout.trim());
 result.initial_unique_text_bytes=fixture.answers.source_excerpts.reduce((n,e)=>n+Buffer.byteLength(e.text),0);result.omitted_unique_ranges=fixture.answers.omitted_source_excerpt_count;result.admitted_cited_source_bytes=fixture.originals.reduce((n,t)=>n+Buffer.byteLength(t),0);result.offline_html_bytes=fs.statSync(path.join(config.fixture,'offline.html')).size;
 const before=state();child=spawn(config.viewer,['--runtime',fixture.runtime,'--project',fixture.project,'--bind','127.0.0.1:0','--view','repository']);child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>logs+=b);exit=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
 const base=await new Promise((ok,no)=>{const timer=setTimeout(()=>no(Error(logs||'listener timeout')),15000);child.once('error',no);child.stderr.on('data',()=>{const m=logs.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);ok(m[0]);}});});
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 const context=await browser.newContext();await context.route('**/*',r=>r.request().url().startsWith(base)?r.continue():r.abort());const page=await context.newPage();activePage=page;const errors=[];page.on('pageerror',e=>errors.push(e.message));
 page.on('response',r=>{if(new URL(r.url()).pathname==='/citation')responses.push((async()=>{try{const body=await r.body();result.reads.push({status:r.status(),bytes:body.length,url:r.url()});}catch{result.reads.push({status:r.status(),url:r.url(),body:'navigation/abort'});}})());});
 for(const locale of ['en','ko'])for(const width of [1440,1024,768,390,320]){
  await page.setViewportSize({width,height:900});await page.goto(base+'?view=repository&language=en&locale='+locale);const included=await initial(page);
  for(const index of [8,9,10,11]){
   const p=await activate(page,index);await exact(p,fixture.originals[index===11?8:index]);
   const pane=p.locator('xpath=..');assert.equal(await pane.getAttribute('aria-modal'),width<1200?'true':null);assert(await pane.evaluate(n=>n.contains(document.activeElement)),'focus belongs to evidence');
   const pre=(await code(p)).locator('xpath=..');await pre.focus();assert(await pre.evaluate(n=>n===document.activeElement));assert.equal(await pre.evaluate(n=>getComputedStyle(n).fontSize),'14px','selected CE1 code typography');
   if(index===9){for(const scheme of ['light','dark']){await page.emulateMedia({colorScheme:scheme});const colors=await contrast(page,p);assert(colors.every(c=>c.ratio>=4.5));}await page.emulateMedia({colorScheme:'light'});}
   if(index===8){const f=path.join(config.output,locale+'-'+width+'.png');await page.screenshot({path:f});result.screens.push({path:f,sha256:hash(fs.readFileSync(f))});}
   await page.keyboard.press('Escape');await page.waitForFunction(id=>document.activeElement?.dataset.origin===id||document.activeElement?.id===id,await p.getAttribute('data-origin'));
  }
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));result.conditions.push({locale,width,included,live:true});
 }
 await page.setViewportSize({width:1440,height:900});await page.goto(base+'?view=repository');
 for(let i=0;i<tasks.length;i++){const p=await activate(page,i);await exact(p,fixture.originals[i===11?8:i]);await close(p);}
 // Decoration cannot turn captured bytes into markup or alter copy semantics.
 for(const tokens of [[[0,1,'</span><script>bad</script>']],[[2,4,'keyword'],[0,1,'string']],[[0,999999,'string']]]){
  await context.route('**/citation?*',async r=>{const response=await r.fetch(),value=await response.json();value.highlight={language:'rust',tokens};await r.fulfill({response,json:value});});
  const p=await activate(page,8);await exact(p,fixture.originals[8]);assert.equal(await (await code(p)).locator('span').count(),0);await close(p);await context.unroute('**/citation?*');
 }
 const bindingKeys=Object.keys(tasks[8].citation);
 for(const key of [...bindingKeys,'next_offset','omitted_before_utf8_bytes','omitted_after_utf8_bytes']){
  await context.route('**/citation?*',async r=>{const response=await r.fetch(),v=await response.json();if(key==='next_offset')v.chunk.next_offset=1;else if(key.startsWith('omitted_'))v.chunk[key]++;else{const value=v.chunk.citation[key];v.chunk.citation[key]=typeof value==='number'?value+1:'foreign-binding';if(key==='start_byte'){v.chunk.citation.end_byte++;v.chunk.displayed_start_byte++;v.chunk.displayed_end_byte++;}}await r.fulfill({response,json:v});});
  const p=await activate(page,8,true,false);await status(p,/unavailable or changed/);assert.equal(await (await code(p)).count(),0,'foreign binding cannot paint: '+key);await close(p);await context.unroute('**/citation?*');result.rejected_citation_bindings.push(key);
 }
 // A validated retry clears a transient refusal, while contradictory current
 // availability cannot revive a removed payload or cached bytes.
 const recovered=await activate(page,8);await exact(recovered,fixture.originals[8]);assert.equal(await recovered.locator('.range-unavailable:visible').count(),0,'successful current retry clears the stale refusal');await close(recovered);
 await context.route('**/citation?*',async r=>{const response=await r.fetch(),value=await response.json();for(const a of value.applicability.source_ranges)if(a.unit_index===tasks[8].unit&&a.citation_index===tasks[8].index){a.available=false;a.limitation='unavailable';}await r.fulfill({response,json:value});});
 const unavailable=await activate(page,8,true,false);await unavailable.locator('.range-unavailable').waitFor();assert.equal(await (await code(unavailable)).count(),0,'unavailable applicability cannot paint a successful stale chunk');await close(unavailable);await context.unroute('**/citation?*');
 const retried=await activate(page,8);await exact(retried,fixture.originals[8]);assert.equal(await retried.locator('.range-unavailable').count(),0);await close(retried);
 // Held successful bytes cannot paint after close or overwrite another claim.
 await page.setViewportSize({width:390,height:900});let release,arrived;const held=new Promise(r=>release=r),arrival=new Promise(r=>arrived=r);
 await context.route('**/citation?*',async r=>{const response=await r.fetch();arrived();await held;await r.fulfill({response}).catch(()=>{});});
 const pending=await activate(page,8,true,false);await arrival;await page.keyboard.press('Escape');release();await context.unroute('**/citation?*');
 const shared=await activate(page,11);await exact(shared,fixture.originals[8]);await pause(100);await exact(shared,fixture.originals[8]);await shared.locator('a.return:visible').focus();await page.keyboard.press('Shift+Tab');assert(await shared.locator('xpath=..').evaluate(n=>n.contains(document.activeElement)));await close(shared);
 assert.equal(state(),before,'GETs preserve canonical, retention, provider and Derived rows');result.get_state_unchanged=true;
 // Current revalidation after a real deletion excludes all dependent cached meaning.
 await page.goto(base+'?view=repository');const selected=await activate(page,8),inspection=await citationInspection(selected,true),url=new URL(await inspection.getAttribute('data-citation-url'),base).href;
 const command=[config.cli,'--runtime',fixture.runtime,'--project',fixture.project,'repository','source','delete','--analysis',fixture.analysis,'--source',tasks[8].citation.source_id,'--json'];const deletion=spawnSync(command[0],command.slice(1),{encoding:'utf8'});fs.writeFileSync(path.join(config.output,'delete.stdout'),deletion.stdout||'');fs.writeFileSync(path.join(config.output,'delete.stderr'),deletion.stderr||'');assert.equal(deletion.status,0);result.deletion={argv:command,exit:deletion.status};
 await inspection.locator('.citation-read').click();await selected.locator('.publication-unavailable').waitFor();assert.equal(await page.locator('[data-unavailable=true] code,[data-unavailable=true] .unit-text,[data-unavailable=true] svg').count(),0);assert.equal(await page.locator('[data-unavailable=true][data-citation-binding]').count(),0,'serialized citation locations are withheld too');
 const denied=await context.request.get(url);assert.equal(denied.status(),409);assert.equal((await denied.json()).reason,'deleted');
 await Promise.all(responses);assert(result.reads.every(r=>r.bytes===undefined||r.bytes<=32768));await context.close();child.kill('SIGTERM');result.listener=await exit;child=null;
 fs.renameSync(fixture.runtime,fixture.runtime+'.unavailable');
 try{
 const offline=await browser.newContext({javaScriptEnabled:false});await offline.route('**/*',r=>r.request().url().startsWith('file:')?r.continue():r.abort());const native=await offline.newPage();activePage=native;
 for(const width of [1440,1024,768,390,320]){
  await native.setViewportSize({width,height:900});await native.goto(pathToFileURL(path.resolve(config.fixture,'offline.html')).href+'#repository-explanation');assert.equal(await native.locator('script,.citation-inspection,[data-citation-url]').count(),0);await initial(native);
  for(const i of [8,9,10,11]){const p=await activate(native,i,false);await exact(p,fixture.originals[i===11?8:i]);await close(p);}result.conditions.push({width,native:true});
 }
 await offline.close();
 }finally{fs.renameSync(fixture.runtime+'.unavailable',fixture.runtime);}
 assert.deepEqual(errors,[]);result.outcome='passed';
 }catch(error){fail(error);await activePage?.screenshot({path:path.join(config.output,'failure.png')}).catch(()=>{});throw error;}
 finally{
  try{await browser?.close();}catch(error){fail(error);}
  try{if(child){child.kill('SIGTERM');result.listener=await exit;}}catch(error){fail(error);}
  fs.writeFileSync(path.join(config.output,'viewer.stdout'),stdout,{flag:'wx'});fs.writeFileSync(path.join(config.output,'viewer.stderr'),logs,{flag:'wx'});result.files=Object.fromEntries(fs.readdirSync(config.output).sort().filter(n=>['.png','.html'].includes(path.extname(n))).map(n=>{const bytes=fs.readFileSync(path.join(config.output,n));return[n,{sha256:hash(bytes),bytes:bytes.length}];}));write('result.json',result);
 }
 console.log(JSON.stringify({outcome:result.outcome,conditions:result.conditions.length,bindingNegatives:result.rejected_citation_bindings.length,reads:result.reads.length}));
}
main().catch(e=>{console.error(e);process.exitCode=1});
