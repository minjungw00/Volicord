// Actual admitted document -> grounded element -> exact range -> visible return.
// The unchanged reference is a separate visual oracle; this driver judges bindings.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),{pathToFileURL}=require('node:url');
const crypto=require('node:crypto');
const {createOutput,openEmbeddedCode,returnFromEmbeddedCode,citationPoint}=require('../../viewer-design/product_review_controls.cjs');
const {inspectDiagram}=require('./diagram_geometry.cjs');
const {observeOverview,installEntryObserver}=require('./overview_entry.cjs');
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const delay=ms=>new Promise(r=>setTimeout(r,ms));
async function settled(page){await page.bringToFront();if(page.readerNoScript){await delay(100);return;}await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await delay(80);}
async function listener(config,c,runtime,write){
 const child=spawn(config.viewer,['--runtime',runtime,'--project',c.project,'--bind','127.0.0.1:0']);let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);
 const done=new Promise(r=>child.once('close',(code,signal)=>r({code,signal})));
 const base=await new Promise((r,j)=>{const timer=setTimeout(()=>j(Error('listener '+stderr)),10000);child.once('error',j);child.stderr.on('data',()=>{const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);r(m[0]);}});});
 return{base,async stop(){child.kill('SIGTERM');const result=await done;write(c.key+'-listener.json',JSON.stringify({...result,stdout,stderr}));}};
}
async function run(config){
 createOutput(config.output);const write=(n,v)=>fs.writeFileSync(path.join(config.output,n),v,{flag:'wx'});
 const result={outcome:'failed',classification:'technical real-publication reading; dependent visual comparison; independent/human/assistive acceptance unobserved',checks:[],external:[],identities:{viewer:hash(config.viewer),chromium:hash(config.chromium),driver:hash(__filename)}};
 const fonts=execFileSync('fc-list',['--format','%{file}\n',':lang=ko'],{env:{...process.env,FONTCONFIG_FILE:config.fontconfig},encoding:'utf8'}).trim().split('\n').filter(Boolean);assert.ok(fonts.length,'Korean fonts required');result.fonts=[...new Set(fonts)].map(p=>({path:p,sha256:hash(p)}));
 let browser,server,activePage;
 try{
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 for(const c of config.cases){
  const publicAnswers=JSON.parse(fs.readFileSync(c.public)),runtime=path.join(config.output,c.key+'-runtime');fs.cpSync(c.runtime,runtime,{recursive:true,errorOnExist:true,force:false});server=await listener(config,c,runtime,write);
  const address=server.base+`?project=${c.project}&view=${c.work?'work&work='+c.work:'repository'}&locale=${c.locale||c.language}&language=${c.language}`;
  for(const width of c.widths||config.widths||[1440,1024,768,390,320])for(const mode of config.modes||['live','offline','no-js']){
   const live=['live','live-no-js','live-unenhanced'].includes(mode),scripted=mode==='live';
   console.log(c.key,width,mode);const context=await browser.newContext({viewport:{width,height:900},javaScriptEnabled:!mode.includes('no-js')});context.setDefaultTimeout(15000);context.setDefaultNavigationTimeout(15000);
   const page=await context.newPage();page.readerNoScript=mode.includes('no-js');activePage=page;const errors=[];page.on('pageerror',e=>errors.push(String(e)));
   if(mode==='live-unenhanced')await context.addInitScript(()=>{window.ResizeObserver=undefined;});
   await context.route('**/*',route=>{const u=route.request().url();if(u.startsWith(server.base)||u.startsWith('file:')||u.startsWith('data:'))return route.continue();result.external.push(u);return route.abort();});
   await installEntryObserver(page);
   if(scripted&&!c.work&&width===(config.widths||[1440])[0]){
    await page.goto(server.base+`?project=${c.project}&view=overview&locale=${c.locale||c.language}&language=${c.language}`);await settled(page);
    await page.screenshot({path:path.join(config.output,c.key+'-overview-entry.png')});
    await page.locator('.repository-reading-link').click();await settled(page);
    assert.equal(new URL(page.url()).searchParams.get('view'),'repository');
    assert.equal(new URL(page.url()).searchParams.get('project'),c.project);
    assert.equal(await page.locator('.project-navigation a[aria-current=page]').getAttribute('data-destination'),'repository');
    write(c.key+'-overview-article.json',JSON.stringify({url:page.url(),project:c.project,actualEntry:true}));
   }
   await page.goto(live?address:pathToFileURL(c.offline).href+'#'+(c.work?'work-'+c.work:'repository-explanation'));if(page.readerNoScript)await delay(200);else await page.evaluate(()=>document.fonts.ready);await settled(page);
   const prefix=`${c.key}-${width}-${mode}`;await page.screenshot({path:path.join(config.output,prefix+'-article-initial.png')});
   assert.deepEqual(await require('./reader_hierarchy.cjs').citationHierarchy(page),[],
    'whole declared corpus keeps compact article actions and exact accessible purpose/range');
   if(live)assert.equal(await page.locator('.project-navigation a[aria-current=page]').getAttribute('data-destination'),c.work?'work':'repository');
   else assert.equal(await page.locator('.project-navigation a').count(),3);
   const figures=page.locator('.admitted-diagram');assert.equal(await figures.count(),publicAnswers.diagrams.length,'whole declared publication figures');
   for(const d of publicAnswers.diagrams){
    const figure=page.locator(`.admitted-diagram[data-semantic-fingerprint="${d.semantic_fingerprint}"]`);await figure.scrollIntoViewIfNeeded();await settled(page);
    const baseKey=prefix+'-'+d.id;await page.screenshot({path:path.join(config.output,baseKey+'-initial.png')});
    await figure.screenshot({path:path.join(config.output,baseKey+'-figure.png')});
    const entry=await observeOverview(figure,d);write(baseKey+'-entry.json',JSON.stringify(entry,null,2));assert.deepEqual(entry.errors,[],'ordinary figure entry');
    assert.equal(await figure.getAttribute('data-reader-publication'),c.publication,'exact managed publication');
    const svg=figure.locator('svg');assert.equal(await svg.getAttribute('data-renderer'),config.renderer||'volicord-static-svg/15');
    const geometry=await inspectDiagram(svg,d,publicAnswers.prose);write(baseKey+'-geometry.json',JSON.stringify(geometry,null,2));assert.ok(geometry.errors.length===0,'independent glyph/path/meaning checks: '+geometry.errors.slice(0,3).map(e=>e.kind).join(', '));
    const m=d.meaning,defaultElement=(m.relations||m.groups?.flatMap(g=>g.messages)||m.nodes||m.participants)[0];
    // Enumerate independently from the public semantic record, including roles
    // that have no standalone admitted element token (guards, owner, identifier).
    const inventory=[...(m.nodes||m.participants),...(m.relations||m.groups?.flatMap(g=>g.messages)||[]),
     ...(m.nodes||m.participants).filter(n=>n.identifier).map(n=>({id:n.id+':identifier',label:n.identifier})),
     ...(m.relations||[]).filter(r=>r.condition).map(r=>({id:r.id+':condition',label:r.condition})),
     ...(m.groups||[]).flatMap((g,i)=>g.condition?[{id:'guard:'+(i+1),label:g.condition}]:[]),
     ...(m.owner?[{id:'state:owner',label:m.owner}]:[])];
    const tasks=config.all_elements?inventory.map(e=>e.id):(c.elements?.[d.id]||[defaultElement.id]);assert.ok(tasks.length,'explicit evidence tasks cannot be empty');
    for(const [taskIndex,elementId] of tasks.entries()) {
    const semantic=inventory.find(e=>e.id===elementId);assert.ok(semantic,'declared exact element exists');
    const key=baseKey+(taskIndex?'-'+elementId:'');
    const label=semantic.label,unit=publicAnswers.prose[label.unit];const citationIndex=unit.citations.findIndex(q=>q.claim_start_byte<=label.start_byte&&q.claim_end_byte>=label.end_byte);assert.ok(citationIndex>=0,'covering admitted Source');
    const citation=unit.citations[citationIndex],expected=fs.readFileSync(path.join(c.repository,citation.locator)).subarray(citation.start_byte,citation.end_byte).toString('utf8');assert.equal('sha256:'+crypto.createHash('sha256').update(expected).digest('hex'),citation.range_sha256);
    const figureId=await figure.getAttribute('id'),element=figure.locator(`a.diagram-element[id="${figureId}-${semantic.id}"]`),claimId=(await element.getAttribute('href')).slice(1),claim=page.locator(`[id="${claimId}"]`);
    await element.focus();await settled(page);const before=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left,y:scrollY}));
    await page.keyboard.press('Enter');await settled(page);
    const href=await claim.locator(`a.evidence-link[href$="-c${citationIndex+1}"]`).getAttribute('href').catch(()=>null)||await claim.locator('a.evidence-link').first().getAttribute('href');
    const point=await citationPoint(page,href),pointId=await point.getAttribute('id');let returnPoint=point;
    if(scripted){
     await point.waitFor({state:'visible'});assert.equal(await point.getAttribute('data-reader-publication'),c.publication);
     await page.waitForFunction(id=>{const p=document.getElementById(id);const b=p?.querySelector('.citation-read');return b&&!b.disabled&&b.dataset.nextOffset==='0'&&p.querySelector('.citation-status')?.textContent.includes(document.documentElement.lang==='ko'?'표시 범위':'Displayed');},pointId);
     assert.equal(await point.locator('.citation-inspection code').textContent(),expected,'complete exact cited bytes');
     assert.equal(await point.getAttribute('aria-modal'),width<1200?'true':null,'actual evidence modality');
     assert.equal(await point.locator('.evidence-original').getAttribute('open'),null,'original wording deliberately disclosed');
     assert.equal(await page.locator('main').evaluate(n=>n.inert),false,'Source remains operable inside main');
     if(width>=1200){assert.equal(await page.locator('.doc').first().evaluate(n=>n.inert),false);}
     else {assert.equal(await page.locator('.project-shell').evaluate(n=>n.inert),true);}
    }else{
     await claim.locator('a.evidence-link').first().click();await settled(page);
     const opened=await openEmbeddedCode(point);assert.equal(await opened.textContent(),expected,'native/offline complete exact code');
     await returnFromEmbeddedCode(point);
    }
    await page.screenshot({path:path.join(config.output,key+'-code.png')});
    if(!scripted){
     const origin=point.locator(':scope > .source-origin-target:target'),chooser=origin.locator('.native-range-selector>summary');
     assert.equal(await origin.count(),1,'native claim origin is retained');
     assert.equal(await chooser.count(),unit.citations.filter(q=>q.claim_start_byte<=label.start_byte&&q.claim_end_byte>=label.end_byte).length>1?1:0,'native range choices reflect admitted covering citations');
     if(await chooser.count()){
      await chooser.click();const next=origin.locator('.native-range-links a:not([aria-current])').first();
      const nextHref=await next.getAttribute('href');await next.click();await settled(page);
      const nextPoint=await citationPoint(page,nextHref),n=Number(nextHref.match(/-c(\d+)$/)[1])-1,q=unit.citations[n];
      assert.equal(await nextPoint.getAttribute('data-source-id'),q.source_id,'native alternate exact Source');
      const alternate=fs.readFileSync(path.join(c.repository,q.locator)).subarray(q.start_byte,q.end_byte).toString('utf8');
      assert.equal('sha256:'+crypto.createHash('sha256').update(alternate).digest('hex'),q.range_sha256);
      assert.equal(await (await openEmbeddedCode(nextPoint)).textContent(),alternate,'native alternate complete bytes');
      await returnFromEmbeddedCode(nextPoint);returnPoint=nextPoint;
      await page.screenshot({path:path.join(config.output,key+'-alternate-code.png')});
     }
    }
    if(scripted)await page.keyboard.press('Escape');else {
      await returnPoint.locator('a.return:visible').click();
      assert.equal(new URL(page.url()).hash,'#'+figureId+'-'+semantic.id,'native exact diagram return');
    }await settled(page);
    if(scripted){
     assert.equal(await element.evaluate(n=>n===document.activeElement),true,'exact element focus return');const after=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left}));write(key+'-return.json',JSON.stringify({before,after}));assert.ok(Math.abs(before.top-after.top)<3,'visible return after reflow');
     assert.equal(await page.locator('.source-point[data-selected]').count(),0);
     assert.equal(await page.locator('.project-shell').evaluate(n=>n.inert),false,'no stale modal inert state');
    }
    await page.screenshot({path:path.join(config.output,key+'-returned.png')});
    if(config.interactions&&scripted&&d===publicAnswers.diagrams[0]){
      const interaction=await require('./evidence_interactions.cjs').inspect(page,context,{address,figureId,elementId:figureId+'-'+semantic.id,claimId,sourceId:pointId,width});write(key+'-interactions.json',JSON.stringify(interaction,null,2));
    }
    result.checks.push({case:c.key,diagram:d.id,element:semantic.id,width,mode,publication:c.publication,unit:label.unit,citation:citationIndex,code_bytes:Buffer.byteLength(expected),entry,geometry_errors:geometry.errors});
    }
    // Restore ordinary entry before observing the next declared figure.
    await page.goto(live?address:pathToFileURL(c.offline).href+'#'+(c.work?'work-'+c.work:'repository-explanation'));await page.reload();await settled(page);
   }
   assert.deepEqual(errors,[],'Product script errors');await context.close();activePage=null;
  }
  await server.stop();server=null;
 }
 assert.deepEqual(result.external,[]);result.outcome='passed';
 }catch(error){result.error=String(error);if(activePage&&!activePage.isClosed()){result.url=activePage.url();result.focus=await activePage.evaluate(()=>document.activeElement?.outerHTML).catch(()=>null);await activePage.screenshot({path:path.join(config.output,'failure.png')}).catch(()=>{});}throw error;
 }finally{await browser?.close();await server?.stop();write('result.json',JSON.stringify(result,null,2));}
}
module.exports={run};
