// Read-only journeys through the actual grouped draft, against frozen original facts.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const {citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('./product_review_controls.cjs');

function inventory(){
  const doc=document.querySelector('.snapshot-page[id^="work-"] .doc');
  const units=[...doc.querySelectorAll('.unit[id]')].filter(n=>n.querySelector(':scope > .unit-text')).map(n=>[n.id,n.querySelector(':scope > .unit-text').textContent]).sort();
  const identified=[...document.querySelectorAll('[id]')];
  return {units,texts:[...doc.querySelectorAll('.unit-text')].map(n=>n.textContent).sort(),
    ids:identified.filter(n=>!n.closest('svg')||n.matches('a')).map(n=>n.id),allIds:identified.map(n=>n.id),
    code:[...document.querySelectorAll('pre.code-region code')].map(n=>n.textContent).sort(),
    goal:doc.querySelector('blockquote.goal').textContent,
    json:doc.querySelector('[id$="-full-verification"] pre').textContent,
    citations:[...doc.querySelectorAll('.unit > .evidence-actions')].map(n=>({unit:n.parentElement.id,links:[...n.querySelectorAll('a.evidence-link')].map(a=>({href:a.getAttribute('href'),origin:a.dataset.origin}))})),
    checks:[...doc.querySelectorAll('.verification-check')].map(n=>[...n.children].filter(n=>n.matches('h4, dl, blockquote')).map(n=>n.textContent))};
}

