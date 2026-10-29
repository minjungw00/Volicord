// Focus/history/expansion/race controls on an actual admitted live document.
const assert=require('node:assert/strict');
const pause=ms=>new Promise(r=>setTimeout(r,ms));
const settle=async page=>{await page.bringToFront();await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await pause(100);};
async function complete(page){await page.waitForFunction(()=>{const p=document.querySelector('.source-point[data-selected=true]');const b=p?.querySelector('.citation-read');return b&&!b.disabled&&b.dataset.nextOffset==='0'&&p.querySelector('.citation-inspection code');});}
async function inspect(page,context,{address,figureId,elementId,claimId,sourceId,width}){
 const figure=page.locator(`[id="${figureId}"]`),element=page.locator(`[id="${elementId}"]`),point=()=>page.locator('.source-point[data-selected=true]');
 const notes=[];
 async function open(){await element.focus();await page.keyboard.press('Enter');await complete(page);await settle(page);}
 // Native modified activation keeps the current tab's selection, URL and scroll.
 await element.focus();await settle(page);
 const state=()=>page.evaluate(()=>({url:location.href,y:scrollY,focus:document.activeElement?.id,selected:[...document.querySelectorAll('.source-point[data-selected]')].map(n=>n.id),open:[...document.querySelectorAll('details')].map(n=>n.open)}));
 const before=await state();
 const [tab]=await Promise.all([context.waitForEvent('page',{timeout:15000}),element.click({modifiers:['Control']})]);
 await tab.waitForURL(url=>url.hash==='#'+claimId,{waitUntil:'load',timeout:15000});assert.ok(tab.url().endsWith('#'+claimId));await tab.close();await settle(page);assert.deepEqual(await state(),before,'native modifier leaves original context');notes.push('native modifier');
 await open();
 if(width<1200){
  for(let i=0;i<20;i++){await page.keyboard.press(i%2?'Shift+Tab':'Tab');assert.equal(await point().evaluate(n=>n.contains(document.activeElement)),true,'modal focus stays in evidence');}
  if(width>760)await page.locator('.evidence-backdrop').click({position:{x:8,y:100}});else await page.keyboard.press('Escape');
  await settle(page);assert.equal(await page.locator('.source-point[data-selected]').count(),0);await open();
 }
 // Return, Back and reload all resolve the same publication and exact element.
 const sourceURL=page.url();await page.keyboard.press('Escape');await settle(page);await page.goBack();await complete(page);assert.equal(page.url(),sourceURL);await page.reload();await complete(page);await page.keyboard.press('Escape');await settle(page);assert.equal(await element.evaluate(n=>n===document.activeElement),true,'reload returns to exact scoped element');notes.push('history and reload');
 // Resizing changes modality without retaining an old inert scope or stale code.
 await open();const selected=await point().getAttribute('id');
 for(const w of [1200,1199,981,980,761,760,390,1440,width]){await page.setViewportSize({width:w,height:900});await settle(page);assert.equal(await point().getAttribute('id'),selected);assert.equal(await point().getAttribute('aria-modal'),w<1200?'true':null);assert.equal(await page.locator('main').evaluate(n=>n.inert),false);}
 await page.keyboard.press('Escape');await settle(page);assert.equal(await page.locator('.project-shell').evaluate(n=>n.inert),false);notes.push('modality resize');
 // Expanded reading preserves the selected figure, pan/zoom and exact origin.
 const expand=figure.locator('[data-diagram-action=expand]');await expand.click();assert.equal(await expand.getAttribute('aria-expanded'),'true');await figure.locator('[data-diagram-action=in]').click();assert.equal(await figure.locator('.diagram-scale').textContent(),'125%');
 await element.focus();await settle(page);const expandedBefore=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left}));await page.keyboard.press('Enter');await complete(page);await page.keyboard.press('Escape');await settle(page);
 assert.equal(await expand.getAttribute('aria-expanded'),'true');assert.equal(await element.evaluate(n=>n===document.activeElement),true);const expandedAfter=await element.evaluate(n=>({top:n.getBoundingClientRect().top,left:n.getBoundingClientRect().left}));assert.ok(Math.abs(expandedBefore.top-expandedAfter.top)<3,'expanded visible return '+JSON.stringify({expandedBefore,expandedAfter}));await page.keyboard.press('Escape');await settle(page);assert.equal(await expand.getAttribute('aria-expanded'),'false');notes.push('expanded exact return');
 // A delayed real response cannot populate the newly chosen range or claim.
 await page.goto(address);await settle(page);
 const first=page.locator(`[id="${elementId}"]`);let held=false;
 await context.route('**/citation?*',async route=>{if(!held){held=true;await pause(700);}await route.continue().catch(()=>{});});
 await first.focus();await page.keyboard.press('Enter');
 const range=point().locator('.point-others a:not([aria-current])').first();
 if(await range.count()){
  const chooser=point().locator('.range-selector>summary');if(await chooser.count())await chooser.click();
  const next=(await range.getAttribute('href')).slice(1);await range.click();await complete(page);await pause(850);assert.equal(await point().getAttribute('id'),next);assert.ok(await point().locator('.citation-inspection code').textContent());notes.push('late range response isolation');
 }else{await complete(page);notes.push('one range; alternate range not applicable');}
 await context.unroute('**/citation?*');await page.keyboard.press('Escape');await settle(page);
 // A native shared-payload alias must activate its own original claim binding.
 const alias=page.locator('.shared-source-target').first();
 if(await alias.count()){
  const id=await alias.getAttribute('id'),own=await alias.getAttribute('data-selected-point');await page.goto(address+'#'+id);await complete(page);assert.equal(await point().getAttribute('id'),own);const inspection=point().locator('.citation-inspection');const binding=JSON.parse(await inspection.getAttribute('data-citation-binding'));assert.ok(binding.claim_sha256);assert.equal(await point().getAttribute('data-origin'),'reader-'+own.split('reader-')[1].replace(/-c\d+$/,''));notes.push('shared alias own claim');
 }
 await page.goto(address+'#foreign-publication-same-name');await settle(page);assert.equal(await page.locator('.source-point[data-selected]').count(),0);assert.equal(await page.locator('.fragment-unavailable').isVisible(),true);notes.push('foreign fragment refusal');
 const native=await page.locator(`[id="${claimId}"] a.evidence-link`).first().getAttribute('href');
 await page.goto(address+native);await complete(page);assert.equal(await point().getAttribute('id'),sourceId,'native origin resolves its exact Source');
 await page.keyboard.press('Escape');await settle(page);assert.equal(await element.evaluate(n=>n===document.activeElement),true,'native deep entry returns to its exact element');
 await page.goto(address+native+'!foreign-claim');await settle(page);assert.equal(await page.locator('.source-point[data-selected]').count(),0);assert.equal(await page.locator('.fragment-unavailable').isVisible(),true);notes.push('native origin and foreign claim refusal');
 // A restored page revalidates selected evidence. A destroyed enhancement
 // aborts pending reads and releases inert/observers/listeners, even when a
 // response was already obtained before teardown.
 await page.goto(address+native+'!'+claimId);await complete(page);
 const expectedNativeCode=await point().locator('.citation-inspection code').textContent();
 let resumedRequests=0;const count=()=>resumedRequests++;
 page.on('request',count);
 await page.evaluate(()=>{
  window.dispatchEvent(new PageTransitionEvent('pagehide',{persisted:true}));
  window.dispatchEvent(new PageTransitionEvent('pageshow',{persisted:true}));
 });
 await complete(page);page.off('request',count);
 assert.ok(resumedRequests>0,'restored selected evidence must request current permission');
 notes.push('restoration revalidates authority');
 let release,arrived;
 const heldResponse=new Promise(r=>arrived=r),released=new Promise(r=>release=r);
 await context.route('**/citation?*',async route=>{
  const response=await route.fetch();arrived();await released;
  await route.fulfill({response}).catch(()=>{});
 });
 await point().locator('.citation-read').click();await heldResponse;
 await page.evaluate(()=>window.dispatchEvent(new PageTransitionEvent('pagehide',{persisted:false})));
 release();await pause(300);
 assert.equal(await page.locator('[data-fetched-code]').count(),0,'late bytes cannot paint after teardown');
 assert.equal(await page.locator('[aria-modal=true]').count(),0,'teardown releases modal ownership');
 assert.equal(await page.locator('.project-shell').evaluate(n=>n.inert),false,'teardown restores background');
 assert.equal(await page.locator('body').getAttribute('data-reader-enhanced'),null,'teardown restores native fragment consumption');
 const nativePoint=page.locator(`[id="${sourceId}"]`);
 const {openEmbeddedCode}=require('../../viewer-design/product_review_controls.cjs');
 assert.equal(await (await openEmbeddedCode(nativePoint)).textContent(),expectedNativeCode,'native evidence survives enhancement teardown');
 await context.unroute('**/citation?*');await page.reload();await complete(page);
 notes.push('teardown rejects a completed late response and releases modal state');
 // Denial includes accessible names on the retained opaque panel, not only
 // removed descendants. A narrow panel must not retain its old Source path.
 await page.setViewportSize({width:390,height:900});await settle(page);
 const publication=await point().getAttribute('data-reader-publication');
 const oldPath=await point().locator('.source-path').textContent();
 assert.ok((await point().getAttribute('aria-label')).includes(oldPath));
 await context.route('**/citation?*',route=>route.fulfill({status:410,contentType:'application/json',body:JSON.stringify({reason:'deleted',publication_readable:false})}));
 await point().locator('.citation-read').click();
 await point().locator('.publication-unavailable').waitFor();
 const names=await page.locator(`[data-reader-publication="${publication}"]`).evaluateAll(nodes=>nodes.map(n=>({label:n.getAttribute('aria-label')||'',title:n.getAttribute('title')||''})));
 assert.ok(names.every(n=>!n.label.includes(oldPath)&&!n.title.includes(oldPath)),'denied Source path cannot survive in accessible attributes');
 await context.unroute('**/citation?*');await page.setViewportSize({width,height:900});
 await settle(page);assert.equal(await page.locator(`[data-reader-publication="${publication}"] svg`).count(),0,'resize cannot rebuild denied meaning');
 notes.push('typed denial clears retained accessible Source labels');
 await page.goto(address);await settle(page);
 return {width,sourceId,checks:notes};
}
module.exports={inspect};
