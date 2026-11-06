// Inspect the actual pane scroll owner, including code/metadata siblings beyond
// the selected claim's box. No Source or publication authority is simulated.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),{pathToFileURL}=require('node:url'),crypto=require('node:crypto');
const {citationPoint,citationReady}=require('./product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2])),out=path.resolve(config.output);
fs.mkdirSync(out,{recursive:false});const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const result={outcome:'failed',viewer_sha256:sha(config.viewer),driver_sha256:sha(__filename),rows:[],failures:[],unobserved:['physical touch','assistive technology','other browser engines']};
const save=()=>fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));
async function settle(page){await page.evaluate(()=>document.fonts.ready);await page.waitForTimeout(60);}
async function main(){let browser;try{
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 for(const c of config.cases){let log='';const child=spawn(config.viewer,['--runtime',c.runtime,'--project',c.project,'--bind','127.0.0.1:0']);
  try{const address=await new Promise((ok,no)=>{const timer=setTimeout(()=>no(Error(log||'listener timeout')),15000);child.on('error',no);child.stderr.on('data',b=>{log+=b;const m=log.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);ok(m[0]);}});});
   for(const width of config.widths||[1440,1024,768,390,320])for(const mode of config.modes||['live','no-js','offline'])for(const entry of ['body','diagram']){
    const scripted=mode==='live',context=await browser.newContext({viewport:{width,height:900},javaScriptEnabled:scripted}),page=await context.newPage();page.setDefaultTimeout(10000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await context.route('**/*',r=>r.request().url().startsWith(address)||r.request().url().startsWith('file:')||r.request().url().startsWith('data:')?r.continue():r.abort());
    const key=[c.key,width,mode,entry].join('-'),row={case:c.key,width,mode,entry,scroll:[]};
    try{
     const fragment=c.work?'#work-'+c.work:'#repository-explanation';await page.goto(mode==='offline'?pathToFileURL(path.resolve(c.offline)).href+fragment:address+'?view='+(c.work?'work&work='+c.work:'repository')+'&locale='+c.locale+'&language='+c.language);await settle(page);
     const selector=entry==='body'?'.doc:visible .unit > .evidence-actions > a[data-citation-targets]':'.doc:visible .diagram-claim > .evidence-actions > a[data-citation-targets]';
     const selected=await page.locator(selector).evaluateAll(actions=>actions.map(a=>{const target=document.getElementById(a.hash.slice(1)),point=target?.closest('.source-point'),code=document.getElementById(point?.dataset.citedCode);return {href:a.getAttribute('href'),id:a.dataset.origin,bytes:code?.querySelector('code')?.textContent.length||0};}).sort((a,b)=>b.bytes-a.bytes)[0]);assert(selected&&selected.bytes>300,'real long cited code required');
     // Diagram claims are disclosed only by their exact fragment, not guessed text.
     if(entry==='diagram'){await page.evaluate(id=>{location.hash='#'+id;},selected.id);await settle(page);}
     const link=page.locator(selector+'[href="'+selected.href+'"]');await link.first().click();await settle(page);
     const pane=page.locator('.appendix:visible');const point=await citationPoint(page,selected.href);if(scripted)await citationReady(point);
     const bar=pane.locator('.point-bar:visible');assert.equal(await bar.count(),1,'one selected navigation bar');
     await pane.evaluate(n=>{n.scrollTop=0;});await settle(page);
     const initialBar=await bar.boundingBox();assert(await pane.locator('.point-title:visible').first().evaluate((n,bottom)=>n.getBoundingClientRect().top>=bottom,initialBar.y+initialBar.height),'navigation reserves space above the first context');
     const max=await pane.evaluate(n=>n.scrollHeight-n.clientHeight);assert(max>240,'must scroll beyond the short claim into complete code');
     for(const offset of [0,Math.min(max,600),max]){
      await pane.evaluate((n,y)=>{n.scrollTop=y;},offset);await settle(page);
      row.scroll.push(await bar.evaluate(n=>{const b=n.getBoundingClientRect(),p=n.closest('.appendix').getBoundingClientRect(),close=n.querySelector('a.return').getBoundingClientRect();return {scroll:n.closest('.appendix').scrollTop,top:b.top,bottom:b.bottom,left:b.left,right:b.right,paneTop:p.top,paneLeft:p.left,paneRight:p.right,closeTop:close.top,closeBottom:close.bottom,hit:!!document.elementFromPoint(close.x+close.width/2,close.y+close.height/2)?.closest('a.return')};}));
     }
     await page.screenshot({path:path.join(out,key+'-deep.png')});
     assert(row.scroll.every(s=>Math.abs(s.top-s.paneTop)<=1&&s.bottom<=900&&s.left>=s.paneLeft&&s.right<=s.paneRight&&s.closeTop>=0&&s.hit),'navigation and close stay visible and clickable through the full pane scroll');
     const printHeader=await bar.elementHandle();await page.emulateMedia({media:'print'});assert.equal(await printHeader.evaluate(n=>getComputedStyle(n).position),'static','print uses ordinary flow');await page.emulateMedia({media:'screen'});await settle(page);
     let returnHash;
     if(scripted){
      const select=pane.locator('select:visible');if(await select.locator('option').count()>1){await select.selectOption({index:1});await settle(page);assert.equal(await pane.locator('.point-bar:visible').count(),1,'range switch keeps one header');}
      if(width>760){const handle=pane.locator('.evidence-resizer');await handle.press('ArrowLeft');await settle(page);const bounds=await pane.locator('.point-bar:visible').boundingBox(),p=await pane.boundingBox();assert(bounds.x>=p.x&&bounds.x+bounds.width<=p.x+p.width,'fixed header follows pane width');await handle.press('Enter');}
      returnHash=await pane.locator('.point-bar:visible a.return').getAttribute('href');await pane.locator('.point-bar:visible a.return').focus();assert.equal(await pane.locator('.point-bar:visible a.return').evaluate(n=>n===document.activeElement),true);await page.keyboard.press('Escape');
     }else{returnHash=await bar.locator('a.return').getAttribute('href');await bar.locator('a.return').click();}
     await settle(page);assert.equal(await page.locator('.appendix:visible').count(),0,'deep-scroll close returns to the document');assert.equal(await page.evaluate(()=>location.hash),returnHash,'the selected context keeps its exact return');assert.deepEqual(errors,[]);result.rows.push(row);
    }catch(e){result.failures.push({...row,error:e.stack});await page.screenshot({path:path.join(out,key+'-failure.png')}).catch(()=>{});if(config.fail_fast)throw e;}
    finally{save();await context.close();}
   }
  }finally{child.kill('SIGTERM');fs.writeFileSync(path.join(out,c.key+'-viewer.stderr'),log);}
 }
 assert.equal(result.failures.length,0);result.outcome='passed';
 }finally{save();await browser?.close();}console.log(JSON.stringify({outcome:result.outcome,conditions:result.rows.length}));}
main().catch(e=>{console.error(e);process.exitCode=1});
