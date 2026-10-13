// Exact source visibility, not element rectangle existence inside a closed disclosure.
'use strict';
const fs = require('node:fs');
const {pathToFileURL} = require('node:url');
const {chromium} = require(process.argv[4]);
const requireFact = (value, reason) => { if (!value) throw new Error(reason); };
const inspect = e => {
  let closed = 0;
  for (let p = e.parentElement; p; p = p.parentElement) {
    if (p.tagName === 'DETAILS' && !p.open) closed++;
  }
  const rect = e.getBoundingClientRect();
  return {closed, visible: e.checkVisibility(), top: rect.top, bottom: rect.bottom, text: e.textContent};
};
(async () => {
  const browser = await chromium.launch({executablePath: process.argv[3], headless: true, args: ['--no-sandbox']});
  const report = {browser: browser.version(), observations: [], screen_reader: 'unobserved', korean_fonts: 'unobserved'};
  try {
    for (const sample of JSON.parse(fs.readFileSync(process.argv[2]))) {
      for (const [width, height] of [[1360, 900], [390, 900], [320, 900], [320, 568]]) {
        const page = await browser.newPage({viewport: {width, height}});
        try {
          const open = () => page.goto(pathToFileURL(sample.display).href);
          const noOverflow = async () => requireFact(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), `${sample.name}: overflow at ${width}`);
          await open();
          const prose = await page.locator('.prose').first().evaluate(inspect);
          const nav = page.locator('.primary-navigation a');
          const count = await nav.count();
          requireFact(count === sample.codes.length, `${sample.name}: primary navigation count/order`);
          requireFact(await page.locator('script,form,iframe').count() === 0, 'active content');
          requireFact(await page.locator('details[open]').count() === 0, 'diagnostics start closed');
          await noOverflow();
          const firstScreen = async () => {
            const shown = await page.locator('.prose').first().evaluate(inspect);
            requireFact(shown.visible && shown.closed === 0 && shown.top + 100 <= height,
                        `${sample.name}: substantive prose outside initial viewport`);
            const direction = page.locator('.reading-shortcuts a').first();
            requireFact((await direction.boundingBox()).y < height, 'recorded direction shortcut outside initial viewport');
          };
          if (sample.codes.length) {
            await firstScreen();
            if (width === 1360) requireFact((await nav.last().boundingBox()).y < height, `${sample.name}: primary path outside viewport`);
            // An independently authored hierarchy regression must fail the same
            // actual-page assertion, then the restored candidate must pass.
            await page.locator('.prose').first().evaluate((e, h) => e.style.marginTop = (h + 100) + 'px', height);
            let rejected = false;
            try { await firstScreen(); } catch (error) { rejected = String(error).includes('substantive prose outside initial viewport'); }
            requireFact(rejected, 'first-screen negative control was not rejected');
            await open(); await firstScreen();
          }
          const activations = [];
          for (const mode of ['mouse', 'keyboard']) {
            for (let n = 0; n < count; n++) {
              await open();
              const link = nav.nth(n);
              const expected = sample.codes[n];
              requireFact((await link.textContent()).startsWith(expected.label), 'Source location does not match independently counted coordinates');
              if (mode === 'keyboard') {
                let reached = false;
                for (let step = 0; step < 150; step++) {
                  await page.keyboard.press('Tab');
                  if (await link.evaluate(e => e === document.activeElement)) { reached = true; break; }
                }
                requireFact(reached, 'primary link unreachable by Tab');
                await page.keyboard.press('Enter');
              } else { await link.click(); }
              const href = await link.getAttribute('href');
              requireFact(await page.evaluate(h => location.hash === h, href), 'wrong fragment');
              const code = page.locator(`[id="${href.slice(1)}"] > pre.code`);
              const shown = await code.evaluate(inspect);
              requireFact(shown.closed === 0 && shown.visible && shown.top < height && shown.bottom > 0, 'single activation left exact code hidden');
              requireFact(shown.text === expected.text.replace(/\r\n/g, '\n'), 'selected bytes changed in display');
              await noOverflow();
              activations.push({mode, selection: expected.index, exact_visible_code: true});
            }
          }
          await open();
          if (sample.codes.length) {
            const direction = page.locator('.reading-shortcuts a').first();
            await direction.focus(); await page.keyboard.press('Enter');
            const href = await direction.getAttribute('href');
            const shown = await page.locator(`[id="${href.slice(1)}"]`).evaluate(inspect);
            requireFact(shown.closed === 0 && shown.visible && shown.top < height && shown.bottom > 0,
                        'recorded direction not accessible by one activation');
          }
          await open();
          const focused = page.locator('.primary-sites .change-hunk:visible pre.diff');
          requireFact(await focused.count() === sample.diffs.length, 'focused comparison visibility/count');
          for (let n = 0; n < sample.diffs.length; n++) {
            const shown = await focused.nth(n).evaluate(inspect);
            requireFact(shown.closed === 0 && shown.visible && shown.text === sample.diffs[n], 'focused change differs from authored expectation');
          }
          if (sample.noDiff) requireFact(await page.locator('pre.diff,pre.full-diff,pre.focused-diff').count() === 0, 'invented no-change diff');
          // Diagnostics remain keyboard usable and do not overflow when expanded.
          const summary = page.locator('.reading-receipt > summary').first();
          await summary.focus(); await page.keyboard.press('Enter');
          requireFact(await summary.evaluate(e => e.parentElement.open), 'receipt disclosure did not open');
          await noOverflow();
          let baseline;
          if (height === 568 && sample.narrowBaseline) {
            await page.goto(pathToFileURL(sample.narrowBaseline).href);
            const old = await page.locator('.prose').first().evaluate(inspect);
            requireFact(old.top > height, 'narrow baseline did not reproduce first-screen failure');
            baseline = {prose_top: old.top, height, original_narrow_failure: true};
          }
          if (width === 1360 && sample.baseline) {
            await page.goto(pathToFileURL(sample.baseline).href);
            const oldProse = await page.locator('.prose').first().evaluate(inspect);
            const oldLink = page.locator('.primary-sites a').first();
            await oldLink.click();
            const href = await oldLink.getAttribute('href');
            const oldCode = await page.locator(`[id="${href.slice(1)}"] pre.code`).first().evaluate(inspect);
            requireFact(oldProse.top > 900, 'baseline failed to reproduce hierarchy problem');
            requireFact(oldCode.closed > 0 && !oldCode.visible, 'baseline failed to reproduce hidden code');
            baseline = {prose_top: oldProse.top, code_closed_ancestors: oldCode.closed, code_visible: oldCode.visible};
          }
          report.observations.push({name: sample.name, width, height, prose_top: prose.top, primary_links: count,
                                    activations, visible_focused_changes: sample.diffs.length, no_overflow: true, baseline});
        } finally { await page.close(); }
      }
    }
    process.stdout.write(JSON.stringify(report) + '\n');
  } finally { await browser.close(); }
})().catch(error => { process.stderr.write(String(error)); process.exitCode = 1; });
