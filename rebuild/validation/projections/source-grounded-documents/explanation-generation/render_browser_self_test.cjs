// Narrow renderer regression over authored source/prose; no provider or semantic verdict.
'use strict';
const {pathToFileURL} = require('node:url');
const {chromium} = require(process.argv[4]);
const requireFact = (value, reason) => { if (!value) throw new Error(reason); };

(async () => {
  const browser = await chromium.launch({executablePath: process.argv[3], headless: true, args: ['--no-sandbox']});
  const result = {browser: browser.version(), layouts: [], human_comprehension: 'unobserved'};
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
        await page.evaluate(() => { document.body.style.overflowWrap = 'normal'; });
        requireFact(await overflow(), 'unwrapped negative control did not reproduce overflow');
        result.layouts.push({width, normal_no_overflow: true, unwrapped_control_overflows: true,
                             keyboard_source_visible: true, disclosure_opened: true});
      } finally { await context.close(); }
    }
    process.stdout.write(JSON.stringify(result));
  } finally { await browser.close(); }
})().catch(error => { process.stderr.write(String(error)); process.exitCode = 1; });
