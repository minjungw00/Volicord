// Supporting browser review of the actual Rust Product, using maintained control records.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process');

function reviewedFiles(directory){const files={};for(const name of fs.readdirSync(directory).sort()){const file=path.join(directory,name);if(!fs.lstatSync(file).isFile()||!['.png','.html'].includes(path.extname(name)))continue;const bytes=fs.readFileSync(file);files[name]={sha256:require('node:crypto').createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length};}return files;}
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const fixture=JSON.parse(fs.readFileSync(config.fixture,'utf8'));
const {chromium}=require(config.playwright);
const outputRoot=path.resolve(config.output);fs.mkdirSync(outputRoot,{recursive:true});const output=fs.mkdtempSync(path.join(outputRoot,'run-'));
const result={candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
  diff_sha256:require('node:crypto').createHash('sha256').update(execFileSync('git',['diff','HEAD','--','rebuild'])).digest('hex'),
  viewer_sha256:require('node:crypto').createHash('sha256').update(fs.readFileSync(config.viewer)).digest('hex'),
  fixture_sha256:require('node:crypto').createHash('sha256').update(fs.readFileSync(config.fixture)).digest('hex'),
  driver_sha256:require('node:crypto').createHash('sha256').update(fs.readFileSync(__filename)).digest('hex'),synthetic_controls:true,human_acceptance:'pending',cases:[],screenshots:[],listeners:[]};
