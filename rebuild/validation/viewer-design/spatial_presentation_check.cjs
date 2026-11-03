// Rendered presentation and modal journeys, not a semantic or human quality oracle.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const {citationPoint,openEmbeddedCode,returnFromEmbeddedCode}=require('./product_review_controls.cjs');
(async()=>{
  const c=JSON.parse(fs.readFileSync(process.argv[2])),out=path.resolve(c.presentation_output);assert.ok(out.includes('/rebuild/.local/'));fs.mkdirSync(out);
  const manifest=JSON.parse(fs.readFileSync(path.join(c.output,'manifest.json'))),rows=[];
  const browser=await require(c.playwright).chromium.launch({executablePath:c.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:c.fontconfig}});
  let failure;
  try{
    for(const candidate of manifest.candidates.filter(n=>n.editorial==='edited'))for(const width of [1440,1024,768,390,320]){
      const source=c.cases.find(n=>candidate.id.startsWith(n.key+'-'));
      const page=await browser.newPage({viewport:{width,height:1000}});page.setDefaultTimeout(6000);const errors=[];
      page.on('pageerror',e=>errors.push(String(e)));
      try{
        await page.route('**/*',r=>r.request().url().startsWith('file:')?r.continue():r.abort());
        await page.goto(pathToFileURL(path.join(c.output,candidate.file)).href+'#work-'+source.work);await page.evaluate(()=>document.fonts.ready);
        const doc=page.locator('.doc[data-work-layout]'),nav=doc.locator('.document-nav'),trigger=nav.locator(':scope > summary');
        const card=page.locator('#works .work-card[data-work-id="'+source.work+'"]'),state=await card.getAttribute('data-work-state');
        const listBadge=card.locator('.work-state-badge'),detailBadge=doc.locator('.record-strip .work-state-badge');
        assert.equal(await detailBadge.getAttribute('data-state'),state);assert.equal(await listBadge.getAttribute('data-state'),state);
        const sidebarDot=page.locator('.status-side-link[data-state="'+state+'"] .state-dot');
        const color=await sidebarDot.evaluate(n=>getComputedStyle(n).backgroundColor);
        assert.equal(await detailBadge.locator('.state-dot').evaluate(n=>getComputedStyle(n).backgroundColor),color,'detail reuses sidebar state icon/color');
        assert.equal(await listBadge.locator('.state-dot').evaluate(n=>getComputedStyle(n).backgroundColor),color,'catalogue reuses sidebar state icon/color');
        assert.deepEqual(await detailBadge.evaluate(n=>[getComputedStyle(n).color,getComputedStyle(n).backgroundColor]),await listBadge.evaluate(n=>[getComputedStyle(n).color,getComputedStyle(n).backgroundColor]));
        assert.equal(await doc.locator('.record-strip').evaluate(n=>getComputedStyle(n).borderBottomWidth),'0px','no line between status and dates');
        const gaps=await doc.locator('.work-section-stack > .article-section').evaluateAll(ns=>ns.slice(1).map((n,i)=>n.getBoundingClientRect().top-ns[i].getBoundingClientRect().bottom));
        assert.equal(gaps.length,2);assert.ok(gaps.every(gap=>gap>=24&&gap<=40));assert.ok(Math.abs(gaps[0]-gaps[1])<1,'actual section gaps are consistent');
        const layerHeads=doc.locator('[data-reading-group="checks"] > section > h2.reading-layer-heading');
        assert.deepEqual(await layerHeads.allTextContents(),['검증 보고','설명 한계']);
        const layerStyles=await layerHeads.evaluateAll(ns=>ns.map(n=>{const s=getComputedStyle(n);return [s.fontSize,s.fontWeight,s.lineHeight,s.color,s.margin,n.getBoundingClientRect().left];}));
        assert.deepEqual(layerStyles[0],layerStyles[1],'report and limits have the same heading depth/style');
        const reports=doc.locator('.verification-report-row');assert.equal(await reports.count(),await doc.locator('.verification-execution').count());
        for(const row of await reports.all()){
          assert.equal(await row.evaluate(n=>n.tagName),'DETAILS');assert.equal(await row.evaluate(n=>n.open),false);
          assert.equal(await row.evaluate(n=>getComputedStyle(n).borderTopStyle),'solid');
          const summary=row.locator(':scope > summary');await summary.focus();await summary.press('Enter');assert.ok(await row.locator('.verification-check-summary').first().isVisible());await summary.press('Enter');
        }
        await doc.locator('[data-reading-group="checks"]').scrollIntoViewIfNeeded();await page.screenshot({path:path.join(out,`${candidate.id}-${width}-checks.png`)});
        await page.evaluate(()=>{location.hash='works';});await card.waitFor({state:'visible'});await page.screenshot({path:path.join(out,`${candidate.id}-${width}-catalogue.png`)});
        await card.locator('a.card-action').click();
        await trigger.focus();await trigger.press('Enter');
        assert.equal(Math.round((await trigger.boundingBox()).height),56,'one compact TOC trigger size');
        const list=nav.locator(':scope > nav'),listBox=await list.boundingBox(),navBox=await nav.boundingBox();
        assert.ok(listBox.y>=navBox.y+navBox.height&&listBox.x>=navBox.x-1&&listBox.x+listBox.width<=navBox.x+navBox.width+1,'TOC dropdown stays in its document bounds');
        assert.ok(listBox.height<=421,'one bounded TOC list');
        assert.equal(await list.locator('ol').evaluateAll(ns=>ns.filter(n=>n.scrollHeight>n.clientHeight&&getComputedStyle(n).overflowY==='auto').length),0,'no nested scrolling lists');
        await page.screenshot({path:path.join(out,`${candidate.id}-${width}-toc.png`)});
        await page.keyboard.press('Escape');assert.equal(await nav.evaluate(n=>n.open),false);

        const paragraph=doc.locator('.unit[id$="-p12"]'),button=paragraph.locator('.paragraph-evidence-action');await paragraph.scrollIntoViewIfNeeded();
        const buttonBox=await button.boundingBox(),unitBox=await paragraph.boundingBox();
        assert.ok(Math.abs(buttonBox.x+buttonBox.width-unitBox.x-unitBox.width)<3,'quiet action sits at paragraph lower right');
        const style=await button.evaluate(n=>{const s=getComputedStyle(n);return {font:parseFloat(s.fontSize),background:s.backgroundColor,border:s.borderTopWidth};});
        assert.ok(style.font<=12);assert.equal(style.border,'0px');assert.equal(style.background,'rgba(0, 0, 0, 0)');
        await button.focus();await button.press('Enter');
        const appendix=page.locator('.appendix'),point=page.locator('.source-point:target');await point.waitFor({state:'visible'});
        assert.equal(await point.locator('.evidence-range-select:visible').count(),1,'one immediately usable range dropdown');
        assert.equal(await point.locator('.native-range-selector:visible,.range-selector:visible,.native-range-list:visible').count(),0,'no duplicate selection UI');
        const header=point.locator(':scope > .reading-panel-header');assert.equal(Math.round((await header.boundingBox()).height),56,'evidence header size');
        if(width<1200){assert.equal(await appendix.getAttribute('aria-modal'),'true');assert.ok(await doc.evaluate(n=>!!n.closest('[inert]')));}
        else{assert.equal(await appendix.getAttribute('role'),'region');assert.equal(await appendix.getAttribute('aria-modal'),null);}
        await page.screenshot({path:path.join(out,`${candidate.id}-${width}-evidence.png`)});
        await page.keyboard.press('Escape');await point.waitFor({state:'hidden'});assert.equal(await doc.evaluate(n=>!!n.closest('[inert]')),false);

        const figure=page.locator('.work-overview'),stage=figure.locator('.diagram-stage'),viewport=stage.locator(':scope > .diagram-viewport');
        await figure.scrollIntoViewIfNeeded();
        const controls=figure.locator(candidate.navigation==='camera'?'.spatial-navigation':'.native-overview-controls');
        const a=await controls.boundingBox(),b=await viewport.boundingBox();
        assert.ok(a.x>=b.x&&a.y>=b.y&&a.x+a.width<=b.x+b.width+1&&a.y+a.height<=b.y+b.height+1,'controls are inside the drawing viewport');
        assert.ok(b.height<=401,'compact normal drawing area');assert.ok(a.height<=48,'no tall separate toolbar band');
        await page.screenshot({path:path.join(out,`${candidate.id}-${width}-figure.png`)});
        if(candidate.navigation==='camera')await figure.locator('[data-world-action="expand"]').click();
        else{await figure.locator('.native-expand-toggle').focus();await page.keyboard.press('Space');}
        assert.equal(await figure.getAttribute('role'),'dialog');assert.equal(await figure.getAttribute('aria-modal'),'true');
        const panelBox=await figure.boundingBox();
        if(width<=760){assert.equal(Math.round(panelBox.width),width);assert.equal(Math.round(panelBox.x),0);assert.equal(Math.round(panelBox.y),0);}
        else assert.ok(panelBox.x>=23&&panelBox.y>=23&&panelBox.width<=1121);
        assert.equal(Math.round((await figure.locator('.figure-panel-header').boundingBox()).height),56,'figure and evidence share header size');
        assert.ok(await figure.locator('[data-panel-close]').evaluate(n=>n===document.activeElement),'modal begins at its close action');
        await page.keyboard.press('Shift+Tab');assert.ok(await figure.evaluate(n=>n.contains(document.activeElement)),'modal reverse Tab stays inside');
        await page.screenshot({path:path.join(out,`${candidate.id}-${width}-expanded.png`)});
        const pub=JSON.parse(fs.readFileSync(source.public)),diagram=pub.diagrams.find(n=>n.id===source.diagram);
        const edges=diagram.meaning.groups.flatMap(n=>n.messages);
        const covering=e=>pub.prose[e.label.unit].citations.filter(q=>q.claim_start_byte<=e.label.start_byte&&q.claim_end_byte>=e.label.end_byte);
        const edge=edges.find(e=>covering(e).length>1)||edges[0],figureId=await figure.getAttribute('id');
        const element=figure.locator('a[id="'+figureId+'-'+edge.id+'"]');await element.focus();await element.press('Enter');
        const claim=page.locator('[id="'+(await element.getAttribute('href')).slice(1)+'"]'),link=claim.locator('a.evidence-link').first();
        await link.focus();await link.press('Enter');
        const selected=page.locator('[id="'+decodeURIComponent(new URL(page.url()).hash.slice(1))+'"]');
        const picker=selected.locator(':scope > .paragraph-source-picker select');
        const next=await picker.locator('option').last().getAttribute('value');await picker.selectOption(next);
        const codePoint=await citationPoint(page,next),citation=covering(edge).at(-1);
        assert.equal(await(await openEmbeddedCode(codePoint)).textContent(),fs.readFileSync(path.join(source.repository,citation.locator)).subarray(citation.start_byte,citation.end_byte).toString());
        await returnFromEmbeddedCode(codePoint);
        const back=page.locator('[id="'+next.slice(1)+'"] > .point-bar a.return');await back.focus();await back.press('Enter');
        await page.waitForFunction(id=>document.activeElement?.id===id,await element.getAttribute('id'));
        assert.equal(await figure.getAttribute('aria-modal'),'true','exact return restores the expanded diagram');
        assert.equal(await page.locator('[aria-modal="true"]').count(),1,'only one modal owns focus after handoff');
        await page.keyboard.press('Escape');assert.equal(await figure.getAttribute('aria-modal'),null);assert.equal(await page.locator('.reading-backdrop').isVisible(),false);

        // Opening the overall evidence category must not dump source/record data.
        const records=doc.locator('[data-reading-group="records"]'),full=records.locator(':scope > details').nth(1);
        await full.locator(':scope > summary').click();assert.equal(await full.locator('pre:visible').count(),0);
        const categories=full.locator('[id$="-audit"] > details');assert.ok(await categories.count()>=6);
        await categories.last().locator(':scope > summary').click();assert.equal(await full.locator('pre:visible').count(),0,'raw originals have their own closed disclosure');
        await records.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(out,`${candidate.id}-${width}-records.png`)});
        assert.deepEqual(errors,[]);rows.push({candidate:candidate.id,width,compact_toc:true,quiet_action:true,direct_range_picker:true,internal_controls:true,shared_panel_header:true,modal_focus_and_return:true,folded_raw_originals:true});
      }catch(error){await page.screenshot({path:path.join(out,`${candidate.id}-${width}-failure.png`)});throw error;}finally{await page.close();}
    }
  }catch(error){failure=error.stack;throw error;}finally{
    fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:failure?'failed':'passed',failure,rows,unobserved:['physical touch','assistive technology','independent human reading','other browser engines','live Runtime lifecycle']},null,2));await browser.close();
  }
  console.log(JSON.stringify({status:'passed',conditions:rows.length}));
})().catch(error=>{console.error(error);process.exitCode=1;});
