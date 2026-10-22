// Challenge critical-task assertions against unchanged actual Product exports.
// Run after product_review.cjs; mutations affect disposable browser DOM only.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const {sourceTasks,criticalLink,criticalPoint,criticalBytes}=require('./product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium}=require(config.playwright);
(async()=>{
 const browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
 let checks=0;
 try{for(const c of config.cases){
  const status=JSON.parse(fs.readFileSync(path.join(config.output,c.key+'-public-status.json')));
  const context=await browser.newContext({javaScriptEnabled:false}),page=await context.newPage();
  try{for(const destination of ['repository',...(c.work?['work']:[])]){
   const answers=destination==='repository'?status.repository_explanation:status.work_history.find(w=>w.work_item_id===c.work).answers;
   const tasks=sourceTasks(c,destination,answers);
   for(const task of tasks){
    const url=pathToFileURL(path.join(config.output,c.key+'-offline.html')).href+'#'+(destination==='repository'?'repository-explanation':'work-'+c.work);
    const article=page.locator(destination==='repository'?'#repository-explanation .doc':`article.doc[data-work-id="${c.work}"]`);
    await page.goto(url);await page.reload();
    let link=await criticalLink(article,task),target=await link.getAttribute('href');
    const point=page.locator(`[id="${target.slice(1)}"]`);
    await criticalPoint(point,task);await criticalBytes(page,point,task,true);checks++;
    const other=article.locator('a.evidence-link:visible').last();
    assert.notEqual(await other.getAttribute('href'),target,'counterexample keeps an unrelated last link');
    await link.evaluate(n=>n.remove());
    await assert.rejects(()=>criticalLink(article,task),/critical claim must own/);checks++;
    await other.click();assert.equal(new URL(page.url()).hash,await other.getAttribute('href'),'unrelated last link still works');
    await page.goto(url);await page.reload();link=await criticalLink(article,task);target=await link.getAttribute('href');
    const fresh=page.locator(`[id="${target.slice(1)}"]`);
    await fresh.evaluate(n=>n.dataset.sourceId='foreign-source');
    await assert.rejects(()=>criticalPoint(fresh,task),/critical Source identity/);checks++;
    await page.goto(url);await page.reload();await fresh.locator('pre.code-region code').evaluate(n=>n.textContent=n.textContent.slice(0,-1));
    await assert.rejects(()=>criticalBytes(page,fresh,task,true),/offline complete critical window/);checks++;
    await page.goto(url);await page.reload();await fresh.locator('pre.code-region').evaluate(n=>n.remove());
    await assert.rejects(()=>criticalBytes(page,fresh,task,true),/offline critical window cannot be omitted/);checks++;
   }
  }}finally{await context.close();}
 }}finally{await browser.close();}
 console.log(JSON.stringify({outcome:'passed',checks,scope:'actual Product export DOM counterexamples; no semantic or human verdict'}));
})().catch(error=>{console.error(error);process.exitCode=1;});
