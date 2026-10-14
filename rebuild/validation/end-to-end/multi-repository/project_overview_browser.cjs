// Focused actual Product reading checks. Authored fixtures are support, not
// generated-output quality or human acceptance. Never generate or record prose.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {spawn, execFileSync} = require('node:child_process');
const {once} = require('node:events');
const {pathToFileURL} = require('node:url');
const crypto = require('node:crypto');
const config = JSON.parse(fs.readFileSync(process.argv[2]));
const {chromium} = require(config.playwright);
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const observations = [];
function receipt(status, error = null) {
  fs.writeFileSync(path.join(config.output, 'observations.json'), JSON.stringify({status, error, observations,
    binaries:{viewer:hash(fs.readFileSync(path.join(config.bin,'volicord-viewer')))},
    semantic_quality:'not_assessed', human_acceptance:'not_assessed'}, null, 2));
}
function run(binary, args) { return execFileSync(binary, args, {encoding:'utf8'}); }
async function listener(binary, fixture, log) {
  const child = spawn(binary, ['--runtime', fixture.runtime, '--project', fixture.project, '--bind', '127.0.0.1:0']);
  let stderr = '', stdout = '';
  child.stdout.on('data', bytes => { stdout += bytes; });
  const address = await new Promise((resolve, reject) => {
    child.stderr.on('data', bytes => {
      stderr += bytes;
      const match = stderr.match(/Volicord local viewer: (http:\/\/127\.0\.0\.1:\d+\/)/);
      if (match) resolve(match[1]);
    });
    child.once('exit', code => reject(Error(`Viewer startup exit ${code}: ${stderr}`)));
    child.once('error', reject);
  });
  return {address, async stop() {
    const exit = once(child, 'exit'); child.kill('SIGTERM');
    const [code, signal] = await exit;
    fs.writeFileSync(log + '.stderr', stderr); fs.writeFileSync(log + '.stdout', stdout);
    fs.writeFileSync(log + '.exit.json', JSON.stringify({code, signal, cause:'explicit cleanup'}));
  }};
}
async function inspect(page, fixture, locale, source, baseline) {
  await page.goto(source);
  await page.evaluate(() => document.fonts.ready);
  const overview = page.locator('#overview');
  const card = overview.locator(`article[data-work-id="${fixture.work}"]`).first();
  assert.equal(await card.getAttribute('data-work-state'), 'open');
  const meaning = card.locator('[data-question=Purpose], [data-question=RecordedGoal]').first();
  if (baseline) {
    assert.ok(await page.locator('.project-purpose').first().evaluate((e, id) => !!(e.compareDocumentPosition(document.querySelector(`article[data-work-id="${id}"]`)) & Node.DOCUMENT_POSITION_FOLLOWING), fixture.work));
    return {first_meaning_y: (await (await meaning.count() ? meaning : card.locator('h4')).boundingBox()).y};
  }
  assert.equal(await meaning.count(), 1, 'recorded problem or explicitly quoted Goal');
  assert.ok(await card.evaluate(e => !!(e.compareDocumentPosition(document.querySelector('.project-purpose')) & Node.DOCUMENT_POSITION_FOLLOWING)), 'Work precedes absent Project Purpose');
  const first = await meaning.boundingBox();
  assert.ok(first.y >= 0 && first.y < page.viewportSize().height - 24, `ordinary meaning in first screen: ${first.y}`);
  assert.ok((await page.locator('.project-purpose').first().innerText()).includes(locale === 'en' ? 'No source-grounded Project purpose is recorded.' : 'source-grounded 프로젝트 목적이 기록되지 않았습니다.'));
  assert.equal(await overview.locator('#overview-current + .work-list article, #overview-completed + .work-list article').count(), 0, 'no invented current/completed Work');
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), 'no page overflow');
  const target = card.locator('h4 a');
  await target.focus(); await page.keyboard.press('Enter');
  const detail = page.locator(`#work-${fixture.work}`);
  await detail.waitFor();
  assert.equal(await detail.count(), 1, 'keyboard opens exact Work detail');
  if (fixture.host_output === 'authored-control-recorded') {
    const citation = detail.locator('a.source-citation').first();
    await citation.focus(); await page.keyboard.press('Enter');
    const id = await citation.getAttribute('href');
    assert.equal(await page.evaluate(() => '#' + document.activeElement.id), id, 'keyboard reaches exact Source');
    const source = page.locator(`[id=${JSON.stringify(id.slice(1))}]`);
    assert.equal(await source.locator('code').textContent(), 'def value():\n    return 5\n');
    assert.ok((await source.innerText()).includes(locale === 'en' ? 'Before-state was not retained.' : '이전 상태가 보존되지 않았습니다.'));
    assert.equal(await detail.locator('.explanation-grounding').getAttribute('open'), null, 'audit secondary');
  }
  return {first_meaning_y:first.y};
}
(async () => {
  assert.ok(run('fc-list', [':lang=ko']).trim(), 'Korean font prerequisite');
  const browser = await chromium.launch({executablePath:config.chromium, headless:true, args:['--no-sandbox']});
  try {
    for (const file of config.fixtures) {
      const fixture = JSON.parse(fs.readFileSync(file));
      const label = path.basename(path.dirname(file));
      const live = await listener(path.join(config.bin, 'volicord-viewer'), fixture, path.join(config.output, label));
      try {
        for (const locale of ['en', 'ko']) {
          const snapshot = path.join(config.output, `${label}-${locale}-snapshot.html`);
          run(path.join(config.bin, 'volicord-viewer'), ['--runtime',fixture.runtime,'--project',fixture.project,'--locale',locale,'--language',fixture.language || 'en','--snapshot',snapshot]);
          const before = path.join(config.output, `${label}-${locale}-before.html`);
          if (config.before_viewer) run(config.before_viewer, ['--runtime',fixture.runtime,'--project',fixture.project,'--locale',locale,'--language',fixture.language || 'en','--snapshot',before]);
          for (const viewport of [{width:1440,height:900}, {width:390,height:900}, {width:320,height:568}]) {
            const page = await browser.newPage({viewport});
            for (const [mode, url] of [['live',`${live.address}?locale=${locale}&language=${fixture.language || 'en'}`], ['snapshot',pathToFileURL(snapshot).href]]) {
              const reading = await inspect(page, fixture, locale, url, false);
              await page.goto(url); await page.screenshot({path:path.join(config.output, `${label}-${locale}-${viewport.width}-${mode}.png`)});
              observations.push({label,locale,viewport,mode,...reading});
            }
            if (config.before_viewer) {
              const reading = await inspect(page, fixture, locale, pathToFileURL(before).href, true);
              await page.screenshot({path:path.join(config.output, `${label}-${locale}-${viewport.width}-before.png`)});
              observations.push({label,locale,viewport,mode:'before',...reading});
            }
            await page.close();
          }
        }
      } finally { await live.stop(); }
    }
  } finally { await browser.close(); }
  receipt('passed');
  process.stdout.write(JSON.stringify({status:'passed', observations:observations.length}) + '\n');
})().catch(error => { receipt('failed', error.message); console.error(error.stack); process.exitCode = 1; });
