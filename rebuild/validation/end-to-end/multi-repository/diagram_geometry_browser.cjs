// Renderer stress controls, distinct from actual admitted publication/code tasks.
const fs=require('node:fs');const path=require('node:path');const assert=require('node:assert/strict');const crypto=require('node:crypto');
const {execFileSync}=require('node:child_process');
const {inspectDiagram}=require('./diagram_geometry.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
(async()=>{
  fs.mkdirSync(config.output,{recursive:false});
  const env={...process.env,FONTCONFIG_FILE:config.fontconfig};
  const fonts=[...new Set(execFileSync('fc-list',['--format','%{file}\n',':lang=ko'],{env,encoding:'utf8'}).trim().split('\n').filter(Boolean))];assert(fonts.length,'Korean fonts required');
  const browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env});
  const rows=[];
  try{
    for(const f of JSON.parse(fs.readFileSync(path.join(config.fixtures,'fixtures.json')))) for(const width of config.widths||[1440,320]) {
      const context=await browser.newContext({viewport:{width,height:1000}});const page=await context.newPage();const external=[];
      await page.route('**/*',r=>{external.push(r.request().url());return r.abort();});
      const svg=fs.readFileSync(path.join(config.fixtures,f.file),'utf8');
      await page.setContent('<html lang="'+f.ui_locale+'"><style>'+fs.readFileSync(config.css,'utf8')+'</style><figure id="'+f.id+'"><div class="diagram-full-view"><div class="diagram-viewport">'+svg+'</div></div></figure></html>');
      await page.evaluate(()=>document.fonts.ready);
      const ns=await page.locator('svg').getAttribute('data-origin-namespace')||f.id;
      const result=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);rows.push({fixture:f.id,width,...result});
      fs.writeFileSync(path.join(config.output,f.id+'-'+width+'.json'),JSON.stringify(result,null,2),{flag:'wx'});
      await page.locator('figure').screenshot({path:path.join(config.output,f.id+'-'+width+'.png')});
      assert.deepEqual(external,[]);
      const legends=page.locator('svg text:visible').filter({hasText:/^(All nodes:|All relations:|모든 노드:|모든 관계:)/});
      if(await legends.count()) {
        const legend=await legends.first().elementHandle();
        await legend.evaluate(n=>n.style.fontSize='16px');
        const promotedType=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(promotedType.errors.some(e=>e.kind==='type-hierarchy'),'a type key at node-label size must fail');
        await legend.evaluate(n=>n.style.removeProperty('font-size'));
        fs.writeFileSync(path.join(config.output,f.id+'-'+width+'-type-hierarchy-control.json'),JSON.stringify(promotedType.errors,null,2),{flag:'wx'});
        await legend.evaluate(n=>n.style.visibility='hidden');
        const lostType=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(lostType.errors.some(e=>e.kind==='missing-visible-type'),'a hidden common key must not erase relation types');
        await legend.evaluate(n=>n.style.removeProperty('visibility'));
        fs.writeFileSync(path.join(config.output,f.id+'-'+width+'-type-control.json'),JSON.stringify(lostType.errors,null,2),{flag:'wx'});
      }
      if(f.diagram.meaning.kind==='sequence') {
        const m=f.diagram.meaning;
        const first=page.locator('svg a[id="'+ns+'-'+m.groups[0].messages[0].id+'"]');
        await first.evaluate(n=>{const t=document.createElementNS('http://www.w3.org/2000/svg','text');t.id='extra-identifier-band';t.textContent='Unnecessary full participant identifier';n.append(t);});
        const repeated=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(repeated.errors.some(e=>e.kind==='repeated-message-identity'),'extra supporting identifier bands cannot replace action hierarchy');
        await page.locator('#extra-identifier-band').evaluate(n=>n.remove());
        // Local context must qualify this message, even when an earlier guard
        // is no longer on screen. Injecting an earlier group's meaning fails.
        const last=m.groups.at(-1).messages.at(-1);
        const local=page.locator('svg a[id="'+ns+'-'+last.id+'"] text:visible').last();
        const saved=await local.innerHTML();
        await local.evaluate(n=>n.textContent='par · no relative order · opt · condition: previous guard');
        const wrongContext=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(wrongContext.errors.some(e=>['missing-visible-meaning','missing-visible-type'].includes(e.kind)&&e.element===last.id),'a previous guard/order must fail local scope');
        await local.evaluate((n,v)=>n.innerHTML=v,saved);
        fs.writeFileSync(path.join(config.output,f.id+'-'+width+'-context-control.json'),JSON.stringify(wrongContext.errors,null,2),{flag:'wx'});
        const header=await page.locator('svg text:visible').filter({hasText:await page.locator('svg').evaluate(n=>n.classList.contains('spatial-world'))?(f.ui_locale==='ko'?'1. 병렬 · 상대 순서 없음':'1. Parallel · no relative order'):(f.ui_locale==='ko'?'1. par · 상대 순서 없음':'1. par · no relative order')}).elementHandle();
        assert(header,'original group header');
        const original=await header.innerHTML();
        await header.evaluate(n=>n.querySelector('tspan').textContent='1. strict · sequential');
        const wrongGroup=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(wrongGroup.errors.some(e=>e.kind==='wrong-group-order'),'wrong par/strict meaning must fail');
        await header.evaluate((n,original)=>n.innerHTML=original,original);
        const sequential=f.diagram.meaning.groups.find(g=>g.order==='sequential');
        if(sequential){
        const messageId=sequential.messages.at(-1).id;
        const previousId=sequential.messages.length>1?sequential.messages[0].id:f.diagram.meaning.groups[0].messages[0].id;
        const message=page.locator('svg a[id="'+ns+'-'+messageId+'"]');
        await message.evaluate((n,id)=>{const prior=[...n.parentElement.querySelectorAll('a')].find(a=>a.id===n.ownerSVGElement.dataset.originNamespace+'-'+id);const shift=prior.getBBox().y-n.getBBox().y-100;n.setAttribute('transform',`translate(0,${shift})`);},previousId);
        const wrongOrder=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(wrongOrder.errors.some(e=>e.kind==='wrong-message-order'),'transformed sequential message order must fail');
        await message.evaluate(n=>n.removeAttribute('transform'));
        fs.writeFileSync(path.join(config.output,f.id+'-'+width+'-order-controls.json'),JSON.stringify({wrongGroup:wrongGroup.errors,wrongOrder:wrongOrder.errors},null,2),{flag:'wx'});
        }
      }
      if(f.diagram.meaning.kind==='state') {
        const node=f.diagram.meaning.nodes[0];
        const global=await page.locator('svg').evaluate(n=>n.classList.contains('spatial-world'));
        const local=page.locator('svg a[id="'+ns+'-'+(global?'state:owner':node.id)+'"] text:visible').last();
        const saved=await local.innerHTML();
        await local.evaluate(n=>n.textContent='State owner: previous figure');
        const wrongOwner=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(wrongOwner.errors.some(e=>e.kind==='missing-visible-meaning'&&e.element===(global?'state:owner':node.id)),'foreign local ownership must fail');
        await local.evaluate((n,v)=>n.innerHTML=v,saved);
        fs.writeFileSync(path.join(config.output,f.id+'-'+width+'-owner-control.json'),JSON.stringify(wrongOwner.errors,null,2),{flag:'wx'});
      }
      if(rows.length===1) {
        const edges=f.diagram.meaning.relations;
        if(edges?.length>1){
          const route=id=>page.locator(`svg a.diagram-route[data-element="${ns}-${id}"] .relation-line:visible`);
          const first=route(edges[0].id),original=await first.getAttribute('d');
          await first.evaluate(n=>{const points=[];for(let at=n.getTotalLength();at>0;at-=2){const p=n.getPointAtLength(at);points.push(`${p.x} ${p.y}`);}const end=n.getPointAtLength(0);points.push(`${end.x} ${end.y}`);n.setAttribute('d','M'+points.join('L'));});
          const reversed=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
          assert(reversed.errors.some(e=>e.kind==='wrong-edge-direction'),'reversed geometry cannot reverse admitted meaning');
          await first.evaluate((n,d)=>n.setAttribute('d',d),original);
          const pair=edges.flatMap((a,i)=>edges.slice(i+1).filter(b=>a.from===b.from||a.to===b.to).map(b=>[a,b]))[0];
          if(pair){const a=route(pair[0].id),b=route(pair[1].id),saved=await b.getAttribute('d');
            await b.evaluate((n,d)=>n.setAttribute('d',d),await a.getAttribute('d'));
            const overlap=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
            assert(overlap.errors.some(e=>e.kind==='route-merge'),'shared endpoints cannot exempt a long common trunk');
            await b.evaluate((n,d)=>n.setAttribute('d',d),saved);
          }
          const label=page.locator(`svg a[id="${ns}-${edges[0].id}"]`);
          if(await page.locator('svg').getAttribute('data-layout-engine')?.then(v=>v?.startsWith('elkjs/'))){
            await label.evaluate(n=>n.setAttribute('transform','translate(0,10000)'));
            const detached=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
            assert(detached.errors.some(e=>e.kind==='detached-route-label'),'an unrelated label location cannot pass association');
            await label.evaluate(n=>n.removeAttribute('transform'));
          }
        }
        const text=await page.locator('svg a.diagram-element').first().locator('text:visible').first().elementHandle();
        const old=await text.getAttribute('style');await text.evaluate(n=>n.style.display='none');
        const hidden=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(hidden.errors.some(e=>e.kind==='missing-visible-meaning'),'hidden semantic wording must fail');
        await text.evaluate((n,old)=>old===null?n.removeAttribute('style'):n.setAttribute('style',old),old);
        await text.evaluate(n=>{const b=n.querySelector('tspan').getBBox();const p=document.createElementNS('http://www.w3.org/2000/svg','path');p.id='injected-collision';p.setAttribute('class','relation-line');p.setAttribute('d',`M${b.x-4} ${b.y+b.height/2}H${b.x+b.width+4}`);n.ownerSVGElement.append(p);});
        const collision=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(collision.errors.some(e=>e.kind==='path-through-text'),'crossed glyphs must fail');
        await page.locator('#injected-collision').evaluate(n=>n.setAttribute('class','relation-leader'));
        const leaderCollision=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(leaderCollision.errors.some(e=>e.kind==='path-through-text'),'quiet leaders crossing glyphs must also fail');
        await page.locator('#injected-collision').evaluate(n=>n.remove());
        fs.writeFileSync(path.join(config.output,'negative-controls.json'),JSON.stringify({hiddenLabelRejected:true,crossedTextRejected:true,crossedLeaderRejected:true}),{flag:'wx'});
      }
      await context.close();
    }
    fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify({rows,fonts:fonts.map(p=>({path:p,sha256:sha(p)})),browser:sha(config.chromium)},null,2),{flag:'wx'});
    assert.deepEqual(rows.flatMap(r=>r.errors.map(e=>({fixture:r.fixture,...e}))),[],'long-label actual glyph/path geometry');
  }finally{await browser.close();}
})().catch(e=>{fs.writeFileSync(path.join(config.output,'failure.txt'),e.stack,{flag:'wx'});console.error(e);process.exitCode=1;});
