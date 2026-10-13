// Actual output-to-reader observations; independent expectations supplied by tests.
const fs = require('fs');
const {pathToFileURL} = require('url');
const {chromium} = require(process.argv[4]);
const plan = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
function check(ok, message) { if (!ok) throw Error(message); }
(async () => {
  const browser = await chromium.launch({executablePath: process.argv[3], headless:true, args:['--no-sandbox']});
  const observations = [];
  try {
    for (const sample of plan) {
      for (const [width,height] of [[1360,900],[390,900],[320,900],[320,568]]) {
        const page = await browser.newPage({viewport:{width,height}});
        await page.goto(pathToFileURL(sample.display).href);
        await page.evaluate(() => document.fonts.ready);
        const prose = await page.locator('.prose-paragraph').allTextContents();
        check(prose.join('') === sample.prose, 'original prose differs');
        const title = await page.locator('header h2').textContent();
        check(title === sample.title, 'problem title differs');
        const start = await page.locator('.prose-paragraph').first().boundingBox();
        check(start.y < height-100, 'prose obscured by hierarchy');
        check(await page.locator('.site-purpose').allTextContents().then(v=>JSON.stringify(v)===JSON.stringify(sample.purposes)), 'site purposes differ');
        check(await page.locator('html').evaluate(e=>e.scrollWidth<=innerWidth), 'viewport overflow');
        const links=page.locator('.primary-navigation a');
        check(await links.count()===sample.codes.length, 'primary ordering/count differs');
        const screenshot=sample.display+`.${width}x${height}.png`;
        await page.screenshot({path:screenshot,fullPage:false});
        for(let n=0;n<sample.codes.length;n++) {
          const link=links.nth(n), expected=sample.codes[n];
          const actualLabel=await link.getAttribute('aria-label');
          check(actualLabel===expected.label || actualLabel.startsWith(expected.label+' · '), 'source locator differs: '+actualLabel+' / '+expected.label);
          const hash=await link.getAttribute('href');
          check(await page.locator(hash).count()===1,'source target absent');
          await link.click();
          const target=page.locator(hash);
          check(await target.locator('pre.code').textContent()===expected.text, 'selected source differs');
          check(await target.evaluate(e=>{for(let p=e;p;p=p.parentElement) if(p.tagName==='DETAILS'&&!p.open)return false;return true}), 'code hidden');
          await page.keyboard.press('Tab');
          // Locate each source through the actual sequential keyboard order.
          await page.reload();
          let reached=false;
          for(let tab=0;tab<200;tab++) {
            await page.keyboard.press('Tab');
            if(await link.evaluate(e=>e===document.activeElement)){ reached=true;break; }
          }
          check(reached,'keyboard source unreachable');
          await page.keyboard.press('Enter');
          check(await page.evaluate(()=>location.hash)===hash,'keyboard target differs');
          check(await target.locator('pre.code').isVisible(),'keyboard code invisible');
        }
        const diffs=await page.locator('pre.diff').allTextContents();
        check(diffs.length===sample.diffCount, 'false or missing comparison');
        if(sample.missingBefore) check((await page.locator('body').textContent()).includes('Missing state is not inferred'), 'missing-before warning absent');
        const langs=await page.locator('.prose-paragraph').evaluateAll(es=>es.map(e=>e.lang));
        check(langs.every(l=>l===sample.language),'Korean/generated language differs');
        await page.screenshot({path:screenshot+'.code.png',fullPage:false});
        observations.push({name:sample.name,width,height,proseY:start.y,keyboard:true,code:true,diffCount:diffs.length,language:sample.language,screenshot});
        await page.close();
      }
    }
    process.stdout.write(JSON.stringify({observations}));
  } finally { await browser.close(); }
})().catch(e=>{process.stderr.write(e.stack);process.exitCode=1});
