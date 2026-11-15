// Real Viewer response chunks through a local transport gate. The shell is
// forwarded immediately; only later bytes are held. No generated page fixture.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {spawn,execFileSync}=require('node:child_process');
const {createOutput}=require('../../viewer-design/product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));createOutput(config.output);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const result={outcome:'failed',conditions:[],requests:[],human_acceptance:'unobserved'};
let child,browser,proxy,exit,active,gate=null,logs='',stdout='';
const bounded=(promise,label)=>{let timer;return Promise.race([promise,new Promise((_,reject)=>timer=setTimeout(()=>reject(Error(label)),30000))]).finally(()=>clearTimeout(timer));};
function state(){return execFileSync('python3',['-c',`import sqlite3,json,hashlib,pathlib,sys
result={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 c=sqlite3.connect('file:'+str(p)+'?mode=ro',uri=True)
 for (t,) in c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"):
  rows=c.execute('SELECT * FROM "'+t.replace('"','""')+'"').fetchall()
  result[p.name+':'+t]=hashlib.sha256(json.dumps(sorted([repr(r) for r in rows]),ensure_ascii=False).encode()).hexdigest()
print(json.dumps(result,sort_keys=True))`,config.runtime],{encoding:'utf8'});}
function newGate(){
 let arrive;const g={released:false,buffer:[],started:performance.now(),arrival:new Promise(r=>arrive=r),arrive};
 g.arrive=arrive;g.release=()=>{g.released=true;for(const b of g.buffer)g.response.write(b);g.buffer=[];if(g.ended)g.response.end();};
 return g;
}
async function main(){
 try{
  result.candidate=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();result.diff_sha256=hash(execFileSync('git',['diff','HEAD','--','rebuild']));result.driver_sha256=hash(fs.readFileSync(__filename));result.viewer_sha256=hash(fs.readFileSync(config.viewer));result.config_sha256=hash(fs.readFileSync(process.argv[2]));
  const before=state();child=spawn(config.viewer,['--runtime',config.runtime,'--project',config.project,'--bind','127.0.0.1:0','--view','work','--locale','ko','--language','ko']);child.stderr.on('data',b=>logs+=b);child.stdout.on('data',b=>stdout+=b);exit=new Promise(r=>child.once('close',(code,signal)=>r({code,signal})));
  const upstream=await bounded(new Promise((resolve,reject)=>{child.once('error',reject);child.stderr.on('data',()=>{const m=logs.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m)resolve(new URL(m[0]));});}),'listener unavailable');
  proxy=http.createServer((request,response)=>{
   const initial=request.headers['sec-fetch-mode']==='navigate'&&request.headers['sec-fetch-dest']==='document';const g=initial?gate:null;
   const headers={...request.headers,host:upstream.host};if(headers.origin)headers.origin=upstream.origin;
   result.requests.push({path:request.url,method:request.method,destination:request.headers['sec-fetch-dest']});
   const forward=http.request(new URL(request.url,upstream),{method:request.method,headers},incoming=>{
    response.writeHead(incoming.statusCode,incoming.headers);
    let prefix=Buffer.alloc(0),sent=false;
    incoming.on('data',bytes=>{
     if(g?.truncated)return;
     if(!g){response.write(bytes);return;}
     if(!sent){prefix=Buffer.concat([prefix,bytes]);const marker=Buffer.from('<div data-initial-content>'),at=prefix.indexOf(marker);if(at<0)return;
      const end=at+marker.length;response.write(prefix.subarray(0,end));sent=true;g.response=response;g.shell_ms=performance.now()-g.started;g.arrive();
      if(prefix.length>end)g.buffer.push(prefix.subarray(end));prefix=null;
     }else if(g.released)response.write(bytes);else g.buffer.push(bytes);
    });
    incoming.on('end',()=>{if(!g){response.end();return;}g.ended=true;g.upstream_complete_ms=performance.now()-g.started;if(!sent){response.end(prefix);g.arrive();}else if(g.released)response.end();});
    incoming.on('error',()=>{if(!response.writableEnded)response.destroy();});
   });
   forward.on('error',()=>{if(!response.headersSent)response.writeHead(502);if(!response.writableEnded)response.end();});request.pipe(forward);
  });await new Promise(r=>proxy.listen(0,'127.0.0.1',r));const base='http://127.0.0.1:'+proxy.address().port+'/';
  result.transport='local proxy forwards real shell and holds later bytes; Host/Origin rewritten to the owned upstream authority';
  browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
  const target=(view,locale,work,fragment='')=>{const u=new URL(base);const q={locale,language:locale};if(view)q.view=view;if(work)q.work=work;u.search=new URLSearchParams(q);u.hash=fragment;return u.href;};
  async function journey(name,{view,locale='ko',work,width=1440,scripted=true,reduce=false,fragment='',reload=false,truncate=false},reuse){
   const context=reuse?.context||await browser.newContext({javaScriptEnabled:scripted,...(reduce?{reducedMotion:'reduce'}:{})});context.setDefaultTimeout(30000);
   if(!reuse)await context.route('**/*',r=>r.request().url().startsWith(base)?r.continue():r.abort());
   const page=reuse?.page||await context.newPage();active=page;await page.setViewportSize({width,height:900});
   const errors=[];page.on('pageerror',e=>errors.push(e.message));gate=newGate();const g=gate;const url=target(view,locale,work,fragment);let settled=false;
   const navigation=(reload?page.reload():page.goto(url)).then(response=>{settled=true;return response;});
   await bounded(g.arrival,'initial shell did not arrive');await page.locator('.page-loading-spinner').waitFor();
   assert.equal(settled,false,'the document is still loading while its animation is already visible');
   assert.equal(await page.locator('[data-reader-publication],[data-citation-url]').count(),0,'no old or premature body/source meaning');
   assert.equal(await page.locator('body').getAttribute('data-reader-enhanced'),null,'reader initialization waits for the whole response');
   const sample=()=>page.locator('.page-loading-spinner').evaluate(n=>({reduce:matchMedia('(prefers-reduced-motion: reduce)').matches,animation:getComputedStyle(n).animationName,transform:getComputedStyle(n).transform,time:n.getAnimations()[0]?.currentTime}));
   const first=await sample();await page.waitForTimeout(230);const second=await sample();
   if(first.reduce)assert.equal(first.animation,'none');else assert(first.transform!==second.transform&&second.time>first.time,'first-entry CSS animation advances before body completion');
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
   await page.screenshot({path:path.join(config.output,name+'-loading.png')});
   if(truncate){g.truncated=true;g.response.end();g.released=true;g.buffer=[];}else g.release();
   const response=await navigation;assert.equal(response.status(),200);assert.equal(response.headers()['transfer-encoding'],'chunked');
   if(truncate){
    await page.locator('.page-navigation-error').waitFor();gate=null;await page.locator('[data-retry-page]').click();await page.waitForFunction(()=>document.body.dataset.readerEnhanced==='true'&&!document.querySelector('.page-navigation-error,.page-loading'));
   }else{
    assert.equal(await page.locator('.page-loading:visible').count(),0);
    assert.equal(await page.locator('#viewer-content').count(),1);
    assert.equal(await page.locator('script[data-viewer-reading]').count(),1);
    if(scripted){assert.equal(await page.locator('body').getAttribute('data-reader-enhanced'),'true');assert.equal(await page.locator('[data-initial-content]').count(),0);}
    else assert.equal(await page.locator('body').getAttribute('data-reader-enhanced'),null);
    const heading=page.locator('#viewer-content h1').first();assert(await heading.isVisible(),'the final native document is visible');
    if(fragment){assert.equal(new URL(page.url()).hash,'#'+fragment);assert(await page.locator('[id="'+fragment+'"]').isVisible(),'exact deep fragment survives first entry');}
   }
   assert.deepEqual(errors,[]);result.conditions.push({name,scripted,reduce:first.reduce,locale,width,shell_ms:g.shell_ms,upstream_complete_ms:g.upstream_complete_ms});gate=null;console.log(name);
   return{context,page};
  }
  for(const locale of ['ko','en'])for(const view of ['overview','work','detail','repository']){
   const item=await journey(view+'-'+locale,{view:view==='detail'?'work':view,locale,work:view==='detail'?config.work:undefined,width:locale==='ko'?390:1440});await item.context.close();
  }
  const defaults=await journey('default-view',{view:null});await journey('reload',{view:null,reload:true},defaults);await defaults.context.close();
  const native=await journey('native-work',{view:'work',work:config.work,scripted:false});
  const point=await native.page.locator('.source-point').first().getAttribute('id');await native.context.close();
  const deep=await journey('native-deep-fragment',{view:'work',work:config.work,scripted:false,fragment:point,width:390});await deep.context.close();
  const repository=await journey('native-repository',{view:'repository',locale:'en',scripted:false});await repository.context.close();
  const reduced=await journey('reduced-motion',{view:'overview',reduce:true});await reduced.context.close();
  const incomplete=await journey('incomplete-response-retry',{view:'work',work:config.work,truncate:true});await incomplete.context.close();
  assert.equal(state(),before,'initial reads, completion and retry preserve canonical, Derived, provider and retention rows');result.get_state_unchanged=true;result.outcome='passed';
 }catch(error){result.error=error.stack||String(error);await active?.screenshot({path:path.join(config.output,'failure.png')}).catch(()=>{});throw error;}
 finally{await browser?.close();if(proxy)await new Promise(r=>proxy.close(r));if(child){child.kill('SIGTERM');result.listener=await exit;}fs.writeFileSync(path.join(config.output,'viewer.stdout'),stdout);fs.writeFileSync(path.join(config.output,'viewer.stderr'),logs);fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});}
 console.log(JSON.stringify({outcome:result.outcome,conditions:result.conditions.length}));
}
main().catch(error=>{console.error(error);process.exitCode=1;});
