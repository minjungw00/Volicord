// Native file/HTTP fallback and camera input transitions on the same sealed files.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const {citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('./product_review_controls.cjs');
(async()=>{
 const c=JSON.parse(fs.readFileSync(process.argv[2])),root=path.resolve(c.output),out=path.resolve(c.mode_output);
 assert.ok(out.includes('/rebuild/.local/'));fs.mkdirSync(out);
 const manifest=JSON.parse(fs.readFileSync(path.join(root,'manifest.json'))),rows=[];
 const server=http.createServer((req,res)=>{const file=path.resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname));if(!file.startsWith(root+path.sep)||!fs.existsSync(file)){res.writeHead(404);res.end();return;}res.setHeader('Content-Type','text/html; charset=utf-8');res.end(fs.readFileSync(file));});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const base='http://127.0.0.1:'+server.address().port;
 const browser=await require(c.playwright).chromium.launch({executablePath:c.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});
 let failure;
 try{
  for(const candidate of manifest.candidates){
   const source=c.cases.find(x=>candidate.id.startsWith(x.key+'-')),pub=JSON.parse(fs.readFileSync(source.public)),d=pub.diagrams.find(x=>x.id===source.diagram);
   const e=(d.meaning.relations||d.meaning.groups?.flatMap(g=>g.messages))[0],unit=pub.prose[e.label.unit];
   const citation=unit.citations.find(q=>q.claim_start_byte<=e.label.start_byte&&q.claim_end_byte>=e.label.end_byte);
   const expected=fs.readFileSync(path.join(source.repository,citation.locator)).subarray(citation.start_byte,citation.end_byte).toString();
   for(const mode of ['file-no-js','http-no-js','http-scripted'])for(const width of [1440,390,320]){
    const context=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:!mode.includes('no-js')});const page=await context.newPage();page.setDefaultTimeout(6000);
    const external=[],errors=[];page.on('pageerror',e=>errors.push(String(e)));await context.route('**/*',r=>{const u=r.request().url();if(u.startsWith('file:')||u.startsWith(base+'/'))return r.continue();external.push(u);return r.abort();});
    try{
     const url=(mode.startsWith('file')?pathToFileURL(path.join(root,candidate.file)).href:base+'/'+candidate.file)+'#'+(source.work?'work-'+source.work:'repository-explanation');
     await page.goto(url);await page.evaluate(()=>document.fonts.ready);
     const figure=page.locator(`.admitted-diagram[data-semantic-fingerprint="${d.semantic_fingerprint}"]`),id=await figure.getAttribute('id');
     await figure.evaluate(n=>n.scrollIntoView({block:'start'}));
     assert.equal(await figure.locator('.diagram-claim:visible').count(),0,'ordinary entry has no selected claim');
     if(mode.includes('no-js'))assert.equal(await figure.locator('.spatial-enhancement:visible').count(),0);
     await page.screenshot({path:path.join(out,`${candidate.id}-${mode}-${width}-initial.png`)});
     const element=figure.locator(`[id="${id}-${e.id}"]`);await element.focus();await page.keyboard.press('Enter');
     const claim=page.locator('[id="'+(await element.getAttribute('href')).slice(1)+'"]'),link=claim.locator('a.evidence-link').first();await link.focus();await page.keyboard.press('Enter');
     const point=await citationPoint(page,await link.getAttribute('href'));assert.equal(await(await openEmbeddedCode(point)).textContent(),expected);await returnFromEmbeddedCode(point);
     const back=point.locator(':scope > .source-origin-target:target a.return, :scope > .point-bar a.return').filter({visible:true}).first();await back.focus();await page.keyboard.press('Enter');
     assert.equal(new URL(page.url()).hash,'#'+id+'-'+e.id);
     if(mode==='http-scripted'&&candidate.navigation==='camera'){
      const viewport=figure.locator('.spatial-viewport');await figure.locator('[data-world-action="reset"]').click();
      // Pointer drag uses an empty background point, not a claim or text origin.
      await viewport.evaluate(n=>n.scrollIntoView({block:'center'}));
      const blank=await viewport.evaluate(n=>{const b=n.getBoundingClientRect();for(let y=Math.max(140,b.top+12);y<Math.min(innerHeight-12,b.bottom);y+=30)for(let x=b.left+12;x<Math.min(innerWidth-12,b.right);x+=30){const t=document.elementFromPoint(x,y);if(t&&n.contains(t)&&!t.closest('a,text'))return {x,y};}throw Error('no visible blank background');});
      await page.mouse.move(blank.x,blank.y);await page.mouse.down();await page.mouse.move(blank.x-140,blank.y-20,{steps:6});await page.mouse.up();
      assert.ok(await viewport.evaluate(n=>n.scrollLeft)>20,'blank-background mouse pan');
      await figure.locator('[data-world-action="expand"]').click();assert.ok(await figure.evaluate(n=>n.classList.contains('world-expanded')));await page.keyboard.press('Escape');assert.equal(await figure.evaluate(n=>n.classList.contains('world-expanded')),false);
      await figure.locator('[data-world-action="right"]').click();const selected=figure.locator('.spatial-world [data-element="'+e.id+'"]');await selected.focus();const name=await selected.getAttribute('aria-label');
      await page.setViewportSize({width:width===1440?390:1440,height:1000});assert.ok((await figure.locator('.spatial-selection').innerText()).includes(name),'resize retains selected meaning');
      await viewport.focus();const y=await page.evaluate(()=>scrollY);await page.mouse.wheel(0,200);assert.ok(await page.evaluate(()=>scrollY)>=y,'wheel does not hijack browser/page semantics');
      // Native Tab can leave the viewport; no focus trap is installed.
      await viewport.focus();await page.keyboard.press('Shift+Tab');assert.equal(await viewport.evaluate(n=>n===document.activeElement),false);
     }
     assert.deepEqual(external,[]);assert.deepEqual(errors,[]);rows.push({candidate:candidate.id,mode,width,exact_code_and_return:true});
    }catch(error){await page.screenshot({path:path.join(out,`${candidate.id}-${mode}-${width}-failure.png`)});throw error;}finally{await context.close();}
   }
  }
 }catch(e){failure=e.stack;throw e;}finally{
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:failure?'failed':'passed',failure,rows,unobserved:['physical touch hardware','browser chrome zoom','screen reader','independent first reading','live Product lifecycle']},null,2));await browser.close();await new Promise(r=>server.close(r));
 }
 console.log(JSON.stringify({status:'passed',rows:rows.length}));
})().catch(e=>{console.error(e);process.exitCode=1;});
