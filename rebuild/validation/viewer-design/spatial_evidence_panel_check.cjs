// Actual evidence-window journeys against frozen Source bytes; no Runtime access.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
(async()=>{
 const c=JSON.parse(fs.readFileSync(process.argv[2])),out=path.resolve(c.panel_output);assert.ok(out.includes('/rebuild/.local/'));fs.mkdirSync(out);
 const m=JSON.parse(fs.readFileSync(path.join(c.output,'manifest.json'))),rows=[];
 const browser=await require(c.playwright).chromium.launch({executablePath:c.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});let failure;
 try{for(const candidate of m.candidates.filter(n=>n.editorial==='edited'))for(const width of [1440,1200,1024,768,390,320]){
  const source=c.cases.find(n=>candidate.id.startsWith(n.key+'-')),pub=JSON.parse(fs.readFileSync(source.public));
  const p=await browser.newPage({viewport:{width,height:1000}});p.setDefaultTimeout(6000);const errors=[],external=[];
  p.on('pageerror',e=>errors.push(String(e)));await p.route('**/*',r=>{if(r.request().url().startsWith('file:'))return r.continue();external.push(r.request().url());return r.abort();});
  try{
   await p.goto(pathToFileURL(path.join(c.output,candidate.file)).href+'#work-'+source.work);await p.evaluate(()=>document.fonts.ready);
   const doc=p.locator('.doc[data-work-layout]'),limits=doc.locator('.limits-section'),rawLimits=doc.locator('[id$="-verification-originals"] [id$="-limit-originals"]');
   assert.equal(await limits.locator('.reading-layer-body > p,.reading-warning').count(),0,'record originals leave the explanation-limits reading layer');
   assert.ok(await rawLimits.locator('.limit-original-body > p').count()>0);assert.ok(await rawLimits.locator('.reading-warning').count()>0);
   await limits.locator('a[href$="-limit-originals-content"]').focus();await p.keyboard.press('Enter');assert.ok(await rawLimits.evaluate(n=>n.open),'limit-original link opens the actual raw destination');
   const action=doc.locator('.unit[id$="-p12"] .paragraph-evidence-action');await action.focus();await action.press('Enter');
   const pane=p.locator('.appendix'),handle=pane.locator('.evidence-resize-handle');
   assert.equal(await pane.locator('.evidence-original').count(),0,'identical original-sentence copies are removed');
   assert.equal(await pane.locator('details > summary').evaluateAll(ns=>ns.some(n=>['원래 문장','원래 주장'].includes(n.textContent.trim()))),false,'remaining context uses its distinct role');
   async function activeCode(citation){
    const id=decodeURIComponent(new URL(p.url()).hash.slice(1)),target=p.locator('[id="'+id+'"]'),codeId=await target.getAttribute('data-cited-code');
    assert.ok(codeId,'the selected exact range has its original code binding');const disclosure=p.locator('[id="'+codeId+'"]');
    assert.equal(await disclosure.evaluate(n=>n.open),true,'complete code is already expanded without another action');
    const code=disclosure.locator('pre.code-region code');assert.ok(await code.isVisible());
    assert.equal(await code.textContent(),fs.readFileSync(path.join(source.repository,citation.locator)).subarray(citation.start_byte,citation.end_byte).toString());
    assert.equal(await pane.locator('.evidence-range-select:visible').count(),1);assert.equal(p.context().pages().length,1);return disclosure;
   }
   let code=await activeCode(pub.prose[11].citations[0]);const initialWidth=(await pane.boundingBox()).width;
   const reference=doc.locator('[id$="-editorial-basis"] > summary'),metadata=pane.locator('.source-metadata:visible > summary').first();
   const style=n=>{const s=getComputedStyle(n);return [s.fontSize,s.fontWeight,s.lineHeight,s.minHeight,s.padding,s.color];};
   assert.deepEqual(await metadata.evaluate(style),await reference.evaluate(style),'metadata disclosure shares Work category typography and spacing');
   await code.locator(':scope > summary').focus();await p.keyboard.press('Enter');assert.equal(await code.evaluate(n=>n.open),false,'readers can still fold the code');
   if(width>760){
    assert.ok(await handle.isVisible());const hash=new URL(p.url()).hash,box=await handle.boundingBox(),shell=p.locator('main.shell.viewer-shell'),before=await shell.boundingBox();
    await p.mouse.move(box.x+box.width/2,box.y+box.height/2);await p.mouse.down();await p.mouse.move(box.x+box.width/2-100,box.y+box.height/2,{steps:8});await p.mouse.up();
    assert.ok(Math.abs((await pane.boundingBox()).width-initialWidth-100)<2,'dragging the left boundary increases width by the pointer distance');
    if(width>=1200){const after=await shell.boundingBox();assert.ok(Math.abs(before.width-after.width-100)<2,'desktop article makes room for the resized evidence window');}
    assert.equal(new URL(p.url()).hash,hash);assert.equal(await code.evaluate(n=>n.open),false,'resizing preserves a deliberate fold choice');
    await handle.focus();await p.keyboard.press('ArrowRight');assert.ok(Math.abs((await pane.boundingBox()).width-initialWidth-68)<2,'keyboard controls resize the same window');
    await p.keyboard.press('Home');assert.ok(Math.abs((await pane.boundingBox()).width-Number(await handle.getAttribute('aria-valuemin')))<2);
    await p.keyboard.press('End');assert.ok(Math.abs((await pane.boundingBox()).width-Number(await handle.getAttribute('aria-valuemax')))<2);
    assert.equal(await handle.getAttribute('aria-valuenow'),String(Math.round((await pane.boundingBox()).width)));
    await p.screenshot({path:path.join(out,`${candidate.id}-${width}-resized.png`)});
    await p.keyboard.press('Enter');assert.ok(Math.abs((await pane.boundingBox()).width-initialWidth)<2,'Enter restores default width');
    await p.keyboard.press('ArrowLeft');await pane.locator('.panel-width-reset:visible').click();assert.ok(Math.abs((await pane.boundingBox()).width-initialWidth)<2,'header reset restores the same default');
   }else{assert.equal(await handle.isVisible(),false);assert.equal(Math.round(initialWidth),width,'small screens retain a full-screen reader');}
   const picker=pane.locator('.evidence-range-select:visible'),options=await picker.locator('option').count();
   if(options>1){await picker.selectOption(await picker.locator('option').last().getAttribute('value'));code=await activeCode(pub.prose[11].citations.at(-1));await p.waitForFunction(()=>document.activeElement?.matches('.evidence-range-select'));}
   else{await code.locator(':scope > summary').focus();await p.keyboard.press('Enter');}
   assert.equal(await p.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'resizing/code scroll does not widen the document');
   await p.screenshot({path:path.join(out,`${candidate.id}-${width}-code.png`)});
   await p.keyboard.press('Escape');assert.equal(await pane.isVisible(),false);assert.equal(await doc.evaluate(n=>!!n.closest('[inert]')),false);
   const role=doc.locator('.diagram-stage svg a[id$="-still-wait"]');await role.focus();await p.keyboard.press('Enter');
   const claim=p.locator('[id="'+(await role.getAttribute('href')).slice(1)+'"]');await claim.locator('.paragraph-evidence-action').focus();await p.keyboard.press('Enter');
   const alias=p.locator(':target'),root=await alias.getAttribute('data-selected-point'),context=p.locator('[id="'+root+'"] > .source-shared-context');
   assert.ok(await context.isVisible(),'a distinct diagram name retains its explicitly bound original body context');
   await context.locator(':scope > summary').focus();await p.keyboard.press('Enter');
   const diagram=pub.diagrams.find(n=>n.id===source.diagram),edge=diagram.meaning.groups.flatMap(n=>n.messages).find(n=>n.id==='still-wait');
   assert.equal((await context.locator(':scope > :not(summary)').allTextContents()).join(''),pub.prose[edge.label.unit].text,'connected body is the frozen bound paragraph, not a file-level guess');
   await p.screenshot({path:path.join(out,`${candidate.id}-${width}-connected-context.png`)});
   await p.keyboard.press('Escape');assert.equal(await pane.isVisible(),false);
   assert.deepEqual(errors,[]);assert.deepEqual(external,[]);rows.push({candidate:candidate.id,width,default_original_code:true,fold_choice:true,range_change:true,drag_keyboard_reset:width>760,raw_limits:true,metadata_style:true});
  }catch(e){await p.screenshot({path:path.join(out,`${candidate.id}-${width}-failure.png`)});throw e;}finally{await p.close();}
 }}catch(e){failure=e.stack;throw e;}finally{fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:failure?'failed':'passed',failure,rows,unobserved:['physical touch','assistive technology','other engines','independent human reading','live Runtime lifecycle']},null,2));await browser.close();}
 console.log(JSON.stringify({status:'passed',conditions:rows.length}));
})().catch(e=>{console.error(e);process.exitCode=1;});
