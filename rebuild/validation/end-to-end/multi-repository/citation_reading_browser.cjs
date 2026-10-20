// Exact candidate support: original fixture bytes are the oracle, never the DOM.
// Local technical observations only; no semantic/human/design approval.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,spawnSync,execFileSync}=require('node:child_process');
const {pathToFileURL}=require('node:url'),crypto=require('node:crypto');

function reviewedFiles(directory){const files={};for(const name of fs.readdirSync(directory).sort()){const file=path.join(directory,name);if(!fs.lstatSync(file).isFile()||!['.png','.html'].includes(path.extname(name)))continue;const bytes=fs.readFileSync(file);files[name]={sha256:require('node:crypto').createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length};}return files;}
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const {chromium}=require(config.playwright);
if(config.fontconfig)assert.ok(fs.existsSync(config.fontconfig),'fontconfig file must exist');
const fixture=JSON.parse(fs.readFileSync(path.join(config.fixture,'fixture.json'),'utf8'));
fs.mkdirSync(config.output,{recursive:true});
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function utf8Prefix(source,offset=0){
 const b=Buffer.from(source);let end=Math.min(b.length,offset+4096);
 while(end<b.length && (b[end]&0xc0)===0x80)end--;
 return b.subarray(offset,end).toString('utf8');
}
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
async function initialCode(page) {
 const observed=await page.locator('.source-point > .code-region code').evaluateAll(codes=>codes.map(code=>({
  source:code.closest('.source-point').dataset.sourceId,text:code.textContent,syntax:code.dataset.syntax,
  spans:code.querySelectorAll('span').length,numbers:code.parentElement.querySelector('.source-line-numbers').children.length,
  tags:[...code.querySelectorAll('*')].map(n=>n.tagName),html:code.innerHTML,
 })));
 assert.equal(observed.length,12,'all original and shared claim excerpts remain present');
 for(const code of observed){
  const expected=fixture.answers.source_excerpts.find(e=>e.citation.source_id===code.source);
  assert.ok(expected);assert.equal(code.text,expected.text,'parsed static/offline code equals original admitted UTF-8');
  assert.equal(code.numbers,code.text.split('\n').length-Number(code.text.endsWith('\n')));
  assert.ok(code.spans<=256);assert.ok(code.tags.every(tag=>tag==='SPAN'),'captured text never becomes HTML');
 }
 return {code_elements:observed.length,code_utf8_bytes:observed.reduce((n,c)=>n+Buffer.byteLength(c.text),0),
  token_spans:observed.reduce((n,c)=>n+c.spans,0),highlight_markup_bytes:observed.reduce((n,c)=>n+Buffer.byteLength(c.html)-Buffer.byteLength(c.text.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')),0)};
}
async function selectedCode(point, expected) {
 const code=point.locator('.citation-inspection code');
 assert.equal(await code.textContent(),expected);
 const data=await code.evaluate(code=>{
  const range=document.createRange();range.selectNodeContents(code);
  const selection=getSelection();selection.removeAllRanges();selection.addRange(range);
  const selected=selection.toString();selection.removeAllRanges();
  const exactRange=range.toString();
  // Compare native copy/selection with an otherwise identical plain-text pane:
  // Chromium may omit a final empty rendered line or normalize CR line endings.
  const plain=code.cloneNode(false);plain.textContent=code.textContent;code.replaceWith(plain);
  const plainRange=document.createRange();plainRange.selectNodeContents(plain);selection.addRange(plainRange);
  const plainSelected=selection.toString();selection.removeAllRanges();plain.replaceWith(code);
  return {range:exactRange,selected,plainSelected,spans:code.querySelectorAll('span').length,html:code.innerHTML};
 });
 assert.equal(data.range,expected,'DOM range preserves exact characters without gutter text');
 assert.equal(data.selected,data.plainSelected,'native selection/copy behaves exactly as the original plain Source pane');
 assert.ok(data.spans<=256);
 const initial=point.locator(':scope > pre.code-region code');
 if(await initial.count() && await initial.textContent()===expected)
  assert.equal(data.html,await initial.innerHTML(),'equivalent exact chunks get identical static/live token markup');
}
async function openSource(page,index,keyboard=false) {
 const link=page.locator('.doc a.evidence-link').nth(index);
 const target=page.locator(`[id="${(await link.getAttribute('href')).slice(1)}"]`);
 const expected=new URL(await target.locator('.citation-inspection').getAttribute('data-citation-url'),page.url()).href;
 const fetched=page.waitForResponse(r=>r.url()===expected);
 if(keyboard){await link.focus();await page.keyboard.press('Enter');}else await link.click();
 await fetched;
 await target.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
 await target.locator('.citation-inspection pre.code-region:not([hidden])').waitFor({state:'visible'});
 return target;
}
async function syntaxContrast(page) {
 return page.evaluate(()=>{
  const region=document.querySelector('.source-point:target .code-region:not([hidden])') || document.querySelector('.code-region');
  const rgb=s=>s.match(/[\d.]+/g).slice(0,3).map(Number);
  const luminance=c=>c.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;}).reduce((n,v,i)=>n+v*[.2126,.7152,.0722][i],0);
  const bg=luminance(rgb(getComputedStyle(region).backgroundColor));
  return ['keyword','comment','string','type','function','number','variable'].map(kind=>{
   const span=document.createElement('span');span.className='syn-'+kind;span.textContent='contrast sample';region.append(span);
   const color=getComputedStyle(span).color,fg=luminance(rgb(color));span.remove();
   return {kind,color,ratio:(Math.max(bg,fg)+.05)/(Math.min(bg,fg)+.05)};
  });
 });
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
  await context.addInitScript(()=>{
   window.codePaints=[];
   const create=document.createDocumentFragment.bind(document);
   document.createDocumentFragment=()=>{const f=create();f.paintStart=performance.now();return f;};
   const replace=Element.prototype.replaceChildren;
   Element.prototype.replaceChildren=function(...args){
    const start=args[0]?.paintStart;
    const value=replace.apply(this,args);
    if(this.matches('code') && Number.isFinite(start)){
     const end=performance.now();this.getBoundingClientRect();
     window.codePaints.push({dom_ms:end-start,layout_ms:performance.now()-end,spans:this.children.length,utf8_bytes:new TextEncoder().encode(this.textContent).length});
    }
    return value;
   };
  });
  const measured=[];
  await context.route('**/*',route=>new URL(route.request().url()).origin===new URL(base).origin?route.continue():route.abort());
  const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
  result.code_paints=[];result.initial_code=[];result.external_requests=[];result.syntax_contrast=[];
  page.on('request',r=>{if(new URL(r.url()).origin!==new URL(base).origin)result.external_requests.push(r.url());});
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
    const anchor=navigation.locator(`a[href="#${id}"]`);if(!await navigation.evaluate(n=>n.open))await navigation.locator(':scope > summary').click();
    if(await anchor.locator('xpath=ancestor::details[contains(@class,"toc-more")]').count() && !await navigation.locator('.toc-more').evaluate(n=>n.open))await navigation.locator('.toc-more > summary').click();await anchor.focus();await page.keyboard.press('Enter');
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
   result.initial_code.push({locale,width,...await initialCode(page),...await page.evaluate(()=>({
    dom_elements:document.querySelectorAll('*').length,dom_content_loaded_ms:performance.getEntriesByType('navigation')[0].domContentLoadedEventEnd,
   }))});
   const points=page.locator('.source-point');assert.equal(await points.count(),12);
   const link=page.locator('.doc a.evidence-link').nth(8);exactPoint=(await link.getAttribute('href')).slice(1);
   await openSource(page,8,true);
   const point=page.locator(`[id="${exactPoint}"]`);
   await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   await selectedCode(point,fixture.originals[8]);
   assert.equal(await point.locator('.citation-inspection code').getAttribute('data-syntax'),'rust');
   assert.ok(await point.locator('.citation-inspection .syn-keyword').count());
   assert.equal(await point.locator('.claim-quote').textContent(),fixture.answers.prose[1].text);
   assert.equal(await page.evaluate(()=>document.activeElement.id),exactPoint);
   assert.equal(await page.locator('.doc').evaluate(n=>n.inert),width<900);
   assert.equal(await page.locator('.masthead').evaluate(n=>n.inert),width<900);
   assert.equal(await point.locator('.citation-inspection pre').evaluate(n=>getComputedStyle(n).fontSize),width<=600?'12px':width<=900?'13px':'15px');
   assert.equal(await point.getAttribute('role'),width<900?'dialog':null);
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
   const codePane=point.locator('.citation-inspection pre');
   await codePane.focus();assert.equal(await codePane.evaluate(n=>document.activeElement===n),true,'code overflow is keyboard focusable');
   const overflow=await codePane.evaluate(n=>({width:n.clientWidth,scroll:n.scrollWidth}));
   if(width<=390)assert.ok(overflow.scroll>overflow.width,'long original identifiers/lines scroll within compact Source pane');
   if(width<=390){await page.keyboard.press('ArrowRight');await page.waitForFunction(()=>document.querySelector('.source-point:target .citation-inspection pre').scrollLeft>0);await codePane.evaluate(n=>n.scrollLeft=0);}
   const box=await point.boundingBox();if(width>900)assert.ok(box.x>500,'desktop right evidence pane');
   const screenshot=path.join(config.output,`${locale}-${width}.png`);await page.screenshot({path:screenshot});result.screens.push({path:screenshot,sha256:hash(fs.readFileSync(screenshot))});
   for(const n of [9,10]) {
    await point.locator('.return').click();
    const other=await openSource(page,n);
    const text=await other.locator('.citation-inspection code').textContent();
    assert.equal(text,Buffer.from(fixture.originals[n]).subarray(0,Buffer.byteLength(text)).toString('utf8'));
    assert.ok(Buffer.byteLength(text)<=4096);await other.locator('.return').click();await openSource(page,8);
    await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   }
   if(locale==='en' && [1440,390,320].includes(width)) {
    await point.locator('.return').click();
    const js=await openSource(page,9);
    const original=Buffer.from(fixture.originals[9]);const displayed=await js.locator('.citation-inspection code').textContent();
    await selectedCode(js,original.subarray(0,Buffer.byteLength(displayed)).toString());
    assert.equal(await js.locator('.citation-inspection code').getAttribute('data-syntax'),'javascript');
    assert.ok(await js.locator('.citation-inspection .syn-keyword').count());
    for(const scheme of ['light','dark']) {
     await page.emulateMedia({colorScheme:scheme});await page.screenshot({path:path.join(config.output,`javascript-${width}-${scheme}.png`)});
     const colors=await syntaxContrast(page);assert.ok(colors.every(c=>c.ratio>=4.5),'small code tokens contrast at least 4.5:1');
     result.syntax_contrast.push({width,scheme,colors});
    }
    await page.emulateMedia({forcedColors:'active'});
    assert.equal(await js.locator('.citation-inspection .syn-keyword').first().evaluate(n=>getComputedStyle(n).fontWeight),'650');
    assert.ok(await js.locator('.citation-inspection pre').evaluate(n=>parseFloat(getComputedStyle(n).borderLeftWidth)>0),'observed-range rule survives without syntax colors');
    await page.emulateMedia({forcedColors:'none'});
    await page.emulateMedia({colorScheme:'light'});await js.locator('.return').click();await openSource(page,8);
    await point.locator('.citation-status').filter({hasText:/Displayed/}).waitFor();
    await page.emulateMedia({colorScheme:'dark'});await page.screenshot({path:path.join(config.output,`rust-${width}-dark.png`)});await page.emulateMedia({colorScheme:'light'});
   }
   await page.reload();await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   assert.equal(await point.locator('.citation-inspection code').textContent(),fixture.originals[8]);
   await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>document.activeElement.id),await point.getAttribute('data-origin'));
   await page.goBack();await point.locator('.citation-status').filter({hasText:/Displayed|표시 범위/}).waitFor();
   await point.locator('.return').click();
   // A second claim sharing the same Source must keep its own quote/return target.
   const shared=await openSource(page,11);
   assert.equal(await shared.locator('.claim-quote').textContent(),fixture.answers.prose[2].text);
   assert.equal(await shared.locator('.citation-inspection code').textContent(),fixture.originals[8]);
   result.code_paints.push(...await page.evaluate(()=>window.codePaints));
  }
  // A permitted response delayed in transport cannot paint a closed or switched claim.
  await page.goto(base+'?view=repository');
  for(let index=0;index<11;index++) {
   const point=await openSource(page,index);
   const displayed=await point.locator('.citation-inspection code').textContent();
   await selectedCode(point,Buffer.from(fixture.originals[index]).subarray(0,Buffer.byteLength(displayed)).toString());
   assert.equal(await point.locator('code script,code img,code a').count(),0);
   const locator=fixture.answers.prose[index<8?0:1].citations[index<8?index:index-8].locator;
   if(/\.(log|unknown)$/.test(locator))assert.equal(await point.locator('.citation-inspection code span').count(),0,'unknown/log fallback');
   await point.locator('.return').click();
  }
  // Bad decoration is a neutral fallback, never executable captured markup.
  for(const tokens of [[[0,1,'</span><script>bad</script>']],[[2,4,'keyword'],[0,1,'string']],[[0,999999,'string']]]){
   await context.route('**/citation?*',async route=>{
    const response=await route.fetch();const result=await response.json();result.highlight={language:'rust',tokens};
    await route.fulfill({response,json:result});
   });
   const fetched=page.waitForResponse(r=>new URL(r.url()).pathname==='/citation');
   await page.locator('.doc a.evidence-link').nth(8).click();await fetched;
   const point=page.locator('.source-point:target');await point.locator('.citation-status').filter({hasText:/Displayed/}).waitFor();
   assert.equal(await point.locator('.citation-inspection code').textContent(),fixture.originals[8]);
   assert.equal(await point.locator('.citation-inspection code span').count(),0);
   await point.locator('.return').click();await context.unroute('**/citation?*');
  }
  // A valid Source in the wrong requested page cannot overwrite this selection.
  await context.route('**/citation?*',async route=>{
   const response=await route.fetch();const result=await response.json();result.chunk.omitted_before_utf8_bytes=1;
   await route.fulfill({response,json:result});
  });
  await page.locator('.doc a.evidence-link').nth(8).click();
  const wrongPage=page.locator('.source-point:target');await wrongPage.locator('.citation-status').filter({hasText:/unavailable or changed/}).waitFor();
  assert.equal(await wrongPage.locator('code').count(),0);await wrongPage.locator('.return').click();await context.unroute('**/citation?*');
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
  await selectedCode(selected,utf8Prefix(fixture.originals[0]));
  const nextOffset=Number(await selected.locator('.citation-read').getAttribute('data-next-offset'));
  await selected.locator('.citation-read').click();await selected.locator('.citation-status').filter({hasText:new RegExp('bytes '+nextOffset+'–')}).waitFor();
  await selectedCode(selected,utf8Prefix(fixture.originals[0],nextOffset));
  assert.deepEqual(errors,[]);
  assert.deepEqual(result.external_requests,[],'no external fetch, grammar, font or telemetry');
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
    result.offline_code=await initialCode(p);
    const nav=p.locator('#repository-explanation .document-nav');assert.equal(await nav.count(),1);
    for(const href of await nav.locator('a').evaluateAll(a=>a.map(n=>n.getAttribute('href')))){
     if(!await nav.evaluate(n=>n.open))await nav.locator(':scope > summary').click();
     const a=nav.locator(`a[href="${href}"]`);if(await a.locator('xpath=ancestor::details[contains(@class,"toc-more")]').count() && !await nav.locator('.toc-more').evaluate(n=>n.open))await nav.locator('.toc-more > summary').click();await a.focus();await p.keyboard.press('Enter');
     assert.equal(new URL(p.url()).hash,href);assert.ok(await p.locator(`[id="${href.slice(1)}"]`).isVisible());
    }
    await p.goto(pathToFileURL(path.join(config.fixture,'offline.html')).href+'#repository-explanation');
    const link=p.locator('#repository-explanation .doc a.evidence-link').nth(8);await link.focus();await p.keyboard.press('Enter');
    const point=p.locator('.source-point:target');assert.equal(await point.locator('code').textContent(),fixture.originals[8],'complete representative mechanism embedded offline');
    assert.ok((await point.textContent()).includes('Offline: only the displayed bytes are included'));
    await p.screenshot({path:path.join(config.output,`offline-${width}.png`)});await point.locator('.return').click();
    if([1440,390,320].includes(width)) {
     await p.locator('#repository-explanation .doc a.evidence-link').nth(9).click();
     const js=p.locator('.source-point:target');
     assert.ok(await js.locator('code .syn-keyword').count(),'script-free JavaScript tokens already in HTML');
     for(const scheme of ['light','dark']){
      await p.emulateMedia({colorScheme:scheme});await p.screenshot({path:path.join(config.output,`offline-javascript-${width}-${scheme}.png`)});
     }
     await p.emulateMedia({colorScheme:'light'});await js.locator('.return').click();
    }
   }
   await offline.close();
  } finally {fs.renameSync(fixture.runtime+'.unavailable',fixture.runtime);}
  result.outcome='passed';
 } catch(e){result.outcome='failed';result.error=e.stack;process.exitCode=1;}
 finally {
  if(browser)await browser.close();if(!stopped){child.kill('SIGTERM');result.listener=await exit;}
  fs.writeFileSync(path.join(config.output,'viewer.stdout'),stdout);fs.writeFileSync(path.join(config.output,'viewer.stderr'),stderr);
  result.files=reviewedFiles(config.output);
  fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify(result,null,2));
  console.log(JSON.stringify({outcome:result.outcome,error:result.error,reads:result.reads.length,output:config.output}));
 }
})();
