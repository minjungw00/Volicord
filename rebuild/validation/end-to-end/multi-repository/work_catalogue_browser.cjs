// Public local-server catalogue controls. Synthetic inputs stay outside real homes.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {spawn}=require('node:child_process');
const {createOutput}=require('../../viewer-design/product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));createOutput(config.output);
const write=(name,data)=>fs.writeFileSync(path.join(config.output,name),typeof data==='string'?data:JSON.stringify(data,null,2),{flag:'wx'});
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const result={outcome:'failed',scope:'technical public query/navigation and dependent reference comparison; human/assistive acceptance unobserved',viewer_sha256:hash(config.viewer),checks:[],measurements:[],external:[]};
let browser,listener;
const frame=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
async function start(item,runtime){
 const child=spawn(config.viewer,['--runtime',runtime,'--project',item.project,'--bind','127.0.0.1:0']);let stderr='',stdout='';
 child.stderr.on('data',b=>stderr+=b);child.stdout.on('data',b=>stdout+=b);
 const exit=new Promise(r=>child.once('close',(code,signal)=>r({code,signal})));
 const base=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('listener timeout '+stderr)),10000);child.once('error',reject);child.stderr.on('data',()=>{const match=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
 return {base,async stop(){child.kill('SIGTERM');const status=await exit;write(item.key+'-listener.json',{...status,stdout,stderr});}};
}
(async()=>{try{
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
 for(const item of config.cases){
  const runtime=path.join(config.output,item.key+'-runtime');fs.cpSync(item.runtime,runtime,{recursive:true,force:false,errorOnExist:true});listener=await start(item,runtime);
  const context=await browser.newContext({locale:'ko-KR'});context.setDefaultTimeout(10000);
  await context.route('**/*',r=>new URL(r.request().url()).origin===new URL(listener.base).origin?r.continue():(result.external.push(r.request().url()),r.abort()));
  const page=await context.newPage();const address=(view='work',extra='')=>listener.base+`?project=${item.project}&view=${view}&locale=${item.locale||'en'}&language=${item.language||'en'}`+extra;
  for(const width of config.widths||[1440,1024,981,980,768,761,760,390,320]){
   await page.setViewportSize({width,height:900});
   for(const view of ['overview','work']){
    await page.goto(address(view));await page.evaluate(()=>document.fonts.ready);await frame(page);
    await page.screenshot({path:path.join(config.output,`${item.key}-${view}-${width}-initial.png`)});
    const geometry=await page.evaluate(()=>({overflow:document.documentElement.scrollWidth-innerWidth,rows:document.querySelectorAll('.work-row').length,contents:document.querySelectorAll('.document-nav').length,nav:[...document.querySelectorAll('.project-navigation [aria-current=page]')].map(n=>n.dataset.destination),font:getComputedStyle(document.body).fontFamily,main:document.querySelector('main').getBoundingClientRect().toJSON()}));
    assert.ok(geometry.overflow<=1,JSON.stringify({width,view,...geometry}));assert.equal(geometry.contents,0);assert.deepEqual(geometry.nav,[view]);
    if(item.synthetic){assert.equal(geometry.rows,view==='work'?8:5);}
    result.checks.push({case:item.key,view,width,...geometry});
   }
   if(width<=980){await page.locator('.project-shell>summary').click();await frame(page);assert.equal(await page.locator('main').evaluate(n=>n.inert),true);await page.keyboard.press('Escape');await frame(page);assert.equal(await page.locator('main').evaluate(n=>n.inert),false);}
  }
  if(item.synthetic){
   await page.setViewportSize({width:1440,height:900});await page.goto(address());
   await page.locator('#work-search').fill('needle');await page.locator('.catalog-controls select').selectOption('oldest');await page.locator('.catalog-controls button').click();
   assert.equal(await page.locator('[data-result-count]').getAttribute('data-result-count'),'17');
   const expected=Array.from({length:17},(_,n)=>item.ids[`control-${String(n*2).padStart(3,'0')}`]);
   const selected=[];for(let n=0;n<3;n++){
    const rows=await page.locator('.work-row').evaluateAll(nodes=>nodes.map(n=>n.dataset.workId));selected.push(...rows);
    if(n<2)await page.locator(`.pagination a[aria-label="Page ${n+2}"]`).click();
   }
   assert.deepEqual(selected,expected);assert.ok(new URL(page.url()).searchParams.has('basis'));
   await page.locator('.filter-tab').filter({hasText:'Completed'}).click();
   assert.equal(await page.locator('[data-result-count]').getAttribute('data-result-count'),'6');assert.equal(new URL(page.url()).searchParams.get('page'),'0');
   await page.locator('#work-search').fill('no-such-needle');await page.locator('.catalog-controls button').click();assert.equal(await page.locator('.work-row').count(),0);
   const duplicate=await page.goto(address('overview',`&project=${item.empty}`)); // Duplicate parameters must be refused.
   assert.equal(duplicate.status(),400);
   await page.goto(listener.base+`?project=${item.empty}&view=overview`);assert.match(await page.locator('main').innerText(),/No Work has been recorded/);
   await page.goto(listener.base+'?project='+('0'.repeat(32))+'&view=overview');assert.equal((await page.locator('body').innerText()).includes('Synthetic'),false);
  }
  for(let n=0;n<5;n++){
   const started=performance.now(),response=await context.request.get(address());const body=await response.body();assert.equal(response.status(),200);
   result.measurements.push({case:item.key,workload:item.synthetic?'34 Goals / 66 admitted card publications / 33 Checkpoints':'genuine permitted retained records; sparse history',sample:n,milliseconds:performance.now()-started,html_bytes:body.length});
  }
  const native=await browser.newContext({javaScriptEnabled:false});const nojs=await native.newPage();await nojs.goto(address());
  if(item.synthetic){await nojs.locator('#work-search').fill('needle');await nojs.locator('.catalog-controls button').click();assert.equal(await nojs.locator('[data-result-count]').getAttribute('data-result-count'),'17');await nojs.locator('.pagination a').filter({hasText:'2'}).click();assert.equal(await nojs.locator('.work-row').count(),8);}
  await native.close();await context.close();await listener.stop();listener=null;
 }
 if(config.reference){
  for(const width of [1440,1024,768,390,320])for(const name of ['index','works']){
   const reference=await browser.newContext();await reference.route('http://reference.invalid/**',route=>route.fulfill({contentType:'text/html',body:fs.readFileSync(path.join(config.reference,name+'.html'))}));
   const page=await reference.newPage();await page.setViewportSize({width,height:900});await page.goto('http://reference.invalid/'+name+'.html');await page.evaluate(()=>document.fonts.ready);await frame(page);
   assert.equal(await page.locator('#utility-view').isVisible(),true,'reference route must be active');
   await page.screenshot({path:path.join(config.output,`reference-${name}-${width}.png`)});await reference.close();
  }
 }
 assert.deepEqual(result.external,[]);result.outcome='passed';
}catch(error){result.error=String(error.stack||error);process.exitCode=1;}finally{if(listener)await listener.stop();if(browser)await browser.close();write('result.json',result);console.log(JSON.stringify({outcome:result.outcome,checks:result.checks.length,error:result.error}));}})();
