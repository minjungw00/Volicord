// Local browser support over actual recorded Product outputs. No prose authoring.
const fs = require('node:fs');
const assert = require('node:assert/strict');
const {pathToFileURL} = require('node:url');
const [configuration] = process.argv.slice(2);
const config = JSON.parse(fs.readFileSync(configuration));
const {chromium} = require(config.playwright);
async function check(page, item, language, expected, offline) {
  const work = page.locator(`article[data-work-id="${item.work}"]`).filter({has:page.locator('.source-excerpt')}).first();
  assert.equal(await work.count(), 1, 'selected Work has cited code');
  const source = work.locator(`.source-excerpt[data-source-id="${item.after}"]`);
  assert.equal(await source.count(), 1, 'exact current Source identity');
  assert.equal(await source.locator('code').innerText(), 'def value():\n    return 5\n', 'actual current code bytes');
  assert.ok((await work.locator('.explanation-grounding').textContent()).includes(expected.provenance.realization_fingerprint), 'matching output identity');
  const comparison = await source.locator('.source-comparison').innerText();
  const terms = language === 'ko'
    ? {changed:'다릅니다',no_change:'동일합니다',missing_before_state:'보존되지 않았습니다'}
    : {changed:'differ',no_change:'identical bytes',missing_before_state:'not retained'};
  assert.ok(comparison.includes(terms[item.comparison]), 'honest comparison state');
  if (item.before) {
    const prior = work.locator(`.source-excerpt[data-source-id="${item.before}"] code`);
    assert.equal(await prior.innerText(), `def value():\n    return ${item.comparison==='changed'?4:5}\n`, 'actual before bytes');
  }
  const heading = work.locator('h3').first();
  assert.equal(await heading.innerText(), expected.prose.find(a=>a.question==='Purpose').text, 'problem-first output heading');
  assert.equal(await heading.getAttribute('lang'), language, 'generated language distinct from source language');
  assert.equal(await source.locator('code').getAttribute('lang'), '', 'original Source language unknown');
  assert.equal(await work.locator('.explanation-grounding').getAttribute('open'), null, 'closed diagnostic material');
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
      await page.setViewportSize({width:390,height:900});
      for (const item of config.cases) for (const language of ['en','ko']) for (const view of ['work','code']) {
        const scope = view === 'code' ? '&scope=work' : '';
        await page.goto(`${config.url}?view=${view}${scope}&work=${item.work}&locale=${language}&language=${language}`);
        assert.equal(await page.locator('.source-excerpt').count(),0,'unavailable dependencies supply no code excerpt');
        const expected=config.basis[item.project][language];
        assert.ok(!(await page.locator('main').innerText()).includes(expected.prose.find(a=>a.question==='ReportedChange').text),'stale interpretation withheld');
        assert.ok((await page.locator('main').innerText()).includes(language==='ko'?'근거가 변경':'Explanation is stale'),'honest stale notice');
        if (view==='work') assert.ok((await page.locator('main').textContent()).includes(config.expected_diagnostic),'distinct missing/changed dependency diagnostic');
      }
      process.stdout.write(JSON.stringify({status:'passed',control:config.denied_state})+'\n');
      return;
    }
    for (const item of config.cases) for (const language of ['en','ko']) {
      const expected=config.basis[item.project][language];
      for (const width of [390,768,1440]) {
        await page.setViewportSize({width,height:900});
        const url=config.offline ? pathToFileURL(config.snapshots[item.project][language]).href+`#work-${item.work}`
          : `${config.url}?view=work&work=${item.work}&locale=${language}&language=${language}`;
        if (config.offline) await context.setOffline(true);
        await page.goto(url); await page.evaluate(()=>document.fonts.ready);
        await check(page,item,language,expected,config.offline);
        if (!config.offline) {
          await page.goto(`${config.url}?view=code&scope=work&work=${item.work}&locale=${language}&language=${language}`);
          const scopedCode=page.locator(`.source-excerpt[data-source-id="${item.after}"] code`);
          assert.equal(await scopedCode.innerText(),'def value():\n    return 5\n','Work-scoped Code page uses exact excerpt');
          assert.equal(await page.locator('.code-work-meaning h3').getAttribute('lang'),language,'Code title preserves generated language');
          assert.equal(await page.locator('.code-work-meaning [data-question="ReportedChange"]').getAttribute('lang'),language,'Code interpretation preserves requested language');
          await page.goto(url);
          // Native keyboard sequence, no click/focus substitution for the link.
          for (let step=0;step<100;step++) {
            await page.keyboard.press('Tab');
            if (await page.evaluate(()=>document.activeElement.classList.contains('source-citation'))) break;
          }
          assert.ok(await page.evaluate(()=>document.activeElement.classList.contains('source-citation')),'citation reachable with Tab');
          await page.keyboard.press('Enter');
          assert.ok(await page.evaluate(()=>document.activeElement.classList.contains('source-excerpt')),'Enter focuses exact code target');
          const code=page.locator(`.source-excerpt[data-source-id="${item.after}"] code`);
          const original=await code.innerText();
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
        await page.screenshot({path:`${config.output}/${item.comparison}-${language}-${width}-${config.offline?'offline':'live'}.png`});
        observations.push({comparison:item.comparison,language,width,mode:config.offline?'offline':'live',status:'passed'});
      }
    }
  } finally { await browser.close(); }
  fs.writeFileSync(`${config.output}/${config.offline?'offline':'live'}-observations.json`,JSON.stringify(observations,null,2));
  process.stdout.write(JSON.stringify({status:'passed',observations:observations.length,human_comprehension:'not_assessed'})+'\n');
})().catch(error=>{process.stderr.write(error.stack+'\n');process.exitCode=1;});
