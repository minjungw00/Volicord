// Actual HTTP and offline reader controls. Authored input is never model or human evidence.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {spawn, spawnSync} = require('node:child_process');
const {pathToFileURL} = require('node:url');
const crypto = require('node:crypto');
const config = JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium} = require(config.playwright);
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const observations = [];
const processes = [];
const output = path.resolve(config.output);
assert.ok(output.includes('/rebuild/.local/'), 'review artifacts must be ignored');
fs.mkdirSync(output, {recursive:false});
function run(label, program, args) {
  const result = spawnSync(program, args, {encoding:'utf8', timeout:60000});
  fs.writeFileSync(path.join(output,label+'.stdout'),result.stdout || '');
  fs.writeFileSync(path.join(output,label+'.stderr'),result.stderr || '');
  processes.push({label,program,args,exit_code:result.status,signal:result.signal,error:result.error?.message});
  assert.equal(result.status,0,label+': '+result.stderr);
  return result.stdout;
}
function retained(runtime, label) {
  // Logical read-only dump identities include retention fields without exposing bytes.
  const script = `import hashlib,json,sqlite3,sys
from pathlib import Path
r=Path(sys.argv[1]); result={}
for p in sorted(r.glob('*.sqlite3')):
 with sqlite3.connect(p.resolve().as_uri()+'?mode=ro',uri=True) as c: result[p.name]=hashlib.sha256(('\\n'.join(c.iterdump())).encode()).hexdigest()
for p in sorted((r/'derived').rglob('*')):
 if p.is_file():result[str(p.relative_to(r))]=hashlib.sha256(p.read_bytes()).hexdigest()
print(json.dumps(result,sort_keys=True))`;
  return run(label,'python3',['-c',script,runtime]).trim();
}
async function listener(f) {
  const args=['--runtime',f.runtime,'--project',f.project,'--bind','127.0.0.1:0','--language',f.language];
  const process = spawn(path.join(config.bin,'volicord-viewer'),args);
  let stdout='',stderr='';
  process.stdout.on('data', b=>stdout+=b);
  const url=await new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(Error('local Viewer startup timeout')),15000);
    process.stderr.on('data',b=>{stderr+=b;const m=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);resolve(m[0]);}});
    process.once('exit',code=>{clearTimeout(timer);reject(Error('Viewer startup exit '+code+stderr));});
    process.once('error',reject);
  });
  return {url, async stop(){await new Promise(resolve=>{
    process.once('exit',(code,signal)=>{fs.writeFileSync(path.join(output,f.key+'-viewer.stdout'),stdout);fs.writeFileSync(path.join(output,f.key+'-viewer.stderr'),stderr);processes.push({label:f.key+'-viewer',args,exit_code:code,signal,termination:'explicit cleanup'});resolve();});
    process.kill('SIGTERM');
  });}};
}
function role(key){return key.split('_').map(x=>x[0].toUpperCase()+x.slice(1)).join('');}
async function inspect(page, f, basis, url, offline, locale, width) {
  const response=await page.goto(url);
  if(!offline)assert.equal(response.status(),200);
  await page.evaluate(()=>document.fonts.ready);
  // The actual entry control selects the exact Work, with keyboard activation.
  if(!offline){
    const card=page.locator(`#overview article[data-work-id="${f.work}"]`).first();
    assert.equal(await card.locator('h4 a').textContent(),f.expected_units.find(u=>u.role==='heading').text);
    assert.equal(await card.locator('[data-question=Synopsis]').textContent(),f.expected_units.find(u=>u.role==='synopsis').text);
    const target=card.locator('h4 a');await target.focus();await page.keyboard.press('Enter');
    await page.waitForURL(url=>url.searchParams.get('work')===f.work);
    assert.equal(new URL(page.url()).searchParams.get('work'),f.work);
  } else {
    const target=page.locator(`#overview article[data-work-id="${f.work}"] h4 a`).first();
    await target.focus();await page.keyboard.press('Enter');
  }
  const work=page.locator(`section[id="work-${f.work}"] article[data-work-id="${f.work}"]`).first();
  await work.waitFor();
  const heading=work.locator('h3[data-question=Heading]').first();
  assert.equal(await heading.textContent(),f.expected_units.find(u=>u.role==='heading').text);
  assert.equal(await heading.getAttribute('data-title-basis'),'generated-interpretation');
  assert.equal(await heading.getAttribute('lang'),f.language,'content language independent of UI locale');
  for(const question of new Set(f.expected_units.map(u=>role(u.role)))){
    const actual=work.locator(`[data-question="${question}"][data-content-language="${f.language}"]`);
    assert.deepEqual(await actual.allTextContents(),f.expected_units.filter(u=>role(u.role)===question).map(u=>u.text),'exact independent authored content for '+question);
  }
  const original=work.locator('.work-goal blockquote');
  assert.ok((await original.textContent()).includes('과거 원래 지시 인용:'),'original task retained without treating it as new authority');
  assert.equal(await original.getAttribute('lang'),'');
  assert.ok(await work.locator('[data-section=Limits]').evaluate(n=>!!(n.compareDocumentPosition(n.parentElement.querySelector('[data-section=Purpose]')) & Node.DOCUMENT_POSITION_FOLLOWING)),'limits precede narrative');
  assert.ok((await work.locator('.explanation-grounding').textContent()).includes(basis.provenance.realization_fingerprint));
  assert.equal(await work.locator('.explanation-grounding').getAttribute('open'),null);
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no ordinary page overflow');
  await heading.scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(output,`${f.key}-${locale}-${width}-${offline?'offline':'http'}-opening.png`)});
  let trips=0;
  // Reading guidance and comparison claims may share one code range. Each point
  // must select its own claim and return to its own original paragraph.
  for(const [index,unit] of f.expected_units.entries()){
    if(!['reading_point','comparison'].includes(unit.role))continue;
    const paragraph=work.locator(`p[data-question="${role(unit.role)}"]`).filter({hasText:unit.text}).first();
    assert.equal(await paragraph.textContent(),unit.text);
    const id=await paragraph.getAttribute('id');
    const citation=work.locator(`a.source-citation[href="#${id}-c1"]`).first();
    assert.equal(await citation.count(),1,'point owns its exact citation control');
    await citation.focus();await page.keyboard.press('Enter');
    await page.waitForURL(url=>url.hash==='#'+id+'-c1');
    assert.equal(await page.evaluate(()=>document.activeElement.id),id+'-c1','native fragment focus selects the exact reading point');
    const point=page.locator(`[id="${id}-c1"]`);
    assert.equal(await point.locator('.source-purpose-claim').textContent(),unit.text);
    const figure=point.locator('xpath=ancestor::figure');
    const expected=f.expected_sources[unit.refs[0]];
    const excerpt=basis.source_excerpts.find(e=>e.citation.source_id===expected.source && e.citation.start_byte===expected.start && e.citation.end_byte===expected.end);
    assert.ok(excerpt,'exact expected Source range in public readback');
    assert.equal(await figure.locator('pre > code').textContent(),excerpt.text);
    assert.ok(expected.body.startsWith(excerpt.text),'permitted displayed bytes equal independent original source prefix');
    assert.ok((await figure.locator('figcaption').textContent()).includes(`${expected.path}:${expected.start_line}–${expected.end_line}`));
    const code=figure.locator('pre[role=region]').first();
    await code.focus();await page.keyboard.press('ArrowRight');
    const scrolling=await code.evaluate(n=>({width:n.clientWidth,scrollWidth:n.scrollWidth,left:n.scrollLeft}));
    if(scrolling.scrollWidth>scrolling.width+2)await page.waitForFunction(n=>n.scrollLeft>0,await code.elementHandle(),{timeout:2000});
    await point.scrollIntoViewIfNeeded();
    if(trips===0)await page.screenshot({path:path.join(output,`${f.key}-${locale}-${width}-${offline?'offline':'http'}-code.png`)});
    await point.locator('.return-to-claim').focus();await page.keyboard.press('Enter');
    await page.waitForURL(url=>url.hash==='#'+id);
    assert.equal(await page.evaluate(()=>document.activeElement.id),id,'exact claim return focus');
    await page.goBack();assert.equal(new URL(page.url()).hash,'#'+id+'-c1','real browser history returns to the selected code point');
    await page.goForward();assert.equal(new URL(page.url()).hash,'#'+id,'real browser history restores the paragraph');
    trips++;
  }
  const ids=await page.locator('[id]').evaluateAll(nodes=>nodes.map(n=>n.id));
  assert.equal(ids.length,new Set(ids).size,'no duplicate document anchors');
  if(offline){
    assert.equal(await page.locator('script,form').count(),0);
    assert.ok(await page.locator('a[href]').evaluateAll(nodes=>nodes.every(n=>n.getAttribute('href').startsWith('#'))));
  }
  observations.push({key:f.key,locale,width,offline,round_trips:trips,source_claims:'exact',ordinary_overflow:false});
}
function receipt(status,error=null){fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status,error,observations,processes,
  fixture_sha256:hash(fs.readFileSync(config.fixture)),candidate:config.candidate,
  binaries:Object.fromEntries(['volicord','volicord-viewer','volicord-mcp'].map(n=>[n,hash(fs.readFileSync(path.join(config.bin,n)))])),
  content_classification:'source-derived authored controls; no autonomous generation',human_review:'pending',
  unavailable_checks:['native browser zoom','screen-reader use','human comprehension']},null,2));}
