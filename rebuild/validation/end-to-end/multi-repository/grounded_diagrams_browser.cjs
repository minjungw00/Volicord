// Real retained publications only. Config supplies independently selected element,
// original Source file and exact expected snippet, never a generated SVG oracle.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {spawn,execFileSync} = require('node:child_process');
const assert = require('node:assert/strict');
const config = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const {chromium} = require(config.playwright);
const output = path.resolve(config.output);
fs.mkdirSync(output, {recursive:false});
const hash = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const executionIdentities={viewer:hash(config.viewer),driver:hash(__filename),chromium:hash(config.chromium)};
const sleep = ms => new Promise(r => setTimeout(r,ms));
const {openEmbeddedCode,returnFromEmbeddedCode}=require('../../viewer-design/product_review_controls.cjs');
executionIdentities.controls=hash(path.resolve(__dirname,'../../viewer-design/product_review_controls.cjs'));
const failureReasons=['expired','deleted','changed','withheld','comparison_unavailable','unavailable','missing','corrupt','unsupported'];
(async () => {
  const browserEnv={...process.env,...(config.fontconfig?{FONTCONFIG_FILE:config.fontconfig}:{})};
  const fontPaths=config.cases.some(c=>c.language?.startsWith('ko')||c.ui_locale?.startsWith('ko'))
    ? [...new Set(execFileSync('fc-list',['--format','%{file}\n',':lang=ko'],{encoding:'utf8',env:browserEnv}).trim().split('\n').filter(Boolean))] : [];
  if(config.cases.some(c=>c.language?.startsWith('ko')||c.ui_locale?.startsWith('ko'))) assert(fontPaths.length,'Korean font prerequisite unavailable');
  const koreanFonts=fontPaths.map(p=>({path:p,sha256:hash(p)}));
  const browser = await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox'],env:browserEnv});
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
      const child=spawn(config.viewer,['--runtime',c.runtime,'--project',c.project,'--language',c.language || 'en','--locale',c.ui_locale || c.language || 'en','--bind','127.0.0.1:0'],{stdio:['ignore','ignore',log]});
      let activePage;
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
          const page=await context.newPage();activePage=page;const external=[];const errors=[];
          page.on('pageerror',e=>errors.push(e.message));
          await context.route('**/*',route=> {
            const u=route.request().url();
            if(u.startsWith(base)||u.startsWith('file:')||u.startsWith('data:')) return route.continue();
            external.push(u);return route.abort();
          });
          const query=c.work?'?view=work&work='+c.work:'?view=repository';
          const started=Date.now();
          await page.goto(mode==='live'?base+query:'file://'+path.resolve(c.offline),{waitUntil:'load'});
          await page.evaluate(()=>document.fonts.ready);
          const load_ms=Date.now()-started;
          const figure=page.locator('.admitted-diagram[data-semantic-fingerprint="'+d.semantic_fingerprint+'"]');
          await figure.waitFor({state:'attached'});
          // A saved snapshot has native top-level page fragments.
          if(mode!=='live') await page.evaluate(id=>{location.hash=id;},c.work?'work-'+c.work:'repository-explanation');
          await require('./overview_entry.cjs').installEntryObserver(page);
          // Ordinary vertical navigation only. Preserve the screen before any
          // expansion, pan, selection, Source lookup or DOM geometry inspection.
          await figure.evaluate(n=>n.scrollIntoView({block:'start'}));
          await page.screenshot({path:path.join(output,`${c.key}-${width}-${mode}-initial.png`)});
          const entry=await require('./overview_entry.cjs').observeOverview(figure,d);
          fs.writeFileSync(path.join(output,`${c.key}-${width}-${mode}-entry.json`),JSON.stringify(entry,null,2),{flag:'wx'});
          assert.deepEqual(entry.errors,[],'ordinary connected overview entry');
          if(config.outline && mode==='live') {
            const toc=page.locator('.document-nav');await toc.locator(':scope > summary').click();
            await toc.locator('a.toc-figure[href=\"#'+await figure.getAttribute('id')+'\"]').click();
            assert(await figure.evaluate(n=>n===document.activeElement),'outline exact figure focus');
          }
          const svg=figure.locator('svg');assert.equal(await svg.count(),1);
          assert.equal(await svg.getAttribute('data-renderer'),(config.renderer || 'volicord-static-svg/6'));
          assert.equal(await page.locator('script').count()>0,mode==='live','live scripting versus script-free static export');
          if(mode==='live') await page.waitForFunction(id=>{
            const viewport=document.getElementById(id)?.querySelector('.diagram-viewport');
            if(!viewport)return false;
            const r=viewport.getBoundingClientRect();
            return [...viewport.querySelectorAll('.participant-box')].some(n=>{
              const b=n.getBoundingClientRect();
              return b.width>0 && b.left>=r.left-1 && b.right<=r.right+1 && b.top>=r.top-1 && b.bottom<=r.bottom+1;
            });
          },await figure.getAttribute('id'));
          const size=await page.evaluate(()=>({body:document.documentElement.scrollWidth,viewport:innerWidth}));
          assert(size.body<=size.viewport+1,JSON.stringify(size));
          await figure.screenshot({path:path.join(output,`${c.key}-${width}-${mode}.png`)});
          if(config.capture_canvas && width===(config.widths || [1440,390])[0] && mode==='live') {
            const canvas=await svg.evaluate(n=>({svg:n.outerHTML,width:n.width.baseVal.value,height:n.height.baseVal.value,styles:[...document.querySelectorAll('style')].map(s=>s.textContent).join('\n')}));
            const html='<meta charset="utf-8"><style>'+canvas.styles+'</style><style>body{margin:0;padding:0}.diagram-viewport{max-height:none;overflow:visible;border:0}.diagram-viewport .connected-overview{width:var(--map-wide-width);height:var(--map-wide-height)}.map-wide{display:inline!important}.map-narrow{display:none!important}</style><div class="diagram-viewport">'+canvas.svg+'</div>';
            fs.writeFileSync(path.join(output,c.key+'-canvas.html'),html,{flag:'wx'});
            const review=await context.newPage();await review.setViewportSize({width:Math.ceil(canvas.width),height:Math.min(1000,Math.ceil(canvas.height))});
            await review.setContent(html);await review.evaluate(()=>document.fonts.ready);await review.screenshot({path:path.join(output,c.key+'-canvas.png'),fullPage:true});await review.close();
          }
          let visual_geometry=null;
          if(config.visual_geometry) {
            visual_geometry=await require('./diagram_geometry.cjs').inspectDiagram(svg,d,pub.prose);
            fs.writeFileSync(path.join(output,`${c.key}-${width}-${mode}-geometry.json`),JSON.stringify(visual_geometry,null,2),{flag:'wx'});
            if(!config.geometry_report_only) assert.deepEqual(visual_geometry.errors,[],'actual displayed-label and glyph/path geometry');
          }
          const svg_bytes=await svg.evaluate(n=>new TextEncoder().encode(n.outerHTML).length);
          if(config.orientation_only) {
            results.push({entry,visual_geometry,case:c.key,width,mode,realization:pub.provenance.realization_fingerprint,publication:pub.provenance.publication_id,semantic_fingerprint:d.semantic_fingerprint,svg_bytes,load_ms,renderer:(config.renderer||'volicord-static-svg/6'),evidence_journey:'not_exercised'});
            await context.close();continue;
          }
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
          await page.evaluate(id=>{window.__diagramActivation=null;document.addEventListener('click',event=>{const n=event.target.closest('a.diagram-element');if(n?.id===id){const v=n.closest('.diagram-viewport');window.__diagramActivation={left:v.scrollLeft,top:v.scrollTop,y:scrollY,viewportTop:v.getBoundingClientRect().top,elementTop:n.getBoundingClientRect().top};}},{capture:true});},elementId);
          await page.keyboard.press('Enter');
          const prior=await page.evaluate(()=>window.__diagramActivation);
          if(mode==='live') assert(prior,'browser position at actual element activation');
          const claimId=(await element.getAttribute('href')).slice(1);
          const claim=page.locator('[id="'+claimId+'"]');
          if(config.connection_details) {
            const edge=(d.meaning.relations||d.meaning.groups.flatMap(g=>g.messages)).find(e=>e.id===c.element);
            if(edge) {
              const detail=claim.locator('.diagram-connection');
              const label=l=>Buffer.from(pub.prose[l.unit].text).subarray(l.start_byte,l.end_byte).toString('utf8');
              const nodes=d.meaning.nodes||d.meaning.participants;
              assert.deepEqual(await detail.locator('.connection-endpoint > span').allTextContents(),[edge.from,edge.to].map(id=>label(nodes.find(n=>n.id===id).label)),'independently admitted directed endpoints');
              assert.equal(await detail.locator('.connection-wording').evaluate(n=>n.firstChild.textContent),label(edge.label));
              const box=await detail.boundingBox();assert(box.width<=width-32,'selected connection fits the reading width');
              const geometry=await detail.evaluate(n=>({width:n.clientWidth,scroll:n.scrollWidth,labels:[...n.querySelectorAll('.connection-endpoint,.connection-wording,.connection-condition')].map(e=>({height:e.clientHeight,scroll:e.scrollWidth,width:e.clientWidth,font:getComputedStyle(e).fontSize}))}));
              assert(geometry.scroll<=geometry.width+1 && geometry.labels.every(l=>l.scroll<=l.width+1&&parseFloat(l.font)>=16),'complete native-size selected connection');
              if(config.capture_sources) await claim.screenshot({path:path.join(output,`${c.key}-${width}-${mode}-connection.png`)});
            }
          }
          // A claim may expose only its relevant subset of unit citations.
          // Select the independently specified original coordinates, not the
          // position of a filtered button in the selected detail.
          const boundPoint=page.locator('.source-point[data-reader-publication="'+pub.provenance.publication_id+'"][id$="-p'+(c.unit+1)+'-c'+(c.citation+1)+'"]');
          assert.equal(await boundPoint.count(),1,'exact independently selected citation origin');
          const boundPointId=await boundPoint.getAttribute('id');
          const action=claim.locator('a.evidence-link[href="#'+boundPointId+'"]');
          assert.equal(await action.count(),1,'critical purpose has exact code action');
          const selectedStarted=Date.now();
          await action.click();
          const pointId=(await action.getAttribute('href')).slice(1);
          const point=page.locator('[id="'+pointId+'"]');
          if(mode==='live') {
            await page.waitForFunction(id=>{const n=document.getElementById(id);return n?.querySelector('.citation-inspection pre.code-region code')?.textContent.length>0;},pointId);
            let collected='';let offset=0;let pages=0;
            while(true) {
              const chunk=await point.locator('.citation-inspection pre.code-region code').textContent();
              assert(Buffer.byteLength(chunk)<=4096,'bounded live citation page');
              assert.equal(chunk,Buffer.from(expected).subarray(offset,offset+Buffer.byteLength(chunk)).toString('utf8'),'exact original page');
              collected+=chunk;pages++;
              const next=Number(await point.locator('.citation-read').getAttribute('data-next-offset'));
              if(next===0)break;
              assert.equal(next,Buffer.byteLength(collected),'contiguous UTF-8 page offsets');
              offset=next;await point.locator('.citation-read').click();
              await point.locator('.citation-status').filter({hasText:new RegExp('bytes '+(citation.start_byte+offset)+'–')}).waitFor();
            }
            assert.equal(collected,expected,'all critical implementing bytes across bounded pages');
            assert(pages>0);
          } else {
            const code=await (await openEmbeddedCode(point)).allTextContents();
            assert(code.includes(expected),'offline/no-JS complete exact implementing bytes');
            await returnFromEmbeddedCode(point);
          }
          const selected_evidence_ms=Date.now()-selectedStarted;
          const returnStarted=Date.now();let return_ms=null;
          if(config.capture_sources) await page.screenshot({path:path.join(output,`${c.key}-${width}-${mode}-source.png`)});
          if(config.precise_return && mode==='live') {
            await point.locator('a.return').click();
            await claim.locator('a.diagram-return').click();
            await page.waitForFunction(id=>document.activeElement?.id===id,elementId);
            const restored=await figure.locator('.diagram-viewport').evaluate(n=>({left:n.scrollLeft,top:n.scrollTop,y:scrollY,viewportTop:n.getBoundingClientRect().top,elementTop:document.activeElement.getBoundingClientRect().top}));
            for(const coordinate of ['left','top','y','viewportTop','elementTop']) assert(Math.abs(restored[coordinate]-prior[coordinate])<=5,'exact figure/reading return '+coordinate+' '+JSON.stringify({prior,restored}));
            return_ms=Date.now()-returnStarted;
            await page.keyboard.press('Enter');await action.click();
            assert((await page.url()).endsWith('!'+claimId),'publication-bound Source deep link');
            await page.reload({waitUntil:'load'});
            await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-inspection pre.code-region code')?.textContent.length>0,pointId);
            await point.locator('a.return').click();
            await page.waitForFunction(id=>document.activeElement?.id===id,claimId);
            await claim.locator('a.diagram-return').click();
            await page.waitForFunction(id=>document.activeElement?.id===id,elementId);
            await figure.locator('.diagram-alternative > summary').click();
            const textLink=figure.locator('.diagram-alternative a.diagram-text-link[data-element=\"'+elementId+'\"]');
            await textLink.click();
            await claim.locator('a.diagram-return').click();
            await page.waitForFunction(id=>document.activeElement?.matches('a.diagram-text-link') && document.activeElement.dataset.element===id,elementId);
            // Delayed reads cannot repaint a point after browser Back changes selection.
            await point.locator('.citation-inspection pre').evaluateAll(ns=>ns.forEach(n=>n.remove()));
            await page.route(/\/citation\?/,async route=>{await sleep(200);try{await route.continue();}catch{}});
            await element.focus();await page.keyboard.press('Enter');await action.click();await page.goBack();await sleep(300);
            assert.equal(await point.locator('.citation-inspection pre').count(),0);
            await page.unroute(/\/citation\?/);
            // Expansion Escape restores its exact control and reading location.
            if(config.adverse_navigation) {
              if(await figure.locator('[data-diagram-action=expand]').getAttribute('aria-expanded')==='true') await figure.locator('[data-diagram-action=expand]').click();
              const control=figure.locator('[data-diagram-action=expand]');
              await control.focus();const beforeExpand=await page.evaluate(()=>scrollY);
              await page.keyboard.press('Enter');assert.equal(await control.getAttribute('aria-expanded'),'true');await figure.locator('.diagram-viewport').focus();
              await page.keyboard.press('Escape');
              assert.equal(await control.getAttribute('aria-expanded'),'false');
              assert(await control.evaluate(n=>n===document.activeElement),'expansion control focus');
              assert(Math.abs(await page.evaluate(()=>scrollY)-beforeExpand)<=5,'expansion scroll return');
              // The same element can be reached from SVG and text. Back must
              // retain the earlier activation, not the latest label-keyed entry.
              await element.focus();await page.keyboard.press('Enter');
              await claim.locator('a.diagram-return').click();
              const svgReturnUrl=page.url();
              await textLink.click();await claim.locator('a.diagram-return').click();
              assert(await textLink.evaluate(n=>n===document.activeElement));
              await page.goBack();await page.goBack();
              assert.equal(page.url(),svgReturnUrl);
              await page.waitForFunction(id=>document.activeElement?.id===id,elementId);
              // An unrelated current origin cannot be attached to a Source.
              const foreign=await page.locator('.diagram-claim').evaluateAll((ns,{claimId,pointId})=>ns.find(n=>n.id!==claimId && ![...n.querySelectorAll('a.evidence-link')].some(a=>a.getAttribute('href')==='#'+pointId))?.id,{claimId,pointId});
              if(foreign) {
                await page.evaluate(({pointId,foreign})=>{location.hash=pointId+'!'+foreign;},{pointId,foreign});
                await page.waitForFunction(()=>!document.querySelector('.fragment-unavailable').hidden);
                assert.equal(await page.locator('.source-point[data-selected=true]').count(),0);
              }
              await page.evaluate(()=>{location.hash='reader-retired-publication-diagram-missing';});
              await page.waitForFunction(()=>!document.querySelector('.fragment-unavailable').hidden);
              assert.equal(await page.locator('.source-point[data-selected=true]').count(),0);
              await page.evaluate(id=>{location.hash=id;},elementId);
            }
            // Supporting text origins retain their exact position while the
            // complete visual overview stays available throughout selection.
            for(const representation of ['.diagram-relations','.diagram-alternative']) {
              const list=figure.locator(representation);
              if(!await list.evaluate(n=>n.open))await list.locator(':scope > summary').click();
              const origin=list.locator('a.diagram-text-link[data-element="'+elementId+'"]');
              await origin.focus();await origin.evaluate(n=>n.scrollIntoView({block:'center'}));
              assert(await svg.isVisible(),'overview remains available at text-origin activation');
              const top=await origin.evaluate(n=>n.getBoundingClientRect().top);
              await page.keyboard.press('Enter');await action.click();
              await page.waitForFunction(id=>document.getElementById(id)?.querySelector('.citation-inspection pre.code-region code')?.textContent.length>0,pointId);
              assert(expected.startsWith(await point.locator('.citation-inspection pre.code-region code').textContent()),'same implementing Source from text origin');
              await point.locator('a.return').click();await claim.locator('a.diagram-return').click();
              await page.waitForFunction(({elementId,representation})=>{
                const n=document.activeElement;
                return n?.matches('a.diagram-text-link') && n.dataset.element===elementId && !!n.closest(representation);
              },{elementId,representation});
              assert(await origin.evaluate(n=>n===document.activeElement),'same text origin');
              assert(Math.abs(await origin.evaluate(n=>n.getBoundingClientRect().top)-top)<=5,'visible text-origin return '+representation);
            }
            if(config.faults && c.faults!==false) for(const reason of failureReasons) {
              await page.goto(base+query,{waitUntil:'load'});
              const failedPublication=await point.locator('.citation-inspection').getAttribute('data-publication-id');
              const independent=await page.locator('[data-reader-publication]').evaluateAll((ns,id)=>ns.filter(n=>n.dataset.readerPublication!==id).map(n=>({id:n.id,publication:n.dataset.readerPublication,html:n.outerHTML})),failedPublication);
              assert(await page.locator('.source-point pre.code-region').count()>1,'multiple original cached Source payloads challenge publication-wide withholding');
              await page.route(/\/citation\?/,route=>route.fulfill({status:410,contentType:'application/json',body:JSON.stringify({reason})}));
              await element.focus();await page.keyboard.press('Enter');await action.click();
              await page.waitForFunction(id=>document.getElementById(id)?.dataset.unavailable==='true',await figure.getAttribute('id'));
              assert.equal(await figure.locator('svg,.diagram-alternative,.diagram-claim').count(),0,'known failure removes all cached diagram meanings');
              assert.equal(await point.locator('pre.code-region').count(),0);
              assert.equal(await page.locator('[data-reader-publication]').evaluateAll((ns,id)=>ns.filter(n=>n.dataset.readerPublication===id).reduce((count,n)=>count+n.querySelectorAll('pre.code-region,.claim-quote,.unit-text,svg,.diagram-text,a.evidence-link').length,0),failedPublication),0,'no hidden code, prose, caption or evidence alias survives known publication loss');
              assert.deepEqual(await page.locator('[data-reader-publication]').evaluateAll((ns,id)=>ns.filter(n=>n.dataset.readerPublication!==id).map(n=>({id:n.id,publication:n.dataset.readerPublication,html:n.outerHTML})),failedPublication),independent,'independently published name/summary content is unchanged');
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
          if(return_ms===null)return_ms=Date.now()-returnStarted;
          assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
          results.push({entry,visual_geometry,case:c.key,width,mode,realization:pub.provenance.realization_fingerprint,semantic_fingerprint:d.semantic_fingerprint,element:elementId,unit:c.unit,citation:c.citation,exact_source_bytes:Buffer.byteLength(expected),svg_bytes,load_ms,selected_evidence_ms,return_ms,html_bytes:fs.statSync(c.offline).size,controlled_failure_reasons:mode==='live'&&config.faults&&c.faults!==false?failureReasons:[],external_requests:external,renderer:(config.renderer || 'volicord-static-svg/6')});
          await context.close();
        }
      } catch(error) {
        if(activePage && !activePage.isClosed()) {
          const observed=await activePage.evaluate(()=>({url:location.href,focus:document.activeElement?.outerHTML.slice(0,1000),sourceOpen:document.body.dataset.sourceOpen,selected:[...document.querySelectorAll('.source-point[data-selected]')].map(n=>n.id),expanded:[...document.querySelectorAll('.diagram-expanded')].map(n=>n.id)})).catch(()=>null);
          fs.writeFileSync(path.join(output,c.key+'-failure-state.json'),JSON.stringify(observed,null,2),{flag:'wx'});
          await activePage.screenshot({path:path.join(output,c.key+'-failure.png')}).catch(()=>{});
        }
        throw error;
      } finally {child.kill('SIGTERM');fs.closeSync(log);}
    }
    assert.equal(hash(config.viewer),executionIdentities.viewer,'executable unchanged during browser checks');
    assert.equal(hash(__filename),executionIdentities.driver,'driver unchanged during browser checks');
    fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'passed',observations:results,identities:executionIdentities,korean_fonts:koreanFonts,human_acceptance:'unobserved'},null,2),{flag:'wx'});
    process.stdout.write(JSON.stringify({status:'passed',observations:results.length})+'\n');
  } finally {await browser.close();}
})().catch(e=>{fs.writeFileSync(path.join(output,'failure.txt'),e.stack,{flag:'wx'});console.error(e);process.exitCode=1;});
