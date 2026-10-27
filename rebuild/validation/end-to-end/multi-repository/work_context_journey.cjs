// Coupled list/detail/native-history checks over actual public GETs.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const frame=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
async function positioned(row){await row.evaluate(n=>{window.scrollBy(0,n.getBoundingClientRect().top-250);n.focus({preventScroll:true});});return row.evaluate(n=>({id:n.id,top:n.getBoundingClientRect().top,y:scrollY}));}
async function restored(page,expected){
 await page.waitForFunction(id=>document.activeElement?.id===id,expected.id);await frame(page);
 const actual=await page.locator(`[id="${expected.id}"]`).evaluate(n=>({top:n.getBoundingClientRect().top,focus:n===document.activeElement}));
 assert.equal(actual.focus,true);assert.ok(Math.abs(actual.top-expected.top)<3,JSON.stringify({expected,actual}));
}
module.exports=async function({page,context,address,item,output,result}){
 for(const width of [1440,1024,768,390,320]){
  await page.setViewportSize({width,height:900});
  await page.goto(item.synthetic?address('work','&q=needle&sort=oldest&page=1'):address());await frame(page);
  const row=item.synthetic?page.locator('.work-row').nth(5):page.locator(`.work-row[data-work-id="${item.work}"]`);
  assert.equal(await row.count(),1,'the intended Work is in the actual query');
  const expected=await positioned(row);const target=await row.getAttribute('href');
  assert.equal(new URL(target,page.url()).searchParams.get('from'),'list');
  if(width===1440){
   const before=await page.evaluate(()=>({url:location.href,y:scrollY,focus:document.activeElement.id,state:history.state}));
   const popup=context.waitForEvent('page');await row.click({modifiers:['Control']});const tab=await popup;await tab.waitForLoadState('load');assert.equal(await tab.locator('.doc[data-work-id]').first().getAttribute('data-work-id'),expected.id.slice(4));await tab.close();await page.bringToFront();await frame(page);
   assert.deepEqual(await page.evaluate(()=>({url:location.href,y:scrollY,focus:document.activeElement.id,state:history.state})),before);
  }
  const detailStarted=performance.now();await row.click();await page.waitForSelector('.doc[data-work-id]');await frame(page);
  (result.read_stages||=[]).push({case:item.key,width,stage:'detail_with_its_comparison',milliseconds:performance.now()-detailStarted});
  assert.equal(await page.locator('.doc[data-work-id]').first().getAttribute('data-work-id'),expected.id.slice(4));
  assert.equal(await page.locator('.project-navigation [aria-current=page]').getAttribute('data-destination'),'work');
  assert.equal(await page.locator('.recorded-timeline').count(),1);assert.ok(await page.locator('.document-nav').count());
  if(item.expected_reports!==undefined){assert.equal(await page.locator('.work-timeline>li').count(),item.expected_reports);assert.equal(await page.locator('.doc[data-work-state]').first().getAttribute('data-work-state'),'paused');}
  await page.screenshot({path:path.join(output,`${item.key}-detail-${width}-initial.png`)});
  const breadcrumb=page.locator('.doc>.breadcrumb a').first(),back=await breadcrumb.getAttribute('href');
  assert.equal(new URL(back,page.url()).hash,'#'+expected.id);
  if(item.synthetic){assert.equal(new URL(back,page.url()).searchParams.get('q'),'needle');assert.equal(new URL(back,page.url()).searchParams.get('page'),'1');}
  await page.reload();await frame(page);assert.equal(await breadcrumb.getAttribute('href'),back);
  if(item.evidence_required){
   const trigger=page.locator('.doc a.evidence-link:visible').first();assert.ok(await trigger.count(),'actual evidence trigger required');
   await trigger.scrollIntoViewIfNeeded();await trigger.focus();const triggerTop=await trigger.evaluate(n=>n.getBoundingClientRect().top);
   const sourceStarted=performance.now();await trigger.click();const point=page.locator('.source-point[data-selected=true]');await point.waitFor({state:'visible'});
   const control=point.locator('.citation-inspection');assert.equal(await control.count(),1);
   await page.waitForFunction(()=>{const n=document.querySelector('.source-point[data-selected=true] .citation-inspection .citation-read');return n&&!n.disabled;});
   for(let n=0;n<4&&Number(await control.locator('button').getAttribute('data-next-offset'))>0;n++){await control.locator('button').click();await page.waitForFunction(()=>!document.querySelector('.source-point[data-selected=true] .citation-inspection button').disabled);}
   const binding=JSON.parse(await control.getAttribute('data-citation-binding'));
   const expectedCode=fs.readFileSync(path.join(item.repository,binding.locator)).subarray(binding.start_byte,binding.end_byte).toString('utf8');
   assert.equal(await control.locator('pre code').innerText(),expectedCode,'complete original cited bytes');
   result.read_stages.push({case:item.key,width,stage:'complete_exact_source_range',milliseconds:performance.now()-sourceStarted,utf8_bytes:Buffer.byteLength(expectedCode)});
   await page.screenshot({path:path.join(output,`${item.key}-detail-${width}-evidence.png`)});
   await point.locator('a.return').first().click();await frame(page);
   assert.equal(await trigger.evaluate(n=>n===document.activeElement),true);assert.ok(Math.abs(await trigger.evaluate(n=>n.getBoundingClientRect().top)-triggerTop)<3,'exact visible evidence return');
  }
  // Native cross-document history, then explicit return and reload.
  while(new URL(page.url()).hash)await page.goBack();
  await page.goBack();await restored(page,expected);
  await page.goForward();await page.waitForSelector('.doc[data-work-id]');await breadcrumb.click();await restored(page,expected);
  await page.reload();await restored(page,expected);
  // A direct entry has no invented origin, even after a previous list visit.
  await page.goto(address('work','&work='+expected.id.slice(4)));await frame(page);
  const direct=new URL(await breadcrumb.getAttribute('href'),page.url());assert.equal(direct.hash,'');assert.equal(direct.searchParams.has('q'),false);assert.equal(direct.searchParams.has('page'),false);
  result.checks.push({case:item.key,width,journey:'list/detail/evidence/native-back-forward/explicit-return/reload/direct-entry',evidence:!!item.evidence_required,selected:expected.id});
 }
 if(item.synthetic){
  // Full-document navigation must discard a late response rather than splice
  // its metadata into a new Project. The old response is obtained from GET.
  await page.goto(address('work','&q=needle&sort=oldest'));
  const row=page.locator('.work-row').first(),id=await row.getAttribute('data-work-id');
  let release,observed;const held=new Promise(r=>release=r),started=new Promise(r=>observed=r);
  const delay=async route=>{const u=new URL(route.request().url());if(u.searchParams.get('work')===id){const response=await route.fetch();observed();await held;await route.fulfill({response}).catch(()=>{});}else await route.continue();};
  await page.route('**/*',delay);await row.evaluate(n=>n.click());await started;
  const foreign=new URL(address('overview'));foreign.searchParams.set('project',item.empty);
  await page.goto(foreign.href);release();await frame(page);await page.unroute('**/*',delay);
  assert.equal(await page.locator('.doc[data-work-id]').count(),0);assert.equal(await page.locator('.work-row').count(),0);assert.match(await page.locator('main').innerText(),/No Work has been recorded/);
  result.checks.push({case:item.key,journey:'late detail response cannot replace another Project'});
 }
};

