// Exact current Product pages and whole offline documents. No generation or human verdict.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {spawn, spawnSync, execFileSync} = require('node:child_process');
const {pathToFileURL} = require('node:url'), crypto = require('node:crypto');
const c = JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium} = require(c.playwright);
fs.mkdirSync(c.output, {recursive:false});
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const result = {candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
  diff_sha256:hash(execFileSync('git',['diff','HEAD','--','rebuild'])), viewer_sha256:hash(fs.readFileSync(c.viewer)),
  semantic_approval:'not_established',human_acceptance:'unassessed',screen_reader:'not_observed',native_zoom:'not_observed',checks:[],processes:[]};
function command(label, bin, args) {
  const p=spawnSync(bin,args,{encoding:'utf8',timeout:60000});
  fs.writeFileSync(path.join(c.output,label+'.stdout'),p.stdout||''); fs.writeFileSync(path.join(c.output,label+'.stderr'),p.stderr||'');
  result.processes.push({label,argv:[bin,...args],exit:p.status,signal:p.signal});
  assert.equal(p.status,0,p.stderr); return p.stdout;
}
function state(x,label) {return command(label,'python3',['-c',`import sqlite3,pathlib,hashlib,json,sys
r={}
for p in sorted(pathlib.Path(sys.argv[1]).glob('*.sqlite3')):
 with sqlite3.connect(p.resolve().as_uri()+'?mode=ro',uri=True) as db:r[p.name]=hashlib.sha256(('\\n'.join(db.iterdump())).encode()).hexdigest()
print(json.dumps(r,sort_keys=True))`,x.runtime]);}
async function serve(x) {
 const child=spawn(c.viewer,['--runtime',x.runtime,'--project',x.project,'--bind','127.0.0.1:0']);let err='',out='';child.stdout.on('data',b=>out+=b);
 const base=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('listener timeout')),15000);child.stderr.on('data',b=>{err+=b;const m=err.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});child.on('error',reject);child.on('exit',()=>{clearTimeout(timer);reject(Error(err));});});
 return {base,async stop(){await new Promise(resolve=>{child.once('exit',(code,signal)=>{fs.writeFileSync(path.join(c.output,x.key+'-viewer.stderr'),err);fs.writeFileSync(path.join(c.output,x.key+'-viewer.stdout'),out);result.processes.push({label:x.key+'-viewer',exit:code,signal,termination:'explicit cleanup'});resolve();});child.kill();});}};
}
async function fragments(page) {
 const errors=await page.evaluate(()=>{const counts=new Map();for(const n of document.querySelectorAll('[id]'))counts.set(n.id,(counts.get(n.id)||0)+1);
 const errors=[...counts].filter(([,v])=>v!==1).map(([id,n])=>`${id}: ${n} targets`);
 for(const a of document.querySelectorAll('a[href^="#"]'))if(counts.get(a.hash.slice(1))!==1)errors.push(`bad link ${a.getAttribute('href')}`);
 for(const n of document.querySelectorAll('[data-origin]'))if(counts.get(n.dataset.origin)!==1)errors.push(`bad return ${n.dataset.origin}`);
 return errors;});assert.deepEqual(errors,[],'every internal link and return has exactly one target');
}
(async()=>{const browser=await chromium.launch({executablePath:c.chromium,headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});
 result.browser_version=browser.version();
 try {for(const x of c.cases){const before=state(x,x.key+'-before');const server=await serve(x);
 try {
 const snapshot=path.join(c.output,x.key+'.html');command(x.key+'-snapshot',c.viewer,['--runtime',x.runtime,'--project',x.project,'--locale',x.locale,'--language',x.language,'--snapshot',snapshot]);
 for(const width of [1440,1024,768,390,320])for(const mode of ['live','offline','live-no-js']){
 const js=mode==='live';const ctx=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:js});
 await ctx.route('**/*',r=>{const u=new URL(r.request().url());return u.protocol==='file:'||u.origin===new URL(server.base).origin?r.continue():r.abort();});
 try{const page=await ctx.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const url=mode==='offline'?pathToFileURL(snapshot).href+(x.work?'#work-'+x.work:'#repository-explanation'):server.base+`?view=${x.work?'work&work='+x.work:'repository'}&locale=${x.locale}&language=${x.language}`;
 await page.goto(url);await fragments(page);

 if(mode==='offline')assert.equal(await page.locator('script,form').count(),0);
 assert.deepEqual(errors,[]);result.checks.push({case:x.key,width,mode,status:'passed'});
 }finally{await ctx.close();}}
 }finally{await server.stop();}
 assert.equal(state(x,x.key+'-after'),before,'reading/export must not mutate logical stores');
 }result.status='passed';}finally{await browser.close();}
})().catch(e=>{result.status='failed';result.error=e.stack;process.exitCode=1;}).finally(()=>{fs.writeFileSync(path.join(c.output,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({status:result.status,checks:result.checks.length,error:result.error}));});
