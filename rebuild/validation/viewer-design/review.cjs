// Delivery check: relocate the complete package, start exactly `python3 serve.py`,
// stop it, then read script-free files with networking disabled. No original paths.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),{pathToFileURL}=require('node:url');
const c=JSON.parse(fs.readFileSync(process.argv[2])),{chromium}=require(c.playwright);
fs.mkdirSync(c.output,{recursive:false});
const temp=fs.mkdtempSync(path.join(os.tmpdir(),'viewer-review-')),root=path.join(temp,'review');
fs.cpSync(c.build,root,{recursive:true});
const result={status:'running',checks:[],requests:[],human_acceptance:'pending',relocated:true};
let proc,browser,stdout='',stderr='';
async function stop(){if(!proc)return;if(proc.exitCode!==null||proc.signalCode!==null){result.server={stdout,stderr,code:proc.exitCode,signal:proc.signalCode};proc=null;return;}await new Promise(resolve=>{proc.once('exit',(code,signal)=>{result.server={stdout,stderr,code,signal,cleanup:'explicit SIGTERM'};resolve();});proc.kill('SIGTERM');});proc=null;}
(async()=>{try{
 proc=spawn('python3',['serve.py'],{cwd:root});
 const url=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('preview startup timeout')),10000);proc.stdout.on('data',b=>{stdout+=b;const m=stdout.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});proc.stderr.on('data',b=>stderr+=b);proc.once('error',reject);proc.once('exit',code=>{clearTimeout(timer);reject(Error('preview exited '+code));});});
 browser=await chromium.launch({executablePath:c.chromium,headless:true,args:['--no-sandbox','--disable-background-networking'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});
 for(const width of [1440,390]){
  const page=await browser.newPage({viewport:{width,height:900}});page.on('request',r=>result.requests.push(r.url()));
  for(const file of ['index.html','journeys.html','journey-observations.html','observations.html','initial/journeys.html','viewer/index.html','viewer/overview.html','viewer/works.html','viewer/missing.html']){
   assert.equal((await page.goto(url+file)).status(),200,file);
   await page.evaluate(()=>document.fonts.ready);
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'relocated page overflow '+file);
  }
  await page.goto(url+'viewer/click.html');await page.locator('.condition-table').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(c.output,`comparison-${width}.png`)});
  await page.goto(url+'viewer/b.html#original');assert(await page.locator('#original').evaluate(n=>n.open));
  await page.goto(url+'viewer/b.html#point-5-0');await page.locator('.source-point.active').waitFor({state:'visible'});
  assert.equal(await page.locator('.appendix').getAttribute('aria-modal'),width<=900?'true':null);
  await page.locator('.source-point.active .return').focus();await page.keyboard.press('Escape');
  await page.waitForFunction(()=>document.activeElement.id==='claim-5');
  result.checks.push({width,preview:'relocated Python launch; entry, initial, measurements, catalog, missing explanation, conditions, direct disclosure/Source and return'});
  await page.close();
 }
 assert(result.requests.every(u=>new URL(u).origin===new URL(url).origin),'remote request');
 await stop();
 const standalone=path.join(temp,'viewer-only');fs.cpSync(path.join(root,'viewer'),standalone,{recursive:true});
 for(const width of [1440,390]){
  const local=await browser.newPage({viewport:{width,height:900}});await local.context().setOffline(true);
  await local.goto(pathToFileURL(path.join(standalone,'b.html')).href+'#point-5-0');
  await local.locator('.source-point.active').waitFor({state:'visible'});
  assert.equal(await local.locator('.doc').isVisible(),width>900,'Viewer works without review files');
  await local.keyboard.press('Escape');await local.waitForFunction(()=>document.activeElement.id==='claim-5');
  await local.close();
 }
 result.checks.push({standalone_viewer:'only viewer/ copied; scripts/Source selection/return work offline with no review files at 1440/390'});
 const context=await browser.newContext({javaScriptEnabled:false,viewport:{width:320,height:900}});await context.setOffline(true);
 const page=await context.newPage();
 for(const key of ['b','click']){
  await page.goto(pathToFileURL(path.join(standalone,key+'-offline.html')).href);
  assert.equal(await page.locator('script,form').count(),0);
  assert(await page.locator('.goal').isVisible(),'script-free original Goal visible');
  const link=page.locator('[data-origin="claim-5"].evidence-link').first();await link.focus();await page.keyboard.press('Enter');
  const point=page.locator('#point-5-0');await point.locator('.return').focus();await page.keyboard.press('Enter');
  assert.equal(await page.evaluate(()=>document.activeElement.id),'claim-5');
 }
 await context.close();result.checks.push({offline:'server stopped; network disabled; relocated B/Click native citation and return at 320px'});
 result.status='passed';
}catch(error){result.status='failed';result.error=error.stack;process.exitCode=1;}
finally{if(browser)await browser.close();await stop();fs.writeFileSync(path.join(c.output,'result.json'),JSON.stringify(result,null,2));if(result.status==='passed')fs.rmSync(temp,{recursive:true});console.log(JSON.stringify(result));}
})();
