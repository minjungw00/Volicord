// Real retained publications only. Config supplies independently selected element,
// original Source file and exact expected snippet, never a generated SVG oracle.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {spawn} = require('node:child_process');
const assert = require('node:assert/strict');
const config = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const {chromium} = require(config.playwright);
const output = path.resolve(config.output);
fs.mkdirSync(output, {recursive:false});
const hash = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const executionIdentities={viewer:hash(config.viewer),driver:hash(__filename),chromium:hash(config.chromium)};
const sleep = ms => new Promise(r => setTimeout(r,ms));
(async () => {
  const browser = await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox']});
  const results=[];
  try {
    for (const c of config.cases) {
      const pub = JSON.parse(fs.readFileSync(c.public,'utf8'));
      const d = pub.diagrams.find(d=>d.id===c.diagram);
      assert(d, 'actual admitted figure');
      const citation=pub.prose[c.unit].citations[c.citation];
      assert(citation, 'independently chosen critical citation');
      const expected=fs.readFileSync(path.join(c.repository,citation.locator)).subarray(citation.start_byte,citation.end_byte).toString('utf8');
      assert(expected.includes(c.expected_code),'Source-derived implementing range');
      const log=fs.openSync(path.join(output,c.key+'.viewer.stderr'),'wx');
      const child=spawn(config.viewer,['--runtime',c.runtime,'--project',c.project,'--language','en','--bind','127.0.0.1:0'],{stdio:['ignore','ignore',log]});
      try {
        let base;
        for(let i=0;i<300;i++) {
          base=fs.readFileSync(path.join(output,c.key+'.viewer.stderr'),'utf8').match(/http:\/\/127\.0\.0\.1:\d+\//)?.[0];
          if(base) break;
          assert(child.exitCode===null,'live Viewer startup');await sleep(50);
        }
        assert(base,'live address');
        for(const width of config.widths || [1440,390]) for(const mode of config.modes || ['live','offline','no-js']) {
          const context=await browser.newContext({viewport:{width,height:900},javaScriptEnabled:mode!=='no-js',serviceWorkers:'block'});
          const page=await context.newPage();const external=[];const errors=[];
          page.on('pageerror',e=>errors.push(e.message));
          await context.route('**/*',route=> {
            const u=route.request().url();
            if(u.startsWith(base)||u.startsWith('file:')||u.startsWith('data:')) return route.continue();
            external.push(u);return route.abort();
          });
          const query=c.work?'?view=work&work='+c.work:'?view=repository';
          const started=Date.now();
          await page.goto(mode==='live'?base+query:'file://'+path.resolve(c.offline),{waitUntil:'load'});
          const load_ms=Date.now()-started;
          const figure=page.locator('.admitted-diagram[data-semantic-fingerprint="'+d.semantic_fingerprint+'"]');
          await figure.waitFor({state:'attached'});
          // A saved snapshot has native top-level page fragments.
          if(mode!=='live') await page.evaluate(id=>{location.hash=id;},c.work?'work-'+c.work:'repository-explanation');
          await figure.scrollIntoViewIfNeeded();
          const svg=figure.locator('svg');assert.equal(await svg.count(),1);
          assert.equal(await svg.getAttribute('data-renderer'),'volicord-static-svg/2');
          const size=await page.evaluate(()=>({body:document.documentElement.scrollWidth,viewport:innerWidth}));
          assert(size.body<=size.viewport+1,JSON.stringify(size));
          await figure.screenshot({path:path.join(output,`${c.key}-${width}-${mode}.png`)});
          const svg_bytes=await svg.evaluate(n=>new TextEncoder().encode(n.outerHTML).length);
          if(config.precise_return && mode==='live') {
            await figure.locator('[data-diagram-action=in]').click();
            assert.equal(await figure.locator('.diagram-scale').textContent(),'125%');
            await figure.locator('[data-diagram-action=reset]').click();
            await figure.locator('[data-diagram-action=expand]').click();
            assert.equal(await figure.locator('[data-diagram-action=expand]').getAttribute('aria-expanded'),'true');
            const viewport=figure.locator('.diagram-viewport');await viewport.focus();
            await page.keyboard.press('ArrowRight');await sleep(100);
            const pan=await viewport.evaluate(n=>({left:n.scrollLeft,overflow:n.scrollWidth>n.clientWidth}));
            if(pan.overflow) assert(pan.left>0,'native keyboard pan');
          }
          const element=figure.locator('svg a[id$="-'+c.element+'"]');assert.equal(await element.count(),1);
          const elementId=await element.getAttribute('id');
          // Exact native SVG/text anchor access, then the actual selected Source.
          await element.focus();
          await page.evaluate(id=>{window.__diagramActivation=null;document.addEventListener('click',event=>{const n=event.target.closest('a.diagram-element');if(n?.id===id){const v=n.closest('.diagram-viewport');window.__diagramActivation={left:v.scrollLeft,top:v.scrollTop,y:scrollY};}},{capture:true});},elementId);
          await page.keyboard.press('Enter');
          const prior=await page.evaluate(()=>window.__diagramActivation);
          if(mode==='live') assert(prior,'browser position at actual element activation');
          const claimId=(await element.getAttribute('href')).slice(1);
          const claim=page.locator('[id="'+claimId+'"]');
          const actions=claim.locator('a.evidence-link');
          const action=actions.nth(c.citation);
          assert.equal(await action.count(),1,'critical purpose has exact code action');
          await action.click();
          const pointId=(await action.getAttribute('href')).slice(1);
          const point=page.locator('[id="'+pointId+'"]');
          if(mode==='live') {
            await page.waitForFunction(id=>{const n=document.getElementById(id);return n?.querySelector('.citation-inspection pre.code-region code')?.textContent.length>0;},pointId);
            assert.equal(await point.locator('.citation-inspection pre.code-region code').textContent(),expected);
          } else {
            const code=await point.locator('pre.code-region code').allTextContents();
            assert(code.includes(expected),'offline/no-JS complete exact implementing bytes');
          }
          if(config.precise_return && mode==='live') {
            await point.locator('a.return').click();
            await claim.locator('a.diagram-return').click();
            await page.waitForFunction(id=>document.activeElement?.id===id,elementId);
            const restored=await figure.locator('.diagram-viewport').evaluate(n=>({left:n.scrollLeft,top:n.scrollTop,y:scrollY}));
            for(const coordinate of ['left','top','y']) assert(Math.abs(restored[coordinate]-prior[coordinate])<=5,'exact figure/reading return '+coordinate+' '+JSON.stringify({prior,restored}));
            await page.keyboard.press('Enter');await action.click();
            assert((await page.url()).endsWith('!'+claimId),'publication-bound Source deep link');
            await page.reload({waitUntil:'load'});
            await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-inspection pre.code-region code')?.textContent.length>0,pointId);
            await point.locator('a.return').click();
            await page.waitForFunction(id=>document.activeElement?.id===id,claimId);
            await claim.locator('a.diagram-return').click();
            await page.waitForFunction(id=>document.activeElement?.id===id,elementId);
            await figure.locator('.diagram-alternative > summary').click();
            const textLink=figure.locator('a.diagram-text-link[data-element=\"'+elementId+'\"]');
            await textLink.click();
            await claim.locator('a.diagram-return').click();
            await page.waitForFunction(id=>document.activeElement?.matches('a.diagram-text-link') && document.activeElement.dataset.element===id,elementId);
            // Delayed reads cannot repaint a point after browser Back changes selection.
            await point.locator('.citation-inspection pre').evaluateAll(ns=>ns.forEach(n=>n.remove()));
            await page.route(/\/citation\?/,async route=>{await sleep(200);try{await route.continue();}catch{}});
            await element.focus();await page.keyboard.press('Enter');await action.click();await page.goBack();await sleep(300);
            assert.equal(await point.locator('.citation-inspection pre').count(),0);
            await page.unroute(/\/citation\?/);
            if(config.faults) for(const reason of ['expired','deleted','changed','withheld','comparison_unavailable']) {
              await page.goto(base+query,{waitUntil:'load'});
              await page.route(/\/citation\?/,route=>route.fulfill({status:410,contentType:'application/json',body:JSON.stringify({reason})}));
              await element.focus();await page.keyboard.press('Enter');await action.click();
              await page.waitForFunction(id=>document.getElementById(id)?.dataset.unavailable==='true',await figure.getAttribute('id'));
              assert.equal(await figure.locator('svg,.diagram-alternative,.diagram-claim').count(),0,'known failure removes all cached diagram meanings');
              assert.equal(await point.locator('pre.code-region').count(),0);
              await page.keyboard.press('Escape');
              assert(await page.locator('[id=\"'+claimId+'\"]').count(),'opaque exact return survives withholding');
              await page.unroute(/\/citation\?/);
            }
          } else if(config.precise_return) {
            await point.locator('.diagram-return-index > summary').click();
            await point.locator('.diagram-return-index a[href=\"#'+claimId+'\"]').click();
            await claim.locator('a.diagram-return').click();
            assert.equal(await page.evaluate(()=>location.hash),'#'+elementId,'exact native no-script return');
          } else {
            await point.locator('a.return').click();
            await page.goBack(); // native history also preserves script-free access
          }
          assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
          results.push({case:c.key,width,mode,realization:pub.provenance.realization_fingerprint,semantic_fingerprint:d.semantic_fingerprint,element:elementId,unit:c.unit,citation:c.citation,exact_source_bytes:Buffer.byteLength(expected),svg_bytes,load_ms,html_bytes:fs.statSync(c.offline).size,controlled_failure_reasons:mode==='live'&&config.faults?['expired','deleted','changed','withheld','comparison_unavailable']:[],external_requests:external,renderer:'volicord-static-svg/2'});
          await context.close();
        }
      } finally {child.kill('SIGTERM');fs.closeSync(log);}
    }
    assert.equal(hash(config.viewer),executionIdentities.viewer,'executable unchanged during browser checks');
    assert.equal(hash(__filename),executionIdentities.driver,'driver unchanged during browser checks');
    fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'passed',observations:results,identities:executionIdentities,human_acceptance:'unobserved'},null,2),{flag:'wx'});
    process.stdout.write(JSON.stringify({status:'passed',observations:results.length})+'\n');
  } finally {await browser.close();}
})().catch(e=>{fs.writeFileSync(path.join(output,'failure.txt'),e.stack,{flag:'wx'});console.error(e);process.exitCode=1;});
