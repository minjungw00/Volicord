// Inspect actual Product HTML against an immutable public readback, without
// authoring prose, granting semantic approval, or changing Runtime state.
const fs = require('node:fs');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const {pathToFileURL} = require('node:url');
const config = JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium} = require(config.playwright);
const saved = JSON.parse(fs.readFileSync(config.readback));
const work = saved.selected_work;
const answers = work.answers;
const digest = path => crypto.createHash('sha256').update(fs.readFileSync(path)).digest('hex');
assert.equal(answers.explanation_state, 'current');
const koreanFonts = (config.body_language.startsWith('ko') || config.displays.some(d => d.locale === 'ko'))
  ? execFileSync('fc-list', ['--format', '%{file}\n', ':lang=ko'], {encoding:'utf8'}).trim().split('\n').filter(Boolean)
  : [];
if (config.body_language.startsWith('ko') || config.displays.some(d => d.locale === 'ko')) assert.ok(koreanFonts.length > 0, 'Korean font prerequisite unavailable');
// Small public-transport preflight before starting Chromium. This checks exact
// claimed ranges, not relevance/entailment, execution success or current files.
const textHash = text => 'sha256:' + crypto.createHash('sha256').update(text).digest('hex');
for (const paragraph of answers.prose) for (const c of paragraph.citations) {
  const bytes = Buffer.from(paragraph.text);
  assert.ok(Number.isInteger(c.claim_start_byte) && Number.isInteger(c.claim_end_byte)
    && c.claim_start_byte >= 0 && c.claim_end_byte > c.claim_start_byte && c.claim_end_byte <= bytes.length, 'valid claim coordinates');
  assert.equal(textHash(bytes.subarray(c.claim_start_byte, c.claim_end_byte)), c.claim_sha256, 'exact claim hash');
}
for (const excerpt of answers.source_excerpts) {
  if (excerpt.omitted_utf8_bytes === 0) assert.equal(textHash(excerpt.text), excerpt.citation.range_sha256, 'exact complete Source range hash');
}
const labels = {
  en: {Purpose:'Goal / problem', ReportedChange:'Change / investigation', ExpectedEffect:'Result / expected effect', Verification:'Verification', NextStep:'Next step / uncertainty', Limits:'Remaining uncertainty'},
  ko: {Purpose:'목표 / 문제', ReportedChange:'변경 / 조사', ExpectedEffect:'결과 / 기대 효과', Verification:'검증', NextStep:'다음 단계 / 불확실성', Limits:'남은 불확실성'},
};
(async () => {
  const browser = await chromium.launch({executablePath:config.chromium, headless:true, args:['--no-sandbox']});
  const observations = [];
  try {
    for (const display of config.displays) for (const width of [1440, 768, 390, 320]) {
      const page = await browser.newPage({viewport:{width, height:900}});
      await page.context().setOffline(true);
      await page.goto(pathToFileURL(display.path).href + '#work-' + work.work_item_id);
      await page.evaluate(() => document.fonts.ready);
      const article = page.locator(`article[data-work-id="${work.work_item_id}"]`).filter({has:page.locator('.answer-section')}).first();
      assert.equal(await article.count(), 1, 'actual detailed Work available');
      assert.equal(await article.locator('h3').first().textContent(), config.goal_title, 'canonical request title');
      assert.equal(await article.locator('h3').first().getAttribute('data-title-basis'), 'goal-quotation');
      const prose = article.locator('.work-explanation > .answer-section > p[data-question]');
      assert.deepEqual(await prose.allTextContents(), answers.prose.map(p => p.text), 'byte-equivalent paragraph wording and order');
      assert.ok((await article.locator('.explanation-grounding').textContent()).includes(answers.provenance.realization_fingerprint), 'exact output identity');
      assert.equal(await article.locator('.explanation-grounding').getAttribute('open'), null, 'audit initially closed');
      assert.equal(await article.locator('.source-excerpt').count(), answers.source_excerpts.length, 'exact excerpt count');
      for (const excerpt of answers.source_excerpts) {
        const c = excerpt.citation;
        const candidates = article.locator(`.source-excerpt[data-source-id="${c.source_id}"]`);
        const codes = await candidates.locator('code').allTextContents();
        const matches = codes.map((text, index) => text === excerpt.text ? index : -1).filter(index => index >= 0);
        assert.equal(matches.length, 1, 'exact admitted Source range');
        const source = candidates.nth(matches[0]);
        assert.equal(await source.locator('code').textContent(), excerpt.text, 'original code bytes');
        assert.ok((await source.locator('figcaption').textContent()).includes(`${c.locator}:${c.start_line}–${c.end_line}`), 'exact Source locator and lines');
        const target = await source.getAttribute('id');
        const links = article.locator(`a.source-citation[href="#${target}"]`);
        const purposes = answers.prose.filter(p => p.citations.some(x => x.source_id === c.source_id && x.field === c.field && x.start_byte === c.start_byte && x.end_byte === c.end_byte && x.use_kind === c.use_kind)).map(p => labels[display.locale][p.question]);
        assert.ok(await links.count() > 0, 'range directly reachable');
        assert.ok((await links.allTextContents()).every(text => purposes.some(p => text.startsWith(p + ' · '))), 'citation labels explain the bound purpose');
        const bindings = source.locator('.source-claim-basis');
        const expectedClaims = answers.prose.flatMap((p, index) => p.citations
          .filter(x => x.source_id === c.source_id && x.field === c.field && x.start_byte === c.start_byte && x.end_byte === c.end_byte && x.use_kind === c.use_kind)
          .map(x => ({question:p.question, paragraph:String(index + 1), start:String(x.claim_start_byte), end:String(x.claim_end_byte),
            claim:Buffer.from(p.text).subarray(x.claim_start_byte, x.claim_end_byte).toString('utf8'), hash:x.claim_sha256})));
        assert.equal(await bindings.count(), expectedClaims.length, 'every paragraph claim retained on shared Source card');
        for (let index=0; index<expectedClaims.length; index++) {
          const expected = expectedClaims[index], bound = bindings.nth(index);
          assert.equal(await bound.getAttribute('data-question'), expected.question);
          assert.equal(await bound.getAttribute('data-paragraph'), expected.paragraph);
          assert.equal(await bound.getAttribute('data-claim-start'), expected.start);
          assert.equal(await bound.getAttribute('data-claim-end'), expected.end);
          assert.equal(await bound.getAttribute('data-statement-role'), 'generated-interpretation');
          assert.equal(await bound.locator('.source-bound-claim').getAttribute('data-content-language'), config.body_language);
          assert.equal(await bound.locator('.source-bound-claim').textContent(), expected.claim);
          assert.ok((await bound.locator('pre').textContent()).includes(expected.hash), 'exact paragraph claim hash');
        }
        await links.first().click();
        assert.equal(await page.evaluate(() => document.activeElement.id), target, 'mouse reaches exact Source');
        assert.equal(await source.locator(':scope > pre').evaluate(e => getComputedStyle(e).whiteSpace), 'pre', 'code indentation preserved');
      }
      await page.goto(pathToFileURL(display.path).href + '#work-' + work.work_item_id);
      for (let step=0; step<500; step++) {
        await page.keyboard.press('Tab');
        if (await page.evaluate(() => document.activeElement.classList.contains('source-citation'))) break;
      }
      assert.ok(await page.evaluate(() => document.activeElement.classList.contains('source-citation')), 'citation reachable by keyboard');
      const keyboardTarget = await page.evaluate(() => document.activeElement.getAttribute('href').slice(1));
      await page.keyboard.press('Enter');
      assert.equal(await page.evaluate(() => document.activeElement.id), keyboardTarget, 'Enter reaches exact range');
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), 'page remains within viewport');
      await page.goto(pathToFileURL(display.path).href + '#work-' + work.work_item_id);
      const first = await prose.first().boundingBox();
      assert.ok(first.y >= 0 && first.y < 700, 'problem appears before detailed code and audit');
      assert.ok(await prose.first().evaluate(e => parseFloat(getComputedStyle(e).lineHeight) >= 24), 'readable prose leading');
      await page.screenshot({path:`${config.output}/${display.locale}-${width}.png`});
      observations.push({locale:display.locale, body_language:config.body_language, width, display_sha256:digest(display.path), status:'passed'});
      await page.close();
    }
  } finally { await browser.close(); }
  fs.writeFileSync(`${config.output}/observations.json`, JSON.stringify({readback_sha256:digest(config.readback), output:answers.provenance.realization_fingerprint, korean_fonts:koreanFonts.map(path => ({path, sha256:digest(path)})), preflight:{claim_hashes:'passed', source_range_hashes:'passed', semantic_support:'not_assessed'}, observations, semantic_review:'pending', human_comprehension:'pending'}, null, 2));
  process.stdout.write(JSON.stringify({status:'passed', observations:observations.length, semantic_review:'pending', human_comprehension:'pending'}) + '\n');
})().catch(error => {process.stderr.write(error.stack + '\n'); process.exitCode=1;});