(async()=>{
  assert.ok(run('korean-fonts','fc-list',[':lang=ko']).trim(),'Korean font prerequisite');
  const fixture=JSON.parse(fs.readFileSync(config.fixture));
  const browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox']});
  try{
    for(const f of fixture.cases){
      const before=retained(f.runtime,f.key+'-storage-before');
      const status=JSON.parse(run(f.key+'-status',path.join(config.bin,'volicord'),['--runtime',f.runtime,'--project',f.project,'--json','status','--language',f.language]));
      const basis=status.work_history.find(w=>w.work_item_id===f.work).answers;
      assert.equal(basis.explanation_state,'current');
      assert.deepEqual(basis.prose.map(p=>p.text),f.expected_units.map(u=>u.text));
      const live=await listener(f);
      try{
        for(const locale of ['en','ko']){
          const snapshot=path.join(output,`${f.key}-${locale}.html`);
          run(f.key+'-'+locale+'-snapshot',path.join(config.bin,'volicord-viewer'),['--runtime',f.runtime,'--project',f.project,'--locale',locale,'--language',f.language,'--snapshot',snapshot]);
          for(const width of [1440,390,320]){
            const context=await browser.newContext({viewport:{width,height:900}});
            try{
              const page=await context.newPage();
              await inspect(page,f,basis,`${live.url}?locale=${locale}&language=${f.language}`,false,locale,width);
              // Same maintained prose consumer and exact fragments on Code view.
              const explore=page.locator(`a[href*="view=code"][href*="work=${f.work}"]`).last();
              await explore.focus();await page.keyboard.press('Enter');
              await page.waitForURL(url=>url.searchParams.get('view')==='code' && url.searchParams.get('work')===f.work);
              const meaning=page.locator('.code-work-meaning');
              assert.equal(await meaning.locator('h3').first().textContent(),f.expected_units[0].text);
              assert.deepEqual(await meaning.locator('p[data-question=ReadingPoint]').allTextContents(),f.expected_units.filter(u=>u.role==='reading_point').map(u=>u.text));
              assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
              await context.setOffline(true);
              const unavailable=f.runtime+'.offline-check';
              fs.renameSync(f.runtime,unavailable);
              try{await inspect(page,f,basis,pathToFileURL(snapshot).href,true,locale,width);}
              finally{fs.renameSync(unavailable,f.runtime);}
            }finally{await context.close();}
          }
        }
      }finally{await live.stop();}
      assert.equal(retained(f.runtime,f.key+'-storage-after'),before,'HTTP/navigation/CLI/export leave canonical and managed content, retention and indexes unchanged');
    }
  }finally{await browser.close();}
  receipt('passed');console.log(JSON.stringify({status:'passed',screens:observations.length,round_trips:observations.reduce((n,o)=>n+o.round_trips,0),human_review:'pending'}));
})().catch(error=>{receipt('failed',error.stack);console.error(error.stack);process.exitCode=1;});
