// Direct scoped controls against original public semantics and original Source bytes.
// This is implementer-dependent inspection, never an isolated reader or user selection.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {pathToFileURL}=require('node:url');
const {citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('./product_review_controls.cjs');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
async function check(config){
 const out=path.resolve(config.check_output);assert.ok(out.includes('/rebuild/.local/'));fs.mkdirSync(out);
 const root=path.resolve(config.output),manifest=JSON.parse(fs.readFileSync(path.join(root,'manifest.json')));
 const result={status:'running',checks:[],failures:[],external:[],browser_errors:[],classification:'dependent implementer technical inspection; first human reading unobserved'};
 const {chromium}=require(config.playwright);const browser=await chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 const settle=page=>page.waitForTimeout(50);
 const write=()=>fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));
 try{
  for(const c of config.cases){
   const publicData=JSON.parse(fs.readFileSync(c.public));const d=publicData.diagrams.find(x=>x.id===c.diagram),m=d.meaning;
   const nodes=m.nodes||m.participants,edges=m.relations||m.groups.flatMap(g=>g.messages);
   const roles=[...nodes.map(n=>({id:n.id,label:n.label})),...nodes.filter(n=>n.identifier).map(n=>({id:n.id+':identifier',label:n.identifier})),...edges.map(e=>({id:e.id,label:e.label})),...edges.filter(e=>e.condition).map(e=>({id:e.id+':condition',label:e.condition})),...(m.groups||[]).flatMap((g,i)=>g.condition?[{id:'guard:'+(i+1),label:g.condition}]:[]),...(m.owner?[{id:'state:owner',label:m.owner}]:[])];
   let baselineText=null,baselineCode=null;
   for(const v of c.variants){
    const candidate=manifest.candidates.find(x=>x.id===c.key+'-'+v.id);assert.ok(candidate);assert.equal(hash(fs.readFileSync(path.join(root,candidate.file))),candidate.sha256);
    const context=await browser.newContext({viewport:{width:1440,height:1000}}),page=await context.newPage();page.setDefaultTimeout(6000);
    page.on('pageerror',e=>result.browser_errors.push(String(e)));
    await context.route('**/*',route=>{if(route.request().url().startsWith('file:'))return route.continue();result.external.push(route.request().url());return route.abort();});
    const url=pathToFileURL(path.join(root,candidate.file)).href;
    try{
     const widths=config.widths||[1440,1024,768,390,320];
     for(const width of widths){
      const prefix=candidate.id+'-'+width;
      await page.setViewportSize({width,height:1000});await page.goto('about:blank');await page.goto(url+'#'+candidate.entry);await page.evaluate(()=>document.fonts.ready);await settle(page);
      await page.screenshot({path:path.join(out,prefix+'-entry.png')});
      if(c.work){
       const row=page.locator(`#works .work-card[data-work-id="${c.work}"]`);
       assert.equal(await row.count(),1);const link=row.locator('a[href="#work-'+c.work+'"]').first();await link.focus();await page.keyboard.press('Enter');await settle(page);
       assert.equal(new URL(page.url()).hash,'#work-'+c.work);
       await page.screenshot({path:path.join(out,prefix+'-work.png')});
       assert.match(await page.locator(`#work-${c.work} .record-strip`).innerText(),/일시 중지/);
      }
      const figure=page.locator(`.admitted-diagram[data-semantic-fingerprint="${d.semantic_fingerprint}"]`);
      const figureId=await figure.getAttribute('id');
      await figure.evaluate(n=>n.scrollIntoView({block:'start'}));await settle(page);
      await page.screenshot({path:path.join(out,prefix+'-initial.png')});
      const texts=await page.locator('.unit-text').allTextContents(),code=await page.locator('.code-region code').allTextContents();
      if(v.id==='A'){baselineText=texts;baselineCode=code;}
      if(v.editorial==='current'){assert.deepEqual(texts,baselineText,'same-input explanatory units');assert.deepEqual(code,baselineCode,'same-input complete code');}
      const viewport=figure.locator('.spatial-viewport');
      if(v.navigation!=='baseline'){
       const svg=viewport.locator('svg');
       assert.deepEqual((await svg.locator('a.diagram-element').evaluateAll(ns=>ns.map(n=>n.dataset.element))).sort(),roles.map(r=>r.id).sort(),'all original selectable roles');
       for(const r of roles){const a=svg.locator(`[data-element="${r.id}"]`);assert.ok((await a.textContent()).includes(r.label.text),'complete role wording '+r.id);}
       const geometry=await svg.evaluate((svg,m)=>{
        const errors=[];for(const text of svg.querySelectorAll('text')){const b=text.getBBox();if(b.x<0||b.y<0||b.x+b.width>Number(svg.dataset.worldWidth)+1||b.y+b.height>Number(svg.dataset.worldHeight)+1)errors.push({kind:'glyph outside world',text:text.textContent,b:{x:b.x,y:b.y,w:b.width,h:b.height}});}
        const roles=[...svg.querySelectorAll('a[data-element]')],role=id=>roles.find(a=>a.dataset.element===id);
        const nodes=m.nodes||m.participants,edges=m.relations||m.groups.flatMap(g=>g.messages);
        for(const e of edges){
          const p=role(e.id).querySelector('path.world-edge'),s=p.getPointAtLength(0),t=p.getPointAtLength(p.getTotalLength());
          const rect=id=>role(id).querySelector('rect').getBBox();
          const on=(point,b)=>point.x>=b.x-1&&point.x<=b.x+b.width+1&&point.y>=b.y-1&&point.y<=b.y+b.height+1&&Math.min(Math.abs(point.x-b.x),Math.abs(point.x-b.x-b.width),Math.abs(point.y-b.y),Math.abs(point.y-b.y-b.height))<2;
          if(m.kind==='sequence'){
            const from=rect(e.from),to=rect(e.to);if(Math.abs(s.x-from.x-from.width/2)>1||Math.abs(t.x-to.x-to.width/2)>1)errors.push({kind:'wrong endpoint',element:e.id});
          }else if(!on(s,rect(e.from))||!on(t,rect(e.to)))errors.push({kind:'wrong endpoint',element:e.id});
          if(!p.hasAttribute('marker-end'))errors.push({kind:'missing direction',element:e.id});
        }
        return {errors,world:[svg.dataset.worldWidth,svg.dataset.worldHeight]};
       },m);assert.deepEqual(geometry.errors,[],'complete readable-size world bounds and actual endpoints');
       if(v.editorial==='edited'&&v.navigation==='camera')await figure.locator('[data-world-action="reset"]').click();
       await viewport.focus();const left=await viewport.evaluate(n=>n.scrollLeft);await page.keyboard.press('ArrowRight');await page.waitForTimeout(150);assert.ok(await viewport.evaluate(n=>n.scrollLeft)>left,'native keyboard pan');
       if(v.navigation==='camera'){
        await figure.locator('[data-world-action="fit"]').click();assert.match(await figure.locator('output').innerText(),v.editorial==='edited'?/%/:/위치 확인용/);
        await figure.locator('[data-world-action="reset"]').click();assert.equal(await figure.locator('output').innerText(),v.editorial==='edited'?'75%':'100%');
        if(!await figure.locator('[data-world-action="right"]').isVisible())await figure.locator('.spatial-navigation details > summary').click();
        await figure.locator('[data-world-action="right"]').click();assert.ok(await viewport.evaluate(n=>n.scrollLeft)>0,'non-drag pointer pan');
        if(v.editorial==='edited')await figure.locator('.spatial-navigation details > summary').click();
        await figure.locator('[data-world-action="in"]').click();assert.equal(await figure.locator('output').innerText(),v.editorial==='edited'?'100%':'125%');
       }
      }
      // Optional explicit tasks exercise source-frozen questions at every width.
      // Complete role inventory/wording/endpoint checks above are never filtered.
      if(c.elements)assert.ok(c.elements.length&&c.elements.every(id=>roles.some(r=>r.id===id)),'explicit tasks must name existing roles');
      const selected=c.elements?roles.filter(r=>c.elements.includes(r.id)):width===1440?roles:roles.filter(r=>[edges[0].id,edges.at(-1).id,roles.at(-1).id].includes(r.id));
      let trips=0;
      for(const r of selected){
       await page.goto(url+'#'+(c.work?'work-'+c.work:'repository-explanation'));await settle(page);
       const element=figure.locator(`a.diagram-element[id="${figureId}-${r.id}"]`);
       await element.focus();await element.evaluate(n=>n.scrollIntoView({block:'center',inline:'center'}));await settle(page);
       const origin=await element.getAttribute('id'),href=await element.getAttribute('href');
       const before=await viewport.count()?await viewport.evaluate(n=>({left:n.scrollLeft,top:n.scrollTop})):null;
       await page.keyboard.press('Enter');await settle(page);
       const claim=page.locator('[id="'+href.slice(1)+'"]');
       const links=claim.locator('a.evidence-link');assert.ok(await links.count()>0,'exact claim has original covering ranges');
       const unit=publicData.prose[r.label.unit],covering=unit.citations.filter(q=>q.claim_start_byte<=r.label.start_byte&&q.claim_end_byte>=r.label.end_byte);
       assert.equal(await links.count(),covering.length,'complete covering range set');
       for(let i=0;i<covering.length;i++){
        const link=links.nth(i);
        if(!await link.isVisible()){
         const details=link.locator('xpath=ancestor::details[1]');
         await details.locator(':scope > summary').click();
        }
        await link.focus();await page.keyboard.press('Enter');await settle(page);
        const point=await citationPoint(page,await link.getAttribute('href'));
        const expected=fs.readFileSync(path.join(c.repository,covering[i].locator)).subarray(covering[i].start_byte,covering[i].end_byte).toString('utf8');
        assert.equal(await point.getAttribute('data-source-id'),covering[i].source_id);
        assert.equal(await (await openEmbeddedCode(point)).textContent(),expected,'independent complete original code');
        if(trips===0&&i===0)await page.screenshot({path:path.join(out,prefix+'-code.png')});
        await returnFromEmbeddedCode(point);
        const back=point.locator(':scope > .source-origin-target:target a.return').first();
        const fallback=point.locator('a.return:visible').first();const ret=await back.count()?back:fallback;
        assert.equal(await ret.getAttribute('href'),'#'+origin,'exact publication/view/element return');
        await ret.focus();await page.keyboard.press('Enter');await settle(page);
        assert.equal(decodeURIComponent(new URL(page.url()).hash),'#'+origin);
        if(v.navigation!=='baseline'){
         assert.equal(await element.evaluate(n=>n===document.activeElement),true,'focus restored');
         const after=await viewport.evaluate(n=>({left:n.scrollLeft,top:n.scrollTop}));assert.ok(Math.abs(after.left-before.left)<2&&Math.abs(after.top-before.top)<2,'camera position restored: '+JSON.stringify({origin,before,after}));
        }
        // Reopen the same claim for another covering Source, retaining the actual origin.
        if(i+1<covering.length){await element.focus();await page.keyboard.press('Enter');await settle(page);}
       }
       trips++;
      }
      await page.screenshot({path:path.join(out,prefix+'-return.png')});
      result.checks.push({candidate:candidate.id,width,mode:'file-scripted',exact_role_trips:trips});write();
     }
    }catch(e){result.failures.push({candidate:candidate.id,error:e.stack});await page.screenshot({path:path.join(out,candidate.id+'-failure.png')}).catch(()=>{});write();}
    finally{await context.close();}
   }
   const b=manifest.candidates.find(x=>x.id===c.key+'-B'),camera=manifest.candidates.find(x=>x.id===c.key+'-C');
   if(b&&camera)assert.equal(b.geometry_sha256,camera.geometry_sha256,'B and C use identical SVG geometry');
  }
  assert.deepEqual(result.external,[],'no external requests');assert.deepEqual(result.browser_errors,[],'no browser script failures');
  result.status=result.failures.length?'failed':'passed';write();assert.equal(result.failures.length,0,'inspect preserved failures');
 }catch(e){result.status='failed';result.error=e.stack;write();throw e;}finally{await browser.close();}
 console.log(JSON.stringify({status:result.status,checks:result.checks.length,output:out}));
}
if(require.main===module)check(JSON.parse(fs.readFileSync(process.argv[2]))).catch(e=>{console.error(e);process.exitCode=1;});
module.exports={check};
