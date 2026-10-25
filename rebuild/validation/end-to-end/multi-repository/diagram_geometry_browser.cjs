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
    for(const f of JSON.parse(fs.readFileSync(path.join(config.fixtures,'fixtures.json')))) {
      const context=await browser.newContext({viewport:{width:1440,height:1000}});const page=await context.newPage();const external=[];
      await page.route('**/*',r=>{external.push(r.request().url());return r.abort();});
      const svg=fs.readFileSync(path.join(config.fixtures,f.file),'utf8');
      await page.setContent('<html lang="'+f.ui_locale+'"><style>'+fs.readFileSync(config.css,'utf8')+'</style><figure id="'+f.id+'"><div class="diagram-viewport">'+svg+'</div></figure></html>');
      await page.evaluate(()=>document.fonts.ready);
      const result=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);rows.push({fixture:f.id,...result});
      fs.writeFileSync(path.join(config.output,f.id+'.json'),JSON.stringify(result,null,2),{flag:'wx'});
      await page.locator('figure').screenshot({path:path.join(config.output,f.id+'.png')});
      assert.deepEqual(external,[]);
      if(f.diagram.meaning.kind==='sequence') {
        const header=await page.locator('svg text').filter({hasText:f.ui_locale==='ko'?'1. par · 상대 순서 없음':'1. par · no relative order'}).elementHandle();
        assert(header,'original group header');
        const original=await header.innerHTML();
        await header.evaluate(n=>n.querySelector('tspan').textContent='1. strict · sequential');
        const wrongGroup=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(wrongGroup.errors.some(e=>e.kind==='wrong-group-order'),'wrong par/strict meaning must fail');
        await header.evaluate((n,original)=>n.innerHTML=original,original);
        const message=page.locator('svg a[id$="-cross-sequential"]');
        await message.evaluate(n=>n.setAttribute('transform','translate(0,-200)'));
        const wrongOrder=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(wrongOrder.errors.some(e=>e.kind==='wrong-message-order'),'transformed sequential message order must fail');
        await message.evaluate(n=>n.removeAttribute('transform'));
        fs.writeFileSync(path.join(config.output,f.id+'-order-controls.json'),JSON.stringify({wrongGroup:wrongGroup.errors,wrongOrder:wrongOrder.errors},null,2),{flag:'wx'});
      }
      if(rows.length===1) {
        const text=page.locator('svg a').first().locator('text').nth(1);
        const old=await text.getAttribute('style');await text.evaluate(n=>n.style.display='none');
        const hidden=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(hidden.errors.some(e=>e.kind==='missing-visible-meaning'),'hidden semantic wording must fail');
        await text.evaluate((n,old)=>old===null?n.removeAttribute('style'):n.setAttribute('style',old),old);
        await text.evaluate(n=>{const b=n.querySelector('tspan').getBBox();const p=document.createElementNS('http://www.w3.org/2000/svg','path');p.id='injected-collision';p.setAttribute('class','relation-line');p.setAttribute('d',`M${b.x-4} ${b.y+b.height/2}H${b.x+b.width+4}`);n.ownerSVGElement.append(p);});
        const collision=await inspectDiagram(page.locator('svg'),f.diagram,f.prose);
        assert(collision.errors.some(e=>e.kind==='path-through-text'),'crossed glyphs must fail');
        await page.locator('#injected-collision').evaluate(n=>n.remove());
        fs.writeFileSync(path.join(config.output,'negative-controls.json'),JSON.stringify({hiddenLabelRejected:true,crossedTextRejected:true}),{flag:'wx'});
      }
      await context.close();
    }
    fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify({rows,fonts:fonts.map(p=>({path:p,sha256:sha(p)})),browser:sha(config.chromium)},null,2),{flag:'wx'});
    assert.deepEqual(rows.flatMap(r=>r.errors.map(e=>({fixture:r.fixture,...e}))),[],'long-label actual glyph/path geometry');
  }finally{await browser.close();}
})().catch(e=>{fs.writeFileSync(path.join(config.output,'failure.txt'),e.stack,{flag:'wx'});console.error(e);process.exitCode=1;});
