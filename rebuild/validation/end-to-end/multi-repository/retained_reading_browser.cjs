// Real TTL transition on one normally admitted polyglot diagram publication.
// Focused local support; no provider, regeneration or semantic/design verdict.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {spawn, spawnSync} = require('node:child_process');
const {pathToFileURL} = require('node:url');
const {createOutput} = require('../../viewer-design/product_review_controls.cjs');
const config = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
createOutput(config.output);
const write = (name, data) => fs.writeFileSync(path.join(config.output, name), data, {flag:'wx'});
const result = {outcome:'failed', commands:[], external_requests:[], scope:'technical retained-reading controls; human/R3 acceptance separate'};
let listener, browser, sequence = 0;
function command(label, argv, env = {}) {
  const child = spawnSync(argv[0], argv.slice(1), {env:{...process.env, ...env}, maxBuffer:8*1024*1024, timeout:180000});
  const key = `${++sequence}-${label}`;
  write(key+'.stdout', child.stdout || ''); write(key+'.stderr', child.stderr || '');
  result.commands.push({label, argv, exit:child.status, signal:child.signal, error:child.error?.message});
  assert.equal(child.error, undefined, `${label}: process observation failed`);
  assert.equal(child.status, 0, `${label}: ${child.stderr}`);
  return child.stdout.toString();
}
async function start(fixture) {
  let stderr = '', stdout = '';
  const process = spawn(config.viewer, ['--runtime',fixture.runtime,'--project',fixture.project,'--bind','127.0.0.1:0','--view','repository']);
  process.stdout.on('data', b => stdout += b); process.stderr.on('data', b => stderr += b);
  const exit = new Promise(resolve => process.on('close', (code, signal) => resolve({code, signal})));
  const base = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(Error('listener timeout: '+stderr)), 10000);
    process.once('error', error => {clearTimeout(timer); reject(error);});
    process.stderr.on('data', () => {const match=stderr.match(/http:\/\/127\.0\.0\.1:\d+\//); if(match){clearTimeout(timer); resolve(match[0]);}});
  });
  return {base, async stop() {process.kill('SIGTERM'); const status=await exit;
    write(`listener-${++sequence}.stdout`,stdout); write(`listener-${sequence}.stderr`,stderr); result.commands.push({label:'listener',...status});}};
}
(async () => {
  try {
    const fixturePath = path.join(config.output,'fixture');
    command('seed', config.seed_command, {VOLICORD_RETAINED_REVIEW_ROOT:fixturePath});
    const f = JSON.parse(fs.readFileSync(path.join(fixturePath,'fixture.json'),'utf8'));
    const prefix=[config.cli,'--runtime',f.runtime,'--project',f.project];
    listener = await start(f);
    browser = await require(config.playwright).chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox'],
      env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})}});
    const context=await browser.newContext();
    await context.route('**/*', route => {
      if(new URL(route.request().url()).origin===new URL(listener.base).origin) return route.continue();
      result.external_requests.push(route.request().url()); return route.abort();
    });
    const page=await context.newPage(); await page.setViewportSize({width:1440,height:900});
    await page.goto(listener.base+'?view=repository');
    const point=(u,c)=>page.locator(`.source-point:has(.citation-inspection[data-unit="${u}"][data-citation="${c}"])`);
    const initial=point(1,0), id=await initial.getAttribute('id');
    await page.goto(listener.base+'?view=repository#'+id);
    await initial.locator('.citation-status').filter({hasText:'Displayed'}).waitFor();
    assert.equal(await initial.locator('.citation-inspection code').textContent(),f.originals[8]);
    const expectedUnits=await page.locator('#repository-explanation .unit-text').allTextContents();
    const diagrams=await page.locator('#repository-explanation .admitted-diagram').count(); assert.ok(diagrams);
    await page.screenshot({path:path.join(config.output,'before-expiry.png')});
    const wait=Math.max(0,Math.ceil(f.expires_at/1000-Date.now())+100);
    assert.ok(wait<60000,'seed must have sufficient remaining real TTL');
    await page.waitForTimeout(wait);
    await initial.locator('.citation-read').click();
    await initial.locator('.citation-status').filter({hasText:'expired'}).waitFor();
    assert.equal(await initial.locator('pre.code-region').count(),0);
    assert.equal(await page.locator('#repository-explanation .admitted-diagram').count(),diagrams);
    assert.deepEqual(await page.locator('#repository-explanation .unit-text').allTextContents(),expectedUnits);
    assert.equal(await page.locator('.publication-unavailable').count(),0);
    await page.screenshot({path:path.join(config.output,'same-page-expired-range.png')});
    const validId=await point(1,1).getAttribute('id');
    await page.goto(listener.base+'?view=repository#'+validId);
    await point(1,1).locator('.citation-status').filter({hasText:'Displayed'}).waitFor();
    assert.equal(await point(1,1).locator('.citation-inspection code').textContent(),f.originals[9]);
    assert.ok(await page.locator('[data-range-state="expired"]').count());
    fs.appendFileSync(path.join(f.repository,'inventory0.py'),'# changed uncited dependency\n');
    await point(1,1).locator('.citation-read').click();
    await page.locator('.applicability[data-comparison-state="changed"]').waitFor();
    assert.equal(await point(1,1).locator('.citation-inspection code').textContent(),f.originals[9]);
    fs.renameSync(f.repository,f.repository+'.unavailable');
    try {
      await point(1,1).locator('.citation-read').click();
      await page.locator('.applicability[data-comparison-state="unavailable"]').waitFor();
      assert.equal(await point(1,1).locator('.citation-inspection code').textContent(),f.originals[9]);
      assert.equal(await page.locator('#repository-explanation .admitted-diagram').count(),diagrams);
    } finally {fs.renameSync(f.repository+'.unavailable',f.repository);}
    const cli=JSON.parse(command('cli-read',[...prefix,'--json','repository','explain','read','--language','en']));
    assert.equal(cli.answers.provenance.publication_id,f.publication);
    assert.deepEqual(cli.answers.prose,f.answers.prose);
    assert.ok(cli.answers.applicability.source_ranges.some(r=>r.limitation==='expired'));
    assert.ok(!cli.answers.source_excerpts.some(e=>e.text===f.originals[8]));
    assert.ok(cli.answers.source_excerpts.some(e=>e.text===f.originals[9]));
    const human=command('cli-human',[...prefix,'repository','explain','read','--language','en']);
    assert.ok(human.includes('expired')&&!human.includes('return verify(value)'));
    const requests=[{jsonrpc:'2.0',id:1,method:'initialize',params:{capabilities:{}}},
      {jsonrpc:'2.0',id:2,method:'tools/call',params:{name:'repository_understanding',arguments:{project_id:f.project,requested_language:'en',reading_focus:'repository_explanation'}}},
      {jsonrpc:'2.0',id:3,method:'tools/call',params:{name:'explanation_read',arguments:{project_id:f.project,subject_kind:'repository',subject_id:f.analysis,language:'en',action:'page',offset:0}}}];
    const mcp=spawnSync(config.mcp,[],{input:requests.map(r=>JSON.stringify(r)).join('\n')+'\n',env:{...process.env,VOLICORD_RUNTIME_DIR:f.runtime},maxBuffer:8*1024*1024,timeout:30000});
    write('mcp.stdout',mcp.stdout||'');write('mcp.stderr',mcp.stderr||'');result.commands.push({label:'mcp',exit:mcp.status,signal:mcp.signal});assert.equal(mcp.status,0);
    const responses=mcp.stdout.toString().trim().split('\n').map(s=>JSON.parse(s));
    const answers=responses.find(r=>r.id===2).result.structuredContent.repository_explanation;
    assert.deepEqual(answers.prose,f.answers.prose);assert.equal(answers.provenance.publication_id,f.publication);
    assert.ok(!answers.source_excerpts.some(e=>e.text===f.originals[8]));assert.ok(answers.source_excerpts.some(e=>e.text===f.originals[9]));
    assert.equal(responses.find(r=>r.id===3).result.structuredContent.publication_id,f.publication);
    const offline=path.join(config.output,'after-expiry.html');
    command('export',[...prefix,'--json','viewer','export','--output',offline,'--language','en']);
    await context.close(); await listener.stop(); listener=null;
    // A new listener process opens the same publication and permission state.
    listener=await start(f);
    const restarted=await browser.newPage();await restarted.goto(listener.base+'?view=repository');
    assert.equal(await restarted.locator('#repository-explanation .admitted-diagram').count(),diagrams);
    assert.ok(await restarted.locator('[data-range-state="expired"]').count());await restarted.close();
    const nojs=await browser.newContext({javaScriptEnabled:false});const native=await nojs.newPage();
    for(const width of [1440,390]) {
      await native.setViewportSize({width,height:900});await native.goto(pathToFileURL(offline).href+'#repository-explanation');
      assert.equal(await native.locator('script').count(),0);assert.ok(await native.locator('.admitted-diagram').count());
      const code=await native.locator('code').allTextContents();assert.ok(!code.includes(f.originals[8]));assert.ok(code.includes(f.originals[9]));
      assert.ok(await native.locator('[data-range-state="expired"]').count());
      await native.screenshot({path:path.join(config.output,`offline-${width}.png`)});
    }
    await nojs.close();assert.deepEqual(result.external_requests,[]);
    result.outcome='passed';result.publication=f.publication;result.real_ttl_expiry=true;result.same_page_meaning_preserved=true;result.restart=true;
  } catch(error) {result.error=error.stack;process.exitCode=1;}
  finally {if(browser)await browser.close();if(listener)await listener.stop();write('result.json',JSON.stringify(result,null,2));console.log(JSON.stringify(result));}
})();
