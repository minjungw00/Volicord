// Narrow renderer regression over authored source/prose; no provider or semantic verdict.
'use strict';
const {pathToFileURL} = require('node:url');
const {chromium} = require(process.argv[4]);
const requireFact = (value, reason) => { if (!value) throw new Error(reason); };

(async () => {
  const browser = await chromium.launch({executablePath: process.argv[3], headless: true, args: ['--no-sandbox']});
  const result = {browser: browser.version(), layouts: [], human_comprehension: 'unobserved',
                  screen_reader_usability: 'unobserved', korean_fonts: 'not_assessed_by_geometry'};
  try {
    for (const width of [390, 320]) {
      const context = await browser.newContext({viewport: {width, height: 844}});
      try {
        const page = await context.newPage();
        await page.goto(pathToFileURL(process.argv[2]).href);
        const overflow = () => page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1);
        requireFact(!await overflow(), `ordinary reading overflows at ${width}px`);
        requireFact(await page.locator('details[open]').count() === 0, 'audit must start closed');
        const summary = page.locator('summary').first();
        let reached = false;
        for (let step = 0; step < 100; step++) {
          await page.keyboard.press('Tab');
          if (await summary.evaluate(e => e === document.activeElement)) { reached = true; break; }
        }
        requireFact(reached, 'disclosure unreachable by Tab');
        await page.keyboard.press('Enter');
        requireFact(await summary.evaluate(e => e.parentElement.open), 'Enter did not open native disclosure');
        requireFact(!await overflow(), 'expanded disclosure overflows');
        await page.goto(pathToFileURL(process.argv[2]).href);
        requireFact(await page.locator('script').count() === 0, 'offline HTML contains script');
        const initialHunks = await page.locator('.change-hunk:visible').count();
        const remaining = page.locator('.remaining-hunks').first();
        let remainderOpened = false;
        if (await remaining.count()) {
          const remainderSummary = remaining.locator(':scope > summary');
          let found = false;
          for (let step = 0; step < 150; step++) {
            await page.keyboard.press('Tab');
            if (await remainderSummary.evaluate(e => e === document.activeElement)) { found = true; break; }
          }
          requireFact(found, 'remaining change hunks unreachable by Tab');
          await page.keyboard.press('Enter');
          requireFact(await remaining.evaluate(e => e.open), 'Enter did not reveal remaining hunks');
          requireFact(await page.locator('.change-hunk:visible').count() > initialHunks, 'remainder has no newly visible hunks');
          requireFact(!await overflow(), 'remaining hunks overflow');
          remainderOpened = true;
          await page.goto(pathToFileURL(process.argv[2]).href);
        }
        const hunkLink = page.locator('nav[aria-label="Derived change hunks"] a').first();
        let hunkVisible = false;
        if (await hunkLink.count()) {
          await hunkLink.focus();
          const hunkHref = await hunkLink.getAttribute('href');
          await page.keyboard.press('Enter');
          hunkVisible = await page.evaluate(h => location.hash === h && document.getElementById(h.slice(1)).getClientRects().length > 0, hunkHref);
          requireFact(hunkVisible, 'keyboard navigation did not reveal exact change hunk');
          requireFact(!await overflow(), 'focused hunk overflows');
          await page.goto(pathToFileURL(process.argv[2]).href);
        }
        const link = page.locator('a[href^="#"]').first();
        reached = false;
        for (let step = 0; step < 100; step++) {
          await page.keyboard.press('Tab');
          if (await link.evaluate(e => e === document.activeElement)) { reached = true; break; }
        }
        requireFact(reached, 'source link unreachable by Tab');
        const href = await link.getAttribute('href');
        await page.keyboard.press('Enter');
        requireFact(await page.evaluate(h => location.hash === h && document.getElementById(h.slice(1)).getClientRects().length > 0, href),
                    'Enter did not reveal exact source target');
        requireFact(!await overflow(), 'source reading overflows');
        // Sensitivity control: the authored missing-source path must reproduce the defect
        // when the ordinary text wrapping rule is disabled. Code/pre wrapping is untouched.
        await page.evaluate(() => {
          const control = document.createElement('p');
          control.textContent = 'Unavailable source: repository/' + 'verylongmissingcomponent'.repeat(16) + '/missing.py';
          document.body.append(control);
        });
        requireFact(!await overflow(), 'wrapped authored sensitivity path overflows');
        await page.evaluate(() => { document.body.style.overflowWrap = 'normal'; });
        requireFact(await overflow(), 'unwrapped negative control did not reproduce overflow');
        result.layouts.push({width, normal_no_overflow: true, unwrapped_control_overflows: true,
                             keyboard_source_visible: true, disclosure_opened: true,
                             initial_visible_hunks: initialHunks, remainder_opened: remainderOpened,
                             keyboard_hunk_visible: hunkVisible});
      } finally { await context.close(); }
    }
    process.stdout.write(JSON.stringify(result));
  } finally { await browser.close(); }
})().catch(error => { process.stderr.write(String(error)); process.exitCode = 1; });