module.exports.deleteSelection=async function({page,address,item,runtime,cli,output,result}){
 assert.ok(cli,'actual CLI required for synthetic deletion control');
 await page.goto(address('work','&q=needle&sort=oldest&page=1'));
 const row=page.locator('.work-row').first(),id=await row.getAttribute('data-work-id');await row.click();
 const checkpoint=await page.locator('.work-timeline>li').first().getAttribute('data-checkpoint-id');
 for(const [kind,identity] of [['checkpoint',checkpoint],['context_item',id]]){
  const argv=['--runtime',runtime,'--project',item.project,'--json','advanced','records','forget',kind,identity,'--source',item.source];
  const run=require('node:child_process').spawnSync(cli,argv,{encoding:'utf8',timeout:30000});
  fs.writeFileSync(path.join(output,`${item.key}-forget-${kind}.json`),JSON.stringify({argv,status:run.status,signal:run.signal,stdout:run.stdout,stderr:run.stderr,error:run.error?.message},null,2),{flag:'wx'});
  assert.equal(run.status,0,'supported canonical deletion in disposable synthetic Runtime');
 }
 const response=await page.reload();assert.equal(response.status(),404);assert.equal(await page.locator('.doc').count(),0);
 const back=page.locator('.catalogue-return');assert.equal(await back.count(),1);assert.equal(new URL(await back.getAttribute('href'),page.url()).searchParams.get('q'),'needle');
 await back.click();assert.equal(await page.locator('.catalogue-refresh').count(),1);assert.equal(await page.locator(`.work-row[data-work-id="${id}"]`).count(),0);
 assert.equal(new URL(await page.locator('#works').getAttribute('data-catalogue-route'),page.url()).searchParams.get('page'),'0');
 result.checks.push({case:item.key,journey:'actual Checkpoint/Goal deletion; missing detail; preserved query with explicit refresh',deleted:id});
};
