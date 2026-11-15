// Real live GET navigation with held responses, not simulated page content.
// No authoring, mutations, provider calls or human acceptance are supplied.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),crypto=require('node:crypto');
const {createOutput,citationReady}=require('../../viewer-design/product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));createOutput(config.output);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const result={outcome:'failed',checks:[],human_acceptance:'unobserved'};
let browser,child,exit,page,logs='',stdout='';
const check=name=>{result.checks.push(name);console.log(name);};
const bounded=(promise,name)=>Promise.race([promise,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(Error('Timed out: '+name)),30000);timer.unref();})]);
async function frame(page,selector='main > section'){
 return page.locator(selector).evaluate(n=>{const h=n.querySelector('h1'),r=n.getBoundingClientRect(),v=h.getBoundingClientRect(),s=getComputedStyle(h);return{x:r.x,width:r.width,text:h.textContent,heading:{x:v.x,y:v.y,font:s.font,spacing:s.letterSpacing}};});
}
function sameFrame(pending,actual){
 for(const field of ['x','width'])assert(Math.abs(pending[field]-actual[field])<1,'shared page gutter '+field);
 for(const field of ['x','y'])assert(Math.abs(pending.heading[field]-actual.heading[field])<1,'same loading/actual heading position '+field);
 assert.equal(pending.heading.font,actual.heading.font);assert.equal(pending.heading.spacing,actual.heading.spacing);
}
async function main(){
 try{
  result.candidate=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
  result.diff_sha256=hash(execFileSync('git',['diff','HEAD','--','rebuild']));
  result.driver_sha256=hash(fs.readFileSync(__filename));result.config_sha256=hash(fs.readFileSync(process.argv[2]));
  result.viewer_sha256=hash(fs.readFileSync(config.viewer));
  child=spawn(config.viewer,['--runtime',config.runtime,'--project',config.project,'--bind','127.0.0.1:0']);
  child.stderr.on('data',b=>logs+=b);child.stdout.on('data',b=>stdout+=b);
  exit=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
  const base=await new Promise((ok,no)=>{const timer=setTimeout(()=>no(Error(logs||'listener timeout')),15000);child.once('error',no);child.stderr.on('data',()=>{const m=logs.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);ok(m[0]);}});});
  const chromium=require(config.playwright).chromium;
  browser=config.cdp_endpoint?await chromium.connectOverCDP(config.cdp_endpoint):await chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
  result.browser_mode=config.cdp_endpoint?'connected-headful-isolated-context':'launched-headless';
  const context=await browser.newContext();context.setDefaultTimeout(30000);await context.route('**/*',r=>r.request().url().startsWith(base)?r.continue():r.abort());
  page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const url=(view,locale='ko',extra={})=>{const u=new URL(base);u.search=new URLSearchParams({project:config.project,view,locale,language:locale,...extra});return u.href;};
  const ready=async(view,locale='ko')=>{await page.waitForFunction(({view,locale})=>document.body.dataset.view===view&&document.documentElement.lang===locale&&document.body.dataset.readerEnhanced==='true'&&!document.querySelector('.page-loading,.page-navigation-error'),{view,locale});};
  const nav=async view=>{if(page.viewportSize().width<=980&&!await page.locator('.project-shell').evaluate(n=>n.open))await page.locator('.project-shell > summary').click();await page.locator('.project-navigation a[data-destination="'+view+'"]').click();};
  // This marker can survive only an enhanced navigation, never a document load.
  await page.goto(url('overview'));
  result.font_qualification=config.expected_ko_font?'required':'not_asserted';
  if(config.expected_ko_font){
   const cdp=await context.newCDPSession(page);await cdp.send('DOM.enable');await cdp.send('CSS.enable');
   const document=await cdp.send('DOM.getDocument',{depth:0});const node=await cdp.send('DOM.querySelector',{nodeId:document.root.nodeId,selector:'.project-meta'});
   result.korean_platform_fonts=(await cdp.send('CSS.getPlatformFontsForNode',{nodeId:node.nodeId})).fonts;
   assert(result.korean_platform_fonts.some(f=>f.familyName.startsWith(config.expected_ko_font)&&f.glyphCount>0),'the actual browser must render Korean with its configured font');
   await cdp.detach();result.font_qualification='passed';check('configured-korean-fonts');
  }
  await page.evaluate(()=>window.navigationTestIdentity=1);
  for(const locale of ['ko','en'])for(const width of [390,768,1440]){
   await page.setViewportSize({width,height:900});await page.goto(url('overview',locale));
   const overviewFrame=await frame(page);
   let release,arrived;const held=new Promise(r=>release=r),arrival=new Promise(r=>arrived=r);
   const matcher=u=>u.pathname==='/'&&u.searchParams.get('view')==='repository';
   await context.route(matcher,async r=>{const response=await r.fetch();arrived();await held;await r.fulfill({response}).catch(()=>{});});
   if(width<=980)await page.locator('.project-shell > summary').click();
   console.log('requesting '+locale+' '+width);await nav('repository');await bounded(arrival,'repository response '+locale+' '+width);
   assert.equal(new URL(page.url()).searchParams.get('view'),'repository');
   assert.equal(await page.locator('#viewer-content').getAttribute('aria-busy'),'true');
   assert.equal(await page.locator('.page-loading').count(),1);assert.equal(await page.locator('[data-reader-publication]').count(),0,'loading does not carry the old article or evidence');
   assert.match(await page.locator('.page-loading-message').textContent(),locale==='ko'?/불러오는/:/Loading/);
   assert.equal(await page.locator('.project-navigation [aria-current=page]').getAttribute('data-destination'),'repository');
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
   const loadingGeometry=await frame(page,'.page-loading');
   await page.emulateMedia({reducedMotion:'reduce'});
   assert.equal(await page.locator('.page-loading-spinner').evaluate(n=>getComputedStyle(n).animationName),'none');
   assert.equal(await page.locator('.page-loading-skeleton span').first().evaluate(n=>getComputedStyle(n,'::after').animationName),'none');
   await page.emulateMedia({reducedMotion:'no-preference'});
   assert.equal(await page.locator('.page-loading-spinner').evaluate(n=>getComputedStyle(n).animationName),'viewer-page-spin');
   const motion=()=>page.locator('.page-loading').evaluate(n=>{const a=n.querySelector('.page-loading-spinner'),b=n.querySelector('.page-loading-skeleton span');return{spin:getComputedStyle(a).transform,shimmer:getComputedStyle(b,'::after').transform,time:a.getAnimations()[0].currentTime};});
   const first=await motion(),pixels=await page.locator('.page-loading-skeleton').screenshot();await page.waitForTimeout(230);const second=await motion();
   assert(second.time>first.time&&first.spin!==second.spin&&first.shimmer!==second.shimmer,'actual spinner and skeleton advance while the page response is held');
   assert.notDeepEqual(await page.locator('.page-loading-skeleton').screenshot(),pixels,'body loading pixels actually change');
   await page.screenshot({path:path.join(config.output,`loading-${locale}-${width}.png`)});
   release();await ready('repository',locale);await context.unroute(matcher);
   const actual=await frame(page);sameFrame(loadingGeometry,actual);
   for(const field of ['x','width'])assert(Math.abs(overviewFrame[field]-actual[field])<1,'Overview and document share the same horizontal measure');
   assert.equal(await page.locator('.source-scope-icon,[data-source-scope-action]').count(),0);
   check(`immediate-loading-${locale}-${width}`);
   for(const target of ['overview','work','detail']){
    let releaseFrame,frameArrived;const heldFrame=new Promise(r=>releaseFrame=r),frameArrival=new Promise(r=>frameArrived=r);
    const view=target==='detail'?'work':target;
    const targetRoute=u=>u.pathname==='/'&&u.searchParams.get('view')===view&&u.searchParams.has('work')===(target==='detail');
    await context.route(targetRoute,async r=>{const response=await r.fetch();frameArrived();await heldFrame;await r.fulfill({response}).catch(()=>{});});
    if(target==='detail')await page.locator('a.work-row').first().click();else await nav(view);
    await bounded(frameArrival,'frame '+target);const pendingFrame=await frame(page,'.page-loading');
    assert(pendingFrame.text,'known Project/Work title is present while loading');
    await page.screenshot({path:path.join(config.output,`loading-${target}-${locale}-${width}.png`)});
    releaseFrame();await ready(view,locale);await context.unroute(targetRoute);
    const actualFrame=await frame(page);sameFrame(pendingFrame,actualFrame);assert.equal(pendingFrame.text,actualFrame.text,'same known header content');
    for(const field of ['x','width'])assert(Math.abs(overviewFrame[field]-actualFrame[field])<1,'all pages share the same horizontal measure');
    check(`matching-${target}-header-${locale}-${width}`);
   }
  }
  await page.setViewportSize({width:1440,height:900});await page.goto(url('overview'));
  // Observe the entire real navigation, including response parsing and mounting.
  // Gaps/tasks are measurements, not an invented universal frame-rate guarantee.
  await page.evaluate(()=>{
   const value=window.navigationAnimationObservation={frames:[],longTasks:[],stop:false};
   value.observer=new PerformanceObserver(list=>{for(const e of list.getEntries())value.longTasks.push({start:e.startTime,duration:e.duration});});value.observer.observe({type:'longtask'});
   const tick=t=>{const n=document.querySelector('.page-loading-spinner');value.frames.push({t,transform:n?getComputedStyle(n).transform:null});if(!value.stop&&value.frames.length<5000)requestAnimationFrame(tick);};requestAnimationFrame(tick);
  });
  await nav('repository');await ready('repository');await page.waitForTimeout(50);
  result.real_navigation=await page.evaluate(()=>{const v=navigationAnimationObservation;v.stop=true;v.observer.disconnect();const frames=v.frames;return{frames,long_tasks:v.longTasks,max_frame_gap_ms:Math.max(0,...frames.slice(1).map((n,i)=>n.t-frames[i].t))};});
  assert(new Set(result.real_navigation.frames.filter(f=>f.transform).map(f=>f.transform)).size>2,'motion advances during the real unheld data load');check('real-response-animation-through-parse-and-mount');
  await page.goto(url('overview'));
  await page.evaluate(()=>window.navigationTestIdentity=1);
  let releaseOld,oldArrived;const heldOld=new Promise(r=>releaseOld=r),oldArrival=new Promise(r=>oldArrived=r);
  const repository=u=>u.pathname==='/'&&u.searchParams.get('view')==='repository';
  await context.route(repository,async r=>{const response=await r.fetch();oldArrived();await heldOld;await r.fulfill({response}).catch(()=>{});});
  await nav('repository');await bounded(oldArrival,'superseded response');await nav('work');await ready('work');
  releaseOld();await context.unroute(repository);await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  assert.equal(await page.evaluate(()=>window.navigationTestIdentity),1);assert.equal(await page.locator('script[data-viewer-reading]').count(),1);assert.equal(await page.locator('body').getAttribute('data-view'),'work');check('superseded-response-does-not-overwrite');
  // Current mounted list handlers must submit real GET search and sort values.
  await page.locator('input[name=q]').fill('privacy');await page.locator('input[name=q]').press('Enter');
  await page.waitForFunction(()=>new URL(location.href).searchParams.get('q')==='privacy'&&!document.querySelector('.page-loading'));
  assert.equal(await page.locator('input[name=q]').inputValue(),'privacy');
  const sort=page.locator('select[name=sort]'),options=await sort.locator('option').evaluateAll(xs=>xs.map(x=>x.value));
  const choice=options.find(x=>x!==new URL(page.url()).searchParams.get('sort'));await sort.selectOption(choice);
  await page.waitForFunction(value=>new URL(location.href).searchParams.get('sort')===value&&!document.querySelector('.page-loading'),choice);check('search-sort-after-remount');
  await page.locator('select[data-language-picker]').selectOption({label:'English'});await ready('work','en');check('language-after-remount');
  // A failed request leaves the selected URL, truthful error and deliberate retry.
  await context.route(repository,r=>r.abort());await nav('repository');await page.locator('.page-navigation-error').waitFor();
  assert.equal(new URL(page.url()).searchParams.get('view'),'repository');assert.equal(await page.locator('.doc').count(),0);
  await context.unroute(repository);await page.locator('[data-retry-page]').click();await ready('repository','en');check('network-error-retry');
  await context.route(repository,r=>r.fulfill({status:200,contentType:'text/html',body:'<h1>invalid response</h1>'}));
  await nav('repository');await page.locator('.page-navigation-error').waitFor();await context.unroute(repository);
  await page.locator('[data-retry-page]').click();await ready('repository','en');check('malformed-response-refused');
  // Evidence remains exact native-fragment navigation, with fresh handlers after
  // several remounts; full page loading must never intercept it.
  const evidence=page.locator('.doc .paragraph-evidence-action').first();
  if(await evidence.count()){
   await evidence.click();const point=page.locator('.source-point[data-selected=true]');await citationReady(point);
   assert.equal(await page.locator('.page-loading').count(),0);await page.keyboard.press('Escape');check('source-reader-after-remount');
  }
  await nav('work');await ready('work','en');
  const row=page.locator('a.work-row').first();assert(await row.count(),'a real recorded Work is required');
  await row.scrollIntoViewIfNeeded();const rowId=await row.getAttribute('id'),top=await row.evaluate(n=>n.getBoundingClientRect().top);
  await row.click();await page.waitForFunction(()=>new URL(location.href).searchParams.has('work')&&!document.querySelector('.page-loading'));
  await page.goBack();await ready('work','en');await page.waitForFunction(id=>document.activeElement?.id===id,rowId);
  assert(Math.abs(await page.locator('#'+rowId).evaluate(n=>n.getBoundingClientRect().top)-top)<3);check('back-restores-list-position-focus');
  await page.goForward();await page.waitForFunction(()=>new URL(location.href).searchParams.has('work')&&!document.querySelector('.page-loading'));check('forward-revalidates-work');
  // HTTP accepts both hexadecimal cases for a Project ID. Client-side binding
  // must preserve that same identity rather than reject the normalized response.
  await page.evaluate(href=>{const a=document.createElement('a');a.href=href;a.textContent='Project selection';document.getElementById('viewer-content').append(a);a.click();},url('overview','en',{project:config.project.toUpperCase()}));
  await ready('overview','en');assert.equal(await page.locator('body').getAttribute('data-project-id'),config.project.toLowerCase());check('uppercase-project-selection');
  // A different bundle must use native navigation. Its fetched script is never
  // evaluated inside the currently executing page, even before that fallback.
  let foreignBundle=true;await page.evaluate(()=>window.navigationTestIdentity=2);
  await context.route(repository,async r=>{const response=await r.fetch();if(foreignBundle){foreignBundle=false;const body=(await response.text()).replace('<script data-viewer-reading>','<script data-viewer-reading>window.untrustedFetchedScript=true;');await r.fulfill({response,body});}else await r.fulfill({response});});
  const fallbackStart=Date.now();await nav('repository');await ready('repository','en');result.changed_bundle_fallback_ms=Date.now()-fallbackStart;await context.unroute(repository);
  assert.equal(await page.evaluate(()=>window.navigationTestIdentity),undefined);assert.equal(await page.evaluate(()=>window.untrustedFetchedScript),undefined);check('changed-bundle-native-fallback-without-evaluation');
  // Modified clicks keep native separate-document behavior.
  const popupPromise=context.waitForEvent('page');await page.locator('.project-navigation a[data-destination=overview]').click({modifiers:['Control']});
  const popup=await popupPromise;await popup.waitForLoadState();assert.equal(new URL(popup.url()).searchParams.get('view'),'overview');await popup.close();check('modified-click-native-tab');
  assert.deepEqual(errors,[]);
  // A headful Chrome can exit when its last review window closes. Open the
  // script-free review window before releasing our enhanced context.
  const native=await browser.newContext({javaScriptEnabled:false});const nativePage=await native.newPage();page=nativePage;await nativePage.goto(url('overview'));await context.close();
  await nativePage.locator('.project-navigation a[data-destination=work]').click();assert.equal(new URL(nativePage.url()).searchParams.get('view'),'work');await native.close();check('no-script-native-navigation');
  result.outcome='passed';
 }catch(error){result.error=error.stack||String(error);result.page=await page?.evaluate(()=>({url:location.href,body:document.body.dataset,main:document.getElementById('viewer-content')?.textContent.slice(0,1200)})).catch(()=>null);await page?.screenshot({path:path.join(config.output,'failure.png')}).catch(()=>{});throw error;}
 finally{
  await browser?.close();if(child){child.kill('SIGTERM');result.listener=await exit;}
  fs.writeFileSync(path.join(config.output,'viewer.stdout'),stdout);fs.writeFileSync(path.join(config.output,'viewer.stderr'),logs);
  fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
 }
 console.log(JSON.stringify({outcome:result.outcome,checks:result.checks.length}));
}
main().catch(error=>{console.error(error);process.exitCode=1;});
