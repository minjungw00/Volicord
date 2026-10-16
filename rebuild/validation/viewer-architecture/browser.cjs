// Focused disposable comparison, following the existing reader browser support.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process');
const {performance}=require('node:perf_hooks');
const {pathToFileURL}=require('node:url');
const crypto=require('node:crypto');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium}=require(config.playwright);
const out=path.resolve(config.output);fs.mkdirSync(out,{recursive:false});
const authored=JSON.parse(fs.readFileSync(path.join(config.retained,'review-input-final.json')));
const fixtures=JSON.parse(fs.readFileSync(path.join(config.retained,'maintained-review/fixtures.json'))).cases;
const results={environment:{node:process.version,platform:process.platform,cpus:require('node:os').cpus(),memory:require('node:os').totalmem(),config},samples:[],checks:[],networks:[],processes:[]};
const digest=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
results.identity={supportHead:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),driverSha256:digest(__filename),configSha256:digest(process.argv[2]),files:Object.fromEntries(['reader.rs','reader.css','data.rs','data.json','app.js','enhance.js','vue.js','package-lock.json','build.mjs','rust-reader','vue-reader'].map(name=>[name,digest(path.join(config.build,name))]))};
function stat(pid){
  const text=fs.readFileSync(`/proc/${pid}/stat`,'utf8').split(') ')[1].split(' ');
  const status=fs.readFileSync(`/proc/${pid}/status`,'utf8');
  return {cpuTicks:Number(text[11])+Number(text[12]),rssKiB:Number(status.match(/VmRSS:\s+(\d+)/)?.[1]||0)};
}
async function start(binary,args=[],stderrUrl=false){
  const started=performance.now(),proc=spawn(binary,args,{env:{...process.env,PATH:'/usr/bin:/bin'}});let stdout='',stderr='';
  const url=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('listener timeout: '+stderr)),15000);
    const inspect=()=>{const m=(stderrUrl?stderr:stdout).match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}};
    proc.stdout.on('data',b=>{stdout+=b;inspect();});proc.stderr.on('data',b=>{stderr+=b;inspect();});
    proc.on('error',reject);proc.once('exit',code=>{clearTimeout(timer);reject(Error('listener exit '+code+stderr));});});
  return {proc,url,startupMs:performance.now()-started,async stop(){await new Promise(resolve=>{proc.once('exit',(code,signal)=>{results.processes.push({binary,args,code,signal,termination:'explicit SIGTERM cleanup',stdout,stderr});resolve();});proc.kill('SIGTERM');});}};
}
async function painted(page){await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));}
async function metrics(page,session){
  const pm=Object.fromEntries((await session.send('Performance.getMetrics')).metrics.map(m=>[m.name,m.value]));
  return {...await page.evaluate(()=>({dom:document.querySelectorAll('*').length,details:document.querySelectorAll('details').length,scrollHeight:document.documentElement.scrollHeight,overflow:document.documentElement.scrollWidth-innerWidth})),
    scriptMs:pm.ScriptDuration*1000,taskMs:pm.TaskDuration*1000,layoutMs:pm.LayoutDuration*1000,jsHeapBytes:pm.JSHeapUsedSize,
    domCounters:await session.send('Memory.getDOMCounters')};
}
async function browserMemory(browser){
  const session=await browser.newBrowserCDPSession();
  const info=await session.send('SystemInfo.getProcessInfo');const rows=[];
  for(const item of info.processInfo){try{const s=fs.readFileSync(`/proc/${item.id}/smaps_rollup`,'utf8');rows.push({type:item.type,pid:item.id,rssKiB:Number(s.match(/^Rss:\s+(\d+)/m)?.[1]||0),pssKiB:Number(s.match(/^Pss:\s+(\d+)/m)?.[1]||0)});}catch(e){rows.push({type:item.type,pid:item.id,unavailable:e.code});}}
  await session.detach();return rows;
}
function launch(){return chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox','--disable-background-networking'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig||process.env.FONTCONFIG_FILE}});}
async function validate(browser,candidate,server){
  for(const width of [1440,768,390,320])for(const work of authored.works){
    const page=await browser.newPage({viewport:{width,height:900}}),traffic=[];
    page.on('request',r=>traffic.push(r.url()));
    await page.goto(server.url);await page.waitForSelector('[data-work]');
    await page.locator(`[data-work=${work.key}]`).focus();await page.keyboard.press('Enter');
    await page.waitForSelector('[data-unit]');await page.evaluate(()=>document.fonts.ready);
    assert.equal(await page.locator('h1').textContent(),work.units[0].text);
    assert.deepEqual(await page.locator('[data-unit]').allTextContents(),work.units.slice(1).map(u=>u.text));
    const fixture=fixtures.find(f=>f.key===work.key);
    const status=JSON.parse(fs.readFileSync(path.join(config.retained,`browser-final/${work.key}-status.stdout`)));
    const selected=status.work_history.find(w=>w.work_item_id===fixture.work);
    assert.equal(await page.locator('#audit pre').textContent(),JSON.stringify(selected,null,2),'complete identical diagnostic');
    assert.equal(await page.locator('.goal').textContent(),work.goal);
    const synopsis=await page.locator('[data-unit="1"]').boundingBox();assert(synopsis.y<700,'first-screen synopsis');
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
    await page.screenshot({path:path.join(out,`${candidate}-${work.key}-${width}-reading.png`)});
    // Independent authored ref expectations plus retained public Source bytes.
    const links=page.locator('a[data-source]'),tripCount=await links.count();
    for(let i=0;i<await links.count();i++){
      const link=links.nth(i),target=await link.getAttribute('data-source'),origin=await link.getAttribute('data-claim');
      const label=await link.textContent();const match=selected.answers.source_excerpts.find(s=>label===`${s.citation.locator}:${s.citation.start_line}–${s.citation.end_line}`);assert(match,'exact available source locator');
      await link.focus();const originalScroll=await page.evaluate(()=>scrollY);await page.keyboard.press('Enter');
      assert.equal(await page.evaluate(()=>document.activeElement.id),target);
      const source=page.locator('#'+target);assert(await source.isVisible());
      const lines=await source.locator('.line').evaluateAll(nodes=>nodes.map(n=>{const copy=n.cloneNode(true);copy.querySelector('.number').remove();return copy.textContent;}));
      assert.deepEqual(lines,match.text.replace(/\n$/,'').split('\n'),'same code bytes modulo explicit line-number spans');
      assert.equal(await source.locator('.explained').count(),lines.length);
      if(width>800)assert(await page.locator('.narrative').isVisible());else assert(!await page.locator('.narrative').isVisible());
      await page.locator('.return').focus();await page.keyboard.press('Enter');
      assert.equal(await page.evaluate(()=>document.activeElement.id),origin);
      assert(Math.abs(await page.evaluate(()=>scrollY)-originalScroll)<=1,'exact reading scroll restored');
    }
    const mechanism=page.locator('.reading_point a[data-source]').first();await mechanism.click();
    await page.screenshot({path:path.join(out,`${candidate}-${work.key}-${width}-code.png`)});
    await page.locator('.return').click();
    await page.goto(server.url+`?work=${work.key}&failure=1`);await page.waitForSelector('.unavailable');
    assert.equal(await page.locator('h1').textContent(),work.units[0].text);
    await page.screenshot({path:path.join(out,`${candidate}-${work.key}-${width}-failure.png`)});
    await page.goto(server.url+'?work=missing');await page.waitForSelector('.unavailable');assert.equal(await page.locator('h1').textContent(),'Work unavailable');
    assert(traffic.every(url=>new URL(url).origin===new URL(server.url).origin),'same-host requests only');
    results.networks.push({candidate,work:work.key,width,requests:traffic});results.checks.push({candidate,work:work.key,width,result:'passed',citationTrips:tripCount});
    await page.close();
  }
  if(candidate==='vue'){
    const page=await browser.newPage();await page.route('**/data.json',route=>route.fulfill({status:503,body:'unavailable'}));await page.goto(server.url);await page.waitForSelector('#app');await page.waitForFunction(()=>document.getElementById('app').textContent.includes('Source data unavailable'));results.checks.push({candidate,dependencyFailure:'passed'});await page.close();
    const hostile=JSON.parse(fs.readFileSync(path.join(config.build,'data.json'))),attack="</script><script>window.__attack=1</script><img src=https://example.invalid onerror='window.__attack=1'>";
    hostile[0].units[1].text=attack;hostile[0].sources[0].text=attack;hostile[0].goal=attack+' very-long-text'.repeat(1000);
    const security=await browser.newPage({viewport:{width:320,height:900}});const traffic=[];security.on('request',r=>traffic.push(r.url()));
    await security.route('**/data.json',route=>route.fulfill({contentType:'application/json',body:JSON.stringify(hostile)}));await security.goto(server.url+'?work=b');await security.waitForSelector('[data-unit]');
    assert.equal(await security.locator('[data-unit="1"]').textContent(),attack);assert.equal(await security.locator('img').count(),0);assert.equal(await security.evaluate(()=>window.__attack),undefined);
    await security.locator('details').first().locator('summary').click();assert(await security.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));assert(traffic.every(u=>new URL(u).origin===new URL(server.url).origin));results.checks.push({candidate,hostileProseAndSource:'passed',longText:'passed'});await security.close();
  }
}
async function measure(candidate,server,key,repeat){
  const started=performance.now(),browser=await launch(),launchMs=performance.now()-started;
  results.environment.chromium=browser.version();
  try{const page=await browser.newPage({viewport:{width:1440,height:900}});const session=await page.context().newCDPSession(page);await session.send('Performance.enable');
    for(const temperature of ['cold','warm']){
      await page.goto('about:blank');const before=stat(server.proc.pid),t=performance.now();
      const fixture=fixtures.find(f=>f.key===key),baseline=candidate==='baseline';
      const suffix=baseline?`?view=work&work=${fixture.work}`:`?work=${key}`;
      await page.goto(server.url+suffix);
      if(baseline){await page.waitForSelector('[data-question=Synopsis]');assert.equal(await page.locator('[data-question=Synopsis]').first().textContent(),authored.works.find(w=>w.key===key).units[1].text);}
      else await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');await painted(page);
      const contentMs=performance.now()-t,after=stat(server.proc.pid),m=await metrics(page,session);
      const paints=await page.evaluate(()=>({paints:performance.getEntriesByType('paint').map(p=>({name:p.name,startTime:p.startTime})),navigation:performance.getEntriesByType('navigation').map(n=>({responseStart:n.responseStart,responseEnd:n.responseEnd,domContentLoadedEventEnd:n.domContentLoadedEventEnd}))}));
      await page.goto(server.url);await page.waitForSelector(baseline?'#overview':'[data-work]');
      const navStart=performance.now();
      if(baseline){await page.locator(`#overview article[data-work-id="${fixture.work}"] h4 a`).click();await page.waitForSelector('[data-question=Synopsis]');}
      else {await page.locator(`[data-work=${key}]`).click();await page.waitForSelector('[data-unit]');}
      await painted(page);const navigationMs=performance.now()-navStart;
      const sourceStart=performance.now();await page.locator(baseline?'.answer-section[data-section=ReadingPoint] a.source-citation':'.reading_point a[data-source]').first().click();
      if(!baseline)await page.waitForFunction(()=>document.body.classList.contains('code-focus'));await painted(page);const selectionMs=performance.now()-sourceStart;
      const returnStart=performance.now();await page.locator(baseline?'.source-reading-point:target .return-to-claim':'.return').click();await painted(page);const returnMs=performance.now()-returnStart;
      results.samples.push({candidate,key,repeat,temperature,launchMs:temperature==='cold'?launchMs:null,contentMs,navigationMs,selectionMs,returnMs,serverCpuTicks:after.cpuTicks-before.cpuTicks,serverRssKiB:after.rssKiB,...m,...paints,browserMemory:await browserMemory(browser)});
    }
  }finally{await browser.close();}
}
(async()=>{const servers=[];try{
  for(const candidate of ['rust','vue'])servers.push([candidate,await start(path.join(config.build,candidate+'-reader'))]);
  // Interleave candidate order; no simultaneous measurements.
  for(let repeat=0;repeat<(config.repeats??7);repeat++)for(const [candidate,server] of repeat%2?[...servers].reverse():servers)for(const key of ['b','click'])await measure(candidate,server,key,repeat);
  if(config.baseline_bin && (config.repeats??7)>0)for(const fixture of fixtures){
    const server=await start(path.join(config.baseline_bin,'volicord-viewer'),['--runtime',fixture.runtime,'--project',fixture.project,'--bind','127.0.0.1:0','--locale','ko','--language','ko'],true);
    try{for(let repeat=0;repeat<(config.repeats||7);repeat++)await measure('baseline',server,fixture.key,repeat);}finally{await server.stop();}
  }
  const browser=await launch();try{for(const [candidate,server]of servers)await validate(browser,candidate,server);
    const security=await browser.newPage({viewport:{width:320,height:900}});await security.goto(pathToFileURL(config.security_html).href);assert.equal(await security.locator('script,img').count(),0);assert.equal(await security.evaluate(()=>window.__attack),undefined);await security.locator('details').first().locator('summary').click();assert(await security.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));results.checks.push({candidate:'rust',hostileRendererBrowser:'passed',longText:'passed'});await security.close();
    for(const work of authored.works){const page=await browser.newPage({javaScriptEnabled:false,viewport:{width:320,height:900}});await page.context().setOffline(true);const file=path.join(config.build,work.key+'-offline.html');await page.goto(pathToFileURL(file).href);assert.deepEqual(await page.locator('[data-unit]').allTextContents(),work.units.slice(1).map(u=>u.text));assert.equal(await page.locator('script').count(),0);assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
      const link=page.locator('.reading_point a[data-source]').first(),origin=await link.getAttribute('data-claim'),target=await link.getAttribute('data-source');await link.focus();await page.keyboard.press('Enter');assert.equal(await page.evaluate(()=>document.activeElement.id),target);await page.locator(`#${target} [data-return="${origin}"]`).focus();await page.keyboard.press('Enter');assert.equal(await page.evaluate(()=>document.activeElement.id),origin);
      results.checks.push({offline:work.key,result:'passed',sha256:digest(file)});await page.close();}
    // Historical full-snapshot baseline: static bytes, never described as live latency.
    for(const key of ['b','click']){const file=path.join(config.retained,`browser-final/${key}-ko.html`);const page=await browser.newPage({viewport:{width:1440,height:900}});const session=await page.context().newCDPSession(page);await session.send('Performance.enable');await page.goto(pathToFileURL(file).href);results.checks.push({baseline:key,sha256:digest(file),bytes:fs.statSync(file).size,...await metrics(page,session),classification:'historical snapshot; no live server timing'});await page.close();}
  }finally{await browser.close();}
  results.serverStartups=servers.map(([candidate,server])=>({candidate,ms:server.startupMs}));
}catch(error){results.error=error.stack;process.exitCode=1;}finally{for(const [,server]of servers)await server.stop();fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(results,null,2));console.log(JSON.stringify({samples:results.samples.length,checks:results.checks.length,error:results.error||null,output:out}));}})();