async function serve(project){
 const argv=['--runtime',fixture.runtime,'--project',project,'--bind','127.0.0.1:0'];
 const receipt={project,argv:[config.viewer,...argv],started_at:new Date().toISOString(),stdout:`listener-${result.listeners.length}.stdout`,stderr:`listener-${result.listeners.length}.stderr`};result.listeners.push(receipt);
 const child=spawn(config.viewer,argv,{stdio:['ignore','pipe','pipe']});
 let stderr='',stdout='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);
 const exit=new Promise(resolve=>child.on('exit',(code,signal)=>resolve({code,signal})));
 const base=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error(stderr||'listener timeout')),15000);child.stderr.on('data',()=>{const match=stderr.match(/http:\/\/127.0.0.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});child.on('exit',()=>{clearTimeout(timer);reject(Error(stderr));});});
 return {base,async stop(){child.kill('SIGTERM');receipt.termination=await exit;receipt.ended_at=new Date().toISOString();fs.writeFileSync(path.join(output,receipt.stderr),stderr);fs.writeFileSync(path.join(output,receipt.stdout),stdout);}};
}
(async()=>{let browser,listener;try{
 listener=await serve(fixture.project);browser=await chromium.launch({executablePath:config.chromium,headless:true,env:{...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})},args:['--no-sandbox','--disable-dev-shm-usage']});
 result.browser_version=browser.version();
 for(const width of [1440,390,320]){
  const page=await browser.newPage({viewport:{width,height:1000}});
  const failed=[];page.on('pageerror',e=>failed.push(e.message));
  async function open(target){const response=await page.goto(new URL(target,listener.base).href);assert.equal(response.status(),200,target);}
  async function shot(name){assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'horizontal overflow');const file=`${width}-${name}.png`;await page.screenshot({path:path.join(output,file),fullPage:false});result.screenshots.push(file);}
  await open('/?view=tools&tool=memory');assert.equal(await page.locator('.masthead nav').count(),1);
  assert.equal(await page.locator('.inspection-query').count(),1);assert.ok(await page.locator('.inspection-record').count()<=10);
  assert.equal(await page.locator('pre:visible').count(),0);await shot('records');
  const search=page.locator('input[name=q]');await search.focus();assert.equal(await search.evaluate(n=>document.activeElement===n),true);
  await search.fill('not-present-record-marker');await page.locator('.inspection-query button').click();
  assert.equal(await page.locator('.inspection-record').count(),0);await shot('empty-filter');
  await page.locator('input[name=q]').fill('');await page.locator('.inspection-query select[name=class]').selectOption('checkpoint');
  await page.locator('.inspection-filters>summary').click();
  await page.locator('.inspection-query select[name=limit]').selectOption('5');
  await page.locator('.inspection-query button').click();assert.ok(await page.locator('.inspection-record').count()>0);
  const ids=await page.locator('.inspection-record .record-meta code').allTextContents();
  const next=page.locator('.inspection-paging a').filter({hasText:'Next records'});assert.equal(await next.count(),1);
  await next.click();const later=await page.locator('.inspection-record .record-meta code').allTextContents();
  assert.ok(later.every(id=>!ids.includes(id)));await shot('checkpoint-page');
  await open(`/inspection?class=work&id=${fixture.work}`);assert.ok(await page.locator('.inspection-relationships a').count()>0);
  await page.locator('.inspection-field select[name=field]').selectOption('statement');await page.locator('.inspection-field button').click();
  assert.ok((await page.locator('#field-detail pre').first().textContent()).length>0);await page.locator('#field-detail').scrollIntoViewIfNeeded();await shot('exact-field');
  await page.locator('.inspection-record a').filter({hasText:'Read Work document'}).click();assert.ok(await page.locator('.doc').count()>0);
  const evidence=page.locator('.doc .evidence-index a[href^="#"]');const evidenceLinks=await evidence.count();assert.ok(evidenceLinks>0);
  await page.locator('.doc .evidence-index .evidence-entry>summary').first().click();await evidence.first().click();assert.ok(new URL(page.url()).hash);
  result.cases.push({width,journey:'Work evidence index deep link',identity:fixture.work,evidence_links:evidenceLinks});
  await open(`/inspection?class=decision&id=${fixture.decision}`);await page.locator('.inspection-record a').filter({hasText:'Read Decision document'}).click();assert.ok(await page.locator('.doc').count()>0);
  const decisionEvidence=page.locator('.doc .evidence-index a[href^="#"]');assert.ok(await decisionEvidence.count()>0);await page.locator('.doc .evidence-index .evidence-entry>summary').first().click();await decisionEvidence.first().click();assert.ok(new URL(page.url()).hash);
  result.cases.push({width,journey:'Decision evidence index deep link',identity:fixture.decision,status:'passed'});
  await open(`/inspection?class=candidate&id=${fixture.expired}`);assert.ok((await page.locator('.inspection-record').textContent()).includes('expired'));assert.ok(!(await page.content()).includes('expired-review-secret'));await shot('expired-candidate');
  await open(`/inspection?class=derived&id=${fixture.derived}`);assert.ok(!(await page.content()).includes('derived-review-secret'));await shot('derived-metadata');
  const sources=await page.request.get(new URL('/inspection.json?class=source&q=User',listener.base).href);const source=(await sources.json()).page.rows[0];assert.ok(source);
  await open(`/inspection?class=source&id=${source.identity}&revision=${source.revision}&field=body`);assert.ok((await page.locator('#field-detail').textContent()).includes('unavailable'));assert.ok(!(await page.content()).includes('private-source-marker'));await page.locator('#field-detail').scrollIntoViewIfNeeded();await shot('source-body-unavailable');
  await open('/?view=tools&tool=evidence');await page.locator('.inspection-support>summary').click();assert.ok(await page.locator('#checkpoints').isVisible());
  await open('/?view=tools&tool=status');assert.ok(await page.locator('#health').count());assert.ok((await page.textContent('main')).includes('volicord analyze'));await shot('analysis');
  await open('/?view=tools&tool=documents');assert.equal(await page.locator('.document-preview').count(),4);await page.locator('.document-preview>summary').first().click();await shot('documents');
  assert.equal(failed.length,0,JSON.stringify(failed));result.cases.push({width,journey:'search/filter/pagination/detail/restrictions/tools',status:'passed'});await page.close();
 }
 // Public HTTP checks use the real listening executable, in addition to Rust transport controls.
 const context=await browser.newContext();const request=context.request;
 for(const query of ['class=source&q=private-source-marker','class=candidate&q=expired-review-secret','class=derived&q=derived-review-secret',"class=work&q=%27%20OR%201%3D1%20--"]){const response=await request.get(new URL('/inspection.json?'+query,listener.base).href);assert.equal(response.status(),200);assert.equal((await response.json()).page.permitted_match_count,0);result.cases.push({query,status:response.status(),matches:0});}
 const foreign=await request.get(new URL('/inspection.json?class=work&id='+fixture.empty_project,listener.base).href);assert.equal(foreign.status(),404);
 await context.close();await listener.stop();listener=null;
 listener=await serve(fixture.empty_project);const empty=await browser.newPage();await empty.goto(new URL('/inspection?class=work',listener.base).href);assert.equal(await empty.locator('.inspection-record').count(),0);await empty.screenshot({path:path.join(output,'empty-project.png')});result.screenshots.push('empty-project.png');await empty.close();
 await listener.stop();listener=null;
 const snapshot=path.join(output,'product-snapshot.html');if(fs.existsSync(snapshot))fs.unlinkSync(snapshot);
 execFileSync(config.viewer,['--runtime',fixture.runtime,'--project',fixture.project,'--snapshot',snapshot]);
 const staticPage=await browser.newPage({javaScriptEnabled:false});const html=fs.readFileSync(snapshot,'utf8');assert.ok(!/<script|<form|request_authenticity|\/inspection\?/.test(html));assert.ok(!html.includes('private-source-marker'));assert.ok(!html.includes('expired-review-secret'));assert.ok(!html.includes('derived-review-secret'));
 await staticPage.goto(require('node:url').pathToFileURL(snapshot).href);const anchors=await staticPage.locator('a[href^="#"]').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('href').slice(1)));for(const id of anchors)assert.ok(await staticPage.locator(`[id="${id}"]`).count(),id);await staticPage.close();
 result.cases.push({journey:'script-free offline permitted subset',status:'passed',native_anchors:anchors.length});result.status='passed';
 }catch(error){result.status='failed';result.error=error.stack;process.exitCode=1;}finally{
 if(listener)await listener.stop();if(browser)await browser.close();
 fs.writeFileSync(path.join(output,'review.html'),'<!doctype html><meta charset="utf-8"><title>Project inspection review</title><h1>Actual Product inspection review</h1><p>Supporting synthetic controls; human acceptance pending. See result.json for candidate/executable identity.</p><p><a href="product-snapshot.html">Script-free Product snapshot</a> · <a href="result.json">Results</a></p>'+result.screenshots.map(s=>`<p><a href="${s}">${s}</a></p><img style="max-width:100%" src="${s}" alt="${s}">`).join(''));
 result.files=reviewedFiles(output);fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));
 fs.writeFileSync(path.join(outputRoot,'current.json'),JSON.stringify({output,status:result.status,candidate:result.candidate},null,2));
 fs.writeFileSync(path.join(outputRoot,'current.html'),`<!doctype html><meta charset="utf-8"><title>Current inspection review</title><a href="${path.basename(output)}/review.html">Open current Product inspection review (${result.status})</a>`);
 console.log(JSON.stringify({status:result.status,cases:result.cases.length,error:result.error,output}));
 }})();
