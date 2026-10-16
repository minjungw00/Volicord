// Bounded actual navigation measurements. No automated comprehension rating.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');

module.exports=async function journeys({browser,url,build,out,refined=false}) {
 const input=JSON.parse(fs.readFileSync(path.join(build,'inputs/journeys.json')));
 const authored=JSON.parse(fs.readFileSync(path.join(build,'inputs/review-input-final.json')));
 const rows=[];
 async function geometry(locator){
  return locator.evaluate(n=>{const r=n.getBoundingClientRect(),s=getComputedStyle(n);return {top:r.top+scrollY,viewport_top:r.top,width:r.width,height:r.height,font:s.fontSize,line_height:s.lineHeight,visible:r.top<innerHeight&&r.bottom>0&&r.width>0};});
 }
 async function activate(page,link,steps,label){
  const before=await page.evaluate(()=>scrollY);
  await link.focus(); const at=await page.evaluate(()=>scrollY);
  await page.keyboard.press('Enter');
  steps.push({action:label,input:'focus + Enter',scroll_to_control:Math.abs(at-before),url:page.url()});
 }
 async function paragraphs(page,work,layout){
  const article=layout==='baseline'?page.locator('#work-'+work.work):page.locator('.doc');
  if(work.key==='missing')return {article};
  const units=authored.works.find(w=>w.key===work.key).units;
  const at=role=>units.findIndex(u=>u.role===role);
  const p=role=>layout==='baseline'?article.locator(`[data-question="${({purpose:'Purpose',reading_point:'ReadingPoint',limits:'Limits',comparison:'Comparison'})[role]}"]`).first():article.locator(`[data-unit="${at(role)}"]`);
  return {article,problem:p('purpose'),mechanism:p('reading_point'),limits:p('limits'),comparison:work.key==='click'?p('comparison'):null,index:at('reading_point')};
 }
 for(const width of [1440,1024,768,390,320])for(const work of input.cases)for(const layout of ['baseline','target']){
  const context=await browser.newContext({viewport:{width,height:900}});const page=await context.newPage();
  const steps=[];let row={width,work:work.key,identity:work.work,layout,steps,human_time:null,human_errors:null,human_understanding:'unobserved'};
  try{
   await page.goto(url+work[layout]);await page.evaluate(()=>document.fonts.ready);
   if(work.key!=='missing'){
    const link=layout==='baseline'?page.locator(`#overview [data-work-id="${work.work}"] h4 a`).first():page.locator(`[data-work-id="${work.work}"] a`);
    row.find={candidates:await page.locator(layout==='baseline'?'#overview [data-work-id]':'[data-work-id]').count(),control:await geometry(link)};
    let tabs=0;
    while(!(await link.evaluate(n=>n===document.activeElement))&&tabs<200){await page.keyboard.press('Tab');tabs++;}
    assert(tabs<200,'discovery reachable by native Tab');row.find.native_tabs=tabs;
    const t=performance.now();await activate(page,link,steps,'find Work');
    if(layout==='target')await page.waitForURL('**/'+work.key+'.html');
    row.machine_entry_ms=performance.now()-t;
   }
   const p=await paragraphs(page,work,layout);
   if(work.key==='missing'){
    const text=await p.article.textContent();
    const status=JSON.parse(fs.readFileSync(path.join(build,'inputs/missing-status.json'))).work_history[0];
    for(const fact of status.answers.facts)if(fact.question!=='RecordedGoal')assert(text.includes(fact.text),'missing case retains '+fact.question);
    assert(text.includes(input.missing_case.goal),'same complete Goal');
    assert.equal(await p.article.locator('.source-citation,.evidence-link,.code-region').count(),0,'no invented code explanation');
    row.outcome={explanation:'missing',relevant_code:'unavailable',reason:'no Work-bound Source',recovery:'requires evidence/host explanation; layout cannot repair content'};
   }else{
    row.opening={problem:await geometry(p.problem),mechanism:await geometry(p.mechanism),limits:await geometry(p.limits)};
    if(layout==='target'&&refined){
     const jump=page.locator('.audit-links a[href="#limits"]');
     row.limit_shortcut=await geometry(jump);assert(row.limit_shortcut.visible,'limits shortcut on opening screen');
    }
    row.typography=await p.mechanism.evaluate(n=>({font:getComputedStyle(n).fontSize,line_height:getComputedStyle(n).lineHeight,width:n.getBoundingClientRect().width,text_length:n.textContent.length}));
    const start=await page.evaluate(()=>scrollY);
    await p.mechanism.scrollIntoViewIfNeeded();
    row.read_scroll=Math.abs((await page.evaluate(()=>scrollY))-start);
    const origin=layout==='baseline'?await p.mechanism.getAttribute('id'):'claim-'+p.index;
    const cite=layout==='baseline'?p.mechanism.locator('xpath=..').locator('a.source-citation').first():page.locator(`[data-origin="${origin}"].evidence-link`).first();
    const href=await cite.getAttribute('href');
    const t=performance.now();await activate(page,cite,steps,'select mechanism Source');
    const point=page.locator(href);await point.waitFor({state:'visible'});
    const area=layout==='baseline'?point.locator('xpath=ancestor::figure[1]'):point;
    const excerpt=area.locator(layout==='baseline'?'pre[role=region]':'.code-region');
    await excerpt.waitFor({state:'visible'});
    row.machine_code_ms=performance.now()-t;
    const ref=work.mechanism_ref;
    const source=input.source_first.find(e=>e.work===work.work&&e.ref===ref);
    assert(source,'independent expected Source');
    assert((await area.textContent()).includes(source.citation.locator),'selected exact locator');
    const original=fs.readFileSync(path.join(build,`inputs/${work.key}-source-${ref}.txt`));
    const text=original.subarray(source.citation.start_byte,source.citation.end_byte).toString();
    const actual=layout==='baseline'?await excerpt.textContent():(await excerpt.locator('.code-text').allTextContents()).join('\n')+'\n';
    assert.equal(actual,text,'purposeful exact Source range');
    row.code={...await geometry(excerpt),selected:source.citation,range_exact:true,horizontal_scroll:await excerpt.evaluate(n=>n.scrollWidth>n.clientWidth)};
    await excerpt.focus();await page.keyboard.press('ArrowRight');
    row.return_origin=origin;
    const back=layout==='baseline'?point.locator(`a.return-to-claim[href="#${origin}"]`).first():point.locator('.return');
    await page.screenshot({path:path.join(out,`journey-${work.key}-${layout}-${width}-code.png`)});
    await activate(page,back,steps,'return to original paragraph');
    await page.waitForFunction(id=>location.hash==='#'+id,origin);
    if(layout==='target')await page.waitForFunction(id=>document.activeElement.id===id&&!document.body.classList.contains('code-open'),origin);
    row.return={correct_hash:true,focused:await page.evaluate(id=>document.activeElement.id===id,origin),paragraph:await geometry(p.mechanism)};
    // Record rather than silently pass baseline's focus/position limitations.
    if(layout==='target')assert(row.return.focused,'target return focus');
    if(work.key==='click'){
     const units=authored.works.find(w=>w.key==='click').units.filter(u=>u.role==='comparison');
     const content=await p.article.textContent();for(const u of units)assert(content.includes(u.text),'all three conditional observations');
     row.comparison={conditions:units.length,geometry:await geometry(p.comparison),human_correctness:null};
     if(layout==='target'&&refined){
      assert.deepEqual(await page.locator('.condition-table tbody td code').allTextContents(),['B','A','A'],'independent stdout outcomes');
      const table=page.locator('.condition-table');await table.scrollIntoViewIfNeeded();
      row.comparison.table=await geometry(table);
      await page.screenshot({path:path.join(out,`journey-click-conditions-${width}.png`)});
      const links=table.locator('tbody a');await activate(page,links.nth(1),steps,'compare condition and inspect explanation');
      assert.equal(new URL(page.url()).hash,'#claim-7');
      await page.screenshot({path:path.join(out,`journey-click-condition-explanation-${width}.png`)});
     }
    }
    if(layout==='target'&&refined){
     await activate(page,page.locator('.audit-links a[href="#limits"]'),steps,'inspect verification limits');
     await page.waitForFunction(()=>location.hash==='#limits');
     assert((await geometry(p.limits)).visible,'limits reached directly');
    }else await p.limits.scrollIntoViewIfNeeded();
    row.limits_reachable=await p.limits.isVisible();
    // Reproduction of defects is explicit, with positive and adverse controls.
    if(layout==='target'){
     await page.goto(url+work.key+'.html#original');await page.evaluate(()=>document.fonts.ready);
     row.fragment_disclosure_open=await page.locator('#original').evaluate(n=>n.open);
     await page.goto(url+work.key+'.html');
     await page.locator(`[data-origin="claim-${p.index}"].evidence-link`).first().click();
     const active=page.locator('.source-point.active');await active.waitFor({state:'visible'});
     await active.locator('.return').focus();await page.keyboard.press('Shift+Tab');
     row.code_focus_contained=await page.evaluate(()=>!!document.activeElement.closest('.appendix'));
     if(refined){assert(row.fragment_disclosure_open,'direct disclosure must open');assert(row.code_focus_contained,'modal keyboard focus contained');}
     if(refined){
      assert.equal(await page.locator('.doc').evaluate(n=>n.inert),true,'background excluded');
      await page.goto(url+work.key+'.html#point-'+p.index+'-0');
      await page.locator('.source-point.active').waitFor({state:'visible'});
      assert.equal(await page.locator('.appendix').getAttribute('role'),'dialog','direct Source selection');
     }
     await page.keyboard.press('Escape');
     if(refined){assert.equal(await page.locator('#original').evaluate(n=>n.open),false,'unrelated disclosure remains closed');}
    }
   }
   row.page_overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1);
   assert(!row.page_overflow,'ordinary page overflow');
   await page.screenshot({path:path.join(out,`journey-${work.key}-${layout}-${width}.png`)});
   row.status='passed';
  }finally{rows.push(row);fs.writeFileSync(path.join(out,'journeys.json'),JSON.stringify({status:'incomplete',rows},null,2));await context.close();}
 }
 // Content negatives use disposable rendered copies. Identity/range admission has
 // separate original-source controls; these challenge the human-readable consumer.
 const page=await browser.newPage();
 await page.goto(url+'b.html');
 const clause=authored.works.find(w=>w.key==='b').units.find(u=>u.role==='limits').text;
 async function limitsCheck(){assert.equal(await page.locator('.guard .unit-text').textContent(),clause,'adverse outcome retained');}
 await limitsCheck();await page.locator('.guard .unit-text').evaluate(n=>n.textContent='All checks passed.');
 await assert.rejects(limitsCheck,/adverse outcome retained/);
 await page.goto(url+'b.html');await limitsCheck();
 await page.locator('[data-origin="claim-5"].evidence-link').first().evaluate(n=>n.href='#point-7-0');
 await page.locator('[data-origin="claim-5"].evidence-link').first().click();
 assert(!(await page.locator('.source-point.active .source-path').textContent()).includes('archive/cli.rs:1423'),'wrong-purpose selection detected');
 await page.goto(url+'b.html');await page.locator('[data-origin="claim-5"].evidence-link').first().click();
 assert((await page.locator('.source-point.active .source-path').textContent()).includes('archive/cli.rs:1423'),'restored purpose selection passes');
 if(refined){
  await page.evaluate(()=>document.addEventListener('keydown',e=>{if(e.key==='Tab')e.stopImmediatePropagation()},true));
  await page.locator('.doc').evaluate(n=>n.inert=false);await page.locator('.masthead').evaluate(n=>n.inert=false);
  await page.locator('.source-point.active .return').focus();await page.keyboard.press('Shift+Tab');
  assert.equal(await page.evaluate(()=>!!document.activeElement.closest('.appendix')),false,'removing focus containment reproduces failure');
  await page.goto(url+'b.html#point-5-0');await page.reload();await page.locator('.source-point.active .return').focus();await page.keyboard.press('Shift+Tab');
  assert(await page.evaluate(()=>!!document.activeElement.closest('.appendix')),'restored focus containment passes');
 }
 await page.close();
 const receipt={kind:'browser measurements and scripted tasks; no human or independent model comprehension verdict',refined,rows,negative_controls:['deleted adverse outcome rejected; restored positive passes','wrong-purpose Source differs from independent expected range; restored positive passes',...(refined?['removed focus containment reproduces failure; restored positive passes']:[])],human_acceptance:'pending'};
 fs.writeFileSync(path.join(out,'journeys.json'),JSON.stringify(receipt,null,2));
 return {journeys:rows.length,refined,defects:{closed_fragment:rows.filter(r=>r.fragment_disclosure_open===false).length,focus_escape:rows.filter(r=>r.code_focus_contained===false).length},human_acceptance:'pending'};
};
