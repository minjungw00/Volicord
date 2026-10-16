// Local browser support over actual recorded Product outputs. No prose authoring.
const fs = require('node:fs');
const assert = require('node:assert/strict');
const {pathToFileURL} = require('node:url');
const [configuration] = process.argv.slice(2);
const config = JSON.parse(fs.readFileSync(configuration));
const {chromium} = require(config.playwright);
function expectedSources(item) {
  return item.expected_sources || [
    {source_id:item.after,text:'def value():\n    return 5\n'},
    ...(item.before ? [{source_id:item.before,text:`def value():\n    return ${item.comparison==='changed'?4:5}\n`}] : [])
  ];
}
function checkDependencyDiagnostic(text, expected) {
  assert.ok(text.includes(expected),'distinct missing/changed dependency diagnostic');
}
async function checkSources(work, item, navigation=false) {
  for (const expected of expectedSources(item)) {
    const source=work.locator(`.source-excerpt[data-source-id="${expected.source_id}"]`);
    assert.equal(await source.count(),1,'exact current Source identity');
    assert.equal(await source.locator('code').textContent(),expected.text,'actual current code bytes');
    assert.equal(await source.locator('.source-reading-point').first().getAttribute('data-statement-role'), 'generated-interpretation', 'purpose remains a host claim');
    assert.equal(await source.locator('pre[role=region]').getAttribute('tabindex'), '0', 'keyboard can enter code scroll region');
    assert.equal(await source.locator('code').evaluate(n=>getComputedStyle(n).whiteSpace), 'pre', 'code lines preserve indentation without breaking identifiers');
    assert.equal(await source.locator('code').evaluate(n=>getComputedStyle(n).overflowWrap), 'normal', 'code identifiers do not inherit prose wrapping');
    if (expected.start_line) {
      assert.ok((await source.textContent()).includes(`${item.path}:${expected.start_line}–${expected.end_line}`),'absolute observed file lines');
      assert.ok((await source.textContent()).includes('Bounded file observation') || (await source.textContent()).includes('제한된 파일 관측'),'bounded observation notice');
    }
    if (expected.use_kind && navigation) {
      const label=expected.use_kind==='before_code'
        ? ['Before: historical observation','이전: 과거 관측']
        : ['Observed file in selected snapshot','선택 스냅샷에서 관측한 파일'];
      const targets=await source.locator('.source-reading-point[id]').evaluateAll(nodes=>nodes.map(n=>'#'+n.id));
      const links=await work.locator('a.source-citation').evaluateAll((nodes,targets)=>nodes.filter(n=>targets.includes(n.getAttribute('href'))).map(n=>n.textContent),targets);
      assert.ok(links.length>0,'exact Source has a claim-specific citation link');
      assert.ok(links.every(text=>label.some(term=>text.includes(` · ${term} · `))),'purpose and current/historical evidence use remain distinct');
    }
  }
}
async function checkCodeMeaning(page, language, expected) {
  const original = ['Synopsis','Limits'].flatMap(role=>expected.prose.filter(a=>a.question===role)).concat(expected.prose.filter(a=>!['Heading','Synopsis','Limits'].includes(a.question)));
  // Citation details also carry data-question, on an enclosing section whose
  // claim quotation has its own language. Inspect the ordinary reading paragraphs.
  const paragraphs = page.locator('.code-work-meaning .answer-section > p[data-question], .code-work-meaning > p[data-question]');
  assert.deepEqual(await paragraphs.evaluateAll(nodes=>nodes.map(n=>n.dataset.question)),
    original.map(answer=>answer.question),'Code preserves every reading paragraph in order');
  assert.deepEqual(await paragraphs.allTextContents(),original.map(answer=>answer.text),
    'Code preserves complete original interpretation');
  assert.ok(await paragraphs.evaluateAll((nodes,language)=>nodes.length>0 && nodes.every(n=>
    n.getAttribute('lang')===language && n.getAttribute('data-content-language')===language),language),
    'Code interpretation preserves requested language');
  return paragraphs;
}
async function sourcePanelRoundTrip(page, width) {
  const point=page.locator('.source-reading-point:target');
  const pointId=await point.getAttribute('id');
  const panelId=await point.locator('xpath=ancestor::figure').getAttribute('id');
  const panel=page.locator(`[id="${panelId}"]`);
  const box=await panel.boundingBox();
  assert.ok(box && Math.abs(box.y)<2 && Math.abs(box.x+box.width-width)<2,
    'selected Source panel occupies the viewport edge');
  assert.ok(width>900 ? box.x>width*.4 : box.x<2,
    'desktop retains prose beside Source; narrow screens focus Source');
  const returnLink=point.locator('.return-to-claim');
  const destination=(await returnLink.getAttribute('href')).slice(1);
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement.className),'return-to-claim',
    'native Tab reaches the exact paragraph return');
  await page.keyboard.press('Enter');
  assert.equal(await page.evaluate(()=>document.activeElement.id),destination,
    'native return restores the exact cited paragraph');
  assert.equal(await panel.evaluate(n=>getComputedStyle(n).position),'relative',
    'return closes the Source panel');
  // Reopen using the original citation for subsequent code keyboard checks.
  await page.locator(`a.source-citation[href="#${pointId}"]`).press('Enter');
  assert.equal(await page.evaluate(()=>document.activeElement.id),pointId);
}
async function check(page, item, language, expected, offline) {
  const work = page.locator(`article[data-work-id="${item.work}"]`).filter({has:page.locator('.source-excerpt')}).first();
  assert.equal(await work.count(), 1, 'selected Work has cited code');
  const source = work.locator(`.source-excerpt[data-source-id="${item.after}"]`);
  assert.equal(await source.count(), 1, 'exact current Source identity');
  await checkSources(work,item,true);
  assert.ok((await work.locator('.explanation-grounding').textContent()).includes(expected.provenance.realization_fingerprint), 'matching output identity');
  const comparison = await source.locator('.source-comparison').innerText();
  const terms = language === 'ko'
    ? {changed:'다릅니다',no_change:'동일합니다',missing_before_state:'보존되지 않았습니다'}
    : {changed:'differ',no_change:'identical bytes',missing_before_state:'not retained'};
  assert.ok(comparison.includes(terms[item.comparison]), 'honest comparison state');
  const heading = work.locator('h3').first();
  assert.equal(await heading.innerText(), language === 'ko' ? '작업 기록' : 'Recorded Work', 'neutral label does not invent a title');
  assert.equal(await heading.getAttribute('data-title-basis'), 'neutral-label', 'heading authority distinct from generated prose');
  assert.equal(await heading.getAttribute('lang'), language, 'fixed heading language');
  assert.equal(await work.locator('[data-question="Purpose"]').first().textContent(), expected.prose.find(a=>a.question==='Purpose').text, 'unchanged generated Purpose');
  for (const question of new Set(expected.prose.map(a=>a.question))) {
    const paragraphs=work.locator(`[data-content-language="${language}"][data-question="${question}"]`);
    assert.deepEqual(await paragraphs.allTextContents(),expected.prose.filter(a=>a.question===question).map(a=>a.text),'complete original interpretation paragraphs in order');
  }
  assert.equal(await source.locator('code').getAttribute('lang'), '', 'original Source language unknown');
  assert.equal(await work.locator('.explanation-grounding').getAttribute('open'), null, 'closed diagnostic material');
  assert.ok(await work.locator('.fact-states').evaluate(n => !!(n.compareDocumentPosition(n.parentElement.querySelector('.explanation-code')) & Node.DOCUMENT_POSITION_FOLLOWING)), 'verification and direction precede code');
  const boundClaim = expected.prose.flatMap(p=>p.citations.map(c=>Buffer.from(p.text).subarray(c.claim_start_byte,c.claim_end_byte).toString('utf8')))[0];
  assert.equal(await source.locator('.source-purpose-claim').first().textContent(), boundClaim, 'exact bound behavior claim explains code purpose');
  if (item.before) {
    const beforeLink = source.locator('.comparison-navigation a');
    assert.equal(await beforeLink.count(), 1, 'explicit retained comparison navigation');
    assert.ok((await beforeLink.getAttribute('href')).includes(item.before));
  }
  const ids = await page.locator('.source-excerpt').evaluateAll(nodes=>nodes.map(n=>n.id));
  assert.equal(ids.length,new Set(ids).size,'unique exact navigation targets');
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1), 'ordinary responsive page');
  if (offline) {
    assert.equal(await page.locator('form,script').count(),0,'closed offline HTML');
    assert.ok(await page.locator('a[href]').evaluateAll(nodes=>nodes.every(n=>n.getAttribute('href').startsWith('#'))),'offline fragments only');
  }
  return work;
}
(async()=>{
  const browser=await chromium.launch({executablePath:config.chromium,headless:true,args:['--no-sandbox']});
  const context=await browser.newContext();
  const page=await context.newPage();
  const observations=[];
  try {
    if (config.denied_state) {
      let diagnosticControls = 0;
      await page.setViewportSize({width:390,height:900});
      for (const item of config.cases) for (const language of ['en','ko']) for (const view of ['work','code']) {
        const scope = view === 'code' ? '&scope=work' : '';
        await page.goto(`${config.url}?view=${view}${scope}&work=${item.work}&locale=${language}&language=${language}`);
        assert.equal(await page.locator('.source-excerpt').count(),0,'unavailable dependencies supply no code excerpt');
        const expected=config.basis[item.project][language];
        assert.ok(!(await page.locator('main').innerText()).includes(expected.prose.find(a=>a.question==='ReportedChange').text),'stale interpretation withheld');
        assert.ok((await page.locator('main').innerText()).includes(language==='ko'?'근거가 변경':'Explanation is stale'),'honest stale notice');
        if (view==='work') {
          const limitation = page.locator('.explanation-diagnostic');
          assert.ok(await limitation.isVisible(), 'dependency failure is ordinary reading');
          assert.equal(await limitation.locator('xpath=ancestor::details').count(), 0, 'genuine limitation is not collapsed');
          assert.ok(await limitation.locator('.dependency-reason').isVisible(), 'readable failure reason remains visible');
          const readableReason = await limitation.locator('.dependency-reason').innerText();
          const reasonTerm = config.denied_state === 'changed_source'
            ? (language === 'ko' ? '바이트가 변경' : 'bytes have changed')
            : (language === 'ko' ? '더 이상 읽을 수 없습니다' : 'no longer readable');
          assert.ok(readableReason.includes(reasonTerm), 'ordinary reason distinguishes changed and missing Source');
          const text = await page.locator('main').textContent();
          checkDependencyDiagnostic(text,config.expected_diagnostic);
          const otherCause = config.denied_state==='changed_source'
            ? 'current Source unavailable' : 'Source bytes changed';
          assert.throws(()=>checkDependencyDiagnostic(
            text.replaceAll(config.expected_diagnostic,otherCause),config.expected_diagnostic),
            /distinct missing\/changed dependency diagnostic/,
            'negative: opposite dependency failure cannot satisfy the actual cause');
          checkDependencyDiagnostic(text,config.expected_diagnostic);
          diagnosticControls++;
        }
      }
      assert.equal(diagnosticControls,config.cases.length*2,'each Work locale checks the actual failure cause');
      process.stdout.write(JSON.stringify({status:'passed',control:config.denied_state,diagnostic_controls:diagnosticControls})+'\n');
      return;
    }
    for (const item of config.cases) for (const language of (config.languages || ['en','ko'])) {
      const expected=config.basis[item.project][language];
      for (const width of [390,768,1440]) {
        await page.setViewportSize({width,height:900});
        const url=config.offline ? pathToFileURL(config.snapshots[item.project][language]).href+`#work-${item.work}`
          : `${config.url}?view=work&work=${item.work}&locale=${language}&language=${language}`;
        if (config.offline) await context.setOffline(true);
        await page.goto(url); await page.evaluate(()=>document.fonts.ready);
        await check(page,item,language,expected,config.offline);
        await page.screenshot({path:`${config.output}/${item.comparison}-${language}-${width}-reading.png`});
        if (!config.offline) {
          await page.goto(`${config.url}?view=code&scope=work&work=${item.work}&locale=${language}&language=${language}`);
          await checkSources(page.locator('.code-work-meaning'),item);
          assert.equal(await page.locator('.code-work-meaning h3').getAttribute('data-title-basis'),'neutral-label','Code title preserves original Goal basis');
          const codeParagraphs=await checkCodeMeaning(page,language,expected);
          const questionLabels=language==='ko'
            ? {Purpose:'문제',ReportedChange:'변경 / 조사',ExpectedEffect:'결과 / 기대 효과',Limits:'남은 불확실성'}
            : {Purpose:'Problem',ReportedChange:'Change / investigation',ExpectedEffect:'Result / expected effect',Limits:'Remaining uncertainty'};
          const questions=expected.prose.filter(a=>a.question==='Limits')
            .concat(expected.prose.filter(a=>a.question!=='Limits'))
            .map(a=>a.question).filter(q=>questionLabels[q]);
          const sections=questions.filter((q,index)=>index===0 || q!==questions[index-1]).map(q=>questionLabels[q]);
          const headings=(await page.locator('.code-work-meaning h4').allTextContents()).filter(label=>Object.values(questionLabels).includes(label));
          assert.deepEqual(headings,sections,'Code groups adjacent paragraphs under one section label');
          await page.screenshot({path:`${config.output}/${item.comparison}-${language}-${width}-code-reading.png`});
          const first=codeParagraphs.first();
          await first.evaluate(n=>n.setAttribute('lang','foreign-language'));
          await assert.rejects(()=>checkCodeMeaning(page,language,expected),/requested language/,
            'negative: wrong original prose language rejected');
          await first.evaluate((n,language)=>n.setAttribute('lang',language),language);
          await first.evaluate(n=>n.removeAttribute('data-content-language'));
          await assert.rejects(()=>checkCodeMeaning(page,language,expected),/requested language/,
            'negative: missing original prose language rejected');
          await first.evaluate((n,language)=>n.setAttribute('data-content-language',language),language);
          const originalText=await first.textContent();
          await first.evaluate(n=>n.textContent='fabricated explanation control');
          await assert.rejects(()=>checkCodeMeaning(page,language,expected),/complete original interpretation/,
            'negative: changed original prose rejected');
          await first.evaluate((n,text)=>n.textContent=text,originalText);
          const lastHtml=await codeParagraphs.last().evaluate(n=>n.outerHTML);
          await codeParagraphs.last().evaluate(n=>n.remove());
          await assert.rejects(()=>checkCodeMeaning(page,language,expected),/every reading paragraph/,
            'negative: omitted original paragraph rejected');
          await page.locator('.code-work-meaning').evaluate((n,html)=>n.insertAdjacentHTML('beforeend',html),lastHtml);
          await checkCodeMeaning(page,language,expected);
          await page.goto(url);
          // Native keyboard sequence, no click/focus substitution for the link.
          for (let step=0;step<100;step++) {
            await page.keyboard.press('Tab');
            if (await page.evaluate(()=>document.activeElement.classList.contains('source-citation'))) break;
          }
          assert.ok(await page.evaluate(()=>document.activeElement.classList.contains('source-citation')),'citation reachable with Tab');
          await page.keyboard.press('Enter');
          assert.ok(await page.evaluate(()=>document.activeElement.classList.contains('source-reading-point')),'Enter focuses the exact claim beside its code');
          await sourcePanelRoundTrip(page,width);
          for (let step=0;step<12;step++) {
            await page.keyboard.press('Tab');
            if (await page.evaluate(()=>document.activeElement.matches('.source-excerpt pre[role=region]'))) break;
          }
          assert.ok(await page.evaluate(()=>document.activeElement.matches('.source-excerpt pre[role=region]')), 'Tab reaches code scrolling region');
          const scrolling = await page.evaluate(()=>{const n=document.activeElement;return {width:n.clientWidth,full:n.scrollWidth};});
          if (scrolling.full > scrolling.width + 1) {
            await page.keyboard.press('End');
            await page.keyboard.press('ArrowRight');
            await page.waitForTimeout(200);
            assert.ok(await page.evaluate(()=>document.activeElement.scrollLeft > 0), 'keyboard scrolls long code without widening prose');
          }
          const code=page.locator(`.source-excerpt[data-source-id="${item.after}"] code`);
          const original=await code.textContent();
          await code.evaluate(n=>n.textContent='fabricated control');
          await assert.rejects(()=>check(page,item,language,expected,false),/actual current code bytes/,'negative: fabricated excerpt rejected');
          await code.evaluate((n,text)=>n.textContent=text,original);
          await check(page,item,language,expected,false);
          const source=page.locator(`.source-excerpt[data-source-id="${item.after}"]`);
          await source.evaluate(n=>n.dataset.sourceId='foreign-source');
          await assert.rejects(()=>check(page,item,language,expected,false),/exact current Source identity/,'negative: wrong Source rejected');
          await page.locator('[data-source-id="foreign-source"]').evaluate((n,id)=>n.dataset.sourceId=id,item.after);
          await check(page,item,language,expected,false);
        }
        if (config.offline) {
          for (let step=0;step<100;step++) {
            await page.keyboard.press('Tab');
            if (await page.evaluate(()=>document.activeElement.classList.contains('source-citation'))) break;
          }
          assert.ok(await page.evaluate(()=>document.activeElement.classList.contains('source-citation')),'offline citation reachable with Tab');
          await page.keyboard.press('Enter');
          assert.ok(await page.evaluate(()=>document.activeElement.classList.contains('source-reading-point')),'offline Enter focuses the exact claim beside its code');
          await sourcePanelRoundTrip(page,width);
          const source=page.locator(`.source-excerpt[data-source-id="${item.after}"] code`);
          const original=await source.textContent();
          await source.evaluate(n=>n.textContent='fabricated control');
          await assert.rejects(()=>check(page,item,language,expected,true),/actual current code bytes/,'offline fabricated excerpt rejected');
          await source.evaluate((n,text)=>n.textContent=text,original);
          await check(page,item,language,expected,true);
        }
        await page.screenshot({path:`${config.output}/${item.comparison}-${language}-${width}-${config.offline?'offline':'live'}.png`});
        observations.push({comparison:item.comparison,language,width,mode:config.offline?'offline':'live',status:'passed'});
      }
    }
  } finally { await browser.close(); }
  fs.writeFileSync(`${config.output}/${config.offline?'offline':'live'}-observations.json`,JSON.stringify(observations,null,2));
  process.stdout.write(JSON.stringify({status:'passed',observations:observations.length,human_comprehension:'not_assessed'})+'\n');
})().catch(error=>{process.stderr.write(error.stack+'\n');process.exitCode=1;});