(async()=>{
  const c=JSON.parse(fs.readFileSync(process.argv[2])),out=path.resolve(c.work_layout_output);
  assert.ok(out.includes('/rebuild/.local/'));fs.mkdirSync(out);
  const manifest=JSON.parse(fs.readFileSync(path.join(c.output,'manifest.json'))),rows=[],staticRows=[];
  const browser=await require(c.playwright).chromium.launch({executablePath:c.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});
  let failure;
  try{
    for(const source of c.cases.filter(n=>n.work)){
      const baseline=await browser.newPage({javaScriptEnabled:false});await baseline.goto(pathToFileURL(source.offline).href+'#work-'+source.work);
      const original=await baseline.evaluate(inventory);await baseline.close();
      const pub=JSON.parse(fs.readFileSync(source.public));
      for(const candidate of manifest.candidates.filter(n=>n.id.startsWith(source.key+'-')&&n.editorial==='edited')){
        for(const scripted of [false,true])for(const width of [1440,1024,768,390,320]){
          const context=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:scripted}),page=await context.newPage();page.setDefaultTimeout(6000);
          const external=[],errors=[];page.on('pageerror',e=>errors.push(String(e)));await context.route('**/*',r=>{if(r.request().url().startsWith('file:'))return r.continue();external.push(r.request().url());return r.abort();});
          try{
            const url=pathToFileURL(path.join(c.output,candidate.file)).href+'#work-'+source.work;
            await page.goto(url);await page.evaluate(()=>document.fonts.ready);
            const actual=await page.evaluate(inventory);
            assert.deepEqual(actual.units,original.units,'all original unit identities and wording survive');
            assert.deepEqual(actual.texts,original.texts,'all original heading/lead/body wording survives');
            assert.deepEqual(actual.code,original.code,'one physical copy of every original code payload');
            assert.equal(actual.goal,original.goal);assert.equal(actual.json,original.json);
            for(const id of original.ids)assert.ok(actual.ids.includes(id),'original fragment lost: '+id);
            assert.equal(new Set(actual.allIds).size,actual.allIds.length,'unique fragment identities');
            const groupState=await page.locator('.doc[data-work-layout]').evaluate(doc=>({
              groups:[...doc.querySelectorAll(':scope > [data-reading-group]')].map(n=>n.dataset.readingGroup),
              headingNumbers:[...doc.querySelectorAll('[data-section-number]')].map(n=>[n.dataset.sectionNumber,getComputedStyle(n,'::before').content]),
              history:doc.querySelector('.recorded-timeline').closest('[data-reading-group]').dataset.readingGroup,
              commands:[...doc.querySelectorAll('.verification-execution')].map(n=>[n.closest('[data-reading-group]').dataset.readingGroup,n.open]),
              facts:[...doc.querySelectorAll('.verification-check-summary')].map(n=>[...n.children].map(n=>n.textContent)),
              primary:[...doc.querySelector('[data-reading-group="work"]').querySelectorAll('h2')].map(n=>n.textContent),
              navGroups:doc.querySelectorAll('.document-nav .work-toc-group').length,
            }));
            assert.deepEqual(groupState.groups,['work','checks','records']);assert.equal(groupState.history,'records');
            assert.deepEqual(groupState.facts,original.checks,'readable report preserves independent original facts');
            assert.ok(groupState.commands.length>0);assert.ok(groupState.commands.every(([group,open])=>group==='records'&&!open));
            assert.equal(groupState.navGroups,3);assert.equal(groupState.headingNumbers.length,3);
            groupState.headingNumbers.forEach(([number,content],i)=>{assert.equal(number,String(i+1).padStart(2,'0'));assert.equal(content,JSON.stringify(number));});
            assert.ok(!groupState.primary.some(s=>/기록의 흐름|호출 연결과 원래 기록|무엇을 확인/.test(s)));

            const figure=page.locator('.work-overview'),viewport=figure.locator(':scope > .diagram-full-view > .diagram-viewport').first(),svg=viewport.locator(':scope > svg');
            await figure.scrollIntoViewIfNeeded();
            await page.waitForFunction(()=>{const v=document.querySelector('.work-overview .diagram-full-view > .diagram-viewport'),b=v.querySelector('svg').getBoundingClientRect();return b.width>0&&b.height>0&&b.width<=v.clientWidth+1&&b.height<=v.clientHeight+1;});
            assert.equal(await svg.count(),1,'one primary drawing without an overview copy');
            await page.screenshot({path:path.join(out,`${candidate.id}-${scripted}-${width}-overview.png`)});
            if(scripted&&candidate.navigation==='camera'){
              await figure.locator('[data-world-action="reset"]').click();
              assert.match(await figure.locator('output').textContent(),/^75%/);
              await figure.locator('[data-world-action="fit"]').click();
            }else{
              await figure.locator('[data-native-scale="read"]').check();
              assert.equal(await svg.evaluate(n=>getComputedStyle(n).zoom),'0.75');
              await figure.locator('[data-native-scale="fit"]').check();
            }

            // The complete citation set of every paragraph remains accessible.
            for(const entry of original.citations){
              const unit=page.locator('[id="'+entry.unit+'"]'),text=unit.locator(':scope > p.unit-text');
              if(!await text.count()||!entry.links.length)continue;
              assert.equal(await unit.locator(':scope > .evidence-actions a').count(),1);
              const action=unit.locator('.paragraph-evidence-action');
              const group=page.locator('[id="'+(await action.getAttribute('href')).slice(1)+'"]');
              assert.deepEqual(await group.locator('.paragraph-evidence-list a').evaluateAll(ns=>ns.map(n=>({href:n.getAttribute('href'),origin:n.dataset.origin}))),entry.links,'complete exact citation set');
            }
            // Multiple code citations, a material condition, and report facts use
            // the same actual keyboard route into a single evidence pane.
            for(const index of [8,12,16]){
              const originalEntry=original.citations.find(n=>n.unit.endsWith('-p'+index));if(!originalEntry?.links.length)continue;
              const unit=page.locator('[id="'+originalEntry.unit+'"]');
              await page.evaluate(id=>{let n=document.getElementById(id);while(n){if(n.tagName==='DETAILS')n.open=true;n=n.parentElement;}},originalEntry.unit);
              const action=unit.locator('.paragraph-evidence-action');await action.focus();await page.keyboard.press('Enter');
              const group=page.locator('.paragraph-evidence-target:target');await group.waitFor({state:'visible'});
              for(let i=0;i<originalEntry.links.length;i++){
                const link=group.locator('.paragraph-evidence-list a').nth(i),href=await link.getAttribute('href');await link.focus();await page.keyboard.press('Enter');
                const point=await citationPoint(page,href);await point.waitFor({state:'visible'});
                const citation=pub.prose[index-1]?.citations[i];
                if(citation&&await point.getAttribute('data-cited-code')){
                  const expected=fs.readFileSync(path.join(source.repository,citation.locator)).subarray(citation.start_byte,citation.end_byte).toString();
                  assert.equal(await(await openEmbeddedCode(point)).textContent(),expected);await returnFromEmbeddedCode(point);
                }
                const switcher=point.locator(':scope > .point-bar .paragraph-evidence-switch');
                await switcher.focus();await page.keyboard.press('Enter');await group.waitFor({state:'visible'});
              }
              const back=group.locator('a.return');await back.focus();await page.keyboard.press('Enter');
              assert.equal(new URL(page.url()).hash,'#'+originalEntry.unit);
            }

            // Closed original records must be reachable from the real document
            // navigation and on a fresh direct-fragment load without JavaScript.
            const nav=page.locator('.doc[data-work-layout] .document-nav');await nav.evaluate(n=>n.open=true);
            const originalNav=nav.locator('.work-toc-group').last();await originalNav.locator('summary').focus();await page.keyboard.press('Enter');
            const rawLink=originalNav.locator('a').last(),href=await rawLink.getAttribute('href');await rawLink.focus();await page.keyboard.press('Enter');
            if(scripted)assert.equal(await nav.evaluate(n=>n.open),false,'TOC selection closes its overlay');
            else if(await nav.evaluate(n=>n.open)){await nav.locator(':scope > summary').focus();await page.keyboard.press('Enter');}
            const raw=page.locator('[id="'+href.slice(1)+'"]');await raw.locator('summary').first().waitFor({state:'visible'});
            if(!await raw.evaluate(n=>n.open)){await raw.locator(':scope > summary').focus();await page.keyboard.press('Enter');}
            const targetBox=await raw.locator(':scope > summary').boundingBox(),navBox=await nav.boundingBox();
            assert.ok(targetBox.y>=navBox.y+navBox.height,'selected original heading remains below persistent TOC');
            const command=raw.locator('.verification-execution').first();await command.locator(':scope > summary').focus();await page.keyboard.press('Enter');
            assert.ok(await command.evaluate(n=>n.open));
            const checkId=await command.locator('.verification-check').first().getAttribute('id');
            await page.goto(url.split('#')[0]+'#'+checkId);await page.locator('[id="'+checkId+'"]').waitFor({state:'visible'});
            assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
            rows.push({candidate:candidate.id,scripted,width,original_units:actual.texts.length,code_payloads:actual.code.length,all_citation_sets:true,overview:true,nested_record_navigation:true});
          }catch(error){await page.screenshot({path:path.join(out,`${candidate.id}-${scripted}-${width}-failure.png`)});throw error;}finally{await context.close();}
        }
        staticRows.push({candidate:candidate.id,goal_and_verification_bytes:true});
      }
    }
  }catch(e){failure=e.stack;throw e;}finally{
    fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:failure?'failed':'passed',failure,rows,staticRows,unobserved:['physical touch','screen reader','independent first reading','live Runtime lifecycle']},null,2));await browser.close();
  }
  console.log(JSON.stringify({status:'passed',conditions:rows.length}));
})().catch(e=>{console.error(e);process.exitCode=1;});
