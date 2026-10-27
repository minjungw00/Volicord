// Real request/fragment ownership with scripting disabled at context creation.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const assert=require('node:assert/strict'),crypto=require('node:crypto'),{spawn}=require('node:child_process');
const {pathToFileURL}=require('node:url');
const {createOutput,citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('../../viewer-design/product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));createOutput(config.output);
const write=(n,x)=>fs.writeFileSync(path.join(config.output,n),JSON.stringify(x,null,2),{flag:'wx'});
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const result={outcome:'failed',scope:'actual script-free file/HTTP and live-no-JS navigation; no human/assistive verdict',checks:[],external:[],viewer:hash(fs.readFileSync(config.viewer))};
let browser,files,child,active;
const pause=p=>p.waitForTimeout(100);
async function cue(page,kind,c,width,snapshot,projectName){
 const names=c.locale==='ko'?{overview:'개요',repository:'저장소 설명',work:'작업'}:{overview:'Overview',repository:'Repository explanation',work:'Work'};
 if(width<=980){assert.equal(await page.locator('.mobile-location').isVisible(),true,'the location cue cannot be hidden');assert.equal((await page.locator('.mobile-location').innerText()).trim(),names[kind],'native location must match owning destination');}
 assert.equal(await page.locator('.mobile-project').textContent(),projectName,'exact Project identity display');
 const selected=await page.locator('.project-navigation .nav-link').evaluateAll(ns=>ns.filter(n=>getComputedStyle(n).backgroundColor==='rgb(226, 242, 253)').map(n=>n.dataset.destination));
 assert.deepEqual(selected,[kind],'one truthful native destination');
 if(snapshot){assert.equal(await page.locator('script').count(),0,'genuine script-free export');assert.equal(await page.locator('main>.snapshot-page:visible').count(),1,'exactly the owning document, including Overview');
  const accessible=await page.locator('.project-navigation .native-current').evaluateAll(ns=>ns.filter(n=>getComputedStyle(n).display!=='none').map(n=>n.closest('a').dataset.destination));assert.deepEqual(accessible,[kind],'accessible current-page text shares native ownership');
  assert.equal(await page.locator('.project-shell>summary').getAttribute('aria-label'),null,'a frozen label must not override Project and native context');
 }
}
async function navigate(page,destination,width){
 if(width<=980){await page.locator('.project-shell>summary').focus();await page.keyboard.press('Enter');}
 await page.locator(`.project-navigation a[data-destination="${destination}"]`).click();
 if(width<=980&&await page.locator('.project-shell[open]').count()){await page.locator('.project-shell>summary').focus();await page.keyboard.press('Enter');}
 await pause(page);
}
(async()=>{try{
 const sources=new Map(config.cases.map(c=>['/'+c.key+'.html',fs.readFileSync(c.offline)]));
 files=http.createServer((req,res)=>{const b=sources.get(new URL(req.url,'http://localhost').pathname);if(!b){res.writeHead(404).end();return;}res.setHeader('Content-Type','text/html; charset=utf-8');res.end(b);});
 await new Promise(r=>files.listen(0,'127.0.0.1',r));const fileBase=`http://127.0.0.1:${files.address().port}`;
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 for(const c of config.cases){
  const runtime=path.join(config.output,c.key+'-runtime');fs.cpSync(c.runtime,runtime,{recursive:true,force:false,errorOnExist:true});
  child=spawn(config.viewer,['--runtime',runtime,'--project',c.project,'--bind','127.0.0.1:0']);let stderr='';child.stderr.on('data',b=>stderr+=b);const done=new Promise(r=>child.once('close',r));
  const base=await new Promise((r,j)=>{const timer=setTimeout(()=>j(Error(stderr||'listener timeout')),10000);child.once('error',j);child.stderr.on('data',()=>{const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);r(m[0]);}});});
  for(const transport of ['live-no-js','snapshot-http'])for(const width of config.widths||[1440,1024,768,390,320]){
   const snapshot=transport==='snapshot-http',context=await browser.newContext({javaScriptEnabled:false,viewport:{width,height:900}});context.setDefaultTimeout(15000);const page=await context.newPage();active=page;
   await context.route('**/*',route=>[new URL(base).origin,fileBase].includes(new URL(route.request().url()).origin)?route.continue():(result.external.push(route.request().url()),route.abort()));
   const address=snapshot?fileBase+'/'+c.key+'.html':base+`?project=${c.project}&view=overview&locale=${c.locale}&language=${c.language}`;
   const response=await page.goto(address);await pause(page);if(snapshot)assert.equal(hash(await response.body()),hash(fs.readFileSync(c.offline)),'server delivers unchanged export bytes');
   const projectName=await page.locator('.mobile-project').textContent(),prefix=`${c.key}-${transport}-${width}`;
   await cue(page,'overview',c,width,snapshot,projectName);await page.screenshot({path:path.join(config.output,prefix+'-overview.png')});
   await navigate(page,c.work?'work':'repository',width);
   await cue(page,c.work?'work':'repository',c,width,snapshot,projectName);
   if(c.work){await page.locator(snapshot?`main a[href="#work-${c.work}"]:visible`:`.work-row[data-work-id="${c.work}"]`).first().click();await pause(page);}
   const kind=c.work?'work':'repository';await cue(page,kind,c,width,snapshot,projectName);await page.screenshot({path:path.join(config.output,prefix+'-document.png')});
   const toc=page.locator('.document-nav').first();if(await toc.count()){if(await toc.locator('summary').isVisible())await toc.locator('summary').click();await toc.locator('a').first().click();await pause(page);await cue(page,kind,c,width,snapshot,projectName);}
   const element=page.locator('.admitted-diagram a.diagram-element').first();
   if(await element.count()){
    const origin=await element.getAttribute('id'),claim=(await element.getAttribute('href')).slice(1);
    await element.focus();await page.keyboard.press('Enter');await pause(page);await cue(page,kind,c,width,snapshot,projectName);
    const link=page.locator(`[id="${claim}"] a.evidence-link`).first(),href=await link.getAttribute('href'),point=await citationPoint(page,href);await link.click();await pause(page);await cue(page,kind,c,width,snapshot,projectName);
    const code=await openEmbeddedCode(point);assert.ok((await code.textContent()).length,'permitted native complete code remains available');const directCode=page.url();await cue(page,kind,c,width,snapshot,projectName);await page.screenshot({path:path.join(config.output,prefix+'-code.png')});await returnFromEmbeddedCode(point);
    await point.locator('a.return:visible').click();await pause(page);assert.equal(new URL(page.url()).hash,'#'+origin);await cue(page,kind,c,width,snapshot,projectName);await page.screenshot({path:path.join(config.output,prefix+'-returned.png')});
    await page.goBack();await pause(page);await cue(page,kind,c,width,snapshot,projectName);await page.goForward();await pause(page);await cue(page,kind,c,width,snapshot,projectName);
    await page.goto(directCode);await pause(page);await cue(page,kind,c,width,snapshot,projectName);await page.reload();await pause(page);await cue(page,kind,c,width,snapshot,projectName);
    await returnFromEmbeddedCode(point);await point.locator('a.return:visible').click();await pause(page);await cue(page,kind,c,width,snapshot,projectName);
   }
   if(snapshot){await page.goto(address+'#unknown-native-destination');await pause(page);await cue(page,'overview',c,width,true,projectName);assert.equal(await page.locator('#overview').isVisible(),true);assert.equal(await page.locator('.snapshot-link-help').isVisible(),true,'unknown target has an explicit unavailable-link notice');}
   await navigate(page,'overview',width);await cue(page,'overview',c,width,snapshot,projectName);
   result.checks.push({case:c.key,transport,width,publication:c.publication});await context.close();active=null;
  }
  const context=await browser.newContext({javaScriptEnabled:false,viewport:{width:390,height:900}}),page=await context.newPage();await page.goto(pathToFileURL(c.offline).href+'#'+(c.work?'work-'+c.work:'repository-explanation'));await pause(page);await cue(page,c.work?'work':'repository',c,390,true,await page.locator('.mobile-project').textContent());await context.close();
  child.kill('SIGTERM');await done;write(c.key+'-listener.json',{stderr});child=null;
 }
 assert.deepEqual(result.external,[]);result.outcome='passed';
}catch(e){result.error=String(e.stack||e);if(active&&!active.isClosed())await active.screenshot({path:path.join(config.output,'failure.png')}).catch(()=>{});process.exitCode=1;}finally{child?.kill('SIGTERM');await browser?.close();await new Promise(r=>files?files.close(r):r());write('result.json',result);console.log(JSON.stringify({outcome:result.outcome,checks:result.checks.length,error:result.error}));}})();
