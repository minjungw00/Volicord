// Actual local-server shell/contents controls. Reference screenshots remain immutable.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),crypto=require('node:crypto');
const {createOutput}=require('../../viewer-design/product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));createOutput(config.output);
const write=(name,data)=>fs.writeFileSync(path.join(config.output,name),data,{flag:'wx'});
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const result={outcome:'failed',scope:'technical shell/navigation; dependent design comparison; no human or assistive acceptance',checks:[],requests:[],identities:{viewer:hash(config.viewer),chromium:hash(config.chromium)}};
let browser,listener;
const frame=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
async function presentation(page,reference=false){return page.evaluate(reference=>{
 const properties=(selector,keys)=>{const n=document.querySelector(selector);if(!n)throw Error('missing design surface '+selector);const s=getComputedStyle(n);return Object.fromEntries(keys.map(k=>[k,s[k]]));};
 return {title:properties(reference?'#repo-document h1':'.doc-head h1',['fontSize','fontWeight','lineHeight','letterSpacing','fontFamily','color']),
 lead:properties(reference?'#repo-document .lead':'.doc-head .lead',['fontSize','lineHeight','fontFamily','color']),
 rail:properties(reference?'.sidebar':'.project-rail',['width','backgroundColor'])};
 },reference);}
async function start(item,runtime){
 const process=spawn(config.viewer,['--runtime',runtime,'--project',item.project,'--bind','127.0.0.1:0']);let stderr='',stdout='';
 process.stdout.on('data',b=>stdout+=b);process.stderr.on('data',b=>stderr+=b);
 const exit=new Promise(r=>process.once('close',(code,signal)=>r({code,signal})));
 const base=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('listener timeout '+stderr)),10000);process.once('error',reject);process.stderr.on('data',()=>{const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});});
 return {base,async stop(){process.kill('SIGTERM');const status=await exit;write(item.key+'-listener.json',JSON.stringify({...status,stdout,stderr}));}};
}
(async()=>{try{
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
 for(const item of config.cases){
  const runtime=path.join(config.output,item.key+'-runtime');fs.cpSync(item.runtime,runtime,{recursive:true,errorOnExist:true,force:false});listener=await start(item,runtime);
  const context=await browser.newContext({locale:'ko-KR'});await context.route('**/*',route=>{if(new URL(route.request().url()).origin===new URL(listener.base).origin)return route.continue();result.requests.push(route.request().url());return route.abort();});
  context.setDefaultTimeout(10000);context.setDefaultNavigationTimeout(10000);
  const page=await context.newPage();const address=(view='repository',extra='')=>listener.base+`?project=${item.project}&view=${view}&locale=${item.locale||'ko'}&language=${item.language||'ko'}`+extra;
  for(const width of config.widths||[1440,1201,1200,1199,1024,1012,1011,981,980,979,768,761,760,759,390,320]){
   console.log('width',item.key,width);
   await page.setViewportSize({width,height:900});await page.goto(address());await page.evaluate(()=>document.fonts.ready);await frame(page);
   await page.screenshot({path:path.join(config.output,`${item.key}-${width}-initial.png`)});
   if(item.hierarchy){
    const publication=JSON.parse(fs.readFileSync(item.public));
    const checks=await require('./reader_hierarchy.cjs').verify(page,item.hierarchy,publication,width===(config.widths||[1440])[0]);
    write(`${item.key}-${width}-hierarchy.json`,JSON.stringify(checks));
    await page.goto(address());await page.evaluate(()=>document.fonts.ready);await frame(page);
   }
   const initial=await page.evaluate(()=>({overflow:document.documentElement.scrollWidth-innerWidth,rail:getComputedStyle(document.querySelector('.project-rail')).width,article:document.querySelector('.doc').clientWidth,nav:[...document.querySelectorAll('.project-navigation [aria-current=page]')].map(n=>n.dataset.destination),font:getComputedStyle(document.body).fontFamily,ink:getComputedStyle(document.body).color,toc:document.querySelector('.document-nav')?.getBoundingClientRect().toJSON(),contents:document.querySelector('.document-nav>summary')?.getClientRects().length}));
   if(config.reference){
    const ref=await browser.newContext({viewport:{width,height:900},locale:'ko-KR'});
    await ref.route('**/*',route=>new URL(route.request().url()).origin==='http://reference.invalid'?route.fulfill({contentType:'text/html',body:fs.readFileSync(path.join(config.reference,'repository.html'))}):route.abort());
    const oracle=await ref.newPage();await oracle.goto('http://reference.invalid/repository.html');await oracle.evaluate(()=>document.fonts.ready);await frame(oracle);
    const actual=await presentation(page),expected=await presentation(oracle,true);
    const hierarchy=await oracle.locator('#repo-document .desktop-map').first().evaluate(n=>({
     nodes:[...n.querySelectorAll('.map-node')].map(n=>({name:n.querySelector('.node-name')?.textContent,identifier:n.querySelector('.node-code')?.textContent})),
     relations:[...n.querySelectorAll('.edge-label')].map(n=>n.textContent),
     section:n.closest('.article-section').querySelector('h2')?.textContent,
     helper:n.closest('.article-section').querySelector('.section-heading p')?.textContent
    }));
    write(`${item.key}-${width}-reference-comparison.json`,JSON.stringify({reference_sha256:hash(path.join(config.reference,'repository.html')),actual,expected,reference_hierarchy:hierarchy,declared_expectation:item.hierarchy||null,comparison_scope:'dependent visual inspection; record/count/content differences are not fidelity failures'}));
    await oracle.screenshot({path:path.join(config.output,`${item.key}-${width}-reference.png`)});await ref.close();
    if(!config.reference_report_only){assert.deepEqual(actual,expected,'unchanged reference typography and rail');
     await page.locator('.project-rail').evaluate(n=>n.style.backgroundColor='purple');assert.notDeepEqual(await presentation(page),expected,'palette substitution must fail');await page.locator('.project-rail').evaluate(n=>n.style.removeProperty('background-color'));
    }
   }
   assert.ok(initial.overflow<=1,'ordinary overflow');assert.deepEqual(initial.nav,['repository']);assert.equal(initial.ink,'rgb(21, 46, 64)');
   if(width<=980){
    const trigger=page.locator('.project-shell>summary');await trigger.click();await frame(page);
    assert.equal(await page.locator('main').evaluate(n=>n.inert),true);assert.equal(await page.locator('.project-rail').getAttribute('aria-modal'),'true');
    const before=await page.locator('.project-rail').evaluate(n=>n.contains(document.activeElement));assert.equal(before,true);
    for(let n=0;n<24;n++){await page.keyboard.press(n%2?'Shift+Tab':'Tab');assert.equal(await page.locator('.project-rail').evaluate(n=>n.contains(document.activeElement)),true);}
    await page.keyboard.press('Escape');assert.equal(await trigger.evaluate(n=>n===document.activeElement),true);assert.equal(await page.locator('main').evaluate(n=>n.inert),false);
    await trigger.click();await page.setViewportSize({width:1200,height:900});await frame(page);assert.equal(await page.locator('main').evaluate(n=>n.inert),false);assert.equal(await page.locator('.project-rail').getAttribute('aria-modal'),null);await page.setViewportSize({width,height:900});await frame(page);
   }
   const toc=page.locator('.document-nav').first();
   if(await toc.count()){
    const links=toc.locator('.toc-primary a');assert.ok(await links.count());
    const ids=await links.evaluateAll(nodes=>nodes.map(n=>n.hash.slice(1)));assert.equal(new Set(ids).size,ids.length);
    const target=ids.at(-1);if(await toc.locator('summary').isVisible())await toc.locator('summary').click();
    await links.last().click();await frame(page);
    const geometry=await page.locator(`[id="${target}"]`).evaluate(n=>({top:n.getBoundingClientRect().top,focus:n===document.activeElement}));assert.ok(geometry.focus,'explicit heading focus');
    assert.ok(geometry.top>= (width<=980?56:0)+55,'heading below sticky contents');
    await page.goto(address()+'#'+target);await frame(page);assert.equal(await page.locator(`[id="${target}"]`).evaluate(n=>n===document.activeElement),true);
    await page.evaluate(()=>window.scrollTo(0,document.documentElement.scrollHeight));await frame(page);assert.equal(await links.last().getAttribute('aria-current'),'location');
    // Modified navigation must leave originating focus/scroll/open state unchanged.
    if(await toc.locator('summary').isVisible())await toc.locator('summary').click();
    await links.first().focus();await frame(page);
    const baseline=await page.evaluate(()=>({hash:location.hash,y:scrollY,focus:document.activeElement?.outerHTML,open:document.querySelector('.document-nav').open}));
    console.log('modifier',width);const popup=context.waitForEvent('page',{timeout:10000});await links.first().click({modifiers:['Control']});const tab=await popup;await tab.waitForLoadState('load',{timeout:10000});await tab.close();await page.bringToFront();await frame(page);
    assert.deepEqual(await page.evaluate(()=>({hash:location.hash,y:scrollY,focus:document.activeElement?.outerHTML,open:document.querySelector('.document-nav').open})),baseline);
   }
   result.checks.push({case:item.key,width,...initial});
  }
  console.log('destinations',item.key);
  for(const view of ['overview','work']){await page.goto(address(view));assert.equal(await page.locator('.document-nav').count(),0);assert.equal(await page.locator('.project-navigation [aria-current=page]').getAttribute('data-destination'),view);}
  if(item.work){await page.goto(address('work','&work='+item.work));assert.equal(await page.locator('.project-navigation [aria-current=page]').getAttribute('data-destination'),'work');}
  console.log('nojs',item.key);
  const nojs=await browser.newContext({javaScriptEnabled:false,viewport:{width:390,height:900}});nojs.setDefaultTimeout(10000);const native=await nojs.newPage();await native.goto(address());
  const nativeTOC=native.locator('.document-nav').first();if(await nativeTOC.count()){if(await nativeTOC.locator('summary').isVisible())await nativeTOC.locator('summary').click();await nativeTOC.locator('a').first().click();assert.ok(new URL(native.url()).hash);}
  await nojs.close();await context.close();console.log('stop',item.key);await listener.stop();listener=null;
 }
 assert.deepEqual(result.requests,[]);result.outcome='passed';
}catch(error){result.error=String(error);console.log(String(error));process.exitCode=1;}finally{await browser?.close();await listener?.stop();write('result.json',JSON.stringify(result,null,2));}})();
