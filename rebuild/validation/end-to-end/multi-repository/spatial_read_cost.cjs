// Small installed-reader diagnosis. Never invokes generation, capture or qualification.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const crypto=require('node:crypto'),{spawn}=require('node:child_process'),{pathToFileURL}=require('node:url');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
fs.mkdirSync(config.output);const write=(name,value)=>fs.writeFileSync(path.join(config.output,name),JSON.stringify(value,null,2),{flag:'wx'});
const result={scope:'one sample per installed publication/mode/width; OS caches uncontrolled; no p95, speedup, qualification or human acceptance claim',rows:[],processes:[],external:[],outcome:'failed',identities:{viewer:hash(config.viewer),driver:hash(__filename),chromium:hash(config.chromium)}};
async function serve(c){
 const child=spawn(config.viewer,['--runtime',c.runtime,'--project',c.project,'--bind','127.0.0.1:0']);let stderr='',stdout='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);
 const closed=new Promise(r=>child.once('close',(code,signal)=>r({code,signal})));
 const address=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('installed listener timeout '+stderr)),15000);child.once('error',reject);child.stderr.on('data',()=>{const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});});
 return{address,async stop(){child.kill('SIGTERM');result.processes.push({case:c.key,...await closed,termination:'explicit listener cleanup'});write(c.key+'-listener.json',{stdout,stderr});}};
}
(async()=>{
 let browser,server;
 try{
  browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
  for(const c of config.cases){
   server=await serve(c);const original=JSON.parse(fs.readFileSync(c.public));
   for(const width of config.widths||[1440,390])for(const mode of ['live','file-no-js']){
    const context=await browser.newContext({viewport:{width,height:900},javaScriptEnabled:mode==='live'}),page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(String(e)));
    await context.route('**/*',route=>{const u=route.request().url();if(u.startsWith(server.address)||u.startsWith('file:')||u.startsWith('data:'))return route.continue();result.external.push(u);return route.abort();});
    const url=mode==='live'?server.address+`?view=${c.work?'work&work='+c.work:'repository'}&locale=${c.locale||c.language}&language=${c.language}`:pathToFileURL(c.offline).href+'#'+(c.work?'work-'+c.work:'repository-explanation');
    const started=performance.now();await page.goto(url);await page.evaluate(()=>document.fonts.ready);await page.waitForTimeout(100);
    assert.equal(await page.locator('.admitted-diagram').count(),original.diagrams.length,'profile the real publication');
    assert.equal(await page.locator(`.admitted-diagram[data-reader-publication="${c.publication}"]`).count(),original.diagrams.length,'exact current publication');
    if(mode==='file-no-js')assert.equal(await page.locator('script').count(),0,'actual script-free export');
    const metrics=await page.evaluate(()=>{const n=performance.getEntriesByType('navigation')[0];return{navigation:n&&{response_start_ms:n.responseStart,response_end_ms:n.responseEnd,dom_content_loaded_ms:n.domContentLoadedEventEnd,dom_complete_ms:n.domComplete,load_event_end_ms:n.loadEventEnd},paint:performance.getEntriesByType('paint').map(p=>({name:p.name,start_ms:p.startTime})),dom_elements:document.querySelectorAll('*').length,svg_elements:document.querySelectorAll('svg *').length,inline_script_bytes:[...document.scripts].reduce((n,s)=>n+new TextEncoder().encode(s.textContent).length,0)};});
    const session=await context.newCDPSession(page);await session.send('Performance.enable');const renderer=await session.send('Performance.getMetrics');await session.detach();
    const heap=Object.fromEntries(renderer.metrics.filter(m=>['JSHeapUsedSize','JSHeapTotalSize','Nodes','LayoutCount'].includes(m.name)).map(m=>[m.name,m.value]));
    assert.deepEqual(errors,[]);result.rows.push({case:c.key,publication:c.publication,width,mode,elapsed_to_observation_ms:performance.now()-started,html_file_bytes:fs.statSync(c.offline).size,...metrics,renderer:heap});await context.close();
   }
   await server.stop();server=null;
  }
  assert.deepEqual(result.external,[]);result.outcome='passed';
 }catch(error){result.error=String(error);throw error;}
 finally{await browser?.close();await server?.stop();write('result.json',result);}
})().catch(error=>{console.error(error);process.exitCode=1;});
