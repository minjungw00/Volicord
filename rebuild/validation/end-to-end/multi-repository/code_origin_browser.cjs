// Exact origin/stream separation in real live and script-free code windows.
// Expectations come from independently checked pinned files, never rendered DOM.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),{pathToFileURL}=require('node:url');
const {createOutput,openEmbeddedCode,returnFromEmbeddedCode,citationInspection,nativeEvidenceContext}=require('../../viewer-design/product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));createOutput(config.output);
const result={outcome:'failed',checks:[],external:[],scope:'technical origin/byte/navigation checks; no semantic or human acceptance'};
let browser,child,active;
const pause=p=>p.waitForTimeout(100);
async function inspect(page,expected,mode,prefix){
 const points=page.locator(`.source-point[data-source-id="${expected.source_id}"]`);
 assert(await points.count()>0,'original Source identity is present');
 const point=points.first(),id=await point.getAttribute('id');
 await page.goto(page.url().split('#')[0]+'#'+id);await pause(page);
 const code=await openEmbeddedCode(point),expectedText=fs.readFileSync(expected.body_file,'utf8');
 assert.equal(await code.textContent(),expectedText,'complete independent original bytes');
 assert.equal(await code.getAttribute('data-syntax'),expected.syntax,'explicit verified origin syntax or truthful plain fallback');
 if(expected.syntax!=='plain')assert(await code.locator('[class^="syn-"]').count()>0,'readable inert syntax runs');
 const citation=JSON.parse(await point.getAttribute('data-citation-binding'));
 assert.equal(citation.field,expected.field);assert.equal(citation.start_line,expected.start_line);assert.equal(citation.end_line,expected.end_line);
 const container=code.locator('xpath=../..');
 if(expected.origin){
  const origin=container.locator('.code-origin');assert.equal(await origin.count(),1);
  for(const part of [expected.origin.path,expected.origin.revision,`${expected.origin.start_line}–${expected.origin.end_line}`])
   assert((await origin.textContent()).includes(part),'separate verified original location: '+part);
  const stream=point.locator('.source-stream');assert((await stream.textContent()).includes(expected.field));
  assert((await stream.textContent()).includes(`${expected.start_line}–${expected.end_line}`));
 }else assert.equal(await container.locator('.code-origin').count(),0);
 if(mode==='live'){
  const responses=[];const listener=async r=>{if(new URL(r.url()).pathname==='/citation')responses.push(await r.json());};page.on('response',listener);
  const control=await citationInspection(point,true);await control.locator('.citation-read').click();
  await page.waitForFunction(({id,text})=>{
   const p=document.getElementById(id),code=document.getElementById(p?.dataset.citedCode)?.querySelector('code');
   return code?.textContent===text&&!document.querySelector(`[id="${p.dataset.sourceMetadata}"] .citation-read`)?.disabled;
  },{id,text:expectedText});
  page.off('response',listener);
  assert(responses.length>0,'actual exact citation HTTP response');
  for(const response of responses){
   assert.equal(response.chunk.citation.source_id,expected.source_id);
   assert.equal(response.chunk.citation.field,expected.field);
   assert.equal(response.highlight.language,expected.syntax==='plain'?null:expected.syntax);
   if(expected.origin)assert.equal(response.chunk.code_origin.revision,expected.origin.revision);
   else assert.equal(response.chunk.code_origin,null);
  }
  assert.equal(await code.textContent(),expectedText,'complete paged code remains byte faithful');
  assert.equal(await code.getAttribute('data-syntax'),expected.syntax);
 }
 if(expected.origin)await container.locator('.code-origin').scrollIntoViewIfNeeded();
 await page.screenshot({path:path.join(config.output,prefix+'-code.png')});
 await returnFromEmbeddedCode(point);
 const back=(await nativeEvidenceContext(point)).locator('a.return:visible');
 const target=(await back.getAttribute('href')).slice(1);await back.click();await pause(page);
 assert.equal(new URL(page.url()).hash.split('!')[0],'#'+target,'exact claim return');
 if(mode==='live')assert.equal(await page.evaluate(()=>document.activeElement?.id),target,'exact focus return');
 result.checks.push({prefix,source_id:expected.source_id,bytes:Buffer.byteLength(expectedText),mode,syntax:expected.syntax,origin:expected.origin||null,return_target:target});
}
(async()=>{try{
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 for(const baseline of config.baselines||[])for(const width of config.widths||[1440,390]){
  const context=await browser.newContext({javaScriptEnabled:false,viewport:{width,height:900}}),page=await context.newPage();active=page;
  await page.goto(pathToFileURL(baseline.offline).href);assert.equal(await page.locator('script').count(),0);
  for(const [n,e] of baseline.expected.entries())await inspect(page,e,'baseline',`${baseline.key}-${width}-${n}`);
  await context.close();active=null;
 }
 for(const c of config.cases){
  const runtime=path.join(config.output,c.key+'-runtime');fs.cpSync(c.runtime,runtime,{recursive:true,errorOnExist:true,force:false});
  child=spawn(config.viewer,['--runtime',runtime,'--project',c.project,'--bind','127.0.0.1:0']);let stderr='';child.stderr.on('data',b=>stderr+=b);
  const done=new Promise(r=>child.once('close',r));
  const base=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error(stderr||'listener timeout')),10000);child.once('error',reject);child.stderr.on('data',()=>{const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});});
  for(const mode of ['live','live-no-js','offline'])for(const width of config.widths||[1440,390]){
   const context=await browser.newContext({javaScriptEnabled:mode==='live',viewport:{width,height:900}}),page=await context.newPage();active=page;
   const address=mode==='offline'?pathToFileURL(c.offline).href:base+`?project=${c.project}&view=work&work=${c.work}&language=${c.language}&locale=${c.locale}`;
   await context.route('**/*',route=>['file:',new URL(base).protocol].includes(new URL(route.request().url()).protocol)&&(new URL(route.request().url()).protocol==='file:'||new URL(route.request().url()).origin===new URL(base).origin)?route.continue():(result.external.push(route.request().url()),route.abort()));
   await page.goto(address);
   if(mode==='offline')assert.equal(await page.locator('script').count(),0,'genuine script-free export');
   for(const [n,e] of c.expected.entries())await inspect(page,e,mode,`${c.key}-${mode}-${width}-${n}`);
   await context.close();active=null;
  }
  child.kill('SIGTERM');await done;child=null;
 }
 assert.deepEqual(result.external,[]);result.outcome='passed';
}catch(e){result.error=String(e.stack||e);process.exitCode=1;if(active&&!active.isClosed())await active.screenshot({path:path.join(config.output,'failure.png')}).catch(()=>{});
}finally{child?.kill('SIGTERM');await browser?.close();fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify(result,null,2),{flag:'wx'});console.log(JSON.stringify({outcome:result.outcome,checks:result.checks.length,error:result.error}));}})();
